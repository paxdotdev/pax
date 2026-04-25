# Scrolling & Viewports
<!-- summary: Scrollers, nested scroll regions, and viewport-based layout. -->
<!-- tags: scrolling, viewports -->

- Scroller basics: `scroll_width`, `scroll_height`, and content sizing.
- Nested scroll areas and layout constraints.
- Scroll-driven composition patterns.
- Mixed native and vector content in scroll regions.
- Examples from `scroll-island-mini` and `rounded-scroller-tiles`.

## Autosized Scrollers

`Scroller` supports the shared autosize API, but its default is specialized for ordinary vertical documents. `autosize=true` derives `scroll_height` from measured content while leaving `scroll_width` viewport-bound. Add `autosize_x=true` only when the content should define a horizontal scrollable extent.

```pax
<Scroller width=100% height=100% autosize=true>
    <Stacker autosize=true gutter=16px>
        slot(0)
    </Stacker>
</Scroller>
```

Breakout content inside a scroller still belongs to that scroller's render and clipping tree. This keeps native compositor scrolling stable, but it also means `layout_role=LayoutRole::Breakout` does not escape a scroller's clip boundary.
