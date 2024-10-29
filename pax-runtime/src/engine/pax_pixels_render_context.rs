use kurbo::{BezPath, PathEl};
use pax_pixels::{point, Box2D, Image, Path, Transform2D, WgpuRenderer};
use pax_runtime_api::RenderContext;
use std::{cell::RefCell, collections::HashMap, future::Future, pin::Pin, rc::Rc};

pub struct PaxPixelsRenderer {
    backends: Rc<RefCell<Vec<RenderLayerState>>>,
    layer_factory:
        Rc<dyn Fn(usize) -> Pin<Box<dyn Future<Output = Option<WgpuRenderer<'static>>>>>>,
    image_map: HashMap<String, Image>,
}

pub enum RenderLayerState {
    Pending,
    Ready(WgpuRenderer<'static>),
}

impl PaxPixelsRenderer {
    pub fn new(
        layer_factory: impl Fn(usize) -> Pin<Box<dyn Future<Output = Option<WgpuRenderer<'static>>>>>
            + 'static,
    ) -> Self {
        Self {
            backends: Default::default(),
            layer_factory: Rc::new(layer_factory),
            image_map: Default::default(),
        }
    }
}

impl PaxPixelsRenderer {
    fn with_layer_context(&self, layer: usize, f: impl FnOnce(&mut WgpuRenderer)) {
        let mut backends = self.backends.borrow_mut();
        match backends.get_mut(layer) {
            Some(layer_state) => match layer_state {
                RenderLayerState::Pending => {
                    log::warn!(
                        "tried to retrieve layer {} context that wasn't ready",
                        layer
                    );
                }
                RenderLayerState::Ready(renderer) => f(renderer),
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
            let path = convert_kurbo_to_lyon_path(&path);
            let fill = to_pax_pixels_fill(fill);
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
            context.stroke_path(
                convert_kurbo_to_lyon_path(&path),
                to_pax_pixels_fill(fill),
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
    fn clip(&mut self, layer: usize, _path: kurbo::BezPath) {
        self.with_layer_context(layer, |_context| {
            // TODO
            // keep supporting paths here, or instead use rect?
            // context.push_clipping_bounds(..);
        });
    }
    fn transform(&mut self, layer: usize, affine: kurbo::Affine) {
        self.with_layer_context(layer, |context| {
            context.push_transform(Transform2D::from_array(
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
    }

    fn draw_image(&mut self, layer: usize, image_path: &str, rect: kurbo::Rect) {
        self.with_layer_context(layer, |context| {
            if let Some(image) = self.image_map.get(image_path) {
                context.draw_image(
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
                        if let (Some(change), Some(backend)) =
                            (backends.borrow_mut().get_mut(i), backend)
                        {
                            *change = RenderLayerState::Ready(backend);
                            if let Some(dirty_bit) = dirty_canvases.borrow_mut().get_mut(i) {
                                *dirty_bit = true;
                            }
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

    fn flush(&mut self, layer: usize) {
        self.with_layer_context(layer, |context| {
            context.flush();
        });
    }

    fn resize(&mut self, width: usize, height: usize) {
        for backend in &mut *self.backends.borrow_mut() {
            match backend {
                RenderLayerState::Pending => {
                    log::warn!("tried to resize backend that was pending")
                }
                RenderLayerState::Ready(renderer) => renderer.resize(width as f32, height as f32),
            }
        }
    }
}

fn to_pax_pixels_fill(fill: &pax_runtime_api::Fill) -> pax_pixels::Fill {
    match fill {
        pax_runtime_api::Fill::Solid(color) => pax_pixels::Fill::Solid({
            let [r, g, b, a] = color.to_rgba_0_1();
            pax_pixels::Color::rgba(r as f32, g as f32, b as f32, a as f32)
        }),
        // TODO fill in impls
        pax_runtime_api::Fill::LinearGradient(_gradient) => todo!(),
        pax_runtime_api::Fill::RadialGradient(_graident) => todo!(),
    }
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
