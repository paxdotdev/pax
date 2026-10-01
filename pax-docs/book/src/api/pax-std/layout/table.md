# layout::table
<!-- summary: API docs for pax-std::layout::table. -->
<!-- tags: api, pax-std -->

## Structs
### `Cell`
Selects one cell from a parent `Table`.

#### Properties
##### `x`
Type: `Property`<`usize`>

Zero-based column index.

##### `y`
Type: `Property`<`usize`>

Zero-based row index.

---

### `Col`
Selects one column from a parent `Table`.

#### Properties
##### `x`
Type: `Property`<`usize`>

Zero-based column index.

---

### `Row`
Selects one row from a parent `Table`.

#### Properties
##### `y`
Type: `Property`<`usize`>

Zero-based row index.

---

### `Span`
Selects a rectangular region from a parent `Table`.

#### Properties
##### `x`
Type: `Property`<`usize`>

Zero-based column index.

##### `y`
Type: `Property`<`usize`>

Zero-based row index.

##### `w`
Type: `Property`<`usize`>

Number of columns to span.

##### `h`
Type: `Property`<`usize`>

Number of rows to span.

---

### `Table`
A simple grid container for positioning children by rows and columns.

`Table` establishes row/column counts in local store. Child components such
as `Row`, `Col`, `Cell`, and `Span` read that context to size and position
their own slotted content.

#### Properties
##### `rows`
Type: `Property`<`usize`>

Number of table rows.

##### `columns`
Type: `Property`<`usize`>

Number of table columns.
