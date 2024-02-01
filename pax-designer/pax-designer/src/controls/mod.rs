pub mod file_and_component_picker;
pub mod logobar;
pub mod settings;
pub mod toolbar;
pub mod tree;

use pax_lang::api::*;
use pax_lang::*;

use pax_std::primitives::{Frame, Group, Rectangle, Text};

use file_and_component_picker::FileAndComponentPicker;
use logobar::Logobar;
use settings::Settings;
use toolbar::Toolbar;
use tree::Tree;

#[pax]
#[file("controls/mod.pax")]
pub struct Controls {}

impl Controls {
   
    pub fn handle_click(&mut self, _ctx: &NodeContext, _args: ArgsClick) {
        panic!("Mouse click");
    }
}