use pax_engine::{api::NodeContext, Property};
use pax_runtime_api::Color;

use crate::{glass::control_point::ControlPointStyling, model::GlassNode};

use super::ControlPointSet;

// TODO implement stacker control set
pub fn stacker_size_control_set(_ctx: NodeContext, _item: GlassNode) -> Property<ControlPointSet> {
    let control_point_styling = ControlPointStyling {
        round: false,
        stroke: Color::BLACK,
        fill: Color::rgba(100.into(), 100.into(), 100.into(), 100.into()),
        stroke_width_pixels: 2.0,
        // TODO make this size fat in one direciton, dep on manifest version
        size_pixels: 10.0,
    };
    let deps = [];
    Property::computed(
        move || ControlPointSet {
            points: vec![],
            styling: control_point_styling.clone(),
        },
        &deps,
    )
}
