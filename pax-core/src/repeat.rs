use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;
use std::ops::Deref;


use piet_common::RenderContext;
use crate::{ComponentInstance, RenderNode, RenderNodePtr, RenderNodePtrList, RenderTreeContext, InstantiationArgs, HandlerRegistry};
use pax_runtime_api::{Layer, log, PropertyInstance, PropertyLiteral, Size2D, Transform2D};
use pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};

/// A special "control-flow" primitive associated with the `for` statement.
/// Repeat allows for nodes to be rendered dynamically per data specified in `source_expression`.
/// That is: for a `source_expression` of length `n`, `Repeat` will render its
/// template `n` times, each with an embedded component context (`RepeatItem`)
/// with an index `i` and a pointer to that relevant datum `source_expression[i]`
pub struct RepeatInstance<R: 'static + RenderContext> {
    pub instance_id: u64,
    pub repeated_template: RenderNodePtrList<R>,
    pub transform: Rc<RefCell<dyn PropertyInstance<Transform2D>>>,
    pub source_expression_vec: Option<Box<dyn PropertyInstance<Vec<Rc<PropertiesCoproduct>>>>>,
    pub source_expression_range: Option<Box<dyn PropertyInstance<std::ops::Range<isize>>>>,
    pub active_children: RenderNodePtrList<R>,
    pub next_frame_children: Option<RenderNodePtrList<R>>,
    /// Used for hacked dirty-checking, in the absence of our centralized dirty-checker
    cached_old_value_vec: Option<Vec<Rc<PropertiesCoproduct>>>,
    cached_old_value_range: Option<std::ops::Range<isize>>,
    cached_old_bounds: (f64, f64),
}

impl<R: 'static + RenderContext> RenderNode<R> for RepeatInstance<R> {

    fn get_instance_id(&self) -> u64 {
        self.instance_id
    }

    fn instantiate(mut args: InstantiationArgs<R>) -> Rc<RefCell<Self>> where Self: Sized {

        let mut instance_registry = (*args.instance_registry).borrow_mut();
        let instance_id  = instance_registry.mint_id();
        let ret = Rc::new(RefCell::new(RepeatInstance {
            instance_id,
            repeated_template: match args.children {
                None => {Rc::new(RefCell::new(vec![]))}
                Some(children) => children
            },
            transform: args.transform,
            source_expression_vec: args.repeat_source_expression_vec,
            source_expression_range: args.repeat_source_expression_range,
            active_children: Rc::new(RefCell::new(vec![])),
            next_frame_children: None,
            cached_old_value_vec: None,
            cached_old_value_range: None,
            cached_old_bounds: (0.0, 0.0),
        }));

        instance_registry.register(instance_id, Rc::clone(&ret) as RenderNodePtr<R>);
        ret
    }

    fn compute_properties(&mut self, rtc: &mut RenderTreeContext<R>) {



        if self.next_frame_children.is_some() {
            self.active_children = self.next_frame_children.take().unwrap();
            self.next_frame_children = None;
        }

        let (is_dirty, normalized_vec_of_props) = if let Some(se) = &self.source_expression_vec {
            //Handle case where the source expression is a Vec<Property<T>>,
            // like `for elem in self.data_list`
            let new_value = if let Some(tc) = rtc.compute_vtable_value(se._get_vtable_id().clone()) {
                if let TypesCoproduct::stdCOCOvecCOCOVecLABRstdCOCOrcCOCORcLABRPropertiesCoproductRABRRABR(vec) = tc { vec } else { unreachable!() }
            } else {
                se.get().clone()
            };

            //let is_dirty = true;
            //Major hack: will only consider a new vec dirty if its cardinality changes.
            let is_dirty = {
                rtc.bounds != self.cached_old_bounds ||
                if self.cached_old_value_vec.is_none() {
                    true
                } else {
                    self.cached_old_value_vec.as_ref().unwrap().len() != new_value.len()
                }
            };
            self.cached_old_bounds = rtc.bounds.clone();
            self.cached_old_value_vec = Some(new_value.clone());
            (is_dirty, new_value)
        } else if let Some(se) = &self.source_expression_range {
            //Handle case where the source expression is a Range,
            // like `for i in 0..5`
            let new_value = if let Some(tc) = rtc.compute_vtable_value(se._get_vtable_id().clone()) {
                if let TypesCoproduct::stdCOCOopsCOCORangeLABRisizeRABR(vec) = tc { vec } else { unreachable!() }
            } else { unreachable!() };

            //let is_dirty = true;
            //Major hack: will only consider a new vec dirty if its cardinality changes.
            let is_dirty = {
                rtc.bounds != self.cached_old_bounds ||
                if self.cached_old_value_range.is_none() {
                    true
                } else {
                    self.cached_old_value_range.as_ref().unwrap().len() != new_value.len()
                }
            };
            self.cached_old_bounds = rtc.bounds.clone();
            self.cached_old_value_range = Some(new_value.clone());
            let normalized_vec_of_props = new_value.into_iter().enumerate().map(|(i, elem)|{Rc::new(PropertiesCoproduct::isize(elem))}).collect();
            (is_dirty, normalized_vec_of_props)
        } else {unreachable!()};

        if is_dirty {
            //Any stated children (repeat template members) of Repeat should be forwarded to the `RepeatItem`-wrapped `ComponentInstance`s
            //so that `Slot` works as expected
            let forwarded_children = match (*rtc.runtime).borrow_mut().peek_stack_frame() {
                Some(frame) => {Rc::clone(&(*frame.borrow()).get_unflattened_adoptees())},
                None => {Rc::new(RefCell::new(vec![]))},
            };

            let mut instance_registry = (*rtc.engine.instance_registry).borrow_mut();

            (*self.active_children).borrow_mut().iter().for_each(|child| {
                let instance_id = (*(*child)).borrow_mut().get_instance_id();
                instance_registry.deregister(instance_id);
                instance_registry.mark_for_unmount(instance_id);
            });

            //reset children:
            //wrap source_expression into `RepeatItems`, which attach
            //the necessary data as stack frame context
            self.next_frame_children = Some(Rc::new(RefCell::new(
                normalized_vec_of_props.iter().enumerate().map(|(i, datum)| {
                    let instance_id = instance_registry.mint_id();

                    let render_node : RenderNodePtr<R> = Rc::new(RefCell::new(
                        ComponentInstance {
                            instance_id,
                            children: Rc::clone(&forwarded_children),
                            template: Rc::clone(&self.repeated_template),
                            transform: Rc::new(RefCell::new(PropertyLiteral::new(Transform2D::default()))),
                            properties: Rc::new(RefCell::new(PropertiesCoproduct::RepeatItem(Rc::clone(datum), i))),
                            timeline: None,
                            handler_registry: None,
                            compute_properties_fn: Box::new(|props, rtc|{
                                //no-op since the Repeat RenderNode handles the necessary calc (see `RepeatInstance::compute_properties`)
                            }),

                        }
                    ));

                    instance_registry.register(instance_id, Rc::clone(&render_node));
                    instance_registry.mark_mounted(rtc.get_id_chain(instance_id));

                    // (&*render_node).borrow_mut().mount_recursive(rtc);

                    render_node
                }).collect()
            )));

        }



        // pax_runtime_api::log(&format!("finished computing repeat properties, virt len: {}", (*self.virtual_children).borrow().len()));
    }


    fn should_flatten(&self) -> bool {
        true
    }
    fn get_rendering_children(&self) -> RenderNodePtrList<R> {
        Rc::clone(&self.active_children)
    }
    fn get_size(&self) -> Option<Size2D> { None }
    fn compute_size_within_bounds(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }
    fn get_transform(&mut self) -> Rc<RefCell<dyn PropertyInstance<Transform2D>>> { Rc::clone(&self.transform) }
    fn get_layer_type(&mut self) -> Layer {
        Layer::DontCare
    }

    fn handle_did_mount(&mut self, _rtc: &mut RenderTreeContext<R>, z_index: u32) {
        self.cached_old_value_range = None;
        self.cached_old_value_vec = None;
    }

}


/*
lab journal, zb
---------------

To support polymorphic data <T> inside stack frames,
we need a `dyn SomeTrait` contract that stack frame data
can adhere to (instead of arbitrary `T`)

ex. `repeat` element stackframe data:
{
    index: usize,
    datum: T
}

We could have any stack frame abide by this contract:

StackFrameData<T> {
    get_index() -> usize;
    get_datum() -> Box<dyn T>;
}
...but how does the consumer know it's dealing with `T`?  Where does `T` come from?

Ultimately, it's userland.  E.g. consider the user-provided data:
cats = [{fur_color: Colors.WHITE, eye_color: Colors.BLUE}, {fur_color: Colors.BROWN, eye_color: Colors.GREEN}]
describes a schema and thus `T` of {fur_color: Color, eye_color: Color}

Perhaps this gets easier if we introduce our `scope` object here, and deal with a string:value (dynamic) lookup?

This turns our StackFrameData approach into:

StackFrame {
    get_scope() -> Scope;
}

along with

Scope {
    get_type(key: &str) -> PolymorphicType // used for unsafe unboxing of value
    get_value(key: &str) -> PolymorphicValue
}

When working with a Scope inside a `repeat`, the user looks up values & types by (string) key.

Seems like a suitable solution.

 */


//Can we operate on a guarantee that for `n` elements in a repeat, the consumer (expression)
//will be invoked exactly `n` times?  If so, we could push a stackframe for each datum (in reverse)
//so that each invocation consumes a new stack frame, in order.  The tricky piece of this is
//a need to introduce stack frame `pop`s somewhere before the did_eval_properties_in_place lifecycle
//method, in a way that's unique to `repeat`.

//An alternative approach to this problem, which operates with the grain of "one stack frame
//per component instance," is to add an iterator to a new RepeatPropertiesContainer, which
//yields the next `RepeatProperties` on each invocation.  This may require simply modifying
//the inject_and_evaluate logic.  Perhaps we can introduce a `.next` method on Evaluator, with
//a default implementation that's a no-op, but which Repeat can override to step through
//an iterator.

// rtc.runtime.borrow_mut().push_stack_frame(
//     Rc::clone(&self.children),
//       Box::new(Scope {
//           properties: Rc::clone(&self.properties) as Rc<dyn Any>
//       })
// );
