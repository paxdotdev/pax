use anyhow::Result;
use pax_engine::api::*;
use pax_engine::node_layout::LayoutProperties;
use pax_engine::*;
use pax_manifest::{
    ComponentTemplate, PaxType, TemplateNodeId, TreeIndexPosition, TypeId,
    UniqueTemplateNodeIdentifier,
};
use pax_std::*;

pax_engine::pax_message::use_RefCell!();

use std::cell::OnceCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock};
pub mod treeobj;
use serde::Deserialize;
use std::collections::{HashMap, HashSet, VecDeque};
use treeobj::TreeObj;

use crate::designer_node_type::DesignerNodeType;
use crate::glass::SetEditingComponent;
use crate::math::coordinate_spaces::Glass;
use crate::math::IntoDecompositionConfiguration;
use crate::model::action::orm::{tree_movement::MoveNode, NodeLayoutSettings};
use crate::model::action::Action;
use crate::model::tools::SelectNodes;
use crate::model::{self, GlassNode};

#[pax]
#[engine_import_path("pax_engine")]
#[file("controls/tree/mod.pax")]
pub struct Tree {
    pub tree_objects: Property<Vec<FlattenedTreeEntry>>,
    pub dragging: Property<bool>,
    pub drag_id: Property<usize>,
    pub drag_id_start: Property<usize>,
    pub drag_x_start: Property<f64>,
    pub drag_indent: Property<isize>,
    pub drag_ignore_container: Property<bool>,
}

impl Interpolatable for TreeMsg {}

#[derive(Clone)]
pub enum TreeMsg {
    ObjDoubleClicked(usize),
    ObjMouseDown(usize, f64),
    ObjMouseMove(usize, f64, bool),
}

thread_local! {
    pub static TREE_CLICK_PROP: std::cell::RefCell<VecDeque<TreeMsg>> = Default::default();
    pub static GLOBAL_MOUSEUP_PROP: Property<bool> = Default::default();
    pub static TREE_HIDDEN_NODES: Property<HashSet<TemplateNodeId>> = Default::default();
}

pub fn trigger_global_mouseup() {
    GLOBAL_MOUSEUP_PROP.with(|p| p.set(true));
}

struct TreeEntry {
    node_id: TemplateNodeId,
    node_type: DesignerNodeType,
    children: Vec<TreeEntry>,
}

impl TreeEntry {
    fn flatten(self, ind: &mut usize, indent_level: isize) -> Vec<FlattenedTreeEntry> {
        let mut all = vec![];
        let desc = self.node_type.metadata();
        all.push(FlattenedTreeEntry {
            node_id: self.node_id,
            name: desc.name,
            image_path: desc.image_path,
            ind: *ind,
            indent_level,
            is_visible: true,
            is_selected: false,
            is_container: desc.is_container,
        });
        *ind += 1;
        all.extend(
            self.children
                .into_iter()
                .flat_map(|c| c.flatten(ind, indent_level + 1)),
        );
        all
    }
}

#[pax]
#[engine_import_path("pax_engine")]
#[custom(Imports)]
pub struct FlattenedTreeEntry {
    pub name: String,
    pub node_id: TemplateNodeId,
    pub image_path: String,
    pub ind: usize,
    pub indent_level: isize,
    pub is_visible: bool,
    pub is_selected: bool,
    pub is_container: bool,
}

impl Tree {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        model::read_app_state(|app_state| {
            let type_id = app_state.selected_component_id.clone();
            let manifest_ver = borrow!(ctx.designtime).get_manifest_version();
            let selected = app_state.selected_template_node_ids.clone();
            let ctx = ctx.clone();
            let hidden_nodes = TREE_HIDDEN_NODES.with(|p| p.clone());
            let deps = [
                selected.untyped(),
                type_id.untyped(),
                manifest_ver.untyped(),
                hidden_nodes.untyped(),
            ];

            self.tree_objects.replace_with(Property::computed(
                move || {
                    let type_id = type_id.get();
                    let mut tree = get_tree(type_id, &ctx);
                    let selected = selected.get();
                    for entry in &mut tree {
                        entry.is_selected = selected.contains(&entry.node_id);
                    }
                    tree
                },
                &deps,
            ));
        });
    }

    pub fn pre_render(&mut self, ctx: &NodeContext) {
        let tree_obj = self.tree_objects.clone();
        let selected_comp =
            model::read_app_state(|app_state| app_state.selected_component_id.clone());

        let dragging = self.dragging.clone();
        let drag_id = self.drag_id.clone();
        let drag_id_start = self.drag_id_start.clone();
        while let Some(msg) = TREE_CLICK_PROP.with_borrow_mut(|msgs| msgs.pop_front()) {
            match msg {
                TreeMsg::ObjDoubleClicked(sender) => {
                    let node_id = tree_obj.read(|t| t[sender].node_id.clone());
                    let uuid = UniqueTemplateNodeIdentifier::build(selected_comp.get(), node_id);
                    let mut dt = borrow_mut!(ctx.designtime);
                    let builder = dt.get_orm_mut().get_node(uuid, false).unwrap();
                    let type_id_of_tree_target = builder.get_type_id();

                    model::perform_action(&SetEditingComponent(type_id_of_tree_target), &ctx);
                }
                TreeMsg::ObjMouseDown(sender, x_offset) => {
                    model::perform_action(
                        &SelectNodes {
                            ids: &[tree_obj.read(|t| t[sender].node_id.clone())],
                            mode: model::tools::SelectMode::Dynamic,
                        },
                        &ctx,
                    );
                    dragging.set(true);
                    drag_id_start.set(sender);
                    self.drag_x_start.set(x_offset);
                    self.drag_indent
                        .set(tree_obj.read(|t| t[sender].indent_level));
                }
                // TODO make less ugly
                TreeMsg::ObjMouseMove(sender, x_offset, ignore_container) => {
                    drag_id.set(sender);
                    let tree_obj = tree_obj.get();
                    let original_indent = tree_obj[drag_id_start.get()].indent_level;
                    if drag_id_start.get() == sender {
                        let offset = x_offset - self.drag_x_start.get();
                        let potential_indent = (offset / 15.0) as isize + original_indent;
                        let above = tree_obj
                            .get(sender.saturating_sub(1))
                            .map(|a| a.indent_level + a.is_container as isize)
                            .unwrap_or(0);
                        // find the object closest below in the list with indent equal or less to original_indent
                        let mut curr_ind = sender + 1;
                        while curr_ind < tree_obj.len()
                            && tree_obj[curr_ind].indent_level > original_indent
                        {
                            curr_ind += 1;
                        }
                        let below = tree_obj.get(curr_ind).map(|b| b.indent_level).unwrap_or(0);
                        self.drag_indent
                            .set(potential_indent.clamp(below.min(above), above));
                        self.drag_ignore_container.set(false);
                    } else {
                        let curr_is_container = model::with_action_context(ctx, |ac| {
                            ac.get_glass_node_by_global_id(&UniqueTemplateNodeIdentifier::build(
                                ac.app_state.selected_component_id.get(),
                                tree_obj[sender].node_id.clone(),
                            ))
                            .map(|n| n.get_node_type(ctx).metadata().is_container)
                        })
                        .unwrap_or(false);
                        self.drag_indent.set(original_indent);
                        self.drag_ignore_container
                            .set(curr_is_container && ignore_container);
                    }
                }
            };
        }

        // TODO make less ugly
        if GLOBAL_MOUSEUP_PROP.with(|p| p.get()) {
            GLOBAL_MOUSEUP_PROP.with(|p| p.set(false));
            let from_ind = self.drag_id_start.get();
            let to = self.drag_id.get();
            let drag_indent = self.drag_indent.get();
            let tree = self.tree_objects.get();

            if self.dragging.get() && (from_ind != to || tree[from_ind].indent_level != drag_indent)
            {
                let from = tree[from_ind].clone();
                let (to, child_ind_override) = if from_ind != to {
                    (Some(tree[to].clone()), None)
                } else {
                    // Find the first container whos indent level is one
                    // less than drag_indent
                    let mut children_above = 0;
                    let mut curr_ind = to as isize - 1;
                    while curr_ind >= 0 && tree[curr_ind as usize].indent_level != drag_indent - 1 {
                        if tree[curr_ind as usize].indent_level == drag_indent {
                            children_above += 1;
                        }
                        curr_ind -= 1;
                    }
                    (
                        (curr_ind >= 0).then(|| tree[curr_ind as usize].clone()),
                        Some(TreeIndexPosition::At(children_above)),
                    )
                };
                if let Err(e) = Self::tree_move(
                    from.node_id,
                    to.map(|t| t.node_id),
                    child_ind_override,
                    &ctx,
                    self.drag_ignore_container.get(),
                ) {
                    log::warn!("failed to move tree node: {e}");
                };
            }
            self.dragging.set(false);
        }
    }

    // TODO make less ugly
    fn tree_move(
        from: TemplateNodeId,
        to_parent: Option<TemplateNodeId>,
        child_ind_override: Option<TreeIndexPosition>,
        ctx: &NodeContext,
        ignore_container: bool,
    ) -> Result<()> {
        log::debug!(
            "from: {:?}, to_parent: {:?}, child_ind_override: {:?}, ignore_container: {:?}",
            from,
            to_parent,
            child_ind_override,
            ignore_container
        );
        model::with_action_context(ctx, |ctx| {
            let comp_id = ctx.app_state.selected_component_id.get();
            let from_uid = UniqueTemplateNodeIdentifier::build(comp_id.clone(), from.clone());
            let to_uid = to_parent
                .as_ref()
                .map(|t| UniqueTemplateNodeIdentifier::build(comp_id.clone(), t.clone()))
                .unwrap_or(
                    ctx.engine_context
                        .get_userland_root_expanded_node()
                        .and_then(|n| n.global_id())
                        .ok_or_else(|| {
                            anyhow::anyhow!("failed to get userland root in tree move")
                        })?,
                );
            let from_node = ctx.get_glass_node_by_global_id(&from_uid)?;
            let to_node = ctx.get_glass_node_by_global_id(&to_uid)?;
            let to_node_container = match to_node
                .get_node_type(&ctx.engine_context)
                .metadata()
                .is_container
                && !ignore_container
            {
                true => to_node.clone(),
                false => {
                    if to_parent.is_none() {
                        to_node.clone()
                    } else {
                        let parent = to_node
                            .raw_node_interface
                            .template_parent()
                            .expect("all nodes in tree view should have a template parent");
                        // let index = parent
                        GlassNode::new(&parent, &ctx.glass_transform())
                    }
                }
            };

            TREE_HIDDEN_NODES.with(|p| {
                p.update(|v| {
                    v.remove(&to_node_container.id.get_template_node_id());
                })
            });

            let index = child_ind_override.unwrap_or_else(|| {
                if to_node_container.id == to_node.id {
                    return TreeIndexPosition::Top;
                }
                let mut dt = borrow_mut!(ctx.engine_context.designtime);
                let pos = dt
                    .get_orm_mut()
                    .get_siblings(&to_node.id)
                    .and_then(|v| v.iter().position(|n| n == &to_node.id))
                    .unwrap_or_default();
                TreeIndexPosition::At(pos)
            });
            let keep_bounds = NodeLayoutSettings::KeepScreenBounds {
                node_transform_and_bounds: &from_node.transform_and_bounds.get(),
                node_decomposition_config: &from_node.layout_properties.into_decomposition_config(),
                parent_transform_and_bounds: &to_node_container.transform_and_bounds.get(),
            };
            let node_layout = if to_node_container
                .get_node_type(&ctx.engine_context)
                .metadata()
                .is_slot_container
            {
                NodeLayoutSettings::Fill::<Glass>
            } else {
                // TODO decide how to handle tree movement:
                // - keeping screen bounds most intuitive (is what Figma does)
                //   but this makes objects that have expressions be immovable
                // - keep all properties (makes everything with exprs movable)
                // - keep some properties (still would complain if x/y is expr):
                // ```
                // NodeLayoutSettings::WithProperties(LayoutProperties {
                //     x: Some(Size::ZERO()),
                //     y: Some(Size::ZERO()),
                //     ..from_node.layout_properties
                // })
                // ```
                keep_bounds
            };
            let t = ctx.transaction("moving object in tree");
            t.run(|| {
                MoveNode {
                    node_id: &from_node.id,
                    new_parent_uid: &to_node_container.id,
                    index,
                    node_layout,
                }
                .perform(ctx)
            })
        })
    }
}

fn get_tree(type_id: TypeId, ctx: &NodeContext) -> Vec<FlattenedTreeEntry> {
    let dt = borrow_mut!(ctx.designtime);
    let Ok(comp) = dt.get_orm().get_component(&type_id) else {
        pax_engine::log::warn!("couldn't find component for tree view");
        return Vec::new();
    };
    let Some(template) = comp.template.as_ref() else {
        pax_engine::log::warn!("treeview component template embty");
        return Vec::new();
    };
    let mut ind = 0;
    let flattened: Vec<FlattenedTreeEntry> = template
        .get_root()
        .iter()
        .flat_map(|tnid| {
            let tree = to_tree(tnid, &template);
            tree.map(|t| t.flatten(&mut ind, 0)).unwrap_or_default()
        })
        .collect();
    flattened
}

fn to_tree(tnid: &TemplateNodeId, component_template: &ComponentTemplate) -> Option<TreeEntry> {
    let node = component_template.get_node(tnid)?;
    if node.type_id.get_pax_type() == &PaxType::Comment {
        return None;
    }
    let children = if TREE_HIDDEN_NODES.with(|p| p.get().contains(tnid)) {
        Vec::new()
    } else {
        component_template
            .get_children(tnid)
            .unwrap_or_default()
            .iter()
            .filter_map(|c_tnid| to_tree(c_tnid, component_template))
            .collect()
    };
    let node_type = DesignerNodeType::from_type_id(node.type_id.clone());
    Some(TreeEntry {
        node_id: tnid.clone(),
        node_type,
        children,
    })
}
