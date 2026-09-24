use crate::helpers::INTERFACE_DIR_NAME;
use crate::project_metadata::PaxProjectMetadata;
use color_eyre::eyre;
use eyre::{eyre, WrapErr};
use pax_manifest::{
    ComponentTemplate, PaxManifest, PaxType, RouteMetadataDefinition, TemplateNodeId, TypeId,
};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const ROUTE_CATALOG_FILE: &str = "route-metadata.json";
const ROUTE_METADATA_START: &str = "<!-- pax-route-metadata:start -->";
const ROUTE_METADATA_END: &str = "<!-- pax-route-metadata:end -->";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct WebRouteCatalog {
    pub version: u8,
    pub site: WebSiteMetadata,
    pub routes: Vec<WebRouteCatalogEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct WebSiteMetadata {
    pub title: String,
    pub site_name: String,
    pub site_url: Option<String>,
    pub base_path: String,
    pub social_image: Option<String>,
    pub social_image_alt: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct WebRouteCatalogEntry {
    pub pattern: String,
    pub concrete_path: Option<String>,
    pub depth: usize,
    pub order: usize,
    pub is_default: bool,
    pub metadata: RouteMetadataDefinition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RouteSegment {
    Literal(String),
    Param(String),
    CatchAll,
}

#[derive(Clone)]
struct WalkContext {
    prefix: Vec<RouteSegment>,
    scope_pattern: Vec<RouteSegment>,
    can_consume_segments: bool,
    inherited_metadata: Option<RouteMetadataDefinition>,
    route_depth: usize,
    dynamic_topology: bool,
}

struct RouteCollector<'a> {
    manifest: &'a PaxManifest,
    routes: Vec<WebRouteCatalogEntry>,
    next_order: usize,
    active_components: HashSet<TypeId>,
}

pub(crate) fn prepare_web_route_metadata(
    pax_dir: &Path,
    manifest: &PaxManifest,
    project_metadata: &PaxProjectMetadata,
    is_release: bool,
) -> Result<(), eyre::Report> {
    let interface_path = pax_dir.join(INTERFACE_DIR_NAME).join("web");
    let index_path = interface_path.join("index.html");
    if !index_path.exists() {
        return Ok(());
    }

    let site = web_site_metadata(project_metadata)?;
    let catalog = collect_web_route_catalog(manifest, site)?;
    validate_release_metadata(&catalog, is_release)?;

    fs::write(
        interface_path.join(ROUTE_CATALOG_FILE),
        serde_json::to_string_pretty(&catalog)?,
    )?;

    emit_route_entry_documents(&interface_path, &catalog)
}

fn web_site_metadata(
    project_metadata: &PaxProjectMetadata,
) -> Result<WebSiteMetadata, eyre::Report> {
    let title = project_metadata
        .web_title()
        .unwrap_or_else(|| "Pax Web".to_string());
    let site_name = project_metadata
        .web_site_name()
        .unwrap_or_else(|| title.clone());
    let site_url = project_metadata
        .web_site_url()
        .map(normalize_site_url)
        .transpose()?;
    let base_path = site_url
        .as_deref()
        .and_then(|value| Url::parse(value).ok())
        .map(|url| normalized_base_path(url.path()))
        .unwrap_or_else(|| "/".to_string());

    Ok(WebSiteMetadata {
        title,
        site_name,
        site_url,
        base_path,
        social_image: project_metadata.web_social_image().map(str::to_string),
        social_image_alt: project_metadata.web_social_image_alt().map(str::to_string),
    })
}

fn normalize_site_url(value: &str) -> Result<String, eyre::Report> {
    let mut url = Url::parse(value)
        .wrap_err_with(|| format!("`package.metadata.pax.web.site_url` is invalid: {value}"))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(eyre!(
            "`package.metadata.pax.web.site_url` must be an absolute HTTP(S) URL"
        ));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(eyre!(
            "`package.metadata.pax.web.site_url` cannot contain a query or fragment"
        ));
    }
    let path = normalized_base_path(url.path());
    url.set_path(&path);
    Ok(url.to_string().trim_end_matches('/').to_string())
}

fn normalized_base_path(path: &str) -> String {
    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else {
        format!("/{trimmed}/")
    }
}

fn collect_web_route_catalog(
    manifest: &PaxManifest,
    site: WebSiteMetadata,
) -> Result<WebRouteCatalog, eyre::Report> {
    let mut collector = RouteCollector {
        manifest,
        routes: vec![],
        next_order: 0,
        active_components: HashSet::new(),
    };
    let context = WalkContext {
        prefix: vec![],
        scope_pattern: vec![RouteSegment::CatchAll],
        can_consume_segments: true,
        inherited_metadata: None,
        route_depth: 0,
        dynamic_topology: false,
    };
    collector.walk_component(&manifest.main_component_type_id, &context)?;
    validate_concrete_routes(&collector.routes)?;

    Ok(WebRouteCatalog {
        version: 1,
        site,
        routes: collector.routes,
    })
}

impl RouteCollector<'_> {
    fn walk_component(
        &mut self,
        component_type_id: &TypeId,
        context: &WalkContext,
    ) -> Result<(), eyre::Report> {
        if !self.active_components.insert(component_type_id.clone()) {
            return Ok(());
        }
        let result =
            (|| {
                let component = self.manifest.components.get(component_type_id).ok_or_else(|| {
                eyre!("Route metadata analysis could not resolve component {component_type_id}")
            })?;
                let Some(template) = component.template.as_ref() else {
                    return Ok(());
                };
                self.walk_nodes(template, &template.get_root(), context)
            })();
        self.active_components.remove(component_type_id);
        result
    }

    fn walk_nodes(
        &mut self,
        template: &ComponentTemplate,
        node_ids: &[TemplateNodeId],
        context: &WalkContext,
    ) -> Result<(), eyre::Report> {
        for node_id in node_ids {
            let Some(node) = template.get_node(node_id) else {
                continue;
            };

            if matches!(node.type_id.get_pax_type(), PaxType::Router) {
                self.walk_router(template, node_id, context)?;
                continue;
            }

            let mut child_context = context.clone();
            if matches!(
                node.type_id.get_pax_type(),
                PaxType::If | PaxType::Repeat | PaxType::Slot
            ) {
                child_context.dynamic_topology = true;
            }

            if let Some(component) = self.manifest.components.get(&node.type_id) {
                if component.template.is_some() {
                    self.walk_component(&node.type_id, &child_context)?;
                    if template
                        .get_children(node_id)
                        .map(|children| !children.is_empty())
                        .unwrap_or(false)
                    {
                        // Children of component instances are projected through
                        // slots, so their final route position is not statically
                        // determined by the parent template alone.
                        child_context.dynamic_topology = true;
                    }
                }
            }
            if let Some(children) = template.get_children(node_id) {
                self.walk_nodes(template, &children, &child_context)?;
            }
        }
        Ok(())
    }

    fn walk_router(
        &mut self,
        template: &ComponentTemplate,
        router_id: &TemplateNodeId,
        context: &WalkContext,
    ) -> Result<(), eyre::Report> {
        let node = template
            .get_node(router_id)
            .ok_or_else(|| eyre!("Route metadata analysis lost Router node {router_id}"))?;
        let settings = node.control_flow_settings.as_ref().ok_or_else(|| {
            eyre!("Route metadata analysis found Router without control-flow settings")
        })?;

        for branch in &settings.route_branches {
            let effective_metadata = branch
                .metadata
                .clone()
                .or_else(|| context.inherited_metadata.clone());
            let next_depth = context.route_depth + 1;

            let (pattern, concrete_path, next_prefix, next_scope_pattern, next_can_consume) =
                if branch.default {
                    let concrete_path = if context.prefix.is_empty() {
                        None
                    } else {
                        literal_path(&context.prefix)
                    };
                    (
                        context.scope_pattern.clone(),
                        concrete_path,
                        context.prefix.clone(),
                        context.scope_pattern.clone(),
                        context.can_consume_segments,
                    )
                } else {
                    let raw_path = branch.path.as_deref().ok_or_else(|| {
                        eyre!("Route metadata analysis found Route without path or default")
                    })?;
                    let local_pattern = parse_route_pattern(raw_path)?;
                    if !context.can_consume_segments && !local_pattern.is_empty() {
                        if effective_metadata
                            .as_ref()
                            .map(|metadata| metadata.index)
                            .unwrap_or(false)
                        {
                            return Err(eyre!(
                                "Indexable route `{raw_path}` is unreachable because its ancestor consumed the full scoped path; use a terminal `*` on the ancestor before nesting additional routes"
                            ));
                        }
                    }
                    let mut full_pattern = context.prefix.clone();
                    full_pattern.extend(local_pattern);
                    let has_catch_all = matches!(full_pattern.last(), Some(RouteSegment::CatchAll));
                    let concrete_path = concrete_route_path(&full_pattern);
                    let next_prefix = if has_catch_all {
                        full_pattern[..full_pattern.len() - 1].to_vec()
                    } else {
                        full_pattern.clone()
                    };
                    (
                        full_pattern.clone(),
                        concrete_path,
                        next_prefix,
                        full_pattern,
                        has_catch_all,
                    )
                };

            if let Some(metadata) = effective_metadata.clone() {
                if context.dynamic_topology && metadata.index {
                    return Err(eyre!(
                        "Indexable route `{}` is behind dynamic `if`, `repeat`, or slot topology and cannot produce deterministic web metadata",
                        display_route_pattern(&pattern)
                    ));
                }
                self.routes.push(WebRouteCatalogEntry {
                    pattern: display_route_pattern(&pattern),
                    concrete_path,
                    depth: next_depth,
                    order: self.next_order,
                    is_default: branch.default,
                    metadata,
                });
                self.next_order += 1;
            }

            let child_context = WalkContext {
                prefix: next_prefix,
                scope_pattern: next_scope_pattern,
                can_consume_segments: next_can_consume,
                inherited_metadata: effective_metadata,
                route_depth: next_depth,
                dynamic_topology: context.dynamic_topology,
            };
            for child_id in &branch.child_ids {
                let node = template
                    .get_node(child_id)
                    .ok_or_else(|| eyre!("Route metadata analysis lost branch node {child_id}"))?;
                let is_route_shell = self
                    .manifest
                    .components
                    .get(&node.type_id)
                    .and_then(|component| component.route_branch.as_ref())
                    .is_some();
                if is_route_shell {
                    // The parser retains the declared branch's presentation shell. Its
                    // caller content is already scoped by this static route selection;
                    // it is not an arbitrary component projection boundary. Analyze the
                    // shell's own template too, without relaxing its internal if/slots.
                    self.walk_component(&node.type_id, &child_context)?;
                    if let Some(children) = template.get_children(child_id) {
                        self.walk_nodes(template, &children, &child_context)?;
                    }
                } else {
                    self.walk_nodes(template, std::slice::from_ref(child_id), &child_context)?;
                }
            }
        }
        Ok(())
    }
}

fn parse_route_pattern(path: &str) -> Result<Vec<RouteSegment>, eyre::Report> {
    if path.is_empty() {
        return Err(eyre!("Route path must not be empty"));
    }
    let normalized = if path == "/" {
        ""
    } else {
        path.trim_matches('/')
    };
    if normalized.is_empty() {
        return Ok(vec![]);
    }

    let raw_segments = normalized
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let mut segments = Vec::with_capacity(raw_segments.len());
    for (index, segment) in raw_segments.iter().enumerate() {
        if *segment == "*" {
            if index + 1 != raw_segments.len() {
                return Err(eyre!("Route catch-all `*` must be terminal in `{path}`"));
            }
            segments.push(RouteSegment::CatchAll);
        } else if let Some(param) = segment.strip_prefix(':') {
            if param.is_empty() {
                return Err(eyre!("Route params must be named in `{path}`"));
            }
            segments.push(RouteSegment::Param(param.to_string()));
        } else if segment.contains('*') {
            return Err(eyre!("Route `{path}` only supports a terminal `*` segment"));
        } else {
            segments.push(RouteSegment::Literal((*segment).to_string()));
        }
    }
    Ok(segments)
}

fn display_route_pattern(pattern: &[RouteSegment]) -> String {
    if pattern.is_empty() {
        return "/".to_string();
    }
    format!(
        "/{}",
        pattern
            .iter()
            .map(|segment| match segment {
                RouteSegment::Literal(value) => value.clone(),
                RouteSegment::Param(value) => format!(":{value}"),
                RouteSegment::CatchAll => "*".to_string(),
            })
            .collect::<Vec<_>>()
            .join("/")
    )
}

fn literal_path(pattern: &[RouteSegment]) -> Option<String> {
    if pattern
        .iter()
        .any(|segment| !matches!(segment, RouteSegment::Literal(_)))
    {
        return None;
    }
    Some(display_route_pattern(pattern))
}

fn concrete_route_path(pattern: &[RouteSegment]) -> Option<String> {
    let without_catch_all = if matches!(pattern.last(), Some(RouteSegment::CatchAll)) {
        &pattern[..pattern.len() - 1]
    } else {
        pattern
    };
    literal_path(without_catch_all)
}

fn validate_concrete_routes(routes: &[WebRouteCatalogEntry]) -> Result<(), eyre::Report> {
    let mut concrete: BTreeMap<&str, &WebRouteCatalogEntry> = BTreeMap::new();
    for route in routes {
        let Some(path) = route.concrete_path.as_deref() else {
            continue;
        };
        let Some(existing) = concrete.get(path) else {
            concrete.insert(path, route);
            continue;
        };
        if route.depth == existing.depth {
            if existing.is_default && !route.is_default {
                concrete.insert(path, route);
            } else if existing.is_default == route.is_default && route.metadata != existing.metadata
            {
                return Err(eyre!(
                    "Ambiguous route metadata: concrete path `{path}` has competing declarations at the same nested route depth"
                ));
            }
            continue;
        }
        if route.depth > existing.depth {
            concrete.insert(path, route);
        }
    }
    Ok(())
}

fn validate_release_metadata(
    catalog: &WebRouteCatalog,
    is_release: bool,
) -> Result<(), eyre::Report> {
    if !is_release {
        return Ok(());
    }
    if catalog
        .routes
        .iter()
        .any(|route| route.metadata.index && route.concrete_path.is_some())
        && catalog.site.site_url.is_none()
    {
        return Err(eyre!(
            "Release web builds with indexable RouteMetadata require `package.metadata.pax.web.site_url`"
        ));
    }
    Ok(())
}

fn emit_route_entry_documents(
    interface_path: &Path,
    catalog: &WebRouteCatalog,
) -> Result<(), eyre::Report> {
    let index_path = interface_path.join("index.html");
    let base_html = fs::read_to_string(&index_path)?;
    let base_html = set_or_insert_base_href(&base_html, &catalog.site.base_path);
    let concrete_routes = preferred_concrete_routes(&catalog.routes);

    let root_html = concrete_routes
        .get("/")
        .map(|route| render_route_document(&base_html, catalog, route))
        .transpose()?
        .unwrap_or_else(|| base_html.clone());
    fs::write(&index_path, root_html)?;

    for (path, route) in concrete_routes {
        if path == "/" {
            continue;
        }
        let output_path = route_entry_path(interface_path, path)?;
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(
            output_path,
            render_route_document(&base_html, catalog, route)?,
        )?;
    }
    Ok(())
}

fn preferred_concrete_routes<'a>(
    routes: &'a [WebRouteCatalogEntry],
) -> BTreeMap<&'a str, &'a WebRouteCatalogEntry> {
    let mut concrete: BTreeMap<&str, &WebRouteCatalogEntry> = BTreeMap::new();
    for route in routes {
        let Some(path) = route.concrete_path.as_deref() else {
            continue;
        };
        match concrete.get(path) {
            Some(existing) if existing.depth > route.depth => {}
            Some(existing)
                if existing.depth == route.depth
                    && (!existing.is_default || route.is_default)
                    && existing.order < route.order => {}
            Some(existing)
                if existing.depth == route.depth && !existing.is_default && route.is_default => {}
            _ => {
                concrete.insert(path, route);
            }
        }
    }
    concrete
}

fn route_entry_path(interface_path: &Path, route_path: &str) -> Result<PathBuf, eyre::Report> {
    let mut output = interface_path.to_path_buf();
    for segment in route_path.trim_matches('/').split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(eyre!(
                "Concrete route `{route_path}` cannot be materialized as a web entry"
            ));
        }
        output.push(segment);
    }
    Ok(output.join("index.html"))
}

fn render_route_document(
    base_html: &str,
    catalog: &WebRouteCatalog,
    route: &WebRouteCatalogEntry,
) -> Result<String, eyre::Report> {
    let canonical_url = absolute_site_url(&catalog.site, route.concrete_path.as_deref());
    let social_image = route
        .metadata
        .social_image
        .as_deref()
        .or(catalog.site.social_image.as_deref())
        .and_then(|value| absolute_resource_url(&catalog.site, value));
    let social_image_alt = route
        .metadata
        .social_image_alt
        .as_deref()
        .or(catalog.site.social_image_alt.as_deref());

    let mut tags = vec![
        format!(
            "        <meta name=\"description\" content=\"{}\">",
            escape_html_attr(&route.metadata.description)
        ),
        format!(
            "        <meta name=\"robots\" content=\"{}\">",
            if route.metadata.index {
                "index,follow"
            } else {
                "noindex,follow"
            }
        ),
        "        <meta property=\"og:type\" content=\"website\">".to_string(),
        format!(
            "        <meta property=\"og:site_name\" content=\"{}\">",
            escape_html_attr(&catalog.site.site_name)
        ),
        format!(
            "        <meta property=\"og:title\" content=\"{}\">",
            escape_html_attr(&route.metadata.title)
        ),
        format!(
            "        <meta property=\"og:description\" content=\"{}\">",
            escape_html_attr(&route.metadata.description)
        ),
        format!(
            "        <meta name=\"twitter:card\" content=\"{}\">",
            if social_image.is_some() {
                "summary_large_image"
            } else {
                "summary"
            }
        ),
        format!(
            "        <meta name=\"twitter:title\" content=\"{}\">",
            escape_html_attr(&route.metadata.title)
        ),
        format!(
            "        <meta name=\"twitter:description\" content=\"{}\">",
            escape_html_attr(&route.metadata.description)
        ),
    ];
    if let Some(url) = canonical_url {
        tags.push(format!(
            "        <link rel=\"canonical\" href=\"{}\">",
            escape_html_attr(&url)
        ));
        tags.push(format!(
            "        <meta property=\"og:url\" content=\"{}\">",
            escape_html_attr(&url)
        ));
    }
    if let Some(image) = social_image {
        tags.push(format!(
            "        <meta property=\"og:image\" content=\"{}\">",
            escape_html_attr(&image)
        ));
        tags.push(format!(
            "        <meta name=\"twitter:image\" content=\"{}\">",
            escape_html_attr(&image)
        ));
        if let Some(alt) = social_image_alt {
            tags.push(format!(
                "        <meta property=\"og:image:alt\" content=\"{}\">",
                escape_html_attr(alt)
            ));
            tags.push(format!(
                "        <meta name=\"twitter:image:alt\" content=\"{}\">",
                escape_html_attr(alt)
            ));
        }
    }

    let html = set_or_insert_html_title(base_html, &route.metadata.title);
    Ok(set_route_metadata_tags(&html, &tags.join("\n")))
}

fn absolute_site_url(site: &WebSiteMetadata, route_path: Option<&str>) -> Option<String> {
    let site_url = site.site_url.as_deref()?;
    let route_path = route_path?;
    let mut base = Url::parse(&format!("{}/", site_url.trim_end_matches('/'))).ok()?;
    let joined_path = if route_path == "/" {
        String::new()
    } else {
        route_path.trim_start_matches('/').to_string()
    };
    base = base.join(&joined_path).ok()?;
    Some(base.to_string().trim_end_matches('/').to_string())
}

fn absolute_resource_url(site: &WebSiteMetadata, resource: &str) -> Option<String> {
    if let Ok(url) = Url::parse(resource) {
        if matches!(url.scheme(), "http" | "https") {
            return Some(url.to_string());
        }
    }
    let site_url = site.site_url.as_deref()?;
    let base = Url::parse(&format!("{}/", site_url.trim_end_matches('/'))).ok()?;
    base.join(resource.trim_start_matches('/'))
        .ok()
        .map(|url| url.to_string())
}

fn set_or_insert_base_href(html: &str, href: &str) -> String {
    let without_base = html
        .lines()
        .filter(|line| !line.to_ascii_lowercase().contains("<base "))
        .collect::<Vec<_>>()
        .join("\n");
    let base = format!("        <base href=\"{}\">\n", escape_html_attr(href));
    // Establish the base before relative resources and the embedded-host bootstrap.
    // That bootstrap can then update this element instead of creating a second one.
    if let Some(head) = without_base.to_ascii_lowercase().find("<head>") {
        let after_head = head + "<head>".len();
        format!(
            "{}\n{}{}",
            &without_base[..after_head],
            base,
            &without_base[after_head..]
        )
    } else {
        insert_before_head_end(&without_base, &base)
    }
}

fn set_or_insert_html_title(html: &str, title: &str) -> String {
    let escaped_title = escape_html_text(title);
    let lower = html.to_ascii_lowercase();
    if let Some(start) = lower.find("<title>") {
        if let Some(relative_end) = lower[start..].find("</title>") {
            let content_start = start + "<title>".len();
            let end = start + relative_end;
            return format!(
                "{}{}{}",
                &html[..content_start],
                escaped_title,
                &html[end..]
            );
        }
    }
    insert_before_head_end(html, &format!("        <title>{escaped_title}</title>\n"))
}

fn set_route_metadata_tags(html: &str, tags: &str) -> String {
    let without_existing = if let (Some(start), Some(end)) = (
        html.find(ROUTE_METADATA_START),
        html.find(ROUTE_METADATA_END),
    ) {
        let end = end + ROUTE_METADATA_END.len();
        format!("{}{}", &html[..start], &html[end..])
    } else {
        html.to_string()
    };
    let block = format!("        {ROUTE_METADATA_START}\n{tags}\n        {ROUTE_METADATA_END}\n");
    insert_before_head_end(&without_existing, &block)
}

fn insert_before_head_end(html: &str, insertion: &str) -> String {
    if let Some(head_end) = html.to_ascii_lowercase().find("</head>") {
        format!("{}{}{}", &html[..head_end], insertion, &html[head_end..])
    } else {
        format!("{insertion}{html}")
    }
}

fn escape_html_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_html_attr(value: &str) -> String {
    escape_html_text(value)
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use pax_manifest::{
        ComponentDefinition, ComponentTemplate, ControlFlowRouteBranchDefinition,
        ControlFlowSettingsDefinition, TemplateNodeDefinition,
    };
    use std::collections::{BTreeMap, HashMap};
    use tempfile::tempdir;

    fn metadata(title: &str, index: bool) -> RouteMetadataDefinition {
        RouteMetadataDefinition {
            title: title.to_string(),
            description: format!("{title} description"),
            index,
            social_image: None,
            social_image_alt: None,
        }
    }

    fn manifest_with_routes(branches: Vec<ControlFlowRouteBranchDefinition>) -> PaxManifest {
        let main = TypeId::build_singleton("crate::App", Some("App"));
        let mut template = ComponentTemplate::new(main.clone(), Some("src/lib.pax".to_string()));
        template.add_root_node_back(TemplateNodeDefinition {
            type_id: TypeId::build_router(),
            control_flow_settings: Some(ControlFlowSettingsDefinition {
                route_branches: branches,
                ..Default::default()
            }),
            ..Default::default()
        });
        let component = ComponentDefinition {
            type_id: main.clone(),
            is_main_component: true,
            is_primitive: false,
            is_struct_only_component: false,
            module_path: "crate".to_string(),
            primitive_instance_import_path: None,
            template: Some(template),
            settings: None,
            timelines: vec![],
            route_branch: None,
        };
        PaxManifest {
            components: BTreeMap::from([(main.clone(), component)]),
            main_component_type_id: main,
            type_table: HashMap::new(),
            assets_dirs: vec![],
            engine_import_path: "pax_kit::pax_engine".to_string(),
        }
    }

    fn manifest_with_template(main: TypeId, template: ComponentTemplate) -> PaxManifest {
        let component = ComponentDefinition {
            type_id: main.clone(),
            is_main_component: true,
            is_primitive: false,
            is_struct_only_component: false,
            module_path: "crate".to_string(),
            primitive_instance_import_path: None,
            template: Some(template),
            settings: None,
            timelines: vec![],
            route_branch: None,
        };
        PaxManifest {
            components: BTreeMap::from([(main.clone(), component)]),
            main_component_type_id: main,
            type_table: HashMap::new(),
            assets_dirs: vec![],
            engine_import_path: "pax_kit::pax_engine".to_string(),
        }
    }

    fn test_site() -> WebSiteMetadata {
        WebSiteMetadata {
            title: "Example".to_string(),
            site_name: "Example".to_string(),
            site_url: Some("https://example.com".to_string()),
            base_path: "/".to_string(),
            social_image: None,
            social_image_alt: None,
        }
    }

    #[test]
    fn literal_and_catch_all_prefixes_become_concrete_entries() {
        let manifest = manifest_with_routes(vec![
            ControlFlowRouteBranchDefinition {
                path: Some("/".to_string()),
                default: false,
                modal: false,
                child_ids: vec![],
                metadata: Some(metadata("Home", true)),
            },
            ControlFlowRouteBranchDefinition {
                path: Some("/blog/*".to_string()),
                default: false,
                modal: false,
                child_ids: vec![],
                metadata: Some(metadata("Blog", true)),
            },
            ControlFlowRouteBranchDefinition {
                path: None,
                default: true,
                modal: false,
                child_ids: vec![],
                metadata: Some(metadata("Missing", false)),
            },
        ]);

        let catalog = collect_web_route_catalog(&manifest, test_site()).unwrap();
        assert_eq!(catalog.routes[0].concrete_path.as_deref(), Some("/"));
        assert_eq!(catalog.routes[1].concrete_path.as_deref(), Some("/blog"));
        assert_eq!(catalog.routes[2].concrete_path, None);
        assert_eq!(catalog.routes[2].pattern, "/*");
        assert!(catalog.routes[2].is_default);
    }

    #[test]
    fn parameterized_routes_remain_symbolic() {
        let manifest = manifest_with_routes(vec![ControlFlowRouteBranchDefinition {
            path: Some("/docs/:slug".to_string()),
            default: false,
            modal: false,
            child_ids: vec![],
            metadata: Some(metadata("Docs", true)),
        }]);

        let catalog = collect_web_route_catalog(&manifest, test_site()).unwrap();
        assert_eq!(catalog.routes[0].pattern, "/docs/:slug");
        assert_eq!(catalog.routes[0].concrete_path, None);
    }

    #[test]
    fn explicit_route_metadata_wins_over_an_earlier_default_at_the_same_depth() {
        let default = WebRouteCatalogEntry {
            pattern: "/docs/*".to_string(),
            concrete_path: Some("/docs".to_string()),
            depth: 2,
            order: 0,
            is_default: true,
            metadata: metadata("Missing docs", false),
        };
        let explicit = WebRouteCatalogEntry {
            pattern: "/docs".to_string(),
            concrete_path: Some("/docs".to_string()),
            depth: 2,
            order: 1,
            is_default: false,
            metadata: metadata("Docs", true),
        };

        validate_concrete_routes(&[default.clone(), explicit.clone()]).unwrap();
        let routes = [default, explicit];
        let preferred = preferred_concrete_routes(&routes);
        assert_eq!(preferred["/docs"].metadata.title, "Docs");
    }

    #[test]
    fn indexable_routes_behind_dynamic_topology_are_rejected() {
        let main = TypeId::build_singleton("crate::App", Some("App"));
        let mut template = ComponentTemplate::new(main.clone(), Some("src/lib.pax".to_string()));
        let conditional = template
            .add_root_node_back(TemplateNodeDefinition {
                type_id: TypeId::build_if(),
                ..Default::default()
            })
            .get_template_node_id();
        template.add_child_back(
            conditional,
            TemplateNodeDefinition {
                type_id: TypeId::build_router(),
                control_flow_settings: Some(ControlFlowSettingsDefinition {
                    route_branches: vec![ControlFlowRouteBranchDefinition {
                        path: Some("/conditional".to_string()),
                        default: false,
                        modal: false,
                        child_ids: vec![],
                        metadata: Some(metadata("Conditional", true)),
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            },
        );

        let manifest = manifest_with_template(main, template);
        let error = collect_web_route_catalog(&manifest, test_site())
            .expect_err("dynamic indexable route should fail");
        assert!(error.to_string().contains("behind dynamic"));
    }

    #[test]
    fn release_indexable_routes_require_a_site_url() {
        let catalog = WebRouteCatalog {
            version: 1,
            site: WebSiteMetadata {
                site_url: None,
                ..test_site()
            },
            routes: vec![WebRouteCatalogEntry {
                pattern: "/".to_string(),
                concrete_path: Some("/".to_string()),
                depth: 1,
                order: 0,
                is_default: false,
                metadata: metadata("Home", true),
            }],
        };

        let error = validate_release_metadata(&catalog, true)
            .expect_err("release indexable route should require site_url");
        assert!(error.to_string().contains("require"));
    }

    #[test]
    fn nested_routes_compose_against_the_parent_catch_all_remainder() {
        let main = TypeId::build_singleton("crate::App", Some("App"));
        let mut template = ComponentTemplate::new(main.clone(), Some("src/lib.pax".to_string()));
        let root_router = template
            .add_root_node_back(TemplateNodeDefinition {
                type_id: TypeId::build_router(),
                control_flow_settings: Some(ControlFlowSettingsDefinition::default()),
                ..Default::default()
            })
            .get_template_node_id();
        let nested_router = template
            .add_child_back(
                root_router.clone(),
                TemplateNodeDefinition {
                    type_id: TypeId::build_router(),
                    control_flow_settings: Some(ControlFlowSettingsDefinition {
                        route_branches: vec![ControlFlowRouteBranchDefinition {
                            path: Some("members/:member_id".to_string()),
                            default: false,
                            modal: false,
                            child_ids: vec![],
                            metadata: Some(metadata("Member", true)),
                        }],
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .get_template_node_id();
        let mut root_node = template.get_node(&root_router).unwrap().clone();
        root_node
            .control_flow_settings
            .as_mut()
            .unwrap()
            .route_branches = vec![ControlFlowRouteBranchDefinition {
            path: Some("/teams/:team_id/*".to_string()),
            default: false,
            modal: false,
            child_ids: vec![nested_router],
            metadata: None,
        }];
        template.set_node(root_router, root_node);

        let manifest = manifest_with_template(main, template);
        let catalog = collect_web_route_catalog(&manifest, test_site()).unwrap();

        assert_eq!(catalog.routes.len(), 1);
        assert_eq!(
            catalog.routes[0].pattern,
            "/teams/:team_id/members/:member_id"
        );
        assert_eq!(catalog.routes[0].depth, 2);
        assert_eq!(catalog.routes[0].concrete_path, None);
    }

    #[test]
    fn nested_metadata_crosses_only_the_declared_route_shell() {
        use pax_manifest::RouteBranchDescriptor;
        let main = TypeId::build_singleton("crate::App", Some("App"));
        let shell = TypeId::build_singleton("crate::PanelRoute", Some("PanelRoute"));
        let mut template = ComponentTemplate::new(main.clone(), None);
        let root = template
            .add_root_node_back(TemplateNodeDefinition {
                type_id: TypeId::build_router(),
                control_flow_settings: Some(ControlFlowSettingsDefinition::default()),
                ..Default::default()
            })
            .get_template_node_id();
        let shell_node = template
            .add_child_back(
                root.clone(),
                TemplateNodeDefinition {
                    type_id: shell.clone(),
                    ..Default::default()
                },
            )
            .get_template_node_id();
        let nested = template
            .add_child_back(
                shell_node.clone(),
                TemplateNodeDefinition {
                    type_id: TypeId::build_router(),
                    control_flow_settings: Some(ControlFlowSettingsDefinition {
                        route_branches: vec![ControlFlowRouteBranchDefinition {
                            path: Some("about".into()),
                            default: false,
                            modal: false,
                            child_ids: vec![],
                            metadata: None,
                        }],
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .get_template_node_id();
        let mut root_node = template.get_node(&root).unwrap().clone();
        root_node
            .control_flow_settings
            .as_mut()
            .unwrap()
            .route_branches = vec![ControlFlowRouteBranchDefinition {
            path: Some("/notes/*".into()),
            default: false,
            modal: false,
            child_ids: vec![shell_node.clone()],
            metadata: Some(metadata("Notes", true)),
        }];
        template.set_node(root, root_node);
        let mut manifest = manifest_with_template(main.clone(), template);
        let mut shell_component = manifest.components[&main].clone();
        shell_component.type_id = shell.clone();
        shell_component.is_main_component = false;
        shell_component.template = Some(ComponentTemplate::new(shell.clone(), None));
        shell_component.route_branch = Some(RouteBranchDescriptor {
            path_property: "pattern".into(),
            default_property: "fallback".into(),
            modal: false,
        });
        manifest.components.insert(shell.clone(), shell_component);
        let catalog = collect_web_route_catalog(&manifest, test_site()).unwrap();
        assert_eq!(catalog.routes.len(), 2);
        assert_eq!(
            catalog.routes[1].concrete_path.as_deref(),
            Some("/notes/about")
        );
        assert_eq!(catalog.routes[1].metadata.title, "Notes");
        assert_eq!(catalog.routes[1].depth, 2);

        // Ordinary component input still has unknown slot topology.
        manifest.components.get_mut(&shell).unwrap().route_branch = None;
        assert!(collect_web_route_catalog(&manifest, test_site())
            .unwrap_err()
            .to_string()
            .contains("slot topology"));
        manifest.components.get_mut(&shell).unwrap().route_branch =
            Some(RouteBranchDescriptor::default());
        let template = manifest
            .components
            .get_mut(&main)
            .unwrap()
            .template
            .as_mut()
            .unwrap();
        let mut dynamic = template.get_node(&nested).unwrap().clone();
        dynamic.type_id = TypeId::build_if();
        // Wrap a fresh nested router in an actual conditional instead of its static site.
        let router = template.get_node(&nested).unwrap().clone();
        dynamic.control_flow_settings = Some(ControlFlowSettingsDefinition::default());
        template.set_node(nested.clone(), dynamic);
        template.add_child_back(nested, router);
        assert!(collect_web_route_catalog(&manifest, test_site())
            .unwrap_err()
            .to_string()
            .contains("dynamic `if`"));
    }

    #[test]
    fn route_metadata_is_removed_from_runtime_program_ir() {
        let manifest = manifest_with_routes(vec![ControlFlowRouteBranchDefinition {
            path: Some("/".to_string()),
            default: false,
            modal: false,
            child_ids: vec![],
            metadata: Some(metadata("Home", true)),
        }]);

        let ir = pax_manifest::program_ir::ProgramIR::from_manifest(&manifest);
        let component = ir.components.get(&ir.main_component_type_id).unwrap();
        let template = component.template.as_ref().unwrap();
        let router_id = template.get_root().remove(0);
        let route = &template
            .get_node(&router_id)
            .unwrap()
            .control_flow_settings
            .as_ref()
            .unwrap()
            .route_branches[0];

        assert!(route.metadata.is_none());
    }

    #[test]
    fn generated_base_precedes_the_embedded_bootstrap() {
        let html = include_str!("../files/interfaces/web/public/index.html");
        let generated = set_or_insert_base_href(html, "/");
        assert!(generated.find("<base href=").unwrap() < generated.find("<script>").unwrap());
        assert_eq!(generated.matches("<base ").count(), 1);
        let regenerated = set_or_insert_base_href(&generated, "/prefix/");
        assert_eq!(regenerated.matches("<base ").count(), 1);
        assert!(regenerated.contains("<base href=\"/prefix/\">"));
    }

    #[test]
    fn emits_root_and_nested_entry_documents_without_a_404_document() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("index.html"),
            "<html><head><title>Base</title><script type=\"application/json\" id=\"pax-web-config\">{\"server_owned_prefixes\":[\"/downloads\"]}</script></head><body>app</body></html>",
        )
        .unwrap();
        let catalog = WebRouteCatalog {
            version: 1,
            site: test_site(),
            routes: vec![
                WebRouteCatalogEntry {
                    pattern: "/".to_string(),
                    concrete_path: Some("/".to_string()),
                    depth: 1,
                    order: 0,
                    is_default: false,
                    metadata: metadata("Home", true),
                },
                WebRouteCatalogEntry {
                    pattern: "/blog/*".to_string(),
                    concrete_path: Some("/blog".to_string()),
                    depth: 1,
                    order: 1,
                    is_default: false,
                    metadata: metadata("Blog", true),
                },
            ],
        };

        emit_route_entry_documents(dir.path(), &catalog).unwrap();

        let root = fs::read_to_string(dir.path().join("index.html")).unwrap();
        let blog = fs::read_to_string(dir.path().join("blog/index.html")).unwrap();
        assert!(root.contains("<title>Home</title>"));
        assert!(blog.contains("<title>Blog</title>"));
        assert!(blog.contains("https://example.com/blog"));
        assert!(blog.contains("<base href=\"/\">"));
        for document in [&root, &blog] {
            assert_eq!(document.matches("id=\"pax-web-config\"").count(), 1);
            assert!(document.contains(r#"{"server_owned_prefixes":["/downloads"]}"#));
        }
        assert!(!dir.path().join("404.html").exists());
    }
}
