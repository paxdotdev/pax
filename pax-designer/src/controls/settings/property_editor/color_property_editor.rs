use pax_engine::api::{pax_value::ToFromPaxAny, *};
use pax_engine::*;

use crate::controls::settings::color_picker::ColorPicker;

use super::{PropertyAreas, PropertyEditorData};

use pax_std::*;

#[pax]
#[engine_import_path("pax_engine")]
#[file("controls/settings/property_editor/color_property_editor.pax")]
pub struct ColorPropertyEditor {
    pub data: Property<PropertyEditorData>,
    pub color: Property<Color>,
    pub external: Property<bool>,
    pub property_listener: Property<bool>,
}

impl ColorPropertyEditor {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let index = self.data.get().editor_index;
        if index != 0 {
            let _ = ctx.peek_local_store(|PropertyAreas(areas): &mut PropertyAreas| {
                areas.update(|areas| {
                    while areas.len() <= index {
                        areas.push(0.0)
                    }
                    areas[index - 1] = 40.0;
                });
            });
        }
        let data = self.data.clone();
        let deps = [data.untyped()];
        let ctxc = ctx.clone();
        let external = self.external.clone();
        self.color.replace_with(Property::computed(
            move || {
                external.set(true);
                let value = pax_engine::pax_lang::from_pax(&data.get().get_value_as_str(&ctxc));
                if let Ok(value) = value {
                    let color: Color = Color::try_coerce(value).unwrap_or_default();
                    return color;
                }
                Color::default()
            },
            &deps,
        ));

        let color = self.color.clone();
        let deps = [color.untyped()];
        let external = self.external.clone();
        let data = self.data.clone();
        let ctxc = ctx.clone();
        self.property_listener.replace_with(Property::computed(
            move || {
                let color = color.get();
                if !external.get() {
                    let rgba = color.to_rgba_0_1();
                    let col_str = format!(
                        "rgba({}, {}, {}, {})",
                        (rgba[0] * 255.0) as u8,
                        (rgba[1] * 255.0) as u8,
                        (rgba[2] * 255.0) as u8,
                        (rgba[3] * 255.0) as u8
                    );
                    if let Err(e) = data.get().set_value(&ctxc, &col_str) {
                        log::warn!("failed to set fill color: {e}");
                    }
                }
                external.set(false);
                true
            },
            &deps,
        ));
    }

    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        self.property_listener.get();
    }
}
