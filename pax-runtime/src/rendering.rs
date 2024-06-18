use std::collections::HashMap;

use std::iter;
use std::rc::Rc;
use_RefCell!();
use crate::api::{CommonProperties, RenderContext};
use pax_manifest::UniqueTemplateNodeIdentifier;
use pax_message::NativeInterrupt;
use pax_runtime_api::pax_value::PaxAny;
use pax_runtime_api::{borrow, use_RefCell, PropertyId};
use piet::{Color, StrokeStyle};
use rustc_hash::FxHashMap;

use crate::api::{Layer, Scroll};

use crate::{
    ExpandedNode, ExpressionTable, HandlerRegistry, RuntimeContext, RuntimePropertiesStackFrame,
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
        Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>>,
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry>>>,
    pub children: Option<InstanceNodePtrList>,
    pub component_template: Option<InstanceNodePtrList>,

    pub template_node_identifier: Option<UniqueTemplateNodeIdentifier>,
    // Used by RuntimePropertyStackFrame to pull out struct's properties based on their names
    pub properties_scope_factory:
        Option<Box<dyn Fn(Rc<RefCell<PaxAny>>) -> FxHashMap<String, PropertyId>>>,
}

#[derive(Clone)]
pub enum NodeType {
    Component,
    Primitive,
}

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

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result;

    /// Updates the expanded node, recomputing it's properties and possibly updating it's children
    fn update(self: Rc<Self>, _expanded_node: &Rc<ExpandedNode>, _context: &Rc<RuntimeContext>) {}

    /// Second lifecycle method during each render loop, occurs after
    /// properties have been computed, but before rendering
    /// Example use-case: perform side-effects to the drawing contexts.
    /// This is how [`Frame`] performs clipping, for example.
    /// Occurs in a pre-order traversal of the render tree.
    #[allow(unused_variables)]
    fn handle_pre_render(
        &self,
        expanded_node: &ExpandedNode,
        context: &Rc<RuntimeContext>,
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
        context: &Rc<RuntimeContext>,
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
        context: &Rc<RuntimeContext>,
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
        context: &Rc<RuntimeContext>,
    ) {

        log::warn!("Mounting: {:?}", expanded_node);
        let env = Rc::clone(&expanded_node.stack);
        let children = borrow!(self.base().get_instance_children());
        let children_with_envs = children.iter().cloned().zip(iter::repeat(env));
        let new_children =  expanded_node.generate_children(children_with_envs, context);
        expanded_node.attach_children(new_children, context);
    }

    /// Fires during element unmount, when an element is about to be removed from the render tree (e.g. by a `Conditional`)
    /// A use-case: send a message to native renderers that a `Text` element should be removed
    #[allow(unused_variables)]
    fn handle_unmount(&self, expanded_node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
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

    fn handle_text_change(&self, _expanded_node: &Rc<ExpandedNode>, _text: String) {
        // no-op for most, except for TextInstance
        // TODO find a more general framework for exposing callbacks into primitives
        // that doesn't need a method like this for each form type (textbox, checkbox, etc)
    }

    fn handle_native_interrupt(
        &self,
        _expanded_node: &Rc<ExpandedNode>,
        _interrupt: &NativeInterrupt,
    ) {
        // no-op for many
    }
}

pub struct BaseInstance {
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry>>>,
    pub instance_prototypical_properties_factory:
        Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>>,
    pub instance_prototypical_common_properties_factory: Box<
        dyn Fn(
            Rc<RuntimePropertiesStackFrame>,
            Rc<ExpressionTable>,
        ) -> Rc<RefCell<CommonProperties>>,
    >,
    pub template_node_identifier: Option<UniqueTemplateNodeIdentifier>,
    pub properties_scope_factory:
        Option<Box<dyn Fn(Rc<RefCell<PaxAny>>) -> FxHashMap<String, PropertyId>>>,
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
