use std::cell::RefCell;
use std::rc::Rc;

pub enum PropertiesCoproduct {
    
    Frame(pax_example::pax_reexports::pax_std::primitives::Frame),
    
    Stacker(pax_example::pax_reexports::pax_std::stacker::Stacker),
    
    Group(pax_example::pax_reexports::pax_std::primitives::Group),
    
    Text(pax_example::pax_reexports::pax_std::primitives::Text),
    
    Rectangle(pax_example::pax_reexports::pax_std::primitives::Rectangle),
    
    Jabberwocky(pax_example::pax_reexports::crate::Jabberwocky),
    
}

//used namely for return types of expressions — may have other purposes
pub enum TypesCoproduct {
    
    VecLABRLPARusizeCOMMpaxCOCOapiCOCOSizeRPARRABR(Vec<(usize,pax::api::Size)>),
    
    VecLABRRcLABRpax_stdCOCOtypesCOCOStackerCellPropertiesRABRRABR(Vec<Rc<pax_std::types::StackerCellProperties>>),
    
    f64(f64),
    
    i64(i64),
    
    paxCOCOapiCOCOSize(pax::api::Size),
    
    pax_stdCOCOtypesCOCOColor(pax_std::types::Color),
    
    pax_stdCOCOtypesCOCOFont(pax_std::types::Font),
    
    pax_stdCOCOtypesCOCOStackerDirection(pax_std::types::StackerDirection),
    
    pax_stdCOCOtypesCOCOStroke(pax_std::types::Stroke),
    
    stdCOCOstringCOCOString(std::string::String),
    
    usize(usize),
    
}
