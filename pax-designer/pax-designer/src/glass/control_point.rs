use super::GlassPoint;
use pax_engine::api::*;
use pax_engine::math::Point2;
use pax_engine::*;
use pax_std::primitives::{Group, Path, Rectangle};
use pax_std::types::{Color, Fill};
use serde::Deserialize;

use crate::model;
use crate::model::AppState;
use crate::model::ToolState;

use crate::model::action::pointer::Pointer;
use crate::model::input::Dir;
use crate::model::math;
use crate::model::math::coordinate_spaces::{self, World};

#[pax]
#[file("glass/control_point.pax")]
pub struct ControlPoint {
    pub location: Property<GlassPoint>,
    pub ind: Property<Numeric>,
}

impl ControlPoint {
    pub fn mouse_down(&mut self, ctx: &NodeContext, args: ArgsMouseDown) {
        pax_engine::log::info!("clicked control point {}", self.ind.get().get_as_int());
        update_control_point(args, self.ind.get());
    }
}
