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

#[cfg(test)]
mod tests {
    #[test]
    fn floating_window_chrome_does_not_intercept_projected_controls() {
        let template = include_str!("device_frame.pax");
        let chrome = template.split("<Frame").next().unwrap();
        assert_eq!(chrome.matches("_raycastable=false").count(), 4);
        assert!(chrome.contains("fill=TRANSPARENT _raycastable=false"));
    }
}
