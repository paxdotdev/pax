fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(scroll_cull_stress)?.run()
}
