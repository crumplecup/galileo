#![allow(dead_code)]
use galileo::GalileoResult;
use std::sync::Arc;

mod run_ui;
mod state;

use with_egui::run;

#[tokio::main]
async fn main() -> GalileoResult<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let attr = winit::window::Window::default_attributes()
        .with_title("Galileo Feature Layers")
        .with_transparent(true)
        .with_inner_size(winit::dpi::PhysicalSize {
            height: 1024,
            width: 1024,
        });
    let window = event_loop.create_window(attr)?;
    let window = Arc::new(window);

    run(window, event_loop).await;
    Ok(())
}
