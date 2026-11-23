use super::*;
use crate::{
    BaseInstance, CommonPropertiesInit, ComponentInstance, InstanceFlags, InstanceNode,
    InstantiationArgs, PropertiesInit, PropertiesScopeInit,
};
use pax_runtime_api::{
    pax_value::{PaxAny, PaxValue},
    Platform, Size, OS,
};
use std::cell::RefCell;

#[derive(Clone, Copy)]
enum Kind {
    Canvas,
    Native,
    Group,
    ResponsiveGroup,
    Clip,
    Scroller,
    BeforeNative,
}

struct Probe {
    base: BaseInstance,
    kind: Kind,
    retained: bool,
    placement_checks: std::cell::Cell<usize>,
}

fn args(children: Vec<Rc<dyn InstanceNode>>) -> InstantiationArgs {
    InstantiationArgs {
        prototypical_common_properties: CommonPropertiesInit::Default,
        prototypical_properties: PropertiesInit::Factory(Box::new(|_, existing| {
            existing
                .is_none()
                .then(|| Rc::new(RefCell::new(PaxAny::Builtin(PaxValue::default()))))
        })),
        children: Some(RefCell::new(children)),
        component_template: None,
        handler_registry: None,
        component_settings: None,
        component_camera: None,
        template_node_identifier: None,
        template_node_type_id: None,
        template_node_selector_info: None,
        transition_config: Default::default(),
        properties_scope: PropertiesScopeInit::None,
    }
}

fn probe(kind: Kind, children: Vec<Rc<dyn InstanceNode>>) -> Rc<dyn InstanceNode> {
    make_probe(kind, children, false)
}

fn retained_probe(kind: Kind, children: Vec<Rc<dyn InstanceNode>>) -> Rc<dyn InstanceNode> {
    make_probe(kind, children, true)
}

fn make_probe(
    kind: Kind,
    children: Vec<Rc<dyn InstanceNode>>,
    retained: bool,
) -> Rc<Probe> {
    Rc::new(Probe {
        kind,
        retained,
        placement_checks: Default::default(),
        base: BaseInstance::new(
            args(children),
            InstanceFlags {
                invisible_to_slot: false,
                invisible_to_raycasting: true,
                is_component: false,
                is_slot: false,
                layer: match kind {
                    Kind::Canvas => Layer::Canvas,
                    Kind::Native | Kind::Scroller | Kind::BeforeNative => Layer::Native,
                    _ => Layer::DontCare,
                },
            },
        ),
    })
}

impl InstanceNode for Probe {
    fn instantiate(_: InstantiationArgs) -> Rc<Self> {
        unreachable!()
    }
    fn base(&self) -> &BaseInstance {
        &self.base
    }
    fn geometry_is_scroller_local(&self) -> bool {
        self.placement_checks.set(self.placement_checks.get() + 1);
        self.retained
    }
    fn canvas_geometry_is_local(&self) -> bool {
        self.retained
    }
    fn resolve_scroll_offset(&self, _: &ExpandedNode) -> Option<(f64, f64)> {
        (self.retained && matches!(self.kind, Kind::Scroller)).then_some((0., 0.))
    }
    fn resolve_local_occlusion_path(&self, _: &ExpandedNode) -> Option<BezPath> {
        (self.retained && matches!(self.kind, Kind::Canvas))
            .then(|| Rect::new(0., 0., 40., 40.).to_path(0.1))
    }
    fn resolve_local_clip_path(&self, node: &ExpandedNode) -> Option<BezPath> {
        (self.retained && matches!(self.kind, Kind::Clip | Kind::Scroller)).then(|| {
            let (w, h) = if matches!(self.kind, Kind::Scroller) {
                node.layout_bounds.get()
            } else {
                (60., 60.)
            };
            kurbo::RoundedRect::new(0., 0., w, h, 8.).to_path(0.1)
        })
    }
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Probe").finish()
    }
    fn scrolls_content(&self, _: &ExpandedNode) -> bool {
        matches!(self.kind, Kind::Scroller)
    }
    fn materializes_native_surface(&self, node: &ExpandedNode) -> bool {
        matches!(self.kind, Kind::BeforeNative)
            || (matches!(self.kind, Kind::ResponsiveGroup) && node.layout_bounds.get().0 < 80.)
    }
    fn materializes_native_surface_before_children(&self, _: &ExpandedNode) -> bool {
        matches!(self.kind, Kind::BeforeNative)
    }
    fn clips_content(&self, _: &ExpandedNode) -> bool {
        matches!(self.kind, Kind::Scroller | Kind::Clip)
    }
    fn accepts_transferred_children(&self) -> bool {
        matches!(self.kind, Kind::Group | Kind::Clip | Kind::Scroller)
    }
    fn resolve_occlusion_path(&self, node: &ExpandedNode) -> Option<BezPath> {
        matches!(self.kind, Kind::Canvas).then(|| {
            Affine::from(node.transform_and_bounds.get().transform)
                * Rect::new(0., 0., 40., 40.).to_path(0.1)
        })
    }
    fn resolve_effect_clip_path(&self, node: &ExpandedNode) -> Option<BezPath> {
        let clips = matches!(self.kind, Kind::Clip | Kind::Scroller)
            || (matches!(self.kind, Kind::ResponsiveGroup)
                && (80.0..160.0).contains(&node.layout_bounds.get().0));
        clips.then(|| {
            let bounds = if matches!(self.kind, Kind::Scroller) {
                node.transform_and_bounds.get().bounds
            } else {
                (60., 60.)
            };
            Affine::from(node.transform_and_bounds.get().transform)
                * kurbo::RoundedRect::new(0., 0., bounds.0, bounds.1, 8.).to_path(0.1)
        })
    }
}

fn mount(children: Vec<Rc<dyn InstanceNode>>) -> (Rc<RuntimeContext>, Rc<ExpandedNode>) {
    use pax_runtime_api::{LocalProperty as Property, TargetInfo};
    let ctx = Rc::new(crate::test_support::runtime_context(crate::Globals {
        elapsed_frames: Property::new(0),
        elapsed_millis: Property::new(0),
        viewport: Property::new(TransformAndBounds {
            transform: Default::default(),
            bounds: (300., 200.),
        }),
        gyro: Property::new(Default::default()),
        accel: Property::new(Default::default()),
        route_location: Property::new(crate::RouteLocation::root()),
        browser_allows_scroller_vector_layers: Property::new(true),
        browser_allows_nested_scroller_vector_layers: Property::new(true),
        renderer_is_gpu: true,
        platform: Platform::Unknown,
        os: OS::Unknown,
        target: TargetInfo::new(Platform::Unknown, OS::Unknown),
        #[cfg(feature = "designtime")]
        designtime: crate::test_support::designtime(),
        get_elapsed_millis: Rc::new(|| 0),
    }));
    let mut root_args = args(Vec::new());
    root_args.children = None;
    root_args.component_template = Some(RefCell::new(vec![probe(Kind::Group, children)]));
    let root = ExpandedNode::initialize_root(ComponentInstance::instantiate(root_args), &ctx);
    settle(&root, &ctx);
    ctx.dirty_canvases
        .borrow_mut()
        .resize(ctx.layer_count.get(), false);
    clear(&ctx);
    (ctx, root)
}

fn settle(root: &Rc<ExpandedNode>, ctx: &Rc<RuntimeContext>) {
    root.recurse_update(ctx);
    update_node_occlusion(root, ctx);
}

fn settle_occlusion_effects(root: &Rc<ExpandedNode>, ctx: &RuntimeContext) {
    for _ in 0..16 {
        ctx.drain_node_effects();
        if !ctx.take_occlusion_dirty() {
            return;
        }
        update_node_occlusion(root, ctx);
    }
    panic!("scroller insertion did not settle");
}

fn clear(ctx: &RuntimeContext) {
    ctx.clear_all_dirty_canvases();
    for id in ctx.dirty_canvas_node_ids() {
        ctx.clear_canvas_node_dirty(&id);
    }
    ctx.take_canvas_node_orders();
    ctx.take_native_messages();
}

#[test]
fn viewport_canvas_motion_keeps_invalidating_retained_paint() {
    use pax_runtime_api::LocalProperty;
    use std::cell::Cell;

    let (ctx, root) = mount(vec![retained_probe(Kind::Canvas, vec![])]);
    let parent = root.children.get()[0].clone();
    let node = parent.children.get()[0].clone();
    assert!(node.canvas_owner.get().is_none());
    let paints = Rc::new(Cell::new(0));
    let count = paints.clone();
    node.changed_listener.replace_with(LocalProperty::computed(
        move || count.set(count.get() + 1),
        &[node.canvas_transform_and_bounds.untyped()],
    ));
    ctx.register_expanded_node_effect_property_named(&node, &node.changed_listener, "paint");
    ctx.drain_node_effects();
    ctx.canvas_surface_transform_for_node(&node);

    for y in [-80., -60., -20., 0., 20., 0., -80.] {
        let previous = paints.get();
        parent
            .get_common_properties()
            .borrow()
            .y
            .set(Some(Size::Pixels(y.into())));
        ctx.drain_node_effects();
        assert!(
            paints.get() > previous,
            "movement to {y} must invalidate paint"
        );
        let transform = ctx.canvas_surface_transform_for_node(&node);
        assert_eq!(transform.as_coeffs()[5], y);
    }
}

#[test]
fn fallback_paint_tracks_the_assigned_surface_when_world_geometry_stays_fixed() {
    let (ctx, root) = mount(vec![
        probe(Kind::Group, vec![]),
        probe(Kind::Group, vec![]),
        retained_probe(Kind::Canvas, vec![]),
    ]);
    let nodes = root.children.get()[0].children.get();
    let (first, second, leaf) = (&nodes[0], &nodes[1], &nodes[2]);
    let set_x = |n: &ExpandedNode, x: f64| {
        n.get_common_properties()
            .borrow()
            .x
            .set(Some(Size::Pixels(x.into())));
    };
    set_x(first, 15.);
    set_x(second, 15.);
    set_x(leaf, 80.);
    ctx.register_layer_scroller_owner(1, first.id);
    ctx.register_layer_scroller_owner(2, second.id);
    let assign = |layer| {
        let mut order = leaf.occlusion.get();
        order.render_layer_id = layer;
        assign_occlusion(leaf, order, Layer::Canvas, &ctx, false);
    };
    let check = |owner: Option<&ExpandedNode>| {
        let projected = leaf.presentation_transform_and_bounds.get();
        let actual_owner = ctx.get_layer_scroller_owner(leaf.occlusion.get().render_layer_id);
        assert_eq!(actual_owner, owner.map(|n| n.id));
        assert_eq!(leaf.canvas_owner.get(), actual_owner.map(|n| n.to_u32()));
        let inverse = owner.map_or(Affine::IDENTITY, |owner| {
            Affine::from(owner.presentation_transform_and_bounds.get().transform).inverse()
        });
        let expected = inverse * Affine::from(projected.transform);
        assert_eq!(ctx.canvas_surface_transform_for_node(leaf), expected);
        assert_eq!(
            Affine::from(leaf.canvas_transform_and_bounds.get().transform),
            expected
        );
    };
    ctx.drain_node_effects();
    check(None);
    let paints = std::rc::Rc::new(std::cell::Cell::new(0));
    let count = paints.clone();
    leaf.changed_listener
        .replace_with(pax_runtime_api::LocalProperty::computed(
            move || count.set(count.get() + 1),
            &[leaf.canvas_transform_and_bounds.untyped()],
        ));
    ctx.register_expanded_node_effect_property_named(leaf, &leaf.changed_listener, "paint");
    ctx.drain_node_effects();
    paints.set(0);
    leaf.presentation_transform_and_bounds.invalidate();
    ctx.drain_node_effects();
    assert!(paints.get() == 0, "identical flat placement retains paint");

    assign(1);
    ctx.drain_node_effects();
    check(Some(first));
    assert_eq!(paints.replace(0), 1);
    ctx.take_canvas_node_removals();
    assign(2);
    ctx.drain_node_effects();
    check(Some(second));
    assert_eq!(
        paints.replace(0),
        1,
        "a different surface must receive its paint"
    );
    assert!(ctx
        .take_canvas_node_removals()
        .contains(&(1, leaf.id.to_u32())));
    assert!(ctx.is_canvas_node_dirty(&leaf.id));

    set_x(first, 40.);
    ctx.drain_node_effects();
    check(Some(second));
    assert!(
        paints.get() == 0,
        "the retired surface cannot invalidate this leaf"
    );
    set_x(second, 35.);
    check(Some(second)); // geometry is readable before servicing queued paint
    ctx.drain_node_effects();
    assert_eq!(paints.replace(0), 1);
    // A structural rebuild may recycle the layer number for a different owner.
    clear(&ctx);
    ctx.register_layer_scroller_owner(2, first.id);
    assign(2);
    assert!(ctx.is_canvas_node_dirty(&leaf.id));
    ctx.drain_node_effects();
    check(Some(first));
    assign(0);
    ctx.drain_node_effects();
    check(None);
}

#[cfg(feature = "3d")]
#[test]
fn unchanged_xy_footprint_still_invalidates_paint_when_depth_changes() {
    let (ctx, root) = mount(vec![retained_probe(Kind::Canvas, vec![])]);
    let n = root.children.get()[0].children.get()[0].clone();
    let z = n.get_common_properties().borrow().z.clone();
    z.set(Some(10.0.into()));
    ctx.drain_node_effects();
    let before = n.canvas_transform_and_bounds.get();
    let paints = Rc::new(std::cell::Cell::new(0));
    let count = paints.clone();
    n.changed_listener
        .replace_with(pax_runtime_api::LocalProperty::computed(
            move || count.set(count.get() + 1),
            &[n.canvas_transform_and_bounds.untyped()],
        ));
    ctx.register_expanded_node_effect_property_named(&n, &n.changed_listener, "paint");
    ctx.drain_node_effects();
    paints.set(0);
    z.set(Some(20.0.into()));
    ctx.drain_node_effects();
    assert_eq!(n.canvas_transform_and_bounds.get(), before);
    assert_eq!(paints.get(), 1);
}

#[cfg(feature = "3d")]
#[test]
fn local_coverage_recovers_after_rebuilding_while_a_group_is_tilted() {
    use crate::scene_geometry::native_placement::PlacementDependency;
    use pax_runtime_api::properties::property_has_direct_outbound;
    let (ctx, root) = mount(vec![retained_probe(
        Kind::Scroller,
        vec![retained_probe(
            Kind::Group,
            vec![
                retained_probe(Kind::Canvas, vec![]),
                retained_probe(Kind::Native, vec![]),
            ],
        )],
    )]);
    let host = root.children.get()[0].children.get()[0].clone();
    let group = host.children.get()[0].clone();
    let children = group.children.get();
    let uses_local_coverage = |node: &ExpandedNode| {
        node.local_compositing.get()
            && node
                .retained_native_placement()
                .is_some_and(|(id, _)| id == host.id.to_u32())
    };
    settle_occlusion_effects(&root, &ctx);
    assert!(children.iter().all(|node| uses_local_coverage(node)));

    let leaf = &children[0];
    let dependency = leaf.retained_placement_dependency(leaf.local_compositing.clone());
    let world = leaf.presentation_geometry.untyped();
    let bounds = leaf.layout_bounds.untyped();
    let fallback = if leaf.motion_retained.get() {
        &bounds
    } else {
        &world
    };
    let observes_fallback = || property_has_direct_outbound(fallback, &dependency.untyped());
    let is_local =
        || matches!(dependency.get(), PlacementDependency::Local(id, _) if id == host.id.to_u32());
    assert!(is_local());
    assert!(!observes_fallback());

    let tilt = group.get_common_properties().borrow().rotate_y.clone();
    tilt.set(Some(pax_runtime_api::Rotation::Degrees(25.0.into())));
    settle_occlusion_effects(&root, &ctx);
    assert!(children.iter().all(|node| !uses_local_coverage(node)));
    assert!(!is_local());
    assert!(observes_fallback());
    assert_full_rebuild_matches(&root, &ctx);

    tilt.set(None);
    settle_occlusion_effects(&root, &ctx);
    assert!(children.iter().all(|node| uses_local_coverage(node)));
    assert!(is_local());
    assert!(!observes_fallback());
    assert_full_rebuild_matches(&root, &ctx);
}

#[test]
fn rounded_clip_follows_its_selected_path_across_scroller_transfers() {
    let (ctx, root) = mount(vec![
        retained_probe(
            Kind::Scroller,
            vec![
                retained_probe(Kind::Clip, vec![retained_probe(Kind::Canvas, vec![])]),
                retained_probe(Kind::Native, vec![]),
            ],
        ),
        retained_probe(Kind::Group, vec![]),
        retained_probe(
            Kind::Scroller,
            vec![
                retained_probe(Kind::Canvas, vec![]),
                retained_probe(Kind::Native, vec![]),
            ],
        ),
    ]);
    let parents = root.children.get()[0].children.get();
    let clip = parents[0].children.get()[0].clone();
    let check = |local: bool| {
        settle_occlusion_effects(&root, &ctx);
        assert_eq!(
            clip.local_compositing.get() && clip.retained_native_placement().is_some(),
            local
        );
        let projected = clip.world_clip_geometry.get().0.unwrap();
        ctx.drain_node_effects();
        // A projected-path invalidation should reach occlusion only while
        // occlusion is using that coordinate space.
        clip.world_clip_geometry.invalidate();
        assert_eq!(pax_runtime_api::properties::has_pending_effects(), !local);
        assert_eq!(clip.world_clip_geometry.get().0.as_ref(), Some(&projected));
        settle_occlusion_effects(&root, &ctx);
        assert_full_rebuild_matches(&root, &ctx);
    };
    check(true);
    #[cfg(feature = "3d")]
    {
        let tilt = clip.get_common_properties().borrow().rotate_y.clone();
        tilt.set(Some(pax_runtime_api::Rotation::Degrees(30.into())));
        check(false);
        tilt.set(None);
        check(true);
    }
    let handle = crate::NodeRef::new(&clip, &ctx);
    handle
        .move_to_append_child(&crate::NodeRef::new(&parents[2], &ctx))
        .unwrap();
    check(true);
    assert_eq!(clip.native_placement_owner(), Some(parents[2].id.to_u32()));
    handle
        .move_to_append_child(&crate::NodeRef::new(&parents[1], &ctx))
        .unwrap();
    check(false);
    handle.move_to_home().unwrap();
    check(true);
}

#[test]
fn clip_dependencies_cross_layout_groups_and_follow_responsive_roles() {
    let (ctx, root) = mount(vec![
        probe(
            Kind::Clip,
            vec![probe(
                Kind::ResponsiveGroup,
                vec![probe(Kind::Group, vec![probe(Kind::Canvas, vec![])])],
            )],
        ),
        probe(Kind::Native, vec![]),
    ]);
    let branch = root.children.get()[0].clone();
    let clip = branch.children.get()[0].clone();
    let native = branch.children.get()[1].clone();
    let owner = clip.children.get()[0].clone();
    let group = owner.children.get()[0].clone();
    let canvas = group.children.get()[0].clone();
    native
        .get_common_properties()
        .borrow()
        .width
        .set(Some(Size::Pixels(40.into())));

    for width in [200, 120, 200, 60, 200] {
        owner
            .get_common_properties()
            .borrow()
            .width
            .set(Some(Size::Pixels(width.into())));
        settle_occlusion_effects(&root, &ctx);
        {
            let state = ctx.occlusion_state.borrow();
            let links = &state.clip_dependents[&clip.id.to_u32()];
            assert!(links.contains(&canvas.id.to_u32()));
            assert!(!links.contains(&group.id.to_u32()));
            assert_eq!(links.contains(&owner.id.to_u32()), width < 160);
            let record = &state.records[&owner.id.to_u32()];
            let clips = (80..160).contains(&width);
            assert_eq!(record.clip.is_some(), clips);
            assert_eq!(record.layer == Layer::Native, width < 80);
            assert_eq!(
                state.records[&canvas.id.to_u32()]
                    .clips
                    .contains(&owner.id.to_u32()),
                clips,
            );
        }
        // Force a direct placement notification and an inherited clip change
        // in the same pass. The intermediate group needs neither coverage visit.
        ctx.mark_node_occlusion_placement_dirty(group.id.to_u32());
        ctx.mark_node_occlusion_dirty(clip.id.to_u32());
        let before = ctx.occlusion_stats();
        update_node_occlusion(&root, &ctx);
        assert_eq!(
            ctx.occlusion_stats().records_updated - before.records_updated,
            if width < 160 { 3 } else { 2 }
        );
        assert_full_rebuild_matches(&root, &ctx);
    }
    assert_ne!(native.native_mask_hash.get(), 0);
    group
        .get_common_properties()
        .borrow()
        .x
        .set(Some(Size::Pixels(100.into())));
    settle_occlusion_effects(&root, &ctx);
    assert_eq!(native.native_mask_hash.get(), 0);
    assert_full_rebuild_matches(&root, &ctx);
}

#[test]
fn retained_host_motion_and_subsequent_clip_edits_match_full_rebuild() {
    let (ctx, root) = mount(vec![retained_probe(
        Kind::Clip,
        vec![retained_probe(
            Kind::Scroller,
            vec![
                retained_probe(Kind::Clip, vec![retained_probe(Kind::Canvas, vec![])]),
                retained_probe(Kind::Native, vec![]),
            ],
        )],
    )]);
    let outer = root.children.get()[0].children.get()[0].clone();
    let host = outer.children.get()[0].clone();
    let clip = host.children.get()[0].clone();
    let canvas = clip.children.get()[0].clone();
    let native = host.children.get()[1].clone();
    for _ in 0..3 {
        settle(&root, &ctx);
    }
    assert!(canvas.local_compositing.get());
    assert!(native.local_compositing.get());
    let before = ctx.occlusion_stats();
    outer
        .get_common_properties()
        .borrow()
        .x
        .set(Some(Size::Pixels(25.into())));
    for _ in 0..3 {
        settle(&root, &ctx);
    }
    assert!(ctx.occlusion_stats().records_updated - before.records_updated <= 3);
    // Do not rebuild before these edits: they must query retained paths at their
    // current world pose, not at the pose where each path was first prepared.
    native
        .get_common_properties()
        .borrow()
        .x
        .set(Some(Size::Pixels(12.into())));
    settle(&root, &ctx);
    assert_full_rebuild_matches(&root, &ctx);
    for node in [&outer, &clip, &canvas] {
        node.get_common_properties()
            .borrow()
            .x
            .set(Some(Size::Pixels(33.into())));
        settle(&root, &ctx);
        assert_full_rebuild_matches(&root, &ctx);
    }
}

#[test]
fn surface_motion_reuses_opted_in_local_paint_but_not_custom_world_hooks() {
    use std::cell::Cell;
    struct PaintProbe {
        base: BaseInstance,
        local: bool,
        calls: Cell<usize>,
        extent: Cell<f64>,
        alpha: Cell<f64>,
        alpha_reads: Cell<u32>,
    }
    impl InstanceNode for PaintProbe {
        fn instantiate(_: InstantiationArgs) -> Rc<Self> {
            unreachable!()
        }
        fn base(&self) -> &BaseInstance {
            &self.base
        }
        fn resolve_debug(
            &self,
            f: &mut std::fmt::Formatter,
            _: Option<&ExpandedNode>,
        ) -> std::fmt::Result {
            f.write_str("PaintProbe")
        }
        fn canvas_geometry_is_local(&self) -> bool {
            self.local
        }
        fn local_coverage_alpha(&self, _: &ExpandedNode) -> Option<f64> {
            self.local.then(|| {
                self.alpha_reads.set(self.alpha_reads.get() + 1);
                self.alpha.get()
            })
        }
        fn prepare_canvas_geometry(
            &self,
            node: &ExpandedNode,
        ) -> crate::scene_geometry::CanvasGeometry {
            self.calls.set(self.calls.get() + 1);
            let x = if self.local {
                0.
            } else {
                node.presentation_transform_and_bounds.get().transform.m[4]
            };
            crate::scene_geometry::CanvasGeometry {
                local_bounds: Some(Rect::new(x, 0., x + self.extent.get(), 20.)),
                path: None,
            }
        }
    }
    let make = |local| {
        Rc::new(PaintProbe {
            base: BaseInstance::new(
                args(vec![]),
                InstanceFlags {
                    layer: Layer::Canvas,
                    invisible_to_slot: false,
                    invisible_to_raycasting: true,
                    is_component: false,
                    is_slot: false,
                },
            ),
            local,
            calls: Cell::new(0),
            extent: Cell::new(40.),
            alpha: Cell::new(0.5),
            alpha_reads: Cell::new(0),
        })
    };
    let local = make(true);
    let custom = make(false);
    let (ctx, root) = mount(vec![probe(
        Kind::Scroller,
        vec![local.clone(), local.clone(), custom.clone()],
    )]);
    let scroller = root.children.get()[0].children.get()[0].clone();
    let children = scroller.children.get();
    let calls = (local.calls.get(), custom.calls.get());
    let initial_alpha_reads = local.alpha_reads.get();
    for x in [2., 13., -8.] {
        scroller
            .get_common_properties()
            .borrow()
            .x
            .set(Some(Size::Pixels(x.into())));
        settle(&root, &ctx);
    }
    assert_eq!(
        local.calls.get(),
        calls.0,
        "local shape preparation must not scale with movement"
    );
    assert!(
        custom.calls.get() > calls.1,
        "custom world-dependent hooks retain their contract"
    );
    // A canvas redraw can be caused by placement or lighting alone.
    ctx.mark_canvas_node_dirty(children[0].id);
    ctx.mark_node_occlusion_placement_dirty(children[0].id.to_u32());
    settle(&root, &ctx);
    assert_eq!(local.alpha_reads.get(), initial_alpha_reads);
    scroller
        .get_common_properties()
        .borrow()
        .opacity
        .set(Some(0.5.into()));
    settle(&root, &ctx);
    let effective =
        |i: usize| ctx.occlusion_state.borrow().records[&children[i].id.to_u32()].opacity;
    assert_eq!(effective(0), 0.25);
    assert_eq!(effective(2), 0.5);
    local.alpha.set(0.75);
    let before = ctx.canvas_geometry_for_node(&children[0]);
    local.extent.set(90.);
    ctx.mark_canvas_node_dirty(children[0].id);
    let after = ctx.canvas_geometry_for_node(&children[0]);
    assert_eq!(after.local.local_bounds.unwrap().width(), 90.);
    ctx.mark_node_occlusion_dirty(children[0].id.to_u32());
    settle(&root, &ctx);
    assert_eq!(effective(0), 0.375);
    assert_eq!(effective(1), 0.25);
    assert_ne!(before.coverage_bounds, after.coverage_bounds);
    assert_eq!(
        ctx.canvas_geometry_for_node(&children[1])
            .local
            .local_bounds
            .unwrap()
            .width(),
        40.,
        "each expanded occurrence owns its cache"
    );
}

#[test]
fn local_coverage_cache_tracks_shape_inputs_and_occurrence_lifetimes() {
    use pax_runtime_api::{LocalProperty, Variable};
    use std::cell::Cell;

    struct ShapeProbe {
        base: BaseInstance,
        cached: bool,
        shape: LocalProperty<f64>,
        manual: Cell<f64>,
        reads: RefCell<HashMap<u32, usize>>,
    }
    impl InstanceNode for ShapeProbe {
        fn instantiate(_: InstantiationArgs) -> Rc<Self> {
            unreachable!()
        }
        fn base(&self) -> &BaseInstance {
            &self.base
        }
        fn resolve_debug(
            &self,
            f: &mut std::fmt::Formatter,
            _: Option<&ExpandedNode>,
        ) -> std::fmt::Result {
            f.debug_struct("ShapeProbe").finish()
        }
        fn local_occlusion_path_is_retained(&self) -> bool {
            true
        }
        fn local_occlusion_path_is_placement_independent(&self) -> bool {
            self.cached
        }
        fn resolve_local_occlusion_path(&self, node: &ExpandedNode) -> Option<BezPath> {
            *self.reads.borrow_mut().entry(node.id.to_u32()).or_default() += 1;
            let width = node
                .presentation_transform_and_bounds
                .get()
                .bounds
                .0
                .min(40.)
                + self.shape.get()
                + self.manual.get();
            Some(Rect::new(0., 0., width, 20.).to_path(0.1))
        }
    }

    let mut uncached_masks = Vec::new();
    for cached in [false, true] {
        let shape = LocalProperty::new(0.);
        let dependency = shape.clone();
        let mut arguments = args(vec![]);
        arguments.properties_scope = PropertiesScopeInit::Factory(Box::new(move |_| {
            HashMap::from([(
                "shape".into(),
                Variable::new_from_typed_property(dependency.clone()),
            )])
        }));
        let shared = Rc::new(ShapeProbe {
            base: BaseInstance::new(
                arguments,
                InstanceFlags {
                    layer: Layer::Canvas,
                    invisible_to_slot: false,
                    invisible_to_raycasting: true,
                    is_component: false,
                    is_slot: false,
                },
            ),
            cached,
            shape: shape.clone(),
            manual: Cell::new(0.),
            reads: RefCell::new(HashMap::new()),
        });
        // One InstanceNode backs two separately retained expanded occurrences.
        let (ctx, root) = mount(vec![
            shared.clone(),
            shared.clone(),
            probe(Kind::Native, vec![]),
        ]);
        let _graph = ctx.property_graph.enter();
        let parent = root.children.get()[0].clone();
        let children = parent.children.get();
        let a = children[0].clone();
        let b = children[1].clone();
        let retained_path = ctx.occlusion_state.borrow().records[&a.id.to_u32()]
            .path
            .clone()
            .unwrap();
        let before = shared.reads.borrow().clone();
        for (sample, x) in [2., 4., 6.].into_iter().enumerate() {
            a.get_common_properties()
                .borrow()
                .x
                .set(Some(Size::Pixels(x.into())));
            a.get_common_properties()
                .borrow()
                .opacity
                .set(Some(0.5.into()));
            settle(&root, &ctx);
            assert_eq!(
                ctx.occlusion_state.borrow().records[&a.id.to_u32()].opacity,
                0.5
            );
            let mask = children[2].native_mask_hash.get();
            if cached {
                assert_eq!(
                    mask, uncached_masks[sample],
                    "retained local masks match live world projection"
                );
                let state = ctx.occlusion_state.borrow();
                let record = &state.records[&a.id.to_u32()];
                assert!(record.path_is_local);
                assert!(Rc::ptr_eq(record.path.as_ref().unwrap(), &retained_path));
                assert_eq!(record.path_bounds, Some(Rect::new(0., 0., 40., 20.)));
            } else {
                uncached_masks.push(mask);
            }
            assert_full_rebuild_matches(&root, &ctx);
        }
        if cached {
            assert_eq!(
                *shared.reads.borrow(),
                before,
                "placement and opacity retain local shape"
            );
        } else {
            assert!(
                shared.reads.borrow()[&a.id.to_u32()] > before[&a.id.to_u32()],
                "custom callbacks keep their existing live behavior"
            );
        }
        let before = shared.reads.borrow().clone();
        a.get_common_properties()
            .borrow()
            .width
            .set(Some(Size::Pixels(10.0.into())));
        settle(&root, &ctx);
        assert!(shared.reads.borrow()[&a.id.to_u32()] > before[&a.id.to_u32()]);
        if cached {
            assert_eq!(
                shared.reads.borrow()[&b.id.to_u32()],
                before[&b.id.to_u32()]
            );
        }
        let before = shared.reads.borrow().clone();
        shape.set(7.);
        settle(&root, &ctx);
        for node in [&a, &b] {
            assert!(shared.reads.borrow()[&node.id.to_u32()] > before[&node.id.to_u32()]);
        }
        let before = shared.reads.borrow()[&a.id.to_u32()];
        shared.manual.set(11.);
        ctx.mark_node_occlusion_dirty(a.id.to_u32());
        ctx.mark_node_occlusion_dirty(b.id.to_u32());
        settle(&root, &ctx);
        assert!(shared.reads.borrow()[&a.id.to_u32()] > before);
        assert_full_rebuild_matches(&root, &ctx);
        let path = ctx.occlusion_state.borrow().records[&a.id.to_u32()]
            .path
            .clone()
            .unwrap();
        assert_eq!(path.bounding_box().width(), 28.);

        attach(&parent, vec![b.clone(), children[2].clone()], &ctx);
        settle(&root, &ctx);
        assert!(
            a.local_occlusion_geometry.get().0.is_none(),
            "unmount releases retained shape"
        );
        let before = shared.reads.borrow()[&a.id.to_u32()];
        attach(&parent, children, &ctx);
        settle(&root, &ctx);
        assert!(
            shared.reads.borrow()[&a.id.to_u32()] > before,
            "remount prepares fresh coverage"
        );
        assert_full_rebuild_matches(&root, &ctx);
    }
}

#[test]
fn collapsing_native_surface_clears_old_mask_and_expansion_restores_it() {
    let (ctx, root) = mount(vec![
        probe(Kind::Canvas, vec![]),
        probe(Kind::Native, vec![]),
    ]);
    let native = root.children.get()[0].children.get()[1].clone();
    assert_ne!(native.native_mask_hash.get(), 0);
    for (scale, expected_entries) in [(0.0, 0), (1.0, 1), (0.0, 0), (1.0, 1)] {
        native
            .get_common_properties()
            .borrow()
            .scale_x
            .set(Some(Size::Percent((scale * 100.0).into())));
        native
            .get_common_properties()
            .borrow()
            .scale_y
            .set(Some(Size::Percent((scale * 100.0).into())));
        settle(&root, &ctx);
        let masks: Vec<_> = ctx
            .take_native_messages()
            .into_iter()
            .filter_map(|message| match message {
                pax_message::NativeMessage::NativeMaskUpdate(patch)
                    if patch.id == native.id.to_u32() =>
                {
                    Some(patch)
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            masks.len(),
            1,
            "each scale transition updates the existing mask"
        );
        assert_eq!(masks[0].entries.len(), expected_entries);
        assert!(masks[0]
            .entries
            .iter()
            .all(|entry| { !entry.path.contains("NaN") && !entry.path.contains("inf") }));
        assert_full_rebuild_matches(&root, &ctx);
        clear(&ctx);
    }
}

fn attach(parent: &Rc<ExpandedNode>, children: Vec<Rc<ExpandedNode>>, ctx: &Rc<RuntimeContext>) {
    let children = parent.attach_children(children, ctx, &parent.parent_frame);
    parent.children.set(children);
}

fn add(parent: &Rc<ExpandedNode>, kind: Kind, ctx: &Rc<RuntimeContext>) -> Rc<ExpandedNode> {
    let mut children = parent.children.get();
    let node = parent
        .create_children_detached(
            vec![(probe(kind, vec![]), parent.stack.clone())],
            ctx,
            &Rc::downgrade(parent),
        )
        .remove(0);
    children.push(node.clone());
    attach(parent, children, ctx);
    node
}

#[test]
fn occlusion_reads_mounted_branches_while_a_responsive_switch_is_pending() {
    use crate::properties::input_capture::InputPublisher;
    use crate::{ConditionalInstance, ConditionalProperties};
    use pax_runtime_api::{pax_value::ToFromPaxAny, LocalProperty};

    for (full_rebuild, transfer) in [(false, false), (true, false), (false, true), (true, true)] {
        let wide = LocalProperty::new(true);
        let condition = wide.clone();
        let mut conditional = args(vec![
            probe(Kind::Native, vec![]),
            probe(Kind::Native, vec![]),
        ]);
        conditional.prototypical_properties =
            PropertiesInit::Factory(Box::new(move |_, existing| {
                existing.is_none().then(|| {
                    Rc::new(RefCell::new(
                        ConditionalProperties {
                            boolean_expression: condition.clone(),
                            conditional_branches: vec![condition.clone(), LocalProperty::new(true)],
                        }
                        .to_pax_any(),
                    ))
                })
            }));
        let (ctx, root) = mount(vec![
            probe(Kind::Canvas, vec![]),
            ConditionalInstance::instantiate_with_branch_child_ranges(
                conditional,
                vec![0..1, 1..2],
            ),
        ]);
        ctx.register_root_expanded_node(&root);
        let group = root.children.get()[0].clone();
        let branch = group.children.get()[1].clone();
        if transfer {
            // Incoming placements flatten the conditional while traversing the group.
            let canvas = group.children.get()[0].clone();
            crate::NodeRef::new(&canvas, &ctx)
                .move_to_append_child(&crate::NodeRef::new(&group, &ctx))
                .unwrap();
        }
        settle_occlusion_effects(&root, &ctx);
        let old = branch.children.get()[0].clone();
        let mut publisher = InputPublisher::default();
        let hits = |view: &crate::properties::input_capture::CapturedInputView, id: u32| {
            view.hit_targets((8., 8.).into(), &HashMap::new(), true)
                .unwrap()
                .iter()
                .any(|hit| hit.identity.id == id)
        };
        let displayed = publisher.capture(&ctx).unwrap();
        assert!(hits(&displayed, old.id.to_u32()));

        wide.set(false);
        if full_rebuild {
            ctx.occlusion_state.borrow_mut().records.clear();
        }
        ctx.mark_occlusion_dirty_for_node(branch.id.to_u32());
        update_node_occlusion(&root, &ctx);
        assert!(ctx
            .occlusion_state
            .borrow()
            .records
            .contains_key(&old.id.to_u32()));
        if transfer {
            // A synchronous logical query through the same placement resolves
            // the pending switch as soon as the caller asks for the new tree.
            let logical = group.logical_children(&ctx);
            assert!(!logical.iter().any(|node| node.id == old.id));
            assert!(logical.iter().any(|node| node.id == branch.children.get()[0].id));
        }
        settle_occlusion_effects(&root, &ctx);
        let new = branch.children.get()[0].clone();
        assert_ne!(new.id, old.id);
        {
            let state = ctx.occlusion_state.borrow();
            assert!(!state.records.contains_key(&old.id.to_u32()));
            assert!(state.records.contains_key(&new.id.to_u32()));
        }
        let next = publisher.capture(&ctx).unwrap();
        assert!(hits(&displayed, old.id.to_u32()));
        assert!(!hits(&next, old.id.to_u32()));
        assert!(hits(&next, new.id.to_u32()));
        assert_full_rebuild_matches(&root, &ctx);
    }
}

fn assert_full_rebuild_matches(root: &Rc<ExpandedNode>, ctx: &RuntimeContext) {
    let membership = |state: &OcclusionState| {
        let pairs = |map: &HashMap<u32, Vec<u32>>| {
            let mut pairs: Vec<_> = map
                .iter()
                .flat_map(|(&parent, children)| children.iter().map(move |&child| (parent, child)))
                .collect();
            pairs.sort_unstable();
            pairs
        };
        (
            pairs(&state.clip_dependents),
            pairs(&state.scroll_containers),
            state.presentation_containers.clone(),
            state.owned_layers.clone(),
            state.isolated_scrollers.clone(),
            state.leaf_scroll_regions.clone(),
            state.unsafe_layers.clone(),
            state.topology.order.clone(),
            state
                .canvas_counts
                .iter()
                .filter(|(_, count)| **count > 0)
                .map(|(&id, &count)| (id, count))
                .collect::<HashMap<_, _>>(),
        )
    };
    let expected_membership = membership(&ctx.occlusion_state.borrow());
    let expected: HashMap<_, _> = ctx
        .occlusion_state
        .borrow()
        .records
        .iter()
        .filter_map(|(&id, record)| {
            let n = ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(id))?;
            Some((
                id,
                (
                    n.occlusion.get(),
                    n.native_mask_hash.get(),
                    n.presentation_cache_hash.get(),
                    record.clips.clone(),
                    record.render_layer_id,
                    n.local_compositing.get(),
                    record.coverage_bounds,
                ),
            ))
        })
        .collect();
    ctx.occlusion_state.borrow_mut().records.clear();
    ctx.mark_occlusion_dirty();
    update_node_occlusion(root, ctx);
    assert_eq!(
        membership(&ctx.occlusion_state.borrow()),
        expected_membership
    );
    assert_eq!(
        ctx.occlusion_state.borrow().records.len(),
        expected.len(),
        "local reconciliation must neither drop nor retain extra nodes"
    );
    for (id, (order, mask, presentation, clips, layer, local, bounds)) in expected {
        let n = ctx
            .get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(id))
            .unwrap();
        assert_eq!(n.occlusion.get(), order, "order for {id}");
        assert_eq!(
            n.local_compositing.get(),
            local,
            "local compositing eligibility for {id}"
        );
        assert_eq!(n.native_mask_hash.get(), mask, "mask for {id}");
        assert_eq!(
            n.presentation_cache_hash.get(),
            presentation,
            "presentation for {id}"
        );
        let state = ctx.occlusion_state.borrow();
        assert_eq!(state.records[&id].clips, clips, "clip lineage for {id}");
        assert_eq!(state.records[&id].render_layer_id, layer, "layer for {id}");
        let rebuilt = state.records[&id].coverage_bounds;
        if let (CoverageBounds::Finite(a), CoverageBounds::Finite(b)) = (rebuilt, bounds) {
            // Cancelling a shared scroll through an affine inverse can round
            // differently from retaining the original surface-space bounds.
            let edges = |r: Rect| [r.x0, r.y0, r.x1, r.y1];
            assert!(
                edges(a)
                    .into_iter()
                    .zip(edges(b))
                    .all(|(x, y)| (x - y).abs() <= 1.0e-9),
                "coverage for {id}: retained {b:?}, rebuilt {a:?}"
            );
        } else {
            assert_eq!(rebuilt, bounds, "coverage for {id}");
        }
    }
}

#[test]
fn viewport_resize_updates_fixed_clips_and_preserves_order_until_a_breakpoint() {
    use crate::properties::input_capture::InputPublisher;
    use crate::{ConditionalInstance, ConditionalProperties};
    use pax_runtime_api::{pax_value::ToFromPaxAny, LocalProperty};

    for platform in [Platform::Web, Platform::Native] {
        let mut engine = crate::test_support::empty_engine(
            (300., 200.),
            platform,
            OS::Mac,
            Box::new(|| 0),
            Default::default(),
        );
        let ctx = engine.runtime_context.clone();
        let viewport = ctx.globals().viewport;
        let deps = [viewport.untyped()];
        let wide = LocalProperty::computed(move || viewport.get().bounds.0 >= 350., &deps);
        let mut conditional = args(vec![probe(Kind::Native, vec![])]);
        conditional.prototypical_properties = PropertiesInit::Factory(Box::new(move |_, old| {
            old.is_none().then(|| {
                Rc::new(RefCell::new(
                    ConditionalProperties {
                        boolean_expression: wide.clone(),
                        conditional_branches: vec![wide.clone()],
                    }
                    .to_pax_any(),
                ))
            })
        }));
        let mut root_args = args(vec![]);
        root_args.component_template = Some(RefCell::new(vec![probe(
            Kind::Group,
            vec![
                retained_probe(
                    Kind::Scroller,
                    vec![
                        retained_probe(Kind::Canvas, vec![]),
                        retained_probe(Kind::Native, vec![]),
                    ],
                ),
                ConditionalInstance::instantiate(conditional),
            ],
        )]));
        let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
        let group = root.children.get()[0].clone();
        let host = group.children.get()[0].clone();
        let branch = group.children.get()[1].clone();
        let native = host.children.get()[1].clone();
        for (node, width, height) in [(&group, 500., 200.), (&host, 120., 80.)] {
            let common = node.get_common_properties();
            common.borrow().width.set(Some(Size::Pixels(width.into())));
            common
                .borrow()
                .height
                .set(Some(Size::Pixels(height.into())));
        }
        host.get_common_properties()
            .borrow()
            .x
            .set(Some(Size::Pixels(240.0.into())));
        engine.tick();
        let mut publisher = InputPublisher::default();
        publisher.capture(&ctx).unwrap();
        let owner = host.browser_content_layer_id.get().unwrap();
        for (width, switched) in [(301., false), (330., false), (360., true), (300., true)] {
            let before = ctx.occlusion_stats();
            engine.set_viewport_size((width, 200.));
            let messages = engine.tick();
            let after = ctx.occlusion_stats();
            if !switched {
                assert_eq!(after.topology_nodes_visited, before.topology_nodes_visited);
                assert_eq!(after.rebuilds, before.rebuilds);
                assert_eq!(
                    after.structural_reconciliations,
                    before.structural_reconciliations
                );
            }
            assert_eq!(branch.children.get().len(), usize::from(width >= 350.));
            assert!(messages.iter().any(|m| matches!(m,
                pax_message::NativeMessage::ScrollerUpdate(p) if p.id == host.id.to_u32()
                && p.presented_clip_bounds == Some([240., 0., width, 80.]))));
            assert_eq!(ctx.get_layer_scroller_owner(owner as usize), Some(host.id));
            let hits = |view: &crate::properties::input_capture::CapturedInputView| {
                view.hit_targets((width - 2., 30.).into(), &HashMap::new(), true)
                    .unwrap()
                    .into_iter()
                    .map(|hit| hit.identity.id)
                    .collect::<Vec<_>>()
            };
            let captured = hits(&publisher.capture(&ctx).unwrap());
            assert!(captured.contains(&native.id.to_u32()));
            assert_full_rebuild_matches(&root, &ctx);
            settle_occlusion_effects(&root, &ctx);
            assert_eq!(
                hits(&InputPublisher::default().capture(&ctx).unwrap()),
                captured
            );
        }
    }
}

fn scroll_surface(ctx: &RuntimeContext, node: &ExpandedNode, y: f64) {
    ctx.set_scroller_surface_state(
        node.id.to_u32(),
        crate::properties::ScrollerSurfaceState {
            viewport_width: 60.,
            viewport_height: 60.,
            content_width: 60.,
            content_height: 1200.,
            scroll_y: y,
            presentation_scroll_y: y,
            clip_content: true,
            ..Default::default()
        },
    );
}

#[test]
fn web_container_presentation_reports_scroller_bounds_through_clipping_frames() {
    use pax_message::NativeMessage;
    let (ctx, root) = mount(vec![probe(
        Kind::Clip,
        vec![probe(Kind::Scroller, vec![probe(Kind::Canvas, vec![])])],
    )]);
    ctx.edit_globals(|g| g.platform = Platform::Web);
    let clip = root.children.get()[0].children.get()[0].clone();
    let scroller = clip.children.get()[0].clone();
    for (x, full) in [(12., true), (13., false)] {
        clip.get_common_properties()
            .borrow()
            .x
            .set(Some(Size::Pixels(x.into())));
        if full {
            ctx.mark_occlusion_dirty();
        }
        settle(&root, &ctx);
        let messages = ctx.take_native_messages();
        assert!(!messages
            .iter()
            .any(|m| matches!(m, NativeMessage::FrameUpdate(p)
            if p.presented_bounds.is_some() || p.presented_clip_bounds.is_some())));
        let patch = messages
            .iter()
            .find_map(|m| match m {
                NativeMessage::ScrollerUpdate(p)
                    if p.id == scroller.id.to_u32() && p.presented_bounds.is_some() =>
                {
                    Some(p)
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(patch.presented_clip_bounds, Some([x, 0., x + 60., 60.]));
        assert_eq!(patch.content_layer_id, None);
    }
}

#[test]
fn shared_scroll_motion_preserves_masks_without_evaluating_them() {
    let (ctx, root) = mount(vec![probe(
        Kind::Clip,
        vec![probe(
            Kind::Scroller,
            vec![probe(
                Kind::Scroller,
                vec![probe(Kind::Canvas, vec![]), probe(Kind::Native, vec![])],
            )],
        )],
    )]);
    let outer = root.children.get()[0].children.get()[0].children.get()[0].clone();
    let inner = outer.children.get()[0].clone();
    let native = inner.children.get()[1].clone();
    outer
        .get_common_properties()
        .borrow()
        .scale_x
        .set(Some(Size::Percent(55.0.into())));
    outer
        .get_common_properties()
        .borrow()
        .scale_y
        .set(Some(Size::Percent(55.0.into())));
    outer
        .get_common_properties()
        .borrow()
        .rotate
        .set(Some(pax_runtime_api::Rotation::Degrees(17.0.into())));
    scroll_surface(&ctx, &outer, 0.);
    scroll_surface(&ctx, &inner, 0.);
    settle(&root, &ctx);
    let initial = native.native_mask_hash.get();
    assert_ne!(initial, 0);
    clear(&ctx);
    for (outer_y, inner_y) in [(23.25, 0.), (300.125, 17.5), (0., 0.)] {
        let before = ctx.occlusion_stats();
        scroll_surface(&ctx, &outer, outer_y);
        scroll_surface(&ctx, &inner, inner_y);
        settle(&root, &ctx);
        assert_eq!(ctx.occlusion_stats().rebuilds, before.rebuilds);
        assert_eq!(
            ctx.occlusion_stats().masks_evaluated,
            before.masks_evaluated
        );
        assert_eq!(native.native_mask_hash.get(), initial);
        assert!(!ctx
            .take_native_messages()
            .iter()
            .any(|m| matches!(m, pax_message::NativeMessage::NativeMaskUpdate(_))));
        assert_full_rebuild_matches(&root, &ctx);
        clear(&ctx);
    }
}

#[test]
fn moving_and_resizing_nested_scrollers_reconcile_only_the_affected_branch() {
    let (ctx, root) = mount(
        (0..8)
            .map(|_| {
                probe(
                    Kind::Scroller,
                    vec![probe(
                        Kind::Scroller,
                        vec![
                            probe(Kind::Clip, vec![probe(Kind::Canvas, vec![])]),
                            probe(Kind::Native, vec![]),
                        ],
                    )],
                )
            })
            .collect(),
    );
    let outer = root.children.get()[0].children.get()[0].clone();
    let inner = outer.children.get()[0].clone();
    scroll_surface(&ctx, &outer, 11.25);
    scroll_surface(&ctx, &inner, 4.5);
    settle(&root, &ctx);
    for (x, width, scale, angle) in [
        (20., 180., 0.7, 17.),
        (90., 100., 1.2, -9.),
        (0., 300., 1., 0.),
    ] {
        let before = ctx.occlusion_stats();
        {
            let p = outer.get_common_properties();
            let p = p.borrow();
            p.x.set(Some(Size::Pixels(x.into())));
            p.width.set(Some(Size::Pixels(width.into())));
            p.scale_x.set(Some(Size::Percent((scale * 100.).into())));
            p.scale_y.set(Some(Size::Percent((scale * 100.).into())));
            p.rotate
                .set(Some(pax_runtime_api::Rotation::Degrees(angle.into())));
        }
        settle(&root, &ctx);
        let after = ctx.occlusion_stats();
        assert_eq!(
            after.rebuilds, before.rebuilds,
            "stable scroller ownership must not rebuild"
        );
        assert_eq!(after.topology_nodes_visited, before.topology_nodes_visited);
        assert!(after.records_updated - before.records_updated <= 5);
        assert_full_rebuild_matches(&root, &ctx);
    }
}

#[test]
fn scroller_extents_do_not_rebuild_coverage_but_clip_policy_does() {
    let (ctx, root) = mount(vec![probe(
        Kind::Scroller,
        vec![probe(Kind::Canvas, vec![]), probe(Kind::Native, vec![])],
    )]);
    let scroller = root.children.get()[0].children.get()[0].clone();
    scroll_surface(&ctx, &scroller, 0.);
    settle(&root, &ctx);
    let before = ctx.occlusion_stats();
    assert_eq!(
        ctx.set_scroller_surface_state(
            scroller.id.to_u32(),
            crate::ScrollerSurfaceState {
                viewport_width: 120.,
                viewport_height: 90.,
                content_width: 240.,
                content_height: 1800.,
                clip_content: true,
                ..Default::default()
            }
        ),
        crate::ScrollerSurfaceStateChange::Geometry
    );
    settle(&root, &ctx);
    let after = ctx.occlusion_stats();
    assert_eq!(after.rebuilds, before.rebuilds);
    assert_eq!(
        after.structural_reconciliations,
        before.structural_reconciliations
    );
    assert_eq!(after.records_updated, before.records_updated);
    assert_eq!(after.masks_evaluated, before.masks_evaluated);
    assert_full_rebuild_matches(&root, &ctx);
    assert_eq!(
        ctx.set_scroller_surface_state(
            scroller.id.to_u32(),
            crate::ScrollerSurfaceState {
                clip_content: false,
                ..Default::default()
            }
        ),
        crate::ScrollerSurfaceStateChange::Structural
    );
}

#[test]
fn singular_and_shared_layer_scroller_geometry_keeps_the_full_fallback() {
    let (ctx, root) = mount(vec![probe(
        Kind::Scroller,
        vec![probe(Kind::Canvas, vec![]), probe(Kind::Native, vec![])],
    )]);
    let scroller = root.children.get()[0].children.get()[0].clone();
    scroll_surface(&ctx, &scroller, 0.);
    settle(&root, &ctx);
    for scale in [0., 100.] {
        let before = ctx.occlusion_stats();
        scroller
            .get_common_properties()
            .borrow()
            .scale_x
            .set(Some(Size::Percent(scale.into())));
        settle(&root, &ctx);
        assert!(ctx.occlusion_stats().rebuilds > before.rebuilds);
        assert_full_rebuild_matches(&root, &ctx);
    }
    ctx.globals()
        .browser_allows_scroller_vector_layers
        .set(false);
    ctx.mark_occlusion_dirty();
    settle(&root, &ctx);
    let before = ctx.occlusion_stats();
    scroller
        .get_common_properties()
        .borrow()
        .x
        .set(Some(Size::Pixels(25.0.into())));
    settle(&root, &ctx);
    assert!(ctx.occlusion_stats().rebuilds > before.rebuilds);
    assert_full_rebuild_matches(&root, &ctx);
}

#[test]
fn isolated_scroller_clip_resize_preserves_content_local_masks() {
    let (ctx, root) = mount(vec![probe(
        Kind::Scroller,
        vec![
            probe(Kind::Clip, vec![probe(Kind::Canvas, vec![])]),
            probe(Kind::Native, vec![]),
        ],
    )]);
    let scroller = root.children.get()[0].children.get()[0].clone();
    for child in scroller.children.get() {
        child
            .get_common_properties()
            .borrow()
            .width
            .set(Some(Size::Pixels(80.0.into())));
        child
            .get_common_properties()
            .borrow()
            .height
            .set(Some(Size::Pixels(80.0.into())));
    }
    scroll_surface(&ctx, &scroller, 0.);
    settle(&root, &ctx);
    let mask = scroller.children.get()[1].native_mask_hash.get();
    for width in [20., 120., 300.] {
        let before = ctx.occlusion_stats();
        scroller
            .get_common_properties()
            .borrow()
            .width
            .set(Some(Size::Pixels(width.into())));
        settle(&root, &ctx);
        let after = ctx.occlusion_stats();
        assert_eq!(after.rebuilds, before.rebuilds);
        assert_eq!(after.records_updated - before.records_updated, 4);
        assert_eq!(scroller.children.get()[1].native_mask_hash.get(), mask);
        assert_full_rebuild_matches(&root, &ctx);
    }
}

#[test]
fn shared_clips_are_not_baked_into_native_cutouts() {
    let (ctx, root) = mount(vec![probe(
        Kind::Clip,
        vec![probe(Kind::Canvas, vec![]), probe(Kind::Native, vec![])],
    )]);
    let clip = root.children.get()[0].children.get()[0].clone();
    let native = clip.children.get()[1].clone();
    native.native_mask_hash.set(0);
    ctx.mark_node_occlusion_dirty(native.id.to_u32());
    settle(&root, &ctx);
    let messages = ctx.take_native_messages();
    let mask = messages
        .iter()
        .find_map(|m| match m {
            pax_message::NativeMessage::NativeMaskUpdate(p) if p.id == native.id.to_u32() => {
                Some(p)
            }
            _ => None,
        })
        .expect("native overlaps the vector");
    assert_eq!(mask.entries.len(), 1);
    assert!(
        mask.entries[0].clips.is_empty(),
        "the shared native container already enforces this clip"
    );
    assert_full_rebuild_matches(&root, &ctx);
}

#[test]
fn moving_clip_batches_coverage_and_keeps_displayed_edge_hits() {
    use crate::properties::input_capture::InputPublisher;

    let (ctx, root) = mount(vec![
        probe(Kind::Clip, vec![probe(Kind::Canvas, vec![])]),
        probe(Kind::Native, vec![]),
    ]);
    ctx.register_root_expanded_node(&root);
    let group = root.children.get()[0].clone();
    let clip = group.children.get()[0].clone();
    let canvas = clip.children.get()[0].clone();
    let native = group.children.get()[1].clone();
    for (node, width) in [(&clip, 40), (&canvas, 40), (&native, 100)] {
        let properties = node.get_common_properties();
        properties
            .borrow()
            .width
            .set(Some(Size::Pixels(width.into())));
        properties
            .borrow()
            .height
            .set(Some(Size::Pixels(40.into())));
    }
    settle_occlusion_effects(&root, &ctx);
    let original_mask = native.native_mask_hash.get();
    assert_ne!(original_mask, 0);
    let points = [19.75, 20.25, 39.75, 40.25, 59.75, 60.25];
    let hits = |view: &crate::properties::input_capture::CapturedInputView| {
        points.map(|x| {
            view.hit_targets((x, 20.).into(), &HashMap::new(), true)
                .unwrap()
                .iter()
                .find(|hit| {
                    hit.identity.id == canvas.id.to_u32() || hit.identity.id == native.id.to_u32()
                })
                .map(|hit| hit.identity.id)
        })
    };
    let mut publisher = InputPublisher::default();
    let front = publisher.capture(&ctx).unwrap();
    let old_hits = hits(&front);
    let x = clip.get_common_properties().borrow().x.clone();
    x.set(Some(Size::Pixels(120.into())));
    assert_eq!(
        Affine::from(clip.presentation_transform_and_bounds.get().transform).as_coeffs()[4],
        120.
    );
    x.set(Some(Size::Pixels(20.into())));
    settle_occlusion_effects(&root, &ctx);
    assert_ne!(native.native_mask_hash.get(), original_mask);
    assert_ne!(native.native_mask_hash.get(), 0);
    let moved = hits(&publisher.capture(&ctx).unwrap());
    let c = Some(canvas.id.to_u32());
    let n = Some(native.id.to_u32());
    assert_eq!(old_hits, [c, c, c, n, n, n]);
    assert_eq!(moved, [n, c, c, c, c, n]);
    assert_eq!(hits(&front), old_hits);
    assert_full_rebuild_matches(&root, &ctx);
    settle_occlusion_effects(&root, &ctx);
    assert_eq!(
        hits(&InputPublisher::default().capture(&ctx).unwrap()),
        moved
    );
}

#[test]
fn occluder_only_clips_and_relative_geometry_still_update_cutouts() {
    let (ctx, root) = mount(vec![probe(
        Kind::Scroller,
        vec![
            probe(Kind::Clip, vec![probe(Kind::Canvas, vec![])]),
            probe(Kind::Native, vec![]),
        ],
    )]);
    let scroller = root.children.get()[0].children.get()[0].clone();
    let clip = scroller.children.get()[0].clone();
    let native = scroller.children.get()[1].clone();
    native
        .get_common_properties()
        .borrow()
        .width
        .set(Some(Size::Pixels(40.0.into())));
    native
        .get_common_properties()
        .borrow()
        .height
        .set(Some(Size::Pixels(40.0.into())));
    scroll_surface(&ctx, &scroller, 13.25);
    settle(&root, &ctx);
    native.native_mask_hash.set(0);
    ctx.mark_node_occlusion_dirty(native.id.to_u32());
    settle(&root, &ctx);
    let mask = ctx
        .take_native_messages()
        .into_iter()
        .find_map(|m| match m {
            pax_message::NativeMessage::NativeMaskUpdate(p) if p.id == native.id.to_u32() => {
                Some(p)
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(mask.entries.len(), 1);
    assert_eq!(
        mask.entries[0].clips.len(),
        1,
        "keep the vector's private rounded clip, not the shared scroller clip"
    );
    let inverse = Affine::from(native.transform_and_bounds.get().transform.inverse());
    let exact = inverse
        * ctx.occlusion_state.borrow().records[&clip.id.to_u32()]
            .clip
            .as_ref()
            .unwrap()
            .as_ref()
            .clone();
    assert_eq!(mask.entries[0].clips[0], bez_path_to_svg_path_data(&exact));
    let initial = native.native_mask_hash.get();
    clip.get_common_properties()
        .borrow()
        .x
        .set(Some(Size::Pixels(100.0.into())));
    settle(&root, &ctx);
    assert_eq!(native.native_mask_hash.get(), 0);
    assert_full_rebuild_matches(&root, &ctx);
    clip.get_common_properties()
        .borrow()
        .x
        .set(Some(Size::Pixels(0.0.into())));
    settle(&root, &ctx);
    assert_eq!(native.native_mask_hash.get(), initial);
    assert_full_rebuild_matches(&root, &ctx);
}

#[test]
fn shared_layer_scrolling_still_reconciles_relative_overlap() {
    let (ctx, root) = mount(vec![
        probe(Kind::Scroller, vec![probe(Kind::Canvas, vec![])]),
        probe(Kind::Native, vec![]),
    ]);
    let group = root.children.get()[0].clone();
    let scroller = group.children.get()[0].clone();
    let native = group.children.get()[1].clone();
    ctx.globals()
        .browser_allows_scroller_vector_layers
        .set(false);
    scroll_surface(&ctx, &scroller, 0.);
    ctx.mark_occlusion_dirty();
    settle(&root, &ctx);
    let initial = native.native_mask_hash.get();
    assert_ne!(initial, 0);
    let before = ctx.occlusion_stats();
    scroll_surface(&ctx, &scroller, 30.);
    settle(&root, &ctx);
    assert!(ctx.occlusion_stats().rebuilds > before.rebuilds);
    assert_ne!(native.native_mask_hash.get(), initial);
    assert_full_rebuild_matches(&root, &ctx);
}

#[test]
fn escaped_content_keeps_conservative_scroll_reconciliation() {
    let (ctx, root) = mount(vec![probe(
        Kind::Scroller,
        vec![probe(Kind::Canvas, vec![]), probe(Kind::Native, vec![])],
    )]);
    let scroller = root.children.get()[0].children.get()[0].clone();
    scroller.children.get()[1]
        .get_common_properties()
        .borrow()
        .unclippable
        .set(Some(true));
    scroll_surface(&ctx, &scroller, 0.);
    settle(&root, &ctx);
    let before = ctx.occlusion_stats();
    scroll_surface(&ctx, &scroller, 30.25);
    settle(&root, &ctx);
    assert!(ctx.occlusion_stats().rebuilds > before.rebuilds);
    assert_eq!(
        ctx.occlusion_stats().rebuild_reason,
        Some("unisolated-scroll")
    );
    assert_full_rebuild_matches(&root, &ctx);
}

#[test]
fn retained_exit_ticks_preserve_topology_until_children_are_removed() {
    use crate::conditional::{ConditionalInstance, ConditionalProperties};
    use pax_runtime_api::{pax_value::ToFromPaxAny, LocalProperty};

    let mut clip_args = args(vec![
        probe(Kind::Canvas, vec![]),
        probe(Kind::Native, vec![]),
    ]);
    clip_args.transition_config.has_exit = true;
    clip_args.transition_config.exit_frame_count = 4;
    clip_args.transition_config.timeout_ms = 5_000;
    let clip: Rc<dyn InstanceNode> = Rc::new(Probe {
        placement_checks: Default::default(),
        retained: false,
        kind: Kind::Clip,
        base: BaseInstance::new(
            clip_args,
            InstanceFlags {
                invisible_to_slot: false,
                invisible_to_raycasting: true,
                layer: Layer::DontCare,
                is_component: false,
                is_slot: false,
            },
        ),
    });
    let condition = LocalProperty::new(true);
    let bound_condition = condition.clone();
    let mut conditional_args = args(vec![clip]);
    conditional_args.prototypical_properties =
        PropertiesInit::Factory(Box::new(move |_, existing| {
            existing.is_none().then(|| {
                Rc::new(RefCell::new(
                    ConditionalProperties {
                        boolean_expression: bound_condition.clone(),
                        conditional_branches: vec![],
                    }
                    .to_pax_any(),
                ))
            })
        }));
    let (ctx, root) = mount(vec![ConditionalInstance::instantiate(conditional_args)]);
    let conditional = root.children.get()[0].children.get()[0].clone();
    let exiting = conditional.children.get()[0].clone();

    condition.set(false);
    settle(&root, &ctx);
    ctx.drain_node_effects();
    update_node_occlusion(&root, &ctx);
    for frame in 1..4 {
        let before = ctx.occlusion_stats();
        ctx.globals().elapsed_frames.set(frame);
        // Visual changes still update coverage; only unchanged membership is quiet.
        exiting
            .get_common_properties()
            .borrow()
            .opacity
            .set(Some((1.0 - frame as f64 * 0.1).into()));
        ctx.drain_node_effects();
        settle(&root, &ctx);
        let after = ctx.occlusion_stats();
        assert_eq!(after.rebuilds, before.rebuilds, "exit frame {frame}");
        assert_eq!(
            after.structural_reconciliations,
            before.structural_reconciliations
        );
        assert_eq!(conditional.children.get()[0].id, exiting.id);
        assert!(exiting.attached.get() > 0);
        assert_full_rebuild_matches(&root, &ctx);
    }

    let before = ctx.occlusion_stats();
    ctx.globals().elapsed_frames.set(4);
    ctx.drain_node_effects();
    settle(&root, &ctx);
    assert!(
        ctx.occlusion_stats().rebuilds > before.rebuilds,
        "clip removal must reconcile"
    );
    assert!(conditional.children.get().is_empty());
    assert_eq!(exiting.attached.get(), 0);
    assert!(!conditional.exit_cleanup_active.get());
    assert_full_rebuild_matches(&root, &ctx);
}

#[test]
fn detached_sidecar_opacity_stays_reactive_without_visible_rebuilds() {
    let (ctx, root) = mount(vec![probe(Kind::Canvas, vec![])]);
    let _graph = ctx.property_graph.enter();
    let parent = root.children.get()[0].clone();
    let sidecar = parent
        .create_children_detached(
            vec![(probe(Kind::Group, vec![]), parent.stack.clone())],
            &ctx,
            &Rc::downgrade(&parent),
        )
        .remove(0);
    parent.attach_sidecar_children(vec![sidecar.clone()], &ctx, &parent.parent_frame);
    settle(&root, &ctx);
    // A detached provider's inherited properties can be demanded by settings
    // consumers even though it never enters the mounted scene registry.
    sidecar.computed_opacity.get();
    let before = ctx.occlusion_stats();
    for opacity in [0.2, 0.4, 0.6, 1.0] {
        parent
            .get_common_properties()
            .borrow()
            .opacity
            .set(Some(opacity.into()));
        assert_eq!(sidecar.computed_opacity.get(), opacity);
        settle(&root, &ctx);
        assert_eq!(ctx.occlusion_stats().rebuilds, before.rebuilds);
        assert!(!ctx
            .occlusion_state
            .borrow()
            .records
            .contains_key(&sidecar.id.to_u32()));
    }
    assert_full_rebuild_matches(&root, &ctx);
}

#[test]
fn occlusion_eligibility_follows_mount_unmount_and_remount_not_viewport() {
    let (ctx, root) = mount(vec![probe(Kind::Native, vec![])]);
    let _graph = ctx.property_graph.enter();
    let parent = root.children.get()[0].clone();
    let original = parent.children.get();
    let node = parent
        .create_children_detached(
            vec![(probe(Kind::Canvas, vec![]), parent.stack.clone())],
            &ctx,
            &Rc::downgrade(&parent),
        )
        .remove(0);
    ctx.mark_node_occlusion_dirty(node.id.to_u32());
    assert!(!ctx
        .occlusion_state
        .borrow()
        .dirty
        .contains(&node.id.to_u32()));

    let mut mounted = original.clone();
    mounted.push(node.clone());
    attach(&parent, mounted.clone(), &ctx);
    // Registration, not an existing coverage record, establishes eligibility.
    ctx.mark_node_occlusion_dirty(node.id.to_u32());
    assert!(ctx
        .occlusion_state
        .borrow()
        .dirty
        .contains(&node.id.to_u32()));
    settle(&root, &ctx);
    assert!(ctx
        .occlusion_state
        .borrow()
        .records
        .contains_key(&node.id.to_u32()));
    node.get_common_properties()
        .borrow()
        .x
        .set(Some(Size::Pixels(2000.0.into())));
    settle(&root, &ctx);
    ctx.mark_node_occlusion_dirty(node.id.to_u32());
    assert!(ctx
        .occlusion_state
        .borrow()
        .dirty
        .contains(&node.id.to_u32()));
    settle(&root, &ctx);
    assert_full_rebuild_matches(&root, &ctx);

    attach(&parent, original, &ctx);
    settle(&root, &ctx);
    let before = ctx.occlusion_stats();
    ctx.mark_node_occlusion_dirty(node.id.to_u32());
    settle(&root, &ctx);
    assert_eq!(ctx.occlusion_stats().rebuilds, before.rebuilds);
    assert!(!ctx
        .occlusion_state
        .borrow()
        .records
        .contains_key(&node.id.to_u32()));
    attach(&parent, mounted, &ctx);
    settle(&root, &ctx);
    assert!(ctx
        .occlusion_state
        .borrow()
        .records
        .contains_key(&node.id.to_u32()));
    assert_full_rebuild_matches(&root, &ctx);
}

#[test]
fn insertion_and_removal_preserve_geometry_and_unchanged_masks() {
    let (ctx, root) = mount(vec![
        probe(Kind::Canvas, vec![]),
        probe(Kind::Native, vec![]),
    ]);
    let group = root.children.get()[0].clone();
    let old = group.children.get();
    let geometry = ctx.canvas_geometry_for_node(&old[0]);
    let before = ctx.occlusion_stats();
    let added = add(&group, Kind::Canvas, &ctx);
    settle(&root, &ctx);
    assert_eq!(ctx.occlusion_stats().rebuilds, before.rebuilds);
    assert_eq!(
        ctx.occlusion_stats().structural_reconciliations,
        before.structural_reconciliations + 1
    );
    assert!(Rc::ptr_eq(
        &geometry,
        &ctx.canvas_geometry_for_node(&old[0])
    ));
    assert!(!ctx.is_canvas_node_dirty(&old[0].id));
    assert!(ctx.is_canvas_node_dirty(&added.id));
    assert_full_rebuild_matches(&root, &ctx);
    clear(&ctx);
    let before = ctx.occlusion_stats();
    attach(&group, old, &ctx);
    settle(&root, &ctx);
    assert_eq!(ctx.occlusion_stats().rebuilds, before.rebuilds);
    assert_full_rebuild_matches(&root, &ctx);
}

#[test]
fn staged_prefixes_leave_room_for_a_visibility_branch() {
    fn layout(ids: &[u32], previous: &OcclusionTopology) -> OcclusionTopology {
        let mut topology = OcclusionTopology::default();
        for &id in ids {
            let index = topology.begin(id, None, 0, None, 1);
            topology.order.push((id, Occlusion::default()));
            topology.end(index, 1);
        }
        topology.assign_z_indices(previous);
        assert!(topology.order.windows(2).all(|pair| pair[0].1.z_index < pair[1].1.z_index));
        topology
    }
    let mut ids = Vec::new();
    let mut topology = OcclusionTopology::default();
    for stage in 0..25 {
        ids.splice(0..0, stage * 128..(stage + 1) * 128);
        topology = layout(&ids, &topology);
    }
    ids.splice(512..512, 4000..4015);
    let next = layout(&ids, &topology);
    for &(id, order) in &topology.order {
        assert_eq!(next.previous_order(id), Some(order));
    }
}

#[test]
fn insertion_keeps_native_and_canvas_orders_and_rebalances_exhausted_gaps() {
    let (ctx, root) = mount(vec![probe(Kind::Canvas, vec![]), probe(Kind::Native, vec![])]);
    let group = root.children.get()[0].clone();
    let siblings = group.children.get();
    let original = siblings.iter().map(|n| n.occlusion.get()).collect::<Vec<_>>();
    for i in 0..40 {
        let new = add(&group, Kind::Canvas, &ctx);
        let mut nodes = group.children.get();
        nodes.pop();
        nodes.insert(1, new);
        attach(&group, nodes.clone(), &ctx);
        settle(&root, &ctx);
        assert!(nodes.windows(2).all(|pair|
            pair[0].occlusion.get().z_index > pair[1].occlusion.get().z_index));
        if i == 0 {
            assert_eq!(siblings[0].occlusion.get(), original[0]);
            assert_eq!(siblings[1].occlusion.get(), original[1]);
        }
        assert_full_rebuild_matches(&root, &ctx);
    }
}

#[test]
fn native_insertion_reorder_opacity_and_removal_match_full_masks() {
    let (ctx, root) = mount(vec![
        probe(Kind::Canvas, vec![]),
        probe(Kind::Native, vec![]),
    ]);
    let group = root.children.get()[0].clone();
    let added = add(&group, Kind::Native, &ctx);
    settle(&root, &ctx);
    assert!(ctx.occlusion_stats().structural_reconciliations > 0);
    assert_ne!(added.native_mask_hash.get(), 0);
    assert_full_rebuild_matches(&root, &ctx);
    let before = ctx.occlusion_stats();
    let mut nodes = group.children.get();
    nodes.reverse();
    attach(&group, nodes, &ctx);
    settle(&root, &ctx);
    assert_eq!(ctx.occlusion_stats().rebuilds, before.rebuilds);
    assert_eq!(added.native_mask_hash.get(), 0);
    assert_full_rebuild_matches(&root, &ctx);
    group.children.get()[2]
        .get_common_properties()
        .borrow()
        .opacity
        .set(Some(0.35.into()));
    add(&group, Kind::Canvas, &ctx);
    settle(&root, &ctx);
    assert_full_rebuild_matches(&root, &ctx);
    let remaining: Vec<_> = group
        .children
        .get()
        .into_iter()
        .filter(|n| n.id != added.id)
        .collect();
    attach(&group, remaining, &ctx);
    settle(&root, &ctx);
    assert_full_rebuild_matches(&root, &ctx);
}

#[test]
fn leaf_changes_inside_nested_clips_reuse_exact_coverage() {
    let (ctx, root) = mount(vec![probe(
        Kind::Clip,
        vec![probe(
            Kind::Clip,
            vec![probe(Kind::Canvas, vec![]), probe(Kind::Native, vec![])],
        )],
    )]);
    let inner = root.children.get()[0].children.get()[0].children.get()[0].clone();
    let before = ctx.occlusion_stats();
    add(&inner, Kind::Native, &ctx);
    settle(&root, &ctx);
    assert_eq!(ctx.occlusion_stats().rebuilds, before.rebuilds);
    assert_full_rebuild_matches(&root, &ctx);
}

#[test]
fn inserting_a_rounded_clip_repairs_shared_and_neighbor_masks_in_a_scroller() {
    let (ctx, root) = mount(vec![retained_probe(
        Kind::Scroller,
        vec![retained_probe(Kind::Native, vec![])],
    )]);
    let scroller = root.children.get()[0].children.get()[0].clone();
    let neighbor = scroller.children.get()[0].clone();
    scroll_surface(&ctx, &scroller, 5.);
    settle(&root, &ctx);
    clear(&ctx);

    let clip = scroller
        .create_children_detached(
            vec![(
                retained_probe(
                    Kind::Clip,
                    vec![
                        retained_probe(Kind::Canvas, vec![]),
                        retained_probe(Kind::Native, vec![]),
                    ],
                ),
                scroller.stack.clone(),
            )],
            &ctx,
            &Rc::downgrade(&scroller),
        )
        .remove(0);
    let before = ctx.occlusion_stats();
    attach(&scroller, vec![clip.clone(), neighbor.clone()], &ctx);
    settle(&root, &ctx);
    assert_eq!(ctx.occlusion_stats().rebuilds, before.rebuilds);
    let inner = clip.children.get()[1].clone();
    assert_eq!(inner.occlusion.get().parent_frame, Some(clip.id.to_u32()));
    let masks: HashMap<_, _> = ctx
        .take_native_messages()
        .into_iter()
        .filter_map(|message| match message {
            pax_message::NativeMessage::NativeMaskUpdate(patch) => Some((patch.id, patch.entries)),
            _ => None,
        })
        .collect();
    // The native child shares the clip with the canvas. Its neighbor sees
    // the same occluder through the rounded path.
    let shared = &masks[&inner.id.to_u32()];
    let external = &masks[&neighbor.id.to_u32()];
    assert_eq!(shared.len(), 1);
    assert!(shared[0].clips.is_empty());
    assert_eq!(external.len(), 1);
    assert_eq!(external[0].clips.len(), 1);
    // Scroll with the membership produced by insertion, before the full-rebuild comparison.
    scroll_surface(&ctx, &scroller, 19.);
    settle(&root, &ctx);
    assert_eq!(ctx.occlusion_stats().rebuilds, before.rebuilds);
    assert_full_rebuild_matches(&root, &ctx);
}

#[test]
fn affine_clip_and_isolated_scroller_insertions_reconcile() {
    for kind in [Kind::Clip, Kind::Scroller] {
        let (ctx, root) = mount(vec![
            probe(Kind::Canvas, vec![]),
            probe(Kind::Native, vec![]),
        ]);
        let group = root.children.get()[0].clone();
        let before = ctx.occlusion_stats();
        let container = add(&group, kind, &ctx);
        settle(&root, &ctx);
        assert_eq!(ctx.occlusion_stats().rebuilds, before.rebuilds);
        assert_full_rebuild_matches(&root, &ctx);
        let before = ctx.occlusion_stats();
        let leaf = group.children.get()[0].clone();
        crate::NodeRef::new(&leaf, &ctx)
            .move_to_append_child(&crate::NodeRef::new(&container, &ctx))
            .unwrap();
        settle(&root, &ctx);
        assert_eq!(ctx.occlusion_stats().rebuilds, before.rebuilds);
        assert_full_rebuild_matches(&root, &ctx);
    }
}

#[test]
fn inserting_nested_scrollers_reassigns_existing_surfaces_and_preserves_displayed_hits() {
    use crate::properties::input_capture::InputPublisher;
    use pax_message::NativeMessage;

    let branch = || {
        retained_probe(
            Kind::Scroller,
            vec![
                retained_probe(Kind::Canvas, vec![]),
                retained_probe(Kind::Native, vec![]),
            ],
        )
    };
    let (ctx, root) = mount(vec![branch(), branch()]);
    ctx.register_root_expanded_node(&root);
    ctx.edit_globals(|globals| globals.platform = Platform::Web);
    let group = root.children.get()[0].clone();
    settle_occlusion_effects(&root, &ctx);
    ctx.take_native_messages();
    let original = group.children.get();
    let existing_layer = original[0].browser_content_layer_id.get().unwrap();
    let inserted = group
        .create_children_detached(
            vec![(
                retained_probe(Kind::Scroller, vec![branch()]),
                group.stack.clone(),
            )],
            &ctx,
            &Rc::downgrade(&group),
        )
        .remove(0);
    let before = ctx.occlusion_stats();
    attach(
        &group,
        vec![original[0].clone(), inserted.clone(), original[1].clone()],
        &ctx,
    );
    settle_occlusion_effects(&root, &ctx);
    assert_eq!(ctx.occlusion_stats().rebuilds, before.rebuilds);
    let reused = ctx.occlusion_stats().topology_nodes_reused - before.topology_nodes_reused;
    assert!(
        reused >= 4,
        "existing scroller content should reuse topology, reused {reused}"
    );
    let nested = inserted.children.get()[0].clone();
    let messages = ctx.take_native_messages();
    for host in [&original[0], &inserted, &nested] {
        let layer = host.browser_content_layer_id.get().unwrap();
        assert_eq!(ctx.get_layer_scroller_owner(layer as usize), Some(host.id));
        assert!(messages
            .iter()
            .any(|m| matches!(m, NativeMessage::ScrollerUpdate(p)
            if p.id == host.id.to_u32() && p.content_layer_id == Some(layer))));
    }
    assert_eq!(
        original[0].browser_content_layer_id.get(),
        Some(existing_layer + 2)
    );
    assert_eq!(ctx.layer_count.get(), 5);
    assert!(messages
        .iter()
        .any(|m| matches!(m, NativeMessage::ShrinkLayersTo(5))));
    let hits = || {
        settle_occlusion_effects(&root, &ctx);
        let view = InputPublisher::default().capture(&ctx).unwrap();
        [(8., 8.), (40., 40.), (301., 201.)].map(|point| {
            view.hit_targets(point.into(), &HashMap::new(), true)
                .unwrap()
                .into_iter()
                .map(|hit| (hit.identity.id, hit.identity.mount_epoch))
                .collect::<Vec<_>>()
        })
    };
    let incremental_hits = hits();
    assert!(incremental_hits[0]
        .iter()
        .any(|(id, _)| *id == nested.children.get()[1].id.to_u32()));
    assert_full_rebuild_matches(&root, &ctx);
    assert_eq!(hits(), incremental_hits);
    assert!(!ctx.take_native_messages().iter().any(|m| matches!(m,
        NativeMessage::ScrollerUpdate(p) if p.content_layer_id.is_some())));
}

#[test]
fn leaf_admission_in_a_retained_scroller_matches_full_classification() {
    let run = |full: bool| {
        let (ctx, root) = mount(vec![retained_probe(Kind::Scroller, vec![])]);
        let host = root.children.get()[0].children.get()[0].clone();
        settle_occlusion_effects(&root, &ctx);
        let leaves = host.create_children_detached(
            [Kind::Canvas, Kind::Native]
                .into_iter()
                .map(|kind| (retained_probe(kind, vec![]), host.stack.clone())),
            &ctx,
            &Rc::downgrade(&host),
        );
        attach(&host, leaves.clone(), &ctx);
        if full {
            ctx.occlusion_state.borrow_mut().records.clear();
            ctx.mark_occlusion_dirty();
        }
        settle_occlusion_effects(&root, &ctx);
        assert!(leaves.iter().all(|leaf| leaf.local_compositing.get()));
        let mask = leaves[1].native_mask_hash.get();
        assert_ne!(mask, 0);
        let before = ctx.occlusion_stats();
        host.get_common_properties()
            .borrow()
            .x
            .set(Some(Size::Pixels(25.into())));
        settle_occlusion_effects(&root, &ctx);
        assert_eq!(leaves[1].native_mask_hash.get(), mask);
        let after = ctx.occlusion_stats();
        (
            mask,
            after.records_updated - before.records_updated,
            after.masks_evaluated - before.masks_evaluated,
        )
    };
    assert_eq!(run(false), run(true));
}

#[test]
fn shifted_scroller_layers_keep_coverage_indices_and_native_masks() {
    let shelf = || {
        retained_probe(
            Kind::Scroller,
            (0..16)
                .flat_map(|_| {
                    [
                        retained_probe(Kind::Canvas, vec![]),
                        retained_probe(Kind::Native, vec![]),
                    ]
                })
                .collect(),
        )
    };
    let (ctx, root) = mount(vec![shelf(), shelf(), shelf()]);
    let group = root.children.get()[0].clone();
    let hosts = group.children.get();
    settle_occlusion_effects(&root, &ctx);
    let probe = [Rect::new(4., 4., 5., 5.)];
    let indices = |layer| {
        let mut lists = [
            ctx.canvas_nodes_intersecting(layer, &probe)
                .unwrap_or_default(),
            ctx.compositing_nodes_intersecting(layer, &probe, false),
            ctx.compositing_nodes_intersecting(layer, &probe, true),
        ];
        for ids in &mut lists {
            ids.sort_unstable();
        }
        lists
    };
    let snapshot = |host: &ExpandedNode| {
        let children = host.children.get();
        let geometry = ctx.canvas_geometry_for_node(&children[0]);
        let mask = children[1].native_mask_hash.get();
        let lists = indices(geometry.layer);
        assert!(lists[0].contains(&children[0].id.to_u32()));
        assert!(lists[1].contains(&children[0].id.to_u32()));
        assert!(lists[2].contains(&children[1].id.to_u32()));
        assert_ne!(mask, 0);
        (geometry, mask, lists)
    };
    let before: Vec<_> = hosts.iter().map(|host| snapshot(host)).collect();
    clear(&ctx);
    ctx.take_canvas_node_removals();
    let stats = ctx.occlusion_stats();
    let inserted = add(&group, Kind::Scroller, &ctx);
    settle_occlusion_effects(&root, &ctx);
    let after = ctx.occlusion_stats();
    assert_eq!(after.rebuilds, stats.rebuilds);
    let updated = after.records_updated - stats.records_updated;
    assert!(updated < 8, "{updated} coverage records refreshed");
    let evaluated = after.masks_evaluated - stats.masks_evaluated;
    assert!(evaluated < 4, "{evaluated} masks evaluated");
    let removals = ctx.take_canvas_node_removals();
    let draws = ctx.dirty_canvas_node_ids();
    for (host, (old, old_mask, old_indices)) in hosts.iter().zip(before) {
        let (new, mask, indices) = snapshot(host);
        for canvas in host.children.get().iter().step_by(2) {
            assert!(removals.contains(&(old.layer, canvas.id.to_u32())));
            assert!(draws.contains(&canvas.id));
        }
        assert_eq!(new.layer, old.layer + 1);
        assert_eq!(new.local_paint_revision, old.local_paint_revision);
        assert_eq!(new.surface_transform, old.surface_transform);
        assert_eq!(mask, old_mask);
        assert_eq!(indices, old_indices);
    }
    let new_layer = ctx.occlusion_state.borrow().owned_layers[&inserted.id.to_u32()];
    assert!(indices(new_layer).iter().all(Vec::is_empty));
    assert_full_rebuild_matches(&root, &ctx);
}

#[test]
fn layer_renumber_repairs_concurrent_edits_and_transfer_to_the_vacated_number() {
    let shelf = || {
        retained_probe(
            Kind::Scroller,
            vec![
                retained_probe(Kind::Canvas, vec![]),
                retained_probe(Kind::Native, vec![]),
            ],
        )
    };
    let (ctx, root) = mount(vec![shelf(), shelf(), shelf()]);
    let group = root.children.get()[0].clone();
    let hosts = group.children.get();
    settle_occlusion_effects(&root, &ctx);
    let edited = hosts[0].children.get();
    let removed = hosts[1].children.get();
    let source = hosts[2].children.get();
    assert_ne!(edited[1].native_mask_hash.get(), 0);
    assert_ne!(source[1].native_mask_hash.get(), 0);
    assert_ne!(removed[1].native_mask_hash.get(), 0);
    let old_layer = source[0].occlusion.get().render_layer_id;
    let before = ctx.occlusion_stats();
    let destination = add(&group, Kind::Scroller, &ctx);
    let target = add(&destination, Kind::Native, &ctx);
    // One canvas changes owners but keeps its number, another moves away,
    // and a third unmounts. Repair old overlaps on all three shifted shelves.
    crate::NodeRef::new(&source[0], &ctx)
        .move_to_insert_before(&crate::NodeRef::new(&target, &ctx))
        .unwrap();
    attach(&hosts[1], vec![removed[1].clone()], &ctx);
    edited[0]
        .get_common_properties()
        .borrow()
        .x
        .set(Some(Size::Pixels(400.into())));
    settle_occlusion_effects(&root, &ctx);
    assert_eq!(ctx.occlusion_stats().rebuilds, before.rebuilds);
    assert_eq!(source[0].occlusion.get().render_layer_id, old_layer);
    assert_eq!(source[0].canvas_owner.get(), Some(destination.id.to_u32()));
    assert_eq!(edited[1].native_mask_hash.get(), 0);
    assert_eq!(source[1].native_mask_hash.get(), 0);
    assert_eq!(removed[1].native_mask_hash.get(), 0);
    assert_ne!(target.native_mask_hash.get(), 0);
    let region = [Rect::new(3., 3., 4., 4.)];
    assert!(ctx
        .compositing_nodes_intersecting(old_layer, &region, false)
        .contains(&source[0].id.to_u32()));
    assert!(ctx
        .compositing_nodes_intersecting(old_layer, &region, true)
        .contains(&target.id.to_u32()));
    assert_full_rebuild_matches(&root, &ctx);
}

#[test]
fn adding_clipped_scrollers_extends_only_their_membership() {
    let distant_canvas = make_probe(Kind::Canvas, vec![], true);
    let (ctx, root) = mount(vec![
        retained_probe(Kind::Scroller, vec![retained_probe(Kind::Native, vec![])]),
        retained_probe(Kind::Scroller, vec![distant_canvas.clone()]),
    ]);
    let group = root.children.get()[0].clone();
    let parent = group.children.get()[0].clone();
    settle_occlusion_effects(&root, &ctx);
    let removed = parent.children.get()[0].clone();
    let checks = distant_canvas.placement_checks.get();
    let before = ctx.occlusion_stats();
    let clip = parent
        .create_children_detached(
            [(
                retained_probe(
                    Kind::Clip,
                    vec![retained_probe(
                        Kind::Scroller,
                        vec![
                            retained_probe(Kind::Canvas, vec![]),
                            retained_probe(Kind::Native, vec![]),
                        ],
                    )],
                ),
                parent.stack.clone(),
            )],
            &ctx,
            &Rc::downgrade(&parent),
        )
        .remove(0);
    attach(&parent, vec![clip.clone()], &ctx);
    settle_occlusion_effects(&root, &ctx);
    assert_eq!(ctx.occlusion_stats().rebuilds, before.rebuilds);
    assert_eq!(distant_canvas.placement_checks.get(), checks);
    let child_host = clip.children.get()[0].clone();
    {
        let state = ctx.occlusion_state.borrow();
        assert!(!state.leaf_scroll_regions.contains(&parent.id.to_u32()));
        assert!(state.leaf_scroll_regions.contains(&child_host.id.to_u32()));
        assert!(!state.clip_dependents[&parent.id.to_u32()].contains(&removed.id.to_u32()));
        assert!(state.scroll_containers[&parent.id.to_u32()].contains(&child_host.id.to_u32()));
    }
    assert_full_rebuild_matches(&root, &ctx);
    assert!(distant_canvas.placement_checks.get() > checks);
}

#[test]
fn remount_before_settlement_replaces_the_leafs_old_clip_links() {
    let (ctx, root) = mount(vec![retained_probe(
        Kind::Scroller,
        vec![
            retained_probe(Kind::Clip, vec![retained_probe(Kind::Canvas, vec![])]),
            retained_probe(Kind::Clip, vec![]),
        ],
    )]);
    let host = root.children.get()[0].children.get()[0].clone();
    let clips = host.children.get();
    let leaf = clips[0].children.get()[0].clone();
    settle_occlusion_effects(&root, &ctx);
    let before = ctx.occlusion_stats();
    attach(&clips[0], vec![], &ctx);
    attach(&clips[1], vec![leaf.clone()], &ctx);
    settle_occlusion_effects(&root, &ctx);
    assert_eq!(ctx.occlusion_stats().rebuilds, before.rebuilds);
    assert_full_rebuild_matches(&root, &ctx);
}

#[test]
fn simultaneous_insertion_and_collapse_reclassify_the_existing_scroller() {
    let (ctx, root) = mount(vec![retained_probe(
        Kind::Scroller,
        vec![retained_probe(Kind::Canvas, vec![])],
    )]);
    let group = root.children.get()[0].clone();
    let host = group.children.get()[0].clone();
    settle_occlusion_effects(&root, &ctx);
    assert!(host.children.get()[0].local_compositing.get());
    host.get_common_properties()
        .borrow()
        .scale_x
        .set(Some(Size::Percent(0.into())));
    add(&group, Kind::Scroller, &ctx);
    settle_occlusion_effects(&root, &ctx);
    assert!(!host.children.get()[0].local_compositing.get());
    assert_full_rebuild_matches(&root, &ctx);
}

#[test]
fn inserting_a_shared_or_singular_scroll_surface_uses_the_full_rebuild() {
    for allow_layer in [false, true] {
        let (ctx, root) = mount(vec![probe(Kind::Native, vec![])]);
        ctx.globals()
            .browser_allows_scroller_vector_layers
            .set(allow_layer);
        let group = root.children.get()[0].clone();
        let before = ctx.occlusion_stats();
        let scroller = add(&group, Kind::Scroller, &ctx);
        if allow_layer {
            scroller
                .get_common_properties()
                .borrow()
                .scale_x
                .set(Some(Size::Percent(0.into())));
        }
        settle(&root, &ctx);
        assert!(ctx.occlusion_stats().rebuilds > before.rebuilds);
        assert_eq!(
            ctx.occlusion_stats().structural_reconciliations,
            before.structural_reconciliations
        );
        assert_full_rebuild_matches(&root, &ctx);
    }
}

#[test]
fn transferred_clip_subtree_repairs_both_scroll_islands_and_retains_distant_geometry() {
    let (ctx, root) = mount(vec![
        probe(
            Kind::Scroller,
            vec![
                probe(
                    Kind::Group,
                    vec![probe(
                        Kind::Clip,
                        vec![probe(Kind::Canvas, vec![]), probe(Kind::Native, vec![])],
                    )],
                ),
                probe(Kind::Native, vec![]),
            ],
        ),
        probe(Kind::Scroller, vec![probe(Kind::Native, vec![])]),
        probe(
            Kind::Scroller,
            (0..32).map(|_| probe(Kind::Canvas, vec![])).collect(),
        ),
    ]);
    let hosts = root.children.get()[0].children.get();
    let moved = hosts[0].children.get()[0].clone();
    let clip = moved.children.get()[0].clone();
    let canvas = clip.children.get()[0].clone();
    let distant = hosts[2].children.get()[0].clone();
    let handle = crate::NodeRef::new(&moved, &ctx);
    for destination in [&hosts[1], &hosts[0], &hosts[1]] {
        let geometry = ctx.canvas_geometry_for_node(&distant);
        let before = ctx.occlusion_stats();
        handle
            .move_to_append_child(&crate::NodeRef::new(destination, &ctx))
            .unwrap();
        settle(&root, &ctx);
        assert_eq!(ctx.occlusion_stats().rebuilds, before.rebuilds);
        assert!(ctx.occlusion_stats().topology_nodes_reused > before.topology_nodes_reused);
        assert!(Rc::ptr_eq(
            &geometry,
            &ctx.canvas_geometry_for_node(&distant)
        ));
        assert!(ctx.occlusion_state.borrow().records[&canvas.id.to_u32()]
            .clips
            .contains(&destination.id.to_u32()));
        assert_full_rebuild_matches(&root, &ctx);
        scroll_surface(&ctx, destination, 25.);
        settle(&root, &ctx);
        assert_full_rebuild_matches(&root, &ctx);
        clear(&ctx);
    }
}

#[test]
fn overlay_flights_repair_departure_arrival_reversal_and_teardown() {
    use crate::api::{Duration, EasingCurve};
    let (mut engine, root, handle, target, nav) =
        crate::node_query::tests::flight_fixture_with_content(probe(
            Kind::Clip,
            vec![probe(Kind::Canvas, vec![]), probe(Kind::Native, vec![])],
        ));
    let ctx = engine.runtime_context.clone();
    let moved = handle.resolve().unwrap().0;
    let clip = moved.children.get()[0].clone();
    let canvas = clip.children.get()[0].clone();
    for returning in [false, true, false] {
        let before = ctx.occlusion_stats();
        let motion = handle.animated(Duration::Frames(4.into()), EasingCurve::Linear);
        if returning {
            motion.move_to_home().unwrap();
        } else {
            motion.move_to_append_child(&target).unwrap();
        }
        for _ in 0..5 {
            let before_tick = ctx.occlusion_stats();
            engine.tick();
            assert_eq!(ctx.occlusion_stats().rebuilds, before_tick.rebuilds);
            assert_full_rebuild_matches(&root, &ctx);
        }
        assert!(
            ctx.occlusion_stats().structural_reconciliations > before.structural_reconciliations
        );
        assert!(!moved.move_in_flight.get());
        assert_eq!(canvas.occlusion.get().render_layer_id == 0, !returning);
    }
    handle
        .animated(Duration::Frames(8.into()), EasingCurve::Linear)
        .move_to_home()
        .unwrap();
    engine.tick();
    handle
        .animated(Duration::Frames(4.into()), EasingCurve::Linear)
        .move_to_append_child(&target)
        .unwrap();
    engine.tick();
    assert_full_rebuild_matches(&root, &ctx);
    nav.recurse_unmount(&ctx);
    engine.tick();
    assert_full_rebuild_matches(&root, &ctx);
    handle.move_to_home().unwrap();
    engine.tick();
    assert_full_rebuild_matches(&root, &ctx);
}

#[test]
fn structural_edit_and_moving_leaf_repair_old_and_new_overlaps() {
    let (ctx, root) = mount(vec![
        probe(Kind::Canvas, vec![]),
        probe(Kind::Native, vec![]),
    ]);
    let group = root.children.get()[0].clone();
    group.children.get()[0]
        .get_common_properties()
        .borrow()
        .x
        .set(Some(Size::Pixels(400.0.into())));
    add(&group, Kind::Native, &ctx);
    settle(&root, &ctx);
    assert_eq!(group.children.get()[1].native_mask_hash.get(), 0);
    assert_full_rebuild_matches(&root, &ctx);
}

#[cfg(feature = "3d")]
#[test]
fn overlay_transfer_repairs_camera_projection_and_clip_membership() {
    use pax_manifest::{
        ExpressionInfo, LiteralBlockDefinition, SettingElement, Token, ValueDefinition,
    };
    let camera = LiteralBlockDefinition::new(
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
    );
    let (mut engine, root, handle, target, _) =
        crate::node_query::tests::flight_fixture_with_content_and_camera(
            probe(
                Kind::Clip,
                vec![probe(Kind::Canvas, vec![]), probe(Kind::Native, vec![])],
            ),
            Some(camera),
        );
    let ctx = engine.runtime_context.clone();
    let moved = handle.resolve().unwrap().0;
    let clip = moved.children.get()[0].clone();
    let canvas = clip.children.get()[0].clone();
    canvas
        .get_common_properties()
        .borrow()
        .z
        .set(Some(25.0.into()));
    engine.tick();
    assert!(canvas.has_perspective_camera());
    assert_full_rebuild_matches(&root, &ctx);
    for in_overlay in [true, false, true, false] {
        let old_geometry = ctx.canvas_geometry_for_node(&canvas);
        let before = ctx.occlusion_stats();
        if in_overlay {
            handle.move_to_append_child(&target).unwrap();
        } else {
            handle.move_to_home().unwrap();
        }
        engine.tick();
        assert_eq!(ctx.occlusion_stats().rebuilds, before.rebuilds);
        assert_eq!(canvas.has_perspective_camera(), !in_overlay);
        assert!(!Rc::ptr_eq(
            &old_geometry,
            &ctx.canvas_geometry_for_node(&canvas)
        ));
        assert_full_rebuild_matches(&root, &ctx);
    }
}

#[test]
fn leaf_edits_do_not_evaluate_distant_native_masks() {
    let (ctx, root) = mount(vec![
        probe(Kind::Group, vec![probe(Kind::Native, vec![])]),
        probe(Kind::Group, vec![probe(Kind::Native, vec![])]),
    ]);
    let groups = root.children.get()[0].children.get();
    groups[1]
        .get_common_properties()
        .borrow()
        .x
        .set(Some(Size::Pixels(800.0.into())));
    settle(&root, &ctx);
    clear(&ctx);
    let before = ctx.occlusion_stats();
    add(&groups[0], Kind::Canvas, &ctx);
    settle(&root, &ctx);
    let after = ctx.occlusion_stats();
    assert_eq!(after.rebuilds, before.rebuilds);
    assert_eq!(after.masks_evaluated - before.masks_evaluated, 1);
    assert_full_rebuild_matches(&root, &ctx);
}

#[test]
fn backend_scroller_policy_changes_and_reload_keep_the_full_fallback() {
    let (ctx, root) = mount(vec![probe(
        Kind::Scroller,
        vec![probe(Kind::Canvas, vec![])],
    )]);
    let group = root.children.get()[0].clone();
    assert_eq!(ctx.layer_count.get(), 2);
    let before = ctx.occlusion_stats();
    ctx.globals()
        .browser_allows_scroller_vector_layers
        .set(false);
    ctx.mark_occlusion_dirty();
    settle(&root, &ctx);
    assert_eq!(ctx.layer_count.get(), 1);
    assert!(ctx.occlusion_stats().rebuilds > before.rebuilds);
    let before = ctx.occlusion_stats();
    group.children.get()[0]
        .recreate_with_new_data(probe(Kind::Clip, vec![probe(Kind::Native, vec![])]), &ctx);
    settle(&root, &ctx);
    assert!(ctx.occlusion_stats().rebuilds > before.rebuilds);
    assert_full_rebuild_matches(&root, &ctx);
}

#[test]
fn finishing_a_mount_reuses_siblings_when_a_detached_provider_evaluates_children() {
    let (ctx, root) = mount(vec![
        probe(Kind::Group, vec![probe(Kind::Native, vec![])]),
        probe(
            Kind::Group,
            (0..32).map(|_| probe(Kind::Canvas, vec![])).collect(),
        ),
    ]);
    let _graph = ctx.property_graph.enter();
    let parent = root.children.get()[0].children.get()[0].clone();
    let before = ctx.occlusion_stats();
    let child = add(&parent, Kind::Clip, &ctx);
    let sidecar = child
        .create_children_detached(
            vec![(probe(Kind::Group, vec![]), child.stack.clone())],
            &ctx,
            &Rc::downgrade(&child),
        )
        .remove(0);
    child.attach_sidecar_children(vec![sidecar.clone()], &ctx, &child.parent_frame);
    sidecar.children_listener.get();
    parent.finish_pending_child(&child, &ctx);
    settle(&root, &ctx);

    let after = ctx.occlusion_stats();
    assert_eq!(after.rebuilds, before.rebuilds);
    assert_eq!(
        after.structural_reconciliations,
        before.structural_reconciliations + 1
    );
    assert!(after.topology_nodes_reused - before.topology_nodes_reused >= 33);
    assert_full_rebuild_matches(&root, &ctx);
}

#[test]
fn unmounted_projected_condition_updates_occlusion_through_its_mounted_slot() {
    use pax_runtime_api::{pax_value::ToFromPaxAny, LocalProperty};

    let visible = LocalProperty::new(true);
    let condition = visible.clone();
    let mut conditional = args(vec![probe(Kind::Canvas, vec![])]);
    conditional.prototypical_properties = PropertiesInit::Factory(Box::new(move |_, existing| {
        existing.is_none().then(|| {
            Rc::new(RefCell::new(
                crate::ConditionalProperties {
                    boolean_expression: condition.clone(),
                    ..Default::default()
                }
                .to_pax_any(),
            ))
        })
    }));
    let mut slot = args(vec![]);
    slot.prototypical_properties = PropertiesInit::Factory(Box::new(|_, existing| {
        existing.is_none().then(|| {
            Rc::new(RefCell::new(
                crate::slot::Slot {
                    is_remainder: LocalProperty::new(true),
                    ..Default::default()
                }
                .to_pax_any(),
            ))
        })
    }));
    let mut receiver = args(vec![crate::ConditionalInstance::instantiate(conditional)]);
    receiver.component_template = Some(RefCell::new(vec![crate::SlotInstance::instantiate(slot)]));
    let (ctx, root) = mount(vec![
        ComponentInstance::instantiate(receiver),
        probe(Kind::Native, vec![]),
    ]);
    let children = root.children.get()[0].children.get();
    let controller = children[0]
        .expanded_projected_children
        .borrow()
        .as_ref()
        .unwrap()[0]
        .clone();
    assert_eq!(controller.attached.get(), 0);
    let neighbor = &children[1];
    let covered = neighbor.native_mask_hash.get();
    for show in [false, true] {
        visible.set(show);
        ctx.drain_node_effects();
        settle(&root, &ctx);
        assert_eq!(neighbor.native_mask_hash.get() == covered, show);
        assert_full_rebuild_matches(&root, &ctx);
    }
}

#[test]
fn localized_edits_reuse_unaffected_nested_islands_and_match_full_reconciliation() {
    let (ctx, root) = mount(
        (0..12)
            .map(|_| {
                probe(
                    Kind::Scroller,
                    vec![probe(
                        Kind::Clip,
                        (0..20)
                            .map(|i| {
                                probe(
                                    if i % 2 == 0 {
                                        Kind::Canvas
                                    } else {
                                        Kind::Native
                                    },
                                    vec![],
                                )
                            })
                            .collect(),
                    )],
                )
            })
            .collect(),
    );
    let groups = root.children.get()[0].children.get();
    for step in 0..12 {
        let group = groups[step % groups.len()].children.get()[0].clone();
        let before = ctx.occlusion_stats();
        let original = group.children.get();
        add(&group, Kind::Canvas, &ctx);
        settle(&root, &ctx);
        let after = ctx.occlusion_stats();
        assert_eq!(after.rebuilds, before.rebuilds);
        assert!(after.topology_nodes_visited - before.topology_nodes_visited < 10);
        assert!(after.topology_nodes_reused - before.topology_nodes_reused > 250);
        assert_full_rebuild_matches(&root, &ctx);

        let mut reversed = group.children.get();
        reversed.reverse();
        attach(&group, reversed, &ctx);
        settle(&root, &ctx);
        assert_full_rebuild_matches(&root, &ctx);

        attach(&group, original, &ctx);
        settle(&root, &ctx);
        assert_full_rebuild_matches(&root, &ctx);
    }
}

#[test]
fn retiring_mask_sources_invalidates_the_owner_and_reuses_unrelated_branches() {
    let (ctx, root) = mount(vec![
        probe(Kind::Clip, vec![probe(Kind::Canvas, vec![])]),
        probe(
            Kind::Group,
            (0..32).map(|_| probe(Kind::Native, vec![])).collect(),
        ),
    ]);
    let group = root.children.get()[0].clone();
    let owner = group.children.get()[0].clone();
    let source = owner
        .create_children_detached(
            [(
                probe(Kind::Group, vec![probe(Kind::Canvas, vec![])]),
                owner.stack.clone(),
            )],
            &ctx,
            &Rc::downgrade(&owner),
        )
        .remove(0);
    source.mount_as_render_source(&owner, &ctx);
    settle(&root, &ctx);
    assert!(!ctx
        .occlusion_state
        .borrow()
        .contains_node(source.id.to_u32()));
    let before = ctx.occlusion_stats();
    source.recurse_unmount(&ctx);
    assert!(ctx
        .occlusion_state
        .borrow()
        .dirty
        .contains(&owner.id.to_u32()));
    settle(&root, &ctx);
    let after = ctx.occlusion_stats();
    assert_eq!(before.rebuilds, after.rebuilds);
    assert!(after.topology_nodes_reused - before.topology_nodes_reused >= 32);
    assert_full_rebuild_matches(&root, &ctx);
}

#[test]
fn local_edits_cannot_hide_dirty_siblings_or_unknown_structure() {
    let (ctx, root) = mount(vec![
        probe(Kind::Group, vec![probe(Kind::Native, vec![])]),
        probe(
            Kind::Clip,
            vec![probe(Kind::Canvas, vec![]), probe(Kind::Native, vec![])],
        ),
    ]);
    let groups = root.children.get()[0].children.get();
    groups[0]
        .get_common_properties()
        .borrow()
        .x
        .set(Some(Size::Pixels(400.0.into())));
    settle(&root, &ctx);
    add(&groups[0], Kind::Canvas, &ctx);
    groups[1].children.get()[0]
        .get_common_properties()
        .borrow()
        .x
        .set(Some(Size::Pixels(100.0.into())));
    settle(&root, &ctx);
    assert_full_rebuild_matches(&root, &ctx);

    let before = ctx.occlusion_stats();
    ctx.mark_occlusion_dirty();
    update_node_occlusion(&root, &ctx);
    let after = ctx.occlusion_stats();
    assert_eq!(after.topology_nodes_reused, before.topology_nodes_reused);
    assert!(after.topology_nodes_visited > before.topology_nodes_visited);
    assert_full_rebuild_matches(&root, &ctx);
}

#[test]
fn successive_local_edits_keep_retained_ranges_and_native_before_children_order_valid() {
    let (ctx, root) = mount(
        (0..6)
            .map(|_| {
                probe(
                    Kind::BeforeNative,
                    vec![probe(Kind::Canvas, vec![]), probe(Kind::Native, vec![])],
                )
            })
            .collect(),
    );
    let groups = root.children.get()[0].children.get();
    let before = ctx.occlusion_stats();
    for step in 0..30 {
        let group = &groups[(step * 7) % groups.len()];
        add(group, Kind::Canvas, &ctx);
        settle(&root, &ctx);
        let mut children = group.children.get();
        children.reverse();
        attach(group, children.clone(), &ctx);
        settle(&root, &ctx);
        if step % 3 == 0 {
            children.pop();
            attach(group, children, &ctx);
            settle(&root, &ctx);
        }
    }
    let after = ctx.occlusion_stats();
    assert_eq!(after.rebuilds, before.rebuilds);
    assert!(after.topology_nodes_reused > before.topology_nodes_reused);
    assert_full_rebuild_matches(&root, &ctx);
}

#[test]
fn same_pass_remount_restores_order_even_when_the_retained_position_is_unchanged() {
    let (ctx, root) = mount(vec![probe(
        Kind::Scroller,
        vec![probe(Kind::Canvas, vec![]), probe(Kind::Native, vec![])],
    )]);
    let scroller = root.children.get()[0].children.get()[0].clone();
    let original = scroller.children.get();
    let canvas = &original[0];
    let order = canvas.occlusion.get();
    assert_ne!(order, Default::default());
    attach(&scroller, vec![original[1].clone()], &ctx);
    attach(&scroller, original.clone(), &ctx);
    assert_eq!(
        canvas.occlusion.get(),
        Default::default(),
        "remount resets presentation"
    );
    settle(&root, &ctx);
    assert_eq!(canvas.occlusion.get(), order);
    assert_full_rebuild_matches(&root, &ctx);
}
