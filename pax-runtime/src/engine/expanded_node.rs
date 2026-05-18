use crate::api::TextInput;
use crate::node_interface::NodeLocal;
use pax_runtime_api::pax_value::{ImplToFromPaxAny, PaxAny, ToFromPaxAny};
use pax_runtime_api::{
    borrow, borrow_mut, use_RefCell, Focus, Interpolatable, Layer, NativeLiquidGlassScope, Percent,
    Property, SelectStart, Variable,
};

use crate::api::math::Point2;
use crate::constants::{
    ACCEL_HANDLERS, BUTTON_CLICK_HANDLERS, CHECKBOX_CHANGE_HANDLERS, CLICK_HANDLERS,
    CONTEXT_MENU_HANDLERS, DOUBLE_CLICK_HANDLERS, DROP_HANDLERS, FOCUSED_HANDLERS, GYRO_HANDLERS,
    KEY_DOWN_HANDLERS, KEY_PRESS_HANDLERS, KEY_UP_HANDLERS, MOUSE_DOWN_HANDLERS,
    MOUSE_MOVE_HANDLERS, MOUSE_OUT_HANDLERS, MOUSE_OVER_HANDLERS, MOUSE_UP_HANDLERS,
    PHOTO_PICKER_CHANGE_HANDLERS, SCROLL_HANDLERS, SELECT_START_HANDLERS, TAP_HANDLERS,
    TEXTBOX_CHANGE_HANDLERS, TEXTBOX_INPUT_HANDLERS, TEXT_INPUT_HANDLERS, TOUCH_END_HANDLERS,
    TOUCH_MOVE_HANDLERS, TOUCH_START_HANDLERS, WHEEL_HANDLERS,
};
use_RefCell!();
use crate::{ExpandedNodeIdentifier, Globals, LayoutHull, LayoutProperties, TransformAndBounds};
use core::fmt;
use std::cell::Cell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::{Rc, Weak};

use crate::api::{
    Accel, Axis, ButtonClick, CheckboxChange, Click, CommonProperties, ContextMenu, DoubleClick,
    Drop, Event, Gyro, KeyDown, KeyPress, KeyUp, LayoutRole, MouseDown, MouseMove, MouseOut,
    MouseOver, MouseUp, NodeContext, PhotoPickerChange, RenderContext, Scroll, Size, TextboxChange,
    TextboxInput, TouchEnd, TouchMove, TouchStart, Wheel, Window,
};
use pax_manifest::cartridge_generation::{
    TRANSITION_PHASE_ENTER, TRANSITION_PHASE_EXIT, TRANSITION_PHASE_IDLE, TRANSITION_PHASE_SYMBOL,
    TRANSITION_PLAYHEAD_MILLIS_SYMBOL, TRANSITION_PLAYHEAD_SYMBOL,
};
use pax_manifest::{
    SelectorExpr, SettingsBlockElement, TypeId, UniqueTemplateNodeIdentifier, ValueDefinition,
};

use crate::{
    add_symmetric_padding_to_content_layout_hull, apply_container_frame, apply_padding_frame,
    compute_tab, project_child_layout_hull_to_parent_space, ComponentInstance, ContainerFrame,
    HandlerLocation, InstanceNode, InstanceNodePtr, ReceivedChildrenSource, RuntimeContext,
    RuntimePropertiesStackFrame,
};

#[derive(Clone, Debug)]
pub struct RuntimeSettingsLayer {
    pub provider_id: ExpandedNodeIdentifier,
    pub provider_type_id: TypeId,
    pub settings: Vec<SettingsBlockElement>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeSettingsSignatureEntry {
    pub provider_id: ExpandedNodeIdentifier,
    pub provider_type_id: TypeId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeSettingsSource {
    ComponentSettings,
    ImportedLayer {
        provider_id: ExpandedNodeIdentifier,
        provider_type_id: TypeId,
    },
    Inline,
}

#[derive(Clone, Debug)]
pub struct RuntimeResolvedPropertyEntry {
    pub source: RuntimeSettingsSource,
    pub selector: Option<SelectorExpr>,
    pub value: ValueDefinition,
    pub axis_index: Option<usize>,
}

pub type RuntimeResolvedPropertyColumns = BTreeMap<String, Vec<RuntimeResolvedPropertyEntry>>;

#[derive(Clone, Copy)]
enum PointerActivationSource {
    Mouse,
    Touch,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct FilteredRenderStats {
    pub path_nodes_visited: usize,
    pub dirty_nodes_rendered: usize,
    pub skipped_subtrees: usize,
}

#[derive(Clone)]
pub struct ExpandedNode {
    #[allow(dead_code)]
    /// Unique ID of this expanded node, roughly encoding an address in the tree, where the first u32 is the instance ID
    /// and the subsequent u32s represent addresses within an expanded tree via Repeat.
    pub id: ExpandedNodeIdentifier,

    /// Pointer to the unexpanded `instance_node` underlying this ExpandedNode
    pub instance_node: RefCell<InstanceNodePtr>,

    /// Pointer (`Weak` to avoid Rc cycle memory leaks) to the ExpandedNode
    /// rendered directly above this one.
    pub render_parent: RefCell<Weak<ExpandedNode>>,

    /// Pointer (`Weak` to avoid Rc cycle memory leaks) to the ExpandedNode
    /// in the template directly above this one.
    pub template_parent: Weak<ExpandedNode>,

    /// Id of closest frame present in the node tree.
    /// included as a parameter on AnyCreatePatch when
    /// creating a native element to know what clipping context
    /// to attach to
    pub parent_frame: Property<Option<ExpandedNodeIdentifier>>,

    /// Nearest active native liquid-glass scope inherited by descendants.
    pub liquid_glass_scope: Property<Option<NativeLiquidGlassScope>>,

    /// Reference to the _component for which this `ExpandedNode` is a template member._ Used at least for
    /// resolving projected children for `slot`. `Option`al because the very root instance node (root component, root instance node)
    /// has a corollary "root component expanded node."  That very root expanded node _does not have_ a containing ExpandedNode component,
    /// thus `containing_component` is `Option`al.
    pub containing_component: Weak<ExpandedNode>,

    /// Persistent clone of the state of the `PropertiesTreeShared#runtime_properties_stack` at the time that this node was expanded (this is expected to remain immutable
    /// through the lifetime of the program after the initial expansion; however, if that constraint changes, this should be
    /// explicitly updated to accommodate.)
    pub stack: Rc<RuntimePropertiesStackFrame>,

    /// Pointers to the ExpandedNode beneath this one. Used for rendering
    /// recursion and other render-tree traversals.
    pub children: Property<Vec<Rc<ExpandedNode>>>,

    /// The concrete render-tree children currently attached beneath this node.
    ///
    /// This may temporarily include children retained for exit transitions, so
    /// it should not be confused with the semantic payload exposed through
    /// `NodeContext::received_children`.
    pub mounted_children: RefCell<Vec<Rc<ExpandedNode>>>,

    /// Children that represent the currently selected control-flow/template output.
    ///
    /// When this node's received-child source is `Owned`, this is the internal
    /// source for `NodeContext::received_children`.
    pub active_children: RefCell<Vec<Rc<ExpandedNode>>>,
    /// Reactive mirror of `active_children` for consumers that need the active
    /// semantic set.
    pub active_children_view: Property<Vec<Rc<ExpandedNode>>>,

    /// Children retained only so exit transitions can finish before unmount.
    ///
    /// When this node's received-child source is `Owned`, this is the internal
    /// source for `NodeContext::retained_received_children`.
    pub exiting_children: RefCell<Vec<Rc<ExpandedNode>>>,
    /// Reactive mirror of `exiting_children` for consumers that need retained exits.
    pub exiting_children_view: Property<Vec<Rc<ExpandedNode>>>,

    /// Auxiliary children participate in update/layout but are not mounted or rendered directly.
    pub sidecar_children: RefCell<Vec<Rc<ExpandedNode>>>,

    /// Each ExpandedNode has a unique "stamp" of computed properties
    pub properties: RefCell<Rc<RefCell<PaxAny>>>,

    /// Each ExpandedNode has unique, computed `CommonProperties`
    pub common_properties: RefCell<Rc<RefCell<CommonProperties>>>,
    /// Measured bounds reported by chassis/native layout or by container-owned bottom-up layout.
    /// When width/height are omitted, these values are used as the fallback concrete size.
    pub measured_size: Property<Option<(f64, f64)>>,
    /// Selector-facing metadata used by runtime and designtime queries.
    pub selector_metadata: RefCell<RuntimeSelectorMetadata>,

    /// The layout information (width, height, transform) used to render this node.
    /// computed property based on parent bounds + common properties
    pub transform_and_bounds: Property<TransformAndBounds<NodeLocal, Window>>,
    /// Engine-owned subtree hull used by container measurement.
    pub subtree_layout_hull: Property<LayoutHull>,
    /// Optional container-assigned virtual wrapper frame applied before this node's own layout.
    pub container_frame: Property<Option<ContainerFrame>>,

    /// The accumulated opacity inherited from render ancestors and this node's
    /// own common opacity value.
    pub computed_opacity: Property<f64>,

    /// For nodes that own projection, tracks projected children in their
    /// expanded, non-collapsed form.
    ///
    /// Projection is a transport detail used to deliver received payload into a
    /// node's encapsulated implementation, typically for `slot(...)`.
    /// Repeat/conditional descendants are still present here.
    pub expanded_projected_children: RefCell<Option<Vec<Rc<ExpandedNode>>>>,
    /// Flattened version of the projected children above, where repeat and
    /// conditionals are recursively replaced by their current outputs.
    ///
    /// This is the raw projected family, not necessarily the same thing as the
    /// semantic `received_children` view seen by all consumers.
    pub expanded_and_flattened_projected_children: Property<Vec<Rc<ExpandedNode>>>,
    /// Number of expanded and flattened projected children.
    pub flattened_projected_children_count: Property<usize>,

    /// Flag that is > 0 if this node is part of the root tree. If it is,
    /// updates to this nodes children also marks them as attached (+1), triggering
    /// mount and dismount on addition/removal. This is needed mainly for slot,
    /// since an entire "shadow tree" needs to be expanded and updated for
    /// each slot child, but only the ones that have a "connected" slot should
    /// trigger mount/dismount updates
    pub attached: Cell<u32>,

    /// Logical render-layer assignment plus z-order for the current render tree pass.
    ///
    /// Layer 0 is the root surface stack, and non-root layers are reserved for scroller-owned
    /// vector islands.
    pub occlusion: Property<Occlusion>,

    /// Hash of the last native occlusion mask emitted for this node.
    pub native_mask_hash: Cell<u64>,

    /// Last browser-owned descendant content layer published for scroller-style native hosts.
    pub browser_content_layer_id: Cell<Option<u32>>,

    /// Hash of the last presentation metadata emitted for clipping/scrolling chassis consumers.
    pub presentation_cache_hash: Cell<u64>,

    /// A map of all properties available on this expanded node.
    /// Used by the RuntimePropertiesStackFrame to resolve symbols.
    pub properties_scope: RefCell<HashMap<String, Variable>>,

    /// The flattened index of this node in its container (if this container
    /// cares about slot children, ex: component, path).
    pub slot_index: Property<Option<usize>>,

    /// property used to "freeze" (stop firing tick) on a node and all it's children
    pub suspended: Property<bool>,

    /// used by native elements to trigger sending of native messages
    /// used by canvas elements to dirtify their canvas
    pub changed_listener: Property<()>,

    /// Tracks whether this node's occlusion-affecting inputs changed.
    pub occlusion_listener: Property<()>,

    /// Pulls the node's children property only when its upstream dependencies changed.
    pub children_listener: Property<()>,
    /// Rebinds `subtree_layout_hull` when the child list changes.
    pub subtree_layout_hull_listener: Property<()>,
    /// Reactive content-measurement effect used by autosize-style container features.
    pub content_measurement_listener: Property<()>,
    /// Rebinds `content_measurement_listener` when the content child list changes.
    pub content_measurement_rebind_listener: Property<()>,
    /// Guards one-time setup of the reactive content-measurement listener pair.
    pub content_measurement_bound: Cell<bool>,
    /// Cached update traversal state. Structural effects mark ancestors dirty when child
    /// ownership changes; the update pass clears this for subtrees with no remaining
    /// imperative update work.
    subtree_requires_non_reactive_update: Cell<bool>,

    /// Dirty signal emitted when the projected child set changes structurally.
    pub projected_children_changed: Property<()>,

    /// subscription properties: added to this expanded node by calling ctx.subscribe in a node event handler
    pub subscriptions: RefCell<Vec<Property<()>>>,

    /// Current lifecycle transition phase for this node.
    pub transition_phase: Property<u64>,
    /// Frame at which the active lifecycle transition began.
    pub transition_origin_frame: Property<u64>,
    /// Millisecond clock value at which the active lifecycle transition began.
    pub transition_origin_millis: Property<u64>,
    /// Local playhead, in frames, for the active lifecycle transition.
    pub transition_playhead: Property<f64>,
    /// Local playhead, in milliseconds, for the active lifecycle transition.
    pub transition_playhead_millis: Property<f64>,
    /// Wall-clock start for exit timeout enforcement.
    pub exit_started_millis: Cell<Option<u128>>,
    /// Effect property used to release deferred exit children.
    pub exit_cleanup_listener: Property<()>,
    /// Whether exit cleanup should do work on frame ticks.
    pub exit_cleanup_active: Cell<bool>,
    /// Imported provider layers currently active for this component instance.
    pub imported_settings_layers: RefCell<Vec<RuntimeSettingsLayer>>,
    /// Ordered provider-node ids used to detect when the imported layer stack changed.
    pub imported_settings_signature: RefCell<Vec<RuntimeSettingsSignatureEntry>>,
    /// Layered property entries ordered from lowest to highest precedence.
    pub resolved_property_columns: RefCell<RuntimeResolvedPropertyColumns>,
    /// Winning property entry per key after layering and inline override.
    pub resolved_property_provenance: RefCell<BTreeMap<String, RuntimeResolvedPropertyEntry>>,
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct Occlusion {
    /// Logical render layer: 0 for the root surface stack, >0 for scroller-owned vector islands.
    pub render_layer_id: usize,
    pub z_index: i32,
    // this is used to perform last patches logic,
    // and is updated to reflect this nodes parent_frame
    // when occlusion is calculated
    pub parent_frame: Option<u32>,
}

impl Interpolatable for Occlusion {}

impl ImplToFromPaxAny for ExpandedNode {}
impl Interpolatable for ExpandedNode {}

#[derive(Clone)]
pub struct RuntimeSelectorMetadata {
    pub type_id: TypeId,
    pub id: Property<Option<String>>,
    pub classes: Property<Vec<String>>,
}

impl RuntimeSelectorMetadata {
    fn from_base(
        base: &crate::rendering::BaseInstance,
        common_properties: &Rc<RefCell<CommonProperties>>,
    ) -> Self {
        let classes = base
            .template_node_selector_info
            .as_ref()
            .map(|info| {
                info.classes
                    .iter()
                    .map(|token| token.token_value.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Self {
            type_id: base.template_node_type_id.clone().unwrap_or_default(),
            id: common_properties.borrow().id.clone(),
            classes: Property::new_with_name(classes, "selector classes"),
        }
    }

    pub fn matches(&self, selector: &SelectorExpr) -> bool {
        match selector {
            SelectorExpr::Id(id) => self.id.get().as_deref() == Some(id.as_str()),
            SelectorExpr::Class(class_name) => self
                .classes
                .get()
                .into_iter()
                .any(|node_class| node_class == *class_name),
            SelectorExpr::Type(type_name) => {
                self.type_id.to_string() == *type_name
                    || self.type_id.get_pascal_identifier().as_deref() == Some(type_name.as_str())
            }
        }
    }
}

macro_rules! dispatch_event_handler {
    ($fn_name:ident, $arg_type:ty, $handler_key:ident, $recurse:expr) => {
        pub fn $fn_name(
            self: &Rc<Self>,
            event: Event<$arg_type>,
            globals: &Globals,
            ctx: &Rc<RuntimeContext>,
        ) -> bool {
            self.run_event_handlers_for_key($handler_key, &event, ctx);

            if $recurse {
                if let Some(parent) = self.template_parent.upgrade() {
                    return parent.$fn_name(event, globals, ctx);
                }
            }
            event.cancelled()
        }
    };
}

impl ExpandedNode {
    pub fn initialize_root(template: Rc<ComponentInstance>, ctx: &Rc<RuntimeContext>) -> Rc<Self> {
        let root_node = Self::new(
            template,
            ctx.globals().stack_frame(),
            ctx,
            Weak::new(),
            Weak::new(),
        );
        root_node.bind_to_parent_bounds(ctx);
        Rc::clone(&root_node).recurse_mount(ctx);
        root_node
    }

    fn has_event_handlers(&self, handler_key: &str) -> bool {
        borrow!(self.instance_node)
            .base()
            .get_handler_registry()
            .is_some_and(|registry| {
                borrow!(*registry)
                    .handlers
                    .get(handler_key)
                    .is_some_and(|handlers| !handlers.is_empty())
            })
    }

    fn run_event_handlers_for_key<T: Clone + 'static>(
        self: &Rc<Self>,
        handler_key: &str,
        event: &Event<T>,
        ctx: &Rc<RuntimeContext>,
    ) {
        if let Some(registry) = borrow!(self.instance_node).base().get_handler_registry() {
            let borrowed_registry = &borrow!(*registry);
            if let Some(handlers) = borrowed_registry.handlers.get(handler_key) {
                if !handlers.is_empty() {
                    let component_properties = if let Some(cc) = self.containing_component.upgrade()
                    {
                        Rc::clone(&*borrow!(cc.properties))
                    } else {
                        Rc::clone(&*borrow!(self.properties))
                    };

                    let context = self.get_node_context(ctx);
                    handlers.iter().for_each(|handler| {
                        let properties = if let HandlerLocation::Component = &handler.location {
                            Rc::clone(&*borrow!(self.properties))
                        } else {
                            Rc::clone(&component_properties)
                        };
                        (handler.function)(
                            Rc::clone(&properties),
                            &context,
                            Some(event.clone().to_pax_any()),
                        );
                    });
                }
            };
        }
    }

    fn new(
        template: Rc<dyn InstanceNode>,
        env: Rc<RuntimePropertiesStackFrame>,
        context: &Rc<RuntimeContext>,
        containing_component: Weak<ExpandedNode>,
        parent: Weak<ExpandedNode>,
    ) -> Rc<Self> {
        let transition_config = template.base().transition_config().clone();
        let has_transition_bindings = transition_config.has_enter || transition_config.has_exit;
        let transition_phase = Property::new_with_name(TRANSITION_PHASE_IDLE, "transition phase");
        let transition_origin_frame =
            Property::new_with_name(context.globals().frames_elapsed.get(), "transition origin");
        let transition_origin_millis = Property::new_with_name(
            context.globals().elapsed_millis.get(),
            "transition origin millis",
        );
        let (transition_playhead, transition_playhead_millis) = if has_transition_bindings {
            let frames_elapsed = context.globals().frames_elapsed.clone();
            let elapsed_millis = context.globals().elapsed_millis.clone();
            let frames_elapsed_for_playhead = frames_elapsed.clone();
            let transition_origin_frame_for_playhead = transition_origin_frame.clone();
            let transition_playhead = Property::computed_with_name(
                move || {
                    frames_elapsed_for_playhead
                        .get()
                        .saturating_sub(transition_origin_frame_for_playhead.get())
                        as f64
                },
                &[frames_elapsed.untyped(), transition_origin_frame.untyped()],
                "transition playhead",
            );
            let elapsed_millis_for_playhead = elapsed_millis.clone();
            let transition_origin_millis_for_playhead = transition_origin_millis.clone();
            let transition_playhead_millis = Property::computed_with_name(
                move || {
                    elapsed_millis_for_playhead
                        .get()
                        .saturating_sub(transition_origin_millis_for_playhead.get())
                        as f64
                },
                &[elapsed_millis.untyped(), transition_origin_millis.untyped()],
                "transition playhead millis",
            );
            (transition_playhead, transition_playhead_millis)
        } else {
            (
                Property::new_with_name(0.0, "transition playhead"),
                Property::new_with_name(0.0, "transition playhead millis"),
            )
        };

        let env = if has_transition_bindings {
            env.push(
                vec![
                    (
                        TRANSITION_PHASE_SYMBOL.to_string(),
                        Variable::new_from_typed_property(transition_phase.clone()),
                    ),
                    (
                        TRANSITION_PLAYHEAD_SYMBOL.to_string(),
                        Variable::new_from_typed_property(transition_playhead.clone()),
                    ),
                    (
                        TRANSITION_PLAYHEAD_MILLIS_SYMBOL.to_string(),
                        Variable::new_from_typed_property(transition_playhead_millis.clone()),
                    ),
                ]
                .into_iter()
                .collect(),
            )
        } else {
            env
        };

        let properties = template
            .base()
            .instance_prototypical_properties
            .materialize(env.clone(), None)
            .unwrap();

        let common_properties = template
            .base()
            .instance_prototypical_common_properties
            .materialize(env.clone(), None)
            .unwrap();
        let selector_metadata =
            RuntimeSelectorMetadata::from_base(template.base(), &common_properties);

        let mut property_scope = borrow!(*common_properties).retrieve_property_scope();

        property_scope.extend(template.base().properties_scope.build(properties.clone()));

        let id = context.gen_uid();
        let res = Rc::new(ExpandedNode {
            id,
            stack: env,
            instance_node: RefCell::new(Rc::clone(&template)),
            attached: Cell::new(0),
            properties: RefCell::new(properties),
            common_properties: RefCell::new(common_properties),
            measured_size: Property::default(),
            selector_metadata: RefCell::new(selector_metadata),

            // these two refer to their rendering parent, not their
            // template parent
            render_parent: Default::default(),
            parent_frame: Default::default(),
            liquid_glass_scope: Default::default(),
            template_parent: parent,

            containing_component,
            children: Property::new_with_name(
                Vec::new(),
                &format!("node children (node id: {})", id.0),
            ),
            mounted_children: RefCell::new(Vec::new()),
            active_children: RefCell::new(Vec::new()),
            active_children_view: Property::new(Vec::new()),
            exiting_children: RefCell::new(Vec::new()),
            exiting_children_view: Property::new(Vec::new()),
            sidecar_children: RefCell::new(Vec::new()),
            transform_and_bounds: Property::new(TransformAndBounds::default()),
            subtree_layout_hull: Property::new(LayoutHull::default()),
            container_frame: Property::new(None),
            computed_opacity: Property::new(1.0),
            expanded_projected_children: Default::default(),
            expanded_and_flattened_projected_children: Default::default(),
            flattened_projected_children_count: Property::new(0),
            occlusion: Property::new(Occlusion::default()),
            native_mask_hash: Cell::new(0),
            browser_content_layer_id: Cell::new(None),
            presentation_cache_hash: Cell::new(0),
            properties_scope: RefCell::new(property_scope),
            slot_index: Property::default(),
            suspended: Property::new(false),
            changed_listener: Property::default(),
            occlusion_listener: Property::default(),
            children_listener: Property::default(),
            subtree_layout_hull_listener: Property::default(),
            content_measurement_listener: Property::default(),
            content_measurement_rebind_listener: Property::default(),
            content_measurement_bound: Cell::new(false),
            subtree_requires_non_reactive_update: Cell::new(true),
            projected_children_changed: Property::default(),
            subscriptions: Default::default(),
            transition_phase,
            transition_origin_frame,
            transition_origin_millis,
            transition_playhead,
            transition_playhead_millis,
            exit_started_millis: Cell::new(None),
            exit_cleanup_listener: Property::default(),
            exit_cleanup_active: Cell::new(false),
            imported_settings_layers: RefCell::new(Vec::new()),
            imported_settings_signature: RefCell::new(Vec::new()),
            resolved_property_columns: RefCell::new(BTreeMap::new()),
            resolved_property_provenance: RefCell::new(BTreeMap::new()),
        });
        template
            .base()
            .instance_prototypical_common_properties
            .materialize(Rc::clone(&res.stack), Some(Rc::clone(&res)));
        template
            .base()
            .instance_prototypical_properties
            .materialize(Rc::clone(&res.stack), Some(Rc::clone(&res)));
        let common_properties = Rc::clone(&*borrow!(res.common_properties));
        *res.selector_metadata.borrow_mut() =
            RuntimeSelectorMetadata::from_base(template.base(), &common_properties);
        res.refresh_properties_scope(&template);
        res.bind_occlusion_listener(context);
        res.bind_children_listener(context);
        res.bind_subtree_layout_hull(context);
        res
    }

    pub fn recreate_with_new_data(
        self: &Rc<Self>,
        template: Rc<dyn InstanceNode>,
        context: &Rc<RuntimeContext>,
    ) {
        *borrow_mut!(self.instance_node) = Rc::clone(&template);
        template
            .base()
            .instance_prototypical_common_properties
            .materialize(Rc::clone(&self.stack), Some(Rc::clone(&self)));
        template
            .base()
            .instance_prototypical_properties
            .materialize(Rc::clone(&self.stack), Some(Rc::clone(&self)));
        let common_properties = Rc::clone(&*borrow!(self.common_properties));
        *self.selector_metadata.borrow_mut() =
            RuntimeSelectorMetadata::from_base(template.base(), &common_properties);
        self.refresh_properties_scope(&template);
        self.bind_to_parent_bounds(context);
        self.bind_occlusion_listener(context);
        self.mark_non_reactive_update_subtree_dirty();
        context.mark_occlusion_dirty();
        context.set_canvas_dirty(self.occlusion.get().render_layer_id);
    }

    pub fn fully_recreate_with_new_data(
        self: &Rc<Self>,
        template: Rc<dyn InstanceNode>,
        context: &Rc<RuntimeContext>,
    ) {
        Rc::clone(self).recurse_unmount(context);
        let new_expanded_node = Self::new(
            template.clone(),
            Rc::clone(&self.stack),
            context,
            Weak::clone(&self.containing_component),
            Weak::clone(&self.template_parent),
        );
        *borrow_mut!(self.instance_node) = Rc::clone(&*borrow!(new_expanded_node.instance_node));
        *borrow_mut!(self.properties) = Rc::clone(&*borrow!(new_expanded_node.properties));
        *borrow_mut!(self.properties_scope) = borrow!(new_expanded_node.properties_scope).clone();
        *borrow_mut!(self.common_properties) =
            Rc::clone(&*borrow!(new_expanded_node.common_properties));
        *self.selector_metadata.borrow_mut() = new_expanded_node.selector_metadata.borrow().clone();
        self.occlusion.set(Default::default());

        self.bind_to_parent_bounds(context);
        self.bind_occlusion_listener(context);
        self.mark_non_reactive_update_subtree_dirty();
        context.mark_occlusion_dirty();
        Rc::clone(self).recurse_mount(context);
        context.drain_node_effects();
        self.recurse_emit_mount_updates();
    }

    fn recurse_emit_mount_updates(self: &Rc<Self>) {
        // `fully_recreate_with_new_data` can remount native-backed nodes after the main
        // update traversal has already consumed `changed_listener`. Emit the initial
        // native updates here so the chassis does not paint a frame with skeletal defaults.
        self.changed_listener.get();
        self.occlusion_listener.get();
        for child in self.children.get().iter() {
            child.recurse_emit_mount_updates();
        }
        for child in borrow!(self.sidecar_children).iter() {
            child.recurse_emit_mount_updates();
        }
    }

    fn refresh_properties_scope(self: &Rc<Self>, template: &Rc<dyn InstanceNode>) {
        let common_properties = Rc::clone(&*borrow!(self.common_properties));
        let mut refreshed_scope = borrow!(*common_properties).retrieve_property_scope();
        refreshed_scope.extend(
            template
                .base()
                .properties_scope
                .build(Rc::clone(&*borrow!(self.properties))),
        );
        *borrow_mut!(self.properties_scope) = refreshed_scope;
    }

    /// Returns whether this node is a descendant of the ExpandedNode described by `other_expanded_node_id` (id)
    /// Currently requires traversing linked list of ancestry, incurring a O(log(n)) cost for a tree of `n` elements.
    /// This could be mitigated with caching/memoization, perhaps by storing a HashSet on each ExpandedNode describing its ancestry chain.
    pub fn is_descendant_of(&self, other_expanded_node_id: &ExpandedNodeIdentifier) -> bool {
        if let Some(parent) = borrow!(self.render_parent).upgrade() {
            // We have a parent — if it matches the ID, this node is indeed an ancestor of other_expanded_node_id.  Otherwise, recurse upward.
            if parent.id.eq(other_expanded_node_id) {
                true
            } else {
                parent.is_descendant_of(other_expanded_node_id)
            }
        } else {
            false
        }
    }

    pub fn create_children_detached(
        self: &Rc<Self>,
        templates: impl IntoIterator<Item = (Rc<dyn InstanceNode>, Rc<RuntimePropertiesStackFrame>)>,
        context: &Rc<RuntimeContext>,
        template_parent: &Weak<ExpandedNode>,
    ) -> Vec<Rc<ExpandedNode>> {
        let containing_component = if borrow!(self.instance_node).base().flags().is_component {
            Rc::downgrade(&self)
        } else {
            Weak::clone(&self.containing_component)
        };

        let mut children = Vec::new();

        for (template, env) in templates {
            children.push(Self::new(
                template,
                env,
                context,
                Weak::clone(&containing_component),
                Weak::clone(&template_parent),
            ));
        }
        children
    }

    pub(crate) fn current_attached_children(&self) -> Vec<Rc<ExpandedNode>> {
        borrow!(self.mounted_children).clone()
    }

    fn sync_mounted_children_from_active_and_exiting(&self) -> Vec<Rc<ExpandedNode>> {
        let active_children = borrow!(self.active_children).clone();
        let exiting_children = borrow!(self.exiting_children).clone();
        self.active_children_view.set(active_children.clone());
        self.exiting_children_view.set(exiting_children.clone());
        let mut combined = exiting_children;
        combined.extend(active_children);
        *borrow_mut!(self.mounted_children) = combined.clone();
        combined
    }

    fn has_child(children: &[Rc<ExpandedNode>], target: &Rc<ExpandedNode>) -> bool {
        children.iter().any(|child| Rc::ptr_eq(child, target))
    }

    fn template_node_identifier(&self) -> Option<UniqueTemplateNodeIdentifier> {
        borrow!(self.instance_node)
            .base()
            .template_node_identifier
            .clone()
    }

    fn transition_source_matches(
        sources: &[UniqueTemplateNodeIdentifier],
        own_identifier: Option<&UniqueTemplateNodeIdentifier>,
        requested_source: Option<&UniqueTemplateNodeIdentifier>,
    ) -> bool {
        if let Some(source) = requested_source {
            return sources.contains(source);
        }
        if sources.is_empty() {
            return true;
        }
        own_identifier
            .map(|identifier| sources.contains(identifier))
            .unwrap_or(false)
    }

    fn has_exit_transition_for_source(
        &self,
        requested_source: Option<&UniqueTemplateNodeIdentifier>,
    ) -> bool {
        let instance_node = borrow!(self.instance_node);
        let transition_config = instance_node.base().transition_config();
        transition_config.has_exit
            && Self::transition_source_matches(
                &transition_config.exit_sources,
                instance_node.base().template_node_identifier.as_ref(),
                requested_source,
            )
    }

    fn mark_non_reactive_update_subtree_dirty(&self) {
        if self.subtree_requires_non_reactive_update.replace(true) {
            return;
        }
        if let Some(parent) = borrow!(self.render_parent).upgrade() {
            parent.mark_non_reactive_update_subtree_dirty();
        }
    }

    fn start_self_enter_transition_for_source(
        &self,
        context: &Rc<RuntimeContext>,
        requested_source: Option<&UniqueTemplateNodeIdentifier>,
    ) -> bool {
        let instance_node = borrow!(self.instance_node);
        let transition_config = instance_node.base().transition_config();
        if !transition_config.has_enter
            || !Self::transition_source_matches(
                &transition_config.enter_sources,
                instance_node.base().template_node_identifier.as_ref(),
                requested_source,
            )
        {
            return false;
        }
        drop(instance_node);
        self.transition_phase.set(TRANSITION_PHASE_ENTER);
        self.transition_origin_frame
            .set(context.globals().frames_elapsed.get());
        self.transition_origin_millis
            .set(context.globals().elapsed_millis.get());
        self.exit_started_millis.set(None);
        true
    }

    fn start_self_enter_transition(self: &Rc<Self>, context: &Rc<RuntimeContext>) {
        let started = self.start_self_enter_transition_for_source(context, None);
        if started {
            if let Some(source) = self.template_node_identifier() {
                self.start_bound_enter_transitions(context, &source);
            }
        }
    }

    fn start_self_exit_transition_for_source(
        self: &Rc<Self>,
        context: &Rc<RuntimeContext>,
        requested_source: Option<&UniqueTemplateNodeIdentifier>,
    ) -> bool {
        if !self.has_exit_transition_for_source(requested_source) {
            return false;
        }
        self.transition_phase.set(TRANSITION_PHASE_EXIT);
        self.transition_origin_frame
            .set(context.globals().frames_elapsed.get());
        self.transition_origin_millis
            .set(context.globals().elapsed_millis.get());
        self.exit_started_millis
            .set(Some((context.globals().get_elapsed_millis)()));
        true
    }

    fn start_self_exit_transition(self: &Rc<Self>, context: &Rc<RuntimeContext>) -> bool {
        let started = self.start_self_exit_transition_for_source(context, None);
        if started {
            if let Some(source) = self.template_node_identifier() {
                self.start_bound_exit_transitions(context, &source);
            }
        }
        started
    }

    fn start_bound_enter_transitions(
        self: &Rc<Self>,
        context: &Rc<RuntimeContext>,
        source: &UniqueTemplateNodeIdentifier,
    ) {
        if let Some(containing_component) = self.containing_component.upgrade() {
            containing_component.start_descendant_enter_transitions_for_source(context, source);
        }
    }

    fn start_bound_exit_transitions(
        self: &Rc<Self>,
        context: &Rc<RuntimeContext>,
        source: &UniqueTemplateNodeIdentifier,
    ) {
        if let Some(containing_component) = self.containing_component.upgrade() {
            containing_component.start_descendant_exit_transitions_for_source(context, source);
        }
    }

    fn start_descendant_enter_transitions_for_source(
        self: &Rc<Self>,
        context: &Rc<RuntimeContext>,
        source: &UniqueTemplateNodeIdentifier,
    ) {
        for child in borrow!(self.mounted_children).iter() {
            child.start_enter_transition_for_source_recursive(context, source);
        }
        for child in borrow!(self.sidecar_children).iter() {
            child.start_enter_transition_for_source_recursive(context, source);
        }
    }

    fn start_descendant_exit_transitions_for_source(
        self: &Rc<Self>,
        context: &Rc<RuntimeContext>,
        source: &UniqueTemplateNodeIdentifier,
    ) {
        for child in borrow!(self.mounted_children).iter() {
            child.start_exit_transition_for_source_recursive(context, source);
        }
        for child in borrow!(self.sidecar_children).iter() {
            child.start_exit_transition_for_source_recursive(context, source);
        }
    }

    fn start_enter_transition_for_source_recursive(
        self: &Rc<Self>,
        context: &Rc<RuntimeContext>,
        source: &UniqueTemplateNodeIdentifier,
    ) {
        if self.template_node_identifier().as_ref() != Some(source) {
            self.start_self_enter_transition_for_source(context, Some(source));
        }
        for child in borrow!(self.mounted_children).iter() {
            child.start_enter_transition_for_source_recursive(context, source);
        }
        for child in borrow!(self.sidecar_children).iter() {
            child.start_enter_transition_for_source_recursive(context, source);
        }
    }

    fn trigger_bound_enter_transitions_for_mounted_sources_recursive(
        self: &Rc<Self>,
        context: &Rc<RuntimeContext>,
    ) {
        if let Some(source) = self.template_node_identifier() {
            let should_trigger = {
                let instance_node = borrow!(self.instance_node);
                instance_node
                    .base()
                    .transition_config()
                    .enter_sources
                    .contains(&source)
            };
            if should_trigger {
                self.start_bound_enter_transitions(context, &source);
            }
        }
        for child in borrow!(self.mounted_children).iter() {
            child.trigger_bound_enter_transitions_for_mounted_sources_recursive(context);
        }
        for child in borrow!(self.sidecar_children).iter() {
            child.trigger_bound_enter_transitions_for_mounted_sources_recursive(context);
        }
    }

    fn start_exit_transition_for_source_recursive(
        self: &Rc<Self>,
        context: &Rc<RuntimeContext>,
        source: &UniqueTemplateNodeIdentifier,
    ) {
        if self.template_node_identifier().as_ref() != Some(source) {
            self.start_self_exit_transition_for_source(context, Some(source));
        }
        for child in borrow!(self.mounted_children).iter() {
            child.start_exit_transition_for_source_recursive(context, source);
        }
        for child in borrow!(self.sidecar_children).iter() {
            child.start_exit_transition_for_source_recursive(context, source);
        }
    }

    fn start_exit_transition_tree(self: &Rc<Self>, context: &Rc<RuntimeContext>) -> bool {
        let mut started = self.start_self_exit_transition(context);
        for child in borrow!(self.mounted_children).iter() {
            started |= child.start_exit_transition_tree(context);
        }
        started
    }

    fn self_exit_transition_complete(&self, context: &Rc<RuntimeContext>) -> bool {
        if self.transition_phase.get() != TRANSITION_PHASE_EXIT {
            return true;
        }
        let transition_config = borrow!(self.instance_node)
            .base()
            .transition_config()
            .clone();
        let duration_complete = if let Some(exit_millis_count) = transition_config.exit_millis_count
        {
            self.transition_playhead_millis.get() >= exit_millis_count as f64
        } else {
            self.transition_playhead.get() >= transition_config.exit_frame_count as f64
        };
        let timeout_complete = self
            .exit_started_millis
            .get()
            .map(|started| {
                (context.globals().get_elapsed_millis)().saturating_sub(started)
                    >= transition_config.timeout_ms as u128
            })
            .unwrap_or(false);
        duration_complete || timeout_complete
    }

    fn exit_transition_tree_complete(self: &Rc<Self>, context: &Rc<RuntimeContext>) -> bool {
        self.self_exit_transition_complete(context)
            && borrow!(self.mounted_children)
                .iter()
                .all(|child| child.exit_transition_tree_complete(context))
    }

    fn enable_exit_cleanup_listener(self: &Rc<Self>, context: &Rc<RuntimeContext>) {
        if self.exit_cleanup_active.get() {
            return;
        }
        self.exit_cleanup_active.set(true);
        let weak_self = Rc::downgrade(self);
        let cloned_context = Rc::clone(context);
        let frames_elapsed = context.globals().frames_elapsed.clone();
        let frames_elapsed_dep = frames_elapsed.untyped();
        self.exit_cleanup_listener
            .replace_with(Property::computed_with_name(
                move || {
                    let _ = frames_elapsed.get();
                    if let Some(node) = weak_self.upgrade() {
                        node.prune_completed_exit_children(&cloned_context);
                    }
                },
                &[frames_elapsed_dep],
                "exit transition cleanup",
            ));
        context.register_node_effect_property(self.id, &self.exit_cleanup_listener);
    }

    fn prune_completed_exit_children(self: &Rc<Self>, context: &Rc<RuntimeContext>) {
        let exiting = std::mem::take(&mut *borrow_mut!(self.exiting_children));
        if exiting.is_empty() {
            self.exit_cleanup_active.set(false);
            return;
        }

        let mut retained = Vec::new();
        for child in exiting {
            if child.exit_transition_tree_complete(context) {
                child.recurse_unmount(context);
            } else {
                retained.push(child);
            }
        }
        *borrow_mut!(self.exiting_children) = retained;
        let combined = self.sync_mounted_children_from_active_and_exiting();
        self.children.set(combined);
        context.mark_occlusion_dirty();

        if borrow!(self.exiting_children).is_empty() {
            self.exit_cleanup_active.set(false);
        }
    }

    pub fn attach_children(
        self: &Rc<Self>,
        new_children: Vec<Rc<ExpandedNode>>,
        context: &Rc<RuntimeContext>,
        parent_frame: &Property<Option<ExpandedNodeIdentifier>>,
    ) -> Vec<Rc<ExpandedNode>> {
        self.attach_children_with_liquid_glass_scope(
            new_children,
            context,
            parent_frame,
            &self.liquid_glass_scope,
        )
    }

    pub fn attach_children_with_liquid_glass_scope(
        self: &Rc<Self>,
        new_children: Vec<Rc<ExpandedNode>>,
        context: &Rc<RuntimeContext>,
        parent_frame: &Property<Option<ExpandedNodeIdentifier>>,
        liquid_glass_scope: &Property<Option<NativeLiquidGlassScope>>,
    ) -> Vec<Rc<ExpandedNode>> {
        for child in new_children.iter() {
            // set parent and connect up viewport bounds to new parent
            *borrow_mut!(child.render_parent) = Rc::downgrade(self);
            // set frame clipping reference
            let parent_frame = parent_frame.clone();
            let deps = [parent_frame.untyped()];
            child
                .parent_frame
                .replace_with(Property::computed(move || parent_frame.get(), &deps));
            let liquid_glass_scope = liquid_glass_scope.clone();
            let deps = [liquid_glass_scope.untyped()];
            child
                .liquid_glass_scope
                .replace_with(Property::computed(move || liquid_glass_scope.get(), &deps));

            // suspension is used in the designer to turn of/on tick/update
            child.inherit_suspend(self);
            child.bind_to_parent_bounds(context);
        }
        let mut newly_mounted_children = Vec::new();
        if self.attached.get() > 0 {
            let old_active_children = borrow!(self.active_children).clone();
            for child in old_active_children.iter() {
                if Self::has_child(&new_children, child) {
                    continue;
                }
                if child.transition_phase.get() == TRANSITION_PHASE_EXIT {
                    if child.exit_transition_tree_complete(context) {
                        Rc::clone(child).recurse_unmount(context);
                    } else {
                        borrow_mut!(self.exiting_children).push(Rc::clone(child));
                        self.enable_exit_cleanup_listener(context);
                    }
                    continue;
                }
                if child.start_exit_transition_tree(context) {
                    borrow_mut!(self.exiting_children).push(Rc::clone(child));
                    self.enable_exit_cleanup_listener(context);
                } else {
                    Rc::clone(child).recurse_unmount(context);
                }
            }
            for child in new_children.iter() {
                if !Self::has_child(&old_active_children, child) {
                    Rc::clone(child).recurse_mount(context);
                    newly_mounted_children.push(Rc::clone(child));
                }
            }
        }
        *borrow_mut!(self.active_children) = new_children;
        let mounted = self.sync_mounted_children_from_active_and_exiting();
        for child in newly_mounted_children {
            child.trigger_bound_enter_transitions_for_mounted_sources_recursive(context);
        }
        self.mark_non_reactive_update_subtree_dirty();
        mounted
    }

    pub fn attach_sidecar_children(
        self: &Rc<Self>,
        new_children: Vec<Rc<ExpandedNode>>,
        context: &Rc<RuntimeContext>,
        parent_frame: &Property<Option<ExpandedNodeIdentifier>>,
    ) -> Vec<Rc<ExpandedNode>> {
        for child in new_children.iter() {
            *borrow_mut!(child.render_parent) = Rc::downgrade(self);
            let parent_frame = parent_frame.clone();
            let deps = [parent_frame.untyped()];
            child
                .parent_frame
                .replace_with(Property::computed(move || parent_frame.get(), &deps));
            let liquid_glass_scope = self.liquid_glass_scope.clone();
            let deps = [liquid_glass_scope.untyped()];
            child
                .liquid_glass_scope
                .replace_with(Property::computed(move || liquid_glass_scope.get(), &deps));
            child.inherit_suspend(self);
            child.bind_to_parent_bounds(context);
        }
        *borrow_mut!(self.sidecar_children) = new_children.clone();
        self.mark_non_reactive_update_subtree_dirty();
        new_children
    }

    fn bind_to_parent_bounds(self: &Rc<Self>, ctx: &Rc<RuntimeContext>) {
        let render_parent = borrow!(self.render_parent).upgrade();
        let parent_transform_and_bounds = render_parent
            .as_ref()
            .map(|n| n.transform_and_bounds.clone())
            .unwrap_or_else(|| ctx.globals().viewport);
        let (parent_padding_x, parent_padding_y) = render_parent
            .as_ref()
            .map(|n| {
                let common_props = n.get_common_properties();
                let common_props = borrow!(common_props);
                (
                    common_props.padding_x.clone(),
                    common_props.padding_y.clone(),
                )
            })
            .unwrap_or_default();
        let container_frame = self.container_frame.clone();
        let deps = [
            parent_transform_and_bounds.untyped(),
            parent_padding_x.untyped(),
            parent_padding_y.untyped(),
            container_frame.untyped(),
        ];
        let effective_parent_transform_and_bounds = Property::computed(
            move || {
                apply_container_frame(
                    apply_padding_frame(
                        parent_transform_and_bounds.get(),
                        parent_padding_x.get(),
                        parent_padding_y.get(),
                    ),
                    container_frame.get(),
                )
            },
            &deps,
        );
        let common_props = borrow!(self.common_properties);
        let extra_transform = borrow!(common_props).transform.clone();

        let transform_and_bounds = compute_tab(
            self.layout_properties(),
            extra_transform,
            effective_parent_transform_and_bounds,
        );
        self.transform_and_bounds.replace_with(transform_and_bounds);

        let parent_opacity = borrow!(self.render_parent)
            .upgrade()
            .map(|n| n.computed_opacity.clone())
            .unwrap_or_else(|| Property::new(1.0));
        let common_props = self.get_common_properties();
        let self_opacity = borrow!(common_props).opacity.clone();
        let deps = [parent_opacity.untyped(), self_opacity.untyped()];
        self.computed_opacity.replace_with(Property::computed(
            move || {
                let parent = parent_opacity.get().clamp(0.0, 1.0);
                let local = self_opacity
                    .get()
                    .unwrap_or_default()
                    .to_float_0_1()
                    .clamp(0.0, 1.0);
                (parent * local).clamp(0.0, 1.0)
            },
            &deps,
        ));
    }

    fn bind_occlusion_listener(self: &Rc<Self>, ctx: &Rc<RuntimeContext>) {
        let instance_node = borrow!(self.instance_node);
        let mut deps: Vec<_> = borrow!(self.properties_scope)
            .iter()
            .filter(|(name, _)| instance_node.property_requires_occlusion_recompute(name))
            .map(|(_, v)| v.get_untyped_property().clone())
            .collect();
        drop(instance_node);

        deps.extend([
            self.children.untyped(),
            self.transform_and_bounds.untyped(),
            self.computed_opacity.untyped(),
            self.liquid_glass_scope.untyped(),
        ]);

        let context = Rc::clone(ctx);
        self.occlusion_listener.replace_with(Property::computed(
            move || {
                context.mark_occlusion_dirty();
            },
            &deps,
        ));
        ctx.register_node_effect_property(self.id, &self.occlusion_listener);
    }

    fn bind_children_listener(self: &Rc<Self>, ctx: &Rc<RuntimeContext>) {
        let deps = [self.children.untyped()];
        let weak_self = Rc::downgrade(self);
        let context = Rc::clone(ctx);
        self.children_listener.replace_with(Property::computed(
            move || {
                let Some(node) = weak_self.upgrade() else {
                    return;
                };
                let _ = node.children.get();
                if borrow!(node.instance_node).base().flags().is_component
                    || borrow!(node.expanded_projected_children).is_some()
                {
                    node.compute_flattened_projected_children();
                }
                context.mark_occlusion_dirty();
            },
            &deps,
        ));
        ctx.register_node_effect_property(self.id, &self.children_listener);
    }

    fn bind_subtree_layout_hull(self: &Rc<Self>, ctx: &Rc<RuntimeContext>) {
        self.rebind_subtree_layout_hull();
        let deps = [self.children.untyped()];
        let weak_self = Rc::downgrade(self);
        self.subtree_layout_hull_listener
            .replace_with(Property::computed_with_name(
                move || {
                    let Some(node) = weak_self.upgrade() else {
                        return;
                    };
                    let _ = node.children.get();
                    node.rebind_subtree_layout_hull();
                },
                &deps,
                "subtree layout hull listener",
            ));
        ctx.register_node_effect_property(self.id, &self.subtree_layout_hull_listener);
    }
    fn rebind_subtree_layout_hull(self: &Rc<Self>) {
        let self_transform_and_bounds = self.transform_and_bounds.clone();
        let layout_properties = self.layout_properties();
        let common_props = self.get_common_properties();
        let (width, height, x, y, padding_x, padding_y) = {
            let common_props = borrow!(common_props);
            (
                common_props.width.clone(),
                common_props.height.clone(),
                common_props.x.clone(),
                common_props.y.clone(),
                common_props.padding_x.clone(),
                common_props.padding_y.clone(),
            )
        };
        let children = self.children.get();
        let has_children = !children.is_empty();
        let mut deps = vec![self_transform_and_bounds.untyped()];
        if has_children {
            deps.extend([
                width.untyped(),
                height.untyped(),
                x.untyped(),
                y.untyped(),
                padding_x.untyped(),
                padding_y.untyped(),
            ]);
        } else {
            deps.push(layout_properties.untyped());
        }
        for child in children.iter() {
            let child_cp = child.get_common_properties();
            deps.push(borrow!(child_cp).layout_role.untyped());
            deps.push(child.transform_and_bounds.untyped());
            deps.push(child.subtree_layout_hull.untyped());
        }

        let property_name = format!("subtree layout hull (node id: {})", self.id.0);
        self.subtree_layout_hull
            .replace_with(Property::computed_with_name(
                move || {
                    let self_tab = self_transform_and_bounds.get();
                    let (contributes_x, contributes_y, padding_x, padding_y) = if has_children {
                        (
                            layout_axis_can_contribute_from_parts(width.get(), x.get()),
                            layout_axis_can_contribute_from_parts(height.get(), y.get()),
                            padding_x.get(),
                            padding_y.get(),
                        )
                    } else {
                        let layout_properties = layout_properties.get();
                        (
                            layout_axis_can_contribute(&layout_properties, Axis::X),
                            layout_axis_can_contribute(&layout_properties, Axis::Y),
                            None,
                            None,
                        )
                    };
                    let mut hull = LayoutHull::from_axis_ranges(
                        contributes_x.then_some((0.0, self_tab.bounds.0)),
                        contributes_y.then_some((0.0, self_tab.bounds.1)),
                    );
                    let child_projection_tab = if has_children {
                        apply_padding_frame(self_tab, padding_x, padding_y)
                    } else {
                        self_tab
                    };
                    let mut children_hull = LayoutHull::default();

                    for child in children.iter() {
                        if child.is_layout_breakout() {
                            // Breakout descendants are rendered and hit-tested normally but do
                            // not participate in ancestor layout hull aggregation.
                            continue;
                        }
                        let projected_hull = project_child_layout_hull_to_parent_space(
                            child_projection_tab,
                            child.transform_and_bounds.get(),
                            child.subtree_layout_hull.get(),
                        );
                        children_hull = children_hull.union(projected_hull);
                    }

                    if has_children {
                        hull = hull.union(add_symmetric_padding_to_content_layout_hull(
                            children_hull,
                            padding_x,
                            padding_y,
                        ));
                    } else {
                        hull = hull.union(children_hull);
                    }

                    hull
                },
                &deps,
                &property_name,
            ));
    }

    pub fn inherit_suspend(self: &Rc<Self>, node: &Rc<Self>) {
        let cp = self.get_common_properties();
        let self_suspended = borrow!(cp)._suspended.clone();
        let parent_suspended = node.suspended.clone();
        let deps = [parent_suspended.untyped(), self_suspended.untyped()];
        self.suspended.replace_with(Property::computed(
            move || {
                self_suspended
                    .get()
                    .unwrap_or_else(|| parent_suspended.get())
            },
            &deps,
        ));
    }

    pub fn generate_children(
        self: &Rc<Self>,
        templates: impl IntoIterator<Item = (Rc<dyn InstanceNode>, Rc<RuntimePropertiesStackFrame>)>,
        context: &Rc<RuntimeContext>,
        parent_frame: &Property<Option<ExpandedNodeIdentifier>>,
        is_mount: bool,
    ) -> Vec<Rc<ExpandedNode>> {
        self.generate_children_with_liquid_glass_scope(
            templates,
            context,
            parent_frame,
            &self.liquid_glass_scope,
            is_mount,
        )
    }

    pub fn generate_children_with_liquid_glass_scope(
        self: &Rc<Self>,
        templates: impl IntoIterator<Item = (Rc<dyn InstanceNode>, Rc<RuntimePropertiesStackFrame>)>,
        context: &Rc<RuntimeContext>,
        parent_frame: &Property<Option<ExpandedNodeIdentifier>>,
        liquid_glass_scope: &Property<Option<NativeLiquidGlassScope>>,
        is_mount: bool,
    ) -> Vec<Rc<ExpandedNode>> {
        let new_children = self.create_children_detached(templates, context, &Rc::downgrade(&self));
        let res = if is_mount {
            self.attach_children_with_liquid_glass_scope(
                new_children,
                context,
                parent_frame,
                liquid_glass_scope,
            )
        } else {
            for child in new_children.iter() {
                child.recurse_control_flow_expansion(context);
            }

            new_children
        };
        res
    }

    /// This method recursively updates all node properties. When dirty-dag exists, this won't
    /// need to be here since all property dependencies can be set up and removed during mount/unmount
    pub fn run_lifecycle_handlers(
        self: &Rc<Self>,
        handler_key: &str,
        context: &Rc<RuntimeContext>,
    ) {
        if let Some(ref registry) = borrow!(self.instance_node).base().handler_registry {
            if !self.suspended.get() {
                for handler in borrow!(registry)
                    .handlers
                    .get(handler_key)
                    .unwrap_or(&Vec::new())
                {
                    (handler.function)(
                        Rc::clone(&*borrow!(self.properties)),
                        &self.get_node_context(context),
                        None,
                    )
                }
            }
        }
    }

    pub fn recurse_update(self: &Rc<Self>, context: &Rc<RuntimeContext>) {
        // Settle queued reactive effects before checking which subtrees still need
        // imperative work, then flush any effects produced by that non-reactive pass.
        context.drain_node_effects();
        self.recurse_update_mounted(context);
        context.drain_node_effects();
    }

    fn recurse_update_mounted(self: &Rc<Self>, context: &Rc<RuntimeContext>) {
        if !self.subtree_requires_non_reactive_update.get() {
            return;
        }
        let instance_node = Rc::clone(&*borrow!(self.instance_node));
        if instance_node.requires_non_reactive_update(self) {
            instance_node.update(&self, context);
        }
        let mut subtree_requires_non_reactive_update =
            Rc::clone(&*borrow!(self.instance_node)).requires_non_reactive_update(self);
        let children = borrow!(self.mounted_children).clone();
        for child in children.iter() {
            child.recurse_update_mounted(context);
            subtree_requires_non_reactive_update |=
                child.subtree_requires_non_reactive_update.get();
        }
        let sidecar_children = borrow!(self.sidecar_children).clone();
        for child in sidecar_children.iter() {
            child.recurse_update_mounted(context);
            subtree_requires_non_reactive_update |=
                child.subtree_requires_non_reactive_update.get();
        }
        self.subtree_requires_non_reactive_update
            .set(subtree_requires_non_reactive_update);
    }

    pub fn recurse_sync_import_settings(self: &Rc<Self>, context: &Rc<RuntimeContext>) {
        if borrow!(self.instance_node).base().flags().is_component {
            self.sync_imported_settings(context);
        }
        for child in self.children.get().iter() {
            child.recurse_sync_import_settings(context);
        }
        for child in borrow!(self.sidecar_children).iter() {
            child.recurse_sync_import_settings(context);
        }
    }

    fn recurse_reapply_runtime_settings_for_component_owner(
        self: &Rc<Self>,
        owner_component_id: ExpandedNodeIdentifier,
        context: &Rc<RuntimeContext>,
    ) {
        if self.is_import_settings_node() {
            return;
        }

        let owned_by_component = self
            .containing_component
            .upgrade()
            .map(|component| component.id == owner_component_id)
            .unwrap_or(false);

        if owned_by_component {
            let instance = borrow!(self.instance_node).clone();
            self.recreate_with_new_data(instance, context);
        }

        let children = self.children.get();
        for child in children.iter() {
            child.recurse_reapply_runtime_settings_for_component_owner(owner_component_id, context);
        }

        let sidecar_children = borrow!(self.sidecar_children).clone();
        for child in sidecar_children.iter() {
            child.recurse_reapply_runtime_settings_for_component_owner(owner_component_id, context);
        }
    }

    pub fn recurse_control_flow_expansion(self: &Rc<Self>, context: &Rc<RuntimeContext>) {
        borrow!(self.instance_node)
            .clone()
            .handle_control_flow_node_expansion(&self, context);
    }

    pub fn recurse_mount(self: &Rc<Self>, context: &Rc<RuntimeContext>) {
        if self.attached.get() == 0 {
            // Materialize projected children before mount so Slot can resolve them.
            borrow!(self.instance_node)
                .clone()
                .handle_setup_projected_children(&self, context);

            // Pre-mount pass to make sure projected children are expanded and we compute the correct flattened list.
            if let Some(projected_children) = borrow!(self.expanded_projected_children).as_ref() {
                for projected_child in projected_children {
                    projected_child.recurse_control_flow_expansion(context);
                }
                // This is needed to resolve slot connections in a single tick.
                self.compute_flattened_projected_children();
            }
            self.attached.set(self.attached.get() + 1);
            self.start_self_enter_transition(context);
            context.add_to_cache(&self);
            if let Some(ref registry) = borrow!(self.instance_node).base().handler_registry {
                for handler in borrow!(registry)
                    .handlers
                    .get("mount")
                    .unwrap_or(&Vec::new())
                {
                    (handler.function)(
                        Rc::clone(&*borrow!(self.properties)),
                        &self.get_node_context(context),
                        None,
                    )
                }
            }
            borrow!(self.instance_node)
                .clone()
                .handle_mount(&self, context);
            context.register_node_effect_property(self.id, &self.changed_listener);
        }
    }

    pub fn recurse_unmount(self: Rc<Self>, context: &Rc<RuntimeContext>) {
        // WARNING: do NOT make recurse_unmount result in expr evaluation,
        // in this case: do not refer to self.children expression.
        // expr evaluation in this context can trigger get's of "old data", ie try to get
        // an index of a for loop source that doesn't exist anymore
        if self.attached.get() == 1 {
            self.attached.set(self.attached.get() - 1);
            context.remove_from_cache(&self);
            for child in borrow!(self.mounted_children).iter() {
                Rc::clone(child).recurse_unmount(context);
            }
            borrow!(self.instance_node).handle_unmount(&self, context);
            if let Some(ref registry) = borrow!(self.instance_node).base().handler_registry {
                for handler in borrow!(registry)
                    .handlers
                    .get("unmount")
                    .unwrap_or(&Vec::new())
                {
                    (handler.function)(
                        Rc::clone(&*borrow!(self.properties)),
                        &self.get_node_context(context),
                        None,
                    )
                }
            }

            if self.instance_node.borrow().base().flags().layer == Layer::Canvas {
                context.enqueue_canvas_node_removal(
                    self.occlusion.get().render_layer_id,
                    self.id.to_u32(),
                );
                context.set_canvas_dirty(self.occlusion.get().render_layer_id);
            }

            // Needed because occlusion updates are only sent on diffs so we reset it when unmounting
            self.occlusion.set(Default::default());
            self.browser_content_layer_id.set(None);
            self.changed_listener.replace_with(Property::default());
            borrow_mut!(self.subscriptions).clear();
            borrow_mut!(self.active_children).clear();
            borrow_mut!(self.exiting_children).clear();
            borrow_mut!(self.mounted_children).clear();
            borrow_mut!(self.imported_settings_layers).clear();
            borrow_mut!(self.imported_settings_signature).clear();
            borrow_mut!(self.resolved_property_columns).clear();
            borrow_mut!(self.resolved_property_provenance).clear();
            self.active_children_view.set(Vec::new());
            self.exiting_children_view.set(Vec::new());
            self.exit_cleanup_active.set(false);
            self.exit_cleanup_listener.replace_with(Property::default());
        }
    }

    pub fn is_import_settings_node(&self) -> bool {
        borrow!(self.instance_node)
            .base()
            .template_node_type_id
            .as_ref()
            .and_then(|type_id| type_id.get_pascal_identifier())
            .as_deref()
            == Some("ImportSettings")
    }

    pub fn is_in_import_settings_subtree(&self) -> bool {
        let mut current = borrow!(self.render_parent).upgrade();
        while let Some(node) = current {
            if node.is_import_settings_node() {
                return true;
            }
            current = borrow!(node.render_parent).upgrade();
        }
        false
    }

    fn collect_imported_settings_from_node(
        node: &Rc<Self>,
        in_import_settings: bool,
        descend_components: bool,
        providers: &mut Vec<RuntimeSettingsLayer>,
    ) {
        if node.is_import_settings_node() {
            let sidecar_children = borrow!(node.sidecar_children).clone();
            for child in sidecar_children.iter() {
                Self::collect_imported_settings_from_node(child, true, true, providers);
            }
            return;
        }

        if borrow!(node.instance_node).base().flags().is_component {
            if in_import_settings {
                let base = borrow!(node.instance_node);
                let component_settings = base.base().component_settings.clone();
                let provider_type_id = base
                    .base()
                    .template_node_type_id
                    .clone()
                    .unwrap_or_default();
                drop(base);
                if let Some(settings) = component_settings {
                    if !settings.is_empty() {
                        providers.push(RuntimeSettingsLayer {
                            provider_id: node.id,
                            provider_type_id,
                            settings,
                        });
                    }
                }
                let children = node.children.get();
                for child in children.iter() {
                    Self::collect_imported_settings_from_node(child, false, true, providers);
                }
                let sidecar_children = borrow!(node.sidecar_children).clone();
                for child in sidecar_children.iter() {
                    Self::collect_imported_settings_from_node(child, false, true, providers);
                }
                return;
            }
            if !descend_components {
                return;
            }
        }

        let children = node.children.get();
        for child in children.iter() {
            Self::collect_imported_settings_from_node(
                child,
                in_import_settings,
                descend_components,
                providers,
            );
        }
        let sidecar_children = borrow!(node.sidecar_children).clone();
        for child in sidecar_children.iter() {
            Self::collect_imported_settings_from_node(
                child,
                in_import_settings,
                descend_components,
                providers,
            );
        }
    }

    pub fn sync_imported_settings(self: &Rc<Self>, context: &Rc<RuntimeContext>) {
        let mut providers = Vec::new();
        let children = self.children.get();
        for child in children.iter() {
            Self::collect_imported_settings_from_node(child, false, false, &mut providers);
        }
        let sidecar_children = borrow!(self.sidecar_children).clone();
        for child in sidecar_children.iter() {
            Self::collect_imported_settings_from_node(child, false, false, &mut providers);
        }

        let signature = providers
            .iter()
            .map(|layer| RuntimeSettingsSignatureEntry {
                provider_id: layer.provider_id,
                provider_type_id: layer.provider_type_id.clone(),
            })
            .collect::<Vec<_>>();
        if *borrow!(self.imported_settings_signature) == signature {
            return;
        }

        *borrow_mut!(self.imported_settings_signature) = signature;
        *borrow_mut!(self.imported_settings_layers) = providers;

        let root_children = self.children.get();
        for child in root_children.iter() {
            if child.is_import_settings_node() {
                continue;
            }
            child.recurse_reapply_runtime_settings_for_component_owner(self.id, context);
            child.recurse_emit_mount_updates();
        }
    }

    pub fn recurse_render_queue(
        self: &Rc<Self>,
        ctx: &Rc<RuntimeContext>,
        rcs: &mut dyn RenderContext,
    ) {
        let cp = self.get_common_properties();
        let cp = borrow!(cp);
        if cp.unclippable.get().unwrap_or(false) {
            ctx.queue_render(Rc::clone(&self));
        } else {
            self.recurse_render(ctx, rcs);
        }
    }

    pub(crate) fn render_parent_node(&self) -> Option<Rc<ExpandedNode>> {
        borrow!(self.render_parent).upgrade()
    }

    pub(crate) fn is_unclippable(&self) -> bool {
        let cp = self.get_common_properties();
        let cp = borrow!(cp);
        cp.unclippable.get().unwrap_or(false)
    }

    pub(crate) fn recurse_render_filtered(
        self: &Rc<Self>,
        ctx: &Rc<RuntimeContext>,
        rcs: &mut dyn RenderContext,
        render_path_nodes: &HashSet<ExpandedNodeIdentifier>,
        dirty_nodes: &HashSet<ExpandedNodeIdentifier>,
        stats: &mut FilteredRenderStats,
    ) {
        stats.path_nodes_visited += 1;
        borrow!(self.instance_node).handle_pre_render(&self, ctx, rcs);
        for child in self.children.get().iter().rev() {
            if render_path_nodes.contains(&child.id) {
                child.recurse_render_filtered(ctx, rcs, render_path_nodes, dirty_nodes, stats);
            } else {
                stats.skipped_subtrees += 1;
            }
        }
        if dirty_nodes.contains(&self.id) {
            borrow!(self.instance_node).render(&self, ctx, rcs);
            stats.dirty_nodes_rendered += 1;
        }
        borrow!(self.instance_node).handle_post_render(&self, ctx, rcs);
    }

    pub fn recurse_render(self: &Rc<Self>, ctx: &Rc<RuntimeContext>, rcs: &mut dyn RenderContext) {
        borrow!(self.instance_node).handle_pre_render(&self, ctx, rcs);
        for child in self.children.get().iter().rev() {
            child.recurse_render_queue(ctx, rcs);
        }
        borrow!(self.instance_node).render(&self, ctx, rcs);
        borrow!(self.instance_node).handle_post_render(&self, ctx, rcs);
    }

    /// Manages unpacking an `Rc<RefCell<PaxValue>>`, downcasting into
    /// the parameterized `target_type`, and executing a provided closure `body` in the
    /// context of that unwrapped variant (including support for mutable operations),
    /// the closure is executed.  Used at least by calculating properties in `expand_node` and
    /// passing `&mut self` into event handlers (where the typed `self` is retrieved from an instance of `PaxValue`)
    pub fn with_properties_unwrapped<T: ToFromPaxAny, R>(
        &self,
        callback: impl FnOnce(&mut T) -> R,
    ) -> R {
        self.try_with_properties_unwrapped(callback)
            .expect("properties not of expected type")
    }

    pub fn try_with_properties_unwrapped<T: ToFromPaxAny, R>(
        &self,
        callback: impl FnOnce(&mut T) -> R,
    ) -> Option<R> {
        // Borrow the contents of the RefCell mutably.
        let properties = borrow_mut!(self.properties);
        let mut borrowed = borrow_mut!(properties);
        // Downcast the unwrapped value to the specified `target_type` (or panic)
        let Ok(mut val) = T::mut_from_pax_any(&mut *borrowed) else {
            return None;
        };
        Some(callback(&mut val))
    }

    pub fn recurse_visit_postorder(self: &Rc<Self>, func: &mut impl FnMut(&Rc<Self>)) {
        // NOTE: This is to make sure the projected children list is updated before trying to access children,
        // to make stacker/scroller behave correctly when number of children is dynamic. (ex: tree view in designer)
        self.compute_flattened_projected_children();
        for child in self.children.get().iter().rev() {
            child.recurse_visit_postorder(func)
        }
        func(self);
    }

    pub fn get_node_context(self: &Rc<Self>, ctx: &Rc<RuntimeContext>) -> NodeContext {
        let globals = ctx.globals();
        let t_and_b = self.transform_and_bounds.clone();
        let deps = [t_and_b.untyped()];
        let bounds_self = Property::computed(move || t_and_b.get().bounds, &deps);
        let render_parent = borrow!(self.render_parent).upgrade();
        let t_and_b_parent = render_parent
            .as_ref()
            .map(|parent| parent.transform_and_bounds.clone())
            .unwrap_or_else(|| globals.viewport.clone());
        let (parent_padding_x, parent_padding_y) = render_parent
            .as_ref()
            .map(|parent| {
                let common_props = parent.get_common_properties();
                let common_props = borrow!(common_props);
                (
                    common_props.padding_x.clone(),
                    common_props.padding_y.clone(),
                )
            })
            .unwrap_or_default();
        let deps = [
            t_and_b_parent.untyped(),
            parent_padding_x.untyped(),
            parent_padding_y.untyped(),
        ];
        let bounds_parent = Property::computed(
            move || {
                apply_padding_frame(
                    t_and_b_parent.get(),
                    parent_padding_x.get(),
                    parent_padding_y.get(),
                )
                .bounds
            },
            &deps,
        );

        let owns_projected_children = borrow!(self.instance_node).base().flags().is_component
            || borrow!(self.expanded_projected_children).is_some();

        let projected_children_count = if owns_projected_children {
            self.flattened_projected_children_count.clone()
        } else {
            self.containing_component
                .upgrade()
                .map(|v| v.flattened_projected_children_count.clone())
                .unwrap_or_default()
        };

        let projected_children = if owns_projected_children {
            self.expanded_and_flattened_projected_children.clone()
        } else {
            self.containing_component
                .upgrade()
                .map(|v| v.expanded_and_flattened_projected_children.clone())
                .unwrap_or_default()
        };

        let projected_children_changed = if owns_projected_children {
            self.projected_children_changed.clone()
        } else {
            self.containing_component
                .upgrade()
                .map(|v| v.projected_children_changed.clone())
                .unwrap_or_default()
        };
        // Normalize semantic payload for container consumers. This hides the
        // engine transport distinction (`Owned` vs `Projected`) behind the
        // single concept "received children".
        let received_children = match borrow!(self.instance_node).received_children_source() {
            ReceivedChildrenSource::Owned => self.active_children_view.clone(),
            ReceivedChildrenSource::Projected => {
                if owns_projected_children {
                    self.expanded_and_flattened_projected_children.clone()
                } else {
                    self.containing_component
                        .upgrade()
                        .map(|v| v.expanded_and_flattened_projected_children.clone())
                        .unwrap_or_default()
                }
            }
        };
        let received_children_count_source = received_children.clone();
        let received_children_count = Property::computed(
            move || received_children_count_source.get().len(),
            &[received_children.untyped()],
        );
        let received_children_signal = received_children.clone();
        let received_children_changed = Property::computed(
            move || {
                let _ = received_children_signal.get();
            },
            &[received_children.untyped()],
        );
        // Normalize the corresponding exit-retained payload set. These nodes
        // are no longer active content, but containers may still need them for
        // ghosting or overlay placement while `@out` transitions finish.
        let retained_received_children =
            match borrow!(self.instance_node).received_children_source() {
                ReceivedChildrenSource::Owned => self.exiting_children_view.clone(),
                ReceivedChildrenSource::Projected
                    if borrow!(self.expanded_projected_children).is_some() =>
                {
                    self.exiting_children_view.clone()
                }
                ReceivedChildrenSource::Projected => Property::default(),
            };
        let retained_received_children_signal = retained_received_children.clone();
        let retained_received_children_changed = Property::computed(
            move || {
                let _ = retained_received_children_signal.get();
            },
            &[retained_received_children.untyped()],
        );

        let last_frame = Rc::new(RefCell::new(globals.frames_elapsed.get()));
        let suspended = self.suspended.clone();
        let frames_elapsed = globals.frames_elapsed.clone();
        let elapsed_millis = globals.elapsed_millis.clone();
        let deps = [frames_elapsed.untyped(), suspended.untyped()];
        // TODO: this still triggers the dirty dag dependencies of elapsed
        // frames even if the value is the same. Try to make it not trigger
        // dependencides when frozen
        let frames_elapsed_frozen_if_suspended = Property::computed(
            move || {
                if suspended.get() {
                    *borrow!(last_frame)
                } else {
                    let val = frames_elapsed.get();
                    *borrow_mut!(last_frame) = val;
                    val
                }
            },
            &deps,
        );
        // PREF: possibly change this to just take a reference to expanded node, and
        // expose methods to retrieve these values from the expanded noe when
        // needed, instead of re-creating/copying
        NodeContext {
            slot_index: self.slot_index.clone(),
            local_stack_frame: Rc::clone(&self.stack),
            expanded_node: Rc::downgrade(&self),
            containing_component: Weak::clone(&self.containing_component),
            frames_elapsed: frames_elapsed_frozen_if_suspended,
            elapsed_millis,
            gyro: globals.gyro.clone(),
            accel: globals.accel.clone(),
            bounds_self,
            bounds_parent,
            measured_size: self.measured_size.clone(),
            subtree_layout_hull: self.subtree_layout_hull.clone(),
            runtime_context: ctx.clone(),
            platform: globals.platform.clone(),
            os: globals.os.clone(),
            get_elapsed_millis: globals.get_elapsed_millis,
            projected_children_count,
            projected_children,
            node_transform_and_bounds: self.transform_and_bounds.get(),
            projected_children_changed,
            received_children,
            received_children_count,
            received_children_changed,
            retained_received_children,
            retained_received_children_changed,
            #[cfg(feature = "designtime")]
            designtime: globals.designtime.clone(),
        }
    }

    pub fn get_common_properties(&self) -> Rc<RefCell<CommonProperties>> {
        Rc::clone(&*borrow!(self.common_properties))
    }

    pub fn is_layout_breakout(&self) -> bool {
        let common_props = self.get_common_properties();
        let is_breakout = borrow!(common_props).layout_role.get() == Some(LayoutRole::Breakout);
        is_breakout
    }

    /// Determines whether the provided ray, orthogonal to the view plane,
    /// intersects this `ExpandedNode`.
    pub fn ray_cast_test(&self, ray: Point2<Window>) -> bool {
        let cp = borrow!(self.common_properties);

        // skip raycast if false
        if !borrow!(&*cp)._raycastable.get().unwrap_or(true) {
            return false;
        }
        drop(cp);
        borrow!(self.instance_node).ray_cast_test(self, ray)
    }

    pub fn compute_flattened_projected_children(&self) {
        // All of this should ideally be reactively updated,
        // but currently doesn't exist a way to "listen to"
        // an entire node tree, and generate the flattened list
        // only when changed.
        if let Some(projected_children) = borrow!(self.expanded_projected_children).as_ref() {
            let new_flattened = flatten_expanded_nodes_for_projection(projected_children);
            let old_and_new_filtered_same =
                self.expanded_and_flattened_projected_children
                    .read(|flattened| {
                        flattened
                            .iter()
                            .map(|n| n.id)
                            .eq(new_flattened.iter().map(|n| n.id))
                    });

            if !old_and_new_filtered_same {
                self.flattened_projected_children_count
                    .set(new_flattened.len());
                self.expanded_and_flattened_projected_children
                    .set(new_flattened);
                for (i, projected_child) in self
                    .expanded_and_flattened_projected_children
                    .get()
                    .iter()
                    .enumerate()
                {
                    if projected_child.slot_index.get() != Some(i) {
                        projected_child.slot_index.set(Some(i));
                    };
                }
            }
        }
    }

    dispatch_event_handler!(dispatch_scroll, Scroll, SCROLL_HANDLERS, true);
    dispatch_event_handler!(dispatch_touch_start, TouchStart, TOUCH_START_HANDLERS, true);

    dispatch_event_handler!(dispatch_touch_move, TouchMove, TOUCH_MOVE_HANDLERS, true);
    dispatch_event_handler!(dispatch_touch_end, TouchEnd, TOUCH_END_HANDLERS, true);
    dispatch_event_handler!(dispatch_key_down, KeyDown, KEY_DOWN_HANDLERS, false);
    dispatch_event_handler!(dispatch_key_up, KeyUp, KEY_UP_HANDLERS, false);
    dispatch_event_handler!(dispatch_key_press, KeyPress, KEY_PRESS_HANDLERS, false);
    dispatch_event_handler!(
        dispatch_checkbox_change,
        CheckboxChange,
        CHECKBOX_CHANGE_HANDLERS,
        true
    );
    dispatch_event_handler!(
        dispatch_textbox_change,
        TextboxChange,
        TEXTBOX_CHANGE_HANDLERS,
        true
    );
    dispatch_event_handler!(dispatch_text_input, TextInput, TEXT_INPUT_HANDLERS, true);
    dispatch_event_handler!(
        dispatch_textbox_input,
        TextboxInput,
        TEXTBOX_INPUT_HANDLERS,
        true
    );
    dispatch_event_handler!(
        dispatch_button_click,
        ButtonClick,
        BUTTON_CLICK_HANDLERS,
        true
    );
    dispatch_event_handler!(
        dispatch_photo_picker_change,
        PhotoPickerChange,
        PHOTO_PICKER_CHANGE_HANDLERS,
        true
    );
    dispatch_event_handler!(dispatch_mouse_down, MouseDown, MOUSE_DOWN_HANDLERS, true);
    dispatch_event_handler!(dispatch_mouse_up, MouseUp, MOUSE_UP_HANDLERS, true);
    dispatch_event_handler!(dispatch_mouse_move, MouseMove, MOUSE_MOVE_HANDLERS, true);
    dispatch_event_handler!(dispatch_mouse_over, MouseOver, MOUSE_OVER_HANDLERS, false);
    dispatch_event_handler!(dispatch_mouse_out, MouseOut, MOUSE_OUT_HANDLERS, false);
    dispatch_event_handler!(
        dispatch_double_click,
        DoubleClick,
        DOUBLE_CLICK_HANDLERS,
        true
    );
    dispatch_event_handler!(
        dispatch_context_menu,
        ContextMenu,
        CONTEXT_MENU_HANDLERS,
        true
    );
    fn dispatch_pointer_activation(
        self: &Rc<Self>,
        event: Event<Click>,
        source: PointerActivationSource,
        ctx: &Rc<RuntimeContext>,
    ) -> bool {
        let has_click = self.has_event_handlers(CLICK_HANDLERS);
        let has_tap = self.has_event_handlers(TAP_HANDLERS);
        let handler_key = match (has_click, has_tap, source) {
            (true, true, PointerActivationSource::Mouse) => Some(CLICK_HANDLERS),
            (true, true, PointerActivationSource::Touch) => Some(TAP_HANDLERS),
            (true, false, _) => Some(CLICK_HANDLERS),
            (false, true, _) => Some(TAP_HANDLERS),
            (false, false, _) => None,
        };

        if let Some(handler_key) = handler_key {
            self.run_event_handlers_for_key(handler_key, &event, ctx);
        }

        if let Some(parent) = self.template_parent.upgrade() {
            return parent.dispatch_pointer_activation(event, source, ctx);
        }
        event.cancelled()
    }

    pub fn dispatch_click(
        self: &Rc<Self>,
        event: Event<Click>,
        _globals: &Globals,
        ctx: &Rc<RuntimeContext>,
    ) -> bool {
        self.dispatch_pointer_activation(event, PointerActivationSource::Mouse, ctx)
    }

    pub fn dispatch_tap(
        self: &Rc<Self>,
        event: Event<Click>,
        _globals: &Globals,
        ctx: &Rc<RuntimeContext>,
    ) -> bool {
        self.dispatch_pointer_activation(event, PointerActivationSource::Touch, ctx)
    }

    dispatch_event_handler!(dispatch_wheel, Wheel, WHEEL_HANDLERS, true);
    dispatch_event_handler!(dispatch_drop, Drop, DROP_HANDLERS, true);
    dispatch_event_handler!(dispatch_gyro, Gyro, GYRO_HANDLERS, false);
    dispatch_event_handler!(dispatch_accel, Accel, ACCEL_HANDLERS, false);
    dispatch_event_handler!(dispatch_focus, Focus, FOCUSED_HANDLERS, false);
    dispatch_event_handler!(
        dispatch_select_start,
        SelectStart,
        SELECT_START_HANDLERS,
        false
    );

    pub fn dispatch_custom_event(
        self: &Rc<Self>,
        identifier: &str,
        ctx: &Rc<RuntimeContext>,
    ) -> Result<(), String> {
        let component_origin_instance = borrow!(self.instance_node);
        let registry = component_origin_instance
            .base()
            .handler_registry
            .as_ref()
            .ok_or_else(|| "no registry present".to_owned())?;

        let parent_component = self
            .containing_component
            .upgrade()
            .ok_or_else(|| "can't dispatch from root (has no parent)".to_owned())?;
        let properties = borrow!(parent_component.properties);

        for handler in borrow!(registry)
            .handlers
            .get(identifier)
            .expect("presence should have been checked when added to custom_event_queue")
        {
            (handler.function)(Rc::clone(&*properties), &self.get_node_context(ctx), None)
        }
        Ok(())
    }

    // Sets measured bounds when the chassis or container layout resolves them empirically.
    pub fn set_measured_size(self: &Rc<ExpandedNode>, width: f64, height: f64) {
        self.measured_size.set(Some((width, height)));
    }

    // Backward-compatible alias used by chassis interrupt plumbing.
    pub fn chassis_resize_request(self: &Rc<ExpandedNode>, width: f64, height: f64) {
        self.set_measured_size(width, height);
    }

    /// Helper method that returns a collection of common properties
    /// related to layout (position, size, scale, anchor, etc),
    pub fn layout_properties(self: &Rc<ExpandedNode>) -> Property<LayoutProperties> {
        let common_props = self.get_common_properties();
        let common_props = borrow!(common_props);
        let cp_width = common_props.width.clone();
        let cp_height = common_props.height.clone();
        let cp_transform = common_props.transform.clone();
        let cp_anchor_x = common_props.anchor_x.clone();
        let cp_anchor_y = common_props.anchor_y.clone();
        let cp_scale_x = common_props.scale_x.clone();
        let cp_scale_y = common_props.scale_y.clone();
        let cp_skew_x = common_props.skew_x.clone();
        let cp_skew_y = common_props.skew_y.clone();
        let cp_rotate = common_props.rotate.clone();
        let cp_x = common_props.x.clone();
        let cp_y = common_props.y.clone();
        let measured_size = self.measured_size.clone();
        let deps = [
            cp_width.untyped(),
            cp_height.untyped(),
            cp_transform.untyped(),
            cp_anchor_x.untyped(),
            cp_anchor_y.untyped(),
            cp_scale_x.untyped(),
            cp_scale_y.untyped(),
            cp_skew_x.untyped(),
            cp_skew_y.untyped(),
            cp_rotate.untyped(),
            cp_x.untyped(),
            cp_y.untyped(),
            measured_size.untyped(),
        ];

        Property::computed(
            move || {
                // Used for auto sized text, might be used for other things later
                let fallback = measured_size.get();
                let (w_fallback, h_fallback) = match fallback {
                    Some((wf, hf)) => (Some(wf), Some(hf)),
                    None => (None, None),
                };

                LayoutProperties {
                    x: cp_x.get(),
                    y: cp_y.get(),
                    width: cp_width
                        .get()
                        .or(w_fallback.map(|v| Size::Pixels(v.into()))),
                    height: cp_height
                        .get()
                        .or(h_fallback.map(|v| Size::Pixels(v.into()))),
                    rotate: cp_rotate.get(),
                    // TODO make the common prop only accept percent
                    scale_x: cp_scale_x
                        .get()
                        .map(|v| Percent((100.0 * v.expect_percent()).into())),
                    scale_y: cp_scale_y
                        .get()
                        .map(|v| Percent((100.0 * v.expect_percent()).into())),
                    anchor_x: cp_anchor_x.get(),
                    anchor_y: cp_anchor_y.get(),
                    skew_x: cp_skew_x.get(),
                    skew_y: cp_skew_y.get(),
                }
            },
            &deps,
        )
    }
}

fn layout_axis_can_contribute(layout: &LayoutProperties, axis: Axis) -> bool {
    let size = match axis {
        Axis::X => layout.width,
        Axis::Y => layout.height,
    };
    let position = match axis {
        Axis::X => layout.x,
        Axis::Y => layout.y,
    };

    size.is_some_and(|size| !size_depends_on_parent(size))
        && !position.is_some_and(size_depends_on_parent)
}

fn layout_axis_can_contribute_from_parts(size: Option<Size>, position: Option<Size>) -> bool {
    size.is_some_and(|size| !size_depends_on_parent(size))
        && !position.is_some_and(size_depends_on_parent)
}

fn size_depends_on_parent(size: Size) -> bool {
    match size {
        Size::Pixels(_) => false,
        Size::Percent(percent) => percent.to_float().abs() > f64::EPSILON,
        Size::Combined(_, percent) => percent.to_float().abs() > f64::EPSILON,
    }
}

/// Given a projected child list, flatten away "slot-invisible" nodes (namely, `if` and `for`)
/// and return a top-level list of renderable projected children.
fn flatten_expanded_nodes_for_projection(nodes: &[Rc<ExpandedNode>]) -> Vec<Rc<ExpandedNode>> {
    let mut result: Vec<Rc<ExpandedNode>> = vec![];
    for node in nodes {
        if borrow!(node.instance_node).base().flags().invisible_to_slot {
            result.extend(flatten_expanded_nodes_for_projection(
                node.children
                    .get()
                    .clone()
                    .into_iter()
                    .collect::<Vec<_>>()
                    .as_slice(),
            ));
        } else {
            result.push(Rc::clone(&node))
        }
    }
    result
}

impl std::fmt::Debug for ExpandedNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //see: https://users.rust-lang.org/t/reusing-an-fmt-formatter/8531/4
        //maybe this utility should be moved to a more accessible place?
        pub struct Fmt<F>(pub F)
        where
            F: Fn(&mut fmt::Formatter) -> fmt::Result;

        impl<F> fmt::Debug for Fmt<F>
        where
            F: Fn(&mut fmt::Formatter) -> fmt::Result,
        {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                (self.0)(f)
            }
        }

        f.debug_struct("ExpandedNode")
            .field(
                "instance_node",
                &Fmt(|f| borrow!(self.instance_node).resolve_debug(f, Some(self))),
            )
            .field("id", &self.id)
            .field("common_properties", &borrow!(self.common_properties))
            .field("transform_and_bounds", &self.transform_and_bounds)
            .field("children", &self.children.get().iter().collect::<Vec<_>>())
            .field(
                "projected_children",
                &self
                    .expanded_and_flattened_projected_children
                    .get()
                    .iter()
                    .map(|v| v.id)
                    .collect::<Vec<_>>(),
            )
            .field("render_placement", &self.occlusion.get())
            .field(
                "containing_component",
                &self.containing_component.upgrade().map(|v| v.id.clone()),
            )
            .finish()
    }
}
