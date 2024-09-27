use pax_engine::{
    api::{Color, Interpolatable},
    log,
    math::{Transform2, Vector2},
    pax_manifest::{TreeIndexPosition, UniqueTemplateNodeIdentifier},
    pax_runtime::TransformAndBounds,
    NodeInterface, NodeLocal, Slot,
};

use crate::{
    designer_node_type::{designer_behavior_extensions::IntentDefinition, DesignerNodeType},
    model::{
        action::{
            orm::{tree_movement::MoveNode, NodeLayoutSettings},
            Action, ActionContext,
        },
        GlassNode,
    },
};

use super::{DesignerComponentBehaviorExtensions, IntentState};

pub struct StackerDesignerBehavior;

// Designer Behavior Extensions could be moved to userland at some point and be
// implemented directly on stacker instead of on this type (would also allow for
// different behaviors depending on stacker props)
impl DesignerComponentBehaviorExtensions for StackerDesignerBehavior {
    fn get_intents(&self, ctx: &mut ActionContext, node: &NodeInterface) -> IntentState {
        // TODO move this logic to make it available in MovingTool as well
        let mut search_space = vec![node.clone()];
        let mut slot_nodes_sorted = vec![];
        let curr_node_t_and_b = node.transform_and_bounds().get();
        let stacker_id = node.global_id().unwrap();
        while let Some(node) = search_space.pop() {
            search_space.extend(node.children());
            let node_type = ctx.designer_node_type(&node.global_id().unwrap());
            if matches!(node_type, DesignerNodeType::Slot) {
                slot_nodes_sorted.push(node);
            }
        }

        if slot_nodes_sorted.is_empty() {
            return IntentState {
                intent_areas: vec![create_drop_into_intent(
                    stacker_id,
                    Transform2::identity(),
                    0,
                )],
            };
        }

        slot_nodes_sorted.sort_by_key(|n| {
            n.with_properties(|properties: &mut Slot| properties.index.get().to_int())
        });
        let slot_data: Vec<_> = slot_nodes_sorted
            .into_iter()
            .enumerate()
            .map(|(i, n)| {
                let slot_node_t_and_b = n.transform_and_bounds().get();
                // ideally make a way to get node relative bounds directy from engine
                let parent_relative_slot_t_and_b = TransformAndBounds {
                    transform: curr_node_t_and_b.transform,
                    bounds: (1.0, 1.0),
                }
                .inverse()
                    * slot_node_t_and_b;
                (i, parent_relative_slot_t_and_b)
            })
            .collect();

        let mut intent_areas = vec![];
        create_all_drop_between_intents(&mut intent_areas, &slot_data, &stacker_id);
        create_all_drop_into_intents(&mut intent_areas, &slot_data, &stacker_id);

        IntentState { intent_areas }
    }
}

fn create_all_drop_between_intents(
    intent_areas: &mut Vec<IntentDefinition>,
    slot_data: &[(usize, TransformAndBounds<NodeLocal>)],
    stacker_id: &UniqueTemplateNodeIdentifier,
) {
    const DROP_BETWEEN_INTENT_HEIGHT: f64 = 15.0;
    let (first_ind, slot_t_and_b) = slot_data
        .first()
        .expect("should at least be one slot because of check above");
    let (width, _) = slot_t_and_b.bounds;
    intent_areas.push(create_drop_between_intent(
        stacker_id.clone(),
        slot_t_and_b.transform
            * Transform2::<NodeLocal>::translate(Vector2::new(width / 2.0, 0.0))
            * Transform2::<NodeLocal>::scale_sep(Vector2::new(
                width - DROP_BETWEEN_INTENT_HEIGHT * 1.5,
                DROP_BETWEEN_INTENT_HEIGHT,
            ))
            * Transform2::<NodeLocal>::translate(Vector2::new(-0.5, 0.0)),
        *first_ind,
    ));
    for slots in slot_data.windows(2) {
        let (_, t_and_b_over) = slots[0];
        let (index_under, t_and_b_under) = slots[1];
        let t_and_b_between = t_and_b_over.interpolate(&t_and_b_under, 0.5);
        let (width, height) = t_and_b_between.bounds;
        let intent_area = t_and_b_between.transform
            * Transform2::<NodeLocal>::translate(Vector2::new(width / 2.0, height / 2.0))
            * Transform2::<NodeLocal>::scale_sep(Vector2::new(
                width - DROP_BETWEEN_INTENT_HEIGHT * 1.5,
                DROP_BETWEEN_INTENT_HEIGHT,
            ))
            * Transform2::<NodeLocal>::translate(Vector2::new(-0.5, -0.5));
        intent_areas.push(create_drop_between_intent(
            stacker_id.clone(),
            intent_area,
            index_under,
        ));
    }
    let (last_ind, slot_t_and_b) = slot_data
        .last()
        .expect("should at least be one slot because of check above");
    let (width, height) = slot_t_and_b.bounds;
    intent_areas.push(create_drop_between_intent(
        stacker_id.clone(),
        slot_t_and_b.transform
            * Transform2::<NodeLocal>::translate(Vector2::new(width / 2.0, height))
            * Transform2::<NodeLocal>::scale_sep(Vector2::new(
                width - DROP_BETWEEN_INTENT_HEIGHT * 1.5,
                DROP_BETWEEN_INTENT_HEIGHT,
            ))
            * Transform2::<NodeLocal>::translate(Vector2::new(-0.5, -1.0)),
        *last_ind + 1,
    ));
}

fn create_drop_between_intent(
    parent_stacker: UniqueTemplateNodeIdentifier,
    transform: Transform2<NodeLocal>,
    index: usize,
) -> IntentDefinition {
    struct StackerDropBetweenAction {
        parent_stacker: UniqueTemplateNodeIdentifier,
        index: TreeIndexPosition,
        nodes_to_move: Vec<NodeInterface>,
    }

    impl Action for StackerDropBetweenAction {
        fn perform(&self, ctx: &mut ActionContext) -> anyhow::Result<()> {
            // TODO handle multiple node moving by grouping them before moving into stacker.
            let node = self.nodes_to_move.first().unwrap();
            MoveNode {
                node_id: &node.global_id().unwrap(),
                new_parent_uid: &self.parent_stacker,
                index: self.index.clone(),
                node_layout: NodeLayoutSettings::<NodeLocal>::Fill,
            }
            .perform(ctx)?;
            Ok(())
        }
    }

    IntentDefinition {
        area: transform,
        fill: Color::rgba(0.into(), 0.into(), 0.into(), 70.into()),
        stroke: Color::rgba(50.into(), 50.into(), 50.into(), 150.into()),
        intent_drop_behavior_factory: Box::new(move |selected_nodes| {
            Box::new({
                StackerDropBetweenAction {
                    index: TreeIndexPosition::At(index),
                    parent_stacker: parent_stacker.to_owned(),
                    nodes_to_move: selected_nodes.to_owned(),
                }
            })
        }),
    }
}

fn create_all_drop_into_intents(
    intent_areas: &mut Vec<IntentDefinition>,
    slot_data: &[(usize, TransformAndBounds<NodeLocal>)],
    stacker_id: &UniqueTemplateNodeIdentifier,
) {
    const DROP_INTO_PADDING: f64 = 15.0;
    for (index, t_and_b_into) in slot_data {
        let (width, height) = t_and_b_into.bounds;
        let intent_area = t_and_b_into.transform
            * Transform2::<NodeLocal>::translate(Vector2::new(width / 2.0, height / 2.0))
            * Transform2::<NodeLocal>::scale_sep(Vector2::new(
                width - 2.0 * DROP_INTO_PADDING,
                height - 2.0 * DROP_INTO_PADDING,
            ))
            * Transform2::<NodeLocal>::translate(Vector2::new(-0.5, -0.5));
        intent_areas.push(create_drop_into_intent(
            stacker_id.clone(),
            intent_area,
            *index,
        ));
    }
}

fn create_drop_into_intent(
    parent_stacker: UniqueTemplateNodeIdentifier,
    transform: Transform2<NodeLocal>,
    index: usize,
) -> IntentDefinition {
    struct StackerDropIntoAction {
        parent_stacker: UniqueTemplateNodeIdentifier,
        index: TreeIndexPosition,
        nodes_to_move: Vec<NodeInterface>,
    }

    impl Action for StackerDropIntoAction {
        fn perform(&self, ctx: &mut ActionContext) -> anyhow::Result<()> {
            let node = self.nodes_to_move.first().unwrap();
            // TODO find container at index, if a container, add to it, otherwise
            // group and add. (make this a single operation "add to or group"?)
            // MoveNode {
            //     node_id: &node.global_id().unwrap(),
            //     new_parent_uid: &self.parent_stacker,
            //     index: self.index.clone(),
            //     node_layout: NodeLayoutSettings::<NodeLocal>::Fill,
            // }
            // .perform(ctx)?;
            Ok(())
        }
    }

    IntentDefinition {
        area: transform,
        fill: Color::rgba(255.into(), 255.into(), 255.into(), 40.into()),
        stroke: Color::rgba(200.into(), 200.into(), 200.into(), 150.into()),
        intent_drop_behavior_factory: Box::new(move |selected_nodes| {
            Box::new({
                StackerDropIntoAction {
                    index: TreeIndexPosition::At(index),
                    parent_stacker: parent_stacker.to_owned(),
                    nodes_to_move: selected_nodes.to_owned(),
                }
            })
        }),
    }
}
