use crate::{State, UserEvent};
use galileo::GalileoResult;
use std::sync::Arc;

use winit::{
    event::{KeyEvent, WindowEvent},
    event_loop, window,
};

pub struct App {
    state: Option<State>,
    proxy: event_loop::EventLoopProxy<UserEvent>,
    delegate: galileo::control::EventProcessor,
}

impl App {
    pub fn new(proxy: event_loop::EventLoopProxy<UserEvent>) -> Self {
        let mut delegate = galileo::control::EventProcessor::default();
        delegate.add_handler(galileo::control::MapController::default());
        Self {
            state: None,
            proxy,
            delegate,
        }
    }

    pub fn request_window(
        &self,
        event_loop: &event_loop::ActiveEventLoop,
        attributes: Option<window::WindowAttributes>,
    ) -> GalileoResult<()> {
        // If attributes is None, use a default set of window attributes
        let attr = if let Some(attributes) = attributes {
            attributes
        } else {
            window::Window::default_attributes()
                .with_title("Galileo + Egui")
                .with_transparent(true)
                .with_visible(false)
        };
        // Create window the non-deprecated way from the ActiveEventLoop
        let window = event_loop.create_window(attr)?;
        // let adapter = accesskit_winit::Adapter::with_event_loop_proxy(&window, self.proxy.clone());
        window.set_visible(true);
        let window = Arc::new(window);
        // let proxy = self.proxy.clone();
        // Did I create a window?
        tracing::trace!("Window created: {:?}", window.id());
        let proxy = self.proxy.clone();
        tokio::spawn(async move {
            let state = State::new(window, proxy.clone()).await;
            match proxy.send_event(UserEvent::State(state)) {
                Ok(_) => tracing::info!("State sent."),
                Err(e) => tracing::warn!("State not sent: {}", e.to_string()),
            }
        });
        Ok(())
    }

    pub fn delegate(&mut self, event: &winit::event::WindowEvent) {
        // Phone emulator in browsers works funny with scaling, using this code fixes it.
        // But my real phone works fine without it, so it's commented out for now, and probably
        // should be deleted later, when we know that it's not needed on any devices.

        // #[cfg(target_arch = "wasm32")]
        // let scale = window.scale_factor();
        //
        // #[cfg(not(target_arch = "wasm32"))]
        let scale = 1.0;

        if let Some(state) = &mut self.state {
            let map = state.map_mut();
            if let Some(raw_event) = map.input_handler_mut().process_user_input(event, scale) {
                let mut content = map.map().write().expect("Poisoned lock.");
                self.delegate.handle(raw_event, &mut content);
            }
            state.window().request_redraw();
            tracing::trace!("Redraw requested for {:?}", state.window().id());
        }
    }

    // pub async fn request_lens(
    //     adapter: accesskit_winit::Adapter,
    //     proxy: event_loop::EventLoopProxy<Event>,
    //     window: Arc<winit::window::Window>,
    // ) -> Arrive<()> {
    //     let lens = Lens::new(adapter, proxy.clone(), window).await;
    //     proxy.send_event(Event::Lens(lens))?;
    //     Ok(())
    // }
}

impl winit::application::ApplicationHandler<UserEvent> for App {
    fn about_to_wait(&mut self, _event_loop: &event_loop::ActiveEventLoop) {
        if let Some(state) = &mut self.state {
            state.about_to_wait();
        }
    }

    fn resumed(&mut self, event_loop: &event_loop::ActiveEventLoop) {
        self.request_window(event_loop, None)
            .expect("Could not request window.");
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if let Some(state) = &mut self.state {
            if id == state.window().id() {
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
                    } => event_loop.exit(),
                    WindowEvent::Resized(physical_size) => {
                        state.resize(physical_size);
                    }
                    WindowEvent::RedrawRequested => match state.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            state.resize(state.size)
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                        Err(wgpu::SurfaceError::Timeout) => {
                            // Ignore timeouts.
                        }
                    },
                    other => {
                        // dispatches event to egui
                        state.handle_event(&other);
                        state.window().request_redraw();
                        return;
                    }
                };
                state.handle_event(&event);
                state.window().request_redraw();
            }
        }
    }

    fn user_event(&mut self, event_loop: &event_loop::ActiveEventLoop, event: UserEvent) {
        // tracing::info!("Event detected: {:?}", event);
        match event {
            UserEvent::Map(window_event) => {
                tracing::info!("Map event detected.");
                // egui event response was not consumed
                // event is passed to galileo event processor
                self.delegate(&window_event);
            }
            UserEvent::State(state) => {
                tracing::info!("State arrived!");
                // add pointer position to galileo event processor
                let pp = state.galileo_state.pointer_position();
                self.delegate.add_handler(
                    move |ev: &galileo::control::UserEvent, _map: &mut galileo::Map| {
                        if let galileo::control::UserEvent::PointerMoved(
                            galileo::control::MouseEvent {
                                screen_pointer_position,
                                ..
                            },
                        ) = ev
                        {
                            *pp.write().expect("poisoned lock") = *screen_pointer_position;
                        }

                        galileo::control::EventPropagation::Propagate
                    },
                );
                tracing::info!("Pointer positied handler delegated.");
                self.state = Some(state);
                // no effect
                // problem description: map is initialized but not rendering
                if let Some(state) = &mut self.state {
                    tracing::info!("State found.");
                    state.render().expect("could not render map");
                    tracing::info!("State rendered.");
                    state.window().request_redraw();
                    tracing::info!("Redraw requested.");
                }
            }
        }
    }
}

impl winit::application::ApplicationHandler for State {
    fn about_to_wait(&mut self, _event_loop: &event_loop::ActiveEventLoop) {
        self.about_to_wait();
    }

    fn resumed(&mut self, _event_loop: &event_loop::ActiveEventLoop) {
        // self.request_window(event_loop, None)
        //     .expect("Could not request window.");
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if id == self.window().id() {
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
                } => event_loop.exit(),
                WindowEvent::Resized(physical_size) => {
                    self.resize(physical_size);
                }
                WindowEvent::RedrawRequested => match self.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        self.resize(self.size)
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                    Err(wgpu::SurfaceError::Timeout) => {
                        // Ignore timeouts.
                    }
                },
                other => {
                    self.handle_event(&other);
                    self.window().request_redraw();
                    return;
                }
            };
            self.handle_event(&event);
            self.window().request_redraw();
        }
    }
}
