fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(tokyo_metro)?.run()
}
