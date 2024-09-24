use std::rc::Rc;

use pax_engine::api::*;
use pax_engine::*;
use pax_std::*;

use crate::controls::settings::color_picker::ColorPicker;
use crate::model::tools::{PaintbrushToolSettings, PAINTBRUSH_TOOL};

#[pax]
#[engine_import_path("pax_engine")]
#[file("controls/tool_settings_views/paintbrush_settings_view.pax")]
pub struct PaintbrushSettings {
    pub fill_color: Property<Color>,
    pub stroke_color: Property<Color>,
    pub brush_radius_text: Property<String>,
    pub stroke_width_text: Property<String>,
}

impl PaintbrushSettings {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        let paintbrush_settings = PAINTBRUSH_TOOL.with(|p| p.clone());
        let curr = paintbrush_settings.get();

        self.fill_color.set(curr.fill_color);
        self.stroke_color.set(curr.stroke_color);
        self.brush_radius_text.set(curr.brush_radius.to_string());
        self.stroke_width_text.set(curr.stroke_width.to_string());

        let fill_color = self.fill_color.clone();
        let stroke_color = self.stroke_color.clone();
        let brush_radius_text = self.brush_radius_text.clone();
        let stroke_width_text = self.stroke_width_text.clone();

        // keep track of old values and use if new parsed value is bad
        let old_brush_radius = Rc::new(std::cell::Cell::new(curr.brush_radius));
        let old_stroke_width = Rc::new(std::cell::Cell::new(curr.stroke_width));

        let deps = [
            fill_color.untyped(),
            stroke_color.untyped(),
            brush_radius_text.untyped(),
            stroke_width_text.untyped(),
        ];
        paintbrush_settings.replace_with(Property::computed(
            move || {
                let brush_radius = if let Ok(val) = brush_radius_text.get().parse::<f64>() {
                    let val = val.clamp(5.0, 100.0);
                    old_brush_radius.set(val);
                    val
                } else {
                    old_brush_radius.get()
                };

                let stroke_width = if let Ok(val) = stroke_width_text.get().parse::<u32>() {
                    let val = val.clamp(0, 50);
                    old_stroke_width.set(val);
                    val
                } else {
                    old_stroke_width.get()
                };
                PaintbrushToolSettings {
                    fill_color: fill_color.get(),
                    stroke_color: stroke_color.get(),
                    brush_radius,
                    stroke_width,
                }
            },
            &deps,
        ));
    }
}
