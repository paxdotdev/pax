use pax_engine::{api::borrow, log, math::Point2, NodeInterface, Property};

use crate::{
    glass::intent::IntentDef,
    math::coordinate_spaces::Glass,
    model::{
        action::{Action, ActionContext, RaycastMode},
        GlassNode,
    },
};

pub struct DropIntentHandler {
    drop_intent_curent_action_factory: Option<Box<dyn Fn(&[NodeInterface]) -> Box<dyn Action>>>,
    moving_nodes: Vec<NodeInterface>,
    intent_areas: Property<Vec<IntentDef>>,
}

impl DropIntentHandler {
    pub fn new(ignore_nodes: &[NodeInterface]) -> Self {
        Self {
            moving_nodes: ignore_nodes.to_owned(),
            drop_intent_curent_action_factory: None,
            intent_areas: Property::default(),
        }
    }

    pub fn get_intent_areas_prop(&self) -> Property<Vec<IntentDef>> {
        self.intent_areas.clone()
    }

    pub fn update(&mut self, ctx: &mut ActionContext, point: Point2<Glass>) {
        self.drop_intent_curent_action_factory = None;
        let Some(node) = ctx.raycast_glass(
            point,
            // TODO hit things like stackers even with no items in them?
            RaycastMode::Top,
            &self.moving_nodes,
        ) else {
            self.intent_areas.set(vec![]);
            return;
        };
        // TODO PERF if hit same object as before, skip re-running get_intents, and just do the rest again (cached)
        let node_type = ctx.designer_node_type(&node.global_id().unwrap());
        let node_type_metadata =
            node_type.metadata(borrow!(ctx.engine_context.designtime).get_orm());
        let intents = node_type_metadata
            .designer_behavior_extensions()
            .get_intents(ctx, &node);
        let to_glass_transform = ctx.glass_transform();
        let node_transform = GlassNode::new(&node, &to_glass_transform)
            .transform_and_bounds
            .get();
        self.intent_areas.set(
            intents
                .intent_areas
                .into_iter()
                .find_map(|intent| {
                    let transform = node_transform.transform * intent.draw_area;
                    let hit_transform = node_transform.transform * intent.hit_area;
                    hit_transform.contains_point(point).then(|| {
                        self.drop_intent_curent_action_factory =
                            Some(intent.intent_drop_behavior_factory);
                        IntentDef::new(transform, intent.fill.clone(), intent.stroke.clone())
                    })
                })
                .as_slice()
                .to_vec(),
        );
    }

    pub fn handle_drop(&mut self, ctx: &mut ActionContext) -> bool {
        if let Some(factory) = &self.drop_intent_curent_action_factory {
            let action = factory(&self.moving_nodes);
            if let Err(e) = action.perform(ctx) {
                log::warn!("failed to perform intent movement: {e}");
            };
        }
        self.drop_intent_curent_action_factory.is_some()
    }
}
