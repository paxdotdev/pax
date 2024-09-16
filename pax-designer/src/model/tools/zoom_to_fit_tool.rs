use std::ops::ControlFlow;

use pax_engine::{api::Color, log, math::Point2, Property};
use pax_std::Size;

use crate::{
    glass::{RectTool, ToolVisualizationState},
    math::{coordinate_spaces::Glass, AxisAlignedBox},
    model::{
        action::{world::ZoomToFit, Action, ActionContext},
        input::{Dir, InputEvent},
        ToolBehavior,
    },
};

pub struct ZoomToFitTool {
    origin: Point2<Glass>,
    bounds: Property<AxisAlignedBox>,
}

impl ZoomToFitTool {
    pub fn new(point: Point2<Glass>) -> Self {
        Self {
            origin: point,
            bounds: Property::new(AxisAlignedBox::new(Point2::default(), Point2::default())),
        }
    }
}

impl ToolBehavior for ZoomToFitTool {
    fn pointer_down(&mut self, _point: Point2<Glass>, _ctx: &mut ActionContext) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn pointer_move(&mut self, point: Point2<Glass>, _ctx: &mut ActionContext) -> ControlFlow<()> {
        self.bounds.set(AxisAlignedBox::new(self.origin, point));
        ControlFlow::Continue(())
    }

    fn pointer_up(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
        self.pointer_move(point, ctx);
        let bounds = self.bounds.get();
        if bounds.width() < 50.0 || bounds.height() < 50.0 {
            return ControlFlow::Break(());
        }

        let glass_to_world = ctx.world_transform();
        if let Err(e) = (ZoomToFit {
            top_left: glass_to_world * bounds.top_left(),
            bottom_right: glass_to_world * bounds.bottom_right(),
        }
        .perform(ctx))
        {
            log::warn!("failed to zoom: {e}");
        };

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
                        stroke: Color::rgba(0.into(), 20.into(), 200.into(), 200.into()),
                        fill: Color::rgba(0.into(), 20.into(), 200.into(), 30.into()),
                    },
                    ..Default::default()
                }
            },
            &deps,
        )
    }

    fn finish(&mut self, _ctx: &mut ActionContext) -> anyhow::Result<()> {
        Ok(())
    }
}
