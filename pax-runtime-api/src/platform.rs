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

/// Runtime-inherited Apple liquid-glass effect scope.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct NativeLiquidGlassScope {
    /// Stable node id for the nearest liquid-glass scope.
    pub group_id: u32,
    /// Desired spacing between grouped glass surfaces, in pixels.
    pub spacing: f64,
    /// Whether supported Apple surfaces should use the interactive glass effect.
    pub interactive: bool,
    /// Optional tint for supported Apple surfaces.
    pub tint: Option<Color>,
    /// Apple glass style name, currently "regular" or "clear".
    pub variant: String,
}

impl NativeLiquidGlassScope {
    /// Converts runtime style data into the serialized native-message payload.
    pub fn to_message(&self) -> AppleLiquidGlassPatch {
        AppleLiquidGlassPatch {
            group_id: self.group_id,
            spacing: self.spacing,
            interactive: self.interactive,
            tint: self.tint.as_ref().map(Into::into),
            variant: self.variant.clone(),
        }
    }
}

impl Interpolatable for NativeLiquidGlassScope {}

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

/// Current device orientation reported by a gyroscope/orientation sensor.
///
/// On web targets, this is sourced from `DeviceOrientationEvent` and mapped as:
/// `x = beta`, `y = gamma`, and `z = alpha`, all in degrees.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Gyro {
    /// Front-to-back tilt, in degrees.
    pub x: f64,
    /// Left-to-right tilt, in degrees.
    pub y: f64,
    /// Compass/z-axis rotation, in degrees.
    pub z: f64,
}

impl ToPaxValue for Gyro {
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Object(
            vec![
                ("x".to_string(), self.x.to_pax_value()),
                ("y".to_string(), self.y.to_pax_value()),
                ("z".to_string(), self.z.to_pax_value()),
            ]
            .into_iter()
            .collect(),
        )
    }
}

impl Interpolatable for Gyro {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        Gyro {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
            z: self.z + (other.z - self.z) * t,
        }
    }
}

/// Current device acceleration, in meters per second squared.
///
/// On web targets, this uses `DeviceMotionEvent.accelerationIncludingGravity`
/// when available, falling back to `DeviceMotionEvent.acceleration`.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Accel {
    /// Acceleration along the x axis.
    pub x: f64,
    /// Acceleration along the y axis.
    pub y: f64,
    /// Acceleration along the z axis.
    pub z: f64,
}

impl ToPaxValue for Accel {
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Object(
            vec![
                ("x".to_string(), self.x.to_pax_value()),
                ("y".to_string(), self.y.to_pax_value()),
                ("z".to_string(), self.z.to_pax_value()),
            ]
            .into_iter()
            .collect(),
        )
    }
}

impl Interpolatable for Accel {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        Accel {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
            z: self.z + (other.z - self.z) * t,
        }
    }
}

/// Phantom coordinate space representing the outer window.
pub struct Window;

impl Space for Window {}
