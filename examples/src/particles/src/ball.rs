use pax_engine::*;
use pax_engine::api::*;

#[pax]
pub struct Ball {
    pub x: Property<isize>,
    pub y: Property<isize>,
    pub magnitude: Property<f64>,
}