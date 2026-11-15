# Creative example captures

`oscilloscope.jpg` is a browser capture of the canonical
`examples/led-oscilloscope` crate, hosted by the website's `/oscilloscope/` route.
Captured 2026-11-02; it is not generated artwork. Compact layouts use this
capture; desktop uses a live component. Both are presented upright, without
additional window chrome, at the page's content gutters. The capture retains
its original aspect ratio; the live viewport widens on larger desktops.

To refresh, run the website with `pax-cli run --path pax-website --target web
--libdev`, open `/oscilloscope/` at a roughly 1000×652 CSS-pixel viewport,
and wait for artwork and fonts to settle. Close the source drawer. Capture the
application below the 52px route header, preserving its aspect ratio.
Keep temporary full-page captures outside the repository.

The oscilloscope capture includes the canonical cabinet and controls.
No live example runs behind a homepage capture.
