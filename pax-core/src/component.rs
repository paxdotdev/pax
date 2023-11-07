use piet_common::RenderContext;
use std::cell::RefCell;
use std::rc::Rc;

use crate::{HandlerRegistry, InstantiationArgs, NodeType, InstanceNode, InstanceNodePtr, InstanceNodePtrList, RenderTreeContext, PropertiesTreeContext, ExpandedNode};
use pax_properties_coproduct::PropertiesCoproduct;

use pax_runtime_api::{CommonProperties, Layer, Size, Timeline};

use crate::PropertiesComputable;

/// A render node with its own runtime context.  Will push a frame
/// to the runtime stack including the specified `slot_children` and
/// `PropertiesCoproduct` object.  `Component` is used at the root of
/// applications, at the root of reusable components like `Stacker`, and
/// in special applications like `Repeat` where it houses the `RepeatItem`
/// properties attached to each of Repeat's virtual nodes.
pub struct ComponentInstance<R: 'static + RenderContext> {
    pub(crate) instance_id: u32,
    pub template: InstanceNodePtrList<R>,
    pub slot_children: InstanceNodePtrList<R>,
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry<R>>>>,
    pub timeline: Option<Rc<RefCell<Timeline>>>,
    pub compute_properties_fn:
        Box<dyn FnMut(Rc<RefCell<PropertiesCoproduct>>, &mut RenderTreeContext<R>)>,

    instance_prototypical_properties: Rc<RefCell<PropertiesCoproduct>>,
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
    fn get_handler_registry(&self) -> Option<Rc<RefCell<HandlerRegistry<R>>>> {
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
            instance_prototypical_common_properties: Rc::new(RefCell::new(args.common_properties)),
            instance_prototypical_properties: Rc::new(RefCell::new(args.properties)),
            compute_properties_fn: args
                .compute_properties_fn
                .expect("must pass a compute_properties_fn to a Component instance"),
            timeline: None,
            handler_registry: args.handler_registry,
        }));

        node_registry.register(instance_id, Rc::clone(&ret) as InstanceNodePtr<R>);
        ret
    }

    // fn get_size(&self) -> Option<(Size, Size)> {
    //     None
    // }
    // fn compute_size_within_bounds(&self, bounds: (f64, f64)) -> (f64, f64) {
    //     bounds
    // }

    fn manages_own_properties_subtree(&self) -> bool {
        true
    }


    fn handle_compute_properties(&mut self, ptc: &mut PropertiesTreeContext<R>) -> Rc<RefCell<ExpandedNode<R>>> {

        //ptc.push_stack_frame(...);
        //recurse and compute template subtree (once [per root])
        //ptc.pop_stack_frame();

        todo!("");
    }


    fn get_layer_type(&mut self) -> Layer {
        Layer::DontCare
    }
}
