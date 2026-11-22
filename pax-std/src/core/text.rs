use pax_engine::api::LocalProperty;
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};

use pax_engine::api::{Color, Layer, Numeric, Paint, Property, RenderContext, Size};
use pax_engine::*;

use pax_message::{
    AnyCreatePatch, ColorMessage, FontPatch, FontStyleMessage, FontWeightMessage,
    TextAlignHorizontalMessage, TextAlignVerticalMessage, TextPatch, TextStyleMessage,
    WebFontMessage,
};

use pax_runtime::api::{borrow, borrow_mut, use_RefCell};

use_RefCell!();
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
#[cfg(feature = "designtime")]
use {
    kurbo::{RoundedRect, Shape},
    pax_runtime::DEBUG_TEXT_GREEN_BACKGROUND,
};

use crate::common::{native_surface_opacity, patch_if_needed};

/// Renders and styles text through the target's native text system.
///
/// A constrained width with an omitted height allows native measurement to
/// determine the height of wrapped content. Selection, editing, font loading,
/// and accessibility behavior depend on the target's native implementation;
/// test the intended interaction on each shipping target.
#[pax]
#[engine_import_path("pax_engine")]
#[custom(Default)]
#[primitive("pax_std::core::text::TextInstance")]
#[pax_document(text)]
pub struct Text {
    /// Whether the text can be edited by the user.
    /// Android edits the authored source when `markdown` is also enabled.
    pub editable: Property<bool>,
    /// Requests selectable text. On the current iOS/iPadOS path, non-editable
    /// text uses the interactive selection view only when `clip` is enabled.
    pub selectable: Property<bool>,
    /// Whether text overflow is clipped to the node bounds.
    pub clip: Property<bool>,
    /// Text content to display, also assignable with `<Text>content</Text>`.
    /// Body expressions read state; use `text=bind:...` for write-through editing.
    #[pax(content)]
    pub text: Property<String>,
    /// Text styling.
    pub style: Property<TextStyle>,
    // Secondary style link used by native text patch plumbing.
    pub _style_link: Property<TextStyle>,
    /// Whether `text` should be interpreted as Markdown.
    /// Android renders CommonMark and Pax-generated source highlighting through
    /// native text spans; other HTML remains literal and images show alt text.
    pub markdown: Property<bool>,
    /// Whether long text lines should wrap inside the node bounds.
    pub wrap: Property<bool>,
}

impl Default for Text {
    fn default() -> Self {
        Self {
            editable: Property::new(false),
            selectable: Property::new(true),
            clip: Property::new(false),
            text: Property::new("".to_string()),
            style: Property::new(TextStyle::default()),
            _style_link: Property::new(TextStyle::default()),
            markdown: Property::new(false),
            wrap: Property::new(true),
        }
    }
}

// Runtime instance backing `<Text>`.
pub struct TextInstance {
    base: BaseInstance,
    measurements: RefCell<TextMeasurements>,
}

#[derive(Clone, PartialEq)]
struct TextMeasurementKey {
    content: String,
    markdown: bool,
    wrap: bool,
    clip: bool,
    editable: bool,
    selectable: bool,
    style: TextStyleMessage,
    style_link: TextStyleMessage,
    width_constraint_bits: Option<u64>,
    height_constraint_bits: Option<u64>,
}

#[derive(Default)]
struct TextMeasurementState {
    next_generation: u64,
    requested_key: Option<TextMeasurementKey>,
    pending_generation: Option<u64>,
    invalidation_revision: LocalProperty<u64>,
}

thread_local! {
    // A remount can keep an expanded ID while replacing its template instance.
    // Tokens must therefore outlive both the node's cache and that instance.
    // Expanded nodes are thread-confined, so a per-thread sequence is sufficient.
    static TEXT_MEASUREMENT_GENERATION: Cell<u64> = const { Cell::new(0) };
}

// Repeats share a template instance, but each expanded native Text owns an
// independent request sequence. Keep generations until unmount, even while
// both dimensions are temporarily explicit, so late replies remain stale.
#[derive(Default)]
struct TextMeasurements {
    nodes: HashMap<u32, TextMeasurementState>,
}

impl TextMeasurements {
    fn request(&mut self, id: u32, key: TextMeasurementKey, measured: bool) -> Option<u64> {
        self.nodes.entry(id).or_default().request(key, measured)
    }

    fn accept(&mut self, id: u32, generation: u64) -> bool {
        self.nodes
            .get_mut(&id)
            .is_some_and(|state| state.accept(generation))
    }

    fn invalidate(&mut self, id: u32, generation: u64) -> bool {
        self.nodes
            .get_mut(&id)
            .is_some_and(|state| state.invalidate(generation))
    }

    fn clear(&mut self, id: u32) {
        if let Some(state) = self.nodes.get_mut(&id) {
            state.requested_key = None;
            state.pending_generation = None;
        }
    }
}

impl TextMeasurementState {
    fn request(&mut self, key: TextMeasurementKey, has_current_measurement: bool) -> Option<u64> {
        let key_changed = self.requested_key.as_ref() != Some(&key);
        if key_changed || (!has_current_measurement && self.pending_generation.is_none()) {
            self.next_generation = TEXT_MEASUREMENT_GENERATION.with(|counter| {
                let next = counter
                    .get()
                    .checked_add(1)
                    .expect("text measurement token exhausted");
                counter.set(next);
                next
            });
            let generation = self.next_generation;
            self.requested_key = Some(key);
            self.pending_generation = Some(generation);
            Some(generation)
        } else {
            None
        }
    }

    fn accept(&mut self, generation: u64) -> bool {
        if self.pending_generation != Some(generation) {
            return false;
        }
        self.pending_generation = None;
        true
    }

    fn invalidate(&mut self, generation: u64) -> bool {
        if self.next_generation != generation || self.requested_key.is_none() {
            return false;
        }
        // Keep the previous measured bounds until the replacement arrives, but
        // reject any in-flight response measured with the old native metrics.
        self.requested_key = None;
        self.pending_generation = None;
        self.invalidation_revision
            .set(self.invalidation_revision.get().wrapping_add(1));
        true
    }
}

impl TextInstance {
    fn request_measurement_generation(
        &self,
        id: u32,
        key: TextMeasurementKey,
        has_current_measurement: bool,
    ) -> Option<u64> {
        self.measurements
            .borrow_mut()
            .request(id, key, has_current_measurement)
    }

    fn clear_measurement_request(&self, expanded_node: &Rc<ExpandedNode>) {
        self.measurements
            .borrow_mut()
            .clear(expanded_node.id.to_u32());
        if expanded_node.measured_size.get().is_some() {
            expanded_node.measured_size.set(None);
        }
    }

    fn accept_measurement_response(
        &self,
        expanded_node: &Rc<ExpandedNode>,
        generation: u64,
        width: f64,
        height: f64,
    ) {
        let mut measurements = self.measurements.borrow_mut();
        if !measurements.accept(expanded_node.id.to_u32(), generation) {
            return;
        }
        drop(measurements);
        expanded_node.set_measured_size(width, height);
    }
}

impl InstanceNode for TextInstance {
    fn may_supply_clip(&self) -> bool {
        false
    }

    fn geometry_is_scroller_local(&self) -> bool {
        true
    }

    fn property_requires_input_recompute(&self, _property_name: &str) -> bool {
        false
    }

    fn instantiate(args: InstantiationArgs) -> Rc<Self>
    where
        Self: Sized,
    {
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    invisible_to_slot: false,
                    invisible_to_raycasting: false,
                    layer: Layer::Native,
                    is_component: false,
                    is_slot: false,
                },
            ),
            measurements: RefCell::new(TextMeasurements::default()),
        })
    }

    fn render(
        &self,
        _expanded_node: &ExpandedNode,
        _context: &Rc<RuntimeContext>,
        _rc: &mut dyn RenderContext,
    ) {
        // Text is presented by the chassis native text system.

        #[cfg(feature = "designtime")]
        if DEBUG_TEXT_GREEN_BACKGROUND {
            let tab = _expanded_node.presentation_transform_and_bounds.get();
            let width: f64 = tab.bounds.0;
            let height: f64 = tab.bounds.1;
            let rect = RoundedRect::new(0.0, 0.0, width, height, 0.0);
            let bez_path = rect.to_path(0.1);
            let transformed_bez_path = Into::<kurbo::Affine>::into(tab.transform) * bez_path;
            _rc.fill(
                _expanded_node.occlusion.get().render_layer_id,
                transformed_bez_path,
                &Paint::Solid(Color::rgba(0.into(), 255.into(), 0.into(), 100.into())),
            );
        }
    }

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        // Send creation message
        let id = expanded_node.id.to_u32();
        context.enqueue_native_message(pax_message::NativeMessage::TextCreate(AnyCreatePatch {
            id,
            parent_frame: expanded_node.parent_frame.get().map(|v| v.to_u32()),
            render_layer_id: 0,
        }));

        // send update message when relevant properties change
        let weak_self_ref = Rc::downgrade(&expanded_node);
        let instance = Rc::clone(&self);
        let last_patch = Rc::new(RefCell::new(TextPatch {
            id,
            ..Default::default()
        }));

        // A world-pose change must not clone strings/styles or reconstruct a
        // measurement key. Width/height constraints cut off before that work.
        let bounds = expanded_node.presentation_bounds();
        let cp = expanded_node.get_common_properties();
        let cp = borrow!(cp);
        let width = cp.width.clone();
        let height = cp.height.clone();
        let deps = [bounds.untyped(), width.untyped(), height.untyped()];
        let constraints = LocalProperty::computed_with_cutoff(
            move || {
                let (w, h) = bounds.get();
                (
                    if width.get().is_some() { w } else { -1. },
                    if height.get().is_some() { h } else { -1. },
                )
            },
            &deps,
            PartialEq::eq,
        );
        // OS metric changes must invalidate the content/measurement computation,
        // not just the independent placement listener that consumes its cache.
        let invalidation = self
            .measurements
            .borrow_mut()
            .nodes
            .entry(id)
            .or_default()
            .invalidation_revision
            .clone();
        let mut deps = vec![constraints.untyped(), invalidation.untyped()];
        expanded_node.with_properties_unwrapped(|p: &mut Text| {
            deps.extend([
                p.text.untyped(),
                p.markdown.untyped(),
                p.wrap.untyped(),
                p.style.untyped(),
                p._style_link.untyped(),
                p.editable.untyped(),
                p.selectable.untyped(),
                p.clip.untyped(),
            ]);
        });
        let content_context = Rc::clone(context);
        let content_state = last_patch.clone();
        let content = LocalProperty::computed(
            move || {
                let Some(expanded_node) = weak_self_ref.upgrade() else {
                    return;
                };
                let _work = content_context.profile_node_work(id, "text content and measurement");
                let mut old_state = borrow_mut!(content_state);

                let mut patch = TextPatch {
                    id,
                    ..Default::default()
                };
                expanded_node.with_properties_unwrapped(|properties: &mut Text| {
                    // send width/height only if common props exist, otherwise we are in "listening mode"
                    // trying to infer width and height from the engine. To signal this we
                    // send width/height = -1.0, telling chassis that "you tell me!".
                    let (width, height) = constraints.get();
                    let content = properties.text.local().get();
                    let markdown = properties.markdown.local().get();
                    let wrap = properties.wrap.local().get();
                    let style: TextStyleMessage = properties.style.local().get().message(true);
                    let style_link: TextStyleMessage =
                        properties._style_link.local().get().message(true);
                    let editable = properties.editable.local().get();
                    let selectable = properties.selectable.local().get();
                    let clip = properties.clip.local().get();

                    let updates = [
                        // Content
                        patch_if_needed(
                            &mut old_state.content,
                            &mut patch.content,
                            content.clone(),
                        ),
                        patch_if_needed(&mut old_state.markdown, &mut patch.markdown, markdown),
                        patch_if_needed(&mut old_state.wrap, &mut patch.wrap, wrap),
                        // Styles
                        patch_if_needed(&mut old_state.style, &mut patch.style, style.clone()),
                        patch_if_needed(
                            &mut old_state.style_link,
                            &mut patch.style_link,
                            style_link.clone(),
                        ),
                        patch_if_needed(&mut old_state.editable, &mut patch.editable, editable),
                        patch_if_needed(
                            &mut old_state.selectable,
                            &mut patch.selectable,
                            selectable,
                        ),
                        patch_if_needed(&mut old_state.clip, &mut patch.clip, clip),
                        // Transform and bounds
                        patch_if_needed(&mut old_state.size_x, &mut patch.size_x, width),
                        patch_if_needed(&mut old_state.size_y, &mut patch.size_y, height),
                    ];
                    let measurement_key = text_measurement_key(
                        content.clone(),
                        markdown,
                        wrap,
                        clip,
                        editable,
                        selectable,
                        style,
                        style_link,
                        width,
                        height,
                    );
                    if let Some(measurement_key) = measurement_key {
                        patch.measure_generation = instance.request_measurement_generation(
                            id,
                            measurement_key,
                            expanded_node.measured_size.get().is_some(),
                        );
                    } else {
                        instance.clear_measurement_request(&expanded_node);
                    }

                    if updates.into_iter().any(|v| v == true) || patch.measure_generation.is_some()
                    {
                        // HACK: markdown flag and content is needed at the same time, so if we have one,
                        // include the other:
                        if patch.markdown.is_some() && patch.content.is_none() {
                            patch.content = Some(content);
                        }
                        if patch.content.is_some() && patch.markdown.is_none() {
                            patch.markdown = Some(markdown);
                        }
                        content_context
                            .enqueue_native_message(pax_message::NativeMessage::TextUpdate(patch));
                    }
                });
                ()
            },
            &deps,
        );
        let placement = expanded_node.native_placement(context);
        let weak_node = Rc::downgrade(expanded_node);
        let context = Rc::clone(context);
        let deps = [
            content.untyped(),
            placement.untyped(),
            expanded_node.parent_frame.untyped(),
            expanded_node.occlusion.untyped(),
            expanded_node.computed_opacity.untyped(),
        ];
        expanded_node
            .changed_listener
            .replace_with(LocalProperty::computed(
                move || {
                    content.get();
                    let Some(node) = weak_node.upgrade() else {
                        return;
                    };
                    let (scroller, transform) = placement.get();
                    let mut old = borrow_mut!(last_patch);
                    let mut patch = TextPatch {
                        id,
                        ..Default::default()
                    };
                    let owner_changed = patch_if_needed(
                        &mut old.transform_scroller,
                        &mut patch.transform_scroller,
                        scroller,
                    );
                    let changed = [
                        patch_if_needed(
                            &mut old.parent_frame,
                            &mut patch.parent_frame,
                            node.parent_frame.get().map(|v| v.to_u32()),
                        ),
                        patch_if_needed(
                            &mut old.z_index,
                            &mut patch.z_index,
                            node.occlusion.get().z_index,
                        ),
                        patch_if_needed(
                            &mut old.transform,
                            &mut patch.transform,
                            transform.clone(),
                        ),
                        patch_if_needed(
                            &mut old.opacity,
                            &mut patch.opacity,
                            native_surface_opacity(&node, &context),
                        ),
                    ];
                    // A coordinate-space switch is atomic even if the coefficients match.
                    if owner_changed {
                        patch.transform = Some(transform);
                    }
                    if owner_changed || changed.into_iter().any(|c| c) {
                        context
                            .enqueue_native_message(pax_message::NativeMessage::TextUpdate(patch));
                    }
                },
                &deps,
            ));
    }

    fn handle_unmount(&self, expanded_node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        let id = expanded_node.id.to_u32();
        self.measurements.borrow_mut().nodes.remove(&id);
        expanded_node
            .changed_listener
            .replace_with(LocalProperty::default());
        context.enqueue_native_message(pax_message::NativeMessage::TextDelete(id));
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        match expanded_node {
            Some(expanded_node) => expanded_node.with_properties_unwrapped(|r: &mut Text| {
                f.debug_struct("Text")
                    .field("text", &r.text.local().get())
                    .finish()
            }),
            None => f.debug_struct("Text").finish_non_exhaustive(),
        }
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    fn property_requires_occlusion_recompute(&self, _property_name: &str) -> bool {
        // Native text occludes by its frame; content/style updates are native patches.
        false
    }

    fn handle_native_interrupt(
        &self,
        expanded_node: &Rc<ExpandedNode>,
        interrupt: &pax_message::NativeInterrupt,
    ) {
        if let pax_message::NativeInterrupt::TextInput(args) = interrupt {
            expanded_node.with_properties_unwrapped(|properties: &mut Text| {
                properties.text.set(args.text.clone())
            });
        } else if let pax_message::NativeInterrupt::TextMeasurementResponse(args) = interrupt {
            self.accept_measurement_response(
                expanded_node,
                args.generation,
                args.width,
                args.height,
            );
        } else if let pax_message::NativeInterrupt::TextMeasurementInvalidated(args) = interrupt {
            if self
                .measurements
                .borrow_mut()
                .invalidate(expanded_node.id.to_u32(), args.generation)
            {
                // Native metrics are outside the property graph. Schedule the
                // normal patch evaluator even when authored bounds are unchanged.
                expanded_node.changed_listener.invalidate();
            }
        } else {
            log::warn!("text element was handed interrupt it doesn't use");
        }
    }
}

fn explicit_constraint_bits(value: f64) -> Option<u64> {
    (value >= 0.0).then_some(value.to_bits())
}

#[cfg(test)]
mod measurement_tests {
    use super::*;

    fn key(content: &str) -> TextMeasurementKey {
        text_measurement_key(
            content.to_owned(),
            false,
            true,
            false,
            false,
            true,
            TextStyleMessage::default(),
            TextStyleMessage::default(),
            200.0,
            -1.0,
        )
        .unwrap()
    }

    #[test]
    fn native_metric_change_remeasures_an_unchanged_authored_key() {
        let mut state = TextMeasurementState::default();
        let first = state.request(key("hello"), false).unwrap();
        assert!(state.accept(first));
        assert_eq!(state.request(key("hello"), true), None);
        assert!(state.invalidate(first));
        assert!(
            !state.invalidate(first),
            "coalesce duplicate native notifications"
        );
        let replacement = state.request(key("hello"), true).unwrap();
        assert_ne!(replacement, first);
        assert!(!state.accept(first));
        assert!(state.accept(replacement));
        assert!(!state.accept(replacement), "each response is accepted once");
    }

    #[test]
    fn native_metric_change_rejects_an_already_in_flight_measurement() {
        let mut state = TextMeasurementState::default();
        let first = state.request(key("hello"), false).unwrap();
        assert!(state.invalidate(first));
        assert!(!state.accept(first));
        let replacement = state.request(key("hello"), false).unwrap();
        assert!(state.accept(replacement));
    }

    #[test]
    fn stale_native_invalidation_cannot_cancel_a_new_content_request() {
        let mut state = TextMeasurementState::default();
        let first = state.request(key("hello"), false).unwrap();
        let second = state.request(key("updated"), false).unwrap();
        assert!(!state.invalidate(first));
        assert!(!state.accept(first));
        assert!(state.accept(second));
    }

    #[test]
    fn fixed_dimensions_do_not_request_intrinsic_measurement() {
        assert!(text_measurement_key(
            "fixed".to_owned(),
            false,
            true,
            false,
            false,
            true,
            TextStyleMessage::default(),
            TextStyleMessage::default(),
            0.0,
            30.0,
        )
        .is_none());
    }

    #[test]
    fn repeated_text_nodes_own_independent_measurement_sequences() {
        let mut measurements = TextMeasurements::default();
        let first = measurements.request(10, key("same"), false).unwrap();
        let sibling = measurements.request(20, key("same"), false).unwrap();
        assert!(measurements.accept(10, first));
        assert!(measurements.accept(20, sibling));

        assert!(measurements.invalidate(10, first));
        assert!(measurements.invalidate(20, sibling));
        let first = measurements.request(10, key("same"), true).unwrap();
        let sibling = measurements.request(20, key("different"), true).unwrap();
        // One sibling becoming fixed-size must not discard the other's request.
        measurements.clear(10);
        assert!(!measurements.accept(10, first));
        assert!(measurements.accept(20, sibling));
        let replacement = measurements.request(10, key("same"), true).unwrap();
        assert_ne!(first, replacement);
        assert!(!measurements.accept(10, first));
        assert!(measurements.accept(10, replacement));

        measurements.nodes.remove(&10);
        assert!(!measurements.invalidate(10, replacement));
        assert!(measurements.nodes.contains_key(&20));
    }

    #[test]
    fn remounted_text_rejects_old_responses_even_when_its_template_is_replaced() {
        let mut old_template = TextMeasurements::default();
        let old = old_template.request(10, key("same"), false).unwrap();
        old_template.nodes.remove(&10);
        let mut new_template = TextMeasurements::default();
        let replacement = new_template.request(10, key("same"), false).unwrap();
        assert_ne!(old, replacement);
        assert!(!new_template.accept(10, old));
        assert!(!new_template.invalidate(10, old));
        assert!(new_template.accept(10, replacement));
    }
}

fn text_measurement_key(
    content: String,
    markdown: bool,
    wrap: bool,
    clip: bool,
    editable: bool,
    selectable: bool,
    style: TextStyleMessage,
    style_link: TextStyleMessage,
    width: f64,
    height: f64,
) -> Option<TextMeasurementKey> {
    if width >= 0.0 && height >= 0.0 {
        return None;
    }

    Some(TextMeasurementKey {
        content,
        markdown,
        wrap,
        clip,
        editable,
        selectable,
        style,
        style_link,
        width_constraint_bits: explicit_constraint_bits(width),
        height_constraint_bits: explicit_constraint_bits(height),
    })
}

/// Struct describing platform-agnostic text display properties.
/// Interpolation eases font size and fill; font selection, underline, and
/// alignment switch to the destination immediately. Glyphs do not morph.
#[pax]
#[engine_import_path("pax_engine")]
#[custom(Default, Interpolatable)]
pub struct TextStyle {
    #[serde(default)]
    /// Font family/source/style/weight configuration.
    pub font: Property<Font>,
    #[serde(default)]
    /// Font size, in pixels.
    pub font_size: Property<Size>,
    #[serde(default)]
    /// Text color. Native text patches reduce gradient fills to their first
    /// stop's color; use a solid fill for predictable text color.
    pub fill: Property<Paint>,
    #[serde(default)]
    /// Whether text should be underlined.
    pub underline: Property<bool>,
    #[serde(default)]
    /// Alignment for multiline text layout.
    pub align_multiline: Property<TextAlignHorizontal>,
    #[serde(default)]
    /// Vertical text alignment within its bounds.
    pub align_vertical: Property<TextAlignVertical>,
    #[serde(default)]
    /// Horizontal text alignment within its bounds.
    pub align_horizontal: Property<TextAlignHorizontal>,
}

impl pax_engine::api::Interpolatable for TextStyle {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        Self {
            font: Property::new(other.font.get()),
            font_size: Property::new(self.font_size.get().interpolate(&other.font_size.get(), t)),
            fill: Property::new(self.fill.get().interpolate(&other.fill.get(), t)),
            underline: Property::new(other.underline.get()),
            align_multiline: Property::new(other.align_multiline.get()),
            align_vertical: Property::new(other.align_vertical.get()),
            align_horizontal: Property::new(other.align_horizontal.get()),
        }
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font: Property::new(Font::default()),
            font_size: Property::new(Size::Pixels(Numeric::F64(20.0))),
            fill: Property::new(Paint::Solid(Color::BLACK)),
            underline: Property::new(false),
            align_multiline: Property::new(TextAlignHorizontal::Left),
            align_vertical: Property::new(TextAlignVertical::Top),
            align_horizontal: Property::new(TextAlignHorizontal::Left),
        }
    }
}

impl TextStyle {
    // Native patches must observe the graph's settled values, including
    // expression-bound fields inside a style record.
    pub(crate) fn message(&self, local: bool) -> TextStyleMessage {
        fn read<T: pax_engine::api::properties::SharedPropertyValue>(
            property: &Property<T>,
            local: bool,
        ) -> T {
            if local {
                property.local().get()
            } else {
                property.get()
            }
        }
        TextStyleMessage {
            font: Some(read(&self.font, local).clone().into()),
            font_size: Some(read(&self.font_size, local).expect_pixels().to_float()),
            fill: Some(Into::<ColorMessage>::into(
                &read(&self.fill, local).representative_color(),
            )),
            underline: Some(read(&self.underline, local).clone()),
            align_multiline: Some(Into::<TextAlignHorizontalMessage>::into(&read(
                &self.align_multiline,
                local,
            ))),
            align_vertical: Some(Into::<TextAlignVerticalMessage>::into(&read(
                &self.align_vertical,
                local,
            ))),
            align_horizontal: Some(Into::<TextAlignHorizontalMessage>::into(&read(
                &self.align_horizontal,
                local,
            ))),
        }
    }
}

impl<'a> Into<TextStyleMessage> for &'a TextStyle {
    fn into(self) -> TextStyleMessage {
        self.message(false)
    }
}

impl PartialEq for TextStyle {
    fn eq(&self, other: &Self) -> bool {
        let text_style_message: TextStyleMessage = other.into();
        self == &text_style_message
    }
}

impl PartialEq<TextStyleMessage> for TextStyle {
    fn eq(&self, other: &TextStyleMessage) -> bool {
        let font_equal = other
            .font
            .as_ref()
            .map_or(false, |font| self.font.get().eq(font));

        let font_size_approx_equal = other.font_size.map_or(false, |size| {
            (Numeric::F64(size).to_float()
                - self.font_size.get().expect_pixels().clone().to_float())
            .abs()
                < 1e-3
        });

        let fill_equal = other.fill.as_ref().map_or(false, |fill| {
            self.fill.get().representative_color().eq(fill)
        });

        let underline_equal = other
            .underline
            .map_or(false, |underline| underline == self.underline.get().clone());

        let align_multiline_equal = other
            .align_multiline
            .as_ref()
            .map_or(false, |align_multiline| {
                self.align_multiline.get().eq(align_multiline)
            });

        let align_vertical_equal = other
            .align_vertical
            .as_ref()
            .map_or(false, |align_vertical| {
                self.align_vertical.get().eq(align_vertical)
            });

        let align_horizontal_equal = other
            .align_horizontal
            .as_ref()
            .map_or(false, |align_horizontal| {
                self.align_horizontal.get().eq(align_horizontal)
            });

        font_equal
            && font_size_approx_equal
            && fill_equal
            && underline_equal
            && align_multiline_equal
            && align_vertical_equal
            && align_horizontal_equal
    }
}

/// Describes a font available to native text renderers.
///
/// Pax templates may use either the explicit [`Font::Web`] constructor or a
/// contextual shorthand. A string names a locally available family with an
/// empty source URL and normal style and weight:
///
/// ```pax
/// font: "Times New Roman"
/// ```
///
/// Named object fields describe font sources and optional modifiers. Omitted
/// fields retain their defaults. Supplying `family` without `url` selects a
/// locally available family, matching the string shorthand. `weight` accepts
/// either [`FontWeight`] or its CSS numeric equivalent from `100` through
/// `900`:
///
/// ```pax
/// font: {
///     family: "Inter"
///     url: "https://example.com/Inter-Italic.ttf"
///     style: FontStyle::Italic
///     weight: 700
/// }
/// ```
///
/// Nonempty URLs identify font files, except that Google Fonts URLs containing
/// `fonts.googleapis.com/css` receive special stylesheet handling. Relative
/// asset URLs work on web. Apple chassis register TTF/OTF files from the project's
/// `assets/` directory at startup; select their actual family name with an empty
/// URL, or use that family with the web asset URL in a shared declaration. The
/// native Web-font loader itself does not resolve relative URLs into bundle
/// resources. Browser-only aliases do not name the registered Apple family.
/// Windows and Linux resolve Pax's built-in default Roboto source to the
/// standard library's packaged normal/italic font files. Other remote sources
/// require explicit local assets on those hosts; the default does not enable
/// a general remote font loader.
///
/// Font shorthand has no positional ("magic index") fields: named keys are
/// canonical, and positional list/tuple forms are not accepted. Use the
/// explicit [`Font::Web`] constructor as verbose longhand when needed.
#[pax]
#[engine_import_path("pax_engine")]
#[custom(Default, CoercionRules)]
pub enum Font {
    /// Font described by family name, source URL, style, and weight.
    Web(String, String, FontStyle, FontWeight),
}

impl Default for Font {
    fn default() -> Self {
        Self::Web(
            "Roboto".to_string(),
            DEFAULT_FONT_URL.to_string(),
            FontStyle::Normal,
            FontWeight::Normal,
        )
    }
}

const DEFAULT_FONT_URL: &str = "https://fonts.googleapis.com/css2?family=Roboto:ital,wght@0,100;0,300;0,400;0,500;0,700;0,900;1,100;1,300;1,400;1,500;1,700;1,900&display=swap";

// Resolve at the native-message boundary, not while compiling the manifest:
// a cartridge baked on another OS must retain the same default-font meaning.
fn platform_font_url<'a>(family: &str, url: &'a str, style: &FontStyle) -> &'a str {
    if cfg!(any(
        target_os = "windows",
        target_os = "linux",
        target_os = "android"
    )) {
        native_default_font_url(family, url, style)
    } else {
        url
    }
}

fn native_default_font_url<'a>(family: &str, url: &'a str, style: &FontStyle) -> &'a str {
    if family == "Roboto" && url == DEFAULT_FONT_URL {
        match style {
            FontStyle::Normal => "pax-default-fonts/Roboto.ttf",
            FontStyle::Italic | FontStyle::Oblique => "pax-default-fonts/Roboto-Italic.ttf",
        }
    } else {
        url
    }
}

/// Describes available font styles.
#[pax]
#[engine_import_path("pax_engine")]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
    Oblique,
}

/// Describes available font weights.
#[pax]
#[engine_import_path("pax_engine")]
#[derive(PartialEq)]
pub enum FontWeight {
    Thin,
    ExtraLight,
    Light,
    #[default]
    Normal,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
}

impl CoercionRules for Font {
    fn try_coerce(value: PaxValue) -> Result<Self, String> {
        match value {
            PaxValue::String(family) => Ok(Self::Web(
                family,
                String::new(),
                FontStyle::Normal,
                FontWeight::Normal,
            )),
            PaxValue::Object(fields) => Self::from_object(fields),
            PaxValue::Enum(contents) => {
                let (enum_name, variant_name, values) = *contents;
                if enum_name != "Font" || variant_name != "Web" {
                    return Err(format!(
                        "expected Font::Web, got {enum_name}::{variant_name}"
                    ));
                }
                if values.len() != 4 {
                    return Err(format!(
                        "Font::Web expects 4 arguments, got {}",
                        values.len()
                    ));
                }

                let mut values = values.into_iter();
                Ok(Self::Web(
                    String::try_coerce(values.next().unwrap())?,
                    String::try_coerce(values.next().unwrap())?,
                    Self::coerce_style(values.next().unwrap())?,
                    Self::coerce_weight(values.next().unwrap())?,
                ))
            }
            PaxValue::Option(value) => match *value {
                Some(value) => Self::try_coerce(value),
                None => Err("None can't be coerced into Font".to_string()),
            },
            value => Err(format!("{value:?} can't be coerced into Font")),
        }
    }
}

impl Font {
    fn from_object(fields: Vec<(String, PaxValue)>) -> Result<Self, String> {
        let mut family = None;
        let mut url = None;
        let mut style = None;
        let mut weight = None;

        for (key, value) in fields {
            let field = match key.as_str() {
                "family" => &mut family,
                "url" => &mut url,
                "style" => &mut style,
                "weight" => &mut weight,
                _ => return Err(format!("unknown Font field `{key}`")),
            };
            if field.replace(value).is_some() {
                return Err(format!("duplicate Font field `{key}`"));
            }
        }

        let family_was_supplied = family.is_some();
        let Self::Web(default_family, default_url, default_style, default_weight) = Self::default();
        let family = family
            .map(String::try_coerce)
            .transpose()?
            .unwrap_or(default_family);
        let url = match url {
            Some(url) => String::try_coerce(url)?,
            None if family_was_supplied => String::new(),
            None => default_url,
        };
        let style = style
            .map(Self::coerce_style)
            .transpose()?
            .unwrap_or(default_style);
        let weight = weight
            .map(Self::coerce_weight)
            .transpose()?
            .unwrap_or(default_weight);

        Ok(Self::Web(family, url, style, weight))
    }

    fn enum_type_name(value: &PaxValue) -> Option<&str> {
        match value {
            PaxValue::Enum(contents) => Some(contents.0.as_str()),
            _ => None,
        }
    }

    fn coerce_style(value: PaxValue) -> Result<FontStyle, String> {
        if Self::enum_type_name(&value) != Some("FontStyle") {
            return Err("expected FontStyle".to_string());
        }
        FontStyle::try_coerce(value)
    }

    fn coerce_weight(value: PaxValue) -> Result<FontWeight, String> {
        if let PaxValue::Numeric(value) = value {
            let value = value.to_float();
            if !value.is_finite() || value.fract() != 0.0 {
                return Err("font weight must be a whole CSS weight from 100 to 900".to_string());
            }
            return match value as i32 {
                100 => Ok(FontWeight::Thin),
                200 => Ok(FontWeight::ExtraLight),
                300 => Ok(FontWeight::Light),
                400 => Ok(FontWeight::Normal),
                500 => Ok(FontWeight::Medium),
                600 => Ok(FontWeight::SemiBold),
                700 => Ok(FontWeight::Bold),
                800 => Ok(FontWeight::ExtraBold),
                900 => Ok(FontWeight::Black),
                _ => Err(
                    "font weight must be one of 100, 200, 300, 400, 500, 600, 700, 800, or 900"
                        .to_string(),
                ),
            };
        }
        if Self::enum_type_name(&value) != Some("FontWeight") {
            return Err("expected FontWeight or a numeric CSS weight".to_string());
        }
        FontWeight::try_coerce(value)
    }
}

#[cfg(test)]
mod font_coercion_tests {
    #[test]
    fn text_style_interpolates_continuous_fields_and_switches_discrete_fields() {
        use super::*;
        use pax_engine::api::Interpolatable;
        let from = TextStyle::default();
        from.font_size.set(Size::Pixels(10.into()));
        from.fill.set(Paint::Solid(Color::BLACK));
        let to = TextStyle::default();
        to.font_size.set(Size::Pixels(30.into()));
        to.fill.set(Paint::Solid(Color::WHITE));
        to.underline.set(true);
        to.align_horizontal.set(TextAlignHorizontal::Right);
        let mid = from.interpolate(&to, 0.5);
        assert_eq!(mid.font_size.get().expect_pixels().to_float(), 20.0);
        assert_eq!(
            mid.fill.get(),
            Paint::Solid(Color::BLACK.interpolate(&Color::WHITE, 0.5))
        );
        assert!(mid.underline.get());
        assert!(matches!(
            mid.align_horizontal.get(),
            TextAlignHorizontal::Right
        ));
        to.font_size.set(Size::Pixels(100.into()));
        assert_eq!(mid.font_size.get().expect_pixels().to_float(), 20.0);
    }
    use super::*;

    fn string(value: &str) -> PaxValue {
        PaxValue::String(value.to_string())
    }

    fn enum_value(type_name: &str, variant_name: &str) -> PaxValue {
        PaxValue::Enum(Box::new((
            type_name.to_string(),
            variant_name.to_string(),
            vec![],
        )))
    }

    fn object(fields: Vec<(&str, PaxValue)>) -> PaxValue {
        PaxValue::Object(
            fields
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect(),
        )
    }

    fn numeric(value: i32) -> PaxValue {
        PaxValue::Numeric(value.into())
    }

    fn assert_web(
        font: Font,
        expected_family: &str,
        expected_url: &str,
        expected_style: FontStyle,
        expected_weight: FontWeight,
    ) {
        let Font::Web(family, url, style, weight) = font;
        assert_eq!(family, expected_family);
        assert_eq!(url, expected_url);
        assert!(matches!(
            (style, expected_style),
            (FontStyle::Normal, FontStyle::Normal)
                | (FontStyle::Italic, FontStyle::Italic)
                | (FontStyle::Oblique, FontStyle::Oblique)
        ));
        assert_eq!(weight, expected_weight);
    }

    #[test]
    fn string_coerces_to_local_family_defaults() {
        assert_web(
            Font::try_coerce(string("Times New Roman")).unwrap(),
            "Times New Roman",
            "",
            FontStyle::Normal,
            FontWeight::Normal,
        );
    }

    #[test]
    fn native_default_font_resolves_both_faces_without_rewriting_custom_sources() {
        for (style, expected) in [
            (FontStyle::Normal, "pax-default-fonts/Roboto.ttf"),
            (FontStyle::Italic, "pax-default-fonts/Roboto-Italic.ttf"),
            (FontStyle::Oblique, "pax-default-fonts/Roboto-Italic.ttf"),
        ] {
            assert_eq!(
                native_default_font_url("Roboto", DEFAULT_FONT_URL, &style),
                expected
            );
            assert_eq!(
                native_default_font_url("Other", DEFAULT_FONT_URL, &style),
                DEFAULT_FONT_URL
            );
            assert_eq!(
                native_default_font_url("Roboto", "fonts/custom.ttf", &style),
                "fonts/custom.ttf"
            );
            assert_eq!(native_default_font_url("Roboto", "", &style), "");
        }
        for style in [FontStyle::Normal, FontStyle::Italic, FontStyle::Oblique] {
            let font = Font::Web(
                "Roboto".into(),
                DEFAULT_FONT_URL.into(),
                style,
                FontWeight::Bold,
            );
            let patch = FontPatch::from(font.clone());
            assert!(
                font == patch,
                "unchanged native font must not be patched again every frame"
            );
            let FontPatch::Web(patch) = patch else {
                panic!("expected web font metadata")
            };
            assert!(patch.weight == Some(FontWeightMessage::Bold));
        }
    }

    #[test]
    fn object_shorthand_supports_named_fields_and_defaults() {
        let family = "Inter";
        let url = "https://example.com/inter.css";

        assert_web(
            Font::try_coerce(object(vec![
                ("family", string(family)),
                ("url", string(url)),
            ]))
            .unwrap(),
            family,
            url,
            FontStyle::Normal,
            FontWeight::Normal,
        );
        assert_web(
            Font::try_coerce(object(vec![
                ("family", string(family)),
                ("weight", numeric(700)),
            ]))
            .unwrap(),
            family,
            "",
            FontStyle::Normal,
            FontWeight::Bold,
        );
        assert_web(
            Font::try_coerce(object(vec![
                ("style", enum_value("FontStyle", "Italic")),
                ("weight", enum_value("FontWeight", "Medium")),
            ]))
            .unwrap(),
            "Roboto",
            "https://fonts.googleapis.com/css2?family=Roboto:ital,wght@0,100;0,300;0,400;0,500;0,700;0,900;1,100;1,300;1,400;1,500;1,700;1,900&display=swap",
            FontStyle::Italic,
            FontWeight::Medium,
        );
    }

    #[test]
    fn explicit_web_constructor_remains_supported() {
        let value = PaxValue::Enum(Box::new((
            "Font".to_string(),
            "Web".to_string(),
            vec![
                string("Inter"),
                string("https://example.com/inter.css"),
                enum_value("FontStyle", "Normal"),
                enum_value("FontWeight", "SemiBold"),
            ],
        )));

        assert_web(
            Font::try_coerce(value).unwrap(),
            "Inter",
            "https://example.com/inter.css",
            FontStyle::Normal,
            FontWeight::SemiBold,
        );
    }

    #[test]
    fn numeric_css_weights_cover_every_runtime_variant() {
        let cases = [
            (100, FontWeight::Thin),
            (200, FontWeight::ExtraLight),
            (300, FontWeight::Light),
            (400, FontWeight::Normal),
            (500, FontWeight::Medium),
            (600, FontWeight::SemiBold),
            (700, FontWeight::Bold),
            (800, FontWeight::ExtraBold),
            (900, FontWeight::Black),
        ];

        for (weight, expected) in cases {
            assert_web(
                Font::try_coerce(object(vec![("weight", numeric(weight))])).unwrap(),
                "Roboto",
                "https://fonts.googleapis.com/css2?family=Roboto:ital,wght@0,100;0,300;0,400;0,500;0,700;0,900;1,100;1,300;1,400;1,500;1,700;1,900&display=swap",
                FontStyle::Normal,
                expected,
            );
        }
    }

    #[test]
    fn list_shorthand_and_invalid_object_fields_are_rejected() {
        assert!(Font::try_coerce(PaxValue::Vec(vec![
            string("Inter"),
            string("https://example.com/inter.css"),
        ]))
        .is_err());
        assert!(Font::try_coerce(object(vec![("weight", numeric(450))])).is_err());
        assert!(Font::try_coerce(object(vec![("unknown", string("value"))])).is_err());
        assert!(Font::try_coerce(object(vec![
            ("family", string("Inter")),
            ("family", string("Roboto")),
        ]))
        .is_err());
    }
}

/// Describes available horizontal text alignments.
#[pax]
#[engine_import_path("pax_engine")]
pub enum TextAlignHorizontal {
    #[default]
    Left,
    Center,
    Right,
}

/// Describes available vertical text alignments.
#[pax]
#[engine_import_path("pax_engine")]
pub enum TextAlignVertical {
    #[default]
    Top,
    Center,
    Bottom,
}

impl Into<TextAlignHorizontalMessage> for &TextAlignHorizontal {
    fn into(self) -> TextAlignHorizontalMessage {
        match self {
            TextAlignHorizontal::Center => TextAlignHorizontalMessage::Center,
            TextAlignHorizontal::Left => TextAlignHorizontalMessage::Left,
            TextAlignHorizontal::Right => TextAlignHorizontalMessage::Right,
        }
    }
}

impl PartialEq<TextAlignHorizontalMessage> for TextAlignHorizontal {
    fn eq(&self, other: &TextAlignHorizontalMessage) -> bool {
        match (self, other) {
            (TextAlignHorizontal::Center, TextAlignHorizontalMessage::Center) => true,
            (TextAlignHorizontal::Left, TextAlignHorizontalMessage::Left) => true,
            (TextAlignHorizontal::Right, TextAlignHorizontalMessage::Right) => true,
            _ => false,
        }
    }
}

// Converts an optional horizontal alignment to its native patch representation.
pub fn opt_align_to_message(
    opt_alignment: &Option<TextAlignHorizontal>,
) -> Option<TextAlignHorizontalMessage> {
    opt_alignment.as_ref().map(|alignment| match alignment {
        TextAlignHorizontal::Center => TextAlignHorizontalMessage::Center,
        TextAlignHorizontal::Left => TextAlignHorizontalMessage::Left,
        TextAlignHorizontal::Right => TextAlignHorizontalMessage::Right,
    })
}

// Compares an optional Pax value against an optional native patch value.
pub fn opt_value_eq_opt_msg<T, U>(opt_value: &Option<T>, opt_value_msg: &Option<U>) -> bool
where
    T: PartialEq<U>,
{
    match (opt_value, opt_value_msg) {
        (Some(value), Some(value_msg)) => value.eq(value_msg),
        (None, None) => true,
        _ => false,
    }
}

impl Into<TextAlignVerticalMessage> for &TextAlignVertical {
    fn into(self) -> TextAlignVerticalMessage {
        match self {
            TextAlignVertical::Top => TextAlignVerticalMessage::Top,
            TextAlignVertical::Center => TextAlignVerticalMessage::Center,
            TextAlignVertical::Bottom => TextAlignVerticalMessage::Bottom,
        }
    }
}

impl PartialEq<TextAlignVerticalMessage> for TextAlignVertical {
    fn eq(&self, other: &TextAlignVerticalMessage) -> bool {
        match (self, other) {
            (TextAlignVertical::Top, TextAlignVerticalMessage::Top) => true,
            (TextAlignVertical::Center, TextAlignVerticalMessage::Center) => true,
            (TextAlignVertical::Bottom, TextAlignVerticalMessage::Bottom) => true,
            _ => false,
        }
    }
}

impl From<Font> for FontPatch {
    fn from(font: Font) -> Self {
        match font {
            Font::Web(family, url, style, weight) => {
                let url = platform_font_url(&family, &url, &style).to_owned();
                FontPatch::Web(WebFontMessage {
                    family: Some(family),
                    url: Some(url),
                    style: Some(style.into()),
                    weight: Some(weight.into()),
                })
            }
        }
    }
}

impl PartialEq<FontStyleMessage> for FontStyle {
    fn eq(&self, other: &FontStyleMessage) -> bool {
        match (self, other) {
            (FontStyle::Normal, FontStyleMessage::Normal) => true,
            (FontStyle::Italic, FontStyleMessage::Italic) => true,
            (FontStyle::Oblique, FontStyleMessage::Oblique) => true,
            _ => false,
        }
    }
}

impl PartialEq<FontPatch> for Font {
    fn eq(&self, other: &FontPatch) -> bool {
        match (self, other) {
            (Font::Web(family, url, style, weight), FontPatch::Web(web_font_patch)) => {
                web_font_patch
                    .family
                    .as_ref()
                    .map_or(false, |f| f == family)
                    && web_font_patch
                        .url
                        .as_ref()
                        .map_or(false, |u| u == platform_font_url(family, url, style))
                    && web_font_patch.style.as_ref().map_or(false, |s| style.eq(s))
                    && web_font_patch
                        .weight
                        .as_ref()
                        .map_or(false, |w| FontWeightMessage::from(weight.clone()).eq(w))
            }
            _ => false,
        }
    }
}

impl From<FontStyleMessage> for FontStyle {
    fn from(style_msg: FontStyleMessage) -> Self {
        match style_msg {
            FontStyleMessage::Normal => FontStyle::Normal,
            FontStyleMessage::Italic => FontStyle::Italic,
            FontStyleMessage::Oblique => FontStyle::Oblique,
        }
    }
}

impl From<FontStyle> for FontStyleMessage {
    fn from(style: FontStyle) -> Self {
        match style {
            FontStyle::Normal => FontStyleMessage::Normal,
            FontStyle::Italic => FontStyleMessage::Italic,
            FontStyle::Oblique => FontStyleMessage::Oblique,
        }
    }
}

impl From<FontWeightMessage> for FontWeight {
    fn from(weight_msg: FontWeightMessage) -> Self {
        match weight_msg {
            FontWeightMessage::Thin => FontWeight::Thin,
            FontWeightMessage::ExtraLight => FontWeight::ExtraLight,
            FontWeightMessage::Light => FontWeight::Light,
            FontWeightMessage::Normal => FontWeight::Normal,
            FontWeightMessage::Medium => FontWeight::Medium,
            FontWeightMessage::SemiBold => FontWeight::SemiBold,
            FontWeightMessage::Bold => FontWeight::Bold,
            FontWeightMessage::ExtraBold => FontWeight::ExtraBold,
            FontWeightMessage::Black => FontWeight::Black,
        }
    }
}

impl From<FontWeight> for FontWeightMessage {
    fn from(weight: FontWeight) -> Self {
        match weight {
            FontWeight::Thin => FontWeightMessage::Thin,
            FontWeight::ExtraLight => FontWeightMessage::ExtraLight,
            FontWeight::Light => FontWeightMessage::Light,
            FontWeight::Normal => FontWeightMessage::Normal,
            FontWeight::Medium => FontWeightMessage::Medium,
            FontWeight::SemiBold => FontWeightMessage::SemiBold,
            FontWeight::Bold => FontWeightMessage::Bold,
            FontWeight::ExtraBold => FontWeightMessage::ExtraBold,
            FontWeight::Black => FontWeightMessage::Black,
        }
    }
}

impl FontWeight {
    /// Returns the next heavier named font weight.
    pub fn increase(weight: FontWeight) -> FontWeight {
        match weight {
            FontWeight::Thin => FontWeight::ExtraLight,
            FontWeight::ExtraLight => FontWeight::Light,
            FontWeight::Light => FontWeight::Normal,
            FontWeight::Normal => FontWeight::Medium,
            FontWeight::Medium => FontWeight::SemiBold,
            FontWeight::SemiBold => FontWeight::Bold,
            FontWeight::Bold => FontWeight::ExtraBold,
            FontWeight::ExtraBold => FontWeight::Black,
            FontWeight::Black => FontWeight::Black,
        }
    }
    /// Returns the next lighter named font weight.
    pub fn decrease(weight: FontWeight) -> FontWeight {
        match weight {
            FontWeight::Thin => FontWeight::Thin,
            FontWeight::ExtraLight => FontWeight::Thin,
            FontWeight::Light => FontWeight::ExtraLight,
            FontWeight::Normal => FontWeight::Light,
            FontWeight::Medium => FontWeight::Normal,
            FontWeight::SemiBold => FontWeight::Medium,
            FontWeight::Bold => FontWeight::SemiBold,
            FontWeight::ExtraBold => FontWeight::Bold,
            FontWeight::Black => FontWeight::ExtraBold,
        }
    }
}

#[cfg(test)]
mod isolation_tests;

#[cfg(test)]
mod sibling_measurement_tests {
    use super::*;

    fn key(content: &str) -> TextMeasurementKey {
        text_measurement_key(
            content.into(),
            false,
            true,
            false,
            false,
            true,
            TextStyleMessage::default(),
            TextStyleMessage::default(),
            100.0,
            -1.0,
        )
        .unwrap()
    }

    #[test]
    fn expanded_siblings_do_not_supersede_each_others_measurements() {
        let mut state = TextMeasurements::default();
        let a = state.request(1, key("first"), false).unwrap();
        let b = state.request(2, key("second"), false).unwrap();
        assert_eq!(state.request(1, key("first"), false), None);
        assert_eq!(state.request(2, key("second"), false), None);
        assert!(!state.accept(1, b));
        assert!(state.accept(2, b));
        assert!(state.accept(1, a));
        assert_eq!(state.request(1, key("first"), true), None);
        assert_eq!(state.request(2, key("second"), true), None);
    }

    #[test]
    fn stale_responses_are_rejected_after_changes_and_remounts() {
        let mut state = TextMeasurements::default();
        let original = state.request(1, key("old"), false).unwrap();
        let updated = state.request(1, key("new"), false).unwrap();
        assert!(!state.accept(1, original));
        assert!(state.accept(1, updated));
        assert!(!state.accept(1, updated));
        state.nodes.remove(&1);
        let remounted = state.request(1, key("new"), false).unwrap();
        assert!(!state.accept(1, updated));
        assert!(state.accept(1, remounted));
    }
}
