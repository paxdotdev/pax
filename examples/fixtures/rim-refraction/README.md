# Contour light

Hover near the center of **Get Started** or **GitHub** to catch cyan, magenta
and yellow on both the top and bottom edges. Move sideways to cycle the colors
within each glint. Moving back retraces them. The contour is 1 logical pixel
wide and blends at 75% opacity. The two faces use the website's opal and
obsidian paints and materials. Labels and the GitHub icon stay outside the capture,
and the whole face retains its link.

Run from the repository root with
`pax-cli run --path examples/fixtures/rim-refraction --target web`.

- **View** cycles through the CMY rim, an identity capture and the plain face.
  Identity sets rim intensity to zero inside the same lit effect and keeps
  its source, sampling margin and composite comparable.
- **Light** cycles through the real point light, diagonal strip, both and
  ambient alone. The point starts at 65% of viewport width and 25% of
  height. Mouse or touch moves it, and it holds its last position on release.
- **Two buttons** cycles to six faces and then adds a cutout path and a
  tilted ellipsoid below the original pair. The ellipsoid uses the 3D
  feature. Its visible outline supplies the contour.
- **Play motion** sweeps the strip and rotates the ellipsoid in the views
  that use them. Motion starts paused and suspends offscreen. A stationary
  pointer can illuminate the edge while the page scrolls beneath it.

CPU rendering displays the ordinary button faces and path. The ellipsoid
requires GPU rendering. See the [RimRefraction guide](../../../pax-docs/book/src/compositing-effects.md#rimrefraction)
for the threshold, lighting scope and limits of projected silhouettes.
