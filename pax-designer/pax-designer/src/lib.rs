#![allow(unused_imports)]

use pax_lang::*;
use pax_lang::api::*;

pub mod glass;
pub mod controls;
pub mod designtime_component_viewer;

use crate::glass::Glass;
use crate::controls::Controls;
use crate::designtime_component_viewer::DesigntimeComponentViewer;

#[derive(Pax)]
#[main]
#[file("lib.pax")]
pub struct PaxDesigner {
    pub active_component_id: Property<String>,
}
