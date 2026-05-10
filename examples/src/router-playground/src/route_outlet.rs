#![allow(unused_imports)]

use crate::{GuidePanel, RouteInspector, TeamPanel};
use pax_kit::*;

#[pax]
#[file("route_outlet.pax")]
pub struct RouteOutlet {
    pub is_mobile: Property<bool>,
}
