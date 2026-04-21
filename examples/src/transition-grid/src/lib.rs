#![allow(unused_imports)]

use pax_kit::*;
use rand::Rng;

pub mod cell_button;
pub mod hint_pill;
pub use cell_button::*;
pub use hint_pill::*;

const INITIAL_CELLS: usize = 20;
const GRID_COLUMNS: usize = 6;
const CELL_SIZE: f64 = 72.0;
const CELL_GAP: f64 = 16.0;
const CELL_STRIDE: f64 = CELL_SIZE + CELL_GAP;
const INITIAL_HUE: f64 = 188.0;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub cells: Property<Vec<CellData>>,
    pub next_id: Property<usize>,
    pub show_hint: Property<bool>,
    pub total_clicks: Property<usize>,
    pub recycle_count: Property<usize>,
}

#[pax]
#[custom(Defaults)]
pub struct CellData {
    pub id: usize,
    pub hue: f64,
    pub hits: usize,
    pub x: f64,
    pub y: f64,
}

impl Example {
    pub fn handle_mount(&mut self, _ctx: &NodeContext) {
        let mut cells: Vec<_> = (0..INITIAL_CELLS).map(new_cell).collect();
        layout_cells(&mut cells);
        self.cells.set(cells);
        self.next_id.set(INITIAL_CELLS);
        self.show_hint.set(true);
    }

    pub fn handle_cell_click(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        let Some(index) = clicked_repeat_index(ctx) else {
            return;
        };

        let mut cells = self.cells.get();
        if index >= cells.len() {
            return;
        }

        let mut rng = rand::thread_rng();
        self.show_hint.set(false);
        self.total_clicks.set(self.total_clicks.get() + 1);

        let cell = &mut cells[index];

        cell.hits += 1;
        cell.hue = rng.gen_range(0.0..360.0);

        if cell.hits >= 5 {
            cells.remove(index);
            let insert_at = rng.gen_range(0..=cells.len());
            let new_id = self.next_id.get();
            self.next_id.set(new_id + 1);
            cells.insert(insert_at, new_cell(new_id));
            layout_cells(&mut cells);
            self.recycle_count.set(self.recycle_count.get() + 1);
        }

        self.cells.set(cells);
    }
}

fn new_cell(id: usize) -> CellData {
    CellData {
        id,
        hue: INITIAL_HUE,
        hits: 0,
        x: 0.0,
        y: 0.0,
    }
}

fn layout_cells(cells: &mut [CellData]) {
    for (index, cell) in cells.iter_mut().enumerate() {
        cell.x = (index % GRID_COLUMNS) as f64 * CELL_STRIDE + CELL_SIZE / 2.0;
        cell.y = (index / GRID_COLUMNS) as f64 * CELL_STRIDE + CELL_SIZE / 2.0;
    }
}

fn clicked_repeat_index(ctx: &NodeContext) -> Option<usize> {
    ctx.slot_index.get().or_else(|| {
        ctx.local_stack_frame
            .resolve_symbol("i")
            .and_then(|variable| Numeric::try_coerce(variable.get_as_pax_value()).ok())
            .map(|numeric| numeric.to_float().round() as usize)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pax_kit::pax_engine::api::{PaxValue, ToPaxValue};

    #[test]
    fn cell_data_to_pax_value_keeps_all_fields() {
        let cell = CellData {
            id: 7,
            hue: 188.0,
            hits: 3,
            x: 124.0,
            y: 212.0,
        };

        assert_eq!(
            cell.to_pax_value(),
            PaxValue::Object(
                vec![
                    ("id".to_string(), 7usize.to_pax_value()),
                    ("hue".to_string(), 188.0f64.to_pax_value()),
                    ("hits".to_string(), 3usize.to_pax_value()),
                    ("x".to_string(), 124.0f64.to_pax_value()),
                    ("y".to_string(), 212.0f64.to_pax_value()),
                ]
                .into_iter()
                .collect()
            )
        );
    }

}
