#[cfg(feature = "designtime")]
use std::cell::RefCell;
#[cfg(feature = "designtime")]
use std::rc::Rc;

#[cfg(feature = "designtime")]
use pax_designtime::{
    orm::{template::SerializedTemplateNodeSubtree, ReloadType},
    DesigntimeManager,
};
#[cfg(feature = "designtime")]
use pax_language::{parse_pax_str, Rule};
#[cfg(feature = "designtime")]
use pax_manifest::{
    ComponentTemplate, PaxManifest, SettingElement, TypeId, UniqueTemplateNodeIdentifier,
    ValueDefinition,
};
#[cfg(feature = "designtime")]
use serde::Serialize;

#[cfg(feature = "designtime")]
use crate::api::math::{Point2, TransformParts};
#[cfg(feature = "designtime")]
use crate::{
    api::Window, DefinitionToInstanceTraverser, ExpandedNode, InstanceNode, PaxEngine,
    ReusableInstanceNodeArgs,
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
pub struct DesigntimeInspectNodeListPayload {
    pub status: String,
    pub node_count: Option<usize>,
    pub nodes_json: Option<String>,
    pub error: Option<String>,
}

#[cfg(feature = "designtime")]
#[derive(Serialize)]
pub struct DesigntimeReplaceNodePayload {
    pub status: String,
    pub component_type_id: String,
    pub template_node_id: usize,
    pub reload_scope: String,
    pub reloaded_template_node_id: Option<usize>,
    pub source_path: Option<String>,
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
enum DesigntimeSelectorQuery {
    Id(String),
    Class(String),
    Type(String),
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
pub fn build_designtime_ray_cast_payload(
    engine: &PaxEngine,
    definition_to_instance_traverser: &dyn DefinitionToInstanceTraverser,
    x: f64,
    y: f64,
    hit_invisible: bool,
) -> DesigntimeInspectNodeListPayload {
    let root = engine
        .runtime_context
        .get_userland_root_expanded_node()
        .or_else(|| engine.runtime_context.get_root_expanded_node());
    let Some(root) = root else {
        return designtime_inspect_nodes_error("no expanded node is available to inspect");
    };

    let manifest = definition_to_instance_traverser.get_manifest();
    let nodes = engine.runtime_context.get_elements_beneath_ray(
        Some(root),
        Point2::<Window>::new(x, y),
        false,
        vec![],
        hit_invisible,
    );
    build_designtime_inspect_node_list_payload(nodes, Some(&*manifest))
}

#[cfg(feature = "designtime")]
pub fn build_designtime_selector_query_payload(
    engine: &PaxEngine,
    definition_to_instance_traverser: &dyn DefinitionToInstanceTraverser,
    selector: &str,
) -> DesigntimeInspectNodeListPayload {
    let selector = match parse_designtime_selector_query(selector) {
        Ok(selector) => selector,
        Err(error) => return designtime_inspect_nodes_error(error),
    };

    let root = engine
        .runtime_context
        .get_userland_root_expanded_node()
        .or_else(|| engine.runtime_context.get_root_expanded_node());
    let Some(root) = root else {
        return designtime_inspect_nodes_error("no expanded node is available to inspect");
    };

    let manifest = definition_to_instance_traverser.get_manifest();
    let mut matches = vec![];
    collect_designtime_selector_matches(&root, &manifest, &selector, &mut matches);
    build_designtime_inspect_node_list_payload(matches, Some(&*manifest))
}

#[cfg(feature = "designtime")]
pub fn apply_designtime_replace_node_subtemplate(
    definition_to_instance_traverser: &dyn DefinitionToInstanceTraverser,
    designtime_manager: &Rc<RefCell<DesigntimeManager>>,
    component_type_id_str: &str,
    template_node_id: usize,
    subtemplate: &str,
) -> DesigntimeReplaceNodePayload {
    let component_type_id = {
        let manifest = definition_to_instance_traverser.get_manifest();
        match resolve_designtime_component_type_id(&manifest, component_type_id_str) {
            Ok(component_type_id) => component_type_id,
            Err(error) => {
                return designtime_replace_node_error(
                    component_type_id_str,
                    template_node_id,
                    "error",
                    None,
                    error,
                );
            }
        }
    };

    let target_uni = UniqueTemplateNodeIdentifier::build(
        component_type_id.clone(),
        pax_manifest::TemplateNodeId::build(template_node_id),
    );

    let (new_subtree, reload_uni, reload_scope) = {
        let manifest = definition_to_instance_traverser.get_manifest();
        let Some(component) = manifest.components.get(&component_type_id) else {
            return designtime_replace_node_error(
                component_type_id.to_string(),
                template_node_id,
                "error",
                None,
                "target component is not present in the current manifest",
            );
        };
        let Some(template) = component.template.as_ref() else {
            return designtime_replace_node_error(
                component_type_id.to_string(),
                template_node_id,
                "error",
                None,
                "target component does not have a template",
            );
        };
        if template
            .get_node(&target_uni.get_template_node_id())
            .is_none()
        {
            return designtime_replace_node_error(
                component_type_id.to_string(),
                template_node_id,
                "error",
                None,
                "target template node does not exist",
            );
        }

        if subtemplate.trim().is_empty() {
            let parent_uni =
                template
                    .get_parent(&target_uni.get_template_node_id())
                    .map(|parent_id| {
                        UniqueTemplateNodeIdentifier::build(component_type_id.clone(), parent_id)
                    });
            let reload_scope = if parent_uni.is_some() {
                "parent-subtree".to_string()
            } else {
                "tree".to_string()
            };
            (None, parent_uni, reload_scope)
        } else {
            match parse_designtime_subtemplate(&manifest, &component_type_id, subtemplate) {
                Ok(parsed_subtree) => (
                    Some(parsed_subtree),
                    Some(target_uni.clone()),
                    "subtree".to_string(),
                ),
                Err(error) => {
                    return designtime_replace_node_error(
                        component_type_id.to_string(),
                        template_node_id,
                        "error",
                        None,
                        error,
                    );
                }
            }
        }
    };

    let source_path = {
        let manifest = definition_to_instance_traverser.get_manifest();
        manifest
            .components
            .get(&component_type_id)
            .and_then(|component| component.template.as_ref())
            .and_then(ComponentTemplate::get_file_path)
    };

    if let Err(error) = designtime_manager
        .borrow_mut()
        .get_orm_mut()
        .replace_node_subtree(target_uni, new_subtree, reload_uni.clone())
    {
        return designtime_replace_node_error(
            component_type_id.to_string(),
            template_node_id,
            &reload_scope,
            reload_uni
                .as_ref()
                .map(|uni| uni.get_template_node_id().as_usize()),
            error,
        );
    }

    if let Err(error) = designtime_manager
        .borrow_mut()
        .send_component_update(&component_type_id)
    {
        return designtime_replace_node_error(
            component_type_id.to_string(),
            template_node_id,
            &reload_scope,
            reload_uni
                .as_ref()
                .map(|uni| uni.get_template_node_id().as_usize()),
            format!("failed to serialize updated component: {error}"),
        );
    }

    DesigntimeReplaceNodePayload {
        status: "ok".to_string(),
        component_type_id: component_type_id.to_string(),
        template_node_id,
        reload_scope,
        reloaded_template_node_id: reload_uni
            .as_ref()
            .map(|uni| uni.get_template_node_id().as_usize()),
        source_path,
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
                let root = definition_to_instance_traverser
                    .get_main_component(USERLAND_COMPONENT_ROOT)
                    as Rc<dyn InstanceNode>;
                engine.full_reload_userland(root);
            }
            ReloadType::Subtree(uni) => {
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

                let pax_type = template_node.type_id.get_pax_type();
                let instance_node = match pax_type {
                    pax_manifest::PaxType::If
                    | pax_manifest::PaxType::Slot
                    | pax_manifest::PaxType::Repeat => definition_to_instance_traverser
                        .build_control_flow(
                            &uni.get_containing_component_type_id(),
                            &uni.get_template_node_id(),
                            None,
                        ),
                    _ => definition_to_instance_traverser.build_template_node(
                        &uni.get_containing_component_type_id(),
                        &uni.get_template_node_id(),
                        None,
                    ),
                };

                for node in nodes {
                    node.fully_recreate_with_new_data(
                        Rc::clone(&instance_node),
                        &engine.runtime_context,
                    );
                }
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
fn designtime_inspect_nodes_error(error: impl Into<String>) -> DesigntimeInspectNodeListPayload {
    DesigntimeInspectNodeListPayload {
        status: "error".to_string(),
        node_count: None,
        nodes_json: None,
        error: Some(error.into()),
    }
}

#[cfg(feature = "designtime")]
fn designtime_replace_node_error(
    component_type_id: impl Into<String>,
    template_node_id: usize,
    reload_scope: impl Into<String>,
    reloaded_template_node_id: Option<usize>,
    error: impl Into<String>,
) -> DesigntimeReplaceNodePayload {
    DesigntimeReplaceNodePayload {
        status: "error".to_string(),
        component_type_id: component_type_id.into(),
        template_node_id,
        reload_scope: reload_scope.into(),
        reloaded_template_node_id,
        source_path: None,
        error: Some(error.into()),
    }
}

#[cfg(feature = "designtime")]
fn resolve_designtime_component_type_id(
    manifest: &PaxManifest,
    component_type_id_str: &str,
) -> Result<TypeId, String> {
    manifest
        .components
        .keys()
        .find(|type_id| {
            type_id.to_string() == component_type_id_str
                || type_id.get_pascal_identifier().as_deref() == Some(component_type_id_str)
        })
        .cloned()
        .ok_or_else(|| {
            format!("component {component_type_id_str} is not present in the current manifest")
        })
}

#[cfg(feature = "designtime")]
fn parse_designtime_subtemplate(
    manifest: &PaxManifest,
    containing_component_type_id: &TypeId,
    subtemplate: &str,
) -> Result<SerializedTemplateNodeSubtree, String> {
    let ast = parse_pax_str(Rule::pax_component_definition, subtemplate)
        .map_err(|error| format!("subtemplate failed to parse: {error}"))?;
    let settings =
        pax_manifest::parsing::parse_settings_from_component_definition_string(ast.clone());
    if !settings.is_empty() {
        return Err(
            "node subtemplates cannot contain a component-level @settings block".to_string(),
        );
    }

    let mut parse_context = pax_manifest::parsing::TemplateNodeParseContext {
        template: ComponentTemplate::new(containing_component_type_id.clone(), None),
        pascal_identifier_to_type_id_map: manifest
            .components
            .iter()
            .filter_map(|(type_id, _)| {
                type_id
                    .get_pascal_identifier()
                    .map(|identifier| (identifier, type_id.clone()))
            })
            .collect(),
    };
    pax_manifest::parsing::parse_template_from_component_definition_string(
        &mut parse_context,
        subtemplate,
        ast,
    );

    let root_ids = parse_context.template.get_root();
    if root_ids.len() != 1 {
        return Err(format!(
            "node subtemplate must produce exactly one root node, found {}",
            root_ids.len()
        ));
    }

    serialize_template_subtree(&parse_context.template, &root_ids[0])
}

#[cfg(feature = "designtime")]
fn serialize_template_subtree(
    template: &ComponentTemplate,
    node_id: &pax_manifest::TemplateNodeId,
) -> Result<SerializedTemplateNodeSubtree, String> {
    let node = template
        .get_node(node_id)
        .cloned()
        .ok_or_else(|| format!("template node {node_id} is missing from parsed subtemplate"))?;
    let children = template
        .get_children(node_id)
        .unwrap_or_default()
        .into_iter()
        .map(|child_id| serialize_template_subtree(template, &child_id))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SerializedTemplateNodeSubtree { node, children })
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
fn build_designtime_inspect_node_list_payload(
    nodes: Vec<Rc<ExpandedNode>>,
    manifest: Option<&PaxManifest>,
) -> DesigntimeInspectNodeListPayload {
    let mut node_count = 0;
    let serialized_nodes = nodes
        .iter()
        .map(|node| {
            serialize_designtime_inspect_tree_node(node, manifest, 0, Some(0), &mut node_count)
        })
        .collect::<Vec<_>>();
    let nodes_json = match serde_json::to_string(&serialized_nodes) {
        Ok(nodes_json) => nodes_json,
        Err(err) => {
            return designtime_inspect_nodes_error(format!(
                "failed to serialize inspect node list JSON: {err}"
            ));
        }
    };

    DesigntimeInspectNodeListPayload {
        status: "ok".to_string(),
        node_count: Some(serialized_nodes.len()),
        nodes_json: Some(nodes_json),
        error: None,
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
        crate::api::Layer::NativeNonOccluding => "native-non-occluding",
        crate::api::Layer::Canvas => "canvas",
        crate::api::Layer::DontCare => "dont-care",
    }
}

#[cfg(feature = "designtime")]
fn parse_designtime_selector_query(selector: &str) -> Result<DesigntimeSelectorQuery, String> {
    let trimmed = selector.trim();
    if trimmed.is_empty() {
        return Err("selector cannot be empty".to_string());
    }

    if trimmed.starts_with('#') || trimmed.starts_with('.') {
        parse_pax_str(Rule::selector, trimmed)
            .map_err(|error| format!("selector failed to parse: {error}"))?;
        let (prefix, value) = trimmed.split_at(1);
        return match prefix {
            "#" => Ok(DesigntimeSelectorQuery::Id(value.to_string())),
            "." => Ok(DesigntimeSelectorQuery::Class(value.to_string())),
            _ => unreachable!("validated selector must start with # or ."),
        };
    }

    if trimmed
        .split("::")
        .all(|segment| !segment.is_empty() && is_designtime_identifier(segment))
    {
        return Ok(DesigntimeSelectorQuery::Type(trimmed.to_string()));
    }

    Err(
        "selector must be an id (`#id`), class (`.class`), or element type (`Ellipse` or `crate::Example`)"
            .to_string(),
    )
}

#[cfg(feature = "designtime")]
fn is_designtime_identifier(identifier: &str) -> bool {
    let mut chars = identifier.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

#[cfg(feature = "designtime")]
fn collect_designtime_selector_matches(
    node: &Rc<ExpandedNode>,
    manifest: &PaxManifest,
    selector: &DesigntimeSelectorQuery,
    matches: &mut Vec<Rc<ExpandedNode>>,
) {
    node.compute_flattened_slot_children();
    if designtime_selector_matches_node(node, manifest, selector) {
        matches.push(Rc::clone(node));
    }

    let children = node
        .mounted_children
        .borrow()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    for child in children {
        collect_designtime_selector_matches(&child, manifest, selector, matches);
    }
}

#[cfg(feature = "designtime")]
fn designtime_selector_matches_node(
    node: &Rc<ExpandedNode>,
    manifest: &PaxManifest,
    selector: &DesigntimeSelectorQuery,
) -> bool {
    match selector {
        DesigntimeSelectorQuery::Id(id) => {
            let common_properties = Rc::clone(&*node.common_properties.borrow());
            let common_properties = common_properties.borrow();
            common_properties.id.get().as_deref() == Some(id.as_str())
        }
        DesigntimeSelectorQuery::Class(class_name) => {
            designtime_node_template_classes(node, manifest)
                .into_iter()
                .any(|node_class| node_class == class_name.as_str())
        }
        DesigntimeSelectorQuery::Type(type_name) => {
            designtime_node_matches_type(node, manifest, type_name)
        }
    }
}

#[cfg(feature = "designtime")]
fn designtime_node_template_classes<'a>(
    node: &Rc<ExpandedNode>,
    manifest: &'a PaxManifest,
) -> Vec<&'a str> {
    let global_id = {
        let instance_node = node.instance_node.borrow();
        instance_node.base().template_node_identifier.clone()
    };
    let Some(global_id) = global_id else {
        return vec![];
    };
    let Some(template_node) = manifest.get_template_node(&global_id) else {
        return vec![];
    };
    let Some(settings) = template_node.settings.as_ref() else {
        return vec![];
    };

    settings
        .iter()
        .filter_map(|setting| match setting {
            SettingElement::Setting(token, ValueDefinition::Identifier(identifier))
                if token.token_value == "class" =>
            {
                Some(identifier.name.as_str())
            }
            _ => None,
        })
        .collect()
}

#[cfg(feature = "designtime")]
fn designtime_node_matches_type(
    node: &Rc<ExpandedNode>,
    manifest: &PaxManifest,
    type_name: &str,
) -> bool {
    let global_id = {
        let instance_node = node.instance_node.borrow();
        instance_node.base().template_node_identifier.clone()
    };
    let Some(global_id) = global_id else {
        return false;
    };
    let Some(template_node) = manifest.get_template_node(&global_id) else {
        return false;
    };
    template_node.type_id.to_string() == type_name
        || template_node.type_id.get_pascal_identifier().as_deref() == Some(type_name)
}
