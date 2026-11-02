fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!()?.run_with(pax_kit::host::WindowOptions {
        title: Some("Pax native text layout".into()),
        size: (1000.0, 1000.0),
        ..Default::default()
    })
}
