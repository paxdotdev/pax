# Handwriter Fonts

These five SVG stroke fonts are bundled from the selected subset of the Evil Mad
Scientist "Hershey Text" SVG font collection:

- `EMSAllure`
- `EMSLeague`
- `EMSReadability`
- `EMSTech`
- `HersheyScript1`

- Source: https://gitlab.com/oskay/svg-fonts
- Retrieved for this repository on 2026-07-07.
- EMS fonts are derived from SIL Open Font License fonts. The upstream SVG
  files retain their embedded font-specific metadata and license references.
- Hershey fonts retain their embedded Hershey attribution and usage notes.

Keep these files as source assets: `Handwriter` parses them into Pax
`PathElement` data at runtime initialization for the selected text/font pair.
