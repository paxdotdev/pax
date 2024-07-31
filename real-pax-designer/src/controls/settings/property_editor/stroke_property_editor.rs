use std::rc::Rc;

use pax_engine::api::{pax_value::ToFromPaxAny, *};
use pax_engine::*;

use crate::controls::settings::AREAS_PROP;

use super::PropertyEditorData;

use pax_engine::api::Stroke;
use pax_std::*;

#[pax]
#[file("controls/settings/property_editor/stroke_property_editor.pax")]
pub struct StrokePropertyEditor {
    pub data: Property<PropertyEditorData>,

    // All the below props should be private: never set by user, used for internal state
    pub stroke: Property<Stroke>,
    pub red: Property<String>,
    pub green: Property<String>,
    pub blue: Property<String>,
    pub alpha: Property<String>,
    pub stroke_width: Property<String>,
    pub color: Property<Color>,
    pub palette: Property<Vec<Color>>,
}

impl StrokePropertyEditor {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let index = self.data.get().editor_index;
        if index != 0 {
            AREAS_PROP.with(|areas| {
                areas.update(|areas| {
                    while areas.len() <= index {
                        areas.push(0.0)
                    }
                    areas[index - 1] = 150.0;
                });
            });
        }
        self.palette.set(vec![
            Color::WHITE,
            Color::GREEN,
            Color::RED,
            Color::YELLOW,
            Color::BLUE,
            Color::ORANGE,
        ]);
        let data = self.data.clone();
        let deps = [data.untyped()];
        let ctx = ctx.clone();
        self.stroke.replace_with(Property::computed(
            move || {
                let val_str = data.get().get_value_as_str(&ctx);
                let stroke: Stroke =
                    pax_manifest::deserializer::from_pax_try_coerce::<Stroke>(&val_str)
                        .unwrap_or_default();
                stroke
            },
            &deps,
        ));

        let deps = [self.stroke.untyped()];
        let stroke = self.stroke.clone();
        self.color
            .replace_with(Property::computed(move || stroke.get().color.get(), &deps));
        let stroke = self.stroke.clone();
        self.stroke_width.replace_with(Property::computed(
            move || {
                stroke
                    .get()
                    .width
                    .get()
                    .expect_pixels()
                    .to_int()
                    .to_string()
            },
            &deps,
        ));

        fn get_color_channel(color: &Property<Color>, i: usize) -> String {
            ((color.get().to_rgba_0_1()[i] * 256.0) as u8).to_string()
        }
        let color = self.color.clone();
        let deps = [color.untyped()];
        self.red.replace_with(Property::computed(
            move || get_color_channel(&color, 0),
            &deps,
        ));

        let color = self.color.clone();
        self.blue.replace_with(Property::computed(
            move || get_color_channel(&color, 1),
            &deps,
        ));

        let color = self.color.clone();
        self.green.replace_with(Property::computed(
            move || get_color_channel(&color, 2),
            &deps,
        ));

        let color = self.color.clone();
        self.alpha.replace_with(Property::computed(
            move || get_color_channel(&color, 3),
            &deps,
        ));
    }

    pub fn red_change(&mut self, ctx: &NodeContext, event: Event<TextboxChange>) {
        self.set_channel(0, &event.text);
        self.commit_stroke(ctx);
    }

    pub fn blue_change(&mut self, ctx: &NodeContext, event: Event<TextboxChange>) {
        self.set_channel(1, &event.text);
        self.commit_stroke(ctx);
    }

    pub fn green_change(&mut self, ctx: &NodeContext, event: Event<TextboxChange>) {
        self.set_channel(2, &event.text);
        self.commit_stroke(ctx);
    }

    pub fn alpha_change(&mut self, ctx: &NodeContext, event: Event<TextboxChange>) {
        self.set_channel(3, &event.text);
        self.commit_stroke(ctx);
    }

    pub fn width_change(&mut self, ctx: &NodeContext, event: Event<TextboxChange>) {
        if let Ok(v) = event.text.parse::<u32>() {
            self.stroke.update(|stroke| {
                stroke.width.set(Size::Pixels(Numeric::U32(v)));
            });
        }
        self.commit_stroke(ctx);
    }

    pub fn set_channel(&mut self, i: usize, val: &str) {
        if let Some(v) = color_channel(val) {
            self.stroke.update(|stroke| {
                let col = stroke.color.get();
                let mut c = col.to_rgba_0_1();
                c[i] = v as f64 / 256.0;
                stroke.color.set(Color::from_rgba_0_1(c));
            });
        }
    }

    pub fn commit_stroke(&mut self, ctx: &NodeContext) {
        let stroke = self.stroke.get();
        let [r, g, b, a] = stroke.color.get().to_rgba_0_1().map(|v| (v * 256.0) as u8);
        let w = stroke.width.get().expect_pixels().to_int();
        self.data
            .get()
            .set_value(
                ctx,
                &format!(
                    "{{color: rgba({}, {}, {}, {}), width: {}px}}",
                    r, g, b, a, w,
                ),
            )
            .unwrap();
    }

    pub fn palette_slot0_clicked(&mut self, ctx: &NodeContext, event: Event<Click>) {
        self.palette_color_clicked(ctx, event, 0);
    }
    pub fn palette_slot1_clicked(&mut self, ctx: &NodeContext, event: Event<Click>) {
        self.palette_color_clicked(ctx, event, 1);
    }
    pub fn palette_slot2_clicked(&mut self, ctx: &NodeContext, event: Event<Click>) {
        self.palette_color_clicked(ctx, event, 2);
    }
    pub fn palette_slot3_clicked(&mut self, ctx: &NodeContext, event: Event<Click>) {
        self.palette_color_clicked(ctx, event, 3);
    }
    pub fn palette_slot4_clicked(&mut self, ctx: &NodeContext, event: Event<Click>) {
        self.palette_color_clicked(ctx, event, 4);
    }
    pub fn palette_slot5_clicked(&mut self, ctx: &NodeContext, event: Event<Click>) {
        self.palette_color_clicked(ctx, event, 5);
    }
    pub fn palette_slot6_clicked(&mut self, ctx: &NodeContext, event: Event<Click>) {
        self.palette_color_clicked(ctx, event, 6);
    }
    pub fn palette_slot7_clicked(&mut self, ctx: &NodeContext, event: Event<Click>) {
        self.palette_color_clicked(ctx, event, 7);
    }
    pub fn palette_slot8_clicked(&mut self, ctx: &NodeContext, event: Event<Click>) {
        self.palette_color_clicked(ctx, event, 8);
    }

    pub fn palette_color_clicked(&mut self, ctx: &NodeContext, _event: Event<Click>, i: usize) {
        if let Some(color) = self.palette.get().get(i) {
            self.stroke.update(|stroke| {
                stroke.color.set(color.clone());
            });
            self.commit_stroke(ctx);
        }
    }
}

fn color_channel(text: &str) -> Option<u8> {
    if text.is_empty() {
        return Some(0);
    }
    text.parse::<u8>().ok()
}
