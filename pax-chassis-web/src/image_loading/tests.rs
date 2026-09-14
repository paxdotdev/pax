use super::*;
use pax_runtime::api::{
    pax_value::{PaxAny, ToFromPaxAny},
    Fill, Material, Platform, Stroke, OS,
};
use pax_runtime::{
    CommonPropertiesInit, ComponentInstance, ExpandedNode, InstanceNode, InstantiationArgs,
    PropertiesInit, PropertiesScopeInit,
};
use pax_std::media::image::{Image, ImageInstance, ImageSource};
use piet::kurbo::{Affine, BezPath, Rect};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

// Model retained node recording while exercising the real Image primitive and
// engine render gate. GPU rasterization is covered separately by pixel tests.
#[derive(Default)]
struct RecordingRenderer {
    images: HashMap<String, (usize, usize)>,
    retained: HashMap<u32, String>,
    pending: Option<(u32, Option<String>)>,
    layer_count: usize,
    flushes: usize,
}

impl RenderContext for RecordingRenderer {
    fn fill_with_opacity(&mut self, _: usize, _: BezPath, _: &Fill, _: f64) {}
    fn stroke_with_opacity(&mut self, _: usize, _: BezPath, _: &Stroke, _: f64) {}
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
    }
    fn save(&mut self, _: usize) {}
    fn restore(&mut self, _: usize) {}
    fn clip(&mut self, _: usize, _: BezPath) {}
    fn transform(&mut self, _: usize, _: Affine) {}
    fn load_image(&mut self, path: &str, _: &[u8], width: usize, height: usize) {
        self.images.insert(path.into(), (width, height));
    }
    fn draw_image(&mut self, _: usize, path: &str, _: Rect) {
        assert!(self.images.contains_key(path));
        self.pending.as_mut().unwrap().1 = Some(path.into());
    }
    fn get_image_size(&mut self, path: &str) -> Option<(usize, usize)> {
        self.images.get(path).copied()
    }
    fn image_loaded(&self, path: &str) -> bool {
        self.images.contains_key(path)
    }
    fn layers(&self) -> usize {
        self.layer_count
    }
    fn resize_layers_to(&mut self, count: usize, _: Rc<RefCell<Vec<bool>>>) {
        self.layer_count = count;
    }
    fn clear(&mut self, _: usize) {
        self.retained.clear();
    }
    fn flush(&mut self, _: usize, _: Rc<RefCell<Vec<bool>>>) {
        self.flushes += 1;
    }
    fn resize(&mut self, _: usize, _: usize) {}
    fn refresh_layers(&mut self, _: &[usize]) {}
    fn begin_node(&mut self, _: usize, id: u32, _: i32, _: u32) -> bool {
        assert!(self.pending.replace((id, None)).is_none());
        true
    }
    fn end_node(&mut self, _: usize, id: u32) -> bool {
        let (pending_id, image) = self.pending.take().unwrap();
        assert_eq!(id, pending_id);
        if let Some(path) = image {
            self.retained.insert(id, path);
        } else {
            self.retained.remove(&id);
        }
        true
    }
    fn remove_node(&mut self, _: usize, id: u32) -> bool {
        self.retained.remove(&id);
        true
    }
}

fn args(properties: PropertiesInit) -> InstantiationArgs {
    InstantiationArgs {
        prototypical_common_properties: CommonPropertiesInit::Default,
        prototypical_properties: properties,
        handler_registry: None,
        children: None,
        component_template: None,
        component_settings: None,
        template_node_identifier: None,
        template_node_type_id: None,
        template_node_selector_info: None,
        transition_config: Default::default(),
        properties_scope: PropertiesScopeInit::None,
    }
}

fn fixture(paths: &[&'static str]) -> (PaxEngine, Vec<Rc<ExpandedNode>>, RecordingRenderer) {
    let children = paths
        .iter()
        .map(|&path| {
            ImageInstance::instantiate(args(PropertiesInit::Factory(Box::new(move |_, node| {
                node.is_none().then(|| {
                    let image = Image::default();
                    image.source.set(ImageSource::Url(path.into()));
                    Rc::new(RefCell::new(image.to_pax_any()))
                })
            })))) as Rc<dyn InstanceNode>
        })
        .collect();
    let mut root_args = args(PropertiesInit::Factory(Box::new(|_, node| {
        node.is_none()
            .then(|| Rc::new(RefCell::new(PaxAny::Builtin(Default::default()))))
    })));
    root_args.component_template = Some(RefCell::new(children));
    let mut engine = PaxEngine::new_empty(
        (320.0, 240.0),
        Platform::Web,
        OS::Mac,
        Box::new(|| 0),
        Default::default(),
    );
    let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
    engine.tick();
    let nodes = root.mounted_children.borrow().clone();
    let mut renderer = RecordingRenderer::default();
    // Tests create several engines in one process. Initialize each backend
    // explicitly rather than relying on the engine's process-wide layer-count cache.
    let layers = engine.runtime_context.layer_count.get();
    engine.runtime_context.resize_canvas_layers_to(layers);
    renderer.resize_layers_to(layers, engine.runtime_context.dirty_canvases.clone());
    engine.render(&mut renderer);
    assert!(renderer.retained.is_empty());
    assert!(!engine.runtime_context.has_canvas_render_work());
    (engine, nodes, renderer)
}

fn complete(engine: &PaxEngine, renderer: &mut RecordingRenderer, id: u32, path: &str) {
    complete_image_load(
        engine,
        renderer,
        &ImageDataArgs {
            id,
            path: path.into(),
            width: 1,
            height: 1,
        },
        &[255, 0, 0, 255],
    );
}

#[test]
fn delayed_image_load_wakes_an_idle_canvas_without_remounting() {
    let (mut engine, nodes, mut renderer) = fixture(&["first.png"]);
    let id = nodes[0].id;
    assert!(engine.runtime_context.is_canvas_node_dirty(&id));
    let flushes = renderer.flushes;
    // Reproduce the old completion path: populating the cache alone leaves
    // the canvas asleep, even though this mounted Image is still node-dirty.
    renderer.load_image("first.png", &[255, 0, 0, 255], 1, 1);
    engine.render(&mut renderer);
    assert!(renderer.retained.is_empty());
    assert_eq!(renderer.flushes, flushes);
    complete(&engine, &mut renderer, id.to_u32(), "first.png");
    assert!(renderer.image_loaded("first.png"));
    assert!(
        engine.runtime_context.has_canvas_render_work(),
        "decode completion must wake the clean layer"
    );
    engine.render(&mut renderer);
    assert_eq!(
        renderer.retained.get(&id.to_u32()).map(String::as_str),
        Some("first.png")
    );
    assert!(renderer.flushes > flushes);
    assert!(!engine.runtime_context.is_canvas_node_dirty(&id));
    assert!(!engine.runtime_context.has_canvas_render_work());
    let flushes = renderer.flushes;
    engine.render(&mut renderer);
    assert_eq!(
        renderer.flushes, flushes,
        "loaded images must settle back to idle"
    );
}

#[test]
fn completion_invalidates_only_its_live_requesting_node_and_current_layer() {
    let (engine, nodes, mut renderer) = fixture(&["shared.png", "other.png"]);
    for node in &nodes {
        engine.runtime_context.clear_canvas_node_dirty(&node.id);
    }
    engine
        .runtime_context
        .dirty_canvases
        .borrow_mut()
        .resize(3, false);
    nodes[0]
        .occlusion
        .update(|occlusion| occlusion.render_layer_id = 2);
    complete(&engine, &mut renderer, nodes[0].id.to_u32(), "shared.png");
    assert_eq!(
        engine.runtime_context.dirty_canvas_node_ids(),
        vec![nodes[0].id]
    );
    assert_eq!(engine.runtime_context.dirty_canvas_layers(), vec![2]);
}

#[test]
fn concurrent_requests_for_one_source_each_receive_completion() {
    let (mut engine, nodes, mut renderer) = fixture(&["shared.png", "shared.png"]);
    for node in &nodes {
        complete(&engine, &mut renderer, node.id.to_u32(), "shared.png");
    }
    engine.render(&mut renderer);
    for node in &nodes {
        assert_eq!(
            renderer.retained.get(&node.id.to_u32()).map(String::as_str),
            Some("shared.png")
        );
    }
}

#[test]
fn late_completion_after_unmount_keeps_the_cache_without_scheduling_stale_nodes() {
    let (mut engine, nodes, mut renderer) = fixture(&["old.png"]);
    nodes[0].clone().recurse_unmount(&engine.runtime_context);
    engine.render(&mut renderer);
    assert!(engine.get_expanded_node(nodes[0].id).is_none());
    let dirty_nodes = engine.runtime_context.dirty_canvas_node_ids();
    complete(&engine, &mut renderer, nodes[0].id.to_u32(), "old.png");
    assert!(renderer.image_loaded("old.png"));
    assert!(!engine.runtime_context.has_canvas_render_work());
    assert_eq!(engine.runtime_context.dirty_canvas_node_ids(), dirty_nodes);
}

#[test]
fn out_of_order_completions_render_the_current_source() {
    let (mut engine, nodes, mut renderer) = fixture(&["old.png"]);
    nodes[0].with_properties_unwrapped(|image: &mut Image| {
        image.source.set(ImageSource::Url("new.png".into()));
    });
    engine.tick();
    engine.render(&mut renderer);
    complete(&engine, &mut renderer, nodes[0].id.to_u32(), "old.png");
    engine.render(&mut renderer);
    assert!(
        renderer.retained.is_empty(),
        "a stale response must not change the current source"
    );
    complete(&engine, &mut renderer, nodes[0].id.to_u32(), "new.png");
    engine.render(&mut renderer);
    assert_eq!(
        renderer
            .retained
            .get(&nodes[0].id.to_u32())
            .map(String::as_str),
        Some("new.png")
    );
}
