//! The application-facing Pax facade: components, properties, `runnable!`, and
//! platform hosting through one dependency.
//!
//! Enable the `host` feature to use `pax_kit::init`, `pax_kit::init_with`, and
//! `pax_kit::host` on macOS, iOS and iPadOS.
//! Starter applications enable it through their `standalone` feature; UI library
//! and web builds leave it disabled. Build native applications with `pax-cli`
//! so the matching platform framework is linked and packaged.
//! Unsupported targets report a diagnostic here, so ordinary launchers need
//! only one `main` function without platform conditionals.

#[cfg(all(feature = "host", not(any(target_os = "macos", target_os = "ios"))))]
compile_error!(
    "The pax-kit/host feature supports native entry on macOS, iOS and iPadOS. For web, use pax-cli run --target web without the standalone feature."
);

//Reexport primary modules
pub use pax_engine;
pub use pax_std;

/// Native application entry and embedding on macOS, iOS and iPadOS.
/// Enable the `host` feature in the launcher, normally through
/// `standalone = ["pax-kit/host"]` in the application's Cargo manifest.
/// Build with `pax-cli` to link the platform hosting framework.
///
/// ```ignore
/// fn main() -> Result<(), pax_kit::host::Error> {
///     pax_kit::init()?.run()
/// }
/// ```
#[cfg(all(feature = "host", any(target_os = "macos", target_os = "ios")))]
pub use pax_host as host;

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
