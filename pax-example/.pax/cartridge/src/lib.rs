
//Prelude: Rust
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::ops::Deref;
use std::rc::Rc;
//Prelude: Pax
use pax_runtime_api::{ArgsCoproduct, Size, SizePixels, PropertyInstance, PropertyLiteral, Size2D, Transform2D};
use pax_core::{ComponentInstance, RenderNodePtr, PropertyExpression, RenderNodePtrList, RenderTreeContext, ExpressionContext, PaxEngine, RenderNode, InstanceRegistry, HandlerRegistry, InstantiationArgs, ConditionalInstance, SlotInstance, StackFrame};
use pax_core::pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};
use pax_core::repeat::{RepeatInstance};
use piet_common::RenderContext;

// generate imports, pointing to userland cartridge `pub mod pax_reexports`

use pax_example::pax_reexports::pax_std::types::Color;

use pax_example::pax_reexports::pax_std::types::Stroke;


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
    
    //Transform2D :: translate(100px, 100px) * Transform2D :: rotate(2.25)
    vtable.insert(3, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        TypesCoproduct::Transform2D(
            (Transform2D::translate((Size::Pixel(100)),(Size::Pixel(100)),)*Transform2D::rotate((2.25),))
        )
    }));
    
    //Color :: rgb(100 %, 0, 0) 
    vtable.insert(0, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        TypesCoproduct::__pax_stdCOCOtypesCOCOColor(
            Color::rgb((Size::Percent(100)),(0),(0),)
        )
    }));
    
    //Transform2D :: translate(0, 0) * Transform2D :: rotate(1.25)
    vtable.insert(1, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        TypesCoproduct::Transform2D(
            (Transform2D::translate((0),(0),)*Transform2D::rotate((1.25),))
        )
    }));
    
    //Color :: rgb(0, 0, 100 %) 
    vtable.insert(4, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        TypesCoproduct::__pax_stdCOCOtypesCOCOColor(
            Color::rgb((0),(0),(Size::Percent(100)),)
        )
    }));
    
    //Color :: rgb(0, 100 %, 0) 
    vtable.insert(2, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        TypesCoproduct::__pax_stdCOCOtypesCOCOColor(
            Color::rgb((0),(Size::Percent(100)),(0),)
        )
    }));
    
    //Transform2D :: translate(200px, 250px) * Transform2D :: rotate(3.25)
    vtable.insert(5, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        TypesCoproduct::Transform2D(
            (Transform2D::translate((Size::Pixel(200)),(Size::Pixel(250)),)*Transform2D::rotate((3.25),))
        )
    }));
    

    vtable
}

//Begin component factory literals

