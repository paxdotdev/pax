pub mod reflection;

//FUTURE: feature-flag, only for Web builds
#[allow(unused_imports)]
use wasm_bindgen::prelude::*;

pub use serde;
use serde::{Deserialize, Serialize};

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Serialize)]
pub enum NativeMessage {
    TextCreate(AnyCreatePatch),
    TextUpdate(TextPatch),
    TextDelete(Vec<u32>), //node instance ID, "id_chain"
    FrameCreate(AnyCreatePatch),
    FrameUpdate(FramePatch),
    FrameDelete(Vec<u32>),
    CheckboxCreate(AnyCreatePatch),
    CheckboxUpdate(CheckboxPatch),
    CheckboxDelete(Vec<u32>),
    TextboxCreate(AnyCreatePatch),
    TextboxUpdate(TextboxPatch),
    TextboxDelete(Vec<u32>),
    ButtonCreate(AnyCreatePatch),
    ButtonUpdate(ButtonPatch),
    ButtonDelete(Vec<u32>),
    ScrollerCreate(AnyCreatePatch),
    ScrollerUpdate(ScrollerPatch),
    ScrollerDelete(Vec<u32>),
    ImageLoad(ImagePatch),
    LayerAdd(LayerAddPatch), //FUTURE: native form controls
    OcclusionUpdate(OcclusionPatch),
}

#[derive(Deserialize)]
#[repr(C)]
pub enum NativeInterrupt {
    Clap(ClapInterruptArgs),
    Scroll(ScrollInterruptArgs),
    TouchStart(TouchStartInterruptArgs),
    TouchMove(TouchMoveInterruptArgs),
    TouchEnd(TouchEndInterruptArgs),
    KeyDown(KeyDownInterruptArgs),
    KeyUp(KeyUpInterruptArgs),
    KeyPress(KeyPressInterruptArgs),
    Click(ClickInterruptArgs),
    DoubleClick(DoubleClickInterruptArgs),
    MouseMove(MouseMoveInterruptArgs),
    Wheel(WheelInterruptArgs),
    MouseDown(MouseDownInterruptArgs),
    MouseUp(MouseUpInterruptArgs),
    MouseOver(MouseOverInterruptArgs),
    MouseOut(MouseOutInterruptArgs),
    ContextMenu(ContextMenuInterruptArgs),
    Image(ImageLoadInterruptArgs),
    AddedLayer(AddedLayerArgs),
    FormCheckboxToggle(FormCheckboxToggleArgs),
    FormTextboxChange(FormTextboxChangeArgs),
    FormTextboxInput(FormTextboxInputArgs),
    FormButtonClick(FormButtonClickArgs),
}

#[derive(Deserialize)]
#[repr(C)]
pub struct FormCheckboxToggleArgs {
    pub state: bool,
    pub id_chain: Vec<u32>,
}

#[derive(Deserialize)]
#[repr(C)]
pub struct FormTextboxChangeArgs {
    pub text: String,
    pub id_chain: Vec<u32>,
}

#[derive(Deserialize)]
#[repr(C)]
pub struct FormTextboxInputArgs {
    pub text: String,
    pub id_chain: Vec<u32>,
}

#[derive(Deserialize)]
#[repr(C)]
pub struct FormButtonClickArgs {
    pub id_chain: Vec<u32>,
}

#[derive(Deserialize)]
#[repr(C)]
pub struct ClapInterruptArgs {
    pub x: f64,
    pub y: f64,
}

#[derive(Deserialize)]
#[repr(C)]
pub struct ScrollInterruptArgs {
    pub delta_x: f64,
    pub delta_y: f64,
}

#[derive(Deserialize)]
#[repr(C)]
pub struct TouchMessage {
    pub x: f64,
    pub y: f64,
    pub identifier: i64,
    pub delta_x: f64,
    pub delta_y: f64,
}

#[derive(Deserialize)]
#[repr(C)]
pub struct TouchStartInterruptArgs {
    pub touches: Vec<TouchMessage>,
}

#[derive(Deserialize)]
#[repr(C)]
pub struct TouchMoveInterruptArgs {
    pub touches: Vec<TouchMessage>,
}

#[derive(Deserialize)]
#[repr(C)]
pub struct TouchEndInterruptArgs {
    pub touches: Vec<TouchMessage>,
}

#[derive(Deserialize)]
#[repr(C)]
pub enum MouseButtonMessage {
    Left,
    Right,
    Middle,
    Unknown,
}

#[derive(Deserialize)]
#[repr(C)]
pub enum ModifierKeyMessage {
    Shift,
    Control,
    Alt,
    Command,
}

#[derive(Deserialize)]
#[repr(C)]
pub struct KeyDownInterruptArgs {
    pub key: String,
    pub modifiers: Vec<ModifierKeyMessage>,
    pub is_repeat: bool,
}

#[derive(Deserialize)]
#[repr(C)]
pub struct KeyUpInterruptArgs {
    pub key: String,
    pub modifiers: Vec<ModifierKeyMessage>,
    pub is_repeat: bool,
}

#[derive(Deserialize)]
#[repr(C)]
pub struct KeyPressInterruptArgs {
    pub key: String,
    pub modifiers: Vec<ModifierKeyMessage>,
    pub is_repeat: bool,
}

#[derive(Deserialize)]
#[repr(C)]
pub struct ClickInterruptArgs {
    pub x: f64,
    pub y: f64,
    pub button: MouseButtonMessage,
    pub modifiers: Vec<ModifierKeyMessage>,
}

#[derive(Deserialize)]
#[repr(C)]
pub struct DoubleClickInterruptArgs {
    pub x: f64,
    pub y: f64,
    pub button: MouseButtonMessage,
    pub modifiers: Vec<ModifierKeyMessage>,
}

#[derive(Deserialize)]
#[repr(C)]
pub struct MouseMoveInterruptArgs {
    pub x: f64,
    pub y: f64,
    pub button: MouseButtonMessage,
    pub modifiers: Vec<ModifierKeyMessage>,
}

#[derive(Deserialize)]
#[repr(C)]
pub struct WheelInterruptArgs {
    pub x: f64,
    pub y: f64,
    pub delta_x: f64,
    pub delta_y: f64,
    pub modifiers: Vec<ModifierKeyMessage>,
}

#[derive(Deserialize)]
#[repr(C)]
pub struct MouseDownInterruptArgs {
    pub x: f64,
    pub y: f64,
    pub button: MouseButtonMessage,
    pub modifiers: Vec<ModifierKeyMessage>,
}

#[derive(Deserialize)]
#[repr(C)]
pub struct MouseUpInterruptArgs {
    pub x: f64,
    pub y: f64,
    pub button: MouseButtonMessage,
    pub modifiers: Vec<ModifierKeyMessage>,
}

#[derive(Deserialize)]
#[repr(C)]
pub struct MouseOverInterruptArgs {
    pub x: f64,
    pub y: f64,
    pub button: MouseButtonMessage,
    pub modifiers: Vec<ModifierKeyMessage>,
}

#[derive(Deserialize)]
#[repr(C)]
pub struct MouseOutInterruptArgs {
    pub x: f64,
    pub y: f64,
    pub button: MouseButtonMessage,
    pub modifiers: Vec<ModifierKeyMessage>,
}

#[derive(Deserialize)]
#[repr(C)]
pub struct ContextMenuInterruptArgs {
    pub x: f64,
    pub y: f64,
    pub button: MouseButtonMessage,
    pub modifiers: Vec<ModifierKeyMessage>,
}

#[derive(Deserialize)]
#[repr(C)]
pub enum ImageLoadInterruptArgs {
    Reference(ImagePointerArgs),
    Data(ImageDataArgs),
}
#[derive(Deserialize)]
#[repr(C)]
pub struct ImagePointerArgs {
    pub id_chain: Vec<u32>,
    pub path: String,
    pub image_data: u64,
    pub image_data_length: usize,
    pub width: usize,
    pub height: usize,
}

#[derive(Deserialize)]
#[repr(C)]
pub struct ImageDataArgs {
    pub id_chain: Vec<u32>,
    pub path: String,
    pub width: usize,
    pub height: usize,
}

#[repr(C)]
pub struct InterruptBuffer {
    pub data_ptr: *const u8,
    pub length: u64,
}

#[repr(C)]
pub struct NativeMessageQueue {
    pub data_ptr: *mut [u8],
    pub length: u64,
}

#[derive(Serialize)]
pub struct MessageQueue {
    pub messages: Vec<NativeMessage>,
}

#[derive(Deserialize)]
#[repr(C)]
pub struct AddedLayerArgs {
    pub num_layers_added: u32,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize)]
#[repr(C)]
pub struct FramePatch {
    pub id_chain: Vec<u32>,
    pub size_x: Option<f64>,
    pub size_y: Option<f64>,
    pub transform: Option<Vec<f64>>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize, Clone)]
#[repr(C)]
pub struct CheckboxPatch {
    pub id_chain: Vec<u32>,
    pub transform: Option<Vec<f64>>,
    pub size_x: Option<f64>,
    pub size_y: Option<f64>,
    pub checked: Option<bool>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize, Clone)]
#[repr(C)]
pub struct TextboxPatch {
    pub id_chain: Vec<u32>,
    pub transform: Option<Vec<f64>>,
    pub size_x: Option<f64>,
    pub size_y: Option<f64>,
    pub text: Option<String>,
    pub background: Option<ColorVariantMessage>,
    pub stroke_color: Option<ColorVariantMessage>,
    pub stroke_width: Option<f64>,
    pub border_radius: Option<f64>,
    pub style: Option<TextStyleMessage>,
    pub focus_on_mount: Option<bool>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize)]
#[repr(C)]
pub struct ButtonPatch {
    pub id_chain: Vec<u32>,
    pub transform: Option<Vec<f64>>,
    pub size_x: Option<f64>,
    pub size_y: Option<f64>,
    pub content: Option<String>,
    pub style: Option<TextStyleMessage>,
}

#[derive(Default, Serialize)]
#[repr(C)]
pub struct CheckboxStyleMessage {
    //pub fill: Option<ColorMessage>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize)]
#[repr(C)]
pub struct OcclusionPatch {
    pub id_chain: Vec<u32>,
    pub z_index: u32,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize, Clone)]
#[repr(C)]
pub struct TextPatch {
    pub id_chain: Vec<u32>,
    pub content: Option<String>,
    pub transform: Option<Vec<f64>>,
    pub size_x: Option<f64>,
    pub size_y: Option<f64>,
    pub style: Option<TextStyleMessage>,
    pub style_link: Option<TextStyleMessage>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize, Clone, PartialEq)]
#[repr(C)]
pub struct TextStyleMessage {
    pub font: Option<FontPatch>,
    pub font_size: Option<f64>,
    pub fill: Option<ColorMessage>,
    pub underline: Option<bool>,
    pub align_multiline: Option<TextAlignHorizontalMessage>,
    pub align_vertical: Option<TextAlignVerticalMessage>,
    pub align_horizontal: Option<TextAlignHorizontalMessage>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize)]
#[repr(C)]
pub struct ImagePatch {
    pub id_chain: Vec<u32>,
    pub path: Option<String>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Serialize, Clone, PartialEq)]
#[repr(C)]
pub enum ColorMessage {
    Rgba([f64; 4]),
    Rgb([f64; 3]),
}

impl Default for ColorMessage {
    fn default() -> Self {
        ColorMessage::Rgba([1.0, 0.5, 0.0, 1.0])
    }
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize, Clone, PartialEq)]
#[repr(C)]
pub enum TextAlignHorizontalMessage {
    #[default]
    Left,
    Center,
    Right,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize, Clone, PartialEq)]
#[repr(C)]
pub enum TextAlignVerticalMessage {
    #[default]
    Top,
    Center,
    Bottom,
}

#[derive(Serialize)]
#[repr(C)]
pub struct LinkStyleMessage {
    pub font: Option<FontPatch>,
    pub fill: Option<ColorMessage>,
    pub underline: Option<bool>,
    pub size: Option<f64>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize)]
#[repr(C)]
pub struct ScrollerPatch {
    pub id_chain: Vec<u32>,
    pub size_x: Option<f64>,
    pub size_y: Option<f64>,
    pub size_inner_pane_x: Option<f64>,
    pub size_inner_pane_y: Option<f64>,
    pub transform: Option<Vec<f64>>,
    pub scroll_x: Option<bool>,
    pub scroll_y: Option<bool>,
    pub subtree_depth: u32,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Serialize)]
#[repr(C)]
pub struct AnyCreatePatch {
    pub id_chain: Vec<u32>,
    pub clipping_ids: Vec<Vec<u32>>,
    pub scroller_ids: Vec<Vec<u32>>,
    pub z_index: u32,
}

// Possible approach to heterogeneous rich text:
// #[repr(C)]
// pub struct TextCommand {
//     pub set_font: Option<String>,
//     pub set_weight: Option<String>,
//     pub set_fill_color: Option<String>,
//     pub set_stroke_color: Option<String>,
//     pub set_decoration: Option<String>,
// }

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Serialize, Clone, PartialEq)]
#[repr(C)]
pub enum FontPatch {
    System(SystemFontMessage),
    Web(WebFontMessage),
    Local(LocalFontMessage),
}

impl Default for FontPatch {
    fn default() -> Self {
        Self::System(SystemFontMessage::default())
    }
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Serialize, Clone, PartialEq)]
#[repr(C)]
pub struct SystemFontMessage {
    pub family: Option<String>,
    pub style: Option<FontStyleMessage>,
    pub weight: Option<FontWeightMessage>,
}

impl Default for SystemFontMessage {
    fn default() -> Self {
        Self {
            family: Some("Brush Script MT".to_string()),
            style: Some(FontStyleMessage::Normal),
            weight: Some(FontWeightMessage::Normal),
        }
    }
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Serialize, Clone, PartialEq)]
#[repr(C)]
pub struct WebFontMessage {
    pub family: Option<String>,
    pub url: Option<String>,
    pub style: Option<FontStyleMessage>,
    pub weight: Option<FontWeightMessage>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Serialize, Clone, PartialEq)]
#[repr(C)]
pub struct LocalFontMessage {
    pub family: Option<String>,
    pub path: Option<String>,
    pub style: Option<FontStyleMessage>,
    pub weight: Option<FontWeightMessage>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Clone, Serialize, PartialEq)]
#[repr(C)]
pub enum FontStyleMessage {
    Normal,
    Italic,
    Oblique,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Clone, Serialize, PartialEq)]
#[repr(C)]
pub enum FontWeightMessage {
    Thin,
    ExtraLight,
    Light,
    Normal,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Serialize)]
#[repr(C)]
pub struct LayerAddPatch {
    pub num_layers_to_add: usize,
}
