<a id="components--composition"></a>

# Components and Composition
<!-- summary: Build reusable components, compose caller-provided content, preserve identity, and share state. -->
<!-- tags: components, composition, control-flow, slots, stores -->

A panel becomes more useful when it can appear in several places, with a
different title or progress value each time. A component gives that panel a
name and an interface: the values it accepts, the actions it offers, and the
content its caller can supply.

This chapter extracts the Field notes panel from [Properties](state-properties.md)
and [Events and Rust](event-handling-rust.md) into a small reusable card. It
then builds on the same boundary to explain conditional views, lists, slots,
and shared state. If you are new to Pax source, start with
[Templates](template-language.md#reading-a-template).

## Make a reusable component

Create `src/note_card.rs` and `src/note_card.pax` beside your application's
`lib.rs` and `lib.pax`:

```text
src/
├── lib.rs          Notes: application state and actions
├── lib.pax         places cards and supplies their inputs
├── note_card.rs    NoteCard: its public properties and defaults
└── note_card.pax   the card's internal interface
```

In `src/note_card.rs`, declare the card's inputs:

```rust
use pax_kit::*;

#[pax]
#[file("note_card.pax")]
#[custom(Default)]
pub struct NoteCard {
    pub title: Property<String>,
    pub progress: Property<f64>,
}

impl Default for NoteCard {
    fn default() -> Self {
        Self {
            title: Property::new("Untitled note".to_string()),
            progress: Property::new(0.0),
        }
    }
}
```

The type has `#[pax]` and an associated template, like the main component.
Only the application root has `#[main]`. Public properties form the card's
value interface; here progress uses a zero-to-one range. The custom default
provides useful values when a caller omits an input. Each new card instance
gets its own initial properties.

Give `src/note_card.pax` the display portion of the panel:

```pax
<Text x=24px y=24px width={100% - 48px} height=36px
    text={self.title}
    style={font_size: 24px, fill: rgb(32, 40, 48)}
/>
<Text x=24px y=72px width={100% - 48px} height=28px
    text={"Progress: " + (self.progress * 100) + "%"}
    style={font_size: 16px, fill: rgb(72, 80, 88)}
/>
<Group x=24px y=116px width={100% - 48px} height=12px>
    <Rectangle width={(self.progress * 100)%} height=100%
        fill=rgb(55, 120, 90) corner_radius=6
    />
    <Rectangle width=100% height=100%
        fill=rgb(220, 216, 206) corner_radius=6
    />
</Group>
<Rectangle width=100% height=100%
    fill=rgb(246, 241, 230) corner_radius=12
/>
```

The card uses the layout area supplied by its caller. The percentage widths
follow that area; the fixed offsets give this example its padding. Its
background remains last so the text and bar render in front. For sizing and
container choices, read [Layout and Responsiveness](layout-responsiveness.md).

### Use the card

Here is a complete `src/lib.rs` for the two-card example:

```rust
use pax_kit::*;

pub mod note_card;
pub use note_card::NoteCard;

#[pax]
#[main]
#[file("lib.pax")]
#[custom(Default)]
pub struct Notes {
    pub title: Property<String>,
    pub progress: Property<f64>,
}

impl Default for Notes {
    fn default() -> Self {
        Self {
            title: Property::new("Field notes".to_string()),
            progress: Property::new(0.25),
        }
    }
}

impl Notes {
    pub fn advance(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.progress.set((self.progress.get() + 0.25).min(1.0));
    }
}
```

`pub mod` includes the Rust module, and `pub use` makes `NoteCard` available
at the crate root. This is a straightforward pattern for a project's reusable
components. The `.pax` file is associated through `#[file(...)]`; it needs no
separate Rust module declaration.

Template paths are looked up from the package root, with a `src/` fallback.
If you move the pair into a deeper directory, update that path explicitly,
for example `#[file("src/cards/note_card.pax")]`, along with the Rust module
wiring. The path is not relative to the Rust file containing the attribute.

In `src/lib.pax`, place two cards and an application-level button:

```pax
<Button x=24px y=24px width=160px height=28px
    label="Advance" @button_click=self.advance
/>
<NoteCard x=24px y=72px width=320px height=160px
    title={self.title} progress={self.progress}
/>
<NoteCard x=24px y=248px width=320px height=160px
    title="Sketchbook" progress=0.75
/>
```

The first card starts at 25 percent and follows the application's progress.
The second displays 75 percent independently. Pressing Advance changes only
the first card. Remove the second card's title and progress attributes to try
its defaults: “Untitled note” and zero progress.

Restart the app after adding the Rust files and declarations. The default
hot-reload lane updates template edits; see
[Developer Workflow](developer-workflow.md#hot-reloading) for the application-logic
reload options.

## Inputs and state ownership

In `lib.pax`, `{self.title}` reads `Notes.title`. Inside `note_card.pax`, the
same expression reads that particular `NoteCard.title`. The attribute on the
invocation connects the two scopes. A child template can use its own fields
without knowing the containing application's type or variable names.

Values supplied by the caller take the place of the child's defaults for
those inputs. Use defaults for fallback state, and avoid unconditionally
writing caller-supplied inputs in a mount handler. Such a write can replace
the displayed value even though the parent is meant to own it.

An ordinary binding such as `progress={self.progress}` derives the child's
input from the parent. It does not provide write-through to the parent.
When the parent owns progress, have application actions update that source.
The card's label and bar then follow its input. [Properties](state-properties.md#computed-properties)
explains what happens if Rust writes directly to a computed field.

For a component designed to edit a shared value, `bind:` explicitly connects
the same property handle at both ends:

```pax
<NoteCard x=24px y=72px width=320px height=160px
    title={self.title} progress=bind:progress
/>
```

The read-only card looks the same. If it gains an editing handler that calls
`self.progress.set(...)`, that handler can now update `Notes.progress` too.
Both fields must have compatible property types; `bind:` takes a property
identifier, not an arbitrary formula. Use it when editing that value is part
of the component's intended interface. See
[PAXEL](data-binding-expressions.md#bindings) for the binding syntax and
[property handles](state-properties.md#property-handles) for the Rust model.

State used only inside one card can stay on that card. State shared by several
cards usually belongs in their common owner. For an action such as “advance
this note,” a named event can let the owner decide how state should change;
we will build that connection [below](#component-actions).

<div class="docs-media-placeholder">
<p><strong>Diagram planned:</strong> follow an input from Notes into two separate NoteCard instances, then compare an expression input with an explicit shared binding.</p>
<!-- Production brief:
- Show two component instances with distinct local state and parent-supplied values.
- Keep the source owner and the place where an action writes visually identifiable.
- Show a formula connection versus a shared property handle without implying copied whole components.
Treatments: a source-to-card wiring diagram; an exploded pair of component
boundaries; or a step-through parent/child state trace. Prefer the wiring diagram. -->
</div>

<a id="control-flow-and-identity"></a>
## Data-driven components

A component invocation can be part of a conditional branch or a repeated
group. The template describes which instances should exist for the current
state; Rust changes that state in response to actions.

### Conditional content

For example, replace the first card with a completion message once progress
reaches one:

```pax
<Group x=24px y=72px width=320px height=160px>
    if self.progress < 1.0 {
        <NoteCard width=100% height=100%
            title={self.title} progress={self.progress}
        />
    } else {
        <Text width=100% height=100% text="Ready for the next idea."
            style={font_size: 20px, fill: rgb(55, 120, 90)}
        />
    }
</Group>
```

Conditions use PAXEL directly after `if`; the braces enclose template
content. An `else if` chain can choose among several states, and `else` is
optional. Pax uses the first true branch, or the final `else` when present.
Each branch can contain several elements.

Changing branches removes the old content and mounts the new content. Local
state belongs to those component instances: if a card is removed and later
created again, its local state starts again. Keep state that must survive
that change in `Notes` or another longer-lived owner. Exit transitions can
retain departing content temporarily, and rapid re-entry can reuse it; see
[Animation and Motion](animation-motion.md) for that lifecycle.

A structural `if` controls which elements exist. A conditional inside
[`@settings`](template-language.md#conditional-settings) changes settings on
existing elements. Choose according to whether the interface needs different
content or different styling.

### Ranges and collections

Use `for` to repeat template content. A range is useful for a fixed set of
decorative elements or positions:

```pax
for i in 0..3 {
    <Text x=24px y={(24 + i * 32)px} width=240px height=28px
        text={"Position " + i}
    />
}
```

This produces positions 0, 1, and 2; the upper bound is excluded. A loop
receives each value as its item. With `for (item, i) in ...`, the second
binding is its current zero-based index. Bindings are available inside that
loop's body; nested loops can give their own items and indices distinct names.

For application data, add a small record to `lib.rs`:

```rust
#[pax]
pub struct NoteEntry {
    pub id: usize,
    pub title: String,
}
```

Add `pub entries: Property<Vec<NoteEntry>>` to `Notes`. In its existing
`Default` implementation, initialize that field with:

```rust
entries: Property::new(vec![
    NoteEntry { id: 10, title: "Field notes".to_string() },
    NoteEntry { id: 20, title: "Sketchbook".to_string() },
    NoteEntry { id: 30, title: "References".to_string() },
]),
```

The record's plain fields describe one value in the collection. The outer
property publishes changes to the collection; use `set` or `update` as
described in [Properties](state-properties.md#collections-and-value-snapshots).

### Keyed lists

Replace the two individual cards with a list:

```pax
<Group x=24px y=72px width=320px height=528px>
    for (entry, i) in self.entries key entry.id {
        <NoteCard y={(i * 176)px} width=100% height=160px
            title={entry.title}
        />
    }
</Group>
```

Here `i` determines position, while `entry.id` determines identity. If the
entries change from `[10, 20, 30]` to `[30, 10, 20]`, Pax reuses the existing
component groups in their new order. Local state, such as a card's expanded
view or an in-progress edit, follows the entry with the same key. New keys
mount new groups; removed keys leave the active list and can play exit
transitions. A loop body may contain several elements, which share that
iteration's identity.

Choose a string or integer key that is stable for the item's lifetime and
unique within that loop. The current index is generally a poor key for a
list that can be reordered. Duplicate or unsupported key values cause a
runtime warning and positional fallback, so check the data instead of relying
on that fallback to preserve state.

An unkeyed loop reuses positions. It is useful for fixed grids and repeated
decoration; for mutable application lists, use an item identifier. Keys govern
reuse within their own loop, not identity across unrelated loops or branches.
Layout remains a separate choice: this example uses explicit row positions;
[Stacker](layout-responsiveness.md) and [Scroller](scrolling-viewports.md) can
arrange or reveal a larger collection.

Explore [Transition Grid](animation-motion.md#try-it-transition-grid) to see
a keyed list being inserted into, removed from, and reversed. Its child
components animate while Stacker owns their placement. The tile counters
are stored in the parent collection; the child's animated color is local
component state.

## Slots

Sometimes the reusable part is a frame around content its caller supplies.
A card frame might reserve a header area while accepting text, controls, or
another component in its body. Slots specify where that caller-provided
content appears inside the frame's template.

Add a component declaration to `lib.rs`:

```rust
#[pax]
#[file("note_frame.pax")]
pub struct NoteFrame {}
```

In `src/note_frame.pax`, use one explicit slot and a remainder slot:

```pax
<Group x=24px y=24px width={100% - 48px} height=32px>
    slot(0)
</Group>
<Group x=24px y=72px width={100% - 48px} height={100% - 96px}>
    slot()
</Group>
<Rectangle width=100% height=100%
    fill=rgb(246, 241, 230) corner_radius=12
/>
```

Use it from `lib.pax` with content between its opening and closing tags:

```pax
<NoteFrame x=24px y=72px width=320px height=220px>
    <Text width=100% height=100% text={self.title}
        style={font_size: 24px, fill: rgb(32, 40, 48)}
    />
    <Text width=100% height=32px text="Make room for the next idea."
        style={font_size: 16px, fill: rgb(72, 80, 88)}
    />
    <Button y=48px width=100% height=36px label="Advance"
        @button_click=self.advance
    />
</NoteFrame>
```

`slot(0)` projects the first child into the header area. `slot()` projects all
children not consumed by an earlier active slot site in this component; here
the body Group receives the second Text and the Button. The caller positions
those body elements within the area the frame supplies. The slot syntax is
positional, with zero-based indices. These children remain authored in
`Notes`: their `self.title` and `self.advance` still refer to `Notes`, even
though the frame supplies their layout containers.

Slots project the existing child content. They do not make a new independent
copy at each insertion site. An explicit index that is out of range or already
consumed renders empty and produces a warning. An empty remainder slot is
valid and silent. Put fixed slot sites before the remainder site; an earlier
`slot()` can consume everything before a later explicit site gets a chance.

Consumption follows the active slot sites in template-tree order. Conditional
sites or a changing slot index can change which children reach the remainder.
Likewise, `if` and `for` in caller content can change the currently available
projected children. Wrap several elements in one Group when they should be
supplied together as one child. The
[`slot-projection-resolver` example](https://github.com/paxproject/pax/tree/dev/examples/src/slot-projection-resolver)
explores these dynamic cases.

The frame's private background and layout structure stay inside its template.
Caller content and private implementation structure serve different roles;
container code should use `NodeContext::received_children` for its semantic
content rather than treating all runtime descendants as supplied children.
Custom container internals are beyond this chapter.

<div class="docs-example-placeholder">
<p><strong>Interactive example planned:</strong> supply a heading and body controls to a frame, then watch them move between fixed and remainder slots.</p>
<!-- Production brief:
- Distinguish caller-owned expressions/handlers from the frame's placement rules.
- Show zero-based indices, one-time consumption, and a valid empty remainder.
- Reuse the slot-projection-resolver's verified behavior without bringing all its diagnostics into the beginner path.
Treatments: a notebook frame with interchangeable content; a labeled tray of
colored tiles; or a compact media player accepting different controls. Prefer
the notebook frame, with source tabs for both caller and frame. -->
</div>

## Component actions

A reusable component can name an action without prescribing the application's
response. For example, an Advance control can ask its owner to advance a note.
The owner may update progress, validate some data, or choose another note.

The complete [custom-event example in Events](event-handling-rust.md#dispatch-custom-events)
builds an `AdvanceButton` that dispatches `advance` to a handler on Notes.
Use that pattern when the component should express an intent and leave the
state change to its caller. Use an input for a value the component reads,
and an explicit `bind:` when editing the shared value is part of its interface.

Custom events carry a name only, so design their data interface alongside
the component's properties or shared state. Keep the action's required or
optional binding explicit. Events owns the sender/receiver code, context,
delivery timing, and error behavior; this chapter's component boundary is
what makes that connection reusable.

## Shared state farther down the tree

Explicit inputs and actions keep a small component easy to reuse. For state
needed throughout a larger subtree, a local store lets descendants find a
shared value without passing it through every intermediate component.

Define a type that names the store's purpose in `lib.rs`:

```rust
pub struct NotesStore {
    pub progress: Property<f64>,
}

impl Store for NotesStore {}
```

Provide it from the owning component's mount method:

```rust
impl Notes {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        ctx.push_local_store(NotesStore {
            progress: self.progress.clone(),
        });
    }
}
```

If `Notes` already has `on_mount`, add this registration to that method.
Cloning the property shares the owner's handle. The store does not need
`#[pax]` because it is accessed from Rust rather than used as a template
component or value.

A descendant handler with a `ctx: &NodeContext` argument can obtain the
handle and reset progress:

```rust
let progress = ctx
    .peek_local_store(|store: &mut NotesStore| store.progress.clone())
    .expect("this component must be inside a NotesStore provider");
progress.set(0.0);
```

Import `NotesStore` from its defining module in the consumer. The lookup walks
the runtime property stack and finds the nearest store of that Rust type;
it returns `Err` when none is available. This example requires a provider.
Handle the error explicitly if the consumer should also work on its own.

Use a distinct store type for each role. Inserting the same type again in one
stack frame replaces that frame's store; a nearer provider shadows an outer
one. Lookup follows runtime scope, including component and projection scope,
so visual proximity alone does not establish access. Keep the borrowed-store
closure short: cloning a needed property handle lets subsequent work happen
after the borrow ends, as above.

The store supplies access to state; the `Property` inside it supplies reactive
updates. A plain Rust field in a store does not become reactive just by being
stored there. The
[`router-playground` example](https://github.com/paxproject/pax/tree/dev/examples/src/router-playground)
uses a scoped store to share its mobile-menu state with navigation components.
Read [Properties](state-properties.md#property-handles) for handle lifetime
and thread constraints.

<a id="authoring-primitives"></a>
<a id="declare-and-connect-a-runtime-node"></a>
<a id="implement-behavior-at-the-right-level"></a>
<a id="native-integration-and-engine-changes"></a>
<a id="source-examples-to-study"></a>

For an advanced alternative that implements an element directly through the
Rust runtime, read [Primitives](primitives.md). That chapter covers when to
use a primitive, how to author one, and its rendering and lifecycle responsibilities.

## Read more

You can now give a reusable view its own interface, choose the owner of its
state, and assemble instances from data and caller-provided content. Continue
with [Layout and Responsiveness](layout-responsiveness.md) to make those
components fit different spaces. [Scrolling](scrolling-viewports.md) covers
larger collections, [Routing](routing.md) organizes screens, and
[Motion](animation-motion.md) adds transitions as content enters, leaves, or
changes position.
