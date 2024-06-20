use std::cell::RefCell;
use std::ops::ControlFlow;
use std::rc::Rc;

use model::action::orm::CreateComponent;
use pax_engine::api::*;
use pax_engine::math::Vector2;
use pax_engine::*;
use pax_manifest::TypeId;
use pax_std::primitives::Rectangle;
use pax_std::primitives::Text;

use crate::glass::SetEditingComponent;
use crate::math::coordinate_spaces::Glass;
use crate::math::AxisAlignedBox;
use crate::model;
use crate::model::action::ActionContext;
use crate::model::input::Dir;
use crate::model::input::InputEvent;
use crate::model::ToolBehaviour;
use math::Point2;

use super::SetLibraryState;

#[pax]
#[file("controls/file_and_component_picker/component_library_item.pax")]
pub struct ComponentLibraryItem {
    pub data: Property<ComponentLibraryItemData>,
}

#[pax]
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

impl ToolBehaviour for DropComponent {
    fn pointer_down(&mut self, _point: Point2<Glass>, _ctx: &mut ActionContext) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn pointer_move(&mut self, _point: Point2<Glass>, _ctx: &mut ActionContext) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn pointer_up(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
        let w_center = ctx.world_transform() * point;
        let (w, h) = self.bounds_pixels;
        let v = Vector2::new(w, h) / 2.0;
        let bounds = AxisAlignedBox::new(w_center + v, w_center - v);
        ctx.execute(CreateComponent {
            bounds,
            type_id: self.type_id.clone(),
            custom_props: vec![],
        })
        .unwrap();
        ctx.execute(SetLibraryState { open: false }).unwrap();
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

    fn visualize(&self, _glass: &mut crate::glass::Glass) {}
}

impl ComponentLibraryItem {
    pub fn on_double_click(&mut self, ctx: &NodeContext, _args: Event<DoubleClick>) {
        model::perform_action(SetEditingComponent(self.data.get().type_id.clone()), ctx);
    }
    pub fn on_down(&mut self, ctx: &NodeContext, _args: Event<MouseDown>) {
        model::with_action_context(ctx, |ctx| {
            let data = self.data.get();
            ctx.app_state
                .tool_behaviour
                .set(Some(Rc::new(RefCell::new(DropComponent {
                    type_id: data.type_id.clone(),
                    bounds_pixels: data.bounds_pixels,
                }))));
        });
    }
}
