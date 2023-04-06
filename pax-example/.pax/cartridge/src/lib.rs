
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

use pax_example::pax_reexports::usize;

use pax_example::pax_reexports::HelloRGB;

use pax_example::pax_reexports::pax_std::primitives::Ellipse;

use pax_example::pax_reexports::pax_std::primitives::Group;

use pax_example::pax_reexports::pax_std::primitives::Path;

use pax_example::pax_reexports::pax_std::primitives::Rectangle;

use pax_example::pax_reexports::pax_std::primitives::Text;


//pull in entire const token stream here e.g. `const JABBERWOCKY : &str = r#"’Twas brillig, and the slithy toves `...


pub fn instantiate_expression_table<R: 'static + RenderContext>() -> HashMap<usize, Box<dyn Fn(ExpressionContext<R>) -> TypesCoproduct>> {
    let mut vtable: HashMap<usize, Box<dyn Fn(ExpressionContext<R>) -> TypesCoproduct>> = HashMap::new();

    
    //Transform2D::anchor(50%,50%)*Transform2D::align(50%,50%)*Transform2D::scale(-0.1+0.9/(rotation*rotation+0.9),-0.1+0.9/(rotation*rotation+0.9))
    vtable.insert(0, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        
            let rotation = {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().nth_ancestor(0) {
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
            ((Transform2D::anchor((Size::Percent(50.into())),(Size::Percent(50.into())),)*(Transform2D::align((Size::Percent(50.into())),(Size::Percent(50.into())),)).into())*(Transform2D::scale((((-Numeric::from(0.1 ))+(Numeric::from(0.9 )/((rotation *(rotation ).into())+Numeric::from(0.9))))),(((-Numeric::from(0.1 ))+(Numeric::from(0.9 )/((rotation *(rotation ).into())+Numeric::from(0.9))))),)).into())
        )
    }));
    
    //squares
    vtable.insert(1, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        
            let squares = {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().nth_ancestor(1) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();

                

                
                    if let PropertiesCoproduct::HelloRGB(p) = properties {
                        
                            p.squares.get().clone()
                        
                    } else {
                        unreachable!("1")
                    }
                
            };
        

        #[allow(unused_parens)]
        TypesCoproduct::(
            squares
        )
    }));
    
    //Color::hlc(ticks/4.0+i*360.0/18.75,75.0,150.0)
    vtable.insert(2, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        
            let ticks = {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().nth_ancestor(1) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();

                

                
                    if let PropertiesCoproduct::HelloRGB(p) = properties {
                        
                            Numeric::from(p.ticks.get())
                        
                    } else {
                        unreachable!("2")
                    }
                
            };
        
            let i = {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().nth_ancestor(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();

                

                
                    if let PropertiesCoproduct::RepeatItem(elem, i) = properties {
                        
                            
                            Numeric::from(*i)
                        

                    } else {unreachable!()}
                
            };
        

        #[allow(unused_parens)]
        TypesCoproduct::__pax_stdCOCOtypesCOCOColor(
            Color::hlc((((ticks /Numeric::from(4.0 ))+((i *(Numeric::from(360.0 )).into())/Numeric::from(18.75)))),(Numeric::from(75.0)),(Numeric::from(150.0)),)
        )
    }));
    
    //Transform2D::anchor(50%,50%)*Transform2D::align(50%,50%)*Transform2D::rotate((i+2)*rotation)*Transform2D::scale(1.0+(i*rotation/2.0),1.0+(i*rotation/2.0))
    vtable.insert(3, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        
            let i = {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().nth_ancestor(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();

                

                
                    if let PropertiesCoproduct::RepeatItem(elem, i) = properties {
                        
                            
                            Numeric::from(*i)
                        

                    } else {unreachable!()}
                
            };
        
            let rotation = {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().nth_ancestor(1) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();

                

                
                    if let PropertiesCoproduct::HelloRGB(p) = properties {
                        
                            Numeric::from(p.rotation.get())
                        
                    } else {
                        unreachable!("3")
                    }
                
            };
        

        #[allow(unused_parens)]
        TypesCoproduct::Transform2D(
            (((Transform2D::anchor((Size::Percent(50.into())),(Size::Percent(50.into())),)*(Transform2D::align((Size::Percent(50.into())),(Size::Percent(50.into())),)).into())*(Transform2D::rotate((((i +Numeric::from(2))*(rotation).into())),)).into())*(Transform2D::scale(((Numeric::from(1.0 )+((i *(rotation ).into())/Numeric::from(2.0)))),((Numeric::from(1.0 )+((i *(rotation ).into())/Numeric::from(2.0)))),)).into())
        )
    }));
    
    //Transform2D::translate(5000.0,5000.0)
    vtable.insert(4, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        #[allow(unused_parens)]
        TypesCoproduct::Transform2D(
            Transform2D::translate((Numeric::from(5000.0)),(Numeric::from(5000.0)),)
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
                                                         click_handlers: vec![],
                                                         will_render_handlers: vec![
                                                                     |properties, args|{
                                                                         let properties = &mut *properties.as_ref().borrow_mut();
                                                                         let properties = if let PropertiesCoproduct::HelloRGB(p) = properties {p} else {unreachable!()};
                                                                         HelloRGB::handle_will_render(properties,args);
                                                                     },
                                                                 ],
                                                         did_mount_handlers: vec![
                                                                      |properties|{
                                                                          let properties = &mut *properties.as_ref().borrow_mut();
                                                                          let properties = if let PropertiesCoproduct::HelloRGB(p) = properties {p} else {unreachable!()};
                                                                          HelloRGB::handle_did_mount(properties);
                                                                      },
                                                                  ],
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
                                                     click_handlers: vec![],
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
        
            fill: Box::new( PropertyExpression::new(2) ),
        
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
    repeat_source_expression_vec: Some(Box::new(PropertyExpression::new(1))),
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
,

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
    transform: Rc::new(RefCell::new(PropertyExpression::new(4))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Default::default())),Box::new(PropertyLiteral::new(Default::default()))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
            

pax_std_primitives::ellipse::EllipseInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::Ellipse( Ellipse {
        
            stroke: Box::new( PropertyLiteral::new(Default::default()) ),
        
            fill: Box::new( PropertyLiteral::new(Default::default()) ),
        
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
        
            

pax_std_primitives::text::TextInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::Text( Text {
        
            text: Box::new( PropertyLiteral::new(Default::default()) ),
        
            font: Box::new( PropertyLiteral::new(Default::default()) ),
        
            fill: Box::new( PropertyLiteral::new(Default::default()) ),
        
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
        
            

pax_std_primitives::rectangle::RectangleInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::Rectangle( Rectangle {
        
            stroke: Box::new( PropertyLiteral::new(Default::default()) ),
        
            fill: Box::new( PropertyLiteral::new(Default::default()) ),
        
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
            let properties = if let PropertiesCoproduct::HelloRGB(p) = properties {p} else {unreachable!()};

            
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
            
            if let Some(new_value) = rtc.compute_eased_value(properties.heartbeat._get_transition_manager()) {
            properties.heartbeat.set(new_value);
            } else if let Some(new_value) = rtc.compute_vtable_value(properties.heartbeat._get_vtable_id()) {
            let new_value = if let TypesCoproduct::__f64(v) = new_value { v } else { unreachable!() };
            properties.heartbeat.set(new_value);
            }
            
            if let Some(new_value) = rtc.compute_eased_value(properties.squares._get_transition_manager()) {
            properties.squares.set(new_value);
            } else if let Some(new_value) = rtc.compute_vtable_value(properties.squares._get_vtable_id()) {
            let new_value = if let TypesCoproduct::VecLABR__f64RABR(v) = new_value { v } else { unreachable!() };
            properties.squares.set(new_value);
            }
            
        })),
    })
}





