//! Linux GPU transport for the native desktop cartridge.
//!
//! The engine and retained renderer belong to the shared desktop chassis. This
//! crate negotiates external Vulkan memory and manages its fenced ownership.

#[cfg(target_os = "linux")]
mod vulkan;
#[cfg(target_os = "linux")]
pub use vulkan::create_backend;
