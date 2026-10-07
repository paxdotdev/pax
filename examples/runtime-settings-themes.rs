fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(runtime_settings_themes)?.run()
}
