//! Small linkage model only; this is not Pax's production engine or ABI.
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct Program {
    pub id: &'static str,
    pub initialize: extern "C" fn() -> u64,
    pub count: fn(usize) -> usize,
}

pub struct Engine(pub u64);
static LIVE: AtomicUsize = AtomicUsize::new(0);

pub fn create(program: Program) -> *mut Engine {
    let engine = Box::new(Engine((program.initialize)()));
    LIVE.fetch_add(1, Ordering::SeqCst);
    Box::into_raw(engine)
}

/// The pointer must originate from create in this linked runtime.
pub unsafe fn destroy(engine: *mut Engine) {
    if !engine.is_null() {
        drop(Box::from_raw(engine));
        LIVE.fetch_sub(1, Ordering::SeqCst);
    }
}

pub fn live() -> usize { LIVE.load(Ordering::SeqCst) }

// These identical source-level function names remain ordinary Rust symbols.
// The generated programs can also have private, crate-local setup functions.
#[macro_export]
macro_rules! declare_program {
    ($id:literal, $value:expr, $counts:expr) => {
        static CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        pub fn component() -> u64 { $value }
        fn application_setup() -> u64 { component() }
        pub extern "C" fn pax_init_entry() -> u64 {
            CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            application_setup()
        }
        pub fn initializations() -> usize {
            CALLS.load(std::sync::atomic::Ordering::SeqCst)
        }
        pub fn program() -> $crate::Program {
            $crate::Program { id: $id, initialize: pax_init_entry, count: $counts }
        }
    };
}

// A real adapter must own the entire exported surface, including runtime
// functions. Here it owns a small representative init/use/free interface.
#[macro_export]
macro_rules! export_program {
    ($selected:path) => {
        #[no_mangle]
        pub extern "C" fn pax_cartridge_abi_version() -> u32 { 1 }
        #[no_mangle]
        pub unsafe extern "C" fn pax_cartridge_program_id(out: *mut u8, capacity: usize) -> usize {
            let id = $selected().id.as_bytes();
            if !out.is_null() && capacity >= id.len() {
                std::ptr::copy_nonoverlapping(id.as_ptr(), out, id.len());
            }
            id.len()
        }
        #[no_mangle]
        pub unsafe extern "C" fn pax_init_entry(entry: *const u8, length: usize) -> *mut $crate::Engine {
            if length > 128 || (length > 0 && entry.is_null()) { return std::ptr::null_mut(); }
            let selected = $selected();
            if length > 0 && std::slice::from_raw_parts(entry, length) != selected.id.as_bytes() {
                return std::ptr::null_mut();
            }
            $crate::create(selected)
        }
        #[no_mangle]
        pub unsafe extern "C" fn pax_tick(engine: *mut $crate::Engine, delta: u64) -> u64 {
            (*engine).0 += delta;
            (*engine).0
        }
        #[no_mangle]
        pub unsafe extern "C" fn pax_dealloc_engine(engine: *mut $crate::Engine) {
            $crate::destroy(engine);
        }
        #[no_mangle]
        pub extern "C" fn probe_count(index: usize) -> usize { ($selected().count)(index) }
        #[no_mangle]
        pub extern "C" fn probe_live() -> usize { $crate::live() }
    };
}
