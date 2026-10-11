fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(route_card)?.run()
}
