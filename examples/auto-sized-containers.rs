fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(auto_sized_containers)?.run()
}
