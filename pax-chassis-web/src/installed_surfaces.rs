use pax_message::serde::Deserialize;
use std::{cell::RefCell, collections::HashSet, rc::Rc};

pub(crate) type InstalledSurfaces = Rc<RefCell<InstalledSurfacePlan>>;

// A committed DOM transaction, not a wish list. Pending allocations never
// occupy a GPU initialization slot. Priority revisions are not lease identity.
#[derive(Default, Deserialize)]
#[serde(crate = "pax_message::serde", rename_all = "camelCase")]
pub(crate) struct InstalledSurfacePlan {
    #[serde(skip)]
    pub updating: bool,
    pub version: u32,
    pub revision: u32,
    pub surfaces: Vec<InstalledSurface>,
    pub pending: Vec<String>,
}

#[derive(Clone, Deserialize)]
#[serde(crate = "pax_message::serde", rename_all = "camelCase")]
pub(crate) struct InstalledSurface {
    pub id: String,
    pub layer_id: usize,
    pub generation: String,
    pub key: String,
    pub host_signature: String,
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
    pub backing_width: u32,
    pub backing_height: u32,
    pub density_clamped: bool,
    pub replay_priority: i32,
    pub active: bool,
}

impl InstalledSurfacePlan {
    // Re-read at dispatch and after waits. A priority belongs to the current
    // committed demand, never to a job's enqueue time or an expired deadline.
    pub fn initialization_priority(
        &self,
        layer: Option<usize>,
        available: impl Fn(&InstalledSurface) -> bool,
    ) -> i32 {
        if self.updating {
            return i32::MAX;
        }
        self.surfaces
            .iter()
            .filter(|s| s.active && layer.is_none_or(|layer| s.layer_id == layer))
            .filter(|s| available(s))
            .map(|s| s.replay_priority)
            .min()
            .unwrap_or(i32::MAX)
    }

    pub fn accepts(&self, next: &Self) -> bool {
        let mut identities = HashSet::new();
        next.version == 2
            && next.revision > self.revision
            && next.surfaces.iter().all(|s| {
                !s.id.is_empty()
                    && !s.generation.is_empty()
                    && !s.host_signature.is_empty()
                    && identities.insert(s.id.as_str())
            })
            && next.pending.iter().all(|id| identities.insert(id.as_str()))
            && next
                .pending
                .iter()
                .all(|id| !next.surfaces.iter().any(|surface| surface.id == *id))
            && next.surfaces.iter().all(|s| {
                s.width.is_finite()
                    && s.height.is_finite()
                    && s.width > 0.0
                    && s.height > 0.0
                    && s.backing_width > 0
                    && s.backing_height > 0
                    && s.left.is_finite()
                    && s.top.is_finite()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(revision: u32) -> InstalledSurfacePlan {
        InstalledSurfacePlan {
            version: 2,
            revision,
            surfaces: vec![InstalledSurface {
                id: "1-tile-0".into(),
                layer_id: 1,
                generation: "7".into(),
                key: "tile-0".into(),
                host_signature: "scroller:3".into(),
                left: 0.0,
                top: 0.0,
                width: 100.0,
                height: 100.0,
                backing_width: 200,
                backing_height: 200,
                density_clamped: false,
                replay_priority: -1,
                active: true,
            }],
            ..Default::default()
        }
    }

    #[test]
    fn publication_is_monotonic_and_priority_is_not_identity() {
        let current = plan(3);
        assert!(!current.accepts(&plan(3)));
        let mut next = plan(4);
        next.surfaces[0].replay_priority = 27;
        assert!(current.accepts(&next));
        assert_eq!(next.surfaces[0].generation, current.surfaces[0].generation);
        next.version = 3;
        assert!(!current.accepts(&next));
    }

    #[test]
    fn invalid_or_duplicate_assignments_cannot_be_published() {
        let current = InstalledSurfacePlan::default();
        let mut next = plan(1);
        next.pending.push("1-tile-0".into());
        assert!(!current.accepts(&next));
        next.pending.clear();
        next.surfaces.push(next.surfaces[0].clone());
        assert!(!current.accepts(&next));
        next.surfaces.pop();
        next.surfaces[0].width = f64::NAN;
        assert!(!current.accepts(&next));
    }

    #[test]
    fn pending_allocations_and_empty_teardown_are_explicit() {
        let current = plan(2);
        let mut next = plan(3);
        next.pending.push("2-tile-0".into());
        assert!(current.accepts(&next));
        next.surfaces.clear();
        next.pending.clear();
        assert!(current.accepts(&next));
        // A newly mounted chassis has an independent revision lifetime.
        assert!(InstalledSurfacePlan::default().accepts(&plan(1)));
    }

    #[test]
    fn waiting_initialization_yields_to_current_demand_not_old_priority() {
        let mut current = plan(1);
        let mut next = current.surfaces[0].clone();
        next.id = "2-tile-0".into();
        next.layer_id = 2;
        next.replay_priority = 10;
        current.surfaces.push(next);
        assert_eq!(current.initialization_priority(None, |_| true), -1);
        // Simulate a scroll while a job is waiting for the GPU-context lock.
        current.surfaces[0].replay_priority = 20;
        current.surfaces[1].replay_priority = -2;
        assert!(
            current.initialization_priority(Some(1), |_| true)
                > current.initialization_priority(None, |_| true)
        );
        // Already-owned backings and parked/retired work cannot block new work.
        assert_eq!(
            current.initialization_priority(None, |s| s.layer_id != 2),
            20
        );
        current.surfaces[0].active = false;
        assert_eq!(current.initialization_priority(Some(1), |_| true), i32::MAX);
        current.surfaces.clear();
        assert_eq!(current.initialization_priority(None, |_| true), i32::MAX);
        current = plan(2);
        current.updating = true;
        assert_eq!(current.initialization_priority(None, |_| true), i32::MAX);
    }
}
