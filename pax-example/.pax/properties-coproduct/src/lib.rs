use std::cell::RefCell;
use std::rc::Rc;

//Component types
#[repr(u32)]
pub enum PropertiesCoproduct {
    /* entries generated via properties-coproduct-lib.tera */
    None,
    RepeatList(Vec<Rc<RefCell<PropertiesCoproduct>>>),
    RepeatItem(Rc<PropertiesCoproduct>, usize),
    isize(isize),
    Range_isize_(std::ops::Range<isize>),

    
    Camera(pax_example::pax_reexports::camera::Camera),
    
    Color(pax_example::pax_reexports::pax_std::types::Color),
    
    Ellipse(pax_example::pax_reexports::pax_std::primitives::Ellipse),
    
    Example(pax_example::pax_reexports::Example),
    
    Fireworks(pax_example::pax_reexports::fireworks::Fireworks),
    
    Frame(pax_example::pax_reexports::pax_std::primitives::Frame),
    
    Grids(pax_example::pax_reexports::grids::Grids),
    
    Group(pax_example::pax_reexports::pax_std::primitives::Group),
    
    HelloRGB(pax_example::pax_reexports::hello_rgb::HelloRGB),
    
    RectDef(pax_example::pax_reexports::grids::RectDef),
    
    Rectangle(pax_example::pax_reexports::pax_std::primitives::Rectangle),
    
    Stroke(pax_example::pax_reexports::pax_std::types::Stroke),
    
    TypeExample(pax_example::pax_reexports::camera::TypeExample),
    
}

//Property types
#[repr(u32)]
pub enum TypesCoproduct {
    
    Range_isize_(std::ops::Range<isize>),
    
    Size(pax_runtime_api::Size),
    
    Size2D(pax_runtime_api::Size2D),
    
    SizePixels(pax_runtime_api::SizePixels),
    
    String(String),
    
    Transform2D(pax_runtime_api::Transform2D),
    
    Vec_Rc_PropertiesCoproduct___(std::vec::Vec<std::rc::Rc<PropertiesCoproduct>>),
    
    bool(bool),
    
    crateCOCOcameraCOCOTypeExample(pax_example::pax_reexports::camera::TypeExample),
    
    f64(f64),
    
    isize(isize),
    
    pax_stdCOCOtypesCOCOColor(pax_example::pax_reexports::pax_std::types::Color),
    
    pax_stdCOCOtypesCOCOStroke(pax_example::pax_reexports::pax_std::types::Stroke),
    
    stdCOCOvecCOCOVecLABRcrateCOCOgridsCOCORectDefRABR(pax_example::pax_reexports::std::vec::Vec<pax_example::pax_reexports::grids::RectDef>),
    
    usize(pax_example::pax_reexports::usize),
    
}
