//! Desktop engine ownership and GPU handoff for the Windows and Linux hosts.
//!
//! The platform backend and retained renderer live in the cartridge. Only copied
//! JSON, opaque session/frame tokens, and explicitly retained native GPU handles
//! cross the host ABI. All methods run on the owning UI thread.

use pax_gpu::render_backend::SharedGpuContext;
use serde::{Deserialize, Serialize};

mod session;
#[cfg(any(target_os = "windows", target_os = "linux"))]
mod rich_text;
pub use session::*;

/// One host-selected physical tile and backing generation.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSurfaceInfo {
    /// Logical runtime render layer.
    pub layer: usize,
    /// Stable tile key within the logical layer.
    pub key: String,
    /// Parent host and backing generation. Replace when pixel dimensions change.
    pub host_signature: String,
    /// Tile origin in its layer's logical coordinate space.
    pub origin_x: f32,
    /// Tile origin in its layer's logical coordinate space.
    pub origin_y: f32,
    /// Logical tile width.
    pub logical_width: f32,
    /// Logical tile height.
    pub logical_height: f32,
    /// Backing width in physical pixels.
    pub width: u32,
    /// Backing height in physical pixels.
    pub height: u32,
    /// Physical pixels per logical unit on each axis.
    pub dpr: [f32; 2],
    /// Visible tiles have priority zero; positive priorities represent warm tiles.
    #[serde(default)]
    pub replay_priority: i32,
}

/// A retained renderer output, borrowed by handle for an export submission.
pub struct DesktopSurfaceFrame {
    /// The exact tile generation corresponding to this texture.
    pub info: DesktopSurfaceInfo,
    /// Mutable renderer output. A backend must submit its copy on the context's
    /// queue before returning; retaining this handle does not freeze its pixels.
    pub texture: wgpu::Texture,
}

/// Platform-owned export allocations and synchronization on one WGPU device.
///
/// Rendering and publishing use the same queue. Each published token remains
/// owned by the consumer until `release`; reuse additionally requires producer
/// and consumer GPU completion. Polling and backpressure never block the UI.
/// Errors are terminal for this session unless the backend can prove recovery.
pub trait DesktopGpuBackend {
    /// Device negotiated with this platform's external-memory requirements.
    fn context(&self) -> SharedGpuContext;
    /// Byte-premultiplied compositor format; RGBA8 or BGRA8 unorm only.
    fn format(&self) -> wgpu::TextureFormat;
    /// Process completed fences and resource retirement without waiting.
    fn poll(&mut self) -> Result<(), String>;
    /// Check bounded capacity for the active surface set before rendering. This
    /// does not reserve a slot: the subsequent publication may contain only its
    /// changed subset, or be omitted when every surface is unchanged.
    /// False leaves current presentation intact and does not consume a slot.
    fn can_publish(&mut self, surfaces: &[DesktopSurfaceInfo]) -> Result<bool, String>;
    /// Queue copies and return platform descriptors, including a unique `token`
    /// per exported image. Native handles stay valid until release and GPU
    /// retirement. Publication is transactional: errors retain submitted work.
    fn publish(&mut self, surfaces: Vec<DesktopSurfaceFrame>) -> Result<serde_json::Value, String>;
    /// Accept consumer completion evidence for exactly one publication token.
    /// Descriptor fields are platform-defined, versioned by the backend output.
    /// Reject duplicate, stale and unknown tokens without changing ownership.
    fn release(&mut self, token: u64, completion: serde_json::Value) -> Result<(), String>;
    /// Begin/continue shutdown, rejecting new publications. True means every
    /// export can safely be destroyed; this must never wait for a GPU fence.
    fn shutdown_poll(&mut self) -> Result<bool, String>;
    /// Adapter, transport, live/retiring allocations and pending frame counts.
    fn diagnostics(&self) -> serde_json::Value;
}

/// Creates a platform backend inside the cartridge, never across the C ABI.
pub type DesktopGpuFactory = fn() -> Result<Box<dyn DesktopGpuBackend>, String>;
