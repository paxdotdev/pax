use pax_engine::api::{pax_value::ToFromPaxAny, *};
use pax_engine::*;

use crate::controls::settings::AREAS_PROP;

use super::PropertyEditorData;

use pax_std::primitives::*;

#[pax]
#[file("controls/settings/property_editor/color_property_editor.pax")]
pub struct ColorPropertyEditor {
    pub data: Property<PropertyEditorData>,

    // All the below props should be private: never set by user, used for internal state
    pub red: Property<String>,
    pub green: Property<String>,
    pub blue: Property<String>,
    pub alpha: Property<String>,
    pub color: Property<Color>,
    pub palette: Property<Vec<Color>>,
}

impl ColorPropertyEditor {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let index = self.data.get().editor_index;
        if index != 0 {
            AREAS_PROP.with(|areas| {
                areas.update(|areas| {
                    while areas.len() <= index {
                        areas.push(0.0)
                    }
                    areas[index - 1] = 107.0;
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
        self.color.replace_with(Property::computed(
            move || {
                let val_str = data.get().get_value_as_str(&ctx);
                let color: Color =
                    pax_manifest::deserializer::from_pax_try_coerce::<Color>(&val_str)
                        .unwrap_or_default();
                color
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
        self.commit_color(ctx);
    }

    pub fn blue_change(&mut self, ctx: &NodeContext, event: Event<TextboxChange>) {
        self.set_channel(1, &event.text);
        self.commit_color(ctx);
    }

    pub fn green_change(&mut self, ctx: &NodeContext, event: Event<TextboxChange>) {
        self.set_channel(2, &event.text);
        self.commit_color(ctx);
    }

    pub fn alpha_change(&mut self, ctx: &NodeContext, event: Event<TextboxChange>) {
        self.set_channel(3, &event.text);
        self.commit_color(ctx);
    }

    pub fn set_channel(&mut self, i: usize, val: &str) {
        if let Some(v) = color_channel(val) {
            self.color.update(|col| {
                let mut c = col.to_rgba_0_1();
                c[i] = v as f64 / 256.0;
                *col = Color::from_rgba_0_1(c);
            });
        }
    }

    pub fn text_change(&mut self, ctx: &NodeContext, _event: Event<TextboxChange>) {
        self.commit_color(ctx);
    }

    pub fn commit_color(&mut self, ctx: &NodeContext) {
        let [r, g, b, a] = self.color.get().to_rgba_0_1().map(|v| (v * 256.0) as u8);
        self.data
            .get()
            .set_value(ctx, &format!("rgba({}, {}, {}, {})", r, g, b, a))
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
            self.color.set(color.clone());
            self.commit_color(ctx);
        }
    }
}

fn color_channel(text: &str) -> Option<u8> {
    if text.is_empty() {
        return Some(0);
    }
    text.parse::<u8>().ok()
}
