//! Platform, viewport, and coordinate-space marker types.

use super::*;

impl Interpolatable for SafeAreaInsets {}

/// Describes known operating systems / targets.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
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
    /// iPadOS / iOS on iPad-class devices.
    IPad,
    /// OS has not been detected or reported.
    #[default]
    Unknown,
}

impl OS {
    /// Helper to determine if the OS is a mobile platform.
    pub fn is_mobile(&self) -> bool {
        match self {
            OS::Android | OS::IPhone | OS::IPad => true,
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

    /// Returns true for either iPhone-class iOS or iPadOS.
    pub fn is_ios(&self) -> bool {
        matches!(self, OS::IPhone | OS::IPad)
    }

    /// Returns true for iPhone-class iOS.
    pub fn is_iphone(&self) -> bool {
        matches!(self, OS::IPhone)
    }

    /// Returns true for iPadOS / iPad-class iOS.
    pub fn is_ipad(&self) -> bool {
        matches!(self, OS::IPad)
    }

    /// Returns true for macOS.
    pub fn is_macos(&self) -> bool {
        matches!(self, OS::Mac)
    }

    /// Returns true for Android.
    pub fn is_android(&self) -> bool {
        matches!(self, OS::Android)
    }

    /// Returns true for Windows.
    pub fn is_windows(&self) -> bool {
        matches!(self, OS::Windows)
    }

    /// Returns true for Linux.
    pub fn is_linux(&self) -> bool {
        matches!(self, OS::Linux)
    }
}

/// Describes categories of known platforms, for differentiating certain engine behaviors.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
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

impl Platform {
    /// Returns true when hosted by the web chassis.
    pub fn is_web(&self) -> bool {
        matches!(self, Platform::Web)
    }

    /// Returns true when hosted by a native chassis.
    pub fn is_native(&self) -> bool {
        matches!(self, Platform::Native)
    }
}

/// Derived target facts exposed to PAXEL and Rust event handlers.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetInfo {
    /// Browser-hosted rendering target.
    pub web: bool,
    /// Native application rendering target.
    pub native: bool,
    /// Any iOS-family target, including iPhone and iPad.
    pub ios: bool,
    /// iPhone-class iOS target.
    pub iphone: bool,
    /// iPadOS / iPad-class iOS target.
    pub ipad: bool,
    /// macOS target.
    pub macos: bool,
    /// Android target.
    pub android: bool,
    /// Windows target.
    pub windows: bool,
    /// Linux target.
    pub linux: bool,
    /// Any mobile OS target.
    pub mobile: bool,
    /// Any desktop OS target.
    pub desktop: bool,
}

impl TargetInfo {
    /// Build target facts from the chassis platform and detected OS.
    pub fn new(platform: Platform, os: OS) -> Self {
        Self {
            web: platform.is_web(),
            native: platform.is_native(),
            ios: os.is_ios(),
            iphone: os.is_iphone(),
            ipad: os.is_ipad(),
            macos: os.is_macos(),
            android: os.is_android(),
            windows: os.is_windows(),
            linux: os.is_linux(),
            mobile: os.is_mobile(),
            desktop: os.is_desktop(),
        }
    }
}

impl ToPaxValue for TargetInfo {
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Object(
            vec![
                ("web".to_string(), self.web.to_pax_value()),
                ("native".to_string(), self.native.to_pax_value()),
                ("ios".to_string(), self.ios.to_pax_value()),
                ("iphone".to_string(), self.iphone.to_pax_value()),
                ("ipad".to_string(), self.ipad.to_pax_value()),
                ("macos".to_string(), self.macos.to_pax_value()),
                ("android".to_string(), self.android.to_pax_value()),
                ("windows".to_string(), self.windows.to_pax_value()),
                ("linux".to_string(), self.linux.to_pax_value()),
                ("mobile".to_string(), self.mobile.to_pax_value()),
                ("desktop".to_string(), self.desktop.to_pax_value()),
            ]
            .into_iter()
            .collect(),
        )
    }
}

impl Interpolatable for TargetInfo {}

/// Struct representing the outermost viewport of a rendering scene, for example a browser window
/// or native application window.
#[derive(Default, Debug, Clone, Copy)]
pub struct Viewport {
    /// Viewport width in pixels.
    pub width: f64,
    /// Viewport height in pixels.
    pub height: f64,
    /// Larger viewport dimension in pixels.
    pub major: f64,
    /// Smaller viewport dimension in pixels.
    pub minor: f64,
    /// Width divided by height. Returns 0.0 when height is 0.
    pub aspect: f64,
    /// True when width is greater than height.
    pub landscape: bool,
    /// True when height is greater than width.
    pub portrait: bool,
    /// True when width and height are effectively equal.
    pub square: bool,
}

impl Viewport {
    /// Equality tolerance used when classifying square viewports.
    pub const SQUARE_EPSILON: f64 = 0.5;

    /// Build viewport facts from width and height in logical pixels.
    pub fn new(width: f64, height: f64) -> Self {
        let square = (width - height).abs() <= Self::SQUARE_EPSILON;
        Self {
            width,
            height,
            major: width.max(height),
            minor: width.min(height),
            aspect: if height.abs() <= f64::EPSILON {
                0.0
            } else {
                width / height
            },
            landscape: width > height + Self::SQUARE_EPSILON,
            portrait: height > width + Self::SQUARE_EPSILON,
            square,
        }
    }
}

impl ToPaxValue for Viewport {
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Object(
            vec![
                ("width".to_string(), self.width.to_pax_value()),
                ("height".to_string(), self.height.to_pax_value()),
                ("major".to_string(), self.major.to_pax_value()),
                ("minor".to_string(), self.minor.to_pax_value()),
                ("aspect".to_string(), self.aspect.to_pax_value()),
                ("landscape".to_string(), self.landscape.to_pax_value()),
                ("portrait".to_string(), self.portrait.to_pax_value()),
                ("square".to_string(), self.square.to_pax_value()),
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
            major: self.major + (other.major - self.major) * t,
            minor: self.minor + (other.minor - self.minor) * t,
            aspect: self.aspect + (other.aspect - self.aspect) * t,
            landscape: self.landscape,
            portrait: self.portrait,
            square: self.square,
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
