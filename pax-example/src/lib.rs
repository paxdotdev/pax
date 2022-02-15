use pax::*;
use pax_std::{Group, Rectangle};

pub struct DeeperStruct {
    a: i64,
    b: &'static str,
}

#[pax]
pub struct Root {
    pub num_clicks : i64,
    pub current_rotation: f64,
    pub deeper_struct: DeeperStruct,
}

impl Root {
    pub fn new() -> Self {
        Self {
            num_clicks: 0,
            current_rotation: 0.0,
            deeper_struct: DeeperStruct {
                a: 100,
                b: "Profundo!",
            }
        }
    }

    pub fn handle_tick(evt: EventTick) {
        //continued in lib-expanded.rs
    }
}
