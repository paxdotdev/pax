use crate::*;
use pax_engine::api::*;
use pax_engine::*;

use super::animated_pax_logo_banner::settled_banner_paths;
use super::animated_pax_logo_post::settled_post_paths;

/// The settled Pax logo, using the animation's final geometry without masks.
///
/// This component has no playback or click behavior. It uses the same palette
/// and 920.72 × 463.95 coordinate system as [`AnimatedPaxLogo`].
#[pax]
#[engine_import_path("pax_engine")]
#[custom(Default)]
#[file("drawing/pax_logo/static_pax_logo.pax")]
pub struct StaticPaxLogo {
    /// The surrounding surface; Dark exchanges the signboard and letter fills.
    pub background_mode: Property<LogoBackgroundMode>,
    /// Signboard/post fill in Light mode, lettering/counters in Dark mode.
    pub fill: Property<Paint>,
    /// Lettering/backing fill in Light mode, signboard/post in Dark mode.
    pub letter_fill: Property<Paint>,
    pub banner_elements: Property<Vec<PathElement>>,
    pub p_ring_elements: Property<Vec<PathElement>>,
    pub p_counter_elements: Property<Vec<PathElement>>,
    pub a_elements: Property<Vec<PathElement>>,
    pub a_counter_elements: Property<Vec<PathElement>>,
    pub x_elements: Property<Vec<PathElement>>,
    pub roll_start_cap_elements: Property<Vec<PathElement>>,
    pub roll_body_elements: Property<Vec<PathElement>>,
    pub roll_end_cap_elements: Property<Vec<PathElement>>,
    pub top_fabric_elements: Property<Vec<PathElement>>,
    pub fabric_elements: Property<Vec<PathElement>>,
    pub backing_fabric_elements: Property<Vec<PathElement>>,
}

impl Default for StaticPaxLogo {
    fn default() -> Self {
        let [banner, p_ring, p_counter, a, a_counter, x] = settled_banner_paths();
        let [roll_start, roll_body, roll_end, top_fabric, fabric, backing_fabric] =
            settled_post_paths();
        Self {
            background_mode: Property::new(LogoBackgroundMode::Light),
            fill: Property::new(Paint::Solid(Color::BLACK)),
            letter_fill: Property::new(Paint::Solid(Color::WHITE)),
            banner_elements: Property::new(banner),
            p_ring_elements: Property::new(p_ring),
            p_counter_elements: Property::new(p_counter),
            a_elements: Property::new(a),
            a_counter_elements: Property::new(a_counter),
            x_elements: Property::new(x),
            roll_start_cap_elements: Property::new(roll_start),
            roll_body_elements: Property::new(roll_body),
            roll_end_cap_elements: Property::new(roll_end),
            top_fabric_elements: Property::new(top_fabric),
            fabric_elements: Property::new(fabric),
            backing_fabric_elements: Property::new(backing_fabric),
        }
    }
}
