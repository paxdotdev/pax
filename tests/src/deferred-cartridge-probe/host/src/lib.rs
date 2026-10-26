use pax_kit::*;
use std::cell::{Cell, RefCell};
mod bundle;
#[cfg(feature = "monolithic")]
use bundle::probe_bind;
#[allow(unused_imports)]
use bundle::DeferredOutlet;

#[cfg(feature = "monolithic")]
#[path = "../../logic.rs"]
mod logic;

pax_kit::runnable! { root: crate::Probe }

// wasm-bindgen removes the linker's raw table export. Its supported intrinsic
// keeps and exposes the actual function table without rewriting the WASM file.
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn probe_table() -> wasm_bindgen::JsValue {
    wasm_bindgen::function_table()
}

thread_local! {
    static VALUES: RefCell<Vec<Option<LocalProperty<u32>>>> = const { RefCell::new(Vec::new()) };
    static READY: RefCell<Option<Property<bool>>> = const { RefCell::new(None) };
    static HANDLER: Cell<u32> = const { Cell::new(0) };
    static LAST_INSTANCE: Cell<u32> = const { Cell::new(0) };
    static MOUNTS: Cell<u32> = const { Cell::new(0) };
    static UNMOUNTS: Cell<u32> = const { Cell::new(0) };
}

#[no_mangle]
pub extern "C" fn probe_read(handle: u32) -> u32 {
    let value = VALUES.with(|v| {
        v.borrow()
            .get(handle.wrapping_sub(1) as usize)
            .cloned()
            .flatten()
    });
    value.map(|v| v.get()).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn probe_write(handle: u32, value: u32) -> u32 {
    let property = VALUES.with(|v| {
        v.borrow()
            .get(handle.wrapping_sub(1) as usize)
            .cloned()
            .flatten()
    });
    if let Some(property) = property {
        property.set(value);
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn probe_register(slot: u32, abi: u32, capabilities: u32) -> u32 {
    if abi != 2
        || capabilities != 0
        || slot == 0
        || HANDLER.with(Cell::get) != 0
        || !bundle::staged()
    {
        return 0;
    }
    HANDLER.with(|h| h.set(slot));
    READY.with(|r| {
        if let Some(p) = r.borrow().as_ref() {
            p.set(true);
        }
    });
    1
}

fn call(op: u32, a: u32, b: u32) -> u32 {
    #[cfg(feature = "monolithic")]
    unsafe {
        logic::dispatch(op, a, b)
    }
    #[cfg(not(feature = "monolithic"))]
    {
        let slot = HANDLER.with(Cell::get);
        if slot == 0 {
            return 0;
        }
        // Fixture-only contract: the trusted same-build loader installs a WASM
        // (i32,i32,i32)->i32 function first. WASM checks its signature on call.
        let function: unsafe extern "C" fn(u32, u32, u32) -> u32 =
            unsafe { std::mem::transmute(slot as usize) };
        unsafe { function(op, a, b) }
    }
}

#[no_mangle]
pub extern "C" fn probe_stats(which: u32) -> u32 {
    match which {
        0 => MOUNTS.with(Cell::get),
        1 => UNMOUNTS.with(Cell::get),
        2 => LAST_INSTANCE.with(Cell::get),
        3 => call(4, LAST_INSTANCE.with(Cell::get), 0),
        4 => cfg!(feature = "monolithic") as u32,
        _ => 0,
    }
}

#[no_mangle]
pub extern "C" fn probe_stale_click(token: u32) -> u32 {
    call(2, token, 0)
}

#[inline(never)]
fn direct_math(a: u32, b: u32) -> u32 {
    a.wrapping_mul(1664525).wrapping_add(b ^ 1013904223)
}

#[no_mangle]
pub extern "C" fn probe_bench(iterations: u32, mode: u32, seed: u32) -> u32 {
    let mut value = seed;
    for i in 0..iterations {
        value = if mode == 0 {
            direct_math(value, i)
        } else {
            call(5, value, i)
        };
    }
    value
}

#[no_mangle]
pub extern "C" fn probe_event_bench(iterations: u32) -> u32 {
    let token = LAST_INSTANCE.with(Cell::get);
    for _ in 0..iterations {
        if call(2, token, 0) == 0 {
            return 0;
        }
    }
    probe_read(1)
}

#[pax]
#[file("lib.pax")]
pub struct Probe {
    pub count: Property<u32>,
    pub step: Property<u32>,
    pub ready: Property<bool>,
    pub visible: Property<bool>,
    pub warm: Property<bool>,
}

impl Probe {
    pub fn mount(&mut self, ctx: &NodeContext) {
        bundle::capture_primitives(ctx);
        self.step.set(1);
        self.visible.set(true);
        VALUES.with(|v| *v.borrow_mut() = vec![Some(self.count.local()), Some(self.step.local())]);
        READY.with(|r| *r.borrow_mut() = Some(self.ready.clone()));
        if cfg!(feature = "monolithic") {
            bundle::stage_embedded();
            self.ready.set(true);
        }
    }
    pub fn reset(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.count.set(0);
    }
    pub fn change_step(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.step.set(if self.step.get() == 1 { 7 } else { 1 });
    }
    pub fn toggle(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.visible.set(!self.visible.get());
    }
    pub fn theme(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.warm.set(!self.warm.get());
    }
}
