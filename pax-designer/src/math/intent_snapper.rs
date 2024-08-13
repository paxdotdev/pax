use std::iter;

use pax_engine::{
    layout::TransformAndBounds,
    math::{Point2, Vector2},
    pax_manifest::UniqueTemplateNodeIdentifier,
    NodeInterface, Property,
};

use crate::{
    glass::SnapLines,
    model::{action::ActionContext, GlassNode},
    ROOT_PROJECT_ID,
};

use super::{
    coordinate_spaces::{Glass, SelectionSpace},
    AxisAlignedBox,
};

pub struct IntentSnapper {
    tol: f64,
    snap_set: SnapSet,
    snap_lines: Property<SnapLines>,
}

impl IntentSnapper {
    pub fn new(ctx: &ActionContext, ignore: &[UniqueTemplateNodeIdentifier]) -> Self {
        let mut snap_set = SnapSet::new(iter::empty());
        let root = ctx
            .engine_context
            .get_nodes_by_id(ROOT_PROJECT_ID)
            .into_iter()
            .next()
            .unwrap();
        let glass_transform = ctx.glass_transform().get();
        let glass_t_and_b = TransformAndBounds {
            transform: glass_transform,
            bounds: (1.0, 1.0),
        };
        let mut to_process = vec![root];
        while let Some(node) = to_process.pop() {
            if ignore.contains(&node.global_id().unwrap()) {
                continue;
            }
            let t_and_b = glass_t_and_b * node.transform_and_bounds().get();
            let axis_box = AxisAlignedBox::bound_of_points(t_and_b.corners());
            let center = t_and_b.center();
            snap_set.scene_vert.push(axis_box.top_left().x);
            snap_set
                .scene_vert
                .push(axis_box.top_left().x + axis_box.width());
            snap_set.scene_horiz.push(axis_box.top_left().y);
            snap_set
                .scene_horiz
                .push(axis_box.top_left().y + axis_box.height());
            snap_set.add(&[center]);

            to_process.extend(node.children())
        }
        Self {
            tol: 10.0,
            snap_set,
            snap_lines: Property::default(),
        }
    }

    pub fn get_snap_lines_prop(&self) -> Property<SnapLines> {
        self.snap_lines.clone()
    }

    pub fn snap(&self, set: &SnapSet) -> Vector2<Glass> {
        let info = self.snap_set.compute_snap(&set, self.tol);
        self.snap_lines.set(SnapLines {
            vertical: info.vertical_hits,
            horizontal: info.horizontal_hits,
        });
        info.offset
    }
}

pub struct SnapSet {
    scene_vert: Vec<f64>,
    scene_horiz: Vec<f64>,
}

pub struct SnapInfo {
    vertical_hits: Vec<f64>,
    horizontal_hits: Vec<f64>,
    offset: Vector2<Glass>,
}

impl SnapSet {
    pub fn new<'a>(points: impl IntoIterator<Item = &'a Point2<Glass>>) -> Self {
        let mut set = Self {
            scene_vert: Vec::new(),
            scene_horiz: Vec::new(),
        };
        set.add(points);
        set
    }

    pub fn from_transform_and_bounds(t_and_b: TransformAndBounds<SelectionSpace, Glass>) -> Self {
        let corners = t_and_b.corners();
        let center = t_and_b.center();
        let points_of_interest = corners.iter().chain(iter::once(&center));
        Self::new(points_of_interest)
    }

    pub fn add<'a>(&mut self, points: impl IntoIterator<Item = &'a Point2<Glass>>) {
        for p in points {
            self.scene_vert.push(p.x);
            self.scene_horiz.push(p.y);
        }
    }

    pub fn compute_snap(&self, other: &SnapSet, tol: f64) -> SnapInfo {
        let (vertical_hits, x_offset) =
            Self::compute_snap_axis(&self.scene_vert, &other.scene_vert, tol);
        let (horizontal_hits, y_offset) =
            Self::compute_snap_axis(&self.scene_horiz, &other.scene_horiz, tol);

        SnapInfo {
            vertical_hits,
            horizontal_hits,
            offset: Vector2::new(x_offset, y_offset),
        }
    }

    fn compute_snap_axis(slf: &[f64], other: &[f64], tol: f64) -> (Vec<f64>, f64) {
        let mut hits = vec![];
        let mut closest_snap = f64::MAX;
        let mut closest_offset = 0.0;
        for o in other {
            for s in slf {
                let offset = s - o;
                let dist = offset.abs();
                if dist < tol {
                    if dist < closest_snap {
                        hits.clear();
                        closest_snap = dist;
                        closest_offset = offset;
                        hits.push(*s);
                    }
                }
                if (offset - closest_offset).abs() < 1e-2 {
                    hits.push(*s)
                }
            }
        }
        (hits, closest_offset)
    }
}
