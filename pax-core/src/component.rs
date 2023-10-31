use piet_common::RenderContext;
use std::cell::RefCell;
use std::rc::Rc;

use crate::{HandlerRegistry, InstantiationArgs, NodeType, InstanceNode, InstanceNodePtr, InstanceNodePtrList, RenderTreeContext, ExpandedNode};
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
    /// A flag used for special-casing how we manage the runtime properties stack. When
    /// evaluating a given component's template, we must push to the runtime stack for component's
    /// managed by `for`, while ignoring the runtime properties stack for non-`for`-managed components (most userland components)
    pub is_managed_by_repeat: bool,

    instance_prototypical_properties: Rc<RefCell<PropertiesCoproduct>>,
    instance_prototypical_common_properties: Rc<RefCell<CommonProperties>>,
}

impl<R: 'static + RenderContext> InstanceNode<R> for ComponentInstance<R> {

    fn get_instance_id(&self) -> u32 {
        self.instance_id
    }
    fn get_rendering_children(&self) -> InstanceNodePtrList<R> {
        Rc::clone(&self.template)
    }
    fn get_node_type(&self) -> NodeType {
        if self.is_managed_by_repeat {
            NodeType::RepeatManagedComponent
        } else {
            NodeType::Component
        }
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
        let mut instance_registry = (*args.instance_registry).borrow_mut();
        let instance_id = instance_registry.mint_instance_id();

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
            is_managed_by_repeat: false,
        }));

        instance_registry.register(instance_id, Rc::clone(&ret) as InstanceNodePtr<R>);
        ret
    }

    fn get_size(&self) -> Option<(Size, Size)> {
        None
    }
    fn compute_size_within_bounds(&self, bounds: (f64, f64)) -> (f64, f64) {
        bounds
    }

    fn handle_push_runtime_properties_stack_frame(&mut self, rtc: &mut RenderTreeContext<R>) {
        (*rtc.runtime).borrow_mut().push_stack_frame(
            Rc::clone(&self.properties),
            self.timeline.clone(),
        );
    }

    fn handle_compute_properties(&mut self, rtc: &mut RenderTreeContext<R>) -> Rc<RefCell<ExpandedNode<R> {
        self.common_properties.compute_properties(rtc);
        (*self.compute_properties_fn)(Rc::clone(&self.properties), rtc);
    }

    fn handle_pop_runtime_properties_stack_frame(&mut self, rtc: &mut RenderTreeContext<R>) {
        (*rtc.runtime).borrow_mut().pop_stack_frame();
    }

    fn get_layer_type(&mut self) -> Layer {
        Layer::DontCare
    }
}
