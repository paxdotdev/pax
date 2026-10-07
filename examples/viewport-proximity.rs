fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(viewport_proximity)?.run()
}
