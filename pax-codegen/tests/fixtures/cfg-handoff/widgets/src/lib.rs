#![deny(unexpected_cfgs)]
pub use cfg_leaf::Leaf;
use pax_kit::*;

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

pax_kit::runnable! { root: crate::Wrapper, init: crate::initialize_wrapper }

static INITIALIZATIONS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
fn initialize_wrapper(_: &InitContext) -> AppResult<Wrapper> {
    INITIALIZATIONS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    Ok(Wrapper::default())
}

pub fn initializer_count() -> usize {
    INITIALIZATIONS.load(std::sync::atomic::Ordering::SeqCst)
}

// Every active component should agree on one provider, even in other modules.
pub mod another {
    use pax_kit::*;
    #[pax]
    pub struct Other {}
}
