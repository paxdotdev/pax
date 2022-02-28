use std::cell::RefCell;
use std::rc::Rc;


use crate::{HandlerRegistry, ComponentInstance, RenderNode, RenderNodePtr, RenderNodePtrList, RenderTreeContext, InstantiationArgs};
use pax_runtime_api::{Property, PropertyLiteral, Size2D, Transform};
use pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};



/// A special "control-flow" primitive, Repeat allows for nodes
/// to be rendered dynamically per data specified in `data_list`.
/// That is: for a `data_list` of length `n`, `Repeat` will render its
/// template `n` times, each with an embedded component context (`RepeatItem`)
/// with an index `i` and a pointer to that relevant datum `data_list[i]`
pub struct RepeatInstance {
    pub template: RenderNodePtrList, //TODO: private?
    pub transform: Rc<RefCell<dyn Property<Transform>>>,
    pub data_list: Box<dyn Property<Vec<Rc<PropertiesCoproduct>>>>,
    pub virtual_children: RenderNodePtrList,
}


impl RenderNode for RepeatInstance {
    fn compute_properties(&mut self, rtc: &mut RenderTreeContext) {


        if let Some(data_list) = rtc.get_computed_value(self.data_list._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Vec_Rc_PropertiesCoproduct___(v) = data_list { v } else { unreachable!() };
            self.data_list.set(new_value);
        }

        //TODO: cache and be smarter

        //reset children:
        //wrap data_list into repeat_items and attach "puppeteer" components that attach
        //the necessary data as stack frame context
        self.virtual_children = Rc::new(RefCell::new(
            self.data_list.get().iter().enumerate().map(|(i, datum)| {

                let render_node : RenderNodePtr = Rc::new(RefCell::new(
                    ComponentInstance {
                        adoptees: Rc::new(RefCell::new(vec![])),
                        template: Rc::clone(&self.template),
                        transform: Rc::new(RefCell::new(PropertyLiteral (Transform::default()))),
                        properties: Rc::new(RefCell::new(PropertiesCoproduct::RepeatItem(Rc::clone(datum), i))),
                        timeline: None,
                        handler_registry: None,
                        compute_properties_fn: Box::new(|props, rtc|{
                            //no-op since the Repeat RenderNode handles the necessary calc (see `RepeatInstnace::compute_in_place`)
                        })
                    }
                ));

                render_node
            }).collect()
        ));

    }


    fn should_flatten(&self) -> bool {
        true
    }
    fn get_rendering_children(&self) -> RenderNodePtrList {
        Rc::clone(&self.virtual_children)
    }
    fn get_size(&self) -> Option<Size2D> { None }
    fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }
    fn get_transform(&mut self) -> Rc<RefCell<dyn Property<Transform>>> { Rc::clone(&self.transform) }

    fn instantiate(args: InstantiationArgs) -> Rc<RefCell<Self>> where Self: Sized {

        let new_id = pax_runtime_api::generate_unique_id();
        let ret = Rc::new(RefCell::new(RepeatInstance {
            template: match args.children {
                None => {Rc::new(RefCell::new(vec![]))}
                Some(children) => children
            },
            transform: args.transform,
            data_list: args.data_list.unwrap(),
            virtual_children: Rc::new(RefCell::new(vec![]))
        }));

        (*args.instance_map).borrow_mut().insert(new_id, Rc::clone(&ret) as Rc<RefCell<dyn RenderNode>>);
        ret
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
//a need to introduce stack frame `pop`s somewhere before the post_eval_properties_in_place lifecycle
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
