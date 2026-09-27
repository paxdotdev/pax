use crate::{engine::layer_tiling::ScrollerTilingPolicy, Globals, PaxEngine, RuntimeContext};
use pax_runtime_api::{Platform, OS};

#[cfg(feature = "designtime")]
pub(crate) fn designtime() -> std::rc::Rc<std::cell::RefCell<pax_designtime::DesigntimeManager>> {
    std::rc::Rc::new(std::cell::RefCell::new(
        pax_designtime::DesigntimeManager::new_offline(pax_manifest::PaxManifest {
            components: Default::default(),
            main_component_type_id: Default::default(),
            type_table: Default::default(),
            assets_dirs: Vec::new(),
            engine_import_path: "pax_engine".into(),
        }),
    ))
}

pub(crate) fn runtime_context(globals: Globals) -> RuntimeContext {
    #[cfg(feature = "designtime")]
    {
        RuntimeContext::new_empty(globals)
    }
    #[cfg(not(feature = "designtime"))]
    {
        RuntimeContext::new(globals)
    }
}

pub(crate) fn empty_engine(
    viewport_size: (f64, f64),
    platform: Platform,
    os: OS,
    clock: Box<dyn Fn() -> u128>,
    policy: ScrollerTilingPolicy,
) -> PaxEngine {
    #[cfg(feature = "designtime")]
    {
        PaxEngine::new_empty_with_designtime(
            viewport_size,
            designtime(),
            platform,
            os,
            clock,
            policy,
        )
    }
    #[cfg(not(feature = "designtime"))]
    {
        PaxEngine::new_empty(viewport_size, platform, os, clock, policy)
    }
}
