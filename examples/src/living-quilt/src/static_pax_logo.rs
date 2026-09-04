use pax_kit::*;

use crate::animated_pax_logo_banner::settled_banner_paths;
use crate::animated_pax_logo_post::settled_post_paths;

#[pax]
#[custom(Default)]
#[file("static_pax_logo.pax")]
pub struct StaticPaxLogo {
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
