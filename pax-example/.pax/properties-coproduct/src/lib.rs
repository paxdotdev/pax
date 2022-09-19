use std::cell::RefCell;
use std::rc::Rc;

pub enum PropertiesCoproduct {
    
    Rectangle(pax_example::pax_reexports::pax_std::primitives::Rectangle),
    
    Text(pax_example::pax_reexports::pax_std::primitives::Text),
    
    Group(pax_example::pax_reexports::pax_std::primitives::Group),
    
    Frame(pax_example::pax_reexports::pax_std::primitives::Frame),
    
    Stacker(pax_example::pax_reexports::pax_std::stacker::Stacker),
    
    Jabberwocky(pax_example::pax_reexports::crate::Jabberwocky),
    
}

//used namely for return types of expressions — may have other purposes
pub enum TypesCoproduct {
    
    crateCOCOtypesCOCOStroke(pax_std::types::Stroke),
    
    crateCOCOtypesCOCOColor(pax_std::types::Color),
    
    String(std::string::String),
    
    crateCOCOtypesCOCOFont(pax_std::types::Font),
    
    crateCOCOtypesCOCOColor(pax_std::types::Color),
    
    VecLABRARcLABRAStackerCellPropertiesRABRARABRA(Vec<Rc<StackerCellProperties>>),
    
    StackerDirection(pax_std::types::StackerDirection),
    
    usize(usize),
    
    Size(pax::api::Size),
    
    VecLABRALPARENusizeCOMMASizeRPARENRABRA(Vec<(usize,Size)>),
    
    VecLABRALPARENusizeCOMMASizeRPARENRABRA(Vec<(usize,Size)>),
    
    i64(i64),
    
    f64(f64),
    
}
