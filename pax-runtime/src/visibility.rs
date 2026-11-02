//! Retained presentation facts shared by visibility consumers.
//!
//! Records contain geometry only: paint opacity, hit eligibility, native masking
//! and event dispatch remain consumer policy. The facade reads current computed
//! projection inputs before consulting this cache, including between display ticks.

use std::{collections::HashMap, rc::Rc};

use kurbo::Point;
use pax_runtime_api::{
    math::{PlanarProjection, ProjectedPlane},
    DepthPlane,
};

pub(crate) use pax_runtime_api::visibility::{RelationKey, Relations};

#[derive(Debug)]
pub(crate) struct PreparedPresentation {
    projection: Option<ProjectedPlane>,
    inverse: Option<PlanarProjection>,
    depth: Option<DepthPlane>,
}

impl PreparedPresentation {
    fn new(projection: Option<ProjectedPlane>) -> Self {
        Self {
            projection,
            inverse: projection.and_then(|p| p.mapping().inverse()),
            depth: projection.and_then(|p| p.depth_plane()),
        }
    }

    pub(crate) fn projection(&self) -> Option<ProjectedPlane> {
        self.projection
    }

    pub(crate) fn inverse_mapping(&self) -> Option<PlanarProjection> {
        self.inverse
    }

    pub(crate) fn depth_plane(&self) -> Option<DepthPlane> {
        self.depth
    }

    pub(crate) fn local_point(&self, point: Point) -> Option<Point> {
        let local = self.inverse?.project(point)?;
        self.projection?.contains_local(local).then_some(local)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PresentationStats {
    pub preparations: u64,
    pub reuses: u64,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct PresentationInputs {
    pub geometry: crate::SpatialGeometry,
    pub view: crate::camera::PresentationView,
    pub scroll: [f64; 3],
}
impl PresentationInputs {
    pub(crate) fn project(self) -> Option<ProjectedPlane> {
        use pax_runtime_api::math::{Transform3, Vector3};
        let [x, y, z] = self.scroll;
        self.view.plane(crate::SpatialGeometry::new(
            Transform3::translate(Vector3::new(x, y, z)) * self.geometry.world_transform(),
            self.geometry.bounds,
        ))
    }
}

#[derive(Default)]
pub(crate) struct Presentations {
    records: HashMap<u32, (u64, Option<PresentationInputs>, Rc<PreparedPresentation>)>,
    pub projections: u64,
    pub stats: PresentationStats,
}

impl Presentations {
    pub(crate) fn prepare_inputs(
        &mut self,
        node: u32,
        epoch: u64,
        inputs: PresentationInputs,
    ) -> Rc<PreparedPresentation> {
        if let Some((old_epoch, Some(old_inputs), record)) = self.records.get(&node) {
            if *old_epoch == epoch && *old_inputs == inputs {
                self.stats.reuses += 1;
                return record.clone();
            }
        }
        self.projections += 1;
        let prepared = self.prepare(node, epoch, inputs.project());
        self.records.get_mut(&node).unwrap().1 = Some(inputs);
        prepared
    }

    pub(crate) fn prepare(
        &mut self,
        node: u32,
        mount_epoch: u64,
        projection: Option<ProjectedPlane>,
    ) -> Rc<PreparedPresentation> {
        if let Some((epoch, _, old)) = self.records.get(&node) {
            if *epoch == mount_epoch && old.projection == projection {
                self.stats.reuses += 1;
                return old.clone();
            }
        }
        let record = Rc::new(PreparedPresentation::new(projection));
        self.records
            .insert(node, (mount_epoch, None, record.clone()));
        self.stats.preparations += 1;
        record
    }

    pub(crate) fn remove(&mut self, node: u32) {
        self.records.remove(&node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pax_runtime_api::math::{Generic, Transform3};

    #[test]
    fn presentation_inputs_reject_unchanged_work_before_projection() {
        let mut cache = Presentations::default();
        let mut inputs = PresentationInputs {
            geometry: crate::SpatialGeometry::new(Transform3::identity(), (100., 100.)),
            view: Default::default(),
            scroll: [0.; 3],
        };
        let first = cache.prepare_inputs(7, 1, inputs);
        for _ in 0..64 {
            assert!(Rc::ptr_eq(&first, &cache.prepare_inputs(7, 1, inputs)));
        }
        assert_eq!(cache.projections, 1);
        inputs.scroll[1] = -20.;
        let moved = cache.prepare_inputs(7, 1, inputs);
        assert!(!Rc::ptr_eq(&first, &moved));
        assert_eq!(cache.projections, 2);
        let local = moved.local_point(Point::new(10., 10.)).unwrap();
        assert!(local.distance(Point::new(10., 30.)) < 1e-9);
        assert!(!Rc::ptr_eq(&moved, &cache.prepare_inputs(7, 2, inputs)));
        assert_eq!(
            cache.records.len(),
            1,
            "only the current mount and placement are retained"
        );
    }

    #[test]
    fn presentation_cache_replaces_pose_and_mount_without_retaining_history() {
        let mut cache = Presentations::default();
        let flat = Some(ProjectedPlane::affine(kurbo::Affine::IDENTITY));
        let first = cache.prepare(7, 1, flat);
        let weak = Rc::downgrade(&first);
        assert!(Rc::ptr_eq(&first, &cache.prepare(7, 1, flat)));
        drop(first);
        let moved = cache.prepare(
            7,
            1,
            flat.map(|p| p.post_affine(kurbo::Affine::translate((5., 0.)))),
        );
        assert!(weak.upgrade().is_none());
        assert_eq!(
            moved.local_point(Point::new(15., 20.)),
            Some(Point::new(10., 20.))
        );
        let moved_weak = Rc::downgrade(&moved);
        let remounted = cache.prepare(7, 2, moved.projection());
        assert!(!Rc::ptr_eq(&moved, &remounted));
        drop(moved);
        assert!(moved_weak.upgrade().is_none());
        let remounted_weak = Rc::downgrade(&remounted);
        drop(remounted);
        cache.remove(7);
        assert!(remounted_weak.upgrade().is_none());
        assert!(cache.records.is_empty());
        assert_eq!(
            cache.stats,
            PresentationStats {
                preparations: 3,
                reuses: 1
            }
        );
    }

    #[test]
    fn cached_inverse_preserves_camera_clipping_and_invalid_recovery() {
        let mut cache = Presentations::default();
        let mut matrix = Transform3::<Generic>::identity().coeffs();
        matrix[2] = 0.02;
        matrix[14] = -0.5;
        let projection = ProjectedPlane::new(Transform3::<Generic>::new(matrix), true).unwrap();
        let record = cache.prepare(1, 1, Some(projection));
        for x in [-10., 0., 24., 25., 50., 75., 76., 100.] {
            let point = Point::new(x, 10.);
            assert_eq!(record.local_point(point), projection.local_point(point));
        }
        assert_eq!(record.depth_plane(), projection.depth_plane());
        let invalid = cache.prepare(1, 1, None);
        assert!(invalid.local_point(Point::ZERO).is_none());
        let collapsed = cache.prepare(
            1,
            1,
            Some(ProjectedPlane::affine(kurbo::Affine::scale_non_uniform(
                0., 1.,
            ))),
        );
        assert!(collapsed.local_point(Point::ZERO).is_none());
        let restored = cache.prepare(1, 1, Some(projection));
        assert_eq!(
            restored.local_point(Point::new(50., 10.)),
            Some(Point::new(50., 10.))
        );
        assert_eq!(cache.records.len(), 1);
    }

    #[test]
    fn destination_chart_changes_preserve_original_camera_clipping() {
        let mut cache = Presentations::default();
        let mut matrix = Transform3::<Generic>::identity().coeffs();
        matrix[2] = 0.02;
        matrix[14] = -0.5;
        let camera = ProjectedPlane::new(Transform3::<Generic>::new(matrix), true).unwrap();
        let chart = PlanarProjection([1., 0., 0.001, 0., 1., 0.002, 10., -5., 1.]);
        let rebased = camera.post_mapping(chart);
        let record = cache.prepare(1, 1, Some(rebased));
        assert_eq!(rebased.camera_clip_rows(), camera.camera_clip_rows());
        for x in [0., 24., 26., 50., 74., 76., 100.] {
            let local = Point::new(x, 10.);
            let point = rebased.mapping().project(local).unwrap();
            let actual = record.local_point(point);
            assert_eq!(actual.is_some(), camera.contains_local(local));
            if let Some(actual) = actual {
                assert!((actual - local).hypot() < 1e-10);
            }
        }
        // Identical destination matrices can still have different camera clip
        // equations: rebuilding from the destination must not alias this record.
        let wrong_clip = ProjectedPlane::new(rebased.matrix(), true).unwrap();
        assert_ne!(rebased, wrong_clip);
        let other = cache.prepare(1, 1, Some(wrong_clip));
        assert!(!Rc::ptr_eq(&record, &other));
    }
}
