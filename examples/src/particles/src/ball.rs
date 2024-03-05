use pax_engine::*;
use pax_engine::api::*;

use pax_std::primitives::Ellipse;

#[pax]
#[file("ball.pax")]
pub struct Ball {
    pub magnitude: Property<Numeric>,

}

