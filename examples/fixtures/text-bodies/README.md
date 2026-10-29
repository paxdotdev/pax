# Text bodies

A regression fixture for ordinary text bodies, block indentation, character
references, exact raw strings, explicit Markdown, a custom `#[pax(content)]`
receiver, reactive Button content, and editable Text two-way bindings.

Run `pax-cli run --path examples/fixtures/text-bodies --target web --libdev`
from the monorepo root, or `cargo run -p text-bodies` for native macOS.
Click the button and edit the note to check reactive updates. Change or remove
a body while the CLI runs to exercise template hot reload. The raw specimen
must retain its leading spaces, literal markup, backslash, and final newline.

See [text-body syntax](../../../pax-docs/book/src/template-language.md#text-bodies)
for whitespace rules. Markdown rendering remains target dependent.
