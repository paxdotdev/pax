fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(materials)?.run()
}
