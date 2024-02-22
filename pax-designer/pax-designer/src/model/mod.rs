pub mod action;
pub mod input;
pub mod math;

use crate::math::AxisAlignedBox;
use crate::model::action::ActionContext;
use crate::model::input::RawInput;
use action::Action;
use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use math::coordinate_spaces::World;
use pax_designtime::DesigntimeManager;
use pax_engine::math::{Transform2, Vector2};
use pax_engine::{api::NodeContext, math::Point2, rendering::TransformAndBounds};
use pax_std::types::Color;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use math::coordinate_spaces::Glass;

use self::input::{Dir, InputEvent, InputMapper};
use self::math::coordinate_spaces;

// Needs to be changed if we use a multithreaded async runtime
thread_local!(
    static GLOBAL_STATE: RefCell<GlobalDesignerState> = RefCell::new(GlobalDesignerState::new());
);

#[derive(Default)]
pub struct GlobalDesignerState {
    pub undo_stack: action::UndoStack,
    pub app_state: AppState,
}

impl GlobalDesignerState {
    fn new() -> Self {
        Self {
            app_state: AppState {
                selected_component_id: "pax_designer::pax_reexports::designer_project::Example"
                    .to_owned(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

// Complete app state
// Contains as few invalid states as possible and no derived values
#[derive(Default)]
pub struct AppState {
    //globals
    pub selected_component_id: String,
    pub selected_template_node_id: Option<usize>,
    pub glass_to_world_transform: Transform2<Glass, World>,
    pub mouse_position: Point2<Glass>,

    //toolbar
    pub selected_tool: Tool,

    //glass
    pub tool_state: ToolState,

    //keyboard
    pub keys_pressed: HashSet<InputEvent>,

    //settings
    pub input_mapper: InputMapper,
}

pub fn read_app_state(closure: impl FnOnce(&AppState)) {
    GLOBAL_STATE.with_borrow_mut(|model| {
        closure(&model.app_state);
    });
}

pub fn with_action_context<R: 'static>(
    ctx: &NodeContext,
    func: impl FnOnce(&mut ActionContext) -> R,
) -> R {
    GLOBAL_STATE.with_borrow_mut(|model| {
        let GlobalDesignerState {
            ref mut undo_stack,
            ref mut app_state,
            ..
        } = *model;
        func(&mut ActionContext {
            undo_stack,
            engine_context: ctx,
            app_state,
        })
    })
}

// This represents values that can be deterministically produced from the app
// state and the projects manifest
pub struct DerivedAppState {
    pub selected_bounds: Option<AxisAlignedBox>,
}

pub fn read_app_state_with_derived(
    ctx: &NodeContext,
    closure: impl FnOnce(&AppState, &DerivedAppState),
) {
    let selected_bounds = with_action_context(ctx, |ac| ac.selected_bounds());
    GLOBAL_STATE.with_borrow_mut(|model| {
        closure(&model.app_state, &DerivedAppState { selected_bounds });
    });
}

pub fn perform_action(action: impl Action, ctx: &NodeContext) -> Result<()> {
    with_action_context(ctx, |ac| ac.execute(action))
}

pub fn process_keyboard_input(ctx: &NodeContext, dir: Dir, input: String) -> anyhow::Result<()> {
    // useful! keeping around for now
    // pax_engine::log::debug!("key {:?}: {}", dir, input);
    let action =
        GLOBAL_STATE.with_borrow_mut(|model| -> anyhow::Result<Option<Box<dyn Action>>> {
            let raw_input = RawInput::try_from(input)?;
            let AppState {
                ref mut input_mapper,
                ref mut keys_pressed,
                ..
            } = &mut model.app_state;

            let event = input_mapper
                .to_event(raw_input)
                .with_context(|| "no mapped input")?;
            match dir {
                Dir::Down => keys_pressed.insert(event.clone()),
                Dir::Up => keys_pressed.remove(event),
            };
            let action = input_mapper.to_action(event, dir);
            Ok(action)
        })?;
    if let Some(action) = action {
        perform_action(action, ctx)
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tool {
    #[default]
    Pointer,
    Rectangle,
}

#[derive(Clone, Default)]
pub enum ToolState {
    #[default]
    Idle,
    MovingControlPoint {
        move_func: Rc<dyn Fn(&mut ActionContext, Point2<Glass>)>,
    },
    Panning {
        original_transform: Transform2<Glass, World>,
        glass_start: Point2<Glass>,
    },
    MovingObject {
        offset: Vector2<Glass>,
    },
    BoxSelect {
        p1: Point2<Glass>,
        p2: Point2<Glass>,
        fill: Color,
        stroke: Color,
    },
}

#[derive(Clone)]
pub enum ControlPointPos {
    First,
    Middle,
    Last,
}
