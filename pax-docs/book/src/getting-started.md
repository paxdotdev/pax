# Getting Started
<!-- summary: How Pax projects are structured and run from the CLI. -->
<!-- tags: foundations, cli, build -->

- Development environment: platform-specific setup guide, dependencies, etc.
- CLI basics: `pax-cli create`, `pax-cli run`, and `pax-cli build` workflows.
- Project layout: `src/lib.pax`, `src/lib.rs`, assets, and Cargo manifest basics.
- Target platforms and build modes: web, macOS, iOS, and iPadOS; debug vs release.
- Assets and URLs: image sources, web fonts, and static files.
- Deployment and packaging overview: where build outputs land and how to ship them.

## Project Metadata

Rust-backed Pax projects can define build-time project metadata in `Cargo.toml`
under `[package.metadata.pax]`. Pax reads this table while materializing the web
or Apple interface. Cargo ignores the table, so it is safe to keep Pax-specific
packaging data here.

```toml
[package.metadata.pax]
title = "Pax Example"
icon = "assets/icon.png"

[package.metadata.pax.web]
title = "Pax Web"
favicon = "assets/favicon.png"

[package.metadata.pax.ios]
title = "Pax iOS"
bundle_identifier = "dev.pax.example"
development_team = "ABCDE12345"

[package.metadata.pax.ios.info_plist]
NSCameraUsageDescription = "Capture photos when the camera picker is used."

[package.metadata.pax.macos]
title = "Pax macOS"
bundle_identifier = "dev.pax.example.macos"
```

Common keys are inherited by target-specific tables. iPadOS first checks
`[package.metadata.pax.ipados]`, then falls back to iOS, then to common values.
Paths are relative to the project root unless absolute. Explicit CLI flags still
take precedence over Cargo metadata where both exist.

Supported common keys:

- `title`: browser title, Apple display name, and target title default.
- `icon`: shared source image for generated target icons and fallback web favicon.
- `bundle_identifier`: Apple bundle identifier default.
- `marketing_version`: Apple marketing version default. If omitted, Cargo
  `package.version` is used for Apple builds.
- `build_number`: Apple build number.
- `development_team`: Apple development team for signing.
- `info_plist`: shared string values emitted into generated Apple Info.plists.

Supported web keys:

- `title`: overrides the common title for web.
- `favicon`: source file copied as the web favicon.
- `icon`: web-specific icon source used to generate a fallback favicon.

Supported Apple target keys for `ios`, `ipados`, and `macos`:

- `title`
- `icon`
- `bundle_identifier`
- `marketing_version`
- `build_number`
- `development_team`
- `info_plist`: string values emitted into generated Apple Info.plists. Declare
  as a nested table, for example
  `[package.metadata.pax.ios.info_plist] NSCameraUsageDescription = "..."`

iOS and iPadOS use a single generated 1024x1024 `AppIcon` asset. The source
image must be square and opaque. macOS generates the full AppIcon size set from
the same square source image.
