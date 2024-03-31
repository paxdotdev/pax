use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use super::api::math::Point2;
use std::iter;
use std::rc::Rc;

use crate::api::math::Transform2;
use crate::api::{CommonProperties, RenderContext, Window};
use crate::node_interface::NodeLocal;
use pax_manifest::UniqueTemplateNodeIdentifier;
use pax_runtime_api::properties::ErasedProperty;
use piet::{Color, StrokeStyle};

use crate::api::{Layer, Scroll, Size};

use crate::{
    ExpandedNode, ExpressionTable, Globals, HandlerRegistry, RuntimeContext,
    RuntimePropertiesStackFrame,
};

/// Type aliases to make it easier to work with nested Rcs and
/// RefCells for instance nodes.
pub type InstanceNodePtr = Rc<dyn InstanceNode>;
pub type InstanceNodePtrList = RefCell<Vec<InstanceNodePtr>>;

pub struct InstantiationArgs {
    pub prototypical_common_properties_factory: Box<
        dyn Fn(
            Rc<RuntimePropertiesStackFrame>,
            Rc<ExpressionTable>,
        ) -> Rc<RefCell<CommonProperties>>,
    >,
    pub prototypical_properties_factory:
        Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<dyn Any>>>,
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry>>>,
    pub children: Option<InstanceNodePtrList>,
    pub component_template: Option<InstanceNodePtrList>,

    ///used by Component instances, specifically to unwrap dyn Any properties
    ///and recurse into descendant property computation
    pub compute_properties_fn: Option<Box<dyn Fn(&ExpandedNode, &ExpressionTable, &Globals)>>,

    pub template_node_identifier: Option<UniqueTemplateNodeIdentifier>,

    // Used by RuntimePropertyStackFrame to pull out struct's properties based on their names
    pub properties_scope_factory:
        Option<Box<dyn Fn(Rc<RefCell<dyn Any>>) -> HashMap<String, ErasedProperty>>>,
}

/// Stores the computed transform and the pre-transform bounding box (where the
/// other corner is the origin).  Useful for ray-casting, along with
#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Clone, PartialEq)]
pub struct TransformAndBounds {
    pub transform: Transform2<NodeLocal, Window>,
    pub bounds: (f64, f64),
    // pub clipping_bounds: Option<(f64, f64)>,
}

impl TransformAndBounds {
    pub fn corners(&self) -> [Point2<Window>; 4] {
        let width = self.bounds.0;
        let height = self.bounds.1;

        let top_left = self.transform * Point2::new(0.0, 0.0);
        let top_right = self.transform * Point2::new(width, 0.0);
        let bottom_left = self.transform * Point2::new(0.0, height);
        let bottom_right = self.transform * Point2::new(width, height);

        [top_left, top_right, bottom_right, bottom_left]
    }

    //Applies the separating axis theorem to determine whether two `TransformAndBounds` intersect.
    pub fn intersects(&self, other: &Self) -> bool {
        let corners_self = self.corners();
        let corners_other = other.corners();

        for i in 0..2 {
            let axis = (corners_self[i] - corners_self[(i + 1) % 4]).normal();

            let self_projections: Vec<_> = corners_self
                .iter()
                .map(|&p| p.to_vector().project_onto(axis).length())
                .collect();
            let other_projections: Vec<_> = corners_other
                .iter()
                .map(|&p| p.to_vector().project_onto(axis).length())
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
    /// The default implementation retrieves the expanded_node's [`crate::api::CommonProperties#width`] and [`crate::api::CommonProperties#height`]
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

    /// Used by elements that need to communicate across native rendering bridge (for example: Text, Clipping masks, scroll containers)
    /// Called by engine after [`expand_node`], passed calculated size and transform matrix coefficients for convenience
    /// Expected to induce side-effects (if appropriate) via enqueueing messages to the native message queue
    ///
    /// An implementor of `handle_native_patches` is responsible for determining which properties if any have changed
    /// (e.g. by keeping a local patch object as a cache of last known values.)
    #[allow(unused_variables)]
    fn handle_native_patches(
        &self,
        expanded_node: &ExpandedNode,
        context: &Rc<RefCell<RuntimeContext>>,
    ) {
        //no-op default implementation
    }

    /// Updates the expanded node, recomputing it's properties and possibly updating it's children
    fn update(
        self: Rc<Self>,
        _expanded_node: &Rc<ExpandedNode>,
        _context: &Rc<RefCell<RuntimeContext>>,
    ) {
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
        context: &Rc<RefCell<RuntimeContext>>,
        rcs: &mut dyn RenderContext,
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
        context: &Rc<RefCell<RuntimeContext>>,
        rcs: &mut dyn RenderContext,
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
        expanded_node: &ExpandedNode,
        context: &Rc<RefCell<RuntimeContext>>,
        rcs: &mut dyn RenderContext,
    ) {
        //no-op default implementation
    }

    /// Fires during the tick when a node is first attached to the render tree.  For example,
    /// this event fires by all nodes on the global first tick, and by all nodes in a subtree
    /// when a `Conditional` subsequently turns on a subtree (i.e. when the `Conditional`s criterion becomes `true` after being `false` through the end of at least 1 frame.)
    /// A use-case: send a message to native renderers that a `Text` element should be rendered and tracked
    #[allow(unused_variables)]
    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RefCell<RuntimeContext>>,
    ) {
        let env = Rc::clone(&expanded_node.stack);
        let children = self.base().get_instance_children().borrow();
        let children_with_envs = children.iter().cloned().zip(iter::repeat(env));
        expanded_node.set_children(children_with_envs, context);
    }

    /// Fires during element unmount, when an element is about to be removed from the render tree (e.g. by a `Conditional`)
    /// A use-case: send a message to native renderers that a `Text` element should be removed
    #[allow(unused_variables)]
    fn handle_unmount(
        &self,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RefCell<RuntimeContext>>,
    ) {
        //no-op default implementation
    }
    /// Invoked by event interrupts to pass scroll information to render node
    #[allow(unused_variables)]
    fn handle_scroll(&self, args_scroll: Scroll) {
        //no-op default implementation
    }

    fn get_template(&self) -> Option<&InstanceNodePtrList> {
        None
    }
}

pub struct BaseInstance {
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry>>>,
    pub instance_prototypical_properties_factory:
        Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<dyn Any>>>,
    pub instance_prototypical_common_properties_factory: Box<
        dyn Fn(
            Rc<RuntimePropertiesStackFrame>,
            Rc<ExpressionTable>,
        ) -> Rc<RefCell<CommonProperties>>,
    >,
    pub template_node_identifier: Option<UniqueTemplateNodeIdentifier>,
    pub properties_scope_factory:
        Option<Box<dyn Fn(Rc<RefCell<dyn Any>>) -> HashMap<String, ErasedProperty>>>,
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
            template_node_identifier: args.template_node_identifier,
            properties_scope_factory: args.properties_scope_factory,
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
    pub fn get_instance_children(&self) -> &InstanceNodePtrList {
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
