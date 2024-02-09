use anyhow::{anyhow, Result};

use super::action::{ActionManager, CreateRectangle};
use pax_designtime::DesigntimeManager;

/// Finite State Machine (FSM) manager for Pax Designer user inputs
/// User inputs like click, mousemove, and keypress are modeled as FSM transitions
/// Actions like click to select, click-and-drag to translate, click-and-drag control point to resize
/// are all modeled into this FSM

#[derive(Default)]
pub struct InputManager {
    current_state: FSMState,
    current_modifiers: ModifierKeys,
}

impl InputManager {
    fn set_fsm_state(&mut self, new_state: FSMState) {
        let new_state_cloned = new_state.clone();
        self.current_state = new_state;
        self.perform_side_effects(&new_state_cloned);
    }
}

#[derive(Clone)]
pub enum Tool {
    RectangleArmed,
    RectangleState(ScreenspacePoint, ScreenspacePoint),
}

#[derive(Clone, Copy, Debug)]
pub struct ScreenspacePoint {
    pub x: f64,
    pub y: f64,
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
    command: bool, //Note: this is ctrl on windows/nix.  the `windows (host)` key on windows/nix is a no-op; the `ctrl` key on macos is a no-op
}

impl InputManager {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn transition(&mut self, event: FSMEvent) -> Result<String> {
        match (&self.current_state, event) {
            (_, FSMEvent::InterfaceEvent(Interface::ActivatePointerTool)) => {
                self.set_fsm_state(FSMState::Idle);
                return Ok("pointer tool activated".to_owned());
            }
            (_, FSMEvent::InterfaceEvent(Interface::ActivateRectangleTool)) => {
                self.set_fsm_state(FSMState::Tool(Tool::RectangleArmed));
                return Ok("rectangle tool activated".to_owned());
            }
            (FSMState::Tool(Tool::RectangleArmed), FSMEvent::MouseDown(point)) => {
                self.set_fsm_state(FSMState::Tool(Tool::RectangleState(point, point)));
            }
            (FSMState::Tool(Tool::RectangleState(p1, _)), FSMEvent::MouseMove(point)) => {
                self.set_fsm_state(FSMState::Tool(Tool::RectangleState(*p1, point)));
            }
            (FSMState::Tool(Tool::RectangleState(p1, _)), FSMEvent::MouseUp(p2)) => {
                self.set_fsm_state(FSMState::Idle);
                return action_manager
                    .perform(Box::new(CreateRectangle {}), designtime)
                    .map(|_| "rect created".to_owned());
            }
            // // Define state transitions
            // (State::Idle, Event::KeyPress(KeyCode::Space)) => State::Panning,
            // (State::Panning, Event::MouseClick) => State::Selecting,
            // // ... more transitions
            //
            // Default case if no transition is defined
            (_, _) => (),
        };
        Ok("no-op".to_owned())
    }

    /// After entering a new state, this method is called to trigger any registered relevant side effects,
    /// possibly elsewhere in the system (such as affected selection state) or within this FSM (e.g. transition immediately to another state after performing a certain side-effect)
    fn perform_side_effects(&mut self, new_state: &FSMState) {
        match new_state {
            FSMState::DoSelect() => {
                todo!("use selection manager to perform selection");
                self.set_fsm_state(FSMState::ArmedForTranslate);
            }
            _ => { /*Default: no-op.  Most states don't induce side-effects.*/ }
        }
    }
}
