use std::cell::RefCell;
use std::rc::Rc;

pub enum PropertiesCoproduct {
    //core
    None,
    RepeatList(Vec<Rc<RefCell<PropertiesCoproduct>>>),
    RepeatItem(Rc<PropertiesCoproduct>, usize),
    isize(isize), //used by range for repeat (0..10)
    //generated
    StackerCellProperties(pax_example::pax_reexports::pax_std::types::StackerCellProperties),
    Rectangle(pax_example::pax_reexports::pax_std::primitives::Rectangle),
    Text(pax_example::pax_reexports::pax_std::primitives::Text),
    Group(pax_example::pax_reexports::pax_std::primitives::Group),
    Stacker(pax_example::pax_reexports::pax_std::components::Stacker),
    Root(pax_example::Root),

}



//used namely for return types of expressions — may have other purposes
pub enum TypesCoproduct {
    //core (?)
    f64(f64),
    bool(bool),
    isize(isize),
    usize(usize), //used by Slot for index
    Vec_Rc_PropertiesCoproduct___(Vec<Rc<PropertiesCoproduct>>),
    String(String),
    //generated
    Transform2D(pax_example::pax_reexports::Transform2D),
    SizePixels(pax_example::pax_reexports::SizePixels),
    Stroke(pax_example::pax_reexports::pax_std::types::Stroke),
    Color(pax_example::pax_reexports::pax_std::types::Color),
    Size(pax_example::pax_reexports::pax_std::types::Size),
    StackerDirection(pax_example::pax_reexports::pax_std::types::StackerDirection),
    Vec_StackerCellProperties_(Vec<pax_example::pax_reexports::pax_std::types::StackerCellProperties>),
    Vec_LPAREN_usize_COMMA_Size_RPAREN(Vec<(usize, pax_example::pax_reexports::pax_std::types::Size)>),
}




//
// pub enum PatchCoproduct {
//
//     // Rectangle(pax_example::exports::pax_std::primitives::rectangle::Rectangle),
//     // Group(pax_example::exports::pax_std::primitives::group::Group),
//     RootPatch(pax_example::RootPatch),
// }
