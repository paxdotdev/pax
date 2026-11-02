fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(native_scroll_snap)?.run()
}
