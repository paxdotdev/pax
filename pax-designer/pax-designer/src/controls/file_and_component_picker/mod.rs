use pax_designtime::DesigntimeManager;
use pax_engine::api::*;
use pax_engine::*;
use pax_manifest::{PaxType, TypeId};
use std::rc::Rc;

use crate::model;
use crate::model::action::{Action, ActionContext};
use crate::USER_PROJ_ROOT_IMPORT_PATH;

pub mod component_library_item;
use component_library_item::{ComponentLibraryItem, ComponentLibraryItemData};

use pax_std::*;

#[pax]
#[file("controls/file_and_component_picker/mod.pax")]
pub struct FileAndComponentPicker {
    pub library_active: Property<bool>,
    pub registered_components: Property<Vec<ComponentLibraryItemData>>,
    pub library_active_toggle_image: Property<String>,
}

#[derive(Clone, Default)]
pub struct SetLibraryState {
    pub open: bool,
}

impl Interpolatable for SetLibraryState {}

impl Action for SetLibraryState {
    fn perform(&self, _ctx: &mut ActionContext) -> anyhow::Result<()> {
        LIBRARY_STATE.with(|state| state.set(self.clone()));
        Ok(())
    }
}

thread_local! {
    static LIBRARY_STATE: Property<SetLibraryState> = Property::new(SetLibraryState { open: false });
}

impl FileAndComponentPicker {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        self.bind_library_active();
        self.bind_library_active_toggle_image();
        self.bind_registered_components(ctx);
    }

    fn bind_library_active(&mut self) {
        let library_state = LIBRARY_STATE.with(|state| state.clone());
        let deps = [library_state.untyped()];
        self.library_active
            .replace_with(Property::computed(move || library_state.get().open, &deps));
    }

    fn bind_library_active_toggle_image(&mut self) {
        let library_active = self.library_active.clone();
        let deps = [library_active.untyped()];
        self.library_active_toggle_image
            .replace_with(Property::computed(
                move || {
                    if library_active.get() {
                        "assets/icons/x.png".to_string()
                    } else {
                        "assets/icons/chevron-down.png".to_string()
                    }
                },
                &deps,
            ));
    }

    fn bind_registered_components(&mut self, ctx: &NodeContext) {
        let dt = Rc::clone(&ctx.designtime);
        let library_active = self.library_active.clone();
        let selected_component =
            model::read_app_state(|app_state| app_state.selected_component_id.clone());
        let manifest_ver = ctx.designtime.borrow().get_manifest_version();

        let deps = [
            library_active.untyped(),
            selected_component.untyped(),
            manifest_ver.untyped(),
        ];
        self.registered_components.replace_with(Property::computed(
            move || {
                if !library_active.get() {
                    return vec![];
                }

                let dt = dt.borrow_mut();
                dt.get_orm()
                    .get_components()
                    .iter()
                    .filter_map(|type_id| {
                        Self::get_component_data(&dt, type_id, &[selected_component.get()])
                    })
                    .collect()
            },
            &deps,
        ));
    }

    fn get_component_data(
        dt: &DesigntimeManager,
        type_id: &TypeId,
        filter: &[TypeId],
    ) -> Option<ComponentLibraryItemData> {
        let is_userland_or_mock = type_id
            .import_path()
            .is_some_and(|p| p.starts_with(USER_PROJ_ROOT_IMPORT_PATH))
            || matches!(type_id.get_pax_type(), PaxType::BlankComponent { .. });

        if !is_userland_or_mock {
            return None;
        }

        let comp = dt.get_orm().get_component(type_id).ok()?;

        if comp.is_struct_only_component || filter.contains(&comp.type_id) {
            return None;
        }

        Some(ComponentLibraryItemData {
            name: comp.type_id.get_pascal_identifier().unwrap_or_default(),
            file_path: comp.module_path.clone(),
            type_id: comp.type_id.clone(),
            bounds_pixels: (200.0, 200.0),
        })
    }

    pub fn library_toggle(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        model::perform_action(
            &SetLibraryState {
                open: !self.library_active.get(),
            },
            ctx,
        );
    }
}
