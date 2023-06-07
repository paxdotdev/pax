pub mod pax_reexports {
    pub use crate::Example;
    pub mod camera {
        pub use crate::camera::Camera;
        pub use crate::camera::TypeExample;
    }
    pub mod fireworks {
        pub use crate::fireworks::Fireworks;
    }
    pub mod hello_rgb {
        pub use crate::hello_rgb::HelloRGB;
    }
    pub use f64;
    pub mod pax_std{
        pub mod primitives{
            pub use pax_std::primitives::Ellipse;
            pub use pax_std::primitives::Frame;
            pub use pax_std::primitives::Group;
            pub use pax_std::primitives::Rectangle;
        }
        pub mod types{
            pub use pax_std::types::Color;
            pub use pax_std::types::Stroke;
        }
    }
    pub use usize;

}