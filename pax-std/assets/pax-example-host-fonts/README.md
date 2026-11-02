# Example source viewer fonts

`ExampleHost` and `ExampleSourceViewer` use the bundled Manrope face in
`assets/fonts/Manrope.ttf` for chrome and JetBrains Mono at weight 400 for code.
JetBrains Mono is an unmodified variable TrueType file with a named 400 instance
and a 100–800 weight axis; its local filename omits the axis suffix.

These are ordinary component-package assets. Static component discovery adds
`pax-std/assets/` to the shared asset inventory. Web and native builds resolve the
same relative `assets/` paths, without a Google Fonts request.

JetBrains Mono and its license were downloaded on 2026-10-21 from Google Fonts
repository commit `45e3d5269e4a17d823f6d84470bc084dc5ae46f6`. The face is distributed
under the adjacent SIL Open Font License notice.

| Local file | Pinned upstream source | SHA-256 |
| --- | --- | --- |
| `JetBrainsMono.ttf` | [JetBrainsMono\[wght\].ttf](https://github.com/google/fonts/blob/45e3d5269e4a17d823f6d84470bc084dc5ae46f6/ofl/jetbrainsmono/JetBrainsMono%5Bwght%5D.ttf) | `48715a42ec242c21e9f02692891e147d022299a52e48d5e413e1a942193ffeda` |
| `OFL-JetBrainsMono.txt` | [JetBrains Mono OFL.txt](https://github.com/google/fonts/blob/45e3d5269e4a17d823f6d84470bc084dc5ae46f6/ofl/jetbrainsmono/OFL.txt) | `b2fe5e8987594e9ffd1d2ca52a2f5d73eb8335243893c5d6254b5ad69269591d` |


Asset discovery is currently per package, so applications using other `pax-std`
components also package these files. Web requests a font only when its face is
used; no font data is embedded in Web's Wasm. Direct native Cargo launchers embed
and stage the font inventory, as they do for other packaged fonts.
