use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

// IMPORTANT NOTE:
// This file is never used in builds or at runtime.  The purpose of this file is to appease static
// builds and build tools like IDEs, so they can operate on a static Rust codebase without issues.
// This file is a placeholder, replaced with generated code for any pax build.
// Changes made to this file will not be reflected anywhere.
//
// ****
// To make changes to this file, see pax-compiler/templates/properties-coproduct-lib.tera
// ****

#[derive(Default)]
pub enum PropertiesCoproduct {
    //core
    #[default]
    None,
    Repeat(pax_runtime_api::RepeatProperties),
    Slot(pax_runtime_api::SlotProperties),
    Conditional(pax_runtime_api::ConditionalProperties),
    RepeatItem(Rc<PropertiesCoproduct>, usize),
    #[allow(non_camel_case_types)]
    usize(usize),//used by Repeat + numeric ranges, e.g. `for i in 0..5`
    #[allow(non_camel_case_types)]
    isize(isize),//used by Repeat + numeric ranges, e.g. `for i in 0..5`

}

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
    Rotation(pax_runtime_api::Rotation),
    Numeric(pax_runtime_api::Numeric),
    StringBox(pax_runtime_api::StringBox)
    //generated / userland
}

///Contains modal _vec_ and _range_ variants, describing whether the Repeat source
///is encoded as a Vec<T> (where T is a PropertiesCoproduct type) or as a Range<isize>
pub struct RepeatProperties {
    pub repeat_source_expression_vec: Option<Box<dyn PropertyInstance<Vec<Rc<PropertiesCoproduct>>>>>,
    pub repeat_source_expression_range: Option<Box<dyn PropertyInstance<std::ops::Range<isize>>>>,
}

pub struct SlotProperties {
    pub index: Box<dyn PropertyInstance<pax_runtime_api::Numeric>>,
}

pub struct ConditionalProperties {
    pub boolean_expression: Box<dyn PropertyInstance<bool>>,
}
