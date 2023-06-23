
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

use pax_example::pax_reexports::core::option::Option;

use pax_example::pax_reexports::f64;

use pax_example::pax_reexports::pax_std::types::Color;

use pax_example::pax_reexports::pax_std::types::Stroke;

use pax_example::pax_reexports::pax_std::types::text::Font;

use pax_example::pax_reexports::pax_std::types::text::FontStyle;

use pax_example::pax_reexports::pax_std::types::text::FontWeight;

use pax_example::pax_reexports::pax_std::types::text::LinkStyle;

use pax_example::pax_reexports::pax_std::types::text::SizeWrapper;

use pax_example::pax_reexports::pax_std::types::text::TextAlignHorizontal;

use pax_example::pax_reexports::pax_std::types::text::TextAlignVertical;

use pax_example::pax_reexports::std::string::String;

use pax_example::pax_reexports::usize;

use pax_example::pax_reexports::Example;

use pax_example::pax_reexports::fireworks::Fireworks;

use pax_example::pax_reexports::hello_rgb::HelloRGB;

use pax_example::pax_reexports::pax_std::primitives::Ellipse;

use pax_example::pax_reexports::pax_std::primitives::Group;

use pax_example::pax_reexports::pax_std::primitives::Image;

use pax_example::pax_reexports::pax_std::primitives::Rectangle;

use pax_example::pax_reexports::pax_std::primitives::Text;

use pax_example::pax_reexports::words::Words;


//pull in entire const token stream here e.g. `const JABBERWOCKY : &str = r#"’Twas brillig, and the slithy toves `...


pub fn instantiate_expression_table<R: 'static + RenderContext>() -> HashMap<usize, Box<dyn Fn(ExpressionContext<R>) -> TypesCoproduct>> {
    let mut vtable: HashMap<usize, Box<dyn Fn(ExpressionContext<R>) -> TypesCoproduct>> = HashMap::new();

    
    //Transform2D::align(50%,50%)*Transform2D::anchor(50%,50%)*Transform2D::rotate(rotation)
    vtable.insert(0, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        
            let rotation = {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();

                
                    if let PropertiesCoproduct::HelloRGB(p) = properties {
                        
                            Numeric::from(p.rotation.get())
                        
                    } else {
                        unreachable!("0")
                    }
                
            };
        

        #[allow(unused_parens)]
        TypesCoproduct::Transform2D(
            ((Transform2D::align((Size::Percent(50.into())),(Size::Percent(50.into())),)*(Transform2D::anchor((Size::Percent(50.into())),(Size::Percent(50.into())),)).into())*(Transform2D::rotate((rotation),)).into())
        )
    }));
    
    //Color::rgb(0.4,0.5,0)
    vtable.insert(1, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        #[allow(unused_parens)]
        TypesCoproduct::__pax_stdCOCOtypesCOCOColor(
            Color::rgb((Numeric::from(0.4)),(Numeric::from(0.5)),(Numeric::from(0)),)
        )
    }));
    
    //Transform2D::anchor(0%,0%)*Transform2D::align(0%,0%)
    vtable.insert(2, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        #[allow(unused_parens)]
        TypesCoproduct::Transform2D(
            (Transform2D::anchor((Size::Percent(0 .into())),(Size::Percent(0 .into())),)*(Transform2D::align((Size::Percent(0 .into())),(Size::Percent(0 .into())),)).into())
        )
    }));
    
    //Transform2D::anchor(50%,50%)*Transform2D::align(50%,50%)
    vtable.insert(3, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        #[allow(unused_parens)]
        TypesCoproduct::Transform2D(
            (Transform2D::anchor((Size::Percent(50.into())),(Size::Percent(50.into())),)*(Transform2D::align((Size::Percent(50.into())),(Size::Percent(50.into())),)).into())
        )
    }));
    
    //Transform2D::anchor(50%,50%)*Transform2D::align(50%,50%)
    vtable.insert(4, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        #[allow(unused_parens)]
        TypesCoproduct::Transform2D(
            (Transform2D::anchor((Size::Percent(50.into())),(Size::Percent(50.into())),)*(Transform2D::align((Size::Percent(50.into())),(Size::Percent(50.into())),)).into())
        )
    }));
    
    //Color::rgba(0.0,0.0,0.0,1.0)
    vtable.insert(5, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        #[allow(unused_parens)]
        TypesCoproduct::__pax_stdCOCOtypesCOCOColor(
            Color::rgba((Numeric::from(0.0)),(Numeric::from(0.0)),(Numeric::from(0.0)),(Numeric::from(1.0)),)
        )
    }));
    
    //Transform2D::anchor(0%,0%)*Transform2D::align(0%,0%)
    vtable.insert(6, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        #[allow(unused_parens)]
        TypesCoproduct::Transform2D(
            (Transform2D::anchor((Size::Percent(0 .into())),(Size::Percent(0 .into())),)*(Transform2D::align((Size::Percent(0 .into())),(Size::Percent(0 .into())),)).into())
        )
    }));
    
    //Transform2D::anchor(50%,50%)*Transform2D::align(50%,50%)
    vtable.insert(7, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        #[allow(unused_parens)]
        TypesCoproduct::Transform2D(
            (Transform2D::anchor((Size::Percent(50.into())),(Size::Percent(50.into())),)*(Transform2D::align((Size::Percent(50.into())),(Size::Percent(50.into())),)).into())
        )
    }));
    
    //Color::rgba(0.0,0.0,0.0,1.0)
    vtable.insert(8, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        #[allow(unused_parens)]
        TypesCoproduct::__pax_stdCOCOtypesCOCOColor(
            Color::rgba((Numeric::from(0.0)),(Numeric::from(0.0)),(Numeric::from(0.0)),(Numeric::from(1.0)),)
        )
    }));
    
    //Transform2D::anchor(50%,50%)*Transform2D::align(50%,50%)
    vtable.insert(9, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        #[allow(unused_parens)]
        TypesCoproduct::Transform2D(
            (Transform2D::anchor((Size::Percent(50.into())),(Size::Percent(50.into())),)*(Transform2D::align((Size::Percent(50.into())),(Size::Percent(50.into())),)).into())
        )
    }));
    
    //current_route == 2 
    vtable.insert(10, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        
            let current_route = {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();

                
                    if let PropertiesCoproduct::Example(p) = properties {
                        
                            Numeric::from(p.current_route.get())
                        
                    } else {
                        unreachable!("10")
                    }
                
            };
        

        #[allow(unused_parens)]
        TypesCoproduct::bool(
            (current_route ==Numeric::from(2 ))
        )
    }));
    
    //current_route == 1 
    vtable.insert(11, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        
            let current_route = {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();

                
                    if let PropertiesCoproduct::Example(p) = properties {
                        
                            Numeric::from(p.current_route.get())
                        
                    } else {
                        unreachable!("11")
                    }
                
            };
        

        #[allow(unused_parens)]
        TypesCoproduct::bool(
            (current_route ==Numeric::from(1 ))
        )
    }));
    
    //current_route == 0 
    vtable.insert(12, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        
            let current_route = {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();

                
                    if let PropertiesCoproduct::Example(p) = properties {
                        
                            Numeric::from(p.current_route.get())
                        
                    } else {
                        unreachable!("12")
                    }
                
            };
        

        #[allow(unused_parens)]
        TypesCoproduct::bool(
            (current_route ==Numeric::from(0 ))
        )
    }));
    
    //0 .. 60
    vtable.insert(13, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        #[allow(unused_parens)]
        TypesCoproduct::Range_isize_(
            0 ..60
        )
    }));
    
    //Transform2D::anchor(50%,50%)*Transform2D::align(50%,50%)*Transform2D::rotate((i+2)*rotation+ticks/1000.0)*Transform2D::scale(0.75+(i*rotation),0.75+(i*rotation))*Transform2D::scale(1-((rotation/5)+i/1000.0),1-((rotation/5)+i/1000.0))
    vtable.insert(14, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        
            let i = {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();

                
                    if let PropertiesCoproduct::RepeatItem(elem, i) = properties {
                        
                            
                                
                                let unwrapped = if let PropertiesCoproduct::isize(i) = **elem {i} else {unreachable!()};
                                Numeric::from(unwrapped)
                            
                        

                    } else {unreachable!()}
                
            };
        
            let rotation = {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(1) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();

                
                    if let PropertiesCoproduct::Fireworks(p) = properties {
                        
                            Numeric::from(p.rotation.get())
                        
                    } else {
                        unreachable!("14")
                    }
                
            };
        
            let ticks = {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(1) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();

                
                    if let PropertiesCoproduct::Fireworks(p) = properties {
                        
                            Numeric::from(p.ticks.get())
                        
                    } else {
                        unreachable!("14")
                    }
                
            };
        

        #[allow(unused_parens)]
        TypesCoproduct::Transform2D(
            ((((Transform2D::anchor((Size::Percent(50.into())),(Size::Percent(50.into())),)*(Transform2D::align((Size::Percent(50.into())),(Size::Percent(50.into())),)).into())*(Transform2D::rotate(((((i +Numeric::from(2))*(rotation ).into())+(ticks /Numeric::from(1000.0)))),)).into())*(Transform2D::scale(((Numeric::from(0.75)+(i *(rotation).into()))),((Numeric::from(0.75)+(i *(rotation).into()))),)).into())*(Transform2D::scale(((Numeric::from(1 )-((rotation /Numeric::from(5))+(i /Numeric::from(1000.0))))),((Numeric::from(1 )-((rotation /Numeric::from(5))+(i /Numeric::from(1000.0))))),)).into())
        )
    }));
    
    //Color::hlc(ticks+i*360.0/30.0,75.0,150.0)
    vtable.insert(15, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        
            let ticks = {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(1) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();

                
                    if let PropertiesCoproduct::Fireworks(p) = properties {
                        
                            Numeric::from(p.ticks.get())
                        
                    } else {
                        unreachable!("15")
                    }
                
            };
        
            let i = {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();

                
                    if let PropertiesCoproduct::RepeatItem(elem, i) = properties {
                        
                            
                                
                                let unwrapped = if let PropertiesCoproduct::isize(i) = **elem {i} else {unreachable!()};
                                Numeric::from(unwrapped)
                            
                        

                    } else {unreachable!()}
                
            };
        

        #[allow(unused_parens)]
        TypesCoproduct::__pax_stdCOCOtypesCOCOColor(
            Color::hlc(((ticks +((i *(Numeric::from(360.0 )).into())/Numeric::from(30.0)))),(Numeric::from(75.0)),(Numeric::from(150.0)),)
        )
    }));
    

    vtable
}

//Begin component factory literals

    
pub fn instantiate_component_HelloRGB<R: 'static + RenderContext>(instance_registry: Rc<RefCell<InstanceRegistry<R>>>, mut args: InstantiationArgs<R>) -> Rc<RefCell<ComponentInstance<R>>>  {
    args.component_template = Some(Rc::new(RefCell::new(vec![

pax_std_primitives::ellipse::EllipseInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::Ellipse( Ellipse {
        
            stroke: Box::new( PropertyLiteral::new(Default::default()) ),
        
            fill: Box::new( PropertyExpression::new(1) ),
        
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
                                                     did_mount_handlers: vec![],
                                                     scroll_handlers: vec![|stack_frame, args|{
                                                                     let properties = (*stack_frame).borrow().get_properties();
                                                                     let properties = &mut *properties.as_ref().borrow_mut();
                                                                     let properties = if let PropertiesCoproduct::HelloRGB(p) = properties {p} else {unreachable!()};
                                                                     HelloRGB::handle_scroll(properties,args);
                                                                 },],
                                                 }
                                             ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyExpression::new(0))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Percent(50.into()))),Box::new(PropertyLiteral::new(Size::Percent(50.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
])));

    args.handler_registry = Some(Rc::new(RefCell::new(HandlerRegistry {
        click_handlers: vec![],
        will_render_handlers: vec![],
        did_mount_handlers: vec![],
        scroll_handlers: vec![],
    })));

    args.compute_properties_fn = Some(Box::new(|properties, rtc|{
        let properties = &mut *properties.as_ref().borrow_mut();
        let properties = if let PropertiesCoproduct::HelloRGB(p) = properties {p} else {unreachable!()};

        
            if let Some(new_value) = rtc.compute_eased_value(properties.rotation._get_transition_manager()) {
            properties.rotation.set(new_value);
            } else if let Some(new_value) = rtc.compute_vtable_value(properties.rotation._get_vtable_id()) {
            let new_value = if let TypesCoproduct::__f64(v) = new_value { v } else { unreachable!() };
            properties.rotation.set(new_value);
            }
        
    }));

    ComponentInstance::instantiate(args)
}




    
pub fn instantiate_component_Words<R: 'static + RenderContext>(instance_registry: Rc<RefCell<InstanceRegistry<R>>>, mut args: InstantiationArgs<R>) -> Rc<RefCell<ComponentInstance<R>>>  {
    args.component_template = Some(Rc::new(RefCell::new(vec![

pax_std_primitives::group::GroupInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::Group( Group {
        
    }),
    handler_registry:  Some(Rc::new(RefCell::new(
                                                 HandlerRegistry {
                                                     click_handlers: vec![],
                                                     will_render_handlers: vec![],
                                                     did_mount_handlers: vec![],
                                                     scroll_handlers: vec![],
                                                 }
                                             ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Default::default())),Box::new(PropertyLiteral::new(Default::default()))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
            

pax_std_primitives::text::TextInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::Text( Text {
        
            text: Box::new( PropertyLiteral::new("Layer".try_into().unwrap()) ),
        
            font: Box::new( PropertyLiteral::new(Default::default()) ),
        
            fill: Box::new( PropertyLiteral::new(Default::default()) ),
        
            size_font: Box::new( PropertyLiteral::new(Default::default()) ),
        
            style_link: Box::new( PropertyLiteral::new(Default::default()) ),
        
            align_multiline: Box::new( PropertyLiteral::new(Default::default()) ),
        
            align_vertical: Box::new( PropertyLiteral::new(Default::default()) ),
        
            align_horizontal: Box::new( PropertyLiteral::new(Default::default()) ),
        
            font_weight: Box::new( PropertyLiteral::new(Default::default()) ),
        
            font_style: Box::new( PropertyLiteral::new(Default::default()) ),
        
    }),
    handler_registry:  Some(Rc::new(RefCell::new(
                                                 HandlerRegistry {
                                                     click_handlers: vec![],
                                                     will_render_handlers: vec![],
                                                     did_mount_handlers: vec![],
                                                     scroll_handlers: vec![],
                                                 }
                                             ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Pixels(300.into()))),Box::new(PropertyLiteral::new(Size::Pixels(300.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
            

pax_std_primitives::image::ImageInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::Image( Image {
        
            path: Box::new( PropertyLiteral::new("assets/images/pax-logo.png".try_into().unwrap()) ),
        
    }),
    handler_registry:  Some(Rc::new(RefCell::new(
                                                 HandlerRegistry {
                                                     click_handlers: vec![],
                                                     will_render_handlers: vec![],
                                                     did_mount_handlers: vec![],
                                                     scroll_handlers: vec![],
                                                 }
                                             ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyExpression::new(2))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Pixels(280.into()))),Box::new(PropertyLiteral::new(Size::Pixels(120.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
            

pax_std_primitives::image::ImageInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::Image( Image {
        
            path: Box::new( PropertyLiteral::new("assets/images/jpeg_test.jpg".try_into().unwrap()) ),
        
    }),
    handler_registry:  Some(Rc::new(RefCell::new(
                                                 HandlerRegistry {
                                                     click_handlers: vec![],
                                                     will_render_handlers: vec![],
                                                     did_mount_handlers: vec![],
                                                     scroll_handlers: vec![],
                                                 }
                                             ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyExpression::new(3))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Pixels(500.into()))),Box::new(PropertyLiteral::new(Size::Pixels(500.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
            

pax_std_primitives::text::TextInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::Text( Text {
        
            text: Box::new( PropertyLiteral::new("apple".try_into().unwrap()) ),
        
            font: Box::new( PropertyLiteral::new(Default::default()) ),
        
            fill: Box::new( PropertyLiteral::new(Default::default()) ),
        
            size_font: Box::new( PropertyLiteral::new(Default::default()) ),
        
            style_link: Box::new( PropertyLiteral::new(Default::default()) ),
        
            align_multiline: Box::new( PropertyLiteral::new(Default::default()) ),
        
            align_vertical: Box::new( PropertyLiteral::new(Default::default()) ),
        
            align_horizontal: Box::new( PropertyLiteral::new(Default::default()) ),
        
            font_weight: Box::new( PropertyLiteral::new(Default::default()) ),
        
            font_style: Box::new( PropertyLiteral::new(Default::default()) ),
        
    }),
    handler_registry:  Some(Rc::new(RefCell::new(
                                                 HandlerRegistry {
                                                     click_handlers: vec![],
                                                     will_render_handlers: vec![],
                                                     did_mount_handlers: vec![],
                                                     scroll_handlers: vec![],
                                                 }
                                             ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Pixels(300.into()))),Box::new(PropertyLiteral::new(Size::Pixels(300.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
            

pax_std_primitives::rectangle::RectangleInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::Rectangle( Rectangle {
        
            stroke: Box::new( PropertyLiteral::new(Default::default()) ),
        
            fill: Box::new( PropertyExpression::new(5) ),
        
    }),
    handler_registry:  Some(Rc::new(RefCell::new(
                                                 HandlerRegistry {
                                                     click_handlers: vec![],
                                                     will_render_handlers: vec![],
                                                     did_mount_handlers: vec![],
                                                     scroll_handlers: vec![],
                                                 }
                                             ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyExpression::new(4))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Pixels(550.into()))),Box::new(PropertyLiteral::new(Size::Pixels(550.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
            

pax_std_primitives::text::TextInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::Text( Text {
        
            text: Box::new( PropertyLiteral::new("carrot".try_into().unwrap()) ),
        
            font: Box::new( PropertyLiteral::new(Default::default()) ),
        
            fill: Box::new( PropertyLiteral::new(Default::default()) ),
        
            size_font: Box::new( PropertyLiteral::new(Default::default()) ),
        
            style_link: Box::new( PropertyLiteral::new(Default::default()) ),
        
            align_multiline: Box::new( PropertyLiteral::new(Default::default()) ),
        
            align_vertical: Box::new( PropertyLiteral::new(Default::default()) ),
        
            align_horizontal: Box::new( PropertyLiteral::new(Default::default()) ),
        
            font_weight: Box::new( PropertyLiteral::new(Default::default()) ),
        
            font_style: Box::new( PropertyLiteral::new(Default::default()) ),
        
    }),
    handler_registry:  Some(Rc::new(RefCell::new(
                                                 HandlerRegistry {
                                                     click_handlers: vec![],
                                                     will_render_handlers: vec![],
                                                     did_mount_handlers: vec![],
                                                     scroll_handlers: vec![],
                                                 }
                                             ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Pixels(300.into()))),Box::new(PropertyLiteral::new(Size::Pixels(300.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
            

pax_std_primitives::image::ImageInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::Image( Image {
        
            path: Box::new( PropertyLiteral::new("assets/images/pax-logo.png".try_into().unwrap()) ),
        
    }),
    handler_registry:  Some(Rc::new(RefCell::new(
                                                 HandlerRegistry {
                                                     click_handlers: vec![],
                                                     will_render_handlers: vec![],
                                                     did_mount_handlers: vec![],
                                                     scroll_handlers: vec![],
                                                 }
                                             ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyExpression::new(6))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Pixels(280.into()))),Box::new(PropertyLiteral::new(Size::Pixels(120.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
            

pax_std_primitives::image::ImageInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::Image( Image {
        
            path: Box::new( PropertyLiteral::new("assets/images/jpeg_test.jpg".try_into().unwrap()) ),
        
    }),
    handler_registry:  Some(Rc::new(RefCell::new(
                                                 HandlerRegistry {
                                                     click_handlers: vec![],
                                                     will_render_handlers: vec![],
                                                     did_mount_handlers: vec![],
                                                     scroll_handlers: vec![],
                                                 }
                                             ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyExpression::new(7))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Pixels(500.into()))),Box::new(PropertyLiteral::new(Size::Pixels(500.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
            

pax_std_primitives::text::TextInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::Text( Text {
        
            text: Box::new( PropertyLiteral::new("hello world".try_into().unwrap()) ),
        
            font: Box::new( PropertyLiteral::new(Default::default()) ),
        
            fill: Box::new( PropertyLiteral::new(Default::default()) ),
        
            size_font: Box::new( PropertyLiteral::new(Default::default()) ),
        
            style_link: Box::new( PropertyLiteral::new(Default::default()) ),
        
            align_multiline: Box::new( PropertyLiteral::new(Default::default()) ),
        
            align_vertical: Box::new( PropertyLiteral::new(Default::default()) ),
        
            align_horizontal: Box::new( PropertyLiteral::new(Default::default()) ),
        
            font_weight: Box::new( PropertyLiteral::new(Default::default()) ),
        
            font_style: Box::new( PropertyLiteral::new(Default::default()) ),
        
    }),
    handler_registry:  Some(Rc::new(RefCell::new(
                                                 HandlerRegistry {
                                                     click_handlers: vec![],
                                                     will_render_handlers: vec![],
                                                     did_mount_handlers: vec![],
                                                     scroll_handlers: vec![],
                                                 }
                                             ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Pixels(300.into()))),Box::new(PropertyLiteral::new(Size::Pixels(300.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
            

pax_std_primitives::rectangle::RectangleInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::Rectangle( Rectangle {
        
            stroke: Box::new( PropertyLiteral::new(Default::default()) ),
        
            fill: Box::new( PropertyExpression::new(8) ),
        
    }),
    handler_registry:  Some(Rc::new(RefCell::new(
                                                 HandlerRegistry {
                                                     click_handlers: vec![],
                                                     will_render_handlers: vec![],
                                                     did_mount_handlers: vec![],
                                                     scroll_handlers: vec![],
                                                 }
                                             ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyExpression::new(9))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Pixels(550.into()))),Box::new(PropertyLiteral::new(Size::Pixels(550.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
])));

    args.handler_registry = Some(Rc::new(RefCell::new(HandlerRegistry {
        click_handlers: vec![],
        will_render_handlers: vec![],
        did_mount_handlers: vec![],
        scroll_handlers: vec![],
    })));

    args.compute_properties_fn = Some(Box::new(|properties, rtc|{
        let properties = &mut *properties.as_ref().borrow_mut();
        let properties = if let PropertiesCoproduct::Words(p) = properties {p} else {unreachable!()};

        
    }));

    ComponentInstance::instantiate(args)
}




    
pub fn instantiate_root_component<R: 'static + RenderContext>(instance_registry: Rc<RefCell<InstanceRegistry<R>>>) -> Rc<RefCell<ComponentInstance<R>>> {
    ComponentInstance::instantiate( InstantiationArgs{
        properties: PropertiesCoproduct::Example( Example::default() ),
        handler_registry:  Some(Rc::new(RefCell::new(
             HandlerRegistry {
                 click_handlers: vec![],
                 will_render_handlers: vec![],
                 did_mount_handlers: vec![],
                 scroll_handlers: vec![],
             }
         ))),
        instance_registry: Rc::clone(&instance_registry),
        transform: Transform2D::default_wrapped(),
        size: None,
        children: None,
        component_template: Some(Rc::new(RefCell::new(vec![

pax_std_primitives::group::GroupInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::Group( Group {
        
    }),
    handler_registry:  Some(Rc::new(RefCell::new(
                                                 HandlerRegistry {
                                                     click_handlers: vec![|stack_frame, args|{
                                                                    let properties = (*stack_frame).borrow().get_properties();
                                                                    let properties = &mut *properties.as_ref().borrow_mut();
                                                                    let properties = if let PropertiesCoproduct::Example(p) = properties {p} else {unreachable!()};
                                                                    Example::modulate(properties,args);
                                                                },],
                                                     will_render_handlers: vec![],
                                                     did_mount_handlers: vec![],
                                                     scroll_handlers: vec![],
                                                 }
                                             ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Default::default())),Box::new(PropertyLiteral::new(Default::default()))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
            

ConditionalInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::None,
    handler_registry:  Some(Rc::new(RefCell::new(
                                                 HandlerRegistry {
                                                     click_handlers: vec![],
                                                     will_render_handlers: vec![],
                                                     did_mount_handlers: vec![],
                                                     scroll_handlers: vec![],
                                                 }
                                             ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Default::default())),Box::new(PropertyLiteral::new(Default::default()))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
            

instantiate_component_Fireworks( Rc::clone(&instance_registry),
 InstantiationArgs {
    properties: PropertiesCoproduct::Fireworks( Fireworks {
        
            rotation: Box::new( PropertyLiteral::new(Default::default()) ),
        
            ticks: Box::new( PropertyLiteral::new(Default::default()) ),
        
    }),
    handler_registry:  Some(Rc::new(RefCell::new(
                                                 HandlerRegistry {
                                                     click_handlers: vec![],
                                                     will_render_handlers: vec![],
                                                     did_mount_handlers: vec![],
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
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: Some(Box::new(PropertyExpression::new(10))),
    compute_properties_fn: None,
})
,
        
            

ConditionalInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::None,
    handler_registry:  Some(Rc::new(RefCell::new(
                                                 HandlerRegistry {
                                                     click_handlers: vec![],
                                                     will_render_handlers: vec![],
                                                     did_mount_handlers: vec![],
                                                     scroll_handlers: vec![],
                                                 }
                                             ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Default::default())),Box::new(PropertyLiteral::new(Default::default()))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
            

instantiate_component_HelloRGB( Rc::clone(&instance_registry),
 InstantiationArgs {
    properties: PropertiesCoproduct::HelloRGB( HelloRGB {
        
            rotation: Box::new( PropertyLiteral::new(Default::default()) ),
        
    }),
    handler_registry:  Some(Rc::new(RefCell::new(
                                                 HandlerRegistry {
                                                     click_handlers: vec![],
                                                     will_render_handlers: vec![],
                                                     did_mount_handlers: vec![],
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
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: Some(Box::new(PropertyExpression::new(11))),
    compute_properties_fn: None,
})
,
        
            

ConditionalInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::None,
    handler_registry:  Some(Rc::new(RefCell::new(
                                                 HandlerRegistry {
                                                     click_handlers: vec![],
                                                     will_render_handlers: vec![],
                                                     did_mount_handlers: vec![],
                                                     scroll_handlers: vec![],
                                                 }
                                             ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Default::default())),Box::new(PropertyLiteral::new(Default::default()))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
            

instantiate_component_Words( Rc::clone(&instance_registry),
 InstantiationArgs {
    properties: PropertiesCoproduct::Words( Words {
        
    }),
    handler_registry:  Some(Rc::new(RefCell::new(
                                                 HandlerRegistry {
                                                     click_handlers: vec![],
                                                     will_render_handlers: vec![],
                                                     did_mount_handlers: vec![],
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
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: Some(Box::new(PropertyExpression::new(12))),
    compute_properties_fn: None,
})
,
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
]))),
        scroller_args: None,
        slot_index: None,
        repeat_source_expression_vec: None,
        repeat_source_expression_range: None,
        conditional_boolean_expression: None,
        compute_properties_fn: Some(Box::new(|properties, rtc|{
            let properties = &mut *properties.as_ref().borrow_mut();
            let properties = if let PropertiesCoproduct::Example(p) = properties {p} else {unreachable!()};

            
            if let Some(new_value) = rtc.compute_eased_value(properties.current_route._get_transition_manager()) {
            properties.current_route.set(new_value);
            } else if let Some(new_value) = rtc.compute_vtable_value(properties.current_route._get_vtable_id()) {
            let new_value = if let TypesCoproduct::__usize(v) = new_value { v } else { unreachable!() };
            properties.current_route.set(new_value);
            }
            
        })),
    })
}





    
pub fn instantiate_component_Fireworks<R: 'static + RenderContext>(instance_registry: Rc<RefCell<InstanceRegistry<R>>>, mut args: InstantiationArgs<R>) -> Rc<RefCell<ComponentInstance<R>>>  {
    args.component_template = Some(Rc::new(RefCell::new(vec![

pax_std_primitives::group::GroupInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::Group( Group {
        
    }),
    handler_registry:  Some(Rc::new(RefCell::new(
                                                 HandlerRegistry {
                                                     click_handlers: vec![],
                                                     will_render_handlers: vec![],
                                                     did_mount_handlers: vec![],
                                                     scroll_handlers: vec![|stack_frame, args|{
                                                                     let properties = (*stack_frame).borrow().get_properties();
                                                                     let properties = &mut *properties.as_ref().borrow_mut();
                                                                     let properties = if let PropertiesCoproduct::Fireworks(p) = properties {p} else {unreachable!()};
                                                                     Fireworks::handle_scroll(properties,args);
                                                                 },],
                                                 }
                                             ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Default::default())),Box::new(PropertyLiteral::new(Default::default()))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
            

RepeatInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::None,
    handler_registry:  Some(Rc::new(RefCell::new(
                                                 HandlerRegistry {
                                                     click_handlers: vec![],
                                                     will_render_handlers: vec![],
                                                     did_mount_handlers: vec![],
                                                     scroll_handlers: vec![],
                                                 }
                                             ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Default::default())),Box::new(PropertyLiteral::new(Default::default()))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
            

pax_std_primitives::rectangle::RectangleInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::Rectangle( Rectangle {
        
            stroke: Box::new( PropertyLiteral::new(Default::default()) ),
        
            fill: Box::new( PropertyExpression::new(15) ),
        
    }),
    handler_registry:  Some(Rc::new(RefCell::new(
                                                 HandlerRegistry {
                                                     click_handlers: vec![],
                                                     will_render_handlers: vec![],
                                                     did_mount_handlers: vec![],
                                                     scroll_handlers: vec![],
                                                 }
                                             ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyExpression::new(14))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Pixels(300.into()))),Box::new(PropertyLiteral::new(Size::Pixels(300.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: Some(Box::new(PropertyExpression::new(13))),
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
])));

    args.handler_registry = Some(Rc::new(RefCell::new(HandlerRegistry {
        click_handlers: vec![],
        will_render_handlers: vec![
                 |properties, args|{
                     let properties = &mut *properties.as_ref().borrow_mut();
                     let properties = if let PropertiesCoproduct::Fireworks(p) = properties {p} else {unreachable!()};
                     Fireworks::handle_will_render(properties,args);
                 },
             ],
        did_mount_handlers: vec![],
        scroll_handlers: vec![],
    })));

    args.compute_properties_fn = Some(Box::new(|properties, rtc|{
        let properties = &mut *properties.as_ref().borrow_mut();
        let properties = if let PropertiesCoproduct::Fireworks(p) = properties {p} else {unreachable!()};

        
            if let Some(new_value) = rtc.compute_eased_value(properties.rotation._get_transition_manager()) {
            properties.rotation.set(new_value);
            } else if let Some(new_value) = rtc.compute_vtable_value(properties.rotation._get_vtable_id()) {
            let new_value = if let TypesCoproduct::__f64(v) = new_value { v } else { unreachable!() };
            properties.rotation.set(new_value);
            }
        
            if let Some(new_value) = rtc.compute_eased_value(properties.ticks._get_transition_manager()) {
            properties.ticks.set(new_value);
            } else if let Some(new_value) = rtc.compute_vtable_value(properties.ticks._get_vtable_id()) {
            let new_value = if let TypesCoproduct::__usize(v) = new_value { v } else { unreachable!() };
            properties.ticks.set(new_value);
            }
        
    }));

    ComponentInstance::instantiate(args)
}




