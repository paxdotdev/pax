fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(neon_opacity)?.run()
}
