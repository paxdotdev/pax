extern crate self as pool_floor;
pub mod pool;
pub mod surface;
pub use pool::PoolFloor;

#[cfg(feature = "demo")]
pub mod demo;
#[cfg(feature = "demo")]
pub use demo::Example;
#[cfg(feature = "demo")]
pax_kit::runnable! { root: crate::demo::Example }
