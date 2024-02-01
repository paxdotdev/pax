use pax_lang::api::*;
use pax_lang::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::Text;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

use std::cell::{OnceCell, RefCell};
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock};
pub mod treeobj;
use std::collections::HashMap;
use treeobj::TreeObj;

#[pax]
#[file("controls/tree/tree.pax")]
pub struct Tree {
    pub tree_objects: Property<Vec<FlattenedTreeEntry>>,
    pub visible_tree_objects: Property<Vec<FlattenedTreeEntry>>,
    pub header_text: Property<String>,
    pub project_loaded: Property<bool>,
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
            Desc::Checkbox => ("Ceckbox", "10-checkbox"),
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
            collapsed: false,
            not_leaf: !self.1.is_empty(),
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
    pub collapsed: bool,
    pub not_leaf: bool,
}

impl Tree {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        self.header_text
            .set("Tree View: No loaded project".to_owned());
    }

    fn to_tree(root: usize, graph: &HashMap<usize, (String, Vec<usize>)>) -> TreeEntry {
        let (name, children) = &graph[&root];
        TreeEntry(
            match name.as_str() {
                "pax_std::primitives::Group" => Desc::Group,
                "pax_std::primitives::Frame" => Desc::Frame,
                "pax_std::primitives::Ellipse" => Desc::Ellipse,
                "pax_std::primitives::Text" => Desc::Text,
                "pax_std::primitives::Stacker" => Desc::Stacker,
                "pax_std::primitives::Rectangle" => Desc::Rectangle,
                "pax_std::primitives::Path" => Desc::Path,
                "pax_std::primitives::Textbox" => Desc::Textbox,
                "pax_std::primitives::Checkbox" => Desc::Checkbox,
                "pax_std::primitives::Scroller" => Desc::Scroller,
                "pax_std::primitives::Button" => Desc::Button,
                "pax_std::primitives::Image" => Desc::Image,
                "pax_std::primitives::Slider" => Desc::Slider,
                "pax_std::primitives::Dropdown" => Desc::Dropdown,
                other => Desc::Component(
                    other
                        .rsplit_once("::")
                        .unwrap_or(("", &other))
                        .1
                        .to_string(),
                ),
            },
            children
                .iter()
                .filter(|id| graph[id].0 != "COMMENT")
                .map(|&id| Self::to_tree(id, graph))
                .collect(),
        )
    }

    pub fn set_tree1(&mut self, ctx: &NodeContext, _args: ArgsButtonClick) {
        let type_id = "crate::controls::Controls";
        self.set_tree(type_id, ctx);
    }

    pub fn set_tree2(&mut self, ctx: &NodeContext, _args: ArgsButtonClick) {
        let type_id = {
            let dt = ctx.designtime.borrow();
            dt.get_orm().get_main_component().to_owned()
        };
        self.set_tree(&type_id, ctx);
    }

    pub fn set_tree(&mut self, type_id: &str, ctx: &NodeContext) {
        self.project_loaded.set(true);
        self.header_text.set("".to_owned());
        let dt = ctx.designtime.borrow_mut();
        let graph = dt
            .get_orm()
            .get_component(type_id)
            .expect("has template")
            .iter()
            .map(|(&k, v)| (k, (v.type_id.to_owned(), v.child_ids.to_owned())))
            .collect();
        let mut ind = 0;
        let flattened: Vec<FlattenedTreeEntry> = Self::to_tree(0, &graph)
            .1
            .into_iter()
            .flat_map(|v| v.flatten(&mut ind, 0))
            .collect();
        self.tree_objects.set(flattened.clone());
        self.visible_tree_objects.set(flattened);
    }

    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        let mut channel = TREE_CLICK_SENDER.lock().unwrap();
        if let Some(sender) = channel.take() {
            let tree = &mut self.tree_objects.get_mut();
            tree[sender].collapsed = !tree[sender].collapsed;
            let collapsed = tree[sender].collapsed;
            for i in (sender + 1)..tree.len() {
                if tree[sender].indent_level < tree[i].indent_level {
                    tree[i].visible = !collapsed;
                    tree[i].collapsed = collapsed;
                } else {
                    break;
                }
            }
            self.visible_tree_objects.set(
                self.tree_objects
                    .get()
                    .iter()
                    .filter(|o| o.visible)
                    .cloned()
                    .collect(),
            );
        }
    }
}
