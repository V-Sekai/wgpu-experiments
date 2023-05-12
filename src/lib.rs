use std::borrow::Borrow;

use cfg_if::cfg_if;
use color_eyre::{eyre::bail, eyre::eyre, eyre::WrapErr, Help, Result};
use log::error;
use log::{debug, info, warn};
use wgpu::util::{DeviceExt, RenderEncoder};
use winit::dpi::PhysicalSize;
use winit::event::VirtualKeyCode;
use winit::event_loop::ControlFlow;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};
use winit_input_helper::WinitInputHelper;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::types::{Pos, Rgb, Vertex};

mod types;

struct GameState {}
impl GameState {
	pub fn new() -> Self {
		GameState {}
	}

	pub fn update(&mut self, _input: &WinitInputHelper) {
		// Nothing yet...
	}
}

struct RenderState {
	// Fields dropped in order of declaration.
	// Surface must be dropped before window.
	surface: wgpu::Surface,
	device: wgpu::Device,
	queue: wgpu::Queue,
	config: wgpu::SurfaceConfiguration,
	window: Window,
	pipeline: wgpu::RenderPipeline,
	vtx_buf: wgpu::Buffer,
	idx_buf: wgpu::Buffer,
	num_indices: u32,
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

		debug!(
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
		debug!("Chosen adapter: {:#?}", adapter.get_info());

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

		let pipeline = {
			// Can also use `include_wgsl!()`
			let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
				label: Some("shader.wgsl"),
				source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
			});

			let pipeline_layout =
				device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
					label: Some("Render Pipeline Layout"),
					bind_group_layouts: &[],
					push_constant_ranges: &[],
				});

			device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
				label: Some("Render Pipeline"),
				layout: Some(&pipeline_layout),
				vertex: wgpu::VertexState {
					module: &shader,
					entry_point: "vs_main",
					buffers: &[Vertex::vb_layout()],
				},
				fragment: Some(wgpu::FragmentState {
					module: &shader,
					entry_point: "fs_main",
					targets: &[Some(wgpu::ColorTargetState {
						// Shader texture format will be same as what we configured earlier
						format: config.format,
						// Blend will simply replace old pixel data with new
						blend: Some(wgpu::BlendState::REPLACE),
						// We are writing to all RGBA channels
						write_mask: wgpu::ColorWrites::ALL,
					})],
				}),
				primitive: wgpu::PrimitiveState {
					topology: wgpu::PrimitiveTopology::TriangleList,
					strip_index_format: None,
					front_face: wgpu::FrontFace::Ccw,
					cull_mode: Some(wgpu::Face::Back),
					// The next three avoid needing additional features
					unclipped_depth: false,
					polygon_mode: wgpu::PolygonMode::Fill,
					conservative: false,
				},
				depth_stencil: None,
				// We won't be using multisampling, so do 1x
				multisample: wgpu::MultisampleState {
					count: 1,
					mask: !0,
					alpha_to_coverage_enabled: false,
				},
				// I don't understand this one, but the tutorial set it to `None`
				multiview: None,
			})
		};

		const VERTICES: &[Vertex] = &[
			Vertex::new(Pos::new(0.0, 0.5, 0.0), Rgb::new(1., 0., 0.)),
			Vertex::new(Pos::new(-0.5, -0.5, 0.0), Rgb::new(0., 1., 0.)),
			Vertex::new(Pos::new(0.5, -0.5, 0.0), Rgb::new(0., 0., 1.)),
		];

		const INDICES: &[u16] = &[0, 1, 2];

		let vtx_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Vertex Buffer"),
			contents: bytemuck::cast_slice(VERTICES),
			usage: wgpu::BufferUsages::VERTEX,
		});

		let idx_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Index Buffer"),
			contents: bytemuck::cast_slice(INDICES),
			usage: wgpu::BufferUsages::INDEX,
		});

		Ok(Self {
			surface,
			device,
			queue,
			config,
			window,
			pipeline,
			vtx_buf,
			idx_buf,
			num_indices: INDICES.len() as u32,
		})
	}

	pub fn render(&mut self, _gs: &GameState) -> Result<(), wgpu::SurfaceError> {
		let output = self.surface.get_current_texture()?;
		let view = output
			.texture
			.create_view(&wgpu::TextureViewDescriptor::default());
		let mut encoder =
			self.device
				.create_command_encoder(&wgpu::CommandEncoderDescriptor {
					label: Some("Render Encoder"),
				});

		{
			let mut render_pass =
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

			render_pass.set_pipeline(&self.pipeline);
			render_pass.set_vertex_buffer(0, self.vtx_buf.slice(..));
			render_pass
				.set_index_buffer(self.idx_buf.slice(..), wgpu::IndexFormat::Uint16);
			// render_pass.draw(0..self.num_vertices, 0..1)
			render_pass.draw_indexed(0..self.num_indices, 0, 0..1)
		}

		let commands = encoder.finish();
		self.queue.submit([commands]);
		output.present();

		Ok(())
	}

	pub fn resize(&mut self, size: PhysicalSize<u32>) {
		if size.width == 0 && size.height == 0 {
			return;
		}
		self.config.width = size.width;
		self.config.height = size.height;
		self.surface.configure(&self.device, &self.config);
	}

	pub fn size(&self) -> PhysicalSize<u32> {
		PhysicalSize {
			width: self.config.width,
			height: self.config.height,
		}
	}
}

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

		use winit::platform::web::WindowExtWebSys;
		web_sys::window()
			.and_then(|win| win.document())
			.and_then(|doc| {
				let parent = doc.get_element_by_id("wgpu-parent")?;
				let width = parent.client_width() as u32;
				let height = parent.client_height() as u32;
				info!("width: {}, height: {}", width, height);

				window.set_inner_size(PhysicalSize::new(width, height));

				let canvas = window.canvas();
				let style = canvas.style();
				style.remove_property("width").unwrap();
				style.remove_property("height").unwrap();

				parent.append_child(&canvas).ok()?;

				Some(())
			})
			.expect("Couldn't append canvas to document body.");
	}

	let mut input = WinitInputHelper::new();
	let mut state = RenderState::new(window)
		.await
		.wrap_err("Error when initializing wgpu state")?;
	let mut game_state = GameState::new();

	info!("Starting event loop");
	event_loop.run(move |event, _e_loop, control_flow| {
		// When true, input_helper is done processing events.
		if !input.update(&event) {
			return;
		}

		// Handle close events
		{
			if input.key_pressed(VirtualKeyCode::Escape)
				|| input.close_requested()
				|| input.destroyed()
			{
				info!("Close Requested");
				*control_flow = ControlFlow::Exit;
				return;
			}
		}

		if let Some(size) = input.window_resized() {
			state.resize(size);
		}

		game_state.update(&input);

		use wgpu::SurfaceError as E;
		match state.render(&game_state) {
			Ok(_) => {}
			Err(E::Lost) => state.resize(state.size()),
			Err(E::OutOfMemory) => {
				error!("Out of memory!");
				*control_flow = ControlFlow::Exit;
			}
			Err(err) => {
				warn!("Error in event loop: {:?}", err);
			}
		}
	})
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub async fn wasm_start() -> Result<(), JsError> {
	run().await.map_err(|e| {
		let e: &(dyn std::error::Error + Send + Sync + 'static) = e.as_ref();
		JsError::from(e)
		// let b: Box<dyn std::error::Error + 'static> = e.into();
		// JsError::from(b)
	})
}
