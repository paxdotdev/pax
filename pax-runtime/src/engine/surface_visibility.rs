//! Conservative GPU surface admission for desktop scroller layers.

use crate::{ExpandedNodeIdentifier, RuntimeContext};
use kurbo::{Affine, Rect, Shape};

/// A scroller's content-to-window mapping and intersected ancestor warm region.
///
/// This only controls backing-surface residency. Nodes and native controls remain
/// mounted. Recompute it when planning surfaces, including after presentation
/// scrolling, so a previously cold tile can acquire a fresh backing surface.
pub struct ScrollerSurfaceVisibility {
    content_to_window: Affine,
    warm_bounds: Rect,
}

impl ScrollerSurfaceVisibility {
    /// Whether a content-space tile can contribute inside the warmed clip region.
    /// Tile bounds must include any coverage already reserved for render effects.
    pub fn contains_tile(&self, bounds: Rect) -> bool {
        finite_rect(bounds)
            && intersection(
                Some(self.warm_bounds),
                self.content_to_window.transform_rect_bbox(bounds),
            )
            .is_some()
    }
}

impl RuntimeContext {
    /// Resolve conservative scroller surface admission through all ancestor clips.
    ///
    /// Padding is in window pixels. Invalid or collapsed geometry has no resident
    /// surface until the next valid plan; it does not unmount application state.
    pub fn scroller_surface_visibility(
        &self,
        owner: ExpandedNodeIdentifier,
        padding: f64,
    ) -> Option<ScrollerSurfaceVisibility> {
        if !padding.is_finite() || padding < 0.0 {
            return None;
        }
        let (width, height) = self.globals().viewport.get().bounds;
        let viewport = Rect::new(0.0, 0.0, width, height);
        if !finite_rect(viewport) {
            return None;
        }
        let root_warm = viewport.inflate(padding, padding);
        let owner = self.get_expanded_node_by_eid(owner)?;
        let mut chain = vec![owner.clone()];
        let mut parent = owner.render_parent_node();
        while let Some(node) = parent {
            parent = node.render_parent_node();
            chain.push(node);
        }
        chain.reverse();
        let mut inherited_scroll = Affine::IDENTITY;
        let mut warm = Some(root_warm);
        for node in chain {
            if node.get_common_properties().borrow().unclippable.get() == Some(true) {
                // Escaping a clip does not escape the window or scrolling.
                warm = Some(root_warm);
            }
            let tab = node.transform_and_bounds.get();
            let world = Affine::from(tab.transform);
            if !finite_transform(world) {
                return None;
            }
            let instance = node.instance_node.borrow();
            let scrolls = instance.scrolls_content(&node);
            if scrolls && instance.clips_content(&node) {
                let clip = (inherited_scroll * world).transform_rect_bbox(Rect::new(
                    0.0,
                    0.0,
                    tab.bounds.0,
                    tab.bounds.1,
                ));
                warm = padded_intersection(warm, clip, padding);
            }
            if let Some(path) = instance.resolve_effect_clip_path(&node) {
                let clip = (inherited_scroll * path).bounding_box();
                warm = padded_intersection(warm, clip, padding);
            }
            drop(instance);
            if scrolls {
                inherited_scroll =
                    inherited_scroll * self.scroller_content_presentation_transform(&node);
                if !finite_transform(inherited_scroll) {
                    return None;
                }
            }
        }
        let content_to_window =
            inherited_scroll * Affine::from(owner.transform_and_bounds.get().transform);
        finite_transform(content_to_window).then_some(ScrollerSurfaceVisibility {
            content_to_window,
            warm_bounds: warm?,
        })
    }
}

fn finite_transform(transform: Affine) -> bool {
    let determinant = transform.determinant();
    transform.as_coeffs().iter().all(|value| value.is_finite())
        && determinant.is_finite()
        && determinant != 0.0
}

fn finite_rect(rect: Rect) -> bool {
    [rect.x0, rect.y0, rect.x1, rect.y1]
        .iter()
        .all(|value| value.is_finite())
        && rect.width() > 0.0
        && rect.height() > 0.0
}

fn intersection(a: Option<Rect>, b: Rect) -> Option<Rect> {
    if !finite_rect(b) {
        return None;
    }
    let rect = a?.intersect(b);
    finite_rect(rect).then_some(rect)
}

fn padded_intersection(a: Option<Rect>, b: Rect, padding: f64) -> Option<Rect> {
    // Padding must not turn an empty or invalid clip into visible coverage.
    finite_rect(b)
        .then(|| intersection(a, b.inflate(padding, padding)))
        .flatten()
}
