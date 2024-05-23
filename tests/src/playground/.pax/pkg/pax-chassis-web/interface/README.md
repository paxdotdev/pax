# pax-chassis-web/interface

This project acts as the interface between the Pax engine, via the chassis, and a native platform, in this case
the Web browser.

Written in TypeScript, this project handles receiving and unpacking messages from the engine via the chassis, as well
as exposing APIs for initializing the chassis+engine, for attaching requestAnimationFrame to drive the engine's tick,
and exposing an index.html that works as a host for the static site built with Pax, as well as an index.js and 
other wrappers (e.g. React, Angular, Vue, WebComponents) for other consumption patterns.  

This project is copied into `.pax/pkg` and the output artifacts are built into `.pax/build` for a given project.