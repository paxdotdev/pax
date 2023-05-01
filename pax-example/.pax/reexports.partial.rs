pub mod pax_reexports { 
	pub use crate::Example;
	pub mod fireworks {
		pub use crate::fireworks::Fireworks;
	}
	pub mod hello_rgb {
		pub use crate::hello_rgb::HelloRGB;
	}
	pub mod words {
		pub use crate::words::Words;
	}
	pub use f64;
	pub mod pax_std {
		pub mod primitives {
			pub use pax_std::primitives::Ellipse;
			pub use pax_std::primitives::Group;
			pub use pax_std::primitives::Rectangle;
			pub use pax_std::primitives::Text;
		}
		pub mod types {
			pub use pax_std::types::Alignment;
			pub use pax_std::types::Color;
			pub use pax_std::types::Font;
			pub use pax_std::types::Stroke;
		}
	}
	pub mod std {
		pub mod string {
			pub use std::string::String;
		}
		pub mod vec {
			pub use std::vec::Vec;
		}
	}
	pub use usize;
}