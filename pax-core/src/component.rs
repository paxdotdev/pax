use piet_common::RenderContext;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use crate::{
    recurse_expand_nodes, ExpandedNode, HandlerRegistry, InstanceNode, InstanceNodePtr,
    InstanceNodePtrList, InstantiationArgs, NodeType, PropertiesTreeContext,
};

use pax_runtime_api::{CommonProperties, Layer, Timeline};

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

    instance_prototypical_properties_factory: Box<dyn Fn() -> Rc<RefCell<dyn Any>>>,
    instance_prototypical_common_properties_factory: Box<dyn Fn() -> Rc<RefCell<CommonProperties>>>,
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
            instance_prototypical_common_properties_factory: args
                .prototypical_common_properties_factory,
            instance_prototypical_properties_factory: args.prototypical_properties_factory,
            compute_properties_fn: args
                .compute_properties_fn
                .expect("must pass a compute_properties_fn to a Component instance"),
            timeline: None,
            handler_registry: args.handler_registry,
        }));

        node_registry.register(instance_id, Rc::clone(&ret) as InstanceNodePtr<R>);
        ret
    }

    fn is_invisible_to_raycasting(&self) -> bool {
        true
    }

    fn expand(&self, ptc: &mut PropertiesTreeContext<R>) -> Rc<RefCell<crate::ExpandedNode<R>>> {
        ExpandedNode::get_or_create_with_prototypical_properties(
            self.instance_id,
            ptc,
            &(self.instance_prototypical_properties_factory)(),
            &(self.instance_prototypical_common_properties_factory)(),
        )
    }

    fn expand_node_and_compute_properties(
        &mut self,
        ptc: &mut PropertiesTreeContext<R>,
    ) -> Rc<RefCell<ExpandedNode<R>>> {
        let this_expanded_node = self.expand(ptc);

        let expanded_and_flattened_slot_children = {
            let slot_children = self.slot_children.borrow();
            //Expand children in the context of the current containing component
            let mut expanded_slot_children = vec![];
            for child in (*slot_children).iter() {
                let mut new_ptc = ptc.clone();
                new_ptc.current_instance_node = Rc::clone(child);
                new_ptc.current_expanded_node = None;
                //TODOSAM does the stack need to be modified here?
                let child_expanded_node = recurse_expand_nodes(&mut new_ptc);
                expanded_slot_children.push(child_expanded_node);
            }

            //Now flatten those expanded children, ignoring (replacing with children) and node that`is_invisible_to_slot`, namely
            //[`ConditionalInstance`] and [`RepeatInstance`]
            let mut expanded_and_flattened_slot_children = vec![];
            for expanded_slot_child in expanded_slot_children {
                expanded_and_flattened_slot_children.extend(flatten_expanded_node_for_slot(
                    &Rc::clone(&expanded_slot_child),
                ));
            }

            expanded_and_flattened_slot_children
        };

        {
            this_expanded_node
                .borrow_mut()
                .set_expanded_and_flattened_slot_children(Some(
                    expanded_and_flattened_slot_children,
                ));
        }

        //TODOSAM: make sure this is the right place to do this when we have more than one component!
        let last_containing_component = std::mem::replace(
            &mut ptc.current_containing_component,
            Rc::downgrade(&this_expanded_node),
        );

        //Compute properties
        (*self.compute_properties_fn)(
            Rc::clone(&this_expanded_node.borrow().get_properties()),
            ptc,
        );

        ptc.push_stack_frame(Rc::clone(&this_expanded_node.borrow().get_properties()));

        for child in self.template.borrow().iter() {
            let mut new_ptc = ptc.clone();
            new_ptc.current_instance_node = Rc::clone(child);
            new_ptc.current_expanded_node = None;
            let child_expanded_node = recurse_expand_nodes(&mut new_ptc);
            child_expanded_node.borrow_mut().parent_expanded_node =
                Some(Rc::downgrade(&this_expanded_node));
            this_expanded_node
                .borrow_mut()
                .append_child_expanded_node(child_expanded_node);
        }

        ptc.pop_stack_frame();
        ptc.current_containing_component = last_containing_component;
        this_expanded_node
    }

    fn get_layer_type(&mut self) -> Layer {
        Layer::DontCare
    }

    #[cfg(debug_assertions)]
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode<R>>,
    ) -> std::fmt::Result {
        f.debug_struct("Component").finish()
    }
}

/// Helper function that accepts a
fn flatten_expanded_node_for_slot<R: 'static + RenderContext>(
    node: &Rc<RefCell<ExpandedNode<R>>>,
) -> Vec<Rc<RefCell<ExpandedNode<R>>>> {
    let mut result = vec![];

    let is_invisible_to_slot = {
        let node_borrowed = node.borrow();
        let instance_node_borrowed = node_borrowed.instance_node.borrow();
        instance_node_borrowed.is_invisible_to_slot()
    };
    if is_invisible_to_slot {
        // If the node is invisible, recurse on its children
        for child in node.borrow().get_children_expanded_nodes().iter() {
            result.extend(flatten_expanded_node_for_slot(child));
        }
    } else {
        // If the node is visible, add it to the result
        result.push(Rc::clone(node));
    }

    result
}
