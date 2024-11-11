#![allow(dead_code)]
use clap::Parser;
use galileo::GalileoResult;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
// use std::sync::Arc;

mod run_ui;
mod state;

use with_egui::{run, App, UserEvent};

#[tokio::main]
async fn main() -> GalileoResult<()> {
    trace_init();
    let cli = Cli::parse();
    // env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    match cli.command.as_str() {
        "run_app" => {
            let event_loop =
                winit::event_loop::EventLoop::<UserEvent>::with_user_event().build()?;
            tracing::info!("Event loop created.");
            let proxy = event_loop.create_proxy();
            tracing::info!("Proxy created.");
            let mut app = App::new(proxy);
            tracing::info!("App created.");
            event_loop.run_app(&mut app)?;
        }
        "run" => {
            let attr = winit::window::Window::default_attributes()
                .with_title("Galileo Feature Layers")
                .with_transparent(true)
                .with_inner_size(winit::dpi::PhysicalSize {
                    height: 1024,
                    width: 1024,
                });
            let event_loop =
                winit::event_loop::EventLoop::<UserEvent>::with_user_event().build()?;
            let proxy = event_loop.create_proxy();
            let window = event_loop.create_window(attr)?;
            let window = std::sync::Arc::new(window);

            run(window, event_loop, proxy).await;
        }
        _ => tracing::warn!("Unrecognized command."),
    }

    Ok(())
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(short = 'c', long, help = "Command to execute.")]
    pub command: String,
}

/// The `trace_init` function initializing logging using the [`tracing`] and [`tracing_subscriber`]
/// crates.
/// Pass the desired log level into the environment when running the app from cargo.
/// E.g. `$RUST_LOG="trace" cargo run` for debugging.
pub fn trace_init() {
    if tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .try_init()
        .is_ok()
    {};
    tracing::info!("Loading Galileo + Egui...");
}
