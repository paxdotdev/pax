use pax_runtime::engine::layer_tiling::ScrollerTilingPolicy;
use std::{cell::Cell, rc::Rc};
use wasm_bindgen::JsValue;
use web_sys::Window;

// Shared by the plan exporter and renderer. Discovering a device limit changes
// the plan contract once, rather than making JS repeatedly repair valid sizes.
#[derive(Clone)]
pub(crate) struct WebSurfacePolicy {
    pub(crate) browser: BrowserSurfacePolicy,
    backend_limit: Rc<Cell<u32>>,
    revision: Rc<Cell<u32>>,
}

impl WebSurfacePolicy {
    pub(crate) fn new(browser: BrowserSurfacePolicy) -> Self {
        Self {
            browser,
            backend_limit: Rc::new(Cell::new(u32::MAX)),
            revision: Rc::new(Cell::new(0)),
        }
    }

    pub(crate) fn observe_backend_limit(&self, limit: u32) {
        let limit = limit.max(1).min(self.backend_limit.get());
        if limit != self.backend_limit.replace(limit) {
            self.revision.set(self.revision.get().wrapping_add(1));
        }
    }

    pub(crate) fn revision(&self) -> u32 {
        self.revision.get()
    }

    pub(crate) fn max_surface_dimension(&self, layer: usize) -> u32 {
        self.browser
            .effective_max_surface_dimension(layer, self.backend_limit.get())
    }
}

const NESTED_SURFACE_DIMENSION_CAP: u32 = 2048;
// A scene can own several color, stencil, multisample, and effect textures
// per visible layer. A per-texture dimension limit alone does not bound that
// footprint on iPhone Safari; apply the base density before projecting layers.
const IPHONE_WEBGPU_BASE_DENSITY_CAP: f64 = 1.0;
const CPU_TARGET_TILE_BACKING_DIMENSION: f64 = 2048.0;
const CPU_TOTAL_CANVAS_BUDGET: usize = 56;
const CPU_PREWARM_VIEWPORT_PAD_X_MULTIPLIER: f64 = 2.0;
const CPU_PREWARM_VIEWPORT_PAD_Y_MULTIPLIER: f64 = 6.0;
const CPU_PREWARM_VIEWPORT_PAD_MIN_X: f64 = 1024.0;
const CPU_PREWARM_VIEWPORT_PAD_MIN_Y: f64 = 4096.0;
const WEB_TILE_OVERSCAN_COLUMNS: i32 = 0;
const WEB_TILE_OVERSCAN_ROWS: i32 = 0;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct BrowserSurfacePolicy {
    use_cpu_fallback: bool,
    surface_dimension_cap: Option<u32>,
    ios_webkit: bool,
    phone_webkit: bool,
}

impl BrowserSurfacePolicy {
    pub(crate) fn detect(window: &Window) -> Self {
        let user_agent = window.navigator().user_agent().unwrap_or_default();
        let webgpu_available =
            js_sys::Reflect::has(window.navigator().as_ref(), &JsValue::from_str("gpu"))
                .unwrap_or(false);
        Self::for_navigator(
            &user_agent,
            window.navigator().max_touch_points(),
            webgpu_available,
        )
    }

    fn for_navigator(user_agent: &str, touch_points: i32, webgpu_available: bool) -> Self {
        let ipad = user_agent.contains("iPad") || desktop_mode_ipad(user_agent, touch_points);
        let isi_os = user_agent.contains("iPhone") || ipad || user_agent.contains("iPod");
        let ios_webkit = isi_os
            && user_agent.contains("AppleWebKit")
            && !user_agent.contains("CriOS")
            && !user_agent.contains("FxiOS")
            && !user_agent.contains("EdgiOS")
            && !user_agent.contains("Chrome/")
            && !user_agent.contains("Chromium/");
        // Keep the tested nested backing limit independent of the renderer;
        // Safari can advertise a larger device limit than its surfaces sustain.
        let surface_dimension_cap = ios_webkit.then_some(NESTED_SURFACE_DIMENSION_CAP);
        Self {
            use_cpu_fallback: !webgpu_available,
            surface_dimension_cap,
            ios_webkit,
            phone_webkit: ios_webkit && !ipad,
        }
    }

    pub(crate) fn select_cpu(&mut self, cpu: bool) {
        self.use_cpu_fallback = cpu;
        self.surface_dimension_cap =
            (cpu || self.ios_webkit).then_some(NESTED_SURFACE_DIMENSION_CAP);
    }

    pub(crate) fn use_cpu_fallback(self) -> bool {
        self.use_cpu_fallback
    }

    pub(crate) fn base_density_limit(self) -> Option<f64> {
        (self.phone_webkit && !self.use_cpu_fallback).then_some(IPHONE_WEBGPU_BASE_DENSITY_CAP)
    }

    pub(crate) fn base_density(self, display_ratio: f64) -> f64 {
        display_ratio.min(self.base_density_limit().unwrap_or(f64::INFINITY))
    }

    pub(crate) fn allow_multisampling(self) -> bool {
        !self.phone_webkit
    }

    pub(crate) fn effective_max_surface_dimension(self, layer: usize, backend_limit: u32) -> u32 {
        if layer == 0 {
            return backend_limit;
        }
        self.surface_dimension_cap
            .map(|cap| cap.min(backend_limit))
            .unwrap_or(backend_limit)
    }

    pub(crate) fn defer_transient_root_host_surfaces(self, layer: usize) -> bool {
        // A GPU dimension limit isn't a reason to skip an otherwise drawable host.
        self.use_cpu_fallback && self.surface_dimension_cap.is_some() && layer > 0
    }

    pub(crate) fn scroller_tiling_policy(self) -> ScrollerTilingPolicy {
        let mut policy = ScrollerTilingPolicy::default();
        policy.tile_overscan_columns = WEB_TILE_OVERSCAN_COLUMNS;
        policy.tile_overscan_rows = WEB_TILE_OVERSCAN_ROWS;
        // Bound each preparation unit to roughly one viewport on either backend.
        // The shared planner still enforces backing-dimension and canvas limits.
        policy.target_tile_viewports = Some(1.0);
        if self.use_cpu_fallback {
            // Vello CPU replays dirty draws into browser canvases. Spend more canvas memory on
            // warm runway so fast scrolls are less likely to expose tiles before replay catches up.
            policy.target_tile_backing_dimension = CPU_TARGET_TILE_BACKING_DIMENSION;
            policy.prewarm_viewport_pad_x_multiplier = CPU_PREWARM_VIEWPORT_PAD_X_MULTIPLIER;
            policy.prewarm_viewport_pad_y_multiplier = CPU_PREWARM_VIEWPORT_PAD_Y_MULTIPLIER;
            policy.prewarm_viewport_pad_min_x = CPU_PREWARM_VIEWPORT_PAD_MIN_X;
            policy.prewarm_viewport_pad_min_y = CPU_PREWARM_VIEWPORT_PAD_MIN_Y;
            policy.max_surfaces_per_layer = Some(CPU_TOTAL_CANVAS_BUDGET);
        }
        if self.ios_webkit {
            // Keep Safari's fixed browser-surface budget explicit at the planner boundary. The JS
            // canvas pool still enforces the page-wide cap, but the engine should not request a
            // per-layer surface set that the chassis can never materialize.
            let surface_dimension_cap = self
                .surface_dimension_cap
                .unwrap_or(NESTED_SURFACE_DIMENSION_CAP);
            policy.max_tile_backing_width = Some(surface_dimension_cap as f64);
            policy.max_tile_backing_height = Some(surface_dimension_cap as f64);
            policy.max_tile_backing_area = Some((surface_dimension_cap as f64).powi(2));
        }
        policy
    }
}

pub(crate) fn desktop_mode_ipad(user_agent: &str, touch_points: i32) -> bool {
    // iPadOS desktop browsing identifies as Macintosh, but unlike a Mac
    // it exposes multiple direct touch contacts (not trackpad gestures).
    user_agent.contains("Macintosh") && touch_points > 1
}

#[cfg(test)]
mod tests {
    use super::*;

    const IOS_SAFARI_USER_AGENT: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 Version/18.0 Mobile/15E148 Safari/604.1";
    const DESKTOP_USER_AGENT: &str =
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 Chrome/140.0 Safari/537.36";

    #[test]
    fn discovered_limits_invalidate_shared_plan_policy_once() {
        let policy = WebSurfacePolicy::new(BrowserSurfacePolicy::default());
        let renderer = policy.clone();
        assert_eq!(policy.revision(), 0);
        renderer.observe_backend_limit(8192);
        assert_eq!(policy.max_surface_dimension(1), 8192);
        assert_eq!(policy.revision(), 1);
        renderer.observe_backend_limit(8192);
        renderer.observe_backend_limit(16384);
        assert_eq!(policy.revision(), 1);
        renderer.observe_backend_limit(2048);
        assert_eq!(policy.max_surface_dimension(1), 2048);
        assert_eq!(policy.revision(), 2);
    }

    #[test]
    fn desktop_webgpu_uses_gpu_policy() {
        let policy = BrowserSurfacePolicy::for_navigator(DESKTOP_USER_AGENT, 0, true);

        assert!(!policy.use_cpu_fallback());
        assert_eq!(policy.surface_dimension_cap, None);
        assert_eq!(policy.scroller_tiling_policy().max_surfaces_per_layer, None);
        assert_eq!(
            policy.scroller_tiling_policy().target_tile_viewports,
            Some(1.0)
        );
    }

    #[test]
    fn failed_or_forced_gpu_uses_cpu_surface_budget() {
        let mut browser = BrowserSurfacePolicy::for_navigator(DESKTOP_USER_AGENT, 0, true);
        browser.select_cpu(true);
        assert!(browser.use_cpu_fallback());
        assert_eq!(
            browser.scroller_tiling_policy().max_surfaces_per_layer,
            Some(CPU_TOTAL_CANVAS_BUDGET)
        );
        assert_eq!(browser.effective_max_surface_dimension(1, 8192), 2048);
    }

    #[test]
    fn no_webgpu_uses_cpu_policy() {
        let policy = BrowserSurfacePolicy::for_navigator(DESKTOP_USER_AGENT, 0, false);

        assert!(policy.use_cpu_fallback());
        assert_eq!(
            policy.scroller_tiling_policy().target_tile_viewports,
            Some(1.0)
        );
        assert_eq!(policy.surface_dimension_cap, None);
        assert_eq!(
            policy.scroller_tiling_policy().max_surfaces_per_layer,
            Some(CPU_TOTAL_CANVAS_BUDGET)
        );
    }

    #[test]
    fn iphone_webgpu_uses_bounded_density_but_cpu_fallback_keeps_display_density() {
        let policy = BrowserSurfacePolicy::for_navigator(IOS_SAFARI_USER_AGENT, 5, true);
        let tiling = policy.scroller_tiling_policy();

        assert!(!policy.use_cpu_fallback());
        assert!(!policy.defer_transient_root_host_surfaces(1));
        assert!(!policy.allow_multisampling());
        assert_eq!(policy.base_density(3.0), IPHONE_WEBGPU_BASE_DENSITY_CAP);
        assert_eq!(policy.base_density(1.0), 1.0);
        assert_eq!(
            policy.effective_max_surface_dimension(1, u32::MAX),
            NESTED_SURFACE_DIMENSION_CAP
        );
        assert_eq!(
            tiling.max_tile_backing_area,
            Some((NESTED_SURFACE_DIMENSION_CAP as f64).powi(2))
        );
        assert_eq!(tiling.max_surfaces_per_layer, None);
        for mut cpu in [
            policy,
            BrowserSurfacePolicy::for_navigator(IOS_SAFARI_USER_AGENT, 5, false),
        ] {
            cpu.select_cpu(true);
            assert!(cpu.defer_transient_root_host_surfaces(1));
            assert_eq!(cpu.base_density(3.0), 3.0);
            assert_eq!(
                cpu.scroller_tiling_policy().max_surfaces_per_layer,
                Some(CPU_TOTAL_CANVAS_BUDGET)
            );
        }
    }

    #[test]
    fn requiring_gpu_keeps_ios_nested_backings_bounded_without_cpu_prewarming() {
        let mut browser = BrowserSurfacePolicy::for_navigator(IOS_SAFARI_USER_AGENT, 5, true);
        browser.select_cpu(false);
        assert!(!browser.use_cpu_fallback());
        assert!(!browser.defer_transient_root_host_surfaces(1));
        assert_eq!(browser.effective_max_surface_dimension(0, 8192), 8192);
        assert_eq!(browser.effective_max_surface_dimension(1, 8192), 2048);
        assert_eq!(browser.effective_max_surface_dimension(1, 1024), 1024);
        assert_eq!(
            browser.scroller_tiling_policy().max_tile_backing_width,
            Some(2048.0)
        );
        assert_eq!(
            browser.scroller_tiling_policy().max_surfaces_per_layer,
            None
        );
    }

    #[test]
    fn ipad_safari_keeps_its_nested_budget_in_either_browsing_mode() {
        const DESKTOP: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 Version/26.6.1 Safari/605.1.15";
        const MOBILE: &str = "Mozilla/5.0 (iPad; CPU OS 18_0 like Mac OS X) AppleWebKit/605.1.15 Mobile/15E148 Safari/604.1";
        for ua in [DESKTOP, MOBILE] {
            let gpu = BrowserSurfacePolicy::for_navigator(ua, 5, true);
            assert!(!gpu.use_cpu_fallback());
            assert_eq!(gpu.base_density(3.0), 3.0);
            assert!(gpu.allow_multisampling());
            assert_eq!(gpu.effective_max_surface_dimension(1, 8192), 2048);
            assert!(BrowserSurfacePolicy::for_navigator(ua, 5, false).use_cpu_fallback());
        }
        assert_eq!(
            BrowserSurfacePolicy::for_navigator(DESKTOP, 0, true).surface_dimension_cap,
            None
        );
    }
}
