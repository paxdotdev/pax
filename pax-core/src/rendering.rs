use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::iter;
use std::ops::Mul;
use std::rc::Rc;

use kurbo::Affine;
use pax_runtime_api::{CommonProperties, RenderContext};
use piet::{Color, StrokeStyle};

use pax_runtime_api::{ArgsScroll, Layer, PropertyInstance, Size};

use crate::form_event::FormEvent;
use crate::{ExpandedNode, ExpressionTable, Globals, HandlerRegistry, RuntimeContext};

/// Type aliases to make it easier to work with nested Rcs and
/// RefCells for instance nodes.
pub type InstanceNodePtr = Rc<dyn InstanceNode>;
pub type InstanceNodePtrList = Vec<InstanceNodePtr>;

pub struct ScrollerArgs {
    pub size_inner_pane: [Box<dyn PropertyInstance<f64>>; 2],
    pub axes_enabled: [Box<dyn PropertyInstance<bool>>; 2],
}

pub struct InstantiationArgs {
    pub prototypical_common_properties_factory: Box<dyn Fn() -> Rc<RefCell<CommonProperties>>>,
    pub prototypical_properties_factory: Box<dyn Fn() -> Rc<RefCell<dyn Any>>>,
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry>>>,
    pub children: Option<InstanceNodePtrList>,
    pub component_template: Option<InstanceNodePtrList>,
    pub scroller_args: Option<ScrollerArgs>,

    ///used by Component instances, specifically to unwrap dyn Any properties
    ///and recurse into descendant property computation
    pub compute_properties_fn: Option<Box<dyn Fn(&ExpandedNode, &ExpressionTable, &Globals)>>,
}

#[derive(Copy, Clone)]
pub struct Point2D {
    x: f64,
    y: f64,
}

impl Point2D {
    fn subtract(self, other: Point2D) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }

    fn dot(self, other: Point2D) -> f64 {
        self.x * other.x + self.y * other.y
    }

    fn normal(self) -> Self {
        Self {
            x: -self.y,
            y: self.x,
        }
    }

    fn project_onto(self, axis: Point2D) -> f64 {
        let dot_product = self.dot(axis);
        dot_product / (axis.x.powi(2) + axis.y.powi(2))
    }
}

impl Mul<Point2D> for Affine {
    type Output = Point2D;

    #[inline]
    fn mul(self, other: Point2D) -> Point2D {
        let coeffs = self.as_coeffs();
        Point2D {
            x: coeffs[0] * other.x + coeffs[2] * other.y + coeffs[4],
            y: coeffs[1] * other.x + coeffs[3] * other.y + coeffs[5],
        }
    }
}

/// Stores the computed transform and the pre-transform bounding box (where the
/// other corner is the origin).  Useful for ray-casting, along with
#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Clone, PartialEq)]
pub struct TransformAndBounds {
    pub transform: Affine,
    pub bounds: (f64, f64),
    // pub clipping_bounds: Option<(f64, f64)>,
}

impl TransformAndBounds {
    pub fn corners(&self) -> [Point2D; 4] {
        let width = self.bounds.0;
        let height = self.bounds.1;

        let top_left = self.transform * Point2D { x: 0.0, y: 0.0 };
        let top_right = self.transform * Point2D { x: width, y: 0.0 };
        let bottom_left = self.transform * Point2D { x: 0.0, y: height };
        let bottom_right = self.transform
            * Point2D {
                x: width,
                y: height,
            };

        [top_left, top_right, bottom_right, bottom_left]
    }

    //Applies the separating axis theorem to determine whether two `TransformAndBounds` intersect.
    pub fn intersects(&self, other: &Self) -> bool {
        let corners_self = self.corners();
        let corners_other = other.corners();

        for i in 0..2 {
            let axis = corners_self[i].subtract(corners_self[(i + 1) % 4]).normal();

            let self_projections: Vec<_> =
                corners_self.iter().map(|&p| p.project_onto(axis)).collect();
            let other_projections: Vec<_> = corners_other
                .iter()
                .map(|&p| p.project_onto(axis))
                .collect();

            let (min_self, max_self) = min_max_projections(&self_projections);
            let (min_other, max_other) = min_max_projections(&other_projections);

            // Check for non-overlapping projections
            if max_self < min_other || max_other < min_self {
                // By the separating axis theorem, non-overlap of projections on _any one_ of the axis-normals proves that these polygons do not intersect.
                return false;
            }
        }
        true
    }
}

fn min_max_projections(projections: &[f64]) -> (f64, f64) {
    let min_projection = *projections
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_projection = *projections
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    (min_projection, max_projection)
}

#[derive(Clone)]
pub enum NodeType {
    Component,
    Primitive,
}

#[cfg(debug_assertions)]
impl std::fmt::Debug for dyn InstanceNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.resolve_debug(f, None)
    }
}

/// Central runtime representation of a properties-computable and renderable node.
/// `InstanceNode`s are conceptually stateless, and rely on [`ExpandedNode`]s for stateful representations.
///
/// An `InstanceNode` sits in between a [`pax_compiler::TemplateNodeDefinition`], the
/// compile-time `definition` analogue to this `instance`, and [`ExpandedNode`].
///
/// There is a 1:1 relationship between [`pax_compiler::TemplateNodeDefinition`]s and `InstanceNode`s.
/// There is a one-to-many relationship between one `InstanceNode` and possibly many variant [`ExpandedNode`]s,
/// due to duplication via `for`.
///
/// `InstanceNode`s are architecturally "type-aware" — they can perform type-specific operations e.g. on the state stored in [`ExpandedNode`], while
/// [`ExpandedNode`]s are "type-blind".  The latter store polymorphic data but cannot operate on it without the type-aware assistance of their linked `InstanceNode`.
///
/// (See [`RepeatInstance#expand_node`] where we visit a singular `InstanceNode` several times, producing multiple [`ExpandedNode`]s.)
pub trait InstanceNode {
    ///Retrieves the base instance, containing common functionality that all instances share
    fn base(&self) -> &BaseInstance;

    fn instantiate(args: InstantiationArgs) -> Rc<Self>
    where
        Self: Sized;

    /// Returns the bounds of an InstanceNode.  This computation requires a stateful [`ExpandedNode`], yet requires
    /// customization at the trait-implementor level (dyn InstanceNode), thus this method accepts an expanded_node
    /// parameter.
    /// The default implementation retrieves the expanded_node's [`pax_runtime_api::CommonProperties#width`] and [`pax_runtime_api::CommonProperties#height`]
    fn get_size(&self, expanded_node: &ExpandedNode) -> (Size, Size) {
        let common_properties = expanded_node.get_common_properties();
        let common_properties_borrowed = common_properties.borrow();
        (
            common_properties_borrowed.width.get().clone(),
            common_properties_borrowed.height.get().clone(),
        )
    }

    #[allow(unused_variables)]
    fn get_clipping_size(&self, expanded_node: &ExpandedNode) -> Option<(Size, Size)> {
        None
    }

    #[cfg(debug_assertions)]
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result;

    /// Expands the current `InstanceNode` into a stateful `ExpandedNode`, with its own instances of properties & common properties, in the context of the
    /// provided `PropertiesTreeContext`.  Node expansion takes into account the "parallel selves" that an `InstanceNode` may have through the
    /// lens of declarative control flow, [`ConditionalInstance`] and [`RepeatInstance`].
    // #[allow(unused_variables)]
    // fn recompute_children(
    //     self: Rc<Self>,
    //     expanded_node: &Rc<ExpandedNode>,
    //     ptc: &mut RuntimeContext,
    // ) {
    //     //Forward the same environment to children
    //     let env = Rc::clone(&expanded_node.stack);
    //     let children_with_envs = self
    //         .base()
    //         .get_template_children()
    //         .iter()
    //         .cloned()
    //         .zip(iter::repeat(env));
    //     expanded_node.set_children(children_with_envs, ptc);
    // }

    // /// Used by elements that need to communicate across native rendering bridge (for example: Text, Clipping masks, scroll containers)
    // /// Called by engine after [`expand_node`], passed calculated size and transform matrix coefficients for convenience
    // /// Expected to induce side-effects (if appropriate) via enqueueing messages to the native message queue
    // ///
    // /// An implementor of `handle_native_patches` is responsible for determining which properties if any have changed
    // /// (e.g. by keeping a local patch object as a cache of last known values.)
    #[allow(unused_variables)]
    fn handle_native_patches(&self, expanded_node: &ExpandedNode, context: &mut RuntimeContext) {
        //no-op default implementation
    }

    fn update_children(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &mut RuntimeContext,
    ) {
        if expanded_node.do_initial_expansion_of_children() {
            let env = Rc::clone(&expanded_node.stack);
            let children_with_envs = self
                .base()
                .get_template_children()
                .iter()
                .cloned()
                .zip(iter::repeat(env));
            expanded_node.set_children(children_with_envs, context);
        }
    }

    /// Second lifecycle method during each render loop, occurs after
    /// properties have been computed, but before rendering
    /// Example use-case: perform side-effects to the drawing contexts.
    /// This is how [`Frame`] performs clipping, for example.
    /// Occurs in a pre-order traversal of the render tree.
    #[allow(unused_variables)]
    fn handle_pre_render(
        &self,
        expanded_node: &ExpandedNode,
        context: &mut RuntimeContext,
        rcs: &mut Box<dyn RenderContext>,
    ) {
        //no-op default implementation
    }

    /// Third lifecycle method during each render loop, occurs
    /// after all descendents have been rendered.
    /// Occurs in a post-order traversal of the render tree. Most primitives
    /// are expected to draw their contents to the rendering context during this event.
    #[allow(unused_variables)]
    fn render(
        &self,
        expanded_node: &ExpandedNode,
        context: &mut RuntimeContext,
        rc: &mut Box<dyn RenderContext>,
    ) {
    }

    /// Fourth and final lifecycle method during each render loop, occurs
    /// after all descendents have been rendered AND the current node has been rendered.
    /// Useful for clean-up, e.g. this is where `Frame` cleans up the drawing contexts
    /// to stop clipping.
    /// Occurs in a post-order traversal of the render tree.
    #[allow(unused_variables)]
    fn handle_post_render(
        &self,
        context: &mut RuntimeContext,
        rcs: &mut HashMap<String, Box<dyn RenderContext>>,
    ) {
        //no-op default implementation
    }

    /// Fires during the tick when a node is first attached to the render tree.  For example,
    /// this event fires by all nodes on the global first tick, and by all nodes in a subtree
    /// when a `Conditional` subsequently turns on a subtree (i.e. when the `Conditional`s criterion becomes `true` after being `false` through the end of at least 1 frame.)
    /// A use-case: send a message to native renderers that a `Text` element should be rendered and tracked
    #[allow(unused_variables)]
    fn handle_mount(&self, expanded_node: &ExpandedNode, context: &mut RuntimeContext) {
        //no-op default implementation
    }

    /// Fires during element unmount, when an element is about to be removed from the render tree (e.g. by a `Conditional`)
    /// A use-case: send a message to native renderers that a `Text` element should be removed
    #[allow(unused_variables)]
    fn handle_unmount(&self, expanded_node: &ExpandedNode, context: &mut RuntimeContext) {
        //no-op default implementation
    }
    /// Invoked by event interrupts to pass scroll information to render node
    #[allow(unused_variables)]
    fn handle_scroll(&self, args_scroll: ArgsScroll) {
        //no-op default implementation
    }

    fn handle_form_event(&self, event: FormEvent) {
        panic!("form event sent to non-compatible component: {:?}", event)
    }
}

pub struct BaseInstance {
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry>>>,
    pub instance_prototypical_properties_factory: Box<dyn Fn() -> Rc<RefCell<dyn Any>>>,
    pub instance_prototypical_common_properties_factory:
        Box<dyn Fn() -> Rc<RefCell<CommonProperties>>>,
    instance_children: InstanceNodePtrList,
    flags: InstanceFlags,
}

pub struct InstanceFlags {
    /// Used for exotic tree traversals for `Slot`, e.g. for `Stacker` > `Repeat` > `Rectangle`
    /// where the repeated `Rectangle`s need to be be considered direct children of `Stacker`.
    /// `Repeat` and `Conditional` override `is_invisible_to_slot` to return true
    pub invisible_to_slot: bool,
    /// Certain elements, such as Groups and Components, are invisible to ray-casting.
    /// Since these container elements are on top of the elements they contain,
    /// this is needed otherwise the containers would intercept rays that should hit their contents.
    pub invisible_to_raycasting: bool,
    /// The layer type (`Layer::Native` or `Layer::Canvas`) for this RenderNode.
    /// Default is `Layer::Canvas`, and must be overwritten for `InstanceNode`s that manage native
    /// content.
    pub layer: Layer,

    /// Only true for ComponentInstance
    pub is_component: bool,
}

impl BaseInstance {
    pub fn new(args: InstantiationArgs, flags: InstanceFlags) -> Self {
        BaseInstance {
            handler_registry: args.handler_registry,
            instance_prototypical_common_properties_factory: args
                .prototypical_common_properties_factory,
            instance_prototypical_properties_factory: args.prototypical_properties_factory,
            instance_children: args.children.unwrap_or_default(),
            flags,
        }
    }

    /// Returns a handle to a node-managed HandlerRegistry, a mapping between event types and handlers.
    /// Each node that can handle events is responsible for implementing this; Component instances generate
    /// the necessary code to wire up userland events like `<SomeNode @click=self.handler>`. Primitives must handle
    /// this explicitly, see e.g. `[pax_std_primitives::RectangleInstance#get_handler_registry]`.
    pub fn get_handler_registry(&self) -> Option<Rc<RefCell<HandlerRegistry>>> {
        match &self.handler_registry {
            Some(registry) => Some(Rc::clone(registry)),
            _ => None,
        }
    }

    /// Return the list of instance nodes that are children of this one.  Intuitively, this will return
    /// instance nodes mapping exactly to the template node definitions.
    /// For `Component`s, `get_instance_children` returns the root(s) of its template, not its `slot_children`.
    /// (see [`get_slot_children`] for the way to retrieve the latter.)
    pub fn get_template_children(&self) -> &InstanceNodePtrList {
        &self.instance_children
    }

    pub fn flags(&self) -> &InstanceFlags {
        &self.flags
    }
}

/// Represents the outer stroke of a drawable element
pub struct StrokeInstance {
    pub color: Color,
    pub width: f64,
    pub style: StrokeStyle,
}

// Recursive workhorse method for rendering, conceptually "as a function of the ExpandedNode tree passed in"
// pub fn recurse_render(
//     rtc: &mut RenderTreeContext,
//     rcs: &mut HashMap<String, Box<dyn pax_runtime_api::RenderContext>>,
//     z_index_info: &mut LayerId,
//     marked_for_unmount: bool,
// ) {
//     //Recurse:
//     //  - fire lifecycle events for this node
//     //  - iterate backwards over children (lowest first); recurse until there are no more descendants.  Read computed properties from ExpandedNodes, e.g. for transform and bounds.
//     //  - we now have the back-most leaf node.  Render it.  Return.
//     //  - we're now at the second back-most leaf node.  Render it.  Return ...

//     let expanded_node = Rc::clone(&rtc.current_expanded_node);

//     // Rendering is a no-op is a node is marked for unmount.  Note that means this entire subtree will be skipped for rendering.
//     let id_chain = { expanded_node.borrow().id_chain.clone() };
//     if rtc
//         .engine
//         .node_registry
//         .borrow()
//         .is_marked_for_unmount(&id_chain)
//     {
//         return;
//     }

//     rtc.current_instance_node = Rc::clone(&expanded_node.borrow().instance_node);
//     //depth work

//     //scroller IDs are used by chassis, for identifying native scrolling containers
//     let scroller_ids = rtc.current_expanded_node.borrow().scroller_stack.clone();
//     let scroller_id = match scroller_ids.last() {
//         None => None,
//         Some(v) => Some(v.clone()),
//     };
//     let canvas_id = LayerId::assemble_canvas_id(
//         scroller_id.clone(),
//         expanded_node.borrow().computed_canvas_index.unwrap(),
//     );

//     manage_handlers_pre_render(rtc);

//     let mut subtree_depth = 0;

//     //keep recursing through children
//     let mut child_z_index_info = z_index_info.clone();
//     if z_index_info.get_current_layer() == Layer::Scroller {
//         let id_chain = expanded_node.borrow().id_chain.clone();
//         child_z_index_info = LayerId::new(Some(id_chain));
//         // let (scroll_offset_x, scroll_offset_y) = node.borrow_mut().get_scroll_offset();
//         // let mut reset_transform = Affine::default();
//         // reset_transform =
//         //     reset_transform.then_translate(Vec2::new(scroll_offset_x, scroll_offset_y));
//         // rtc.transform_scroller_reset = reset_transform.clone();
//     }

//     let children_cloned = expanded_node
//         .borrow_mut()
//         .get_children_expanded_nodes()
//         .clone();

//     children_cloned.iter().rev().for_each(|expanded_node| {
//         //note that we're iterating starting from the last child, for z-index (.rev())
//         let mut new_rtc = rtc.clone();
//         new_rtc.current_expanded_node = Rc::clone(expanded_node);
//         // if it's a scroller reset the z-index context for its children
//         recurse_render(
//             &mut new_rtc,
//             rcs,
//             &mut child_z_index_info.clone(),
//             marked_for_unmount,
//         );
//         //FUTURE: for dependency management, return computed values from subtree above

//         subtree_depth = subtree_depth.max(child_z_index_info.get_level());
//     });

//     let is_viewport_culled = !&expanded_node
//         .borrow()
//         .computed_tab
//         .as_ref()
//         .unwrap()
//         .intersects(&rtc.engine.viewport_tab);

//     // let accumulated_bounds = rtc
//     //     .current_expanded_node
//     //     .borrow()
//     //     .computed_tab
//     //     .as_ref()
//     //     .unwrap()
//     //     .bounds;
//     // let clipping = expanded_node
//     //     .borrow_mut()
//     //     .get_clipping_size_computed(accumulated_bounds);

//     // let clipping_bounds = match expanded_node.borrow_mut().get_clipping_size() {
//     //     None => None,
//     //     Some(_) => Some(clipping),
//     // };

//     // let clipping_aware_bounds = if let Some(cb) = clipping_bounds {
//     //     cb
//     // } else {
//     //     new_accumulated_bounds
//     // };

//     if let Some(rc) = rcs.get_mut(&canvas_id) {
//         //lifecycle: render
//         //this is this node's time to do its own rendering, aside
//         //from the rendering of its children. Its children have already been rendered.
//         if !is_viewport_culled {
//             expanded_node.borrow().instance_node.handle_render(rtc, rc);
//         }
//     } else {
//         if let Some(rc) = rcs.get_mut("0") {
//             if !is_viewport_culled {
//                 expanded_node.borrow().instance_node.handle_render(rtc, rc);
//             }
//         }
//     }

//     //lifecycle: post_render
//     expanded_node
//         .borrow()
//         .instance_node
//         .handle_post_render(rtc, rcs);
// }

// /// Helper method to fire `pre_render` handlers for the node attached to the `rtc`
// fn manage_handlers_pre_render(rtc: &mut RenderTreeContext) {
//     //fire `pre_render` handlers
//     let node = Rc::clone(&rtc.current_expanded_node);
//     let node_borrowed = (*node).borrow();
//     let registry = node_borrowed
//         .instance_node
//         .base()
//         .get_handler_registry()
//         .clone();
//     if let Some(registry) = registry {
//         for handler in (*registry).borrow().pre_render_handlers.iter() {
//             handler(
//                 Rc::clone(&node_borrowed.get_properties()),
//                 &node_borrowed.computed_node_context.clone().unwrap(),
//             );
//         }
//     }
// }
