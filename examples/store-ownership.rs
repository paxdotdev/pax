fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(store_ownership)?.run()
}
