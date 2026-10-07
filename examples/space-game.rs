fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(space_game)?.run()
}
