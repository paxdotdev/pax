//! Clip-aware, time-coupled exposure planning. All rectangles use presentation
//! coordinates until the final inverse mapping into owner content space.

use std::collections::{BTreeMap, HashSet};

use super::surface_scheduling::{
    project_surface, Coverage, CoverageRect, HostObservation, ScrollActivityTracker,
};

/// A host and the conservative clip between it and its nearest scrolling parent.
/// The intermediate clip moves with the parent's content, not its viewport.
pub struct PreparationHost {
    pub geometry: HostObservation,
    pub content: [f64; 2],
    pub intermediate_clip: Option<CoverageRect>,
}

/// Bounded prediction, independent of chassis event types. The horizon is an
/// initial calibration value, not a guarantee of content preparation latency.
#[derive(Clone, Copy)]
pub struct PreparationPolicy {
    pub horizon_ms: f64,
    pub max_viewports: f64,
    /// Independent first-interaction coverage on currently exposed scroll axes.
    pub safety_viewports: f64,
    /// Spatial hysteresis for already submitted assignments; never starts work.
    pub retention_viewports: f64,
    /// Opportunistic forward distance, admitted only when capacity permits.
    pub reserve_viewports: f64,
    /// Recent pauses precede older reserve work; this is not an expiry deadline.
    pub reserve_tail_ms: f64,
}

impl Default for PreparationPolicy {
    fn default() -> Self {
        Self {
            horizon_ms: 250.0,
            max_viewports: 6.0,
            safety_viewports: 0.5,
            retention_viewports: 1.0,
            reserve_viewports: 6.0,
            reserve_tail_ms: 1500.0,
        }
    }
}

/// Conservative end-to-end preparation latency, sampled once per current
/// assignment. Hosts supply monotonic durations from assignment to observed
/// submission, including queueing and first drawing, not just GPU allocation.
#[derive(Default)]
pub struct PreparationLatency {
    seen: HashSet<String>,
    estimate_ms: f64,
}

impl PreparationLatency {
    pub fn observe(&mut self, samples: &[(String, f64)]) {
        let live: HashSet<_> = samples.iter().map(|(id, _)| id.clone()).collect();
        self.seen.retain(|id| live.contains(id));
        for (id, ms) in samples {
            if !ms.is_finite() || *ms < 0.0 || !self.seen.insert(id.clone()) {
                continue;
            }
            // Startup stalls and background tabs cannot grow speculation
            // without bound. Faster samples lower the estimate gradually.
            self.estimate_ms = (self.estimate_ms * 0.95).max(ms.min(350.0));
        }
    }

    pub fn policy(&self) -> PreparationPolicy {
        let mut policy = PreparationPolicy::default();
        policy.horizon_ms = policy
            .horizon_ms
            .max(self.estimate_ms * 2.0 + 50.0)
            .min(750.0);
        policy
    }
}

#[derive(Clone, Copy)]
struct MovingClip {
    rect: CoverageRect,
    delta: [f64; 2],
}

/// Current and anticipated exposure in owner content coordinates. The private
/// clip trajectory lets admission reject corners introduced by a swept AABB.
pub struct PreparedHost {
    /// A malformed host (or ancestor) is unavailable for this observation only.
    /// Its empty plan must not preserve stale coverage or block valid siblings.
    pub unavailable_reason: Option<&'static str>,
    pub visible: CoverageRect,
    pub required: CoverageRect,
    /// Candidate enumeration bounds, including retain-only space. Admission
    /// must call `rank`; this AABB alone does not authorize cold preparation.
    pub prepared: CoverageRect,
    clips: Vec<MovingClip>,
    safety: Vec<Vec<MovingClip>>,
    retention: Vec<Vec<MovingClip>>,
    reserve: Vec<(Vec<MovingClip>, f64)>,
    horizon_ms: f64,
}

impl PreparedHost {
    fn unavailable(reason: &'static str, horizon_ms: f64) -> Self {
        Self {
            unavailable_reason: Some(reason),
            visible: CoverageRect::default(),
            required: CoverageRect::default(),
            prepared: CoverageRect::default(),
            clips: Vec::new(),
            safety: Vec::new(),
            retention: Vec::new(),
            reserve: Vec::new(),
            horizon_ms,
        }
    }

    pub fn horizon_ms(&self) -> f64 {
        self.horizon_ms
    }

    /// Earliest predicted intersection, in milliseconds; `None` is outside the
    /// trajectory. A tile already intersecting the current clips has deadline 0.
    pub fn deadline(&self, projected_tile: CoverageRect) -> Option<f64> {
        if self.unavailable_reason.is_some() {
            return None;
        }
        intersection_start(&self.clips, projected_tile).map(|start| start * self.horizon_ms)
    }

    /// Exact trajectory admission, not the enclosing candidate AABB. Retention
    /// is a lower-priority reuse opportunity, never permission to initialize a
    /// cold tile or to displace required/forward/safety coverage.
    pub fn rank(&self, tile: CoverageRect, retained_ready: bool) -> (Coverage, f64) {
        if self.unavailable_reason.is_some() {
            return (Coverage::OutsidePreparation, 0.0);
        }
        if tile.intersects(self.visible) {
            (Coverage::Visible, 0.0)
        } else if let Some(deadline) = self.deadline(tile) {
            (Coverage::MovingAhead, deadline)
        } else if let Some(distance) = nearest_intersection(&self.safety, tile) {
            (Coverage::StationaryNear, distance)
        } else if let Some(distance) = self
            .reserve
            .iter()
            .filter_map(|(path, age)| intersection_start(path, tile).map(|t| t + age))
            .min_by(f64::total_cmp)
        {
            (Coverage::ForwardReserve, distance)
        } else if retained_ready {
            nearest_intersection(&self.retention, tile)
                .map(|distance| (Coverage::FartherPreparation, distance))
                .unwrap_or((Coverage::OutsidePreparation, 0.0))
        } else {
            (Coverage::OutsidePreparation, 0.0)
        }
    }
}

fn intersection_start(trajectory: &[MovingClip], tile: CoverageRect) -> Option<f64> {
    let mut clips = trajectory.to_vec();
    clips.push(MovingClip {
        rect: tile,
        delta: [0.0; 2],
    });
    feasible_interval(&clips).map(|(start, _)| start)
}

fn nearest_intersection(trajectories: &[Vec<MovingClip>], tile: CoverageRect) -> Option<f64> {
    trajectories
        .iter()
        .filter_map(|t| intersection_start(t, tile))
        .min_by(f64::total_cmp)
}

fn union(a: CoverageRect, b: CoverageRect) -> CoverageRect {
    if !a.has_area() {
        return b;
    }
    if !b.has_area() {
        return a;
    }
    CoverageRect {
        left: a.left.min(b.left),
        top: a.top.min(b.top),
        right: a.right.max(b.right),
        bottom: a.bottom.max(b.bottom),
    }
}

fn intersect(a: CoverageRect, b: CoverageRect) -> CoverageRect {
    CoverageRect {
        left: a.left.max(b.left),
        top: a.top.max(b.top),
        right: a.right.min(b.right),
        bottom: a.bottom.min(b.bottom),
    }
}

fn translated(r: CoverageRect, delta: [f64; 2], t: f64) -> CoverageRect {
    CoverageRect {
        left: r.left + delta[0] * t,
        right: r.right + delta[0] * t,
        top: r.top + delta[1] * t,
        bottom: r.bottom + delta[1] * t,
    }
}

// Intersections of translated rectangles are piecewise linear in time. Solve
// their feasible interval first: endpoint-only sampling misses a small child
// crossing the viewport entirely between samples.
fn feasible_interval(clips: &[MovingClip]) -> Option<(f64, f64)> {
    let mut start: f64 = 0.0;
    let mut end: f64 = 1.0;
    for a in clips {
        if !a.rect.has_area() {
            return None;
        }
        for b in clips {
            for (gap, slope) in [
                (a.rect.left - b.rect.right, a.delta[0] - b.delta[0]),
                (a.rect.top - b.rect.bottom, a.delta[1] - b.delta[1]),
            ] {
                if slope.abs() < 1e-12 {
                    if gap >= 0.0 {
                        return None;
                    }
                } else if slope > 0.0 {
                    end = end.min(-gap / slope);
                } else {
                    start = start.max(-gap / slope);
                }
            }
        }
    }
    (start < end).then_some((start, end))
}

fn swept_intersection(clips: &[MovingClip]) -> CoverageRect {
    let Some((start, end)) = feasible_interval(clips) else {
        return CoverageRect::default();
    };
    let mut times = vec![start, end];
    // Max(left/top) and min(right/bottom) can turn at any pairwise crossing.
    // Include those turns to obtain the complete conservative swept bounds.
    for (i, a) in clips.iter().enumerate() {
        for b in &clips[i + 1..] {
            for (origin, slope) in [
                (a.rect.left - b.rect.left, a.delta[0] - b.delta[0]),
                (a.rect.right - b.rect.right, a.delta[0] - b.delta[0]),
                (a.rect.top - b.rect.top, a.delta[1] - b.delta[1]),
                (a.rect.bottom - b.rect.bottom, a.delta[1] - b.delta[1]),
            ] {
                if slope.abs() > 1e-12 {
                    let t = -origin / slope;
                    if t > start && t < end {
                        times.push(t);
                    }
                }
            }
        }
    }
    let mut out = CoverageRect {
        left: f64::INFINITY,
        top: f64::INFINITY,
        right: f64::NEG_INFINITY,
        bottom: f64::NEG_INFINITY,
    };
    for t in times {
        let r = clips
            .iter()
            .map(|c| translated(c.rect, c.delta, t))
            .reduce(intersect)
            .unwrap();
        // At entry/exit, a zero-width edge still bounds the immediately adjacent
        // nonempty interval. Excluding it loses the leading edge of the sweep.
        if r.right >= r.left && r.bottom >= r.top {
            out.left = out.left.min(r.left);
            out.top = out.top.min(r.top);
            out.right = out.right.max(r.right);
            out.bottom = out.bottom.max(r.bottom);
        }
    }
    if out.has_area() {
        out
    } else {
        CoverageRect::default()
    }
}

fn unproject(rect: CoverageRect, h: &HostObservation) -> Option<CoverageRect> {
    if !rect.has_area() {
        return Some(CoverageRect::default());
    }
    let [a, b, c, d] = h.basis;
    let det = a * d - b * c;
    if det.abs() <= 1e-12 || !det.is_finite() {
        return None;
    }
    // Project a unit content origin through the same host anchor as admission.
    let origin = project_surface(
        CoverageRect {
            left: 0.0,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
        },
        h,
    );
    let points = [
        [rect.left, rect.top],
        [rect.right, rect.top],
        [rect.left, rect.bottom],
        [rect.right, rect.bottom],
    ]
    .map(|[x, y]| {
        let x = x - origin.left;
        let y = y - origin.top;
        [(d * x - c * y) / det, (-b * x + a * y) / det]
    });
    Some(CoverageRect {
        left: points.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min),
        top: points.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min),
        right: points
            .iter()
            .map(|p| p[0])
            .fold(f64::NEG_INFINITY, f64::max),
        bottom: points
            .iter()
            .map(|p| p[1])
            .fold(f64::NEG_INFINITY, f64::max),
    })
}

fn trajectory(
    chain: &[&PreparationHost],
    viewport: CoverageRect,
    deltas: &[[f64; 2]],
) -> Vec<MovingClip> {
    let mut motion = [0.0; 2];
    let mut clips = vec![MovingClip {
        rect: viewport,
        delta: motion,
    }];
    for (h, delta) in chain.iter().zip(deltas) {
        clips.push(MovingClip {
            rect: h.geometry.bounds,
            delta: motion,
        });
        if let Some(rect) = h.intermediate_clip {
            clips.push(MovingClip {
                rect,
                delta: motion,
            });
        }
        let [a, b, c, d] = h.geometry.basis;
        motion[0] += a * delta[0] + c * delta[1];
        motion[1] += b * delta[0] + d * delta[1];
    }
    for clip in &mut clips {
        clip.delta = [motion[0] - clip.delta[0], motion[1] - clip.delta[1]];
    }
    clips
}

fn clamped_travel(host: &PreparationHost, axis: usize, travel: f64) -> f64 {
    let max = (host.content[axis] - host.geometry.viewport[axis]).max(0.0);
    let offset = host.geometry.offset[axis].clamp(0.0, max);
    (offset + travel).clamp(0.0, max) - offset
}

fn validate_host(host: &PreparationHost) -> Result<(), &'static str> {
    let g = &host.geometry;
    let rect_values = |r: CoverageRect| [r.left, r.top, r.right, r.bottom];
    if !g
        .viewport
        .iter()
        .chain(g.offset.iter())
        .chain(g.basis.iter())
        .chain(host.content.iter())
        .copied()
        .chain(rect_values(g.bounds))
        .chain(rect_values(g.visible))
        .chain(host.intermediate_clip.into_iter().flat_map(rect_values))
        .all(f64::is_finite)
    {
        return Err("non-finite surface observation");
    }
    if g.viewport
        .iter()
        .chain(host.content.iter())
        .any(|v| *v < 0.0)
    {
        return Err("negative surface extent");
    }
    let [a, b, c, d] = g.basis;
    let determinant = a * d - b * c;
    if !determinant.is_finite() || determinant.abs() <= 1e-12 {
        return Err("singular surface transform");
    }
    Ok(())
}

/// Plan exposure along one shared time axis. Actual ancestor/child movement is
/// composed. Safety and retention are separate one-axis alternatives for each
/// currently exposed host, not a Cartesian product of nested warm windows.
/// An offscreen shelf inherits parent exposure, not independent horizontal pad.
/// Invalid geometry or ancestry quarantines only that branch for the current
/// observation. Valid siblings continue; corrected observations recover normally.
pub fn prepare_hosts(
    hosts: &BTreeMap<u32, PreparationHost>,
    viewport: CoverageRect,
    activity: &ScrollActivityTracker,
    now: f64,
    policy: PreparationPolicy,
) -> Result<BTreeMap<u32, PreparedHost>, &'static str> {
    if !now.is_finite()
        || ![viewport.left, viewport.top, viewport.right, viewport.bottom]
            .into_iter()
            .all(f64::is_finite)
    {
        return Err("invalid preparation time/viewport");
    }
    let mut out = BTreeMap::new();
    for (&id, host) in hosts {
        let plan = (|| -> Result<PreparedHost, &'static str> {
            let mut chain = Vec::new();
            let mut seen = HashSet::new();
            let mut node = Some(id);
            while let Some(id) = node {
                if !seen.insert(id) {
                    return Err("cyclic surface ancestry");
                }
                let Some(h) = hosts.get(&id) else {
                    return Err("missing surface ancestor");
                };
                validate_host(h)?;
                chain.push(h);
                node = h.geometry.parent;
            }
            chain.reverse();
            let deltas: Vec<_> = chain
                .iter()
                .map(|h| {
                    let m = activity.get(h.geometry.id, now);
                    [0, 1].map(|axis| {
                        let travel = (m.speed[axis] * policy.horizon_ms)
                            .min(h.geometry.viewport[axis] * policy.max_viewports)
                            * m.direction[axis];
                        clamped_travel(h, axis, travel)
                    })
                })
                .collect();
            let clips = trajectory(&chain, viewport, &deltas);
            let mut safety = Vec::new();
            let mut retention = Vec::new();
            let mut reserve = Vec::new();
            let mut exposed = viewport;
            for (index, h) in chain.iter().enumerate() {
                exposed = intersect(exposed, h.geometry.bounds);
                if let Some(clip) = h.intermediate_clip {
                    exposed = intersect(exposed, clip);
                }
                if !exposed.has_area() {
                    continue;
                }
                let last = activity.recent(h.geometry.id, now, f64::INFINITY);
                let recent = activity.recent(h.geometry.id, now, policy.reserve_tail_ms);
                let active = activity.get(h.geometry.id, now);
                for axis in 0..2 {
                    if last.direction[axis] != 0.0 {
                        let mut deltas = vec![[0.0; 2]; chain.len()];
                        deltas[index][axis] = clamped_travel(
                            h,
                            axis,
                            last.direction[axis]
                                * policy.reserve_viewports
                                * h.geometry.viewport[axis],
                        );
                        // A still-moving branch's reserve precedes a paused one.
                        let age = if recent.direction[axis] == 0.0 {
                            2.0
                        } else if active.direction[axis] == 0.0 {
                            1.0
                        } else {
                            0.0
                        };
                        reserve.push((trajectory(&chain, viewport, &deltas), age));
                    }
                    for sign in [-1.0, 1.0] {
                        for (span, paths) in [
                            (policy.safety_viewports, &mut safety),
                            (policy.retention_viewports, &mut retention),
                        ] {
                            let travel =
                                clamped_travel(h, axis, sign * span * h.geometry.viewport[axis]);
                            if travel.abs() <= 0.001 {
                                continue;
                            }
                            let mut deltas = vec![[0.0; 2]; chain.len()];
                            deltas[index][axis] = travel;
                            paths.push(trajectory(&chain, viewport, &deltas));
                        }
                    }
                }
            }
            let visible = clips.iter().map(|c| c.rect).reduce(intersect).unwrap();
            let content = CoverageRect {
                left: 0.0,
                top: 0.0,
                right: host.content[0],
                bottom: host.content[1],
            };
            let required =
                unproject(visible, &host.geometry).ok_or("singular surface transform")?;
            let coverage = safety
                .iter()
                .chain(&retention)
                .chain(reserve.iter().map(|(path, _)| path))
                .fold(swept_intersection(&clips), |bounds, path| {
                    union(bounds, swept_intersection(path))
                });
            let prepared =
                unproject(coverage, &host.geometry).ok_or("singular surface transform")?;
            Ok(PreparedHost {
                unavailable_reason: None,
                visible,
                required: intersect(required, content),
                prepared: intersect(prepared, content),
                clips,
                safety,
                retention,
                reserve,
                horizon_ms: policy.horizon_ms,
            })
        })();
        out.insert(
            id,
            plan.unwrap_or_else(|reason| PreparedHost::unavailable(reason, policy.horizon_ms)),
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn r(x: f64, y: f64, w: f64, h: f64) -> CoverageRect {
        CoverageRect {
            left: x,
            top: y,
            right: x + w,
            bottom: y + h,
        }
    }
    fn host(
        id: u32,
        parent: Option<u32>,
        bounds: CoverageRect,
        content: [f64; 2],
    ) -> PreparationHost {
        PreparationHost {
            geometry: HostObservation {
                id,
                parent,
                bounds,
                visible: bounds,
                viewport: [bounds.right - bounds.left, bounds.bottom - bounds.top],
                offset: [0.0; 2],
                basis: [1.0, 0.0, 0.0, 1.0],
                active: true,
            },
            content,
            intermediate_clip: None,
        }
    }
    #[test]
    fn malformed_branches_do_not_freeze_valid_hosts_and_recover_on_next_observation() {
        let viewport = r(0.0, 0.0, 400.0, 300.0);
        let mut hosts = BTreeMap::from([
            (1, host(1, None, viewport, [400.0, 5000.0])),
            (2, host(2, Some(1), viewport, [400.0, 5000.0])),
            (3, host(3, Some(2), viewport, [400.0, 5000.0])),
            (4, host(4, Some(1), viewport, [400.0, 5000.0])),
        ]);
        let mut activity = ScrollActivityTracker::default();
        activity.observe(4, [0.0, 0.0], 0.0, "same");
        activity.observe(4, [0.0, 80.0], 50.0, "same");
        for defect in 0..5 {
            let bad = hosts.get_mut(&2).unwrap();
            match defect {
                0 => bad.geometry.viewport[0] = -28.0,
                1 => bad.content[1] = f64::NAN,
                2 => bad.geometry.basis = [0.0; 4],
                3 => bad.geometry.parent = Some(999),
                _ => bad.geometry.parent = Some(3),
            }
            let plans =
                prepare_hosts(&hosts, viewport, &activity, 50.0, Default::default()).unwrap();
            for id in [2, 3] {
                let p = &plans[&id];
                assert!(p.unavailable_reason.is_some());
                assert!(!p.prepared.has_area());
                assert_eq!(p.rank(viewport, true).0, Coverage::OutsidePreparation);
                assert_eq!(p.deadline(viewport), None);
            }
            for id in [1, 4] {
                assert!(plans[&id].unavailable_reason.is_none());
                assert_eq!(plans[&id].rank(viewport, false).0, Coverage::Visible);
            }
            assert_eq!(
                plans[&4].rank(r(0.0, 400.0, 400.0, 100.0), false).0,
                Coverage::MovingAhead
            );
            hosts.insert(2, host(2, Some(1), viewport, [400.0, 5000.0]));
            let recovered =
                prepare_hosts(&hosts, viewport, &activity, 60.0, Default::default()).unwrap();
            assert!(recovered.values().all(|p| p.unavailable_reason.is_none()));
            assert_eq!(recovered[&3].rank(viewport, false).0, Coverage::Visible);
        }
    }

    #[test]
    fn idle_safety_follows_parent_exposure_without_multiplying_child_axes() {
        let hosts = BTreeMap::from([
            (1, host(1, None, r(0.0, 0.0, 400.0, 300.0), [400.0, 5000.0])),
            (
                2,
                host(2, Some(1), r(0.0, 200.0, 400.0, 200.0), [5000.0, 200.0]),
            ),
            (
                3,
                host(3, Some(1), r(0.0, 500.0, 400.0, 200.0), [5000.0, 200.0]),
            ),
        ]);
        let p = prepare_hosts(
            &hosts,
            r(0.0, 0.0, 400.0, 300.0),
            &Default::default(),
            0.0,
            Default::default(),
        )
        .unwrap();
        assert_eq!(
            p[&1].rank(r(0.0, 300.0, 400.0, 100.0), false).0,
            Coverage::StationaryNear
        );
        // A shelf beyond safety is enumerated only for retaining old content.
        assert_eq!(
            p[&3].rank(r(0.0, 500.0, 400.0, 100.0), false).0,
            Coverage::OutsidePreparation
        );
        assert_eq!(
            p[&3].rank(r(0.0, 500.0, 400.0, 100.0), true).0,
            Coverage::FartherPreparation
        );
        assert_eq!(
            p[&3].rank(r(450.0, 500.0, 100.0, 100.0), true).0,
            Coverage::OutsidePreparation
        );
        assert_eq!(
            p[&2].rank(r(450.0, 200.0, 100.0, 80.0), false).0,
            Coverage::StationaryNear
        );
        // Parent Y safety + child X safety must not create a Cartesian corner.
        assert_eq!(
            p[&2].rank(r(450.0, 320.0, 100.0, 40.0), false).0,
            Coverage::OutsidePreparation
        );
    }
    #[test]
    fn moving_parent_exposes_child_but_not_its_idle_horizontal_axis() {
        let hosts = BTreeMap::from([
            (1, host(1, None, r(0.0, 0.0, 400.0, 300.0), [400.0, 5000.0])),
            (
                2,
                host(2, Some(1), r(0.0, 400.0, 400.0, 100.0), [5000.0, 100.0]),
            ),
        ]);
        let mut a = ScrollActivityTracker::default();
        a.observe(1, [0.0, 0.0], 0.0, "");
        a.observe(1, [0.0, 100.0], 50.0, "");
        let p = prepare_hosts(
            &hosts,
            r(0.0, 0.0, 400.0, 300.0),
            &a,
            50.0,
            Default::default(),
        )
        .unwrap();
        assert_eq!(p[&2].prepared, r(0.0, 0.0, 400.0, 100.0));
        assert_eq!(p[&2].deadline(r(0.0, 400.0, 100.0, 100.0)), Some(100.0));
        assert_eq!(p[&2].deadline(r(500.0, 400.0, 100.0, 100.0)), None);
    }
    #[test]
    fn thin_child_crossing_between_endpoints_is_not_missed() {
        let clips = [
            MovingClip {
                rect: r(0.0, 0.0, 10.0, 10.0),
                delta: [0.0, 100.0],
            },
            MovingClip {
                rect: r(0.0, 40.0, 10.0, 10.0),
                delta: [0.0; 2],
            },
        ];
        assert_eq!(swept_intersection(&clips), r(0.0, 40.0, 10.0, 10.0));
    }
    #[test]
    fn simultaneous_axes_do_not_admit_cartesian_sweep_corners() {
        let p = PreparedHost {
            unavailable_reason: None,
            visible: r(0.0, 0.0, 10.0, 10.0),
            required: Default::default(),
            prepared: Default::default(),
            clips: vec![MovingClip {
                rect: r(0.0, 0.0, 10.0, 10.0),
                delta: [100.0, 100.0],
            }],
            safety: vec![],
            retention: vec![],
            reserve: vec![],
            horizon_ms: 100.0,
        };
        assert!(p.deadline(r(50.0, 50.0, 5.0, 5.0)).is_some());
        assert_eq!(p.deadline(r(0.0, 90.0, 5.0, 5.0)), None);
    }
    #[test]
    fn intermediate_clip_and_redundant_wrappers_stay_tight() {
        let mut child = host(2, Some(1), r(0.0, 0.0, 400.0, 300.0), [5000.0, 300.0]);
        child.intermediate_clip = Some(r(50.0, 20.0, 100.0, 100.0));
        let hosts = BTreeMap::from([
            (1, host(1, None, r(0.0, 0.0, 400.0, 300.0), [400.0, 5000.0])),
            (2, child),
        ]);
        let p = prepare_hosts(
            &hosts,
            r(0.0, 0.0, 400.0, 300.0),
            &Default::default(),
            0.0,
            Default::default(),
        )
        .unwrap();
        assert_eq!(p[&2].required, r(50.0, 20.0, 100.0, 100.0));
        assert_eq!(
            p[&2].rank(r(60.0, 130.0, 20.0, 20.0), false).0,
            Coverage::OutsidePreparation
        );
    }

    #[test]
    fn exact_tile_boundary_has_idle_coverage_and_pause_does_not_remove_it() {
        let hosts =
            BTreeMap::from([(1, host(1, None, r(0.0, 0.0, 400.0, 640.0), [400.0, 5000.0]))]);
        let p = prepare_hosts(
            &hosts,
            r(0.0, 0.0, 400.0, 640.0),
            &Default::default(),
            5000.0,
            Default::default(),
        )
        .unwrap();
        assert_eq!(p[&1].required, r(0.0, 0.0, 400.0, 640.0));
        assert_eq!(
            p[&1].rank(r(0.0, 640.0, 400.0, 640.0), false).0,
            Coverage::StationaryNear
        );
        assert_eq!(
            p[&1].rank(r(0.0, 1280.0, 400.0, 640.0), true).0,
            Coverage::OutsidePreparation
        );
    }

    #[test]
    fn reversal_has_a_safety_margin_and_far_retention_yields() {
        let mut h = host(1, None, r(0.0, 0.0, 400.0, 300.0), [400.0, 5000.0]);
        h.geometry.offset[1] = 600.0;
        let hosts = BTreeMap::from([(1, h)]);
        let mut activity = ScrollActivityTracker::default();
        activity.observe(1, [0.0, 500.0], 0.0, "same");
        activity.observe(1, [0.0, 600.0], 50.0, "same");
        let moving = prepare_hosts(
            &hosts,
            r(0.0, 0.0, 400.0, 300.0),
            &activity,
            50.0,
            Default::default(),
        )
        .unwrap();
        let paused = prepare_hosts(
            &hosts,
            r(0.0, 0.0, 400.0, 300.0),
            &activity,
            5000.0,
            Default::default(),
        )
        .unwrap();
        let near = r(0.0, 300.0, 400.0, 100.0);
        assert_eq!(moving[&1].rank(near, false).0, Coverage::MovingAhead);
        assert_eq!(paused[&1].rank(near, true).0, Coverage::StationaryNear);
        assert_eq!(
            paused[&1].rank(r(0.0, -100.0, 400.0, 50.0), false).0,
            Coverage::StationaryNear
        );
        let trailing = r(0.0, -250.0, 400.0, 50.0);
        assert_eq!(
            paused[&1].rank(trailing, false).0,
            Coverage::OutsidePreparation
        );
        assert_eq!(
            paused[&1].rank(trailing, true).0,
            Coverage::FartherPreparation
        );
        assert_eq!(
            paused[&1].rank(r(0.0, -500.0, 400.0, 50.0), true).0,
            Coverage::OutsidePreparation
        );
        activity.observe(1, [0.0, 500.0], 100.0, "same");
        let reversed = prepare_hosts(
            &hosts,
            r(0.0, 0.0, 400.0, 300.0),
            &activity,
            100.0,
            Default::default(),
        )
        .unwrap();
        assert_eq!(reversed[&1].rank(trailing, false).0, Coverage::MovingAhead);
    }

    #[test]
    fn readiness_samples_are_bounded_deduplicated_and_include_queue_latency() {
        let mut latency = PreparationLatency::default();
        assert_eq!(latency.policy().horizon_ms, 250.0);
        latency.observe(&[("a".into(), 200.0)]);
        assert_eq!(latency.policy().horizon_ms, 450.0);
        latency.observe(&[("a".into(), 900.0)]);
        assert_eq!(latency.policy().horizon_ms, 450.0);
        latency.observe(&[("b".into(), 10000.0)]);
        assert_eq!(latency.policy().horizon_ms, 750.0);
        latency.observe(&[("bad".into(), f64::NAN)]);
        assert_eq!(latency.policy().horizon_ms, 750.0);
        for i in 0..50 {
            latency.observe(&[(i.to_string(), 20.0)]);
        }
        assert_eq!(latency.policy().horizon_ms, 250.0);
        assert_eq!(latency.seen.len(), 1);
    }

    #[test]
    fn forward_reserve_survives_pauses_at_lower_priority_and_reverses() {
        let mut h = host(1, None, r(0.0, 0.0, 400.0, 300.0), [400.0, 10000.0]);
        h.geometry.offset[1] = 3000.0;
        let hosts = BTreeMap::from([(1, h)]);
        let mut a = ScrollActivityTracker::default();
        a.observe(1, [0.0, 2990.0], 0.0, "");
        a.observe(1, [0.0, 3000.0], 50.0, "");
        let plans = |a: &ScrollActivityTracker, now| {
            prepare_hosts(
                &hosts,
                r(0.0, 0.0, 400.0, 300.0),
                a,
                now,
                Default::default(),
            )
            .unwrap()
        };
        let far = r(0.0, 1500.0, 400.0, 100.0);
        let moving = plans(&a, 50.0);
        assert_eq!(moving[&1].rank(far, false).0, Coverage::ForwardReserve);
        assert_eq!(
            moving[&1].rank(r(0.0, 2100.0, 400.0, 100.0), false).0,
            Coverage::OutsidePreparation
        );
        let paused = plans(&a, 1000.0);
        assert_eq!(paused[&1].rank(far, false).0, Coverage::ForwardReserve);
        assert!(paused[&1].rank(far, false).1 > moving[&1].rank(far, false).1);
        assert_eq!(
            plans(&a, 1600.0)[&1].rank(far, false).0,
            Coverage::ForwardReserve
        );
        assert_eq!(
            plans(&a, 1600.0)[&1].rank(far, true).0,
            Coverage::ForwardReserve
        );
        assert!(plans(&a, 1600.0)[&1].rank(far, false).1 > paused[&1].rank(far, false).1);
        a.observe(1, [0.0, 2999.0], 100.0, "");
        let reversed = plans(&a, 100.0);
        assert_eq!(reversed[&1].rank(far, true).0, Coverage::OutsidePreparation);
        assert_eq!(
            reversed[&1].rank(r(0.0, -1500.0, 400.0, 100.0), false).0,
            Coverage::ForwardReserve
        );
    }

    #[test]
    fn deep_parent_reserve_does_not_multiply_nested_idle_axes() {
        let hosts = BTreeMap::from([
            (
                1,
                host(1, None, r(0.0, 0.0, 400.0, 300.0), [400.0, 10000.0]),
            ),
            (
                2,
                host(2, Some(1), r(0.0, 1500.0, 400.0, 100.0), [5000.0, 100.0]),
            ),
        ]);
        let mut a = ScrollActivityTracker::default();
        a.observe(1, [0.0, 0.0], 0.0, "");
        a.observe(1, [0.0, 10.0], 50.0, "");
        // Even recently moved but now offscreen children cannot multiply pad.
        a.observe(2, [0.0, 0.0], 0.0, "");
        a.observe(2, [10.0, 0.0], 50.0, "");
        let p = prepare_hosts(
            &hosts,
            r(0.0, 0.0, 400.0, 300.0),
            &a,
            500.0,
            Default::default(),
        )
        .unwrap();
        assert_eq!(p[&2].prepared, r(0.0, 0.0, 400.0, 100.0));
        assert_eq!(
            p[&2].rank(r(0.0, 1500.0, 400.0, 100.0), false).0,
            Coverage::ForwardReserve
        );
        assert_eq!(
            p[&2].rank(r(500.0, 1500.0, 100.0, 100.0), false).0,
            Coverage::OutsidePreparation
        );
    }

    #[test]
    fn urgent_prediction_can_exceed_old_two_viewport_cap() {
        let hosts = BTreeMap::from([(
            1,
            host(1, None, r(0.0, 0.0, 400.0, 300.0), [400.0, 10000.0]),
        )]);
        let mut a = ScrollActivityTracker::default();
        a.observe(1, [0.0, 0.0], 0.0, "");
        a.observe(1, [0.0, 500.0], 50.0, "");
        let p = prepare_hosts(
            &hosts,
            r(0.0, 0.0, 400.0, 300.0),
            &a,
            50.0,
            Default::default(),
        )
        .unwrap();
        assert_eq!(
            p[&1].rank(r(0.0, 1000.0, 400.0, 100.0), false).0,
            Coverage::MovingAhead
        );
        assert_eq!(
            p[&1].rank(r(0.0, 1700.0, 400.0, 100.0), false).0,
            Coverage::ForwardReserve
        );
    }

    #[test]
    fn safety_is_local_scale_aware_and_content_bounded() {
        let mut h = host(1, None, r(20.0, 30.0, 200.0, 150.0), [400.0, 1000.0]);
        h.geometry.viewport = [400.0, 300.0];
        h.geometry.basis = [0.5, 0.0, 0.0, 0.5];
        let hosts = BTreeMap::from([(1, h)]);
        let p = prepare_hosts(
            &hosts,
            r(0.0, 0.0, 500.0, 500.0),
            &Default::default(),
            0.0,
            Default::default(),
        )
        .unwrap();
        assert_eq!(p[&1].required, r(0.0, 0.0, 400.0, 300.0));
        assert_eq!(
            p[&1].rank(r(20.0, 180.0, 200.0, 50.0), false).0,
            Coverage::StationaryNear
        );
        assert_eq!(
            p[&1].rank(r(20.0, 260.0, 200.0, 20.0), false).0,
            Coverage::OutsidePreparation
        );
        assert_eq!(
            p[&1].rank(r(20.0, 260.0, 200.0, 20.0), true).0,
            Coverage::FartherPreparation
        );
        assert_eq!(p[&1].prepared, r(0.0, 0.0, 400.0, 600.0));
    }
}
