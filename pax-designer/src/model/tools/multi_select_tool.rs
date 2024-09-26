use std::ops::ControlFlow;

use pax_engine::{
    api::Color, log, math::Point2, pax_manifest::TemplateNodeId, pax_runtime::TransformAndBounds,
    Property,
};
use pax_std::Size;

use crate::{
    glass::{RectTool, ToolVisualizationState},
    math::{coordinate_spaces::Glass, AxisAlignedBox},
    model::{
        action::{
            world::{SelectMode, SelectNodes},
            Action, ActionContext,
        },
        input::{Dir, InputEvent},
        ToolBehavior,
    },
};

pub struct MultiSelectTool {
    p1: Point2<Glass>,
    bounds: Property<AxisAlignedBox>,
    last_set: Vec<TemplateNodeId>,
}
impl MultiSelectTool {
    pub fn new(ctx: &mut ActionContext, point: Point2<Glass>) -> Self {
        if let Err(e) = (SelectNodes {
            ids: &[],
            mode: SelectMode::Dynamic,
        }
        .perform(ctx))
        {
            log::warn!("failed multi-select pointer up: {e}");
        };
        Self {
            p1: point,
            bounds: Property::new(AxisAlignedBox::new(point, point)),
            last_set: Default::default(),
        }
    }
}

impl ToolBehavior for MultiSelectTool {
    fn pointer_down(&mut self, _point: Point2<Glass>, _ctx: &mut ActionContext) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn pointer_move(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
        self.bounds.set(AxisAlignedBox::new(self.p1, point));
        let Some(project_root) = ctx.engine_context.get_userland_root_expanded_node() else {
            log::warn!("coudln't find userland root expanded node");
            return ControlFlow::Break(());
        };
        let selection_box = TransformAndBounds {
            transform: self.bounds.get().as_transform(),
            bounds: (1.0, 1.0),
        };
        let glass_transform = ctx.glass_transform();
        let open_container = ctx
            .derived_state
            .open_containers
            .get()
            .into_iter()
            .next()
            .unwrap();
        let mut to_process = project_root.children();
        let mut hits = vec![];
        while let Some(node) = to_process.pop() {
            if node.global_id().unwrap() == open_container {
                to_process.extend(node.children());
                continue;
            }
            let t_and_b = TransformAndBounds {
                transform: glass_transform.get(),
                bounds: (1.0, 1.0),
            } * node.transform_and_bounds().get();
            if t_and_b.intersects(&selection_box) {
                hits.push(node.global_id().unwrap().get_template_node_id());
            }
        }

        let mut newly_selected_nodes = hits.clone();
        newly_selected_nodes.retain(|e| !self.last_set.contains(e));
        let mut newly_deselected_nodes = self.last_set.clone();
        newly_deselected_nodes.retain(|e| !hits.contains(e));
        let to_toggle: Vec<_> = newly_selected_nodes
            .into_iter()
            .chain(newly_deselected_nodes.into_iter())
            .collect();
        if !to_toggle.is_empty() {
            if let Err(e) = (SelectNodes {
                ids: &to_toggle,
                mode: SelectMode::KeepOthers,
            }
            .perform(ctx))
            {
                log::warn!("failed to multi-select nodes: {e}");
            };
            self.last_set = hits;
        }
        ControlFlow::Continue(())
    }

    fn pointer_up(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
        self.pointer_move(point, ctx);
        ControlFlow::Break(())
    }

    fn keyboard(
        &mut self,
        _event: InputEvent,
        _dir: Dir,
        _ctx: &mut ActionContext,
    ) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn get_visual(&self) -> Property<ToolVisualizationState> {
        let bounds = self.bounds.clone();
        let deps = [bounds.untyped()];
        Property::computed(
            move || {
                let bounds = bounds.get();
                ToolVisualizationState {
                    rect_tool: RectTool {
                        x: Size::Pixels(bounds.top_left().x.into()),
                        y: Size::Pixels(bounds.top_left().y.into()),
                        width: Size::Pixels(bounds.width().into()),
                        height: Size::Pixels(bounds.height().into()),
                        stroke: Color::rgba(50.into(), 50.into(), 100.into(), 200.into()),
                        fill: Color::rgba(100.into(), 100.into(), 255.into(), 30.into()),
                    },
                    outline: Default::default(),
                    snap_lines: Default::default(),
                    event_blocker_active: true,
                }
            },
            &deps,
        )
    }

    fn finish(&mut self, _ctx: &mut ActionContext) -> anyhow::Result<()> {
        Ok(())
    }
}
