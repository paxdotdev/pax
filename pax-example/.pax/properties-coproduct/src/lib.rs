use std::cell::RefCell;
use std::rc::Rc;

pub enum PropertiesCoproduct {
    
    Group(pax_example::pax_reexports::pax_std::primitives::Group),
    
    Frame(pax_example::pax_reexports::pax_std::primitives::Frame),
    
    Stacker(pax_example::pax_reexports::pax_std::stacker::Stacker),
    
    Rectangle(pax_example::pax_reexports::pax_std::primitives::Rectangle),
    
    Text(pax_example::pax_reexports::pax_std::primitives::Text),
    
    Jabberwocky(pax_example::pax_reexports::crate::Jabberwocky),
    
}

//used namely for return types of expressions — may have other purposes
pub enum TypesCoproduct {
    
    VecLABRRcLABRStackerCellPropertiesRABRRABR(Vec<Rc<StackerCellProperties>>),
    
    pax_stdCOCOtypesCOCOStackerDirection(pax_std::types::StackerDirection),
    
    usize(usize),
    
    paxCOCOapiCOCOSize(pax::api::Size),
    
    VecLABRLPARusizeCOMMSizeRPARRABR(Vec<(usize,Size)>),
    
    VecLABRLPARusizeCOMMSizeRPARRABR(Vec<(usize,Size)>),
    
    pax_stdCOCOtypesCOCOStroke(pax_std::types::Stroke),
    
    pax_stdCOCOtypesCOCOColor(pax_std::types::Color),
    
    stdCOCOstringCOCOString(std::string::String),
    
    pax_stdCOCOtypesCOCOFont(pax_std::types::Font),
    
    pax_stdCOCOtypesCOCOColor(pax_std::types::Color),
    
    i64(i64),
    
    f64(f64),
    
}
