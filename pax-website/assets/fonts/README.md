# Hero font

`Manrope.ttf` is the Manrope variable TrueType font (weight axis 200–800),
distributed under the adjacent SIL Open Font License. Downloaded from
[Google Fonts](https://github.com/google/fonts/tree/main/ofl/manrope);
the Google Fonts metadata names upstream revision
`6f81ebecdf65e4463b798cc07b16a4f8d5216917` of `googlefonts/manrope`.

SHA-256: `3ae11c49db0455a3cc33e37d380f20fdb8c7f8b41dc07625c177e3d87a9d6ae6`.

The hero embeds these exact bytes for HarfRust shaping and Skrifa outlines at
weight 800. All website Manrope styles use this same asset and the actual family
name, `Manrope`. Web loads `/assets/fonts/Manrope.ttf`; Apple chassis register
the bundled file at startup and select its ExtraBold named instance (weight 800).
Keep all consumers on the same file when updating the font.
