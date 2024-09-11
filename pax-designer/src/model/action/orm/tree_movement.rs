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
    pub node_layout: NodeLayoutSettings<'a, S>,
}

impl<S: Space> Action for MoveNode<'_, S> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        SetNodeLayout {
            id: self.node_id,
            node_layout: &self.node_layout,
        }
        .perform(ctx)?;
        let parent_location = ctx.location(self.new_parent_uid, &self.index);
        {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            let _undo_id = dt
                .get_orm_mut()
                .move_node(self.node_id.clone(), parent_location.clone())
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
                                let Some(node) = dt.get_orm_mut().get_node(uid.clone(), false)
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
                                let Some(node) = dt.get_orm_mut().get_node(uid.clone(), false)
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
