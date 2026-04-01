pub use pax_engine::api::Size;
use pax_engine::*;
use pax_runtime::{ExpandedNode, RuntimeContext};

#[pax]
#[engine_import_path("pax_engine")]
#[derive(Copy)]
pub struct Point {
    pub x: Size,
    pub y: Size,
}

impl Point {
    pub fn new(x: Size, y: Size) -> Self {
        Self { x, y }
    }

    pub fn to_kurbo_point(self, bounds: (f64, f64)) -> kurbo::Point {
        let x = self.x.evaluate(bounds, api::Axis::X);
        let y = self.y.evaluate(bounds, api::Axis::Y);
        kurbo::Point { x, y }
    }
}

pub fn patch_if_needed<T: PartialEq + Clone>(
    old_state: &mut Option<T>,
    patch: &mut Option<T>,
    new_value: T,
) -> bool {
    if !old_state.as_ref().is_some_and(|v| v == &new_value) {
        *patch = Some(new_value.clone());
        *old_state = Some(new_value);
        true
    } else {
        false
    }
}

pub fn native_surface_opacity(expanded_node: &ExpandedNode, context: &RuntimeContext) -> f64 {
    let opacity = expanded_node.computed_opacity.get().clamp(0.0, 1.0);
    let Some(parent_frame_id) = expanded_node.parent_frame.get() else {
        return opacity;
    };
    let Some(parent_frame) = context.get_expanded_node_by_eid(parent_frame_id) else {
        return opacity;
    };
    let parent_opacity = parent_frame.computed_opacity.get().clamp(0.0, 1.0);
    if parent_opacity <= f64::EPSILON {
        0.0
    } else {
        (opacity / parent_opacity).clamp(0.0, 1.0)
    }
}
