use crate::SiteTheme;
use pax_kit::*;

/// A quiet hardware silhouette; its received content is a real Pax scene.
/// The caller supplies its flow footprint; the internal scene is breakout so
/// a demo's scrollable content cannot enlarge the surrounding editorial layout.
#[pax]
#[file("device_frame.pax")]
pub struct DeviceFrame {
    pub kind: Property<String>,
    /// Extend phone content to the inner bevel, with island and home indicator overlaid.
    pub edge_to_edge: Property<bool>,
}
