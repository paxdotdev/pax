use pax_kit::*;
use std::cell::{Cell, RefCell};
#[path = "../../arena.rs"]
mod arena;
mod bridge;
mod bundle;
#[path = "../../events.rs"]
mod events;
mod fields;
#[path = "../../wire.rs"]
mod wire;
use bundle::DeferredPaxflix;

thread_local! {static READY:RefCell<Option<Property<bool>>>=const{RefCell::new(None)};static EAGER:Cell<bool>=const{Cell::new(false)};}
#[no_mangle]
pub extern "C" fn probe_eager(value: u32) {
    EAGER.with(|x| x.set(value != 0));
}
#[no_mangle]
pub extern "C" fn probe_activate() {
    READY.with(|r| r.borrow().as_ref().unwrap().set(true));
}
#[no_mangle]
pub extern "C" fn probe_unmount() {
    READY.with(|r| r.borrow().as_ref().unwrap().set(false));
}
#[pax]
#[file("shell.pax")]
pub struct Shell {
    pub ready: Property<bool>,
}
impl Shell {
    pub fn mount(&mut self, ctx: &NodeContext) {
        bundle::capture(ctx);
        READY.with(|r| *r.borrow_mut() = Some(self.ready.clone()));
        self.ready.set(EAGER.with(Cell::get));
    }
}
pax_kit::runnable! {root:crate::Shell}
