#![allow(unused_imports)]

use pax_kit::*;

#[pax]
#[has_helpers]
#[file("site_theme.pax")]
pub struct SiteTheme {
    pub light_mode: Property<bool>,
}

#[helpers]
impl SiteTheme {
    /// Monochrome foundation, matching Living Quilt's warm logo and gray tiles.
    pub fn ink(light: bool) -> Color {
        Color::from_hex(if light { "FCF9F2" } else { "11110F" })
    }
    pub fn surface(light: bool) -> Color {
        Color::from_hex(if light { "F1EEE7" } else { "191919" })
    }
    pub fn raised(light: bool) -> Color {
        Color::from_hex(if light { "E4E1D9" } else { "242424" })
    }
    pub fn paper(light: bool) -> Color {
        Color::from_hex(if light { "171715" } else { "FCF9F2" })
    }
    pub fn muted(light: bool) -> Color {
        Color::from_hex(if light { "555550" } else { "B3B3B3" })
    }
    pub fn rule(light: bool) -> Color {
        if light {
            return Color::rgba(23.into(), 23.into(), 21.into(), 52.into());
        }
        Color::rgba(252.into(), 249.into(), 242.into(), 42.into())
    }

    /// Neutral action color; chromatic accents belong to contours and proof.
    pub fn signal(light: bool) -> Color {
        Self::paper(light)
    }
    // Paired face and emission inks: the face carries contrast on ivory; the
    // brighter tints are reserved for narrow rims, glints and scattered light.
    pub fn daylight_ink(hue: usize) -> Color {
        Color::from_hex(["007F9E", "C22181", "A86D00"][hue % 3])
    }
    pub fn daylight_light(hue: usize) -> Color {
        Color::from_hex(["00D8F4", "FF38A6", "FFD129"][hue % 3])
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

    /// Satin-white glass: leave headroom for both the moving sheen and the colored rim.
    pub fn frosted_face() -> Fill {
        Fill {
            paint: Property::new(Paint::Solid(Color::from_hex("ECF0F2"))),
            material: Property::new(Material::frosted(0.58)),
            opacity: Property::new(0.64.into()),
        }
    }

    pub fn action_face(light: bool) -> Fill {
        if light {
            Self::signal(true).into()
        } else {
            Self::frosted_face()
        }
    }

    pub fn pool_paint() -> Color {
        Color::from_hex("EEEEEE")
    }

    pub fn action_surface(light: bool, gpu: bool) -> Fill {
        if !light && gpu && cfg!(feature = "app") {
            Self::pool_paint().into()
        } else {
            Self::action_face(light)
        }
    }

    /// A narrow reflection on dark glass keeps the white label readable during the hero sweep.
    pub fn obsidian() -> Material {
        Material::custom(MaterialParams {
            ambient: Property::new(0.65),
            diffuse: Property::new(0.08),
            specular: Property::new(0.55),
            roughness: Property::new(0.025),
            metallic: Property::new(0.0),
            ..Default::default()
        })
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

    pub fn glass_face(white: bool) -> Fill {
        if white {
            return Self::frosted_face();
        }
        Fill {
            paint: Property::new(Self::obsidian_paint()),
            material: Property::new(Self::obsidian()),
            ..Default::default()
        }
    }
}

// The Pax property is authoritative while mounted, including changes from Paxflix.
// Document routes and the blog's independent header restore the same preference.
pub(crate) fn connect_preference(ctx: &NodeContext, light_mode: &Property<bool>) {
    #[cfg(target_arch = "wasm32")]
    {
        let Some(window) = web_sys::window() else {
            return;
        };
        let storage = window.local_storage().ok().flatten();
        let root = window.document().and_then(|doc| doc.document_element());
        // Canvas cannot paint the overscroll gutter or Safari's browser backdrop.
        if let Some(doc) = window
            .document()
            .filter(|d| d.get_element_by_id("mount").is_some())
        {
            if let (Some(el), Ok(style)) = (&root, doc.create_element("style")) {
                style.set_text_content(Some("html[data-pax-theme=light], html[data-pax-theme=light] body, html[data-pax-theme=light] #mount { background-color:#fcf9f2; color-scheme:light }"));
                let _ = el.append_child(&style);
            }
        }
        if let Some(saved) = storage
            .as_ref()
            .and_then(|s| s.get_item("pax-site-theme").ok().flatten())
        {
            light_mode.set(saved == "light");
        }
        let mode = light_mode.local();
        let update = move || {
            let value = if mode.get() { "light" } else { "dark" };
            if let Some(el) = &root {
                let _ = el.set_attribute("data-pax-theme", value);
                if el.has_attribute("data-theme") {
                    let _ = el.set_attribute("data-theme", value);
                }
            }
            if let Some(s) = &storage {
                let _ = s.set_item("pax-site-theme", value);
            }
        };
        update();
        ctx.subscribe(&[light_mode.untyped()], update);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (ctx, light_mode);
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
    fn white_actions_share_frost_and_leave_headroom_above_the_base_paint() {
        for face in [SiteTheme::glass_face(true), SiteTheme::action_face(false)] {
            let Paint::Solid(color) = face.paint.get() else {
                panic!("expected a single glass color")
            };
            // Even over pure black the translucent face must carry its dark label.
            let alpha = face.opacity.get().to_float_0_1();
            assert!((0.55..0.70).contains(&alpha));
            let [r, g, b, _] = color.to_rgba_0_1();
            let base = luminance(Color::from_rgba_0_1([r * alpha, g * alpha, b * alpha, 1.0]));
            assert!((base + 0.05) / (luminance(SiteTheme::ink(false)) + 0.05) >= 4.5);
            let Material::Lit(material) = face.material.get() else {
                panic!("expected frosted glass")
            };
            assert!(material.frosted.get() > 0.0);
            assert_eq!(material.specular.get(), 0.0);
        }
    }

    #[test]
    fn daylight_letter_faces_contrast_with_ivory_and_curtain_folds() {
        for hue in 0..3 {
            let ink = luminance(SiteTheme::daylight_ink(hue));
            assert!(luminance(SiteTheme::daylight_light(hue)) > ink);
            for surface in [SiteTheme::ink(true), Color::from_hex("DFDCD6")] {
                assert!((luminance(surface) + 0.05) / (ink + 0.05) >= 3.0);
            }
        }
    }

    #[test]
    fn text_palette_has_readable_contrast_in_both_modes() {
        for light in [false, true] {
            for foreground in [SiteTheme::paper(light), SiteTheme::muted(light)] {
                for background in [
                    SiteTheme::ink(light),
                    SiteTheme::surface(light),
                    SiteTheme::raised(light),
                ] {
                    let (a, b) = (luminance(foreground.clone()), luminance(background));
                    assert!((a.max(b) + 0.05) / (a.min(b) + 0.05) >= 4.5);
                }
            }
        }
    }
}

/// Share one 30 Hz phase across visible pool-floor buttons. An OS preference change
/// disconnects the runtime clock so a frozen face can reuse its retained output.
#[cfg(feature = "app")]
pub(crate) fn connect_button_clock(ctx: &NodeContext) {
    let phase = LocalProperty::new(0.);
    let millis = ctx.elapsed_millis.clone();
    let set_motion = {
        let phase = phase.clone();
        move |reduced: bool| {
            let held = phase.get();
            if reduced {
                phase.replace_with(LocalProperty::new(held));
            } else {
                let millis = millis.clone();
                let start = millis.get();
                let deps = [millis.untyped()];
                // A plain computed phase is lazy while there are no visible consumers.
                phase.replace_with(LocalProperty::computed(
                    move || {
                        held + (millis.get().saturating_sub(start) * 30 / 1000) as f64 * (1.25 / 30.)
                    },
                    &deps,
                ));
            }
        }
    };
    ctx.provide_store(ButtonSurfacePhase(phase))
        .expect("website owns its button clock");
    #[cfg(target_arch = "wasm32")]
    if let Some(query) = web_sys::window()
        .and_then(|window| window.match_media("(prefers-reduced-motion: reduce)").ok())
        .flatten()
    {
        use wasm_bindgen::{closure::Closure, JsCast};
        struct MotionPreference(web_sys::MediaQueryList, Closure<dyn FnMut()>);
        impl Store for MotionPreference {}
        impl std::ops::Drop for MotionPreference {
            fn drop(&mut self) {
                let _ = self
                    .0
                    .remove_event_listener_with_callback("change", self.1.as_ref().unchecked_ref());
            }
        }
        set_motion(query.matches());
        let media = query.clone();
        let changed =
            Closure::wrap(Box::new(move || set_motion(media.matches())) as Box<dyn FnMut()>);
        if query
            .add_event_listener_with_callback("change", changed.as_ref().unchecked_ref())
            .is_ok()
        {
            let _ = ctx.provide_store(MotionPreference(query, changed));
        }
        return;
    }
    set_motion(false);
}
