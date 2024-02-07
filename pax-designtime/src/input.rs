use anyhow::{anyhow, Result};
use std::ops::Deref;

use crate::action::ActionManager;

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

#[derive(Default, Clone)]
pub enum FSMState {
    #[default]
    Idle,
    PanningCamera,
    SelectingWithMarquee,
    ArmedForTranslate,
    DoSelect(),
    DoTranslate(ScreenspaceVec2),
    ArmedForResize,
    DoResize(usize, ScreenspacePoint, ScreenspaceVec2), //control point index, axis-aligned top-left is 0, incremented clockwise
    ArmedForRotate,
    DoRotate(ScreenspaceVec2),
    Tool(Tool),
}

#[derive(Clone)]
pub enum Tool {
    RectangleArmed,
    RectangleState(ScreenspacePoint, ScreenspacePoint),
}

pub enum FSMEvent {
    MouseDown(ScreenspacePoint),
    MouseUp(ScreenspacePoint),
    MouseMove(ScreenspacePoint),
    InterfaceEvent(Interface),
    KeydownEvent(KeyCode),
    ControlPointMouseDown(usize),
}

pub enum Interface {
    ActivateRectangleTool,
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

    pub fn transition(
        &mut self,
        event: FSMEvent,
        action_manager: &mut ActionManager,
    ) -> Result<()> {
        let new_state = match (&self.current_state, event) {
            (_, FSMEvent::InterfaceEvent(Interface::ActivateRectangleTool)) => {
                Some(FSMState::Tool(Tool::RectangleArmed))
            }
            (FSMState::Tool(Tool::RectangleArmed), FSMEvent::MouseDown(point)) => {
                Some(FSMState::Tool(Tool::RectangleState(point, point)))
            }
            (FSMState::Tool(Tool::RectangleState(p1, _)), FSMEvent::MouseMove(point)) => {
                Some(FSMState::Tool(Tool::RectangleState(*p1, point)))
            }
            (FSMState::Tool(Tool::RectangleState(p1, _)), FSMEvent::MouseUp(p2)) => {
                action_manager.perform()
                Some(FSMState::Idle)
            }
            // // Define state transitions
            // (State::Idle, Event::KeyPress(KeyCode::Space)) => State::Panning,
            // (State::Panning, Event::MouseClick) => State::Selecting,
            // // ... more transitions
            //
            // Default case if no transition is defined
            (_, _) => None,
        };
        if let Some(new_state) = new_state {
            self.set_fsm_state(new_state);
        }
        Ok(())
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
