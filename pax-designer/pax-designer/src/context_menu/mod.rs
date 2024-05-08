use std::sync::Mutex;

use pax_engine::api::*;
use pax_engine::math::Point2;
use pax_engine::*;

use crate::math::coordinate_spaces::Glass;
use crate::model;
use crate::model::action::orm::SelectedIntoNewComponent;
use crate::model::action::{Action, ActionContext, CanUndo};
use pax_std::primitives::*;

impl Interpolatable for ContextMenuMessage {}

#[derive(Clone, Default)]
pub enum ContextMenuMessage {
    Open {
        pos: Point2<Glass>,
    },
    #[default]
    Close,
}

// Need to disable browser context menu
// Might as well also:
// - impl Eq on MousePosition
// - Disable text highlighting

impl Action for ContextMenuMessage {
    fn perform(self: Box<Self>, _ctx: &mut ActionContext) -> anyhow::Result<CanUndo> {
        CONTEXT_MENU_PROP.with(|context_menu_msg| context_menu_msg.set(*self));
        Ok(CanUndo::No)
    }
}

thread_local! {
    static CONTEXT_MENU_PROP: Property<ContextMenuMessage> = Property::new(ContextMenuMessage::Close);
}

#[pax]
#[file("context_menu/mod.pax")]
pub struct DesignerContextMenu {
    pub visible: Property<bool>,
    pub pos_x: Property<f64>,
    pub pos_y: Property<f64>,
}

impl DesignerContextMenu {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        CONTEXT_MENU_PROP.with(|context_menu_msg| {
            let msg = context_menu_msg.clone();
            let deps = [msg.untyped()];
            self.visible.replace_with(Property::computed(
                move || match msg.get() {
                    ContextMenuMessage::Open { .. } => true,
                    ContextMenuMessage::Close => false,
                },
                &deps,
            ));
            let msg = context_menu_msg.clone();
            self.pos_x.replace_with(Property::computed(
                move || match msg.get() {
                    ContextMenuMessage::Open { pos } => pos.x,
                    ContextMenuMessage::Close => 0.0,
                },
                &deps,
            ));
            let msg = context_menu_msg.clone();
            self.pos_y.replace_with(Property::computed(
                move || match msg.get() {
                    ContextMenuMessage::Open { pos } => pos.y,
                    ContextMenuMessage::Close => 0.0,
                },
                &deps,
            ));
        });
    }

    pub fn create_component(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        model::perform_action(SelectedIntoNewComponent {}, ctx);
        self.visible.set(false);
    }
}
