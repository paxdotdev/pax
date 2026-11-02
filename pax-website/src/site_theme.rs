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

    /// Neutral action color; chromatic accents belong to contours and proof.
    pub fn signal() -> Color {
        Self::paper()
    }
    pub fn cyan() -> Color {
        Color::from_hex("40F3FF")
    }
    pub fn pink() -> Color {
        Color::from_hex("FE32BF")
    }
    pub fn violet() -> Color {
        Self::pink()
    }

    /// Milky, diffuse polish; broad reflections keep dark lettering readable.
    pub fn opal() -> Material {
        Material::custom(MaterialParams {
            ambient: Property::new(0.94),
            diffuse: Property::new(0.10),
            specular: Property::new(0.14),
            roughness: Property::new(0.78),
            metallic: Property::new(0.0),
            ..Default::default()
        })
    }

    /// Crystalline black surface: quiet diffuse response and a narrow bright glint.
    pub fn obsidian() -> Material {
        Material::custom(MaterialParams {
            ambient: Property::new(0.65),
            diffuse: Property::new(0.08),
            specular: Property::new(3.8),
            roughness: Property::new(0.025),
            metallic: Property::new(0.0),
            ..Default::default()
        })
    }

    /// Quiet milky depth beneath the broad strip reflection; no outlined rim.
    pub fn opal_paint() -> Paint {
        polished_paint([
            (0.0, (248, 246, 240)),
            (0.35, (239, 240, 237)),
            (0.60, (231, 234, 233)),
            (0.86, (237, 237, 231)),
            (1.0, (245, 242, 236)),
        ])
    }

    /// Subtle cut planes beneath the real strip reflection, not a chrome bevel.
    /// This remains opaque paint; edge refraction is intentionally separate.
    pub fn obsidian_paint() -> Paint {
        polished_paint([
            (0.0, (43, 47, 54)),
            (0.43, (31, 35, 42)),
            (0.44, (23, 26, 32)),
            (0.85, (28, 32, 39)),
            (1.0, (36, 40, 47)),
        ])
    }
}

fn polished_paint(stops: [(f64, (u8, u8, u8)); 5]) -> Paint {
    Paint::LinearGradient(LinearGradient {
        start: (Size::Percent(0.0.into()), Size::Percent(0.0.into())),
        end: (Size::Percent(0.0.into()), Size::Percent(100.0.into())),
        stops: stops
            .into_iter()
            .map(|(position, (r, g, b))| GradientStop {
                position: Size::Percent((position * 100.0).into()),
                color: Color::from_rgba_0_1([
                    r as f64 / 255.0,
                    g as f64 / 255.0,
                    b as f64 / 255.0,
                    1.0,
                ]),
            })
            .collect(),
    })
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
