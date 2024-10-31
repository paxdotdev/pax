use pax_runtime_api::Fill;
use piet::{
    kurbo::{self, Affine, Shape},
    InterpolationMode, LinearGradient, RadialGradient,
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::api;

struct ImgData<R: piet::RenderContext> {
    img: R::Image,
    size: (usize, usize),
}

type ClearFn = Box<dyn Fn()>;
type ResizeFn = Box<dyn Fn()>;

pub struct PietRenderer<R: piet::RenderContext> {
    backends: Vec<(R, ClearFn, ResizeFn)>,
    image_map: HashMap<String, ImgData<R>>,
    layer_factory: Box<dyn Fn(usize) -> (R, ClearFn, ResizeFn)>,
}

impl<R: piet::RenderContext> PietRenderer<R> {
    pub fn new(layer_factory: impl Fn(usize) -> (R, ClearFn, ResizeFn) + 'static) -> Self {
        Self {
            layer_factory: Box::new(layer_factory),
            backends: Vec::new(),
            image_map: HashMap::new(),
        }
    }
}

impl<R: piet::RenderContext> api::RenderContext for PietRenderer<R> {
    fn fill(&mut self, layer: usize, path: kurbo::BezPath, fill: &Fill) {
        let rect = path.bounding_box();
        let brush = fill_to_piet_brush(fill, rect);
        if let Some((layer, _, _)) = self.backends.get_mut(layer) {
            layer.fill(path, &brush);
        }
    }

    fn stroke(&mut self, layer: usize, path: kurbo::BezPath, fill: &Fill, width: f64) {
        let rect = path.bounding_box();
        let brush = fill_to_piet_brush(fill, rect);
        if let Some((layer, _, _)) = self.backends.get_mut(layer) {
            layer.stroke(path, &brush, width);
        }
    }

    fn save(&mut self, layer: usize) {
        if let Some((layer, _, _)) = self.backends.get_mut(layer) {
            let _ = layer.save();
        }
    }

    fn transform(&mut self, layer: usize, affine: Affine) {
        if let Some((layer, _, _)) = self.backends.get_mut(layer) {
            layer.transform(affine);
        }
    }

    fn clip(&mut self, layer: usize, path: kurbo::BezPath) {
        if let Some((layer, _, _)) = self.backends.get_mut(layer) {
            layer.clip(path);
        }
    }

    fn restore(&mut self, layer: usize) {
        if let Some((layer, _, _)) = self.backends.get_mut(layer) {
            let _ = layer.restore();
        }
    }

    fn load_image(&mut self, path: &str, buf: &[u8], width: usize, height: usize) {
        //is this okay!? we know it's the same kind of backend no matter what layer, but it might be storing data?
        let (render_context, _, _) = self.backends.first_mut().unwrap();
        let img = render_context
            .make_image(width, height, buf, piet::ImageFormat::RgbaSeparate)
            .expect("image creation successful");
        self.image_map.insert(
            path.to_owned(),
            ImgData {
                img,
                size: (width, height),
            },
        );
    }

    fn get_image_size(&mut self, image_path: &str) -> Option<(usize, usize)> {
        self.image_map.get(image_path).map(|img| (img.size))
    }

    fn draw_image(&mut self, layer: usize, image_path: &str, rect: kurbo::Rect) {
        let Some(data) = self.image_map.get(image_path) else {
            return;
        };
        if let Some((layer, _, _)) = self.backends.get_mut(layer) {
            layer.draw_image(&data.img, rect, InterpolationMode::Bilinear);
        }
    }

    fn layers(&self) -> usize {
        self.backends.len()
    }

    fn resize_layers_to(&mut self, layer_count: usize, dirty_canvases: Rc<RefCell<Vec<bool>>>) {
        let current_len = self.backends.len();
        match layer_count.cmp(&current_len) {
            std::cmp::Ordering::Less => {
                self.backends.truncate(layer_count);
            }
            std::cmp::Ordering::Equal => return,
            std::cmp::Ordering::Greater => {
                for i in current_len..layer_count {
                    self.backends.push((self.layer_factory)(i));
                    if let Some(dirty_bit) = dirty_canvases.borrow_mut().get_mut(i) {
                        *dirty_bit = true;
                    }
                }
            }
        }
    }

    fn image_loaded(&self, path: &str) -> bool {
        self.image_map.contains_key(path)
    }

    fn clear(&mut self, layer: usize) {
        if let Some((_, clear_fn, _)) = self.backends.get_mut(layer) {
            (clear_fn)();
        }
    }

    fn flush(&mut self, _layer: usize, _dirty_canvases: Rc<RefCell<Vec<bool>>>) {
        // NOTE: used for GPU rendering to flush changes to the screen, not needed
        // during CPU rendering
    }

    fn resize(&mut self, _width: usize, _height: usize) {
        for (_, _, resize_fn) in &self.backends {
            (resize_fn)();
        }
    }
}

fn fill_to_piet_brush(fill: &Fill, rect: kurbo::Rect) -> piet::PaintBrush {
    match fill {
        Fill::Solid(color) => color.to_piet_color().into(),
        Fill::LinearGradient(linear) => {
            let linear_gradient = LinearGradient::new(
                Fill::to_unit_point(linear.start, (rect.width(), rect.height())),
                Fill::to_unit_point(linear.end, (rect.width(), rect.height())),
                Fill::to_piet_gradient_stops(linear.stops.clone()),
            );
            linear_gradient.into()
        }
        Fill::RadialGradient(radial) => {
            let origin = Fill::to_unit_point(radial.start, (rect.width(), rect.height()));
            let center = Fill::to_unit_point(radial.end, (rect.width(), rect.height()));
            let gradient_stops = Fill::to_piet_gradient_stops(radial.stops.clone());
            let radial_gradient = RadialGradient::new(radial.radius, gradient_stops)
                .with_center(center)
                .with_origin(origin);
            radial_gradient.into()
        }
    }
}
