use std::{collections::HashMap, f64::consts::PI, ops::ControlFlow};

use crate::{
    designer_node_type::DesignerNodeType,
    glass::ToolVisualizationState,
    math::{
        boolean_path_operations::{self, CompoundPath, DesignerPathId},
        coordinate_spaces::{Glass, World},
        IntoDecompositionConfiguration,
    },
    model::{
        action::{
            orm::{CreateComponent, NodeLayoutSettings, SetNodeLayout},
            Action, ActionContext, Transaction,
        },
        ToolBehavior,
    },
};
use anyhow::{anyhow, Result};
use bezier_rs::{Bezier, Identifier, Subpath};
use glam::DVec2;
use pax_engine::{
    api::{borrow, borrow_mut, Axis, Color, Interpolatable, PathElement, Stroke},
    log,
    math::{Point2, Space, Transform2, Vector2},
    pax_manifest::{TreeIndexPosition, UniqueTemplateNodeIdentifier},
    pax_runtime::TransformAndBounds,
    PaxValue, Property, ToPaxValue,
};
use pax_std::{Path, Size};

pub struct PaintbrushTool {
    path_node_being_created: UniqueTemplateNodeIdentifier,
    transaction: Transaction,
    path: Option<CompoundPath>,
    last_pos: Point2<World>,
}

thread_local! {
    // TODO make a general way for tools to be stored in app state, combined with editor binding state
    pub static PAINTBRUSH_TOOL: Property<PaintbrushToolSettings> = Property::new(PaintbrushToolSettings::default());
}

impl Interpolatable for PaintbrushToolSettings {}

// if this isn't used to make the draw capsule slightly cone shaped, the curve segment of the last drawn capsule end perfectly
// overlaps the one of the new one -> bad days
const RATIO_OFFSET: f64 = 0.99;

#[derive(Clone)]
pub struct PaintbrushToolSettings {
    pub brush_radius: f64,
    pub fill_color: Color,
    pub stroke_color: Color,
    pub stroke_width: u32,
}

impl Default for PaintbrushToolSettings {
    fn default() -> Self {
        Self {
            brush_radius: 30.0,
            fill_color: Color::BLACK,
            stroke_color: Color::GRAY,
            stroke_width: 0,
        }
    }
}

impl PaintbrushTool {
    pub fn new(ctx: &mut ActionContext) -> Result<Self> {
        let parent = ctx
            .derived_state
            .open_containers
            .get()
            .into_iter()
            .next()
            .unwrap();
        let settings = PAINTBRUSH_TOOL.with(|p| p.get());
        let t = ctx.transaction("painting");
        let uid = t.run(|| {
            CreateComponent {
                parent_id: &parent,
                parent_index: TreeIndexPosition::Top,
                designer_node_type: DesignerNodeType::Path,
                builder_extra_commands: Some(&|builder| {
                    builder.set_property_from_typed(
                        "stroke",
                        Some(Stroke {
                            color: Property::new(settings.stroke_color.clone()),
                            width: Property::new(Size::Pixels(settings.stroke_width.into())),
                        }),
                    )?;
                    builder.set_property_from_typed("fill", Some(settings.fill_color.clone()))?;
                    Ok(())
                }),
                node_layout: Some(NodeLayoutSettings::Fill),
            }
            .perform(ctx)
        })?;
        Ok(Self {
            path_node_being_created: uid,
            transaction: t,
            path: None,
            last_pos: Point2::default(),
        })
    }
}

impl ToolBehavior for PaintbrushTool {
    fn pointer_down(
        &mut self,
        _point: pax_engine::math::Point2<crate::math::coordinate_spaces::Glass>,
        _ctx: &mut ActionContext,
    ) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn pointer_move(
        &mut self,
        point: pax_engine::math::Point2<crate::math::coordinate_spaces::Glass>,
        ctx: &mut ActionContext,
    ) -> ControlFlow<()> {
        let point = ctx.world_transform() * point;
        if (point - self.last_pos).length_squared() < 2.0 {
            return ControlFlow::Continue(());
        }
        let r = PAINTBRUSH_TOOL.with(|p| p.get().brush_radius);

        let new_path = if let Some(path) = &self.path {
            let capsule = capsule_from_points_and_radius(self.last_pos, point, r);
            path.union(&capsule)
        } else {
            CompoundPath::from_subpath(Subpath::new_ellipse(
                DVec2 {
                    x: point.x - r * RATIO_OFFSET,
                    y: point.y - r * RATIO_OFFSET,
                },
                DVec2 {
                    x: point.x + r * RATIO_OFFSET,
                    y: point.y + r * RATIO_OFFSET,
                },
            ))
        };
        let Ok(path_node) = ctx.get_glass_node_by_global_id(&self.path_node_being_created) else {
            log::warn!("failed to get path node");
            return ControlFlow::Continue(());
        };
        let world_t_and_b = TransformAndBounds {
            transform: ctx.world_transform(),
            bounds: (1.0, 1.0),
        } * path_node.transform_and_bounds.get();

        let pax_path = to_pax_path(&new_path, world_t_and_b.bounds);
        self.path = Some(new_path);
        if let Err(e) = self.transaction.run(|| {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            let node = dt.get_orm_mut().get_node_builder(
                self.path_node_being_created.clone(),
                ctx.app_state
                    .modifiers
                    .get()
                    .contains(&crate::model::input::ModifierKey::Control),
            );
            if let Some(mut node) = node {
                node.set_property_from_typed("elements", Some(pax_path))?;
                node.save()
                    .map_err(|e| anyhow!("failed to write elements on draw: {e}"))?;
            }
            Ok(())
        }) {
            log::warn!("failed to paint: {e}");
        }
        // TODO either commit this, or make elements a property connected to engine
        self.last_pos = point;
        ControlFlow::Continue(())
    }

    fn pointer_up(
        &mut self,
        _point: pax_engine::math::Point2<crate::math::coordinate_spaces::Glass>,
        _ctx: &mut ActionContext,
    ) -> ControlFlow<()> {
        ControlFlow::Break(())
    }

    fn finish(&mut self, ctx: &mut ActionContext) -> Result<()> {
        // Re-normalize bounding box of path to only fill space that was drawn on
        let path_node = ctx.get_glass_node_by_global_id(&self.path_node_being_created)?;
        let to_world = TransformAndBounds {
            transform: ctx.world_transform(),
            bounds: (1.0, 1.0),
        };
        let world_t_and_b = to_world * path_node.transform_and_bounds.get();
        let path_elements = path_node
            .raw_node_interface
            .with_properties(|path: &mut Path| path.elements.get())
            .ok_or_else(|| anyhow!("not a path node"))?;
        let mut points = vec![];
        for e in &path_elements {
            match e {
                &PathElement::Point(x, y) => points.push((x, y)),
                PathElement::Line => (),
                &PathElement::Quadratic(x, y) => points.push((x, y)),
                &PathElement::Cubic(v1, v2, v3, v4) => {
                    points.push((v1, v2));
                    points.push((v3, v4));
                }
                PathElement::Close => (),
                PathElement::Empty => (),
            }
        }
        let (w, h) = world_t_and_b.bounds;
        let mut max_x: f64 = 0.0;
        let mut max_y: f64 = 0.0;
        let mut min_x: f64 = w;
        let mut min_y: f64 = h;
        for (s_x, s_y) in points {
            let x = s_x.evaluate(world_t_and_b.bounds, Axis::X);
            let y = s_y.evaluate(world_t_and_b.bounds, Axis::Y);
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
        let old_origin = world_t_and_b.transform.get_translation();
        let old_origin = (old_origin.x, old_origin.y);
        let old_bounds = world_t_and_b.bounds;
        let new_origin = (min_x, min_y);
        let new_bounds = (max_x - min_x, max_y - min_y);
        self.transaction.run(|| {
            SetNodeLayout {
                id: &path_node.id,
                node_layout: &NodeLayoutSettings::KeepScreenBounds {
                    node_transform_and_bounds: &TransformAndBounds {
                        transform: Transform2::translate(Vector2::new(new_origin.0, new_origin.1)),
                        bounds: new_bounds,
                    },
                    parent_transform_and_bounds: &(to_world
                        * path_node.parent_transform_and_bounds.get()),
                    node_decomposition_config: &path_node
                        .layout_properties
                        .into_decomposition_config(),
                },
            }
            .perform(ctx)?;

            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            let mut node = dt
                .get_orm_mut()
                .get_node_builder(path_node.id.clone(), false)
                .ok_or_else(|| anyhow!("failed to get path node"))?;
            let conv_x = |x| {
                convert_size_to_new_range(
                    x,
                    old_bounds.0,
                    old_origin.0 - new_origin.0,
                    new_bounds.0,
                )
            };
            let conv_y = |y| {
                convert_size_to_new_range(
                    y,
                    old_bounds.1,
                    old_origin.1 - new_origin.1,
                    new_bounds.1,
                )
            };
            let new_elements: Vec<_> = path_elements
                .into_iter()
                .map(|e| match e {
                    PathElement::Empty => PathElement::Empty,
                    PathElement::Point(x, y) => PathElement::Point(conv_x(x), conv_y(y)),
                    PathElement::Line => PathElement::Line,
                    PathElement::Quadratic(x, y) => PathElement::Quadratic(conv_x(x), conv_y(y)),
                    PathElement::Cubic(v1, v2, v3, v4) => {
                        PathElement::Cubic(conv_x(v1), conv_y(v2), conv_x(v3), conv_y(v4))
                    }
                    PathElement::Close => PathElement::Close,
                })
                .collect();
            node.set_property_from_typed("elements", Some(new_elements))?;
            node.save()
                .map_err(|e| anyhow!("failed to save re-normalized path {e}"))?;
            Ok(())
        })
    }

    fn keyboard(
        &mut self,
        event: crate::model::input::InputEvent,
        _dir: crate::model::input::Dir,
        _ctx: &mut ActionContext,
    ) -> ControlFlow<()> {
        match event {
            crate::model::input::InputEvent::FinishCurrentTool => ControlFlow::Break(()),
            _ => ControlFlow::Continue(()),
        }
        // TODO brush size, etc
    }

    fn get_visual(&self) -> Property<ToolVisualizationState> {
        Property::new(ToolVisualizationState::default())
    }
}

fn convert_size_to_new_range(v: Size, area_px: f64, offset_px: f64, new_area_px: f64) -> Size {
    let old_width = area_px;
    let new_width = new_area_px;
    let offset_px = offset_px;

    match v {
        Size::Pixels(px) => {
            let new_px = px.to_float() + offset_px;
            Size::Pixels(new_px.into())
        }
        Size::Percent(perc) => {
            let px = perc.to_float() * old_width / 100.0;
            let new_px = px + offset_px;
            let new_perc = new_px * 100.0 / new_width;
            Size::Percent(new_perc.into())
        }
        Size::Combined(px, perc) => {
            let old_px_total = px.to_float() + perc.to_float() * old_width / 100.0;
            let new_px_total = (old_px_total + offset_px) * new_width / old_width;

            // Maintain the same ratio between pixels and percentage
            let ratio = px.to_float() / (perc.to_float() * old_width / 100.0);
            let new_px = new_px_total / (1.0 + 1.0 / ratio);
            let new_perc = (new_px_total - new_px) * 100.0 / new_width;

            Size::Combined(new_px.into(), new_perc.into())
        }
    }
}

fn capsule_from_points_and_radius(
    start: Point2<World>,
    end: Point2<World>,
    r: f64,
) -> CompoundPath {
    let x_e = (end - start).normalize() * r;
    let y_e = x_e.rotate90();

    let p: Vec<_> = (0..5)
        .map(|i| {
            let (sin, cos) = (i as f64 * PI / 4.0).sin_cos();
            let v = sin * x_e - cos * y_e;
            DVec2 { x: v.x, y: v.y }
        })
        .collect();
    let s = DVec2 {
        x: start.x,
        y: start.y,
    };
    let e = DVec2 { x: end.x, y: end.y };
    let beziers = [
        Bezier::from_linear_dvec2(s - p[0], e + p[4] * RATIO_OFFSET),
        Bezier::cubic_through_points(
            e + p[4] * RATIO_OFFSET,
            e + p[3] * RATIO_OFFSET,
            e + p[2] * RATIO_OFFSET,
            None,
            None,
        ),
        Bezier::cubic_through_points(
            e + p[2] * RATIO_OFFSET,
            e + p[1] * RATIO_OFFSET,
            e + p[0] * RATIO_OFFSET,
            None,
            None,
        ),
        Bezier::from_linear_dvec2(e + p[0] * RATIO_OFFSET, s - p[4]),
        Bezier::cubic_through_points(s - p[4], s - p[3], s - p[2], None, None),
        Bezier::cubic_through_points(s - p[2], s - p[1], s - p[0], None, None),
    ];

    CompoundPath::from_subpath(Subpath::from_beziers(&beziers, true))
}

fn to_pax_path(path: &CompoundPath, bounds: (f64, f64)) -> Vec<PathElement> {
    let mut pax_segs = vec![];
    for subpath in &path.subpaths {
        let first = subpath.get_segment(0).map(|s| s.start).unwrap_or_default();
        pax_segs.push(PathElement::Point(
            Size::Percent((100.0 * first.x / bounds.0).into()),
            Size::Percent((100.0 * first.y / bounds.1).into()),
        ));
        for seg in subpath.iter() {
            match seg.handles {
                bezier_rs::BezierHandles::Linear => {
                    pax_segs.push(PathElement::Line);
                    pax_segs.push(PathElement::Point(
                        Size::Percent((100.0 * seg.end.x / bounds.0).into()),
                        Size::Percent((100.0 * seg.end.y / bounds.1).into()),
                    ));
                }
                bezier_rs::BezierHandles::Quadratic { handle: ctrl } => {
                    pax_segs.push(PathElement::Quadratic(
                        Size::Percent((100.0 * ctrl.x / bounds.0).into()),
                        Size::Percent((100.0 * ctrl.y / bounds.1).into()),
                    ));
                    pax_segs.push(PathElement::Point(
                        Size::Percent((100.0 * seg.end.x / bounds.0).into()),
                        Size::Percent((100.0 * seg.end.y / bounds.1).into()),
                    ));
                }
                bezier_rs::BezierHandles::Cubic {
                    handle_start: ctrl1,
                    handle_end: ctrl2,
                } => {
                    pax_segs.push(PathElement::Cubic(
                        Size::Percent((100.0 * ctrl1.x / bounds.0).into()),
                        Size::Percent((100.0 * ctrl1.y / bounds.1).into()),
                        Size::Percent((100.0 * ctrl2.x / bounds.0).into()),
                        Size::Percent((100.0 * ctrl2.y / bounds.1).into()),
                    ));
                    pax_segs.push(PathElement::Point(
                        Size::Percent((100.0 * seg.end.x / bounds.0).into()),
                        Size::Percent((100.0 * seg.end.y / bounds.1).into()),
                    ));
                }
            }
        }
        if subpath.closed {
            pax_segs.push(PathElement::Close);
        }
    }
    pax_segs
}
