//! Mounted observers for qualified pointer activations.
use crate::constants::CLICK_OUTSIDE_HANDLERS;
use crate::{ExpandedNode, ExpandedNodeIdentifier, RuntimeContext};
use pax_runtime_api::{Click, Event};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Default)]
pub(crate) struct OutsideClicks {
    listeners: HashMap<ExpandedNodeIdentifier, u64>,
    generation: u64,
}

impl OutsideClicks {
    pub(crate) fn register(&mut self, node: &ExpandedNode) {
        if !node.is_render_source()
            && node.has_event_handlers(CLICK_OUTSIDE_HANDLERS)
            && !self.listeners.contains_key(&node.id)
            && !node.retained_for_input_exit()
        {
            self.generation += 1;
            self.listeners.insert(node.id, self.generation);
        }
    }

    pub(crate) fn is_current(&self, id: ExpandedNodeIdentifier, generation: u64) -> bool {
        self.listeners.get(&id) == Some(&generation)
    }

    pub(crate) fn remove(&mut self, id: ExpandedNodeIdentifier) {
        self.listeners.remove(&id);
    }

    pub(crate) fn retire_subtree(&mut self, node: &ExpandedNode) {
        if self.listeners.is_empty() {
            return;
        }
        self.remove(node.id);
        for child in node.mounted_children.borrow().iter() {
            self.retire_subtree(child);
        }
    }

    pub(crate) fn snapshot(
        &self,
        target: &ExpandedNode,
    ) -> Option<HashMap<ExpandedNodeIdentifier, u64>> {
        self.snapshot_excluding(|| {
            std::iter::once(target.id).chain(
                std::iter::successors(target.template_parent.upgrade(), |p| {
                    p.template_parent.upgrade()
                })
                .map(|p| p.id),
            )
        })
    }

    pub(crate) fn snapshot_excluding<I: IntoIterator<Item = ExpandedNodeIdentifier>>(
        &self,
        route: impl FnOnce() -> I,
    ) -> Option<HashMap<ExpandedNodeIdentifier, u64>> {
        if self.listeners.is_empty() {
            return None;
        }
        let mut recipients = self.listeners.clone();
        // Follow the logical route even when a handler stops bubbling or an
        // Overlay places the target outside its parent's screen bounds.
        for id in route() {
            recipients.remove(&id);
        }
        Some(recipients)
    }
}

impl RuntimeContext {
    pub(crate) fn notify_click_outside(
        self: &Rc<Self>,
        recipients: HashMap<ExpandedNodeIdentifier, u64>,
        click: &Click,
    ) {
        for (id, generation) in recipients {
            // A prior callback may have unmounted, remounted, or rebound this ID.
            if self.outside_clicks.borrow().is_current(id, generation) {
                if let Some(node) = self.get_expanded_node_by_eid(id) {
                    node.run_event_handlers_for_key(
                        CLICK_OUTSIDE_HANDLERS,
                        &Event::new(click.clone()),
                        self,
                    );
                }
            }
        }
    }
}
