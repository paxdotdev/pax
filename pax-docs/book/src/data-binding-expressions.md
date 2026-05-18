# Data Binding & Expressions
<!-- summary: PAXEL expressions, data binding syntax, and reactive updates. -->
<!-- tags: paxel, bindings, expressions -->

Pax template expressions are written in PAXEL, a small side-effect-free expression language. Use expressions when a property should be derived from literals, component state, built-in functions, or other expressions.

## Bindings

Use `{...}` for a one-way derived value:

```pax
<Text text={"Score: " + score} />
<Rectangle width={(base_width + 8)px} />
```

Use `bind:` when a native form control should read and write a component property:

```pax
<Textbox text=bind:name />
<Slider value=bind:volume min=0.0 max=1.0 />
```

Expressions are reactive: when an identifier used by the expression changes, Pax recomputes the derived value.

## Operators

| Category | Operators | Example |
| --- | --- | --- |
| Fallback | `??` | `maybe_title ?? "Untitled"` |
| Conditional | `? :` | `is_selected ? YELLOW : GRAY` |
| Boolean | `!`, `&&`, `||` | `enabled && !disabled` |
| Equality | `==`, `!=` | `mode != "hidden"` |
| Comparison | `<`, `<=`, `>`, `>=` | `score >= target` |
| Arithmetic | `+`, `-`, `*`, `/`, `%%`, `^` | `(index + 1) * 24` |
| Range | `..` | `0..count` |
| Access | `.`, `[...]` | `user.name`, `items[index]` |
| Grouping and units | `(...)`, `(expr)px`, `(expr)%`, `(expr)deg`, `(expr)rad`, `(expr)ms`, `(expr)s`, `(expr)f` | `{(width + 12)px}` |

`??` is the null coalescing operator. When the left side is `Some(value)`, the result is `value`. When the left side is `None`, Pax evaluates and returns the right side. Non-option left values pass through unchanged, and the right side is not evaluated unless it is needed.

```pax
<Text text={maybe_title ?? "Untitled"} />
<Rectangle width={maybe_width ?? 240px} />
<Text text={user_label ?? fallback_label ?? "Anonymous"} />
```

Use ternaries for branch selection:

```pax
<Rectangle fill={is_selected ? YELLOW : GRAY} />
<Text text={count == 1 ? "1 item" : "Items: " + count} />
```

## Literals

PAXEL supports booleans, numbers, strings, units, colors, lists, tuples, objects, ranges, enum/function-style calls, and option literals.

```pax
<Rectangle x=25px y=10% rotate=15deg fill=rgba(255, 128, 64, 255) />
<Stacker sizes=[Some(120px), None, Some(30%)] />
<Text text={Some("Ready") ?? "Waiting"} />
```

Time units are available for animation durations and timeline markers: `250ms`, `1s`, and `5f`.  Timeline `duration` also accepts unitless frame counts.  Time units can be produced from expressions, for example `{(100 + self.delay)ms}`.

Objects can contain nested expressions:

```pax
<Rectangle fill={is_hot ? {r: 255, g: heat * 80, b: 0, a: 1} : {r: 0, g: 120, b: 255, a: 1}} />
```

## `$base`

`$base` is available while Pax resolves a property value. It evaluates to the same property's value from the previous precedence layer: defaults first, then component settings, imported settings, inline settings, and timelines in order.

Use `$base` when one layer should adjust rather than replace an earlier value:

```pax
@settings {
    .card {
        x: {$base + 12px}
        opacity: {$base * 0.8}
    }
}
```

In timeline keyframes, wrap `$base` in an expression. This lets an animation move relative to the property position already established by layout or settings:

```pax
<Rectangle id=card y=120px />

@settings {
    #card {
        y: @timeline {
            duration: 20,
            0: {$base - 24px},
            100%: {$base},
        }
    }
}
```

## Built-In Globals

Built-in globals use a `$` prefix and can be read from any PAXEL expression.

`$target` exposes the current platform and operating system as booleans:
`web`, `native`, `ios`, `iphone`, `ipad`, `macos`, `android`, `windows`, `linux`, `mobile`, and `desktop`.
Each field also has a top-level alias with the same name, such as `$web`, `$ios`, `$ipad`, `$mobile`, and `$desktop`.

`$viewport` exposes the current scene size and orientation:
`width`, `height`, `major`, `minor`, `aspect`, `landscape`, `portrait`, and `square`.
`major`, `minor`, `aspect`, `landscape`, `portrait`, and `square` are also available as top-level aliases.

`$gyro` exposes device orientation as `{x, y, z}` in degrees, and `$accel` exposes acceleration as `{x, y, z}` in meters per second squared. On web targets, `$accel` uses acceleration including gravity when available. `$frames` and `$millis` expose runtime clock values.

```pax
<Group
    width={$landscape ? 50% : 100%}
    x={$ios ? 12px : 24px}
/>
```

## Loops

Ranges are commonly used as `for` sources:

```pax
for i in 0..count {
    <Text y={(i * 24)px} text={"Item " + i} />
}
```

## Function Calls

Functions and enum-style constructors use Rust-like path syntax:

```pax
<Rectangle width={Math::max(80, desired_width)} />
<Text text={"Smallest: " + Math::min(a, b)} />
```

## Common Gotchas

Use `%%` for modulo. `%` is reserved for percent units such as `50%` or `(width / 2)%`.

Use `bind:` only for two-way control state. For derived display values, use `{...}`.

Keep template expressions pure. Move side effects, mutations, and event handling into Rust handlers, then expose the resulting state back to Pax properties.
