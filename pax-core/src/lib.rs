#[macro_use]
extern crate lazy_static;

pub use kurbo::{Affine};
pub use piet::{Color, StrokeStyle, Error};

pub use pax_properties_coproduct;

pub mod engine;
pub mod rendering;
pub mod expressions;
//Note: commented out components & primitives when moving
//      to compiled .pax instead of hand-written RIL.  Required
//      in order to get project to compile in absence of properly
//      generated PropertiesCoproduct
// mod components;
// mod primitives;
pub mod component;
pub mod repeat;
pub mod placeholder;
pub mod runtime;
// pub mod timelines;
pub mod designtime;

pub use crate::engine::*;
pub use crate::component::*;
// pub use crate::primitives::*;
pub use crate::rendering::*;
pub use crate::expressions::*;
// pub use crate::components::*;
pub use crate::runtime::*;
// pub use crate::timelines::*;
pub use crate::repeat::*;
pub use crate::placeholder::*;



