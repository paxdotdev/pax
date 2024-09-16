use pax_engine::{
    pax_manifest::{TreeIndexPosition, UniqueTemplateNodeIdentifier},
    Property,
};
use pax_std::{PathElement, Size};

use crate::{
    designer_node_type::DesignerNodeType,
    glass::ToolVisualizationState,
    model::{
        action::{
            orm::{CreateComponent, NodeLayoutSettings},
            Action, ActionContext,
        },
        ToolBehavior,
    },
};

pub struct PaintBrushTool {
    elements: Vec<PathElement>,
    #[allow(unused)]
    path_node_being_created: Option<UniqueTemplateNodeIdentifier>,
}

impl PaintBrushTool {
    pub fn new(ctx: &mut ActionContext) -> Self {
        let parent = ctx
            .derived_state
            .open_container
            .get()
            .into_iter()
            .next()
            .unwrap();
        let t = ctx.transaction("painting");
        let mut path_node_being_created = None;
        let _ = t.run(|| {
            let uid = CreateComponent {
                parent_id: &parent,
                parent_index: TreeIndexPosition::Top,
                designer_node_type: DesignerNodeType::Path,
                builder_extra_commands: None,
                node_layout: NodeLayoutSettings::Fill,
            }
            .perform(ctx)?;
            path_node_being_created = Some(uid);
            Ok(())
        });
        Self {
            elements: Vec::new(),
            path_node_being_created,
        }
    }
}

impl ToolBehavior for PaintBrushTool {
    fn pointer_down(
        &mut self,
        point: pax_engine::math::Point2<crate::math::coordinate_spaces::Glass>,
        _ctx: &mut ActionContext,
    ) -> std::ops::ControlFlow<()> {
        self.elements.push(PathElement::Point(
            Size::Pixels(point.x.into()),
            Size::Pixels(point.y.into()),
        ));
        // TODO either commit this, or make elements a property connected to engine
        std::ops::ControlFlow::Continue(())
    }

    fn pointer_move(
        &mut self,
        point: pax_engine::math::Point2<crate::math::coordinate_spaces::Glass>,
        _ctx: &mut ActionContext,
    ) -> std::ops::ControlFlow<()> {
        self.elements.push(PathElement::Line);
        self.elements.push(PathElement::Point(
            Size::Pixels(point.x.into()),
            Size::Pixels(point.y.into()),
        ));
        // TODO either commit this, or make elements a property connected to engine
        todo!()
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
