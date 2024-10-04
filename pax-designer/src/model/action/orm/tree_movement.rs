use anyhow::{anyhow, Result};
use pax_engine::{
    api::{borrow, borrow_mut},
    log,
    math::{Generic, Space},
    pax_manifest::{PaxType, TreeIndexPosition, UniqueTemplateNodeIdentifier},
};

use crate::model::action::{Action, ActionContext};

use super::{NodeLayoutSettings, SetNodeLayout};

pub struct MoveNode<'a, S = Generic> {
    pub node_id: &'a UniqueTemplateNodeIdentifier,
    pub new_parent_uid: &'a UniqueTemplateNodeIdentifier,
    pub index: TreeIndexPosition,
    pub new_node_layout: Option<NodeLayoutSettings<'a, S>>,
}

impl<S: Space> Action for MoveNode<'_, S> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let node = ctx
            .engine_context
            .get_nodes_by_global_id(self.node_id.clone())
            .into_iter()
            .next();
        let new_parent = ctx
            .engine_context
            .get_nodes_by_global_id(self.new_parent_uid.clone())
            .into_iter()
            .next();

        // some move operations take place before the nodes exist, so don't error if this check
        // can't be made. could potentially check this using the ORM instead at some point
        if let (Some(node), Some(parent)) = (&node, &new_parent) {
            if parent.is_descendant_of(&node) {
                return Err(anyhow!("can't move parent into a child"));
            }
            if parent == node {
                return Err(anyhow!("can't move node into itself"));
            }
        }

        if let Some(parent) = new_parent {
            if Some(&parent)
                != ctx
                    .engine_context
                    .get_userland_root_expanded_node()
                    .as_ref()
            {
                let metadata = ctx
                    .designer_node_type(&parent.global_id().unwrap())
                    .metadata(borrow!(ctx.engine_context.designtime).get_orm());
                if !metadata.is_container {
                    return Err(anyhow!(
                        "can't move node into {} - not a container",
                        metadata.name
                    ));
                }
            };
        }

        if let Some(node_layout) = &self.new_node_layout {
            SetNodeLayout {
                id: self.node_id,
                node_layout,
            }
            .perform(ctx)?;
        }

        {
            let mut new_location = ctx.location(self.new_parent_uid, &self.index);
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            let old_location = dt
                .get_orm()
                .get_node_location(self.node_id)
                .ok_or_else(|| anyhow!("failed to get old node location"))?;
            if old_location.tree_location == new_location.tree_location {
                if let TreeIndexPosition::At(old_index) = old_location.index {
                    let index = match new_location.index {
                        TreeIndexPosition::Top => 0,
                        TreeIndexPosition::Bottom => usize::MAX,
                        TreeIndexPosition::At(i) => i,
                    };
                    if old_index < index {
                        new_location.index = TreeIndexPosition::At(index.saturating_sub(1));
                    }
                }
            }
            let _undo_id = dt
                .get_orm_mut()
                .move_node(self.node_id.clone(), new_location.clone())
                .map_err(|e| anyhow!("couldn't move child node {:?}", e))?;
        }

        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum RelativeMove {
    Top,
    BumpUp,
    BumpDown,
    Bottom,
}

pub struct RelativeMoveSelected {
    pub relative_move: RelativeMove,
}

impl Action for RelativeMoveSelected {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let selected = ctx.selected_nodes();
        let movements: Vec<_> = {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            selected
                .iter()
                .map(|(node_id, _)| {
                    let mut location = dt.get_orm_mut().get_node_location(node_id).unwrap();
                    let siblings = dt
                        .get_orm()
                        .get_siblings(&node_id)
                        .ok_or_else(|| anyhow!("no parent children count"))?;

                    let index = match location.index {
                        TreeIndexPosition::Top => 0,
                        TreeIndexPosition::Bottom => siblings.len() - 1,
                        TreeIndexPosition::At(i) => i,
                    };
                    let new_index = match self.relative_move {
                        RelativeMove::Top => 0,
                        RelativeMove::BumpUp => {
                            let mut index = index.saturating_sub(1);
                            loop {
                                let Some(uid) = siblings.get(index) else {
                                    break;
                                };
                                let Some(node) =
                                    dt.get_orm_mut().get_node_builder(uid.clone(), false)
                                else {
                                    break;
                                };
                                if node.get_type_id().get_pax_type() != &PaxType::Comment {
                                    break;
                                }
                                let Some(new_ind) = index.checked_sub(1) else {
                                    break;
                                };
                                index = new_ind;
                            }
                            index
                        }
                        RelativeMove::BumpDown => {
                            let mut index = index.saturating_add(1);
                            loop {
                                let Some(uid) = siblings.get(index) else {
                                    break;
                                };
                                let Some(node) =
                                    dt.get_orm_mut().get_node_builder(uid.clone(), false)
                                else {
                                    break;
                                };
                                if node.get_type_id().get_pax_type() != &PaxType::Comment {
                                    break;
                                }
                                let Some(new_ind) = index.checked_add(1) else {
                                    break;
                                };
                                index = new_ind;
                            }
                            index
                        }
                        RelativeMove::Bottom => siblings.len(),
                    };
                    location.index = TreeIndexPosition::At(new_index);
                    Ok((node_id, location))
                })
                .collect::<Result<Vec<_>>>()?
        };

        let t = ctx.transaction("relative node movements");

        let mut dt = borrow_mut!(ctx.engine_context.designtime);
        t.run(|| {
            for (node_id, location) in movements {
                dt.get_orm_mut()
                    .move_node(node_id.clone(), location.clone())
                    .map(|_| ())
                    .map_err(|e| anyhow!("couldn't move node: {e}"))?;
            }
            Ok(())
        })
    }
}
