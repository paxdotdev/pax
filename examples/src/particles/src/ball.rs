use pax_engine::*;
use pax_engine::api::*;

use pax_std::primitives::Ellipse;

#[pax]
#[file("ball.pax")]
pub struct Ball {
    pub magnitude: Property<Numeric>,
    pub effective_diameter: Property<Numeric>,
}

impl Ball {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {

        //look up my special ID in static


        // let m = self.magnitude.get();

        //PROBLEM: on mount, self.magnitude.get() is returning the Default, not-yet-initialized value
        //         options include: rejigger mount / tick / update order
        self.effective_diameter.set(Numeric::from(1.5 * self.magnitude.get()));


        //animated
    }
}
