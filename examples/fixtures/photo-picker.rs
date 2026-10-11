fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(photo_picker)?.run()
}
