use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Mutex;

use pax_engine::api::*;
use pax_engine::*;

use pax_manifest::PaxType;
use pax_std::components::Stacker;
use pax_std::primitives::Text;
use pax_std::types::StackerDirection;

pub mod component_library_item;
use component_library_item::ComponentLibraryItem;

use component_library_item::ComponentLibraryItemData;

use pax_std::primitives::Image;
use pax_std::primitives::Rectangle;

use crate::model;
use crate::model::action::Action;
use crate::model::action::ActionContext;
use crate::model::action::CanUndo;
use crate::USER_PROJ_ROOT_IMPORT_PATH;

#[pax]
#[file("controls/file_and_component_picker/mod.pax")]
pub struct FileAndComponentPicker {
    pub library_active: Property<bool>,
    pub registered_components: Property<Vec<ComponentLibraryItemData>>,
    pub library_active_toggle_image: Property<StringBox>,
    pub manifest_ver: Property<usize>,
}

pub struct SetLibraryState {
    pub open: bool,
}

impl Action for SetLibraryState {
    fn perform(self: Box<Self>, _ctx: &mut ActionContext) -> anyhow::Result<CanUndo> {
        *LIBRARY_MSG.lock().unwrap() = Some(*self);
        Ok(CanUndo::No)
    }
}

static LIBRARY_MSG: Mutex<Option<SetLibraryState>> = Mutex::new(None);

impl FileAndComponentPicker {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        self.library_active_toggle_image
            .set(StringBox::from("assets/icons/chevron-down.png".to_string()));
    }

    pub fn pre_render(&mut self, ctx: &NodeContext) {
        let manifest_ver = {
            let dt = ctx.designtime.borrow();
            dt.get_manifest_version()
        };
        if manifest_ver != self.manifest_ver.get() {
            self.set_library(ctx);
            self.manifest_ver.set(manifest_ver);
        }

        if let Some(msg) = LIBRARY_MSG.lock().unwrap().take() {
            if self.library_active.get() != msg.open {
                self.library_active.set(msg.open);
                self.set_library(ctx);
            }
        }
    }

    pub fn library_toggle(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        self.library_active.set(!self.library_active.get());
        self.set_library(ctx);
    }

    pub fn set_library(&mut self, ctx: &NodeContext) {
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
            .iter()
            .filter_map(|type_id| {
                let is_userland_component = type_id
                    .import_path()
                    .is_some_and(|p| p.starts_with(USER_PROJ_ROOT_IMPORT_PATH));

                let is_mock = matches!(type_id.get_pax_type(), PaxType::BlankComponent { .. });

                if !is_userland_component && !is_mock {
                    return None;
                }

                let comp = dt.get_orm().get_component(type_id).unwrap();
                let has_template = !comp.is_struct_only_component;
                let mut is_not_current = false;
                model::read_app_state(|app_state| {
                    is_not_current = app_state.selected_component_id.get() != comp.type_id
                });
                if has_template && is_not_current {
                    Some(ComponentLibraryItemData {
                        name: StringBox::from(comp.type_id.get_pascal_identifier().unwrap()),
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
