pub mod file_and_component_picker;
pub mod logobar;
pub mod settings;
pub mod tool_settings_views;
pub mod toolbar;
pub mod tree;

use pax_engine::api::*;
use pax_engine::*;

use pax_std::*;

use file_and_component_picker::FileAndComponentPicker;
use logobar::Logobar;
use settings::Settings;
use tool_settings_views::paintbrush_settings_view::PaintbrushSettings;
use toolbar::Toolbar;
use tree::Tree;

use crate::model::{self, Tool};

#[pax]
#[engine_import_path("pax_engine")]
#[file("controls/mod.pax")]
pub struct Controls {
    pub tool_with_tool_editor_selected: Property<bool>,
}

impl Controls {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        let selected_tool = model::read_app_state(|app_state| app_state.selected_tool.clone());
        let deps = [selected_tool.untyped()];
        self.tool_with_tool_editor_selected
            .replace_with(Property::computed(
                move || selected_tool.get() == Tool::Paintbrush,
                &deps,
            ));
    }
}
