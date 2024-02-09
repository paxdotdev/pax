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

#[pax]
#[main]
#[file("lib.pax")]
pub struct PaxDesigner {}

impl PaxDesigner {
    pub fn handle_mount(&mut self, _ctx: &NodeContext) {}

    pub fn click_test(&mut self, _ctx: &NodeContext, args: ArgsClick) {}
}
