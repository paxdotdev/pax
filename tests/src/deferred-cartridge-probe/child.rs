#![no_std]

mod logic;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}

#[no_mangle]
pub extern "C" fn probe_abi() -> u32 {
    2
}

#[no_mangle]
pub extern "C" fn probe_capabilities() -> u32 {
    0
}

#[no_mangle]
pub unsafe extern "C" fn probe_dispatch(op: u32, a: u32, b: u32) -> u32 {
    logic::dispatch(op, a, b)
}

static DEFINITION: &[u8] = include_bytes!(env!("PROBE_DEFINITION"));
static DESCRIPTOR: &[u8] = include_bytes!(env!("PROBE_DESCRIPTOR"));
#[no_mangle]
pub extern "C" fn probe_definition_ptr() -> *const u8 {
    DEFINITION.as_ptr()
}
#[no_mangle]
pub extern "C" fn probe_definition_len() -> usize {
    DEFINITION.len()
}
#[no_mangle]
pub extern "C" fn probe_descriptor_ptr() -> *const u8 {
    DESCRIPTOR.as_ptr()
}
#[no_mangle]
pub extern "C" fn probe_descriptor_len() -> usize {
    DESCRIPTOR.len()
}
