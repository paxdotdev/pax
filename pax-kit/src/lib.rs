//! The application-facing Pax facade: components, properties, `runnable!`, and
//! platform hosting through one dependency.
//!
//! Starters enable `standalone` (hosting) and `designtime` by default. Ordinary
//! `cargo run` launches web development; `cargo run -- pax --target=ios` selects
//! an Apple target on a Mac. The shared packager needs no installed Pax CLI or
//! application build script. `init_with!` always initializes in the current
//! native process for application-owned startup and embedding. Merely building
//! or depending on a UI library never launches it.

//Reexport primary modules
pub use pax_engine;
pub use pax_std;

/// Development launch, native application entry, and Apple view embedding.
/// Enable the `host` feature in the launcher, normally through
/// `standalone = ["pax-kit/host"]` in the application's Cargo manifest.
/// `designtime` adds workstation launch through the shared packager. Production
/// packaging excludes designtime; packaged native applications run directly.
///
/// ```ignore
/// fn main() -> Result<(), pax_kit::host::Error> {
///     pax_kit::init!()?.run()
/// }
/// ```
#[cfg(feature = "host")]
pub mod host;

/// Prepare the native UI, using defaults or explicit startup data.
/// These are the same functions available in [`host`]; both require the `host`
/// feature and run the UI initializer on the process main thread.
#[cfg(all(feature = "host", any(target_os = "macos", target_os = "ios")))]
pub use pax_host::{init, init_with};

//Splat-export certain curated modules, for ergo
pub use pax_engine::api::*;
pub use pax_engine::rendering;
pub use pax_engine::*;
pub use pax_std::*;
pub use pax_std::{drawing, layout};
