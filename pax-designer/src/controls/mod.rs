pub mod class_settings_editor;
pub mod file_and_component_picker;
pub mod logobar;
pub mod settings;
pub mod tool_settings_views;
pub mod toolbar;
pub mod tree;

use pax_engine::api::*;
use pax_engine::*;

use pax_std::*;

use class_settings_editor::ClassSettingsEditor;
use file_and_component_picker::FileAndComponentPicker;
use logobar::Logobar;
use settings::Settings;
use tool_settings_views::paintbrush_settings_view::PaintbrushSettings;
use toolbar::Toolbar;
use tree::Tree;

use crate::model::{self, app_state::Tool};

#[pax]
#[engine_import_path("pax_engine")]
#[file("controls/mod.pax")]
pub struct Controls {
    pub settings_view_page: Property<SettingsViewPage>,
}

#[pax]
#[engine_import_path("pax_engine")]
pub enum SettingsViewPage {
    PaintbrushEditor,
    #[default]
    NodeEditor,
    ClassEditor,
}

impl Controls {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        let (selected_tool, current_class_editing) = model::read_app_state(|app_state| {
            (
                app_state.selected_tool.clone(),
                app_state.current_editor_class_name.clone(),
            )
        });
        let deps = [selected_tool.untyped(), current_class_editing.untyped()];
        self.settings_view_page.replace_with(Property::computed(
            move || {
                if current_class_editing.get().is_some() {
                    return SettingsViewPage::ClassEditor;
                }
                if selected_tool.get() == Tool::Paintbrush {
                    return SettingsViewPage::PaintbrushEditor;
                }
                SettingsViewPage::NodeEditor
            },
            &deps,
        ));
    }
}
