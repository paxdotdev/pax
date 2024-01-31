use pax_lang::api::{
    ArgsClick, ArgsClap, ArgsScroll, ArgsTouchStart, ArgsTouchMove, ArgsTouchEnd,
    ArgsKeyDown, ArgsKeyUp, ArgsKeyPress, ArgsDoubleClick, ArgsMouseMove, ArgsWheel,
    ArgsMouseDown, ArgsMouseUp, ArgsMouseOver, ArgsMouseOut, ArgsContextMenu,
    NodeContext,  Property, PropertyLiteral
};
use pax_lang::Pax;
use pax_std::primitives::{Ellipse, Frame, Group, Path, Rectangle, Text, Image};
use pax_std::types::text::*;

#[derive(Pax)]
#[main]
#[file("words.pax")]
pub struct Words {
    pub content: Property<String>,
}

impl Words {
    pub fn handle_mount(&mut self, _ctx: &NodeContext) {
    }

    pub fn handle_clap(&mut self, _ctx: &NodeContext, _args: ArgsClap) {
        self.content.set("Clap".to_string());
    }

    pub fn handle_scroll(&mut self, _ctx: &NodeContext, _args: ArgsScroll) {
        self.content.set("Scroll".to_string());
    }

    pub fn handle_touch_start(&mut self, _ctx: &NodeContext, _args: ArgsTouchStart) {
        self.content.set("Touch Start".to_string());
    }

    pub fn handle_touch_move(&mut self, _ctx: &NodeContext, _args: ArgsTouchMove) {
        self.content.set("Touch Move".to_string());
    }

    pub fn handle_touch_end(&mut self, _ctx: &NodeContext, _args: ArgsTouchEnd) {
        self.content.set("Touch End".to_string());
    }

    pub fn handle_key_down(&mut self, _ctx: &NodeContext, _args: ArgsKeyDown) {
        self.content.set("Key Down".to_string());
    }

    pub fn handle_key_up(&mut self, _ctx: &NodeContext, _args: ArgsKeyUp) {
        self.content.set("Key Up".to_string());
    }

    pub fn handle_key_press(&mut self, _ctx: &NodeContext, _args: ArgsKeyPress) {
        self.content.set("Key Press".to_string());
    }

    pub fn handle_click(&mut self, _ctx: &NodeContext, _args: ArgsClick) {
        self.content.set("Click".to_string());
    }

    pub fn handle_double_click(&mut self, _ctx: &NodeContext, _args: ArgsDoubleClick) {
        self.content.set("Double Click".to_string());
    }

    pub fn handle_mouse_move(&mut self, _ctx: &NodeContext, _args: ArgsMouseMove) {
        self.content.set("Mouse Move".to_string());
    }

    pub fn handle_wheel(&mut self, _ctx: &NodeContext, _args: ArgsWheel) {
        self.content.set("Wheel".to_string());
    }

    pub fn handle_mouse_down(&mut self, _ctx: &NodeContext, _args: ArgsMouseDown) {
        self.content.set("Mouse Down".to_string());
    }

    pub fn handle_mouse_up(&mut self, _ctx: &NodeContext, _args: ArgsMouseUp) {
        self.content.set("Mouse Up".to_string());
    }

    pub fn handle_mouse_over(&mut self, _ctx: &NodeContext, _args: ArgsMouseOver) {
        self.content.set("Mouse Over".to_string());
    }

    pub fn handle_mouse_out(&mut self, _ctx: &NodeContext, _args: ArgsMouseOut) {
        self.content.set("Mouse Out".to_string());
    }

    pub fn handle_context_menu(&mut self, _ctx: &NodeContext, _args: ArgsContextMenu){
        self.content.set("Context Menu".to_string());
    }
}