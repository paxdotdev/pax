use crate::font::normalize_message;
use crate::ink::{rasterize, InkPoint, INK_COLUMNS, INK_ROWS};
#[allow(unused_imports)] // Referenced by message_editor.pax.
use crate::signature_pad::SignaturePad;
use pax_kit::*;

#[pax]
#[file("message_editor.pax")]
pub struct MessageEditor {
    pub message: Property<String>,
    pub open: Property<bool>,
    pub draft: Property<String>,
    pub card_width: Property<f64>,
    pub card_height: Property<f64>,
    pub cover_height: Property<f64>,
    pub surface_height: Property<f64>,
    pub footer_y: Property<f64>,
    pub draw_tab: Property<bool>,
    pub drawing_mode: Property<bool>,
    pub drawing: Property<Vec<InkPoint>>,
    pub drawing_columns: Property<Vec<u32>>,
    pub scroll_x: Property<f64>,
    pub scroll_max: Property<f64>,
}

#[derive(Default)]
pub(crate) struct EditorInk {
    pub points: Vec<InkPoint>,
    pub revision: u64,
    pub view_column: f64,
}
impl Store for EditorInk {}

impl MessageEditor {
    pub fn mount(&mut self, ctx: &NodeContext) {
        self.draft.set(normalize_message(&self.message.get()));
        self.draw_tab.set(self.drawing_mode.get());
        ctx.provide_store(EditorInk {
            points: self.drawing.get(),
            revision: 1,
            view_column: 0.,
        })
        .expect("message editor mounts once");
    }

    pub fn pre_render(&mut self, ctx: &NodeContext) {
        let (width, height) = ctx.bounds_self.get();
        let card_width = (width - 32.).clamp(1., 560.);
        let surface_height = if self.draw_tab.get() {
            (card_width - 48.).max(1.) * INK_ROWS as f64 / INK_COLUMNS as f64
        } else {
            46.
        };
        let footer_y = if self.draw_tab.get() { 224. } else { 188. } + surface_height;
        let card_height = footer_y + 66.;
        self.card_width.set_if_neq(card_width);
        self.surface_height.set_if_neq(surface_height);
        self.footer_y.set_if_neq(footer_y);
        self.card_height.set_if_neq(card_height);
        self.cover_height.set_if_neq(height.max(card_height + 32.));
    }

    pub fn edit(&mut self, _: &NodeContext, event: Event<TextboxInput>) {
        self.draft.set(normalize_message(&event.text));
    }

    pub fn key_down(&mut self, _: &NodeContext, event: Event<KeyDown>) {
        if self.open.get() && event.keyboard.key == "Escape" {
            event.prevent_default();
            self.open.set(false);
        }
    }

    pub fn show_text(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.draw_tab.set(false);
    }

    pub fn show_drawing(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.draw_tab.set(true);
    }

    pub fn clear_drawing(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        let _ = ctx.with_store(|ink: &mut EditorInk| {
            ink.points.clear();
            ink.revision += 1;
            ink.view_column = 0.;
        });
        self.scroll_x.set(0.);
    }

    pub fn confirm(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        self.message.set(normalize_message(&self.draft.get()));
        let _ = ctx.with_store(|ink: &mut EditorInk| {
            self.drawing.set(ink.points.clone());
            self.drawing_columns.set(rasterize(&ink.points));
        });
        self.drawing_mode.set(self.draw_tab.get());
        self.open.set(false);
    }
}
