pub mod settings;
pub mod tree;
pub mod toolbar;
pub mod overflowing_stacker;

use pax_lang::*;
use pax_lang::api::*;

use tree::Tree;
use toolbar::Toolbar;
use settings::Settings;

#[derive(Pax)]
#[file("controls/mod.pax")]
pub struct Controls 
{

}