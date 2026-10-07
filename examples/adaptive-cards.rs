fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(adaptive_cards)?.run()
}
