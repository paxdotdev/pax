pub mod pax_reexports { 
	pub use crate::HelloRGB;
	pub use f64;
	pub mod pax_std {
		pub mod primitives {
			pub use pax_std::primitives::Ellipse;
			pub use pax_std::primitives::Path;
			pub use pax_std::primitives::Rectangle;
			pub use pax_std::primitives::Text;
		}
		pub mod types {
			pub use pax_std::types::Color;
			pub use pax_std::types::Font;
			pub use pax_std::types::PathSegment;
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
}