use crate::api::math::Transform2;
use crate::api::NodeContext;
use crate::node_interface::NodeLocal;
use pax_runtime_api::Interpolatable;

/// Trait for nodes that semantically interpret child content.
///
/// Containers can call this from their existing mount logic to install reactive
/// behavior on top of the runtime's normalized `content_children` view.
pub trait Container {
    fn bind_container(&self, _ctx: &NodeContext) {}
}

/// Engine-internal selector for which child family should be normalized into
/// `NodeContext::content_children`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentChildrenSource {
    Direct,
    Slot,
}

/// Parent-local frame assigned by a container to one of its content children.
///
/// This behaves like a virtual wrapper node inside the parent: the frame's
/// transform is composed onto the parent transform and its bounds become the
/// child container bounds.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ContainerFrame {
    pub transform: Transform2<NodeLocal, NodeLocal>,
    pub bounds: (f64, f64),
}

impl Interpolatable for ContainerFrame {}

#[cfg(test)]
mod tests {
    use crate::api::math::Transform2;
    use crate::api::CommonProperties;
    use crate::api::Layer;
    use crate::{
        BaseInstance, ComponentInstance, ExpandedNode, Globals, InstanceFlags, InstanceNode,
        InstantiationArgs, RuntimeContext, RuntimePropertiesStackFrame, TransformAndBounds,
    };
    use pax_runtime_api::pax_value::PaxAny;
    use pax_runtime_api::{Platform, Property, OS};
    use std::cell::RefCell;
    use std::fmt;
    use std::rc::Rc;

    fn test_globals() -> Globals {
        Globals {
            frames_elapsed: Property::new(0),
            viewport: Property::new(TransformAndBounds {
                transform: Transform2::identity(),
                bounds: (100.0, 100.0),
            }),
            browser_allows_scroller_vector_layers: Property::new(true),
            browser_allows_nested_scroller_vector_layers: Property::new(true),
            platform: Platform::Unknown,
            os: OS::Unknown,
            get_elapsed_millis: Rc::new(|| 0),
        }
    }

    fn default_properties_factory() -> Box<
        dyn Fn(
            Rc<RuntimePropertiesStackFrame>,
            Option<Rc<ExpandedNode>>,
        ) -> Option<Rc<RefCell<PaxAny>>>,
    > {
        Box::new(|_, _| Some(Rc::new(RefCell::new(PaxAny::Builtin(Default::default())))))
    }

    fn default_common_properties_factory() -> Box<
        dyn Fn(
            Rc<RuntimePropertiesStackFrame>,
            Option<Rc<ExpandedNode>>,
        ) -> Option<Rc<RefCell<CommonProperties>>>,
    > {
        Box::new(|_, _| Some(Rc::new(RefCell::new(CommonProperties::default()))))
    }

    fn component_args(
        template: Option<Vec<Rc<dyn InstanceNode>>>,
        children: Option<Vec<Rc<dyn InstanceNode>>>,
    ) -> InstantiationArgs {
        InstantiationArgs {
            prototypical_common_properties: crate::CommonPropertiesInit::Factory(
                default_common_properties_factory(),
            ),
            prototypical_properties: crate::PropertiesInit::Factory(default_properties_factory()),
            handler_registry: None,
            children: children.map(RefCell::new),
            component_template: template.map(RefCell::new),
            template_node_identifier: None,
            transition_config: Default::default(),
            properties_scope: crate::PropertiesScopeInit::None,
        }
    }

    fn direct_node_args(children: Vec<Rc<dyn InstanceNode>>) -> InstantiationArgs {
        InstantiationArgs {
            prototypical_common_properties: crate::CommonPropertiesInit::Factory(
                default_common_properties_factory(),
            ),
            prototypical_properties: crate::PropertiesInit::Factory(default_properties_factory()),
            handler_registry: None,
            children: Some(RefCell::new(children)),
            component_template: None,
            template_node_identifier: None,
            transition_config: Default::default(),
            properties_scope: crate::PropertiesScopeInit::None,
        }
    }

    struct TestDirectNode {
        base: BaseInstance,
    }

    impl InstanceNode for TestDirectNode {
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

        fn resolve_debug(
            &self,
            f: &mut fmt::Formatter,
            _expanded_node: Option<&ExpandedNode>,
        ) -> fmt::Result {
            f.debug_struct("TestDirectNode").finish()
        }

        fn base(&self) -> &BaseInstance {
            &self.base
        }
    }

    #[test]
    fn direct_content_children_reflect_mounted_children() {
        let leaf: Rc<dyn InstanceNode> = TestDirectNode::instantiate(direct_node_args(Vec::new()));
        let direct: Rc<dyn InstanceNode> =
            TestDirectNode::instantiate(direct_node_args(vec![leaf]));
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![Rc::clone(&direct)]), None));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);
        let direct_node = root.children.get().first().cloned().unwrap();
        let node_ctx = direct_node.get_node_context(&context);
        let content_children = node_ctx.content_children.get();

        assert_eq!(content_children.len(), 1);
        assert_eq!(content_children[0].id, direct_node.children.get()[0].id);
        assert_eq!(node_ctx.content_children_count.get(), 1);
    }

    #[test]
    fn slot_content_children_reflect_flattened_slot_children() {
        let leaf: Rc<dyn InstanceNode> = TestDirectNode::instantiate(direct_node_args(Vec::new()));
        let slotted_component: Rc<dyn InstanceNode> =
            ComponentInstance::instantiate(component_args(Some(Vec::new()), Some(vec![leaf])));
        let root_component = ComponentInstance::instantiate(component_args(
            Some(vec![Rc::clone(&slotted_component)]),
            None,
        ));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);
        let component_node = root.children.get().first().cloned().unwrap();
        let node_ctx = component_node.get_node_context(&context);
        let content_children = node_ctx.content_children.get();

        assert_eq!(content_children.len(), 1);
        assert_eq!(
            content_children[0].id,
            component_node.expanded_and_flattened_slot_children.get()[0].id
        );
        assert_eq!(node_ctx.content_children_count.get(), 1);
    }
}
