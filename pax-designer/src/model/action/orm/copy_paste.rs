use crate::model::action::orm::DeleteSelected;
use crate::model::action::world::SelectMode;

use crate::model::action::world::SelectNodes;

use pax_engine::api::borrow_mut;

use pax_engine::pax_manifest::TreeIndexPosition;

use anyhow::anyhow;

use pax_engine::api::borrow;

use anyhow::Result;

use super::super::ActionContext;

use pax_designtime::orm::SubTrees;

use super::super::Action;

use pax_engine::pax_manifest::TemplateNodeId;

pub struct Copy<'a> {
    pub ids: &'a [TemplateNodeId],
}

impl Action<SubTrees> for Copy<'_> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<SubTrees> {
        let comp_id = ctx.app_state.selected_component_id.get();
        let dt = borrow!(ctx.engine_context.designtime);
        let subtree = dt
            .get_orm()
            .copy_subtrees(&comp_id, &self.ids)
            .ok_or_else(|| anyhow!("couldn't copy"))?;
        Ok(subtree)
    }
}

pub struct Paste<'a> {
    pub subtrees: &'a SubTrees,
}

impl Action<Vec<TemplateNodeId>> for Paste<'_> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<Vec<TemplateNodeId>> {
        let parent = ctx.derived_state.open_containers.get()[0].clone();
        let loc = ctx.location(&parent, &TreeIndexPosition::Top);
        let t = ctx.transaction("pasting object");
        let mut dt = borrow_mut!(ctx.engine_context.designtime);
        let ids = t.run(|| {
            let ids = dt
                .get_orm_mut()
                .paste_subtrees(loc, self.subtrees.clone())
                .map_err(|e| anyhow!("failed to paste: {e}"))?;
            SelectNodes {
                ids: &ids,
                mode: SelectMode::DiscardOthers,
            }
            .perform(ctx)?;
            Ok(ids)
        });
        ids
    }
}

pub struct CopySelected;
impl Action for CopySelected {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let ids = ctx.app_state.selected_template_node_ids.get();
        let subtrees = Copy { ids: &ids }.perform(ctx)?;
        // TODO make this copy/paste to actual clipboard
        ctx.app_state.clip_board.set(subtrees);
        Ok(())
    }
}

pub struct PasteClipboard;

impl Action for PasteClipboard {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let t = ctx.transaction("paste");
        // TODO make this copy/paste to actual clipboard
        let subtrees = ctx.app_state.clip_board.get();
        t.run(|| {
            Paste {
                subtrees: &subtrees,
            }
            .perform(ctx)
        })
        .map(|_| ())
    }
}

pub struct CutSelected;

impl Action for CutSelected {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        CopySelected {}.perform(ctx)?;
        DeleteSelected {}.perform(ctx)?;
        Ok(())
    }
}
