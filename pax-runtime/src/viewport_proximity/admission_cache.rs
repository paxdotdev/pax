//! Request-time memoization of the two inputs to mount admission: the inherited
//! viewport frame and the owner's footprint within it. A change to a frame's
//! inputs propagates only if its effective output for children changes.
use super::{Priority, RuntimeContext, ViewportFrame};
use crate::ExpandedNode;
use kurbo::Rect;
use pax_runtime_api::properties::{
    DependencyEventQueue, DependencyEventSubscription, PropertyReadScope,
};
use std::collections::HashMap;

/// Cumulative work performed while selecting background mounts in the release experiment.
#[derive(Default, Clone, Copy, serde::Serialize)]
pub struct RankStats {
    pub choices: u32,
    pub checked_choices: u32,
    pub rank_requests: u32,
    pub rank_calculations: u32,
    pub frame_calculations: u32,
    pub notifications: u32,
    pub reference_ranks: u32,
    pub first_choices: u32,
    pub first_choice_ms: f64,
    pub later_choice_ms: f64,
    pub peak_owners: usize,
    pub overflows: u32,
}

#[derive(Default)]
pub(super) struct RetainedRanks {
    changes: DependencyEventQueue,
    owners: HashMap<u32, Owner>,
    viewport: Option<(Rect, Option<u32>)>,
    revision: u64,
    viewport_revision: u64,
    structure_revision: u64,
}

pub(crate) struct Watched<T> {
    pub(crate) value: T,
    subscription: Option<DependencyEventSubscription>,
    dirty: bool,
}

impl<T> Watched<T> {
    pub(crate) fn observe(
        previous: Option<&mut Self>,
        calc: impl FnOnce() -> T,
        changes: &DependencyEventQueue,
        key: u64,
        stats: &mut RankStats,
    ) -> Self {
        let reads = PropertyReadScope::enter(1024);
        let value = calc();
        let deps = reads.finish();
        if deps.is_err() {
            stats.overflows += 1;
        }
        let subscription = deps.ok().map(|deps| {
            if let Some(mut existing) = previous.and_then(|prior| prior.subscription.take()) {
                existing.refresh(&deps);
                existing
            } else {
                changes.watch(&deps, key)
            }
        });
        Self {
            value,
            subscription,
            dirty: false,
        }
    }

    pub(crate) fn invalidate(&mut self) {
        self.dirty = true;
    }
    pub(crate) fn reusable(&self) -> bool {
        self.subscription.is_some() && !self.dirty
    }
}

#[derive(Default)]
struct Owner {
    epoch: u64,
    parent: Option<(u32, u64)>,
    viewport_revision: u64,
    frame: Option<Watched<ViewportFrame>>,
    frame_version: u64,
    frame_checked: u64,
    rank: Option<Watched<Priority>>,
    rank_checked: u64,
    rank_region: Option<(Option<Rect>, Option<Rect>)>,
}

impl RetainedRanks {
    // No node or property handle is retained. Cached ranks leave with this
    // mounting session, and teardown removes each individual lifetime.
    pub(super) fn begin_choice(
        &mut self,
        ctx: &RuntimeContext,
        viewport: Rect,
        stats: &mut RankStats,
    ) {
        let page_root = ctx
            .get_root_scroller_id()
            .filter(|_| ctx.get_visual_viewport_state().is_some());
        let footprint = (viewport, page_root);
        let viewport_changed = self.viewport != Some(footprint);
        if viewport_changed {
            self.viewport = Some(footprint);
            self.viewport_revision += 1;
        }
        let mut signals = ctx.admission_changes.borrow_mut();
        let mut changed = self.structure_revision != signals.revision || viewport_changed;
        self.structure_revision = signals.revision;
        for id in signals.forget.drain(..) {
            changed |= self.owners.remove(&id).is_some();
        }
        drop(signals);
        while let Some(key) = self.changes.pop() {
            if let Some(owner) = self.owners.get_mut(&((key >> 1) as u32)) {
                if key & 1 == 0 {
                    if let Some(frame) = &mut owner.frame {
                        frame.dirty = true;
                    }
                } else if let Some(rank) = &mut owner.rank {
                    rank.dirty = true;
                }
            }
            changed = true;
            stats.notifications += 1;
        }
        if changed || self.revision == 0 || stats.overflows != 0 {
            self.revision += 1;
        }
    }

    fn frame(
        &mut self,
        ctx: &RuntimeContext,
        node: &ExpandedNode,
        stats: &mut RankStats,
    ) -> (ViewportFrame, u64) {
        let id = node.id.to_u32();
        let epoch = node.mount_epoch.get();
        if let Some(owner) = self.owners.get(&id) {
            if owner.epoch == epoch && owner.frame_checked == self.revision {
                if let Some(frame) = &owner.frame {
                    return (frame.value, owner.frame_version);
                }
            }
        }
        // Read actual presented ancestry even when this owner has never had a
        // viewport-proximity handler. Slots and move flights can replace it.
        let parent = node.presentation_parent_node().map(|p| {
            let (frame, version) = self.frame(ctx, &p, stats);
            ((p.id.to_u32(), version), frame)
        });
        let parent_key = parent.map(|(key, _)| key);
        let viewport = self.viewport.expect("active admission viewport");
        let owner = self.owners.entry(id).or_default();
        if owner.epoch != epoch {
            *owner = Owner {
                epoch,
                ..Owner::default()
            };
        }
        let usable = owner.parent == parent_key
            && owner.viewport_revision == self.viewport_revision
            && owner.frame.as_ref().is_some_and(Watched::reusable);
        if !usable {
            let next = Watched::observe(
                owner.frame.as_mut(),
                || {
                    let instance = node.instance_node.borrow();
                    let scrolls = instance.scrolls_content(node);
                    let escapes = node
                        .get_common_properties()
                        .borrow()
                        .unclippable
                        .get()
                        .unwrap_or(false);
                    let clip = instance
                        .may_supply_clip()
                        .then(|| instance.resolve_effect_clip_path(node))
                        .flatten();
                    ctx.compose_viewport_frame(
                        node,
                        parent.map(|(_, f)| f),
                        viewport.0,
                        scrolls,
                        escapes,
                        viewport.1 == Some(id),
                        clip.as_ref(),
                        || {
                            let tab = node.presentation_transform_and_bounds.get();
                            (kurbo::Affine::from(tab.transform), tab.bounds)
                        },
                    )
                },
                &self.changes,
                u64::from(id) << 1,
                stats,
            );
            stats.frame_calculations += 1;
            if owner
                .frame
                .as_ref()
                .is_none_or(|old| old.value != next.value)
            {
                // The revision is common across nodes. Include the parent id
                // in the child's key so equal revisions cannot alias a move.
                owner.frame_version = self.revision;
            }
            owner.parent = parent_key;
            owner.viewport_revision = self.viewport_revision;
            owner.frame = Some(next);
        }
        let frame = owner.frame.as_ref().unwrap();
        owner.frame_checked = self.revision;
        (frame.value, owner.frame_version)
    }

    pub(super) fn priority(
        &mut self,
        ctx: &RuntimeContext,
        node: &ExpandedNode,
        stats: &mut RankStats,
    ) -> Priority {
        let id = node.id.to_u32();
        if let Some(owner) = self.owners.get(&id) {
            if owner.epoch == node.mount_epoch.get() && owner.rank_checked == self.revision {
                if let Some(rank) = &owner.rank {
                    return rank.value;
                }
            }
        }
        let (frame, _) = self.frame(ctx, node, stats);
        let owner = self.owners.get_mut(&id).unwrap();
        let region = (frame.child_near, frame.child_visible);
        if owner.rank_region != Some(region) || !owner.rank.as_ref().is_some_and(Watched::reusable)
        {
            owner.rank = Some(Watched::observe(
                owner.rank.as_mut(),
                || ctx.priority_in_frame(node, frame),
                &self.changes,
                (u64::from(id) << 1) | 1,
                stats,
            ));
            owner.rank_region = Some(region);
            stats.rank_calculations += 1;
        }
        owner.rank_checked = self.revision;
        let rank = owner.rank.as_ref().unwrap().value;
        stats.peak_owners = stats.peak_owners.max(self.owners.len());
        rank
    }
}

#[derive(Default)]
pub(crate) struct AdmissionChanges {
    pub(crate) enabled: bool,
    pub(crate) revision: u64,
    forget: Vec<u32>,
}
impl RuntimeContext {
    pub(crate) fn admission_ancestry_changed(&self) {
        let mut c = self.admission_changes.borrow_mut();
        if c.enabled {
            c.revision += 1;
        }
    }
    pub(crate) fn forget_admission_owner(&self, id: u32) {
        let mut c = self.admission_changes.borrow_mut();
        if c.enabled {
            c.forget.push(id);
        }
    }
}
