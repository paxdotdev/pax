use pax_engine::api::*;
use pax_engine::pax_manifest::{TemplateNodeId, UniqueTemplateNodeIdentifier};
use pax_engine::*;
use pax_std::*;
use std::cell::RefCell;
use std::io::BufRead;
use std::rc::Rc;

use super::TREE_HIDDEN_NODES;

#[pax]
#[engine_import_prefix("pax_engine")]
#[file("controls/tree/treeobj.pax")]
pub struct TreeObj {
    pub ind: Property<Numeric>,
    pub name: Property<String>,
    pub image_path: Property<String>,
    pub is_selected: Property<bool>,
    pub is_collapsed: Property<bool>,
    pub arrow_path: Property<String>,
    pub is_container: Property<bool>,
    pub uid: Property<TemplateNodeId>,
}

impl TreeObj {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        // TODO: this is very inefficient, each tree obj is checking
        // this set each time any of them update collapsed state. (O(n**2) for a single node)
        let hidden = TREE_HIDDEN_NODES.with(|p| p.clone());
        let uid = self.uid.clone();
        let deps = [hidden.untyped(), uid.untyped()];
        self.is_collapsed.replace_with(Property::computed(
            move || hidden.get().contains(&uid.get()),
            &deps,
        ));
        let collapsed = self.is_collapsed.clone();
        let deps = [collapsed.untyped()];
        self.arrow_path.replace_with(Property::computed(
            move || {
                match collapsed.get() {
                    true => "assets/icons/triangle-right.png",
                    false => "assets/icons/triangle-down.png",
                }
                .into()
            },
            &deps,
        ));
    }

    pub fn arrow_clicked(&mut self, _ctx: &NodeContext, _args: Event<MouseDown>) {
        let id = self.uid.get();
        super::TREE_HIDDEN_NODES.with(|p| {
            p.update(|v| {
                if v.contains(&id) {
                    v.remove(&id);
                } else {
                    v.insert(id);
                }
            });
        })
    }

    pub fn obj_double_clicked(&mut self, _ctx: &NodeContext, _args: Event<DoubleClick>) {
        super::TREE_CLICK_PROP.with_borrow_mut(|cn| {
            cn.push_back(super::TreeMsg::ObjDoubleClicked(
                self.ind.get().clone().into(),
            ));
        });
    }

    pub fn mouse_down(&mut self, _ctx: &NodeContext, event: Event<MouseDown>) {
        event.prevent_default();
        super::TREE_CLICK_PROP.with_borrow_mut(|cn| {
            cn.push_back(super::TreeMsg::ObjMouseDown(
                self.ind.get().clone().into(),
                event.mouse.x,
            ));
        });
    }

    pub fn mouse_move(&mut self, _ctx: &NodeContext, event: Event<MouseMove>) {
        event.prevent_default();
        super::TREE_CLICK_PROP.with_borrow_mut(|cn| {
            cn.push_back(super::TreeMsg::ObjMouseMove(
                self.ind.get().clone().into(),
                event.mouse.x,
            ));
        });
    }
}
