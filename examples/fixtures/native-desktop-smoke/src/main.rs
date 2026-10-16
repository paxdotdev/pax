fn main() -> Result<(), pax_kit::host::Error> {
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    if std::env::args().any(|argument| argument == "--prepare-drop-async") {
        for cycle in 1..=2 {
            let app = pax_kit::init_with!(pax_kit::host::InitOptions {
                startup_data: b"pax-test-prepare-drop-async".to_vec(),
                ..Default::default()
            })?;
            let started = std::time::Instant::now();
            drop(app);
            let elapsed = started.elapsed();
            if elapsed < std::time::Duration::from_millis(250) {
                return Err(pax_kit::host::Error::message(
                    "Prelaunch drop returned before the diagnostic cleanup acknowledgement",
                ));
            }
            println!(
                "prepare-drop-async: cycle {cycle} retired after {}ms",
                elapsed.as_millis()
            );
        }
        // Drop cannot return an error: a final prepare must reject a timed-out
        // second cleanup instead of letting this diagnostic report success.
        drop(pax_kit::init!()?);
        println!("prepare-drop-async: two acknowledged native sessions retired successfully");
        return Ok(());
    }
    if std::env::args().any(|argument| argument == "--prepare-drop") {
        // Exercise session retirement before a platform window or GPU exists.
        for _ in 0..2 {
            drop(pax_kit::init!()?);
        }
        println!("prepare-drop: two native sessions retired successfully");
        return Ok(());
    }
    let options = pax_kit::host::InitOptions {
        startup_data: b"Initialized by user-owned main".to_vec(),
        ..Default::default()
    };
    pax_kit::init_with!(options)?.run_with(pax_kit::host::WindowOptions {
        title: Some("Pax native desktop acceptance".into()),
        size: (800.0, 640.0),
        ..Default::default()
    })
}
