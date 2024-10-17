use std::any::Any;
use std::cell::Cell;
use std::{rc::Rc, sync::Arc};

use anyhow::{anyhow, Error, Result};

use pax_engine::serde::Serialize;
use pax_engine::{
    api::{NodeContext, Window},
    math::{Point2, Space, Transform2},
    NodeInterface,
};
use pax_engine::{log, Property};
use pax_std::drawing::rectangle::Rectangle;
pax_engine::pax_message::use_RefCell!();
use pax_designtime::orm::PaxManifestORM;
use pax_designtime::DesigntimeManager;
use pax_engine::api::{borrow, borrow_mut, Axis, Interpolatable, Size};
use pax_engine::math::Vector2;
use pax_engine::node_layout::TransformAndBounds;
use pax_engine::pax_manifest::{
    NodeLocation, TemplateNodeId, TreeIndexPosition, TreeLocation, TypeId,
    UniqueTemplateNodeIdentifier,
};

use super::{DerivedAppState, GlassNode, SelectionState};
use crate::designer_node_type::DesignerNodeType;
use crate::math::coordinate_spaces::Glass;
use crate::math::coordinate_spaces::World;
use crate::message_log_display::{self, DesignerLogMsg};
use crate::{math::AxisAlignedBox, model::app_state::AppState, DESIGNER_GLASS_ID};
use orm::UndoRequested;

pub mod init;
pub mod meta;
pub mod orm;
pub mod pointer;
pub mod tool;
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
        let curr_id = orm.get_last_undo_id();
        let undo_id = borrow_mut!(self.undo_stack).pop();
        log::trace!("undo from {:?} to {:?} (non-inclusive)", curr_id, undo_id);
        orm.undo_until(undo_id).ok()?;
        if let Some(curr_id) = curr_id {
            borrow_mut!(self.redo_stack).push(curr_id);
        }
        Some(())
    }

    fn redo(&self, orm: &mut PaxManifestORM) -> Option<()> {
        let curr_id = orm.get_last_undo_id();
        let redo_id = borrow_mut!(self.redo_stack).pop()?;
        log::trace!("redo from {:?} to {} (inclusive)", curr_id, redo_id);
        orm.redo_including(redo_id).ok()?;
        if let Some(curr_id) = curr_id {
            borrow_mut!(self.undo_stack).push(curr_id);
        }
        Some(())
    }
}

pub trait Action<R = ()> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<R>;
}

// make this a trait that's implemented for NodeContext instead? (since app state is static)
pub struct ActionContext<'a> {
    pub engine_context: &'a NodeContext,
    pub app_state: &'a mut AppState,
    pub derived_state: &'a DerivedAppState,
    undo_stack: &'a Rc<UndoRedoStack>,
}

impl<'a> ActionContext<'a> {
    pub fn new(
        engine_context: &'a NodeContext,
        app_state: &'a mut AppState,
        derived_state: &'a DerivedAppState,
        undo_stack: &'a Rc<UndoRedoStack>,
    ) -> Self {
        Self {
            engine_context,
            app_state,
            derived_state,
            undo_stack,
        }
    }

    pub fn world_transform(&self) -> Transform2<Glass, World> {
        self.app_state.glass_to_world_transform.get()
    }

    pub fn glass_transform(&self) -> Property<Transform2<Window, Glass>> {
        self.derived_state.to_glass_transform.get()
    }

    pub fn raycast_glass(
        &self,
        point: Point2<Glass>,
        mode: RaycastMode,
        skip: &[NodeInterface],
    ) -> Option<NodeInterface> {
        let window_point = self.glass_transform().get().inverse() * point;
        let all_elements_beneath_ray = self.engine_context.raycast(window_point, false);

        let userland = self.engine_context.get_userland_root_expanded_node()?;
        let userland_id = userland.global_id();

        let potential_targets: Vec<_> = all_elements_beneath_ray
            .into_iter()
            .filter(|elem| !skip.iter().any(|v| elem == v || elem.is_descendant_of(v)))
            .filter(|elem| elem.is_descendant_of(&userland))
            .collect();

        if let RaycastMode::RawNth(index) = mode {
            return potential_targets.into_iter().nth(index);
        }

        // if we hit any node that was already selected, then just return this node
        if let Some(already_selected) = self.derived_state.selected_nodes.read(|selected_nodes| {
            potential_targets
                .iter()
                .find(|n| selected_nodes.iter().map(|(_, s)| s).any(|s| *n == s))
        }) {
            return Some(already_selected.clone());
        }

        let mut target = potential_targets.into_iter().next()?;

        // Find the ancestor that is a direct root element inside container
        // or one that's in the current edit root
        loop {
            let Some(parent) = target.template_parent() else {
                break;
            };

            if parent.global_id() == userland_id {
                break;
            };
            if parent
                .global_id()
                .is_some_and(|v| self.derived_state.open_containers.get().contains(&v))
            {
                break;
            }

            // check one step ahead if we are drilling into a group or similar
            if matches!(mode, RaycastMode::DrillOne) {
                let Some(next_parent) = parent.template_parent() else {
                    break;
                };
                if next_parent.global_id() == userland_id {
                    break;
                };
                if next_parent
                    .global_id()
                    .is_some_and(|v| self.derived_state.open_containers.get().contains(&v))
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
            .and_then(|n| n.global_id())
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

    pub fn designer_node_type(&self, id: &UniqueTemplateNodeIdentifier) -> DesignerNodeType {
        let mut dt = borrow_mut!(self.engine_context.designtime);
        let orm = dt.get_orm_mut();
        let Some(node) = orm.get_node_builder(id.clone(), false) else {
            return DesignerNodeType::Unregistered;
        };
        DesignerNodeType::from_type_id(node.get_type_id())
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
            .ok_or_else(|| {
                anyhow!("couldn't find node in engine (has a designer update tick passed?)")
            })?;
        Ok(GlassNode::new(&node_interface, &self.glass_transform()))
    }

    pub fn transaction(&mut self, user_action_message: &str) -> Transaction {
        Transaction::new(&self, user_action_message)
    }
}

impl Interpolatable for Transaction {}

#[derive(Clone)]
pub struct Transaction {
    before_undo_id: Option<usize>,
    design_time: Rc<RefCell<DesigntimeManager>>,
    component_id: Property<TypeId>,
    undo_stack: Rc<UndoRedoStack>,
    result: Rc<RefCell<Result<()>>>,
    user_action_message: String,
}

impl std::fmt::Debug for Transaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transaction")
            .field("user_action_message", &self.user_action_message)
            .finish_non_exhaustive()
    }
}

impl Transaction {
    pub fn new(ctx: &ActionContext, user_action_message: &str) -> Self {
        log::trace!("transaction {:?} created", user_action_message);
        let design_time = Rc::clone(&ctx.engine_context.designtime);
        let before_undo_id = borrow!(design_time).get_orm().get_last_undo_id();
        let component_id = ctx.app_state.selected_component_id.clone();
        Self {
            undo_stack: Rc::clone(&ctx.undo_stack),
            before_undo_id,
            design_time,
            result: Rc::new(RefCell::new(Ok(()))),
            user_action_message: user_action_message.to_owned(),
            component_id,
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

impl Drop for Transaction {
    fn drop(&mut self) {
        log::trace!("transaction {:?} finished", self.user_action_message);
        if borrow!(self.result).is_ok() {
            if let Some(undo_before) = self.before_undo_id {
                self.undo_stack.push(undo_before);
            }
            let mut dt = borrow_mut!(self.design_time);
            if let Err(e) = dt.send_component_update(&self.component_id.get()) {
                pax_engine::log::error!("failed to save component to file: {:?}", e);
            }
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
