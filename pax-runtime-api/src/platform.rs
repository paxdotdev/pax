//! Platform, viewport, and coordinate-space marker types.

use super::*;

/// Describes known operating systems / targets.
#[derive(Default, Debug, Clone, Copy)]
pub enum OS {
    /// macOS.
    Mac,
    /// Linux desktop.
    Linux,
    /// Windows desktop.
    Windows,
    /// Android.
    Android,
    /// iOS on iPhone-class devices.
    IPhone,
    /// OS has not been detected or reported.
    #[default]
    Unknown,
}

impl OS {
    /// Helper to determine if the OS is a mobile platform.
    pub fn is_mobile(&self) -> bool {
        match self {
            OS::Android | OS::IPhone => true,
            _ => false,
        }
    }

    /// Helper to determine if the OS is a desktop platform.
    pub fn is_desktop(&self) -> bool {
        match self {
            OS::Mac | OS::Linux | OS::Windows => true,
            _ => false,
        }
    }
}

/// Describes categories of known platforms, for differentiating certain engine behaviors.
#[derive(Default, Debug, Clone, Copy)]
pub enum Platform {
    /// Browser-hosted rendering target.
    Web,
    /// Native application target.
    Native,
    /// Platform unknown or not yet reported.
    #[default]
    Unknown,
}

/// Struct representing the outermost viewport of a rendering scene, for example a browser window
/// or native application window.
#[derive(Default, Debug, Clone, Copy)]
pub struct Viewport {
    /// Viewport width in pixels.
    pub width: f64,
    /// Viewport height in pixels.
    pub height: f64,
}

impl ToPaxValue for Viewport {
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Object(
            vec![
                ("width".to_string(), self.width.to_pax_value()),
                ("height".to_string(), self.height.to_pax_value()),
            ]
            .into_iter()
            .collect(),
        )
    }
}

impl Interpolatable for Viewport {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        Viewport {
            width: self.width + (other.width - self.width) * t,
            height: self.height + (other.height - self.height) * t,
        }
    }
}

/// Phantom coordinate space representing the outer window.
pub struct Window;

impl Space for Window {}
