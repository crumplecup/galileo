use crate::control::{EventProcessor, EventPropagation, MapController, UserEvent};
use crate::layer::data_provider::UrlSource;
use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::Layer;
use crate::map::Map;
use crate::render::WgpuRenderer;
use crate::tile_scheme::{TileIndex, TileSchema};
use crate::view::MapView;
use crate::winit::{WinitInputHandler, WinitMessenger};
use crate::GalileoResult;
use galileo_types::cartesian::Size;
use galileo_types::geo::impls::GeoPoint2d;
use maybe_sync::{MaybeSend, MaybeSync};
use std::sync::{Arc, RwLock};
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::Window;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

/// Convenience struct holding all necessary parts of a interactive map, including window handle and an event loop.
///
/// Usually an application using `Galileo` will have control over the window, event loop and rendering backend. This
/// structure can be used for developing map-related functionality separately from an application, or as a reference
/// of how to set up the event loop for Galileo map.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct GalileoMap {
    window: Arc<Window>,
    map: Arc<RwLock<Map>>,
    backend: Arc<RwLock<Option<WgpuRenderer>>>,
    event_processor: EventProcessor,
    input_handler: WinitInputHandler,
    // event_loop: EventLoop<()>,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl GalileoMap {
    /// Runs the main event loop.
    pub fn run(&mut self, event_loop: EventLoop<()>) -> GalileoResult<()> {
        event_loop.run_app(self)?;
        Ok(())
    }
}

impl winit::application::ApplicationHandler for GalileoMap {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        log::info!("Resume called");
        let backend = self.backend.clone();
        let window = self.window.clone();
        let map = self.map.clone();
        crate::async_runtime::spawn(async move {
            let size = window.inner_size();

            let mut renderer =
                WgpuRenderer::new_with_window(window.clone(), Size::new(size.width, size.height))
                    .await
                    .expect("failed to init renderer");

            let new_size = window.inner_size();
            if new_size != size {
                renderer.resize(Size::new(new_size.width, new_size.height));
            }

            *backend.write().expect("poisoned lock") = Some(renderer);
            map.write()
                .expect("poisoned lock")
                .set_size(Size::new(size.width as f64, size.height as f64));
            window.request_redraw();
        });
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if window_id == self.window.id() {
            match event {
                WindowEvent::CloseRequested => {
                    event_loop.exit();
                }
                WindowEvent::Resized(size) => {
                    log::info!("Window resized to: {size:?}");
                    if let Some(backend) = self.backend.write().expect("lock is poisoned").as_mut()
                    {
                        backend.resize(Size::new(size.width, size.height));

                        let mut map = self.map.write().expect("lock is poisoned");
                        map.set_size(Size::new(size.width as f64, size.height as f64));
                    }
                }
                WindowEvent::RedrawRequested => {
                    if let Some(backend) = self.backend.read().expect("lock is poisoned").as_ref() {
                        let map = self.map.read().expect("lock is poisoned");
                        map.load_layers();
                        if let Err(err) = backend.render(&map) {
                            log::error!("Render error: {err:?}");
                        }
                    }
                }
                other => {
                    // Phone emulator in browsers works funny with scaling, using this code fixes it.
                    // But my real phone works fine without it, so it's commented out for now, and probably
                    // should be deleted later, when we know that it's not needed on any devices.

                    // #[cfg(target_arch = "wasm32")]
                    // let scale = window.scale_factor();
                    //
                    // #[cfg(not(target_arch = "wasm32"))]
                    let scale = 1.0;

                    if let Some(raw_event) = self.input_handler.process_user_input(&other, scale) {
                        let mut map = self.map.write().expect("lock is poisoned");
                        self.event_processor.handle(raw_event, &mut map);
                    }
                }
            }
        }
    }

    fn suspended(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        *self.backend.write().expect("Poisoned lock") = None;
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.map.write().expect("lock is poisoned").animate();
    }
}

#[cfg(target_arch = "wasm32")]
type EventHandler = dyn (Fn(&UserEvent, &mut Map) -> EventPropagation);
#[cfg(not(target_arch = "wasm32"))]
type EventHandler = dyn (Fn(&UserEvent, &mut Map) -> EventPropagation) + MaybeSend + MaybeSync;

/// Builder for a [`GalileoMap`].
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct MapBuilder {
    pub(crate) position: GeoPoint2d,
    pub(crate) resolution: f64,
    pub(crate) view: Option<MapView>,
    pub(crate) layers: Vec<Box<dyn Layer>>,
    pub(crate) event_handlers: Vec<Box<EventHandler>>,
}

impl MapBuilder {
    pub async fn build(mut self, window: Arc<winit::window::Window>) -> GalileoMap {
        let backend = Arc::new(RwLock::new(None));

        let input_handler = WinitInputHandler::default();

        let mut event_processor = EventProcessor::default();
        for handler in self.event_handlers.drain(..) {
            event_processor.add_handler(handler);
        }
        event_processor.add_handler(MapController::default());
        let map = self.build_map(window.clone());

        GalileoMap {
            window,
            map,
            backend,
            event_processor,
            input_handler,
        }
    }

    fn build_map(mut self, window: Arc<winit::window::Window>) -> Arc<RwLock<Map>> {
        let messenger = WinitMessenger::new(window);
        for layer in self.layers.iter_mut() {
            layer.set_messenger(Box::new(messenger.clone()))
        }

        let view = self
            .view
            .unwrap_or_else(|| MapView::new(&self.position, self.resolution));

        let map = Map::new(view, self.layers, Some(messenger));

        Arc::new(RwLock::new(map))
    }

    /// Add a give layer to the map.
    pub fn with_layer(mut self, layer: impl Layer + 'static) -> Self {
        self.layers.push(Box::new(layer));
        self
    }

    /// Add an event handler.
    pub fn with_event_handler(
        mut self,
        handler: impl (Fn(&UserEvent, &mut Map) -> EventPropagation) + MaybeSend + MaybeSync + 'static,
    ) -> Self {
        self.event_handlers.push(Box::new(handler));
        self
    }

    /// Set the center of the map.
    pub fn center(mut self, position: GeoPoint2d) -> Self {
        self.position = position;
        self
    }

    /// Set the resolution of the map. For explanation about resolution, see [`MapView::resolution`].
    pub fn resolution(mut self, resolution: f64) -> Self {
        self.resolution = resolution;
        self
    }

    /// Set the view of the map.
    pub fn with_view(mut self, view: MapView) -> Self {
        self.view = Some(view);
        self
    }

    /// Add a vector tile layer with the given parameters.
    pub async fn with_vector_tiles(
        mut self,
        tile_source: impl UrlSource<TileIndex> + 'static,
        tile_scheme: TileSchema,
        style: VectorTileStyle,
    ) -> Self {
        self.layers.push(Box::new(
            Self::create_vector_tile_layer(tile_source, tile_scheme, style).await,
        ));
        self
    }
}

impl Default for MapBuilder {
    fn default() -> Self {
        Self {
            position: GeoPoint2d::default(),
            resolution: 156543.03392800014 / 16.0,
            view: None,
            layers: vec![],
            event_handlers: vec![],
        }
    }
}
