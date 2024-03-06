use pax_engine::api::{
    ArgsClap, ArgsClick, ArgsContextMenu, ArgsDoubleClick, ArgsKeyDown, ArgsKeyPress, ArgsKeyUp,
    ArgsMouseDown, ArgsMouseMove, ArgsMouseOut, ArgsMouseOver, ArgsMouseUp, ArgsScroll,
    ArgsTouchEnd, ArgsTouchMove, ArgsTouchStart, ArgsWheel, NodeContext, Property, PropertyLiteral,
};
use pax_engine::Pax;
use pax_std::primitives::{Ellipse, Frame, Group, Image, Path, Rectangle, Text};
use pax_std::types::text::*;

#[pax]
#[main]
#[file("words.pax")]
pub struct Words {
    pub content: Property<String>,
}

impl Words {
    pub fn handle_mount(&mut self, _ctx: &NodeContext) {}

    pub fn handle_clap(&mut self, _ctx: &NodeContext, _args: Event<Clap>) {
        self.content.set("Clap".to_string());
    }

    pub fn handle_scroll(&mut self, _ctx: &NodeContext, _args: Event<Scroll>) {
        self.content.set("Scroll".to_string());
    }

    pub fn handle_touch_start(&mut self, _ctx: &NodeContext, _args: Event<TouchStart>) {
        self.content.set("Touch Start".to_string());
    }

    pub fn handle_touch_move(&mut self, _ctx: &NodeContext, _args: Event<TouchMove>) {
        self.content.set("Touch Move".to_string());
    }

    pub fn handle_touch_end(&mut self, _ctx: &NodeContext, _args: Event<TouchEnd>) {
        self.content.set("Touch End".to_string());
    }

    pub fn handle_key_down(&mut self, _ctx: &NodeContext, _args: Event<KeyDown>) {
        self.content.set("Key Down".to_string());
    }

    pub fn handle_key_up(&mut self, _ctx: &NodeContext, _args: Event<KeyUp>) {
        self.content.set("Key Up".to_string());
    }

    pub fn handle_key_press(&mut self, _ctx: &NodeContext, _args: Event<KeyPress>) {
        self.content.set("Key Press".to_string());
    }

    pub fn handle_click(&mut self, _ctx: &NodeContext, _args: Event<Click>) {
        self.content.set("Click".to_string());
    }

    pub fn handle_double_click(&mut self, _ctx: &NodeContext, _args: Event<DoubleClick>) {
        self.content.set("Double Click".to_string());
    }

    pub fn handle_mouse_move(&mut self, _ctx: &NodeContext, _args: Event<MouseMove>) {
        self.content.set("Mouse Move".to_string());
    }

    pub fn handle_wheel(&mut self, _ctx: &NodeContext, _args: Event<Wheel>) {
        self.content.set("Wheel".to_string());
    }

    pub fn handle_mouse_down(&mut self, _ctx: &NodeContext, _args: Event<MouseDown>) {
        self.content.set("Mouse Down".to_string());
    }

    pub fn handle_mouse_up(&mut self, _ctx: &NodeContext, _args: Event<MouseUp>) {
        self.content.set("Mouse Up".to_string());
    }

    pub fn handle_mouse_over(&mut self, _ctx: &NodeContext, _args: Event<MouseOver>) {
        self.content.set("Mouse Over".to_string());
    }

    pub fn handle_mouse_out(&mut self, _ctx: &NodeContext, _args: Event<MouseOut>) {
        self.content.set("Mouse Out".to_string());
    }

    pub fn handle_context_menu(&mut self, _ctx: &NodeContext, _args: Event<ContextMenu>) {
        self.content.set("Context Menu".to_string());
    }
}
