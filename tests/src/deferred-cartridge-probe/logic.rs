// Same scalar ABI and component logic in the deferred and monolithic builds.
// No Rust allocation, vtable, string, or Property crosses the boundary.
#[cfg(not(feature = "monolithic"))]
#[link(wasm_import_module = "pax_probe")]
extern "C" {
    fn probe_bind(context: u32, field: u32) -> u32;
    fn probe_read(handle: u32) -> u32;
    fn probe_write(handle: u32, value: u32) -> u32;
}

#[cfg(feature = "monolithic")]
use crate::{probe_bind, probe_read, probe_write};

#[derive(Clone, Copy)]
struct Instance {
    token: u32,
    count: u32,
    step: u32,
    clicks: u32,
    clicks_property: u32,
}

const EMPTY: Instance = Instance {
    token: 0,
    count: 0,
    step: 0,
    clicks: 0,
    clicks_property: 0,
};
static mut INSTANCES: [Instance; 8] = [EMPTY; 8];
static mut GENERATION: u32 = 0;

// Deliberately single-threaded and non-reentrant: the host's property operations
// only read/write values and never synchronously call this module again.
#[inline(never)]
pub unsafe fn dispatch(op: u32, a: u32, b: u32) -> u32 {
    if op == 5 {
        return a.wrapping_mul(1664525).wrapping_add(b ^ 1013904223);
    }
    if op == 1 {
        for i in 0..8 {
            if INSTANCES[i].token == 0 {
                GENERATION = GENERATION.wrapping_add(1).max(1);
                let token = (GENERATION << 4) | (i as u32 + 1);
                INSTANCES[i] = Instance {
                    token,
                    count: probe_bind(a, 0),
                    step: probe_bind(a, 1),
                    clicks: 0,
                    clicks_property: probe_bind(a, 3),
                };
                return token;
            }
        }
        return 0;
    }
    let i = (a & 15).wrapping_sub(1) as usize;
    if i >= 8 || INSTANCES[i].token != a || a == 0 {
        return 0;
    }
    match op {
        2 => {
            let state = INSTANCES[i];
            let value = probe_read(state.count).wrapping_add(probe_read(state.step));
            if probe_write(state.count, value) == 0 {
                return 0;
            }
            INSTANCES[i].clicks += 1;
            probe_write(state.clicks_property, INSTANCES[i].clicks);
            1
        }
        3 => {
            INSTANCES[i] = EMPTY;
            1
        }
        4 => INSTANCES[i].clicks,
        _ => 0,
    }
}
