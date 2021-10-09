use std::cell::RefCell;
use std::rc::Rc;

use piet_web::WebRenderContext;

use crate::{Affine, PropertyTreeContext, RenderNode, RenderNodePtr, RenderNodePtrList, RenderTreeContext, Size, Scope, PolymorphicType, StackFrame, Component, wrap_render_node_ptr_into_list, InjectionContext, Evaluator, Transform};
use std::collections::HashMap;

pub struct Repeat<D> {
    pub children: RenderNodePtrList,
    pub list: Vec<Rc<D>>,
    pub id: String,
    pub transform: Transform,
    virtual_children: RenderNodePtrList,
}

/// Data structure for the virtually duplicated container that surrounds repeated nodes.
/// This is attached to a Component<RepeatFrame> that `Repeat` adds to its children dynamically
/// during property-tree traversal
pub struct RepeatProperties<D> {
    pub i: usize,
    pub datum: Rc<D>,
    pub id: String,
}

impl<D> Repeat<D> {
    pub fn new(list: Vec<Rc<D>>, children: RenderNodePtrList, id: String, transform: Transform) -> Self {
        Repeat {
            list,
            children,
            id,
            transform,
            virtual_children:  Rc::new(RefCell::new(vec![])),

        }
    }
}

impl<D: 'static> RenderNode for Repeat<D> {
    fn eval_properties_in_place(&mut self, ptc: &PropertyTreeContext) {
        //TODO: handle each of Repeat's `Expressable` properties

        //reset children
        self.virtual_children = Rc::new(RefCell::new(Vec::new()));

        //for each element in self.list, create a new child (Component) and push it to self.children
        for (i, datum) in self.list.iter().enumerate() {
            let properties = RepeatProperties { datum: Rc::clone(&datum), i, id: format!("repeat_frame_{}", i) };

            self.virtual_children.borrow_mut().push(Rc::new(RefCell::new(Component {
                template: Rc::clone(&self.children),
                id: "".to_string(),
                transform: Transform::default(),
                properties: Rc::new(properties),
            })));
        }

        //We need to get a unique `RepeatProperties` instance to each evaluated Expression.
        //One way to do this is to push a unique stack frame for each repeated datum, with
        //a necessary cleanup step in post_eval_properties_in_place, which knows how to "slurp up"
        //any

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

        // ptc.runtime.borrow_mut().push_stack_frame(
        //     Rc::clone(&self.children),
        //       Box::new(Scope {
        //           properties: Rc::clone(&self.properties) as Rc<dyn Any>
        //       })
        // );
    }

    fn post_eval_properties_in_place(&mut self, ptc: &PropertyTreeContext) {
        //clean up the stack frame for the next component
        ptc.runtime.borrow_mut().pop_stack_frame();
    }

    fn should_flatten(&self) -> bool {
        true
    }
    fn get_children(&self) -> RenderNodePtrList {
        Rc::clone(&self.virtual_children)
    }
    fn get_size(&self) -> Option<(Size<f64>, Size<f64>)> { None }
    fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }
    fn get_id(&self) -> &str {
        &self.id.as_str()
    }
    fn get_transform_computed(&self) -> &Affine {
        &self.transform.cached_computed_transform
    }

    fn pre_render(&mut self, _rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {}
    fn render(&self, _rtc: &mut RenderTreeContext, _rc: &mut WebRenderContext) {}
    fn post_render(&self, _rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {}

    fn get_transform_mut(&mut self) -> &mut Transform {
        &mut self.transform
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