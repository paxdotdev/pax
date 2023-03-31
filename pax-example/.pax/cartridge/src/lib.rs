
//Prelude: Rust
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::ops::Deref;
use std::rc::Rc;
//Prelude: Pax
use pax_runtime_api::numeric::Numeric;
use pax_runtime_api::{ArgsCoproduct, Size, SizePixels, PropertyInstance, PropertyLiteral, Size2D, Transform2D};
use pax_core::{ComponentInstance, RenderNodePtr, PropertyExpression, RenderNodePtrList, RenderTreeContext, ExpressionContext, PaxEngine, RenderNode, InstanceRegistry, HandlerRegistry, InstantiationArgs, ConditionalInstance, SlotInstance, StackFrame};
use pax_core::pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};
use pax_core::repeat::{RepeatInstance};
use piet_common::RenderContext;

// generate imports, pointing to userland cartridge `pub mod pax_reexports`

use pax_example::pax_reexports::f64;

use pax_example::pax_reexports::pax_std::types::Color;

use pax_example::pax_reexports::pax_std::types::Font;

use pax_example::pax_reexports::pax_std::types::PathSegment;

use pax_example::pax_reexports::pax_std::types::Stroke;

use pax_example::pax_reexports::std::string::String;

use pax_example::pax_reexports::std::vec::Vec;

use pax_example::pax_reexports::HelloRGB;

use pax_example::pax_reexports::pax_std::primitives::Ellipse;

use pax_example::pax_reexports::pax_std::primitives::Path;

use pax_example::pax_reexports::pax_std::primitives::Rectangle;

use pax_example::pax_reexports::pax_std::primitives::Text;


//pull in entire const token stream here e.g. `const JABBERWOCKY : &str = r#"’Twas brillig, and the slithy toves `...


pub fn instantiate_expression_table<R: 'static + RenderContext>() -> HashMap<usize, Box<dyn Fn(ExpressionContext<R>) -> TypesCoproduct>> {
    let mut vtable: HashMap<usize, Box<dyn Fn(ExpressionContext<R>) -> TypesCoproduct>> = HashMap::new();

    
    //Color::rgb(0.5,0,1)
    vtable.insert(0, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        #[allow(unused_parens)]
        TypesCoproduct::__pax_stdCOCOtypesCOCOColor(
            Color::rgb((Numeric::from(0.5)),(Numeric::from(0)),(Numeric::from(1)),)
        )
    }));
    
    //Transform2D::align(50%,50%)*Transform2D::anchor(50%,50%)*Transform2D::rotate(rotation)
    vtable.insert(1, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        
            let rotation = {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().nth_ancestor(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();

                if let PropertiesCoproduct::HelloRGB(p) = properties {
                    
                    *p.rotation.get()
                    
                } else {
                    unreachable!("1")
                }
            };
        

        #[allow(unused_parens)]
        TypesCoproduct::Transform2D(
            ((Transform2D::align((Size::Percent(50.into())),(Size::Percent(50.into())),)*Transform2D::anchor((Size::Percent(50.into())),(Size::Percent(50.into())),))*Transform2D::rotate(((rotation).into()),))
        )
    }));
    
    //Color::rgb(1,0.8,0.1)
    vtable.insert(2, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        #[allow(unused_parens)]
        TypesCoproduct::__pax_stdCOCOtypesCOCOColor(
            Color::rgb((Numeric::from(1)),(Numeric::from(0.8)),(Numeric::from(0.1)),)
        )
    }));
    
    //Transform2D::align(100%,0%)*Transform2D::anchor(100%,0%)
    vtable.insert(3, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        #[allow(unused_parens)]
        TypesCoproduct::Transform2D(
            (Transform2D::align((Size::Percent(100.into())),(Size::Percent(0 .into())),)*Transform2D::anchor((Size::Percent(100.into())),(Size::Percent(0 .into())),))
        )
    }));
    
    //Color::rgb(0.25,0.5,0.5)
    vtable.insert(4, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        #[allow(unused_parens)]
        TypesCoproduct::__pax_stdCOCOtypesCOCOColor(
            Color::rgb((Numeric::from(0.25)),(Numeric::from(0.5)),(Numeric::from(0.5)),)
        )
    }));
    

    vtable
}

//Begin component factory literals

    
pub fn instantiate_root_component<R: 'static + RenderContext>(instance_registry: Rc<RefCell<InstanceRegistry<R>>>) -> Rc<RefCell<ComponentInstance<R>>> {
    ComponentInstance::instantiate( InstantiationArgs{
        properties: PropertiesCoproduct::HelloRGB( HelloRGB::default() ),
        handler_registry:  Some(Rc::new(RefCell::new(
                                                     HandlerRegistry {
                                                         click_handlers: vec![
                                                                    |stack_frame, args|{
                                                                        let properties = ((*stack_frame).borrow().get_properties());
                                                                        let properties = &mut *properties.as_ref().borrow_mut();
                                                                        let properties = if let PropertiesCoproduct::HelloRGB(p) = properties {p} else {unreachable!()};
                                                                        HelloRGB::handle_global_click(properties,args);
                                                                    },
                                                                ],
                                                         will_render_handlers: vec![],
                                                         scroll_handlers: vec![
                                                                     |stack_frame, args|{
                                                                         let properties = ((*stack_frame).borrow().get_properties());
                                                                         let properties = &mut *properties.as_ref().borrow_mut();
                                                                         let properties = if let PropertiesCoproduct::HelloRGB(p) = properties {p} else {unreachable!()};
                                                                         HelloRGB::handle_global_scroll(properties,args);
                                                                     },
                                                                 ],
                                                     }
                                                 ))),
        instance_registry: Rc::clone(&instance_registry),
        transform: Transform2D::default_wrapped(),
        size: None,
        children: None,
        component_template: Some(Rc::new(RefCell::new(vec![

pax_std_primitives::ellipse::EllipseInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::Ellipse( Ellipse {
        
            stroke: Box::new( PropertyLiteral::new(Default::default()) ),
        
            fill: Box::new( PropertyExpression::new(0) ),
        
    }),
    handler_registry:  Some(Rc::new(RefCell::new(
                                                 HandlerRegistry {
                                                     click_handlers: vec![|stack_frame, args|{
                                                                    let properties = (*stack_frame).borrow().get_properties();
                                                                    let properties = &mut *properties.as_ref().borrow_mut();
                                                                    let properties = if let PropertiesCoproduct::HelloRGB(p) = properties {p} else {unreachable!()};
                                                                    HelloRGB::handle_click(properties,args);
                                                                },],
                                                     will_render_handlers: vec![],
                                                     scroll_handlers: vec![|stack_frame, args|{
                                                                     let properties = (*stack_frame).borrow().get_properties();
                                                                     let properties = &mut *properties.as_ref().borrow_mut();
                                                                     let properties = if let PropertiesCoproduct::HelloRGB(p) = properties {p} else {unreachable!()};
                                                                     HelloRGB::handle_scroll(properties,args);
                                                                 },],
                                                 }
                                             ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyExpression::new(1))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Percent(33.33.into()))),Box::new(PropertyLiteral::new(Size::Percent(100.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})

,

pax_std_primitives::rectangle::RectangleInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::Rectangle( Rectangle {
        
            stroke: Box::new( PropertyLiteral::new(Default::default()) ),
        
            fill: Box::new( PropertyExpression::new(2) ),
        
    }),
    handler_registry:  Some(Rc::new(RefCell::new(
                                                 HandlerRegistry {
                                                     click_handlers: vec![],
                                                     will_render_handlers: vec![],
                                                     scroll_handlers: vec![],
                                                 }
                                             ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyExpression::new(3))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Percent(33.33.into()))),Box::new(PropertyLiteral::new(Size::Percent(100.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})

,

pax_std_primitives::text::TextInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::Text( Text {
        
            text: Box::new( PropertyLiteral::new("Hello world".try_into().unwrap()) ),
        
            font: Box::new( PropertyLiteral::new(Default::default()) ),
        
            fill: Box::new( PropertyLiteral::new(Default::default()) ),
        
    }),
    handler_registry:  Some(Rc::new(RefCell::new(
                                                 HandlerRegistry {
                                                     click_handlers: vec![],
                                                     will_render_handlers: vec![],
                                                     scroll_handlers: vec![],
                                                 }
                                             ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Default::default())),Box::new(PropertyLiteral::new(Default::default()))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})

,

pax_std_primitives::path::PathInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::Path( Path {
        
            segments: Box::new( PropertyLiteral::new(Default::default()) ),
        
            stroke: Box::new( PropertyLiteral::new(Default::default()) ),
        
            fill: Box::new( PropertyLiteral::new(Default::default()) ),
        
    }),
    handler_registry:  Some(Rc::new(RefCell::new(
                                                 HandlerRegistry {
                                                     click_handlers: vec![],
                                                     will_render_handlers: vec![],
                                                     scroll_handlers: vec![],
                                                 }
                                             ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Default::default())),Box::new(PropertyLiteral::new(Default::default()))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})

,

pax_std_primitives::rectangle::RectangleInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::Rectangle( Rectangle {
        
            stroke: Box::new( PropertyLiteral::new(Default::default()) ),
        
            fill: Box::new( PropertyExpression::new(4) ),
        
    }),
    handler_registry:  Some(Rc::new(RefCell::new(
                                                 HandlerRegistry {
                                                     click_handlers: vec![],
                                                     will_render_handlers: vec![],
                                                     scroll_handlers: vec![],
                                                 }
                                             ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Percent(100.into()))),Box::new(PropertyLiteral::new(Size::Percent(100.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})

]))),
        scroller_args: None,
        slot_index: None,
        repeat_source_expression: None,
        conditional_boolean_expression: None,
        compute_properties_fn: Some(Box::new(|properties, rtc|{
            let properties = &mut *properties.as_ref().borrow_mut();
            let properties = if let PropertiesCoproduct::HelloRGB(p) = properties {p} else {unreachable!()};

            
            if let Some(new_value) = rtc.compute_eased_value(properties.rotation._get_transition_manager()) {
            properties.rotation.set(new_value);
            } else if let Some(new_value) = rtc.compute_vtable_value(properties.rotation._get_vtable_id()) {
            let new_value = if let TypesCoproduct::__f64(v) = new_value { v } else { unreachable!() };
            properties.rotation.set(new_value);
            }
            
        })),
    })
}





