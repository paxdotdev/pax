use std::cell::RefCell;
use std::rc::Rc;

use piet_web::WebRenderContext;

use crate::{Affine, PropertyTreeContext, RenderNode, RenderNodePtr, RenderNodePtrList, RenderTreeContext, Size, Scope, PolymorphicType, StackFrame};
use std::collections::HashMap;

pub struct Repeat<T> {
    pub children: Rc<RefCell<Vec<RenderNodePtr>>>,
    pub list: Vec<T>,
    pub id: String,
    pub transform: Affine,
}


struct SampleDatum {
   user_avatar_url: String,
   user_id: usize,
}

pub trait RepeatableDatum {
}

impl RepeatableDatum for SampleDatum {}

impl<D> RenderNode for Repeat<D> {
    fn eval_properties_in_place(&mut self, _: &PropertyTreeContext) {
        //TODO: handle each of Repeat's `Expressable` properties
        //

        // TODO:
        //  - add internal component store for virtual nodes, making this node their owner
        //  - add generics throughout Component to represent its data model
        //  - add a Component<RepeatFrame<D>> to the above store for each datum in array
        //  - return
        //

        let frame = RepeatFrame { datum: d, i: i, id: format!("repeat_frame_{}", i) };


        self.list.iter().enumerate().map(|i, datum|{
            let mut adoptees = ();
            //keys:  i, datum
            let mut types = HashMap::new();
            types.insert("i".to_string(), PolymorphicType::Integer);
            let sample_datum = SampleDatum{
                user_avatar_url: "https://catpix.rs/144395".to_string(),
                user_id: i * 1000 + i,
            };
            types.insert("datum".to_string(), PolymorphicType::Object(sample_datum)); //TODO:  what to do here?
            let values = HashMap::new();
            values.insert("i".to_string(), unimplemented!());
            values.insert("datum".to_string(), unimplemented!());
            let scope = Scope {
                types,
                values,
            };
            let new_stack_frame = StackFrame::new(adoptees, scope);
            new_children.push()
        });
        //
        // self.children = Rc::new(RefCell::new(
        //     self.list.iter().enumerate().map(|(i, datum)|{
        //
        //         // 1. construct a `puppeteer` node,
        //         //     - pass it the scope data (i, datum)
        //         // 2. Attach a copy of each child of this `repeat` node
        //         //     as a child of `puppeteer`
        //         // 3. write logic in `puppeteer` that delegates rendering to its contained nodes
        //         // 4. evaluate if we need to support any flattening fanciness around here
        //
        //
        //         let children_borrowed = self.children.borrow();
        //
        //
        //     }).collect()
        // ))
    }

    fn get_align(&self) -> (f64, f64) {
        (0.0, 0.0)
    }
    fn should_flatten(&self) -> bool {
        true
    }
    fn get_children(&self) -> RenderNodePtrList {
        Rc::clone(&self.children)
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

/// Similar in concept to a `Frame` but unrelated to rendering, `ScopedFrame` pushes
/// a frame onto the runtime stack containing the specified `scope`.
///
/// This is useful, for example, in the `Repeat` component definition, where each
/// repeated node needs a unique scope available containing the active index & datum.
struct RepeatFrame<D> {
    pub i: usize,
    pub datum: D,
    pub id: String,
}
//
// impl RenderNode for RepeatFrame {
//     fn eval_properties_in_place(&mut self, ptc: &PropertyTreeContext) {
//         //TODO: handle each of ScopeFrame's `Expressable` properties
//
//
//         ptc.runtime.borrow_mut().push_stack_frame(
//             Rc::clone(&self.children),
//             //TODO:  cloning this data is a potentially heavy operation.  May be worth
//             //       revisiting design here, e.g. to pass a smart/pointer of `Scope` to the stack
//             self.scope.clone(),
//         );
//
//     }
//
//     fn get_align(&self) -> (f64, f64) {
//         (0.0, 0.0)
//     }
//     fn should_flatten(&self) -> bool {
//         true
//     }
//     fn get_children(&self) -> RenderNodePtrList {
//         Rc::clone(&self.children)
//     }
//     fn get_size(&self) -> Option<(Size<f64>, Size<f64>)> { None }
//     fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }
//     fn get_id(&self) -> &str {
//         &self.id.as_str()
//     }
//     fn get_origin(&self) -> (Size<f64>, Size<f64>) {
//         (Size::Pixel(0.0), Size::Pixel(0.0))
//     }
//     fn get_transform(&self) -> &Affine {
//         &self.transform
//     }
//     fn pre_render(&mut self, _rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {}
//     fn render(&self, _rtc: &mut RenderTreeContext, _rc: &mut WebRenderContext) {}
//     fn post_render(&self, _rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {}
// }


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