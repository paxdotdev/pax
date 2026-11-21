//! Website-only handoff. The HTML document owns consent and the Google tag.
use pax_kit::*;

pub(crate) fn action(name: &str, placement: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::{JsCast, JsValue};
        let window = js_sys::global();
        if let Ok(callback) = js_sys::Reflect::get(&window, &"paxSiteAction".into()) {
            if let Some(callback) = callback.dyn_ref::<js_sys::Function>() {
                let _ = callback.call2(&JsValue::UNDEFINED, &name.into(), &placement.into());
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (name, placement);
}

pub(crate) fn watch_drawer(ctx: &NodeContext, open: &Property<bool>, section: &'static str) {
    #[cfg(target_arch = "wasm32")]
    {
        let open = open.local();
        // The cube starts open by design. Only count transitions from closed to open.
        let previous = std::cell::Cell::new(open.get());
        ctx.subscribe(&[open.untyped()], move || {
            let current = open.get();
            if !previous.replace(current) && current {
                action("view_source", section);
            }
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (ctx, open, section);
}
