# Template Language & Structure
<!-- summary: Pax template syntax, element ordering, and structure conventions. -->
<!-- tags: templates, syntax -->

- Element tree anatomy: tags, attributes, and child nodes.
- Z-order rules: earlier siblings render on top of later siblings.
- Identifiers and classes: `id=` selectors and reusable style classes.
- Units and expressions in attributes: `px`, `%`, `deg`, arithmetic.
- Settings blocks: `@settings` and per-node style application.

## IDs and classes

Use `id` for one unique node and `class` for reusable settings selectors. A
single class is a string; multiple classes canonically use one string list:

```pax
<Text id=page_title class="heading" />
<Rectangle class=["card", "elevated", "interactive"] />

@settings {
    #page_title {
        width: 100%
    }

    .card {
        corner_radius: 12
    }
}
```

Class bindings are reactive. An expression may evaluate to either `String` or
`Vec<String>`:

```pax
<Text class={self.current_class} />
<Rectangle class={self.current_classes} />
<Group class={self.selected ? ["card", "selected"] : "card"} />
<Group class={"temporary"} />
```

An empty string or empty list means no classes. A string names exactly one
class and is never split on whitespace. When several classes match, their
settings apply from left to right, so later classes override earlier classes
before `id` and inline settings are applied.

Duplicate names collapse to one class at the position of their final
occurrence. The formatter writes a one-item literal list as a scalar string.

Class names must use the same identifier spelling accepted by `.class`
selectors. Invalid names and values of other runtime types are ignored with a
runtime warning. A class is allowed to have no matching selector.

`class` is reserved selector metadata rather than an ordinary component
property. It cannot be assigned from inside a settings, selector, or timeline
block.

Every inline property, including `class`, should occur at most once. Use a
single class list instead of repeated attributes.

## Conditional Settings

Top-level `@settings` entries can be guarded with `if`, `else if`, and `else`. Conditions are PAXEL expressions that must evaluate to `bool`, so built-in globals such as `$ios`, `$macos`, `$landscape`, `$viewport.width`, `$frames`, and `$millis` are available.

```pax
@settings {
    .card {
        width: 100%
    }

    if $ios && $landscape {
        .card {
            width: 50%
        }
    } else if $macos {
        .card {
            width: 70%
        }
    } else {
        .card {
            width: 90%
        }
    }
}
```

Conditionals are supported at the top level of `@settings` and can contain selector blocks. Inside a selector block, use ordinary PAXEL expressions, ternaries, and `$base` for per-property conditional logic.
