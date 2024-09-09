use std::{cell::RefCell, ops::ControlFlow, rc::Rc};

use anyhow::anyhow;
use pax_engine::api::borrow_mut;
use pax_engine::pax_manifest::UniqueTemplateNodeIdentifier;
use pax_engine::{log, math::Point2, Property};
use pax_std::core::text::Text;

use crate::model::action::tool::SetToolBehaviour;
use crate::model::input::{InputEvent, ModifierKey};
use crate::{
    math::coordinate_spaces::Glass,
    model::{
        action::{Action, ActionContext, RaycastMode},
        ToolBehavior,
    },
};

pub struct TextEdit {
    pub uid: UniqueTemplateNodeIdentifier,
}

impl Action for TextEdit {
    fn perform(&self, ctx: &mut ActionContext) -> anyhow::Result<()> {
        let tool: Option<Rc<RefCell<dyn ToolBehavior>>> = Some(Rc::new(RefCell::new(
            TextEditTool::new(ctx, self.uid.clone())
                .expect("should only edit text with text editing tool"),
        )));
        SetToolBehaviour(tool).perform(ctx)?;
        Ok(())
    }
}

pub struct TextEditTool {
    uid: UniqueTemplateNodeIdentifier,
    text_binding: Property<String>,
}

impl TextEditTool {
    pub fn new(ctx: &mut ActionContext, uid: UniqueTemplateNodeIdentifier) -> Result<Self, String> {
        let mut dt = borrow_mut!(ctx.engine_context.designtime);
        let import_path = dt
            .get_orm_mut()
            .get_node(
                uid.clone(),
                ctx.app_state
                    .modifiers
                    .get()
                    .contains(&ModifierKey::Control),
            )
            .expect("node exists")
            .get_type_id()
            .import_path();

        let text_binding = Property::default();
        match import_path.as_ref().map(|v| v.as_str()) {
            Some("pax_std::core::text::Text") => {
                let node = ctx.get_glass_node_by_global_id(&uid).unwrap();

                node.raw_node_interface.with_properties(|text: &mut Text| {
                    text.editable.replace_with(Property::new(true));
                    let text = text.text.clone();
                    let deps = [text.untyped()];
                    text_binding.replace_with(Property::computed(move || text.get(), &deps));
                });
            }
            _ => return Err("can't edit non-text node".to_owned()),
        }
        Ok(Self { uid, text_binding })
    }
}

impl ToolBehavior for TextEditTool {
    fn pointer_down(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
        if let Some(hit) = ctx.raycast_glass(point, RaycastMode::Top, &[]) {
            let node_id = hit.global_id().unwrap().get_template_node_id();
            if node_id == self.uid.get_template_node_id() {
                return ControlFlow::Continue(());
            }
        }
        if let Err(e) = self.finish(ctx) {
            log::warn!("failed to save text change: {e}");
        };
        ControlFlow::Break(())
    }

    fn pointer_move(&mut self, _point: Point2<Glass>, _ctx: &mut ActionContext) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn pointer_up(&mut self, _point: Point2<Glass>, _ctx: &mut ActionContext) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn finish(&mut self, ctx: &mut ActionContext) -> anyhow::Result<()> {
        let node = ctx.get_glass_node_by_global_id(&self.uid)?;
        node.raw_node_interface.with_properties(|text: &mut Text| {
            text.editable.replace_with(Property::new(false));
        });

        let t = ctx.transaction("text edit");

        t.run(|| {
            // commit text changes
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            if let Some(mut builder) = dt.get_orm_mut().get_node(
                self.uid.clone(),
                ctx.app_state
                    .modifiers
                    .get()
                    .contains(&ModifierKey::Control),
            ) {
                builder.set_typed_property("text", self.text_binding.get())?;
                builder
                    .save()
                    .map_err(|e| anyhow!("couldn't save text: {e}"))?;
            }
            Ok(())
        })
    }

    fn keyboard(
        &mut self,
        _event: crate::model::input::InputEvent,
        _dir: crate::model::input::Dir,
        _ctx: &mut ActionContext,
    ) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn get_visual(&self) -> Property<super::ToolVisualizationState> {
        Property::new(super::ToolVisualizationState {
            event_blocker_active: false,
            ..Default::default()
        })
    }
}
