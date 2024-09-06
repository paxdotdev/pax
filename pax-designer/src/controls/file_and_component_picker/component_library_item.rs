use std::cell::RefCell;
use std::ops::ControlFlow;
use std::rc::Rc;

use model::action::orm::CreateComponent;
use pax_engine::api::*;
use pax_engine::math::Vector2;
use pax_engine::node_layout::TransformAndBounds;
use pax_engine::*;
use pax_manifest::TypeId;
use pax_std::*;

use crate::controls::toolbar::SelectTool;
use crate::glass::SetEditingComponent;
use crate::glass::ToolVisualizationState;
use crate::math::coordinate_spaces::Glass;
use crate::math::AxisAlignedBox;
use crate::model;
use crate::model::action::Action;
use crate::model::action::ActionContext;
use crate::model::input::Dir;
use crate::model::input::InputEvent;
use crate::model::tools::SelectMode;
use crate::model::tools::SelectNodes;
use crate::model::GlassNode;
use crate::model::ToolBehavior;
use math::Point2;

use super::SetLibraryState;

#[pax]
#[engine_import_path("pax_engine")]
#[file("controls/file_and_component_picker/component_library_item.pax")]
pub struct ComponentLibraryItem {
    pub data: Property<ComponentLibraryItemData>,
}

#[pax]
#[engine_import_path("pax_engine")]
pub struct ComponentLibraryItemData {
    pub name: String,
    pub file_path: String,
    pub type_id: TypeId,
    pub bounds_pixels: (f64, f64),
}

struct DropComponent {
    type_id: TypeId,
    bounds_pixels: (f64, f64),
}

impl ToolBehavior for DropComponent {
    fn pointer_down(&mut self, _point: Point2<Glass>, _ctx: &mut ActionContext) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn pointer_move(&mut self, _point: Point2<Glass>, _ctx: &mut ActionContext) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn pointer_up(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
        // If over glass, drop this component, else start editing the component that was clicked
        // this is a very rough heuristic (glass position is x > 0)
        if point.x < 0.0 {
            if let Err(e) = SetEditingComponent(self.type_id.clone()).perform(ctx) {
                log::warn!("failed to set editing component: {e}");
            };
        } else {
            // TODO fill this in again later, for some reason gives (0, 0) (to/from pax value impl?)
            // let (w, h) = self.bounds_pixels;
            let (w, h) = (200.0, 200.0);
            let v = Vector2::new(w, h) / 2.0;
            let bounds = AxisAlignedBox::new(point + v, point - v);
            let root_parent = ctx.derived_state.open_container.get();
            let Ok(parent) = ctx.get_glass_node_by_global_id(&root_parent) else {
                log::warn!("couldn't find open container node");
                return ControlFlow::Break(());
            };
            let t = ctx.transaction("instantiating component");
            let _ = t.run(|| {
                let uid = CreateComponent {
                    parent_id: &parent.id,
                    parent_index: pax_manifest::TreeIndexPosition::Top,
                    node_layout: model::action::orm::NodeLayoutSettings::KeepScreenBounds {
                        node_transform_and_bounds: &TransformAndBounds {
                            transform: bounds.as_transform(),
                            bounds: (1.0, 1.0),
                        }
                        .as_pure_size(),
                        parent_transform_and_bounds: &parent.transform_and_bounds.get(),
                        node_decomposition_config: &Default::default(),
                    },
                    builder_extra_commands: None,
                    type_id: &self.type_id,
                }
                .perform(ctx)?;
                SetLibraryState { open: false }.perform(ctx)?;
                SelectNodes {
                    ids: &[uid.get_template_node_id()],
                    mode: SelectMode::DiscardOthers,
                }
                .perform(ctx)
            });
        }
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

    fn get_visual(&self) -> Property<crate::glass::ToolVisualizationState> {
        Property::new(ToolVisualizationState::default())
    }
}

impl ComponentLibraryItem {
    pub fn on_down(&mut self, ctx: &NodeContext, _args: Event<MouseDown>) {
        model::with_action_context(ctx, |ctx| {
            let data = self.data.get();
            ctx.app_state
                .tool_behavior
                .set(Some(Rc::new(RefCell::new(DropComponent {
                    type_id: data.type_id.clone(),
                    bounds_pixels: data.bounds_pixels,
                }))));
        });
    }
}
