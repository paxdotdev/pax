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
        fill=TRANSPARENT stroke={color: WHITE, width: 40px}/>
</Mask>
```

The example reveals pink only underneath the soft ring. `feather` is Gaussian
standard deviation in logical pixels, independent of display pixel density;
zero disables feathering. Both properties default to zero/false, preserving
existing coverage-mask behavior.

Alpha sources support `Rectangle`, `Ellipse`, and `Path` fills and strokes,
including source-relative opacity, transforms, and linear/radial gradient alpha
(up to eight ordered stops). Source RGB does not matter. Grouping and keyed
`for` loops combine paints using source-over alpha: overlapping half-opacity
sources yield 75% coverage, not XOR. Nested alpha masks on content multiply;
ordinary geometric clips continue to intersect them. An empty alpha source
hides all content. Cached surface-sized GPU textures are reused until paint,
feather, enclosing alpha, or surface dimensions change.

Current boundary: WGPU canvas rendering, verified on web. Native controls and
the legacy Piet renderer do not support alpha masks. Source-side `Frame`/`Mask`
clipping, images, text, and native elements are not alpha sources; use vector
leaves inside `Group`/repeat containers. Alpha masks modulate canvas draw alpha,
not an isolated offscreen group, and do not change hit testing. Keep interactive
hit targets separate from purely visual alpha reveals.
