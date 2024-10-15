use super::SelectionState;
use crate::math::coordinate_spaces::Glass;
use pax_engine::{
    api::Window, math::Transform2, pax_manifest::UniqueTemplateNodeIdentifier, NodeInterface,
    Property,
};

// This represents values that can be deterministically produced from the app
// state and the projects manifest
pub struct DerivedAppState {
    pub to_glass_transform: Property<Property<Transform2<Window, Glass>>>,
    pub selected_nodes: Property<Vec<(UniqueTemplateNodeIdentifier, NodeInterface)>>,
    pub selection_state: Property<SelectionState>,
    /// The currently open containers, example: the parent group of the rectangle currently selected, and the scroller this group is inside
    pub open_containers: Property<Vec<UniqueTemplateNodeIdentifier>>,
}
