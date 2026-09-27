use super::*;
use kurbo::{Affine, BezPath, Rect};
use pax_runtime_api::{Material, Paint, SceneLighting, Stroke};

#[derive(Default)]
struct RecordingRenderer {
    layer_count: usize,
    pending: bool,
    removals: Vec<(usize, u32)>,
}

impl RenderContext for RecordingRenderer {
    fn layers(&self) -> usize {
        self.layer_count
    }
    fn resize_layers_to(&mut self, count: usize, _: Rc<RefCell<Vec<bool>>>) {
        self.layer_count = count;
    }
    fn clear(&mut self, layer: usize) {
        assert!(layer < self.layer_count);
    }
    fn flush(&mut self, layer: usize, _: Rc<RefCell<Vec<bool>>>) {
        assert!(layer < self.layer_count);
    }
    fn set_scene_lighting(&mut self, layer: usize, _: &SceneLighting) {
        assert!(layer < self.layer_count);
    }
    fn remove_node(&mut self, layer: usize, node: u32) -> bool {
        assert!(layer < self.layer_count);
        self.removals.push((layer, node));
        !self.pending
    }
    fn fill_with_opacity(&mut self, _: usize, _: BezPath, _: &Paint, _: f64) {
        unreachable!()
    }
    fn stroke_with_opacity(&mut self, _: usize, _: BezPath, _: &Stroke, _: f64) {
        unreachable!()
    }
    fn stroke_with_draw_range_and_material_and_opacity(
        &mut self,
        _: usize,
        _: BezPath,
        _: &Stroke,
        _: &Material,
        _: f64,
        _: f64,
        _: f64,
    ) {
        unreachable!()
    }
    fn save(&mut self, _: usize) {
        unreachable!()
    }
    fn restore(&mut self, _: usize) {
        unreachable!()
    }
    fn clip(&mut self, _: usize, _: BezPath) {
        unreachable!()
    }
    fn transform(&mut self, _: usize, _: Affine) {
        unreachable!()
    }
    fn load_image(&mut self, _: &str, _: &[u8], _: usize, _: usize) {
        unreachable!()
    }
    fn draw_image(&mut self, _: usize, _: &str, _: Rect) {
        unreachable!()
    }
    fn draw_image_with_opacity(&mut self, _: usize, _: &str, _: Rect, _: f64) {
        unreachable!()
    }
    fn get_image_size(&mut self, _: &str) -> Option<(usize, usize)> {
        unreachable!()
    }
    fn image_loaded(&self, _: &str) -> bool {
        unreachable!()
    }
    fn resize(&mut self, _: usize, _: usize) {
        unreachable!()
    }
    fn refresh_layers(&mut self, _: &[usize]) {
        unreachable!()
    }
}

fn engine() -> PaxEngine {
    crate::test_support::empty_engine(
        (320.0, 240.0),
        Platform::Web,
        OS::Mac,
        Box::new(|| 0),
        Default::default(),
    )
}

#[test]
fn removed_layers_discard_queued_node_removals() {
    let mut engine = engine();
    let mut renderer = RecordingRenderer::default();
    engine.runtime_context.layer_count.set(7);
    engine.render(&mut renderer);
    engine.runtime_context.enqueue_canvas_node_removal(6, 42);
    engine.runtime_context.enqueue_canvas_node_removal(0, 43);
    engine.runtime_context.layer_count.set(6);
    engine.render(&mut renderer);
    assert_eq!(renderer.removals, vec![(0, 43)]);
    assert!(!engine.runtime_context.has_canvas_render_work());
    engine.render(&mut renderer);
    assert_eq!(renderer.removals, vec![(0, 43)]);
}

#[test]
fn layer_synchronization_is_local_to_each_renderer_and_engine() {
    let mut first = engine();
    let mut second = engine();
    let mut first_renderer = RecordingRenderer::default();
    let mut second_renderer = RecordingRenderer::default();
    first.runtime_context.layer_count.set(2);
    second.runtime_context.layer_count.set(2);
    first.render(&mut first_renderer);
    second.render(&mut second_renderer);
    assert_eq!((first_renderer.layers(), second_renderer.layers()), (2, 2));
    assert_eq!(second.runtime_context.dirty_canvases.borrow().len(), 2);
    let mut replacement = RecordingRenderer::default();
    second.render(&mut replacement);
    assert_eq!(replacement.layers(), 2);
}

#[test]
fn pending_layer_removals_retry_until_the_backend_is_ready() {
    let mut engine = engine();
    let mut renderer = RecordingRenderer {
        pending: true,
        ..Default::default()
    };
    engine.runtime_context.layer_count.set(1);
    engine.runtime_context.enqueue_canvas_node_removal(0, 42);
    engine.render(&mut renderer);
    assert!(engine.runtime_context.has_canvas_node_removals());
    renderer.pending = false;
    engine.render(&mut renderer);
    assert_eq!(renderer.removals, vec![(0, 42), (0, 42)]);
    assert!(!engine.runtime_context.has_canvas_render_work());
}
