# Control Flow in Templates
<!-- summary: Declarative conditional rendering and iteration. -->
<!-- tags: control-flow, iteration -->

- `if` blocks for conditional UI.
- `for` loops over ranges and data collections.
- Loop variables and indexing patterns.
- Keyed `for` loops for stable identity across insertions, removals, and reordering.
- Nested loops for grids and repeated structures.
- Managing readability for deeply nested control flow.

## `if` Blocks

Use `if`, `else if`, and `else` blocks to include template children conditionally.
Conditions are PAXEL expressions, so they are declarative and side-effect free.

```pax
if self.is_open {
    <Panel />
} else {
    <Button text="Open" />
}
```

## `for` Loops

Use `for` to repeat template children over a range or collection.
The loop body receives the current item, and may also receive the current index.

```pax
for item in self.items {
    <Text text={item.label} />
}

for (item, i) in self.items {
    <Text text={item.label} opacity={i == 0 ? 1 : 0.8} />
}
```

## Keyed Loops

When repeated children can be inserted, removed, or reordered, add a `key` clause at the end of the `for` header.
The key expression is evaluated for each item and must produce a string or integer.

```pax
for (item, i) in self.items key item.id {
    <Row label={item.label} />
}
```

Keys give Pax a stable identity for each repeated child group.  When the collection changes, children with the same key are reused and moved into the new order.  Children whose keys disappear are removed, and if they have `@out` transitions, they remain mounted until the exit transition completes.  Children with new keys are mounted as new instances and may play `@in` transitions.

Use keys for lists where component state or lifecycle transitions should follow the item, not the item's current index.  Unkeyed loops are positional: they are suitable for simple ranges, fixed-size grids, and repeated decoration where index identity is enough.
