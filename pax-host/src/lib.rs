//! Native process entry and mounting without linking the application cartridge
//! into the launcher. Build Apple applications with `pax-cli`; it links the
//! matching PaxHost framework and packages the separate cartridge.
//!
//! ```ignore
//! # #[cfg(any(target_os = "macos", target_os = "ios"))]
//! fn main() -> Result<(), pax_host::Error> {
//!     let application = pax_host::init()?;
//!     application.run()
//! }
//! ```

#[cfg(any(target_os = "macos", target_os = "ios"))]
/// Apple process entry, view ownership and asynchronous mount observations.
pub mod native;
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub use native::*;
