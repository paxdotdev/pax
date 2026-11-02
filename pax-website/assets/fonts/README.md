# Website fonts

## Manrope

`Manrope.ttf` is the Manrope variable TrueType font (weight axis 200–800),
distributed under the adjacent SIL Open Font License. Downloaded from
[Google Fonts](https://github.com/google/fonts/tree/main/ofl/manrope);
the Google Fonts metadata names upstream revision
`6f81ebecdf65e4463b798cc07b16a4f8d5216917` of `googlefonts/manrope`.

SHA-256: `3ae11c49db0455a3cc33e37d380f20fdb8c7f8b41dc07625c177e3d87a9d6ae6`.

The explicit `tools/trace-hero` generator uses these exact bytes for HarfRust
shaping and Skrifa outlines at weight 800. The website ships the resulting paths
instead of embedding the font in Wasm; regenerate and check the outlines as
described in the [website README](../../README.md#hero-lettering). All website Manrope styles use this same asset and the actual family
name, `Manrope`. Web loads `assets/fonts/Manrope.ttf`; Apple chassis register
the bundled file at startup and select its ExtraBold named instance (weight 800).
Keep all consumers on the same file when updating the font.

The same font and license are bundled under `pax-std/assets/fonts/` for the
shared ExampleHost UI and standalone demos. Keep both copies byte-identical;
the compiler merges identical dependency assets into one output file.

## IBM Plex Mono

The website bundles its existing normal-style IBM Plex Mono faces at weights
400, 500 and 600. Each declaration names the matching Regular, Medium or SemiBold
file under `assets/fonts/`, preserving its authored family, style and weight.
These relative asset URLs use the generated Web document base, including a
configured deployment subpath, and the native packaged asset roots. They replace
the Google Fonts CSS request without substituting another typeface.

Downloaded on 2026-10-21 from the official Google Fonts repository at commit
`45e3d5269e4a17d823f6d84470bc084dc5ae46f6`. The unmodified files are distributed under
the adjacent `OFL-IBMPlexMono.txt`. Sources and SHA-256 checksums:

| Local file | Pinned upstream source | SHA-256 |
| --- | --- | --- |
| `IBMPlexMono-Regular.ttf` | [Regular](https://github.com/google/fonts/blob/45e3d5269e4a17d823f6d84470bc084dc5ae46f6/ofl/ibmplexmono/IBMPlexMono-Regular.ttf) | `6a3412f058c7d8dfd9170c41e85ade48e5156ecb89356110ca57a0a27734af46` |
| `IBMPlexMono-Medium.ttf` | [Medium](https://github.com/google/fonts/blob/45e3d5269e4a17d823f6d84470bc084dc5ae46f6/ofl/ibmplexmono/IBMPlexMono-Medium.ttf) | `a9b4c49bb299e05b5f6c481e7fb5e78943d2793249a0c8874ab574a2d1ea6755` |
| `IBMPlexMono-SemiBold.ttf` | [SemiBold](https://github.com/google/fonts/blob/45e3d5269e4a17d823f6d84470bc084dc5ae46f6/ofl/ibmplexmono/IBMPlexMono-SemiBold.ttf) | `d3c38e55c78f5b0f28009fddba4834ec503278936a5986032424c9bd2d23aa46` |
| `OFL-IBMPlexMono.txt` | [IBM Plex Mono OFL.txt](https://github.com/google/fonts/blob/45e3d5269e4a17d823f6d84470bc084dc5ae46f6/ofl/ibmplexmono/OFL.txt) | `7e6b2818edbd8f6a01ae80641cc8f16a51080d08fb4e532be3a0b6f74adb07da` |

These three faces and their license total 416,956 bytes. The shared source viewer uses the bundled Manrope face for its chrome and
JetBrains Mono for code. Its monospace assets live in
[`pax-std/assets/pax-example-host-fonts/`](../../../pax-std/assets/pax-example-host-fonts/README.md).
