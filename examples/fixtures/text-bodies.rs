fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(text_bodies)?.run()
}
