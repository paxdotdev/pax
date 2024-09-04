use std::any::Any;
use std::cell::Cell;
use std::{rc::Rc, sync::Arc};

use self::orm::UndoRequested;

use super::{DerivedAppState, GlassNode, SelectionState};
use crate::math::coordinate_spaces::World;
use crate::message_log_display::{self, DesignerLogMsg};
use crate::{math::AxisAlignedBox, model::AppState, DESIGNER_GLASS_ID};
use anyhow::{anyhow, Error, Result};
use pax_designtime::orm::PaxManifestORM;
use pax_designtime::DesigntimeManager;
use pax_engine::api::{borrow, borrow_mut, Axis, Size};
use pax_engine::math::Vector2;
use pax_engine::node_layout::TransformAndBounds;
use pax_engine::pax_manifest::{
    NodeLocation, TemplateNodeId, TreeIndexPosition, TreeLocation, UniqueTemplateNodeIdentifier,
};
use pax_engine::{
    api::{NodeContext, Window},
    math::{Point2, Space, Transform2},
    NodeInterface,
};
use pax_engine::{log, Property};
use pax_std::drawing::rectangle::Rectangle;

pax_engine::pax_message::use_RefCell!();

use crate::math::coordinate_spaces::Glass;

pub mod meta;
pub mod orm;
pub mod pointer;
pub mod world;

#[derive(Default)]
pub struct UndoRedoStack {
    undo_stack: RefCell<Vec<usize>>,
    redo_stack: RefCell<Vec<usize>>,
}

impl UndoRedoStack {
    pub fn push(&self, undo_id: usize) {
        borrow_mut!(self.undo_stack).push(undo_id);
        borrow_mut!(self.redo_stack).clear();
    }

    fn undo(&self, orm: &mut PaxManifestORM) -> Option<()> {
        let curr_id = orm.get_last_undo_id()?;
        let undo_id = borrow_mut!(self.undo_stack).pop()?;
        orm.undo_until(undo_id).ok()?;
        borrow_mut!(self.redo_stack).push(curr_id);
        Some(())
    }

    fn redo(&self, orm: &mut PaxManifestORM) -> Option<()> {
        let curr_id = orm.get_last_undo_id()?;
        let redo_id = borrow_mut!(self.redo_stack).pop()?;
        orm.redo_including(redo_id).ok()?;
        borrow_mut!(self.undo_stack).push(curr_id);
        Some(())
    }
}

pub trait Action<R = ()> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<R>;
}

pub struct ActionContext<'a> {
    pub engine_context: &'a NodeContext,
    pub app_state: &'a mut AppState,
    pub derived_state: &'a DerivedAppState,
    pub undo_stack: &'a Rc<UndoRedoStack>,
}

impl ActionContext<'_> {
    pub fn world_transform(&self) -> Transform2<Glass, World> {
        self.app_state.glass_to_world_transform.get()
    }

    pub fn glass_transform(&self) -> Property<Transform2<Window, Glass>> {
        self.derived_state.to_glass_transform.get()
    }

    pub fn selected_nodes(&self) -> Vec<(UniqueTemplateNodeIdentifier, NodeInterface)> {
        self.derived_state.selected_nodes.get()
    }

    pub fn raycast_glass(
        &self,
        point: Point2<Glass>,
        mode: RaycastMode,
        skip: &[NodeInterface],
    ) -> Option<NodeInterface> {
        let window_point = self.glass_transform().get().inverse() * point;
        let all_elements_beneath_ray = self.engine_context.raycast(window_point, false);

        let userland = self.engine_context.get_userland_root_expanded_node();
        let userland_id = userland.global_id();

        let mut potential_targets = all_elements_beneath_ray
            .into_iter()
            .filter(|elem| !skip.iter().any(|v| elem == v || elem.is_descendant_of(v)))
            .filter(|elem| elem.is_descendant_of(&userland));

        if let RaycastMode::RawNth(index) = mode {
            return potential_targets.nth(index);
        }

        let mut target = potential_targets.next()?;

        // Find the ancestor that is a direct root element inside container
        // or one that's in the current edit root
        loop {
            let Some(parent) = target.template_parent() else {
                break;
            };

            // check one step ahead if we are drilling into a group or similar

            if parent.global_id() == userland_id {
                break;
            };
            if parent
                .global_id()
                .is_some_and(|v| self.derived_state.open_container.get() == v)
            {
                break;
            }
            if matches!(mode, RaycastMode::DrillOne) {
                let Some(next_parent) = parent.template_parent() else {
                    break;
                };
                if next_parent.global_id() == userland_id {
                    break;
                };
                if next_parent
                    .global_id()
                    .is_some_and(|v| self.derived_state.open_container.get() == v)
                {
                    break;
                }
            }
            target = target.template_parent().unwrap();
        }
        Some(target)
    }

    pub fn location(
        &self,
        uid: &UniqueTemplateNodeIdentifier,
        index: &TreeIndexPosition,
    ) -> NodeLocation {
        if self
            .engine_context
            .get_userland_root_expanded_node()
            .global_id()
            == Some(uid.clone())
        {
            NodeLocation::new(
                self.app_state.selected_component_id.get(),
                TreeLocation::Root,
                index.clone(),
            )
        } else {
            NodeLocation::new(
                self.app_state.selected_component_id.get(),
                TreeLocation::Parent(uid.get_template_node_id()),
                index.clone(),
            )
        }
    }

    pub fn get_glass_node_by_global_id(
        &mut self,
        uid: &UniqueTemplateNodeIdentifier,
    ) -> Result<GlassNode> {
        let node_interface = self
            .engine_context
            .get_nodes_by_global_id(uid.clone())
            .into_iter()
            .max()
            .ok_or(anyhow!(
                "couldn't find node in engine (has a designer update tick passed?)"
            ))?;
        Ok(GlassNode::new(&node_interface, &self.glass_transform()))
    }

    pub fn transaction(&mut self, user_action_message: &str) -> Transaction {
        Transaction::new(&self, user_action_message)
    }
}

pub struct Transaction {
    before_undo_id: usize,
    design_time: Rc<RefCell<DesigntimeManager>>,
    undo_stack: Rc<UndoRedoStack>,
    result: RefCell<Result<()>>,
    user_action_message: String,
}

impl Transaction {
    pub fn new(ctx: &ActionContext, user_action_message: &str) -> Self {
        let design_time = Rc::clone(&ctx.engine_context.designtime);
        let before_undo_id = borrow!(design_time)
            .get_orm()
            .get_last_undo_id()
            .unwrap_or(0);
        Self {
            undo_stack: Rc::clone(&ctx.undo_stack),
            before_undo_id,
            design_time,
            result: RefCell::new(Ok(())),
            user_action_message: user_action_message.to_owned(),
        }
    }

    pub fn run<V>(&self, t: impl FnOnce() -> Result<V>) -> Result<V> {
        if borrow!(self.result).is_err() {
            return Err(anyhow!("prior transaction operation failed"));
        }
        match t() {
            Ok(v) => Ok(v),
            Err(e) => {
                // TODO improve: this is weird for certain kinds of errors (for
                // example internal ones), find more general framework.
                let user_message = format!(
                    "{} and would have been modified when {}. \
                    To modify individual values use the settings view. \
                    To overwrite all expressions hold Ctrl.",
                    e, self.user_action_message
                );
                message_log_display::log(DesignerLogMsg::message(user_message));
                *borrow_mut!(self.result) = Err(anyhow!("transaction failed: {e}"));
                let mut dt = borrow_mut!(self.design_time);
                dt.get_orm_mut()
                    .undo_until(self.before_undo_id)
                    .ok()
                    .unwrap();
                Err(e)
            }
        }
    }
}

// This exists to never forget to call finish on a created transaction, aim to
// not remove this but instead figure out why finish wasn't called. TODO: might
// want to add a "cancel" method to the struct above to explicitly cancel a
// transaction as well (revert changes and don't push to undo stack)
impl Drop for Transaction {
    fn drop(&mut self) {
        if borrow!(self.result).is_ok() {
            self.undo_stack.push(self.before_undo_id);
        }
    }
}

pub enum RaycastMode {
    // Only hit elements that are either directly bellow the userland project
    // root, or ones that are at the same level as an already selected node
    Top,
    // Hit the children of the "Top" elements
    DrillOne,
    // Hit all elements, and choose the nth hit
    RawNth(usize),
}
