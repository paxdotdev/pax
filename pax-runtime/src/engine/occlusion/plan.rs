//! Pure native-mask plans. Dependencies are compared before projection, clipping
//! and SVG serialization; opacity is deliberately outside the geometry key.
use kurbo::{Affine, BezPath, Rect, Shape};
use pax_message::MaskPathPatch;
use pax_runtime_api::{bez_path_to_svg_path_data, math::ProjectedPlane, DepthPlane};
use std::{collections::HashMap, rc::Rc};

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Mapping {
    Affine(Affine),
    Projected(ProjectedPlane, Option<Rect>),
    Planar(pax_runtime_api::math::PlanarProjection),
}
impl Mapping {
    fn apply(&self, path: &BezPath) -> BezPath {
        match self {
            Self::Affine(m) => *m * path.clone(),
            Self::Projected(m, viewport) => m.project_path(path, *viewport),
            Self::Planar(m) => m.project_path(path, 0.1),
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct PathInput {
    pub path: Rc<BezPath>,
    pub mapping: Mapping,
}
impl PartialEq for PathInput {
    fn eq(&self, other: &Self) -> bool {
        self.mapping == other.mapping
            && (Rc::ptr_eq(&self.path, &other.path) || self.path == other.path)
    }
}
impl PathInput {
    fn resolve(&self) -> BezPath {
        self.mapping.apply(&self.path)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Contribution {
    pub owner: (u32, u64),
    pub path: PathInput,
    pub clips: Vec<((u32, u64), PathInput)>,
    pub depth: Option<(DepthPlane, DepthPlane, i32, i32)>,
    pub opacity: f64,
}
impl Contribution {
    fn same_geometry(&self, other: &Self) -> bool {
        self.owner == other.owner
            && self.path == other.path
            && self.clips == other.clips
            && self.depth == other.depth
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MaskPlan {
    pub(super) size: (f64, f64),
    pub(super) bounds: Rect,
    pub(super) inverse: Mapping,
    pub(super) camera: Option<Option<ProjectedPlane>>,
    pub(super) contributions: Vec<Contribution>,
}

#[derive(Clone)]
pub(crate) struct MaskOutput {
    pub entries: Rc<Vec<MaskPathPatch>>,
    pub hash: u64,
}
struct RetainedPlan {
    input: MaskPlan,
    mapped: Vec<Option<MaskPathPatch>>,
    output: MaskOutput,
    used: u64,
    bytes: usize,
}

#[derive(Clone, Copy, Default, Debug)]
pub(crate) struct MaskPlanStats {
    pub reuses: u64,
    pub mappings: u64,
    pub serializations: u64,
    pub evictions: u64,
    pub oversized: u64,
}

pub(crate) struct MaskPlans {
    records: HashMap<(u32, u64), RetainedPlan>,
    bytes: usize,
    clock: u64,
    max_entries: usize,
    max_bytes: usize,
    pub stats: MaskPlanStats,
}

impl Default for MaskPlans {
    fn default() -> Self {
        Self::with_budget(2048, 4 * 1024 * 1024)
    }
}

impl MaskPlans {
    fn with_budget(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            records: HashMap::new(),
            bytes: 0,
            clock: 0,
            max_entries,
            max_bytes,
            stats: Default::default(),
        }
    }
    pub fn residency(&self) -> (usize, usize) {
        (self.records.len(), self.bytes)
    }
    pub fn prepare(&mut self, receiver: (u32, u64), mut input: MaskPlan) -> MaskOutput {
        input.contributions.sort_by_key(|entry| entry.owner);
        self.clock += 1;
        if let Some(old) = self.records.get_mut(&receiver) {
            if old.input == input {
                self.stats.reuses += 1;
                old.used = self.clock;
                return old.output.clone();
            }
        }
        let old = self.records.remove(&receiver);
        if let Some(old) = &old {
            self.bytes -= old.bytes;
        }
        let same_chart = old.as_ref().is_some_and(|old| {
            old.input.inverse == input.inverse && old.input.bounds == input.bounds
        });
        let mapped: Vec<_> = input
            .contributions
            .iter()
            .map(|entry| {
                if same_chart {
                    let old = old.as_ref().unwrap();
                    if let Ok(i) = old
                        .input
                        .contributions
                        .binary_search_by_key(&entry.owner, |e| e.owner)
                    {
                        if entry.same_geometry(&old.input.contributions[i]) {
                            return old.mapped[i].clone().map(|mut patch| {
                                patch.opacity = Some(entry.opacity);
                                patch
                            });
                        }
                    }
                }
                self.stats.mappings += 1;
                materialize(entry, &input, &mut self.stats)
            })
            .collect();
        let mut entries: Vec<_> = mapped.iter().flatten().cloned().collect();
        super::sort_mask_entries(&mut entries);
        // The camera aperture is geometry too. Reuse its serialized contribution
        // when only coverage weights change.
        let camera = if old
            .as_ref()
            .is_some_and(|old| old.input.camera == input.camera && old.input.size == input.size)
        {
            let old = old.as_ref().unwrap();
            (old.output.entries.len() > old.mapped.iter().flatten().count())
                .then(|| old.output.entries.last().unwrap().clone())
        } else {
            camera_entry(input.camera, input.size).map(|patch| {
                self.stats.serializations += 1;
                patch
            })
        };
        entries.extend(camera);
        let hash = if entries.is_empty() {
            0
        } else {
            super::hash_mask_entries(input.size, &entries)
        };
        let output = MaskOutput {
            entries: Rc::new(entries),
            hash,
        };
        let bytes = retained_bytes(&input, &mapped, mapped.capacity(), &output);
        // Half the shared auxiliary allowance is reserved for plans, half for
        // solid/support relations: never more than 4096 entries / 8 MiB total.
        if bytes > self.max_bytes || self.max_entries == 0 {
            self.stats.oversized += 1;
            return output;
        }
        while self.records.len() >= self.max_entries || self.bytes + bytes > self.max_bytes {
            let key = *self
                .records
                .iter()
                .min_by_key(|(_, value)| value.used)
                .unwrap()
                .0;
            self.bytes -= self.records.remove(&key).unwrap().bytes;
            self.stats.evictions += 1;
        }
        self.bytes += bytes;
        self.records.insert(
            receiver,
            RetainedPlan {
                input,
                mapped,
                output: output.clone(),
                used: self.clock,
                bytes,
            },
        );
        output
    }

    pub fn remove(&mut self, node: u32) {
        self.records.retain(|key, plan| {
            let remove = key.0 == node
                || plan.input.contributions.iter().any(|entry| {
                    entry.owner.0 == node || entry.clips.iter().any(|(owner, _)| owner.0 == node)
                });
            if remove {
                self.bytes -= plan.bytes;
            }
            !remove
        });
    }
}

fn materialize(
    entry: &Contribution,
    plan: &MaskPlan,
    stats: &mut MaskPlanStats,
) -> Option<MaskPathPatch> {
    let path = entry.path.resolve();
    if path.elements().is_empty() {
        return None;
    }
    let mut clips: Vec<_> = entry.clips.iter().map(|(_, clip)| clip.resolve()).collect();
    if let Some((front, back, front_order, back_order)) = entry.depth {
        clips.push(DepthPlane::front_region(
            front,
            back,
            front_order,
            back_order,
            path.bounding_box().intersect(plan.bounds),
        )?);
    }
    if matches!(plan.inverse, Mapping::Affine(_)) {
        let bounds = super::OcclusionBox::new_from_path(&path)?;
        if !super::clipped_coverage_bounds(bounds, &clips).is_some_and(|bounds| {
            bounds.intersects(&super::OcclusionBox {
                x1: plan.bounds.x0,
                y1: plan.bounds.y0,
                x2: plan.bounds.x1,
                y2: plan.bounds.y1,
            })
        }) {
            return None;
        }
    }
    let path = plan.inverse.apply(&path);
    let clips: Vec<_> = clips.iter().map(|clip| plan.inverse.apply(clip)).collect();
    if path.elements().is_empty() || clips.iter().any(|clip| clip.elements().is_empty()) {
        return None;
    }
    stats.serializations += 1 + clips.len() as u64;
    Some(MaskPathPatch {
        path: bez_path_to_svg_path_data(&path),
        clips: clips.iter().map(bez_path_to_svg_path_data).collect(),
        opacity: Some(entry.opacity),
    })
}

pub(super) fn camera_entry(
    projection: Option<Option<ProjectedPlane>>,
    size: (f64, f64),
) -> Option<MaskPathPatch> {
    let projection = projection?;
    let rect = Rect::new(0., 0., size.0, size.1);
    let corners = vec![
        rect.origin(),
        (rect.x1, rect.y0).into(),
        (rect.x1, rect.y1).into(),
        (rect.x0, rect.y1).into(),
    ];
    let visible = projection
        .map(|p| p.clip_polygon(corners.clone(), None))
        .unwrap_or_default();
    if visible == corners {
        return None;
    }
    let mut path = rect.to_path(0.1);
    if let Some(last) = visible.last() {
        path.move_to(*last);
        for p in visible.iter().rev().skip(1) {
            path.line_to(*p);
        }
        path.close_path();
    }
    Some(MaskPathPatch {
        path: bez_path_to_svg_path_data(&path),
        clips: vec![],
        opacity: Some(1.),
    })
}

fn retained_bytes(
    input: &MaskPlan,
    mapped: &[Option<MaskPathPatch>],
    mapped_capacity: usize,
    output: &MaskOutput,
) -> usize {
    // Count shared paths conservatively for each plan retaining them. Source
    // buffers can have spare capacity; charging the next power of two doubled
    // also covers their allocation headers. Wire strings are owned twice here.
    let path_bytes = |p: &PathInput| {
        (p.path.elements().len() + 1).next_power_of_two() * 2 * std::mem::size_of::<kurbo::PathEl>()
            + std::mem::size_of::<PathInput>()
    };
    let patch_bytes = |p: &MaskPathPatch| {
        p.path.capacity()
            + p.clips.capacity() * std::mem::size_of::<String>()
            + p.clips.iter().map(String::capacity).sum::<usize>()
    };
    std::mem::size_of::<RetainedPlan>()
        + input.contributions.capacity() * std::mem::size_of::<Contribution>()
        + input
            .contributions
            .iter()
            .map(|e| {
                path_bytes(&e.path)
                    + e.clips.capacity() * std::mem::size_of::<((u32, u64), PathInput)>()
                    + e.clips.iter().map(|(_, p)| path_bytes(p)).sum::<usize>()
            })
            .sum::<usize>()
        + mapped.iter().flatten().map(&patch_bytes).sum::<usize>()
        + mapped_capacity * std::mem::size_of::<Option<MaskPathPatch>>()
        + output.entries.capacity() * std::mem::size_of::<MaskPathPatch>()
        + output.entries.iter().map(patch_bytes).sum::<usize>()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(projective: bool) -> MaskPlan {
        let bounds = Rect::new(0., 0., 100., 60.);
        MaskPlan {
            size: (100., 60.),
            bounds,
            inverse: if projective {
                Mapping::Projected(
                    ProjectedPlane::affine(Affine::IDENTITY),
                    Some(bounds.inflate(1., 1.)),
                )
            } else {
                Mapping::Affine(Affine::IDENTITY)
            },
            camera: None,
            contributions: vec![Contribution {
                owner: (8, 1),
                path: PathInput {
                    path: Rc::new(bounds.to_path(0.1)),
                    mapping: Mapping::Affine(Affine::IDENTITY),
                },
                clips: vec![(
                    (9, 1),
                    PathInput {
                        path: Rc::new(Rect::new(10., 0., 50., 60.).to_path(0.1)),
                        mapping: Mapping::Affine(Affine::IDENTITY),
                    },
                )],
                depth: None,
                opacity: 1.,
            }],
        }
    }

    #[test]
    fn unchanged_and_opacity_only_plans_skip_mapping_and_serialization() {
        for projective in [false, true] {
            let mut cache = MaskPlans::default();
            let mut plan = fixture(projective);
            let first = cache.prepare((1, 1), plan.clone());
            let work = cache.stats;
            assert!(Rc::ptr_eq(
                &first.entries,
                &cache.prepare((1, 1), plan.clone()).entries
            ));
            assert_eq!(cache.stats.mappings, work.mappings);
            plan.contributions[0].opacity = 0.35;
            let faded = cache.prepare((1, 1), plan.clone());
            assert_ne!(first.hash, faded.hash);
            assert_eq!(first.entries[0].path, faded.entries[0].path);
            assert_eq!(faded.entries[0].opacity, Some(0.35));
            assert_eq!(cache.stats.mappings, work.mappings);
            assert_eq!(cache.stats.serializations, work.serializations);
            plan.contributions[0].clips[0].1.mapping = Mapping::Affine(Affine::translate((5., 0.)));
            let clipped = cache.prepare((1, 1), plan.clone());
            assert_ne!(faded.hash, clipped.hash);
            assert_eq!(cache.stats.mappings, work.mappings + 1);
            assert!(cache.stats.serializations > work.serializations);
            cache.prepare((1, 2), plan.clone());
            assert_eq!(
                cache.stats.mappings,
                work.mappings + 2,
                "mount epoch cannot reuse a retired plan"
            );
            cache.remove(9);
            assert_eq!(
                cache.residency(),
                (0, 0),
                "clip retirement releases dependents"
            );
        }
    }

    #[test]
    fn projective_plans_match_the_previous_mapping_oracle_including_empty_clips() {
        let mut cache = MaskPlans::default();
        let mut plan = fixture(true);
        let bounds = plan.bounds;
        let mapping =
            pax_runtime_api::math::PlanarProjection([1., 0.1, 0.002, -0.1, 1., 0.001, 3., 7., 1.]);
        let inverse =
            ProjectedPlane::affine(Affine::IDENTITY).post_mapping(mapping.inverse().unwrap());
        plan.inverse = Mapping::Projected(inverse, Some(bounds.inflate(1., 1.)));
        for offset in [0., 25., 100., 300., 0.] {
            let clip = Rect::new(offset, 0., offset + 30., 60.).to_path(0.1);
            plan.contributions[0].clips[0].1.path = Rc::new(clip.clone());
            let expected = super::super::project_mask_entry(
                &plan.contributions[0].path.path,
                &[clip],
                inverse,
                bounds,
                1.,
            );
            let result = cache.prepare((1, 1), plan.clone());
            assert_eq!(
                result.entries.as_ref(),
                &expected.into_iter().collect::<Vec<_>>()
            );
        }
        let mut matrix =
            pax_runtime_api::math::Transform3::<pax_runtime_api::math::Generic>::identity()
                .coeffs();
        matrix[2] = 0.02;
        matrix[14] = -0.5;
        plan.camera = Some(ProjectedPlane::new(
            pax_runtime_api::math::Transform3::<pax_runtime_api::math::Generic>::new(matrix),
            true,
        ));
        let clipped = cache.prepare((1, 1), plan.clone());
        assert!(clipped.entries.last().unwrap().path.contains('L'));
        let work = cache.stats.serializations;
        plan.contributions[0].opacity = 0.4;
        cache.prepare((1, 1), plan.clone());
        assert_eq!(
            cache.stats.serializations, work,
            "camera aperture is unchanged by opacity"
        );
        plan.camera = Some(None);
        let hidden = cache.prepare((1, 1), plan);
        assert_ne!(
            hidden.hash, clipped.hash,
            "invalid camera masks the whole control"
        );
    }

    #[test]
    fn plan_budgets_and_retirement_preserve_exact_output() {
        let plan = fixture(false);
        let mut cache = MaskPlans::with_budget(2, 32768);
        let expected = cache.prepare((1, 1), plan.clone());
        for id in 2..30 {
            assert_eq!(cache.prepare((id, 1), plan.clone()).hash, expected.hash);
            assert!(cache.residency().0 <= 2 && cache.residency().1 <= 32768);
        }
        assert!(cache.stats.evictions > 0);
        assert_eq!(
            cache.prepare((1, 1), plan.clone()).entries,
            expected.entries
        );
        cache.remove(8);
        assert_eq!(cache.residency(), (0, 0));
        let mut tiny = MaskPlans::with_budget(1, 1);
        assert_eq!(tiny.prepare((1, 1), plan).entries, expected.entries);
        assert_eq!(tiny.residency(), (0, 0));
        assert_eq!(tiny.stats.oversized, 1);
    }
}
