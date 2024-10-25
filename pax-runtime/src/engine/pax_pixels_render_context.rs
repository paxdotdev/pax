use std::{cell::RefCell, future::Future, pin::Pin, rc::Rc};

use kurbo::{BezPath, PathEl, Shape};
use pax_pixels::{point, Path, WgpuRenderer};
use pax_runtime_api::RenderContext;

pub struct PaxPixelsRenderer {
    backends: Rc<RefCell<Vec<WgpuRenderer<'static>>>>,
    layer_factory: Rc<dyn Fn(usize) -> Pin<Box<dyn Future<Output = WgpuRenderer<'static>>>>>,
}

impl PaxPixelsRenderer {
    pub fn new(
        layer_factory: impl Fn(usize) -> Pin<Box<dyn Future<Output = WgpuRenderer<'static>>>> + 'static,
    ) -> Self {
        Self {
            backends: Default::default(),
            layer_factory: Rc::new(layer_factory),
        }
    }
}

impl RenderContext for PaxPixelsRenderer {
    fn fill(&mut self, layer: usize, path: kurbo::BezPath, fill: &pax_runtime_api::Fill) {
        let mut backends = self.backends.borrow_mut();
        let Some(context) = backends.get_mut(layer) else {
            return;
        };
        let path = convert_kurbo_to_lyon_path(&path);
        let fill = to_pax_pixels_fill(fill);
        log::debug!("{:?}, {:?}", path, fill);
        context.fill_path(path, fill);
    }

    fn stroke(
        &mut self,
        layer: usize,
        path: kurbo::BezPath,
        fill: &pax_runtime_api::Fill,
        width: f64,
    ) {
        let mut backends = self.backends.borrow_mut();
        let Some(context) = backends.get_mut(layer) else {
            return;
        };
        context.stroke_path(
            convert_kurbo_to_lyon_path(&path),
            to_pax_pixels_fill(fill),
            width as f32,
        );
    }

    fn save(&mut self, layer: usize) {
        // TODO
    }
    fn restore(&mut self, layer: usize) {
        // TODO
    }
    fn clip(&mut self, layer: usize, path: kurbo::BezPath) {
        // TODO
        // keep supporting paths here, or instead use rect?
    }
    fn transform(&mut self, layer: usize, affine: kurbo::Affine) {
        // TODO
    }
    fn load_image(&mut self, identifier: &str, image: &[u8], width: usize, height: usize) {
        //TODO
    }
    fn draw_image(&mut self, layer: usize, image_path: &str, rect: kurbo::Rect) {
        //TODO
    }
    fn get_image_size(&mut self, image_path: &str) -> Option<(usize, usize)> {
        // TODO
        Some((100, 100))
    }

    fn image_loaded(&self, image_path: &str) -> bool {
        // TODO
        true
    }

    fn layers(&self) -> usize {
        self.backends.borrow().len()
    }

    fn resize_layers_to(&mut self, layer_count: usize) {
        let current_len = self.backends.borrow().len();
        match layer_count.cmp(&current_len) {
            std::cmp::Ordering::Less => {
                self.backends.borrow_mut().truncate(layer_count);
            }
            std::cmp::Ordering::Equal => return,
            std::cmp::Ordering::Greater => {
                let backends = Rc::clone(&self.backends);
                let factory = Rc::clone(&self.layer_factory);

                wasm_bindgen_futures::spawn_local(async move {
                    for i in current_len..layer_count {
                        let backend = (factory)(i).await;
                        backends.borrow_mut().push(backend);
                    }
                });
            }
        }
    }

    fn clear(&mut self, layer: usize) {
        let mut backends = self.backends.borrow_mut();
        let Some(context) = backends.get_mut(layer) else {
            return;
        };
        context.clear();
    }

    fn flush(&mut self, layer: usize) {
        let mut backends = self.backends.borrow_mut();
        let Some(context) = backends.get_mut(layer) else {
            return;
        };
        context.flush();
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
