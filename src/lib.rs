use cfg_if::cfg_if;
use color_eyre::{eyre::bail, eyre::eyre, eyre::WrapErr, Help, Result};
use log::{debug, info, warn};
use wgpu::Backend;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::ControlFlow;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};
use winit_input_helper::WinitInputHelper;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

struct RenderState {
	surface: wgpu::Surface,
	device: wgpu::Device,
	queue: wgpu::Queue,
	config: wgpu::SurfaceConfiguration,
	size: winit::dpi::PhysicalSize<u32>,
	window: Window,
}
impl RenderState {
	pub async fn new(window: Window) -> Result<Self> {
		let size = window.inner_size();

		let backends =
			wgpu::util::backend_bits_from_env().unwrap_or(wgpu::Backends::all());
		let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
			backends,
			dx12_shader_compiler: Default::default(),
		});

		log::debug!(
			"Available wgpu adapters: {:#?}",
			instance
				.enumerate_adapters(backends)
				.map(|a| a.get_info())
				.collect::<Vec<_>>()
		);

		// Safety: we store both `window` and `surface` in `State` so we can be sure that `surface`
		// is dropped first.
		let surface = unsafe { instance.create_surface(&window) }?;

		let adapter = instance
			.request_adapter(&wgpu::RequestAdapterOptions {
				power_preference: wgpu::PowerPreference::LowPower,
				force_fallback_adapter: false,
				/// Surface that is required to be presentable with the requested adapter. This does not
				/// create the surface, only guarantees that the adapter can present to said surface.
				compatible_surface: Some(&surface),
			})
			.await
			.ok_or(eyre!("Failed to get a wgpu Adapter"))?;
		if !adapter.is_surface_supported(&surface) {
			bail!("Adapter does not support surface!");
		}
		log::debug!("Chosen adapter: {:#?}", adapter.get_info());

		let (device, queue) = {
			let limits = if cfg!(target_arch = "wasm32") {
				wgpu::Limits::downlevel_webgl2_defaults()
			} else {
				wgpu::Limits::downlevel_defaults()
			};
			let desc = wgpu::DeviceDescriptor {
				label: None,
				features: wgpu::Features::empty(),
				limits,
			};
			adapter
				.request_device(&desc, None)
				.await
				.wrap_err("Failed to get wgpu Device")
				.with_note(|| format!("WGPU Adapter was: {:#?}", adapter.get_info()))?
		};

		let config = {
			// NOTE: all capabilities have the most preferred option as the 0th element.
			let caps = surface.get_capabilities(&adapter);
			let format = caps
				.formats
				.iter()
				.copied()
				.filter(|f| f.is_srgb())
				.next()
				.unwrap_or_else(|| {
					warn!("GPU doesn't support sRGB, colors might not be as expected!");
					caps.formats[0]
				});
			wgpu::SurfaceConfiguration {
				// This lets the texture write to the screen (?)
				usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
				format,
				width: size.width,
				height: size.height,
				present_mode: caps.present_modes[0],
				alpha_mode: caps.alpha_modes[0],
				view_formats: vec![],
			}
		};

		surface.configure(&device, &config);

		Ok(Self {
			surface,
			device,
			queue,
			config,
			size,
			window,
		})
	}

	fn render(&mut self) -> Result<()> {
		let output = self.surface.get_current_texture()?;
		let view = output
			.texture
			.create_view(&wgpu::TextureViewDescriptor::default());
		let mut encoder =
			self.device
				.create_command_encoder(&wgpu::CommandEncoderDescriptor {
					label: Some("Render Encoder"),
				});

		encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			label: Some("Render Pass"),
			color_attachments: &[Some(wgpu::RenderPassColorAttachment {
				view: &view,
				resolve_target: None,
				ops: wgpu::Operations {
					load: wgpu::LoadOp::Clear(wgpu::Color {
						r: 0.1,
						g: 0.2,
						b: 0.3,
						a: 1.0,
					}),
					store: true,
				},
			})],
			depth_stencil_attachment: None,
		});

		let commands = encoder.finish();
		self.queue.submit([commands]);
		output.present();

		Ok(())
	}
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() -> Result<()> {
	color_eyre::install()?;
	cfg_if! {
		if #[cfg(target_arch = "wasm32")] {
			std::panic::set_hook(Box::new(console_error_panic_hook::hook));
			console_log::init_with_level(log::Level::Debug)
				.expect("Couldn't initialize logger");
		} else {
			use env_logger::Env;
			let env = Env::default().default_filter_or("wgpu_experiments=debug");
			env_logger::Builder::from_env(env).init();
		}
	}

	let event_loop = EventLoop::new();
	let window = WindowBuilder::new().build(&event_loop).unwrap();

	#[cfg(target_arch = "wasm32")]
	{
		// Winit prevents sizing with CSS, so we have to set
		// the size manually when on web.
		use winit::dpi::PhysicalSize;
		window.set_inner_size(PhysicalSize::new(450, 400));

		use winit::platform::web::WindowExtWebSys;
		web_sys::window()
			.and_then(|win| win.document())
			.and_then(|doc| {
				let dst = doc.get_element_by_id("wasm-example")?;
				let canvas = web_sys::Element::from(window.canvas());
				dst.append_child(&canvas).ok()?;
				Some(())
			})
			.expect("Couldn't append canvas to document body.");
	}

	let mut input = WinitInputHelper::new();
	let mut state = RenderState::new(window)
		.await
		.wrap_err("Error when initializing wgpu state")?;

	info!("Starting event loop");
	event_loop.run(move |event, _e_loop, control_flow| {
		// Draw the current frame
		if let Event::RedrawRequested(_) = event {
			if let Err(e) = state.render() {
				log::error!("{}", e);
				*control_flow = ControlFlow::Exit;
				return;
			}
		}

		// if true, run app logic
		if input.update(&event) {
			// close events
			{
				if input.key_pressed(VirtualKeyCode::Escape)
					|| input.close_requested()
					|| input.destroyed()
				{
					info!("Exiting");
					*control_flow = ControlFlow::Exit;
				}
			}

			update(&input, &mut state);
			state.window.request_redraw();
		}
	})
}

fn update(_i: &WinitInputHelper, _s: &mut RenderState) {
	//todo
}
