# Surface / Solid

An opaque `Ellipsoid` moves through a tilted live panel. The same retained mesh
supplies GPU depth, native occlusion contours, volume bounds, and pointer depth.
The panel's textbox and button remain mounted while the solid moves.

```sh
pax-cli run --path examples/fixtures/solid-intersection --target web
pax-cli run --path examples/fixtures/solid-intersection --target macos
```

This fixture enables the default-off `pax-kit/3d` feature. Use the GPU renderer.

- **Behind / Through / In front** select reproducible intersection states.
- **Play motion** sweeps depth; pause before manually adjusting the depth slider.
- **Panel rotation** changes the intersection plane without remounting controls.
- Type into the textbox, move the solid, then expose the textbox again to check
  preserved content. Click exposed portions of the panel button or the solid to
  exercise their separate counters. Hidden panel regions must not activate it.
- Scroll the page to check canvas/native alignment and retained tile replay.

The solid uses a shared, immutable 64 × 32 sphere mesh. Width, height, depth, and
pose transform that mesh; motion does not upload new vertex/index data. Hard
projected clips are supported. Fractional solid opacity, alpha masks, shader
captures, transparent shells, perspective, and arbitrary concave meshes are not
part of this checkpoint. Native/scroller layers keep their existing stacking
limits; intersections resolve within one logical canvas layer.
