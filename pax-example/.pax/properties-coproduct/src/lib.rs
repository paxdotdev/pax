use std::cell::RefCell;
use std::rc::Rc;

pub enum PropertiesCoproduct {
    /* entries generated via properties-coproduct-lib.tera */
    None,
    RepeatList(Vec<Rc<RefCell<PropertiesCoproduct>>>),
    RepeatItem(Rc<PropertiesCoproduct>, usize),
    isize(isize),
    Range_isize_(std::ops::Range<isize>),

    
    Ellipse(pax_example::pax_reexports::pax_std::primitives::Ellipse),
    
    Group(pax_example::pax_reexports::pax_std::primitives::Group),
    
    HelloRGB(pax_example::pax_reexports::HelloRGB),
    
    Path(pax_example::pax_reexports::pax_std::primitives::Path),
    
    Rectangle(pax_example::pax_reexports::pax_std::primitives::Rectangle),
    
    Text(pax_example::pax_reexports::pax_std::primitives::Text),
    
}

//The following two conversions are used by Repeat to use an Rc<PropertiesCoproduct>
//opaquely in RIL in combination with numerics, using `.into()` on the instance
//of the Rc<PropertiesCoproduct>
//impl From<std::rc::Rc<PropertiesCoproduct>> for PropertiesCoproduct {
//    fn from(rc: Rc<PropertiesCoproduct>) -> Self {
//        (*rc).clone()
//    }
//}
//impl From<PropertiesCoproduct> for pax_runtime_api::numeric::Numeric {
//    fn from(pc: PropertiesCoproduct) -> Self {
//        if let PropertiesCoproduct::isize(i) = pc {
//            pax_runtime_api::numeric::Numeric::from(i) //special handling of `isize`, for use with Repeat
//        } else {
//            unreachable!()
//        }
//    }
//}

//used namely for return types of expressions — may have other purposes
pub enum TypesCoproduct {
    
    Range_isize_(std::ops::Range<isize>),
    
    Size(pax_runtime_api::Size),
    
    Size2D(pax_runtime_api::Size2D),
    
    SizePixels(pax_runtime_api::SizePixels),
    
    String(String),
    
    Transform2D(pax_runtime_api::Transform2D),
    
    VecLABR__pax_stdCOCOtypesCOCOPathSegmentRABR(Vec<pax_example::pax_reexports::pax_std::types::PathSegment>),
    
    Vec_Rc_PropertiesCoproduct___(std::vec::Vec<std::rc::Rc<PropertiesCoproduct>>),
    
    __f64(pax_example::pax_reexports::f64),
    
    __pax_stdCOCOtypesCOCOColor(pax_example::pax_reexports::pax_std::types::Color),
    
    __pax_stdCOCOtypesCOCOFont(pax_example::pax_reexports::pax_std::types::Font),
    
    __pax_stdCOCOtypesCOCOStroke(pax_example::pax_reexports::pax_std::types::Stroke),
    
    __stdCOCOstringCOCOString(pax_example::pax_reexports::std::string::String),
    
    bool(bool),
    
    f64(f64),
    
    isize(isize),
    
    usize(usize),
    
}
