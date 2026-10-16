//! DX12 exports consumed by the WinUI desktop host on the same GPU queue.

pub use pax_chassis_common::desktop::*;

#[cfg(target_os = "windows")]
mod gpu;
#[cfg(target_os = "windows")]
pub use gpu::create_backend;

#[cfg(not(target_os = "windows"))]
pub fn create_backend() -> Result<Box<dyn DesktopGpuBackend>, String> {
    Err("The Windows desktop chassis requires Windows and a DX12 hardware adapter".into())
}
