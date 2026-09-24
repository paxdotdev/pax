# pax-chassis-web/interface

This project acts as the interface between the Pax engine, via the chassis, and a native platform, in this case
the Web browser.

Written in TypeScript, this project handles receiving and unpacking messages from the engine via the chassis, as well
as exposing APIs for initializing the chassis+engine, for attaching requestAnimationFrame to drive the engine's tick,
and exposing an index.html that works as a host for the static site built with Pax, as well as an index.js and 
other wrappers (e.g. React, Angular, Vue, WebComponents) for other consumption patterns.  

Output artifacts are built into `.pax/build` for a given project.

## Query-backed embedding

The default entry document accepts `?pax_route=%2F` for hosting an example below
another site's directory. It resolves assets beside the physical entry file,
while routing and document metadata use the application location encoded in
`pax_route`. Same-tab navigation updates that parameter; history and reload
preserve the host path. Without the parameter, normal pathname routing and
the generated site's base URL are unchanged. Custom HTML hosts must retain
the embedded-base setup before loading scripts or styles.

## Server-owned paths

The compiler embeds the validated Cargo web setting `server_owned_prefixes`
in `<script type="application/json" id="pax-web-config">`. Keep that element
in custom entry documents. `utils/navigation.ts` reads it synchronously and
delegates matching current-tab, same-origin destinations to browser navigation
before app history or metadata updates. The current frame navigates even when
its app normally uses `pax_route`. The sibling `pax-web-config.json` lets local
servers apply the same policy to their history fallback without loading Wasm.
