# CONTRIBUTING

We warmly welcome contributions!  Pax is free and open source software, licensed MIT / Apache 2.0.

### Running the monorepo
Follow the *workstation* (e.g. macOS, Linux) and *target* (e.g. WebAssembly, macOS app) setup instructions here: https://docs.pax.dev/start-creating-a-project.html

The [curated examples](examples/README.md) and [supporting fixtures](examples/fixtures/README.md)
are testbeds for developing the monorepo. Changes to crates such as `pax-runtime`
are reflected when rebuilding them. From the repository root, use
`pax-cli run --path examples/fixtures/fireworks --target web` on any supported
workstation. For direct native execution on macOS, use `cargo run -p fireworks`
or `cargo run --example fireworks`. The directory move preserves both Cargo names.

### Communicating with our team
We strongly recommend re aching out to our team on [Discord](https://discord.gg/Eq8KWAUc6b) before embarking on any substantial contributions, in
case your work overlaps or conflicts with our roadmap.  
