# Experimental font outlines

This unpublished Rust helper preserves the HarfRust/Skrifa work independently of
the website. It is not re-exported by `pax-kit`, and does not introduce a
`pax-std` primitive or a flag on `Text`.

`OutlineFont::new` accepts owned font bytes or a static byte slice and a weight.
The current scope is face zero of a variable TrueType/OpenType font with a
`wght` axis and cap-height metrics, single-line Latin LTR text, and unhinted
outlines. Unsupported input, invalid weights, and missing glyphs return errors.
No wrapping, bidi layout, font fallback, font loading, selection, or rendering
is performed.

`shape_line` preserves whole-run kerning and clusters, including supported
ligatures and combining sequences. Results expose UTF-8 source ranges, cluster
ordinals, glyph IDs, advance positions, font metrics, and Bézier commands in em
units. Coordinates use a baseline at y=0, positive y downward; retain overhangs
and contour winding. Repeated glyph IDs are not unique occurrence keys.
Outlines are cached for the lifetime of the font instance; runs own their data
and can outlive it. Shape only when input changes, not on animation frames.

The website uses this helper only from its explicit extraction tool and tests:

```sh
cargo run --manifest-path pax-website/tools/trace-hero/Cargo.toml -- --check
cargo test -p pax-text
```

See [the font article](../pax-docs/book/src/text-fonts-images.md#experimental-font-outlines)
for usage, and [the website](../pax-website/README.md#hero-lettering) for the
checked-in asset regeneration command. Primitive/component ergonomics remain a
separate design decision; this crate's API is experimental.
