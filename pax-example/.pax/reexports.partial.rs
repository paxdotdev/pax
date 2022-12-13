pub mod pax_reexports { 
	pub use crate::Jabberwocky;
	pub use f64;
	pub use i64;
	pub mod pax {
		pub mod api {
			pub use pax::api::Size;
		}
	}
	pub mod pax_std {
		pub mod primitives {
			pub use pax_std::primitives::Frame;
			pub use pax_std::primitives::Group;
			pub use pax_std::primitives::Rectangle;
			pub use pax_std::primitives::Text;
		}
		pub mod stacker {
			pub use pax_std::stacker::Stacker;
		}
		pub mod types {
			pub use pax_std::types::Color;
			pub use pax_std::types::Font;
			pub use pax_std::types::StackerCell;
			pub use pax_std::types::StackerDirection;
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