use pax_runtime::{
    BaseInstance, ExpandedNode, ExpandedNodeIdentifier, InstanceFlags, InstanceNode,
    InstantiationArgs, RuntimeContext,
};

use pax_engine::api::{Color, Layer, Numeric, Property, RenderContext, Size};
use pax_engine::*;

use pax_message::{
    AnyCreatePatch, ColorMessage, FontPatch, FontStyleMessage, FontWeightMessage, LocalFontMessage,
    SystemFontMessage, TextAlignHorizontalMessage, TextAlignVerticalMessage, TextPatch,
    TextStyleMessage, WebFontMessage,
};

use pax_runtime::api::{borrow, borrow_mut, use_RefCell};

use pax_runtime::api as pax_runtime_api;
use_RefCell!();
use std::collections::HashMap;
use std::rc::Rc;
#[cfg(feature = "designtime")]
use {
    kurbo::{RoundedRect, Shape},
    pax_runtime::DEBUG_TEXT_GREEN_BACKGROUND,
    piet::Color,
};

use crate::common::patch_if_needed;

/// Renders text in a platform-native way
#[pax]
#[primitive("pax_std::core::text::TextInstance")]
pub struct Text {
    pub editable: Property<bool>,
    pub text: Property<String>,
    pub style: Property<TextStyle>,
    pub _style_link: Property<TextStyle>,
}

pub struct TextInstance {
    base: BaseInstance,
    native_message_props: RefCell<HashMap<ExpandedNodeIdentifier, Property<()>>>,
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
                },
            ),
            native_message_props: Default::default(),
        })
    }

    fn update(self: Rc<Self>, expanded_node: &Rc<ExpandedNode>, _context: &Rc<RuntimeContext>) {
        //trigger computation of property that computes + sends native message update
        borrow!(self.native_message_props)
            .get(&expanded_node.id)
            .unwrap()
            .get();
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
            let computed_props = borrow!(expanded_node.layout_properties);
            let tab = &computed_props.as_ref().unwrap().computed_tab;
            let layer_id = format!("{}", expanded_node.occlusion.get().occlusion_layer_id);
            let width: f64 = tab.bounds.0;
            let height: f64 = tab.bounds.1;
            let rect = RoundedRect::new(0.0, 0.0, width, height, 0.0);
            let bez_path = rect.to_path(0.1);
            let transformed_bez_path = Into::<kurbo::Affine>::into(tab.transform) * bez_path;
            rc.fill(
                &layer_id,
                transformed_bez_path,
                &piet::PaintBrush::Color(Color::rgba8(0, 255, 0, 100)),
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
            occlusion_layer_id: 0,
        }));

        // send update message when relevant properties change
        let weak_self_ref = Rc::downgrade(&expanded_node);
        let context = Rc::clone(context);
        let last_patch = Rc::new(RefCell::new(TextPatch {
            id,
            ..Default::default()
        }));

        let deps: Vec<_> = borrow!(expanded_node.properties_scope)
            .values()
            .cloned()
            .chain([expanded_node.transform_and_bounds.untyped()])
            .collect();

        borrow_mut!(self.native_message_props).insert(
            expanded_node.id,
            Property::computed(
                move || {
                    let Some(expanded_node) = weak_self_ref.upgrade() else {
                        unreachable!()
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

                        let updates = [
                            // Content
                            patch_if_needed(
                                &mut old_state.content,
                                &mut patch.content,
                                properties.text.get(),
                            ),
                            // Styles
                            patch_if_needed(
                                &mut old_state.style,
                                &mut patch.style,
                                (&properties.style.get()).into(),
                            ),
                            patch_if_needed(
                                &mut old_state.style_link,
                                &mut patch.style_link,
                                (&properties._style_link.get()).into(),
                            ),
                            patch_if_needed(
                                &mut old_state.editable,
                                &mut patch.editable,
                                properties.editable.get(),
                            ),
                            // Transform and bounds
                            patch_if_needed(&mut old_state.size_x, &mut patch.size_x, width),
                            patch_if_needed(&mut old_state.size_y, &mut patch.size_y, height),
                            patch_if_needed(
                                &mut old_state.transform,
                                &mut patch.transform,
                                computed_tab.transform.coeffs().to_vec(),
                            ),
                        ];
                        if updates.into_iter().any(|v| v == true) {
                            context.enqueue_native_message(pax_message::NativeMessage::TextUpdate(
                                patch,
                            ));
                        }
                    });
                    ()
                },
                &deps,
            ),
        );
    }

    fn handle_unmount(&self, expanded_node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        let id = expanded_node.id.to_u32();
        context.enqueue_native_message(pax_message::NativeMessage::TextDelete(id));
        // Reset so that native_message sending updates while unmounted
        borrow_mut!(self.native_message_props).remove(&expanded_node.id);
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

    fn handle_native_interrupt(
        &self,
        expanded_node: &Rc<ExpandedNode>,
        interrupt: &pax_message::NativeInterrupt,
    ) {
        if let pax_message::NativeInterrupt::TextInput(args) = interrupt {
            expanded_node.with_properties_unwrapped(|properties: &mut Text| {
                properties.text.set(args.text.clone())
            });
        } else {
            log::warn!("text element was handed interrupt it doesn't use");
        }
    }
}

#[pax]
#[custom(Default)]
pub struct TextStyle {
    #[serde(default)]
    pub font: Property<Font>,
    #[serde(default)]
    pub font_size: Property<Size>,
    #[serde(default)]
    pub fill: Property<Color>,
    #[serde(default)]
    pub underline: Property<bool>,
    #[serde(default)]
    pub align_multiline: Property<TextAlignHorizontal>,
    #[serde(default)]
    pub align_vertical: Property<TextAlignVertical>,
    #[serde(default)]
    pub align_horizontal: Property<TextAlignHorizontal>,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font: Property::new(Font::default()),
            font_size: Property::new(Size::Pixels(Numeric::F64(20.0))),
            fill: Property::new(Color::BLACK),
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
            fill: Some(Into::<ColorMessage>::into(&self.fill.get())),
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

        let fill_equal = other
            .fill
            .as_ref()
            .map_or(false, |fill| self.fill.get().eq(fill));

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

#[pax]
#[custom(Default)]
pub enum Font {
    Web(String, String, FontStyle, FontWeight),
}

impl Default for Font {
    fn default() -> Self {
        Self::Web("Roboto".to_string(),
    "https://fonts.googleapis.com/css2?family=Roboto:ital,wght@0,100;0,300;0,400;0,500;0,700;0,900;1,100;1,300;1,400;1,500;1,700;1,900&display=swap".to_string()
                  , FontStyle::Normal, FontWeight::Normal)
    }
}

#[pax]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
    Oblique,
}

#[pax]
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

#[pax]
pub enum TextAlignHorizontal {
    #[default]
    Left,
    Center,
    Right,
}

#[pax]
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

pub fn opt_align_to_message(
    opt_alignment: &Option<TextAlignHorizontal>,
) -> Option<TextAlignHorizontalMessage> {
    opt_alignment.as_ref().map(|alignment| match alignment {
        TextAlignHorizontal::Center => TextAlignHorizontalMessage::Center,
        TextAlignHorizontal::Left => TextAlignHorizontalMessage::Left,
        TextAlignHorizontal::Right => TextAlignHorizontalMessage::Right,
    })
}

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

#[helpers("FontWeight")]
impl FontWeight {
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
