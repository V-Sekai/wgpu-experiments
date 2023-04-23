use color_eyre::Result;
use env_logger::Env;
use log::{debug, info};
use winit::event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

struct State {}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("wgpu_experiments=info")).init();

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
                    log::info!("Exiting");
                    *control_flow = ControlFlow::Exit;
                }
            }

            update(&input, &mut state);
            window.request_redraw();
        }
    })
}

fn render(s: &State) -> Result<()> {
    // do something..
    Ok(())
}

fn update(i: &WinitInputHelper, s: &mut State) {
    //todo
}
