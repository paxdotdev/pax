#![allow(unused_imports)]

use pax_lang::api::*;
use pax_lang::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub ticks: Property<usize>,
    pub num_clicks: Property<usize>,
    pub message: Property<String>,
    pub conditional: Property<bool>,
    pub checked: Property<bool>,
    pub align_vertical: Property<TextAlignVertical>,
    pub color: Property<Color>,
    pub textbox_text: Property<String>,
}

impl Example {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.message.set("Click me".to_string());
        self.color
            .set(Color::rgba(1.0.into(), 0.3.into(), 0.6.into(), 1.0.into()));
    }
    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        let old_ticks = self.ticks.get();
        self.ticks.set(old_ticks + 1);
    }

    pub fn toggle(&mut self, ctx: &NodeContext, args: ArgsClick) {
        let old_num_clicks = self.num_clicks.get();
        self.num_clicks.set(old_num_clicks + 1);
        self.message
            .set(format!("{} clicks", self.num_clicks.get()));
        self.conditional.set(!self.conditional.get());
    }

    pub fn checkbox_change(&mut self, ctx: &NodeContext, args: ArgsCheckboxChange) {
        self.checked.set(!args.checked);
        self.align_vertical.set(match self.checked.get() {
            true => TextAlignVertical::Top,
            false => TextAlignVertical::Bottom,
        });
        if *self.checked.get() {
            self.color
                .set(Color::rgba(0.0.into(), 1.0.into(), 0.0.into(), 1.0.into()));
        } else {
            self.color
                .set(Color::rgba(0.0.into(), 0.0.into(), 1.0.into(), 1.0.into()));
        }
    }

    pub fn textbox_change(&mut self, ctx: &NodeContext, args: ArgsTextboxChange) {
        self.textbox_text.set(args.text);
    }

    pub fn button_click(&mut self, ctx: &NodeContext, args: ArgsButtonClick) {
        self.color
            .set(Color::rgba(1.0.into(), 0.3.into(), 0.6.into(), 1.0.into()));
    }
}
