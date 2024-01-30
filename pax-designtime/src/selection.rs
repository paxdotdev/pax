use uuid::Uuid;

pub struct SelectionManager {
    selection_state: SelectionState,
    last_mutation_id: Uuid,
}

impl Default for SelectionManager {
    fn default() -> Self {
        Self::new()
    }
}

//Current, SelectionState is a Vec of TemplateNodeDefinition IDs, which handles the single-select and multi-select cases
//TODO: consider alternative API making three cases explicit: None, Single(tnd_id), Multi(Vec<tnd_ids>), Direct(Vec<control_point_id>)
pub type SelectionState = Vec<usize>;

impl SelectionManager {
    pub fn new() -> Self {
        SelectionManager {
            selection_state: SelectionState::default(),
            last_mutation_id: Uuid::new_v4(),
        }
    }
}
