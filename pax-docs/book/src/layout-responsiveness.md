# Layout & Responsiveness
<!-- summary: Layout primitives, positioning, and responsive geometry. -->
<!-- tags: layout, responsive -->

- Groups, frames, and stackers as layout building blocks.
- Anchors and alignment: `anchor_x`, `anchor_y`, and text alignment.
- Percent vs pixel sizing, mixed-unit expressions, and responsive math.
- Transforms: scale, rotate, skew, and origin considerations.
- Layout patterns from `marketing-site`, `scroll-island`, and `rounded-scroller-tiles`.

## Multi-Axis Common Properties

Use list syntax for suffix-based common-property pairs. A scalar applies to both axes, and a two-item list maps to `[x, y]`.

```pax
<Group scale=80% anchor=[50%, 50%] skew=[0deg, 4deg] />
```

Axis-specific properties remain available and take precedence over the shorthand in the same settings layer. When a longhand uses `$base`, it sees the value supplied by the shorthand for that axis.

```pax
<Group scale=20% scale_y={$base + 5} />
```

## Autosize

`autosize` lets a container derive one or both of its concrete axes from the layout hull of its content instead of requiring a manually declared `width`, `height`, `scroll_width`, or `scroll_height`.

```pax
<Stacker autosize=true gutter=12px>
    <Group height=48px />
    <Group height=96px />
</Stacker>
```

Autosize is bottom-up: the engine measures each descendant subtree in layout coordinates, projects those bounds into the container's local coordinate space, and uses the forward extent from the container origin. A child at `x=200px` with `width=20px` contributes a width of `220px`; a child entirely at negative x contributes `0px` to the forward width.

Explicit axes still win. If a node has `width=300px`, autosize will not replace that width, but it can still fill in an omitted height. Empty autosized content collapses to `0px` on managed axes. If a managed axis depends only on unresolved percentage sizes, the container can fall back to normal top-down layout; use axis overrides to constrain autosize to axes that can be measured.

### Axis Controls

`autosize=true` means "use this component's default autosize semantics." `autosize_x` and `autosize_y` are optional per-axis overrides, and each override wins for that axis whether it is `true` or `false`.

| Component | `autosize=true` default |
| --- | --- |
| `Stacker` | Autosizes the extending axis only: vertical stackers manage `y`, horizontal stackers manage `x`. |
| `Scroller` | Autosizes the scroll content height only. Use `autosize_x=true` to derive content width too. |
| `Group` | Autosizes both axes from direct content children. |
| `Frame` | Autosizes both axes from direct content children, then clips to that measured frame. |
| `Link` | Autosizes both axes from slotted content so the link hit area follows its children. |
| `Tooltip` | Autosizes both axes from its trigger content; its floating bubble uses `layout_role=LayoutRole::Breakout`. |

```pax
<Scroller width=100% height=100% autosize=true>
    // Vertical content height is measured; horizontal content width stays viewport-bound.
</Scroller>

<Scroller width=100% height=100% autosize=true autosize_x=true>
    // Both scrollable axes are measured from content.
</Scroller>
```

Text participates naturally through `measured_size`: text and native controls can report empirical bounds after settling, and autosized ancestors react to those measured bounds through the same content-hull path.

## Layout Role

Every renderable node has a `layout_role` common property. The default is `LayoutRole::Default`, which participates in parent autosize hulls and container flow. `LayoutRole::Breakout` keeps parent-local coordinates but opts the node out of parent hull measurement and flow.

```pax
<Frame autosize=true>
    <Text text="Trigger" />
    <Frame
        y={100% + 8px}
        width=220px
        autosize_y=true
        layout_role=LayoutRole::Breakout>
        <Text x=12px y=10px width={100% - 24px} text="Floating copy" />
        <Rectangle width=100% height=100% fill=rgb(12.5%, 12.5%, 12.5%) />
    </Frame>
</Frame>
```

Use `Breakout` for overlays, tooltip bubbles, dropdown panels, badges, and other local affordances that should render near a component without making that component larger. `Breakout` is not a portal or browser-style `position: fixed`: in the first implementation it still renders, hit-tests, scrolls, clips, and masks in the normal ancestor tree. Percent sizes on a breakout node still resolve against the same parent bounds they would use in default layout.
