//! Native process entry and mounting without linking the application cartridge
//! into the launcher. Build Apple applications with `pax-cli`; it links the
//! matching PaxHost framework and packages the separate cartridge.
//! Applications normally use `pax_kit::init` or `pax_kit::init_with`, with options,
//! errors and embedding types in `pax_kit::host`. Enable the `pax-kit/host`
//! feature; no direct `pax-host` dependency is needed.
//!
//! ```ignore
//! # #[cfg(any(target_os = "macos", target_os = "ios"))]
//! fn main() -> Result<(), pax_kit::host::Error> {
//!     let application = pax_kit::init()?;
//!     application.run()
//! }
//! ```

#[cfg(any(target_os = "macos", target_os = "ios"))]
/// Apple process entry, view ownership and asynchronous mount observations.
pub mod native;
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub use native::*;
