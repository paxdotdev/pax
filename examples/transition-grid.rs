fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(transition_grid)?.run()
}
