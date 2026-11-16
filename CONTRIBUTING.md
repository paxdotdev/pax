# CONTRIBUTING

We warmly welcome contributions!  Pax is free and open source software, licensed MIT / Apache 2.0.

### Running the monorepo

Start with the [workstation prerequisites](pax-docs/book/src/getting-started.md#prepare-your-workstation)
and the [contributor workflow](pax-docs/book/src/developer-workflow.md#develop-pax-from-a-checkout).
Native Windows and Linux builds additionally need the
[desktop prerequisites](pax-docs/book/src/targets-build-deploy.md#desktop-prerequisites)
and must run on the corresponding operating system.

The [curated examples](examples/README.md) and [supporting fixtures](examples/fixtures/README.md)
are testbeds for developing the monorepo. Their Cargo path dependencies select
the checked-out Pax crates. From the repository root, build and run the local CLI:

```sh
cargo run -p pax-cli -- run --libdev --path examples/fixtures/fireworks --target web
```

Web interface builds also need the Node version in `.nvmrc`, npm, and Bash on
`PATH`. For direct native execution on macOS, use `cargo run -p fireworks`
or `cargo run --example fireworks`. The directory move preserves both Cargo names.

### Communicating with our team
We strongly recommend reaching out to our team on [Discord](https://discord.gg/Eq8KWAUc6b) before embarking on any substantial contributions, in
case your work overlaps or conflicts with our roadmap.  
