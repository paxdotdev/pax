# Handwriter Fonts

These SVG stroke fonts are bundled from the selected subset of the Evil Mad
Scientist "Hershey Text" SVG font collection:

- `EMSAllure`
- `EMSDelight`
- `EMSInvite`
- `EMSLeague`
- `EMSNeato`
- `EMSOsmotron`
- `EMSReadability`
- `EMSTech`
- `HersheySans1`
- `HersheyScript1`

Each font also has a `*Curved.svg` companion generated from the original SVG
font by converting straight-line glyph runs into lightly smoothed cubic
Beziers. Regenerate those companions with `generate_curved.py`. The generated
companions preserve pen lifts and sharp corners where possible, but are fitted
trial assets rather than authoritative font sources.

- Source: https://gitlab.com/oskay/svg-fonts
- Retrieved for this repository on 2026-07-07.
- EMS fonts are derived from SIL Open Font License fonts. The upstream SVG
  files retain their embedded font-specific metadata and license references.
- Hershey fonts retain their embedded Hershey attribution and usage notes.

Keep these files as source assets: `Handwriter` parses them into Pax
`PathElement` data at runtime initialization for the selected text/font pair.
