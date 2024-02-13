#![allow(unused_imports)]

use pax_engine::api::*;
use pax_engine::*;

pub mod controls;
pub mod designtime_component_viewer;
pub mod glass;
use crate::controls::Controls;
use crate::designtime_component_viewer::DesigntimeComponentViewer;
use crate::glass::Glass;
use designer_project::Example;
use pax_std::primitives::Group;

pub mod model;

pub const USERLAND_PROJECT_ID: &'static str = "userland_project";

#[pax]
#[main]
#[file("lib.pax")]
pub struct PaxDesigner {}

impl PaxDesigner {
    pub fn tick(&mut self, ctx: &NodeContext) {
        let container = ctx
            .runtime_context
            .get_expanded_nodes_by_id(USERLAND_PROJECT_ID);
        if let Some(userland_proj) = container.first() {
            let up_lp = userland_proj.layout_properties.borrow_mut();
            if let Some(lp) = up_lp.as_ref() {
                let screen_to_glass_transform = lp.computed_tab.transform.inverse();
                pax_engine::log(&format!(
                    "registered transform: {:?}",
                    screen_to_glass_transform
                ));
                model::register_glass_transform(screen_to_glass_transform);
            }
        }
    }
    pub fn handle_mount(&mut self, _ctx: &NodeContext) {}

    pub fn click_test(&mut self, _ctx: &NodeContext, args: ArgsClick) {}
}
