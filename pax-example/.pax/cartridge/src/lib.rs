
//Prelude: Rust
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::ops::Deref;
use std::rc::Rc;
//Prelude: Pax
use pax_runtime_api::{ArgsCoproduct, SizePixels, PropertyInstance, PropertyLiteral, Size2D, Transform2D};
use pax_core::{ComponentInstance, RenderNodePtr, PropertyExpression, RenderNodePtrList, RenderTreeContext, ExpressionContext, PaxEngine, RenderNode, InstanceRegistry, HandlerRegistry, InstantiationArgs, ConditionalInstance, SlotInstance, StackFrame};
use pax_core::pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};
use pax_core::repeat::{RepeatInstance};
use piet_common::RenderContext;

// generate imports, pointing to userland cartridge `pub mod pax_reexports`

use pax_example::pax_reexports::std::vec::Vec;

use pax_example::pax_reexports::f64;

use pax_example::pax_reexports::pax::api::Size;

use pax_example::pax_reexports::pax_std::types::StackerDirection;

use pax_example::pax_reexports::std::string::String;

use pax_example::pax_reexports::pax_std::types::Color;

use pax_example::pax_reexports::pax_std::types::Font;

use pax_example::pax_reexports::pax_std::types::Stroke;

use pax_example::pax_reexports::pax_std::types::StackerCell;

use pax_example::pax_reexports::usize;

use pax_example::pax_reexports::i64;


//dependency paths below come from pax_primitive macro, where these crate+module paths are passed as parameters:
//e.g.: `use pax_std_primitives::{RectangleInstance, GroupInstance, ScrollerInstance, FrameInstance, TextInstance};`



//pull entire const token stream in here e.g. `const JABBERWOCKY : &str = r#"’Twas brillig, and the slithy toves `...


pub fn instantiate_expression_table<R: 'static + RenderContext>() -> HashMap<u64, Box<dyn Fn(ExpressionContext<R>) -> TypesCoproduct>> {
    let mut vtable: HashMap<u32, Box<dyn Fn(ExpressionContext<R>) -> TypesCoproduct>> = HashMap::new();

    /* Repeat example:
        // {Color::rgba(100%, (100 - (i * 12.5))%, (i * 12.5)%, 100%)}
        vtable.insert(9, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        let (datum, i) = if let PropertiesCoproduct::RepeatItem(datum, i) = &*(*(*ec.stack_frame).borrow().get_properties()).borrow() {
        (Rc::clone(datum), *i)
        } else { unreachable!(9) };

        return TypesCoproduct::Color(
        Color::rgba(1.0, 1.0 - (i as f64 * 0.125), i as f64 * 0.125, 1.0)
        );
        })); */

    /* Offset example:
        const STACK_FRAME_OFFSET : isize = 2;
        let SCOPED_STACK_FRAME = (*ec.stack_frame).borrow().nth_descendant(STACK_FRAME_OFFSET); //just gen `ec.stack_frame` if offset == 0

        let properties = SCOPED_STACK_FRAME.deref().borrow().get_properties();
        let properties = &*(*properties).borrow();

        let current_rotation = if let PropertiesCoproduct::HelloWorld(p) = properties {
        *p.current_rotation.get() as f64
        } else { unreachable!(5) };

        TypesCoproduct::Transform2D(
        Transform2D::anchor(Size::Percent(50.0), Size::Percent(50.0))
        * Transform2D::align(Size::Percent(50.0), Size::Percent(50.0))
        * Transform2D::rotate(current_rotation)
        )
    */
    
    //rotate(self.current_rotation) 
    vtable.insert(0, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        
            let current_rotation = {
                let properties = (*ec.stack_frame).borrow().nth_descendant(0);
                let properties = &*(*properties).borrow();

                if let PropertiesCoproduct::pax_example::pax_reexports::f64(p) = properties {
                    *p.current_rotation.get()
                } else {
                    unreachable!(0)
                }
            };
        

        TypesCoproduct::Transform2D(
            rotate((current_rotation),)
        )
    }));
    
    //rgb(100 %, 100 %, 0) 
    vtable.insert(1, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        TypesCoproduct::__pax_stdCOCOtypesCOCOColor(
            rgb((Size::Percent(100)),(Size::Percent(100)),(0),)
        )
    }));
    

    vtable
}

//Begin component factory literals

