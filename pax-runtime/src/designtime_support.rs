#[cfg(feature = "designtime")]
use std::cell::RefCell;
#[cfg(feature = "designtime")]
use std::rc::Rc;

#[cfg(feature = "designtime")]
use pax_designtime::{orm::ReloadType, DesigntimeManager};
#[cfg(feature = "designtime")]
use pax_manifest::{PaxManifest, UniqueTemplateNodeIdentifier};
#[cfg(feature = "designtime")]
use serde::Serialize;

#[cfg(feature = "designtime")]
use crate::api::math::TransformParts;
#[cfg(feature = "designtime")]
use crate::{
    DefinitionToInstanceTraverser, ExpandedNode, InstanceNode, PaxEngine, ReusableInstanceNodeArgs,
};

#[cfg(feature = "designtime")]
pub const USERLAND_COMPONENT_ROOT: &str = "USERLAND_COMPONENT_ROOT";

#[cfg(feature = "designtime")]
#[derive(Serialize)]
pub struct DesigntimeInspectTreePayload {
    pub status: String,
    pub node_count: Option<usize>,
    pub tree_json: Option<String>,
    pub error: Option<String>,
}

#[cfg(feature = "designtime")]
#[derive(Serialize)]
struct DesigntimeInspectTreeNode {
    engine_id: u32,
    #[serde(rename = "type")]
    node_type: String,
    user_id: Option<String>,
    template: Option<DesigntimeInspectTemplateIdentity>,
    flags: DesigntimeInspectNodeFlags,
    layout: DesigntimeInspectNodeLayout,
    occlusion: DesigntimeInspectNodeOcclusion,
    parent_frame: Option<u32>,
    attached: u32,
    suspended: bool,
    effective_raycastable: bool,
    child_count: usize,
    truncated_child_count: usize,
    children: Vec<DesigntimeInspectTreeNode>,
}

#[cfg(feature = "designtime")]
#[derive(Serialize)]
struct DesigntimeInspectTemplateIdentity {
    global_id: UniqueTemplateNodeIdentifier,
    display: String,
    component: String,
    template_node_id: usize,
    source_path: Option<String>,
}

#[cfg(feature = "designtime")]
#[derive(Serialize)]
struct DesigntimeInspectNodeFlags {
    layer: String,
    invisible_to_slot: bool,
    invisible_to_raycasting: bool,
    is_component: bool,
    is_slot: bool,
    unclippable: Option<bool>,
    raycastable_override: Option<bool>,
    suspended_override: Option<bool>,
}

#[cfg(feature = "designtime")]
#[derive(Serialize)]
struct DesigntimeInspectNodeLayout {
    bounds: [f64; 2],
    transform: [f64; 6],
    origin: [f64; 2],
    scale: [f64; 2],
    skew: [f64; 2],
    rotation: f64,
    rendered_size: Option<[f64; 2]>,
}

#[cfg(feature = "designtime")]
#[derive(Serialize)]
struct DesigntimeInspectNodeOcclusion {
    layer_id: usize,
    z_index: i32,
    parent_frame: Option<u32>,
}

#[cfg(feature = "designtime")]
pub fn build_designtime_inspect_tree_payload(
    engine: &PaxEngine,
    definition_to_instance_traverser: &dyn DefinitionToInstanceTraverser,
    max_depth: i64,
) -> DesigntimeInspectTreePayload {
    let root = engine
        .runtime_context
        .get_userland_root_expanded_node()
        .or_else(|| engine.runtime_context.get_root_expanded_node());
    let Some(root) = root else {
        return designtime_inspect_tree_error("no expanded node is available to inspect");
    };

    let manifest = definition_to_instance_traverser.get_manifest();
    let mut node_count = 0;
    let tree = serialize_designtime_inspect_tree_node(
        &root,
        Some(&*manifest),
        0,
        normalize_designtime_max_depth(max_depth),
        &mut node_count,
    );
    let tree_json = match serde_json::to_string(&tree) {
        Ok(tree_json) => tree_json,
        Err(err) => {
            return designtime_inspect_tree_error(format!(
                "failed to serialize inspect tree JSON: {err}"
            ));
        }
    };

    DesigntimeInspectTreePayload {
        status: "ok".to_string(),
        node_count: Some(node_count),
        tree_json: Some(tree_json),
        error: None,
    }
}

#[cfg(feature = "designtime")]
pub fn apply_designtime_userland_reload(
    engine: &mut PaxEngine,
    definition_to_instance_traverser: &dyn DefinitionToInstanceTraverser,
    designtime_manager: &Rc<RefCell<DesigntimeManager>>,
) {
    let current_manifest_version = designtime_manager
        .borrow()
        .get_last_written_manifest_version();
    let last_rendered_manifest_version = designtime_manager
        .borrow()
        .get_last_rendered_manifest_version()
        .get();

    if current_manifest_version.get() == last_rendered_manifest_version {
        return;
    }

    let reload_queue = designtime_manager.borrow_mut().take_reload_queue();
    for reload_type in reload_queue {
        match reload_type {
            ReloadType::Tree => {
                let root = definition_to_instance_traverser.get_main_component(USERLAND_COMPONENT_ROOT)
                    as Rc<dyn InstanceNode>;
                engine.full_reload_userland(root);
            }
            ReloadType::Node(uni, _) => {
                let manifest = definition_to_instance_traverser.get_manifest();
                let containing_component = manifest
                    .components
                    .get(&uni.get_containing_component_type_id())
                    .unwrap();
                let containing_template = containing_component.template.as_ref().unwrap();
                let template_node = containing_template
                    .get_node(&uni.get_template_node_id())
                    .unwrap();

                let nodes = engine
                    .runtime_context
                    .get_expanded_nodes_by_global_ids(&uni);

                let prior_instance_node = nodes
                    .first()
                    .map(|node| ReusableInstanceNodeArgs::new(node.instance_node.borrow().base()));

                let pax_type = template_node.type_id.get_pax_type();
                let instance_node = match pax_type {
                    pax_manifest::PaxType::If
                    | pax_manifest::PaxType::Slot
                    | pax_manifest::PaxType::Repeat => definition_to_instance_traverser
                        .build_control_flow(
                            &uni.get_containing_component_type_id(),
                            &uni.get_template_node_id(),
                            prior_instance_node,
                        ),
                    _ => definition_to_instance_traverser.build_template_node(
                        &uni.get_containing_component_type_id(),
                        &uni.get_template_node_id(),
                        prior_instance_node,
                    ),
                };
                engine.partial_update_expanded_node(Rc::clone(&instance_node));
            }
        }
    }

    designtime_manager
        .borrow()
        .set_last_rendered_manifest_version(current_manifest_version.get());
}

#[cfg(feature = "designtime")]
fn designtime_inspect_tree_error(error: impl Into<String>) -> DesigntimeInspectTreePayload {
    DesigntimeInspectTreePayload {
        status: "error".to_string(),
        node_count: None,
        tree_json: None,
        error: Some(error.into()),
    }
}

#[cfg(feature = "designtime")]
fn normalize_designtime_max_depth(max_depth: i64) -> Option<usize> {
    if max_depth < 0 {
        None
    } else {
        Some(max_depth as usize)
    }
}

#[cfg(feature = "designtime")]
fn serialize_designtime_inspect_tree_node(
    node: &Rc<ExpandedNode>,
    manifest: Option<&PaxManifest>,
    depth: usize,
    max_depth: Option<usize>,
    node_count: &mut usize,
) -> DesigntimeInspectTreeNode {
    *node_count += 1;

    let (flags, node_type, template) = {
        let instance_node = node.instance_node.borrow();
        let base = instance_node.base();
        (
            base.flags().clone(),
            format!("{:?}", &*instance_node),
            base.template_node_identifier
                .clone()
                .map(|global_id| inspect_template_identity(global_id, manifest)),
        )
    };

    let common_properties = Rc::clone(&*node.common_properties.borrow());
    let common_properties = common_properties.borrow();
    let user_id = common_properties.id.get();
    let unclippable = common_properties.unclippable.get();
    let raycastable_override = common_properties._raycastable.get();
    let suspended_override = common_properties._suspended.get();

    let transform_and_bounds = node.transform_and_bounds.get();
    let transform_parts: TransformParts = transform_and_bounds.transform.into();
    let occlusion = node.occlusion.get();
    let child_nodes = node.mounted_children.borrow();
    let child_count = child_nodes.len();
    let should_truncate = max_depth.is_some_and(|limit| depth >= limit);
    let children = if should_truncate {
        vec![]
    } else {
        child_nodes
            .iter()
            .map(|child| {
                serialize_designtime_inspect_tree_node(
                    child,
                    manifest,
                    depth + 1,
                    max_depth,
                    node_count,
                )
            })
            .collect()
    };
    let truncated_child_count = if should_truncate { child_count } else { 0 };

    DesigntimeInspectTreeNode {
        engine_id: node.id.to_u32(),
        node_type,
        user_id,
        template,
        flags: DesigntimeInspectNodeFlags {
            layer: inspect_layer_name(flags.layer).to_string(),
            invisible_to_slot: flags.invisible_to_slot,
            invisible_to_raycasting: flags.invisible_to_raycasting,
            is_component: flags.is_component,
            is_slot: flags.is_slot,
            unclippable,
            raycastable_override,
            suspended_override,
        },
        layout: DesigntimeInspectNodeLayout {
            bounds: [transform_and_bounds.bounds.0, transform_and_bounds.bounds.1],
            transform: transform_and_bounds.transform.coeffs(),
            origin: [transform_parts.origin.x, transform_parts.origin.y],
            scale: [transform_parts.scale.x, transform_parts.scale.y],
            skew: [transform_parts.skew.x, transform_parts.skew.y],
            rotation: transform_parts.rotation,
            rendered_size: node
                .rendered_size
                .get()
                .map(|(width, height)| [width, height]),
        },
        occlusion: DesigntimeInspectNodeOcclusion {
            layer_id: occlusion.occlusion_layer_id,
            z_index: occlusion.z_index,
            parent_frame: occlusion.parent_frame,
        },
        parent_frame: node.parent_frame.get().map(|id| id.to_u32()),
        attached: node.attached.get(),
        suspended: node.suspended.get(),
        effective_raycastable: raycastable_override.unwrap_or(!flags.invisible_to_raycasting),
        child_count,
        truncated_child_count,
        children,
    }
}

#[cfg(feature = "designtime")]
fn inspect_template_identity(
    global_id: UniqueTemplateNodeIdentifier,
    manifest: Option<&PaxManifest>,
) -> DesigntimeInspectTemplateIdentity {
    let component = global_id.get_containing_component_type_id();
    let source_path = manifest
        .and_then(|manifest| manifest.components.get(&component))
        .and_then(|component| component.template.as_ref())
        .and_then(|template| template.get_file_path());

    DesigntimeInspectTemplateIdentity {
        display: global_id.to_string(),
        component: component.to_string(),
        template_node_id: global_id.get_template_node_id().as_usize(),
        source_path,
        global_id,
    }
}

#[cfg(feature = "designtime")]
fn inspect_layer_name(layer: crate::api::Layer) -> &'static str {
    match layer {
        crate::api::Layer::Native => "native",
        crate::api::Layer::Canvas => "canvas",
        crate::api::Layer::DontCare => "dont-care",
    }
}
