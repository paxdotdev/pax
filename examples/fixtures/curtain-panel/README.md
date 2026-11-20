# Black sateen

A top-gathered curtain lit by a scoped Pax strip light. The first frame is held.
**Play draft** starts a side draft with a period of about 5.6 seconds at the
default 1.95× speed. The shoulders stay pinned while the lower folds bow and
settle at different times.
Fold spacing is in logical pixels, so a narrow panel shows fewer folds.

From the repository root, run:

```sh
cargo run -p pax-cli -- run --path examples/fixtures/curtain-panel --target web --hot-reload=pax
```

Open `/preview.html` on the printed server to suspend the fixture while its
browser tab is hidden.
Scroll below the cloth to adjust the fold spacing, depth, speed, sheen width
and strength. Pause the cloth, move the light, then switch between neutral,
amber and off to inspect the highlight. The native title remains selectable. **Hide title** clears it for a closer look.
Pause holds the current pose. Leaving the viewport also stops the cloth clock.
System reduced-motion preferences are not exposed through a shared Pax API.

The fixture-local `Sateen` shader declares its dark body colour as a solid input.
The Rectangle supplies geometry for native overlap and hit testing. The default fold spacing is 187 logical pixels, depth 0.21, sheen
strength 0.25 and roughness 0.53. The host places a neutral strip light at
64.6% across the panel and starts at half output resolution. A single
fragment pass calculates fold height and slopes, then evaluates the scene
light with two nearby normals to approximate a directional sheen. The folds have planar coverage. Narrow troughs and rolled shoulders
suggest overlapping cloth within the crop. There is no moving outer silhouette,
cast shadow, cloth simulation, image asset or history buffer.

**View** cycles through cloth, a plain Rectangle, and an identity pass.
The identity pass receives the same phase updates during playback. **Resolution**
switches between half and full resolution on each axis. Both shader views skip
subtree capture and composite their output at the renderer’s backing density. The CPU web renderer shows the plain panel and
title with the decorative clock stopped.
