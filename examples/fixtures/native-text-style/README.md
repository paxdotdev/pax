# Native text styling

Six fixed Text swatches separate block alignment from multiline line alignment.
The pale rectangles mark the complete authored bounds. A reactive style toggle
changes color, weight, slant and underline while genuine native editors,
Dropdown and RadioList remain mounted. Edit both Textboxes before and after
toggling, including deleting all text and typing into the empty multiline box.

```sh
pax-cli run --path examples/fixtures/native-text-style --target web
```

For the experimental desktop hosts, use `--target windows` or `--target linux`
on that platform. The native launcher starts at 1024 × 840 logical pixels.
The fixture explicitly requests the system Arial family; the toolkit may use a
fallback where that family is unavailable. It does not bundle a font or assert
identical glyph metrics across platforms.

Use the [text guide](../../../pax-docs/book/src/text-fonts-images.md) and each
host's qualification notes for current support limits. A rendered closed
Dropdown does not qualify the popup's separate native surface. This fixture
uses uniform plain-text styles, not Markdown or rich-text editing.
