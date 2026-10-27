use crate::SiteTheme;
use pax_kit::*;

/// A quiet device or floating-window silhouette for live scenes or captures.
/// The caller supplies its flow footprint; the internal scene is breakout so
/// a demo's scrollable content cannot enlarge the surrounding editorial layout.
#[pax]
#[file("device_frame.pax")]
pub struct DeviceFrame {
    /// `phone`, `tablet`, `desktop` (laptop), or `window` (macOS-style title bar).
    pub kind: Property<String>,
    /// Extend phone content to the inner bevel, with island and home indicator overlaid.
    pub edge_to_edge: Property<bool>,
}
