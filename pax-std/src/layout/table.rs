#[allow(unused)]
use crate::*;
use pax_engine::api::*;
use pax_engine::*;

const PIXEL_ALIGN_FACTOR: f64 = 1.0;

/// A simple grid container for positioning children by rows and columns.
///
/// `Table` establishes row/column counts in local store. Child components such
/// as `Row`, `Col`, `Cell`, and `Span` read that context to size and position
/// their own slotted content.
#[pax]
#[engine_import_path("pax_engine")]
#[inlined(
for i in 0..self._slots {
    slot(i)
}
<Rectangle fill=GREEN/>
@settings {
    @mount: on_mount,
}
)]
pub struct Table {
    /// Number of table rows.
    pub rows: Property<usize>,
    /// Number of table columns.
    pub columns: Property<usize>,
    // Slot count mirrored from `NodeContext`.
    pub _slots: Property<usize>,
}

// Local table geometry shared with row/column child components.
pub struct TableContext {
    rows: Property<usize>,
    columns: Property<usize>,
}

impl Store for TableContext {}

impl Table {
    // Publishes row/column counts for child layout helpers.
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        ctx.push_local_store(TableContext {
            rows: self.rows.clone(),
            columns: self.columns.clone(),
        });
        let slot_children = ctx.slot_children_count.clone();
        let deps = [slot_children.untyped()];
        self._slots
            .replace_with(Property::computed(move || slot_children.get(), &deps));
    }
}

/// Selects one row from a parent `Table`.
#[pax]
#[engine_import_path("pax_engine")]
#[inlined(
<Group anchor_y=0% y={self._y_pos} height={self._height} width=100%>
    for i in 0..self._slots {
        slot(i)
    }
</Group>
@settings {
    @mount: on_mount,
}
)]
pub struct Row {
    /// Zero-based row index.
    pub y: Property<usize>,
    // Computed y position.
    pub _y_pos: Property<Size>,
    // Computed row height.
    pub _height: Property<Size>,
    // Slot count mirrored from `NodeContext`.
    pub _slots: Property<usize>,
}

impl Row {
    // Sizes and positions this row from the parent table context.
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        ctx.peek_local_store(|table_ctx: &mut TableContext| {
            let rows = table_ctx.rows.clone();
            let deps = [rows.untyped()];
            self._height.replace_with(Property::computed(
                move || Size::Percent((PIXEL_ALIGN_FACTOR * 100.0 / rows.get() as f64).into()),
                &deps,
            ));
            let rows = table_ctx.rows.clone();
            let y = self.y.clone();
            let deps = [rows.untyped(), y.untyped()];
            self._y_pos.replace_with(Property::computed(
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
        self._slots
            .replace_with(Property::computed(move || slot_children.get(), &deps));
    }
}

/// Selects one column from a parent `Table`.
#[pax]
#[engine_import_path("pax_engine")]
#[inlined(
<Group anchor_x=0% x={self._x_pos} width={self._width} height=100%>
    for i in 0..self._slots {
        slot(i)
    }
</Group>
@settings {
    @mount: on_mount,
}
)]
pub struct Col {
    /// Zero-based column index.
    pub x: Property<usize>,
    // Computed x position.
    pub _x_pos: Property<Size>,
    // Computed column width.
    pub _width: Property<Size>,
    // Slot count mirrored from `NodeContext`.
    pub _slots: Property<usize>,
}

impl Col {
    // Sizes and positions this column from the parent table context.
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        ctx.peek_local_store(|table_ctx: &mut TableContext| {
            let columns = table_ctx.columns.clone();
            let deps = [columns.untyped()];
            self._width.replace_with(Property::computed(
                move || Size::Percent((PIXEL_ALIGN_FACTOR * 100.0 / columns.get() as f64).into()),
                &deps,
            ));
            let columns = table_ctx.columns.clone();
            let x = self.x.clone();
            let deps = [columns.untyped(), x.untyped()];
            self._x_pos.replace_with(Property::computed(
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
        self._slots
            .replace_with(Property::computed(move || slot_children.get(), &deps));
    }
}

/// Selects a rectangular region from a parent `Table`.
#[pax]
#[engine_import_path("pax_engine")]
#[inlined(
<Group
    anchor_x=0%
    x={self._x_pos}
    width={self._width}
    anchor_y=0%
    y={self._y_pos}
    height={self._height}>

    for i in 0..self._slots {
        slot(i)
    }
</Group>
@settings {
    @mount: on_mount,
}
)]
pub struct Span {
    /// Zero-based column index.
    pub x: Property<usize>,
    /// Zero-based row index.
    pub y: Property<usize>,
    /// Number of columns to span.
    pub w: Property<usize>,
    /// Number of rows to span.
    pub h: Property<usize>,

    // Computed x position.
    pub _x_pos: Property<Size>,
    // Computed y position.
    pub _y_pos: Property<Size>,
    // Computed span width.
    pub _width: Property<Size>,
    // Computed span height.
    pub _height: Property<Size>,
    // Slot count mirrored from `NodeContext`.
    pub _slots: Property<usize>,
}

impl Span {
    // Sizes and positions this span from the parent table context.
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        ctx.peek_local_store(|table_ctx: &mut TableContext| {
            let rows = table_ctx.rows.clone();
            let h = self.h.clone();
            let deps = [rows.untyped(), h.untyped()];
            self._height.replace_with(Property::computed(
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
            self._y_pos.replace_with(Property::computed(
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
            self._width.replace_with(Property::computed(
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
            self._x_pos.replace_with(Property::computed(
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
        self._slots
            .replace_with(Property::computed(move || slot_children.get(), &deps));
    }
}

/// Selects one cell from a parent `Table`.
#[pax]
#[engine_import_path("pax_engine")]
#[inlined(
<Group
    anchor_x=0%
    x={self._x_pos}
    width={self._width}
    anchor_y=0%
    y={self._y_pos}
    height={self._height}>

    for i in 0..self._slots {
        slot(i)
    }
</Group>
@settings {
    @mount: on_mount,
}
)]
pub struct Cell {
    /// Zero-based column index.
    pub x: Property<usize>,
    /// Zero-based row index.
    pub y: Property<usize>,

    // Computed x position.
    pub _x_pos: Property<Size>,
    // Computed y position.
    pub _y_pos: Property<Size>,
    // Computed cell width.
    pub _width: Property<Size>,
    // Computed cell height.
    pub _height: Property<Size>,
    // Slot count mirrored from `NodeContext`.
    pub _slots: Property<usize>,
}

impl Cell {
    // Sizes and positions this cell from the parent table context.
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        ctx.peek_local_store(|table_ctx: &mut TableContext| {
            let rows = table_ctx.rows.clone();
            let deps = [rows.untyped()];
            self._height.replace_with(Property::computed(
                move || Size::Percent((PIXEL_ALIGN_FACTOR * 100.0 / rows.get() as f64).into()),
                &deps,
            ));
            let rows = table_ctx.rows.clone();
            let y = self.y.clone();
            let deps = [rows.untyped(), y.untyped()];
            self._y_pos.replace_with(Property::computed(
                move || {
                    Size::Percent(
                        (PIXEL_ALIGN_FACTOR * y.get() as f64 * 100.0 / rows.get() as f64).into(),
                    )
                },
                &deps,
            ));
            let columns = table_ctx.columns.clone();
            let deps = [columns.untyped()];
            self._width.replace_with(Property::computed(
                move || Size::Percent((PIXEL_ALIGN_FACTOR * 100.0 / columns.get() as f64).into()),
                &deps,
            ));
            let columns = table_ctx.columns.clone();
            let x = self.x.clone();
            let deps = [columns.untyped(), x.untyped()];
            self._x_pos.replace_with(Property::computed(
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
        self._slots
            .replace_with(Property::computed(move || slot_children.get(), &deps));
    }
}
