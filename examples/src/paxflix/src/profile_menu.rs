#![allow(unused_imports)]

use crate::CinemaTheme;
use pax_kit::*;

#[pax]
#[file("profile_menu.pax")]
pub struct ProfileMenu {
    pub light_mode: Property<bool>,
    pub open: Property<bool>,
    pub feedback: Property<String>,
}

impl ProfileMenu {
    pub fn toggle_theme(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.light_mode.set(!self.light_mode.get());
    }

    pub fn close(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.open.set(false);
    }

    pub fn profiles(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.feedback
            .set("You're watching as the demo profile.".into());
    }

    pub fn account(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.feedback
            .set("Your account is on the house. This is a demo.".into());
    }

    pub fn help(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.feedback
            .set("Pick a film to explore. Happy browsing!".into());
    }
}
