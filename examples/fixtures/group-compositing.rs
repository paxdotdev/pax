fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(group_compositing)?.run()
}
