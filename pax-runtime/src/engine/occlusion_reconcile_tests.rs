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
    Clip,
    Scroller,
}

struct Probe {
    base: BaseInstance,
    kind: Kind,
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
        template_node_identifier: None,
        template_node_type_id: None,
        template_node_selector_info: None,
        transition_config: Default::default(),
        properties_scope: PropertiesScopeInit::None,
    }
}

fn probe(kind: Kind, children: Vec<Rc<dyn InstanceNode>>) -> Rc<dyn InstanceNode> {
    Rc::new(Probe {
        kind,
        base: BaseInstance::new(
            args(children),
            InstanceFlags {
                invisible_to_slot: false,
                invisible_to_raycasting: true,
                is_component: false,
                is_slot: false,
                layer: match kind {
                    Kind::Canvas => Layer::Canvas,
                    Kind::Native | Kind::Scroller => Layer::Native,
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
    fn clips_content(&self, _: &ExpandedNode) -> bool {
        matches!(self.kind, Kind::Scroller | Kind::Clip)
    }
    fn resolve_occlusion_path(&self, node: &ExpandedNode) -> Option<BezPath> {
        matches!(self.kind, Kind::Canvas).then(|| {
            Affine::from(node.transform_and_bounds.get().transform)
                * Rect::new(0., 0., 40., 40.).to_path(0.1)
        })
    }
    fn resolve_effect_clip_path(&self, node: &ExpandedNode) -> Option<BezPath> {
        matches!(self.kind, Kind::Clip | Kind::Scroller).then(|| {
            Affine::from(node.transform_and_bounds.get().transform)
                * kurbo::RoundedRect::new(0., 0., 60., 60., 8.).to_path(0.1)
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

fn clear(ctx: &RuntimeContext) {
    ctx.clear_all_dirty_canvases();
    for id in ctx.dirty_canvas_node_ids() {
        ctx.clear_canvas_node_dirty(&id);
    }
    ctx.take_canvas_node_orders();
    ctx.take_native_messages();
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

fn assert_full_rebuild_matches(root: &Rc<ExpandedNode>, ctx: &RuntimeContext) {
    let expected: HashMap<_, _> = ctx
        .occlusion_state
        .borrow()
        .records
        .keys()
        .filter_map(|&id| {
            let n = ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(id))?;
            Some((id, (n.occlusion.get(), n.native_mask_hash.get())))
        })
        .collect();
    ctx.occlusion_state.borrow_mut().records.clear();
    ctx.mark_occlusion_dirty();
    update_node_occlusion(root, ctx);
    for (id, (order, mask)) in expected {
        let n = ctx
            .get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(id))
            .unwrap();
        assert_eq!(n.occlusion.get(), order, "order for {id}");
        assert_eq!(n.native_mask_hash.get(), mask, "mask for {id}");
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
            .clone()
            .unwrap();
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
fn reparenting_between_clip_domains_and_container_edits_fall_back() {
    for kind in [Kind::Clip, Kind::Scroller] {
        let (ctx, root) = mount(vec![
            probe(Kind::Canvas, vec![]),
            probe(Kind::Native, vec![]),
        ]);
        let group = root.children.get()[0].clone();
        let before = ctx.occlusion_stats();
        let container = add(&group, kind, &ctx);
        settle(&root, &ctx);
        assert!(ctx.occlusion_stats().rebuilds > before.rebuilds);
        let before = ctx.occlusion_stats();
        let leaf = group.children.get()[0].clone();
        attach(&container, vec![leaf], &ctx);
        settle(&root, &ctx);
        assert!(ctx.occlusion_stats().rebuilds > before.rebuilds);
        assert_full_rebuild_matches(&root, &ctx);
    }
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
