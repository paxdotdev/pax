use std::cell::RefCell;
use std::rc::Rc;

#[repr(u32)]
pub enum PropertiesCoproduct {
    /* entries generated via properties-coproduct-lib.tera */
    None,
    RepeatList(Vec<Rc<RefCell<PropertiesCoproduct>>>),
    RepeatItem(Rc<PropertiesCoproduct>, usize),
    isize(isize),
    Range_isize_(std::ops::Range<isize>),

    
    Ellipse(pax_example::pax_reexports::pax_std::primitives::Ellipse),
    
    HelloRGB(pax_example::pax_reexports::HelloRGB),
    
}

//used namely for return types of expressions — may have other purposes
#[repr(u32)]
pub enum TypesCoproduct {
    
    Range_isize_(std::ops::Range<isize>),
    
    Size(pax_runtime_api::Size),
    
    Size2D(pax_runtime_api::Size2D),
    
    SizePixels(pax_runtime_api::SizePixels),
    
    String(String),
    
    Transform2D(pax_runtime_api::Transform2D),
    
    Vec_Rc_PropertiesCoproduct___(std::vec::Vec<std::rc::Rc<PropertiesCoproduct>>),
    
    __f64(pax_example::pax_reexports::f64),
    
    __pax_stdCOCOtypesCOCOColor(pax_example::pax_reexports::pax_std::types::Color),
    
    __pax_stdCOCOtypesCOCOStroke(pax_example::pax_reexports::pax_std::types::Stroke),
    
    bool(bool),
    
    f64(f64),
    
    isize(isize),
    
    usize(usize),
    
}
