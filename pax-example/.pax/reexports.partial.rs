pub mod pax_reexports { 
	pub mod core {
		pub mod option {
			pub use core::option::Option;
		}
	}
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
			pub use pax_std::types::Color;
			pub use pax_std::types::Stroke;
			pub mod text {
				pub use pax_std::types::text::Font;
				pub use pax_std::types::text::FontStyle;
				pub use pax_std::types::text::FontWeight;
				pub use pax_std::types::text::LinkStyle;
				pub use pax_std::types::text::SizeWrapper;
				pub use pax_std::types::text::TextAlignHorizontal;
				pub use pax_std::types::text::TextAlignVertical;
			}
		}
	}
	pub mod std {
		pub mod string {
			pub use std::string::String;
		}
	}
	pub use usize;
}