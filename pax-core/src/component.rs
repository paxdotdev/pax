use piet_common::RenderContext;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use crate::{
    recurse_expand_nodes, ExpandedNode, HandlerRegistry, InstanceNode, InstanceNodePtr,
    InstanceNodePtrList, InstantiationArgs, NodeType, PropertiesTreeContext, RenderTreeContext,
};

use pax_runtime_api::{CommonProperties, Layer, Size, Timeline};

use crate::PropertiesComputable;

/// A render node with its own runtime context.  Will push a frame
/// to the runtime stack including the specified `slot_children` and
/// a `dyn Any` properties object.  `Component` is used at the root of
/// applications, at the root of reusable components like `Stacker`, and
/// in special applications like `Repeat` where it houses the `RepeatItem`
/// properties attached to each of Repeat's virtual nodes.
pub struct ComponentInstance<R: 'static + RenderContext> {
    pub(crate) instance_id: u32,
    pub template: InstanceNodePtrList<R>,
    pub slot_children: InstanceNodePtrList<R>,
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry>>>,
    pub timeline: Option<Rc<RefCell<Timeline>>>,
    pub compute_properties_fn: Box<dyn FnMut(Rc<RefCell<dyn Any>>, &mut PropertiesTreeContext<R>)>,

    instance_prototypical_properties: Rc<RefCell<dyn Any>>,
    instance_prototypical_common_properties: Rc<RefCell<CommonProperties>>,
}

impl<R: 'static + RenderContext> InstanceNode<R> for ComponentInstance<R> {
    fn get_instance_id(&self) -> u32 {
        self.instance_id
    }
    fn get_instance_children(&self) -> InstanceNodePtrList<R> {
        Rc::clone(&self.template)
    }
    fn get_node_type(&self) -> NodeType {
        NodeType::Component
    }
    fn get_handler_registry(&self) -> Option<Rc<RefCell<HandlerRegistry>>> {
        match &self.handler_registry {
            Some(registry) => Some(Rc::clone(&registry)),
            _ => None,
        }
    }

    fn get_slot_children(&self) -> Option<InstanceNodePtrList<R>> {
        Some(Rc::clone(&self.slot_children))
    }

    fn instantiate(args: InstantiationArgs<R>) -> Rc<RefCell<Self>> {
        let mut node_registry = (*args.node_registry).borrow_mut();
        let instance_id = node_registry.mint_instance_id();

        let template = match args.component_template {
            Some(t) => t,
            None => Rc::new(RefCell::new(vec![])),
        };

        let ret = Rc::new(RefCell::new(ComponentInstance {
            instance_id,
            template,
            slot_children: match args.children {
                Some(children) => children,
                None => Rc::new(RefCell::new(vec![])),
            },
            instance_prototypical_common_properties: args.common_properties,
            instance_prototypical_properties: args.properties,
            compute_properties_fn: args
                .compute_properties_fn
                .expect("must pass a compute_properties_fn to a Component instance"),
            timeline: None,
            handler_registry: args.handler_registry,
        }));

        node_registry.register(instance_id, Rc::clone(&ret) as InstanceNodePtr<R>);
        ret
    }

    fn manages_own_subtree_for_expansion(&self) -> bool {
        true
    }

    fn expand_node_and_compute_properties(
        &mut self,
        ptc: &mut PropertiesTreeContext<R>,
    ) -> Rc<RefCell<ExpandedNode<R>>> {
        let this_expanded_node = ExpandedNode::get_or_create_with_prototypical_properties(
            ptc,
            &self.instance_prototypical_properties,
            &self.instance_prototypical_common_properties,
        );
        // let properties_wrapped =  this_expanded_node.borrow().get_properties();
        this_expanded_node
            .borrow_mut()
            .set_expanded_and_flattened_slot_children(
                ptc.expanded_and_flattened_slot_children.take(),
            );

        ptc.push_stack_frame(Rc::clone(&this_expanded_node.borrow().get_properties()));

        for template_instance_root in self.template.borrow().iter() {
            let mut new_ptc = ptc.clone();
            new_ptc.current_instance_node = Rc::clone(template_instance_root);
            new_ptc.current_instance_id = template_instance_root.borrow().get_instance_id();
            new_ptc.current_expanded_node = None;
            let template_expanded_root = recurse_expand_nodes(&mut new_ptc);
            this_expanded_node
                .borrow_mut()
                .append_child_expanded_node(template_expanded_root);
        }

        ptc.pop_stack_frame();
        this_expanded_node
    }

    fn get_layer_type(&mut self) -> Layer {
        Layer::DontCare
    }
}
