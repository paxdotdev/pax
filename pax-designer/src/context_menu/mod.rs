use pax_engine::pax_manifest::TypeId;
use pax_engine::{api::*, math::Point2, *};
use pax_std::*;
use std::sync::Mutex;

use crate::math::coordinate_spaces::Glass;
use crate::model;
use crate::model::action::orm::group_ungroup::{GroupSelected, UngroupSelected};
use crate::model::action::orm::SelectedIntoNewComponent;
use crate::model::action::{Action, ActionContext};

#[pax]
#[engine_import_prefix("pax_engine")]
#[file("context_menu/mod.pax")]
pub struct DesignerContextMenu {
    pub visible: Property<bool>,
    pub pos_x: Property<f64>,
    pub pos_y: Property<f64>,
}

impl DesignerContextMenu {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        self.bind_visibility();
        self.bind_position();
    }

    fn bind_visibility(&mut self) {
        let msg_src = CONTEXT_MENU_PROP.with(|ctx_menu_msg| ctx_menu_msg.clone());
        let deps = [msg_src.untyped()];
        self.visible.replace_with(Property::computed(
            move || matches!(msg_src.get(), ContextMenuMsg::Open { .. }),
            &deps,
        ));
    }

    fn bind_position(&mut self) {
        let msg_src = CONTEXT_MENU_PROP.with(|ctx_menu_msg| ctx_menu_msg.clone());
        let deps = [msg_src.untyped()];

        let msg = msg_src.clone();
        self.pos_x.replace_with(Property::computed(
            move || {
                if let ContextMenuMsg::Open { pos } = msg.get() {
                    pos.x
                } else {
                    0.0
                }
            },
            &deps,
        ));

        let msg = msg_src.clone();
        self.pos_y.replace_with(Property::computed(
            move || {
                if let ContextMenuMsg::Open { pos } = msg.get() {
                    pos.y
                } else {
                    0.0
                }
            },
            &deps,
        ));
    }

    pub fn create_component(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        model::perform_action(&SelectedIntoNewComponent {}, ctx);
        self.close_menu();
    }

    pub fn group(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        model::perform_action(
            &GroupSelected {
                new_parent_type_id: &TypeId::build_singleton("pax_std::core::group::Group", None),
            },
            ctx,
        );
        self.close_menu();
    }

    pub fn group_link(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        model::perform_action(
            &GroupSelected {
                new_parent_type_id: &TypeId::build_singleton("pax_std::core::link::Link", None),
            },
            ctx,
        );
        self.close_menu();
    }

    pub fn ungroup(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        model::perform_action(&UngroupSelected {}, ctx);
        self.close_menu();
    }

    fn close_menu(&mut self) {
        self.visible.set(false);
    }
}

thread_local! {
    static CONTEXT_MENU_PROP: Property<ContextMenuMsg> = Property::new(ContextMenuMsg::Close);
}

#[derive(Clone, Default)]
pub enum ContextMenuMsg {
    Open {
        pos: Point2<Glass>,
    },
    #[default]
    Close,
}

impl Interpolatable for ContextMenuMsg {}

impl Action for ContextMenuMsg {
    fn perform(&self, _ctx: &mut ActionContext) -> anyhow::Result<()> {
        CONTEXT_MENU_PROP.with(|context_menu_msg| context_menu_msg.set(self.clone()));
        Ok(())
    }
}
