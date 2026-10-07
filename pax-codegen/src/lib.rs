//! Source analysis and Rust generation shared by Pax build frontends.
//!
//! This crate has no native compiler, dev server, watcher, or rendering-engine
//! dependency. Generating a cartridge does not initialize its UI.

pub mod cartridge_generation;
#[doc(hidden)]
pub mod configured_source;
pub mod program_catalog;
#[doc(hidden)]
pub mod project_layout;
pub mod static_analysis;
pub mod svg_import;

use pax_manifest::{
    ComponentDefinition, ComponentTemplate, PaxManifest, TemplateNodeDefinition, TypeId,
};

/// Generate a cartridge in memory, without creating a build directory.
///
/// `baked` chooses the release program representation. `designtime` is only
/// meaningful for development data and must not be combined with baked data.
pub fn generate(manifest: &PaxManifest, designtime: bool, baked: bool) -> String {
    assert!(
        !(designtime && baked),
        "baked cartridges cannot enable designtime"
    );
    let mut runtime = manifest.clone();
    add_runtime_root(&mut runtime);
    cartridge_generation::render_cartridge(manifest, &runtime, designtime, baked)
}

/// Add the runtime wrapper that gives the authored root a template-node identity.
pub fn add_runtime_root(manifest: &mut PaxManifest) {
    let wrapper = TypeId::build_singleton("ROOT_COMPONENT", Some("RootComponent"));
    let mut node = TemplateNodeDefinition::default();
    node.type_id = manifest.main_component_type_id.clone();
    let mut template = ComponentTemplate::new(wrapper.clone(), None);
    template.add(node);
    manifest.components.insert(
        wrapper.clone(),
        ComponentDefinition {
            type_id: wrapper,
            is_main_component: false,
            is_primitive: false,
            is_struct_only_component: false,
            module_path: String::new(),
            primitive_instance_import_path: None,
            template: Some(template),
            settings: None,
            timelines: Vec::new(),
            route_branch: None,
        },
    );
}
