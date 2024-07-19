use std::{cell::RefCell, rc::Rc};

use anyhow::anyhow;
use pax_engine::{api::NodeContext, log, math::Point2, Property};
use pax_runtime_api::{borrow, borrow_mut, Color, Size};
use pax_std::{stacker::Stacker, types::StackerDirection};

use crate::{
    glass::control_point::{
        ControlPointBehaviour, ControlPointBehaviourFactory, ControlPointStyling,
    },
    math::{coordinate_spaces::Glass, GetUnit},
    model::{
        self,
        action::{self, Action, ActionContext},
        GlassNode, GlassNodeSnapshot,
    },
};

use super::{CPoint, ControlPointSet};

pub fn stacker_divider_control_set(ctx: NodeContext, item: GlassNode) -> Property<ControlPointSet> {
    struct StackerDividerControlBehaviour {
        stacker_node: GlassNodeSnapshot,
        resize_ind: usize,
        start_sizes: Vec<Option<Size>>,
        boundaries: Vec<(f64, f64)>,
        dir: StackerDirection,
    }

    impl ControlPointBehaviour for StackerDividerControlBehaviour {
        fn step(&self, ctx: &mut ActionContext, point: Point2<Glass>) {
            let t = self.stacker_node.transform_and_bounds.as_transform();
            let (_, u, v) = t.decompose();
            let (x_l, y_l) = (u.length(), v.length());
            let box_point = t.inverse() * point;
            let (ratio, total) = match self.dir {
                StackerDirection::Vertical => (box_point.y, y_l),
                StackerDirection::Horizontal => (box_point.x, x_l),
            };

            let mut new_sizes = self.start_sizes.clone();
            while new_sizes.len() < self.boundaries.len() {
                new_sizes.push(None);
            }
            let mut positions: Vec<f64> = self.boundaries.iter().map(|v| v.0).collect();
            if let Some((p, w)) = self.boundaries.last() {
                positions.push(p + w);
            }
            let above = positions[self.resize_ind] / total;
            let above_unit = new_sizes[self.resize_ind].unit();
            let new_val_ratio = (ratio - above).max(0.0);

            new_sizes[self.resize_ind] = Some(match above_unit {
                crate::math::SizeUnit::Pixels => {
                    Size::Pixels(round_2_dec(new_val_ratio * total).into())
                }
                crate::math::SizeUnit::Percent => {
                    Size::Percent(round_2_dec(new_val_ratio * 100.0).into())
                }
            });

            let sizes_str = sizes_to_string(&new_sizes);

            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            let mut builder = dt
                .get_orm_mut()
                .get_node(self.stacker_node.id.clone())
                .unwrap();

            builder.set_property("sizes", &sizes_str).unwrap();

            builder
                .save()
                .map_err(|e| anyhow!("could not save: {}", e))
                .unwrap();
        }
    }

    fn stacker_divider_control_factory(
        resize_ind: usize,
        boundaries: Vec<(f64, f64)>,
        start_sizes: Vec<Option<Size>>,
        item: GlassNode,
        dir: StackerDirection,
    ) -> ControlPointBehaviourFactory {
        let stacker_id = item.id.clone();
        ControlPointBehaviourFactory {
            tool_behaviour: Rc::new(move |_ac, _p| {
                Rc::new(RefCell::new(StackerDividerControlBehaviour {
                    stacker_node: (&item).into(),
                    resize_ind,
                    boundaries: boundaries.clone(),
                    start_sizes: start_sizes.clone(),
                    dir: dir.clone(),
                }))
            }),
            double_click_behaviour: Rc::new(move |ctx| {
                let stacker_node = ctx
                    .engine_context
                    .get_nodes_by_global_id(stacker_id.clone())
                    .into_iter()
                    .next()
                    .unwrap();
                let mut sizes = stacker_node
                    .with_properties(|stacker: &mut Stacker| stacker.sizes.get())
                    .unwrap();
                if let Some(val) = sizes.get_mut(resize_ind) {
                    *val = None
                }

                let sizes_str = sizes_to_string(&sizes);

                let mut dt = borrow_mut!(ctx.engine_context.designtime);
                let mut builder = dt.get_orm_mut().get_node(stacker_id.clone()).unwrap();

                builder.set_property("sizes", &sizes_str).unwrap();

                builder
                    .save()
                    .map_err(|e| anyhow!("could not save: {}", e))
                    .unwrap();
            }),
        }
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
            let (cells, dir, start_sizes) = item
                .raw_node_interface
                .with_properties(|stacker: &mut Stacker| {
                    (
                        stacker._cell_specs.get(),
                        stacker.direction.get(),
                        stacker.sizes.get(),
                    )
                })
                .unwrap();
            let (o, u, v) = item.transform_and_bounds.get().as_transform().decompose();
            let (w, h) = item.transform_and_bounds.get().bounds;
            let boundaries: Vec<_> = cells
                .into_iter()
                .map(|c| match dir {
                    StackerDirection::Vertical => (c.y_px, c.height_px),
                    StackerDirection::Horizontal => (c.x_px, c.width_px),
                })
                .collect();

            let stacker_divider_control_points = boundaries
                .iter()
                .enumerate()
                .map(|(i, &c)| {
                    CPoint::new(
                        o + match dir {
                            StackerDirection::Vertical => (c.0 + c.1) / h * v + u / 2.0,
                            StackerDirection::Horizontal => (c.0 + c.1) / w * u + v / 2.0,
                        },
                        stacker_divider_control_factory(
                            i,
                            boundaries.clone(),
                            start_sizes.clone(),
                            item.clone(),
                            dir.clone(),
                        ),
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
                points: stacker_divider_control_points,
                styling: cp_style,
            }
        },
        &deps,
    )
}

fn round_2_dec(v: f64) -> f64 {
    (v * 100.0).floor() / 100.0
}

pub fn sizes_to_string(sizes: &[Option<Size>]) -> String {
    let mut sizes_str = String::new();
    sizes_str.push('[');
    for e in sizes {
        match e {
            Some(v) => {
                sizes_str.push_str("Some(");
                match v {
                    Size::Pixels(px) => sizes_str.push_str(&format!("{}px", px)),
                    Size::Percent(perc) => sizes_str.push_str(&format!("{}%", perc)),
                    Size::Combined(px, perc) => {
                        sizes_str.push_str(&format!("{}px + {}%", px, perc))
                    }
                }
                sizes_str.push_str(")");
            }
            None => sizes_str.push_str("None"),
        }
        sizes_str.push(',')
    }
    if sizes_str.ends_with(',') {
        sizes_str.pop();
    }
    sizes_str.push(']');
    sizes_str
}
