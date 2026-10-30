fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(light_space)?.run()
}
