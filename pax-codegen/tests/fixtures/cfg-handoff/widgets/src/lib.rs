#![deny(unexpected_cfgs)]
use pax_kit::*;
pub use cfg_leaf::Leaf;

// This first textual component must never become the helper emitter.
#[cfg(any())]
#[pax]
#[file("does-not-exist.pax")]
pub struct DisabledFirst {}

#[cfg(any())]
mod missing;

#[cfg_attr(feature = "fancy", path = "fancy.rs")]
#[cfg_attr(not(feature = "fancy"), path = "plain.rs")]
pub mod selected;
pub use selected::Widget;

#[pax]
#[inlined(<Widget/>)]
pub struct Wrapper {}

// Every active component should agree on one provider, even in other modules.
pub mod another {
    use pax_kit::*;
    #[pax]
    pub struct Other {}
}
