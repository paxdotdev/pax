use std::iter;

use pax_engine::{
    api::Color,
    log,
    math::{Point2, Vector2},
    node_layout::TransformAndBounds,
    pax_manifest::UniqueTemplateNodeIdentifier,
    NodeInterface, NodeLocal, Property,
};

use crate::{
    glass::{SnapInfo, SnapLine},
    model::{
        action::ActionContext,
        input::{InputEvent, ModifierKey},
        GlassNode,
    },
};

use super::{
    coordinate_spaces::{Glass, SelectionSpace},
    AxisAlignedBox,
};

pub struct IntentSnapper {
    tol: f64,
    snap_set: SnapCollection,
    snap_lines: Property<SnapInfo>,
    snap_enabled: Property<bool>,
}

impl IntentSnapper {
    pub fn new_from_scene(ctx: &ActionContext, ignore: &[UniqueTemplateNodeIdentifier]) -> Self {
        let root = ctx.engine_context.get_userland_root_expanded_node();
        let glass_transform = ctx.glass_transform().get();
        let glass_t_and_b = TransformAndBounds {
            transform: glass_transform,
            bounds: (1.0, 1.0),
        };
        let mut snap_set_scene =
            SnapSet::new(Color::rgba(255.into(), 142.into(), 56.into(), 150.into()));
        let t_and_b = glass_t_and_b * root.transform_and_bounds().get();
        snap_set_scene.add_lines_from_axis_aligned_bounds(t_and_b);

        let mut snap_set_children =
            SnapSet::new(Color::rgba(16.into(), 196.into(), 187.into(), 150.into()));
        let mut to_process = root.children();
        while let Some(node) = to_process.pop() {
            if node.global_id().is_some() && ignore.contains(&node.global_id().unwrap()) {
                continue;
            }
            let t_and_b = glass_t_and_b * node.transform_and_bounds().get();
            snap_set_children.add_lines_from_axis_aligned_bounds(t_and_b);
            to_process.extend(node.children())
        }
        Self::new(
            ctx,
            SnapCollection {
                sets: vec![snap_set_scene, snap_set_children],
            },
        )
    }

    pub fn new(ctx: &ActionContext, snap_collection: SnapCollection) -> Self {
        let keys = ctx.app_state.modifiers.clone();
        let deps = [keys.untyped()];
        let snap_enabled =
            Property::computed(move || !keys.get().contains(&ModifierKey::Meta), &deps);
        Self {
            tol: 8.0,
            snap_set: snap_collection,
            snap_lines: Property::default(),
            snap_enabled,
        }
    }

    pub fn get_snap_lines_prop(&self) -> Property<SnapInfo> {
        let snap_lines = self.snap_lines.clone();
        let snap_enabled = self.snap_enabled.clone();
        let deps = [snap_lines.untyped(), snap_enabled.untyped()];
        Property::computed(
            move || {
                snap_enabled
                    .get()
                    .then(|| snap_lines.get())
                    .unwrap_or_default()
            },
            &deps,
        )
    }

    pub fn snap(&self, set: &[Point2<Glass>], lock_x: bool, lock_y: bool) -> Vector2<Glass> {
        if !self.snap_enabled.get() {
            return Vector2::default();
        }
        let info = self.snap_set.compute_snap(&set, self.tol, lock_x, lock_y);
        self.snap_lines.set(SnapInfo {
            vertical: info.vertical_hits,
            horizontal: info.horizontal_hits,
            points: info
                .point_hit
                .as_slice()
                .iter()
                .map(|p| vec![p.x, p.y])
                .collect(),
        });
        info.offset
    }
}

pub struct SnapCollection {
    pub sets: Vec<SnapSet>,
}

impl SnapCollection {
    fn compute_snap(
        &self,
        points: &[Point2<Glass>],
        tol: f64,
        lock_x: bool,
        lock_y: bool,
    ) -> InternalSnapInfo {
        let (vertical_hits, x_offset) = Self::compute_snap_axis(
            self.sets
                .iter()
                .flat_map(|v| v.scene_vert.iter().map(|p| (*p, v.color.clone()))),
            &points.iter().map(|p| p.x).collect::<Vec<_>>(),
            tol,
        );
        let (horizontal_hits, y_offset) = Self::compute_snap_axis(
            self.sets
                .iter()
                .flat_map(|v| v.scene_horiz.iter().map(|p| (*p, v.color.clone()))),
            &points.iter().map(|p| p.y).collect::<Vec<_>>(),
            tol,
        );
        let point_hit = Self::compute_snap_points(
            self.sets.iter().flat_map(|p| &p.scene_points).cloned(),
            &points,
            tol,
        );

        InternalSnapInfo {
            vertical_hits: (!lock_x).then_some(vertical_hits).unwrap_or_default(),
            horizontal_hits: (!lock_y).then_some(horizontal_hits).unwrap_or_default(),
            point_hit: point_hit.map(|v| v.0),
            offset: point_hit
                .map(|v| v.1)
                .unwrap_or(Vector2::new(x_offset, y_offset)),
        }
    }

    fn compute_snap_axis(
        slf: impl Iterator<Item = (f64, Color)>,
        other: &[f64],
        tol: f64,
    ) -> (Vec<SnapLine>, f64) {
        let mut hits = vec![];
        let mut closest_snap = f64::MAX;
        let mut closest_offset = 0.0;
        for (s, col) in slf {
            for o in other {
                let offset = s - o;
                let dist = offset.abs();
                if dist < tol {
                    if dist < closest_snap {
                        hits.clear();
                        closest_snap = dist;
                        closest_offset = offset;
                        hits.push(SnapLine {
                            line: s,
                            color: col.clone(),
                        });
                    }
                }
                if (offset - closest_offset).abs() < 1e-2 {
                    hits.push(SnapLine {
                        line: s,
                        color: col.clone(),
                    })
                }
            }
        }
        (hits, closest_offset)
    }

    fn compute_snap_points(
        slf: impl Iterator<Item = Point2<Glass>>,
        other: &[Point2<Glass>],
        tol: f64,
    ) -> Option<(Point2<Glass>, Vector2<Glass>)> {
        let mut closest_sq = f64::MAX;
        let mut closest_info = None;
        for p_s in slf {
            for p_o in other {
                let offset = p_s - *p_o;
                let dist_sq = offset.length_squared();
                if dist_sq < tol * tol {
                    if dist_sq < closest_sq {
                        closest_sq = dist_sq;
                        closest_info = Some((p_s, offset));
                    }
                }
            }
        }
        closest_info
    }
}

impl Into<SnapCollection> for SnapSet {
    fn into(self) -> SnapCollection {
        SnapCollection { sets: vec![self] }
    }
}

#[derive(Default)]
pub struct SnapSet {
    color: Color,
    scene_vert: Vec<f64>,
    scene_horiz: Vec<f64>,
    scene_points: Vec<Point2<Glass>>,
}

struct InternalSnapInfo {
    vertical_hits: Vec<SnapLine>,
    horizontal_hits: Vec<SnapLine>,
    point_hit: Option<Point2<Glass>>,
    offset: Vector2<Glass>,
}

impl SnapSet {
    pub fn new(color: Color) -> Self {
        Self {
            color,
            scene_vert: Vec::new(),
            scene_horiz: Vec::new(),
            scene_points: Vec::new(),
        }
    }

    pub fn from_points<'a>(
        points: impl IntoIterator<Item = &'a Point2<Glass>> + Clone,
        color: Color,
    ) -> Self {
        let mut set = Self::new(color);
        set.add_points(points.clone());
        set.add_lines(points);
        set
    }

    pub fn lines_from_transform_and_bounds(
        t_and_b: TransformAndBounds<SelectionSpace, Glass>,
        color: Color,
    ) -> Self {
        let corners = t_and_b.corners();
        let center = t_and_b.center();
        let points_of_interest = corners.iter().chain(iter::once(&center));
        let mut set = Self::new(color);
        set.add_lines(points_of_interest);
        set
    }

    pub fn points_from_transform_and_bounds(
        t_and_b: TransformAndBounds<NodeLocal, Glass>,
        color: Color,
    ) -> Self {
        let corners = t_and_b.corners();
        let center = t_and_b.center();
        let middle = [
            corners[0].midpoint_towards(corners[1]),
            corners[1].midpoint_towards(corners[2]),
            corners[2].midpoint_towards(corners[3]),
            corners[3].midpoint_towards(corners[0]),
        ];
        let points_of_interest = corners
            .iter()
            .chain(middle.iter())
            .chain(iter::once(&center));
        let mut set = Self::new(color);
        set.add_points(points_of_interest);
        set
    }

    pub fn add_lines_from_axis_aligned_bounds(
        &mut self,
        t_and_b: TransformAndBounds<NodeLocal, Glass>,
    ) {
        let axis_box = AxisAlignedBox::bound_of_points(t_and_b.corners());
        let center = t_and_b.center();
        self.scene_vert.push(axis_box.top_left().x);
        self.scene_vert
            .push(axis_box.top_left().x + axis_box.width());
        self.scene_horiz.push(axis_box.top_left().y);
        self.scene_horiz
            .push(axis_box.top_left().y + axis_box.height());
        self.add_lines(&[center]);
    }

    pub fn add_lines<'a>(&mut self, points: impl IntoIterator<Item = &'a Point2<Glass>>) {
        for p in points {
            self.scene_vert.push(p.x);
            self.scene_horiz.push(p.y);
        }
    }

    pub fn add_points<'a>(&mut self, points: impl IntoIterator<Item = &'a Point2<Glass>>) {
        self.scene_points.extend(points)
    }
}
