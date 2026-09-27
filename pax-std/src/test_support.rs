use pax_runtime::api::{math::Transform2, Platform, Property, TargetInfo, OS};
use pax_runtime::{Globals, RouteLocation, RuntimeContext, TransformAndBounds};
use std::rc::Rc;

pub(crate) fn runtime_context() -> RuntimeContext {
    runtime_context_with_target((100.0, 100.0), Platform::Unknown, OS::Unknown)
}

pub(crate) fn runtime_context_with_target(
    bounds: (f64, f64),
    platform: Platform,
    os: OS,
) -> RuntimeContext {
    let globals = Globals {
        elapsed_frames: Property::new(0),
        elapsed_millis: Property::new(0),
        viewport: Property::new(TransformAndBounds {
            transform: Transform2::identity(),
            bounds,
        }),
        gyro: Property::default(),
        accel: Property::default(),
        route_location: Property::new(RouteLocation::root()),
        browser_allows_scroller_vector_layers: Property::new(true),
        browser_allows_nested_scroller_vector_layers: Property::new(true),
        platform,
        os,
        target: TargetInfo::new(platform, os),
        get_elapsed_millis: Rc::new(|| 0),
        #[cfg(feature = "designtime")]
        designtime: Rc::new(std::cell::RefCell::new(
            pax_engine::pax_designtime::DesigntimeManager::new_offline(pax_manifest::PaxManifest {
                components: Default::default(),
                main_component_type_id: Default::default(),
                type_table: Default::default(),
                assets_dirs: Vec::new(),
                engine_import_path: "pax_engine".into(),
            }),
        )),
    };
    #[cfg(feature = "designtime")]
    {
        RuntimeContext::new_empty(globals)
    }
    #[cfg(not(feature = "designtime"))]
    {
        RuntimeContext::new(globals)
    }
}
