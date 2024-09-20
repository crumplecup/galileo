use galileo::tile_scheme::TileSchema;
use galileo::{GalileoResult, MapBuilder};
use galileo_types::latlon;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> GalileoResult<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    run(MapBuilder::new()).await?;
    Ok(())
}

pub async fn run(builder: MapBuilder) -> GalileoResult<()> {
    let attr = winit::window::Window::default_attributes()
        .with_title("Galileo Raster Tiles")
        .with_transparent(true)
        .with_inner_size(winit::dpi::PhysicalSize {
            height: 1024,
            width: 1024,
        });
    let event_loop = winit::event_loop::EventLoop::new()?;
    let window = event_loop.create_window(attr)?;
    let window = Arc::new(window);

    builder
        .center(latlon!(37.566, 126.9784))
        .resolution(TileSchema::web(18).lod_resolution(8).unwrap())
        .with_raster_tiles(
            |index| {
                format!(
                    "https://tile.openstreetmap.org/{}/{}/{}.png",
                    index.z, index.x, index.y
                )
            },
            TileSchema::web(18),
        )
        .build(window)
        .await
        .run(event_loop)?;
    Ok(())
}
