use crate::PaxText;
use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;
use std::cmp::Ordering;
use std::iter;

#[pax]
#[inlined(
for i in 0..self.slot_children {
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
    pub slot_children: Property<usize>,
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
        self.slot_children
            .replace_with(Property::computed(move || slot_children.get(), &deps));
    }
}

#[pax]
#[inlined(
<Group anchor_y=0% y={self.y_pos} height={self.height} width=100%>
    for i in 0..self.slot_children {
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
    pub slot_children: Property<usize>,
}

impl Row {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        ctx.peek_local_store(|table_ctx: &mut TableContext| {
            let rows = table_ctx.rows.clone();
            let deps = [rows.untyped()];
            self.height.replace_with(Property::computed(
                move || Size::Percent((100.0 / rows.get() as f64).into()),
                &deps,
            ));
            let rows = table_ctx.rows.clone();
            let y = self.y.clone();
            let deps = [rows.untyped(), y.untyped()];
            self.y_pos.replace_with(Property::computed(
                move || Size::Percent((100.0 / rows.get() as f64 * y.get() as f64).into()),
                &deps,
            ));
        })
        .expect("rows can not exist outside a table");
        let slot_children = ctx.slot_children_count.clone();
        let deps = [slot_children.untyped()];
        self.slot_children
            .replace_with(Property::computed(move || slot_children.get(), &deps));
    }
}
