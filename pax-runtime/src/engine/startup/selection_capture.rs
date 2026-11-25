//! Read-only, explicitly enabled release replay input. The spatial scheduler never
//! consumes this log. Only checked runs pay for the exhaustive frontier observation.
use super::{admission::AdmissionQueue, PendingMount};
use crate::viewport_proximity::{admission_cache::Watched, RankStats};
use crate::{ExpandedNode, RuntimeContext};
use kurbo::{Rect, Shape};
use pax_runtime_api::properties::DependencyEventQueue;
use serde::Serialize;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use web_time::Instant;

type Life = (u32, u64, u32, u64);
fn life(p: &PendingMount) -> Life {
    (
        p.key.0.to_u32(),
        p.parent_epoch,
        p.key.2.to_u32(),
        p.child_epoch,
    )
}
fn edges(r: Rect) -> [f64; 4] {
    [r.x0, r.y0, r.x1, r.y1]
}

#[derive(Clone, PartialEq, Serialize)]
pub(in crate::engine) struct Footprint {
    pub(in crate::engine) id: u32,
    pub(in crate::engine) epoch: u64,
    pub(in crate::engine) host: Option<u32>,
    pub(in crate::engine) bounds: Option<[f64; 4]>,
    pub(in crate::engine) clip: Option<[f64; 4]>,
}
impl Footprint {
    pub(in crate::engine) fn clipped_bounds(&self) -> Option<Rect> {
        let r = self.bounds.map(crate::engine::selection_policy::rect)?;
        let r = self.clip.map_or(r, |clip| {
            r.intersect(crate::engine::selection_policy::rect(clip))
        });
        (r.is_finite() && r.width() > 0. && r.height() > 0.).then_some(r)
    }
}
#[derive(Clone, PartialEq, Serialize)]
struct Job {
    age: u32,
    priority_age: u32,
    life: Life,
    owner: u32,
    group: Option<u32>,
    kind: String,
    component: String,
}
#[derive(Serialize)]
struct Choice {
    at: f64,
    observation: u32,
    add: Vec<Job>,
    remove: Vec<u32>,
    geometry: Vec<Footprint>,
    signal_geometry: Vec<Footprint>,
    discarded: Option<&'static str>,
    eligible: usize,
    selected: Option<u32>,
}
#[derive(Default, Serialize)]
pub struct Report {
    // Mount receipts can be joined to the granting interval and subsequent
    // publication without paying for exhaustive geometry in a timing run.
    mounts: Vec<(f64, u32, u32, u64, Option<u32>, u32)>,
    signal_reads: u64,
    full_reads: u64,
    dependency_overflows: u32,
    signal_disagreements: u32,
    choices: Vec<Choice>,
    // Actual selections continue after expensive frontier capture ends so an
    // earlier alternative can be followed to its real selection.
    receipts: Vec<(f64, u32)>,
}
struct Capture {
    clock: Instant,
    base: f64,
    maximum: usize,
    previous: BTreeMap<u32, Job>,
    footprints: BTreeMap<u32, Footprint>,
    report: Report,
    recording: bool,
    changes: DependencyEventQueue,
    watches: BTreeMap<u32, Watched<Footprint>>,
    structure_revision: u64,
    watch_stats: RankStats,
}
thread_local! {
    static CURRENT: RefCell<Option<Capture>> = const { RefCell::new(None) };
    static OBSERVATION: Cell<u32> = const { Cell::new(0) };
}
pub fn start(base: f64, maximum: usize) {
    OBSERVATION.set(0);
    CURRENT.with_borrow_mut(|c| {
        *c = Some(Capture {
            clock: Instant::now(),
            base,
            maximum,
            previous: Default::default(),
            footprints: Default::default(),
            report: Default::default(),
            recording: false,
            changes: Default::default(),
            watches: Default::default(),
            structure_revision: 0,
            watch_stats: Default::default(),
        })
    });
}
pub fn observation(serial: u32) {
    OBSERVATION.set(serial);
}
pub fn finish() -> Option<Report> {
    CURRENT.with_borrow_mut(|c| c.take().map(|s| s.report))
}
pub(super) fn mounted(ctx: &RuntimeContext, work: &PendingMount, child: &ExpandedNode) {
    CURRENT.with_borrow_mut(|state| {
        let Some(c) = state else { return };
        let group = ctx
            .visible_groups
            .borrow()
            .capture_anchor(child.id.to_u32())
            .map(|(id, _)| id);
        c.report.mounts.push((
            c.base + c.clock.elapsed().as_secs_f64() * 1000.,
            child.id.to_u32(),
            work.key.0.to_u32(),
            child.mount_epoch.get(),
            group,
            work.arrival,
        ));
    });
}
pub(in crate::engine) fn footprint(ctx: &RuntimeContext, owner: &Rc<ExpandedNode>) -> Footprint {
    // Parent bindings are installed before a deferred attachment. A positioned
    // item can already have a more precise rectangle than its entire scroller.
    // Keep the owner's identity while its provisional geometry uses a parent.
    let basis = if owner.attached.get() == 0 {
        let (w, h) = owner.transform_and_bounds.get().bounds;
        let common = owner.get_common_properties();
        let common = common.borrow();
        let sized = common.width.get().is_some()
            && common.height.get().is_some()
            && w.is_finite()
            && h.is_finite()
            && w > 0.
            && h > 0.;
        (!sized).then(|| owner.presentation_parent_node()).flatten()
    } else {
        None
    };
    let basis = basis.as_ref().unwrap_or(owner);
    let (width, height) = basis.transform_and_bounds.get().bounds;
    let bounds = ctx
        .presented_projection_for_node(basis)
        .and_then(|p| p.coverage(Rect::new(0., 0., width, height), None));
    let mut clip: Option<Rect> = None;
    let mut host = None;
    let mut n = Some(basis.clone());
    while let Some(node) = n {
        let instance = node.instance_node.borrow();
        if node.id != basis.id && instance.scrolls_content(&node) {
            host = Some(node.id.to_u32());
            break;
        }
        if instance.may_supply_clip() {
            if let Some(path) = instance.resolve_effect_clip_path(&node) {
                let b = (ctx.presentation_scroll_transform_for_node(&node) * path).bounding_box();
                clip = Some(clip.map_or(b, |prior| prior.intersect(b)));
            }
        }
        if node
            .get_common_properties()
            .borrow()
            .unclippable
            .get()
            .unwrap_or(false)
        {
            break;
        }
        n = node.presentation_parent_node();
    }
    Footprint {
        id: owner.id.to_u32(),
        epoch: owner.mount_epoch.get(),
        host,
        bounds: bounds.map(edges),
        clip: clip.map(edges),
    }
}
pub(super) fn before(ctx: &RuntimeContext, q: &AdmissionQueue) {
    CURRENT.with_borrow_mut(|state| {
        let Some(s) = state else { return };
        s.recording = s.report.choices.len() < s.maximum && OBSERVATION.get() != 0;
        if !s.recording {
            return;
        }
        let at = s.base + s.clock.elapsed().as_secs_f64() * 1000.;
        let mut jobs = BTreeMap::new();
        let mut anchors = BTreeSet::new();
        let mut geometry = Vec::new();
        let mut signal_geometry = Vec::new();
        while let Some(key) = s.changes.pop() {
            if let Some(w) = s.watches.get_mut(&(key as u32)) {
                w.invalidate();
            }
        }
        let revision = ctx.admission_changes.borrow().revision;
        let ancestry_changed = s.structure_revision != revision;
        s.structure_revision = revision;
        let startup = ctx.startup.borrow();
        let groups = ctx.visible_groups.borrow();
        for work in q
            .all_pending()
            .chain(startup.stack.iter())
            .chain(startup.deferred.iter())
            .chain(startup.incoming.iter())
        {
            let (Some(parent), Some(child)) = (work.parent.upgrade(), work.child.upgrade()) else {
                continue;
            };
            if !work.runnable(&parent, &child) {
                continue;
            }
            let id = work.arrival;
            let (group, anchor, priority_age) = groups.scheduling_anchor(&parent, &child, id);
            let owner = anchor.id.to_u32();
            if anchors.insert(owner) {
                if ancestry_changed
                    || s.watches
                        .get(&owner)
                        .is_none_or(|w| !w.reusable() || w.value.epoch != anchor.mount_epoch.get())
                {
                    let old_value = s.watches.get(&owner).map(|w| w.value.clone());
                    let next = Watched::observe(
                        s.watches.get_mut(&owner),
                        || footprint(ctx, &anchor),
                        &s.changes,
                        u64::from(owner),
                        &mut s.watch_stats,
                    );
                    s.report.signal_reads += 1;
                    if old_value.as_ref() != Some(&next.value) {
                        signal_geometry.push(next.value.clone());
                    }
                    s.watches.insert(owner, next);
                }
                let fp = footprint(ctx, &anchor);
                s.report.full_reads += 1;
                if s.watches[&owner].value != fp {
                    s.report.signal_disagreements += 1;
                }
                s.report.dependency_overflows = s.watch_stats.overflows;
                if s.footprints.get(&owner) != Some(&fp) {
                    geometry.push(fp.clone());
                    s.footprints.insert(owner, fp);
                }
            }
            let job = if let Some(old) = s.previous.get(&id).filter(|old| {
                old.owner == owner && old.group == group && old.priority_age == priority_age
            }) {
                old.clone()
            } else {
                let instance = child.instance_node.borrow();
                let base = instance.base();
                let short = |s: String| s.rsplit("::").next().unwrap_or("").to_owned();
                Job {
                    age: id,
                    priority_age,
                    life: life(work),
                    owner,
                    group,
                    kind: base
                        .template_node_type_id
                        .as_ref()
                        .map(|id| short(id.to_string()))
                        .unwrap_or_default(),
                    component: base
                        .template_node_identifier
                        .as_ref()
                        .map(|id| short(id.get_containing_component_type_id().to_string()))
                        .unwrap_or_default(),
                }
            };
            jobs.insert(id, job);
        }
        s.watches.retain(|id, _| anchors.contains(id));
        let add = jobs
            .iter()
            .filter(|(id, j)| s.previous.get(id) != Some(j))
            .map(|(_, j)| j.clone())
            .collect();
        let remove = s
            .previous
            .keys()
            .filter(|k| !jobs.contains_key(k))
            .copied()
            .collect();
        s.report.choices.push(Choice {
            at,
            observation: OBSERVATION.get(),
            add,
            remove,
            geometry,
            signal_geometry,
            discarded: None,
            eligible: jobs.len(),
            selected: None,
        });
        s.previous = jobs;
    });
}
pub(super) fn after(selected: Option<&PendingMount>) {
    CURRENT.with_borrow_mut(|state| {
        let Some(s) = state else { return };
        if let Some(id) = selected.map(|p| p.arrival) {
            s.report
                .receipts
                .push((s.base + s.clock.elapsed().as_secs_f64() * 1000., id));
            if s.recording {
                let decision = s.report.choices.last_mut().unwrap();
                decision.selected = Some(id);
                if !s.previous.contains_key(&id) {
                    let work = selected.unwrap();
                    decision.discarded =
                        Some(match (work.parent.upgrade(), work.child.upgrade()) {
                            (None, _) => "retired parent",
                            (_, None) => "retired child",
                            (Some(p), Some(c))
                                if p.mount_epoch.get() != work.parent_epoch
                                    || c.mount_epoch.get() != work.child_epoch =>
                            {
                                "retired lifetime"
                            }
                            (_, Some(c)) if c.attached.get() != 0 => "already mounted",
                            (Some(p), Some(c)) if !p.accepts_pending_child(&c) => {
                                "replaced attachment"
                            }
                            _ => "changed presentation parent",
                        });
                }
            }
        }
        s.recording = false;
    });
}
