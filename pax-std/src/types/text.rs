use crate::types::Color;
use api::StringBox;
use pax_lang::api::{Numeric, Property, PropertyLiteral, SizePixels};
use pax_lang::*;
use pax_message::{
    ColorVariantMessage, FontPatch, FontStyleMessage, FontWeightMessage, LocalFontMessage,
    SystemFontMessage, TextAlignHorizontalMessage, TextAlignVerticalMessage, TextStyleMessage,
    WebFontMessage,
};

#[derive(Pax)]
#[custom(Default)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct TextStyle {
    pub font: Property<Font>,
    pub font_size: Property<SizePixels>,
    pub fill: Property<Color>,
    pub underline: Property<bool>,
    pub align_multiline: Property<TextAlignHorizontal>,
    pub align_vertical: Property<TextAlignVertical>,
    pub align_horizontal: Property<TextAlignHorizontal>,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font: Box::new(PropertyLiteral::new(Font::default())),
            font_size: Box::new(PropertyLiteral::new(SizePixels(Numeric::Float(20.0)))),
            fill: Box::new(PropertyLiteral::new(Color::rgba(
                Numeric::Float(0.0),
                Numeric::Float(0.0),
                Numeric::Float(0.0),
                Numeric::Float(1.0),
            ))),
            underline: Box::new(PropertyLiteral::new(false)),
            align_multiline: Box::new(PropertyLiteral::new(TextAlignHorizontal::Left)),
            align_vertical: Box::new(PropertyLiteral::new(TextAlignVertical::Top)),
            align_horizontal: Box::new(PropertyLiteral::new(TextAlignHorizontal::Left)),
        }
    }
}

impl<'a> Into<TextStyleMessage> for &'a TextStyle {
    fn into(self) -> TextStyleMessage {
        TextStyleMessage {
            font: Some(self.font.get().clone().into()),
            font_size: Some(f64::from(&self.font_size.get().clone())),
            fill: Some(Into::<ColorVariantMessage>::into(self.fill.get())),
            underline: Some(self.underline.get().clone()),
            align_multiline: Some(Into::<TextAlignHorizontalMessage>::into(
                self.align_multiline.get(),
            )),
            align_vertical: Some(Into::<TextAlignVerticalMessage>::into(
                self.align_vertical.get(),
            )),
            align_horizontal: Some(Into::<TextAlignHorizontalMessage>::into(
                self.align_horizontal.get(),
            )),
        }
    }
}

impl PartialEq<TextStyleMessage> for TextStyle {
    fn eq(&self, other: &TextStyleMessage) -> bool {
        let font_equal = other
            .font
            .as_ref()
            .map_or(false, |font| self.font.get().eq(font));

        let font_size_equal = other.font_size.map_or(false, |size| {
            Numeric::Float(size) == self.font_size.get().clone()
        });

        let fill_equal = other
            .fill
            .as_ref()
            .map_or(false, |fill| self.fill.get().eq(fill));

        let underline_equal = other
            .underline
            .map_or(false, |underline| underline == self.underline.get().clone());

        let align_multiline_equal = other
            .align_multiline
            .as_ref()
            .map_or(false, |align_multiline| {
                self.align_multiline.get().eq(align_multiline)
            });

        let align_vertical_equal = other
            .align_vertical
            .as_ref()
            .map_or(false, |align_vertical| {
                self.align_vertical.get().eq(align_vertical)
            });

        let align_horizontal_equal = other
            .align_horizontal
            .as_ref()
            .map_or(false, |align_horizontal| {
                self.align_horizontal.get().eq(align_horizontal)
            });

        font_equal
            && font_size_equal
            && fill_equal
            && underline_equal
            && align_multiline_equal
            && align_vertical_equal
            && align_horizontal_equal
    }
}

#[derive(Pax)]
#[custom(Default, Imports)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub enum Font {
    System(SystemFont),
    Web(WebFont),
    Local(LocalFont),
}

impl Default for Font {
    fn default() -> Self {
        Self::System(SystemFont::default())
    }
}

#[derive(Pax)]
#[custom(Imports, Default)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct SystemFont {
    pub family: StringBox,
    pub style: FontStyle,
    pub weight: FontWeight,
}

impl Default for SystemFont {
    fn default() -> Self {
        Self {
            family: "Arial".to_string().into(),
            style: FontStyle::Normal,
            weight: FontWeight::Normal,
        }
    }
}

#[derive(Pax)]
#[custom(Imports)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct WebFont {
    pub family: StringBox,
    pub url: StringBox,
    pub style: FontStyle,
    pub weight: FontWeight,
}

#[derive(Pax)]
#[custom(Imports)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct LocalFont {
    pub family: StringBox,
    pub path: StringBox,
    pub style: FontStyle,
    pub weight: FontWeight,
}

#[derive(Pax)]
#[custom(Imports)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
    Oblique,
}

#[derive(Pax)]
#[custom(Imports)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub enum FontWeight {
    Thin,
    ExtraLight,
    Light,
    #[default]
    Normal,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
}

#[derive(Pax)]
#[custom(Imports)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub enum TextAlignHorizontal {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Pax)]
#[custom(Imports)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub enum TextAlignVertical {
    #[default]
    Top,
    Center,
    Bottom,
}

impl Into<TextAlignHorizontalMessage> for &TextAlignHorizontal {
    fn into(self) -> TextAlignHorizontalMessage {
        match self {
            TextAlignHorizontal::Center => TextAlignHorizontalMessage::Center,
            TextAlignHorizontal::Left => TextAlignHorizontalMessage::Left,
            TextAlignHorizontal::Right => TextAlignHorizontalMessage::Right,
        }
    }
}

impl PartialEq<TextAlignHorizontalMessage> for TextAlignHorizontal {
    fn eq(&self, other: &TextAlignHorizontalMessage) -> bool {
        match (self, other) {
            (TextAlignHorizontal::Center, TextAlignHorizontalMessage::Center) => true,
            (TextAlignHorizontal::Left, TextAlignHorizontalMessage::Left) => true,
            (TextAlignHorizontal::Right, TextAlignHorizontalMessage::Right) => true,
            _ => false,
        }
    }
}

pub fn opt_align_to_message(
    opt_alignment: &Option<TextAlignHorizontal>,
) -> Option<TextAlignHorizontalMessage> {
    opt_alignment.as_ref().map(|alignment| match alignment {
        TextAlignHorizontal::Center => TextAlignHorizontalMessage::Center,
        TextAlignHorizontal::Left => TextAlignHorizontalMessage::Left,
        TextAlignHorizontal::Right => TextAlignHorizontalMessage::Right,
    })
}

pub fn opt_value_eq_opt_msg<T, U>(opt_value: &Option<T>, opt_value_msg: &Option<U>) -> bool
where
    T: PartialEq<U>,
{
    match (opt_value, opt_value_msg) {
        (Some(value), Some(value_msg)) => value.eq(value_msg),
        (None, None) => true,
        _ => false,
    }
}

impl Into<TextAlignVerticalMessage> for &TextAlignVertical {
    fn into(self) -> TextAlignVerticalMessage {
        match self {
            TextAlignVertical::Top => TextAlignVerticalMessage::Top,
            TextAlignVertical::Center => TextAlignVerticalMessage::Center,
            TextAlignVertical::Bottom => TextAlignVerticalMessage::Bottom,
        }
    }
}

impl PartialEq<TextAlignVerticalMessage> for TextAlignVertical {
    fn eq(&self, other: &TextAlignVerticalMessage) -> bool {
        match (self, other) {
            (TextAlignVertical::Top, TextAlignVerticalMessage::Top) => true,
            (TextAlignVertical::Center, TextAlignVerticalMessage::Center) => true,
            (TextAlignVertical::Bottom, TextAlignVerticalMessage::Bottom) => true,
            _ => false,
        }
    }
}

impl From<Font> for FontPatch {
    fn from(font: Font) -> Self {
        match font {
            Font::System(system_font) => FontPatch::System(SystemFontMessage {
                family: Some(system_font.family.string),
                style: Some(system_font.style.into()),
                weight: Some(system_font.weight.into()),
            }),
            Font::Web(web_font) => FontPatch::Web(WebFontMessage {
                family: Some(web_font.family.string.into()),
                url: Some(web_font.url.string.into()),
                style: Some(web_font.style.into()),
                weight: Some(web_font.weight.into()),
            }),
            Font::Local(local_font) => FontPatch::Local(LocalFontMessage {
                family: Some(local_font.family.string),
                path: Some(local_font.path.string.into()),
                style: Some(local_font.style.into()),
                weight: Some(local_font.weight.into()),
            }),
        }
    }
}

impl PartialEq<FontStyleMessage> for FontStyle {
    fn eq(&self, other: &FontStyleMessage) -> bool {
        match (self, other) {
            (FontStyle::Normal, FontStyleMessage::Normal) => true,
            (FontStyle::Italic, FontStyleMessage::Italic) => true,
            (FontStyle::Oblique, FontStyleMessage::Oblique) => true,
            _ => false,
        }
    }
}

impl PartialEq<FontWeightMessage> for FontWeight {
    fn eq(&self, other: &FontWeightMessage) -> bool {
        match (self, other) {
            (FontWeight::Thin, FontWeightMessage::Thin) => true,
            (FontWeight::ExtraLight, FontWeightMessage::ExtraLight) => true,
            (FontWeight::Light, FontWeightMessage::Light) => true,
            (FontWeight::Normal, FontWeightMessage::Normal) => true,
            (FontWeight::Medium, FontWeightMessage::Medium) => true,
            (FontWeight::SemiBold, FontWeightMessage::SemiBold) => true,
            (FontWeight::Bold, FontWeightMessage::Bold) => true,
            (FontWeight::ExtraBold, FontWeightMessage::ExtraBold) => true,
            (FontWeight::Black, FontWeightMessage::Black) => true,
            _ => false,
        }
    }
}

impl PartialEq<FontPatch> for Font {
    fn eq(&self, other: &FontPatch) -> bool {
        match (self, other) {
            (Font::System(system_font), FontPatch::System(system_font_patch)) => {
                system_font_patch
                    .family
                    .as_ref()
                    .map_or(false, |family| *family == system_font.family.string)
                    && system_font_patch
                        .style
                        .as_ref()
                        .map_or(false, |style| system_font.style.eq(style))
                    && system_font_patch
                        .weight
                        .as_ref()
                        .map_or(false, |weight| system_font.weight.eq(weight))
            }
            (Font::Web(web_font), FontPatch::Web(web_font_patch)) => {
                web_font_patch
                    .family
                    .as_ref()
                    .map_or(false, |family| *family == web_font.family.string)
                    && web_font_patch
                        .url
                        .as_ref()
                        .map_or(false, |url| *url == web_font.url.string)
                    && web_font_patch
                        .style
                        .as_ref()
                        .map_or(false, |style| web_font.style.eq(style))
                    && web_font_patch
                        .weight
                        .as_ref()
                        .map_or(false, |weight| web_font.weight.eq(weight))
            }
            (Font::Local(local_font), FontPatch::Local(local_font_patch)) => {
                local_font_patch
                    .family
                    .as_ref()
                    .map_or(false, |family| *family == local_font.family.string)
                    && local_font_patch
                        .path
                        .as_ref()
                        .map_or(false, |path| *path == local_font.path.string)
                    && local_font_patch
                        .style
                        .as_ref()
                        .map_or(false, |style| local_font.style.eq(style))
                    && local_font_patch
                        .weight
                        .as_ref()
                        .map_or(false, |weight| local_font.weight.eq(weight))
            }
            _ => false,
        }
    }
}

impl Font {
    pub fn system(family: StringBox, style: FontStyle, weight: FontWeight) -> Self {
        Self::System(SystemFont {
            family,
            style,
            weight,
        })
    }

    pub fn web(family: StringBox, url: StringBox, style: FontStyle, weight: FontWeight) -> Self {
        Self::Web(WebFont {
            family,
            url,
            style,
            weight,
        })
    }

    pub fn local(family: StringBox, path: StringBox, style: FontStyle, weight: FontWeight) -> Self {
        Self::Local(LocalFont {
            family,
            path,
            style,
            weight,
        })
    }
}

impl From<FontStyleMessage> for FontStyle {
    fn from(style_msg: FontStyleMessage) -> Self {
        match style_msg {
            FontStyleMessage::Normal => FontStyle::Normal,
            FontStyleMessage::Italic => FontStyle::Italic,
            FontStyleMessage::Oblique => FontStyle::Oblique,
        }
    }
}

impl From<FontStyle> for FontStyleMessage {
    fn from(style: FontStyle) -> Self {
        match style {
            FontStyle::Normal => FontStyleMessage::Normal,
            FontStyle::Italic => FontStyleMessage::Italic,
            FontStyle::Oblique => FontStyleMessage::Oblique,
        }
    }
}

impl From<FontWeightMessage> for FontWeight {
    fn from(weight_msg: FontWeightMessage) -> Self {
        match weight_msg {
            FontWeightMessage::Thin => FontWeight::Thin,
            FontWeightMessage::ExtraLight => FontWeight::ExtraLight,
            FontWeightMessage::Light => FontWeight::Light,
            FontWeightMessage::Normal => FontWeight::Normal,
            FontWeightMessage::Medium => FontWeight::Medium,
            FontWeightMessage::SemiBold => FontWeight::SemiBold,
            FontWeightMessage::Bold => FontWeight::Bold,
            FontWeightMessage::ExtraBold => FontWeight::ExtraBold,
            FontWeightMessage::Black => FontWeight::Black,
        }
    }
}

impl From<FontWeight> for FontWeightMessage {
    fn from(weight: FontWeight) -> Self {
        match weight {
            FontWeight::Thin => FontWeightMessage::Thin,
            FontWeight::ExtraLight => FontWeightMessage::ExtraLight,
            FontWeight::Light => FontWeightMessage::Light,
            FontWeight::Normal => FontWeightMessage::Normal,
            FontWeight::Medium => FontWeightMessage::Medium,
            FontWeight::SemiBold => FontWeightMessage::SemiBold,
            FontWeight::Bold => FontWeightMessage::Bold,
            FontWeight::ExtraBold => FontWeightMessage::ExtraBold,
            FontWeight::Black => FontWeightMessage::Black,
        }
    }
}
