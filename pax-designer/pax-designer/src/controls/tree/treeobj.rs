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
    pub selected: Property<bool>,
    pub collapsed: Property<bool>,
    pub img_path: Property<String>,
    pub leaf: Property<bool>,
    pub not_leaf: Property<bool>,
}

impl TreeObj {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        self.pre_render(ctx);
    }

    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        self.img_path.set(
            match *self.collapsed.get() {
                true => "assets/icons/tree_view/collapse_arrow_collapsed.png",
                false => "assets/icons/tree_view/collapse_arrow.png",
            }
            .into(),
        );
        self.not_leaf.set(!self.leaf.get());
    }

    pub fn clicked(&mut self, _ctx: &NodeContext, _args: ArgsClick) {
        *super::TREE_SENDER.get().unwrap().lock().unwrap() =
            Some((self.ind.get().clone().into(), super::TreeMessage::Clicked));
    }
}
