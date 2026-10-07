fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(living_quilt)?.run()
}
