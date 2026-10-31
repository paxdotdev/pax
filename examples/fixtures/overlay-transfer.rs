fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(overlay_transfer)?.run()
}
