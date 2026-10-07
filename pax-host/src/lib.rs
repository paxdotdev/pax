//! Apple process entry and mounting across a separate cartridge ABI.
//! Applications use `pax_kit::init!` and `pax_kit::init_with!`; options, errors,
//! and embedding types live in `pax_kit::host`. Enable `pax-kit/host` without
//! adding a direct `pax-host` dependency. The SDK is built on demand for native
//! development. The shared compiler provides bundles and iOS deployment through
//! either the Cargo development entry or the CLI.
//!
//! ```ignore
//! fn main() -> Result<(), pax_kit::host::Error> {
//!     pax_kit::init!()?.run()
//! }
//! ```

#[cfg(any(target_os = "macos", target_os = "ios"))]
/// Apple process entry, view ownership and asynchronous mount observations.
pub mod native;
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub use native::*;

#[doc(hidden)]
pub mod sources;

#[cfg(all(feature = "designtime", target_os = "macos"))]
mod sdk;

/// An initialization, development launch, or mounting failure.
#[derive(Debug, Clone)]
pub struct Error(pub(crate) String);
impl Error {
    #[doc(hidden)]
    pub fn message(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for Error {}

/// Copied application inputs for the separately linked cartridge.
#[derive(Default)]
pub struct InitOptions {
    /// Application-defined encoding, at most 64 KiB. The root initializer reads
    /// these bytes through `InitContext::startup_data()`.
    pub startup_data: Vec<u8>,
    /// Canonical root path from `runnable!`; required for multiple distinct roots.
    pub entry: Option<String>,
}
