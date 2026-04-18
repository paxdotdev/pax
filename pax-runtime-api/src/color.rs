//! Color and opacity types used by Pax style properties and render backends.

use super::*;

/// Raw Percent type, which we use for serialization and dynamic traversal.  At the time
/// of authoring, this type is not used directly at runtime, but is intended for `into` coercion
/// into downstream types, e.g. ColorChannel, Rotation, and Size.  This allows us to be "dumb"
/// about how we parse `%`, and allow the context in which it is used to pull forward a specific
/// type through `into` inference.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Percent(pub Numeric);

impl Display for Percent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}%", self.0)
    }
}

impl Interpolatable for Percent {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        Self(self.0.interpolate(&other.0, t))
    }
}

/// Describes an opacity value either as normalized alpha or as a percent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Opacity {
    /// Unitless alpha in the normalized [0.0, 1.0] range.
    Alpha(Numeric),
    /// Percent alpha in the [0.0, 100.0] range.
    Percent(Numeric),
}

impl Display for Opacity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Opacity::Alpha(value) => write!(f, "{}", value),
            Opacity::Percent(value) => write!(f, "{}%", value),
        }
    }
}

impl Default for Opacity {
    fn default() -> Self {
        Self::Alpha(Numeric::F64(1.0))
    }
}

impl Interpolatable for Opacity {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        Opacity::Alpha(Numeric::F64(
            self.to_float_0_1().interpolate(&other.to_float_0_1(), t),
        ))
    }
}

impl From<f64> for Opacity {
    fn from(value: f64) -> Self {
        Numeric::F64(value).into()
    }
}

impl From<i32> for Opacity {
    fn from(value: i32) -> Self {
        Numeric::from(value).into()
    }
}

impl From<Numeric> for Opacity {
    fn from(value: Numeric) -> Self {
        Opacity::Alpha(value)
    }
}

impl From<Percent> for Opacity {
    fn from(value: Percent) -> Self {
        Opacity::Percent(value.0)
    }
}

impl Opacity {
    /// Normalizes this Opacity as a float in [0.0, 1.0].
    pub fn to_float_0_1(&self) -> f64 {
        match self {
            Opacity::Alpha(value) => value.to_float().clamp(0.0, 1.0),
            Opacity::Percent(value) => (value.to_float() / 100.0).clamp(0.0, 1.0),
        }
    }
}

impl From<f64> for ColorChannel {
    fn from(value: f64) -> Self {
        Numeric::F64(value).into()
    }
}

impl From<i32> for ColorChannel {
    fn from(value: i32) -> Self {
        Numeric::from(value).into()
    }
}

impl Into<ColorChannel> for Percent {
    fn into(self) -> ColorChannel {
        ColorChannel::Percent(self.0)
    }
}

impl Into<Size> for Percent {
    fn into(self) -> Size {
        Size::Percent(self.0)
    }
}

impl Into<Rotation> for Percent {
    fn into(self) -> Rotation {
        Rotation::Percent(self.0)
    }
}

/// Describes a color channel in a unit appropriate to the surrounding color model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ColorChannel {
    /// Used, for example, to express hue in HSL or other rotational color models.
    Rotation(Rotation),
    /// Integer color channel in the `[0, 255]` range.
    Integer(u8),
    /// Percent color channel in the `[0.0, 100.0]` range.
    Percent(Numeric),
}

impl Display for ColorChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColorChannel::Rotation(rot) => write!(f, "{}", rot),
            ColorChannel::Integer(int) => write!(f, "{}", int),
            ColorChannel::Percent(per) => write!(f, "{}%", per),
        }
    }
}

impl Default for ColorChannel {
    fn default() -> Self {
        Self::Percent(Numeric::F64(50.0))
    }
}

impl From<Numeric> for Rotation {
    fn from(value: Numeric) -> Self {
        Rotation::Degrees(value)
    }
}

impl From<Numeric> for ColorChannel {
    fn from(value: Numeric) -> Self {
        Self::Integer(value.to_int().clamp(0, 255) as u8)
    }
}

impl ColorChannel {
    /// Normalizes this color channel as a float in the `[0.0, 1.0]` range.
    pub fn to_float_0_1(&self) -> f64 {
        match self {
            Self::Percent(per) => (per.to_float() / 100.0).clamp(0_f64, 1_f64),
            Self::Integer(zero_to_255) => {
                let f_zero = (*zero_to_255) as f64;
                (f_zero / 255.0_f64).clamp(0_f64, 1_f64)
            }
            Self::Rotation(rot) => rot.to_float_0_1(),
        }
    }
}


/// Entrypoint for specifying and representing colors in Pax.
#[allow(non_camel_case_types)]
#[derive(Default, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum Color {
    /// Models a color in the RGB space, with an alpha channel of 100%
    rgb(ColorChannel, ColorChannel, ColorChannel),
    /// Models a color in the RGBA space
    rgba(ColorChannel, ColorChannel, ColorChannel, ColorChannel),
    /// Models a color in the HSL space, with an alpha channel of 100%
    hsl(Rotation, ColorChannel, ColorChannel),
    /// Models a color in the HSLA space.
    hsla(Rotation, ColorChannel, ColorChannel, ColorChannel),

    /// Cool blue-gray, RGB (100, 116, 139).
    #[default]
    SLATE,
    /// Balanced gray, RGB (107, 114, 128).
    GRAY,
    /// Crisp cool gray, RGB (113, 113, 122).
    ZINC,
    /// Plain neutral gray, RGB (115, 115, 115).
    NEUTRAL,
    /// Warm stone gray, RGB (120, 113, 108).
    STONE,
    /// Bright warm red, RGB (239, 68, 68).
    RED,
    /// Vivid citrus orange, RGB (249, 115, 22).
    ORANGE,
    /// Golden amber, RGB (245, 158, 11).
    AMBER,
    /// Sunny yellow, RGB (234, 179, 8).
    YELLOW,
    /// Electric lime, RGB (132, 204, 22).
    LIME,
    /// Fresh green, RGB (34, 197, 94).
    GREEN,
    /// Jewel emerald, RGB (16, 185, 129).
    EMERALD,
    /// Deep aquatic teal, RGB (20, 184, 166).
    TEAL,
    /// Bright clean cyan, RGB (6, 182, 212).
    CYAN,
    /// Open sky blue, RGB (14, 165, 233).
    SKY,
    /// Saturated primary blue, RGB (59, 130, 246).
    BLUE,
    /// Cool electric indigo, RGB (99, 102, 241).
    INDIGO,
    /// Soft vivid violet, RGB (139, 92, 246).
    VIOLET,
    /// Rich playful purple, RGB (168, 85, 247).
    PURPLE,
    /// Brilliant magenta fuchsia, RGB (217, 70, 239).
    FUCHSIA,
    /// Bright candy pink, RGB (236, 72, 153).
    PINK,
    /// Warm rosy red, RGB (244, 63, 94).
    ROSE,
    /// Pure black, RGB (0, 0, 0).
    BLACK,
    /// Pure white, RGB (255, 255, 255).
    WHITE,
    /// Fully transparent white, RGB (255, 255, 255).
    TRANSPARENT,
    /// Non-rendering transparent, RGB (255, 255, 255).
    NONE,
}

impl Hash for Color {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
    }
}

// implement display respecting all color enum variants, printing out their names
impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::rgb(r, g, b) => write!(f, "rgb({}, {}, {})", r, g, b),
            Self::rgba(r, g, b, a) => write!(f, "rgba({}, {}, {}, {})", r, g, b, a),
            Self::hsl(h, s, l) => write!(f, "hsl({}, {}, {})", h, s, l),
            Self::hsla(h, s, l, a) => write!(f, "hsla({}, {}, {}, {})", h, s, l, a),
            Self::SLATE => write!(f, "SLATE"),
            Self::GRAY => write!(f, "GRAY"),
            Self::ZINC => write!(f, "ZINC"),
            Self::NEUTRAL => write!(f, "NEUTRAL"),
            Self::STONE => write!(f, "STONE"),
            Self::RED => write!(f, "RED"),
            Self::ORANGE => write!(f, "ORANGE"),
            Self::AMBER => write!(f, "AMBER"),
            Self::YELLOW => write!(f, "YELLOW"),
            Self::LIME => write!(f, "LIME"),
            Self::GREEN => write!(f, "GREEN"),
            Self::EMERALD => write!(f, "EMERALD"),
            Self::TEAL => write!(f, "TEAL"),
            Self::CYAN => write!(f, "CYAN"),
            Self::SKY => write!(f, "SKY"),
            Self::BLUE => write!(f, "BLUE"),
            Self::INDIGO => write!(f, "INDIGO"),
            Self::VIOLET => write!(f, "VIOLET"),
            Self::PURPLE => write!(f, "PURPLE"),
            Self::FUCHSIA => write!(f, "FUCHSIA"),
            Self::PINK => write!(f, "PINK"),
            Self::ROSE => write!(f, "ROSE"),
            Self::BLACK => write!(f, "BLACK"),
            Self::WHITE => write!(f, "WHITE"),
            Self::TRANSPARENT => write!(f, "TRANSPARENT"),
            Self::NONE => write!(f, "NONE"),
        }
    }
}

impl Color {
    //TODO: build out tint api and consider other color transforms
    //pub fn tint(tint_offset_amount) -> Self {...}

    /// Constructs an RGB color with 100% alpha.
    pub fn rgb(r: ColorChannel, g: ColorChannel, b: ColorChannel) -> Self {
        Self::rgb(r, g, b)
    }

    /// Constructs an RGBA color.
    pub fn rgba(r: ColorChannel, g: ColorChannel, b: ColorChannel, a: ColorChannel) -> Self {
        Self::rgba(r, g, b, a)
    }

    /// Constructs an HSL color with 100% alpha.
    pub fn hsl(h: Rotation, s: ColorChannel, l: ColorChannel) -> Self {
        Self::hsl(h, s, l)
    }

    /// Constructs an HSLA color.
    pub fn hsla(h: Rotation, s: ColorChannel, l: ColorChannel, a: ColorChannel) -> Self {
        Self::hsla(h, s, l, a)
    }

    // Converts to the Piet color type used by canvas renderers.
    pub fn to_piet_color(&self) -> piet::Color {
        let rgba = self.to_rgba_0_1();
        piet::Color::rgba(rgba[0], rgba[1], rgba[2], rgba[3])
    }

    /// Constructs a color from normalized RGBA channels in the `[0.0, 1.0]` range.
    pub fn from_rgba_0_1(rgba_0_1: [f64; 4]) -> Self {
        Self::rgba(
            ColorChannel::Percent(Numeric::F64(rgba_0_1[0] * 100.0)),
            ColorChannel::Percent(Numeric::F64(rgba_0_1[1] * 100.0)),
            ColorChannel::Percent(Numeric::F64(rgba_0_1[2] * 100.0)),
            ColorChannel::Percent(Numeric::F64(rgba_0_1[3] * 100.0)),
        )
    }

    /// Constructs a color from a six- or eight-character RGB/RGBA hex string.
    pub fn from_hex(hex: &str) -> Self {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap() as f64 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap() as f64 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap() as f64 / 255.0;
        let a = if hex.len() == 8 {
            u8::from_str_radix(&hex[6..8], 16).unwrap() as f64 / 255.0
        } else {
            1.0
        };
        Self::rgba(
            ColorChannel::Percent(Numeric::F64(r * 100.0)),
            ColorChannel::Percent(Numeric::F64(g * 100.0)),
            ColorChannel::Percent(Numeric::F64(b * 100.0)),
            ColorChannel::Percent(Numeric::F64(a * 100.0)),
        )
    }

    /// Returns this color's alpha channel normalized to the `[0.0, 1.0]` range.
    pub fn alpha_0_1(&self) -> f64 {
        self.to_rgba_0_1()[3]
    }

    /// Multiplies this color's alpha channel by `factor`.
    pub fn with_alpha_factor(&self, factor: f64) -> Self {
        let mut rgba = self.to_rgba_0_1();
        rgba[3] = (rgba[3] * factor.clamp(0.0, 1.0)).clamp(0.0, 1.0);
        Self::from_rgba_0_1(rgba)
    }

    /// Returns RGBA channels normalized to the `[0.0, 1.0]` range.
    pub fn to_rgba_0_1(&self) -> [f64; 4] {
        match self {
            Self::hsla(h, s, l, a) => {
                let rgb = hsl_to_rgb(h.to_float_0_1(), s.to_float_0_1(), l.to_float_0_1());
                [rgb[0], rgb[1], rgb[2], a.to_float_0_1()]
            }
            Self::hsl(h, s, l) => {
                let rgb = hsl_to_rgb(h.to_float_0_1(), s.to_float_0_1(), l.to_float_0_1());
                [rgb[0], rgb[1], rgb[2], 1.0]
            }
            Self::rgba(r, g, b, a) => [
                r.to_float_0_1(),
                g.to_float_0_1(),
                b.to_float_0_1(),
                a.to_float_0_1(),
            ],
            Self::rgb(r, g, b) => [r.to_float_0_1(), g.to_float_0_1(), b.to_float_0_1(), 1.0],

            //Color constants from TailwindCSS
            Self::SLATE => Self::rgb(
                Numeric::from(0x64).into(),
                Numeric::from(0x74).into(),
                Numeric::from(0x8b).into(),
            )
            .to_rgba_0_1(),
            Self::GRAY => Self::rgb(
                Numeric::from(0x6b).into(),
                Numeric::from(0x72).into(),
                Numeric::from(0x80).into(),
            )
            .to_rgba_0_1(),
            Self::ZINC => Self::rgb(
                Numeric::from(0x71).into(),
                Numeric::from(0x71).into(),
                Numeric::from(0x7a).into(),
            )
            .to_rgba_0_1(),
            Self::NEUTRAL => Self::rgb(
                Numeric::from(0x73).into(),
                Numeric::from(0x73).into(),
                Numeric::from(0x73).into(),
            )
            .to_rgba_0_1(),
            Self::STONE => Self::rgb(
                Numeric::from(0x78).into(),
                Numeric::from(0x71).into(),
                Numeric::from(0x6c).into(),
            )
            .to_rgba_0_1(),
            Self::RED => Self::rgb(
                Numeric::from(0xeF).into(),
                Numeric::from(0x44).into(),
                Numeric::from(0x44).into(),
            )
            .to_rgba_0_1(),
            Self::ORANGE => Self::rgb(
                Numeric::from(0xf9).into(),
                Numeric::from(0x73).into(),
                Numeric::from(0x16).into(),
            )
            .to_rgba_0_1(),
            Self::AMBER => Self::rgb(
                Numeric::from(0xf5).into(),
                Numeric::from(0x9e).into(),
                Numeric::from(0x0b).into(),
            )
            .to_rgba_0_1(),
            Self::YELLOW => Self::rgb(
                Numeric::from(0xea).into(),
                Numeric::from(0xb3).into(),
                Numeric::from(0x08).into(),
            )
            .to_rgba_0_1(),
            Self::LIME => Self::rgb(
                Numeric::from(0x84).into(),
                Numeric::from(0xcc).into(),
                Numeric::from(0x16).into(),
            )
            .to_rgba_0_1(),
            Self::GREEN => Self::rgb(
                Numeric::from(0x22).into(),
                Numeric::from(0xc5).into(),
                Numeric::from(0x5e).into(),
            )
            .to_rgba_0_1(),
            Self::EMERALD => Self::rgb(
                Numeric::from(0x10).into(),
                Numeric::from(0xb9).into(),
                Numeric::from(0x81).into(),
            )
            .to_rgba_0_1(),
            Self::TEAL => Self::rgb(
                Numeric::from(0x14).into(),
                Numeric::from(0xb8).into(),
                Numeric::from(0xa6).into(),
            )
            .to_rgba_0_1(),
            Self::CYAN => Self::rgb(
                Numeric::from(0x06).into(),
                Numeric::from(0xb6).into(),
                Numeric::from(0xd4).into(),
            )
            .to_rgba_0_1(),
            Self::SKY => Self::rgb(
                Numeric::from(0x0e).into(),
                Numeric::from(0xa5).into(),
                Numeric::from(0xe9).into(),
            )
            .to_rgba_0_1(),
            Self::BLUE => Self::rgb(
                Numeric::from(0x3b).into(),
                Numeric::from(0x82).into(),
                Numeric::from(0xf6).into(),
            )
            .to_rgba_0_1(),
            Self::INDIGO => Self::rgb(
                Numeric::from(0x63).into(),
                Numeric::from(0x66).into(),
                Numeric::from(0xf1).into(),
            )
            .to_rgba_0_1(),
            Self::VIOLET => Self::rgb(
                Numeric::from(0x8b).into(),
                Numeric::from(0x5c).into(),
                Numeric::from(0xf6).into(),
            )
            .to_rgba_0_1(),
            Self::PURPLE => Self::rgb(
                Numeric::from(0xa8).into(),
                Numeric::from(0x55).into(),
                Numeric::from(0xf7).into(),
            )
            .to_rgba_0_1(),
            Self::FUCHSIA => Self::rgb(
                Numeric::from(0xd9).into(),
                Numeric::from(0x46).into(),
                Numeric::from(0xef).into(),
            )
            .to_rgba_0_1(),
            Self::PINK => Self::rgb(
                Numeric::from(0xec).into(),
                Numeric::from(0x48).into(),
                Numeric::from(0x99).into(),
            )
            .to_rgba_0_1(),
            Self::ROSE => Self::rgb(
                Numeric::from(0xf4).into(),
                Numeric::from(0x3f).into(),
                Numeric::from(0x5e).into(),
            )
            .to_rgba_0_1(),
            Self::BLACK => Self::rgb(
                Numeric::from(0x00).into(),
                Numeric::from(0x00).into(),
                Numeric::from(0x00).into(),
            )
            .to_rgba_0_1(),
            Self::WHITE => Self::rgb(
                Numeric::from(0xff).into(),
                Numeric::from(0xff).into(),
                Numeric::from(0xff).into(),
            )
            .to_rgba_0_1(),
            Self::TRANSPARENT | Self::NONE => Self::rgba(
                Numeric::from(0xff).into(),
                Numeric::from(0xff).into(),
                Numeric::from(0xFF).into(),
                Numeric::from(0x00).into(),
            )
            .to_rgba_0_1(),
        }
    }

    /// Returns HSLA channels normalized to the `[0.0, 1.0]` range.
    // Credit: Claude 3.5 Sonnet.
    pub fn to_hsla_0_1(&self) -> [f64; 4] {
        let [r, g, b, a] = self.to_rgba_0_1();

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let chroma = max - min;

        let h = if chroma == 0.0 {
            0.0
        } else if max == r {
            ((g - b) / chroma + 6.0) % 6.0 / 6.0
        } else if max == g {
            ((b - r) / chroma + 2.0) / 6.0
        } else {
            ((r - g) / chroma + 4.0) / 6.0
        };

        let l = (max + min) / 2.0;

        let s = if l == 0.0 || l == 1.0 {
            0.0
        } else {
            (max - l) / l.min(1.0 - l)
        };

        [h, s, l, a]
    }
}

//hsl_to_rgb logic borrowed & modified from https://github.com/emgyrz/colorsys.rs, licensed MIT Copyright (c) 2019 mz <emgyrz@gmail.com>
const RGB_UNIT_MAX: f64 = 255.0;
fn hsl_to_rgb(h: f64, s: f64, l: f64) -> [f64; 3] {
    if s == 0.0 {
        let unit = RGB_UNIT_MAX * l;
        return [
            unit / RGB_UNIT_MAX,
            unit / RGB_UNIT_MAX,
            unit / RGB_UNIT_MAX,
        ];
    }

    let temp1 = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };

    let temp2 = 2.0 * l - temp1;
    let hue = h;

    let temp_r = bound(hue + (1.0 / 3.0), 1.0);
    let temp_g = bound(hue, 1.0);
    let temp_b = bound(hue - (1.0 / 3.0), 1.0);

    let r = calc_rgb_unit(temp_r, temp1, temp2);
    let g = calc_rgb_unit(temp_g, temp1, temp2);
    let b = calc_rgb_unit(temp_b, temp1, temp2);
    [r / RGB_UNIT_MAX, g / RGB_UNIT_MAX, b / RGB_UNIT_MAX]
}

fn calc_rgb_unit(unit: f64, temp1: f64, temp2: f64) -> f64 {
    let mut result = temp2;
    if 6.0 * unit < 1.0 {
        result = temp2 + (temp1 - temp2) * 6.0 * unit
    } else if 2.0 * unit < 1.0 {
        result = temp1
    } else if 3.0 * unit < 2.0 {
        result = temp2 + (temp1 - temp2) * ((2.0 / 3.0) - unit) * 6.0
    }
    result * RGB_UNIT_MAX
}

// Wraps a scalar into the `[0, entire]` interval, used by HSL conversion.
pub fn bound(r: f64, entire: f64) -> f64 {
    let mut n = r;
    loop {
        let less = n < 0.0;
        let bigger = n > entire;
        if !less && !bigger {
            break n;
        }
        if less {
            n += entire;
        } else {
            n -= entire;
        }
    }
}

impl Into<ColorMessage> for &Color {
    fn into(self) -> ColorMessage {
        let rgba = self.to_rgba_0_1();
        ColorMessage::Rgba(rgba)
    }
}
impl PartialEq<ColorMessage> for Color {
    fn eq(&self, other: &ColorMessage) -> bool {
        let self_rgba = self.to_rgba_0_1();

        match other {
            ColorMessage::Rgb(other_rgba) => {
                self_rgba[0] == other_rgba[0]
                    && self_rgba[1] == other_rgba[1]
                    && self_rgba[2] == other_rgba[2]
                    && self_rgba[3] == 1.0
            }
            ColorMessage::Rgba(other_rgba) => {
                self_rgba[0] == other_rgba[0]
                    && self_rgba[1] == other_rgba[1]
                    && self_rgba[2] == other_rgba[2]
                    && self_rgba[3] == other_rgba[3]
            }
        }
    }
}
impl Interpolatable for Color {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        let rgba_s = self.to_rgba_0_1();
        let rgba_o = other.to_rgba_0_1();
        let rgba_i = [
            rgba_s[0].interpolate(&rgba_o[0], t),
            rgba_s[1].interpolate(&rgba_o[1], t),
            rgba_s[2].interpolate(&rgba_o[2], t),
            rgba_s[3].interpolate(&rgba_o[3], t),
        ];
        Color::from_rgba_0_1(rgba_i)
    }
}
