use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

pub enum PropertiesCoproduct {
    //core
    None,
    RepeatList(Vec<Rc<RefCell<PropertiesCoproduct>>>),
    RepeatItem(Rc<PropertiesCoproduct>, usize),
    #[allow(non_camel_case_types)]
    usize(usize),//used by Repeat + numeric ranges, e.g. `for i in 0..5`
    #[allow(non_camel_case_types)]
    isize(isize),//used by Repeat + numeric ranges, e.g. `for i in 0..5`

    //generated
}

//used namely for return types of expressions — may have other purposes

pub enum TypesCoproduct {
    //core: primitives
    #[allow(non_camel_case_types)]
    f64(f64),
    #[allow(non_camel_case_types)]
    bool(bool),
    #[allow(non_camel_case_types)]
    isize(isize),
    #[allow(non_camel_case_types)]
    usize(usize), //used by Slot for index

    #[allow(non_camel_case_types)]
    stdCOCOvecCOCOVecLABRstdCOCOrcCOCORcLABRPropertiesCoproductRABRRABR(Vec<Rc<PropertiesCoproduct>>),
    #[allow(non_camel_case_types)]
    stdCOCOopsCOCORangeLABRisizeRABR(Range<isize>),
    String(String),
    Transform2D(pax_runtime_api::Transform2D),
    SizePixels(pax_runtime_api::SizePixels),
    Size(pax_runtime_api::Size),
    Numeric(pax_runtime_api::Numeric),
    //generated / userland
}


//
// pub enum PatchCoproduct {
//
//     // Rectangle(pax_example::exports::pax_std::primitives::rectangle::Rectangle),
//     // Group(pax_example::exports::pax_std::primitives::group::Group),
//     RootPatch(pax_example::RootPatch),
// }
