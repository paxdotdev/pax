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
use treeobj::TreeObj;

#[derive(Pax)]
#[file("controls/tree/tree.pax")]
pub struct Tree {
    pub tree_objects: Property<Vec<TreeObjEntry>>,
    pub visible_tree_objects: Property<Vec<TreeObjEntry>>,
    pub active: Property<bool>,
}

pub enum TreeMessage {
    Clicked,
}

pub static TREE_SENDER: OnceLock<Arc<Mutex<Option<(usize, TreeMessage)>>>> = OnceLock::new();

impl Tree {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        let _ = TREE_SENDER.set(Arc::new(Mutex::new(None)));
    }

    pub fn set_tree1(&mut self, _ctx: &NodeContext, _args: ArgsButtonClick) {
        self.tree_objects.set(vec![
            TreeObjEntry {
                name: StringBox::from("Frame".to_owned()),
                ind: 0,
                indent_level: 0,
                collapsed: false,
                visible: true,
                leaf: false,
            },
            TreeObjEntry {
                name: StringBox::from("Group".to_owned()),
                ind: 1,
                indent_level: 1,
                collapsed: false,
                visible: true,
                leaf: false,
            },
            TreeObjEntry {
                name: StringBox::from("Ellipse".to_owned()),
                ind: 2,
                indent_level: 2,
                collapsed: false,
                visible: true,
                leaf: true,
            },
            TreeObjEntry {
                name: StringBox::from("Path".to_owned()),
                ind: 3,
                indent_level: 2,
                collapsed: false,
                visible: true,
                leaf: true,
            },
            TreeObjEntry {
                name: StringBox::from("Group".to_owned()),
                ind: 4,
                indent_level: 2,
                collapsed: false,
                visible: true,
                leaf: false,
            },
            TreeObjEntry {
                name: StringBox::from("Group".to_owned()),
                ind: 5,
                indent_level: 3,
                collapsed: false,
                visible: true,
                leaf: false,
            },
            TreeObjEntry {
                name: StringBox::from("Path".to_owned()),
                ind: 6,
                indent_level: 4,
                collapsed: false,
                visible: true,
                leaf: true,
            },
            TreeObjEntry {
                name: StringBox::from("Frame".to_owned()),
                ind: 7,
                indent_level: 0,
                collapsed: false,
                visible: true,
                leaf: false,
            },
            TreeObjEntry {
                name: StringBox::from("Rectangle".to_owned()),
                ind: 8,
                indent_level: 1,
                collapsed: false,
                visible: true,
                leaf: true,
            },
            TreeObjEntry {
                name: StringBox::from("Rectangle".to_owned()),
                ind: 9,
                indent_level: 1,
                collapsed: false,
                visible: true,
                leaf: true,
            },
        ]);
    }

    pub fn set_tree2(&mut self, _ctx: &NodeContext, _args: ArgsButtonClick) {
        self.tree_objects.set(vec![
            TreeObjEntry {
                name: StringBox::from("Group".to_owned()),
                ind: 0,
                indent_level: 0,
                collapsed: false,
                visible: true,
                leaf: false,
            },
            TreeObjEntry {
                name: StringBox::from("Ellipse".to_owned()),
                ind: 1,
                indent_level: 2,
                collapsed: false,
                visible: true,
                leaf: true,
            },
            TreeObjEntry {
                name: StringBox::from("Path".to_owned()),
                ind: 2,
                indent_level: 2,
                collapsed: false,
                visible: true,
                leaf: true,
            },
            TreeObjEntry {
                name: StringBox::from("Rect".to_owned()),
                ind: 3,
                indent_level: 2,
                collapsed: false,
                visible: true,
                leaf: true,
            },
            TreeObjEntry {
                name: StringBox::from("Frame".to_owned()),
                ind: 4,
                indent_level: 0,
                collapsed: false,
                visible: true,
                leaf: false,
            },
            TreeObjEntry {
                name: StringBox::from("Rectangle".to_owned()),
                ind: 5,
                indent_level: 1,
                collapsed: false,
                visible: true,
                leaf: true,
            },
            TreeObjEntry {
                name: StringBox::from("Frame".to_owned()),
                ind: 6,
                indent_level: 0,
                collapsed: false,
                visible: true,
                leaf: false,
            },
            TreeObjEntry {
                name: StringBox::from("Rectangle".to_owned()),
                ind: 7,
                indent_level: 1,
                collapsed: false,
                visible: true,
                leaf: false,
            },
        ]);
    }

    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        let mut channel = TREE_SENDER.get().unwrap().lock().unwrap();
        if let Some((sender, _message)) = channel.take() {
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

#[derive(Pax)]
#[custom(Imports)]
pub struct TreeObjEntry {
    pub name: StringBox,
    pub ind: usize,
    pub indent_level: usize,
    pub visible: bool,
    pub collapsed: bool,
    pub leaf: bool,
}
