use crate::*;
use pax_engine::api::*;
use pax_engine::*;
use std::cmp::Ordering;
use std::iter;

const PIXEL_ALIGN_FACTOR: f64 = 1.0;

#[pax]
#[inlined(
for i in 0..self.slots {
    slot(i)
}
<Rectangle fill=GREEN/>
@settings {
    @mount: on_mount,
}
)]
pub struct Table {
    pub rows: Property<usize>,
    pub columns: Property<usize>,
    pub slots: Property<usize>,
}

pub struct TableContext {
    rows: Property<usize>,
    columns: Property<usize>,
}

impl Store for TableContext {}

impl Table {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        ctx.push_local_store(TableContext {
            rows: self.rows.clone(),
            columns: self.columns.clone(),
        });
        let slot_children = ctx.slot_children_count.clone();
        let deps = [slot_children.untyped()];
        self.slots
            .replace_with(Property::computed(move || slot_children.get(), &deps));
    }
}

#[pax]
#[inlined(
<Group anchor_y=0% y={self.y_pos} height={self.height} width=100%>
    for i in 0..self.slots {
        slot(i)
    }
</Group>
@settings {
    @mount: on_mount,
}
)]
pub struct Row {
    pub y: Property<usize>,
    pub y_pos: Property<Size>,
    pub height: Property<Size>,
    pub slots: Property<usize>,
}

impl Row {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        ctx.peek_local_store(|table_ctx: &mut TableContext| {
            let rows = table_ctx.rows.clone();
            let deps = [rows.untyped()];
            self.height.replace_with(Property::computed(
                move || Size::Percent((PIXEL_ALIGN_FACTOR * 100.0 / rows.get() as f64).into()),
                &deps,
            ));
            let rows = table_ctx.rows.clone();
            let y = self.y.clone();
            let deps = [rows.untyped(), y.untyped()];
            self.y_pos.replace_with(Property::computed(
                move || {
                    Size::Percent(
                        (PIXEL_ALIGN_FACTOR * y.get() as f64 * 100.0 / rows.get() as f64).into(),
                    )
                },
                &deps,
            ));
        })
        .expect("rows can not exist outside a table");
        let slot_children = ctx.slot_children_count.clone();
        let deps = [slot_children.untyped()];
        self.slots
            .replace_with(Property::computed(move || slot_children.get(), &deps));
    }
}

#[pax]
#[inlined(
<Group anchor_x=0% x={self.x_pos} width={self.width} height=100%>
    for i in 0..self.slots {
        slot(i)
    }
</Group>
@settings {
    @mount: on_mount,
}
)]
pub struct Col {
    pub x: Property<usize>,
    pub x_pos: Property<Size>,
    pub width: Property<Size>,
    pub slots: Property<usize>,
}

impl Col {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        ctx.peek_local_store(|table_ctx: &mut TableContext| {
            let columns = table_ctx.columns.clone();
            let deps = [columns.untyped()];
            self.width.replace_with(Property::computed(
                move || Size::Percent((PIXEL_ALIGN_FACTOR * 100.0 / columns.get() as f64).into()),
                &deps,
            ));
            let columns = table_ctx.columns.clone();
            let x = self.x.clone();
            let deps = [columns.untyped(), x.untyped()];
            self.x_pos.replace_with(Property::computed(
                move || {
                    Size::Percent(
                        (PIXEL_ALIGN_FACTOR * x.get() as f64 * 100.0 / columns.get() as f64).into(),
                    )
                },
                &deps,
            ));
        })
        .expect("columns can not exist outside a table");
        let slot_children = ctx.slot_children_count.clone();
        let deps = [slot_children.untyped()];
        self.slots
            .replace_with(Property::computed(move || slot_children.get(), &deps));
    }
}

#[pax]
#[inlined(
<Group
    anchor_x=0%
    x={self.x_pos}
    width={self.width}
    anchor_y=0%
    y={self.y_pos}
    height={self.height}>

    for i in 0..self.slots {
        slot(i)
    }
</Group>
@settings {
    @mount: on_mount,
}
)]
pub struct Span {
    pub x: Property<usize>,
    pub y: Property<usize>,
    pub w: Property<usize>,
    pub h: Property<usize>,

    pub x_pos: Property<Size>,
    pub y_pos: Property<Size>,
    pub width: Property<Size>,
    pub height: Property<Size>,
    pub slots: Property<usize>,
}

impl Span {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        ctx.peek_local_store(|table_ctx: &mut TableContext| {
            let rows = table_ctx.rows.clone();
            let h = self.h.clone();
            let deps = [rows.untyped(), h.untyped()];
            self.height.replace_with(Property::computed(
                move || {
                    Size::Percent(
                        (PIXEL_ALIGN_FACTOR * h.get() as f64 * 100.0 / rows.get() as f64).into(),
                    )
                },
                &deps,
            ));
            let rows = table_ctx.rows.clone();
            let y = self.y.clone();
            let deps = [rows.untyped(), y.untyped()];
            self.y_pos.replace_with(Property::computed(
                move || {
                    Size::Percent(
                        (PIXEL_ALIGN_FACTOR * y.get() as f64 * 100.0 / rows.get() as f64).into(),
                    )
                },
                &deps,
            ));
            let columns = table_ctx.columns.clone();
            let w = self.w.clone();
            let deps = [columns.untyped(), w.untyped()];
            self.width.replace_with(Property::computed(
                move || {
                    Size::Percent(
                        (PIXEL_ALIGN_FACTOR * w.get() as f64 * 100.0 / columns.get() as f64).into(),
                    )
                },
                &deps,
            ));
            let columns = table_ctx.columns.clone();
            let x = self.x.clone();
            let deps = [columns.untyped(), x.untyped()];
            self.x_pos.replace_with(Property::computed(
                move || {
                    Size::Percent(
                        (PIXEL_ALIGN_FACTOR * x.get() as f64 * 100.0 / columns.get() as f64).into(),
                    )
                },
                &deps,
            ));
        })
        .expect("columns can not exist outside a table");
        let slot_children = ctx.slot_children_count.clone();
        let deps = [slot_children.untyped()];
        self.slots
            .replace_with(Property::computed(move || slot_children.get(), &deps));
    }
}

#[pax]
#[inlined(
<Group
    anchor_x=0%
    x={self.x_pos}
    width={self.width}
    anchor_y=0%
    y={self.y_pos}
    height={self.height}>

    for i in 0..self.slots {
        slot(i)
    }
</Group>
@settings {
    @mount: on_mount,
}
)]
pub struct Cell {
    pub x: Property<usize>,
    pub y: Property<usize>,

    pub x_pos: Property<Size>,
    pub y_pos: Property<Size>,
    pub width: Property<Size>,
    pub height: Property<Size>,
    pub slots: Property<usize>,
}

impl Cell {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        ctx.peek_local_store(|table_ctx: &mut TableContext| {
            let rows = table_ctx.rows.clone();
            let deps = [rows.untyped()];
            self.height.replace_with(Property::computed(
                move || Size::Percent((PIXEL_ALIGN_FACTOR * 100.0 / rows.get() as f64).into()),
                &deps,
            ));
            let rows = table_ctx.rows.clone();
            let y = self.y.clone();
            let deps = [rows.untyped(), y.untyped()];
            self.y_pos.replace_with(Property::computed(
                move || {
                    Size::Percent(
                        (PIXEL_ALIGN_FACTOR * y.get() as f64 * 100.0 / rows.get() as f64).into(),
                    )
                },
                &deps,
            ));
            let columns = table_ctx.columns.clone();
            let deps = [columns.untyped()];
            self.width.replace_with(Property::computed(
                move || Size::Percent((PIXEL_ALIGN_FACTOR * 100.0 / columns.get() as f64).into()),
                &deps,
            ));
            let columns = table_ctx.columns.clone();
            let x = self.x.clone();
            let deps = [columns.untyped(), x.untyped()];
            self.x_pos.replace_with(Property::computed(
                move || {
                    Size::Percent(
                        (PIXEL_ALIGN_FACTOR * x.get() as f64 * 100.0 / columns.get() as f64).into(),
                    )
                },
                &deps,
            ));
        })
        .expect("columns can not exist outside a table");
        let slot_children = ctx.slot_children_count.clone();
        let deps = [slot_children.untyped()];
        self.slots
            .replace_with(Property::computed(move || slot_children.get(), &deps));
    }
}
