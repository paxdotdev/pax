
use pax_lang::*;
use pax_std::primitives::{Image, Group};


#[derive(Pax)]
#[file("controls/toolbar_panel.pax")]
pub struct ToolbarPanel {
    icon_path: Property<String>,
    sub_panels: Property<Vec<ToolbarPanel>>,
}