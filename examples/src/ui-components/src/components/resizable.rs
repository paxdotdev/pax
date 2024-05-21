use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;
use std::cmp::Ordering;
use std::iter;

#[pax]
#[file("components/resizable.pax")]
pub struct Resizable {
    pub dividers: Property<Vec<Size>>,
    pub sections: Property<Vec<Section>>,

    // private
    pub index_moving: Property<Option<usize>>,
}

#[pax]
pub struct Section {
    pub x: Size,
    pub y: Size,
    pub width: Size,
    pub height: Size,
    pub i: usize,
}

impl Resizable {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let slot_count = ctx.slot_children_count.clone();
        let dividers = self.dividers.clone();
        let deps = [slot_count.untyped(), dividers.untyped()];

        self.sections.replace_with(Property::computed(
            move || {
                let divs = dividers.get();
                if slot_count.get() != divs.len() {
                    log::warn!("slots in Resizable doesn't match number of Segments")
                };

                let mut positions = Vec::new();
                positions.push(Size::ZERO());
                positions.extend(divs);
                positions.push(Size::default());
                let mut sections = vec![];
                for (i, w) in positions.windows(2).enumerate() {
                    sections.push(Section {
                        x: w[0].clone(),
                        width: w[1] - w[0],
                        y: Size::ZERO(),
                        height: Size::default(),
                        i,
                    })
                }
                log::debug!("{:#?}", sections);
                sections
            },
            &deps,
        ));
    }

    pub fn on_mouse_down(&mut self, ctx: &NodeContext, event: Event<MouseDown>) {
        let bounds = ctx.bounds_self.get();
        let (x, y) = (event.mouse.x, event.mouse.y);
        let (closest_ind, distance) = self
            .dividers
            .get()
            .iter()
            .map(|d| (d.evaluate(bounds, Axis::X) - x).abs())
            .enumerate()
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(Ordering::Equal))
            .unwrap_or_default();
        if distance < 10.0 {
            self.index_moving.set(Some(closest_ind));
            log::debug!("grabbing {:?}", closest_ind);
        }
    }

    pub fn on_mouse_move(&mut self, ctx: &NodeContext, event: Event<MouseMove>) {
        let x = event.mouse.x;
        let bounds = ctx.bounds_self.get();
        if let Some(ind) = self.index_moving.get() {
            self.dividers.update(|dividers| {
                let mut divider = &mut dividers[ind];
                *divider = match divider.clone() {
                    Size::Percent(_) => Size::Percent((100.0 * x / bounds.0).into()),
                    Size::Pixels(_) => Size::Pixels(x.into()),
                    Size::Combined(_, perc) => Size::Combined(
                        (x - Size::Percent(perc).evaluate(bounds, Axis::X)).into(),
                        perc,
                    ),
                };
            });
        }
    }

    pub fn on_mouse_up(&mut self, ctx: &NodeContext, event: Event<MouseUp>) {
        self.index_moving.set(None);
    }
}
