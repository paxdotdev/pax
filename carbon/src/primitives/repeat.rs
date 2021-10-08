use std::cell::RefCell;
use std::rc::Rc;

use piet_web::WebRenderContext;

use crate::{Affine, PropertyTreeContext, RenderNode, RenderNodePtr, RenderNodePtrList, RenderTreeContext, Size, Scope, PolymorphicType, StackFrame, Component, wrap_render_node_ptr_into_list, InjectionContext, Evaluator, PropertySet};
use std::collections::HashMap;

pub struct Repeat<D> {
    pub children: RenderNodePtrList,
    pub list: Vec<Rc<D>>,
    pub id: String,
    pub transform: Affine,
    virtual_children: RenderNodePtrList,
}

/// Data structure for the virtually duplicated container that surrounds repeated nodes.
/// This is attached to a Component<RepeatFrame> that `Repeat` adds to its children dynamically
/// during property-tree traversal
struct RepeatProperties<D> {
    pub i: usize,
    pub datum: Rc<D>,
    pub id: String,
}

impl<D> PropertySet for RepeatProperties<D> {}

impl<D> Repeat<D> {
    pub fn new(list: Vec<Rc<D>>, children: RenderNodePtrList, id: String, transform: Affine) -> Self {
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
                align: (0.0, 0.0),
                origin: (Size::Pixel(0.0), Size::Pixel(0.0)),
                transform: Affine::default(),
                properties,
            })));
        }
    }

    fn get_align(&self) -> (f64, f64) {
        (0.0, 0.0)
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
    fn get_origin(&self) -> (Size<f64>, Size<f64>) {
        (Size::Pixel(0.0), Size::Pixel(0.0))
    }
    fn get_transform(&self) -> &Affine {
        &self.transform
    }
    fn pre_render(&mut self, _rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {}
    fn render(&self, _rtc: &mut RenderTreeContext, _rc: &mut WebRenderContext) {}
    fn post_render(&self, _rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {}
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