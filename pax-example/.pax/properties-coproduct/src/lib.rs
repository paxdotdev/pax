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

    
    crateCOCOExample(pax_example::pax_reexports::Example),
    
    crateCOCOcameraCOCOCamera(pax_example::pax_reexports::camera::Camera),
    
    crateCOCOcameraCOCOTypeExample(pax_example::pax_reexports::camera::TypeExample),
    
    crateCOCOfireworksCOCOFireworks(pax_example::pax_reexports::fireworks::Fireworks),
    
    crateCOCOgridsCOCOGrids(pax_example::pax_reexports::grids::Grids),
    
    crateCOCOgridsCOCORectDef(pax_example::pax_reexports::grids::RectDef),
    
    crateCOCOhello_rgbCOCOHelloRGB(pax_example::pax_reexports::hello_rgb::HelloRGB),
    
    pax_stdCOCOprimitivesCOCOEllipse(pax_example::pax_reexports::pax_std::primitives::Ellipse),
    
    pax_stdCOCOprimitivesCOCOFrame(pax_example::pax_reexports::pax_std::primitives::Frame),
    
    pax_stdCOCOprimitivesCOCOGroup(pax_example::pax_reexports::pax_std::primitives::Group),
    
    pax_stdCOCOprimitivesCOCORectangle(pax_example::pax_reexports::pax_std::primitives::Rectangle),
    
    pax_stdCOCOtypesCOCOColor(pax_example::pax_reexports::pax_std::types::Color),
    
    pax_stdCOCOtypesCOCOStroke(pax_example::pax_reexports::pax_std::types::Stroke),
    
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
