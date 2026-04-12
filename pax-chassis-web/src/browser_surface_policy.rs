use web_sys::Window;

const IOS_BROWSER_SURFACE_DIMENSION_CAP: u32 = 1800;
const IOS_NESTED_LAYER_MIN_DPR: f64 = 0.25;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct BrowserSurfacePolicy {
    force_gl: bool,
    surface_dimension_cap: Option<u32>,
}

impl BrowserSurfacePolicy {
    pub(crate) fn detect(window: &Window) -> Self {
        let Ok(user_agent) = window.navigator().user_agent() else {
            return Self::default();
        };

        let isi_os = user_agent.contains("iPhone")
            || user_agent.contains("iPad")
            || user_agent.contains("iPod");
        if !isi_os {
            return Self::default();
        }

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
        Self {
            force_gl: true,
            surface_dimension_cap: Some(IOS_BROWSER_SURFACE_DIMENSION_CAP),
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
}
