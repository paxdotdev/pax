

pub struct PaxSelectionManager {
    //keep action stack
    //elsewhere in logic, something may register itself as an undoable action (e.g. selection, or manifest ORM)
    //for certain actions, trigger upstream ORM actions (undo an action by ID, or keep track of stack state/status and undo tactically)
    //for other actions (like selection), manage undo/redo separately
}

impl PaxSelectionManager {
    pub fn new() -> Self {
        PaxSelectionManager {}
    }
}