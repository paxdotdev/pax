# Native navigation and default fonts

Run `pax-cli run --path examples/fixtures/native-navigation --target linux`
or use `--target windows` / `--target web`. To exercise baked cartridges, use
`pax-cli build --path examples/fixtures/native-navigation --target linux --release`
(substituting the target), then launch or serve the complete output directory.
Desktop hosts require their documented GPU/runtime prerequisites.

The buttons send current-target navigation through `NodeContext::navigate_to`;
the Home link exercises declarative `Link`. The live JSON panel reads Router's
actual location. `PAX_NAVIGATION_LOG` optionally records each changed location
as a JSON line in native builds, including Release. There is no fixture-only
navigation implementation.

The fixture deliberately supplies no font family or source. Its ordinary,
bold, and italic text qualify the standard default font and packaged assets.
The repeated `@` glyphs provide a distinct width signature for comparison with
common Windows fallback fonts; an authored family name alone is not proof of
the font used to draw the text.
Windows/Linux packages must run without a network font request or an installed
Roboto family. The test should also be run after copying the complete package
to a new directory and removing access to its build tree.

Execute Absolute → Query → Sibling → Parent → Unicode → Fragment → Home link.
Dragging to select the Home link's text must leave the location unchanged;
an ordinary click must navigate once. Keep native text selection enabled for
this check.
Expected locations and direct native routing edge cases live in
`tests/src/native-platform-probes/navigation-cases.json`. Native routes do not
provide a browser history stack. See [routing](../../../pax-docs/book/src/routing.md#native-locations).
