#![allow(unused_imports)]

use pax_kit::*;

#[pax]
#[has_helpers]
#[file("site_theme.pax")]
pub struct SiteTheme {}

#[helpers]
impl SiteTheme {
    /// Monochrome foundation, matching Living Quilt's warm logo and gray tiles.
    pub fn ink() -> Color {
        Color::from_hex("11110F")
    }
    pub fn surface() -> Color {
        Color::from_hex("191919")
    }
    pub fn raised() -> Color {
        Color::from_hex("242424")
    }
    pub fn paper() -> Color {
        Color::from_hex("FCF9F2")
    }
    pub fn muted() -> Color {
        Color::from_hex("B3B3B3")
    }
    pub fn rule() -> Color {
        Color::rgba(252.into(), 249.into(), 242.into(), 42.into())
    }

    /// Accents sampled from Living Quilt; color is reserved for actions and proof.
    pub fn signal() -> Color {
        Color::from_hex("F2F32B")
    }
    pub fn cyan() -> Color {
        Color::from_hex("40F3FF")
    }
    pub fn pink() -> Color {
        Color::from_hex("FE32BF")
    }
    pub fn violet() -> Color {
        // Lift the quilt's violet for small category labels on dark surfaces.
        Color::from_hex("B49BFF")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(color: Color) -> f64 {
        let rgba = color.to_rgba_0_1();
        let linear = |c: f64| {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        linear(rgba[0]) * 0.2126 + linear(rgba[1]) * 0.7152 + linear(rgba[2]) * 0.0722
    }

    #[test]
    fn text_palette_has_readable_contrast_on_dark_surfaces() {
        for foreground in [
            SiteTheme::paper(),
            SiteTheme::muted(),
            SiteTheme::signal(),
            SiteTheme::cyan(),
            SiteTheme::pink(),
            SiteTheme::violet(),
        ] {
            for background in [SiteTheme::ink(), SiteTheme::surface(), SiteTheme::raised()] {
                let contrast =
                    (luminance(foreground.clone()) + 0.05) / (luminance(background) + 0.05);
                assert!(contrast >= 4.5, "text contrast was {contrast:.2}:1");
            }
        }
    }
}
