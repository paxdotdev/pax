# Templates and UI structure
<!-- summary: Read a Pax template, set values, arrange elements, and share settings. -->
<!-- tags: templates, syntax -->

A Pax template describes a component's interface: the elements it contains,
their properties, and their connections to state and events. Reading the tree
gives you its structure; reading the values on each element tells you how that
structure becomes a visible interface.

This chapter starts with a small panel, then introduces the settings mechanisms
you can use as an interface grows. If you haven't run a Pax project yet,
[Getting Started](getting-started.md) covers installation and the first run.

## Reading a template

A component usually pairs a Rust type with a `.pax` file. For a main component
called `Notes`, the Rust declaration can be as small as:

```rust
use pax_kit::*;

pax_kit::runnable! { root: crate::Notes }

#[pax]
#[file("lib.pax")]
pub struct Notes {}
```

`#[pax]` makes the type a Pax component, `runnable!` selects it as this
application's root, and `#[file("lib.pax")]` names its template. Here is that template:

```pax
<Group x=24px y=24px width=320px height=160px>
    <Text x=24px y=24px width={100% - 48px} height=36px
        style={font_size: 24px, fill: rgb(32, 40, 48)}>Field notes</Text>
    <Text x=24px y=72px width={100% - 48px} height=48px
        style={font_size: 16px, fill: rgb(72, 80, 88)}>A place for your next idea.</Text>
    // The background is behind the text above it.
    <Rectangle width=100% height=100%
        fill=rgb(246, 241, 230) corner_radius=12
    />
</Group>
```

Tags name components, using PascalCase: `Group`, `Text`, and `Rectangle` here.
The `Group` has an opening and closing tag because it contains other elements.
The `Text` elements receive words between their tags. This assigns each Text's
`text` property. The `Rectangle` uses a self-closing tag.

The group is the parent of these three elements, and they are siblings of one
another. Each child's position and percentage dimensions are relative to its
container. The group supplies a shared 320-by-160-pixel area for this panel.
A template can also contain several root elements; add a container when you
need them to share a layout area.

Comments such as the `//` line above help explain the intent of a template.
Keep them between elements or settings. Pax also accepts block comments and
HTML-style comments in those positions.

Components can use templates embedded in Rust with `#[inlined(...)]` as well.
Separate `.pax` files keep the interface easy to find and edit. See
[Components and Composition](components-composition.md) for defining and
organizing reusable components.

## Setting values

An attribute such as `width=320px` assigns a property on an element. Some
properties are common across elements, including position and size; others,
such as `text` or `corner_radius`, belong to the receiving component.

The shape of the value tells you how it is supplied:

| Form | Example | Meaning |
| --- | --- | --- |
| Literal | `text="Field notes"` | Use this value directly. |
| Value with a unit | `width=320px` or `width=100%` | Express a dimension in pixels or relative to the parent. |
| Expression | `width={100% - 48px}` | Derive the value with PAXEL. |
| State binding | `text={self.title}` | Read the containing component's `title` property reactively. |
| Event binding | `@click=self.activate` | Invoke the component's `activate` handler when the event occurs. |
| Read/write binding | `text=bind:title` | Connect a supporting control's value to the component's property in both directions. |

The last three examples assume the Rust component exposes the named property
or handler. In a template expression, `self` refers to that containing
component. It gives a child element access to the state of the component whose
template you are reading.

Braces mark PAXEL expressions. When `title` changes, a `Text` element using
`text={self.title}` updates from that value. A control such as
`<Textbox text=bind:title />` can also write edits back to `title`.
See [Data Binding and Expressions](data-binding-expressions.md) for expression
syntax and [State and Properties](state-properties.md) for the Rust side.

Event bindings connect the tree to application behavior. The handler's body
lives in Rust; [Event Handling and Rust Logic](event-handling-rust.md) covers
its signature, event data, and state updates.

## Text bodies

`Text`, `Button`, `Handwriter`, `Toast`, and `ConfirmationDialog` accept text
between their opening and closing tags:

```pax
<Text width=240px height=40px>Hello, world.</Text>
<Button width=180px height=40px>Continue</Button>
<Text width=240px height=40px>{"Hello, " + self.name}</Text>
```

A body is either literal text or one complete `{...}` expression. Expressions
read the containing component's state. Inline interpolation such as
`Hello {self.name}` is not supported; put the entire string expression in braces.
A scalar body cannot be mixed with child elements or template control flow.
Structural children and slots retain their existing behavior.

Bodies assign an ordinary property: `Text.text`, `Button.label`,
`Handwriter.text`, `Toast.message`, `ConfirmationDialog.text`, or a custom
component's [content receiver](components-composition.md#receiving-text-content).
The attribute forms still work, including `text=bind:note` for editable text.
Do not supply both a body and its receiving attribute on the same tag. A body
has the same precedence as an inline attribute over shared settings.

### Whitespace and literal characters

Inline bodies preserve spaces. For a multiline body whose first and last lines
contain only spaces or tabs, Pax removes those two framing lines and the shared
space/tab prefix of the nonblank content lines. Additional indentation, blank
lines, and trailing spaces remain; ordinary body line endings normalize to LF.
The formatter may indent a framed block without changing its value:

```pax
<Text width=320px height=80px>
    First line
        Four intentional spaces on this line
    Last line
</Text>
```

An empty or whitespace-only ordinary body supplies no value, leaving defaults
and settings in effect. Use `{""}` to assign an empty string explicitly.

Literal `<`, `{`, `}`, and `&` need character references in ordinary bodies.
Use `&lt;`, `&#123;`, `&#125;`, and `&amp;`. Pax also accepts `&gt;`, `&quot;`,
`&apos;`, and decimal or hexadecimal Unicode scalar references such as
`&#233;` or `&#x1F680;`. References decode once, after whitespace handling;
decoded characters never become template syntax. Unknown references and bare
ampersands are errors. Quotes, apostrophes, backticks, and backslashes otherwise
remain ordinary text. A body consisting of template comments or `slot(...)`
is structural; use a string expression when those characters are intended text.

### Exact text with raw strings

Use a raw string expression for exact whitespace, literal markup, or embedded
code. The formatter leaves the string payload unchanged, including a first or
last newline, tabs, CRLF, and intentional indentation:

```pax
<Text width=320px height=80px>{r#"  Keep these two spaces.
    Keep these four, too. <Text>{name}</Text> & \n"#}</Text>
```

`r"..."`, `r#"..."#`, and forms with more hashes are PAXEL string literals.
Choose enough hashes that the matching closing quote/hash sequence does not
occur in the text. Raw strings do not decode references or backslash escapes.
They can also be used in attributes and other PAXEL expressions. Existing quoted
strings retain their existing escape behavior. Prefer a raw string expression
inside `#[inlined(...)]` when prose contains punctuation that Rust would parse
as tokens; `.pax` files accept ordinary prose directly.

Text bodies do not enable Markdown. See
[Markdown content](text-fonts-images.md#markdown-content) for the explicit switch
and target limitations.

## Element order

Earlier siblings appear in front of later siblings. In the panel above, both
text elements are in front of the rectangle. Moving the rectangle before the
text would put its opaque fill over the words.

This makes a template read like a stack of layers: foreground details first,
background last. It is worth checking source order whenever an element seems
to have disappeared behind another.

Nesting, draw order, and spatial layout each answer a different question.
Nesting gives elements a shared container; sibling order determines which is
in front; properties such as `x`, `y`, `width`, and `height` determine their
geometry. A `Group` lets you place children in its area. Flow containers such
as `Stacker` can arrange children for you. Continue with
[Layout and Responsiveness](layout-responsiveness.md) for those layout tools,
or [Compositing and Effects](compositing-effects.md) for clipping, masks, and
native-element layering.

## IDs and classes

Inline values keep a small element easy to read. As settings become reusable,
give elements a class and collect those settings in an `@settings` block:

```pax
<Group id=panel x=24px y=24px>
    <Text class="heading" x=24px y=24px width={100% - 48px}
        height=36px>Field notes</Text>
    <Text class="body" x=24px y=72px width={100% - 48px}
        height=48px>A place for your next idea.</Text>
    <Rectangle class="surface" width=100% height=100% />
</Group>

@settings {
    #panel {
        width: 320px
        height: 160px
    }
    .heading {
        style: {
            font_size: 24px
            fill: rgb(32, 40, 48)
        }
    }
    .body {
        style: {
            font_size: 16px
            fill: rgb(72, 80, 88)
        }
    }
    .surface {
        fill: rgb(246, 241, 230)
        corner_radius: 12
    }
}
```

This produces the same panel. Tags use `=` for assignments; the settings block
uses `:`. The nested `style` object groups properties of the text style.

Use an `id` to address an individual node within the template, and a class for
settings that several elements can share. `#panel` selects `id=panel`;
`.heading` selects elements with `class="heading"`. The supported selectors
inside `@settings` are these ID and class forms.

Settings belong to the component's template. A class on a child component's
tag can configure that child instance; the child's internal template has its
own settings. Explicit sharing is covered in [Imported settings](#imported-settings).

### Multiple and reactive classes

A single class is a string. Multiple classes use one ordered string list:

```pax
<Rectangle class=["surface", "selected"] />
```

Within the component's local settings, matching classes apply in the order
listed on the element. Later classes can override earlier ones, matching ID
settings apply after those classes, and inline values apply last. If several
blocks use the same selector, their source order decides which comes later.

For example, an inline `fill=WHITE` wins over a `fill` supplied by `.surface`,
`.selected`, or a local ID selector. Put a value inline when it should remain
specific to that element; put it in a class when another class or a theme should
be able to vary it.

Class bindings can be reactive too. If the containing component exposes a
boolean `selected` property, it can choose the class list:

```pax
<Rectangle class={self.selected ? ["surface", "selected"] : "surface"} />
```

An expression may return a string or a list of strings. It can read a Rust
component property of type `Property<String>` or `Property<Vec<String>>`.

There are a few precise rules worth keeping nearby:

- An empty string or empty list means no classes. A class may have no matching
  selector.
- A string names one class and is never split on whitespace. Write
  `["surface", "selected"]` for two classes.
- Duplicate names keep their final position in the list.
- Class names start with an ASCII letter or underscore, followed by ASCII
  letters, digits, underscores, or hyphens. Invalid names and non-string
  values are ignored with a runtime warning.
- An element has one `class` attribute. The formatter writes a one-item
  literal list as a single string.

`class` selects settings; it cannot itself be assigned inside a selector,
settings, or timeline block. Give every inline property a single assignment.

## Conditional settings

Settings can respond to a condition while keeping the same elements in the
tree. For example, this panel has a fixed width in a wide viewport and leaves
a 24-pixel margin on each side in a narrow one:

```pax
<Group class="panel" x=24px y=24px height=160px>
    <Rectangle class="surface" width=100% height=100% />
</Group>

@settings {
    .panel {
        width: 320px
    }
    .surface {
        fill: rgb(246, 241, 230)
        corner_radius: 12
    }
    if @viewport.width < 600 {
        .panel {
            width: {100% - 48px}
        }
    }
}
```

The condition is a PAXEL expression that evaluates to a boolean. Pax updates
the affected settings when its inputs change. Here, the conditional width
overrides the earlier `.panel` width while the viewport is under 600 pixels;
the earlier width applies again when the condition becomes false.

Use `else if` and `else` for additional branches. Conditions can also use
component state or built-in globals such as `@ios`, `@macos`, and `@landscape`.
The viewport describes the application window; percentages in the assigned
width still resolve against the element's parent. See
[Layout and Responsiveness](layout-responsiveness.md) for choosing responsive
rules for nested components.

Place these conditions around selector blocks inside `@settings`. They may
contain nested conditions. Inside an individual selector, use a PAXEL expression
for a conditional property value. An `if` among the template's elements instead
controls which elements exist; [Conditional content](components-composition.md#conditional-content)
explains that structural form.

## Imported settings

`ImportSettings` lets a component use settings provided by other components.
This is useful for sharing a palette, typography, or other coordinated choices.
The provider subtree supplies settings without drawing its own interface.

For example, declare a provider alongside the main component in Rust:

```rust
#[pax]
#[file("paper_theme.pax")]
pub struct PaperTheme {}
```

Its `paper_theme.pax` file can supply the panel's surface settings:

```pax
<Group />

@settings {
    .surface {
        fill: rgb(232, 240, 226)
        corner_radius: 18
    }
}
```

Add the provider to the panel's template, keeping the visible elements as
siblings of `ImportSettings`:

```pax
<ImportSettings>
    <PaperTheme />
</ImportSettings>

<Group x=24px y=24px width=320px height=160px>
    <Text x=24px y=24px width={100% - 48px} height=36px
        style={font_size: 24px, fill: rgb(32, 40, 48)}>Field notes</Text>
    <Rectangle class="surface" width=100% height=100% />
</Group>
```

The rectangle receives `.surface` from `PaperTheme`. The import applies to
elements authored in the containing component's template. A child component
manages imports for its own internal template.

For ordinary settings, the layers are applied in this order:

1. The containing component's local settings.
2. Imported providers, in provider order.
3. Inline values on the element.

Later layers override earlier ones for the same property. Within each provider,
the same class-list and ID ordering rules apply. A class in an imported provider
can therefore override a local ID setting; layer order is considered before
selector order. When providers are direct siblings inside one `ImportSettings`,
their order in the template is their layer order.

The provider can expose reactive properties of its own. Expressions in its
settings use that provider's `self` and state, so a shared palette can change
reactively. Stable imports reuse their provider layers on unchanged frames;
provider-property updates continue through the existing reactive bindings.
Import discovery subscribes to the tree's structural inputs rather than
traversing it every tick. Conditional imports, provider order changes, and
template reloads refresh those subscriptions and settings before rendering;
ordinary palette-value changes do not rediscover the provider tree.

For ordinary template nodes, import changes retain bindings whose prepared
definitions and provider scopes are unchanged. Added or changed settings replace
only their affected property bindings; removed settings fall back to the remaining
layers or the property's default. This applies in both development and release
builds and does not change when mount handlers run. See
[How Pax Runs](how-pax-runs.md#from-templates-to-running-instances) for the runtime
and compatibility paths.

See [Drawing and Styling](drawing-styling.md) for visual design
choices and [PAXEL's `@base`](data-binding-expressions.md#base) for building on
an earlier property value.

Add a transition to ease changes to the effective imported settings:

```pax
<ImportSettings transition=SettingsTransition::Ease(400ms, TransitionCurve::InOutQuad)>
    <PaperTheme />
</ImportSettings>
```

All changed settings use that duration and curve. The initial appearance is
immediate; subsequent provider changes animate from each receiver's current
value. Omitting `transition` (or using `SettingsTransition::None`) preserves
immediate updates. Inline values still win over imported settings. See
[Automatic settings transitions](animation-motion.md#automatic-settings-transitions)
for interruption, supported types, and ownership rules. Imported providers do
not currently export named timeline tracks.

## Read more

Templates also support structural `if` and `for` blocks, child projection with
`slot(...)`, and motion with `@timeline`. Continue with
[Components and Composition](components-composition.md)
and [Animation and Motion](animation-motion.md) as you encounter those needs.

For working with values, [Data Binding and Expressions](data-binding-expressions.md)
is the next step. To keep source formatting consistent as you edit, use
[`pax-cli fmt`](developer-workflow.md#format-pax-source).
