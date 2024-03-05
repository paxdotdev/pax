use std::sync::Mutex;

use pax_engine::api::*;
use pax_engine::math::Point2;
use pax_engine::*;

use crate::model;
use crate::model::action::orm::SelectedIntoNewComponent;
use crate::model::action::{Action, ActionContext, CanUndo};
use crate::model::math::coordinate_spaces::Glass;
use pax_std::primitives::*;

pub enum ContextMenuMessage {
    Open { pos: Point2<Glass> },
    Close,
}

// Need to disable browser context menu
// Might as well also:
// - impl Eq on MousePosition
// - Disable text highlighting

impl Action for ContextMenuMessage {
    fn perform(self: Box<Self>, _ctx: &mut ActionContext) -> anyhow::Result<CanUndo> {
        *CONTEXT_MENU_CHANNEL.lock().unwrap() = Some(*self);
        Ok(CanUndo::No)
    }
}

static CONTEXT_MENU_CHANNEL: Mutex<Option<ContextMenuMessage>> = Mutex::new(None);

#[pax]
#[file("context_menu/mod.pax")]
pub struct ContextMenu {
    pub visible: Property<bool>,
    pub pos_x: Property<f64>,
    pub pos_y: Property<f64>,
}

impl ContextMenu {
    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        if let Some(message) = CONTEXT_MENU_CHANNEL.lock().unwrap().take() {
            match message {
                ContextMenuMessage::Open { pos } => {
                    self.visible.set(true);
                    self.pos_x.set(pos.x);
                    self.pos_y.set(pos.y);
                }
                ContextMenuMessage::Close => {
                    self.visible.set(false);
                }
            }
        }
    }

    pub fn create_component(&mut self, ctx: &NodeContext, _args: ArgsClick) {
        model::perform_action(SelectedIntoNewComponent {}, ctx);
        self.visible.set(false);
    }
}
