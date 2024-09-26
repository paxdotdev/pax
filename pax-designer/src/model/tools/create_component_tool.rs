use std::ops::ControlFlow;

use pax_designtime::orm::template::builder::NodeBuilder;
use pax_engine::{
    api::Color,
    math::{Point2, Vector2},
    pax_manifest::{TreeIndexPosition, UniqueTemplateNodeIdentifier},
    pax_runtime::TransformAndBounds,
    Property,
};
use pax_std::Size;

use crate::{
    designer_node_type::DesignerNodeType,
    glass::{RectTool, ToolVisualizationState},
    math::{
        coordinate_spaces::Glass, intent_snapper::IntentSnapper, AxisAlignedBox,
        DecompositionConfiguration,
    },
    model::{
        action::{
            orm::{CreateComponent, NodeLayoutSettings},
            world::{SelectMode, SelectNodes},
            Action, ActionContext,
        },
        input::ModifierKey,
        ToolBehavior,
    },
};
use anyhow::{anyhow, Result};

pub struct PostCreationData<'a> {
    pub uid: &'a UniqueTemplateNodeIdentifier,
    pub bounds: &'a AxisAlignedBox<Glass>,
}

pub struct CreateComponentTool {
    designer_node_type: DesignerNodeType,
    origin: Point2<Glass>,
    bounds: Property<AxisAlignedBox>,
    builder_extra_commands: Option<Box<dyn Fn(&mut NodeBuilder) -> Result<()>>>,
    post_creation_hook: Option<Box<dyn Fn(&mut ActionContext, PostCreationData) -> Result<()>>>,
    intent_snapper: IntentSnapper,
}

impl CreateComponentTool {
    pub fn new(
        point: Point2<Glass>,
        designer_node_type: DesignerNodeType,
        ctx: &ActionContext,
    ) -> Self {
        Self {
            designer_node_type,
            origin: point,
            bounds: Property::new(AxisAlignedBox::new(Point2::default(), Point2::default())),
            intent_snapper: IntentSnapper::new_from_scene(ctx, &[]),
            builder_extra_commands: None,
            post_creation_hook: None,
        }
    }

    // WARNING: When this is called, the node exists in the manifest, but NOT in the engine
    // NOTE: This function is run inside the transaction performed while creating the component
    pub fn with_post_creation_hook(
        mut self,
        post_creation: impl Fn(&mut ActionContext, PostCreationData) -> Result<()> + 'static,
    ) -> Self {
        self.post_creation_hook = Some(Box::new(post_creation));
        self
    }

    pub fn with_extra_builder_commands(
        mut self,
        cmds: impl Fn(&mut NodeBuilder) -> Result<()> + 'static,
    ) -> Self {
        self.builder_extra_commands = Some(Box::new(cmds));
        self
    }
}

impl ToolBehavior for CreateComponentTool {
    fn pointer_down(&mut self, _point: Point2<Glass>, _ctx: &mut ActionContext) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn pointer_move(
        &mut self,
        point: Point2<Glass>,
        ctx: &mut ActionContext,
    ) -> std::ops::ControlFlow<()> {
        let modifiers = ctx.app_state.modifiers.get();
        let is_shift_key_down = modifiers.contains(&ModifierKey::Shift);
        let is_alt_key_down = modifiers.contains(&ModifierKey::Alt);
        let offset = self.intent_snapper.snap(&[point], false, false);
        self.bounds.set(
            AxisAlignedBox::new(self.origin, self.origin + Vector2::new(1.0, 1.0))
                .morph_constrained(
                    point + offset,
                    self.origin,
                    is_alt_key_down,
                    is_shift_key_down,
                ),
        );
        ControlFlow::Continue(())
    }

    fn pointer_up(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
        self.pointer_move(point, ctx);
        let bounds = self.bounds.get();
        if bounds.width() < 3.0 || bounds.height() < 3.0 {
            // don't create if to small of a movement
            return ControlFlow::Break(());
        }
        let box_transform = bounds.as_transform();
        let Ok(parent) =
            ctx.get_glass_node_by_global_id(&ctx.derived_state.open_containers.get()[0])
        else {
            return ControlFlow::Continue(());
        };
        let unit = ctx.app_state.unit_mode.get();
        let t = ctx.transaction("creating object");
        let _ = t.run(|| {
            let uid = CreateComponent {
                parent_id: &parent.id,
                parent_index: TreeIndexPosition::Top,
                node_layout: NodeLayoutSettings::KeepScreenBounds {
                    node_transform_and_bounds: &TransformAndBounds {
                        transform: box_transform,
                        bounds: (1.0, 1.0),
                    }
                    .as_pure_size(),
                    parent_transform_and_bounds: &parent.transform_and_bounds.get().as_pure_size(),
                    node_decomposition_config: &DecompositionConfiguration {
                        unit_x_pos: unit,
                        unit_y_pos: unit,
                        unit_width: unit,
                        unit_height: unit,
                        ..Default::default()
                    },
                },
                designer_node_type: self.designer_node_type.clone(),
                builder_extra_commands: self.builder_extra_commands.as_ref().map(|v| v.as_ref()),
            }
            .perform(ctx)?;

            if let Some(post_creation) = &self.post_creation_hook {
                post_creation(
                    ctx,
                    PostCreationData {
                        uid: &uid,
                        bounds: &bounds,
                    },
                )?;
            }
            SelectNodes {
                ids: &[uid.get_template_node_id()],
                mode: SelectMode::DiscardOthers,
            }
            .perform(ctx)?;

            Ok(())
        });
        ControlFlow::Break(())
    }

    fn keyboard(
        &mut self,
        _event: crate::model::input::InputEvent,
        _dir: crate::model::input::Dir,
        _ctx: &mut ActionContext,
    ) -> std::ops::ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn get_visual(&self) -> Property<ToolVisualizationState> {
        let bounds = self.bounds.clone();
        let snap_lines = self.intent_snapper.get_snap_lines_prop();
        let deps = [bounds.untyped(), snap_lines.untyped()];
        Property::computed(
            move || {
                let bounds = bounds.get();
                ToolVisualizationState {
                    rect_tool: RectTool {
                        x: Size::Pixels(bounds.top_left().x.into()),
                        y: Size::Pixels(bounds.top_left().y.into()),
                        width: Size::Pixels(bounds.width().into()),
                        height: Size::Pixels(bounds.height().into()),
                        stroke: Color::rgba(0.into(), 0.into(), 255.into(), 200.into()),
                        fill: Color::rgba(0.into(), 0.into(), 255.into(), 30.into()),
                    },
                    outline: Default::default(),
                    snap_lines: snap_lines.get(),
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
