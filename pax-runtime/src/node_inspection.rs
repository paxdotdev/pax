//! On-demand node attribution shared by debug tools and release performance probes.

use crate::{ExpandedNodeIdentifier, RuntimeContext};
use serde::Serialize;

/// Identity and lifecycle of a currently cached expanded node. No tree traversal or
/// geometry preparation is performed; IDs are meaningful only in this engine instance.
#[derive(Debug, Serialize)]
pub struct NodeInspection {
    pub engine_id: u32,
    pub node_type: Option<String>,
    pub component: Option<String>,
    pub template_node_id: Option<usize>,
    /// One-based template line/column, when the parser supplied a source span.
    /// Release programs retain spans but deliberately omit source file paths.
    pub source_start: Option<(usize, usize)>,
    pub source_end: Option<(usize, usize)>,
    pub template_parent: Option<u32>,
    pub render_parent: Option<u32>,
    pub containing_component: Option<u32>,
    pub render_source_owner: Option<u32>,
    pub attached: u32,
    pub has_occlusion_record: bool,
}

impl RuntimeContext {
    /// Resolve a profiling ID without evaluating reactive properties. Returns
    /// `None` after the node leaves the runtime cache, or for an unknown ID.
    pub fn inspect_node(&self, id: u32) -> Option<NodeInspection> {
        let node = self.get_expanded_node_by_eid(ExpandedNodeIdentifier(id))?;
        let instance = node.instance_node.borrow();
        let base = instance.base();
        let template = base.template_node_identifier.as_ref();
        let location = base
            .template_node_selector_info
            .as_ref()
            .and_then(|info| info.source_location.as_ref());
        let inspection = NodeInspection {
            engine_id: id,
            node_type: base.template_node_type_id.as_ref().map(ToString::to_string),
            component: template.map(|id| id.get_containing_component_type_id().to_string()),
            template_node_id: template.map(|id| id.get_template_node_id().as_usize()),
            source_start: location.map(|location| one_based(location.start_line_col)),
            source_end: location.map(|location| one_based(location.end_line_col)),
            template_parent: node.template_parent.upgrade().map(|node| node.id.to_u32()),
            render_parent: node
                .render_parent
                .borrow()
                .upgrade()
                .map(|node| node.id.to_u32()),
            containing_component: node
                .containing_component
                .upgrade()
                .map(|node| node.id.to_u32()),
            render_source_owner: node.render_source_owner().map(|id| id.to_u32()),
            attached: node.attached.get(),
            has_occlusion_record: self.occlusion_state.borrow().contains_node(id),
        };
        Some(inspection)
    }
}

fn one_based((line, column): (usize, usize)) -> (usize, usize) {
    (line + 1, column + 1)
}
