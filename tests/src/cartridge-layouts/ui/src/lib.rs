pub mod ui;
// A consuming cartridge must be able to name every reflected component.
pub use layout_controls::Counter;

pax_kit::runnable! { root: crate::ui::App, init: crate::ui::init }
