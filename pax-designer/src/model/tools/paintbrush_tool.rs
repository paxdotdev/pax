use std::{collections::HashMap, f64::consts::PI, ops::ControlFlow};

use crate::{
    controls::settings::property_editor::fill_property_editor::color_to_str,
    designer_node_type::DesignerNodeType,
    glass::ToolVisualizationState,
    math::{
        boolean_path_operations::{self, CompoundPath, DesignerPathId},
        coordinate_spaces::{Glass, World},
    },
    model::{
        action::{
            orm::{CreateComponent, NodeLayoutSettings},
            Action, ActionContext, Transaction,
        },
        ToolBehavior,
    },
};
use anyhow::{anyhow, Result};
use bezier_rs::{Bezier, Identifier, Subpath};
use glam::DVec2;
use pax_engine::{
    api::{borrow, borrow_mut, Color, Interpolatable, PathElement},
    log,
    math::{Point2, Space, Vector2},
    pax_manifest::{TreeIndexPosition, UniqueTemplateNodeIdentifier},
    PaxValue, Property, ToPaxValue,
};
use pax_std::Size;

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
            stroke_width: 3,
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
                    builder.set_property(
                        "stroke",
                        &format!(
                            "{{color: {}, width: {}px}}",
                            color_to_str(settings.stroke_color.clone()),
                            settings.stroke_width
                        ),
                    )?;
                    builder.set_property("fill", &color_to_str(settings.fill_color.clone()))?;
                    Ok(())
                }),
                node_layout: NodeLayoutSettings::Fill,
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
        let pax_path = to_pax_path(&new_path);
        self.path = Some(new_path);
        if let Err(e) = self.transaction.run(|| {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            let node = dt.get_orm_mut().get_node(
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

    fn finish(&mut self, _ctx: &mut ActionContext) -> anyhow::Result<()> {
        // TODO
        Ok(())
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

fn to_pax_path(path: &CompoundPath) -> Vec<PathElement> {
    let mut pax_segs = vec![];
    for subpath in &path.subpaths {
        let first = subpath.get_segment(0).map(|s| s.start).unwrap_or_default();
        pax_segs.push(PathElement::Point(
            Size::Pixels(first.x.into()),
            Size::Pixels(first.y.into()),
        ));
        for seg in subpath.iter() {
            match seg.handles {
                bezier_rs::BezierHandles::Linear => {
                    pax_segs.push(PathElement::Line);
                    pax_segs.push(PathElement::Point(
                        Size::Pixels(seg.end.x.into()),
                        Size::Pixels(seg.end.y.into()),
                    ));
                }
                bezier_rs::BezierHandles::Quadratic { handle: ctrl } => {
                    pax_segs.push(PathElement::Quadratic(
                        Size::Pixels(ctrl.x.into()),
                        Size::Pixels(ctrl.y.into()),
                    ));
                    pax_segs.push(PathElement::Point(
                        Size::Pixels(seg.end.x.into()),
                        Size::Pixels(seg.end.y.into()),
                    ));
                }
                bezier_rs::BezierHandles::Cubic {
                    handle_start: ctrl1,
                    handle_end: ctrl2,
                } => {
                    pax_segs.push(PathElement::Cubic(Box::new((
                        Size::Pixels(ctrl1.x.into()),
                        Size::Pixels(ctrl1.y.into()),
                        Size::Pixels(ctrl2.x.into()),
                        Size::Pixels(ctrl2.y.into()),
                    ))));
                    pax_segs.push(PathElement::Point(
                        Size::Pixels(seg.end.x.into()),
                        Size::Pixels(seg.end.y.into()),
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
