use pax_kit::pax_engine::api::cursor::CursorStyle;
use pax_kit::*;

#[pax]
#[custom(Default)]
#[file("logo_card.pax")]
pub struct LogoCard {
    pub is_compact: Property<bool>,
    pub logo_progress: Property<f64>,
}

impl LogoCard {
    pub fn handle_mouse_over(&mut self, ctx: &NodeContext, _event: Event<MouseOver>) {
        ctx.set_cursor(CursorStyle::Pointer);
    }

    pub fn handle_mouse_out(&mut self, ctx: &NodeContext, _event: Event<MouseOut>) {
        ctx.set_cursor(CursorStyle::Auto);
    }
}

impl Default for LogoCard {
    fn default() -> Self {
        Self {
            is_compact: Property::new(false),
            logo_progress: Property::new(1.0),
        }
    }
}
