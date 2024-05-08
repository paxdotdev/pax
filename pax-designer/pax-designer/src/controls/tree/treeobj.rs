use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::Text;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

use std::cell::RefCell;
use std::io::BufRead;
use std::rc::Rc;

#[pax]
#[file("controls/tree/treeobj.pax")]
pub struct TreeObj {
    pub ind: Property<Numeric>,
    pub name: Property<String>,
    pub image_path: Property<String>,
    pub is_selected: Property<bool>,
    pub is_collapsed: Property<bool>,
    pub arrow_path: Property<String>,
    pub is_not_leaf: Property<bool>,
    pub is_not_dummy: Property<bool>,
}

impl TreeObj {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {}

    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        self.arrow_path.set(
            match self.is_collapsed.get() {
                true => "assets/icons/triangle-down.png",
                false => "assets/icons/triangle-right.png",
            }
            .into(),
        );
    }

    pub fn arrow_clicked(&mut self, _ctx: &NodeContext, _args: Event<Click>) {
        super::TREE_CLICK_PROP.with(|cn| {
            cn.set(super::TreeMsg::ArrowClicked(self.ind.get().clone().into()));
        })
    }
    pub fn obj_clicked(&mut self, _ctx: &NodeContext, _args: Event<Click>) {
        super::TREE_CLICK_PROP.with(|cn| {
            cn.set(super::TreeMsg::ObjClicked(self.ind.get().clone().into()));
        });
    }

    pub fn obj_double_clicked(&mut self, _ctx: &NodeContext, _args: Event<DoubleClick>) {
        super::TREE_CLICK_PROP.with(|cn| {
            cn.set(super::TreeMsg::ObjDoubleClicked(
                self.ind.get().clone().into(),
            ));
        });
    }
}
