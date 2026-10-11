//! The application-facing Pax facade: components, properties, `runnable!`, and
//! platform hosting through one dependency.
//!
//! Starters enable `standalone` (hosting) and `designtime` by default. On macOS,
//! `cargo run` runs the native application. `init!` and `init_with!` initialize
//! in the current native process, without parsing arguments or starting
//! development tools. Use `pax-cli` for serving, template reload, and platform
//! packaging/deployment. Macros compile the UI without an application build
//! script; merely building or depending on a UI library never launches it.

//Reexport primary modules
pub use pax_engine;
pub use pax_std;

/// Native application entry and Apple view embedding.
/// Enable the `host` feature in the executable, normally through
/// `standalone = ["pax-kit/host"]` in the application's Cargo manifest.
/// `designtime` enables runtime inspection and template changes; it does not
/// start a development server. Production packaging excludes designtime.
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
