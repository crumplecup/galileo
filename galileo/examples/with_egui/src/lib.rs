use std::sync::Arc;

use winit::{
    event::{Event, KeyEvent, WindowEvent},
    event_loop::{self, ControlFlow, EventLoop},
    window::Window,
};

mod app;
mod event;
mod run_ui;
mod state;

pub use app::App;
pub use event::UserEvent;
pub use state::State;

pub async fn run(
    window: Arc<Window>,
    event_loop: EventLoop<UserEvent>,
    proxy: event_loop::EventLoopProxy<UserEvent>,
) {
    let mut state = state::State::new(Arc::clone(&window), proxy).await;

    let _ = event_loop.run(move |event, ewlt| {
        ewlt.set_control_flow(ControlFlow::Wait);

        match event {
            Event::AboutToWait => {
                state.about_to_wait();
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window().id() => {
                match event {
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                logical_key:
                                    winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape),
                                ..
                            },
                        ..
                    } => ewlt.exit(),
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::RedrawRequested => match state.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            state.resize(state.size)
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => ewlt.exit(),
                        Err(wgpu::SurfaceError::Timeout) => {
                            // Ignore timeouts.
                        }
                    },
                    other => {
                        state.handle_event(other);
                        window.request_redraw();
                        return;
                    }
                };
                state.handle_event(event);
                window.request_redraw();
            }
            _ => {}
        }
    });
}
