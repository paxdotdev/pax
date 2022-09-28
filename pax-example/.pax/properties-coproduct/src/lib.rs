use std::cell::RefCell;
use std::rc::Rc;

pub enum PropertiesCoproduct {
    None,
    RepeatList(Vec<Rc<RefCell<PropertiesCoproduct>>>),
    RepeatItem(Rc<PropertiesCoproduct>, usize),
    
    Frame(pax_example::pax_reexports::pax_std::primitives::Frame),
    
    Stacker(pax_example::pax_reexports::pax_std::stacker::Stacker),
    
    Group(pax_example::pax_reexports::pax_std::primitives::Group),
    
    Text(pax_example::pax_reexports::pax_std::primitives::Text),
    
    Rectangle(pax_example::pax_reexports::pax_std::primitives::Rectangle),
    
    Jabberwocky(pax_example::pax_reexports::Jabberwocky),
    
}

//used namely for return types of expressions — may have other purposes
pub enum TypesCoproduct {
    
    String(String),
    
    Transform2D(pax_runtime_api::Transform2D),
    
    VecLABRLPAR__usizeCOMM__paxCOCOapiCOCOSizeRPARRABR(Vec<(pax_example::pax_reexports::usize,pax_example::pax_reexports::pax::api::Size)>),
    
    VecLABRRcLABR__pax_stdCOCOtypesCOCOStackerCellPropertiesRABRRABR(Vec<Rc<pax_example::pax_reexports::pax_std::types::StackerCellProperties>>),
    
    Vec_Rc_PropertiesCoproduct___(Vec<Rc<PropertiesCoproduct>>),
    
    __f64(pax_example::pax_reexports::f64),
    
    __i64(pax_example::pax_reexports::i64),
    
    __paxCOCOapiCOCOSize(pax_example::pax_reexports::pax::api::Size),
    
    __pax_stdCOCOtypesCOCOColor(pax_example::pax_reexports::pax_std::types::Color),
    
    __pax_stdCOCOtypesCOCOFont(pax_example::pax_reexports::pax_std::types::Font),
    
    __pax_stdCOCOtypesCOCOStackerDirection(pax_example::pax_reexports::pax_std::types::StackerDirection),
    
    __pax_stdCOCOtypesCOCOStroke(pax_example::pax_reexports::pax_std::types::Stroke),
    
    __stdCOCOstringCOCOString(pax_example::pax_reexports::std::string::String),
    
    __usize(pax_example::pax_reexports::usize),
    
    bool(bool),
    
    f64(f64),
    
    isize(isize),
    
    usize(usize),
    
}
