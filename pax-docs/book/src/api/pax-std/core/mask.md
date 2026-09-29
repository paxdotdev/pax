# core::mask
<!-- summary: API docs for pax-std::core::mask. -->
<!-- tags: api, pax-std -->

## Structs
### `Mask`
Clips its first child using its second child subtree, which is not rendered as
visible content. By default, this is a geometric coverage mask. Set `alpha=true`
to use painted alpha instead, and `feather` to soften its edge:

```pax
<Mask width=100% height=100% alpha=true feather=12.0>
    <Rectangle width=100% height=100% fill=#FF0088/>
    <Ellipse x=50% y=50% anchor=50% width=240px height=240px
        fill=TRANSPARENT stroke={paint: WHITE, width: 40px}/>
</Mask>
```

The example reveals pink only underneath the soft ring. `feather` is Gaussian
standard deviation in logical pixels, independent of display pixel density;
zero disables feathering. Both properties default to zero/false, preserving
existing coverage-mask behavior.

Alpha sources support `Rectangle`, `Ellipse`, `Path`, and `Line` appearance layers,
including source-relative opacity, transforms, and linear/radial gradient alpha
(up to eight ordered stops). Source RGB does not matter. Retained vector captures
preserve source-side Frame/Mask clips and group opacity. Two opaque shapes in
a half-opacity Group yield 50% coverage even in their overlap; individually
half-opacity shapes yield 75% there. Nested alpha masks on content multiply;
ordinary geometric clips continue to intersect them. An empty alpha source
hides all content. Captures share group-opacity surfaces and reuse geometry for
paint and GPU stroke-reveal updates. Feathering has a padded, surface-local
domain; captures exceeding device texture limits hide content with a diagnostic.

Source components have a normal logical lifetime: their templates expand,
mount handlers initialize state, reactive changes and animation remain live,
and unmount releases their descendants and subscriptions. This allows a
`Handwriter` (including its animated draw range) to supply the mask directly.
Source nodes are not presented or hit-tested. Native leaves, including
Handwriter's selectable text equivalent, do not create native surfaces; keep
meaningful accessible text outside the mask. User mount side effects still run.

Current boundary: WGPU canvas rendering, verified on web. Native controls and
the legacy Piet renderer do not support alpha masks. Images, text, native
elements, and source-side Scrollers are not captured; use vector leaves and
same-surface containers. Alpha masks modulate canvas draw alpha,
not an isolated offscreen group, and do not change hit testing. Keep interactive
hit targets separate from purely visual alpha reveals.

#### Properties
##### `alpha`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`bool`>

Use painted alpha instead of geometric coverage.

##### `feather`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`f64`>

Gaussian feather standard deviation, in logical pixels, for alpha masks.
