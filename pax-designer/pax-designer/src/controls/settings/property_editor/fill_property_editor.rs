use pax_engine::api::{pax_value::ToFromPaxAny, *};
use pax_engine::*;

use crate::controls::settings::{AreaMsg, AREAS_PROP};

use super::PropertyEditorData;

use pax_std::primitives::*;

#[pax]
#[file("controls/settings/property_editor/fill_property_editor.pax")]
pub struct FillPropertyEditor {
    pub data: Property<PropertyEditorData>,

    // All the below props should be private: never set by user, used for internal state
    pub red: Property<String>,
    pub green: Property<String>,
    pub blue: Property<String>,
    pub alpha: Property<String>,
    pub color: Property<Color>,
    pub palette: Property<Vec<Color>>,
}

impl FillPropertyEditor {
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
        let color = self.color.clone();
        let deps = [color.untyped()];
        self.red.replace_with(Property::computed(
            move || ((color.get().to_rgba_0_1()[0] * 256.0) as u8).to_string(),
            &deps,
        ));
        let color = self.color.clone();
        self.blue.replace_with(Property::computed(
            move || ((color.get().to_rgba_0_1()[1] * 256.0) as u8).to_string(),
            &deps,
        ));
        let color = self.color.clone();
        self.green.replace_with(Property::computed(
            move || ((color.get().to_rgba_0_1()[2] * 256.0) as u8).to_string(),
            &deps,
        ));
        let color = self.color.clone();
        self.alpha.replace_with(Property::computed(
            move || ((color.get().to_rgba_0_1()[3] * 256.0) as u8).to_string(),
            &deps,
        ));
    }

    pub fn red_input(&mut self, _ctx: &NodeContext, event: Event<TextboxInput>) {
        if let Some(v) = color_channel(&event.text) {
            self.color.update(|col| {
                let mut c = col.to_rgba_0_1();
                c[0] = v as f64 / 256.0;
                *col = Color::from_rgba_0_1(c);
            });
        }
    }

    pub fn blue_input(&mut self, _ctx: &NodeContext, event: Event<TextboxInput>) {
        if let Some(v) = color_channel(&event.text) {
            self.color.update(|col| {
                let mut c = col.to_rgba_0_1();
                c[1] = v as f64 / 256.0;
                *col = Color::from_rgba_0_1(c);
            });
        }
    }

    pub fn green_input(&mut self, _ctx: &NodeContext, event: Event<TextboxInput>) {
        if let Some(v) = color_channel(&event.text) {
            self.color.update(|col| {
                let mut c = col.to_rgba_0_1();
                c[2] = v as f64 / 256.0;
                *col = Color::from_rgba_0_1(c);
            });
        }
    }

    pub fn alpha_input(&mut self, _ctx: &NodeContext, event: Event<TextboxInput>) {
        if let Some(v) = color_channel(&event.text) {
            self.color.update(|col| {
                let mut c = col.to_rgba_0_1();
                c[3] = v as f64 / 256.0;
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
