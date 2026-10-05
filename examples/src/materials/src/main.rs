// pax-cli builds this executable separately from the UI cartridge.
#[cfg(any(target_os = "macos", target_os = "ios"))]
fn main() -> Result<(), pax_host::Error> {
    pax_host::init()?.run()
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
fn main() {
    compile_error!("Native standalone entry supports macOS and iOS. For web use pax-cli run --target web without the standalone feature.");
}
