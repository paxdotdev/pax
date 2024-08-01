pub mod file_and_component_picker;
pub mod logobar;
pub mod settings;
pub mod toolbar;
pub mod tree;

use pax_engine::api::*;
use pax_engine::*;

use pax_std::*;

use file_and_component_picker::FileAndComponentPicker;
use logobar::Logobar;
use settings::Settings;
use toolbar::Toolbar;
use tree::Tree;

#[pax]
#[file("controls/mod.pax")]
pub struct Controls {}