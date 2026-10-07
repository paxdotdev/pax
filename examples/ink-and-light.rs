fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(ink_and_light)?.run()
}
