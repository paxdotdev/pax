pub mod pax_reexports { 
	pub use crate::HelloRGB;
	pub use f64;
	pub mod pax_std {
		pub mod primitives {
			pub use pax_std::primitives::Ellipse;
		}
		pub mod types {
			pub use pax_std::types::Color;
			pub use pax_std::types::Stroke;
		}
	}
}