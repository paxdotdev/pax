mod app;
pub use app::App;
use pax_kit::*;

pub fn init(context: &InitContext) -> AppResult<App> {
    let root = App::default();
    root.count.set(if context.startup_data().is_empty() {
        0
    } else {
        std::str::from_utf8(context.startup_data())?.parse()?
    });
    let count = root.count.clone();
    context.on_host_request(move |bytes| {
        if bytes == b"increment" { count.update(|value| *value += 1); }
        Ok(count.get().to_string().into_bytes())
    })?;
    Ok(root)
}
