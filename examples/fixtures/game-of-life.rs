fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(game_of_life)?.run()
}
