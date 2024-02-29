use pax_engine::api::*;
use pax_engine::*;

use pax_std::components::Stacker;
use pax_std::primitives::Text;
use pax_std::types::StackerDirection;

pub mod component_library_item;
use component_library_item::ComponentLibraryItem;

use component_library_item::ComponentLibraryItemData;

#[pax]
#[file("controls/file_and_component_picker/mod.pax")]
pub struct FileAndComponentPicker {
    pub library_active: Property<bool>,
    pub registered_components: Property<Vec<ComponentLibraryItemData>>,
}

impl FileAndComponentPicker {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        self.library_active.set(true);
        //this data will be set from manifest def later on
        self.registered_components.set(vec![
            ComponentLibraryItemData {
                name: StringBox::from("Rectangle".to_owned()),
                file_path: StringBox::from("pax-std/src/rectangle.rs"),
                type_id: "pax_designer::pax_reexports::pax_std::primitives::Rectangle".to_owned(),
            },
            ComponentLibraryItemData {
                name: StringBox::from("Userland Ellipse".to_owned()),
                file_path: StringBox::from("pax-std/src/rectangle.rs"),
                type_id: "pax_designer::pax_reexports::pax_std::primitives::Ellipse".to_owned(),
            },
        ]);
    }

    pub fn pre_render(&mut self, _ctx: &NodeContext) {}
}
