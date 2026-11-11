use super::*;
use crate::{
    BaseInstance, CommonPropertiesInit, ComponentInstance, InstanceFlags, InstanceNode,
    InstantiationArgs, PaxEngine, PropertiesInit, PropertiesScopeInit,
};
use kurbo::Rect;
use pax_runtime_api::{Layer, Platform, Size, OS};

fn args(children: Vec<Rc<dyn InstanceNode>>) -> InstantiationArgs {
    InstantiationArgs {
        prototypical_common_properties: CommonPropertiesInit::Default,
        prototypical_properties: PropertiesInit::Factory(Box::new(|_, node| {
            node.is_none().then(|| {
                Rc::new(RefCell::new(pax_runtime_api::pax_value::PaxAny::Builtin(
                    Default::default(),
                )))
            })
        })),
        handler_registry: None,
        children: Some(RefCell::new(children)),
        component_template: None,
        component_settings: None,
        component_camera: None,
        template_node_identifier: None,
        template_node_type_id: None,
        template_node_selector_info: None,
        transition_config: Default::default(),
        properties_scope: PropertiesScopeInit::None,
    }
}

struct HitNode {
    base: BaseInstance,
    bounded: bool,
    clip: bool,
    scroll: bool,
    scroll_y: LocalProperty<f64>,
    tests: Rc<Cell<u64>>,
    shape: RefCell<Option<Rect>>,
}
impl HitNode {
    fn new(
        children: Vec<Rc<dyn InstanceNode>>,
        bounded: bool,
        clip: bool,
        shape: Option<Rect>,
        tests: Rc<Cell<u64>>,
    ) -> Rc<Self> {
        let scroll_y = LocalProperty::new(0.0);
        let offset = scroll_y.clone();
        let mut arguments = args(children);
        arguments.properties_scope = PropertiesScopeInit::Factory(Box::new(move |_| {
            std::collections::HashMap::from([(
                "scroll_y".into(),
                pax_runtime_api::Variable::new_from_typed_property(offset.clone()),
            )])
        }));
        Rc::new(Self {
            base: BaseInstance::new(
                arguments,
                InstanceFlags {
                    layer: Layer::DontCare,
                    invisible_to_slot: false,
                    invisible_to_raycasting: false,
                    is_component: false,
                    is_slot: false,
                },
            ),
            bounded,
            clip,
            scroll: false,
            scroll_y,
            tests,
            shape: RefCell::new(shape),
        })
    }
}
impl InstanceNode for HitNode {
    fn instantiate(_: InstantiationArgs) -> Rc<Self> {
        unreachable!()
    }
    fn base(&self) -> &BaseInstance {
        &self.base
    }
    fn geometry_is_scroller_local(&self) -> bool {
        true
    }
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("HitNode").finish()
    }
    fn clips_content(&self, _: &ExpandedNode) -> bool {
        self.clip
    }
    fn accepts_transferred_children(&self) -> bool {
        true
    }
    fn scrolls_content(&self, _: &ExpandedNode) -> bool {
        self.scroll
    }
    fn resolve_scroll_offset(&self, _: &ExpandedNode) -> Option<(f64, f64)> {
        self.scroll.then(|| (0., self.scroll_y.get()))
    }
    fn property_requires_input_recompute(&self, name: &str) -> bool {
        self.scroll && name == "scroll_y"
    }
    fn property_requires_clip_recompute(&self, _: &str) -> bool {
        false
    }
    fn resolve_local_clip_path(&self, node: &ExpandedNode) -> Option<kurbo::BezPath> {
        use kurbo::Shape;
        let (w, h) = node.layout_bounds.get();
        self.clip.then(|| Rect::new(0., 0., w, h).to_path(0.1))
    }

    fn prepare_hit_shape(&self, node: &ExpandedNode) -> pax_runtime_api::HitShape {
        self.tests.set(self.tests.get() + 1);
        let (w, h) = node.presentation_transform_and_bounds.get().bounds;
        let mut rect = self.shape.borrow().unwrap_or(Rect::new(0., 0., w, h));
        if !self.bounded {
            rect.x0 = f64::NEG_INFINITY;
        }
        pax_runtime_api::HitShape::Rect(rect)
    }
}

fn fixture(children: Vec<Rc<dyn InstanceNode>>) -> (PaxEngine, Rc<ExpandedNode>) {
    let mut engine = crate::test_support::empty_engine(
        (800., 600.),
        Platform::Web,
        OS::Mac,
        Box::new(|| 0),
        Default::default(),
    );
    let mut root_args = args(vec![]);
    root_args.component_template = Some(RefCell::new(children));
    let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
    engine.tick();
    (engine, root)
}
fn place(node: &ExpandedNode, x: f64, y: f64, w: f64, h: f64) {
    let props = node.get_common_properties().borrow().clone();
    props.x.set(Some(Size::Pixels(x.into())));
    props.y.set(Some(Size::Pixels(y.into())));
    props.width.set(Some(Size::Pixels(w.into())));
    props.height.set(Some(Size::Pixels(h.into())));
}
fn query(
    ctx: &RuntimeContext,
    root: Option<Rc<ExpandedNode>>,
    point: (f64, f64),
    invisible: bool,
) -> Vec<ExpandedNodeIdentifier> {
    ctx.get_elements_beneath_ray(
        root,
        Point2::new(point.0, point.1),
        false,
        vec![],
        invisible,
    )
    .iter()
    .map(|node| node.id)
    .collect()
}
fn oracle(
    ctx: &RuntimeContext,
    root: Option<Rc<ExpandedNode>>,
    point: (f64, f64),
    invisible: bool,
) -> Vec<ExpandedNodeIdentifier> {
    assert!(!ctx.input_query_unindexed.replace(true));
    let result = query(ctx, root, point, invisible);
    ctx.input_query_unindexed.set(false);
    result
}

fn native_scroll(ctx: &RuntimeContext, node: &ExpandedNode, y: f64) {
    ctx.set_scroller_surface_state(
        node.id.to_u32(),
        crate::ScrollerSurfaceState {
            viewport_width: 600.,
            viewport_height: 500.,
            content_width: 1200.,
            content_height: 200_000.,
            scroll_y: y,
            presentation_scroll_y: y,
            clip_content: true,
            ..Default::default()
        },
    );
}

#[test]
fn native_scroll_reuses_descendant_input_bounds_at_both_collection_sizes() {
    for count in [32, 1024] {
        let calls = Rc::new(Cell::new(0));
        let leaves = (0..count)
            .map(|_| HitNode::new(vec![], true, false, None, calls.clone()) as Rc<dyn InstanceNode>)
            .collect();
        let mut host = HitNode::new(leaves, true, true, None, calls);
        Rc::get_mut(&mut host).unwrap().scroll = true;
        let (mut engine, root) = fixture(vec![host]);
        let host = root.children.get()[0].clone();
        let leaves = host.children.get();
        place(&host, 0., 0., 600., 500.);
        for (i, leaf) in leaves.iter().enumerate() {
            place(leaf, 10., i as f64 * 96., 60., 60.);
        }
        native_scroll(&engine.runtime_context, &host, 0.);
        engine.tick();
        query(&engine.runtime_context, None, (20., 20.), false);
        let before = engine.runtime_context.input_query_stats();
        for i in [5, 1, 20, 3, 0] {
            native_scroll(&engine.runtime_context, &host, i as f64 * 96.);
            // The pre-settlement fallback must still observe the new offset.
            let ctx = &engine.runtime_context;
            assert_eq!(
                query(ctx, None, (20., 20.), false),
                oracle(ctx, None, (20., 20.), false)
            );
            engine.tick();
            let ctx = &engine.runtime_context;
            let result = query(ctx, None, (20., 20.), false);
            assert!(result.contains(&leaves[i].id));
            assert_eq!(result, oracle(ctx, None, (20., 20.), false));
            assert_eq!(
                ctx.input_query_stats().bounds_prepared,
                before.bounds_prepared,
                "native scroll should not rebuild {count} descendant footprints"
            );
            assert_eq!(
                ctx.input_query_stats().topology_rebuilds,
                before.topology_rebuilds
            );
        }
    }
}

#[test]
fn nested_rotated_scroll_charts_preserve_clip_escape_and_current_geometry() {
    let calls = Rc::new(Cell::new(0));
    let leaf = HitNode::new(vec![], true, false, None, calls.clone());
    let mut inner = HitNode::new(vec![leaf], true, true, None, calls.clone());
    Rc::get_mut(&mut inner).unwrap().scroll = true;
    let mut outer = HitNode::new(vec![inner], true, true, None, calls);
    Rc::get_mut(&mut outer).unwrap().scroll = true;
    let (mut engine, root) = fixture(vec![outer]);
    let outer = root.children.get()[0].clone();
    let inner = outer.children.get()[0].clone();
    let leaf = inner.children.get()[0].clone();
    place(&outer, 100., 100., 600., 500.);
    place(&inner, 30., 50., 300., 200.);
    place(&leaf, 80., 180., 60., 60.);
    outer
        .get_common_properties()
        .borrow()
        .rotate
        .set(Some(pax_runtime_api::Rotation::Degrees(17.into())));
    inner
        .get_common_properties()
        .borrow()
        .scale_x
        .set(Some(Size::Percent(75.into())));
    native_scroll(&engine.runtime_context, &outer, 0.);
    native_scroll(&engine.runtime_context, &inner, 0.);
    engine.tick();
    query(&engine.runtime_context, None, (20., 20.), false);
    let before = engine.runtime_context.input_query_stats().bounds_prepared;
    for (a, b) in [(30., 70.), (0., 0.), (160., 210.), (10., 90.)] {
        native_scroll(&engine.runtime_context, &outer, a);
        native_scroll(&engine.runtime_context, &inner, b);
        engine.tick();
        let ctx = &engine.runtime_context;
        for x in (0..800).step_by(31) {
            for y in (0..600).step_by(37) {
                let p = (x as f64, y as f64);
                assert_eq!(
                    query(ctx, None, p, false),
                    oracle(ctx, None, p, false),
                    "{a},{b} at {p:?}"
                );
            }
        }
        assert_eq!(ctx.input_query_stats().bounds_prepared, before);
    }
    // Host motion uses the same local chart as scrolling, including rotated
    // ancestors. Compare exact results to uncached world-space preparation.
    for (x, angle) in [(60., -12.), (140., 30.)] {
        outer
            .get_common_properties()
            .borrow()
            .x
            .set(Some(Size::Pixels(x.into())));
        outer
            .get_common_properties()
            .borrow()
            .rotate
            .set(Some(pax_runtime_api::Rotation::Degrees(angle.into())));
        engine.tick();
        let ctx = &engine.runtime_context;
        let before_move = ctx.input_query_stats().bounds_prepared;
        for x in (0..800).step_by(31) {
            for y in (0..600).step_by(37) {
                let point = (x as f64, y as f64);
                assert_eq!(
                    query(ctx, None, point, false),
                    oracle(ctx, None, point, false)
                );
            }
        }
        assert!(ctx.input_query_stats().bounds_prepared - before_move <= 1);
    }
    leaf.get_common_properties()
        .borrow()
        .unclippable
        .set(Some(true));
    place(&leaf, 80., 400., 60., 60.);
    engine.tick();
    let ctx = &engine.runtime_context;
    let world = ctx.presentation_scroll_transform_for_node(&leaf)
        * Affine::from(leaf.presentation_transform_and_bounds.get().transform);
    let p = world * Point::new(20., 20.);
    let result = query(ctx, None, (p.x, p.y), false);
    assert!(result.contains(&leaf.id));
    assert_eq!(result, oracle(ctx, None, (p.x, p.y), false));
    assert!(
        ctx.input_query_stats().bounds_prepared > before,
        "real geometry still invalidates"
    );
}

#[test]
fn input_charts_follow_reparenting_and_scoped_query_roots() {
    let calls = Rc::new(Cell::new(0));
    let leaf = HitNode::new(vec![], true, false, None, calls.clone());
    let mut a = HitNode::new(vec![leaf], true, true, None, calls.clone());
    Rc::get_mut(&mut a).unwrap().scroll = true;
    let mut b = HitNode::new(vec![], true, true, None, calls);
    Rc::get_mut(&mut b).unwrap().scroll = true;
    let (mut engine, root) = fixture(vec![a, b]);
    let a = root.children.get()[0].clone();
    let b = root.children.get()[1].clone();
    let leaf = a.children.get()[0].clone();
    place(&a, 0., 0., 200., 300.);
    place(&b, 300., 0., 200., 300.);
    place(&leaf, 20., 100., 50., 50.);
    native_scroll(&engine.runtime_context, &a, 30.);
    native_scroll(&engine.runtime_context, &b, 60.);
    engine.tick();
    assert!(query(&engine.runtime_context, None, (30., 80.), false).contains(&leaf.id));
    crate::NodeRef::new(&leaf, &engine.runtime_context)
        .move_to_append_child(&crate::NodeRef::new(&b, &engine.runtime_context))
        .unwrap();
    engine.tick();
    let ctx = &engine.runtime_context;
    for root in [None, Some(b.clone()), Some(leaf.clone()), None] {
        for p in [(30., 80.), (330., 50.), (330., 110.)] {
            assert_eq!(
                query(ctx, root.clone(), p, false),
                oracle(ctx, root.clone(), p, false)
            );
        }
    }
    assert!(query(ctx, None, (330., 50.), false).contains(&leaf.id));
    leaf.clone().recurse_unmount(ctx);
    assert!(!query(ctx, None, (330., 50.), false).contains(&leaf.id));
}

#[cfg(feature = "3d")]
#[test]
fn perspective_input_reprojects_on_scroll_and_matches_uncached_queries() {
    use pax_manifest::{
        ExpressionInfo, LiteralBlockDefinition, SettingElement, Token, ValueDefinition,
    };
    let calls = Rc::new(Cell::new(0));
    let leaf = HitNode::new(vec![], true, false, None, calls.clone());
    let mut host = HitNode::new(vec![leaf], true, true, None, calls);
    Rc::get_mut(&mut host).unwrap().scroll = true;
    let mut root_args = args(vec![]);
    root_args.component_template = Some(RefCell::new(vec![host]));
    root_args.component_camera = Some(LiteralBlockDefinition::new(
        [("projection", "CameraMode::Perspective"), ("z", "600px")]
            .into_iter()
            .map(|(key, value)| {
                SettingElement::Setting(
                    Token::new_without_location(key.into()),
                    ValueDefinition::Expression(ExpressionInfo::new(
                        pax_language::parse_pax_expression(value).unwrap(),
                    )),
                )
            })
            .collect(),
    ));
    let mut engine = crate::test_support::empty_engine(
        (800., 600.),
        Platform::Web,
        OS::Mac,
        Box::new(|| 0),
        Default::default(),
    );
    let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
    engine.tick();
    let host = root.children.get()[0].clone();
    let leaf = host.children.get()[0].clone();
    place(&host, 100., 100., 400., 300.);
    place(&leaf, 50., 100., 120., 120.);
    leaf.get_common_properties()
        .borrow()
        .z
        .set(Some(100.0.into()));
    native_scroll(&engine.runtime_context, &host, 0.);
    engine.tick();
    query(&engine.runtime_context, None, (200., 200.), false);
    let mut before = engine.runtime_context.input_query_stats().bounds_prepared;
    for offset in [30., 60., 0.] {
        native_scroll(&engine.runtime_context, &host, offset);
        engine.tick();
        let ctx = &engine.runtime_context;
        for x in (50..700).step_by(43) {
            for y in (50..500).step_by(41) {
                let p = (x as f64, y as f64);
                assert_eq!(
                    query(ctx, None, p, false),
                    oracle(ctx, None, p, false),
                    "scroll={offset} at {p:?}"
                );
            }
        }
        let after = ctx.input_query_stats().bounds_prepared;
        assert!(
            after > before,
            "perspective geometry cannot reuse an affine scroll chart"
        );
        before = after;
    }
}

#[test]
fn bounded_layout_only_nodes_scale_by_candidates_and_reuse_preparation() {
    for count in [32, 1024] {
        let tests = Rc::new(Cell::new(0));
        let children = (0..count)
            .map(|_| HitNode::new(vec![], true, false, None, tests.clone()) as Rc<dyn InstanceNode>)
            .collect();
        let (mut engine, root) = fixture(children);
        let nodes = root.children.get();
        for (i, node) in nodes.iter().enumerate() {
            place(node, i as f64 * 512., 0., 64., 64.);
        }
        engine.tick();
        let ctx = &engine.runtime_context;
        let result = query(ctx, None, (20., 20.), false);
        assert_eq!(result, [nodes[0].id]);
        assert_eq!(tests.get(), count, "prepare each shape once");
        let first = ctx.input_query_stats();
        assert_eq!(first.indexed_queries, 1);
        assert_eq!(first.exact_tests, 1, "only the local interactive candidate");
        assert_eq!(query(ctx, None, (21., 22.), false), result);
        let second = ctx.input_query_stats();
        assert_eq!(second.bounds_prepared, first.bounds_prepared);
        assert_eq!(
            tests.get(),
            count,
            "unchanged queries reuse prepared values"
        );
        assert_eq!(second.topology_rebuilds, first.topology_rebuilds);
        assert_eq!(oracle(ctx, None, (20., 20.), false), result);
        nodes[0]
            .get_common_properties()
            .borrow()
            ._raycastable
            .set(Some(false));
        assert!(query(ctx, None, (20., 20.), false).is_empty());
        nodes[0]
            .get_common_properties()
            .borrow()
            ._raycastable
            .set(Some(true));
        place(&nodes[1], 0., 0., 64., 64.);
        assert_eq!(
            query(ctx, None, (20., 20.), false),
            oracle(ctx, None, (20., 20.), false)
        );
        assert!(ctx.input_query_stats().pending_queries > 0);
        engine.tick();
        let ctx = &engine.runtime_context;
        assert_eq!(
            query(ctx, None, (20., 20.), false),
            [nodes[0].id, nodes[1].id]
        );
        assert_eq!(
            ctx.input_query_stats().topology_rebuilds,
            first.topology_rebuilds
        );
    }
}

#[test]
fn unbounded_shapes_use_the_same_index_and_skip_unrelated_exact_tests() {
    let calls = Rc::new(Cell::new(0));
    let unbounded = HitNode::new(
        vec![],
        false,
        false,
        Some(Rect::new(-200., -200., 0., 0.)),
        calls.clone(),
    );
    let known_calls = Rc::new(Cell::new(0));
    let mut children: Vec<Rc<dyn InstanceNode>> = vec![unbounded];
    children.extend((0..128).map(|_| {
        HitNode::new(vec![], true, false, None, known_calls.clone()) as Rc<dyn InstanceNode>
    }));
    let (mut engine, root) = fixture(children);
    let nodes = root.children.get();
    place(&nodes[0], 100., 100., 20., 20.);
    for node in &nodes[1..] {
        place(node, 2000., 2000., 20., 20.);
    }
    engine.tick();
    let ctx = &engine.runtime_context;
    assert_eq!(query(ctx, None, (10., 10.), false), [nodes[0].id]);
    assert_eq!(calls.get(), 1);
    assert_eq!(known_calls.get(), 128);
    assert_eq!(ctx.input_query_stats().indexed_queries, 1);
    assert_eq!(ctx.input_query_stats().exact_tests, 1);
    assert_eq!(ctx.input_query_stats().unknown_tests, 1);
    assert_eq!(
        query(ctx, None, (10., 10.), false),
        oracle(ctx, None, (10., 10.), false)
    );
}

#[test]
fn custom_shape_invalidation_and_unmount_retire_prepared_values() {
    let preparations = Rc::new(Cell::new(0));
    let instance = HitNode::new(
        vec![],
        true,
        false,
        Some(Rect::new(-100., -100., 0., 0.)),
        preparations.clone(),
    );
    let (mut engine, root) = fixture(vec![instance.clone()]);
    let node = root.children.get()[0].clone();
    place(&node, 100., 100., 20., 20.);
    engine.tick();
    let ctx = &engine.runtime_context;
    assert_eq!(query(ctx, None, (10., 10.), false), [node.id]);
    *instance.shape.borrow_mut() = Some(Rect::new(0., 0., 20., 20.));
    ctx.mark_node_input_dirty(node.id.to_u32());
    assert!(query(ctx, None, (10., 10.), false).is_empty());
    assert_eq!(query(ctx, None, (110., 110.), false), [node.id]);
    assert_eq!(preparations.get(), 2);
    node.clone().recurse_unmount(ctx);
    assert!(!ctx
        .scene_geometry
        .borrow()
        .input
        .records
        .contains_key(&node.id.to_u32()));
}

#[test]
fn indexed_clip_escape_subtree_scope_and_exit_policy_match_traversal() {
    let calls = Rc::new(Cell::new(0));
    let child = HitNode::new(vec![], true, false, None, calls.clone());
    let parent = HitNode::new(vec![child], true, true, None, calls.clone());
    let (mut engine, root) = fixture(vec![parent]);
    let parent = root.children.get()[0].clone();
    let child = parent.children.get()[0].clone();
    place(&parent, 0., 0., 80., 80.);
    place(&child, 100., 0., 50., 50.);
    engine.tick();
    let ctx = &engine.runtime_context;
    assert!(query(ctx, None, (120., 20.), false).is_empty());
    assert_eq!(
        query(ctx, Some(child.clone()), (120., 20.), false),
        [child.id]
    );
    child
        .get_common_properties()
        .borrow()
        .unclippable
        .set(Some(true));
    engine.tick();
    let ctx = &engine.runtime_context;
    for point in [
        (20., 20.),
        (80., 20.),
        (100., 20.),
        (120., 20.),
        (150., 20.),
    ] {
        for invisible in [false, true] {
            assert_eq!(
                query(ctx, None, point, invisible),
                oracle(ctx, None, point, invisible)
            );
        }
    }
    child.transition_phase.set(TRANSITION_PHASE_EXIT);
    assert!(query(ctx, None, (120., 20.), false).is_empty());
    assert!(query(ctx, None, (120., 20.), true).contains(&child.id));
}

#[test]
fn presentation_lineages_and_clip_shapes_share_work_and_observe_native_scroll_immediately() {
    let calls = Rc::new(Cell::new(0));
    let leaves = (0..64)
        .map(|_| HitNode::new(vec![], true, false, None, calls.clone()) as Rc<dyn InstanceNode>)
        .collect();
    let mut inner = HitNode::new(leaves, true, true, None, calls.clone());
    Rc::get_mut(&mut inner).unwrap().scroll = true;
    let mut outer = HitNode::new(vec![inner], true, true, None, calls);
    Rc::get_mut(&mut outer).unwrap().scroll = true;
    let (mut engine, root) = fixture(vec![outer]);
    let outer = root.children.get()[0].clone();
    let inner = outer.children.get()[0].clone();
    let leaves = inner.children.get();
    place(&outer, 0., 0., 600., 500.);
    place(&inner, 0., 0., 500., 400.);
    for leaf in &leaves {
        place(leaf, 50., 70., 20., 20.);
    }
    engine.tick();
    let ctx = &engine.runtime_context;
    let read_all = || {
        for leaf in &leaves {
            ctx.presented_projection_for_node(leaf);
            ctx.presentation_scroll_transform_for_node(leaf);
            ctx.presented_clip_path(&inner);
        }
        ctx.occlusion_stats()
    };
    let first = read_all();
    let clip = inner.local_clip_geometry.get().0.unwrap();
    outer
        .get_common_properties()
        .borrow()
        .opacity
        .set(Some(0.4.into()));
    let painted = read_all();
    assert_eq!(
        first.scroll_lineage_preparations,
        painted.scroll_lineage_preparations
    );
    assert_eq!(
        first.clip_geometry_preparations,
        painted.clip_geometry_preparations
    );
    assert_eq!(
        first.projection_preparations,
        painted.projection_preparations
    );
    assert!(Rc::ptr_eq(
        &clip,
        &inner.local_clip_geometry.get().0.unwrap()
    ));
    ctx.set_scroller_surface_state(
        outer.id.to_u32(),
        crate::ScrollerSurfaceState {
            viewport_width: 600.,
            viewport_height: 500.,
            content_width: 1000.,
            content_height: 1000.,
            scroll_y: 30.,
            presentation_scroll_y: 30.,
            clip_content: true,
            ..Default::default()
        },
    );
    let scrolled = read_all();
    assert_eq!(
        scrolled.scroll_lineage_preparations - painted.scroll_lineage_preparations,
        2,
        "compose the two scrollers once, not once per leaf"
    );
    assert_eq!(
        scrolled.clip_geometry_preparations,
        painted.clip_geometry_preparations
    );
    assert_eq!(
        ctx.presentation_scroll_transform_for_node(&leaves[0]) * Point::ZERO,
        Point::new(0., -30.)
    );
    let p = ctx
        .presented_projection_for_node(&leaves[0])
        .unwrap()
        .mapping()
        .project(Point::ZERO)
        .unwrap();
    assert!(
        (p.y - 40.).abs() < 1e-9,
        "native scroll is visible before effects are drained: {p:?}"
    );
    let again = read_all();
    assert_eq!(
        again.scroll_lineage_preparations,
        scrolled.scroll_lineage_preparations
    );
    assert_eq!(
        again.projection_preparations,
        scrolled.projection_preparations
    );
    assert_eq!(
        again.clip_geometry_preparations,
        scrolled.clip_geometry_preparations
    );
    inner
        .get_common_properties()
        .borrow()
        .width
        .set(Some(Size::Pixels(300.into())));
    read_all();
    assert!(!Rc::ptr_eq(
        &clip,
        &inner.local_clip_geometry.get().0.unwrap()
    ));
    ctx.mark_node_clip_dirty(inner.id.to_u32());
    let before = ctx.occlusion_stats().clip_geometry_preparations;
    read_all();
    assert_eq!(ctx.occlusion_stats().clip_geometry_preparations, before + 1);
}

#[test]
fn retained_scroll_follows_reactive_writes_page_delegation_and_reparenting() {
    use crate::VisualViewportState;
    let calls = Rc::new(Cell::new(0));
    let leaf = HitNode::new(vec![], true, false, None, calls.clone());
    let mut a = HitNode::new(vec![leaf], true, true, None, calls.clone());
    Rc::get_mut(&mut a).unwrap().scroll = true;
    let offset_a = a.scroll_y.clone();
    let mut b = HitNode::new(vec![], true, true, None, calls);
    Rc::get_mut(&mut b).unwrap().scroll = true;
    let offset_b = b.scroll_y.clone();
    let (mut engine, root) = fixture(vec![a, b]);
    let a = root.children.get()[0].clone();
    let b = root.children.get()[1].clone();
    let leaf = a.children.get()[0].clone();
    place(&a, 0., 0., 400., 300.);
    place(&b, 0., 0., 400., 300.);
    place(&leaf, 20., 100., 50., 50.);
    engine.tick();
    let ctx = &engine.runtime_context;
    let y = || {
        ctx.presented_projection_for_node(&leaf)
            .unwrap()
            .mapping()
            .project(Point::ZERO)
            .unwrap()
            .y
    };
    assert_eq!(y(), 100.);
    offset_a.set(30.);
    assert_eq!(
        y(),
        70.,
        "reactive writes are observed before draining effects"
    );
    assert!(query(ctx, None, (30., 80.), false).contains(&leaf.id));
    ctx.set_root_scroller_id(Some(a.id.to_u32()));
    ctx.set_visual_viewport_state(VisualViewportState {
        width: 800.,
        height: 600.,
        offset_x: 0.,
        offset_y: 0.,
        page_scroll_x: 0.,
        page_scroll_y: 30.,
    });
    assert_eq!(y(), 100., "page delegation must not apply scroll twice");
    ctx.clear_visual_viewport_state();
    assert_eq!(y(), 70.);
    ctx.set_root_scroller_id(None);
    offset_b.set(60.);
    b.attach_children(vec![leaf.clone()], ctx, &b.parent_frame);
    assert_eq!(y(), 40.);
    offset_a.set(90.);
    assert_eq!(y(), 40., "the former parent no longer supplies placement");
    offset_b.set(20.);
    assert_eq!(y(), 80.);
    #[cfg(feature = "3d")]
    let prepared = Rc::downgrade(&ctx.prepared_presentation(&leaf));
    leaf.clone().recurse_unmount(ctx);
    assert_eq!(leaf.scroll_state.get(), Default::default());
    assert!(leaf.local_clip_geometry.get().0.is_none());
    assert!(leaf.world_clip_geometry.get().0.is_none());
    #[cfg(feature = "3d")]
    assert!(
        prepared.upgrade().is_none(),
        "retired mounts release their projection"
    );
}
