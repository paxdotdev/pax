use crate::api::TextInput;
use crate::node_interface::NodeLocal;
use pax_language::Computable;
use pax_runtime_api::pax_value::{ImplToFromPaxAny, PaxAny, ToFromPaxAny};
use pax_runtime_api::{
    borrow, borrow_mut, use_RefCell, Focus, Interpolatable, Layer, NativeLiquidGlassScope,
    PaxValue, Percent, Property, SelectStart, Variable,
};

use crate::api::math::Point2;
use crate::constants::{
    ACCEL_HANDLERS, BUTTON_CLICK_HANDLERS, CHECKBOX_CHANGE_HANDLERS, CLICK_HANDLERS,
    CONTEXT_MENU_HANDLERS, DOUBLE_CLICK_HANDLERS, DROP_HANDLERS, FOCUSED_HANDLERS, GYRO_HANDLERS,
    KEY_DOWN_HANDLERS, KEY_PRESS_HANDLERS, KEY_UP_HANDLERS, MOUSE_DOWN_HANDLERS,
    MOUSE_MOVE_HANDLERS, MOUSE_OUT_HANDLERS, MOUSE_OVER_HANDLERS, MOUSE_UP_HANDLERS,
    PHOTO_PICKER_CHANGE_HANDLERS, SCROLL_HANDLERS, SELECT_START_HANDLERS, SLIDER_CHANGE_HANDLERS,
    TAP_HANDLERS, TEXTBOX_CHANGE_HANDLERS, TEXTBOX_INPUT_HANDLERS, TEXT_INPUT_HANDLERS,
    TOUCH_CANCEL_HANDLERS, TOUCH_END_HANDLERS, TOUCH_MOVE_HANDLERS, TOUCH_START_HANDLERS,
    WHEEL_HANDLERS,
};
use_RefCell!();
use crate::cartridge::evaluate_timeline_duration;
use crate::{ExpandedNodeIdentifier, Globals, LayoutHull, LayoutProperties, TransformAndBounds};
use core::fmt;
use std::cell::Cell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::{Rc, Weak};

// Keep the source handles alive, not the parent node. Equality of current
// values is insufficient: a different source must establish a new subscription.
#[derive(Clone)]
struct ParentBindingSources {
    parent: ExpandedNodeIdentifier,
    properties: Vec<pax_runtime_api::properties::UntypedProperty>,
}

impl ParentBindingSources {
    fn same_as(&self, other: &Self) -> bool {
        self.parent == other.parent
            && self.properties.len() == other.properties.len()
            && self
                .properties
                .iter()
                .zip(&other.properties)
                .all(|(a, b)| a.get_id() == b.get_id())
    }
}

#[derive(PartialEq)]
struct ChildStructure {
    binding_generation: u64,
    rendered: Vec<ExpandedNodeIdentifier>,
    active: Vec<ExpandedNodeIdentifier>,
    exiting: Vec<ExpandedNodeIdentifier>,
}

use crate::api::{
    Accel, Axis, ButtonClick, CheckboxChange, Click, CommonProperties, ContextMenu, DoubleClick,
    Drop, Event, Gyro, KeyDown, KeyPress, KeyUp, LayoutRole, MouseDown, MouseMove, MouseOut,
    MouseOver, MouseUp, NodeContext, PhotoPickerChange, RenderContext, Scroll, Size, SliderChange,
    TextboxChange, TextboxInput, TouchCancel, TouchEnd, TouchMove, TouchStart, Wheel, Window,
};
use pax_manifest::cartridge_generation::{
    TRANSITION_GENERATION_SYMBOL, TRANSITION_PHASE_ENTER, TRANSITION_PHASE_EXIT,
    TRANSITION_PHASE_IDLE, TRANSITION_PHASE_SYMBOL, TRANSITION_PLAYHEAD_MILLIS_SYMBOL,
    TRANSITION_PLAYHEAD_SYMBOL, TRANSITION_TAKEOVER_SYMBOL,
};
use pax_manifest::{
    ExpressionInfo, LocationInfo, SelectorExpr, SettingsBlockElement, TypeId,
    UniqueTemplateNodeIdentifier, ValueDefinition,
};

use crate::{
    add_symmetric_padding_to_content_layout_hull, apply_container_frame, apply_padding_frame,
    compute_tab, ComponentInstance, ContainerFrame, HandlerLocation, InstanceNode, InstanceNodePtr,
    ReceivedChildrenSource, RuntimeContext, RuntimePropertiesStackFrame,
};

#[derive(Clone)]
pub struct RuntimeSettingsLayer {
    pub provider_id: ExpandedNodeIdentifier,
    pub provider_type_id: TypeId,
    /// The provider component's lexical scope. Imported settings expressions
    /// must resolve `self` and provider properties here, even though their
    /// resulting values are applied to a different node.
    pub provider_stack: Rc<RuntimePropertiesStackFrame>,
    pub settings: Vec<SettingsBlockElement>,
}

impl fmt::Debug for RuntimeSettingsLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeSettingsLayer")
            .field("provider_id", &self.provider_id)
            .field("provider_type_id", &self.provider_type_id)
            .field("provider_stack", &"<runtime scope>")
            .field("settings", &self.settings)
            .finish()
    }
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

#[derive(Clone, Debug, Default)]
pub struct RuntimeSettingsCondition {
    pub positive: Vec<ExpressionInfo>,
    pub negative: Vec<ExpressionInfo>,
}

#[derive(Clone)]
pub struct RuntimeResolvedPropertyEntry {
    pub source: RuntimeSettingsSource,
    pub selector: Option<SelectorExpr>,
    pub source_location: Option<LocationInfo>,
    /// Overrides the receiving node's scope when this value came from an
    /// imported settings provider.
    pub source_stack: Option<Rc<RuntimePropertiesStackFrame>>,
    pub value: ValueDefinition,
    pub condition: Option<RuntimeSettingsCondition>,
    pub axis_index: Option<usize>,
}

impl fmt::Debug for RuntimeResolvedPropertyEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeResolvedPropertyEntry")
            .field("source", &self.source)
            .field("selector", &self.selector)
            .field("source_location", &self.source_location)
            .field(
                "source_stack",
                &self.source_stack.as_ref().map(|_| "<runtime scope>"),
            )
            .field("value", &self.value)
            .field("condition", &self.condition)
            .field("axis_index", &self.axis_index)
            .finish()
    }
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
    /// Retention token captured when each exiting child is queued. Rescuing a
    /// child advances its token so stale cleanup work cannot unmount it.
    exiting_child_generations: RefCell<HashMap<ExpandedNodeIdentifier, u64>>,
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
    selector_classes_listener: Property<()>,

    /// Tracks whether this node's occlusion-affecting inputs changed.
    pub occlusion_listener: Property<()>,

    /// Pulls the node's children property only when its upstream dependencies changed.
    pub children_listener: Property<()>,
    /// Rebinds `subtree_layout_hull` when the child list changes.
    pub subtree_layout_hull_listener: Property<()>,
    parent_binding_sources: RefCell<Option<ParentBindingSources>>,
    layout_binding_generation: Cell<u64>,
    #[cfg(test)]
    pub(crate) parent_binding_rebuilds: Cell<usize>,
    #[cfg(test)]
    pub(crate) layout_hull_rebuilds: Cell<usize>,
    #[cfg(test)]
    pub(crate) structural_children_updates: Cell<usize>,
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

    /// Dirty signal emitted when active slot sites or slot-index expressions
    /// may change the component-local projection resolution.
    pub slot_projection_changed: Property<()>,

    /// subscription properties: added to this expanded node by calling ctx.subscribe in a node event handler
    pub subscriptions: RefCell<Vec<Property<()>>>,

    /// Current lifecycle transition phase for this node.
    pub transition_phase: Property<u64>,
    /// Monotonic identity for each lifecycle transition run.
    pub transition_generation: Property<u64>,
    /// Whether the current run directly reversed the opposite lifecycle phase.
    pub transition_takeover: Property<bool>,
    /// Class membership captured at lifecycle-transition start. Ordinary settings continue to
    /// follow live classes, while transition tracks keep the selector membership that launched
    /// the current run.
    pub transition_selector_classes: RefCell<Option<Vec<String>>>,
    /// Parent-owned exit retention generation for stale cleanup protection.
    exit_retention_generation: Cell<u64>,
    /// Frame at which the active lifecycle transition began.
    pub transition_origin_frame: Property<u64>,
    /// Millisecond clock value at which the active lifecycle transition began.
    pub transition_origin_millis: Property<u64>,
    /// Local playhead, in frames, for the active lifecycle transition.
    pub transition_playhead: Property<f64>,
    /// Local playhead, in milliseconds, for the active lifecycle transition.
    pub transition_playhead_millis: Property<f64>,
    /// Effect property used to stop enter-transition clock dependencies.
    pub enter_cleanup_listener: Property<()>,
    /// Whether enter cleanup should do work on frame ticks.
    pub enter_cleanup_active: Cell<bool>,
    /// Wall-clock start for exit timeout enforcement.
    pub exit_started_millis: Cell<Option<u128>>,
    /// Whether this node has already warned about truncating its current exit transition.
    exit_timeout_warning_emitted: Cell<bool>,
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
    /// Requests a materialization pass that resets properties removed from the prior columns.
    pub reset_removed_runtime_properties: Cell<bool>,
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
/// Live selector identity for one expanded runtime node.
pub struct RuntimeSelectorMetadata {
    /// Concrete element/component type used by type selectors.
    pub type_id: TypeId,
    /// Reactive node id used by id selectors.
    pub id: Property<Option<String>>,
    /// Reactive, normalized class names in left-to-right cascade order.
    pub classes: Property<Vec<String>>,
}

impl RuntimeSelectorMetadata {
    fn from_base(
        base: &crate::rendering::BaseInstance,
        common_properties: &Rc<RefCell<CommonProperties>>,
        stack: &Rc<RuntimePropertiesStackFrame>,
    ) -> Self {
        let classes = base
            .template_node_selector_info
            .as_ref()
            .and_then(|info| {
                info.class_binding.as_ref().map(|binding| {
                    build_selector_classes_property(binding, info.source_location.as_ref(), stack)
                })
            })
            .unwrap_or_else(|| Property::new_with_name(Vec::new(), "selector classes"));

        Self {
            type_id: base.template_node_type_id.clone().unwrap_or_default(),
            id: common_properties.borrow().id.clone(),
            classes,
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

fn warn_selector_class_once(
    warnings: &Rc<RefCell<HashSet<String>>>,
    key: String,
    message: impl FnOnce() -> String,
) {
    if warnings.borrow_mut().insert(key) {
        log::warn!("{}", message());
    }
}

fn normalize_selector_classes(
    value: PaxValue,
    warnings: &Rc<RefCell<HashSet<String>>>,
    source_location: Option<&LocationInfo>,
) -> Vec<String> {
    let location = source_location
        .map(|location| {
            format!(
                " at {}:{}",
                location.start_line_col.0, location.start_line_col.1
            )
        })
        .unwrap_or_default();
    let candidates = match value {
        PaxValue::String(value) => vec![value],
        PaxValue::Vec(values) => values
            .into_iter()
            .filter_map(|value| match value {
                PaxValue::String(value) => Some(value),
                other => {
                    warn_selector_class_once(warnings, format!("list-type:{other}"), || {
                        format!("ignoring non-string value in class list{location}: {other}")
                    });
                    None
                }
            })
            .collect(),
        other => {
            warn_selector_class_once(warnings, format!("binding-type:{other}"), || {
                format!(
                    "class binding{location} must evaluate to String or Vec<String>; received {other}"
                )
            });
            return Vec::new();
        }
    };

    let mut seen = HashSet::new();
    let mut normalized = candidates
        .into_iter()
        .rev()
        .filter(|class_name| {
            if class_name.is_empty() {
                return false;
            }
            if !matches!(
                SelectorExpr::parse(&format!(".{class_name}")),
                Ok(SelectorExpr::Class(_))
            ) {
                warn_selector_class_once(warnings, format!("invalid-name:{class_name}"), || {
                    format!("ignoring invalid class name '{class_name}'{location}")
                });
                return false;
            }
            seen.insert(class_name.clone())
        })
        .collect::<Vec<_>>();
    normalized.reverse();
    normalized
}

fn build_selector_classes_property(
    binding: &ValueDefinition,
    source_location: Option<&LocationInfo>,
    stack: &Rc<RuntimePropertiesStackFrame>,
) -> Property<Vec<String>> {
    let warnings = Rc::new(RefCell::new(HashSet::new()));
    let source_location = source_location.cloned();
    match binding {
        ValueDefinition::LiteralValue(value) => Property::new_with_name(
            normalize_selector_classes(value.clone(), &warnings, source_location.as_ref()),
            "selector classes",
        ),
        ValueDefinition::Expression(info) => {
            let dependencies = info
                .dependencies
                .iter()
                .filter_map(|dependency| {
                    stack
                        .resolve_symbol_as_erased_property(dependency)
                        .or_else(|| {
                            log::warn!("failed to resolve class binding symbol {dependency}");
                            None
                        })
                })
                .collect::<Vec<_>>();
            let expression = info.expression.clone();
            let expression_label = expression.to_string();
            let stack = Rc::clone(stack);
            let warnings = Rc::clone(&warnings);
            Property::computed_with_name(
                move || match expression.compute(stack.clone()) {
                    Ok(value) => {
                        normalize_selector_classes(value, &warnings, source_location.as_ref())
                    }
                    Err(error) => {
                        warn_selector_class_once(
                            &warnings,
                            format!("evaluation:{expression_label}:{error:?}"),
                            || {
                                let location = source_location
                                    .as_ref()
                                    .map(|location| {
                                        format!(
                                            " at {}:{}",
                                            location.start_line_col.0, location.start_line_col.1
                                        )
                                    })
                                    .unwrap_or_default();
                                format!(
                                    "failed to evaluate class binding `{expression_label}`{location}: {error:?}"
                                )
                            },
                        );
                        Vec::new()
                    }
                },
                &dependencies,
                "selector classes",
            )
        }
        other => {
            log::warn!("unsupported class binding definition: {other}");
            Property::new_with_name(Vec::new(), "selector classes")
        }
    }
}

#[cfg(test)]
mod selector_class_tests {
    use super::{build_selector_classes_property, normalize_selector_classes};
    use crate::RuntimePropertiesStackFrame;
    use pax_language::parse_pax_expression;
    use pax_manifest::{ExpressionInfo, ValueDefinition};
    use pax_runtime_api::{PaxValue, Property, Variable};
    use std::cell::RefCell;
    use std::collections::{HashMap, HashSet};
    use std::rc::Rc;

    #[test]
    fn normalizes_order_duplicates_and_invalid_names() {
        let warnings = Rc::new(RefCell::new(HashSet::new()));
        let classes = normalize_selector_classes(
            PaxValue::Vec(vec![
                PaxValue::String("base".to_string()),
                PaxValue::String("bad name".to_string()),
                PaxValue::String("active".to_string()),
                PaxValue::String("base".to_string()),
                PaxValue::Numeric(1.into()),
            ]),
            &warnings,
            None,
        );

        assert_eq!(classes, vec!["active", "base"]);
        assert_eq!(warnings.borrow().len(), 2);
    }

    #[test]
    fn expression_backed_classes_track_string_and_vector_properties() {
        let constant_binding = ValueDefinition::Expression(ExpressionInfo::new(
            parse_pax_expression(r#""temporary""#).unwrap(),
        ));
        let constant_classes = build_selector_classes_property(
            &constant_binding,
            None,
            &RuntimePropertiesStackFrame::new(HashMap::new()),
        );
        assert_eq!(constant_classes.get(), vec!["temporary"]);

        let scalar = Property::new("base".to_string());
        let stack = RuntimePropertiesStackFrame::new(HashMap::from([(
            "current_class".to_string(),
            Variable::new_from_typed_property(scalar.clone()),
        )]));
        let scalar_binding = ValueDefinition::Expression(ExpressionInfo::new(
            parse_pax_expression("current_class").unwrap(),
        ));
        let scalar_classes = build_selector_classes_property(&scalar_binding, None, &stack);
        assert_eq!(scalar_classes.get(), vec!["base"]);
        scalar.set("active".to_string());
        assert_eq!(scalar_classes.get(), vec!["active"]);

        let vector = Property::new(vec!["base".to_string(), "selected".to_string()]);
        let stack = RuntimePropertiesStackFrame::new(HashMap::from([(
            "current_classes".to_string(),
            Variable::new_from_typed_property(vector.clone()),
        )]));
        let vector_binding = ValueDefinition::Expression(ExpressionInfo::new(
            parse_pax_expression("current_classes").unwrap(),
        ));
        let vector_classes = build_selector_classes_property(&vector_binding, None, &stack);
        assert_eq!(vector_classes.get(), vec!["base", "selected"]);
        vector.set(Vec::new());
        assert!(vector_classes.get().is_empty());
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
        let transition_generation = Property::new_with_name(0, "transition generation");
        let transition_takeover = Property::new_with_name(false, "transition takeover");
        let transition_origin_frame =
            Property::new_with_name(context.globals().elapsed_frames.get(), "transition origin");
        let transition_origin_millis = Property::new_with_name(
            context.globals().elapsed_millis.get(),
            "transition origin millis",
        );
        let transition_playhead = Property::new_with_name(0.0, "transition playhead");
        let transition_playhead_millis = Property::new_with_name(0.0, "transition playhead millis");

        let env = if has_transition_bindings {
            env.push(
                vec![
                    (
                        TRANSITION_PHASE_SYMBOL.to_string(),
                        Variable::new_from_typed_property(transition_phase.clone()),
                    ),
                    (
                        TRANSITION_GENERATION_SYMBOL.to_string(),
                        Variable::new_from_typed_property(transition_generation.clone()),
                    ),
                    (
                        TRANSITION_TAKEOVER_SYMBOL.to_string(),
                        Variable::new_from_typed_property(transition_takeover.clone()),
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
            RuntimeSelectorMetadata::from_base(template.base(), &common_properties, &env);

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
            exiting_child_generations: RefCell::new(HashMap::new()),
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
            selector_classes_listener: Property::default(),
            occlusion_listener: Property::default(),
            children_listener: Property::default(),
            subtree_layout_hull_listener: Property::default(),
            parent_binding_sources: RefCell::new(None),
            layout_binding_generation: Cell::new(0),
            #[cfg(test)]
            parent_binding_rebuilds: Cell::new(0),
            #[cfg(test)]
            layout_hull_rebuilds: Cell::new(0),
            #[cfg(test)]
            structural_children_updates: Cell::new(0),
            content_measurement_listener: Property::default(),
            content_measurement_rebind_listener: Property::default(),
            content_measurement_bound: Cell::new(false),
            subtree_requires_non_reactive_update: Cell::new(true),
            projected_children_changed: Property::default(),
            slot_projection_changed: Property::default(),
            subscriptions: Default::default(),
            transition_phase,
            transition_generation,
            transition_takeover,
            transition_selector_classes: RefCell::new(None),
            exit_retention_generation: Cell::new(0),
            transition_origin_frame,
            transition_origin_millis,
            transition_playhead,
            transition_playhead_millis,
            enter_cleanup_listener: Property::default(),
            enter_cleanup_active: Cell::new(false),
            exit_started_millis: Cell::new(None),
            exit_timeout_warning_emitted: Cell::new(false),
            exit_cleanup_listener: Property::default(),
            exit_cleanup_active: Cell::new(false),
            imported_settings_layers: RefCell::new(Vec::new()),
            imported_settings_signature: RefCell::new(Vec::new()),
            resolved_property_columns: RefCell::new(BTreeMap::new()),
            resolved_property_provenance: RefCell::new(BTreeMap::new()),
            reset_removed_runtime_properties: Cell::new(false),
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
            RuntimeSelectorMetadata::from_base(template.base(), &common_properties, &res.stack);
        res.refresh_properties_scope(&template);
        res.bind_selector_classes_listener(context);
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
            RuntimeSelectorMetadata::from_base(template.base(), &common_properties, &self.stack);
        self.refresh_properties_scope(&template);
        self.bind_selector_classes_listener(context);
        self.reapply_runtime_settings(context);
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
        self.bind_selector_classes_listener(context);
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

    pub(crate) fn rescue_exiting_child_matching(
        self: &Rc<Self>,
        predicate: impl Fn(&Rc<ExpandedNode>) -> bool,
    ) -> Option<Rc<ExpandedNode>> {
        let position = borrow!(self.exiting_children).iter().position(predicate)?;
        let child = borrow_mut!(self.exiting_children).remove(position);
        child
            .exit_retention_generation
            .set(child.exit_retention_generation.get().wrapping_add(1));
        borrow_mut!(self.exiting_child_generations).remove(&child.id);
        if borrow!(self.exiting_children).is_empty() {
            self.exit_cleanup_active.set(false);
            self.exit_cleanup_listener
                .replace_with(Property::new_with_name((), "exit transition cleanup"));
        }
        Some(child)
    }

    fn sync_mounted_children_from_active_and_exiting(&self) -> Vec<Rc<ExpandedNode>> {
        let active_children = borrow!(self.active_children).clone();
        let exiting_children = borrow!(self.exiting_children).clone();
        if !self
            .active_children_view
            .read(|old| Self::same_children(old, &active_children))
        {
            self.active_children_view.set(active_children.clone());
        }
        if !self
            .exiting_children_view
            .read(|old| Self::same_children(old, &exiting_children))
        {
            self.exiting_children_view.set(exiting_children.clone());
        }
        let mut combined = exiting_children;
        combined.extend(active_children);
        *borrow_mut!(self.mounted_children) = combined.clone();
        combined
    }

    fn has_child(children: &[Rc<ExpandedNode>], target: &Rc<ExpandedNode>) -> bool {
        children.iter().any(|child| Rc::ptr_eq(child, target))
    }

    fn same_children(a: &[Rc<ExpandedNode>], b: &[Rc<ExpandedNode>]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(a, b)| Rc::ptr_eq(a, b))
    }

    fn detach_child_for_reparent(&self, target: &Rc<ExpandedNode>) {
        let mut removed = false;
        {
            let active_children = &mut *borrow_mut!(self.active_children);
            let old_len = active_children.len();
            active_children.retain(|child| !Rc::ptr_eq(child, target));
            removed |= active_children.len() != old_len;
        }
        {
            let exiting_children = &mut *borrow_mut!(self.exiting_children);
            let old_len = exiting_children.len();
            exiting_children.retain(|child| !Rc::ptr_eq(child, target));
            removed |= exiting_children.len() != old_len;
        }
        if removed {
            self.sync_mounted_children_from_active_and_exiting();
            self.mark_non_reactive_update_subtree_dirty();
        }
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
        self: &Rc<Self>,
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
        self.begin_transition_run(context, TRANSITION_PHASE_ENTER);
        self.exit_started_millis.set(None);
        self.exit_timeout_warning_emitted.set(false);
        self.enable_enter_cleanup_listener(context);
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
        self.begin_transition_run(context, TRANSITION_PHASE_EXIT);
        self.disable_enter_cleanup_listener();
        self.exit_started_millis
            .set(Some((context.globals().get_elapsed_millis)()));
        self.exit_timeout_warning_emitted.set(false);
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

    fn begin_transition_run(self: &Rc<Self>, context: &Rc<RuntimeContext>, phase: u64) {
        let previous_phase = self.transition_phase.get();
        let is_takeover = matches!(
            (previous_phase, phase),
            (TRANSITION_PHASE_ENTER, TRANSITION_PHASE_EXIT)
                | (TRANSITION_PHASE_EXIT, TRANSITION_PHASE_ENTER)
        );
        self.transition_origin_frame
            .set(context.globals().elapsed_frames.get());
        self.transition_origin_millis
            .set(context.globals().elapsed_millis.get());
        self.transition_takeover.set(is_takeover);
        self.transition_generation
            .set(self.transition_generation.get().wrapping_add(1));
        self.activate_transition_clock(context);
        self.transition_phase.set(phase);
        *self.transition_selector_classes.borrow_mut() =
            Some(self.selector_metadata.borrow().classes.get());
        self.reset_removed_runtime_properties.set(true);
        self.reapply_runtime_settings(context);
    }

    fn activate_transition_clock(&self, context: &Rc<RuntimeContext>) {
        let elapsed_frames = context.globals().elapsed_frames.clone();
        let origin_frame = self.transition_origin_frame.clone();
        let elapsed_frames_for_playhead = elapsed_frames.clone();
        let origin_frame_for_playhead = origin_frame.clone();
        self.transition_playhead
            .replace_with(Property::computed_with_name(
                move || {
                    elapsed_frames_for_playhead
                        .get()
                        .saturating_sub(origin_frame_for_playhead.get()) as f64
                },
                &[elapsed_frames.untyped(), origin_frame.untyped()],
                "transition playhead",
            ));

        let elapsed_millis = context.globals().elapsed_millis.clone();
        let origin_millis = self.transition_origin_millis.clone();
        let elapsed_millis_for_playhead = elapsed_millis.clone();
        let origin_millis_for_playhead = origin_millis.clone();
        self.transition_playhead_millis
            .replace_with(Property::computed_with_name(
                move || {
                    elapsed_millis_for_playhead
                        .get()
                        .saturating_sub(origin_millis_for_playhead.get()) as f64
                },
                &[elapsed_millis.untyped(), origin_millis.untyped()],
                "transition playhead millis",
            ));
    }

    fn deactivate_transition_clock(&self) {
        self.transition_playhead
            .replace_with(Property::new_with_name(0.0, "transition playhead"));
        self.transition_playhead_millis
            .replace_with(Property::new_with_name(0.0, "transition playhead millis"));
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

    fn start_enter_transition_tree(self: &Rc<Self>, context: &Rc<RuntimeContext>) -> bool {
        let was_exiting = self.transition_phase.get() == TRANSITION_PHASE_EXIT;
        let started_here = self.start_self_enter_transition_for_source(context, None);
        if was_exiting && !started_here {
            self.transition_phase.set(TRANSITION_PHASE_IDLE);
            self.transition_takeover.set(false);
            self.deactivate_transition_clock();
            self.exit_started_millis.set(None);
            self.transition_selector_classes.borrow_mut().take();
            self.reset_removed_runtime_properties.set(true);
            self.reapply_runtime_settings(context);
        }
        let mut started = started_here;
        for child in borrow!(self.mounted_children).iter() {
            started |= child.start_enter_transition_tree(context);
        }
        if started_here {
            if let Some(source) = self.template_node_identifier() {
                self.start_bound_enter_transitions(context, &source);
            }
        }
        started
    }

    fn retain_exiting_child(
        self: &Rc<Self>,
        child: Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        if Self::has_child(&borrow!(self.exiting_children), &child) {
            return;
        }
        let generation = child.exit_retention_generation.get().wrapping_add(1);
        child.exit_retention_generation.set(generation);
        borrow_mut!(self.exiting_child_generations).insert(child.id, generation);
        borrow_mut!(self.exiting_children).push(child);
        self.enable_exit_cleanup_listener(context);
    }

    fn self_exit_transition_complete(&self, context: &Rc<RuntimeContext>) -> bool {
        if self.transition_phase.get() != TRANSITION_PHASE_EXIT {
            return true;
        }
        let instance_node = borrow!(self.instance_node);
        let base = instance_node.base();
        let transition_config = base.transition_config().clone();
        let template_node_identifier = base.template_node_identifier.clone();
        let template_node_type_id = base.template_node_type_id.clone();
        drop(instance_node);
        let duration_complete = self.transition_duration_complete(
            transition_config.exit_frame_count,
            transition_config.exit_millis_count,
            &transition_config.exit_dynamic_durations,
        );
        let timeout_complete = self
            .exit_started_millis
            .get()
            .map(|started| {
                (context.globals().get_elapsed_millis)().saturating_sub(started)
                    >= transition_config.timeout_ms as u128
            })
            .unwrap_or(false);
        if timeout_complete
            && !duration_complete
            && !self.exit_timeout_warning_emitted.replace(true)
        {
            log::warn!(
                "Exit transition timed out after {}ms and will be truncated for node {:?}{}{}. Check @out/@transition duration, percent keyframes, and dynamic duration expressions.",
                transition_config.timeout_ms,
                self.id,
                template_node_identifier
                    .as_ref()
                    .map(|identifier| format!(" template={}", identifier))
                    .unwrap_or_default(),
                template_node_type_id
                    .as_ref()
                    .map(|type_id| format!(" type={}", type_id))
                    .unwrap_or_default(),
            );
        }
        duration_complete || timeout_complete
    }

    fn transition_duration_complete(
        &self,
        frame_count: u64,
        millis_count: Option<u64>,
        dynamic_durations: &[ValueDefinition],
    ) -> bool {
        let mut frame_count = frame_count as f64;
        let mut millis_count = millis_count.map(|value| value as f64).unwrap_or_default();
        let mut unresolved_dynamic_duration = false;

        for duration_definition in dynamic_durations {
            match evaluate_timeline_duration(duration_definition, &self.stack) {
                Some(duration) if duration.is_frame_based() => {
                    frame_count = frame_count.max(duration.as_frames_f64().max(0.0).ceil());
                }
                Some(duration) => {
                    millis_count = millis_count.max(duration.as_milliseconds_f64().max(0.0).ceil());
                }
                None => unresolved_dynamic_duration = true,
            }
        }

        if unresolved_dynamic_duration
            && frame_count <= f64::EPSILON
            && millis_count <= f64::EPSILON
        {
            frame_count = 100.0;
        }

        let frame_complete =
            frame_count <= f64::EPSILON || self.transition_playhead.get() >= frame_count;
        let millis_complete =
            millis_count <= f64::EPSILON || self.transition_playhead_millis.get() >= millis_count;
        frame_complete && millis_complete
    }

    fn self_enter_transition_complete(&self) -> bool {
        if self.transition_phase.get() != TRANSITION_PHASE_ENTER {
            return true;
        }
        let instance_node = borrow!(self.instance_node);
        let transition_config = instance_node.base().transition_config().clone();
        drop(instance_node);
        self.transition_duration_complete(
            transition_config.enter_frame_count,
            transition_config.enter_millis_count,
            &transition_config.enter_dynamic_durations,
        )
    }

    fn enable_enter_cleanup_listener(self: &Rc<Self>, context: &Rc<RuntimeContext>) {
        if self.enter_cleanup_active.get() {
            return;
        }
        self.enter_cleanup_active.set(true);
        let weak_self = Rc::downgrade(self);
        let runtime_context = Rc::clone(context);
        let elapsed_frames = context.globals().elapsed_frames.clone();
        let elapsed_frames_dep = elapsed_frames.untyped();
        self.enter_cleanup_listener
            .replace_with(Property::computed_with_name(
                move || {
                    let _ = elapsed_frames.get();
                    if let Some(node) = weak_self.upgrade() {
                        node.complete_self_enter_transition_if_needed(&runtime_context);
                    }
                },
                &[elapsed_frames_dep],
                "enter transition cleanup",
            ));
        context.register_expanded_node_effect_property_named(
            self,
            &self.enter_cleanup_listener,
            "enter transition cleanup",
        );
    }

    fn complete_self_enter_transition_if_needed(self: &Rc<Self>, context: &Rc<RuntimeContext>) {
        if !self.enter_cleanup_active.get() {
            return;
        }
        if self.transition_phase.get() != TRANSITION_PHASE_ENTER {
            self.disable_enter_cleanup_listener();
            return;
        }
        if self.self_enter_transition_complete() {
            self.transition_phase.set(TRANSITION_PHASE_IDLE);
            self.deactivate_transition_clock();
            self.disable_enter_cleanup_listener();
            self.transition_selector_classes.borrow_mut().take();
            self.reset_removed_runtime_properties.set(true);
            self.reapply_runtime_settings(context);
        }
    }

    fn disable_enter_cleanup_listener(&self) {
        self.enter_cleanup_active.set(false);
        self.enter_cleanup_listener
            .replace_with(Property::new_with_name((), "enter transition cleanup"));
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
        let elapsed_frames = context.globals().elapsed_frames.clone();
        let elapsed_frames_dep = elapsed_frames.untyped();
        self.exit_cleanup_listener
            .replace_with(Property::computed_with_name(
                move || {
                    let _ = elapsed_frames.get();
                    if let Some(node) = weak_self.upgrade() {
                        node.prune_completed_exit_children(&cloned_context);
                    }
                },
                &[elapsed_frames_dep],
                "exit transition cleanup",
            ));
        context.register_expanded_node_effect_property_named(
            self,
            &self.exit_cleanup_listener,
            "exit transition cleanup",
        );
    }

    fn prune_completed_exit_children(self: &Rc<Self>, context: &Rc<RuntimeContext>) {
        let exiting = std::mem::take(&mut *borrow_mut!(self.exiting_children));
        if exiting.is_empty() {
            self.exit_cleanup_active.set(false);
            self.exit_cleanup_listener
                .replace_with(Property::new_with_name((), "exit transition cleanup"));
            return;
        }

        let mut retained = Vec::new();
        let active = borrow!(self.active_children).clone();
        for child in exiting {
            let expected_generation = borrow_mut!(self.exiting_child_generations).remove(&child.id);
            if Self::has_child(&active, &child)
                || expected_generation != Some(child.exit_retention_generation.get())
            {
                continue;
            }
            if child.exit_transition_tree_complete(context) {
                child.recurse_unmount(context);
            } else {
                borrow_mut!(self.exiting_child_generations)
                    .insert(child.id, child.exit_retention_generation.get());
                retained.push(child);
            }
        }
        *borrow_mut!(self.exiting_children) = retained;
        self.sync_mounted_children_from_active_and_exiting();
        // `children` is computed for control-flow nodes. A direct write here can
        // mark a pending branch update clean if cleanup and selection change in
        // the same frame, so invalidate it and let its evaluator reconcile.
        self.children.invalidate();
        context.mark_occlusion_dirty();

        if borrow!(self.exiting_children).is_empty() {
            self.exit_cleanup_active.set(false);
            self.exit_cleanup_listener
                .replace_with(Property::new_with_name((), "exit transition cleanup"));
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
        let mut bindings_changed = false;
        for child in new_children.iter() {
            let previous_parent = borrow!(child.render_parent).upgrade();
            let is_reparented = child.attached.get() > 0
                && previous_parent
                    .as_ref()
                    .is_some_and(|parent| parent.id != self.id);
            if is_reparented {
                child.mark_canvas_subtree_dirty_for_reparent(context, true);
                if let Some(previous_parent) = previous_parent.as_ref() {
                    previous_parent.detach_child_for_reparent(child);
                }
            }

            bindings_changed |=
                child.ensure_parent_bindings(self, context, parent_frame, liquid_glass_scope);
            if is_reparented {
                child.rebind_parent_bounds_for_mounted_subtree(context);
                child.mark_canvas_subtree_dirty_for_reparent(context, false);
            }
        }
        // Reconciliation still evaluates item data and parent context before
        // this fast path. Exiting children remain managed by their cleanup lane.
        if Self::same_children(&borrow!(self.active_children), &new_children)
            && (self.attached.get() == 0
                || new_children.iter().all(|child| child.attached.get() > 0))
        {
            if bindings_changed {
                self.mark_non_reactive_update_subtree_dirty();
            }
            return self.current_attached_children();
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
                        self.retain_exiting_child(Rc::clone(child), context);
                    }
                    continue;
                }
                if child.start_exit_transition_tree(context) {
                    self.retain_exiting_child(Rc::clone(child), context);
                } else {
                    Rc::clone(child).recurse_unmount(context);
                }
            }
            for child in new_children.iter() {
                if !Self::has_child(&old_active_children, child) || child.attached.get() == 0 {
                    if child.attached.get() > 0 {
                        child.start_enter_transition_tree(context);
                    } else {
                        Rc::clone(child).recurse_mount(context);
                        newly_mounted_children.push(Rc::clone(child));
                    }
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
            child.ensure_parent_bindings(self, context, parent_frame, &self.liquid_glass_scope);
        }
        *borrow_mut!(self.sidecar_children) = new_children.clone();
        self.mark_non_reactive_update_subtree_dirty();
        new_children
    }

    fn ensure_parent_bindings(
        self: &Rc<Self>,
        parent: &Rc<Self>,
        context: &Rc<RuntimeContext>,
        frame: &Property<Option<ExpandedNodeIdentifier>>,
        glass: &Property<Option<NativeLiquidGlassScope>>,
    ) -> bool {
        let properties = {
            let parent_cp = parent.get_common_properties();
            let parent_cp = borrow!(parent_cp);
            let cp = self.get_common_properties();
            let cp = borrow!(cp);
            vec![
                frame.untyped(),
                glass.untyped(),
                parent.suspended.untyped(),
                parent.transform_and_bounds.untyped(),
                parent.computed_opacity.untyped(),
                parent_cp.padding_x.untyped(),
                parent_cp.padding_y.untyped(),
                self.container_frame.untyped(),
                self.measured_size.untyped(),
                cp._suspended.untyped(),
                cp.opacity.untyped(),
                cp.transform.untyped(),
                cp.width.untyped(),
                cp.height.untyped(),
                cp.x.untyped(),
                cp.y.untyped(),
                cp.anchor_x.untyped(),
                cp.anchor_y.untyped(),
                cp.scale_x.untyped(),
                cp.scale_y.untyped(),
                cp.skew_x.untyped(),
                cp.skew_y.untyped(),
                cp.rotate.untyped(),
            ]
        };
        let sources = ParentBindingSources {
            parent: parent.id,
            properties,
        };
        let same_parent = borrow!(self.render_parent)
            .upgrade()
            .is_some_and(|old| Rc::ptr_eq(&old, parent));
        if same_parent
            && borrow!(self.parent_binding_sources)
                .as_ref()
                .is_some_and(|old| old.same_as(&sources))
        {
            return false;
        }
        *borrow_mut!(self.render_parent) = Rc::downgrade(parent);
        let frame = frame.clone();
        let deps = [frame.untyped()];
        self.parent_frame
            .replace_with(Property::computed(move || frame.get(), &deps));
        let glass = glass.clone();
        let deps = [glass.untyped()];
        self.liquid_glass_scope
            .replace_with(Property::computed(move || glass.get(), &deps));
        self.inherit_suspend(parent);
        self.bind_to_parent_bounds(context);
        *borrow_mut!(self.parent_binding_sources) = Some(sources);
        true
    }

    fn rebind_parent_bounds_for_mounted_subtree(self: &Rc<Self>, context: &Rc<RuntimeContext>) {
        self.bind_to_parent_bounds(context);
        self.bind_occlusion_listener(context);
        let mounted_children = borrow!(self.mounted_children).clone();
        for child in mounted_children {
            child.rebind_parent_bounds_for_mounted_subtree(context);
        }
        let sidecar_children = borrow!(self.sidecar_children).clone();
        for child in sidecar_children {
            child.rebind_parent_bounds_for_mounted_subtree(context);
        }
    }

    fn mark_canvas_subtree_dirty_for_reparent(
        self: &Rc<Self>,
        context: &Rc<RuntimeContext>,
        queue_retained_removal: bool,
    ) {
        let layer = self.occlusion.get().render_layer_id;
        context.set_canvas_dirty(layer);
        if borrow!(self.instance_node).base().flags().layer == Layer::Canvas {
            if queue_retained_removal {
                context.enqueue_canvas_node_removal(layer, self.id.to_u32());
            }
            context.mark_canvas_node_dirty(self.id);
        }
        context.mark_occlusion_dirty();

        let mounted_children = borrow!(self.mounted_children).clone();
        for child in mounted_children {
            child.mark_canvas_subtree_dirty_for_reparent(context, queue_retained_removal);
        }
        let sidecar_children = borrow!(self.sidecar_children).clone();
        for child in sidecar_children {
            child.mark_canvas_subtree_dirty_for_reparent(context, queue_retained_removal);
        }
    }

    fn bind_to_parent_bounds(self: &Rc<Self>, ctx: &Rc<RuntimeContext>) {
        // Explicit reload/reparent rebuilds invalidate the attachment fast path.
        self.parent_binding_sources.borrow_mut().take();
        self.layout_binding_generation
            .set(self.layout_binding_generation.get().wrapping_add(1));
        self.subtree_layout_hull_listener.invalidate();
        self.children_listener.invalidate();
        #[cfg(test)]
        self.parent_binding_rebuilds
            .set(self.parent_binding_rebuilds.get() + 1);
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

        // Structural child changes are handled by children_listener after
        // reconciliation; data-only repeat invalidation is not an occlusion change.
        deps.extend([
            self.transform_and_bounds.untyped(),
            self.computed_opacity.untyped(),
            self.liquid_glass_scope.untyped(),
        ]);

        let context = Rc::clone(ctx);
        self.occlusion_listener
            .replace_with(Property::computed_with_name(
                move || {
                    context.mark_occlusion_dirty();
                },
                &deps,
                "occlusion listener",
            ));
        ctx.register_expanded_node_effect_property_named(
            self,
            &self.occlusion_listener,
            "occlusion listener",
        );
    }

    fn reapply_runtime_settings(self: &Rc<Self>, context: &Rc<RuntimeContext>) {
        let instance = Rc::clone(&*borrow!(self.instance_node));
        instance
            .base()
            .instance_prototypical_common_properties
            .materialize(Rc::clone(&self.stack), Some(Rc::clone(self)));
        instance
            .base()
            .instance_prototypical_properties
            .materialize(Rc::clone(&self.stack), Some(Rc::clone(self)));
        self.reset_removed_runtime_properties.set(false);
        self.refresh_properties_scope(&instance);
        self.mark_non_reactive_update_subtree_dirty();
        context.mark_occlusion_dirty();
        context.set_canvas_dirty(self.occlusion.get().render_layer_id);
    }

    fn bind_selector_classes_listener(self: &Rc<Self>, context: &Rc<RuntimeContext>) {
        let classes = self.selector_metadata.borrow().classes.clone();
        let previous = Rc::new(RefCell::new(classes.get()));
        let weak_self = Rc::downgrade(self);
        let runtime_context = Rc::clone(context);
        self.selector_classes_listener
            .replace_with(Property::computed_with_name(
                move || {
                    let current = classes.get();
                    if *previous.borrow() == current {
                        return;
                    }
                    *previous.borrow_mut() = current;
                    let Some(node) = weak_self.upgrade() else {
                        return;
                    };
                    node.reset_removed_runtime_properties.set(true);
                    node.reapply_runtime_settings(&runtime_context);
                },
                &[self.selector_metadata.borrow().classes.untyped()],
                "selector classes listener",
            ));
        context.register_expanded_node_effect_property_named(
            self,
            &self.selector_classes_listener,
            "selector classes listener",
        );
    }

    fn bind_children_listener(self: &Rc<Self>, ctx: &Rc<RuntimeContext>) {
        let deps = [self.children.untyped()];
        let weak_self = Rc::downgrade(self);
        let context = Rc::clone(ctx);
        let previous = RefCell::new(None);
        self.children_listener
            .replace_with(Property::computed_with_name(
                move || {
                    let Some(node) = weak_self.upgrade() else {
                        return;
                    };
                    if !node.child_structure_changed(&previous) {
                        return;
                    }
                    #[cfg(test)]
                    node.structural_children_updates
                        .set(node.structural_children_updates.get() + 1);
                    if borrow!(node.instance_node).base().flags().is_component
                        || borrow!(node.expanded_projected_children).is_some()
                    {
                        node.compute_flattened_projected_children();
                    }
                    context.mark_occlusion_dirty();

                    let (is_slot, is_component) = {
                        let instance = borrow!(node.instance_node);
                        let flags = instance.base().flags();
                        (flags.is_slot, flags.is_component)
                    };
                    if !is_slot {
                        if is_component {
                            node.slot_projection_changed.set(());
                        } else if let Some(containing_component) =
                            node.containing_component.upgrade()
                        {
                            containing_component.slot_projection_changed.set(());
                        }
                    }
                },
                &deps,
                "children listener",
            ));
        ctx.register_expanded_node_effect_property_named(
            self,
            &self.children_listener,
            "children listener",
        );
    }

    fn bind_subtree_layout_hull(self: &Rc<Self>, ctx: &Rc<RuntimeContext>) {
        self.rebind_subtree_layout_hull();
        let deps = [self.children.untyped()];
        let weak_self = Rc::downgrade(self);
        let previous = RefCell::new(None);
        self.subtree_layout_hull_listener
            .replace_with(Property::computed_with_name(
                move || {
                    let Some(node) = weak_self.upgrade() else {
                        return;
                    };
                    if node.child_structure_changed(&previous) {
                        node.rebind_subtree_layout_hull();
                    }
                },
                &deps,
                "subtree layout hull listener",
            ));
        ctx.register_expanded_node_effect_property_named(
            self,
            &self.subtree_layout_hull_listener,
            "subtree layout hull listener",
        );
    }
    fn child_structure_changed(&self, previous: &RefCell<Option<ChildStructure>>) -> bool {
        // Always evaluate first: reconciliation may update data, rescue an exit,
        // or remove a completed exit even when the source keys are unchanged.
        let children = self.children.get();
        let ids = |children: &[Rc<ExpandedNode>]| {
            children.iter().map(|child| child.id).collect::<Vec<_>>()
        };
        // Slot projection distinguishes active from exiting children even when
        // their combined render order is unchanged at the start of an exit.
        let current = ChildStructure {
            binding_generation: self.layout_binding_generation.get(),
            rendered: ids(&children),
            active: ids(&borrow!(self.active_children)),
            exiting: ids(&borrow!(self.exiting_children)),
        };
        let mut previous = previous.borrow_mut();
        if previous.as_ref() == Some(&current) {
            return false;
        }
        *previous = Some(current);
        true
    }

    fn rebind_subtree_layout_hull(self: &Rc<Self>) {
        #[cfg(test)]
        self.layout_hull_rebuilds
            .set(self.layout_hull_rebuilds.get() + 1);
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
        let bounds_name = format!("layout bounds (node id: {})", self.id.0);
        let bounds_transform_and_bounds = self_transform_and_bounds.clone();
        let bounds = Property::computed_with_cutoff_and_name(
            move || bounds_transform_and_bounds.get().bounds,
            &[self_transform_and_bounds.untyped()],
            <(f64, f64)>::eq,
            &bounds_name,
        );
        let mut own_deps = vec![bounds.untyped()];
        if has_children {
            own_deps.extend([width.untyped(), height.untyped(), x.untyped(), y.untyped()]);
        } else {
            own_deps.push(layout_properties.untyped());
        }

        let own_hull_name = format!("own layout hull (node id: {})", self.id.0);
        let own_hull = Property::computed_with_cutoff_and_name(
            move || {
                let bounds = bounds.get();
                let (contributes_x, contributes_y) = if has_children {
                    (
                        layout_axis_can_contribute_from_parts(width.get(), x.get()),
                        layout_axis_can_contribute_from_parts(height.get(), y.get()),
                    )
                } else {
                    let layout_properties = layout_properties.get();
                    (
                        layout_axis_can_contribute(&layout_properties, Axis::X),
                        layout_axis_can_contribute(&layout_properties, Axis::Y),
                    )
                };
                LayoutHull::from_axis_ranges(
                    contributes_x.then_some((0.0, bounds.0)),
                    contributes_y.then_some((0.0, bounds.1)),
                )
            },
            &own_deps,
            crate::layout_hulls_equivalent,
            &own_hull_name,
        );

        if !has_children {
            self.subtree_layout_hull.replace_with(own_hull);
            return;
        }

        let mut projected_child_hulls = Vec::with_capacity(children.len());
        for child in children.iter() {
            let child_cp = child.get_common_properties();
            let child_layout_role = borrow!(child_cp).layout_role.clone();
            let projection_name = format!(
                "projected child layout hull (parent id: {}, child id: {})",
                self.id.0, child.id.0
            );
            projected_child_hulls.push(crate::projected_child_layout_hull_property(
                self_transform_and_bounds.clone(),
                padding_x.clone(),
                padding_y.clone(),
                child.transform_and_bounds.clone(),
                child.subtree_layout_hull.clone(),
                child_layout_role,
                &projection_name,
            ));
        }

        let mut deps = vec![own_hull.untyped(), padding_x.untyped(), padding_y.untyped()];
        deps.extend(projected_child_hulls.iter().map(Property::untyped));

        let property_name = format!("subtree layout hull (node id: {})", self.id.0);
        self.subtree_layout_hull
            .replace_with(Property::computed_with_cutoff_and_name(
                move || {
                    let mut hull = own_hull.get();
                    let mut children_hull = LayoutHull::default();
                    for projected_hull in &projected_child_hulls {
                        children_hull = children_hull.union(projected_hull.get());
                    }
                    hull = hull.union(add_symmetric_padding_to_content_layout_hull(
                        children_hull,
                        padding_x.get(),
                        padding_y.get(),
                    ));

                    hull
                },
                &deps,
                crate::layout_hulls_equivalent,
                &property_name,
            ));
    }

    pub fn inherit_suspend(self: &Rc<Self>, node: &Rc<Self>) {
        self.parent_binding_sources.borrow_mut().take();
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
            self.reset_removed_runtime_properties.set(true);
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
            context.register_expanded_node_effect_property_named(
                self,
                &self.changed_listener,
                "changed listener",
            );
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
            self.selector_classes_listener
                .replace_with(Property::default());
            borrow_mut!(self.subscriptions).clear();
            borrow_mut!(self.active_children).clear();
            borrow_mut!(self.exiting_children).clear();
            borrow_mut!(self.exiting_child_generations).clear();
            borrow_mut!(self.mounted_children).clear();
            borrow_mut!(self.imported_settings_layers).clear();
            borrow_mut!(self.imported_settings_signature).clear();
            borrow_mut!(self.resolved_property_columns).clear();
            borrow_mut!(self.resolved_property_provenance).clear();
            self.reset_removed_runtime_properties.set(false);
            self.active_children_view.set(Vec::new());
            self.exiting_children_view.set(Vec::new());
            self.transition_phase.set(TRANSITION_PHASE_IDLE);
            self.transition_takeover.set(false);
            self.transition_selector_classes.borrow_mut().take();
            self.deactivate_transition_clock();
            self.enter_cleanup_active.set(false);
            self.enter_cleanup_listener
                .replace_with(Property::new_with_name((), "enter transition cleanup"));
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
                            provider_stack: node.stack.push(borrow!(node.properties_scope).clone()),
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
        // to make stacker/scroller behave correctly when the number of children is dynamic.
        self.compute_flattened_projected_children();
        for child in self.children.get().iter().rev() {
            child.recurse_visit_postorder(func)
        }
        func(self);
    }

    pub fn get_node_context(self: &Rc<Self>, ctx: &Rc<RuntimeContext>) -> NodeContext {
        let globals = ctx.globals();
        let viewport = super::viewport_info_property(&globals.viewport);
        let target = Property::new(globals.target);
        let t_and_b = self.transform_and_bounds.clone();
        let deps = [t_and_b.untyped()];
        let bounds_name = format!("node context bounds (node id: {})", self.id.0);
        let bounds_self = Property::computed_with_cutoff_and_name(
            move || t_and_b.get().bounds,
            &deps,
            <(f64, f64)>::eq,
            &bounds_name,
        );
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

        let last_frame = Rc::new(RefCell::new(globals.elapsed_frames.get()));
        let suspended = self.suspended.clone();
        let elapsed_frames = globals.elapsed_frames.clone();
        let elapsed_millis = globals.elapsed_millis.clone();
        let deps = [elapsed_frames.untyped(), suspended.untyped()];
        // TODO: this still triggers the dirty dag dependencies of elapsed
        // frames even if the value is the same. Try to make it not trigger
        // dependencides when frozen
        let elapsed_frames_frozen_if_suspended = Property::computed(
            move || {
                if suspended.get() {
                    *borrow!(last_frame)
                } else {
                    let val = elapsed_frames.get();
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
            elapsed_frames: elapsed_frames_frozen_if_suspended,
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
            target,
            viewport,
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
    dispatch_event_handler!(
        dispatch_touch_cancel,
        TouchCancel,
        TOUCH_CANCEL_HANDLERS,
        true
    );
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
        dispatch_slider_change,
        SliderChange,
        SLIDER_CHANGE_HANDLERS,
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
