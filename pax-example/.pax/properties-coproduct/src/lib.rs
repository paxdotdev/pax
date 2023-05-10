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
    
    Example(pax_example::pax_reexports::Example),
    
    Fireworks(pax_example::pax_reexports::fireworks::Fireworks),
    
    Group(pax_example::pax_reexports::pax_std::primitives::Group),
    
    HelloRGB(pax_example::pax_reexports::hello_rgb::HelloRGB),
    
    Rectangle(pax_example::pax_reexports::pax_std::primitives::Rectangle),
    
    Text(pax_example::pax_reexports::pax_std::primitives::Text),
    
    Words(pax_example::pax_reexports::words::Words),
    
}

//used namely for return types of expressions — may have other purposes
#[repr(u32)]
pub enum TypesCoproduct {
    
    OptionLABR__pax_stdCOCOtypesCOCOAlignmentRABR(Option<pax_example::pax_reexports::pax_std::types::Alignment>),
    
    Range_isize_(std::ops::Range<isize>),
    
    Size(pax_runtime_api::Size),
    
    Size2D(pax_runtime_api::Size2D),
    
    SizePixels(pax_runtime_api::SizePixels),
    
    String(String),
    
    Transform2D(pax_runtime_api::Transform2D),
    
    Vec_Rc_PropertiesCoproduct___(std::vec::Vec<std::rc::Rc<PropertiesCoproduct>>),
    
    __f64(pax_example::pax_reexports::f64),
    
    __pax_stdCOCOtypesCOCOAlignment(pax_example::pax_reexports::pax_std::types::Alignment),
    
    __pax_stdCOCOtypesCOCOColor(pax_example::pax_reexports::pax_std::types::Color),
    
    __pax_stdCOCOtypesCOCOFont(pax_example::pax_reexports::pax_std::types::Font),
    
    __pax_stdCOCOtypesCOCOStroke(pax_example::pax_reexports::pax_std::types::Stroke),
    
    __pax_stdCOCOtypesCOCOVAlignment(pax_example::pax_reexports::pax_std::types::VAlignment),
    
    __stdCOCOstringCOCOString(pax_example::pax_reexports::std::string::String),
    
    __usize(pax_example::pax_reexports::usize),
    
    bool(bool),
    
    f64(f64),
    
    isize(isize),
    
    usize(usize),
    
}
