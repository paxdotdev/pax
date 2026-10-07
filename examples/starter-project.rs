fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(starter_project)?.run()
}
