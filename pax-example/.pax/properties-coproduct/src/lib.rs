use std::cell::RefCell;
use std::rc::Rc;

pub enum PropertiesCoproduct {
    None,
    RepeatList(Vec<Rc<RefCell<PropertiesCoproduct>>>),
    RepeatItem(Rc<PropertiesCoproduct>, usize),
    
    Group(pax_example::pax_reexports::pax_std::primitives::Group),
    
    HelloRGB(pax_example::pax_reexports::HelloRGB),
    
    Rectangle(pax_example::pax_reexports::pax_std::primitives::Rectangle),
    
}

//used namely for return types of expressions — may have other purposes
pub enum TypesCoproduct {
    
    Size2D(pax_runtime_api::Size2D),
    
    String(String),
    
    Transform2D(pax_runtime_api::Transform2D),
    
    Vec_Rc_PropertiesCoproduct___(std::vec::Vec<std::rc::Rc<PropertiesCoproduct>>),
    
    __pax_stdCOCOtypesCOCOColor(pax_example::pax_reexports::pax_std::types::Color),
    
    __pax_stdCOCOtypesCOCOStroke(pax_example::pax_reexports::pax_std::types::Stroke),
    
    bool(bool),
    
    f64(f64),
    
    isize(isize),
    
    usize(usize),
    
}
