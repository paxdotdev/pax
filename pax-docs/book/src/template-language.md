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
single class may be written as a symbolic identifier; multiple classes
canonically use one list:

```pax
<Text id=page_title class=heading />
<Rectangle class=[card, elevated, interactive] />

@settings {
    #page_title {
        width: 100%
    }

    .card {
        corner_radius: 12
    }
}
```

Class names are static selector symbols, not quoted strings. Runtime-computed
class lists are not currently supported; use conditional settings or bind the
affected property directly when styling must react to state.

Other inline properties should occur at most once. Duplicate non-class
properties are not a supported precedence mechanism.

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
