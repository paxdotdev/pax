use crate::{
    math::{
        self,
        coordinate_spaces::{self, World},
    },
    DESIGNER_GLASS_ID,
};

use super::Action;
use anyhow::anyhow;
use pax_engine::math::{Transform2, Vector2};

pub struct InitWorldTransform;

impl Action for InitWorldTransform {
    fn perform(&self, ctx: &mut super::ActionContext) -> anyhow::Result<()> {
        let stage = ctx.app_state.stage.get();
        let glass_node = ctx
            .engine_context
            .get_nodes_by_id(DESIGNER_GLASS_ID)
            .into_iter()
            .next()
            .ok_or_else(|| {
                anyhow!("couldn't hook up glass to world transform: couldn't find node in engine")
            })?;
        let (w, h) = glass_node.transform_and_bounds().get().bounds;
        ctx.app_state.glass_to_world_transform.set(
            Transform2::<World>::translate(Vector2::new(
                stage.stage_width as f64 / 2.0,
                stage.stage_height as f64 / 2.0,
            )) * Transform2::scale(1.4)
                * Transform2::<coordinate_spaces::Glass>::translate(-Vector2::new(
                    w / 2.0,
                    h / 2.0,
                )),
        );
        Ok(())
    }
}
