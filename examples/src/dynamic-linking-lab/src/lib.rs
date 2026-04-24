use std::sync::{LazyLock, Mutex};

use pax_kit::*;

// Edit these constants in the Rust pane, then press "Recompile Rust".
const LOGIC_BUTTON_LABEL: &str = "Run Hot Logic";
const LOGIC_MESSAGE: &str = "Reloaded Rust logic is hot-swapped";
const LOGIC_ACCENT_RGBA: (u8, u8, u8, u8) = (71, 133, 255, 255);

const DEFAULT_RUST_STATUS: &str =
    "Edit the Rust constants above, then press Recompile Rust.";
const DEFAULT_PAX_STATUS: &str =
    "Edit the Pax pane below. Valid template updates auto-apply after a short debounce.";
const PAX_DEBOUNCE_MS: u64 = 450;

#[derive(Clone)]
struct EditorCache {
    rust_source: String,
    pax_source: String,
    rust_status: String,
    pax_status: String,
}

impl Default for EditorCache {
    fn default() -> Self {
        Self {
            rust_source: include_str!("lib.rs").to_string(),
            pax_source: include_str!("lib.pax").to_string(),
            rust_status: DEFAULT_RUST_STATUS.to_string(),
            pax_status: DEFAULT_PAX_STATUS.to_string(),
        }
    }
}

static EDITOR_CACHE: LazyLock<Mutex<EditorCache>> =
    LazyLock::new(|| Mutex::new(EditorCache::default()));

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub rust_source: Property<String>,
    pub pax_source: Property<String>,
    pub rust_status: Property<String>,
    pub pax_status: Property<String>,
    pub logic_message: Property<String>,
    pub logic_button_label: Property<String>,
    pub accent_color: Property<Color>,
    pub logic_clicks: Property<usize>,
    pub rust_compile_in_flight: Property<bool>,
    pub pending_pax_apply_at_ms: Property<Option<u64>>,
    pub request_counter: Property<u64>,
}

impl Example {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        self.seed_demo_state(true);
        self.rust_compile_in_flight.set(false);
        self.pending_pax_apply_at_ms.set(None);
        self.request_counter.set(0);
    }

    pub fn on_pre_render(&mut self, ctx: &NodeContext) {
        self.seed_demo_state(false);

        #[cfg(feature = "designtime")]
        {
            for response in ctx.take_userland_source_update_responses() {
                self.handle_source_update_response(response);
            }

            if let Some(deadline_ms) = self.pending_pax_apply_at_ms.get() {
                let now_ms = ctx.elapsed_time_millis() as u64;
                if now_ms >= deadline_ms {
                    self.pending_pax_apply_at_ms.set(None);
                    self.set_pax_status("Applying valid Pax update...".to_string());
                    if let Err(err) = ctx.submit_userland_source_update(
                        self.next_request_id("pax"),
                        "src/lib.pax".to_string(),
                        self.pax_source.get(),
                    ) {
                        self.set_pax_status(format!("Pax apply failed to send: {err}"));
                    }
                }
            }
        }
    }

    pub fn run_logic(&mut self, _ctx: &NodeContext, _args: Event<ButtonClick>) {
        let next_click = self.logic_clicks.get() + 1;
        self.logic_clicks.set(next_click);
        self.logic_message
            .set(format!("{LOGIC_MESSAGE} ({next_click})"));
    }

    pub fn recompile_rust(&mut self, ctx: &NodeContext, _args: Event<ButtonClick>) {
        if self.rust_compile_in_flight.get() {
            self.set_rust_status("A Rust rebuild is already running.".to_string());
            return;
        }

        self.rust_compile_in_flight.set(true);
        self.set_rust_status("Compiling and relinking logic module...".to_string());

        #[cfg(feature = "designtime")]
        {
            if let Err(err) = ctx.submit_userland_source_update(
                self.next_request_id("rust"),
                "src/lib.rs".to_string(),
                self.rust_source.get(),
            ) {
                self.rust_compile_in_flight.set(false);
                self.set_rust_status(format!("Rust rebuild failed to send: {err}"));
            }
        }

        #[cfg(not(feature = "designtime"))]
        {
            self.rust_compile_in_flight.set(false);
            self.set_rust_status("Rust recompilation is only available in designtime.".to_string());
        }
    }

    pub fn update_rust_source(&mut self, _ctx: &NodeContext, args: Event<TextboxInput>) {
        self.rust_source.set(args.text.clone());
        with_editor_cache(|cache| {
            cache.rust_source = args.text.clone();
        });
        if !self.rust_compile_in_flight.get() {
            self.set_rust_status("Rust source modified. Press Recompile Rust.".to_string());
        }
    }

    pub fn update_pax_source(&mut self, ctx: &NodeContext, args: Event<TextboxInput>) {
        self.pax_source.set(args.text.clone());
        with_editor_cache(|cache| {
            cache.pax_source = args.text.clone();
        });
        self.pending_pax_apply_at_ms
            .set(Some(ctx.elapsed_time_millis() as u64 + PAX_DEBOUNCE_MS));
        self.set_pax_status("Waiting for valid Pax input...".to_string());
    }

    #[cfg(feature = "designtime")]
    fn handle_source_update_response(
        &mut self,
        response: pax_kit::pax_designtime::messages::UserlandSourceUpdateResponse,
    ) {
        if response.path.ends_with(".rs") {
            if response.status == "ok" {
                self.set_rust_status("Reloading the Rust dylib...".to_string());
            } else {
                self.rust_compile_in_flight.set(false);
                self.set_rust_status(response.error.unwrap_or_else(|| {
                    "Rust rebuild failed for an unknown reason.".to_string()
                }));
            }
        } else if response.path.ends_with(".pax") {
            if response.status == "ok" {
                self.set_pax_status("Applied Pax update.".to_string());
            } else {
                self.set_pax_status(response.error.unwrap_or_else(|| {
                    "Pax apply failed for an unknown reason.".to_string()
                }));
            }
        }
    }

    fn set_rust_status(&mut self, status: String) {
        self.rust_status.set(status.clone());
        with_editor_cache(|cache| {
            cache.rust_status = status;
        });
    }

    fn set_pax_status(&mut self, status: String) {
        self.pax_status.set(status.clone());
        with_editor_cache(|cache| {
            cache.pax_status = status;
        });
    }

    fn next_request_id(&mut self, prefix: &str) -> String {
        let next_counter = self.request_counter.get() + 1;
        self.request_counter.set(next_counter);
        format!("{prefix}-{next_counter}")
    }

    fn seed_demo_state(&mut self, force_reset_preview: bool) {
        let snapshot = with_editor_cache(|cache| cache.clone());
        let fallback_rust_source = include_str!("lib.rs").to_string();
        let fallback_pax_source = include_str!("lib.pax").to_string();
        let rust_source = if snapshot.rust_source.is_empty() {
            fallback_rust_source
        } else {
            snapshot.rust_source
        };
        let pax_source = if snapshot.pax_source.is_empty() {
            fallback_pax_source
        } else {
            snapshot.pax_source
        };
        let rust_status = if snapshot.rust_status.is_empty() {
            DEFAULT_RUST_STATUS.to_string()
        } else {
            snapshot.rust_status
        };
        let pax_status = if snapshot.pax_status.is_empty() {
            DEFAULT_PAX_STATUS.to_string()
        } else {
            snapshot.pax_status
        };

        if force_reset_preview || self.rust_source.get().is_empty() {
            self.rust_source.set(rust_source.clone());
        }
        if force_reset_preview || self.pax_source.get().is_empty() {
            self.pax_source.set(pax_source.clone());
        }
        if force_reset_preview || self.rust_status.get().is_empty() {
            self.rust_status.set(rust_status.clone());
        }
        if force_reset_preview || self.pax_status.get().is_empty() {
            self.pax_status.set(pax_status.clone());
        }
        if force_reset_preview || self.logic_message.get().is_empty() {
            self.logic_message.set(
                "Edit either pane to validate Phase 1 hot logic reloading end to end."
                    .to_string(),
            );
        }
        if force_reset_preview || self.logic_button_label.get().is_empty() {
            self.logic_button_label.set(LOGIC_BUTTON_LABEL.to_string());
        }
        if force_reset_preview || self.logic_clicks.get() == 0 {
            self.logic_clicks.set(0);
        }
        if force_reset_preview {
            self.accent_color.set(logic_accent_color());
        } else {
            let accent = self.accent_color.get();
            let alpha = accent.to_rgba_0_1()[3];
            if alpha == 0.0 {
                self.accent_color.set(logic_accent_color());
            }
        }

        with_editor_cache(|cache| {
            cache.rust_source = rust_source;
            cache.pax_source = pax_source;
            cache.rust_status = rust_status;
            cache.pax_status = pax_status;
        });
    }
}

fn logic_accent_color() -> Color {
    Color::rgba(
        i32::from(LOGIC_ACCENT_RGBA.0).into(),
        i32::from(LOGIC_ACCENT_RGBA.1).into(),
        i32::from(LOGIC_ACCENT_RGBA.2).into(),
        i32::from(LOGIC_ACCENT_RGBA.3).into(),
    )
}

fn with_editor_cache<R>(f: impl FnOnce(&mut EditorCache) -> R) -> R {
    let mut cache = EDITOR_CACHE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    f(&mut cache)
}
																							