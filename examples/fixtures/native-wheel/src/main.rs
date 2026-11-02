fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!()?.run_with(pax_kit::host::WindowOptions {
        title: Some("Pax native wheel input".into()),
        size: (1024.0, 780.0),
        ..Default::default()
    })
}
