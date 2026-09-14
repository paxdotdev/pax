#![allow(unused_imports)]

use pax_kit::pax_engine::api::PaxValue;
use pax_kit::*;
use rand::Rng;

pub mod cell_button;
pub mod hint_pill;
pub use cell_button::*;
pub use hint_pill::*;

const INITIAL_CELLS: usize = 7;
const INITIAL_HUE: f64 = 188.0;
const TILE_HEIGHT: f64 = 56.0;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub cells: Property<Vec<CellData>>,
    pub cell_count: Property<usize>,
    pub cell_sizes: Property<Vec<Option<Size>>>,
    pub next_id: Property<usize>,
    pub total_clicks: Property<usize>,
    pub recycle_count: Property<usize>,
    pub exit_mode: Property<ContainerExitMode>,
    pub reflow_kind: Property<ContainerReflowTransitionKind>,
    pub reflow_curve: Property<ContainerReflowCurve>,
    pub exit_mode_label: Property<String>,
    pub reflow_mode_label: Property<String>,
}

#[pax]
#[custom(Defaults)]
pub struct CellData {
    pub id: usize,
    pub label: String,
    pub hue: f64,
    pub hits: usize,
}

impl Example {
    pub fn handle_mount(&mut self, _ctx: &NodeContext) {
        let cells: Vec<_> = (1..=INITIAL_CELLS).map(new_cell).collect();
        self.set_cells(cells);
        self.next_id.set(INITIAL_CELLS + 1);
        self.set_exit_mode(ContainerExitMode::Flow);
        self.set_reflow_mode(ContainerReflowTransitionKind::Ease);
        self.reflow_curve.set(ContainerReflowCurve::InOutQuad);
    }

    pub fn handle_cell_click(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        let Some(cell_id) = clicked_cell_id(ctx) else {
            return;
        };

        let mut cells = self.cells.get();
        let Some(index) = cells.iter().position(|cell| cell.id == cell_id) else {
            return;
        };

        let mut rng = rand::thread_rng();
        self.total_clicks.set(self.total_clicks.get() + 1);

        let cell = &mut cells[index];
        cell.hits += 1;
        cell.hue = rng.gen_range(0.0..360.0);

        if cell.hits >= 5 {
            cells.remove(index);
            let new_id = self.next_id.get();
            self.next_id.set(new_id + 1);
            cells.push(new_cell(new_id));
            self.recycle_count.set(self.recycle_count.get() + 1);
        }

        self.set_cells(cells);
    }

    pub fn handle_insert_top(&mut self, _ctx: &NodeContext, _args: Event<Click>) {
        let mut cells = self.cells.get();
        let new_id = self.next_id.get();
        self.next_id.set(new_id + 1);
        cells.insert(0, new_cell(new_id));
        self.set_cells(cells);
    }

    pub fn handle_remove_top(&mut self, _ctx: &NodeContext, _args: Event<Click>) {
        let mut cells = self.cells.get();
        if cells.is_empty() {
            return;
        }

        cells.remove(0);
        self.set_cells(cells);
    }

    pub fn handle_reverse_order(&mut self, _ctx: &NodeContext, _args: Event<Click>) {
        let mut cells = self.cells.get();
        cells.reverse();
        self.set_cells(cells);
    }

    pub fn set_exit_flow(&mut self, _ctx: &NodeContext, _args: Event<Click>) {
        self.set_exit_mode(ContainerExitMode::Flow);
    }

    pub fn set_exit_ghost(&mut self, _ctx: &NodeContext, _args: Event<Click>) {
        self.set_exit_mode(ContainerExitMode::Ghost);
    }

    pub fn set_change_snap(&mut self, _ctx: &NodeContext, _args: Event<Click>) {
        self.set_reflow_mode(ContainerReflowTransitionKind::Snap);
    }

    pub fn set_change_ease(&mut self, _ctx: &NodeContext, _args: Event<Click>) {
        self.set_reflow_mode(ContainerReflowTransitionKind::Ease);
    }

    fn set_cells(&mut self, cells: Vec<CellData>) {
        self.cell_count.set(cells.len());
        self.cell_sizes.set(vec![
            Some(Size::Pixels(Numeric::F64(TILE_HEIGHT)));
            cells.len()
        ]);
        self.cells.set(cells);
    }

    fn set_exit_mode(&mut self, mode: ContainerExitMode) {
        self.exit_mode.set(mode.clone());
        self.exit_mode_label.set(match mode {
            ContainerExitMode::Flow => "Flow".to_string(),
            ContainerExitMode::Ghost => "Ghost".to_string(),
        });
    }

    fn set_reflow_mode(&mut self, mode: ContainerReflowTransitionKind) {
        self.reflow_kind.set(mode.clone());
        self.reflow_mode_label.set(match mode {
            ContainerReflowTransitionKind::Snap => "Snap".to_string(),
            ContainerReflowTransitionKind::Ease => "Ease".to_string(),
            ContainerReflowTransitionKind::Named => "Named".to_string(),
        });
    }
}

fn new_cell(id: usize) -> CellData {
    CellData {
        id,
        label: format!("Tile {}", id),
        hue: INITIAL_HUE,
        hits: 0,
    }
}

fn clicked_cell_id(ctx: &NodeContext) -> Option<usize> {
    ctx.local_stack_frame
        .resolve_symbol("cell")
        .map(|variable| variable.get_as_pax_value())
        .and_then(extract_cell_id_from_value)
}

fn extract_cell_id_from_value(value: PaxValue) -> Option<usize> {
    match value {
        PaxValue::Object(fields) => fields
            .into_iter()
            .find_map(|(name, value)| (name == "id").then_some(value))
            .and_then(|value| usize::try_coerce(value).ok()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pax_kit::pax_engine::api::{PaxValue, ToPaxValue};

    #[test]
    fn cell_data_to_pax_value_keeps_all_fields() {
        let cell = CellData {
            id: 7,
            label: "Tile 7".to_string(),
            hue: 188.0,
            hits: 3,
        };

        assert_eq!(
            cell.to_pax_value(),
            PaxValue::Object(
                vec![
                    ("id".to_string(), 7usize.to_pax_value()),
                    ("label".to_string(), "Tile 7".to_string().to_pax_value()),
                    ("hue".to_string(), 188.0f64.to_pax_value()),
                    ("hits".to_string(), 3usize.to_pax_value()),
                ]
                .into_iter()
                .collect()
            )
        );
    }

    #[test]
    fn extract_cell_id_from_value_reads_id_field() {
        let value = PaxValue::Object(
            vec![
                ("id".to_string(), 11usize.to_pax_value()),
                ("label".to_string(), "Tile 11".to_string().to_pax_value()),
            ]
            .into_iter()
            .collect(),
        );

        assert_eq!(extract_cell_id_from_value(value), Some(11));
    }

    #[test]
    fn recycling_appends_new_tile_to_tail() {
        let mut cells = vec![new_cell(1), new_cell(2), new_cell(3)];
        cells[0].hits = 5;
        cells.remove(0);
        cells.push(new_cell(4));

        let ids: Vec<_> = cells.into_iter().map(|cell| cell.id).collect();
        assert_eq!(ids, vec![2, 3, 4]);
    }

    #[test]
    fn reversing_order_is_deterministic() {
        let mut cells = vec![new_cell(1), new_cell(2), new_cell(3)];
        cells.reverse();

        let ids: Vec<_> = cells.into_iter().map(|cell| cell.id).collect();
        assert_eq!(ids, vec![3, 2, 1]);
    }
}
