use pax_manifest::PaxManifest;

fn main() {
    println!("started");
    env_logger::init();
    log::debug!("tiis is a log debug msg");
    let manifest: PaxManifest =
        serde_json::from_str(include_str!("../initial_manifest.json")).unwrap();
    let mut designer = pax_designtime::DesigntimeManager::new(manifest);
    designer.send_manifest_update().unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));
    println!("stopped");
}
