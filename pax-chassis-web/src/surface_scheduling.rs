//! Batched Web adapter for the shared policy. DOM measurement/installation stay
//! in the browser; activity, coverage ranking, and admission do not.
use pax_message::serde::{Deserialize, Serialize};
use pax_runtime::engine::surface_preparation::{self, PreparationHost, PreparedHost};
use pax_runtime::engine::surface_scheduling::{
    self as shared, Coverage, CoverageRect, HostObservation, ScrollActivityTracker, SurfaceBudget,
    SurfaceCandidate,
};
use std::collections::{BTreeMap, HashSet};

#[derive(Deserialize)]
#[serde(crate = "pax_message::serde", rename_all = "camelCase")]
pub(crate) struct Snapshot {
    pub version: u32,
    pub now: f64,
    pub capacity: usize,
    #[serde(default)]
    pub layers: Vec<usize>,
    #[serde(default)]
    pub hosts: Vec<Host>,
    pub candidates: Vec<Candidate>,
    #[serde(default)]
    pub resources: Option<Resources>,
}

#[derive(Deserialize)]
#[serde(crate = "pax_message::serde", rename_all = "camelCase")]
pub(crate) struct Resources {
    viewport_pixels: u64,
    unreleased_pixels: u64,
    #[serde(default)]
    pub unavailable_surfaces: usize,
}

#[derive(Deserialize)]
#[serde(crate = "pax_message::serde", rename_all = "camelCase")]
pub(crate) struct Host {
    id: u32,
    parent: Option<u32>,
    bounds: [f64; 4],
    visible: [f64; 4],
    viewport: [f64; 2],
    offset: [f64; 2],
    observed: [f64; 2],
    basis: [f64; 4],
    active: bool,
    geometry: String,
    #[serde(default)]
    content: Option<[f64; 2]>,
    #[serde(default)]
    intermediate_clip: Option<[f64; 4]>,
}

#[derive(Deserialize)]
#[serde(crate = "pax_message::serde", rename_all = "camelCase")]
pub(crate) struct PreparationSnapshot {
    version: u32,
    now: f64,
    viewport: [f64; 4],
    hosts: Vec<Host>,
    #[serde(default)]
    readiness: Vec<(String, f64)>,
}

pub(crate) struct PreparedTransaction {
    pub now: f64,
    pub hosts: BTreeMap<u32, PreparationHost>,
    pub plans: BTreeMap<u32, PreparedHost>,
}

#[derive(Serialize)]
#[serde(crate = "pax_message::serde", rename_all = "camelCase")]
pub(crate) struct PreparedSummary {
    id: u32,
    visible: [f64; 4],
    required: [f64; 4],
    prepared: [f64; 4],
    horizon_ms: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    unavailable_reason: Option<&'static str>,
}

impl PreparedTransaction {
    pub fn summaries(&self) -> Vec<PreparedSummary> {
        self.plans
            .iter()
            .map(|(&id, p)| PreparedSummary {
                id,
                visible: edges(p.visible),
                required: edges(p.required),
                prepared: edges(p.prepared),
                horizon_ms: p.horizon_ms(),
                unavailable_reason: p.unavailable_reason,
            })
            .collect()
    }
}

fn edges(r: CoverageRect) -> [f64; 4] {
    [r.left, r.top, r.right, r.bottom]
}

pub(crate) fn prepare(
    snapshot: PreparationSnapshot,
    activity: &mut ScrollActivityTracker,
    latency: &mut surface_preparation::PreparationLatency,
) -> Result<PreparedTransaction, &'static str> {
    if snapshot.version != 1
        || !snapshot.now.is_finite()
        || !snapshot.viewport.iter().all(|v| v.is_finite())
    {
        return Err("invalid preparation observation version/time/viewport");
    }
    let mut hosts = BTreeMap::new();
    for h in snapshot.hosts {
        activity.observe(h.id, h.observed, snapshot.now, &h.geometry);
        let content = h.content.ok_or("missing preparation content extent")?;
        let clip = h.intermediate_clip.map(rect);
        let id = h.id;
        if hosts
            .insert(
                id,
                PreparationHost {
                    geometry: observation(h),
                    content,
                    intermediate_clip: clip,
                },
            )
            .is_some()
        {
            return Err("duplicate preparation host");
        }
    }
    activity.retain(&hosts.keys().copied().collect());
    latency.observe(&snapshot.readiness);
    let plans = surface_preparation::prepare_hosts(
        &hosts,
        rect(snapshot.viewport),
        activity,
        snapshot.now,
        latency.policy(),
    )?;
    activity.retain(
        &plans
            .iter()
            .filter_map(|(&id, p)| p.unavailable_reason.is_none().then_some(id))
            .collect(),
    );
    Ok(PreparedTransaction {
        now: snapshot.now,
        hosts,
        plans,
    })
}

fn validate_host(h: &Host) -> Result<(), &'static str> {
    if !h
        .bounds
        .iter()
        .chain(h.visible.iter())
        .chain(h.viewport.iter())
        .chain(h.offset.iter())
        .chain(h.observed.iter())
        .chain(h.basis.iter())
        .chain(h.content.iter().flatten())
        .chain(h.intermediate_clip.iter().flatten())
        .all(|v| v.is_finite())
    {
        return Err("non-finite surface observation");
    }
    if h.viewport
        .iter()
        .chain(h.content.iter().flatten())
        .any(|v| *v < 0.0)
    {
        return Err("negative surface extent");
    }
    Ok(())
}

fn observation(h: Host) -> HostObservation {
    HostObservation {
        id: h.id,
        parent: h.parent,
        bounds: rect(h.bounds),
        visible: rect(h.visible),
        viewport: h.viewport,
        offset: h.offset,
        basis: h.basis,
        active: h.active,
    }
}

#[derive(Deserialize)]
#[serde(crate = "pax_message::serde", rename_all = "camelCase")]
pub(crate) struct Candidate {
    id: String,
    owner: Option<u32>,
    rect: [f64; 4],
    retained: bool,
    ready: bool,
    pixels: u64,
}

#[derive(Serialize)]
#[serde(crate = "pax_message::serde", rename_all = "camelCase")]
pub(crate) struct Decision {
    admitted: Vec<usize>,
    denied: Vec<(usize, &'static str)>,
    ranks: Vec<Rank>,
    overloaded: bool,
    backing_pixels: u64,
    backing_limit: u64,
    optional_pixels: u64,
    optional_limit: u64,
    borrowed_forward_pixels: u64,
    forward_limit: Option<(usize, u64)>,
}

#[derive(Serialize)]
#[serde(crate = "pax_message::serde", rename_all = "camelCase")]
struct Rank {
    coverage: u8,
    distance: f64,
    retained: bool,
    ready: bool,
}

fn rect([left, top, right, bottom]: [f64; 4]) -> CoverageRect {
    CoverageRect {
        left,
        top,
        right,
        bottom,
    }
}

pub(crate) fn schedule(
    snapshot: Snapshot,
    activity: &mut ScrollActivityTracker,
    prepared: Option<&PreparedTransaction>,
) -> Result<Decision, &'static str> {
    if ![1, 2].contains(&snapshot.version) || !snapshot.now.is_finite() {
        return Err("invalid surface observation version/time");
    }
    if snapshot.version == 2 && prepared.map(|p| p.now) != Some(snapshot.now) {
        return Err("missing or mismatched preparation transaction");
    }
    let mut hosts = BTreeMap::new();
    // Version 2 consumes the authoritative preparation transaction. Revalidating
    // a second copy of the DOM observations would reintroduce global rejection
    // when that transaction has already quarantined a malformed branch.
    for h in snapshot.hosts.into_iter().filter(|_| snapshot.version == 1) {
        validate_host(&h)?;
        activity.observe(h.id, h.observed, snapshot.now, &h.geometry);
        if hosts
            .insert(
                h.id,
                HostObservation {
                    id: h.id,
                    parent: h.parent,
                    bounds: rect(h.bounds),
                    visible: rect(h.visible),
                    viewport: h.viewport,
                    offset: h.offset,
                    basis: h.basis,
                    active: h.active,
                },
            )
            .is_some()
        {
            return Err("duplicate surface host observation");
        }
    }
    if snapshot.version == 1 {
        activity.retain(&hosts.keys().copied().collect::<HashSet<_>>());
    }
    let mut candidates = Vec::new();
    for c in snapshot.candidates {
        if !c.rect.iter().all(|v| v.is_finite()) {
            return Err("non-finite surface coverage");
        }
        let mut candidate = SurfaceCandidate {
            id: c.id,
            branch: c.owner.map_or(-1, i64::from),
            coverage: Coverage::Visible,
            distance: 0.0,
            retained: c.retained,
            ready: c.ready,
            backing_pixels: c.pixels,
        };
        if snapshot.version == 2 {
            if let Some(owner) = c.owner {
                let p = prepared.unwrap();
                let host = p.hosts.get(&owner).ok_or("missing prepared owner")?;
                let plan = &p.plans[&owner];
                if plan.unavailable_reason.is_some() {
                    candidate.coverage = Coverage::OutsidePreparation;
                } else {
                    let tile = shared::project_surface(rect(c.rect), &host.geometry);
                    (candidate.coverage, candidate.distance) =
                        plan.rank(tile, c.retained && c.ready);
                }
            }
        } else {
            shared::rank_surface(
                &mut candidate,
                rect(c.rect),
                c.owner,
                &hosts,
                activity,
                snapshot.now,
            );
        }
        candidates.push(candidate);
    }
    let policy = shared::ResidencyPolicy::default();
    let budget = match snapshot.resources {
        Some(r) => policy.available(snapshot.capacity, r.viewport_pixels, r.unreleased_pixels),
        None => SurfaceBudget::count_only(snapshot.capacity),
    };
    let result = shared::schedule_surfaces(&candidates, budget);
    Ok(Decision {
        admitted: result.admitted,
        denied: result
            .denied
            .into_iter()
            .map(|(i, reason)| {
                (
                    i,
                    match reason {
                        shared::DenialReason::VisibleCapacity => "visible-demand-exceeds-budget",
                        shared::DenialReason::PreparationCapacity => "preparation-budget",
                        shared::DenialReason::BackingPixels => "backing-pixel-budget",
                        shared::DenialReason::OptionalPixels => "optional-pixel-budget",
                        shared::DenialReason::OutsidePreparation => {
                            "outside-preparation-trajectory"
                        }
                    },
                )
            })
            .collect(),
        ranks: candidates
            .iter()
            .map(|c| Rank {
                coverage: c.coverage as u8,
                distance: c.distance,
                retained: c.retained,
                ready: c.ready,
            })
            .collect(),
        overloaded: result.overloaded,
        backing_pixels: result.backing_pixels,
        backing_limit: policy.backing_pixels,
        optional_pixels: result.optional_pixels,
        optional_limit: budget.optional_pixels,
        borrowed_forward_pixels: result.borrowed_forward_pixels,
        forward_limit: budget.forward_limit,
    })
}
