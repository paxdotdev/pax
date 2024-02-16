use pax_engine::api::{
    ArgsClick, ArgsClap, ArgsScroll, ArgsTouchStart, ArgsTouchMove, ArgsTouchEnd,
    ArgsKeyDown, ArgsKeyUp, ArgsKeyPress, ArgsDoubleClick, ArgsMouseMove, ArgsWheel,
    ArgsMouseDown, ArgsMouseUp, ArgsMouseOver, ArgsMouseOut, ArgsContextMenu,
    EngineContext,  Property, PropertyLiteral
};
use pax_engine::Pax;
use pax_std::primitives::{Ellipse, Frame, Group, Path, Rectangle, Text, Image};
use pax_std::types::text::*;

#[pax]
#[main]
#[file("words.pax")]
pub struct Words {
    pub content: Property<String>,
}

impl Words {
    pub fn handle_mount(&mut self, _ctx: &EngineContext) {
    }

    pub fn handle_clap(&mut self, _ctx: &EngineContext, _args: ArgsClap) {
        self.content.set("Clap".to_string());
    }

    pub fn handle_scroll(&mut self, _ctx: &EngineContext, _args: ArgsScroll) {
        self.content.set("Scroll".to_string());
    }

    pub fn handle_touch_start(&mut self, _ctx: &EngineContext, _args: ArgsTouchStart) {
        self.content.set("Touch Start".to_string());
    }

    pub fn handle_touch_move(&mut self, _ctx: &EngineContext, _args: ArgsTouchMove) {
        self.content.set("Touch Move".to_string());
    }

    pub fn handle_touch_end(&mut self, _ctx: &EngineContext, _args: ArgsTouchEnd) {
        self.content.set("Touch End".to_string());
    }

    pub fn handle_key_down(&mut self, _ctx: &EngineContext, _args: ArgsKeyDown) {
        self.content.set("Key Down".to_string());
    }

    pub fn handle_key_up(&mut self, _ctx: &EngineContext, _args: ArgsKeyUp) {
        self.content.set("Key Up".to_string());
    }

    pub fn handle_key_press(&mut self, _ctx: &EngineContext, _args: ArgsKeyPress) {
        self.content.set("Key Press".to_string());
    }

    pub fn handle_click(&mut self, _ctx: &EngineContext, _args: ArgsClick) {
        self.content.set("Click".to_string());
    }

    pub fn handle_double_click(&mut self, _ctx: &EngineContext, _args: ArgsDoubleClick) {
        self.content.set("Double Click".to_string());
    }

    pub fn handle_mouse_move(&mut self, _ctx: &EngineContext, _args: ArgsMouseMove) {
        self.content.set("Mouse Move".to_string());
    }

    pub fn handle_wheel(&mut self, _ctx: &EngineContext, _args: ArgsWheel) {
        self.content.set("Wheel".to_string());
    }

    pub fn handle_mouse_down(&mut self, _ctx: &EngineContext, _args: ArgsMouseDown) {
        self.content.set("Mouse Down".to_string());
    }

    pub fn handle_mouse_up(&mut self, _ctx: &EngineContext, _args: ArgsMouseUp) {
        self.content.set("Mouse Up".to_string());
    }

    pub fn handle_mouse_over(&mut self, _ctx: &EngineContext, _args: ArgsMouseOver) {
        self.content.set("Mouse Over".to_string());
    }

    pub fn handle_mouse_out(&mut self, _ctx: &EngineContext, _args: ArgsMouseOut) {
        self.content.set("Mouse Out".to_string());
    }

    pub fn handle_context_menu(&mut self, _ctx: &EngineContext, _args: ArgsContextMenu){
        self.content.set("Context Menu".to_string());
    }
}