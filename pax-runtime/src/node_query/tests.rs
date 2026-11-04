use super::*;
use crate::{
    api::Layer, BaseInstance, CommonPropertiesInit, ComponentInstance, InstanceFlags, InstanceNode,
    InstantiationArgs, PaxEngine, PropertiesInit, PropertiesScopeInit,
};
use pax_runtime_api::{pax_value::PaxAny, LocalProperty, Platform, Variable, OS};
use std::{cell::RefCell, collections::HashMap};

fn args(children: Vec<Rc<dyn InstanceNode>>) -> InstantiationArgs {
    InstantiationArgs {
        prototypical_common_properties: CommonPropertiesInit::Default,
        prototypical_properties: PropertiesInit::Factory(Box::new(|_, node| {
            node.is_none()
                .then(|| Rc::new(RefCell::new(PaxAny::Builtin(Default::default()))))
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

struct TreeNode {
    base: BaseInstance,
    key: Option<&'static str>,
    overlay: bool,
    scroll: bool,
    projected: bool,
}

impl TreeNode {
    fn new(
        children: Vec<Rc<dyn InstanceNode>>,
        transparent: bool,
        key: Option<&'static str>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: BaseInstance::new(
                args(children),
                InstanceFlags {
                    invisible_to_slot: transparent,
                    invisible_to_raycasting: true,
                    layer: Layer::DontCare,
                    is_component: false,
                    is_slot: false,
                },
            ),
            key,
            overlay: false,
            scroll: false,
            projected: false,
        })
    }

    fn scroller(children: Vec<Rc<dyn InstanceNode>>) -> Rc<Self> {
        let mut node = Self::new(children, false, None);
        Rc::get_mut(&mut node).unwrap().scroll = true;
        node
    }

    fn native() -> Rc<Self> {
        Rc::new(Self {
            base: BaseInstance::new(
                args(vec![]),
                InstanceFlags {
                    invisible_to_slot: false,
                    invisible_to_raycasting: false,
                    layer: Layer::Native,
                    is_component: false,
                    is_slot: false,
                },
            ),
            key: None,
            overlay: false,
            scroll: false,
            projected: false,
        })
    }

    fn overlay(children: Vec<Rc<dyn InstanceNode>>) -> Rc<Self> {
        let mut node = Self::new(children, false, None);
        Rc::get_mut(&mut node).unwrap().overlay = true;
        node
    }

    fn projected(children: Vec<Rc<dyn InstanceNode>>) -> Rc<Self> {
        let mut node = Self::new(children, false, None);
        Rc::get_mut(&mut node).unwrap().projected = true;
        node
    }
}

impl InstanceNode for TreeNode {
    fn received_children_source(&self) -> crate::ReceivedChildrenSource {
        if self.projected {
            crate::ReceivedChildrenSource::Projected
        } else {
            crate::ReceivedChildrenSource::Owned
        }
    }
    fn scrolls_content(&self, _: &ExpandedNode) -> bool {
        self.scroll
    }
    fn clips_content(&self, _: &ExpandedNode) -> bool {
        self.scroll
    }
    fn resolve_scroll_offset(&self, _: &ExpandedNode) -> Option<(f64, f64)> {
        self.scroll.then_some((0., 80.))
    }
    fn is_overlay(&self) -> bool {
        self.overlay
    }
    fn accepts_transferred_children(&self) -> bool {
        !self.base.flags().invisible_to_slot
    }
    fn instantiate(args: InstantiationArgs) -> Rc<Self> {
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    invisible_to_slot: false,
                    invisible_to_raycasting: true,
                    layer: Layer::DontCare,
                    is_component: false,
                    is_slot: false,
                },
            ),
            key: None,
            overlay: false,
            scroll: false,
            projected: false,
        })
    }
    fn base(&self) -> &BaseInstance {
        &self.base
    }
    fn resolve_debug(&self, f: &mut fmt::Formatter, _: Option<&ExpandedNode>) -> fmt::Result {
        f.debug_struct("TreeNode").finish()
    }
    fn handle_mount(self: Rc<Self>, node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        let mut env = node.stack.clone();
        if let Some(key) = self.key {
            env = env.push(HashMap::from([(
                "@repeat_key".into(),
                Variable::new_from_typed_property(LocalProperty::new(key.to_string())),
            )]));
        }
        let children = self.base.get_instance_children().borrow().clone();
        let children = node.generate_children(
            children.into_iter().map(|n| (n, env.clone())),
            context,
            &node.parent_frame,
            true,
        );
        node.children.set(children);
    }
}

fn fixture() -> (PaxEngine, Rc<ExpandedNode>, NodeRef) {
    let first = TreeNode::new(vec![], false, None);
    let second = TreeNode::new(vec![], false, None);
    let last = TreeNode::new(vec![], false, None);
    let wrapper = TreeNode::new(vec![first, second], true, Some("string:42"));
    let integer_wrapper = TreeNode::new(vec![last], true, Some("int:42"));
    let parent = TreeNode::new(vec![wrapper, integer_wrapper], false, None);
    let mut root_args = args(vec![]);
    root_args.component_template = Some(RefCell::new(vec![parent]));
    let mut engine = crate::test_support::empty_engine(
        (640., 480.),
        Platform::Web,
        OS::Mac,
        Box::new(|| 0),
        Default::default(),
    );
    let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
    engine.tick();
    let parent = root.children.get()[0].clone();
    parent
        .selector_metadata
        .borrow()
        .id
        .set(Some("parent".into()));
    for (index, child) in content_children(&parent, &engine.runtime_context)
        .iter()
        .enumerate()
    {
        child
            .selector_metadata
            .borrow()
            .id
            .set(Some(format!("child_{index}")));
        child
            .selector_metadata
            .borrow()
            .classes
            .set(vec!["item".into()]);
    }
    let handle = NodeRef::new(&root, &engine.runtime_context);
    (engine, parent, handle)
}

#[test]
fn scoped_queries_preserve_order_and_reject_ambiguity() {
    let (_engine, _, root) = fixture();
    let items = root.query("#parent").query(".item");
    assert_eq!(items.len(), Ok(3));
    assert_eq!(items.clone().one(), Err(NodeError::ExpectedOne(3)));
    assert_eq!(root.query("#absent").one(), Err(NodeError::ExpectedOne(0)));
    assert_eq!(items.nth(0).unwrap(), root.query("#child_0").one().unwrap());
    assert_eq!(
        items.nth(3),
        Err(NodeError::IndexOutOfBounds { index: 3, len: 3 })
    );
    assert!(matches!(
        root.query("#bad .selector").one(),
        Err(NodeError::InvalidSelector(_))
    ));
}

#[test]
fn content_children_and_neighbors_flatten_only_transport_wrappers() {
    let (_engine, _, root) = fixture();
    assert_eq!(root.content_children().len(), Ok(1));
    let children = root.query("#parent").one().unwrap().content_children();
    assert_eq!(children.len(), Ok(3));
    let a = children.nth(0).unwrap();
    let b = children.nth(1).unwrap();
    let c = children.nth(2).unwrap();
    assert_eq!(a.previous(), Ok(None));
    assert_eq!(a.next(), Ok(Some(b.clone())));
    assert_eq!(b.next(), Ok(Some(c.clone())));
    assert_eq!(c.previous(), Ok(Some(b.clone())));
    assert_eq!(c.next(), Ok(None));
    assert_eq!(children.index_of(&b), Ok(1));
    assert_eq!(children.index_of(&root), Err(NodeError::NotAChild));
}

#[test]
fn key_filter_distinguishes_types_and_keeps_multiple_iteration_roots() {
    let (_engine, _, root) = fixture();
    let items = root.query(".item");
    assert_eq!(items.clone().key("42").len(), Ok(2));
    assert_eq!(
        items.clone().key("42").one(),
        Err(NodeError::ExpectedOne(2))
    );
    assert_eq!(items.clone().key(42).one(), root.query("#child_2").one());
    assert_eq!(items.key("missing").len(), Ok(0));
}

#[test]
fn handles_do_not_keep_nodes_mounted_or_retarget_a_remount() {
    let (engine, parent, root) = fixture();
    let handle = root.query("#parent").one().unwrap();
    let snapshot = handle.content_children();
    let context = parent.get_node_context(&engine.runtime_context);
    parent.clone().recurse_unmount(&engine.runtime_context);
    assert_eq!(handle.content_children().len(), Err(NodeError::Unmounted));
    assert_eq!(snapshot.len(), Err(NodeError::Unmounted));
    parent.recurse_mount(&engine.runtime_context);
    assert_eq!(context.app_root(), Err(NodeError::Unmounted));
    assert_eq!(context.detached_nodes().len(), Err(NodeError::Unmounted));
    assert_eq!(context.visiting_nodes().len(), Err(NodeError::Unmounted));
    assert!(handle.resolve().is_err());
    let fresh = NodeRef::new(&parent, &engine.runtime_context);
    assert_ne!(handle, fresh);
    assert!(fresh.resolve().is_ok());
    drop(engine);
    assert!(fresh.resolve().is_err());
}

#[test]
fn same_parent_moves_use_final_content_indices_across_repeat_wrappers() {
    let (mut engine, parent, root) = fixture();
    let parent_ref = root.query("#parent").one().unwrap();
    let a = root.query("#child_0").one().unwrap();
    let b = root.query("#child_1").one().unwrap();
    let c = root.query("#child_2").one().unwrap();
    a.move_to_insert_child(&parent_ref, 2).unwrap();
    assert_eq!(
        parent_ref.content_children().all().unwrap(),
        vec![b.clone(), c.clone(), a.clone()]
    );
    a.move_to_insert_before(&b).unwrap();
    assert_eq!(
        parent_ref.content_children().all().unwrap(),
        vec![a.clone(), b.clone(), c.clone()]
    );
    b.move_to_insert_after(&c).unwrap();
    assert_eq!(
        parent_ref.content_children().all().unwrap(),
        vec![a.clone(), c.clone(), b.clone()]
    );
    a.move_to_append_child(&parent_ref).unwrap();
    assert_eq!(
        parent_ref.content_children().all().unwrap(),
        vec![c.clone(), b.clone(), a.clone()]
    );
    let before = parent_ref.content_children().all().unwrap();
    assert_eq!(
        b.move_to_insert_child(&parent_ref, 3),
        Err(NodeError::IndexOutOfBounds { index: 3, len: 2 })
    );
    assert_eq!(parent_ref.content_children().all().unwrap(), before);
    engine.tick();
    assert_eq!(parent_ref.content_children().all().unwrap(), before);
    assert_eq!(
        parent
            .get_node_context(&engine.runtime_context)
            .received_children
            .get()
            .len(),
        3
    );
}

#[test]
fn abandonment_requires_explicit_recovery_into_a_fresh_mount() {
    let (mut engine, parent, root) = fixture();
    let source = root.query("#child_0").one().unwrap();
    let sibling = root.query("#child_1").one().unwrap();
    let destination = root.query("#child_2").one().unwrap();
    let (source_node, _) = source.resolve().unwrap();
    let authored_parent = source_node.render_parent.borrow().upgrade().unwrap();
    let mount = source_node.mount_epoch.get();
    let original_properties = source_node.properties.borrow().clone();
    source.move_to_append_child(&destination).unwrap();
    engine.tick();
    assert_eq!(
        destination.content_children().all().unwrap(),
        vec![source.clone()]
    );
    assert_eq!(source_node.mount_epoch.get(), mount);
    assert!(Rc::ptr_eq(
        &original_properties,
        &source_node.properties.borrow()
    ));
    // Reconciliation at its original location cannot reclaim the moved node.
    let authored_children = authored_parent.active_children.borrow().clone();
    let children = authored_parent.attach_children(
        authored_children,
        &engine.runtime_context,
        &authored_parent.parent_frame,
    );
    authored_parent.children.set(children);
    assert_eq!(
        destination.content_children().all().unwrap(),
        vec![source.clone()]
    );
    assert_eq!(sibling.previous(), Ok(None));
    let (dest_node, _) = destination.resolve().unwrap();
    dest_node.clone().recurse_unmount(&engine.runtime_context);
    assert_eq!(source.is_detached(), Ok(true));
    assert_eq!(source_node.attached.get(), 0);
    assert_eq!(source.next(), Ok(None));
    let root_node = root.resolve().unwrap().0;
    let ctx = root_node.get_node_context(&engine.runtime_context);
    assert_eq!(ctx.detached_nodes().all().unwrap(), vec![source.clone()]);
    assert_eq!(ctx.app_root().unwrap(), root);
    // Reconciliation must not mount an abandoned entry until explicitly moved.
    let authored_children = authored_parent.active_children.borrow().clone();
    let children = authored_parent.attach_children(
        authored_children,
        &engine.runtime_context,
        &authored_parent.parent_frame,
    );
    authored_parent.children.set(children);
    engine.tick();
    assert_eq!(source_node.attached.get(), 0);
    source.move_to_home().unwrap();
    assert_eq!(source.is_detached(), Ok(false));
    assert_eq!(
        source_node.render_parent.borrow().upgrade().unwrap().id,
        authored_parent.id
    );
    assert_eq!(source.next(), Ok(Some(sibling)));
    assert_eq!(source_node.mount_epoch.get(), mount + 1);
    source_node
        .get_common_properties()
        .borrow()
        ._suspended
        .set(Some(true));
    assert!(source_node.suspended.get());
    source_node
        .get_common_properties()
        .borrow()
        ._suspended
        .set(Some(false));
    assert!(!Rc::ptr_eq(
        &original_properties,
        &source_node.properties.borrow()
    ));
    assert!(ctx.detached_nodes().is_empty().unwrap());
    dest_node.recurse_mount(&engine.runtime_context);
    let new_destination = NodeRef::new(&dest_node, &engine.runtime_context);
    assert!(new_destination.content_children().is_empty().unwrap());
    assert!(destination.resolve().is_err());
    source.move_to_append_child(&new_destination).unwrap();
    parent.recurse_unmount(&engine.runtime_context);
    assert!(source.resolve().is_err());
    assert!(!engine.runtime_context.is_transferred(source.engine_id()));
}

#[test]
fn removing_the_original_declaration_discards_an_unmounted_recovery_entry() {
    let (engine, _, root) = fixture();
    let source = root.query("#child_0").one().unwrap();
    let destination = root.query("#child_2").one().unwrap();
    let source_node = source.resolve().unwrap().0;
    let origin = source_node.render_parent.borrow().upgrade().unwrap();
    source.move_to_append_child(&destination).unwrap();
    destination
        .resolve()
        .unwrap()
        .0
        .recurse_unmount(&engine.runtime_context);
    assert_eq!(source.is_detached(), Ok(true));
    let remaining = origin
        .active_children
        .borrow()
        .iter()
        .filter(|n| n.id != source_node.id)
        .cloned()
        .collect();
    let children = origin.attach_children(remaining, &engine.runtime_context, &origin.parent_frame);
    origin.children.set(children);
    assert_eq!(source.move_to_home(), Err(NodeError::Unmounted));
    assert!(!engine.runtime_context.is_transferred(source.engine_id()));
}

#[test]
fn pre_unmount_can_return_visitors_before_child_and_native_teardown() {
    use crate::{Handler, HandlerRegistry};
    fn return_visitors(_: Rc<RefCell<PaxAny>>, ctx: &crate::api::NodeContext, _: Option<PaxAny>) {
        let visitors = ctx.visiting_nodes().all().unwrap();
        assert_eq!(visitors.len(), 1);
        for visitor in visitors {
            visitor.move_to_home().unwrap();
        }
    }
    let source = TreeNode::new(vec![], false, None);
    let destination = TreeNode::new(vec![], false, None);
    let mut host_args = args(vec![]);
    host_args.component_template = Some(RefCell::new(vec![destination]));
    let mut handlers = HandlerRegistry::default();
    handlers.handlers.insert(
        "pre_unmount".into(),
        vec![Handler::new_inline_handler(return_visitors)],
    );
    host_args.handler_registry = Some(Rc::new(RefCell::new(handlers)));
    let mut root_args = args(vec![]);
    root_args.component_template = Some(RefCell::new(vec![
        source,
        ComponentInstance::instantiate(host_args),
    ]));
    let mut engine = crate::test_support::empty_engine(
        (640., 480.),
        Platform::Web,
        OS::Mac,
        Box::new(|| 0),
        Default::default(),
    );
    let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
    engine.tick();
    let source = root.children.get()[0].clone();
    let host = root.children.get()[1].clone();
    let destination = host.children.get()[0].clone();
    let handle = NodeRef::new(&source, &engine.runtime_context);
    let properties = source.properties.borrow().clone();
    let mount = source.mount_epoch.get();
    handle
        .move_to_append_child(&NodeRef::new(&destination, &engine.runtime_context))
        .unwrap();
    assert_eq!(
        host.get_node_context(&engine.runtime_context)
            .visiting_nodes()
            .all()
            .unwrap(),
        vec![handle.clone()]
    );
    host.recurse_unmount(&engine.runtime_context);
    assert_eq!(source.render_parent.borrow().upgrade().unwrap().id, root.id);
    assert_eq!(source.mount_epoch.get(), mount);
    assert_eq!(source.attached.get(), 1);
    assert!(Rc::ptr_eq(&properties, &source.properties.borrow()));
    assert!(root
        .get_node_context(&engine.runtime_context)
        .detached_nodes()
        .is_empty()
        .unwrap());
}

#[test]
fn moves_reject_cycles_including_latent_return_home_cycles() {
    let (engine, parent, root) = fixture();
    let child = root.query("#child_0").one().unwrap();
    let parent_ref = NodeRef::new(&parent, &engine.runtime_context);
    let other = root.query("#child_2").one().unwrap();
    assert_eq!(
        parent_ref.move_to_append_child(&child),
        Err(NodeError::Cycle)
    );
    assert_eq!(
        parent_ref.move_to_append_child(&parent_ref),
        Err(NodeError::Cycle)
    );
    child.move_to_append_child(&other).unwrap();
    assert_eq!(
        parent_ref.move_to_append_child(&child),
        Err(NodeError::Cycle)
    );
    let (_other_engine, _, unrelated_root) = fixture();
    assert_eq!(
        child.move_to_append_child(&unrelated_root),
        Err(NodeError::DifferentApplication)
    );
}

#[test]
fn overlay_contributions_escape_owner_geometry_but_keep_owner_lifetime() {
    use crate::api::{Opacity, Rotation, Size};
    let overlay_a = TreeNode::overlay(vec![TreeNode::new(vec![], false, None)]);
    let overlay_b = TreeNode::overlay(vec![TreeNode::new(vec![], false, None)]);
    let owner = TreeNode::new(vec![overlay_a, overlay_b], false, None);
    let mut root_args = args(vec![]);
    root_args.component_template = Some(RefCell::new(vec![owner]));
    let mut engine = crate::test_support::empty_engine(
        (640., 480.),
        Platform::Web,
        OS::Mac,
        Box::new(|| 0),
        Default::default(),
    );
    let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
    engine.tick();
    let owner = root.children.get()[0].clone();
    let overlays = owner.children.get();
    let a = overlays[0].clone();
    let b = overlays[1].clone();
    let cp = owner.get_common_properties().borrow().clone();
    cp.x.set(Some(Size::Pixels(200.into())));
    cp.width.set(Some(Size::Pixels(100.into())));
    cp.rotate.set(Some(Rotation::Degrees(25.into())));
    #[cfg(feature = "3d")]
    cp.rotate_y.set(Some(Rotation::Degrees(40.into())));
    cp.opacity.set(Some(Opacity::Alpha(0.2.into())));
    engine.tick();
    assert_eq!(
        root.presentation_children(&engine.runtime_context)
            .iter()
            .map(|n| n.id)
            .collect::<Vec<_>>(),
        vec![a.id, b.id, owner.id]
    );
    assert!(owner
        .presentation_children(&engine.runtime_context)
        .is_empty());
    for overlay in [&a, &b] {
        let geometry = overlay.transform_and_bounds.get();
        assert_eq!(geometry.bounds, (640., 480.));
        assert_eq!(geometry.transform.coeffs(), [1., 0., 0., 1., 0., 0.]);
        assert_eq!(overlay.computed_opacity.get(), 1.);
        assert_eq!(
            overlay.children.get()[0].presentation_layer.get(),
            overlay.presentation_layer.get()
        );
        assert!(overlay.presentation_layer.get() > 0);
        assert!(overlay.parent_frame.get().is_none());
    }
    assert!(a.presentation_layer.get() > b.presentation_layer.get());
    let reordered = owner.attach_children(
        vec![b.clone(), a.clone()],
        &engine.runtime_context,
        &owner.parent_frame,
    );
    owner.children.set(reordered);
    engine.tick();
    assert!(b.presentation_layer.get() > a.presentation_layer.get());
    let handle = NodeRef::new(&a, &engine.runtime_context);
    owner.recurse_unmount(&engine.runtime_context);
    assert!(handle.resolve().is_err());
    assert_eq!(root.presentation_children(&engine.runtime_context).len(), 1);
}

fn presentation_locality_fixture(unrelated: usize) -> (PaxEngine, Rc<ExpandedNode>) {
    let mut root_args = args(vec![]);
    root_args.component_template = Some(RefCell::new(vec![
        TreeNode::overlay(vec![]),
        TreeNode::new(vec![TreeNode::new(vec![], false, None)], false, None),
        TreeNode::new(
            (0..unrelated)
                .map(|_| TreeNode::new(vec![], false, None) as Rc<dyn InstanceNode>)
                .collect(),
            false,
            None,
        ),
    ]));
    let mut engine = crate::test_support::empty_engine(
        (640., 480.),
        Platform::Web,
        OS::Mac,
        Box::new(|| 0),
        Default::default(),
    );
    let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
    engine.tick();
    (engine, root)
}

#[test]
fn presentation_notifications_are_local_with_an_unrelated_overlay() {
    for unrelated_count in [16, 2048] {
        let (mut engine, root) = presentation_locality_fixture(unrelated_count);
        let ctx = engine.runtime_context.clone();
        let _graph = ctx.property_graph.enter();
        let children = root.children.get();
        let edited = &children[1];
        let unrelated = &children[2];
        let unrelated_nodes: Vec<_> = std::iter::once(unrelated.clone())
            .chain(unrelated.children.get())
            .collect();
        let before: Vec<_> = unrelated_nodes
            .iter()
            .map(|n| n.layout_hull_listener_evaluations.get())
            .collect();
        let mut next = edited.children.get();
        next.extend(edited.create_children_detached(
            [(
                TreeNode::new(vec![], false, None) as Rc<dyn InstanceNode>,
                edited.stack.clone(),
            )],
            &ctx,
            &Rc::downgrade(edited),
        ));
        let attached = edited.attach_children(next, &ctx, &edited.parent_frame);
        let added_properties = attached.last().unwrap().get_common_properties();
        added_properties
            .borrow()
            .width
            .set(Some(crate::api::Size::Pixels(900.into())));
        added_properties
            .borrow()
            .height
            .set(Some(crate::api::Size::Pixels(700.into())));
        edited.children.set(attached);
        engine.tick();
        assert_eq!(edited.logical_children(&ctx).len(), 2);
        assert_eq!(edited.subtree_layout_hull.get().max_x, 900.);
        for (node, before) in unrelated_nodes.iter().zip(before) {
            assert_eq!(
                node.layout_hull_listener_evaluations.get(),
                before,
                "a local insertion woke unrelated node {:?} in a {unrelated_count}-node branch",
                node.id
            );
        }
        assert_hulls_match_full_refresh(&root, &ctx);
    }
}

fn mounted_tree(root: &Rc<ExpandedNode>) -> Vec<Rc<ExpandedNode>> {
    let mut nodes = vec![];
    let mut pending = vec![root.clone()];
    let mut seen = std::collections::HashSet::new();
    while let Some(node) = pending.pop() {
        if !seen.insert(node.id) {
            continue;
        }
        pending.extend(node.current_attached_children());
        nodes.push(node);
    }
    nodes
}

fn assert_hulls_match_full_refresh(root: &Rc<ExpandedNode>, ctx: &Rc<RuntimeContext>) {
    let nodes = mounted_tree(root);
    let before: Vec<_> = nodes
        .iter()
        .map(|node| node.subtree_layout_hull.get())
        .collect();
    // Rebuild every hull binding as the oracle, bypassing membership cutoffs.
    // Missing scoped notifications must not leave different retained geometry.
    for node in nodes.iter().rev().filter(|node| node.attached.get() > 0) {
        node.refresh_layout_hull_for_test(ctx);
    }
    ctx.drain_node_effects();
    for (node, before) in nodes.iter().zip(before) {
        assert!(
            crate::layout_hulls_equivalent(&before, &node.subtree_layout_hull.get()),
            "scoped hull differs from full refresh at {:?}",
            node.id
        );
    }
}

#[test]
fn received_children_cutoff_preserves_reorder_move_and_remount_notifications() {
    let (mut engine, root) = presentation_locality_fixture(3);
    let ctx = engine.runtime_context.clone();
    let _graph = ctx.property_graph.enter();
    let children = root.children.get();
    let source = &children[2];
    let destination = &children[1];
    let source_context = source.get_node_context(&ctx);
    let destination_context = destination.get_node_context(&ctx);
    let source_view = source_context.received_children;
    let destination_view = destination_context.received_children;
    assert_eq!(source_context.received_children_count.get(), 3);
    assert_eq!(destination_context.received_children_count.get(), 1);
    let evaluations = Rc::new(std::cell::Cell::new(0));
    let count = evaluations.clone();
    let observed = source_view.clone();
    let listener = LocalProperty::computed(
        move || {
            let ids = observed
                .get()
                .iter()
                .map(|node| node.id)
                .collect::<Vec<_>>();
            count.set(count.get() + 1);
            ids
        },
        &[source_view.untyped()],
    );
    let original = listener.get();
    let initial = evaluations.get();
    // A source write without a membership or binding change should cut off.
    source
        .active_children_view
        .set(source.active_children_view.get());
    ctx.drain_node_effects();
    assert_eq!(listener.get(), original);
    assert_eq!(evaluations.get(), initial);
    let mut reordered = source.children.get();
    reordered.reverse();
    source
        .children
        .set(source.attach_children(reordered, &ctx, &source.parent_frame));
    assert_eq!(
        source_view.get().iter().map(|n| n.id).collect::<Vec<_>>(),
        original.iter().rev().copied().collect::<Vec<_>>()
    );
    ctx.drain_node_effects();
    assert_eq!(
        listener.get(),
        original.iter().rev().copied().collect::<Vec<_>>()
    );
    assert_eq!(evaluations.get(), initial + 1);
    let moved = source.children.get()[0].clone();
    let destination_before = destination_view.get().len();
    ctx.move_node(&moved, destination, 0).unwrap();
    assert_eq!(source_context.received_children_count.get(), 2);
    assert_eq!(
        destination_context.received_children_count.get(),
        destination_before + 1
    );
    assert_eq!(source_view.get().len(), 2);
    ctx.drain_node_effects();
    assert_eq!(listener.get().len(), 2);
    assert_eq!(destination_view.get().len(), destination_before + 1);
    assert_eq!(destination_view.get()[0].id, moved.id);
    ctx.move_home(&moved).unwrap();
    assert_eq!(source_context.received_children_count.get(), 3);
    assert_eq!(
        destination_context.received_children_count.get(),
        destination_before
    );
    assert_eq!(source_view.get().len(), 3);
    ctx.drain_node_effects();
    assert_eq!(listener.get().len(), 3);
    assert_eq!(destination_view.get().len(), destination_before);
    let before_remount = evaluations.get();
    moved.clone().recurse_unmount(&ctx);
    moved.recurse_mount(&ctx);
    ctx.drain_node_effects();
    assert_eq!(listener.get().len(), 3);
    assert_eq!(
        evaluations.get(),
        before_remount + 1,
        "retained Rc identities must not hide a fresh mount's property handles"
    );
    engine.tick();
    assert_hulls_match_full_refresh(&root, &ctx);
}

#[test]
fn projected_received_children_follow_moves_through_control_flow_and_slots() {
    use pax_runtime_api::pax_value::ToFromPaxAny;
    let mut conditional = args(vec![TreeNode::new(vec![], false, None)]);
    conditional.prototypical_properties = PropertiesInit::Factory(Box::new(|_, node| {
        node.is_none().then(|| {
            Rc::new(RefCell::new(
                crate::ConditionalProperties {
                    boolean_expression: LocalProperty::new(true),
                    ..Default::default()
                }
                .to_pax_any(),
            ))
        })
    }));
    let mut slot = args(vec![]);
    slot.prototypical_properties = PropertiesInit::Factory(Box::new(|_, node| {
        node.is_none().then(|| {
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
    receiver.component_template = Some(RefCell::new(vec![TreeNode::projected(vec![
        crate::SlotInstance::instantiate(slot),
    ])]));
    let mut root_args = args(vec![]);
    root_args.component_template = Some(RefCell::new(vec![
        ComponentInstance::instantiate(receiver),
        TreeNode::new(vec![], false, None),
        TreeNode::overlay(vec![]),
    ]));
    let mut engine = crate::test_support::empty_engine(
        (640., 480.),
        Platform::Web,
        OS::Mac,
        Box::new(|| 0),
        Default::default(),
    );
    let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
    engine.tick();
    let ctx = engine.runtime_context.clone();
    let _graph = ctx.property_graph.enter();
    let children = root.children.get();
    let receiver = &children[0];
    let destination = &children[1];
    let inner = receiver.children.get()[0].clone();
    let receiver_view = receiver.get_node_context(&ctx).received_children;
    let inner_view = inner.get_node_context(&ctx).received_children;
    let destination_view = destination.get_node_context(&ctx).received_children;
    let leaf = receiver_view.get()[0].clone();
    let mount = leaf.mount_epoch.get();
    assert_eq!(inner_view.get()[0].id, leaf.id);
    assert!(destination_view.get().is_empty());
    ctx.move_node(&leaf, destination, 0).unwrap();
    assert!(receiver_view.get().is_empty());
    assert!(inner_view.get().is_empty());
    assert_eq!(destination_view.get()[0].id, leaf.id);
    engine.tick();
    assert_hulls_match_full_refresh(&root, &ctx);
    destination.clone().recurse_unmount(&ctx);
    assert!(ctx.is_detached(leaf.id));
    assert!(receiver_view.get().is_empty());
    assert!(inner_view.get().is_empty());
    ctx.move_home(&leaf).unwrap();
    assert_eq!(receiver_view.get()[0].id, leaf.id);
    assert_eq!(inner_view.get()[0].id, leaf.id);
    assert!(leaf.mount_epoch.get() > mount);
    engine.tick();
    assert_hulls_match_full_refresh(&root, &ctx);
}

#[test]
fn yielded_startup_hull_listener_work_scales_with_nodes_despite_overlays() {
    for count in [16, 256] {
        let mut engine = crate::test_support::empty_engine(
            (640., 480.),
            Platform::Web,
            OS::Mac,
            Box::new(|| 0),
            Default::default(),
        );
        engine.enable_resumable_startup();
        let mut root_args = args(vec![]);
        root_args.component_template = Some(RefCell::new(vec![
            TreeNode::overlay(vec![]),
            TreeNode::new(
                (0..count)
                    .map(|_| TreeNode::new(vec![], false, None) as Rc<dyn InstanceNode>)
                    .collect(),
                false,
                None,
            ),
        ]));
        let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
        for _ in 0..100_000 {
            if !engine.startup_pending() {
                break;
            }
            engine.service_startup(0.0, || 0.0);
            if engine.accept_startup_measurements() {
                engine.resume_startup_after_presentation();
            }
        }
        assert!(!engine.startup_pending());
        let nodes = mounted_tree(&root);
        assert_eq!(nodes.len(), count + 3);
        let evaluations: usize = nodes
            .iter()
            .map(|node| node.layout_hull_listener_evaluations.get())
            .sum();
        assert!(
            evaluations <= 12 * nodes.len(),
            "{evaluations} hull listener evaluations for {} nodes",
            nodes.len()
        );
        assert_hulls_match_full_refresh(&root, &engine.runtime_context);
    }
}

#[test]
fn animated_moves_preserve_the_initial_footprint_and_reverse_without_a_jump() {
    use crate::api::{Duration, EasingCurve, Size};
    let (mut engine, parent, root) = fixture();
    let source = root.query("#child_0").one().unwrap();
    let destination = root.query("#child_2").one().unwrap();
    let node = source.resolve().unwrap().0;
    let dest = destination.resolve().unwrap().0;
    let cp = dest.get_common_properties().borrow().clone();
    cp.x.set(Some(Size::Pixels(300.into())));
    cp.y.set(Some(Size::Pixels(50.into())));
    cp.width.set(Some(Size::Pixels(200.into())));
    cp.height.set(Some(Size::Pixels(100.into())));
    engine.tick();
    let start = node.transform_and_bounds.get();
    let mount = node.mount_epoch.get();
    source
        .animated(Duration::Frames(4.into()), EasingCurve::Linear)
        .move_to_append_child(&destination)
        .unwrap();
    let moved = node.transform_and_bounds.get();
    assert_eq!(moved.bounds, start.bounds);
    for (a, b) in moved
        .transform
        .coeffs()
        .into_iter()
        .zip(start.transform.coeffs())
    {
        assert!((a - b).abs() < 1e-8);
    }
    engine.tick();
    engine.tick();
    let mid = node.transform_and_bounds.get();
    assert!(mid.bounds.0 < start.bounds.0 && mid.bounds.0 > 200.);
    assert!(mid.transform.coeffs()[4] > 0. && mid.transform.coeffs()[4] < 300.);
    source
        .animated(Duration::Frames(4.into()), EasingCurve::OutQuad)
        .move_to_home()
        .unwrap();
    let reversed = node.transform_and_bounds.get();
    assert_eq!(reversed.bounds, mid.bounds);
    for (a, b) in reversed
        .transform
        .coeffs()
        .into_iter()
        .zip(mid.transform.coeffs())
    {
        assert!((a - b).abs() < 1e-8);
    }
    parent
        .get_common_properties()
        .borrow()
        .width
        .set(Some(Size::Pixels(500.into())));
    for _ in 0..5 {
        engine.tick();
    }
    let end = node.transform_and_bounds.get();
    assert_eq!(end.bounds.0, 500.);
    assert_eq!(end.transform.coeffs(), [1., 0., 0., 1., 0., 0.]);
    assert_eq!(node.mount_epoch.get(), mount);
}

#[test]
fn keyed_repeat_reconciles_reordered_and_removed_transfers_at_their_original_owner() {
    use crate::repeat::{RepeatInstance, RepeatProperties};
    use pax_manifest::ExpressionInfo;
    use pax_runtime_api::pax_value::ToFromPaxAny;
    let values = |keys: &[&str]| {
        PaxValue::Vec(
            keys.iter()
                .map(|key| PaxValue::String((*key).into()))
                .collect(),
        )
    };
    let source = LocalProperty::new(values(&["a", "b", "c"]));
    let source_for_factory = source.clone();
    let mut repeat_args = args(vec![TreeNode::new(vec![], false, None)]);
    let key = ExpressionInfo::new(pax_language::parse_pax_expression("item").unwrap());
    repeat_args.prototypical_properties = PropertiesInit::Factory(Box::new(move |_, node| {
        node.is_none().then(|| {
            Rc::new(RefCell::new(
                RepeatProperties {
                    source_expression: source_for_factory.clone(),
                    iterator_i_symbol: LocalProperty::new(Some("i".into())),
                    iterator_elem_symbol: LocalProperty::new(Some("item".into())),
                    repeat_key_expression: Some(key.clone()),
                }
                .to_pax_any(),
            ))
        })
    }));
    let origin = TreeNode::new(vec![RepeatInstance::instantiate(repeat_args)], false, None);
    let destination = TreeNode::new(vec![], false, None);
    let mut root_args = args(vec![]);
    root_args.component_template = Some(RefCell::new(vec![origin, destination]));
    let mut engine = crate::test_support::empty_engine(
        (640., 480.),
        Platform::Web,
        OS::Mac,
        Box::new(|| 0),
        Default::default(),
    );
    let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
    engine.tick();
    let origin = NodeRef::new(&root.children.get()[0], &engine.runtime_context);
    let destination = NodeRef::new(&root.children.get()[1], &engine.runtime_context);
    let a = origin.content_children().key("a").one().unwrap();
    let b = origin.content_children().key("b").one().unwrap();
    let mount = a.resolve().unwrap().0.mount_epoch.get();
    a.move_to_append_child(&destination).unwrap();
    source.set(values(&["c", "b", "a"]));
    engine.tick();
    assert_eq!(destination.content_children().key("a").one().unwrap(), a);
    a.move_to_home().unwrap();
    assert_eq!(origin.content_children().nth(2).unwrap(), a);
    assert_eq!(a.previous(), Ok(Some(b)));
    assert_eq!(a.resolve().unwrap().0.mount_epoch.get(), mount);
    a.move_to_append_child(&destination).unwrap();
    destination
        .resolve()
        .unwrap()
        .0
        .recurse_unmount(&engine.runtime_context);
    source.set(values(&["a", "c", "b"]));
    engine.tick();
    assert_eq!(a.is_detached(), Ok(true));
    a.move_to_home().unwrap();
    assert_eq!(origin.content_children().nth(0).unwrap(), a);
    assert_eq!(a.resolve().unwrap().0.mount_epoch.get(), mount + 1);
    source.set(values(&["c", "b"]));
    engine.tick();
    assert_eq!(a.move_to_home(), Err(NodeError::Unmounted));
}

pub(crate) fn flight_fixture() -> (
    PaxEngine,
    Rc<ExpandedNode>,
    NodeRef,
    NodeRef,
    Rc<ExpandedNode>,
) {
    flight_fixture_with_content(TreeNode::native())
}

pub(crate) fn flight_fixture_with_content(
    content: Rc<dyn InstanceNode>,
) -> (
    PaxEngine,
    Rc<ExpandedNode>,
    NodeRef,
    NodeRef,
    Rc<ExpandedNode>,
) {
    use crate::api::Size;
    let higher = TreeNode::overlay(vec![]);
    let nav = TreeNode::overlay(vec![TreeNode::new(vec![], false, None)]);
    let home = TreeNode::scroller(vec![TreeNode::new(vec![content], false, None)]);
    let mut root_args = args(vec![]);
    root_args.component_template = Some(RefCell::new(vec![higher, nav, home]));
    let mut engine = crate::test_support::empty_engine(
        (640., 480.),
        Platform::Web,
        OS::Mac,
        Box::new(|| 0),
        Default::default(),
    );
    let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
    engine.tick();
    let children = root.children.get();
    let nav = children[1].clone();
    let target = nav.children.get()[0].clone();
    let home = children[2].clone();
    let node = home.children.get()[0].clone();
    home.get_common_properties()
        .borrow()
        .y
        .set(Some(Size::Pixels(200.into())));
    home.get_common_properties()
        .borrow()
        .height
        .set(Some(Size::Pixels(200.into())));
    node.get_common_properties()
        .borrow()
        .y
        .set(Some(Size::Pixels(100.into())));
    node.get_common_properties()
        .borrow()
        .height
        .set(Some(Size::Pixels(40.into())));
    target
        .get_common_properties()
        .borrow()
        .width
        .set(Some(Size::Pixels(180.into())));
    target
        .get_common_properties()
        .borrow()
        .height
        .set(Some(Size::Pixels(60.into())));
    engine.tick();
    let handle = NodeRef::new(&node, &engine.runtime_context);
    let target = NodeRef::new(&target, &engine.runtime_context);
    (engine, root, handle, target, nav)
}

#[test]
fn flight_escapes_destination_clips_and_settles_native_and_canvas_order_together() {
    use crate::api::{Duration, EasingCurve};
    let (mut engine, root, handle, target, nav) = flight_fixture();
    let ctx = engine.runtime_context.clone();
    let node = handle.resolve().unwrap().0;
    let native = node.children.get()[0].clone();
    let home = node.render_parent_node().unwrap();
    let higher = root.children.get()[0].clone();
    let before = ctx.presentation_scroll_transform_for_node(&node)
        * kurbo::Affine::from(node.transform_and_bounds.get().transform);
    handle
        .animated(Duration::Frames(4.into()), EasingCurve::Linear)
        .move_to_append_child(&target)
        .unwrap();
    let during = ctx.presentation_scroll_transform_for_node(&node)
        * kurbo::Affine::from(node.transform_and_bounds.get().transform);
    assert_eq!(before, during);
    assert_eq!(
        target.content_children().all().unwrap(),
        vec![handle.clone()]
    );
    assert!(target
        .resolve()
        .unwrap()
        .0
        .presentation_children(&ctx)
        .is_empty());
    let order: Vec<_> = root
        .presentation_children(&ctx)
        .iter()
        .map(|n| n.id)
        .collect();
    assert_eq!(order, vec![higher.id, node.id, nav.id, home.id]);
    engine.tick();
    assert!(node.move_in_flight.get());
    assert!(node.presentation_layer.get() > nav.presentation_layer.get());
    assert!(node.presentation_layer.get() < higher.presentation_layer.get());
    assert_eq!(native.occlusion.get().parent_frame, None);
    let geometry = node.transform_and_bounds.get();
    let point = geometry.transform * pax_runtime_api::math::Point2::new(10., 10.);
    assert!(ctx
        .get_elements_beneath_ray(None, point, false, vec![], false)
        .iter()
        .any(|n| n.id == native.id));
    for _ in 0..4 {
        engine.tick();
    }
    assert!(!node.move_in_flight.get());
    assert_eq!(node.presentation_layer.get(), nav.presentation_layer.get());
    let mount = node.mount_epoch.get();
    handle
        .animated(Duration::Frames(4.into()), EasingCurve::Linear)
        .move_to_home()
        .unwrap();
    engine.tick();
    assert!(node.move_in_flight.get());
    assert_eq!(node.render_parent_node().unwrap().id, home.id);
    assert!(home.presentation_children(&ctx).is_empty());
    assert!(NodeRef::new(&home, &ctx)
        .content_children()
        .all()
        .unwrap()
        .contains(&handle));
    assert_eq!(native.occlusion.get().parent_frame, None);
    let point =
        node.transform_and_bounds.get().transform * pax_runtime_api::math::Point2::new(10., 10.);
    assert!(
        point.y < 200.,
        "return is still outside the destination Scroller clip"
    );
    assert!(ctx
        .get_elements_beneath_ray(None, point, false, vec![], false)
        .iter()
        .any(|n| n.id == native.id));
    for _ in 0..4 {
        engine.tick();
    }
    assert!(!node.move_in_flight.get());
    assert_eq!(node.presentation_layer.get(), 0);
    assert_eq!(native.occlusion.get().parent_frame, Some(home.id.to_u32()));
    assert_eq!(node.mount_epoch.get(), mount);
}

#[test]
fn flight_survives_departed_overlay_loss_and_reversal_without_early_easing_completion() {
    use crate::api::{Duration, EasingCurve};
    let (mut engine, root, handle, target, nav) = flight_fixture();
    let ctx = engine.runtime_context.clone();
    let node = handle.resolve().unwrap().0;
    handle.move_to_append_child(&target).unwrap();
    engine.tick();
    handle
        .animated(
            Duration::Frames(6.into()),
            EasingCurve::Custom(Box::new(|_| 1.0)),
        )
        .move_to_home()
        .unwrap();
    engine.tick();
    assert!(
        node.move_in_flight.get(),
        "an easing value of one is not completion"
    );
    nav.recurse_unmount(&ctx);
    engine.tick();
    assert!(node.move_in_flight.get());
    let order: Vec<_> = root
        .presentation_children(&ctx)
        .iter()
        .map(|n| n.id)
        .collect();
    assert_eq!(
        order[1], node.id,
        "flight stays below the unrelated higher Overlay"
    );
    assert_eq!(node.attached.get(), 1);
    for _ in 0..6 {
        engine.tick();
    }
    assert!(!node.move_in_flight.get());
    assert!(handle.resolve().is_ok());

    // A new departure/return reverses without reusing the departed nav's handle.
    let higher = NodeRef::new(&root.children.get()[0], &ctx);
    handle
        .animated(Duration::Frames(8.into()), EasingCurve::Linear)
        .move_to_append_child(&higher)
        .unwrap();
    engine.tick();
    engine.tick();
    let before = node.transform_and_bounds.get();
    handle
        .animated(Duration::Frames(4.into()), EasingCurve::Linear)
        .move_to_home()
        .unwrap();
    assert_eq!(
        node.transform_and_bounds.get().transform.coeffs(),
        before.transform.coeffs()
    );
    assert!(node.move_in_flight.get());
    for _ in 0..5 {
        engine.tick();
    }
    assert!(!node.move_in_flight.get());
}

#[test]
fn flight_does_not_extend_destination_or_original_declaration_lifetime() {
    use crate::api::{Duration, EasingCurve};
    let (mut engine, _, handle, target, nav) = flight_fixture();
    let ctx = engine.runtime_context.clone();
    let node = handle.resolve().unwrap().0;
    let home = node.render_parent_node().unwrap();
    handle
        .animated(Duration::Frames(8.into()), EasingCurve::Linear)
        .move_to_append_child(&target)
        .unwrap();
    engine.tick();
    nav.recurse_unmount(&ctx);
    assert_eq!(handle.is_detached(), Ok(true));
    assert!(!node.move_in_flight.get());
    handle.move_to_home().unwrap();
    let target = NodeRef::new(
        &ctx.get_root_expanded_node().unwrap().children.get()[0],
        &ctx,
    );
    handle
        .animated(Duration::Frames(8.into()), EasingCurve::Linear)
        .move_to_append_child(&target)
        .unwrap();
    engine.tick();
    home.recurse_unmount(&ctx);
    assert_eq!(handle.move_to_home(), Err(NodeError::Unmounted));
    assert!(!node.move_in_flight.get());
    engine.tick();
}

#[test]
fn yielded_startup_mounts_overlay_contributions_and_their_native_children() {
    let mut engine = crate::test_support::empty_engine(
        (640., 480.),
        Platform::Web,
        OS::Mac,
        Box::new(|| 0),
        Default::default(),
    );
    engine.enable_resumable_startup();
    let mut root_args = args(vec![]);
    root_args.component_template = Some(RefCell::new(vec![TreeNode::new(
        vec![
            TreeNode::overlay(vec![TreeNode::native()]),
            TreeNode::overlay(vec![TreeNode::native()]),
        ],
        false,
        None,
    )]));
    let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
    finish_yielded_startup(&mut engine);
    let owner = root.children.get()[0].clone();
    let overlays = owner.children.get();
    assert_eq!(overlays.len(), 2);
    for overlay in &overlays {
        assert_eq!(
            overlay.attached.get(),
            1,
            "viewport roots still have authored mount owners"
        );
        assert!(overlay.render_parent_node().is_none());
        assert_eq!(overlay.children.get()[0].attached.get(), 1);
    }
    let contributions = root.presentation_children(&engine.runtime_context);
    assert_eq!(
        contributions[..2].iter().map(|n| n.id).collect::<Vec<_>>(),
        overlays.iter().map(|n| n.id).collect::<Vec<_>>()
    );
    assert!(
        engine.runtime_context.presentation_domain(overlays[0].id)
            > engine.runtime_context.presentation_domain(overlays[1].id)
    );
}

#[test]
fn yielded_startup_discards_a_removed_overlay_continuation() {
    let mut engine = crate::test_support::empty_engine(
        (640., 480.),
        Platform::Web,
        OS::Mac,
        Box::new(|| 0),
        Default::default(),
    );
    engine.enable_resumable_startup();
    let mut root_args = args(vec![]);
    root_args.component_template = Some(RefCell::new(vec![TreeNode::overlay(vec![
        TreeNode::native(),
    ])]));
    let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
    let overlay = root.children.get()[0].clone();
    root.children.set(vec![]);
    root.attach_children(vec![], &engine.runtime_context, &root.parent_frame);
    finish_yielded_startup(&mut engine);
    assert_eq!(overlay.attached.get(), 0);
    assert!(overlay.children.get().is_empty());
    assert!(root
        .presentation_children(&engine.runtime_context)
        .is_empty());
}

fn finish_yielded_startup(engine: &mut PaxEngine) {
    for _ in 0..2000 {
        if !engine.startup_pending() {
            return;
        }
        engine.service_startup(0.0, || 0.0);
        if engine.accept_startup_measurements() {
            engine.resume_startup_after_presentation();
        }
    }
    panic!("yielded startup did not finish");
}
