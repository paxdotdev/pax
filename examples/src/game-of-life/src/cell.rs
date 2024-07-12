#![allow(unused_imports)]
#![allow(dead_code)]

use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

#[pax]
#[file("cell.pax")]
pub struct Cell {
    pub on : Property<bool>,
    pub row: Property<usize>,
    pub col: Property<usize>,
    pub cells: Property<Vec<Vec<bool>>>,
    pub color: Property<Color>,
}

impl Cell {

    pub fn mount(&mut self, _ctx: &NodeContext) {
        let cells = self.cells.clone();
        let row = self.row.clone();
        let col = self.col.clone();
        self.on.replace_with(Property::computed(move || {
            cells.get()[row.get()][col.get()]
        }, &[self.cells.untyped()]));
    }

    pub fn toggle(&mut self, _ctx: &NodeContext, _args: Event<Click>) {
        self.cells.update(|cells: &mut Vec<Vec<bool>>| {
            cells[self.row.get()][self.col.get()] = !cells[self.row.get()][self.col.get()];
        });
    }
}