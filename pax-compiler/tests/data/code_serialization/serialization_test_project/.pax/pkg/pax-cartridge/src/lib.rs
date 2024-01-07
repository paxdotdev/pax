 #![allow(unused, unused_imports, non_snake_case, unused_parens)]

// generate imports, pointing to userland cartridge `pub mod pax_reexports`

use serialization_test_project::pax_reexports::pax_std::types::text::Font;

use serialization_test_project::pax_reexports::pax_lang::api::Numeric;

use serialization_test_project::pax_reexports::Example;

use serialization_test_project::pax_reexports::pax_std::types::GradientStop;

use serialization_test_project::pax_reexports::pax_std::types::Fill;

use serialization_test_project::pax_reexports::pax_std::primitives::Rectangle;

use serialization_test_project::pax_reexports::bool;

use serialization_test_project::pax_reexports::pax_std::types::text::LocalFont;

use serialization_test_project::pax_reexports::pax_std::types::Stroke;

use serialization_test_project::pax_reexports::pax_lang::api::Rotation;

use serialization_test_project::pax_reexports::pax_lang::api::Transform2D;

use serialization_test_project::pax_reexports::pax_std::types::RadialGradient;

use serialization_test_project::pax_reexports::f64;

use serialization_test_project::pax_reexports::pax_std::types::text::TextStyle;

use serialization_test_project::pax_reexports::pax_lang::api::Size;

use serialization_test_project::pax_reexports::pax_std::primitives::Text;

use serialization_test_project::pax_reexports::pax_lang::api::StringBox;

use serialization_test_project::pax_reexports::std::vec::Vec;

use serialization_test_project::pax_reexports::pax_std::types::text::TextAlignHorizontal;

use serialization_test_project::pax_reexports::std::string::String;

use serialization_test_project::pax_reexports::pax_std::types::RectangleCornerRadii;

use serialization_test_project::pax_reexports::pax_std::types::Color;

use serialization_test_project::pax_reexports::pax_std::types::ColorVariant;

use serialization_test_project::pax_reexports::pax_std::types::text::SystemFont;

use serialization_test_project::pax_reexports::pax_lang::api::SizePixels;

use serialization_test_project::pax_reexports::pax_std::types::text::FontWeight;

use serialization_test_project::pax_reexports::pax_std::types::text::FontStyle;

use serialization_test_project::pax_reexports::pax_std::types::LinearGradient;

use serialization_test_project::pax_reexports::pax_std::types::text::WebFont;

use serialization_test_project::pax_reexports::usize;

use serialization_test_project::pax_reexports::pax_std::types::text::TextAlignVertical;

use std::any::Any;

use std::cell::RefCell;

use std::collections::HashMap;

use std::collections::VecDeque;

use std::ops::Deref;

use std::rc::Rc;

use pax_core::RepeatItem;

use pax_core::RepeatProperties;

use pax_core::ConditionalProperties;

use pax_core::SlotProperties;

use pax_core::get_numeric_from_wrapped_properties;

use pax_runtime_api::PropertyInstance;

use pax_runtime_api::PropertyLiteral;

use pax_runtime_api::CommonProperties;

use pax_core::ComponentInstance;

use pax_core::InstanceNodePtr;

use pax_core::PropertyExpression;

use pax_core::InstanceNodePtrList;

use pax_core::RenderTreeContext;

use pax_core::PropertiesTreeContext;

use pax_core::ExpressionContext;

use pax_core::PaxEngine;

use pax_core::InstanceNode;

use pax_core::NodeRegistry;

use pax_core::HandlerRegistry;

use pax_core::InstantiationArgs;

use pax_core::ConditionalInstance;

use pax_core::SlotInstance;

use pax_core::properties::RuntimePropertiesStackFrame;

use pax_core::repeat::RepeatInstance;

use piet_common::RenderContext;


//pull in entire const token stream here e.g. `const JABBERWOCKY : &str = r#"’Twas brillig, and the slithy toves `...


pub fn instantiate_expression_table() -> HashMap<usize, Box<dyn Fn(ExpressionContext) -> Box<dyn Any>>> {
    let mut vtable: HashMap<usize, Box<dyn Fn(ExpressionContext) -> Box<dyn Any>>> = HashMap::new();

    
    //Font::system("TimesNewRoman",FontStyle::Normal,FontWeight::Bold)
    
        
    
    vtable.insert(0, Box::new(|ec: ExpressionContext| -> Box<dyn Any> {
        

        

        Box::new(Font::system((StringBox::from("Times New Roman").into()),(FontStyle::Normal),(FontWeight::Bold),))

    }));
    
        
    
    
    //Color::rgba(1.0,1.0,1.0,1.0)
    
        
    
    vtable.insert(1, Box::new(|ec: ExpressionContext| -> Box<dyn Any> {
        

        

        Box::new(Color::rgba((Numeric::from(1.0)),(Numeric::from(1.0)),(Numeric::from(1.0)),(Numeric::from(1.0)),))

    }));
    
        
    
    
    //self.message
    
        
    
    vtable.insert(2, Box::new(|ec: ExpressionContext| -> Box<dyn Any> {
        
            let message =
            {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let mut borrowed = &mut *(*properties).borrow_mut();
                

                    if let Some(p) = borrowed.downcast_ref::<serialization_test_project::pax_reexports::Example>(){

                        
                            //binding simple stringbox property
                            StringBox::from(p.message.get())
                        
                    } else {unreachable!()}
                
            };
            

        

        

        Box::new(message)

    }));
    
        
    
    
    //Fill::Solid(Color::hlc(ticks,75.0,150.0))
    
        
    
    vtable.insert(3, Box::new(|ec: ExpressionContext| -> Box<dyn Any> {
        
            let ticks =
            {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let mut borrowed = &mut *(*properties).borrow_mut();
                

                    if let Some(p) = borrowed.downcast_ref::<serialization_test_project::pax_reexports::Example>(){

                        
                            //binding simple numeric property
                            Numeric::from(p.ticks.get())
                        
                    } else {unreachable!()}
                
            };
            
                let ticks = Numeric::from( ticks );
            

        

        

        Box::new(Fill::Solid((Color::hlc((ticks),(Numeric::from(75.0)),(Numeric::from(150.0)),)),))

    }));
    
        
    
    
    //RectangleCornerRadii::radii(10.0,10.0,10.0,10.0)
    
        
    
    vtable.insert(4, Box::new(|ec: ExpressionContext| -> Box<dyn Any> {
        

        

        Box::new(RectangleCornerRadii::radii((Numeric::from(10.0)),(Numeric::from(10.0)),(Numeric::from(10.0)),(Numeric::from(10.0)),))

    }));
    
        
    
    

    vtable
}

//Begin component factory literals

    
pub fn instantiate_main_component(node_registry: Rc<RefCell<NodeRegistry>>) -> Rc<ComponentInstance> {
    ComponentInstance::instantiate( InstantiationArgs{
        prototypical_properties_factory: Box::new(||{Rc::new(RefCell::new( Example::default() )) as Rc<RefCell<dyn Any>>}),
        prototypical_common_properties_factory: Box::new(||{Rc::new(RefCell::new(CommonProperties::default()))}),
        handler_registry:  Some(Rc::new(RefCell::new({
            #[allow(unused_mut)]
            let mut handler_registry = HandlerRegistry::default();
                
                
                    
                
                handler_registry.pre_render_handlers =  vec![
                    
                        
                            
                        |properties, ctx|{
                                let properties = &mut *properties.as_ref().borrow_mut();
                                if let Some(mut synthesized_self) = properties.downcast_mut::<serialization_test_project::pax_reexports::Example>() {
                                    Example::handle_pre_render(&mut synthesized_self,ctx);
                                } else {panic!()};
                            },
                        
                        
                            
                        
                    
                ];
                
                    
                
                
                
                    
                
                handler_registry.mount_handlers =  vec![
                    
                        
                            
                        |properties, ctx|{
                                let properties = &mut *properties.as_ref().borrow_mut();
                                if let Some(mut synthesized_self) = properties.downcast_mut::<serialization_test_project::pax_reexports::Example>() {
                                    Example::handle_mount(&mut synthesized_self,ctx);
                                } else {panic!()};
                            },
                        
                        
                            
                        
                    
                ];
                
                    
                
                
            handler_registry
        }))),
        node_registry: Rc::clone(&node_registry),
        children: None,
        component_template: Some(vec![

pax_std_primitives::text::TextInstance::instantiate(

 InstantiationArgs {
    prototypical_common_properties_factory: Box::new(||{Rc::new(RefCell::new(CommonProperties {
        
            
                
            
            x: Some(Box::new(PropertyLiteral::new(Into::<Size>::into(Size::Percent(50.into()))))),
            
                
            
        
            
                
            
            y: Some(Box::new(PropertyLiteral::new(Into::<Size>::into(Size::Percent(50.into()))))),
            
                
            
        
            
            scale_x: Default::default(),
            
        
            
            scale_y: Default::default(),
            
        
            
            skew_x: Default::default(),
            
        
            
            skew_y: Default::default(),
            
        
            
                
            
            anchor_x: Some(Box::new(PropertyLiteral::new(Into::<Size>::into(Size::Percent(50.into()))))),
            
                
            
        
            
                
            
            anchor_y: Some(Box::new(PropertyLiteral::new(Into::<Size>::into(Size::Percent(50.into()))))),
            
                
            
        
            
            rotate: Default::default(),
            
        
            
            transform: Transform2D::default_wrapped(),
            
        
            
                
            
            width: Box::new(PropertyLiteral::new(Into::<Size>::into(Size::Pixels(120.into())))),
            
                
            
        
            
                
            
            height: Box::new(PropertyLiteral::new(Into::<Size>::into(Size::Pixels(120.into())))),
            
                
            
        
    }))}),
    prototypical_properties_factory: Box::new(||{Rc::new(RefCell::new(
        {
            let mut cps = Text::default();

            
                
                    
                
                cps.text =  Box::new(PropertyExpression::new(2)) ;
                
                    
                
            
                
                cps.style =  Box::new(PropertyLiteral::new(
{ let mut ret = serialization_test_project::pax_reexports::pax_std::types::text::TextStyle::default();

ret.font = Box::new(PropertyExpression::new(0));


ret.font_size = Box::new(PropertyLiteral::new(Into::<serialization_test_project::pax_reexports::pax_lang::api::SizePixels>::into(Size::Pixels(32.into()))));


ret.fill = Box::new(PropertyExpression::new(1));


ret.align_vertical = Box::new(PropertyLiteral::new(Into::<serialization_test_project::pax_reexports::pax_std::types::text::TextAlignVertical>::into(TextAlignVertical::Center)));


ret.align_horizontal = Box::new(PropertyLiteral::new(Into::<serialization_test_project::pax_reexports::pax_std::types::text::TextAlignHorizontal>::into(TextAlignHorizontal::Center)));


ret.align_multiline = Box::new(PropertyLiteral::new(Into::<serialization_test_project::pax_reexports::pax_std::types::text::TextAlignHorizontal>::into(TextAlignHorizontal::Center
        )));

 ret })) ;
                
            

            cps
        }
    ))}),
    handler_registry:  Some(Rc::new(RefCell::new({
        #[allow(unused_mut)]
        let mut handler_registry = HandlerRegistry::default();
        
        handler_registry
    }))),
    node_registry: Rc::clone(&node_registry),
    children: Some(vec![
        
    ]),
    component_template: None,
    scroller_args: None,
    compute_properties_fn: None,
})
,

pax_std_primitives::rectangle::RectangleInstance::instantiate(

 InstantiationArgs {
    prototypical_common_properties_factory: Box::new(||{Rc::new(RefCell::new(CommonProperties {
        
            
                
            
            x: Some(Box::new(PropertyLiteral::new(Into::<Size>::into(Size::Percent(50.into()))))),
            
                
            
        
            
                
            
            y: Some(Box::new(PropertyLiteral::new(Into::<Size>::into(Size::Percent(50.into()))))),
            
                
            
        
            
            scale_x: Default::default(),
            
        
            
            scale_y: Default::default(),
            
        
            
            skew_x: Default::default(),
            
        
            
            skew_y: Default::default(),
            
        
            
                
            
            anchor_x: Some(Box::new(PropertyLiteral::new(Into::<Size>::into(Size::Percent(50.into()))))),
            
                
            
        
            
                
            
            anchor_y: Some(Box::new(PropertyLiteral::new(Into::<Size>::into(Size::Percent(50.into()))))),
            
                
            
        
            
            rotate: Default::default(),
            
        
            
            transform: Transform2D::default_wrapped(),
            
        
            
                
            
            width: Box::new(PropertyLiteral::new(Into::<Size>::into(Size::Pixels(120.into())))),
            
                
            
        
            
                
            
            height: Box::new(PropertyLiteral::new(Into::<Size>::into(Size::Pixels(120.into())))),
            
                
            
        
    }))}),
    prototypical_properties_factory: Box::new(||{Rc::new(RefCell::new(
        {
            let mut cps = Rectangle::default();

            
                
                    
                
                cps.fill =  Box::new(PropertyExpression::new(3)) ;
                
                    
                
            
                
                    
                
                cps.corner_radii =  Box::new(PropertyExpression::new(4)) ;
                
                    
                
            

            cps
        }
    ))}),
    handler_registry:  Some(Rc::new(RefCell::new({
        #[allow(unused_mut)]
        let mut handler_registry = HandlerRegistry::default();
        
            
                
            
            handler_registry.click_handlers = vec![
                
                    
                
                    |properties, ctx, args|{
                        let properties = &mut *properties.as_ref().borrow_mut();
                        if let Some(mut synthesized_self) = properties.downcast_mut::<serialization_test_project::pax_reexports::Example>() {
                            serialization_test_project::pax_reexports::Example::increment(&mut synthesized_self,ctx,args);
                        } else {panic!()}; //failed to downcast
                    },
                
                
                    
                
            ];
            
                
            
        
        handler_registry
    }))),
    node_registry: Rc::clone(&node_registry),
    children: Some(vec![
        
    ]),
    component_template: None,
    scroller_args: None,
    compute_properties_fn: None,
})
]),
        scroller_args: None,
        compute_properties_fn: Some(Box::new(|node, ptc|{
            let props = node.borrow().get_properties();
            let properties = &mut props.as_ref().borrow_mut();

            if let Some(properties) = properties.downcast_mut::<serialization_test_project::pax_reexports::Example>() {
                
                    if let Some(new_value) = ptc.compute_eased_value(properties.ticks._get_transition_manager()) {
                        properties.ticks.set(new_value);
                    } else if let Some(vtable_id) = properties.ticks._get_vtable_id() {
                        let new_value_wrapped = ptc.compute_vtable_value(node, vtable_id);
                        if let Ok(new_value) = new_value_wrapped.downcast::<serialization_test_project::pax_reexports::usize>() {
                            properties.ticks.set(*new_value);
                        } else {
                            panic!(
                                "generated code tried to downcast to incompatible type \"serialization_test_project::pax_reexports::usize\" for property \"ticks\" on serialization_test_project::pax_reexports::Example"
                            );
                        }
                    }
                
                    if let Some(new_value) = ptc.compute_eased_value(properties.num_clicks._get_transition_manager()) {
                        properties.num_clicks.set(new_value);
                    } else if let Some(vtable_id) = properties.num_clicks._get_vtable_id() {
                        let new_value_wrapped = ptc.compute_vtable_value(node, vtable_id);
                        if let Ok(new_value) = new_value_wrapped.downcast::<serialization_test_project::pax_reexports::usize>() {
                            properties.num_clicks.set(*new_value);
                        } else {
                            panic!(
                                "generated code tried to downcast to incompatible type \"serialization_test_project::pax_reexports::usize\" for property \"num_clicks\" on serialization_test_project::pax_reexports::Example"
                            );
                        }
                    }
                
                    if let Some(new_value) = ptc.compute_eased_value(properties.message._get_transition_manager()) {
                        properties.message.set(new_value);
                    } else if let Some(vtable_id) = properties.message._get_vtable_id() {
                        let new_value_wrapped = ptc.compute_vtable_value(node, vtable_id);
                        if let Ok(new_value) = new_value_wrapped.downcast::<serialization_test_project::pax_reexports::std::string::String>() {
                            properties.message.set(*new_value);
                        } else {
                            panic!(
                                "generated code tried to downcast to incompatible type \"serialization_test_project::pax_reexports::std::string::String\" for property \"message\" on serialization_test_project::pax_reexports::Example"
                            );
                        }
                    }
                
            } else {
                panic!("generated code couldn't downcast properties to \"serialization_test_project::pax_reexports::Example\"");
            }

        })),
    })
}





