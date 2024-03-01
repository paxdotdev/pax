use pax_engine::api::*;
use pax_engine::*;
use pax_manifest::{ComponentTemplate, PaxType, TemplateNodeId, TypeId};
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::Text;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::text::*;
use pax_std::types::*;

use std::cell::{OnceCell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock};
pub mod treeobj;
use serde::Deserialize;
use std::collections::HashMap;
use treeobj::TreeObj;

use crate::model;

#[pax]
#[file("controls/tree/mod.pax")]
pub struct Tree {
    pub tree_objects: Property<Vec<FlattenedTreeEntry>>,
    pub visible_tree_objects: Property<Vec<FlattenedTreeEntry>>,
    pub is_project_loaded: Property<bool>,
}

pub static TREE_CLICK_SENDER: Mutex<Option<usize>> = Mutex::new(None);

struct TreeEntry(Desc, Vec<TreeEntry>);

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
        let (name, img_path) = self.0.info();
        all.push(FlattenedTreeEntry {
            name: StringBox::from(name),
            image_path: StringBox::from(img_path),
            ind: *ind,
            indent_level,
            visible: true,
            is_collapsed: false,
            is_not_leaf: !self.1.is_empty(),
        });
        *ind += 1;
        all.extend(
            self.1
                .into_iter()
                .flat_map(|c| c.flatten(ind, indent_level + 1)),
        );
        all
    }
}

#[pax]
#[custom(Imports)]
pub struct FlattenedTreeEntry {
    pub name: StringBox,
    pub image_path: StringBox,
    pub ind: usize,
    pub indent_level: isize,
    pub visible: bool,
    pub is_collapsed: bool,
    pub is_not_leaf: bool,
}

impl Tree {
    fn to_tree(tnid: &TemplateNodeId, component_template: &ComponentTemplate) -> Option<TreeEntry> {
        let node = component_template.get_node(tnid).unwrap();
        if node.type_id.get_pax_type() == &PaxType::Comment {
            return None;
        }
        let children = component_template
            .get_children(tnid)
            .unwrap_or_default()
            .iter()
            .filter_map(|c_tnid| Self::to_tree(c_tnid, component_template))
            .collect();
        let node_type = Self::resolve_tree_type(node.type_id.clone());
        Some(TreeEntry(node_type, children))
    }

    fn resolve_tree_type(type_id: TypeId) -> Desc {
        let Some(import_path) = type_id.import_path() else {
            return Desc::Component(format!("{:?}", type_id.get_pax_type()));
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
            _ => Desc::Component(
                type_id
                    .get_pascal_identifier()
                    .unwrap_or("ERROR: NO PASCAL IDENT".to_string()),
            ),
        }
    }

    pub fn set_tree(&mut self, type_id: TypeId, ctx: &NodeContext) {
        self.is_project_loaded.set(true);
        let dt = ctx.designtime.borrow_mut();
        let Ok(comp) = dt.get_orm().get_component(&type_id) else {
            pax_engine::log::warn!("couldn't find component for tree view");
            return;
        };
        let Some(template) = comp.template.as_ref() else {
            pax_engine::log::warn!("treeview component template embty");
            return;
        };
        let mut ind = 0;
        let flattened: Vec<FlattenedTreeEntry> = template
            .get_root()
            .iter()
            .flat_map(|tnid| {
                let tree = Self::to_tree(tnid, &template);
                tree.map(|t| t.flatten(&mut ind, 0)).unwrap_or_default()
            })
            .collect();
        self.tree_objects.set(flattened.clone());
        self.visible_tree_objects.set(flattened);
    }

    pub fn pre_render(&mut self, ctx: &NodeContext) {
        // let mut channel = TREE_CLICK_SENDER.lock().unwrap();
        // if let Some(sender) = channel.take() {
        //     let tree = &mut self.tree_objects.get_mut();
        //     tree[sender].collapsed = !tree[sender].collapsed;
        //     let collapsed = tree[sender].collapsed;
        //     for i in (sender + 1)..tree.len() {
        //         if tree[sender].indent_level < tree[i].indent_level {
        //             tree[i].visible = !collapsed;
        //             tree[i].collapsed = collapsed;
        //         } else {
        //             break;
        //         }
        //     }
        //     self.visible_tree_objects.set(
        //         self.tree_objects
        //             .get()
        //             .iter()
        //             .filter(|o| o.visible)
        //             .cloned()
        //             .collect(),
        //     );
        // }
        model::read_app_state(|app_state| {
            let type_id = &app_state.selected_component_id;
            self.set_tree(type_id.clone(), ctx);
        });
    }
}
