use anyhow::anyhow;
use pax_engine::api::Percent;
use pax_engine::{
    api::{borrow, borrow_mut, Color, Interpolatable},
    log,
    math::{Point2, Transform2, Vector2},
    pax_manifest::{TreeIndexPosition, UniqueTemplateNodeIdentifier},
    pax_runtime::TransformAndBounds,
    NodeInterface, NodeLocal, Slot,
};

use crate::{
    designer_node_type::{designer_behavior_extensions::IntentDefinition, DesignerNodeType},
    math::IntoDecompositionConfiguration,
    model::{
        action::{
            orm::{
                group_ungroup::{GroupNodes, GroupType},
                tree_movement::MoveNode,
                NodeLayoutSettings,
            },
            Action, ActionContext,
        },
        GlassNode, GlassNodeSnapshot,
    },
};

use super::{DesignerBehaviorExtensions, IntentState};

pub struct SlotComponentDesignerBehavior {
    pub edge_eval_vertical: Box<dyn Fn(&NodeInterface) -> bool>,
}

// Designer Behavior Extensions could be moved to userland at some point and be
// implemented directly on component instead of on this type (would also allow for
// different behaviors depending on component props)
impl DesignerBehaviorExtensions for SlotComponentDesignerBehavior {
    fn get_intents(&self, ctx: &mut ActionContext, component_node: &NodeInterface) -> IntentState {
        // TODO move this logic to make it available in MovingTool as well
        let mut search_space = vec![component_node.clone()];
        let mut slot_nodes_sorted = vec![];
        let curr_node_t_and_b = component_node.transform_and_bounds().get();
        let component_id = component_node.global_id().unwrap();
        while let Some(node) = search_space.pop() {
            if node.containing_component() != Some(component_node.clone())
                && component_node != &node
            {
                continue;
            }
            search_space.extend(node.children());
            let node_type = ctx.designer_node_type(&node.global_id().unwrap());
            if matches!(node_type, DesignerNodeType::Slot) {
                slot_nodes_sorted.push(node);
            }
        }

        if slot_nodes_sorted.is_empty() {
            return IntentState {
                intent_areas: vec![create_drop_into_intent(
                    component_id,
                    TransformAndBounds {
                        transform: Transform2::identity(),
                        bounds: curr_node_t_and_b.bounds,
                    },
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
                // ideally make a way to get node relative bounds directly from engine
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
        let default_dir = match (self.edge_eval_vertical)(component_node) {
            true => Vector2::y(),
            false => Vector2::x(),
        };
        create_all_drop_between_intents(&mut intent_areas, &slot_data, &component_id, default_dir);
        create_all_drop_into_intents(&mut intent_areas, &slot_data, &component_id);

        IntentState { intent_areas }
    }
}

fn create_all_drop_between_intents(
    intent_areas: &mut Vec<IntentDefinition>,
    slot_data: &[(usize, TransformAndBounds<NodeLocal>)],
    component_id: &UniqueTemplateNodeIdentifier,
    default_dir: Vector2<NodeLocal>,
) {
    for slots in slot_data.windows(2) {
        let (_, t_and_b_over) = slots[0];
        let (index_under, t_and_b_under) = slots[1];
        let line_transform = estimate_transform_between(t_and_b_under, t_and_b_over);
        intent_areas.push(create_drop_between_intent(
            component_id.clone(),
            line_transform,
            index_under,
        ));
    }

    // add first element
    let (first_ind, slot_t_and_b_first) = slot_data
        .first()
        .expect("should at least be one slot because of check above");
    let side_dir = intent_areas
        .first()
        .map(|b| -b.draw_area.decompose().2)
        .unwrap_or(-default_dir);
    let edge = find_most_aligned_edge(&slot_t_and_b_first.corners(), &side_dir);
    let first_segment_area = segment_to_transform_and_bounds(edge.0, edge.1);
    intent_areas.insert(
        0,
        create_drop_between_intent(component_id.clone(), first_segment_area, *first_ind),
    );

    // add last element
    let (last_ind, slot_t_and_b_last) = slot_data
        .last()
        .expect("should at least be one slot because of check above");
    let side_dir = intent_areas
        .last()
        .map(|b| b.draw_area.decompose().2)
        .unwrap_or(default_dir);
    let edge = find_most_aligned_edge(&slot_t_and_b_last.corners(), &side_dir);
    let last_segment_area = segment_to_transform_and_bounds(edge.0, edge.1);
    intent_areas.push(create_drop_between_intent(
        component_id.clone(),
        last_segment_area,
        *last_ind + 1,
    ));
}

struct SlotComponentDropBetweenAction {
    parent_component: UniqueTemplateNodeIdentifier,
    index: TreeIndexPosition,
    nodes_to_move: Vec<NodeInterface>,
}

impl Action for SlotComponentDropBetweenAction {
    fn perform(&self, ctx: &mut ActionContext) -> anyhow::Result<()> {
        for node in self.nodes_to_move.iter().rev() {
            MoveNode {
                node_id: &node.global_id().unwrap(),
                new_parent_uid: &self.parent_component,
                index: self.index.clone(),
                new_node_layout: Some(NodeLayoutSettings::<NodeLocal>::Fill),
            }
            .perform(ctx)?;
        }
        Ok(())
    }
}

fn create_drop_between_intent(
    parent_component: UniqueTemplateNodeIdentifier,
    draw_area: TransformAndBounds<NodeLocal>,
    index: usize,
) -> IntentDefinition {
    const INTENT_HEIGHT: f64 = 40.0;
    let (width, height) = draw_area.bounds;
    let hit_area = draw_area.transform
        * Transform2::<NodeLocal>::translate(Vector2::new(width / 2.0, height / 2.0))
        * Transform2::<NodeLocal>::scale_sep(Vector2::new(
            width - INTENT_HEIGHT * 1.5,
            INTENT_HEIGHT,
        ))
        * Transform2::<NodeLocal>::translate(Vector2::new(-0.5, -0.5));

    IntentDefinition {
        hit_area,
        draw_area: draw_area.as_transform(),
        fill: Color::TRANSPARENT,
        //thick green band for "black keys" between slots
        stroke: Some((
            2.5,
            Color::rgb(0.into(), Percent(90.0.into()).into(), 0.into()),
        )),
        intent_drop_behavior_factory: Box::new(move |selected_nodes| {
            Box::new({
                SlotComponentDropBetweenAction {
                    index: TreeIndexPosition::At(index),
                    parent_component: parent_component.to_owned(),
                    nodes_to_move: selected_nodes.to_owned(),
                }
            })
        }),
    }
}

fn create_all_drop_into_intents(
    intent_areas: &mut Vec<IntentDefinition>,
    slot_data: &[(usize, TransformAndBounds<NodeLocal>)],
    component_id: &UniqueTemplateNodeIdentifier,
) {
    for (index, t_and_b_into) in slot_data {
        intent_areas.push(create_drop_into_intent(
            component_id.clone(),
            *t_and_b_into,
            *index,
        ));
    }
}

fn create_drop_into_intent(
    parent_component: UniqueTemplateNodeIdentifier,
    draw_area: TransformAndBounds<NodeLocal>,
    index: usize,
) -> IntentDefinition {
    struct SlotComponentDropIntoAction {
        parent_component: UniqueTemplateNodeIdentifier,
        index: TreeIndexPosition,
        nodes_to_move: Vec<NodeInterface>,
    }

    impl Action for SlotComponentDropIntoAction {
        fn perform(&self, ctx: &mut ActionContext) -> anyhow::Result<()> {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            let orm = dt.get_orm_mut();
            let component_template_child_ids = orm
                .get_node_children(self.parent_component.clone())
                .map_err(|e| anyhow!("failed to get children {e}"))?;
            let Some(moving_into) = component_template_child_ids
                .get(self.index.get_index(component_template_child_ids.len()))
            else {
                // component might be embty, if so just place anywhere
                drop(dt);
                return SlotComponentDropBetweenAction {
                    parent_component: self.parent_component.clone(),
                    index: TreeIndexPosition::Bottom,
                    nodes_to_move: self.nodes_to_move.clone(),
                }
                .perform(ctx);
            };

            if self.nodes_to_move.len() == 1
                && self
                    .nodes_to_move
                    .first()
                    .and_then(|n| n.global_id())
                    .as_ref()
                    == Some(moving_into)
            {
                return Err(anyhow!("Can't move into self"));
            }

            let node_snapshots: Vec<GlassNodeSnapshot> = self
                .nodes_to_move
                .iter()
                .map(|n| (&GlassNode::new(&n, &ctx.glass_transform())).into())
                .collect();
            let into_node = ctx.get_glass_node_by_global_id(moving_into)?;
            let into_t_and_b = into_node.parent_transform_and_bounds.get();

            // used by below actions
            drop(dt);

            let into_metadata = {
                let into_type = ctx.designer_node_type(moving_into);
                let dt = borrow_mut!(ctx.engine_context.designtime);
                into_type.metadata(dt.get_orm())
            };

            // TODO how to handle slot containers? and scroller for that matter?
            let move_into = if into_metadata.is_container {
                moving_into.clone()
            } else {
                let group_id = GroupNodes {
                    group_type: GroupType::Group,
                    nodes: &[moving_into.clone()],
                    group_bounds: into_t_and_b,
                    group_parent: &self.parent_component,
                    group_location_index: self.index.clone(),
                }
                .perform(ctx)?;
                group_id
            };

            for node in node_snapshots {
                MoveNode {
                    node_id: &node.id,
                    new_parent_uid: &move_into,
                    index: TreeIndexPosition::Top,
                    new_node_layout: Some(NodeLayoutSettings::KeepScreenBounds {
                        node_transform_and_bounds: &node.transform_and_bounds,
                        node_decomposition_config: &node
                            .layout_properties
                            .into_decomposition_config(),
                        parent_transform_and_bounds: &into_t_and_b,
                    }),
                }
                .perform(ctx)?;
            }
            Ok(())
        }
    }

    const DROP_INTO_PADDING: f64 = 20.0;
    let (width, height) = draw_area.bounds;
    let hit_area = draw_area.transform
        * Transform2::<NodeLocal>::translate(Vector2::new(width / 2.0, height / 2.0))
        * Transform2::<NodeLocal>::scale_sep(Vector2::new(
            width - 2.0 * DROP_INTO_PADDING,
            height - 2.0 * DROP_INTO_PADDING,
        ))
        * Transform2::<NodeLocal>::translate(Vector2::new(-0.5, -0.5));

    IntentDefinition {
        hit_area,
        draw_area: draw_area.as_transform(),
        //magenta translucent overlay for "white key" slots
        fill: Color::rgba(
            Percent(90.0.into()).into(),
            0.into(),
            Percent(90.0.into()).into(),
            Percent(10.0.into()).into(),
        ),
        stroke: Some((
            2.0,
            Color::rgb(
                Percent(90.0.into()).into(),
                0.into(),
                Percent(90.0.into()).into(),
            ),
        )),
        intent_drop_behavior_factory: Box::new(move |selected_nodes| {
            Box::new({
                SlotComponentDropIntoAction {
                    index: TreeIndexPosition::At(index),
                    parent_component: parent_component.to_owned(),
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
    let centroid_under = centroid(&corners_under);
    let centroid_over = centroid(&corners_over);

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

fn centroid(corners: &[Point2<NodeLocal>; 4]) -> Point2<NodeLocal> {
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
    direction: &Vector2<NodeLocal>,
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
            let similarity = direction.cross(dir.normalize());
            ((*start, *dir), similarity)
        })
        .max_by(|(_, a), (_, b)| a.total_cmp(&b))
        .map(|(val, _)| val)
        .unwrap()
}
