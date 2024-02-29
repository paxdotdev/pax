use pax_engine::api::*;
use pax_engine::*;
use pax_std::primitives::Rectangle;
use pax_std::primitives::Text;

#[pax]
#[file("controls/file_and_component_picker/component_library_item.pax")]
pub struct ComponentLibraryItem {
    pub data: Property<ComponentLibraryItemData>,
}

#[pax]
pub struct ComponentLibraryItemData {
    pub name: StringBox,
    pub file_path: StringBox,
    pub type_id: String,
}

impl ComponentLibraryItem {
    pub fn on_click(&mut self, _ctx: &NodeContext, _args: ArgsClick) {
        pax_engine::log::info!("arm tool to create: {:?}", self.data.get().type_id);
    }
}

// TODO:
// - make button to hide/show library
// - change click to mousedown -> activates a new ToolBehaviour that drops in the component on pointer up
// - styling
