pub mod pax_reexports { 
	pub use crate::PaxExample;
	pub use f64;
	pub mod pax_std {
		pub mod primitives {
			pub use pax_std::primitives::Group;
			pub use pax_std::primitives::Rectangle;
		}
		pub mod types {
			pub use pax_std::types::Color;
			pub use pax_std::types::Stroke;
		}
	}
	pub mod std {
		pub mod vec {
			pub use std::vec::Vec;
		}
	}
	pub use usize;
}