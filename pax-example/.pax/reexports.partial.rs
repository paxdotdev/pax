pub mod pax_reexports { 
	pub use crate::HelloRGB;
	pub mod pax_std {
		pub mod primitives {
			pub use pax_std::primitives::Group;
			pub use pax_std::primitives::Rectangle;
			pub use pax_std::primitives::Text;
		}
		pub mod types {
			pub use pax_std::types::Color;
			pub use pax_std::types::Font;
			pub use pax_std::types::Stroke;
		}
	}
	pub mod std {
		pub mod string {
			pub use std::string::String;
		}
	}
}