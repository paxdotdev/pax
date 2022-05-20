use std::cell::RefCell;
use std::rc::Rc;

use piet_common::RenderContext;
use crate::{HandlerRegistry, ComponentInstance, TabCache, RenderNode, RenderNodePtr, RenderNodePtrList, RenderTreeContext, InstantiationArgs};
use pax_runtime_api::{PropertyInstance, PropertyLiteral, Size2D, Transform2D};
use pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};

/// A special "control-flow" primitive, Conditional (`if`) allows for a
/// subtree of a component template to be rendered conditionally,
/// based on the value of the property `boolean_expression`.
/// The Pax compiler handles ConditionalInstance specially
/// with the `if` syntax in templates.
pub struct ConditionalInstance<R: 'static + RenderContext> {
    pub instance_id: u64,
    pub primitive_children: RenderNodePtrList<R>,
    pub transform: Rc<RefCell<dyn PropertyInstance<Transform2D>>>,
    pub boolean_expression: Box<dyn PropertyInstance<bool>>,
    pub empty_children: RenderNodePtrList<R>,
    tab_cache: TabCache<R>,
}

impl<R: 'static + RenderContext> RenderNode<R> for ConditionalInstance<R> {


    fn get_tab_cache(&mut self) -> &mut TabCache<R> {
        &mut self.tab_cache
    }

    fn get_instance_id(&self) -> u64 {
        self.instance_id
    }

    fn instantiate(mut args: InstantiationArgs<R>) -> Rc<RefCell<Self>> where Self: Sized {
        let mut instance_registry = (*args.instance_registry).borrow_mut();
        let instance_id = instance_registry.mint_id();
        let ret = Rc::new(RefCell::new(Self {
            instance_id,
            primitive_children: match args.children {
                None => {Rc::new(RefCell::new(vec![]))}
                Some(children) => children
            },
            transform: args.transform,
            boolean_expression: args.conditional_boolean_expression.expect("Conditional requires boolean_expression"),
            empty_children: Rc::new(RefCell::new(vec![])),
            tab_cache: TabCache::new(),
        }));

        instance_registry.register(instance_id, Rc::clone(&ret) as RenderNodePtr<R>);
        ret
    }

    fn compute_properties(&mut self, rtc: &mut RenderTreeContext<R>) {
        if let Some(boolean_expression) = rtc.compute_vtable_value(self.boolean_expression._get_vtable_id()) {
            let old_value = *self.boolean_expression.get();
            let new_value = if let TypesCoproduct::bool(v) = boolean_expression { v } else { unreachable!() };

            if old_value && !new_value {
                //mark subtree as `unmounted` -- this will trigger the dismount lifecycle event in the main render loop
                //Note that it's not required to set_mounted(... true) because the main render loop will automatically
                //flip that bit if a node is rendered.  In other words: it's important that Conditional not continue
                //to return unmounted children in `get_rendering_children`, otherwise they will automatically be `mounted` again.
                (*self.primitive_children).borrow_mut().iter().for_each(|child| {
                    (*(*child)).borrow_mut().unmount_recursive(rtc, false);
                })
            }

            self.boolean_expression.set(new_value);
        }
    }

    fn should_flatten(&self) -> bool {
        true
    }
    fn get_rendering_children(&self) -> RenderNodePtrList<R> {
        if *self.boolean_expression.get() {
            Rc::clone(&self.primitive_children)
        } else {
            Rc::clone(&self.empty_children)
        }

    }
    fn get_size(&self) -> Option<Size2D> { None }
    fn compute_size_within_bounds(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }
    fn get_transform(&mut self) -> Rc<RefCell<dyn PropertyInstance<Transform2D>>> { Rc::clone(&self.transform) }


}
