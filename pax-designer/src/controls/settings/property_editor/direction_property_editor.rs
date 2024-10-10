use pax_engine::api::*;
use pax_engine::*;
use pax_manifest::*;
use pax_std::*;

use super::{PropertyAreas, PropertyEditorData};

#[pax]
#[engine_import_path("pax_engine")]
#[file("controls/settings/property_editor/direction_property_editor.pax")]
pub struct DirectionPropertyEditor {
    pub data: Property<PropertyEditorData>,

    // All the below props should be private: never set by user, used for internal state
    pub horizontal_color: Property<Color>,
    pub vertical_color: Property<Color>,
    pub is_vertical: Property<bool>,
}

impl DirectionPropertyEditor {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let index = self.data.get().editor_index;
        if index != 0 {
            let _ = ctx.peek_local_store(|PropertyAreas(areas): &mut PropertyAreas| {
                areas.update(|areas| {
                    while areas.len() <= index {
                        areas.push(0.0)
                    }
                    areas[index - 1] = 65.0;
                });
            });
        }
        let data = self.data.clone();
        let deps = [data.untyped()];
        let ctx = ctx.clone();
        self.is_vertical.replace_with(Property::computed(
            move || {
                data.get()
                    .get_value_typed(&ctx)
                    .map_err(|e| {
                        log::warn!(
                            "failed to read {} for {} - using default: {e}",
                            "stacker direction",
                            "direction editor"
                        );
                    })
                    .unwrap_or_default()
            },
            &deps,
        ));
        let activated = Color::rgb(30.into(), 30.into(), 30.into());
        let deactivated = Color::BLACK;

        let is_vert = self.is_vertical.clone();
        let activated_cp = activated.clone();
        let deactivated_cp = deactivated.clone();
        let deps = [is_vert.untyped()];
        self.vertical_color.replace_with(Property::computed(
            move || match is_vert.get() {
                true => activated_cp.clone(),
                false => deactivated_cp.clone(),
            },
            &deps,
        ));
        let is_vert = self.is_vertical.clone();
        self.horizontal_color.replace_with(Property::computed(
            move || match is_vert.get() {
                true => deactivated.clone(),
                false => activated.clone(),
            },
            &deps,
        ));
    }

    pub fn set_vertical(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        if let Err(e) = self
            .data
            .get()
            .set_value_typed(ctx, StackerDirection::Vertical)
        {
            log::warn!("failed to set vertical {e}");
        }
    }

    pub fn set_horizontal(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        if let Err(e) = self
            .data
            .get()
            .set_value_typed(ctx, StackerDirection::Horizontal)
        {
            log::warn!("failed to set horizontal {e}");
        }
    }
}
