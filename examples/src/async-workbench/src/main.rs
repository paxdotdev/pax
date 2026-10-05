fn main() -> Result<(), pax_host::Error> {
    // Process setup belongs here; the cartridge initializer owns UI dependencies.
    let application = pax_host::init()?;
    #[cfg(target_os = "macos")]
    return application.run_with(pax_host::WindowOptions {
        title: Some("Async Workbench · Rust main".into()),
        size: (1160.0, 900.0),
        minimum_size: (600.0, 500.0),
        ..Default::default()
    });
    #[cfg(target_os = "ios")]
    application.run()
}
