# Native default font

These unmodified Roboto variable fonts provide Pax's default family offline on
Windows, Linux, and Android. The native standard-library asset inventory carries them
into both debug and baked release packages. Normal and italic faces include the
100–900 weight range; the width axis uses its default value.

Source: [Google Fonts, revision 1c627bfa375fc51cf86fabeca4f6e08a95f0aa5c](https://github.com/google/fonts/tree/1c627bfa375fc51cf86fabeca4f6e08a95f0aa5c/ofl/roboto).
The filenames are shortened for resource URLs; the font data is unchanged.
Redistribution terms are in `OFL-Roboto.txt`.

| Local file | Upstream file | SHA-256 |
| --- | --- | --- |
| `Roboto.ttf` | `Roboto[wdth,wght].ttf` | `d7598e12c5dbef095ff8272cfc55da0250bd07fbdecbac8a530b9b277872a134` |
| `Roboto-Italic.ttf` | `Roboto-Italic[wdth,wght].ttf` | `9725a847af6b460ffca162ae66d20dad48b01876137947180b42d7dcd7887182` |

Only the standard default's exact family/source pair is resolved to these files
at the Windows/Linux/Android message boundary. Custom font URLs retain their meaning.
The macro frontend adds this asset root only when the owning Rust compilation
targets Windows, Linux, or Android, before manifest hashing and binary baking. Web and
Apple neither bundle nor register these files, preserving their existing font
loading and package sizes.
