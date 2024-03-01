use pax_engine::api::{ArgsButtonClick, ArgsClick, NodeContext, Numeric};
use pax_engine::math::Point2;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::primitives::{Group, Image, Rectangle};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::mpsc::channel;

use super::ToolbarItemView;

#[pax]
#[file("controls/toolbar/toolbar_item.pax")]
pub struct ToolbarItemVisual {
    pub data: Property<ToolbarItemView>,
}

impl ToolbarItemVisual {
    pub fn on_click(&mut self, _ctx: &NodeContext, _args: ArgsClick) {
        super::TOOLBAR_CHANNEL.with_borrow_mut(|store| {
            let data = self.data.get();
            *store = Some(super::ToolbarClickEvent::Select(data.row, data.col));
        });
    }

    pub fn dropdown(&mut self, _ctx: &NodeContext, _args: ArgsClick) {
        super::TOOLBAR_CHANNEL.with_borrow_mut(|store| {
            let data = self.data.get();
            *store = Some(super::ToolbarClickEvent::Dropdown(data.row));
        });
    }
}
