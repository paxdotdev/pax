
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

use pax_example::pax_reexports::pax_std::types::Font;

use pax_example::pax_reexports::pax_std::types::Stroke;

use pax_example::pax_reexports::std::string::String;

use pax_example::pax_reexports::HelloRGB;

use pax_example::pax_reexports::pax_std::primitives::Group;

use pax_example::pax_reexports::pax_std::primitives::Rectangle;

use pax_example::pax_reexports::pax_std::primitives::Text;


//pull in entire const token stream here e.g. `const JABBERWOCKY : &str = r#"’Twas brillig, and the slithy toves `...


pub fn instantiate_expression_table<R: 'static + RenderContext>() -> HashMap<usize, Box<dyn Fn(ExpressionContext<R>) -> TypesCoproduct>> {
    let mut vtable: HashMap<usize, Box<dyn Fn(ExpressionContext<R>) -> TypesCoproduct>> = HashMap::new();

    
    //Color :: rgb(1, 0, 0) 
    vtable.insert(0, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        TypesCoproduct::__pax_stdCOCOtypesCOCOColor(
            Color::rgb((1.into()),(0.into()),(0.into()),)
        )
    }));
    
    //Transform2D :: rotate(1.25) * Transform2D :: translate(50, 50)
    vtable.insert(1, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        TypesCoproduct::Transform2D(
            (Transform2D::rotate((1.25.into()),)*Transform2D::translate((50.into()),(50.into()),))
        )
    }));
    
    //Color :: rgb(0, 1, 0) 
    vtable.insert(2, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        TypesCoproduct::__pax_stdCOCOtypesCOCOColor(
            Color::rgb((0.into()),(1.into()),(0.into()),)
        )
    }));
    
    //Transform2D :: rotate(2.25) * Transform2D :: translate(300, 100)
    vtable.insert(3, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        TypesCoproduct::Transform2D(
            (Transform2D::rotate((2.25.into()),)*Transform2D::translate((300.into()),(100.into()),))
        )
    }));
    
    //Color :: rgb(0, 0, 1) 
    vtable.insert(4, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        TypesCoproduct::__pax_stdCOCOtypesCOCOColor(
            Color::rgb((0.into()),(0.into()),(1.into()),)
        )
    }));
    
    //Transform2D :: rotate(3.25) * Transform2D :: translate(500, 550)
    vtable.insert(5, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        TypesCoproduct::Transform2D(
            (Transform2D::rotate((3.25.into()),)*Transform2D::translate((500.into()),(550.into()),))
        )
    }));
    
    //Color :: rgb(0, 0, 0) 
    vtable.insert(6, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        TypesCoproduct::__pax_stdCOCOtypesCOCOColor(
            Color::rgb((0.into()),(0.into()),(0.into()),)
        )
    }));
    

    vtable
}

//Begin component factory literals

    
pub fn instantiate_root_component<R: 'static + RenderContext>(instance_registry: Rc<RefCell<InstanceRegistry<R>>>) -> Rc<RefCell<ComponentInstance<R>>> {
    ComponentInstance::instantiate( InstantiationArgs{
        properties: PropertiesCoproduct::HelloRGB( HelloRGB::default() ),
        handler_registry: None, //TODO: codegen!
        instance_registry: Rc::clone(&instance_registry),
        transform: Transform2D::default_wrapped(),
        size: None,
        children: None,
        component_template: Some(Rc::new(RefCell::new(vec![

pax_std_primitives::text::TextInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::Text( Text {
        
            text: Box::new( PropertyLiteral::new("Hello".into()) ),
        
            font: Box::new( PropertyLiteral::new(Default::default()) ),
        
            fill: Box::new( PropertyLiteral::new(Default::default()) ),
        
    }),
    handler_registry: None,
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Default::default())),Box::new(PropertyLiteral::new(Default::default()))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None, //TODO! handle
    slot_index: None, //TODO! handle
    repeat_source_expression: None, //TODO! handle
    conditional_boolean_expression: None, //TODO! handle
    compute_properties_fn: None,
})

,

pax_std_primitives::rectangle::RectangleInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::Rectangle( Rectangle {
        
            stroke: Box::new( PropertyLiteral::new(Default::default()) ),
        
            fill: Box::new( PropertyExpression::new(0) ),
        
    }),
    handler_registry: None,
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyExpression::new(1))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Pixels(50.into()))),Box::new(PropertyLiteral::new(Size::Pixels(50.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None, //TODO! handle
    slot_index: None, //TODO! handle
    repeat_source_expression: None, //TODO! handle
    conditional_boolean_expression: None, //TODO! handle
    compute_properties_fn: None,
})

,

pax_std_primitives::rectangle::RectangleInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::Rectangle( Rectangle {
        
            stroke: Box::new( PropertyLiteral::new(Default::default()) ),
        
            fill: Box::new( PropertyExpression::new(2) ),
        
    }),
    handler_registry: None,
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyExpression::new(3))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Pixels(150.into()))),Box::new(PropertyLiteral::new(Size::Pixels(150.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None, //TODO! handle
    slot_index: None, //TODO! handle
    repeat_source_expression: None, //TODO! handle
    conditional_boolean_expression: None, //TODO! handle
    compute_properties_fn: None,
})

,

pax_std_primitives::rectangle::RectangleInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::Rectangle( Rectangle {
        
            stroke: Box::new( PropertyLiteral::new(Default::default()) ),
        
            fill: Box::new( PropertyExpression::new(4) ),
        
    }),
    handler_registry: None,
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyExpression::new(5))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Pixels(300.into()))),Box::new(PropertyLiteral::new(Size::Pixels(75.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None, //TODO! handle
    slot_index: None, //TODO! handle
    repeat_source_expression: None, //TODO! handle
    conditional_boolean_expression: None, //TODO! handle
    compute_properties_fn: None,
})

,

pax_std_primitives::rectangle::RectangleInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::Rectangle( Rectangle {
        
            stroke: Box::new( PropertyLiteral::new(Default::default()) ),
        
            fill: Box::new( PropertyExpression::new(6) ),
        
    }),
    handler_registry: None,
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Percent(100.into()))),Box::new(PropertyLiteral::new(Size::Percent(100.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None, //TODO! handle
    slot_index: None, //TODO! handle
    repeat_source_expression: None, //TODO! handle
    conditional_boolean_expression: None, //TODO! handle
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

            
        })),
    })
}





