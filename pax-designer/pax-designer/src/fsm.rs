use std::ops::Deref;

/// Finite State Machine (FSM) manager for Pax Designer user inputs
/// User inputs like click, mousemove, and keypress are modeled as FSM transitions
/// Actions like click to select, click-and-drag to translate, click-and-drag control point to resize
/// are all modeled into this FSM

#[derive(Default)]
struct FSM {
    current_state: FSMState,
    current_modifiers: ModifierKeys,
    // selection_manager: Rc<RefCell<>>,
}


impl FSM {
    fn set_state(&mut self, new_state: FSMState) {
        let new_state_cloned = new_state.clone();
        self.current_state = new_state;
        self.perform_side_effects(&new_state_cloned);
    }
}

#[derive(Default, Clone)]
pub enum FSMState {
    #[default]
    Idle,
    PanningCamera,
    SelectingWithMarquee,
    ArmedForTranslate,
    DoSelect(), //knowing about selection here presupposes a round trip through the engine / raycaster.
                //should that logic sit here, or should the FSM be a bit dumber, letting the Action layer
                //deal with resolving coordinates, and likewise relying on the Action layer to
                //send back the relevant qualifying information (this was a Selection, not a marquee-start) to the FSM.
                //Does this "route layer" qualify its own events or does it rely on the action layer to send feedback?
                //Further, do we handle mousedown/etc. on individual elements like the control points or a rect laid out over selection state?
                //Or do we completely virtualize / calculate the relationship of mousedown vs. control point locations.
                //Seems like we can simplify by relying on event dispatch on control points, so we can have e.g. a ControlPointMouseDown and AnchorPointMouseDown, which are different from generic mousedowns.
                //Further, we can patch into plain ol' mousemove (at the global glass level) for subsequent state changes, rather than having a special ControlPointMouseMove e.g.
                //We can also offer an invisible (but present-in-render-tree) node for rotation handles, which
    DoTranslate(ScreenspaceVec2),
    ArmedForResize,
    DoResize(usize, ScreenspacePoint, ScreenspaceVec2), //control point index, axis-aligned top-left is 0, incremented clockwise
    ArmedForRotate,
    DoRotate(ScreenspaceVec2)
}
pub enum FSMEvent {
    MouseDownEvent(ScreenspacePoint),
    MouseUpEvent(ScreenspacePoint),
    MouseMoveEvent(ScreenspacePoint, ScreenspaceVec2),
    KeydownEvent(KeyCode)
}

#[derive(Clone)]
pub struct ScreenspacePoint {
    x: f64,
    y: f64,
}

#[derive(Clone)]
pub struct ScreenspaceVec2 {
    dx: f64,
    dy: f64,
}
enum KeyCode {
    Space,
    Shift,
}

#[derive(Default)]
pub struct ModifierKeys {
    shift: bool,
    alt: bool,
    command: bool, //Note: this is ctrl on windows/nix.  the command key on windows/nix is a no-op; the ctrl key on macos is a no-op
}

impl FSM {
    fn new() -> Self {
        Default::default()
    }

    fn transition(&mut self, event: FSMEvent) {
         let new_state = match (&self.current_state, event) {
            // // Define state transitions
            // (State::Idle, Event::KeyPress(KeyCode::Space)) => State::Panning,
            // (State::Panning, Event::MouseClick) => State::Selecting,
            // // ... more transitions
            //
            // Default case if no transition is defined
            (_, _) => None,
        };
        if let Some(new_state) = new_state {
            self.set_state(new_state);
        }
    }

    /// After entering a new state, this method is called to trigger any registered relevant side effects,
    /// possibly elsewhere in the system (such as affected selection state) or within this FSM (e.g. transition immediately to another state after performing a certain side-effect)
    fn perform_side_effects(&mut self, new_state: &FSMState) {
        match new_state {
            FSMState::DoSelect() => {
                todo!("use selection manager to perform selection");
                self.set_state(FSMState::ArmedForTranslate);
            }
            _ => {/*Default: no-op.  Most states don't induce side-effects.*/}
        }


    }
}