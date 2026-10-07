fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(timeline_playground)?.run()
}
