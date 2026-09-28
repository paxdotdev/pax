#![cfg(not(feature = "designtime"))]

use pax_runtime::api::*;
use pax_runtime::scene_geometry::{CanvasGeometry, PreparedCanvasGeometry};
use pax_runtime::*;
use pax_runtime_api::pax_value::PaxAny;
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    rc::Rc,
};

fn args() -> InstantiationArgs {
    InstantiationArgs {
        prototypical_common_properties: CommonPropertiesInit::Default,
        prototypical_properties: PropertiesInit::Factory(Box::new(|_, node| {
            node.is_none()
                .then(|| Rc::new(RefCell::new(PaxAny::Builtin(Default::default()))))
        })),
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

#[derive(Default)]
struct Recorder {
    nodes: HashSet<u32>,
    layers: usize,
}
impl RenderContext for Recorder {
    fn begin_node(&mut self, _: usize, id: u32, _: i32, _: u32) -> bool {
        self.nodes.insert(id);
        true
    }
    fn remove_node(&mut self, _: usize, id: u32) -> bool {
        self.nodes.remove(&id);
        true
    }
    fn save(&mut self, _: usize) {}
    fn restore(&mut self, _: usize) {}
    fn clip(&mut self, _: usize, _: kurbo::BezPath) {}
    fn layers(&self) -> usize {
        self.layers
    }
    fn fill_with_opacity(&mut self, _: usize, _: kurbo::BezPath, _: &Fill, _: f64) {}
    fn stroke_with_opacity(&mut self, _: usize, _: kurbo::BezPath, _: &Stroke, _: f64) {}
    fn stroke_with_draw_range_and_material_and_opacity(
        &mut self,
        _: usize,
        _: kurbo::BezPath,
        _: &Stroke,
        _: &Material,
        _: f64,
        _: f64,
        _: f64,
    ) {
    }
    fn transform(&mut self, _: usize, _: kurbo::Affine) {}
    fn load_image(&mut self, _: &str, _: &[u8], _: usize, _: usize) {}
    fn draw_image_with_opacity(&mut self, _: usize, _: &str, _: kurbo::Rect, _: f64) {}
    fn get_image_size(&mut self, _: &str) -> Option<(usize, usize)> {
        None
    }
    fn image_loaded(&self, _: &str) -> bool {
        false
    }
    fn resize_layers_to(&mut self, n: usize, _: Rc<RefCell<Vec<bool>>>) {
        self.layers = n;
    }
    fn clear(&mut self, _: usize) {
        self.nodes.clear();
    }
    fn flush(&mut self, _: usize, _: Rc<RefCell<Vec<bool>>>) {}
    fn resize(&mut self, _: usize, _: usize) {}
    fn refresh_layers(&mut self, _: &[usize]) {}
}

struct GeometryLeaf {
    base: BaseInstance,
    preparations: Cell<usize>,
    drawn: RefCell<Option<Rc<PreparedCanvasGeometry>>>,
}
impl InstanceNode for GeometryLeaf {
    fn instantiate(args: InstantiationArgs) -> Rc<Self> {
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    layer: Layer::Canvas,
                    invisible_to_slot: false,
                    invisible_to_raycasting: true,
                    is_component: false,
                    is_slot: false,
                },
            ),
            preparations: Cell::new(0),
            drawn: RefCell::new(None),
        })
    }
    fn base(&self) -> &BaseInstance {
        &self.base
    }
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("GeometryLeaf").finish()
    }
    fn handle_mount(self: Rc<Self>, node: &Rc<ExpandedNode>, ctx: &Rc<RuntimeContext>) {
        let weak_node = Rc::downgrade(node);
        let ctx = ctx.clone();
        node.changed_listener.replace_with(Property::computed(
            move || {
                if let Some(node) = weak_node.upgrade() {
                    ctx.mark_canvas_node_dirty(node.id);
                    ctx.set_canvas_dirty(node.occlusion.get().render_layer_id);
                }
            },
            &[node.transform_and_bounds.untyped()],
        ));
    }
    fn prepare_canvas_geometry(&self, node: &ExpandedNode) -> CanvasGeometry {
        self.preparations.set(self.preparations.get() + 1);
        CanvasGeometry::for_layout(node.transform_and_bounds.get().bounds)
    }
    fn render(&self, node: &ExpandedNode, ctx: &Rc<RuntimeContext>, rc: &mut dyn RenderContext) {
        let geometry = ctx.canvas_geometry_for_node(node);
        rc.begin_node_with_bounds(
            geometry.layer,
            node.id.to_u32(),
            0,
            geometry.coverage_bounds.unwrap(),
            0,
        );
        rc.end_node(geometry.layer, node.id.to_u32());
        *self.drawn.borrow_mut() = Some(geometry);
        ctx.clear_canvas_node_dirty(&node.id);
    }
}

fn fixture() -> (PaxEngine, Rc<GeometryLeaf>, Rc<ExpandedNode>) {
    let mut engine = PaxEngine::new_empty(
        (320.0, 240.0),
        Platform::Web,
        OS::Mac,
        Box::new(|| 0),
        Default::default(),
    );
    let leaf = GeometryLeaf::instantiate(args());
    let mut root_args = args();
    root_args.component_template = Some(RefCell::new(vec![leaf.clone()]));
    let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
    engine.runtime_context.resize_canvas_layers_to(1);
    engine.tick();
    let node = root.children.get()[0].clone();
    (engine, leaf, node)
}

#[test]
fn replay_and_drawing_share_prepared_geometry_before_first_draw_and_at_rest() {
    let (mut engine, leaf, node) = fixture();
    let ctx = engine.runtime_context.clone();
    assert!(leaf.drawn.borrow().is_none());
    let geometry = ctx.canvas_geometry_for_node(&node);
    let region = geometry.coverage_bounds.unwrap();
    let initial = ctx.scene_geometry_stats();
    for _ in 0..3 {
        assert_eq!(
            ctx.canvas_nodes_intersecting(geometry.layer, &[region]),
            Some(vec![node.id.to_u32()])
        );
    }
    ctx.request_canvas_replay(ReplayCanvasLayerUpdate {
        layer: geometry.layer,
        regions: Some(vec![region]),
    });
    let mut recorder = Recorder::default();
    engine.render(&mut recorder);
    assert!(Rc::ptr_eq(&geometry, leaf.drawn.borrow().as_ref().unwrap()));
    assert_eq!(
        ctx.scene_geometry_stats().preparations,
        initial.preparations
    );
    assert_eq!(
        ctx.scene_geometry_stats().index_updates,
        initial.index_updates
    );
    let after_render = ctx.scene_geometry_stats();
    for _ in 0..5 {
        engine.tick();
        engine.render(&mut recorder);
    }
    assert_eq!(
        ctx.scene_geometry_stats(),
        after_render,
        "idle adds no geometry work"
    );
}

#[test]
fn layout_changes_refresh_before_query_and_unmount_removes_coverage() {
    let (mut engine, leaf, node) = fixture();
    let ctx = engine.runtime_context.clone();
    let original = ctx.canvas_geometry_for_node(&node);
    let preparations = leaf.preparations.get();
    let common = node.get_common_properties();
    common.borrow().x.set(Some(Size::Pixels(4000.0.into())));
    common
        .borrow()
        .rotate
        .set(Some(Rotation::Degrees(30.0.into())));
    engine.tick();
    let moved = ctx.canvas_geometry_for_node(&node);
    assert!(!Rc::ptr_eq(&original, &moved));
    assert_eq!(leaf.preparations.get(), preparations + 1);
    assert_eq!(
        ctx.canvas_nodes_intersecting(original.layer, &[original.coverage_bounds.unwrap()]),
        None
    );
    assert_eq!(
        ctx.canvas_nodes_intersecting(moved.layer, &[moved.coverage_bounds.unwrap()]),
        Some(vec![node.id.to_u32()])
    );
    let counts = ctx.scene_geometry_stats();
    node.clone().recurse_unmount(&ctx);
    assert_eq!(
        ctx.canvas_nodes_intersecting(moved.layer, &[moved.coverage_bounds.unwrap()]),
        None
    );
    assert_eq!(ctx.scene_geometry_stats().preparations, counts.preparations);
}

// A native scroll owner uses the same render ancestry and parent-frame contract
// as ScrollerHost, without needing a platform surface for this geometry test.
struct ScrollOwner {
    base: BaseInstance,
}
impl InstanceNode for ScrollOwner {
    fn instantiate(args: InstantiationArgs) -> Rc<Self> {
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    layer: Layer::Native,
                    invisible_to_slot: false,
                    invisible_to_raycasting: true,
                    is_component: false,
                    is_slot: false,
                },
            ),
        })
    }
    fn base(&self) -> &BaseInstance {
        &self.base
    }
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("ScrollOwner").finish()
    }
    fn scrolls_content(&self, _: &ExpandedNode) -> bool {
        true
    }
    fn clips_content(&self, _: &ExpandedNode) -> bool {
        true
    }
    fn handle_mount(self: Rc<Self>, node: &Rc<ExpandedNode>, ctx: &Rc<RuntimeContext>) {
        let children = self
            .base
            .get_instance_children()
            .borrow()
            .iter()
            .cloned()
            .map(|child| (child, node.stack.clone()))
            .collect::<Vec<_>>();
        let frame = Property::new(Some(node.id));
        let children = node.generate_children(children, ctx, &frame, true);
        node.children.set(children);
    }
}

#[test]
fn nested_scroll_owners_share_content_geometry_across_native_scroll_and_resize() {
    let mut engine = PaxEngine::new_empty(
        (640.0, 480.0),
        Platform::Web,
        OS::Mac,
        Box::new(|| 0),
        Default::default(),
    );
    let ctx = engine.runtime_context.clone();
    ctx.globals()
        .browser_allows_scroller_vector_layers
        .set(true);
    ctx.globals()
        .browser_allows_nested_scroller_vector_layers
        .set(true);
    let leaf = GeometryLeaf::instantiate(args());
    let mut inner_args = args();
    inner_args.children = Some(RefCell::new(vec![leaf.clone()]));
    let inner = ScrollOwner::instantiate(inner_args);
    let mut outer_args = args();
    outer_args.children = Some(RefCell::new(vec![inner]));
    let outer = ScrollOwner::instantiate(outer_args);
    let mut root_args = args();
    root_args.component_template = Some(RefCell::new(vec![outer]));
    let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
    engine.tick();
    let outer = root.children.get()[0].clone();
    let inner = outer.children.get()[0].clone();
    let node = inner.children.get()[0].clone();
    outer
        .get_common_properties()
        .borrow()
        .x
        .set(Some(Size::Pixels(100.0.into())));
    inner
        .get_common_properties()
        .borrow()
        .x
        .set(Some(Size::Pixels(30.0.into())));
    node.get_common_properties()
        .borrow()
        .x
        .set(Some(Size::Pixels(20.0.into())));
    ctx.set_scroller_surface_state(
        outer.id.to_u32(),
        ScrollerSurfaceState {
            viewport_width: 320.0,
            viewport_height: 240.0,
            content_width: 320.0,
            content_height: 10000.0,
            ..Default::default()
        },
    );
    ctx.set_scroller_surface_state(
        inner.id.to_u32(),
        ScrollerSurfaceState {
            viewport_width: 200.0,
            viewport_height: 100.0,
            content_width: 10000.0,
            content_height: 100.0,
            ..Default::default()
        },
    );
    engine.tick();
    let geometry = ctx.canvas_geometry_for_node(&node);
    assert_eq!(ctx.get_layer_scroller_owner(geometry.layer), Some(inner.id));
    assert_eq!(
        geometry.surface_transform * kurbo::Point::ZERO,
        kurbo::Point::new(20.0, 0.0)
    );
    let before = ctx.scene_geometry_stats();
    ctx.update_scroller_surface_scroll(outer.id.to_u32(), 0.0, 1000.0, 0.0, 1000.0);
    ctx.update_scroller_surface_scroll(inner.id.to_u32(), 500.0, 0.0, 500.0, 0.0);
    engine.tick();
    assert!(Rc::ptr_eq(&geometry, &ctx.canvas_geometry_for_node(&node)));
    assert_eq!(ctx.scene_geometry_stats().preparations, before.preparations);
    assert_eq!(
        ctx.scene_geometry_stats().index_updates,
        before.index_updates
    );
    ctx.canvas_nodes_intersecting(
        geometry.layer,
        &[kurbo::Rect::new(500.0, 0.0, 700.0, 100.0)],
    );
    engine.set_viewport_size((800.0, 600.0));
    engine.tick();
    let resized = ctx.canvas_geometry_for_node(&node);
    assert_eq!(
        resized.surface_transform * kurbo::Point::ZERO,
        kurbo::Point::new(20.0, 0.0)
    );
}
