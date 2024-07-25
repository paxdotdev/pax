use pax_engine::api::{ButtonClick, Click, Event, NodeContext, Numeric};
use pax_engine::math::Point2;
use pax_engine::*;
use pax_std::*;
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
    pub fn on_click(&mut self, _ctx: &NodeContext, _args: Event<Click>) {
        super::CLICK_PROP.with(|click_msg| {
            let data = self.data.get();
            click_msg.set(super::ToolbarClickEvent::Select(data.row, data.col));
        });
    }

    pub fn dropdown(&mut self, _ctx: &NodeContext, _args: Event<Click>) {
        super::CLICK_PROP.with(|click_msg| {
            let data = self.data.get();
            click_msg.set(super::ToolbarClickEvent::Dropdown(data.row));
        });
    }
}
