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
    Component,
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
            Desc::Component => ("Component", "08-component"),
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
    fn container(desc: Desc, children: Vec<TreeEntry>) -> Self {
        TreeEntry(desc, children)
    }

    fn item(desc: Desc) -> Self {
        TreeEntry(desc, vec![])
    }

    fn flatten(self, ind: &mut usize, indent_level: usize) -> Vec<FlattenedTreeEntry> {
        let mut all = vec![];
        let (name, img_path) = self.0.info();
        all.push(FlattenedTreeEntry {
            name: StringBox::from(name),
            image_path: StringBox::from(img_path),
            ind: *ind,
            indent_level,
            visible: true,
            collapsed: false,
            leaf: self.1.is_empty(),
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

impl From<TreeEntry> for Vec<FlattenedTreeEntry> {
    fn from(value: TreeEntry) -> Self {
        value.flatten(&mut 0, 0)
    }
}

#[derive(Pax)]
#[custom(Imports)]
pub struct FlattenedTreeEntry {
    pub name: StringBox,
    pub image_path: StringBox,
    pub ind: usize,
    pub indent_level: usize,
    pub visible: bool,
    pub collapsed: bool,
    pub leaf: bool,
}

impl Tree {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        self.header_text
            .set("Tree View: No loaded project".to_owned());
    }

    pub fn set_tree1(&mut self, _ctx: &NodeContext, _args: ArgsButtonClick) {
        self.project_loaded.set(true);
        self.header_text.set("".to_owned());
        self.tree_objects.set(
            TreeEntry::container(
                Desc::Stacker,
                vec![
                    TreeEntry::container(
                        Desc::Component,
                        vec![
                            TreeEntry::item(Desc::Ellipse),
                            TreeEntry::item(Desc::Rectangle),
                            TreeEntry::container(
                                Desc::Scroller,
                                vec![TreeEntry::item(Desc::Text), TreeEntry::item(Desc::Path)],
                            ),
                        ],
                    ),
                    TreeEntry::item(Desc::Rectangle),
                    TreeEntry::item(Desc::Checkbox),
                    TreeEntry::container(
                        Desc::Scroller,
                        vec![
                            TreeEntry::container(
                                Desc::Dropdown,
                                vec![TreeEntry::item(Desc::Image), TreeEntry::item(Desc::Slider)],
                            ),
                            TreeEntry::item(Desc::Button),
                        ],
                    ),
                ],
            )
            .into(),
        );
        self.visible_tree_objects
            .set(self.tree_objects.get().clone());
    }

    pub fn set_tree2(&mut self, _ctx: &NodeContext, _args: ArgsButtonClick) {
        self.project_loaded.set(true);
        self.header_text.set("".to_owned());
        self.tree_objects.set(
            TreeEntry::container(
                Desc::Frame,
                vec![TreeEntry::container(
                    Desc::Group,
                    vec![
                        TreeEntry::item(Desc::Ellipse),
                        TreeEntry::item(Desc::Textbox),
                        TreeEntry::container(
                            Desc::Scroller,
                            vec![
                                TreeEntry::item(Desc::Text),
                                TreeEntry::item(Desc::Rectangle),
                            ],
                        ),
                        TreeEntry::item(Desc::Text),
                        TreeEntry::item(Desc::Rectangle),
                    ],
                )],
            )
            .into(),
        );
        self.visible_tree_objects
            .set(self.tree_objects.get().clone());
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
