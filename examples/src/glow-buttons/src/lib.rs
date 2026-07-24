#![allow(unused_imports)]

use pax_kit::*;

pub mod glow_button;
pub use glow_button::GlowButton;

const COMPACT_BREAKPOINT: f64 = 700.0;
const BUTTON_HEIGHT: f64 = 118.0;
const GRID_GUTTER: f64 = 18.0;
const FOOTER_GAP: f64 = 28.0;
const FOOTER_HEIGHT: f64 = 34.0;
const BOTTOM_PADDING: f64 = 40.0;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub rows: Property<Vec<GlowButtonRow>>,
    pub scroll_y: Property<f64>,
    pub is_compact: Property<bool>,
    pub grid_height: Property<f64>,
    pub footer_y: Property<f64>,
    pub content_height: Property<f64>,
    pub total_presses: Property<usize>,
    pub background_material: Property<Material>,
}

#[pax]
#[custom(Defaults)]
pub struct GlowButtonData {
    pub id: usize,
    pub label: String,
    pub base_color: Color,
    pub glow_color: Color,
}

#[pax]
#[custom(Defaults)]
pub struct GlowButtonRow {
    pub id: usize,
    pub buttons: Vec<GlowButtonData>,
}

impl Example {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.background_material.set(Material::unlit());
        self.sync_layout(ctx);
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        self.sync_layout(ctx);
    }

    pub fn record_press(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.total_presses.set(self.total_presses.get() + 1);
    }

    fn sync_layout(&mut self, ctx: &NodeContext) {
        let (width, _) = ctx.bounds_self.get();
        let is_compact = width < COMPACT_BREAKPOINT;
        if self.rows.get().is_empty() || self.is_compact.get() != is_compact {
            let columns = if is_compact { 2 } else { 3 };
            let rows = make_rows(columns);
            let row_count = rows.len() as f64;
            self.rows.set(rows);
            self.is_compact.set(is_compact);
            self.grid_height
                .set(row_count * BUTTON_HEIGHT + (row_count - 1.0) * GRID_GUTTER);
            let grid_top = if is_compact { 215.0 } else { 202.0 };
            let footer_y = grid_top + self.grid_height.get() + FOOTER_GAP;
            self.footer_y.set(footer_y);
            self.content_height
                .set(footer_y + FOOTER_HEIGHT + BOTTOM_PADDING);
        }
    }
}

fn make_rows(columns: usize) -> Vec<GlowButtonRow> {
    let mut buttons = button_palette().into_iter();
    let mut rows = Vec::new();
    let mut row_id = 0;

    loop {
        let row_buttons = buttons.by_ref().take(columns).collect::<Vec<_>>();
        if row_buttons.is_empty() {
            break;
        }
        rows.push(GlowButtonRow {
            id: row_id,
            buttons: row_buttons,
        });
        row_id += 1;
    }

    rows
}

fn button_palette() -> Vec<GlowButtonData> {
    [
        ("Ruby", "3A1420", "FF4272"),
        ("Tangerine", "3C2113", "FF8A3D"),
        ("Solar", "393013", "FFD84A"),
        ("Verdant", "123326", "42E89A"),
        ("Lagoon", "11343A", "3FE8E0"),
        ("Aether", "122B42", "4DC7FF"),
        ("Cobalt", "151F48", "607CFF"),
        ("Iris", "241847", "9E6CFF"),
        ("Orchid", "391943", "DE68FF"),
        ("Fuchsia", "42172E", "FF5EC4"),
        ("Rosewater", "3E2028", "FF8FA8"),
        ("Moonstone", "26303A", "B7D8FF"),
    ]
    .into_iter()
    .enumerate()
    .map(|(id, (label, base, glow))| GlowButtonData {
        id,
        label: label.to_string(),
        base_color: Color::from_hex(base),
        glow_color: Color::from_hex(glow),
    })
    .collect()
}
