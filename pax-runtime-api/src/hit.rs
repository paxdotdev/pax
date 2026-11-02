//! Immutable input geometry, independent of paint opacity and effect bounds.
use kurbo::{BezPath, Point, Rect, Shape};
use std::rc::Rc;

/// Geometry prepared by a primitive before a point query runs.
///
/// Values contain no live nodes or callbacks. A custom primitive describes its
/// local hit region with a path; it must invalidate input when that geometry
/// changes. Rectangles use strict edges, matching ordinary logical bounds.
#[derive(Clone, Debug, Default)]
pub enum HitShape {
    /// No input area (distinct from an unknown conservative bound).
    #[default]
    Empty,
    /// A rectangle in local layout coordinates, with exclusive edges.
    Rect(Rect),
    /// Nonzero-winding local coverage. `tolerance` also tests offsets along the
    /// two local axes, preserving Line's historical stroke-edge convention.
    Path { path: Rc<BezPath>, tolerance: f64 },
    /// A convex mesh posed in world coordinates. Available with `3d`.
    #[cfg(feature = "3d")]
    Mesh(Rc<crate::mesh3::PosedConvexMesh>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_holes_and_local_tolerance_are_distinct_from_logical_rectangles() {
        let mut path = Rect::new(-10., -10., 30., 30.).to_path(0.1);
        path.move_to((0., 0.));
        path.line_to((0., 20.));
        path.line_to((20., 20.));
        path.line_to((20., 0.));
        path.close_path();
        let shape = HitShape::Path {
            path: Rc::new(path),
            tolerance: 1e-3,
        };
        assert!(shape.contains_local(Point::new(-5., 10.)));
        assert!(
            !shape.contains_local(Point::new(10., 10.)),
            "nonzero-winding hole"
        );
        assert!(shape.contains_local(Point::new(-10.0005, 10.)));
        assert!(!shape.contains_local(Point::new(-10.002, 10.)));
        assert_eq!(
            shape.local_bounds(),
            Some(Rect::new(-10.001, -10.001, 30.001, 30.001))
        );
        assert!(!HitShape::Rect(Rect::new(0., 0., 20., 20.)).contains_local(Point::new(0., 10.)));
        assert!(!shape.contains_local(Point::new(f64::NAN, 10.)));
    }
}

impl HitShape {
    /// Conservative local bounds; mesh bounds are prepared after projection.
    /// Nonfinite bounds must be treated conservatively, never as a finite box.
    pub fn local_bounds(&self) -> Option<Rect> {
        match self {
            Self::Empty => Some(Rect::ZERO),
            Self::Rect(rect) => Some(*rect),
            Self::Path { path, tolerance } => Some(
                path.bounding_box()
                    .inflate(tolerance.max(0.), tolerance.max(0.)),
            ),
            #[cfg(feature = "3d")]
            Self::Mesh(_) => None,
        }
    }

    /// Test local planar coverage. Meshes require a projected surface query.
    pub fn contains_local(&self, point: Point) -> bool {
        if !point.x.is_finite() || !point.y.is_finite() {
            return false;
        }
        match self {
            Self::Empty => false,
            Self::Rect(rect) => {
                point.x > rect.x0 && point.x < rect.x1 && point.y > rect.y0 && point.y < rect.y1
            }
            Self::Path { path, tolerance } => {
                path.contains(point) || {
                    let t = tolerance.max(0.);
                    [(t, 0.), (-t, 0.), (0., t), (0., -t)]
                        .into_iter()
                        .any(|(x, y)| path.contains(Point::new(point.x + x, point.y + y)))
                }
            }
            #[cfg(feature = "3d")]
            Self::Mesh(_) => false,
        }
    }
}
