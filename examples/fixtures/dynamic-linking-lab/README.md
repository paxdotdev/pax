# Dynamic Linking Lab (historical)

This fixture preserves the source editor experiment. Rust logic replacement is
no longer supported. The Rust pane is a preview; save Rust edits in your editor,
then rebuild and restart. The old reload button explains that limitation.

Pax template editing still uses the debug development server:

```sh
pax-cli run --path examples/fixtures/dynamic-linking-lab --target web --hot-reload=pax
```

The separate cartridge boundary remains available for platform linking and
future work. This fixture is not a supported dynamic-linking integration recipe.
