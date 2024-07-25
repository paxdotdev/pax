use pax_engine::api::*;
use pax_engine::*;
use pax_manifest::*;
use pax_std::*;

use crate::controls::settings::AREAS_PROP;

use super::PropertyEditorData;

#[pax]
#[file("controls/settings/property_editor/direction_property_editor.pax")]
pub struct DirectionPropertyEditor {
    pub data: Property<PropertyEditorData>,

    // All the below props should be private: never set by user, used for internal state
    pub horizontal: Property<Color>,
    pub vertical: Property<Color>,
    pub is_vertical: Property<bool>,
}

impl DirectionPropertyEditor {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let index = self.data.get().editor_index;
        if index != 0 {
            AREAS_PROP.with(|areas| {
                areas.update(|areas| {
                    while areas.len() <= index {
                        areas.push(0.0)
                    }
                    areas[index - 1] = 65.0;
                });
            });
        }
        let data = self.data.clone();
        let manifest_ver = borrow!(ctx.designtime).get_manifest_version();
        let deps = [data.untyped(), manifest_ver.untyped()];
        let ctx = ctx.clone();
        self.is_vertical.replace_with(Property::computed(
            move || {
                let s = data.get().get_value_as_str(&ctx);
                !s.contains("StackerDirection::Horizontal")
            },
            &deps,
        ));
        let activated = Color::rgb(30.into(), 30.into(), 30.into());
        let deactivated = Color::BLACK;

        let is_vert = self.is_vertical.clone();
        let activated_cp = activated.clone();
        let deactivated_cp = deactivated.clone();
        let deps = [is_vert.untyped()];
        self.vertical.replace_with(Property::computed(
            move || match is_vert.get() {
                true => activated_cp.clone(),
                false => deactivated_cp.clone(),
            },
            &deps,
        ));
        let is_vert = self.is_vertical.clone();
        self.horizontal.replace_with(Property::computed(
            move || match is_vert.get() {
                true => deactivated.clone(),
                false => activated.clone(),
            },
            &deps,
        ));
    }

    pub fn set_vertical(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        if let Err(e) = self.data.get().set_value(ctx, "StackerDirection::Vertical") {
            log::warn!("failed to set vert {e}");
        }
    }

    pub fn set_horizontal(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        if let Err(e) = self
            .data
            .get()
            .set_value(ctx, "StackerDirection::Horizontal")
        {
            log::warn!("failed to set vert {e}");
        }
    }
}
