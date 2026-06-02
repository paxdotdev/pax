use std::collections::HashSet;
use std::rc::{Rc, Weak};
use_RefCell!();

use pax_runtime_api::{borrow, use_RefCell, ImplToFromPaxAny, Numeric, Property};

use crate::api::Layer;
use crate::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};

/// A special "control-flow" primitive (a la `yield` or perhaps `goto`) that
/// renders projected payload into a node's encapsulated implementation.
///
/// `Slot` relies on raw `projected_children` being present on the runtime stack
/// and will not render any content if there are none. Projection is the engine
/// transport mechanism; semantic container logic should usually reason in terms
/// of `received_children` instead.
///
/// Consider a Stacker:  the owner of a Stacker passes the Stacker some nodes to render
/// inside the cells of the Stacker.  To the owner of the Stacker, those nodes might seem like
/// received children. Inside Stacker's encapsulated implementation, those
/// received children travel as projected children until `Slot` becomes their
/// rendered home. This same technique is portable and applicable elsewhere via
/// `Slot`.
pub struct SlotInstance {
    base: BaseInstance,
}

impl ImplToFromPaxAny for Slot {}

///Contains the index value for slot, either a literal or an expression.
#[derive(Default)]
pub struct Slot {
    // HACK: these two properties are being used in update:
    pub index: Property<Numeric>,
    pub is_remainder: Property<bool>,
    pub last_node_id: Property<usize>,
    // to compute this:
    pub showing_node: Property<Weak<ExpandedNode>>,
}

#[derive(Clone, Copy)]
enum SlotProjection {
    Index(i64),
    Remainder,
}

impl InstanceNode for SlotInstance {
    fn instantiate(args: InstantiationArgs) -> Rc<Self>
    where
        Self: Sized,
    {
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    invisible_to_slot: false,
                    invisible_to_raycasting: true,
                    layer: Layer::DontCare,
                    is_component: false,
                    is_slot: true,
                },
            ),
        })
    }

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        let weak_ref_self = Rc::downgrade(expanded_node);
        let cloned_context = Rc::clone(context);

        let containing = expanded_node
            .containing_component
            .upgrade()
            .expect("slot to have a containing component");

        let nodes = containing.expanded_and_flattened_projected_children.clone();
        let projected_children_listener = containing.projected_children_changed.clone();
        let slot_projection_listener = containing.slot_projection_changed.clone();

        let (index, is_remainder) =
            expanded_node.with_properties_unwrapped(|properties: &mut Slot| {
                (properties.index.clone(), properties.is_remainder.clone())
            });

        if !is_remainder.get() {
            let index_for_listener = index.clone();
            let slot_projection_listener = slot_projection_listener.clone();
            expanded_node
                .changed_listener
                .replace_with(Property::computed_with_name(
                    move || {
                        let _ = index_for_listener.get();
                        slot_projection_listener.set(());
                    },
                    &[index.untyped()],
                    &format!("slot index listener (node id: {})", expanded_node.id.0),
                ));
        }

        let deps = vec![
            index.untyped(),
            is_remainder.untyped(),
            nodes.untyped(),
            slot_projection_listener.untyped(),
        ];

        expanded_node
            .children
            .replace_with(Property::computed_with_name(
                move || {
                    let Some(cloned_expanded_node) = weak_ref_self.upgrade() else {
                        panic!("ran evaluator after expanded node dropped (repeat elem)")
                    };
                    let children =
                        compute_slot_projection(&cloned_expanded_node, &containing, nodes.get());
                    let ret = cloned_expanded_node.attach_children(
                        children,
                        &cloned_context,
                        &cloned_expanded_node.parent_frame,
                    );
                    projected_children_listener.set(());
                    ret
                },
                &deps,
                &format!("projected_children (node id: {})", expanded_node.id.0),
            ));
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Slot").finish()
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::math::Transform2;
    use crate::api::CommonProperties;
    use crate::{
        ComponentInstance, Globals, ReceivedChildrenSource, RouteLocation,
        RuntimePropertiesStackFrame, TransformAndBounds,
    };
    use pax_runtime_api::borrow_mut;
    use pax_runtime_api::pax_value::{PaxAny, PaxValue, ToFromPaxAny};
    use pax_runtime_api::{Platform, TargetInfo, OS};
    use std::cell::RefCell;
    use std::iter;

    fn test_globals() -> Globals {
        Globals {
            elapsed_frames: Property::new(0),
            elapsed_millis: Property::new(0),
            viewport: Property::new(TransformAndBounds {
                transform: Transform2::identity(),
                bounds: (100.0, 100.0),
            }),
            gyro: Property::new(Default::default()),
            accel: Property::new(Default::default()),
            route_location: Property::new(RouteLocation::root()),
            browser_allows_scroller_vector_layers: Property::new(true),
            browser_allows_nested_scroller_vector_layers: Property::new(true),
            platform: Platform::Unknown,
            os: OS::Unknown,
            target: TargetInfo::new(Platform::Unknown, OS::Unknown),
            get_elapsed_millis: Rc::new(|| 0),
        }
    }

    fn default_common_properties_factory() -> Box<
        dyn Fn(
            Rc<RuntimePropertiesStackFrame>,
            Option<Rc<ExpandedNode>>,
        ) -> Option<Rc<RefCell<CommonProperties>>>,
    > {
        Box::new(|_, expanded_node| {
            expanded_node
                .is_none()
                .then(|| Rc::new(RefCell::new(CommonProperties::default())))
        })
    }

    fn empty_properties_factory() -> Box<
        dyn Fn(
            Rc<RuntimePropertiesStackFrame>,
            Option<Rc<ExpandedNode>>,
        ) -> Option<Rc<RefCell<PaxAny>>>,
    > {
        Box::new(|_, expanded_node| {
            expanded_node
                .is_none()
                .then(|| Rc::new(RefCell::new(PaxAny::Builtin(Default::default()))))
        })
    }

    fn component_args(
        template: Option<Vec<Rc<dyn InstanceNode>>>,
        children: Option<Vec<Rc<dyn InstanceNode>>>,
    ) -> InstantiationArgs {
        InstantiationArgs {
            prototypical_common_properties: crate::CommonPropertiesInit::Factory(
                default_common_properties_factory(),
            ),
            prototypical_properties: crate::PropertiesInit::Factory(empty_properties_factory()),
            handler_registry: None,
            children: children.map(RefCell::new),
            component_template: template.map(RefCell::new),
            component_settings: None,
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config: Default::default(),
            properties_scope: crate::PropertiesScopeInit::None,
        }
    }

    fn leaf_args(label: &'static str) -> InstantiationArgs {
        InstantiationArgs {
            prototypical_common_properties: crate::CommonPropertiesInit::Factory(
                default_common_properties_factory(),
            ),
            prototypical_properties: crate::PropertiesInit::Factory(Box::new(
                move |_, expanded_node| {
                    expanded_node.is_none().then(|| {
                        Rc::new(RefCell::new(PaxAny::Builtin(PaxValue::String(
                            label.to_string(),
                        ))))
                    })
                },
            )),
            handler_registry: None,
            children: None,
            component_template: None,
            component_settings: None,
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config: Default::default(),
            properties_scope: crate::PropertiesScopeInit::None,
        }
    }

    fn slot_args(index: Option<Property<Numeric>>) -> InstantiationArgs {
        InstantiationArgs {
            prototypical_common_properties: crate::CommonPropertiesInit::Factory(
                default_common_properties_factory(),
            ),
            prototypical_properties: crate::PropertiesInit::Factory(Box::new(
                move |_, expanded_node| {
                    expanded_node.is_none().then(|| {
                        let mut slot = Slot::default();
                        match &index {
                            Some(index) => slot.index = index.clone(),
                            None => slot.is_remainder = Property::new(true),
                        }
                        Rc::new(RefCell::new(slot.to_pax_any()))
                    })
                },
            )),
            handler_registry: None,
            children: None,
            component_template: None,
            component_settings: None,
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config: Default::default(),
            properties_scope: crate::PropertiesScopeInit::None,
        }
    }

    fn container_args(children: Vec<Rc<dyn InstanceNode>>) -> InstantiationArgs {
        InstantiationArgs {
            prototypical_common_properties: crate::CommonPropertiesInit::Factory(
                default_common_properties_factory(),
            ),
            prototypical_properties: crate::PropertiesInit::Factory(empty_properties_factory()),
            handler_registry: None,
            children: Some(RefCell::new(children)),
            component_template: None,
            component_settings: None,
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config: Default::default(),
            properties_scope: crate::PropertiesScopeInit::None,
        }
    }

    struct TestContainer {
        base: BaseInstance,
    }

    impl InstanceNode for TestContainer {
        fn instantiate(args: InstantiationArgs) -> Rc<Self>
        where
            Self: Sized,
        {
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
            })
        }

        fn handle_mount(
            self: Rc<Self>,
            expanded_node: &Rc<ExpandedNode>,
            context: &Rc<RuntimeContext>,
        ) {
            let env = Rc::clone(&expanded_node.stack);
            let children = borrow!(self.base().get_instance_children());
            let children_with_env = children.iter().cloned().zip(iter::repeat(env));
            let new_children = expanded_node.generate_children(
                children_with_env,
                context,
                &expanded_node.parent_frame,
                true,
            );
            expanded_node.children.set(new_children);
        }

        fn resolve_debug(
            &self,
            f: &mut std::fmt::Formatter,
            _expanded_node: Option<&ExpandedNode>,
        ) -> std::fmt::Result {
            f.debug_struct("TestContainer").finish()
        }

        fn base(&self) -> &BaseInstance {
            &self.base
        }
    }

    struct TestLeaf {
        base: BaseInstance,
    }

    impl InstanceNode for TestLeaf {
        fn instantiate(args: InstantiationArgs) -> Rc<Self>
        where
            Self: Sized,
        {
            Rc::new(Self {
                base: BaseInstance::new(
                    args,
                    InstanceFlags {
                        invisible_to_slot: false,
                        invisible_to_raycasting: false,
                        layer: Layer::DontCare,
                        is_component: false,
                        is_slot: false,
                    },
                ),
            })
        }

        fn resolve_debug(
            &self,
            f: &mut std::fmt::Formatter,
            _expanded_node: Option<&ExpandedNode>,
        ) -> std::fmt::Result {
            f.debug_struct("TestLeaf").finish()
        }

        fn base(&self) -> &BaseInstance {
            &self.base
        }
    }

    struct TestProjectedContainer {
        base: BaseInstance,
    }

    impl InstanceNode for TestProjectedContainer {
        fn instantiate(args: InstantiationArgs) -> Rc<Self>
        where
            Self: Sized,
        {
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
            })
        }

        fn received_children_source(&self) -> ReceivedChildrenSource {
            ReceivedChildrenSource::Projected
        }

        fn handle_setup_projected_children(
            self: Rc<Self>,
            expanded_node: &Rc<ExpandedNode>,
            context: &Rc<RuntimeContext>,
        ) {
            if let Some(containing_component) = expanded_node.containing_component.upgrade() {
                let env = Rc::clone(&expanded_node.stack);
                let children = borrow!(self.base().get_instance_children());
                let children_with_env = children.iter().cloned().zip(iter::repeat(env));
                let new_projected_children = containing_component.create_children_detached(
                    children_with_env,
                    context,
                    &Rc::downgrade(expanded_node),
                );
                *borrow_mut!(expanded_node.expanded_projected_children) =
                    Some(new_projected_children);
            }
        }

        fn handle_mount(
            self: Rc<Self>,
            expanded_node: &Rc<ExpandedNode>,
            context: &Rc<RuntimeContext>,
        ) {
            let weak_ref_self = Rc::downgrade(expanded_node);
            let cloned_context = Rc::clone(context);
            let parent_frame = expanded_node.parent_frame.clone();
            let projected_children = expanded_node
                .expanded_and_flattened_projected_children
                .clone();
            let deps = [projected_children.untyped()];
            expanded_node
                .children
                .replace_with(Property::computed_with_name(
                    move || {
                        let Some(node) = weak_ref_self.upgrade() else {
                            panic!("ran evaluator after expanded node dropped")
                        };
                        node.attach_children(
                            projected_children.get(),
                            &cloned_context,
                            &parent_frame,
                        )
                    },
                    &deps,
                    &format!(
                        "test_projected_container_children (node id: {})",
                        expanded_node.id.0
                    ),
                ));
            let _ = expanded_node.children.get();
        }

        fn update(self: Rc<Self>, expanded_node: &Rc<ExpandedNode>, _context: &Rc<RuntimeContext>) {
            expanded_node.compute_flattened_projected_children();
        }

        fn resolve_debug(
            &self,
            f: &mut std::fmt::Formatter,
            _expanded_node: Option<&ExpandedNode>,
        ) -> std::fmt::Result {
            f.debug_struct("TestProjectedContainer").finish()
        }

        fn base(&self) -> &BaseInstance {
            &self.base
        }
    }

    fn labels(nodes: &[Rc<ExpandedNode>]) -> Vec<String> {
        nodes
            .iter()
            .map(|node| {
                node.with_properties_unwrapped(|label: &mut PaxValue| match label {
                    PaxValue::String(label) => label.clone(),
                    _ => panic!("expected string label"),
                })
            })
            .collect()
    }

    #[test]
    fn remainder_slot_receives_unconsumed_projected_children() {
        let slot_zero: Rc<dyn InstanceNode> =
            SlotInstance::instantiate(slot_args(Some(Property::new(Numeric::I64(0)))));
        let remainder_slot: Rc<dyn InstanceNode> = SlotInstance::instantiate(slot_args(None));
        let nested: Rc<dyn InstanceNode> =
            TestContainer::instantiate(container_args(vec![remainder_slot]));
        let host: Rc<dyn InstanceNode> =
            TestContainer::instantiate(container_args(vec![slot_zero, nested]));
        let projected_children = ["red", "orange", "yellow", "green"]
            .into_iter()
            .map(|label| TestLeaf::instantiate(leaf_args(label)) as Rc<dyn InstanceNode>)
            .collect::<Vec<_>>();
        let slotted_component: Rc<dyn InstanceNode> = ComponentInstance::instantiate(
            component_args(Some(vec![host]), Some(projected_children)),
        );
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![slotted_component]), None));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);

        let component = root.children.get()[0].clone();
        let host = component.children.get()[0].clone();
        let slot_zero = host.children.get()[0].clone();
        let nested = host.children.get()[1].clone();
        let remainder_slot = nested.children.get()[0].clone();

        assert_eq!(labels(&slot_zero.children.get()), vec!["red"]);
        assert_eq!(
            labels(&remainder_slot.children.get()),
            vec!["orange", "yellow", "green"]
        );
    }

    #[test]
    fn remainder_slot_reacts_to_dynamic_prior_index() {
        let selected = Property::new(Numeric::I64(2));
        let featured_slot: Rc<dyn InstanceNode> =
            SlotInstance::instantiate(slot_args(Some(selected.clone())));
        let remainder_slot: Rc<dyn InstanceNode> = SlotInstance::instantiate(slot_args(None));
        let host: Rc<dyn InstanceNode> =
            TestContainer::instantiate(container_args(vec![featured_slot, remainder_slot]));
        let projected_children = ["red", "orange", "yellow", "green"]
            .into_iter()
            .map(|label| TestLeaf::instantiate(leaf_args(label)) as Rc<dyn InstanceNode>)
            .collect::<Vec<_>>();
        let slotted_component: Rc<dyn InstanceNode> = ComponentInstance::instantiate(
            component_args(Some(vec![host]), Some(projected_children)),
        );
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![slotted_component]), None));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);
        let component = root.children.get()[0].clone();
        let host = component.children.get()[0].clone();
        let featured_slot = host.children.get()[0].clone();
        let remainder_slot = host.children.get()[1].clone();
        assert_eq!(labels(&featured_slot.children.get()), vec!["yellow"]);
        assert_eq!(
            labels(&remainder_slot.children.get()),
            vec!["red", "orange", "green"]
        );

        selected.set(Numeric::I64(0));
        root.recurse_update(&context);

        assert_eq!(labels(&featured_slot.children.get()), vec!["red"]);
        assert_eq!(
            labels(&remainder_slot.children.get()),
            vec!["orange", "yellow", "green"]
        );
    }

    #[test]
    fn duplicate_explicit_slot_renders_empty_and_remainder_excludes_consumed_child() {
        let first_slot: Rc<dyn InstanceNode> =
            SlotInstance::instantiate(slot_args(Some(Property::new(Numeric::I64(1)))));
        let duplicate_slot: Rc<dyn InstanceNode> =
            SlotInstance::instantiate(slot_args(Some(Property::new(Numeric::I64(1)))));
        let remainder_slot: Rc<dyn InstanceNode> = SlotInstance::instantiate(slot_args(None));
        let host: Rc<dyn InstanceNode> = TestContainer::instantiate(container_args(vec![
            first_slot,
            duplicate_slot,
            remainder_slot,
        ]));
        let projected_children = ["red", "orange", "yellow", "green"]
            .into_iter()
            .map(|label| TestLeaf::instantiate(leaf_args(label)) as Rc<dyn InstanceNode>)
            .collect::<Vec<_>>();
        let slotted_component: Rc<dyn InstanceNode> = ComponentInstance::instantiate(
            component_args(Some(vec![host]), Some(projected_children)),
        );
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![slotted_component]), None));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);

        let component = root.children.get()[0].clone();
        let host = component.children.get()[0].clone();
        let first_slot = host.children.get()[0].clone();
        let duplicate_slot = host.children.get()[1].clone();
        let remainder_slot = host.children.get()[2].clone();

        assert_eq!(labels(&first_slot.children.get()), vec!["orange"]);
        assert!(duplicate_slot.children.get().is_empty());
        assert_eq!(
            labels(&remainder_slot.children.get()),
            vec!["red", "yellow", "green"]
        );
    }

    #[test]
    fn duplicate_slot_toggle_rehomes_projected_child_once() {
        let first_index = Property::new(Numeric::I64(0));
        let second_index = Property::new(Numeric::I64(0));
        let first_slot: Rc<dyn InstanceNode> =
            SlotInstance::instantiate(slot_args(Some(first_index)));
        let duplicate_slot: Rc<dyn InstanceNode> =
            SlotInstance::instantiate(slot_args(Some(second_index.clone())));
        let remainder_slot: Rc<dyn InstanceNode> = SlotInstance::instantiate(slot_args(None));
        let host: Rc<dyn InstanceNode> = TestContainer::instantiate(container_args(vec![
            first_slot,
            duplicate_slot,
            remainder_slot,
        ]));
        let projected_children = ["green", "blue"]
            .into_iter()
            .map(|label| TestLeaf::instantiate(leaf_args(label)) as Rc<dyn InstanceNode>)
            .collect::<Vec<_>>();
        let slotted_component: Rc<dyn InstanceNode> = ComponentInstance::instantiate(
            component_args(Some(vec![host]), Some(projected_children)),
        );
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![slotted_component]), None));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);
        root.recurse_update(&context);

        let component = root.children.get()[0].clone();
        let host = component.children.get()[0].clone();
        let first_slot = host.children.get()[0].clone();
        let duplicate_slot = host.children.get()[1].clone();
        let remainder_slot = host.children.get()[2].clone();

        assert_eq!(labels(&first_slot.children.get()), vec!["green"]);
        assert!(duplicate_slot.children.get().is_empty());
        assert_eq!(labels(&remainder_slot.children.get()), vec!["blue"]);

        second_index.set(Numeric::I64(1));
        root.recurse_update(&context);

        assert_eq!(labels(&first_slot.children.get()), vec!["green"]);
        assert_eq!(labels(&duplicate_slot.children.get()), vec!["blue"]);
        assert!(remainder_slot.children.get().is_empty());

        second_index.set(Numeric::I64(0));
        root.recurse_update(&context);

        assert_eq!(labels(&first_slot.children.get()), vec!["green"]);
        assert!(duplicate_slot.children.get().is_empty());
        assert_eq!(labels(&remainder_slot.children.get()), vec!["blue"]);
        assert!(duplicate_slot.mounted_children.borrow().is_empty());
        assert_eq!(remainder_slot.mounted_children.borrow().len(), 1);
    }

    #[test]
    fn reparenting_slot_child_clears_previous_owner_lists_before_previous_owner_recomputes() {
        let first_slot: Rc<dyn InstanceNode> =
            SlotInstance::instantiate(slot_args(Some(Property::new(Numeric::I64(0)))));
        let second_slot: Rc<dyn InstanceNode> =
            SlotInstance::instantiate(slot_args(Some(Property::new(Numeric::I64(0)))));
        let remainder_slot: Rc<dyn InstanceNode> = SlotInstance::instantiate(slot_args(None));
        let host: Rc<dyn InstanceNode> = TestContainer::instantiate(container_args(vec![
            first_slot,
            second_slot,
            remainder_slot,
        ]));
        let projected_children = ["green", "blue"]
            .into_iter()
            .map(|label| TestLeaf::instantiate(leaf_args(label)) as Rc<dyn InstanceNode>)
            .collect::<Vec<_>>();
        let slotted_component: Rc<dyn InstanceNode> = ComponentInstance::instantiate(
            component_args(Some(vec![host]), Some(projected_children)),
        );
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![slotted_component]), None));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);
        root.recurse_update(&context);

        let component = root.children.get()[0].clone();
        let host = component.children.get()[0].clone();
        let second_slot = host.children.get()[1].clone();
        let remainder_slot = host.children.get()[2].clone();
        let projected_child = remainder_slot.children.get()[0].clone();

        second_slot.attach_children(
            vec![Rc::clone(&projected_child)],
            &context,
            &second_slot.parent_frame,
        );

        assert!(remainder_slot.active_children.borrow().is_empty());
        assert!(remainder_slot.mounted_children.borrow().is_empty());
        assert_eq!(second_slot.mounted_children.borrow().len(), 1);
        assert_eq!(projected_child.attached.get(), 1);

        remainder_slot.attach_children(Vec::new(), &context, &remainder_slot.parent_frame);

        assert_eq!(second_slot.mounted_children.borrow().len(), 1);
        assert_eq!(projected_child.attached.get(), 1);
    }

    #[test]
    fn out_of_range_explicit_slot_renders_empty_without_consuming_remainder() {
        let out_of_range_slot: Rc<dyn InstanceNode> =
            SlotInstance::instantiate(slot_args(Some(Property::new(Numeric::I64(8)))));
        let remainder_slot: Rc<dyn InstanceNode> = SlotInstance::instantiate(slot_args(None));
        let host: Rc<dyn InstanceNode> =
            TestContainer::instantiate(container_args(vec![out_of_range_slot, remainder_slot]));
        let projected_children = ["red", "orange", "yellow", "green"]
            .into_iter()
            .map(|label| TestLeaf::instantiate(leaf_args(label)) as Rc<dyn InstanceNode>)
            .collect::<Vec<_>>();
        let slotted_component: Rc<dyn InstanceNode> = ComponentInstance::instantiate(
            component_args(Some(vec![host]), Some(projected_children)),
        );
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![slotted_component]), None));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);

        let component = root.children.get()[0].clone();
        let host = component.children.get()[0].clone();
        let out_of_range_slot = host.children.get()[0].clone();
        let remainder_slot = host.children.get()[1].clone();

        assert!(out_of_range_slot.children.get().is_empty());
        assert_eq!(
            labels(&remainder_slot.children.get()),
            vec!["red", "orange", "yellow", "green"]
        );
    }

    #[test]
    fn remainder_slot_before_explicit_slot_consumes_all_remaining_children() {
        let remainder_slot: Rc<dyn InstanceNode> = SlotInstance::instantiate(slot_args(None));
        let later_explicit_slot: Rc<dyn InstanceNode> =
            SlotInstance::instantiate(slot_args(Some(Property::new(Numeric::I64(0)))));
        let host: Rc<dyn InstanceNode> =
            TestContainer::instantiate(container_args(vec![remainder_slot, later_explicit_slot]));
        let projected_children = ["red", "orange", "yellow", "green"]
            .into_iter()
            .map(|label| TestLeaf::instantiate(leaf_args(label)) as Rc<dyn InstanceNode>)
            .collect::<Vec<_>>();
        let slotted_component: Rc<dyn InstanceNode> = ComponentInstance::instantiate(
            component_args(Some(vec![host]), Some(projected_children)),
        );
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![slotted_component]), None));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);

        let component = root.children.get()[0].clone();
        let host = component.children.get()[0].clone();
        let remainder_slot = host.children.get()[0].clone();
        let later_explicit_slot = host.children.get()[1].clone();

        assert_eq!(
            labels(&remainder_slot.children.get()),
            vec!["red", "orange", "yellow", "green"]
        );
        assert!(later_explicit_slot.children.get().is_empty());
    }

    #[test]
    fn remainder_slot_is_discovered_inside_projected_container_children() {
        let remainder_slot: Rc<dyn InstanceNode> = SlotInstance::instantiate(slot_args(None));
        let projected_container: Rc<dyn InstanceNode> =
            TestProjectedContainer::instantiate(container_args(vec![remainder_slot]));
        let projected_children = ["red", "orange", "yellow", "green"]
            .into_iter()
            .map(|label| TestLeaf::instantiate(leaf_args(label)) as Rc<dyn InstanceNode>)
            .collect::<Vec<_>>();
        let slotted_component: Rc<dyn InstanceNode> = ComponentInstance::instantiate(
            component_args(Some(vec![projected_container]), Some(projected_children)),
        );
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![slotted_component]), None));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);

        let component = root.children.get()[0].clone();
        let projected_container = component.children.get()[0].clone();
        let remainder_slot = projected_container.children.get()[0].clone();

        assert_eq!(
            labels(&remainder_slot.children.get()),
            vec!["red", "orange", "yellow", "green"]
        );
    }
}

fn compute_slot_projection(
    current_slot: &Rc<ExpandedNode>,
    containing_component: &Rc<ExpandedNode>,
    projected_children: Vec<Rc<ExpandedNode>>,
) -> Vec<Rc<ExpandedNode>> {
    let slot_sites = active_slot_sites(containing_component);
    let mut consumed = HashSet::new();

    for slot_site in slot_sites {
        let is_current = slot_site.id == current_slot.id;
        match slot_projection(&slot_site) {
            SlotProjection::Index(index) => {
                let Some(index) = normalize_slot_index(index) else {
                    if is_current {
                        log::warn!("slot index {} is negative; rendering no slot child", index);
                        return Vec::new();
                    }
                    continue;
                };

                if index >= projected_children.len() {
                    if is_current {
                        log::warn!(
                            "slot index {} is out of range for {} projected children; rendering no slot child",
                            index,
                            projected_children.len()
                        );
                        return Vec::new();
                    }
                    continue;
                }

                if consumed.contains(&index) {
                    if is_current {
                        log::warn!(
                            "slot index {} was already consumed by an earlier slot projection; rendering no slot child",
                            index
                        );
                        return Vec::new();
                    }
                    continue;
                }

                consumed.insert(index);
                if is_current {
                    return vec![Rc::clone(&projected_children[index])];
                }
            }
            SlotProjection::Remainder => {
                let remainder_indices = (0..projected_children.len())
                    .filter(|index| !consumed.contains(index))
                    .collect::<Vec<_>>();
                if is_current {
                    return remainder_indices
                        .into_iter()
                        .map(|index| Rc::clone(&projected_children[index]))
                        .collect();
                }
                consumed.extend(remainder_indices);
            }
        }
    }

    Vec::new()
}

fn normalize_slot_index(index: i64) -> Option<usize> {
    (index >= 0).then_some(index as usize)
}

fn slot_projection(slot_node: &ExpandedNode) -> SlotProjection {
    slot_node.with_properties_unwrapped(|properties: &mut Slot| {
        if properties.is_remainder.get() {
            SlotProjection::Remainder
        } else {
            SlotProjection::Index(properties.index.get().to_int())
        }
    })
}

fn active_slot_sites(containing_component: &Rc<ExpandedNode>) -> Vec<Rc<ExpandedNode>> {
    let mut slot_sites = Vec::new();
    for child in containing_component.children.get() {
        collect_active_slot_sites(&child, containing_component.id, &mut slot_sites);
    }
    slot_sites
}

fn collect_active_slot_sites(
    node: &Rc<ExpandedNode>,
    containing_component_id: crate::ExpandedNodeIdentifier,
    slot_sites: &mut Vec<Rc<ExpandedNode>>,
) {
    let instance = borrow!(node.instance_node);
    let flags = instance.base().flags();
    if flags.is_slot {
        if node
            .containing_component
            .upgrade()
            .is_some_and(|component| component.id == containing_component_id)
        {
            slot_sites.push(Rc::clone(node));
        }
        return;
    }

    if flags.is_component && node.id != containing_component_id {
        return;
    }
    drop(instance);

    for child in node.children.get() {
        collect_active_slot_sites(&child, containing_component_id, slot_sites);
    }
}
