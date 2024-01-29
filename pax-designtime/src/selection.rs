
pub struct SelectionManager {
    selection_state: SelectionState,

    //TODO: subscription is tricky due to move closures + ownership.  Either figure out an approach to fine-grained moves
    // (e.g. of cloneable property channels, or cloneable property Rc<RefCell<>>s), or make this a brute-force, per-tick check instead of a reactive subscription mechanism.
    // One hybrid approach could be to keep a fingerprint, e.g. a UUID per change.  A "subscriber" can determine that selection state has changed
    // by checking that fingerprint each tick.  The subscriber must update IFF that UUID is different than the subscriber's locally stored copy.
    selection_subscribers: Vec<Box<dyn FnMut(&SelectionState)>>,
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
    fn new() -> Self {
        SelectionManager {
            selection_subscribers: Vec::new(),
            selection_state: SelectionState::default(),
        }
    }



    //in glass
    pub fn handle_pre_tick(&mut self, dt: Rc<RefCell<DesignTimeManager>>) {
        self.
    }



    fn subscribe<F>(&mut self, callback: F)
        where F: FnMut(&SelectionState) + 'static {
        self.selection_subscribers.push(Box::new(callback));
    }

    fn notify_subscribers(&mut self) {
        let state = self.selection_state.clone();
        for subscriber in &mut self.selection_subscribers {
            subscriber(&state);
        }
    }
}
