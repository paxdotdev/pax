use pax_runtime::api::{Clipboard, ClipboardError};
use wasm_bindgen::prelude::*;

// Invoke writeText before yielding: Safari/Firefox require the originating user
// gesture. Only waiting for the Promise belongs in the async task.
#[wasm_bindgen(inline_js = r#"
export function pax_write_clipboard_text(text) {
    if (!globalThis.isSecureContext || !globalThis.navigator?.clipboard?.writeText) {
        throw new DOMException('Clipboard unavailable', 'NotSupportedError');
    }
    return navigator.clipboard.writeText(text);
}
"#)]
extern "C" {
    #[wasm_bindgen(catch)]
    fn pax_write_clipboard_text(text: &str) -> Result<js_sys::Promise, JsValue>;
}

fn error(value: JsValue) -> ClipboardError {
    match js_sys::Reflect::get(&value, &JsValue::from_str("name"))
        .ok()
        .and_then(|name| name.as_string())
        .as_deref()
    {
        Some("NotAllowedError" | "SecurityError") => ClipboardError::NotAllowed,
        Some("NotSupportedError") => ClipboardError::Unsupported,
        _ => ClipboardError::Failed,
    }
}

pub fn adapter() -> Clipboard {
    Clipboard::new(|text, reply| match pax_write_clipboard_text(&text) {
        Ok(promise) => wasm_bindgen_futures::spawn_local(async move {
            reply(
                wasm_bindgen_futures::JsFuture::from(promise)
                    .await
                    .map(|_| ())
                    .map_err(error),
            );
        }),
        Err(failure) => reply(Err(error(failure))),
    })
}
