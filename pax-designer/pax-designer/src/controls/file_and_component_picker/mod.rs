use pax_engine::api::*;
use pax_engine::*;

use pax_std::components::Stacker;
use pax_std::primitives::Text;
use pax_std::types::StackerDirection;

pub mod component_library_item;
use component_library_item::ComponentLibraryItem;

use component_library_item::ComponentLibraryItemData;

use pax_std::primitives::Image;
use pax_std::primitives::Rectangle;

use crate::model;

#[pax]
#[file("controls/file_and_component_picker/mod.pax")]
pub struct FileAndComponentPicker {
    pub library_active: Property<bool>,
    pub registered_components: Property<Vec<ComponentLibraryItemData>>,
    pub library_active_toggle_image: Property<StringBox>,
}

impl FileAndComponentPicker {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        self.library_active_toggle_image
            .set(StringBox::from("assets/icons/chevron-down.png".to_string()));
    }

    pub fn pre_render(&mut self, _ctx: &NodeContext) {}

    pub fn library_toggle(&mut self, ctx: &NodeContext, _args: ArgsClick) {
        self.library_active.set(!self.library_active.get());
        let curr = self.library_active.get();
        self.library_active_toggle_image.set(StringBox::from(
            match curr {
                true => "assets/icons/x.png",
                false => "assets/icons/chevron-down.png",
            }
            .to_string(),
        ));

        let dt = ctx.designtime.borrow_mut();
        let components: Vec<_> = dt
            .get_orm()
            .get_components()
            .values()
            .filter_map(|comp| {
                let is_in_inner_project = comp
                    .type_id
                    .starts_with("pax_designer::pax_reexports::designer_project::");
                let has_template = !comp.is_struct_only_component;
                let mut is_not_current = false;
                model::read_app_state(|app_state| {
                    is_not_current = app_state.selected_component_id != comp.type_id
                });
                if is_in_inner_project && has_template && is_not_current {
                    Some(ComponentLibraryItemData {
                        name: StringBox::from(comp.type_id.rsplit_once("::")?.1.clone()),
                        file_path: StringBox::from(comp.module_path.to_owned()),
                        type_id: comp.type_id.clone(),
                        bounds_pixels: (200.0, 200.0),
                    })
                } else {
                    None
                }
            })
            .collect();

        self.registered_components.set(match curr {
            true => components,
            false => vec![],
        });
    }
}
