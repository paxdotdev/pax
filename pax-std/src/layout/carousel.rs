#![allow(unused_imports)]
use crate::*;
use pax_engine::api::{Click, EasingCurve, Event};
use pax_engine::*;
use pax_runtime::api::NodeContext;

#[pax]
#[engine_import_path("pax_engine")]
#[file("layout/carousel.pax")]
pub struct Carousel {
    pub ticks: Property<usize>,
    pub cell_specs: Property<Vec<CarouselCell>>,
    pub current_cell: Property<usize>,
    pub current_cell_on_change: Property<bool>,
    pub transition: Property<f64>,
}

#[pax]
#[engine_import_path("pax_engine")]
pub struct CarouselCell {
    pub is_active: bool,
    pub x_percent: f64,
}

impl Carousel {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let slot_children_count = ctx.slot_children_count.clone();
        let current_cell = self.current_cell.clone();
        let transition = self.transition.clone();

        let deps = [
            slot_children_count.untyped(),
            current_cell.untyped(),
            transition.untyped(),
        ];

        self.cell_specs.replace_with(Property::computed(
            move || {
                let slot_children_count = slot_children_count.get();
                let current_cell = current_cell.get();
                let transition = transition.get();

                let mut cell_specs = vec![];
                for i in 0..slot_children_count {
                    let is_active = (i as isize) >= (current_cell as isize) - 1
                        && (i as isize) <= (current_cell as isize) + 1;
                    // let is_active = true;

                    let x_percent = 50.0 + ((i as f64 * 100.0) - transition);

                    cell_specs.push(CarouselCell {
                        is_active,
                        x_percent,
                    });
                }

                cell_specs
            },
            &deps,
        ));

        let current_cell = self.current_cell.clone();
        let transition = self.transition.clone();
        let deps = [current_cell.untyped()];

        self.current_cell_on_change.replace_with(Property::computed(
            move || {
                let current_cell = current_cell.get();
                transition.ease_to(current_cell as f64 * 100.0, 60, EasingCurve::OutQuad);
                false
            },
            &deps,
        ));
    }

    pub fn update(&mut self, _ctx: &NodeContext) {
        self.current_cell_on_change.get();
    }

    pub fn increment(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        let current_cell = self.current_cell.get();
        let children_count = ctx.slot_children_count.get();
        self.current_cell.set((current_cell + 1) % children_count);
    }

    pub fn decrement(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        let current_cell = self.current_cell.get();
        let children_count = ctx.slot_children_count.get();
        if current_cell != 0 {
            self.current_cell.set((current_cell - 1) % children_count);
        } else {
            self.current_cell.set(children_count - 1);
        }
    }
}
