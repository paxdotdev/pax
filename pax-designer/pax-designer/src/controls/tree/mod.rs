use pax_engine::api::*;
use pax_engine::*;
use pax_manifest::{
    ComponentTemplate, PaxType, TemplateNodeId, TypeId, UniqueTemplateNodeIdentifier,
};
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::Text;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::text::*;
use pax_std::types::*;

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
use crate::model;
use crate::model::tools::SelectNode;

#[pax]
#[file("controls/tree/mod.pax")]
pub struct Tree {
    pub tree_objects: Property<Vec<FlattenedTreeEntry>>,
    pub visible_tree_objects: Property<Vec<FlattenedTreeEntry>>,
    // last-patches update stuff
    pub old_selected: Property<Vec<TemplateNodeId>>,
    pub old_type_id: Property<TypeId>,
    pub old_manifest_ver: Property<usize>,
}

impl Interpolatable for TreeMsg {}

#[derive(Clone, Default)]
pub enum TreeMsg {
    ArrowClicked(usize),
    ObjClicked(usize),
    ObjDoubleClicked(usize),
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
    fn info(&self) -> (String, String) {
        let (name, img_path_suffix) = match self {
            Desc::Frame => ("Frame", "01-frame"),
            Desc::Group => ("Group", "02-group"),
            Desc::Ellipse => ("Ellipse", "03-ellipse"),
            Desc::Text => ("Text", "04-text"),
            Desc::Stacker => ("Stacker", "05-stacker"),
            Desc::Rectangle => ("Rectangle", "06-rectangle"),
            Desc::Path => ("Path", "07-path"),
            Desc::Component(name) => (name.as_str(), "08-component"),
            Desc::Textbox => ("Textbox", "09-textbox"),
            Desc::Checkbox => ("Checkbox", "10-checkbox"),
            Desc::Scroller => ("Scroller", "11-scroller"),
            Desc::Button => ("Button", "12-button"),
            Desc::Image => ("Image", "13-image"),
            Desc::Slider => ("Slider", "14-slider"),
            Desc::Dropdown => ("Dropdown", "15-dropdown"),
        };
        (
            name.to_owned(),
            format!("assets/icons/tree/tree-icon-{}.png", img_path_suffix),
        )
    }
}

impl TreeEntry {
    fn flatten(self, ind: &mut usize, indent_level: isize) -> Vec<FlattenedTreeEntry> {
        let mut all = vec![];
        let (name, img_path) = self.desc.info();
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
            is_not_dummy: true,
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
    pub is_not_dummy: bool,
}

impl Tree {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        // let mut click_msg = TREE_CLICK_PROP.with(|click_msg| click_msg.clone());

        // self.tree
        //     match msg {
        //         TreeMsg::ArrowClicked(sender) => {
        //             tree[sender].is_collapsed = !tree[sender].is_collapsed;
        //             let collapsed = tree[sender].is_collapsed;
        //             for i in (sender + 1)..tree.len() {
        //                 if tree[sender].indent_level < tree[i].indent_level {
        //                     tree[i].is_visible = !collapsed;
        //                     tree[i].is_collapsed = collapsed;
        //                 } else {
        //                     break;
        //                 }
        //             }
        //             self.visible_tree_objects.set(
        //                 self.tree_objects
        //                     .get()
        //                     .iter()
        //                     .filter(|o| o.is_visible)
        //                     .cloned()
        //                     .collect(),
        //             );
        //         }
        //         TreeMsg::ObjClicked(sender) => model::perform_action(
        //             SelectNode {
        //                 id: self.tree_objects.get()[sender].node_id.clone(),
        //                 overwrite: false,
        //             },
        //             ctx,
        //         ),
        //         TreeMsg::ObjDoubleClicked(sender) => {
        //             let node_id = &self.tree_objects.get()[sender].node_id;
        //             let type_id_of_tree_target = model::read_app_state(|app_state| {
        //                 let uuid = UniqueTemplateNodeIdentifier::build(
        //                     app_state.selected_component_id.get().clone(),
        //                     node_id.clone(),
        //                 );
        //                 let mut dt = ctx.designtime.borrow_mut();
        //                 let builder = dt.get_orm_mut().get_node(uuid).unwrap();
        //                 builder.get_type_id()
        //             });

        //             model::perform_action(SetEditingComponent(type_id_of_tree_target), ctx)
        //         }
        //     }
        //     self.tree_objects.set(tree);
        // }

        //HACK pre dirty-dag, ulgy but works and can be removed later!
        model::read_app_state(|app_state| {
            let type_id = app_state.selected_component_id.clone();
            let manifest_ver = self.old_manifest_ver.clone();
            let ctx = ctx.clone();
            let deps = [type_id.untyped(), manifest_ver.untyped()];

            self.tree_objects.replace_with(Property::computed(
                move || {
                    let type_id = type_id.get();
                    get_tree(type_id, &ctx)
                },
                &deps,
            ));
            let tree = self.tree_objects.clone();
            let selected = app_state.selected_template_node_ids.clone();
            let deps = [selected.untyped(), tree.untyped()];

            self.visible_tree_objects.replace_with(Property::computed(
                move || {
                    let mut tree = tree.get();
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
        // move this logic to engine (expose manifest ver as a prop)
        let manifest_ver = {
            let dt = ctx.designtime.borrow();
            dt.get_manifest_version()
        };
        if manifest_ver != self.old_manifest_ver.get() {
            self.old_manifest_ver.set(manifest_ver);
        }
    }
}

fn get_tree(type_id: TypeId, ctx: &NodeContext) -> Vec<FlattenedTreeEntry> {
    let dt = ctx.designtime.borrow_mut();
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
    let node = component_template.get_node(tnid).unwrap();
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
    match import_path.trim_start_matches("pax_designer::pax_reexports::pax_std::primitives::") {
        "Group" => Desc::Group,
        "Frame" => Desc::Frame,
        "Ellipse" => Desc::Ellipse,
        "Text" => Desc::Text,
        "Stacker" => Desc::Stacker,
        "Rectangle" => Desc::Rectangle,
        "Path" => Desc::Path,
        "Textbox" => Desc::Textbox,
        "Checkbox" => Desc::Checkbox,
        "Scroller" => Desc::Scroller,
        "Button" => Desc::Button,
        "Image" => Desc::Image,
        "Slider" => Desc::Slider,
        "Dropdown" => Desc::Dropdown,
        _ => Desc::Component(format!("{}", type_id.get_pax_type())),
    }
}
