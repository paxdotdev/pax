fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!()?.run_with(pax_kit::host::WindowOptions {
        title: Some("Pax native scroll snapping".into()),
        size: (1024.0, 680.0),
        ..Default::default()
    })
}
