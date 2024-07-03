use pax_engine::{
    layout::{LayoutProperties, TransformAndBounds},
    math::{Point2, Transform2, Vector2},
    NodeInterface, NodeLocal, Property,
};
use pax_manifest::UniqueTemplateNodeIdentifier;
use pax_runtime_api::{Axis, Interpolatable, Size, Window};

use crate::math::{
    coordinate_spaces::{Glass, SelectionSpace},
    AxisAlignedBox,
};

impl Interpolatable for SelectionState {}

#[derive(Clone, Default)]
pub struct SelectionState {
    pub total_bounds: Property<TransformAndBounds<SelectionSpace, Glass>>,
    // Either center if multiple objects selected, or the anchor point for single objects
    pub total_origin: Property<Point2<Glass>>,
    pub items: Vec<SelectedItem>,
}

pub struct SelectionStateSnapshot {
    pub total_bounds: TransformAndBounds<SelectionSpace, Glass>,
    pub total_origin: Point2<Glass>,
    pub items: Vec<RuntimeNodeInfo>,
}

pub struct RuntimeNodeInfo {
    pub id: UniqueTemplateNodeIdentifier,
    pub transform_and_bounds: TransformAndBounds<NodeLocal, Glass>,
    pub parent_transform_and_bounds: TransformAndBounds<NodeLocal, Glass>,
    pub origin: Point2<Glass>,
    pub layout_properties: LayoutProperties,
}

impl From<&SelectionState> for SelectionStateSnapshot {
    fn from(value: &SelectionState) -> Self {
        Self {
            total_bounds: value.total_bounds.get(),
            total_origin: value.total_origin.get(),
            items: value.items.iter().map(Into::into).collect(),
        }
    }
}

impl From<&SelectedItem> for RuntimeNodeInfo {
    fn from(itm: &SelectedItem) -> Self {
        RuntimeNodeInfo {
            id: itm.id.clone(),
            origin: itm.origin.get(),
            transform_and_bounds: itm.transform_and_bounds.get(),
            parent_transform_and_bounds: itm.parent_transform_and_bounds.get(),
            layout_properties: itm.layout_properties.clone(),
        }
    }
}

#[derive(Default, Clone)]
pub struct SelectedItem {
    // unit rectangle to object bounds transform
    pub transform_and_bounds: Property<TransformAndBounds<NodeLocal, Glass>>,
    pub parent_transform_and_bounds: Property<TransformAndBounds<NodeLocal, Glass>>,
    pub origin: Property<Point2<Glass>>,
    pub layout_properties: LayoutProperties,
    pub id: UniqueTemplateNodeIdentifier,
}

impl SelectionState {
    pub fn new(
        selected_nodes: Vec<(UniqueTemplateNodeIdentifier, NodeInterface)>,
        to_glass_transform: Property<Transform2<Window, Glass>>,
    ) -> Self {
        let items: Vec<_> = selected_nodes
            .into_iter()
            .flat_map(|(id, n)| {
                Some(SelectedItem {
                    transform_and_bounds: {
                        let t_and_b = n.transform_and_bounds();
                        let deps = [t_and_b.untyped(), to_glass_transform.untyped()];
                        let to_glass = to_glass_transform.clone();
                        Property::computed(
                            move || {
                                TransformAndBounds {
                                    transform: to_glass.get(),
                                    bounds: (1.0, 1.0),
                                } * t_and_b.get()
                            },
                            &deps,
                        )
                    },
                    parent_transform_and_bounds: {
                        let to_glass = to_glass_transform.clone();
                        let parent_t_and_b = n
                            .render_parent()
                            .map(|v| v.transform_and_bounds())
                            .unwrap_or_default();
                        let deps = [parent_t_and_b.untyped(), to_glass_transform.untyped()];
                        Property::computed(
                            move || {
                                TransformAndBounds {
                                    transform: to_glass.get(),
                                    bounds: (1.0, 1.0),
                                } * parent_t_and_b.get()
                            },
                            &deps,
                        )
                    },
                    origin: {
                        let parent_t_and_b = n
                            .render_parent()
                            .map(|p| p.transform_and_bounds())
                            .unwrap_or_default();
                        let properties = n.layout_properties();
                        let deps = [parent_t_and_b.untyped(), to_glass_transform.untyped()];
                        let to_glass = to_glass_transform.clone();
                        Property::computed(
                            move || {
                                let parent_t_and_b = parent_t_and_b.get();
                                let parent_bounds = parent_t_and_b.bounds;
                                let local_x = properties
                                    .x
                                    .unwrap_or(Size::ZERO())
                                    .evaluate(parent_bounds, Axis::X);
                                let local_y = properties
                                    .y
                                    .unwrap_or(Size::ZERO())
                                    .evaluate(parent_bounds, Axis::Y);
                                to_glass.get()
                                    * parent_t_and_b.transform
                                    * Point2::new(local_x, local_y)
                            },
                            &deps,
                        )
                    },
                    layout_properties: n.layout_properties(),
                    id,
                })
            })
            .collect();

        let deps: Vec<_> = items
            .iter()
            .map(|i| i.transform_and_bounds.untyped())
            .collect();
        let bounds: Vec<_> = items
            .iter()
            .map(|i| i.transform_and_bounds.clone())
            .collect();
        let total_bounds = Property::computed(
            move || {
                if bounds.len() == 1 {
                    let t_and_b = bounds[0].get();
                    t_and_b.cast_spaces().as_pure_scale()
                } else {
                    let axis_box =
                        AxisAlignedBox::bound_of_points(bounds.iter().flat_map(|t_and_b| {
                            let t_and_b = t_and_b.get();
                            let (o, u, v) = t_and_b.transform.decompose();
                            let u = u * t_and_b.bounds.0;
                            let v = v * t_and_b.bounds.1;
                            [o, o + v, o + u, o + v + u]
                        }));
                    let transform = Transform2::compose(
                        axis_box.top_left(),
                        Vector2::new(axis_box.width(), 0.0),
                        Vector2::new(0.0, axis_box.height()),
                    );
                    TransformAndBounds {
                        transform,
                        bounds: (1.0, 1.0),
                    }
                }
            },
            &deps,
        );
        let origin: Vec<_> = items.iter().map(|i| i.origin.clone()).collect();
        let total_origin = if origin.len() == 1 {
            origin[0].clone()
        } else {
            let deps = [total_bounds.untyped()];
            let t_b = total_bounds.clone();
            Property::computed(
                move || {
                    let t_b = t_b.get();
                    let (o, u, v) = t_b.transform.decompose();
                    let u = u * t_b.bounds.0;
                    let v = v * t_b.bounds.1;
                    let center = o + u / 2.0 + v / 2.0;
                    center
                },
                &deps,
            )
        };

        SelectionState {
            total_origin,
            items,
            total_bounds,
        }
    }
}
