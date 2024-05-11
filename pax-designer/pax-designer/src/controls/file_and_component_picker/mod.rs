use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Mutex;

use pax_designtime::DesigntimeManager;
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
    pub library_active_toggle_image: Property<String>,
}

impl Interpolatable for SetLibraryState {}
#[derive(Clone, Default)]
pub struct SetLibraryState {
    pub open: bool,
}

impl Action for SetLibraryState {
    fn perform(self: Box<Self>, _ctx: &mut ActionContext) -> anyhow::Result<CanUndo> {
        LIBRARY_PROP.with(|lib_prop| {
            lib_prop.set(*self);
        });
        Ok(CanUndo::No)
    }
}

thread_local! {
    static LIBRARY_PROP: Property<SetLibraryState> = Property::new(SetLibraryState { open: false });
}

impl FileAndComponentPicker {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let lp = LIBRARY_PROP.with(|l| l.clone());
        let deps = [lp.untyped()];
        self.library_active
            .replace_with(Property::computed(move || lp.get().open, &deps));

        let lib_active = self.library_active.clone();
        let deps = [lib_active.untyped()];
        self.library_active_toggle_image
            .replace_with(Property::computed(
                move || {
                    String::from(match lib_active.get() {
                        true => "assets/icons/x.png",
                        false => "assets/icons/chevron-down.png",
                    })
                },
                &deps,
            ));
        let dt = Rc::clone(&ctx.designtime);
        let lib_active = self.library_active.clone();
        let selected_component =
            model::read_app_state(|app_state| app_state.selected_component_id.clone());
        let manifest_ver = borrow!(ctx.designtime).get_manifest_version();
        let deps = [
            lib_active.untyped(),
            selected_component.untyped(),
            manifest_ver.untyped(),
        ];
        self.registered_components.replace_with(Property::computed(
            move || {
                log::debug!("registered comps changed");
                let dt = borrow_mut!(dt);

                if lib_active.get() == false {
                    return vec![];
                }

                let components: Vec<_> = dt
                    .get_orm()
                    .get_components()
                    .iter()
                    .filter_map(|type_id| {
                        let is_userland_component = type_id
                            .import_path()
                            .is_some_and(|p| p.starts_with(USER_PROJ_ROOT_IMPORT_PATH));

                        let is_mock =
                            matches!(type_id.get_pax_type(), PaxType::BlankComponent { .. });

                        if !is_userland_component && !is_mock {
                            return None;
                        }

                        let comp = dt.get_orm().get_component(type_id).unwrap();
                        let has_template = !comp.is_struct_only_component;
                        let is_not_current = selected_component.get() != comp.type_id;
                        if has_template && is_not_current {
                            Some(ComponentLibraryItemData {
                                name: comp.type_id.get_pascal_identifier().unwrap(),
                                file_path: comp.module_path.clone(),
                                type_id: comp.type_id.clone(),
                                bounds_pixels: (200.0, 200.0),
                            })
                        } else {
                            None
                        }
                    })
                    .collect();

                components
            },
            &deps,
        ));
    }

    pub fn library_toggle(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        model::perform_action(
            SetLibraryState {
                open: !self.library_active.get(),
            },
            ctx,
        );
    }
}
