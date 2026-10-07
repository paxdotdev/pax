fn main() -> Result<(), pax_kit::host::Error> {
    // Process setup belongs here; the cartridge initializer owns UI dependencies.
    let application = pax_kit::init!(async_workbench)?;
    #[cfg(target_os = "macos")]
    return application.run_with(pax_kit::host::WindowOptions {
        title: Some("Async Workbench · Rust main".into()),
        size: (1160.0, 900.0),
        minimum_size: (600.0, 500.0),
        ..Default::default()
    });
    #[cfg(target_os = "ios")]
    application.run()
}
