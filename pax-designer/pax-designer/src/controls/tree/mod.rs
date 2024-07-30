use pax_engine::api::*;
use pax_engine::layout::LayoutProperties;
use pax_engine::*;
use pax_manifest::{
    ComponentTemplate, PaxType, TemplateNodeId, TreeIndexPosition, TypeId,
    UniqueTemplateNodeIdentifier,
};
use pax_std::*;

use std::cell::{OnceCell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock};
pub mod treeobj;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use treeobj::TreeObj;

use crate::glass::SetEditingComponent;
use crate::math::IntoDecompositionConfiguration;
use crate::model::action::orm::{MoveNode, NodeLayoutSettings};
use crate::model::action::Action;
use crate::model::tools::SelectNodes;
use crate::model::{self, GlassNode};

#[pax]
#[file("controls/tree/mod.pax")]
pub struct Tree {
    pub tree_objects: Property<Vec<FlattenedTreeEntry>>,
    pub on_click_handler: Property<bool>,
    pub dragging: Property<bool>,
    pub drag_id: Property<usize>,
    pub drag_id_start: Property<usize>,
}

impl Interpolatable for TreeMsg {}

#[derive(Clone, Default)]
pub enum TreeMsg {
    ArrowClicked(usize),
    ObjDoubleClicked(usize),
    ObjMouseDown(usize),
    ObjMouseMove(usize),
    #[default]
    None,
}

thread_local! {
    pub static TREE_CLICK_PROP: Property<TreeMsg> = Property::new(TreeMsg::None);
}

struct TreeEntry {
    node_id: TemplateNodeId,
    desc: Desc,
    children: Vec<TreeEntry>,
}

enum Desc {
    Frame,
    Group,
    Ellipse,
    Text,
    Stacker,
    Rectangle,
    Path,
    Component(String),
    Textbox,
    Checkbox,
    Scroller,
    Button,
    Image,
    Slider,
    Dropdown,
}

impl Desc {
    fn info(&self) -> (String, String, bool) {
        let (name, img_path_suffix, is_container) = match self {
            Desc::Frame => ("Frame", "01-frame", true),
            Desc::Group => ("Group", "02-group", true),
            Desc::Ellipse => ("Ellipse", "03-ellipse", false),
            Desc::Text => ("Text", "04-text", false),
            Desc::Stacker => ("Stacker", "05-stacker", true),
            Desc::Rectangle => ("Rectangle", "06-rectangle", false),
            Desc::Path => ("Path", "07-path", false),
            Desc::Component(name) => (name.as_str(), "08-component", false),
            Desc::Textbox => ("Textbox", "09-textbox", false),
            Desc::Checkbox => ("Checkbox", "10-checkbox", false),
            Desc::Scroller => ("Scroller", "11-scroller", true),
            Desc::Button => ("Button", "12-button", false),
            Desc::Image => ("Image", "13-image", false),
            Desc::Slider => ("Slider", "14-slider", false),
            Desc::Dropdown => ("Dropdown", "15-dropdown", false),
        };
        (
            name.to_owned(),
            format!("assets/icons/tree/tree-icon-{}.png", img_path_suffix),
            is_container,
        )
    }
}

impl TreeEntry {
    fn flatten(self, ind: &mut usize, indent_level: isize) -> Vec<FlattenedTreeEntry> {
        let mut all = vec![];
        let (name, img_path, container) = self.desc.info();
        all.push(FlattenedTreeEntry {
            node_id: self.node_id,
            name,
            image_path: img_path,
            ind: *ind,
            indent_level,
            is_visible: true,
            is_collapsed: false,
            is_not_leaf: !self.children.is_empty(),
            is_selected: false,
            is_container: container,
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
#[custom(Imports)]
pub struct FlattenedTreeEntry {
    pub name: String,
    pub node_id: TemplateNodeId,
    pub image_path: String,
    pub ind: usize,
    pub indent_level: isize,
    pub is_visible: bool,
    pub is_selected: bool,
    pub is_collapsed: bool,
    pub is_not_leaf: bool,
    pub is_container: bool,
}

impl Tree {
    // TODO re-implement tree collapsing. Do collapsed state as separate prop
    // that visible_tree_objects can listen to, and that is updated by changes
    // in click_msg
    // TreeMsg::ArrowClicked(sender) => {
    //     tree[sender].is_collapsed = !tree[sender].is_collapsed;
    //     let collapsed = tree[sender].is_collapsed;
    //     for i in (sender + 1)..tree.len() {
    //         if tree[sender].indent_level < tree[i].indent_level {
    //             tree[i].is_visible = !collapsed;
    //             tree[i].is_collapsed = collapsed;
    //         } else {
    //             break;
    //         }
    //     }
    //     self.visible_tree_objects.set(
    //         self.tree_objects
    //             .get()
    //             .iter()
    //             .filter(|o| o.is_visible)
    //             .cloned()
    //             .collect(),
    //     );
    // }
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let click_msg = TREE_CLICK_PROP.with(|click_msg| click_msg.clone());
        let deps = [click_msg.untyped()];

        let ctxp = ctx.clone();
        let tree_obj = self.tree_objects.clone();
        let selected_comp =
            model::read_app_state(|app_state| app_state.selected_component_id.clone());

        let dragging = self.dragging.clone();
        let drag_id = self.drag_id.clone();
        let drag_id_start = self.drag_id_start.clone();

        self.on_click_handler.replace_with(Property::computed(
            move || {
                let msg = click_msg.get();
                match msg {
                    TreeMsg::ObjDoubleClicked(sender) => {
                        let node_id = tree_obj.read(|t| t[sender].node_id.clone());
                        let uuid =
                            UniqueTemplateNodeIdentifier::build(selected_comp.get(), node_id);
                        let mut dt = borrow_mut!(ctxp.designtime);
                        let builder = dt.get_orm_mut().get_node(uuid).unwrap();
                        let type_id_of_tree_target = builder.get_type_id();

                        model::perform_action(&SetEditingComponent(type_id_of_tree_target), &ctxp);
                    }
                    TreeMsg::ObjMouseDown(sender) => {
                        model::perform_action(
                            &SelectNodes {
                                ids: &[tree_obj.read(|t| t[sender].node_id.clone())],
                                overwrite: false,
                            },
                            &ctxp,
                        );
                        dragging.set(true);
                        drag_id_start.set(sender);
                    }
                    TreeMsg::ObjMouseMove(sender) => {
                        drag_id.set(sender);
                    }
                    TreeMsg::ArrowClicked(_) => (),
                    TreeMsg::None => (),
                };
                false
            },
            &deps,
        ));

        model::read_app_state(|app_state| {
            let type_id = app_state.selected_component_id.clone();
            let manifest_ver = borrow!(ctx.designtime).get_manifest_version();
            let selected = app_state.selected_template_node_ids.clone();
            let ctx = ctx.clone();
            let deps = [
                selected.untyped(),
                type_id.untyped(),
                manifest_ver.untyped(),
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

    pub fn mouse_up(&mut self, ctx: &NodeContext, _event: Event<MouseUp>) {
        let from = self.drag_id_start.get();
        let to = self.drag_id.get();
        if self.dragging.get() && from != to {
            let (from, to) = self.tree_objects.read(|tree| {
                let from = &tree[from];
                let to = &tree[to];
                (
                    (from.node_id.clone(), from.is_container),
                    (to.node_id.clone(), to.is_container),
                )
            });
            Self::tree_move(from, to, &ctx);
        }
        self.dragging.set(false);
    }

    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        // because of lazy eval. Need to make sure closure fires
        // if its dependents have changed
        self.on_click_handler.get();
    }

    fn tree_move(
        (from_id, _): (TemplateNodeId, bool),
        (to_id, to_is_container): (TemplateNodeId, bool),
        ctx: &NodeContext,
    ) {
        model::with_action_context(ctx, |ctx| {
            let comp_id = ctx.app_state.selected_component_id.get();
            let from_uid = UniqueTemplateNodeIdentifier::build(comp_id.clone(), from_id);
            let to_uid = UniqueTemplateNodeIdentifier::build(comp_id.clone(), to_id);
            let from_node = ctx.get_glass_node_by_global_id(&from_uid);
            let to_node = ctx.get_glass_node_by_global_id(&to_uid);
            let to_node_container = match to_is_container {
                true => to_node.clone(),
                false => {
                    let parent = to_node.raw_node_interface.template_parent().unwrap();
                    // let index = parent
                    GlassNode::new(&parent, &ctx.glass_transform())
                }
            };

            let is_stacker = to_node_container.raw_node_interface.is_of_type::<Stacker>();
            let index = match is_stacker {
                true => {
                    if to_node_container.raw_node_interface == to_node.raw_node_interface {
                        TreeIndexPosition::Top
                    } else {
                        let slot = to_node
                            .raw_node_interface
                            .render_parent()
                            .unwrap()
                            .with_properties(|slot: &mut Slot| slot.index.get().to_int() as usize);
                        slot.map(|s| TreeIndexPosition::At(s)).unwrap_or_default()
                    }
                }
                false => to_node_container
                    .raw_node_interface
                    .children()
                    .into_iter()
                    .enumerate()
                    .find_map(|(i, c)| {
                        (c == to_node.raw_node_interface).then_some(TreeIndexPosition::At(i))
                    })
                    .unwrap_or_default(),
            };
            ctx.undo_save();

            let node_layout = if is_stacker {
                NodeLayoutSettings::Fill::<NodeLocal>
            } else {
                NodeLayoutSettings::WithProperties(LayoutProperties {
                    x: Some(Size::ZERO()),
                    y: Some(Size::ZERO()),
                    ..from_node.layout_properties
                })
            };

            if let Err(e) = (MoveNode {
                node_id: &from_node.id,
                new_parent_uid: &to_node_container.id,
                index,
                node_layout,
            }
            .perform(ctx))
            {
                log::warn!("failed to move tree node: {e}");
            };
        });
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
    let children = component_template
        .get_children(tnid)
        .unwrap_or_default()
        .iter()
        .filter_map(|c_tnid| to_tree(c_tnid, component_template))
        .collect();
    let node_type = resolve_tree_type(node.type_id.clone());
    Some(TreeEntry {
        node_id: tnid.clone(),
        desc: node_type,
        children,
    })
}

fn resolve_tree_type(type_id: TypeId) -> Desc {
    let Some(import_path) = type_id.import_path() else {
        return Desc::Component(format!("{}", type_id.get_pax_type()));
    };
    match import_path.trim_start_matches("pax_std::") {
        "core::group::Group" => Desc::Group,
        "core::frame::Frame" => Desc::Frame,
        "drawing::ellipse::Ellipse" => Desc::Ellipse,
        "core::text::Text" => Desc::Text,
        "layout::stacker::Stacker" => Desc::Stacker,
        "drawing::rectangle::Rectangle" => Desc::Rectangle,
        "drawing::path::Path" => Desc::Path,
        "forms::textbox::Textbox" => Desc::Textbox,
        "forms::checkbox::Checkbox" => Desc::Checkbox,
        "core::scroller::Scroller" => Desc::Scroller,
        "forms::button::Button" => Desc::Button,
        "core::image::Image" => Desc::Image,
        "forms::slider::Slider" => Desc::Slider,
        "forms::dropdown::Dropdown" => Desc::Dropdown,
        _ => Desc::Component(format!("{}", type_id.get_pax_type())),
    }
}
