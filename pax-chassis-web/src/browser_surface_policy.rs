use pax_runtime::engine::layer_tiling::ScrollerTilingPolicy;
use web_sys::Window;

const IOS_BROWSER_SURFACE_DIMENSION_CAP: u32 = 1800;
const IOS_NESTED_LAYER_MIN_DPR: f64 = 0.25;
const IOS_TOTAL_CANVAS_BUDGET: usize = 12;
const WEB_TILE_OVERSCAN_COLUMNS: i32 = 0;
const WEB_TILE_OVERSCAN_ROWS: i32 = 0;
const FIREFOX_PREWARM_VIEWPORT_PAD_Y_MULTIPLIER: f64 = 3.0;
const FIREFOX_PREWARM_VIEWPORT_PAD_MIN_Y: f64 = 1536.0;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct BrowserSurfacePolicy {
    force_gl: bool,
    surface_dimension_cap: Option<u32>,
    ios_webkit: bool,
    firefox: bool,
}

impl BrowserSurfacePolicy {
    pub(crate) fn detect(window: &Window) -> Self {
        let Ok(user_agent) = window.navigator().user_agent() else {
            return Self::default();
        };

        let isi_os = user_agent.contains("iPhone")
            || user_agent.contains("iPad")
            || user_agent.contains("iPod");
        let ios_webkit = isi_os
            && user_agent.contains("AppleWebKit")
            && !user_agent.contains("CriOS")
            && !user_agent.contains("FxiOS")
            && !user_agent.contains("EdgiOS");
        let firefox = user_agent.contains("Firefox/") && !user_agent.contains("FxiOS");

        // Nested browser-owned scroller surfaces on iOS WebKit hit two limits that the generic
        // web path does not:
        // 1. WebGPU browser surfaces in this compositor path do not reliably preserve alpha.
        // 2. Surface setup starts failing once a nested layer's backing dimension grows much
        //    beyond ~1800 px, even when the reported device limits are higher.
        //
        // We keep that policy isolated here so the rest of the web chassis stays platform-agnostic:
        // force the GL canvas path on iOS, and allow descendant layers to downsample below 1x DPR
        // instead of failing surface initialization outright. Root-layer tiling is chosen one
        // level up in the web surface host policy, so the root viewport no longer needs the same
        // backing-dimension cap here.
        let surface_dimension_cap = if ios_webkit {
            Some(IOS_BROWSER_SURFACE_DIMENSION_CAP)
        } else {
            None
        };
        Self {
            force_gl: ios_webkit,
            surface_dimension_cap,
            ios_webkit,
            firefox,
        }
    }

    pub(crate) fn force_gl(self) -> bool {
        self.force_gl
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
        if self.firefox {
            // Firefox reveals entering tiles before replay catches up more readily than Chrome or
            // Safari. Keep extra vertical runway, but use the default tile size so fast scrolling
            // does not increase retarget/swap frequency.
            policy.prewarm_viewport_pad_y_multiplier = FIREFOX_PREWARM_VIEWPORT_PAD_Y_MULTIPLIER;
            policy.prewarm_viewport_pad_min_y = FIREFOX_PREWARM_VIEWPORT_PAD_MIN_Y;
        }
        if self.ios_webkit {
            // Keep Safari's fixed WebGL budget explicit at the planner boundary. The JS canvas
            // pool still enforces the page-wide cap, but the engine should not request a per-layer
            // surface set that the chassis can never materialize.
            policy.tile_overscan_rows = 0;
            policy.max_surfaces_per_layer = Some(IOS_TOTAL_CANVAS_BUDGET);
        }
        policy
    }
}
