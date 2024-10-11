use std::rc::Rc;

use pax_engine::api::{pax_value::ToFromPaxAny, *};
use pax_engine::pax_manifest::ValueDefinition;
use pax_engine::*;

use super::{PropertyAreas, PropertyEditorData};
use crate::controls::settings::color_picker::ColorPicker;

use pax_engine::api::Stroke;
use pax_std::*;

#[pax]
#[engine_import_path("pax_engine")]
#[file("controls/settings/property_editor/stroke_property_editor.pax")]
pub struct StrokePropertyEditor {
    pub data: Property<PropertyEditorData>,

    // All the below props should be private: never set by user, used for internal state
    pub stroke: Property<Stroke>,
    pub stroke_width_text: Property<String>,
    pub stroke_width: Property<f64>,
    pub color: Property<Color>,
    pub external: Property<bool>,
    pub property_listener: Property<bool>,
}

impl StrokePropertyEditor {
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
        self.stroke.replace_with(Property::computed(
            move || {
                external.set(true);
                data.get()
                    .get_value_typed(&ctxc)
                    .map_err(|e| {
                        log::warn!(
                            "failed to read {} for {} - using default: {e}",
                            "stroke",
                            "stroke editor"
                        );
                    })
                    .unwrap_or_default()
                    .unwrap_or_default()
            },
            &deps,
        ));

        let deps = [self.stroke.untyped()];
        let stroke = self.stroke.clone();
        self.color
            .replace_with(Property::computed(move || stroke.get().color.get(), &deps));

        let stroke = self.stroke.clone();
        self.stroke_width.replace_with(Property::computed(
            move || stroke.get().width.get().expect_pixels().to_float(),
            &deps,
        ));

        let stroke_width = self.stroke_width.clone();
        let deps = [stroke_width.untyped()];
        self.stroke_width_text.replace_with(Property::computed(
            move || format!("{:.1}", stroke_width.get()),
            &deps,
        ));

        let color = self.color.clone();
        let color_dep = color.untyped();
        // let stroke_width_cloned = self.stroke_width.clone();
        // let external_cloned = self.external.clone();
        let color = Property::computed(
            move || {
                let color = color.get();
                // TODO this is triggered on initial object selection, fix this
                // if stroke_width_cloned.get() == 0.0 && !external_cloned.get() {
                //     stroke_width_cloned.set(1.0);
                // }
                color
            },
            &[color_dep],
        );

        let stroke_width = self.stroke_width.clone();
        let deps = [color.untyped(), stroke_width.untyped()];
        let external = self.external.clone();
        let data = self.data.clone();
        let ctxc = ctx.clone();
        self.property_listener.replace_with(Property::computed(
            move || {
                let color = color.get();
                let stroke_width = stroke_width.get();
                if !external.get() {
                    let stroke = Stroke {
                        color: Property::new(color),
                        width: Property::new(Size::Pixels(stroke_width.into())),
                    };
                    if let Err(e) = data.get().set_value_typed(&ctxc, stroke) {
                        log::warn!("failed to set stroke: {e}");
                    }
                }
                external.set(false);
                true
            },
            &deps,
        ));
    }

    pub fn width_change(&mut self, _ctx: &NodeContext, event: Event<TextboxChange>) {
        if let Ok(num) = event.text.parse() {
            self.stroke_width.set(num)
        } else {
            log::warn!("can't set stroke: {:?} is not a number", event.text);
        }
    }

    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        self.property_listener.get();
    }
}
