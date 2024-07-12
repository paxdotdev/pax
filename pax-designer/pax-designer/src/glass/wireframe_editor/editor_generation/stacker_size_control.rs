use std::{cell::RefCell, rc::Rc};

use pax_engine::{api::NodeContext, math::Point2, Property};
use pax_runtime_api::{borrow, Color, Size};
use pax_std::{stacker::Stacker, types::StackerDirection};

use crate::{
    glass::control_point::{
        ControlPointBehaviour, ControlPointBehaviourFactory, ControlPointStyling,
    },
    math::coordinate_spaces::Glass,
    model::{
        self,
        action::{self, Action, ActionContext},
        GlassNode, GlassNodeSnapshot,
    },
};

use super::{CPoint, ControlPointSet};

pub fn stacker_size_control_set(ctx: NodeContext, item: GlassNode) -> Property<ControlPointSet> {
    struct StackerCellSizeBehaviour {
        initial_object: Option<GlassNodeSnapshot>,
    }

    impl ControlPointBehaviour for StackerCellSizeBehaviour {
        fn step(&self, ctx: &mut ActionContext, point: Point2<Glass>) {
            if let Some(initial_object) = &self.initial_object {
                let t_and_b = initial_object.transform_and_bounds;
                let point_in_space = t_and_b.transform.inverse() * point;
                if let Err(e) = (action::orm::SetAnchor {
                    object: &initial_object,
                    point: point_in_space,
                }
                .perform(ctx))
                {
                    pax_engine::log::warn!("resize failed: {:?}", e);
                };
            }
        }
    }

    fn stacker_cell_size_factory() -> ControlPointBehaviourFactory {
        Rc::new(move |ac, _p| {
            Rc::new(RefCell::new(StackerCellSizeBehaviour {
                initial_object: (&ac.derived_state.selection_state.get())
                    .items
                    .first()
                    .map(Into::into),
            }))
        })
    }

    // resize points

    let control_point_styling = ControlPointStyling {
        affected_by_transform: true,
        round: false,
        stroke: Color::rgba(255.into(), 255.into(), 255.into(), 150.into()),
        fill: Color::rgba(255.into(), 255.into(), 255.into(), 100.into()),
        stroke_width_pixels: 2.0,
        width: -1.0,
        height: -1.0,
    };
    let to_glass_transform =
        model::read_app_state_with_derived(|_, derived| derived.to_glass_transform.get());
    let dt = borrow!(ctx.designtime);
    let manifest_ver = dt.get_manifest_version();
    let object_transform = item.transform_and_bounds.clone();
    let deps = [object_transform.untyped(), manifest_ver.untyped()];
    let item_id = item.id;
    let ctx = ctx.clone();
    Property::computed(
        move || {
            let item = ctx
                .clone()
                .get_nodes_by_global_id(item_id.clone())
                .into_iter()
                .next()
                .unwrap();
            let item = GlassNode::new(&item, &to_glass_transform);
            let (cells, dir) = item
                .raw_node_interface
                .with_properties(|stacker: &mut Stacker| {
                    (stacker._cell_specs.get(), stacker.direction.get())
                })
                .unwrap();
            // TODO choose u or v depending on stacker direciton
            let (o, u, v) = item.transform_and_bounds.get().as_transform().decompose();
            let stacker_size_control_points = cells
                .into_iter()
                .skip(1)
                .map(|c| {
                    CPoint::new(
                        o + match dir {
                            StackerDirection::Vertical => c.y_px * v.normalize() + u / 2.0,
                            StackerDirection::Horizontal => c.x_px * u.normalize() + v / 2.0,
                        },
                        stacker_cell_size_factory(),
                    )
                })
                .collect();

            let mut cp_style = control_point_styling.clone();
            const MAJOR: f64 = 50.0;
            const MINOR: f64 = 7.0;
            match dir {
                StackerDirection::Vertical => {
                    cp_style.width = MAJOR;
                    cp_style.height = MINOR;
                }
                StackerDirection::Horizontal => {
                    cp_style.width = MINOR;
                    cp_style.height = MAJOR;
                }
            }

            ControlPointSet {
                points: stacker_size_control_points,
                styling: cp_style,
            }
        },
        &deps,
    )
}
