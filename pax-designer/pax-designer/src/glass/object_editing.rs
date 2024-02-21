use std::cell::RefCell;

use crate::model::action::ActionContext;

use super::EditVisual;

struct GenericObjectEditor {
    object: ()
    control_point_actions: Vec<Box<dyn Fn(&mut ActionContext)>>,
}

thread_local!(
    static GENERIC_OBJ_EDITOR: RefCell<GenericObjectEditor> =
        RefCell::new(GenericObjectEditor::new());
);


trait ComponentEditor {
    fn get_visual() ->  EditVisual
}



impl GenericObjectEditor {
    fn new() {
        Self {
            selection_visual:
        }
    }
}
