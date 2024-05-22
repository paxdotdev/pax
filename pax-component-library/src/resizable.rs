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
#[inlined(
    <Group
        @mouse_down=on_mouse_down
        @mouse_move=on_mouse_move
        @mouse_up=on_mouse_up
    >
        for s in self.sections {
            <Group
                x={s.x}
                y={s.y}
                width={s.width}
                height={s.height}
                anchor_x=0%
                anchor_y=0%
            >
                slot(s.i)
            </Group>
        }
        <Rectangle fill=rgb(20, 20, 20)/>
    </Group>

    @settings {
        @mount: on_mount
    }
)]
pub struct Resizable {
    pub dividers: Property<Vec<Size>>,
    pub direction: Property<ResizableDirection>,

    // private
    pub sections: Property<Vec<Section>>,
    pub index_moving: Property<Option<usize>>,
}

#[pax]
pub enum ResizableDirection {
    Vertical,
    #[default]
    Horizontal,
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
        let direction = self.direction.clone();
        let deps = [
            slot_count.untyped(),
            dividers.untyped(),
            direction.untyped(),
        ];

        self.sections.replace_with(Property::computed(
            move || {
                let divs = dividers.get();
                if slot_count.get() != divs.len() + 1 {
                    log::warn!("slots in Resizable doesn't match number of Segments")
                };

                let mut positions = Vec::new();
                positions.push(Size::ZERO());
                positions.extend(divs);
                positions.push(Size::default());
                let mut sections = vec![];
                for (i, w) in positions.windows(2).enumerate() {
                    let (pos_main, extent_main) = (w[0].clone(), w[1] - w[0]);
                    let (pos_off, extent_off) = (Size::ZERO(), Size::default());
                    let (x, width, y, height) = match direction.get() {
                        ResizableDirection::Vertical => {
                            (pos_off, extent_off, pos_main, extent_main)
                        }
                        ResizableDirection::Horizontal => {
                            (pos_main, extent_main, pos_off, extent_off)
                        }
                    };
                    sections.push(Section {
                        x,
                        width,
                        y,
                        height,
                        i,
                    })
                }
                sections
            },
            &deps,
        ));
    }

    pub fn on_mouse_down(&mut self, ctx: &NodeContext, event: Event<MouseDown>) {
        let bounds = ctx.bounds_self.get();

        let (dim, axis) = match self.direction.get() {
            ResizableDirection::Vertical => (event.mouse.y, Axis::Y),
            ResizableDirection::Horizontal => (event.mouse.x, Axis::X),
        };
        let (closest_ind, distance) = self
            .dividers
            .get()
            .iter()
            .map(|d| (d.evaluate(bounds, axis) - dim).abs())
            .enumerate()
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(Ordering::Equal))
            .unwrap_or_default();
        if distance < 10.0 {
            self.index_moving.set(Some(closest_ind));
        }
    }

    pub fn on_mouse_move(&mut self, ctx: &NodeContext, event: Event<MouseMove>) {
        let bounds = ctx.bounds_self.get();

        let (dim, bound) = match self.direction.get() {
            ResizableDirection::Vertical => (event.mouse.y, bounds.1),
            ResizableDirection::Horizontal => (event.mouse.x, bounds.0),
        };

        if let Some(ind) = self.index_moving.get() {
            self.dividers.update(|dividers| {
                let divider = &mut dividers[ind];
                *divider = match divider.clone() {
                    Size::Percent(_) => Size::Percent((100.0 * dim / bound).into()),
                    Size::Pixels(_) => Size::Pixels(dim.into()),
                    Size::Combined(_, perc) => {
                        Size::Combined((dim - perc.to_float() * bound / 100.0).into(), perc)
                    }
                };
            });
        }
    }

    pub fn on_mouse_up(&mut self, _ctx: &NodeContext, _event: Event<MouseUp>) {
        self.index_moving.set(None);
    }
}
