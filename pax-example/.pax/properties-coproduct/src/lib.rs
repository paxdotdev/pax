use std::cell::RefCell;
use std::rc::Rc;

pub enum PropertiesCoproduct {
    None,
    RepeatList(Vec<Rc<RefCell<PropertiesCoproduct>>>),
    RepeatItem(Rc<PropertiesCoproduct>, usize),
    
    Text(pax_example::pax_reexports::pax_std::primitives::Text),
    
    Frame(pax_example::pax_reexports::pax_std::primitives::Frame),
    
    Stacker(pax_example::pax_reexports::pax_std::stacker::Stacker),
    
    Rectangle(pax_example::pax_reexports::pax_std::primitives::Rectangle),
    
    Group(pax_example::pax_reexports::pax_std::primitives::Group),
    
    Jabberwocky(pax_example::pax_reexports::Jabberwocky),
    
}

//used namely for return types of expressions — may have other purposes
pub enum TypesCoproduct {
    
    VecLABRLPAR__usizeCOMM__paxCOCOapiCOCOSizeRPARRABR(Vec<(pax_example::pax_reexports::usize,pax_example::pax_reexports::pax::api::Size)>),
    
    VecLABRRcLABR__pax_stdCOCOtypesCOCOStackerCellPropertiesRABRRABR(Vec<Rc<pax_example::pax_reexports::pax_std::types::StackerCellProperties>>),
    
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
    
}
