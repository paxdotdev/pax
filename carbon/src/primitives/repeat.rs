use std::cell::RefCell;
use std::rc::Rc;

use piet_web::WebRenderContext;

use crate::{Affine, PropertyTreeContext, RenderNode, RenderNodePtr, RenderNodePtrList, RenderTreeContext, Size, Scope, StackFrame, Component, wrap_render_node_ptr_into_list, InjectionContext, Evaluator, Transform, PropertiesCoproduct, RepeatItem, Property, PropertyLiteral};
use std::collections::HashMap;

pub struct Repeat {
    pub children: RenderNodePtrList,
    pub data_list: Box<Property<Vec<Rc<PropertiesCoproduct>>>>,
    pub transform: Transform,

    //TODO: any way to make this legit-private along with the ..Default::default() syntax?
    pub _virtual_children: RenderNodePtrList,
}

/// Data structure for the virtually duplicated container that surrounds repeated nodes.
/// This is attached to a Component<RepeatFrame> that `Repeat` adds to its children dynamically
/// during property-tree traversal
pub struct RepeatProperties {
    pub i: usize,
    pub datum: Rc<PropertiesCoproduct>,
    pub id: String,
}

impl Repeat {
}

impl Default for Repeat {
    fn default() -> Self {
        Repeat {
            children: Rc::new(RefCell::new(vec![])),
            data_list: Box::new(PropertyLiteral {value: vec![]}),
            transform: Default::default(),
            _virtual_children: Rc::new(RefCell::new(vec![]))
        }
    }
}

impl RenderNode for Repeat {
    fn eval_properties_in_place(&mut self, ptc: &PropertyTreeContext) {
        //TODO: handle each of Repeat's `Expressable` properties

        //reset children
        self._virtual_children = Rc::new(RefCell::new(Vec::new()));

        //for each element in self.list, create a new child (Component) and push it to self.children
        for (i, datum) in self.data_list.read().iter().enumerate() {
            let properties = Rc::new(RefCell::new(RepeatItem { i, repeat_properties: Rc::clone(datum)}));

            self._virtual_children.borrow_mut().push(Rc::new(RefCell::new(Component {
                template: Rc::clone(&self.children),
                transform: Transform::default(),
                properties: Rc::new(RefCell::new(PropertiesCoproduct::RepeatItem(properties))),
            })));
        }

    }

    fn post_eval_properties_in_place(&mut self, ptc: &PropertyTreeContext) {
        //clean up the stack frame for the next component
        ptc.runtime.borrow_mut().pop_stack_frame();
    }

    fn should_flatten(&self) -> bool {
        true
    }
    fn get_children(&self) -> RenderNodePtrList {
        Rc::clone(&self._virtual_children)
    }
    fn get_size(&self) -> Option<(Size<f64>, Size<f64>)> { None }
    fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }
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