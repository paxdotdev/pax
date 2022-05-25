#[macro_use]
extern crate lazy_static;

use pax::*;
use pax::api::{ArgsCoproduct, ArgsRender, Property, ArgsClick, EasingCurve};

use pax_std::primitives::{Group, Rectangle};

pub mod pax_types {
    pub mod pax_std {
        pub mod primitives {
            pub use pax_std::primitives::Rectangle;
            pub use pax_std::primitives::Group;
            pub use pax_std::primitives::Text;
        }
        pub mod components {
            pub use pax_std::components::Stacker;
        }
        pub mod types {
            pub use pax_std::types::StackerCellProperties;
            pub use pax_std::types::Color;
            pub use pax_std::types::Font;
            pub use pax_std::types::Stroke;
            pub use pax_std::types::Size;
            pub use pax_std::types::StackerDirection;
        }
    }
    pub use pax::api::Transform2D;
    pub use pax::api::SizePixels;

    pub use crate::Root;
}



#[derive(Default)]
pub struct Root {
    pub num_clicks: Property<isize>,
    pub current_rotation: Property<f64>,
}

impl Root {
    pub fn handle_pre_render(&mut self, args: ArgsRender) {
        if args.frames_elapsed % 180 == 0 {
            //every 3s
            pax::log(&format!("pax::log from frame {}", args.frames_elapsed));
        }
    }

    pub fn handle_click(&mut self, args: ArgsClick) {
        let new_rotation = self.current_rotation.get() + (2.0 * std::f64::consts::PI);
        self.current_rotation.ease_to(new_rotation, 120, EasingCurve::InOutBack );
        self.current_rotation.ease_to_later(0.0, 40, EasingCurve::OutBack );
    }
}


