use pax_engine::api::*;
use pax_engine::*;

use crate::controls::settings::{AreaMsg, REQUEST_PROPERTY_AREA_CHANNEL};

use super::PropertyEditorData;

use pax_std::primitives::Group;
use pax_std::primitives::Rectangle;
use pax_std::primitives::Textbox;

#[pax]
#[file("controls/settings/property_editor/fill_property_editor.pax")]
pub struct FillPropertyEditor {
    pub data: Property<PropertyEditorData>,

    // All the below props should be private: never set by user, used for internal state
    pub last_definition: Property<String>,
    pub red: Property<String>,
    pub green: Property<String>,
    pub blue: Property<String>,
    pub alpha: Property<String>,

    pub palette: Property<Vec<Color>>,
}

impl FillPropertyEditor {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        if let Some(index) = self.data.get().editor_index {
            let mut channel_guard = REQUEST_PROPERTY_AREA_CHANNEL.lock().unwrap();
            let channel = channel_guard.get_or_insert_with(Vec::new);
            channel.push(AreaMsg {
                index,
                vertical_space: 100.0,
            })
        }
        self.set_color(Color::default());
        self.palette.set(vec![
            Color::WHITE,
            Color::GREEN,
            Color::RED,
            Color::YELLOW,
            Color::BLUE,
            Color::ORANGE,
        ]);
    }

    pub fn on_render(&mut self, ctx: &NodeContext) {
        let val_str = self.data.get().get_value_as_str(ctx);
        if self.last_definition.get() != &val_str {
            let color: Color =
                pax_manifest::deserializer::from_pax(&val_str).unwrap_or(Color::BLACK);
            self.set_color(color);
            self.last_definition.set(val_str);
        }
    }

    pub fn red_input(&mut self, _ctx: &NodeContext, event: Event<TextboxInput>) {
        if color_channel(&event.text).is_some() {
            self.red.set(event.text.clone());
        }
    }
    pub fn green_input(&mut self, _ctx: &NodeContext, event: Event<TextboxInput>) {
        if color_channel(&event.text).is_some() {
            self.green.set(event.text.clone());
        }
    }
    pub fn blue_input(&mut self, _ctx: &NodeContext, event: Event<TextboxInput>) {
        if color_channel(&event.text).is_some() {
            self.blue.set(event.text.clone());
        }
    }
    pub fn alpha_input(&mut self, _ctx: &NodeContext, event: Event<TextboxInput>) {
        if color_channel(&event.text).is_some() {
            self.alpha.set(event.text.clone());
        }
    }

    pub fn text_change(&mut self, ctx: &NodeContext, _event: Event<TextboxChange>) {
        self.commit_color(ctx);
    }

    pub fn commit_color(&mut self, ctx: &NodeContext) {
        let red = color_channel(self.red.get());
        let green = color_channel(self.green.get());
        let blue = color_channel(self.blue.get());
        let alpha = color_channel(self.alpha.get());
        if let (Some(r), Some(g), Some(b), Some(a)) = (red, green, blue, alpha) {
            self.red.set(r.to_string());
            self.green.set(g.to_string());
            self.blue.set(b.to_string());
            self.alpha.set(a.to_string());
            self.data
                .get()
                .set_value(ctx, &format!("rgb({}, {}, {})", r, g, b))
                .unwrap();
        } else {
            unreachable!("colors always valid u8s")
        }
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
            self.set_color(color.clone());
            self.commit_color(ctx);
        }
    }

    fn set_color(&mut self, color: Color) {
        let mut ints = color.to_rgba_0_1().into_iter().map(|v| (v * 255.0) as u8);
        self.red.set(ints.next().unwrap().to_string());
        self.green.set(ints.next().unwrap().to_string());
        self.blue.set(ints.next().unwrap().to_string());
        self.alpha.set(ints.next().unwrap().to_string());
    }
}

fn color_channel(text: &str) -> Option<u8> {
    if text.is_empty() {
        return Some(0);
    }
    text.parse::<u8>().ok()
}
