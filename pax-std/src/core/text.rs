use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};

use pax_engine::api::{Color, Fill, Layer, Numeric, Property, RenderContext, Size};
use pax_engine::*;

use pax_message::{
    AnyCreatePatch, ColorMessage, FontPatch, FontStyleMessage, FontWeightMessage,
    TextAlignHorizontalMessage, TextAlignVerticalMessage, TextPatch, TextStyleMessage,
    WebFontMessage,
};

use pax_runtime::api::{borrow, borrow_mut, use_RefCell};

use_RefCell!();
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
pub struct Text {
    /// Whether the text can be edited by the user.
    pub editable: Property<bool>,
    /// Requests selectable text. On the current iOS/iPadOS path, non-editable
    /// text uses the interactive selection view only when `clip` is enabled.
    pub selectable: Property<bool>,
    /// Whether text overflow is clipped to the node bounds.
    pub clip: Property<bool>,
    /// Text content to display.
    pub text: Property<String>,
    /// Text styling.
    pub style: Property<TextStyle>,
    // Secondary style link used by native text patch plumbing.
    pub _style_link: Property<TextStyle>,
    /// Whether `text` should be interpreted as Markdown.
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
    measurement_state: RefCell<TextMeasurementState>,
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
}

impl TextInstance {
    fn request_measurement_generation(
        &self,
        key: TextMeasurementKey,
        has_current_measurement: bool,
    ) -> Option<u64> {
        let mut state = self.measurement_state.borrow_mut();
        let key_changed = state.requested_key.as_ref() != Some(&key);
        if key_changed || (!has_current_measurement && state.pending_generation.is_none()) {
            state.next_generation = state.next_generation.wrapping_add(1).max(1);
            let generation = state.next_generation;
            state.requested_key = Some(key);
            state.pending_generation = Some(generation);
            Some(generation)
        } else {
            None
        }
    }

    fn clear_measurement_request(&self, expanded_node: &Rc<ExpandedNode>) {
        let mut state = self.measurement_state.borrow_mut();
        state.requested_key = None;
        state.pending_generation = None;
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
        let mut state = self.measurement_state.borrow_mut();
        if state.pending_generation != Some(generation) {
            return;
        }
        state.pending_generation = None;
        drop(state);
        expanded_node.set_measured_size(width, height);
    }
}

impl InstanceNode for TextInstance {
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
            measurement_state: RefCell::new(TextMeasurementState::default()),
        })
    }

    fn render(
        &self,
        _expanded_node: &ExpandedNode,
        _context: &Rc<RuntimeContext>,
        _rc: &mut dyn RenderContext,
    ) {
        //no-op -- only native rendering for Text (unless/until we support rasterizing text, which Piet should be able to handle!)

        #[cfg(feature = "designtime")]
        if DEBUG_TEXT_GREEN_BACKGROUND {
            let tab = _expanded_node.transform_and_bounds.get();
            let width: f64 = tab.bounds.0;
            let height: f64 = tab.bounds.1;
            let rect = RoundedRect::new(0.0, 0.0, width, height, 0.0);
            let bez_path = rect.to_path(0.1);
            let transformed_bez_path = Into::<kurbo::Affine>::into(tab.transform) * bez_path;
            _rc.fill(
                _expanded_node.occlusion.get().render_layer_id,
                transformed_bez_path,
                &Fill::Solid(Color::rgba(0.into(), 255.into(), 0.into(), 100.into())),
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
        let context = Rc::clone(context);
        let last_patch = Rc::new(RefCell::new(TextPatch {
            id,
            ..Default::default()
        }));

        let deps: Vec<_> = borrow!(expanded_node.properties_scope)
            .values()
            .cloned()
            .map(|v| v.get_untyped_property().clone())
            .chain([
                expanded_node.transform_and_bounds.untyped(),
                expanded_node.computed_opacity.untyped(),
                expanded_node.occlusion.untyped(),
            ])
            .collect();

        expanded_node
            .changed_listener
            .replace_with(Property::computed(
                move || {
                    let Some(expanded_node) = weak_self_ref.upgrade() else {
                        return;
                    };
                    let mut old_state = borrow_mut!(last_patch);

                    let mut patch = TextPatch {
                        id,
                        ..Default::default()
                    };
                    expanded_node.with_properties_unwrapped(|properties: &mut Text| {
                        let computed_tab = expanded_node.transform_and_bounds.get();
                        let (width, height) = computed_tab.bounds;
                        let cp = expanded_node.get_common_properties();
                        let cp = borrow!(cp);
                        // send width/height only if common props exist, otherwise we are in "listening mode"
                        // trying to infer width and height from the engine. To signal this we
                        // send width/height = -1.0, telling chassis that "you tell me!".
                        let (width, height) = (
                            cp.width.get().is_some().then_some(width).unwrap_or(-1.0),
                            cp.height.get().is_some().then_some(height).unwrap_or(-1.0),
                        );
                        let content = properties.text.get();
                        let markdown = properties.markdown.get();
                        let wrap = properties.wrap.get();
                        let style: TextStyleMessage = (&properties.style.get()).into();
                        let style_link: TextStyleMessage = (&properties._style_link.get()).into();
                        let editable = properties.editable.get();
                        let selectable = properties.selectable.get();
                        let clip = properties.clip.get();

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
                            patch_if_needed(
                                &mut old_state.parent_frame,
                                &mut patch.parent_frame,
                                expanded_node.parent_frame.get().map(|v| v.to_u32()),
                            ),
                            patch_if_needed(
                                &mut old_state.z_index,
                                &mut patch.z_index,
                                expanded_node.occlusion.get().z_index,
                            ),
                            patch_if_needed(
                                &mut old_state.transform,
                                &mut patch.transform,
                                computed_tab.transform.coeffs().to_vec(),
                            ),
                            patch_if_needed(
                                &mut old_state.opacity,
                                &mut patch.opacity,
                                native_surface_opacity(&expanded_node, &context),
                            ),
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
                                measurement_key,
                                expanded_node.measured_size.get().is_some(),
                            );
                        } else {
                            instance.clear_measurement_request(&expanded_node);
                        }

                        if updates.into_iter().any(|v| v == true)
                            || patch.measure_generation.is_some()
                        {
                            // HACK: markdown flag and content is needed at the same time, so if we have one,
                            // include the other:
                            if patch.markdown.is_some() && patch.content.is_none() {
                                patch.content = Some(content);
                            }
                            if patch.content.is_some() && patch.markdown.is_none() {
                                patch.markdown = Some(markdown);
                            }
                            context.enqueue_native_message(pax_message::NativeMessage::TextUpdate(
                                patch,
                            ));
                        }
                    });
                    ()
                },
                &deps,
            ));
    }

    fn handle_unmount(&self, expanded_node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        let id = expanded_node.id.to_u32();
        expanded_node
            .changed_listener
            .replace_with(Property::default());
        context.enqueue_native_message(pax_message::NativeMessage::TextDelete(id));
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        match expanded_node {
            Some(expanded_node) => expanded_node.with_properties_unwrapped(|r: &mut Text| {
                f.debug_struct("Text").field("text", &r.text.get()).finish()
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
        } else {
            log::warn!("text element was handed interrupt it doesn't use");
        }
    }
}

fn explicit_constraint_bits(value: f64) -> Option<u64> {
    (value >= 0.0).then_some(value.to_bits())
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
#[pax]
#[engine_import_path("pax_engine")]
#[custom(Default)]
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
    pub fill: Property<Fill>,
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

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font: Property::new(Font::default()),
            font_size: Property::new(Size::Pixels(Numeric::F64(20.0))),
            fill: Property::new(Fill::Solid(Color::BLACK)),
            underline: Property::new(false),
            align_multiline: Property::new(TextAlignHorizontal::Left),
            align_vertical: Property::new(TextAlignVertical::Top),
            align_horizontal: Property::new(TextAlignHorizontal::Left),
        }
    }
}

impl<'a> Into<TextStyleMessage> for &'a TextStyle {
    fn into(self) -> TextStyleMessage {
        TextStyleMessage {
            font: Some(self.font.get().clone().into()),
            font_size: Some(self.font_size.get().expect_pixels().to_float()),
            fill: Some(Into::<ColorMessage>::into(&match self.fill.get() {
                Fill::Solid(color) => color,
                Fill::LinearGradient(lgrad) => lgrad
                    .stops
                    .first()
                    .map(|f| f.color.clone())
                    .unwrap_or_default(),
                Fill::RadialGradient(rgrad) => rgrad
                    .stops
                    .first()
                    .map(|f| f.color.clone())
                    .unwrap_or_default(),
            })),
            underline: Some(self.underline.get().clone()),
            align_multiline: Some(Into::<TextAlignHorizontalMessage>::into(
                &self.align_multiline.get(),
            )),
            align_vertical: Some(Into::<TextAlignVerticalMessage>::into(
                &self.align_vertical.get(),
            )),
            align_horizontal: Some(Into::<TextAlignHorizontalMessage>::into(
                &self.align_horizontal.get(),
            )),
        }
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
            match self.fill.get() {
                Fill::Solid(color) => color,
                Fill::LinearGradient(lgrad) => lgrad
                    .stops
                    .first()
                    .map(|f| f.color.clone())
                    .unwrap_or_default(),
                Fill::RadialGradient(rgrad) => rgrad
                    .stops
                    .first()
                    .map(|f| f.color.clone())
                    .unwrap_or_default(),
            }
            .eq(fill)
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
/// asset URLs work on web; the native Web-font loader does not resolve them
/// into application bundle resources. Native targets also require the actual
/// font family name, rather than a browser-only alias.
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
        Self::Web("Roboto".to_string(),
    "https://fonts.googleapis.com/css2?family=Roboto:ital,wght@0,100;0,300;0,400;0,500;0,700;0,900;1,100;1,300;1,400;1,500;1,700;1,900&display=swap".to_string()
                  , FontStyle::Normal, FontWeight::Normal)
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
            Font::Web(family, url, style, weight) => FontPatch::Web(WebFontMessage {
                family: Some(family),
                url: Some(url),
                style: Some(style.into()),
                weight: Some(weight.into()),
            }),
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
                    && web_font_patch.url.as_ref().map_or(false, |u| u == url)
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
