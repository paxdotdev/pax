fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(responsive_helpers)?.run()
}
