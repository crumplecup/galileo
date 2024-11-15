//! Map builder functions specific to Web target.

use crate::galileo_map::{GalileoMap, MapBuilder};
use crate::layer::data_provider::dummy::DummyCacheController;
use crate::layer::data_provider::UrlImageProvider;
use crate::layer::data_provider::UrlSource;
use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::vector_tile_layer::tile_provider::loader::WebVtLoader;
use crate::layer::vector_tile_layer::tile_provider::VectorTileProvider;
use crate::layer::{RasterTileLayer, VectorTileLayer};
use crate::platform::web::vt_processor::WebWorkerVtProcessor;
use crate::platform::web::web_workers::WebWorkerService;
use crate::platform::{PlatformService, PlatformServiceImpl};
use crate::tile_scheme::TileIndex;
use crate::TileSchema;
use galileo_types::geo::impls::GeoPoint2d;
use std::sync::Arc;
use wasm_bindgen::prelude::wasm_bindgen;
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

impl MapBuilder {
    /// Creates a raster tile layer.
    pub fn create_raster_tile_layer(
        tile_source: impl UrlSource<TileIndex> + 'static,
        tile_scheme: TileSchema,
    ) -> RasterTileLayer<UrlImageProvider<TileIndex>> {
        let tile_provider = UrlImageProvider::new(tile_source);
        RasterTileLayer::new(tile_scheme, tile_provider, None)
    }

    /// Create a new vector tile layer.
    pub async fn create_vector_tile_layer(
        tile_source: impl UrlSource<TileIndex> + 'static,
        tile_schema: TileSchema,
        style: VectorTileStyle,
    ) -> VectorTileLayer<WebVtLoader<DummyCacheController>, WebWorkerVtProcessor> {
        let tile_provider = Self::create_vector_tile_provider(tile_source, tile_schema.clone());
        VectorTileLayer::from_url(tile_provider, style, tile_schema).await
    }

    /// Create a new vector tile provider.
    pub fn create_vector_tile_provider(
        tile_source: impl UrlSource<TileIndex> + 'static,
        tile_schema: TileSchema,
    ) -> VectorTileProvider<WebVtLoader<DummyCacheController>, WebWorkerVtProcessor> {
        let loader = WebVtLoader::new(
            PlatformServiceImpl::new(),
            DummyCacheController {},
            tile_source,
        );
        let ww_service = WebWorkerService::new(4);
        let processor = WebWorkerVtProcessor::new(tile_schema, ww_service);

        #[allow(clippy::arc_with_non_send_sync)]
        VectorTileProvider::new(Arc::new(loader), Arc::new(processor))
    }
}

#[wasm_bindgen]
impl MapBuilder {
    /// Creates a new map builder and intializes console logger.
    pub fn new() -> Self {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(log::Level::Info).expect("Couldn't init logger");

        log::debug!("Logger is initialized");
        Self::default()
    }

    /// Builds the map and adds it to the given parent HTML element.
    pub async fn build_into(mut self, container: web_sys::Element) -> GalileoMap {
        use winit::platform::web::WindowExtWebSys;

        let event_loop = self
            .event_loop
            .take()
            .unwrap_or_else(|| EventLoop::new().unwrap());
        let window = self.window.take().unwrap_or_else(|| {
            WindowBuilder::new()
                .with_inner_size(PhysicalSize {
                    width: 1024,
                    height: 1024,
                })
                .build(&event_loop)
                .unwrap()
        });

        let canvas = web_sys::Element::from(window.canvas().unwrap());
        container.append_child(&canvas).unwrap();

        let width = container.client_width() as u32;
        let height = container.client_height() as u32;
        log::info!("Requesting canvas size: {width} - {height}");

        let _ = window.request_inner_size(PhysicalSize { width, height });

        sleep(1).await;

        self.window = Some(window);
        self.event_loop = Some(event_loop);

        self.build().await
    }

    /// Adds a new raster tile layer to the layer list.
    pub fn with_raster_tiles(mut self, tile_source: js_sys::Function) -> Self {
        let tile_source_int = move |index: &TileIndex| {
            log::info!("{index:?}");
            let this = wasm_bindgen::JsValue::null();
            tile_source
                .call1(&this, &(*index).into())
                .unwrap()
                .as_string()
                .unwrap()
        };

        let tile_provider = UrlImageProvider::new(tile_source_int);
        self.layers.push(Box::new(RasterTileLayer::new(
            TileSchema::web(18),
            tile_provider,
            None,
        )));

        self
    }
}

async fn sleep(duration: i32) {
    let mut cb = |resolve: js_sys::Function, _reject: js_sys::Function| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, duration)
            .unwrap();
    };

    let p = js_sys::Promise::new(&mut cb);

    wasm_bindgen_futures::JsFuture::from(p).await.unwrap();
}
