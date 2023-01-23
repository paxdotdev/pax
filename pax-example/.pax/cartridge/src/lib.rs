
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


//pull in entire const token stream here e.g. `const JABBERWOCKY : &str = r#"’Twas brillig, and the slithy toves `...


pub fn instantiate_expression_table<R: 'static + RenderContext>() -> HashMap<usize, Box<dyn Fn(ExpressionContext<R>) -> TypesCoproduct>> {
    let mut vtable: HashMap<usize, Box<dyn Fn(ExpressionContext<R>) -> TypesCoproduct>> = HashMap::new();

    
    //Color :: rgb(1, 0, 0) 
    vtable.insert(0, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        TypesCoproduct::__pax_stdCOCOtypesCOCOColor(
            Color::rgb((1.into()),(0.into()),(0.into()),)
        )
    }));
    
    //Transform2D :: translate(0, 0) * Transform2D :: rotate(1.25)
    vtable.insert(1, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        TypesCoproduct::Transform2D(
            (Transform2D::translate((0.into()),(0.into()),)*Transform2D::rotate((1.25.into()),))
        )
    }));
    
    //Color :: rgb(0, 1, 0) 
    vtable.insert(2, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        TypesCoproduct::__pax_stdCOCOtypesCOCOColor(
            Color::rgb((0.into()),(1.into()),(0.into()),)
        )
    }));
    
    //Transform2D :: translate(100, 100) * Transform2D :: rotate(2.25)
    vtable.insert(3, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        TypesCoproduct::Transform2D(
            (Transform2D::translate((100.into()),(100.into()),)*Transform2D::rotate((2.25.into()),))
        )
    }));
    
    //Color :: rgb(0, 0, 1) 
    vtable.insert(4, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        TypesCoproduct::__pax_stdCOCOtypesCOCOColor(
            Color::rgb((0.into()),(0.into()),(1.into()),)
        )
    }));
    
    //Transform2D :: translate(200, 250) * Transform2D :: rotate(3.25)
    vtable.insert(5, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        TypesCoproduct::Transform2D(
            (Transform2D::translate((200.into()),(250.into()),)*Transform2D::rotate((3.25.into()),))
        )
    }));
    

    vtable
}

//Begin component factory literals

   
pub fn instantiate_root_component<R: 'static + RenderContext>(instance_registry: Rc<RefCell<InstanceRegistry<R>>>) -> Rc<RefCell<ComponentInstance<R>>> {

    ComponentInstance::instantiate( InstantiationArgs{
        properties: PropertiesCoproduct::( <<TODO!>> {
            
        %}),
        handler_registry: None, //TODO: codegen!
        instance_registry: Rc::clone(&instance_registry),
        transform: Transform2D::default_wrapped(),
        size: None,
        children: None,
        component_template: Some(<<TODO!>>),
        scroller_args: None,
        slot_index: None,
        repeat_source_expression: None,
        conditional_boolean_expression: None,
        compute_properties_fn: Some(Box::new(|properties, rtc|{
            let properties = &mut *properties.as_ref().borrow_mut();
            let properties = if let PropertiesCoproduct::(p) = properties {p} else {unreachable!()};

            
        })),
    })
}


   
pub fn instantiate_<<TODO!>><R: 'static + RenderContext>(instance_registry: Rc<RefCell<InstanceRegistry<R>>>) -> Rc<RefCell<ComponentInstance<R>>> {

    ComponentInstance::instantiate( InstantiationArgs{
        properties: PropertiesCoproduct::( <<TODO!>> {
            
                stroke: Default::default(), //TODO! codegen specified init values
            
                fill: Default::default(), //TODO! codegen specified init values
            
        %}),
        handler_registry: None, //TODO: codegen!
        instance_registry: Rc::clone(&instance_registry),
        transform: Transform2D::default_wrapped(),
        size: None,
        children: None,
        component_template: Some(<<TODO!>>),
        scroller_args: None,
        slot_index: None,
        repeat_source_expression: None,
        conditional_boolean_expression: None,
        compute_properties_fn: Some(Box::new(|properties, rtc|{
            let properties = &mut *properties.as_ref().borrow_mut();
            let properties = if let PropertiesCoproduct::(p) = properties {p} else {unreachable!()};

            
            if let Some(new_value) = rtc.compute_eased_value(properties.stroke._get_transition_manager()) {
                properties.stroke.set(new_value);
            } else if let Some(new_value) = rtc.compute_vtable_value(properties.stroke._get_vtable_id()) {
                let new_value = if let TypesCoproduct::__pax_stdCOCOtypesCOCOStroke(v) = new_value { v } else { unreachable!() };
                properties.stroke.set(new_value);
            }
            
            if let Some(new_value) = rtc.compute_eased_value(properties.fill._get_transition_manager()) {
                properties.fill.set(new_value);
            } else if let Some(new_value) = rtc.compute_vtable_value(properties.fill._get_vtable_id()) {
                let new_value = if let TypesCoproduct::__pax_stdCOCOtypesCOCOColor(v) = new_value { v } else { unreachable!() };
                properties.fill.set(new_value);
            }
            
        })),
    })
}


   
pub fn instantiate_<<TODO!>><R: 'static + RenderContext>(instance_registry: Rc<RefCell<InstanceRegistry<R>>>) -> Rc<RefCell<ComponentInstance<R>>> {

    ComponentInstance::instantiate( InstantiationArgs{
        properties: PropertiesCoproduct::( <<TODO!>> {
            
        %}),
        handler_registry: None, //TODO: codegen!
        instance_registry: Rc::clone(&instance_registry),
        transform: Transform2D::default_wrapped(),
        size: None,
        children: None,
        component_template: Some(<<TODO!>>),
        scroller_args: None,
        slot_index: None,
        repeat_source_expression: None,
        conditional_boolean_expression: None,
        compute_properties_fn: Some(Box::new(|properties, rtc|{
            let properties = &mut *properties.as_ref().borrow_mut();
            let properties = if let PropertiesCoproduct::(p) = properties {p} else {unreachable!()};

            
        })),
    })
}


