use crate::{NavLogo, SiteTheme};
use pax_kit::*;

/// Shared navigation for the full website and the static blog's embedded header.
/// The owning shell supplies positioning and the homepage's portal-logo twin.
#[pax]
#[has_helpers]
#[file("site_header.pax")]
pub struct SiteHeader {
    pub light_mode: Property<bool>,
    /// Standalone blog headers own the preference; routed site headers inherit it.
    pub owns_theme: Property<bool>,
    pub compact: Property<bool>,
    pub show_theme_toggle: Property<bool>,
    pub show_logo: Property<bool>,
    pub controls_visible: Property<bool>,
    pub font_url: Property<String>,
}

#[helpers]
impl SiteHeader {
    pub fn logo_height(compact: bool) -> f64 {
        if compact { 26.0 } else { 32.0 }
    }

    pub fn logo_width(compact: bool) -> f64 {
        Self::logo_height(compact) * 920.72 / 463.95
    }
}

impl SiteHeader {
    pub fn toggle_theme(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.light_mode.set(!self.light_mode.get());
    }

    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        if self.owns_theme.get() {
            crate::site_theme::connect_preference(ctx, &self.light_mode);
            #[cfg(target_arch = "wasm32")]
            blog_input::connect(ctx, &self.light_mode);
        }
        let bounds = ctx.bounds_self.clone();
        let compact = self.compact.local();
        let deps = [bounds.untyped()];
        let update = move || {
            // The host chooses a 552px or at-most-312px island for its viewport.
            compact.set_if_neq(bounds.get().0 <= 312.0);
        };
        update();
        ctx.subscribe(&deps, update);
    }
}

#[cfg(target_arch = "wasm32")]
mod blog_input {
    use super::*;
    use wasm_bindgen::{closure::Closure, JsCast};

    // The blog also has an HTML header for keyboard users and startup fallback.
    struct ToggleListener(web_sys::Window, Closure<dyn Fn(web_sys::Event)>);
    impl Store for ToggleListener {}
    impl std::ops::Drop for ToggleListener {
        fn drop(&mut self) {
            let _ = self.0.remove_event_listener_with_callback(
                "pax-toggle-theme",
                self.1.as_ref().unchecked_ref(),
            );
        }
    }
    pub(super) fn connect(ctx: &NodeContext, mode: &Property<bool>) {
        let Some(window) = web_sys::window() else {
            return;
        };
        let mode = mode.local();
        let toggle = Closure::wrap(Box::new(move |event: web_sys::Event| {
            event.prevent_default();
            mode.set(!mode.get());
        }) as Box<dyn Fn(web_sys::Event)>);
        if window
            .add_event_listener_with_callback("pax-toggle-theme", toggle.as_ref().unchecked_ref())
            .is_ok()
        {
            let _ = ctx.provide_store(ToggleListener(window, toggle));
        }
    }
}
