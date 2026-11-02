fn main() -> Result<(), pax_kit::host::Error> {
    let options = pax_kit::host::InitOptions {
        entry: Some("crate::Shell".into()),
        ..Default::default()
    };
    pax_kit::init_with!(options)?.run()
}
