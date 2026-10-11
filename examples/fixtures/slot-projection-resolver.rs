fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(slot_projection_resolver)?.run()
}
