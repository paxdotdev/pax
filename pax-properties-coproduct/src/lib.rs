use std::cell::RefCell;
use std::rc::Rc;

pub enum PropertiesCoproduct {
    //core
    None,
    RepeatList(Vec<Rc<RefCell<PropertiesCoproduct>>>),
    RepeatItem(Rc<PropertiesCoproduct>, usize),
    usize(usize),//used by Repeat + numeric ranges, e.g. `for i in 0..5`

    //generated
}

//used namely for return types of expressions — may have other purposes
pub enum TypesCoproduct {
    //core: primitives
    f64(f64),
    bool(bool),
    isize(isize),
    usize(usize), //used by Slot for index

    Vec_Rc_PropertiesCoproduct___(Vec<Rc<PropertiesCoproduct>>),
    String(String),
    Transform2D(pax_runtime_api::Transform2D),
    SizePixels(pax_runtime_api::SizePixels),
    Size(pax_runtime_api::Size),
    //generated / userland
}


//
// pub enum PatchCoproduct {
//
//     // Rectangle(pax_example::exports::pax_std::primitives::rectangle::Rectangle),
//     // Group(pax_example::exports::pax_std::primitives::group::Group),
//     RootPatch(pax_example::RootPatch),
// }
