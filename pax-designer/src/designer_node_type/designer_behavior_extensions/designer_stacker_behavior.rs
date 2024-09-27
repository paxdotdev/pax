use pax_engine::{
    api::{Color, Interpolatable},
    log,
    math::{Point2, Transform2, Vector2},
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
    const INTENT_HEIGHT: f64 = 15.0;
    for slots in slot_data.windows(2) {
        let (_, t_and_b_over) = slots[0];
        let (index_under, t_and_b_under) = slots[1];
        let line_transform = estimate_transform_between(t_and_b_under, t_and_b_over);
        // Create the intent area transform
        let (width, _) = line_transform.bounds;
        let line_width = width - INTENT_HEIGHT;
        let intent_area = line_transform.transform
            * Transform2::<NodeLocal>::translate(Vector2::new(width / 2.0, 0.0))
            * Transform2::<NodeLocal>::scale_sep(Vector2::new(line_width, INTENT_HEIGHT))
            * Transform2::<NodeLocal>::translate(Vector2::new(-0.5, -0.5));

        intent_areas.push(create_drop_between_intent(
            stacker_id.clone(),
            intent_area,
            index_under,
        ));
    }

    // add first element
    let (first_ind, mut slot_t_and_b) = slot_data
        .first()
        .expect("should at least be one slot because of check above");
    if let Some(between_first) = intent_areas.first() {
        let (_, _, v) = between_first.area.decompose();
        let edge = find_most_aligned_edge(&slot_t_and_b.corners(), &-v);
        slot_t_and_b = segment_to_transform_and_bounds(edge.0, edge.1);
    }
    let (width, _) = slot_t_and_b.bounds;
    intent_areas.insert(
        0,
        create_drop_between_intent(
            stacker_id.clone(),
            slot_t_and_b.transform
                * Transform2::<NodeLocal>::translate(Vector2::new(width / 2.0, 0.0))
                * Transform2::<NodeLocal>::scale_sep(Vector2::new(
                    width - INTENT_HEIGHT * 1.5,
                    INTENT_HEIGHT,
                ))
                * Transform2::<NodeLocal>::translate(Vector2::new(-0.5, 0.0)),
            *first_ind,
        ),
    );

    // add last element
    let (last_ind, mut slot_t_and_b) = slot_data
        .last()
        .expect("should at least be one slot because of check above");
    if let Some(between_last) = intent_areas.first() {
        let (_, _, v) = between_last.area.decompose();
        let edge = find_most_aligned_edge(&slot_t_and_b.corners(), &v);
        slot_t_and_b = segment_to_transform_and_bounds(edge.0, edge.1);
    }
    let (width, _) = slot_t_and_b.bounds;
    intent_areas.push(create_drop_between_intent(
        stacker_id.clone(),
        slot_t_and_b.transform
            * Transform2::<NodeLocal>::translate(Vector2::new(width / 2.0, 0.0))
            * Transform2::<NodeLocal>::scale_sep(Vector2::new(
                width - INTENT_HEIGHT * 1.5,
                INTENT_HEIGHT,
            ))
            * Transform2::<NodeLocal>::translate(Vector2::new(-0.5, 0.0)),
        *last_ind,
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
        stroke: None,
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
        stroke: None,
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

fn estimate_transform_between(
    t_and_b_under: TransformAndBounds<NodeLocal>,
    t_and_b_over: TransformAndBounds<NodeLocal>,
) -> TransformAndBounds<NodeLocal> {
    // Extract corners of both rectangles
    let corners_under = t_and_b_under.corners();
    let corners_over = t_and_b_over.corners();

    // Calculate centroids
    let centroid_under = calculate_centroid(&corners_under);
    let centroid_over = calculate_centroid(&corners_over);

    // Vector from under to over centroid
    let centroid_vector = centroid_over - centroid_under;

    // Find the edges most parallel to the centroid vector
    let edge_under = find_most_aligned_edge(&corners_under, &centroid_vector);
    let mut edge_over = find_most_aligned_edge(&corners_over, &(-centroid_vector));

    // if segment vectors not pointing in same dir,
    // re-orient one
    if edge_over.1 * edge_under.1 < 0.0 {
        edge_over = (edge_over.0 + edge_over.1, -edge_over.1);
    }

    // Calculate the line
    let start = edge_under.0.midpoint_towards(edge_over.0);
    let direction = (edge_under.1 + edge_over.1) / 2.0;
    segment_to_transform_and_bounds(start, direction)
}

fn segment_to_transform_and_bounds(
    point: Point2<NodeLocal>,
    dir: Vector2<NodeLocal>,
) -> TransformAndBounds<NodeLocal> {
    let dir_n = dir.normalize();
    let normal = dir_n.normal();

    // Create a transform that maps (0,0)-(1,0) to the line, spanning the entire space
    let transform = Transform2::new([dir_n.x, dir_n.y, normal.x, normal.y, point.x, point.y]);

    TransformAndBounds {
        transform,
        bounds: (dir.length(), 1.0),
    }
}

fn calculate_centroid(corners: &[Point2<NodeLocal>; 4]) -> Point2<NodeLocal> {
    (corners
        .iter()
        .map(|v| v.to_vector())
        .reduce(|a, b| a + b)
        .unwrap()
        / 4.0)
        .to_point()
}

fn find_most_aligned_edge(
    corners: &[Point2<NodeLocal>; 4],
    reference: &Vector2<NodeLocal>,
) -> (Point2<NodeLocal>, Vector2<NodeLocal>) {
    let edges = [
        (corners[0], corners[3] - corners[0]),
        (corners[1], corners[0] - corners[1]),
        (corners[2], corners[1] - corners[2]),
        (corners[3], corners[2] - corners[3]),
    ];
    edges
        .iter()
        .map(|(start, dir)| {
            let similarity = reference.cross(dir.normalize());
            ((*start, *dir), similarity)
        })
        .max_by(|(_, a), (_, b)| a.total_cmp(&b))
        .map(|(val, _)| val)
        .unwrap()
}
