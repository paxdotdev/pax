fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(native_text_style)?.run_with(pax_kit::host::WindowOptions {
        title: Some("Pax native text styling".into()),
        size: (1024.0, 840.0),
        ..Default::default()
    })
}
