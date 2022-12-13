use std::cell::RefCell;
use std::rc::Rc;

pub enum PropertiesCoproduct {
    None,
    RepeatList(Vec<Rc<RefCell<PropertiesCoproduct>>>),
    RepeatItem(Rc<PropertiesCoproduct>, usize),
    
    Frame(pax_example::pax_reexports::pax_std::primitives::Frame),
    
    Group(pax_example::pax_reexports::pax_std::primitives::Group),
    
    Jabberwocky(pax_example::pax_reexports::Jabberwocky),
    
    Rectangle(pax_example::pax_reexports::pax_std::primitives::Rectangle),
    
    Stacker(pax_example::pax_reexports::pax_std::stacker::Stacker),
    
    Text(pax_example::pax_reexports::pax_std::primitives::Text),
    
}

//used namely for return types of expressions — may have other purposes
pub enum TypesCoproduct {
    
    String(String),
    
    Transform2D(pax_runtime_api::Transform2D),
    
    VecLABRLPAR__usizeCOMM__paxCOCOapiCOCOSizeRPARRABR(Vec<(pax_example::pax_reexports::usize,pax_example::pax_reexports::pax::api::Size)>),
    
    VecLABR__pax_stdCOCOtypesCOCOStackerCellRABR(Vec<pax_example::pax_reexports::pax_std::types::StackerCell>),
    
    Vec_Rc_PropertiesCoproduct___(Vec<Rc<PropertiesCoproduct>>),
    
    __f64(pax_example::pax_reexports::f64),
    
    __i64(pax_example::pax_reexports::i64),
    
    __paxCOCOapiCOCOSize(pax_example::pax_reexports::pax::api::Size),
    
    __pax_stdCOCOtypesCOCOColor(pax_example::pax_reexports::pax_std::types::Color),
    
    __pax_stdCOCOtypesCOCOFont(pax_example::pax_reexports::pax_std::types::Font),
    
    __pax_stdCOCOtypesCOCOStackerDirection(pax_example::pax_reexports::pax_std::types::StackerDirection),
    
    __pax_stdCOCOtypesCOCOStroke(pax_example::pax_reexports::pax_std::types::Stroke),
    
    __stdCOCOstringCOCOString(pax_example::pax_reexports::std::string::String),
    
    bool(bool),
    
    f64(f64),
    
    isize(isize),
    
    usize(usize),
    
}
