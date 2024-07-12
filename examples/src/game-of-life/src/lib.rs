#![allow(unused_imports)]

use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

mod cell;
use cell::Cell;

#[pax]
#[custom(Default)]
#[main]
#[file("lib.pax")]
pub struct GameOfLife {
    pub cells: Property<Vec<Vec<bool>>>,
    pub rows: Property<usize>,
    pub cols: Property<usize>,
    pub running: Property<bool>,
}

impl Default for GameOfLife {
    fn default() -> Self {
        let n = 10;
        Self {
            cells: Property::new(vec![vec![false; n]; n]),
            rows: Property::new(n),
            cols: Property::new(n),
            running: Property::new(false),
        }
    }

}

impl GameOfLife {
    pub fn tick(&mut self, _ctx: &NodeContext) {
        if self.running.get() {
            let mut new_cells = vec![vec![false; self.cols.get()]; self.rows.get()];
            for i in 0..self.rows.get() {
                for j in 0..self.cols.get() {
                    let mut count = 0;
                    for x in -1..=1 {
                        for y in -1..=1 {
                            if x == 0 && y == 0 {
                                continue;
                            }
                            let ni = (i as isize + x + self.rows.get() as isize) as usize % self.rows.get();
                            let nj = (j as isize + y + self.cols.get() as isize) as usize % self.cols.get();
                            if self.cells.get()[ni][nj] {
                                count += 1;
                            }
                        }
                    }
                    new_cells[i][j] = match (self.cells.get()[i][j], count) {
                        (true, 2) | (true, 3) => true,
                        (false, 3) => true,
                        _ => false,
                    };
                }
            }
            if new_cells == self.cells.get() {
                self.running.set(false);
            } else {
                self.cells.set(new_cells);
            }
        }
    }

    pub fn start(&mut self, _ctx: &NodeContext, _args: Event<ButtonClick>) {
        self.running.set(true);
    }

    pub fn stop(&mut self, _ctx: &NodeContext, _args: Event<ButtonClick>) {
        self.running.set(false);
    }

    pub fn reset(&mut self, _ctx: &NodeContext, _args: Event<ButtonClick>) {
        self.running.set(false);
        self.cells.set(vec![vec![false; self.cols.get()]; self.rows.get()]);
    }
}