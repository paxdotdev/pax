# Custom loading screen

This web fixture uses `loader = "crate::BootScreen"` to compose the standard Pax
logo/bar with a phase label, cancellation, and a 600 ms exit transition. The main
application contains a native text field and canvas animation. Its entry timeline
starts beneath the exiting loader; the text field cannot receive input until
that exit finishes.

Install the splitter tools described in the [loading-screen guide](../../../pax-docs/book/src/components-composition.md#loading-context-components-experimental), then run:

```sh
pax-cli run --path examples/fixtures/loading-screen --target web --release
```

Append `?pax_loader_ms=5000` to the served URL to slow the application-module stream
for inspection. Cancel, then Retry, to exercise a fresh attempt. The default
screen's Retry button uses the same loading context as this custom Cancel button.
A normal reload without the query uses real network progress.

Loading-screen packaging currently targets web. Other chassis still initialize
the main application directly.
