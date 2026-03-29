use kurbo::{BezPath, PathEl, Shape};
use pax_pixels::{point, Box2D, Image, Path, Transform2D, WgpuRenderer};
use pax_runtime_api::{Axis, RenderContext};
use std::{cell::RefCell, collections::HashMap, future::Future, pin::Pin, rc::Rc};

type LayerDef = (WgpuRenderer<'static>, Pin<Box<dyn Fn() -> LayerSurfaceSize>>);

pub struct LayerSurfaceSize {
    pub surface_width: u32,
    pub surface_height: u32,
    pub dpr: u32,
}

pub struct PaxPixelsRenderer {
    backends: Rc<RefCell<Vec<RenderLayerState>>>,
    layer_factory: Rc<dyn Fn(usize) -> Pin<Box<dyn Future<Output = Option<LayerDef>>>>>,
    image_map: HashMap<String, Image>,
    image_versions: HashMap<String, u64>,
    failed_context_gets: RefCell<Vec<bool>>,
}

pub enum RenderLayerState {
    Pending,
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
        }
    }
}

impl PaxPixelsRenderer {
    fn with_layer_context(&self, layer: usize, f: impl FnOnce(&mut WgpuRenderer)) {
        let mut backends = self.backends.borrow_mut();
        match backends.get_mut(layer) {
            Some(layer_state) => match layer_state {
                RenderLayerState::Pending => {
                    let mut failed_context_gets = self.failed_context_gets.borrow_mut();
                    if failed_context_gets.len() <= layer {
                        failed_context_gets.resize(layer + 1, false);
                    }
                    failed_context_gets[layer] = true;
                    // this happens to often to be useful right now - how to handle asyncness
                    // better here feels important
                    // log::warn!(
                    //     "tried to retrieve layer {} context that wasn't ready",
                    //     layer
                    // );
                }
                RenderLayerState::Ready((renderer, _)) => f(renderer),
            },
            None => log::warn!(
                "tried to retrieve layer {} context for non-existent layer",
                layer
            ),
        }
    }
}

impl RenderContext for PaxPixelsRenderer {
    fn fill(&mut self, layer: usize, path: kurbo::BezPath, fill: &pax_runtime_api::Fill) {
        self.with_layer_context(layer, |context| {
            let bounds = path.bounding_box();
            let path = convert_kurbo_to_lyon_path(&path);
            let fill = to_pax_pixels_fill(fill, bounds, context.current_transform());
            context.fill_path(path, fill);
        });
    }

    fn stroke(
        &mut self,
        layer: usize,
        path: kurbo::BezPath,
        fill: &pax_runtime_api::Fill,
        width: f64,
    ) {
        self.with_layer_context(layer, |context| {
            let bounds = path.bounding_box();
            context.stroke_path(
                convert_kurbo_to_lyon_path(&path),
                to_pax_pixels_fill(fill, bounds, context.current_transform()),
                width as f32,
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
        *self.image_versions.entry(identifier.to_string()).or_insert(0) += 1;
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

    fn resize_layers_to(&mut self, layer_count: usize, dirty_canvases: Rc<RefCell<Vec<bool>>>) {
        let current_len = self.backends.borrow().len();
        match layer_count.cmp(&current_len) {
            std::cmp::Ordering::Less => {
                self.backends.borrow_mut().truncate(layer_count);
            }
            std::cmp::Ordering::Equal => return,
            std::cmp::Ordering::Greater => {
                for i in current_len..layer_count {
                    self.backends.borrow_mut().push(RenderLayerState::Pending);
                    let factory = Rc::clone(&self.layer_factory);
                    let backends = Rc::clone(&self.backends);
                    let dirty_canvases = Rc::clone(&dirty_canvases);
                    wasm_bindgen_futures::spawn_local(async move {
                        let backend = (factory)(i).await;
                        match (backends.borrow_mut().get_mut(i), backend) {
                            (Some(change), Some(layer_def)) => {
                                *change = RenderLayerState::Ready(layer_def);
                                if let Some(dirty_bit) = dirty_canvases.borrow_mut().get_mut(i) {
                                    *dirty_bit = true;
                                }
                            }
                            (Some(_), None) => log::warn!(
                                "failed to set poll state to ready: backend failed to initialize"
                            ),
                            (None, Some(_)) => {
                                log::warn!("failed to set poll state to ready: layer doesn't exist anymore")
                            }
                            (None, None) => log::warn!("failed to set poll state to ready: layer doesn't exist AND backend failed to initialize")
                        }
                    });
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
        // HACK: GPU rendering currently doesn't correctly handle
        // re-dirtyfying canvases that where created but not ready before
        // being drawn to - maybe encapsulate canvas dirtifiation inside
        // rendercontext?
        if let Some(dirty_bit) = dirty_canvases.borrow_mut().get_mut(layer) {
            *dirty_bit = true;
        }

        self.with_layer_context(layer, |context| {
            if let Some(failed) = self.failed_context_gets.borrow_mut().get_mut(layer) {
                if *failed {
                    if let Some(dirty_bit) = dirty_canvases.borrow_mut().get_mut(layer) {
                        // if we failed to draw to this layer last frame because the context wasn't
                        // vailable yet, set this canvas to dirty for next frame
                        *dirty_bit = true;
                    }
                    *failed = false;
                }
            }
            context.flush();
        });
    }

    fn resize(&mut self, width: usize, height: usize) {
        for backend in &mut *self.backends.borrow_mut() {
            match backend {
                RenderLayerState::Pending => {
                    log::warn!("tried to resize backend that was pending")
                }
                RenderLayerState::Ready((renderer, canvas_resizer)) => {
                    let surface = (canvas_resizer)();
                    renderer.resize_surface(
                        surface.surface_width as f32,
                        surface.surface_height as f32,
                    );
                    renderer.set_viewport(width as f32, height as f32, surface.dpr as f32);
                }
            }
        }
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
            let local_pos = pax_pixels::Point2D::new((orig.x + start_x) as f32, (orig.y + start_y) as f32);
            let local_end = pax_pixels::Point2D::new((orig.x + end_x) as f32, (orig.y + end_y) as f32);
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
            let local_pos = pax_pixels::Point2D::new((orig.x + start_x) as f32, (orig.y + start_y) as f32);
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
