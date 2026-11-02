# Creative example captures

`oscilloscope.jpg` and `space-game.jpg` are browser captures of the canonical
`examples/led-oscilloscope` and `examples/fixtures/space-game` crates, hosted by
the website's `/oscilloscope/` and `/space-game/` routes. Captured 2026-11-02.
They are not generated artwork. Device/window frames are rendered separately
by the website so scroll choreography transforms a single screenshot per study.

To refresh, run the website with `pax-cli run --path pax-website --target web
--libdev`, open the relevant route at a roughly 1000×652 CSS-pixel viewport,
and wait for artwork and fonts to settle. Close the source drawer. Capture the
application below the 52px route header, preserving its aspect ratio. For Space
Game, start a fresh round and capture active gameplay, not the game-over screen.
Keep temporary full-page captures outside the repository.

The oscilloscope includes the canonical cabinet and controls. Space Game's
linked version requires WASD and Space; touch controls are not implemented.
Neither example runs behind these homepage images.
