use pax_lang::api::*;
use pax_lang::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::Text;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

use std::cell::RefCell;
use std::io::BufRead;
use std::rc::Rc;

#[derive(Pax)]
#[file("controls/tree/treeobj.pax")]
pub struct TreeObj {
    pub ind: Property<Numeric>,
    pub name: Property<StringBox>,
    pub image_path: Property<StringBox>,
    pub selected: Property<bool>,
    pub collapsed: Property<bool>,
    pub arrow_path: Property<String>,
    pub not_leaf: Property<bool>,
}

impl TreeObj {
    pub fn on_mount(&mut self, ctx: &NodeContext) {}

    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        self.arrow_path.set(
            match *self.collapsed.get() {
                true => "assets/icons/tree/collapse_arrow_collapsed.png",
                false => "assets/icons/tree/collapse_arrow.png",
            }
            .into(),
        );
    }

    pub fn clicked(&mut self, _ctx: &NodeContext, _args: ArgsClick) {
        *super::TREE_CLICK_SENDER.lock().unwrap() = Some(self.ind.get().clone().into());
    }
}
