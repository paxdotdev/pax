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

#[derive(Pax)]
#[file("controls/mod.pax")]
pub struct Controls {}
