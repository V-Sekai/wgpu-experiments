use cfg_if::cfg_if;
use color_eyre::Result;
use log::{debug, info};
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::ControlFlow;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

struct State {}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn run() {
	cfg_if! {
		if #[cfg(target_arch = "wasm32")] {
			std::panic::set_hook(Box::new(console_error_panic_hook::hook));
			console_log::init_with_level(log::Level::Info)
				.expect("Couldn't initialize logger");
		} else {
			use env_logger::Env;
			let env = Env::default().default_filter_or("wgpu_experiments=info");
			env_logger::Builder::from_env(env).init();
		}
	}

	let event_loop = EventLoop::new();
	let window = WindowBuilder::new().build(&event_loop).unwrap();
	let mut input = WinitInputHelper::new();
	let mut state = State {};

	debug!("Starting event loop");
	event_loop.run(move |event, _e_loop, control_flow| {
		// Draw the current frame
		if let Event::RedrawRequested(_) = event {
			if let Err(e) = render(&state) {
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
			window.request_redraw();
		}
	})
}

fn render(_s: &State) -> Result<()> {
	// do something..
	Ok(())
}

fn update(_i: &WinitInputHelper, _s: &mut State) {
	//todo
}
