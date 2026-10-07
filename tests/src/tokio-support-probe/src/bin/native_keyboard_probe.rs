//! Exercise Swift's keyboard wire format and cancellation through a real Calculator cartridge.
//! Pass the built macOS PaxCartridge.framework/PaxCartridge binary (debug or release).
#[cfg(not(target_os = "macos"))]
fn main() {
    panic!("This probe requires macOS");
}

#[cfg(target_os = "macos")]
fn main() {
    use std::{collections::HashMap, ffi::c_void};
    #[repr(C)]
    struct Buffer {
        data: *const u8,
        len: u64,
    }
    type Engine = *mut c_void;
    let path = std::env::args()
        .nth(1)
        .expect("pass Calculator cartridge path");
    let library = unsafe { libloading::Library::new(path).unwrap() };
    unsafe {
        let abi: libloading::Symbol<unsafe extern "C" fn() -> u32> =
            library.get(b"pax_cartridge_abi_version").unwrap();
        assert_eq!(abi(), 3);
        let init: libloading::Symbol<unsafe extern "C" fn(f32, f32) -> Engine> =
            library.get(b"pax_init").unwrap();
        let activate: libloading::Symbol<unsafe extern "C" fn(Engine)> =
            library.get(b"pax_activate_application").unwrap();
        let tick: libloading::Symbol<
            unsafe extern "C" fn(Engine, *mut c_void, f32, f32, f32) -> *mut Buffer,
        > = library.get(b"pax_tick").unwrap();
        let free: libloading::Symbol<unsafe extern "C" fn(*mut Buffer)> =
            library.get(b"pax_dealloc_message_queue").unwrap();
        let interrupt: libloading::Symbol<unsafe extern "C" fn(Engine, *const Buffer) -> bool> =
            library.get(b"pax_interrupt").unwrap();
        let dealloc: libloading::Symbol<unsafe extern "C" fn(Engine)> =
            library.get(b"pax_dealloc_engine").unwrap();
        let engine = init(1000.0, 900.0);
        assert!(!engine.is_null());
        activate(engine);
        let mut text = HashMap::<u64, String>::new();
        let frame = |text: &mut HashMap<u64, String>| {
            let queue = tick(engine, std::ptr::null_mut(), 1000.0, 900.0, 1.0);
            let value: serde_json::Value = flexbuffers::from_slice(std::slice::from_raw_parts(
                (*queue).data,
                (*queue).len as usize,
            ))
            .unwrap();
            free(queue);
            for message in value["messages"].as_array().unwrap() {
                if let Some(patch) = message.get("TextUpdate") {
                    if let Some(content) = patch["content"].as_str() {
                        text.insert(patch["id"].as_u64().unwrap(), content.to_owned());
                    }
                }
                if let Some(id) = message.get("TextDelete").and_then(|v| v.as_u64()) {
                    text.remove(&id);
                }
            }
        };
        let send = |bytes: &[u8]| {
            interrupt(
                engine,
                &Buffer {
                    data: bytes.as_ptr(),
                    len: bytes.len() as u64,
                },
            )
        };
        let key = |phase: &str, key: &str, modifiers: &[&str]| {
            send(&flexbuffers::to_vec(
            serde_json::json!({phase: {"key": key, "modifiers": modifiers, "is_repeat": false}}),
        ).unwrap())
        };
        frame(&mut text);
        // This exact payload is also checked against Swift's live encoder in PaxKeyboardTests.
        let hex = include_str!("../../../../../pax-host/native/common/pax-swift-common/Tests/PaxSwiftCommonTests/Fixtures/keyboard-interrupt.hex").trim();
        let bytes: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();
        assert!(
            !send(&bytes),
            "Calculator ignores Control/Alt/Command shortcuts"
        );
        assert!(
            !key("KeyDown", "F1", &[]),
            "Unhandled keys reach the native host"
        );
        // Calculator starts in Graph. Tab switches to its empty Calculate editor.
        for (name, modifiers) in [
            ("Tab", &[][..]),
            ("6", &[][..]),
            ("+", &["Shift"][..]),
            ("2", &[][..]),
            ("Enter", &[][..]),
        ] {
            assert!(
                key("KeyDown", name, modifiers),
                "{name} must prevent the native default"
            );
            assert!(!key("KeyUp", name, modifiers));
            frame(&mut text);
        }
        assert!(
            text.values().any(|v| v == "› 6+2"),
            "expression missing: {text:?}"
        );
        assert!(
            text.values().any(|v| v.starts_with(' ') && v.trim() == "8"),
            "history result missing: {text:?}"
        );
        assert!(!key("KeyPress", "x", &[]));
        dealloc(engine);
    }
    println!("PASS Swift modifiers, native cancellation, key down/up/press, Calculator 6+2=8");
}
