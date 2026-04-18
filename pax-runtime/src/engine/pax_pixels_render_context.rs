use kurbo::{BezPath, PathEl, Shape};
use pax_pixels::{
    point, Box2D, Image, Path, Stroke as PixelStroke, StrokeCap as PixelStrokeCap, Transform2D,
    WgpuRenderer,
};
use pax_runtime_api::{Axis, RenderContext, ScreenshotData, Stroke, StrokeCap};
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet, VecDeque},
    future::Future,
    pin::Pin,
    rc::Rc,
};

#[cfg(not(target_arch = "wasm32"))]
use pollster;

pub struct LayerRenderer {
    key: String,
    host_signature: String,
    renderer: WgpuRenderer<'static>,
    origin_x: f32,
    origin_y: f32,
    logical_width: f32,
    logical_height: f32,
    surface_width: u32,
    surface_height: u32,
    dpr: [f32; 2],
}

pub struct LayerTarget {
    renderers: Vec<LayerRenderer>,
    active: bool,
    needs_replay: bool,
}

pub struct LayerSurfaceEntry {
    pub key: String,
    pub host_signature: String,
    pub origin_x: f32,
    pub origin_y: f32,
    pub surface: LayerSurfaceSize,
}

pub struct LayerSurfaceLayout {
    pub surfaces: Vec<LayerSurfaceEntry>,
    pub active: bool,
}

type LayerDef = (LayerTarget, Pin<Box<dyn Fn() -> LayerSurfaceLayout>>);

#[derive(Clone, Copy, Debug, Default)]
enum LayoutChangeKind {
    #[default]
    Unchanged,
    OriginOnly,
    Resized,
}

impl LayerRenderer {
    pub fn new(
        key: String,
        host_signature: String,
        renderer: WgpuRenderer<'static>,
        origin_x: f32,
        origin_y: f32,
        logical_width: f32,
        logical_height: f32,
        surface_width: u32,
        surface_height: u32,
        dpr: [f32; 2],
    ) -> Self {
        Self {
            key,
            host_signature,
            renderer,
            origin_x,
            origin_y,
            logical_width,
            logical_height,
            surface_width,
            surface_height,
            dpr,
        }
    }

    pub fn renderer_mut(&mut self) -> &mut WgpuRenderer<'static> {
        &mut self.renderer
    }

    fn update_layout(&mut self, surface: &LayerSurfaceEntry) -> LayoutChangeKind {
        let origin_changed = (self.origin_x - surface.origin_x).abs() > f32::EPSILON
            || (self.origin_y - surface.origin_y).abs() > f32::EPSILON;
        let size_changed = self.logical_width != surface.surface.logical_width
            || self.logical_height != surface.surface.logical_height
            || self.surface_width != surface.surface.surface_width
            || self.surface_height != surface.surface.surface_height
            || self.dpr != surface.surface.dpr;

        self.origin_x = surface.origin_x;
        self.origin_y = surface.origin_y;
        self.logical_width = surface.surface.logical_width;
        self.logical_height = surface.surface.logical_height;
        self.surface_width = surface.surface.surface_width;
        self.surface_height = surface.surface.surface_height;
        self.dpr = surface.surface.dpr;

        if size_changed {
            LayoutChangeKind::Resized
        } else if origin_changed {
            LayoutChangeKind::OriginOnly
        } else {
            LayoutChangeKind::Unchanged
        }
    }
}

impl LayerTarget {
    pub fn new(renderers: Vec<LayerRenderer>, active: bool) -> Self {
        // The first tiled pass keeps node drawing opaque by replaying the same retained scene into
        // each active physical surface. That duplicates retained scene state per tile, but it keeps
        // tiling below the RenderContext seam so primitives stay unaware of browser-surface
        // partitioning.
        Self {
            renderers,
            active,
            needs_replay: false,
        }
    }

    pub fn renderers_mut(&mut self) -> &mut [LayerRenderer] {
        &mut self.renderers
    }

    fn activate(&mut self) {
        if !self.active {
            self.active = true;
            self.needs_replay = true;
        }
    }

    fn deactivate(&mut self) {
        self.active = false;
    }

    fn prepare_for_render(&mut self) {
        if !self.needs_replay {
            return;
        }
        for renderer in &mut self.renderers {
            renderer.renderer.reset_retained_scene();
        }
        self.needs_replay = false;
    }
}

fn layer_layout_matches_target(target: &LayerTarget, layout: &LayerSurfaceLayout) -> bool {
    // `host_signature` identifies the DOM surface host that currently owns a layer tile. It is
    // intentionally separate from node identity: the same expanded node can be rebound to a new
    // browser surface host when scroller islands are recreated.
    layout.surfaces.len() == target.renderers.len()
        && layout
            .surfaces
            .iter()
            .zip(target.renderers.iter())
            .all(|(surface, renderer)| {
                surface.key == renderer.key && surface.host_signature == renderer.host_signature
            })
}

const MAX_CONCURRENT_LAYER_INITIALIZATIONS: usize = 4;

fn pump_layer_initialization_queue(
    factory: Rc<dyn Fn(usize) -> Pin<Box<dyn Future<Output = Option<LayerDef>>>>>,
    backends: Rc<RefCell<Vec<RenderLayerState>>>,
    queue: Rc<RefCell<VecDeque<usize>>>,
    scheduled: Rc<RefCell<HashSet<usize>>>,
    in_flight: Rc<Cell<usize>>,
    ready_layers: Rc<RefCell<Vec<usize>>>,
) {
    while in_flight.get() < MAX_CONCURRENT_LAYER_INITIALIZATIONS {
        let next_layer = queue.borrow_mut().pop_front();
        let Some(layer_index) = next_layer else {
            break;
        };

        in_flight.set(in_flight.get() + 1);
        let factory = Rc::clone(&factory);
        let backends = Rc::clone(&backends);
        let queue = Rc::clone(&queue);
        let scheduled = Rc::clone(&scheduled);
        let in_flight_count = Rc::clone(&in_flight);
        let ready_layers = Rc::clone(&ready_layers);

        let task = async move {
            let backend = (factory)(layer_index).await;
            let mut should_requeue = false;
            match backend {
                Some(layer_def) => {
                    let current_layout = layer_def.1();
                    let layout_matches =
                        layer_layout_matches_target(&layer_def.0, &current_layout);
                    let mut backend_states = backends.borrow_mut();
                    match backend_states.get_mut(layer_index) {
                        Some(change) if layout_matches => {
                            *change = RenderLayerState::Ready(layer_def);
                            ready_layers.borrow_mut().push(layer_index);
                        }
                        Some(change) => {
                            // Layer initialization is async relative to host ownership on the
                            // web. If the DOM host changes while this backend is bootstrapping,
                            // discard it and retry against the latest layout instead of
                            // publishing a stale renderer.
                            *change = RenderLayerState::Pending;
                            should_requeue = true;
                        }
                        None => {
                            log::warn!(
                                "failed to set poll state to ready: layer {} doesn't exist anymore",
                                layer_index
                            );
                        }
                    }
                }
                None => match backends.borrow_mut().get_mut(layer_index) {
                    Some(change) => {
                        *change = RenderLayerState::Failed;
                        log::warn!(
                            "failed to initialize render backend for layer {}",
                            layer_index
                        );
                    }
                    None => log::warn!(
                        "failed to set poll state to ready: layer {} doesn't exist and backend failed to initialize",
                        layer_index
                    ),
                },
            }

            scheduled.borrow_mut().remove(&layer_index);
            if should_requeue && scheduled.borrow_mut().insert(layer_index) {
                queue.borrow_mut().push_back(layer_index);
            }

            in_flight_count.set(in_flight_count.get().saturating_sub(1));
            pump_layer_initialization_queue(
                factory,
                backends,
                queue,
                scheduled,
                in_flight_count,
                ready_layers,
            );
        };

        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(task);
        #[cfg(not(target_arch = "wasm32"))]
        {
            pollster::block_on(task);
        }
    }
}
pub struct LayerSurfaceSize {
    pub logical_width: f32,
    pub logical_height: f32,
    pub surface_width: u32,
    pub surface_height: u32,
    pub dpr: [f32; 2],
}

pub struct PaxPixelsRenderer {
    backends: Rc<RefCell<Vec<RenderLayerState>>>,
    layer_factory: Rc<dyn Fn(usize) -> Pin<Box<dyn Future<Output = Option<LayerDef>>>>>,
    image_map: HashMap<String, Image>,
    image_versions: HashMap<String, u64>,
    failed_context_gets: RefCell<Vec<bool>>,
    ready_layers: Rc<RefCell<Vec<usize>>>,
    replay_layers: Rc<RefCell<Vec<usize>>>,
    pending_layer_initializations: Rc<RefCell<VecDeque<usize>>>,
    scheduled_layer_initializations: Rc<RefCell<HashSet<usize>>>,
    layer_initializations_in_flight: Rc<Cell<usize>>,
}

pub enum RenderLayerState {
    Pending,
    Failed,
    Ready(LayerDef),
}

impl PaxPixelsRenderer {
    pub fn new(
        layer_factory: impl Fn(usize) -> Pin<Box<dyn Future<Output = Option<LayerDef>>>> + 'static,
    ) -> Self {
        Self {
            backends: Default::default(),
            layer_factory: Rc::new(layer_factory),
            image_map: Default::default(),
            image_versions: Default::default(),
            failed_context_gets: RefCell::new(vec![]),
            ready_layers: Default::default(),
            replay_layers: Default::default(),
            pending_layer_initializations: Rc::new(RefCell::new(VecDeque::new())),
            scheduled_layer_initializations: Rc::new(RefCell::new(HashSet::new())),
            layer_initializations_in_flight: Rc::new(Cell::new(0)),
        }
    }
}

impl PaxPixelsRenderer {
    fn queue_layer_initialization(&self, layer_index: usize) {
        if self
            .scheduled_layer_initializations
            .borrow_mut()
            .insert(layer_index)
        {
            self.pending_layer_initializations
                .borrow_mut()
                .push_back(layer_index);
        }
    }

    fn enqueue_layer_initialization(&self, layer_index: usize) {
        self.queue_layer_initialization(layer_index);
        self.schedule_layer_initialization();
    }

    fn with_layer_context(&self, layer: usize, mut f: impl FnMut(&mut WgpuRenderer)) {
        let mut backends = self.backends.borrow_mut();
        match backends.get_mut(layer) {
            Some(layer_state) => match layer_state {
                RenderLayerState::Pending => {
                    let mut failed_context_gets = self.failed_context_gets.borrow_mut();
                    if failed_context_gets.len() <= layer {
                        failed_context_gets.resize(layer + 1, false);
                    }
                    failed_context_gets[layer] = true;
                }
                RenderLayerState::Failed => {}
                RenderLayerState::Ready((target, _)) => {
                    if !target.active {
                        return;
                    }
                    target.prepare_for_render();
                    for renderer in &mut target.renderers {
                        f(&mut renderer.renderer);
                    }
                }
            },
            None => log::warn!(
                "tried to retrieve layer {} context for non-existent layer",
                layer
            ),
        }
    }

    fn schedule_layer_initialization(&self) {
        pump_layer_initialization_queue(
            Rc::clone(&self.layer_factory),
            Rc::clone(&self.backends),
            Rc::clone(&self.pending_layer_initializations),
            Rc::clone(&self.scheduled_layer_initializations),
            Rc::clone(&self.layer_initializations_in_flight),
            Rc::clone(&self.ready_layers),
        );
    }

    fn refresh_layer_layouts<I>(&mut self, layer_indices: I)
    where
        I: IntoIterator<Item = usize>,
    {
        let mut needs_reinitialization = false;
        let mut backends = self.backends.borrow_mut();
        for layer_index in layer_indices {
            let Some(backend) = backends.get_mut(layer_index) else {
                continue;
            };
            match backend {
                RenderLayerState::Pending => {}
                RenderLayerState::Failed => {}
                RenderLayerState::Ready((target, layout_provider)) => {
                    let layout = (layout_provider)();
                    let layout_matches = layer_layout_matches_target(target, &layout);

                    if layout.active {
                        target.activate();
                    } else {
                        target.deactivate();
                    }

                    if !layout_matches {
                        // A different keyed surface set means the DOM host really changed shape
                        // underneath us. Stable slot keys let ordinary scroll slide tile origins in
                        // place; reserve full reinitialization for real additions/removals.
                        *backend = RenderLayerState::Pending;
                        self.queue_layer_initialization(layer_index);
                        needs_reinitialization = true;
                        continue;
                    }

                    for (surface, renderer) in
                        layout.surfaces.iter().zip(target.renderers.iter_mut())
                    {
                        let layout_change = renderer.update_layout(surface);
                        match layout_change {
                            LayoutChangeKind::Unchanged => {}
                            LayoutChangeKind::OriginOnly => {
                                // Reassigning a stable viewport slot to a new absolute tile origin
                                // clears the retained scene for that physical surface. The chassis
                                // must replay the logical layer contents into it on the next tick.
                                renderer.renderer.reset_retained_scene();
                                renderer
                                    .renderer
                                    .set_surface_transform(Transform2D::from_array([
                                        1.0,
                                        0.0,
                                        0.0,
                                        1.0,
                                        -surface.origin_x,
                                        -surface.origin_y,
                                    ]));
                                self.replay_layers.borrow_mut().push(layer_index);
                            }
                            LayoutChangeKind::Resized => {
                                renderer.renderer.reset_retained_scene();
                                renderer
                                    .renderer
                                    .set_surface_transform(Transform2D::from_array([
                                        1.0,
                                        0.0,
                                        0.0,
                                        1.0,
                                        -surface.origin_x,
                                        -surface.origin_y,
                                    ]));
                                renderer.renderer.resize_surface(
                                    surface.surface.surface_width as f32,
                                    surface.surface.surface_height as f32,
                                );
                                renderer.renderer.set_viewport(
                                    surface.surface.logical_width,
                                    surface.surface.logical_height,
                                    surface.surface.dpr,
                                );
                                self.replay_layers.borrow_mut().push(layer_index);
                            }
                        }
                    }
                }
            }
        }
        drop(backends);
        if needs_reinitialization {
            self.schedule_layer_initialization();
        }
    }
}

impl RenderContext for PaxPixelsRenderer {
    fn fill_with_opacity(
        &mut self,
        layer: usize,
        path: kurbo::BezPath,
        fill: &pax_runtime_api::Fill,
        opacity: f64,
    ) {
        self.with_layer_context(layer, |context| {
            let bounds = path.bounding_box();
            let path = convert_kurbo_to_lyon_path(&path);
            let fill = to_pax_pixels_fill(fill, bounds, context.current_transform());
            context.fill_path_with_opacity(path, fill, opacity as f32);
        });
    }

    fn stroke_with_opacity(
        &mut self,
        layer: usize,
        path: kurbo::BezPath,
        stroke: &Stroke,
        opacity: f64,
    ) {
        self.with_layer_context(layer, |context| {
            let bounds = path.bounding_box();
            context.stroke_path_with_opacity(
                convert_kurbo_to_lyon_path(&path),
                PixelStroke {
                    fill: to_pax_pixels_fill(
                        &pax_runtime_api::Fill::Solid(stroke.color.get()),
                        bounds,
                        context.current_transform(),
                    ),
                    weight: stroke.width.get().expect_pixels().to_float() as f32,
                    cap: match stroke.cap.get() {
                        StrokeCap::Butt => PixelStrokeCap::Butt,
                        StrokeCap::Round => PixelStrokeCap::Round,
                        StrokeCap::Square => PixelStrokeCap::Square,
                    },
                },
                opacity as f32,
            );
        });
    }

    fn save(&mut self, layer: usize) {
        self.with_layer_context(layer, |context| {
            context.save();
        });
    }
    fn restore(&mut self, layer: usize) {
        self.with_layer_context(layer, |context| {
            context.restore();
        });
    }
    fn clip(&mut self, layer: usize, path: kurbo::BezPath) {
        self.with_layer_context(layer, |context| {
            let path = convert_kurbo_to_lyon_path(&path);
            context.clip(path);
        });
    }
    fn transform(&mut self, layer: usize, affine: kurbo::Affine) {
        self.with_layer_context(layer, |context| {
            context.transform(Transform2D::from_array(
                affine.as_coeffs().map(|v| v as f32),
            ))
        });
    }

    fn load_image(&mut self, identifier: &str, image: &[u8], width: usize, height: usize) {
        self.image_map.insert(
            identifier.to_string(),
            Image {
                rgba: image.into(),
                pixel_width: width as u32,
                pixel_height: height as u32,
            },
        );
        *self
            .image_versions
            .entry(identifier.to_string())
            .or_insert(0) += 1;
    }

    fn draw_image(&mut self, layer: usize, image_path: &str, rect: kurbo::Rect) {
        self.with_layer_context(layer, |context| {
            if let Some(image) = self.image_map.get(image_path) {
                let version = *self.image_versions.get(image_path).unwrap_or(&0);
                context.draw_image(
                    image_path,
                    version,
                    image,
                    Box2D {
                        min: point(rect.x0 as f32, rect.y0 as f32),
                        max: point(rect.x1 as f32, rect.y1 as f32),
                    },
                );
            }
        });
    }

    fn get_image_size(&mut self, image_path: &str) -> Option<(usize, usize)> {
        self.image_map
            .get(image_path)
            .map(|img| (img.pixel_width as usize, img.pixel_height as usize))
    }

    fn image_loaded(&self, image_path: &str) -> bool {
        self.image_map.contains_key(image_path)
    }

    fn layers(&self) -> usize {
        self.backends.borrow().len()
    }

    fn resize_layers_to(&mut self, layer_count: usize, _dirty_canvases: Rc<RefCell<Vec<bool>>>) {
        let current_len = self.backends.borrow().len();
        match layer_count.cmp(&current_len) {
            std::cmp::Ordering::Less => {
                self.backends.borrow_mut().truncate(layer_count);
            }
            std::cmp::Ordering::Equal => return,
            std::cmp::Ordering::Greater => {
                for i in current_len..layer_count {
                    self.backends.borrow_mut().push(RenderLayerState::Pending);
                    self.enqueue_layer_initialization(i);
                }
            }
        }
    }

    fn clear(&mut self, layer: usize) {
        self.with_layer_context(layer, |context| {
            context.clear();
        });
    }

    fn flush(&mut self, layer: usize, dirty_canvases: Rc<RefCell<Vec<bool>>>) {
        let mut backends = self.backends.borrow_mut();
        match backends.get_mut(layer) {
            Some(RenderLayerState::Pending) => {}
            Some(RenderLayerState::Failed) => {}
            Some(RenderLayerState::Ready((target, _))) => {
                if !target.active {
                    return;
                }
                if let Some(failed) = self.failed_context_gets.borrow_mut().get_mut(layer) {
                    if *failed {
                        if let Some(dirty_bit) = dirty_canvases.borrow_mut().get_mut(layer) {
                            // If we failed to draw to this layer because the context wasn't
                            // available yet, retry once the backend is ready.
                            *dirty_bit = true;
                        }
                        *failed = false;
                    }
                }
                for renderer in &mut target.renderers {
                    renderer.renderer.flush();
                }
            }
            None => log::warn!(
                "tried to flush layer {} context for non-existent layer",
                layer
            ),
        }
    }

    fn resize(&mut self, _width: usize, _height: usize) {
        let layer_count = self.backends.borrow().len();
        self.refresh_layer_layouts(0..layer_count);
    }

    fn refresh_layers(&mut self, layers: &[usize]) {
        self.refresh_layer_layouts(layers.iter().copied());
    }

    fn take_ready_canvas_layers(&mut self) -> Vec<usize> {
        let mut ready_layers = self.ready_layers.borrow_mut();
        let mut ready = std::mem::take(&mut *ready_layers);
        ready.sort_unstable();
        ready.dedup();
        ready
    }

    fn take_replay_canvas_layers(&mut self) -> Vec<usize> {
        let mut replay_layers = self.replay_layers.borrow_mut();
        let mut replay = std::mem::take(&mut *replay_layers);
        replay.sort_unstable();
        replay.dedup();
        replay
    }

    fn request_layer_screenshot(&mut self, layer: usize, request_id: u32) {
        self.with_layer_context(layer, |context| {
            context.request_screenshot_capture(request_id);
        });
    }

    fn take_layer_screenshot(&mut self, layer: usize, request_id: u32) -> Option<ScreenshotData> {
        let mut screenshot = None;
        self.with_layer_context(layer, |context| {
            screenshot =
                context
                    .take_screenshot_capture(request_id)
                    .map(|capture| ScreenshotData {
                        id: request_id,
                        data: capture.rgba,
                        width: capture.width as usize,
                        height: capture.height as usize,
                    });
        });
        screenshot
    }

    fn begin_node(&mut self, layer: usize, node_id: u32, z_index: i32) -> bool {
        let mut began = false;
        self.with_layer_context(layer, |context| {
            began = context.begin_node(node_id, z_index);
        });
        began
    }

    fn end_node(&mut self, layer: usize, node_id: u32) -> bool {
        let mut ended = false;
        self.with_layer_context(layer, |context| {
            ended = context.end_node(node_id);
        });
        ended
    }

    fn remove_node(&mut self, layer: usize, node_id: u32) -> bool {
        let mut removed = false;
        self.with_layer_context(layer, |context| {
            removed = context.remove_node(node_id);
        });
        removed
    }
}

fn to_pax_pixels_fill(
    fill: &pax_runtime_api::Fill,
    rect: kurbo::Rect,
    transform: pax_pixels::Transform2D,
) -> pax_pixels::Fill {
    let bounds = (rect.width(), rect.height());
    let orig = rect.origin();
    match fill {
        pax_runtime_api::Fill::Solid(color) => pax_pixels::Fill::Solid(to_pax_pixels_color(color)),
        pax_runtime_api::Fill::LinearGradient(gradient) => {
            let start_x = gradient.start.0.evaluate(bounds, Axis::X);
            let start_y = gradient.start.1.evaluate(bounds, Axis::Y);
            let end_x = gradient.end.0.evaluate(bounds, Axis::X);
            let end_y = gradient.end.1.evaluate(bounds, Axis::Y);
            let local_pos =
                pax_pixels::Point2D::new((orig.x + start_x) as f32, (orig.y + start_y) as f32);
            let local_end =
                pax_pixels::Point2D::new((orig.x + end_x) as f32, (orig.y + end_y) as f32);
            let world_pos = transform.transform_point(local_pos);
            let world_end = transform.transform_point(local_end);
            let main_axis = world_end - world_pos;
            pax_pixels::Fill::Gradient {
                stops: gradient
                    .stops
                    .iter()
                    .map(|g| pax_pixels::GradientStop {
                        color: to_pax_pixels_color(&g.color),
                        stop: g
                            .position
                            .evaluate((main_axis.length() as f64, 0.0), Axis::X)
                            as f32,
                    })
                    .collect(),
                gradient_type: pax_pixels::GradientType::Linear,
                pos: world_pos,
                main_axis,
                off_axis: pax_pixels::Vector2D::zero(), //not used for linear
            }
        }
        pax_runtime_api::Fill::RadialGradient(gradient) => {
            let start_x = gradient.start.0.evaluate(bounds, Axis::X);
            let start_y = gradient.start.1.evaluate(bounds, Axis::Y);
            let end_x = gradient.end.0.evaluate(bounds, Axis::X);
            let end_y = gradient.end.1.evaluate(bounds, Axis::Y);
            let r = gradient.radius as f32;
            let local_pos =
                pax_pixels::Point2D::new((orig.x + start_x) as f32, (orig.y + start_y) as f32);
            let local_main_axis = pax_pixels::Vector2D::new(
                r * (end_x - start_x) as f32,
                r * (end_y - start_y) as f32,
            );
            let local_off_axis = pax_pixels::Vector2D::new(-local_main_axis.y, local_main_axis.x);
            let world_pos = transform.transform_point(local_pos);
            let world_main_axis = transform.transform_point(pax_pixels::Point2D::new(
                local_pos.x + local_main_axis.x,
                local_pos.y + local_main_axis.y,
            )) - world_pos;
            let world_off_axis = transform.transform_point(pax_pixels::Point2D::new(
                local_pos.x + local_off_axis.x,
                local_pos.y + local_off_axis.y,
            )) - world_pos;
            pax_pixels::Fill::Gradient {
                gradient_type: pax_pixels::GradientType::Radial,
                pos: world_pos,
                main_axis: world_main_axis,
                off_axis: world_off_axis,
                stops: gradient
                    .stops
                    .iter()
                    .map(|g| pax_pixels::GradientStop {
                        color: to_pax_pixels_color(&g.color),
                        stop: g
                            .position
                            .evaluate((world_main_axis.length() as f64, 0.0), Axis::X)
                            as f32,
                    })
                    .collect(),
            }
        }
    }
}

pub fn to_pax_pixels_color(color: &pax_runtime_api::Color) -> pax_pixels::Color {
    let [r, g, b, a] = color.to_rgba_0_1();
    pax_pixels::Color::rgba(r as f32, g as f32, b as f32, a as f32)
}

pub fn convert_kurbo_to_lyon_path(kurbo_path: &BezPath) -> Path {
    let mut builder = Path::builder();
    let mut closed = false;
    for el in kurbo_path.elements() {
        match el {
            PathEl::MoveTo(p) => {
                closed = false;
                builder.begin(point(p.x as f32, p.y as f32));
            }
            PathEl::LineTo(p) => {
                builder.line_to(point(p.x as f32, p.y as f32));
            }
            PathEl::QuadTo(p1, p2) => {
                builder.quadratic_bezier_to(
                    point(p1.x as f32, p1.y as f32),
                    point(p2.x as f32, p2.y as f32),
                );
            }
            PathEl::CurveTo(p1, p2, p3) => {
                builder.cubic_bezier_to(
                    point(p1.x as f32, p1.y as f32),
                    point(p2.x as f32, p2.y as f32),
                    point(p3.x as f32, p3.y as f32),
                );
            }
            PathEl::ClosePath => {
                closed = true;
                builder.end(true);
            }
        }
    }
    if !closed {
        builder.end(false);
    }

    builder.build()
}
