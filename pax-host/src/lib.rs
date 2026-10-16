//! Native process entry and mounting across a separate cartridge ABI.
//! Applications use `pax_kit::init!` and `pax_kit::init_with!`; options, errors,
//! and embedding types live in `pax_kit::host`. Enable `pax-kit/host` without
//! adding a direct `pax-host` dependency. Cargo builds the SDK for native hosting.
//! The CLI provides bundles, development servers, and iOS deployment;
//! initialization itself stays in the calling native process.
//!
//! Direct Windows Cargo launchers stage embedded TTF/OTF/TTC resources into a
//! verified cache beside the executable for WinUI resource loading. This needs a
//! writable executable directory with hard-link support; packaged applications
//! use their existing copied assets and do not extract fonts at startup. No
//! compiler or SDK build tool runs during initialization.
//! Direct desktop Cargo launches also verify that the separately built cdylib
//! matches the launcher's generated program identity. Rebuild both artifacts
//! after Rust changes; a stale cartridge is rejected before initialization.
//! Windows CLI packages merge Windows 10/11 compatibility and PerMonitorV2 DPI
//! declarations into the launcher EXE. Direct Cargo applications must supply
//! their own executable manifest for OS compatibility. The host establishes
//! PerMonitorV2 awareness before creating UI and rejects incompatible awareness
//! already established by the application.
//!
//! ```ignore
//! fn main() -> Result<(), pax_kit::host::Error> {
//!     pax_kit::init!()?.run()
//! }
//! ```

#[cfg(any(test, target_os = "windows"))]
mod fonts;
#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "windows",
    target_os = "linux"
))]
mod library;
#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "windows",
    target_os = "linux"
))]
/// Native process entry, view ownership and asynchronous mount observations.
pub mod native;
#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "windows",
    target_os = "linux"
))]
pub use native::*;

#[doc(hidden)]
pub mod sources;

/// An initialization, native application loop, or mounting failure.
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
