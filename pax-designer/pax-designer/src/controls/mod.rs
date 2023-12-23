pub mod settings;
pub mod tree;
pub mod toolbar;
pub mod file_and_component_picker;
pub mod logobar;

use pax_lang::*;
use pax_lang::api::*;

use pax_std::primitives::{Rectangle, Group, Text};

use logobar::Logobar;
use file_and_component_picker::FileAndComponentPicker;
use tree::Tree;
use toolbar::Toolbar;
use settings::Settings;

#[derive(Pax)]
#[file("controls/mod.pax")]
pub struct Controls 
{
}