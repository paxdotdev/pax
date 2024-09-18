use crate::{
    designer_node_type::DesignerNodeType,
    glass::ToolVisualizationState,
    math::boolean_path_operations::{self, SetOperations},
    model::{
        action::{
            orm::{CreateComponent, NodeLayoutSettings},
            Action, ActionContext, Transaction,
        },
        ToolBehavior,
    },
};
use anyhow::{anyhow, Result};
use pax_engine::{
    api::{borrow, borrow_mut},
    log,
    math::{Point2, Space},
    pax_manifest::{TreeIndexPosition, UniqueTemplateNodeIdentifier},
    Property, ToPaxValue,
};
use pax_std::{PathElement, Size};

pub struct PaintBrushTool {
    path_node_being_created: UniqueTemplateNodeIdentifier,
    transaction: Transaction,
}

impl PaintBrushTool {
    pub fn new(ctx: &mut ActionContext) -> Result<Self> {
        let parent = ctx
            .derived_state
            .open_container
            .get()
            .into_iter()
            .next()
            .unwrap();
        let t = ctx.transaction("painting");
        let uid = t.run(|| {
            CreateComponent {
                parent_id: &parent,
                parent_index: TreeIndexPosition::Top,
                designer_node_type: DesignerNodeType::Path,
                builder_extra_commands: None,
                node_layout: NodeLayoutSettings::Fill,
            }
            .perform(ctx)
        })?;
        Ok(Self {
            path_node_being_created: uid,
            transaction: t,
        })
    }
}

impl ToolBehavior for PaintBrushTool {
    fn pointer_down(
        &mut self,
        _point: pax_engine::math::Point2<crate::math::coordinate_spaces::Glass>,
        _ctx: &mut ActionContext,
    ) -> std::ops::ControlFlow<()> {
        std::ops::ControlFlow::Continue(())
    }

    fn pointer_move(
        &mut self,
        point: pax_engine::math::Point2<crate::math::coordinate_spaces::Glass>,
        ctx: &mut ActionContext,
    ) -> std::ops::ControlFlow<()> {
        let point = ctx.world_transform() * point;
        let path1 = todo!();
        let path2 = todo!();
        let union = path1.union(&path2);

        let pax_path = to_pax_path(union);
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
                let pax_value = pax_path.to_pax_value();
                let str_val = pax_value.to_string();
                log::debug!("str_val: {:?}", str_val);
                // TODO don't override, just add
                node.set_property("elements", &str_val)?;
                node.save()
                    .map_err(|e| anyhow!("failed to write elements on draw: {e}"))?;
            }
            Ok(())
        }) {
            log::warn!("failed to paint: {e}");
        }
        // TODO either commit this, or make elements a property connected to engine
        std::ops::ControlFlow::Continue(())
    }

    fn pointer_up(
        &mut self,
        point: pax_engine::math::Point2<crate::math::coordinate_spaces::Glass>,
        ctx: &mut ActionContext,
    ) -> std::ops::ControlFlow<()> {
        self.pointer_move(point, ctx);
        // Continue here instead? If so, how to cancel?
        std::ops::ControlFlow::Break(())
    }

    fn finish(&mut self, _ctx: &mut ActionContext) -> anyhow::Result<()> {
        // TODO
        Ok(())
    }

    fn keyboard(
        &mut self,
        _event: crate::model::input::InputEvent,
        _dir: crate::model::input::Dir,
        _ctx: &mut ActionContext,
    ) -> std::ops::ControlFlow<()> {
        // TODO brush size, etc
        std::ops::ControlFlow::Continue(())
    }

    fn get_visual(&self) -> Property<ToolVisualizationState> {
        Property::new(ToolVisualizationState::default())
    }
}

fn to_leon_path(path: Vec<PathElement>) -> Option<Subpath> {
    todo!()
    // let mut path_builder = Builder::new();
    // let mut pax_itr = path.into_iter();
    // path_builder.begin(match pax_itr.next()? {
    //     PathElement::Point(x, y) => Point2D::new(
    //         x.expect_pixels().to_float() as f32,
    //         y.expect_pixels().to_float() as f32,
    //     ),
    //     _ => {
    //         log::warn!("path must start with point");
    //         return None;
    //     }
    // });
    // while let Some(elem) = pax_itr.next() {
    //     if elem.is
    //     let point = pax_itr.next() else {
    //         log::warn!("expected all ops to")
    //         return None;
    //     };
    //     match elem {
    //         _ => {
    //             log::warn!("expected next op to be a line type");
    //             return None;
    //         }
    //         PathElement::Line => todo!(),
    //         PathElement::Quadratic(c_x, c_y) => {
    //             path_builder.quadratic_bezier_to(, )
    //         }
    //         ,
    //         PathElement::Cubic(_, _, _, _) => todo!(),
    //     }
    // }
    // Some(path_builder.build())
}

fn to_pax_path(path: Path) -> Vec<PathElement> {
    let mut pax_segs = vec![];
    for seg in &path {
        match seg {
            lyon::path::Event::Begin { at } => pax_segs.push(PathElement::Point(
                Size::Pixels(at.x.into()),
                Size::Pixels(at.y.into()),
            )),
            lyon::path::Event::Line { from: _, to } => {
                pax_segs.push(PathElement::Line);
                pax_segs.push(PathElement::Point(
                    Size::Pixels(to.x.into()),
                    Size::Pixels(to.y.into()),
                ));
            }
            lyon::path::Event::Quadratic { from: _, ctrl, to } => {
                pax_segs.push(PathElement::Quadratic(
                    Size::Pixels(ctrl.x.into()),
                    Size::Pixels(ctrl.y.into()),
                ));
                pax_segs.push(PathElement::Point(
                    Size::Pixels(to.x.into()),
                    Size::Pixels(to.y.into()),
                ));
            }
            lyon::path::Event::Cubic {
                from: _,
                ctrl1,
                ctrl2,
                to,
            } => {
                pax_segs.push(PathElement::Cubic(
                    Size::Pixels(ctrl1.x.into()),
                    Size::Pixels(ctrl1.y.into()),
                    Size::Pixels(ctrl2.x.into()),
                    Size::Pixels(ctrl2.y.into()),
                ));
                pax_segs.push(PathElement::Point(
                    Size::Pixels(to.x.into()),
                    Size::Pixels(to.y.into()),
                ));
            }
            lyon::path::Event::End {
                last: _,
                first: _,
                close,
            } => {
                if close {
                    pax_segs.push(PathElement::Close);
                }
            }
        }
    }
    pax_segs
}
