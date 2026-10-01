//! Headless macOS C-ABI proof against a built Async Workbench cartridge.
//! Calls the real generated entrypoints. No display link or periodic tick runs.
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("This cartridge probe requires macOS.");
}
#[cfg(target_os = "macos")]
fn main() {
    native::run();
}
#[cfg(target_os = "macos")]
mod native {
    use std::{
        collections::HashMap,
        ffi::c_void,
        sync::{Condvar, Mutex},
        time::{Duration, Instant},
    };
    static WAKE: (Mutex<u64>, Condvar) = (Mutex::new(0), Condvar::new());
    extern "C" fn wake(_: u64) {
        *WAKE.0.lock().unwrap() += 1;
        WAKE.1.notify_one();
    }
    #[repr(C)]
    struct Queue {
        data: *const u8,
        len: u64,
    }
    #[repr(C)]
    struct Interrupt {
        data: *const u8,
        len: u64,
    }
    type Engine = *mut c_void;
    struct Host {
        engine: Engine,
        tick: unsafe extern "C" fn(Engine, *mut c_void, f32, f32, f32) -> *mut Queue,
        free_queue: unsafe extern "C" fn(*mut Queue),
        interrupt: unsafe extern "C" fn(Engine, *const Interrupt),
        pending: unsafe extern "C" fn(Engine) -> bool,
        buttons: HashMap<String, u64>,
        text: HashMap<u64, String>,
    }
    impl Host {
        fn tick(&mut self) {
            unsafe {
                let queue = (self.tick)(self.engine, std::ptr::null_mut(), 1160.0, 1100.0, 1.0);
                assert!(!queue.is_null());
                let value: serde_json::Value = flexbuffers::from_slice(std::slice::from_raw_parts(
                    (*queue).data,
                    (*queue).len as usize,
                ))
                .unwrap();
                (self.free_queue)(queue);
                for message in value["messages"].as_array().unwrap() {
                    for (kind, patch) in message.as_object().unwrap() {
                        match kind.as_str() {
                            "ButtonUpdate" => {
                                if let Some(label) = patch["content"].as_str() {
                                    self.buttons
                                        .insert(label.to_owned(), patch["id"].as_u64().unwrap());
                                }
                            }
                            "TextUpdate" => {
                                if let Some(text) = patch["content"].as_str() {
                                    self.text
                                        .insert(patch["id"].as_u64().unwrap(), text.to_owned());
                                }
                            }
                            "TextDelete" => {
                                self.text.remove(&patch.as_u64().unwrap());
                            }
                            "ButtonDelete" => {
                                let id = patch.as_u64().unwrap();
                                self.buttons.retain(|_, value| *value != id);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        fn click(&mut self, label: &str) {
            let id = *self
                .buttons
                .get(label)
                .unwrap_or_else(|| panic!("button {label} missing: {:?}", self.buttons));
            let bytes =
                flexbuffers::to_vec(serde_json::json!({"FormButtonClick":{"id":id}})).unwrap();
            unsafe {
                (self.interrupt)(
                    self.engine,
                    &Interrupt {
                        data: bytes.as_ptr(),
                        len: bytes.len() as u64,
                    },
                );
            }
            self.tick();
        }
        fn contains(&self, text: &str) -> bool {
            self.text.values().any(|v| v.contains(text))
        }
        fn until(&mut self, text: &str) {
            let end = Instant::now() + Duration::from_secs(5);
            while !self.contains(text) {
                // Check pending under the host wake mutex, then atomically wait.
                // There is no repeating timer or speculative tick in this loop.
                let guard = WAKE.0.lock().unwrap();
                if unsafe { (self.pending)(self.engine) } {
                    drop(guard);
                    self.tick();
                    continue;
                }
                let remaining = end.saturating_duration_since(Instant::now());
                assert!(
                    !remaining.is_zero(),
                    "no wake/result for {text}: {:?}",
                    self.text
                );
                let (guard, timeout) = WAKE.1.wait_timeout(guard, remaining).unwrap();
                drop(guard);
                assert!(
                    !timeout.timed_out(),
                    "no host wake for {text}: {:?}",
                    self.text
                );
                self.tick();
            }
            println!("PASS {text}");
        }
    }
    pub fn run() {
        let path = std::env::args()
            .nth(1)
            .expect("pass built Async Workbench dylib path");
        // Keep code loaded for the process, including its one-shot deadline wake.
        let library = Box::leak(Box::new(unsafe { libloading::Library::new(path).unwrap() }));
        unsafe {
            let init: libloading::Symbol<unsafe extern "C" fn(f32, f32) -> Engine> =
                library.get(b"pax_init").unwrap();
            let set_shutdown: libloading::Symbol<unsafe extern "C" fn(u64, extern "C" fn(u64))> =
                library.get(b"pax_set_shutdown_waker").unwrap();
            let set_wake: libloading::Symbol<
                unsafe extern "C" fn(Engine, u64, extern "C" fn(u64)),
            > = library.get(b"pax_set_property_waker").unwrap();
            let activate: libloading::Symbol<unsafe extern "C" fn(Engine)> =
                library.get(b"pax_activate_application").unwrap();
            let dealloc: libloading::Symbol<unsafe extern "C" fn(Engine)> =
                library.get(b"pax_dealloc_engine").unwrap();
            let pump: libloading::Symbol<unsafe extern "C" fn() -> u64> =
                library.get(b"pax_pump_shutdowns").unwrap();
            set_shutdown(1, wake);
            let engine = init(1160.0, 1100.0);
            assert!(!engine.is_null());
            set_wake(engine, 2, wake);
            activate(engine);
            let mut host = Host {
                engine,
                tick: *library.get(b"pax_tick").unwrap(),
                free_queue: *library.get(b"pax_dealloc_message_queue").unwrap(),
                interrupt: *library.get(b"pax_interrupt").unwrap(),
                pending: *library.get(b"pax_has_pending_properties").unwrap(),
                buttons: HashMap::new(),
                text: HashMap::new(),
            };
            host.tick();
            host.until("Application starts: 1");
            host.click("Run · 600 ms");
            host.click("Interact · 0");
            assert!(host.buttons.contains_key("Interact · 1"));
            host.until("600 ms timer completed");
            host.click("Replace A with B");
            host.until("B wins");
            host.until("retained A publication rejected: true");
            host.click("Stream 100 events");
            host.until("all 100 events delivered in order");
            host.click("Run local I/O");
            host.until("Hello from Tokio loopback I/O");
            host.click("Local callback");
            host.until("local callback ran on the UI owner");
            host.click("Progress burst");
            host.until("1000 progress publications");
            host.click("Run · 600 ms");
            host.click("Cancel");
            host.until("Task: Cancelled");
            host.click("Burst +1000");
            host.until("Retained shared value: 1000");
            host.click("Run · 600 ms");
            host.click("Unmount panel");
            host.click("Remount panel");
            host.until("Ready to start a request");
            assert!(host.contains("Application starts: 1"));
            host.click("Panic probe (native)");
            host.until("expected workbench task panic");
            host.click("Blocking probe (native)");
            host.until("bounded blocking work finished");
            // A prepared, discarded candidate must not close the active app.
            host.click("Run · 600 ms");
            let discarded = init(1160.0, 1100.0);
            assert!(!discarded.is_null());
            dealloc(discarded);
            host.click("Interact · 1");
            assert!(host.buttons.contains_key("Interact · 2"));
            // Commit a new instance only after revoking the old instance.
            let next = init(1160.0, 1100.0);
            assert!(!next.is_null());
            let replace: libloading::Symbol<unsafe extern "C" fn(Engine)> =
                library.get(b"pax_shutdown_for_replacement").unwrap();
            replace(host.engine);
            dealloc(host.engine);
            set_wake(next, 3, wake);
            activate(next);
            host.engine = next;
            host.buttons.clear();
            host.text.clear();
            host.tick();
            host.until("Application starts: 1");
            host.until("Retained shared value: 0");
            host.click("Run · 600 ms");
            host.until("600 ms timer completed");
            println!("PASS prepared discard + replacement isolates application state");
            host.click("Blocking probe (native)");
            dealloc(next);
            let end = Instant::now() + Duration::from_secs(5);
            loop {
                let observed = *WAKE.0.lock().unwrap();
                // pump may invoke the host callback; never hold the wake mutex.
                if pump() == 0 {
                    break;
                }
                let guard = WAKE.0.lock().unwrap();
                if *guard != observed {
                    drop(guard);
                    continue;
                }
                let remaining = end.saturating_duration_since(Instant::now());
                assert!(!remaining.is_zero(), "shutdown did not quiesce");
                let (guard, timeout) = WAKE.1.wait_timeout(guard, remaining).unwrap();
                drop(guard);
                assert!(!timeout.timed_out(), "shutdown host wake missing");
            }
            println!("PASS engine disposal + managed shutdown without periodic ticks");
        }
    }
}
