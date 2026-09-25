use pax_runtime::engine::layer_tiling::ScrollerTilingPolicy;
use wasm_bindgen::JsValue;
use web_sys::Window;

const PIET_BROWSER_SURFACE_DIMENSION_CAP: u32 = 2048;
const PIET_TARGET_TILE_BACKING_DIMENSION: f64 = 2048.0;
const IOS_NESTED_LAYER_MIN_DPR: f64 = 0.25;
const PIET_TOTAL_CANVAS_BUDGET: usize = 56;
const PIET_PREWARM_VIEWPORT_PAD_X_MULTIPLIER: f64 = 2.0;
const PIET_PREWARM_VIEWPORT_PAD_Y_MULTIPLIER: f64 = 6.0;
const PIET_PREWARM_VIEWPORT_PAD_MIN_X: f64 = 1024.0;
const PIET_PREWARM_VIEWPORT_PAD_MIN_Y: f64 = 4096.0;
const WEB_TILE_OVERSCAN_COLUMNS: i32 = 0;
const WEB_TILE_OVERSCAN_ROWS: i32 = 0;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct BrowserSurfacePolicy {
    use_piet_fallback: bool,
    surface_dimension_cap: Option<u32>,
    ios_webkit: bool,
}

impl BrowserSurfacePolicy {
    pub(crate) fn detect(window: &Window) -> Self {
        let user_agent = window.navigator().user_agent().unwrap_or_default();
        let webgpu_available =
            js_sys::Reflect::has(window.navigator().as_ref(), &JsValue::from_str("gpu"))
                .unwrap_or(false);
        Self::from_browser_capabilities(&user_agent, webgpu_available)
    }

    fn from_browser_capabilities(user_agent: &str, webgpu_available: bool) -> Self {
        let isi_os = user_agent.contains("iPhone")
            || user_agent.contains("iPad")
            || user_agent.contains("iPod");
        let ios_webkit = isi_os
            && user_agent.contains("AppleWebKit")
            && !user_agent.contains("CriOS")
            && !user_agent.contains("FxiOS")
            && !user_agent.contains("EdgiOS");
        let use_piet_fallback = ios_webkit || !webgpu_available;

        // Nested browser-owned scroller surfaces on iOS WebKit hit two limits that the generic
        // web path does not:
        // 1. WebGPU browser surfaces in this compositor path do not reliably preserve alpha.
        // 2. Surface setup can fail once a nested layer's backing dimension grows beyond the
        //    conservative 2048 px limit, even when a graphics API reports a higher maximum.
        //
        // We keep that policy isolated here so the rest of the web chassis stays platform-agnostic:
        // choose the CPU/Piet renderer on iOS, and allow descendant layers to downsample below 1x
        // DPR instead of failing surface initialization outright. Root-layer tiling is chosen one
        // level up in the web surface host policy, so the root viewport no longer needs the same
        // backing-dimension cap here.
        let surface_dimension_cap = ios_webkit.then_some(PIET_BROWSER_SURFACE_DIMENSION_CAP);
        Self {
            use_piet_fallback,
            surface_dimension_cap,
            ios_webkit,
        }
    }

    pub(crate) fn use_piet_fallback(self) -> bool {
        self.use_piet_fallback
    }

    pub(crate) fn effective_max_surface_dimension(self, layer: usize, backend_limit: u32) -> u32 {
        if layer == 0 {
            return backend_limit;
        }
        self.surface_dimension_cap
            .map(|cap| cap.min(backend_limit))
            .unwrap_or(backend_limit)
    }

    pub(crate) fn minimum_dpr(self, layer: usize) -> f64 {
        if self.surface_dimension_cap.is_some() && layer > 0 {
            IOS_NESTED_LAYER_MIN_DPR
        } else {
            1.0
        }
    }

    pub(crate) fn defer_transient_root_host_surfaces(self, layer: usize) -> bool {
        self.surface_dimension_cap.is_some() && layer > 0
    }

    pub(crate) fn scroller_tiling_policy(self) -> ScrollerTilingPolicy {
        let mut policy = ScrollerTilingPolicy::default();
        policy.tile_overscan_columns = WEB_TILE_OVERSCAN_COLUMNS;
        policy.tile_overscan_rows = WEB_TILE_OVERSCAN_ROWS;
        if self.use_piet_fallback {
            // Piet replays dirty draws into browser canvases. Spend more canvas memory on
            // warm runway so fast scrolls are less likely to expose tiles before replay catches up.
            policy.target_tile_backing_dimension = PIET_TARGET_TILE_BACKING_DIMENSION;
            policy.prewarm_viewport_pad_x_multiplier = PIET_PREWARM_VIEWPORT_PAD_X_MULTIPLIER;
            policy.prewarm_viewport_pad_y_multiplier = PIET_PREWARM_VIEWPORT_PAD_Y_MULTIPLIER;
            policy.prewarm_viewport_pad_min_x = PIET_PREWARM_VIEWPORT_PAD_MIN_X;
            policy.prewarm_viewport_pad_min_y = PIET_PREWARM_VIEWPORT_PAD_MIN_Y;
            policy.max_surfaces_per_layer = Some(PIET_TOTAL_CANVAS_BUDGET);
        }
        if self.ios_webkit {
            // Keep Safari's fixed browser-surface budget explicit at the planner boundary. The JS
            // canvas pool still enforces the page-wide cap, but the engine should not request a
            // per-layer surface set that the chassis can never materialize.
            let surface_dimension_cap = self
                .surface_dimension_cap
                .unwrap_or(PIET_BROWSER_SURFACE_DIMENSION_CAP);
            policy.max_tile_backing_width = Some(surface_dimension_cap as f64);
            policy.max_tile_backing_height = Some(surface_dimension_cap as f64);
            policy.max_tile_backing_area = Some((surface_dimension_cap as f64).powi(2));
        }
        policy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IOS_SAFARI_USER_AGENT: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 Version/18.0 Mobile/15E148 Safari/604.1";
    const DESKTOP_USER_AGENT: &str =
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 Chrome/140.0 Safari/537.36";

    #[test]
    fn desktop_webgpu_uses_gpu_policy() {
        let policy = BrowserSurfacePolicy::from_browser_capabilities(DESKTOP_USER_AGENT, true);

        assert!(!policy.use_piet_fallback());
        assert_eq!(policy.surface_dimension_cap, None);
        assert_eq!(policy.scroller_tiling_policy().max_surfaces_per_layer, None);
    }

    #[test]
    fn no_webgpu_uses_piet_policy() {
        let policy = BrowserSurfacePolicy::from_browser_capabilities(DESKTOP_USER_AGENT, false);

        assert!(policy.use_piet_fallback());
        assert_eq!(policy.surface_dimension_cap, None);
        assert_eq!(
            policy.scroller_tiling_policy().max_surfaces_per_layer,
            Some(PIET_TOTAL_CANVAS_BUDGET)
        );
    }

    #[test]
    fn ios_webkit_uses_conservative_piet_policy_even_with_webgpu() {
        let policy = BrowserSurfacePolicy::from_browser_capabilities(IOS_SAFARI_USER_AGENT, true);
        let tiling = policy.scroller_tiling_policy();

        assert!(policy.use_piet_fallback());
        assert_eq!(
            policy.effective_max_surface_dimension(1, u32::MAX),
            PIET_BROWSER_SURFACE_DIMENSION_CAP
        );
        assert_eq!(policy.minimum_dpr(1), IOS_NESTED_LAYER_MIN_DPR);
        assert_eq!(
            tiling.max_tile_backing_area,
            Some((PIET_BROWSER_SURFACE_DIMENSION_CAP as f64).powi(2))
        );
        assert_eq!(
            tiling.max_surfaces_per_layer,
            Some(PIET_TOTAL_CANVAS_BUDGET)
        );
    }
}
