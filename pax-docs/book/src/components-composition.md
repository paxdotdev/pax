# Components & Composition
<!-- summary: Reusable components, props, and slot patterns. -->
<!-- tags: components, composition -->

- Defining components in Rust and pairing with `.pax` templates.
- Props and public fields: passing data into reusable components.
- Slots and child content: patterns from `slot-particles` and nested layouts.
- Composition of scenes: assembling complex UIs from smaller pieces.
- File organization: keeping component templates discoverable.

Runtime container semantics distinguish between received payload, encapsulated implementation structure, and projection as transport for `slot(...)`.

## Slots

Use `slot(index)` to project one received child by index from inside a component
template. Use `slot()` to project all received children not consumed by earlier
active slot sites in the same component.

`slot()` is useful for components that need a few fixed positions plus a
remainder bucket:

```pax
<Group>
    slot(0)
    slot(1)
    <Stacker>
        slot()
    </Stacker>
</Group>
```

If an explicit `slot(index)` is out of range or asks for a child already consumed
by an earlier slot site, it renders empty and the runtime warns. Empty `slot()`
is valid and silent.
