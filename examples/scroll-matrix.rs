fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(scroll_matrix)?.run()
}
