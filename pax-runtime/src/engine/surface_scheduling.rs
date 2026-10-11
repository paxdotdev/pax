//! Platform-independent surface admission and observed scroll activity.
//!
//! Chassis supply presentation observations, physical costs, and an explicit
//! monotonic clock. This module neither allocates surfaces nor owns GPU leases.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashSet, VecDeque};

/// Movement in local content units per millisecond. Direction is unsmoothed.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollActivity {
    pub direction: [f64; 2],
    pub speed: [f64; 2],
}

#[derive(Debug)]
struct ScrollSample {
    offset: [f64; 2],
    at: f64,
    moved_at: [f64; 2],
    geometry: String,
    activity: ScrollActivity,
}

/// Per-engine activity history, independent of input devices and platform focus.
#[derive(Debug)]
pub struct ScrollActivityTracker {
    samples: BTreeMap<u32, ScrollSample>,
    tail_ms: f64,
}

impl Default for ScrollActivityTracker {
    fn default() -> Self {
        Self::new(250.0)
    }
}

impl ScrollActivityTracker {
    pub fn new(tail_ms: f64) -> Self {
        Self {
            samples: BTreeMap::new(),
            tail_ms: tail_ms.max(0.0),
        }
    }

    /// Geometry changes and movement across long sampling gaps are not velocity
    /// evidence. An unchanged idle observation preserves the last direction for
    /// opportunistic coverage, without refreshing its movement timestamp.
    pub fn observe(&mut self, id: u32, offset: [f64; 2], now: f64, geometry: &str) {
        if !now.is_finite() || !offset.iter().all(|v| v.is_finite()) {
            self.samples.remove(&id);
            return;
        }
        let reset = self.samples.get(&id).map_or(true, |old| {
            old.geometry != geometry
                || now < old.at
                || (now - old.at > 1000.0 && old.offset != offset)
        });
        if reset {
            self.samples.insert(
                id,
                ScrollSample {
                    offset,
                    at: now,
                    moved_at: [f64::NEG_INFINITY; 2],
                    geometry: geometry.into(),
                    activity: ScrollActivity::default(),
                },
            );
            return;
        }
        let old = self.samples.get_mut(&id).unwrap();
        let dt = (now - old.at).max(1.0);
        for axis in 0..2 {
            let delta = offset[axis] - old.offset[axis];
            if delta.abs() > 0.1 {
                old.activity.direction[axis] = delta.signum();
                old.activity.speed[axis] = old.activity.speed[axis] * 0.5 + delta.abs() / dt * 0.5;
                old.moved_at[axis] = now;
            }
        }
        old.offset = offset;
        old.at = now;
    }

    pub fn get(&self, id: u32, now: f64) -> ScrollActivity {
        self.recent(id, now, self.tail_ms)
    }

    /// Last observed local direction within a bounded tail. A longer tail may
    /// retain opportunistic coverage, but must not extend urgent scroll intent.
    pub fn recent(&self, id: u32, now: f64, tail_ms: f64) -> ScrollActivity {
        let Some(s) = self.samples.get(&id) else {
            return ScrollActivity::default();
        };
        let mut activity = s.activity;
        for axis in 0..2 {
            if !now.is_finite() || now < s.at || now - s.moved_at[axis] > tail_ms {
                activity.direction[axis] = 0.0;
            }
        }
        activity
    }

    pub fn retain(&mut self, ids: &HashSet<u32>) {
        self.samples.retain(|id, _| ids.contains(id));
    }
}

/// Lexicographic coverage classes; these are not visual z-order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Coverage {
    Visible,
    MovingAhead,
    /// One-axis reverse coverage, reserved before forward speculation but
    /// prepared after current incoming work. Cold tiles are eligible too.
    ReversalNear,
    ReadyTrailing,
    StationaryNear,
    /// Deeper coverage on a recently moving axis; yields to all near coverage.
    ForwardReserve,
    FartherPreparation,
    OutsidePreparation,
}

#[derive(Clone, Debug)]
pub struct SurfaceCandidate {
    pub id: String,
    pub branch: i64,
    pub coverage: Coverage,
    /// Exposure time for directional preparation, distance for stationary work.
    pub distance: f64,
    pub retained: bool,
    pub ready: bool,
    pub backing_pixels: u64,
}

/// Axis-aligned conservative bounds in one explicitly chosen coordinate space.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CoverageRect {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl CoverageRect {
    pub fn has_area(self) -> bool {
        self.right > self.left && self.bottom > self.top
    }
    pub fn intersects(self, b: Self) -> bool {
        self.has_area()
            && b.has_area()
            && self.left < b.right
            && self.right > b.left
            && self.top < b.bottom
            && self.bottom > b.top
    }
}

/// A chassis observation, not a request to allocate a surface for this host.
pub struct HostObservation {
    pub id: u32,
    pub parent: Option<u32>,
    pub bounds: CoverageRect,
    pub visible: CoverageRect,
    pub viewport: [f64; 2],
    pub offset: [f64; 2],
    pub basis: [f64; 4],
    pub active: bool,
}

/// Project a content-local rectangle using its affine basis and measured host
/// bounds. Browser scroll translation never becomes a scale estimate.
pub fn project_surface(rect: CoverageRect, host: &HostObservation) -> CoverageRect {
    let [a, b, c, d] = host.basis;
    let transform = |r: CoverageRect| {
        let points = [
            [r.left, r.top],
            [r.right, r.top],
            [r.left, r.bottom],
            [r.right, r.bottom],
        ]
        .map(|[x, y]| [a * x + c * y, b * x + d * y]);
        CoverageRect {
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
        }
    };
    let origin = transform(CoverageRect {
        left: 0.0,
        top: 0.0,
        right: host.viewport[0],
        bottom: host.viewport[1],
    });
    let p = transform(CoverageRect {
        left: rect.left - host.offset[0],
        right: rect.right - host.offset[0],
        top: rect.top - host.offset[1],
        bottom: rect.bottom - host.offset[1],
    });
    let x = host.bounds.left - origin.left;
    let y = host.bounds.top - origin.top;
    CoverageRect {
        left: p.left + x,
        right: p.right + x,
        top: p.top + y,
        bottom: p.bottom + y,
    }
}

/// Rank inherited exposure using actual ancestor movement. Child screen motion
/// caused by its parent must not claim independent child-axis scroll intent.
pub fn rank_surface(
    candidate: &mut SurfaceCandidate,
    rect: CoverageRect,
    owner: Option<u32>,
    hosts: &BTreeMap<u32, HostObservation>,
    activity: &ScrollActivityTracker,
    now: f64,
) {
    let Some(host) = owner.and_then(|id| hosts.get(&id)) else {
        return;
    };
    let tile = project_surface(rect, host);
    if tile.intersects(host.visible) {
        candidate.coverage = Coverage::Visible;
        candidate.distance = 0.0;
        return;
    }
    let mut branch = Some(host);
    let mut seen = HashSet::new();
    let mut moving = false;
    let mut ahead = false;
    let mut separation: f64 = 0.0;
    while let Some(host) = branch {
        if !seen.insert(host.id) {
            break;
        }
        let view = host.bounds;
        let dx = if tile.right < view.left {
            tile.right - view.left
        } else if tile.left > view.right {
            tile.left - view.right
        } else {
            0.0
        };
        let dy = if tile.bottom < view.top {
            tile.bottom - view.top
        } else if tile.top > view.bottom {
            tile.top - view.bottom
        } else {
            0.0
        };
        separation = separation.max(dx.hypot(dy));
        let motion = activity.get(host.id, now);
        if motion.direction != [0.0, 0.0] {
            moving = true;
            let [a, b, c, d] = host.basis;
            let vx = motion.direction[0] * motion.speed[0];
            let vy = motion.direction[1] * motion.speed[1];
            let px = a * vx + c * vy;
            let py = b * vx + d * vy;
            ahead |= dx * px > 0.0 || dy * py > 0.0;
            if ahead {
                separation = dx.hypot(dy) / px.hypot(py).max(0.1);
            }
        }
        branch = host.parent.and_then(|id| hosts.get(&id));
    }
    candidate.coverage = if moving && ahead {
        Coverage::MovingAhead
    } else if moving && candidate.ready {
        Coverage::ReadyTrailing
    } else if host.active {
        Coverage::StationaryNear
    } else {
        Coverage::FartherPreparation
    };
    candidate.distance = separation;
}

/// Available capacity after the adapter has accounted for unreleased resources.
#[derive(Clone, Copy, Debug)]
pub struct SurfaceBudget {
    pub surfaces: usize,
    pub backing_pixels: u64,
    pub optional_pixels: u64,
    /// Optional second-pass ceiling (total surfaces, total backing pixels).
    /// Only forward coverage may borrow it, after ordinary admission finishes.
    pub forward_limit: Option<(usize, u64)>,
}

impl SurfaceBudget {
    /// Compatibility policy for adopting the shared scheduler before calibration.
    pub fn count_only(surfaces: usize) -> Self {
        Self {
            surfaces,
            backing_pixels: u64::MAX,
            optional_pixels: u64::MAX,
            forward_limit: None,
        }
    }
}

/// Physical residency policy. The viewport allowance limits speculation, not
/// mandatory composition. Chassis report unreleased backings/reservations that
/// are not already represented by candidates (including unknown preserved plans).
#[derive(Clone, Copy, Debug)]
pub struct ResidencyPolicy {
    pub backing_pixels: u64,
    pub optional_viewports: f64,
}

impl Default for ResidencyPolicy {
    fn default() -> Self {
        Self {
            backing_pixels: 64 * 1024 * 1024,
            optional_viewports: 2.0,
        }
    }
}

impl ResidencyPolicy {
    pub fn available(
        self,
        surfaces: usize,
        viewport_pixels: u64,
        unreleased_pixels: u64,
    ) -> SurfaceBudget {
        let backing_pixels = self.backing_pixels.saturating_sub(unreleased_pixels);
        SurfaceBudget {
            surfaces,
            backing_pixels,
            optional_pixels: ((viewport_pixels as f64 * self.optional_viewports) as u64)
                .min(backing_pixels),
            // Keep one screen of pixels and four slots free for new visible
            // demand. These are borrowing limits, never limits on visibility.
            forward_limit: Some((
                surfaces.saturating_sub(4),
                backing_pixels.saturating_sub(viewport_pixels),
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenialReason {
    VisibleCapacity,
    PreparationCapacity,
    BackingPixels,
    OptionalPixels,
    OutsidePreparation,
}

#[derive(Debug, Default)]
pub struct SurfaceSchedule {
    /// Indices into the immutable input snapshot, in preparation order.
    pub admitted: Vec<usize>,
    pub denied: Vec<(usize, DenialReason)>,
    pub overloaded: bool,
    pub backing_pixels: u64,
    pub optional_pixels: u64,
    pub borrowed_forward_pixels: u64,
}

fn distance(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        f64::MAX
    }
}

/// Visible coverage precedes all speculation. Reverse-buffer residency is
/// funded next, before forward work can consume the optional allowance.
/// Execution still follows coverage/deadline order. Equal-ranked branches
/// interleave; visible survivors win genuine overload without rotating evictions.
pub fn schedule_surfaces(
    candidates: &[SurfaceCandidate],
    budget: SurfaceBudget,
) -> SurfaceSchedule {
    let mut required_count = 0usize;
    let mut required_pixels = 0u64;
    for c in candidates
        .iter()
        .filter(|c| c.coverage == Coverage::Visible)
    {
        required_count += 1;
        required_pixels = required_pixels.saturating_add(c.backing_pixels);
    }
    let overloaded = required_count > budget.surfaces || required_pixels > budget.backing_pixels;
    let compare = |a: &SurfaceCandidate, b: &SurfaceCandidate| {
        a.coverage
            .cmp(&b.coverage)
            .then_with(|| {
                if overloaded {
                    b.retained.cmp(&a.retained)
                } else {
                    Ordering::Equal
                }
            })
            .then_with(|| distance(a.distance).total_cmp(&distance(b.distance)))
            .then_with(|| b.ready.cmp(&a.ready))
    };
    let mut sorted: Vec<_> = (0..candidates.len()).collect();
    sorted.sort_by(|&a, &b| {
        compare(&candidates[a], &candidates[b])
            .then_with(|| candidates[a].branch.cmp(&candidates[b].branch))
            .then_with(|| candidates[a].id.cmp(&candidates[b].id))
    });
    let mut ordered = Vec::with_capacity(sorted.len());
    let mut start = 0;
    while start < sorted.len() {
        let mut end = start + 1;
        while end < sorted.len()
            && compare(&candidates[sorted[start]], &candidates[sorted[end]]) == Ordering::Equal
        {
            end += 1;
        }
        let mut branches: BTreeMap<i64, VecDeque<usize>> = BTreeMap::new();
        for &index in &sorted[start..end] {
            branches
                .entry(candidates[index].branch)
                .or_default()
                .push_back(index);
        }
        while !branches.is_empty() {
            branches.retain(|_, entries| {
                if let Some(index) = entries.pop_front() {
                    ordered.push(index);
                }
                !entries.is_empty()
            });
        }
        start = end;
    }
    let mut out = SurfaceSchedule {
        overloaded,
        ..Default::default()
    };
    let mut admission_order = ordered.clone();
    // Residency and execution answer different questions: protect a small
    // reversible footprint without drawing trailing tiles before incoming ones.
    // Stable sorting preserves distance and sibling fairness within each tier.
    admission_order.sort_by_key(|&i| match candidates[i].coverage {
        Coverage::Visible => 0,
        Coverage::ReversalNear => 1,
        _ => 2,
    });
    for &index in &admission_order {
        let c = &candidates[index];
        let visible = c.coverage == Coverage::Visible;
        let reason = if c.coverage == Coverage::OutsidePreparation {
            Some(DenialReason::OutsidePreparation)
        } else if out.admitted.len() >= budget.surfaces || (overloaded && !visible) {
            Some(if visible {
                DenialReason::VisibleCapacity
            } else {
                DenialReason::PreparationCapacity
            })
        } else if c.backing_pixels > budget.backing_pixels.saturating_sub(out.backing_pixels) {
            Some(DenialReason::BackingPixels)
        } else if !visible
            && c.backing_pixels > budget.optional_pixels.saturating_sub(out.optional_pixels)
        {
            Some(DenialReason::OptionalPixels)
        } else {
            None
        };
        if let Some(reason) = reason {
            out.denied.push((index, reason));
        } else {
            out.admitted.push(index);
            out.backing_pixels = out.backing_pixels.saturating_add(c.backing_pixels);
            if !visible {
                out.optional_pixels = out.optional_pixels.saturating_add(c.backing_pixels);
            }
        }
    }
    // First preserve the ordinary near/safety allowance. Letting forward work
    // borrow inline would spend that allowance before stationary neighbors get
    // their first-interaction safety coverage.
    if let Some((surfaces, pixels)) = budget.forward_limit.filter(|_| !overloaded) {
        out.denied.retain(|&(index, reason)| {
            let c = &candidates[index];
            let forward = matches!(c.coverage, Coverage::MovingAhead | Coverage::ForwardReserve);
            if reason != DenialReason::OptionalPixels
                || !forward
                || out.admitted.len() >= surfaces.min(budget.surfaces)
                || c.backing_pixels
                    > pixels
                        .min(budget.backing_pixels)
                        .saturating_sub(out.backing_pixels)
            {
                return true;
            }
            out.admitted.push(index);
            out.backing_pixels += c.backing_pixels;
            out.optional_pixels += c.backing_pixels;
            out.borrowed_forward_pixels += c.backing_pixels;
            false
        });
    }
    // Residency protection and borrowing change membership, not execution rank.
    let admitted: HashSet<_> = out.admitted.iter().copied().collect();
    out.admitted = ordered
        .into_iter()
        .filter(|i| admitted.contains(i))
        .collect();
    out
}

/// Raster density for the local affine basis, not the rotated bounding box.
/// Orthogonal axes retain their separate scales; shear uses a conservative
/// singular-value bound. Pure presentation rotation cannot change the result.
pub fn projected_density(basis: [f64; 4], display_scale: f64) -> Option<[f64; 2]> {
    if !basis.iter().all(|v| v.is_finite()) || !display_scale.is_finite() || display_scale <= 0.0 {
        return None;
    }
    let [a, b, c, d] = basis;
    let xx = a * a + b * b;
    let yy = c * c + d * d;
    let dot = a * c + b * d;
    if (a * d - b * c).abs() <= 1e-12 {
        return None;
    }
    let scale = if dot.abs() <= 1e-9 * (xx * yy).sqrt() {
        [xx.sqrt(), yy.sqrt()]
    } else {
        let maximum = ((xx + yy + ((xx - yy).powi(2) + 4.0 * dot * dot).sqrt()) * 0.5).sqrt();
        [maximum, maximum]
    };
    let density = scale.map(|s| s * display_scale);
    density
        .iter()
        .all(|v| v.is_finite() && *v > 0.0)
        .then_some(density)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(left: f64, top: f64, right: f64, bottom: f64) -> CoverageRect {
        CoverageRect {
            left,
            top,
            right,
            bottom,
        }
    }

    fn host(id: u32, bounds: CoverageRect, viewport: [f64; 2], basis: [f64; 4]) -> HostObservation {
        HostObservation {
            id,
            parent: None,
            bounds,
            visible: bounds,
            viewport,
            offset: [0.0; 2],
            basis,
            active: true,
        }
    }

    #[test]
    fn scaled_nested_tiles_and_scroll_offsets_share_one_basis() {
        let mut h = host(
            1,
            rect(100.0, 50.0, 600.0, 400.0),
            [1000.0, 700.0],
            [0.5, 0.0, 0.0, 0.5],
        );
        h.offset = [300.0, 100.0];
        assert_eq!(
            project_surface(rect(400.0, 200.0, 600.0, 300.0), &h),
            rect(150.0, 100.0, 250.0, 150.0)
        );
    }

    #[test]
    fn rotation_and_reflection_do_not_treat_the_bounding_box_as_scale() {
        let rotated = host(
            1,
            rect(10.0, 20.0, 110.0, 220.0),
            [200.0, 100.0],
            [0.0, 1.0, -1.0, 0.0],
        );
        assert_eq!(
            project_surface(rect(50.0, 0.0, 100.0, 40.0), &rotated),
            rect(70.0, 70.0, 110.0, 120.0)
        );
        let reflected = host(
            1,
            rect(10.0, 20.0, 210.0, 120.0),
            [200.0, 100.0],
            [-1.0, 0.0, 0.0, 1.0],
        );
        assert_eq!(
            project_surface(rect(0.0, 0.0, 50.0, 40.0), &reflected),
            rect(160.0, 20.0, 210.0, 60.0)
        );
    }

    #[test]
    fn parent_motion_prioritizes_exposure_without_claiming_child_axis_motion() {
        let parent = host(
            1,
            rect(0.0, 0.0, 100.0, 100.0),
            [100.0; 2],
            [1.0, 0.0, 0.0, 1.0],
        );
        let mut child = host(
            2,
            rect(0.0, 200.0, 100.0, 300.0),
            [100.0; 2],
            [1.0, 0.0, 0.0, 1.0],
        );
        child.parent = Some(1);
        child.visible = CoverageRect::default();
        child.active = false;
        let hosts = BTreeMap::from([(1, parent), (2, child)]);
        let mut activity = ScrollActivityTracker::default();
        activity.observe(1, [0.0, 0.0], 0.0, "same");
        activity.observe(1, [0.0, 50.0], 20.0, "same");
        let mut c = candidate("child", 2, Coverage::Visible);
        rank_surface(
            &mut c,
            rect(0.0, 0.0, 100.0, 100.0),
            Some(2),
            &hosts,
            &activity,
            20.0,
        );
        assert_eq!(c.coverage, Coverage::MovingAhead);
        assert_eq!(activity.get(2, 20.0).direction, [0.0; 2]);
        rank_surface(
            &mut c,
            rect(0.0, 0.0, 100.0, 100.0),
            Some(2),
            &hosts,
            &activity,
            300.0,
        );
        assert_eq!(c.coverage, Coverage::FartherPreparation);
    }

    fn candidate(id: &str, branch: i64, coverage: Coverage) -> SurfaceCandidate {
        SurfaceCandidate {
            id: id.into(),
            branch,
            coverage,
            distance: 0.0,
            retained: false,
            ready: false,
            backing_pixels: 20_000,
        }
    }

    #[test]
    fn globally_visible_siblings_beat_all_speculation() {
        let mut input = vec![candidate("a", 1, Coverage::Visible)];
        input.extend((0..64).map(|i| candidate(&format!("warm-{i}"), 1, Coverage::MovingAhead)));
        input.push(candidate("b", 2, Coverage::Visible));
        input.push(candidate("root", 0, Coverage::Visible));
        let out = schedule_surfaces(&input, SurfaceBudget::count_only(4));
        assert_eq!(&out.admitted[..3], &[66, 0, 65]);
        assert_eq!(out.backing_pixels, 80_000);
        assert!(!out.overloaded);
    }

    #[test]
    fn overload_preserves_visible_survivors() {
        let mut input = vec![
            candidate("a1", 1, Coverage::Visible),
            candidate("a2", 1, Coverage::Visible),
            candidate("b1", 2, Coverage::Visible),
            candidate("b2", 2, Coverage::Visible),
        ];
        let out = schedule_surfaces(&input, SurfaceBudget::count_only(2));
        assert_eq!(out.admitted, [0, 2]);
        for i in out.admitted {
            input[i].retained = true;
        }
        input.reverse();
        let out = schedule_surfaces(&input, SurfaceBudget::count_only(2));
        assert_eq!(
            out.admitted
                .iter()
                .map(|i| input[*i].id.as_str())
                .collect::<Vec<_>>(),
            ["a1", "b1"]
        );
    }

    #[test]
    fn pixel_budget_counts_area_and_skips_unaffordable_optional_work() {
        let mut input = vec![
            candidate("visible", 1, Coverage::Visible),
            candidate("large", 1, Coverage::MovingAhead),
            candidate("small", 2, Coverage::MovingAhead),
        ];
        input[1].backing_pixels = 1_000_000;
        let out = schedule_surfaces(
            &input,
            SurfaceBudget {
                surfaces: 64,
                backing_pixels: 60_000,
                optional_pixels: 20_000,
                forward_limit: None,
            },
        );
        assert_eq!(out.admitted, [0, 2]);
        assert_eq!(out.denied, [(1, DenialReason::BackingPixels)]);
        assert_eq!(out.optional_pixels, 20_000);
    }

    #[test]
    fn visible_pixel_overload_preserves_survivors_and_rejects_speculation() {
        let mut input = vec![
            candidate("new", 1, Coverage::Visible),
            candidate("retained", 2, Coverage::Visible),
            candidate("warm", 1, Coverage::MovingAhead),
        ];
        input[1].retained = true;
        let result = schedule_surfaces(
            &input,
            SurfaceBudget {
                surfaces: 64,
                backing_pixels: 20_000,
                optional_pixels: 20_000,
                forward_limit: None,
            },
        );
        assert!(result.overloaded);
        assert_eq!(result.admitted, [1]);
    }

    #[test]
    fn activity_tracks_local_axes_reversal_expiry_and_discontinuities() {
        let mut t = ScrollActivityTracker::default();
        t.observe(1, [0.0, 0.0], 0.0, "a");
        t.observe(2, [0.0, 0.0], 0.0, "a");
        t.observe(1, [0.0, 100.0], 20.0, "a");
        assert_eq!(t.get(1, 20.0).direction, [0.0, 1.0]);
        assert_eq!(t.get(2, 20.0).direction, [0.0, 0.0]);
        t.observe(1, [0.0, 99.0], 30.0, "a");
        assert_eq!(t.get(1, 30.0).direction[1], -1.0);
        t.observe(2, [30.0, 0.0], 40.0, "a");
        assert_eq!(t.get(1, 281.0).direction[1], 0.0);
        assert_eq!(t.get(2, 281.0).direction[0], 1.0);
        t.observe(2, [40.0, 0.0], 300.0, "b");
        assert_eq!(t.get(2, 300.0).direction[0], 0.0);
        t.observe(1, [0.0, 400.0], 5000.0, "a");
        assert_eq!(t.get(1, 5000.0).direction[1], 0.0);
    }

    #[test]
    fn unchanged_idle_sampling_gap_preserves_only_opportunistic_direction() {
        let mut t = ScrollActivityTracker::default();
        t.observe(1, [0.0, 0.0], 0.0, "a");
        t.observe(1, [0.0, 100.0], 20.0, "a");
        t.observe(1, [0.0, 100.0], 5000.0, "a");
        assert_eq!(t.get(1, 5000.0).direction, [0.0; 2]);
        assert_eq!(t.recent(1, 5000.0, 1500.0).direction, [0.0; 2]);
        assert_eq!(t.recent(1, 5000.0, f64::INFINITY).direction, [0.0, 1.0]);
        t.observe(1, [0.0, 100.0], 5010.0, "resized");
        assert_eq!(t.recent(1, 5010.0, f64::INFINITY).direction, [0.0; 2]);
    }

    #[test]
    fn forward_borrowing_preserves_safety_and_restores_execution_order() {
        let input = vec![
            candidate("visible", 1, Coverage::Visible),
            candidate("near", 1, Coverage::MovingAhead),
            candidate("neighbor", 2, Coverage::StationaryNear),
            candidate("forward", 1, Coverage::ForwardReserve),
            candidate("retained", 2, Coverage::FartherPreparation),
        ];
        let result = schedule_surfaces(
            &input,
            SurfaceBudget {
                surfaces: 8,
                backing_pixels: 200_000,
                optional_pixels: 40_000,
                forward_limit: Some((4, 180_000)),
            },
        );
        assert_eq!(result.admitted, [0, 1, 2, 3]);
        assert_eq!(result.borrowed_forward_pixels, 20_000);
        assert_eq!(result.denied, [(4, DenialReason::OptionalPixels)]);
        // A forward tile too large for the base allowance may borrow later,
        // but must still execute ahead of the reserve and stationary safety.
        let result = schedule_surfaces(
            &input,
            SurfaceBudget {
                surfaces: 8,
                backing_pixels: 200_000,
                optional_pixels: 0,
                forward_limit: Some((4, 180_000)),
            },
        );
        assert_eq!(result.admitted, [0, 1, 3]);
    }

    #[test]
    fn reverse_residency_precedes_forward_but_execution_remains_incoming_first() {
        let mut input = vec![
            candidate("visible", 1, Coverage::Visible),
            candidate("incoming", 1, Coverage::MovingAhead),
            candidate("reverse", 1, Coverage::ReversalNear),
            candidate("deep-forward", 1, Coverage::ForwardReserve),
        ];
        let budget = SurfaceBudget {
            surfaces: 4,
            backing_pixels: 80_000,
            optional_pixels: 20_000,
            forward_limit: None,
        };
        let protected = schedule_surfaces(&input, budget);
        assert_eq!(protected.admitted, [0, 2]);
        let borrowed = schedule_surfaces(
            &input,
            SurfaceBudget {
                forward_limit: Some((3, 60_000)),
                ..budget
            },
        );
        assert_eq!(borrowed.admitted, [0, 1, 2]);
        assert_eq!(borrowed.borrowed_forward_pixels, 20_000);
        assert_eq!(borrowed.backing_pixels, 60_000);
        // Reversal transfers membership and rank without a new identity or
        // permission for distant old-direction work to keep its former rank.
        input[1].coverage = Coverage::OutsidePreparation;
        input[2].coverage = Coverage::MovingAhead;
        input[3].coverage = Coverage::ReversalNear;
        let reversed = schedule_surfaces(
            &input,
            SurfaceBudget {
                forward_limit: Some((3, 60_000)),
                ..budget
            },
        );
        assert_eq!(reversed.admitted, [0, 2, 3]);
        assert_eq!(reversed.denied, [(1, DenialReason::OutsidePreparation)]);
        // Mandatory demand always wins even when no reverse buffer fits.
        input[1].coverage = Coverage::Visible;
        let exhausted = schedule_surfaces(
            &input,
            SurfaceBudget {
                surfaces: 2,
                ..budget
            },
        );
        assert_eq!(exhausted.admitted, [1, 0]);
    }

    #[test]
    fn borrowed_forward_respects_visible_headroom_and_retires_on_new_demand() {
        let mut input = vec![candidate("visible", 1, Coverage::Visible)];
        input.extend(
            (0..10).map(|i| candidate(&format!("forward-{i}"), 1, Coverage::ForwardReserve)),
        );
        let budget = SurfaceBudget {
            surfaces: 8,
            backing_pixels: 200_000,
            optional_pixels: 20_000,
            forward_limit: Some((4, 100_000)),
        };
        let result = schedule_surfaces(&input, budget);
        assert_eq!(result.admitted.len(), 4);
        assert_eq!(result.backing_pixels, 80_000);
        // New visible work displaces speculation without increasing either cap.
        input.extend((0..3).map(|i| candidate(&format!("visible-{i}"), 2, Coverage::Visible)));
        let result = schedule_surfaces(&input, budget);
        assert_eq!(result.admitted.len(), 5); // visible + ordinary near allowance
        assert_eq!(result.borrowed_forward_pixels, 0);
        assert!(result
            .admitted
            .iter()
            .take(4)
            .all(|&i| input[i].coverage == Coverage::Visible));
        let b = ResidencyPolicy::default().available(64, 4_000_000, 10_000_000);
        assert_eq!(b.forward_limit, Some((60, 64 * 1024 * 1024 - 14_000_000)));
    }

    #[test]
    fn raster_density_matches_scaled_rotated_and_sheared_screen_basis() {
        assert_eq!(
            projected_density([0.5, 0.0, 0.0, 0.5], 2.0),
            Some([1.0, 1.0])
        );
        assert_eq!(
            projected_density([0.0, 0.5, -2.0, 0.0], 2.0),
            Some([1.0, 4.0])
        );
        assert_eq!(
            projected_density([2.0, 0.0, 0.0, 2.0], 2.0),
            Some([4.0, 4.0])
        );
        let shear = projected_density([1.0, 0.0, 1.0, 1.0], 2.0).unwrap();
        assert!((shear[0] - 3.2360679775).abs() < 1e-8);
        assert_eq!(shear[0], shear[1]);
        assert!(projected_density([0.0; 4], 2.0).is_none());
    }
}
