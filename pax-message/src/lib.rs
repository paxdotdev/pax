pub mod reflection;

//FUTURE: feature-flag, only for Web builds
#[allow(unused_imports)]
use wasm_bindgen::prelude::*;

pub mod refcell_debug;
pub use serde;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Serialize)]
/// Messages emitted by the runtime to create, update, delete, or configure native/chassis resources.
pub enum NativeMessage {
    TextCreate(AnyCreatePatch),
    TextUpdate(TextPatch),
    TextDelete(u32),
    FrameCreate(AnyCreatePatch),
    FrameUpdate(FramePatch),
    FrameDelete(u32),
    EventBlockerCreate(AnyCreatePatch),
    EventBlockerUpdate(EventBlockerPatch),
    EventBlockerDelete(u32),
    CheckboxCreate(AnyCreatePatch),
    CheckboxUpdate(CheckboxPatch),
    CheckboxDelete(u32),
    NativeImageCreate(AnyCreatePatch),
    NativeImageUpdate(NativeImagePatch),
    NativeImageDelete(u32),
    YoutubeVideoCreate(AnyCreatePatch),
    YoutubeVideoUpdate(YoutubeVideoPatch),
    YoutubeVideoDelete(u32),
    TextboxCreate(AnyCreatePatch),
    TextboxUpdate(TextboxPatch),
    TextboxDelete(u32),
    SliderCreate(AnyCreatePatch),
    SliderUpdate(SliderPatch),
    SliderDelete(u32),
    DropdownCreate(AnyCreatePatch),
    DropdownUpdate(DropdownPatch),
    DropdownDelete(u32),
    RadioListCreate(AnyCreatePatch),
    RadioListUpdate(RadioListPatch),
    RadioListDelete(u32),
    ButtonCreate(AnyCreatePatch),
    ButtonUpdate(ButtonPatch),
    ButtonDelete(u32),
    PhotoPickerCreate(AnyCreatePatch),
    PhotoPickerUpdate(PhotoPickerPatch),
    PhotoPickerDelete(u32),
    ScrollerCreate(AnyCreatePatch),
    ScrollerUpdate(ScrollerPatch),
    ScrollerDelete(u32),
    ImageLoad(ImagePatch),
    LayerAdd(LayerAddPatch), //FUTURE: native form controls
    ShrinkLayersTo(u32),
    NativeMaskUpdate(NativeMaskPatch),
    Navigate(NavigationPatch),
    SetCursor(SetCursorPatch),
    Screenshot(ScreenshotPatch),
}

#[derive(Deserialize)]
#[repr(C)]
/// Events and data packets sent from the chassis back into the Pax runtime.
pub enum NativeInterrupt {
    ChassisResizeRequestCollection(Vec<ChassisResizeRequestArgs>),
    SelectStart(SelectStartArgs),
    Focus(FocusInterruptArgs),
    ClickOrTap(ClickOrTapInterruptArgs),
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
    ContextMenu(ContextMenuInterruptArgs),
    Image(ImageLoadInterruptArgs),
    AddedLayer(AddedLayerArgs),
    TextInput(TextInputArgs),
    FormCheckboxToggle(FormCheckboxToggleArgs),
    FormDropdownChange(FormDropdownChangeArgs),
    FormSliderChange(FormSliderChangeArgs),
    FormRadioListChange(FormRadioListChangeArgs),
    FormTextboxChange(FormTextboxChangeArgs),
    FormTextboxInput(FormTextboxInputArgs),
    FormButtonClick(FormButtonClickArgs),
    PhotoPicker(PhotoPickerInterruptArgs),
    // TODO: remove alias once all persisted/older web chassis payloads use `ScrollerPosition`.
    #[serde(alias = "Scrollbar")]
    ScrollerPosition(ScrollerPositionInterruptArgs),
    BrowserConfig(BrowserConfigInterruptArgs),
    RenderSurfaceUpdate(RenderSurfaceUpdateArgs),
    ViewportResize(ViewportResizeArgs),
    RouteChange(RouteChangeInterruptArgs),
    VisualViewportUpdate(VisualViewportUpdateArgs),
    Gyro(GyroInterruptArgs),
    Accel(AccelInterruptArgs),
    DropFile(DropFileArgs),
    Screenshot(ImageLoadInterruptArgs),
}

#[derive(Deserialize)]
#[repr(C)]
/// Chassis response containing measured native control bounds.
pub struct ChassisResizeRequestArgs {
    pub id: u32,
    pub width: f64,
    pub height: f64,
}

#[derive(Deserialize)]
#[repr(C)]
/// Checkbox state-change interrupt payload.
pub struct FormCheckboxToggleArgs {
    pub state: bool,
    pub id: u32,
}

#[derive(Deserialize)]
#[repr(C)]
/// Focus interrupt marker payload.
pub struct FocusInterruptArgs {}

#[derive(Deserialize)]
#[repr(C)]
/// Selection-start interrupt marker payload.
pub struct SelectStartArgs {}

#[derive(Deserialize)]
#[repr(C)]
/// Textbox committed-value change payload.
pub struct FormTextboxChangeArgs {
    pub text: String,
    pub id: u32,
}

#[derive(Deserialize)]
#[repr(C)]
/// Dropdown selection-change payload.
pub struct FormDropdownChangeArgs {
    pub id: u32,
    pub selected_id: u32,
}

#[derive(Deserialize)]
#[repr(C)]
/// Slider value-change payload.
pub struct FormSliderChangeArgs {
    pub id: u32,
    pub value: f64,
}

#[derive(Deserialize)]
#[repr(C)]
/// Radio-list selection-change payload.
pub struct FormRadioListChangeArgs {
    pub id: u32,
    pub selected_id: u32,
}

#[derive(Deserialize)]
#[repr(C)]
/// Raw text input payload delivered by a native text control.
pub struct TextInputArgs {
    pub text: String,
    pub id: u32,
}

#[derive(Deserialize)]
#[repr(C)]
/// Textbox in-progress input payload.
pub struct FormTextboxInputArgs {
    pub text: String,
    pub id: u32,
}

#[derive(Deserialize)]
#[repr(C)]
/// Native button click payload.
pub struct FormButtonClickArgs {
    pub id: u32,
}

#[derive(Deserialize, Clone)]
#[repr(C)]
/// Selected image metadata and optional copied bytes from a native photo picker.
pub struct PhotoPickerAssetArgs {
    pub temp_id: String,
    pub file_name: Option<String>,
    pub mime_type: String,
    pub byte_size: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub source_kind: String,
    pub handle: Option<String>,
    #[serde(default)]
    pub data: Vec<u8>,
}

#[derive(Deserialize, Clone)]
#[repr(C)]
/// Native photo picker completion payload.
pub struct PhotoPickerInterruptArgs {
    pub id: u32,
    pub request_id: u64,
    pub status: String,
    pub message: Option<String>,
    #[serde(default)]
    pub photos: Vec<PhotoPickerAssetArgs>,
}

#[derive(Deserialize)]
#[repr(C)]
/// Pointer tap/click payload normalized to window coordinates.
pub struct ClickOrTapInterruptArgs {
    pub x: f64,
    pub y: f64,
}

#[derive(Deserialize)]
#[repr(C)]
/// Location-targeted scroll delta payload.
pub struct ScrollInterruptArgs {
    pub x: f64,
    pub y: f64,
    pub delta_x: f64,
    pub delta_y: f64,
}

#[derive(Deserialize)]
#[repr(C)]
/// Native scroller position payload, including optional presentation offsets.
pub struct ScrollerPositionInterruptArgs {
    pub id: u32,
    pub scroll_x: f64,
    pub scroll_y: f64,
    pub presentation_scroll_x: Option<f64>,
    pub presentation_scroll_y: Option<f64>,
}

#[derive(Deserialize)]
#[repr(C)]
/// Browser capability flags discovered by the web chassis.
pub struct BrowserConfigInterruptArgs {
    pub allow_scroller_vector_layers: bool,
    pub allow_nested_scroller_vector_layers: bool,
}

#[derive(Deserialize)]
#[repr(C)]
/// Browser notification that a retained render surface must be reconfigured.
pub struct RenderSurfaceUpdateArgs {
    #[serde(default)]
    pub layer_id: Option<u32>,
}

#[derive(Deserialize)]
#[repr(C)]
/// Layout viewport resize payload for the root app surface.
pub struct ViewportResizeArgs {
    pub width: f64,
    pub height: f64,
}

#[derive(Deserialize)]
#[repr(C)]
/// Canonical route-location payload pushed from chassis state into the runtime.
pub struct RouteChangeInterruptArgs {
    pub path_segments: Vec<String>,
    pub query: HashMap<String, Vec<String>>,
    pub fragment: Option<String>,
}

#[derive(Deserialize)]
#[repr(C)]
/// Browser visual viewport payload for page-scroll-backed root scrollers.
pub struct VisualViewportUpdateArgs {
    pub width: f64,
    pub height: f64,
    pub offset_x: f64,
    pub offset_y: f64,
    pub page_scroll_x: f64,
    pub page_scroll_y: f64,
}

#[derive(Deserialize)]
#[repr(C)]
/// Device orientation payload, in degrees.
pub struct GyroInterruptArgs {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Deserialize)]
#[repr(C)]
/// Device acceleration payload, in meters per second squared.
pub struct AccelInterruptArgs {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Deserialize)]
#[repr(C)]
/// One touch point in a multi-touch interrupt.
pub struct TouchMessage {
    pub x: f64,
    pub y: f64,
    pub identifier: i64,
    pub delta_x: f64,
    pub delta_y: f64,
}

#[derive(Deserialize)]
#[repr(C)]
/// Touch-start interrupt payload.
pub struct TouchStartInterruptArgs {
    pub touches: Vec<TouchMessage>,
}

#[derive(Deserialize)]
#[repr(C)]
/// Touch-move interrupt payload.
pub struct TouchMoveInterruptArgs {
    pub touches: Vec<TouchMessage>,
}

#[derive(Deserialize)]
#[repr(C)]
/// Touch-end interrupt payload.
pub struct TouchEndInterruptArgs {
    pub touches: Vec<TouchMessage>,
}

#[derive(Deserialize)]
#[repr(C)]
/// File-drop interrupt payload.
pub struct DropFileArgs {
    pub x: f64,
    pub y: f64,
    pub name: String,
    pub mime_type: String,
    pub size: u64,
}

#[derive(Deserialize, Clone)]
#[repr(C)]
/// Normalized mouse button identifier.
pub enum MouseButtonMessage {
    Left,
    Right,
    Middle,
    Unknown,
}

#[derive(Deserialize)]
#[repr(C)]
/// Normalized keyboard modifier identifier.
pub enum ModifierKeyMessage {
    Shift,
    Control,
    Alt,
    Command,
}

#[derive(Deserialize)]
#[repr(C)]
/// Key-down interrupt payload.
pub struct KeyDownInterruptArgs {
    pub key: String,
    pub modifiers: Vec<ModifierKeyMessage>,
    pub is_repeat: bool,
}

#[derive(Deserialize)]
#[repr(C)]
/// Key-up interrupt payload.
pub struct KeyUpInterruptArgs {
    pub key: String,
    pub modifiers: Vec<ModifierKeyMessage>,
    pub is_repeat: bool,
}

#[derive(Deserialize)]
#[repr(C)]
/// Key-press interrupt payload.
pub struct KeyPressInterruptArgs {
    pub key: String,
    pub modifiers: Vec<ModifierKeyMessage>,
    pub is_repeat: bool,
}

#[derive(Deserialize)]
#[repr(C)]
/// Click interrupt payload.
pub struct ClickInterruptArgs {
    pub x: f64,
    pub y: f64,
    pub button: MouseButtonMessage,
    pub modifiers: Vec<ModifierKeyMessage>,
}

#[derive(Deserialize)]
#[repr(C)]
/// Double-click interrupt payload.
pub struct DoubleClickInterruptArgs {
    pub x: f64,
    pub y: f64,
    pub button: MouseButtonMessage,
    pub modifiers: Vec<ModifierKeyMessage>,
}

#[derive(Deserialize)]
#[repr(C)]
/// Mouse-move interrupt payload.
pub struct MouseMoveInterruptArgs {
    pub x: f64,
    pub y: f64,
    pub button: MouseButtonMessage,
    pub modifiers: Vec<ModifierKeyMessage>,
}

#[derive(Deserialize)]
#[repr(C)]
/// Wheel interrupt payload.
pub struct WheelInterruptArgs {
    pub x: f64,
    pub y: f64,
    pub delta_x: f64,
    pub delta_y: f64,
    pub modifiers: Vec<ModifierKeyMessage>,
}

#[derive(Deserialize)]
#[repr(C)]
/// Mouse-down interrupt payload.
pub struct MouseDownInterruptArgs {
    pub x: f64,
    pub y: f64,
    pub button: MouseButtonMessage,
    pub modifiers: Vec<ModifierKeyMessage>,
}

#[derive(Deserialize)]
#[repr(C)]
/// Mouse-up interrupt payload.
pub struct MouseUpInterruptArgs {
    pub x: f64,
    pub y: f64,
    pub button: MouseButtonMessage,
    pub modifiers: Vec<ModifierKeyMessage>,
}

#[derive(Deserialize)]
#[repr(C)]
/// Mouse-over interrupt payload.
pub struct MouseOverInterruptArgs {
    pub x: f64,
    pub y: f64,
    pub button: MouseButtonMessage,
    pub modifiers: Vec<ModifierKeyMessage>,
}

#[derive(Deserialize)]
#[repr(C)]
/// Mouse-out interrupt payload.
pub struct MouseOutInterruptArgs {
    pub x: f64,
    pub y: f64,
    pub button: MouseButtonMessage,
    pub modifiers: Vec<ModifierKeyMessage>,
}

#[derive(Deserialize)]
#[repr(C)]
/// Context-menu interrupt payload.
pub struct ContextMenuInterruptArgs {
    pub x: f64,
    pub y: f64,
    pub button: MouseButtonMessage,
    pub modifiers: Vec<ModifierKeyMessage>,
}

#[derive(Deserialize)]
#[repr(C)]
/// Image-load response, either by pointer or copied metadata.
pub enum ImageLoadInterruptArgs {
    Reference(ImagePointerArgs),
    Data(ImageDataArgs),
}
#[derive(Deserialize)]
#[repr(C)]
/// Image-load payload carrying a pointer to chassis-owned image bytes.
pub struct ImagePointerArgs {
    pub id: u32,
    pub path: String,
    pub image_data: u64,
    pub image_data_length: usize,
    pub width: usize,
    pub height: usize,
}

#[derive(Deserialize)]
#[repr(C)]
/// Image-load payload carrying image metadata without a byte pointer.
pub struct ImageDataArgs {
    pub id: u32,
    pub path: String,
    pub width: usize,
    pub height: usize,
}

#[repr(C)]
/// Raw FFI buffer containing serialized interrupts.
pub struct InterruptBuffer {
    pub data_ptr: *const u8,
    pub length: u64,
}

#[repr(C)]
/// Raw FFI buffer containing serialized native messages.
pub struct NativeMessageQueue {
    pub data_ptr: *mut [u8],
    pub length: u64,
}

#[derive(Serialize)]
/// Serializable batch of native messages emitted for one runtime tick.
pub struct MessageQueue {
    pub messages: Vec<NativeMessage>,
}

#[derive(Deserialize)]
#[repr(C)]
/// Chassis acknowledgement that render layers were added.
pub struct AddedLayerArgs {
    pub num_layers_added: u32,
    #[serde(default)]
    pub layer_id: Option<u32>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize)]
#[repr(C)]
/// Create/update patch for a native frame host.
pub struct FramePatch {
    pub id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_frame: Option<Option<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z_index: Option<i32>,
    pub clip_content: Option<bool>,
    pub border_radius: Option<f64>,
    pub size_x: Option<f64>,
    pub size_y: Option<f64>,
    pub transform: Option<Vec<f64>>,
    pub clip_path: Option<String>,
    pub opacity: Option<f64>,
    pub presented_bounds: Option<[f64; 4]>,
    pub presented_clip_bounds: Option<[f64; 4]>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize, Clone)]
#[repr(C)]
/// One vector coverage path entry for a native occlusion mask.
pub struct MaskPathPatch {
    pub path: String,
    pub clips: Vec<String>,
    pub opacity: Option<f64>,
}

impl PartialEq for MaskPathPatch {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
            && self.clips == other.clips
            && self.opacity.map(f64::to_bits) == other.opacity.map(f64::to_bits)
    }
}

impl Eq for MaskPathPatch {}

impl std::hash::Hash for MaskPathPatch {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.path.hash(state);
        self.clips.hash(state);
        self.opacity.map(f64::to_bits).hash(state);
    }
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize, Clone, PartialEq)]
#[repr(C)]
/// Create/update patch for a native occlusion mask surface.
pub struct NativeMaskPatch {
    pub id: u32,
    pub size_x: f64,
    pub size_y: f64,
    pub entries: Vec<MaskPathPatch>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize)]
#[repr(C)]
/// Create/update patch for an invisible native hit-test blocker.
pub struct EventBlockerPatch {
    pub id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_frame: Option<Option<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z_index: Option<i32>,
    pub size_x: Option<f64>,
    pub size_y: Option<f64>,
    pub transform: Option<Vec<f64>>,
    pub opacity: Option<f64>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize, Clone)]
#[repr(C)]
/// Create/update patch for a native checkbox.
pub struct CheckboxPatch {
    pub id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_frame: Option<Option<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z_index: Option<i32>,
    pub background: Option<ColorMessage>,
    pub background_checked: Option<ColorMessage>,
    pub outline_color: Option<ColorMessage>,
    pub outline_width: Option<f64>,
    pub border_radius: Option<f64>,
    pub transform: Option<Vec<f64>>,
    pub size_x: Option<f64>,
    pub size_y: Option<f64>,
    pub opacity: Option<f64>,
    pub checked: Option<bool>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize, Clone)]
#[repr(C)]
/// Create/update patch for a native image element.
pub struct NativeImagePatch {
    pub id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_frame: Option<Option<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z_index: Option<i32>,
    pub transform: Option<Vec<f64>>,
    pub size_x: Option<f64>,
    pub size_y: Option<f64>,
    pub opacity: Option<f64>,
    pub url: Option<String>,
    pub fit: Option<String>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize, Clone)]
#[repr(C)]
/// Create/update patch for an embedded YouTube video.
pub struct YoutubeVideoPatch {
    pub id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_frame: Option<Option<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z_index: Option<i32>,
    pub transform: Option<Vec<f64>>,
    pub size_x: Option<f64>,
    pub size_y: Option<f64>,
    pub opacity: Option<f64>,
    pub url: Option<String>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize, Clone)]
#[repr(C)]
/// Create/update patch for a native dropdown/select element.
pub struct DropdownPatch {
    pub id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_frame: Option<Option<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z_index: Option<i32>,
    pub selected_id: Option<u32>,
    pub options: Option<Vec<String>>,
    pub transform: Option<Vec<f64>>,
    pub size_x: Option<f64>,
    pub size_y: Option<f64>,
    pub opacity: Option<f64>,
    pub background: Option<ColorMessage>,
    pub stroke_color: Option<ColorMessage>,
    pub stroke_width: Option<f64>,
    pub border_radius: Option<f64>,
    pub style: Option<TextStyleMessage>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize, Clone)]
#[repr(C)]
/// Create/update patch for a native radio list.
pub struct RadioListPatch {
    pub id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_frame: Option<Option<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z_index: Option<i32>,
    pub selected_id: Option<u32>,
    pub options: Option<Vec<String>>,
    pub style: Option<TextStyleMessage>,
    pub background_checked: Option<ColorMessage>,
    pub outline_color: Option<ColorMessage>,
    pub outline_width: Option<f64>,
    pub background: Option<ColorMessage>,
    pub transform: Option<Vec<f64>>,
    pub size_x: Option<f64>,
    pub size_y: Option<f64>,
    pub opacity: Option<f64>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize, Clone)]
#[repr(C)]
/// Create/update patch for a native slider.
pub struct SliderPatch {
    pub id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_frame: Option<Option<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z_index: Option<i32>,
    pub value: Option<f64>,
    pub step: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub transform: Option<Vec<f64>>,
    pub size_x: Option<f64>,
    pub size_y: Option<f64>,
    pub opacity: Option<f64>,
    pub accent: Option<ColorMessage>,
    pub background: Option<ColorMessage>,
    pub border_radius: Option<f64>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize, Clone)]
#[repr(C)]
/// Create/update patch for a native textbox.
pub struct TextboxPatch {
    pub id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_frame: Option<Option<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z_index: Option<i32>,
    pub transform: Option<Vec<f64>>,
    pub size_x: Option<f64>,
    pub size_y: Option<f64>,
    pub opacity: Option<f64>,
    pub text: Option<String>,
    pub background: Option<ColorMessage>,
    pub stroke_color: Option<ColorMessage>,
    pub stroke_width: Option<f64>,
    pub border_radius: Option<f64>,
    pub style: Option<TextStyleMessage>,
    pub focus_on_mount: Option<bool>,
    pub placeholder: Option<String>,
    pub outline_color: Option<ColorMessage>,
    pub outline_width: Option<f64>,
    pub is_text_area: Option<bool>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize)]
#[repr(C)]
/// Create/update patch for a native button.
pub struct ButtonPatch {
    pub id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_frame: Option<Option<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z_index: Option<i32>,
    pub hover_color: Option<ColorMessage>,
    pub outline_stroke_color: Option<ColorMessage>,
    pub outline_stroke_width: Option<f64>,
    pub border_radius: Option<f64>,
    pub transform: Option<Vec<f64>>,
    pub size_x: Option<f64>,
    pub size_y: Option<f64>,
    pub opacity: Option<f64>,
    pub content: Option<String>,
    pub color: Option<ColorMessage>,
    pub style: Option<TextStyleMessage>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize, Clone)]
#[repr(C)]
/// Create/update patch for a transparent native photo picker hit target.
pub struct PhotoPickerPatch {
    pub id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_frame: Option<Option<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z_index: Option<i32>,
    pub transform: Option<Vec<f64>>,
    pub size_x: Option<f64>,
    pub size_y: Option<f64>,
    pub opacity: Option<f64>,
    pub trigger: Option<u64>,
    pub source: Option<String>,
    pub allow_multiple: Option<bool>,
    pub accept: Option<String>,
    pub include_bytes: Option<bool>,
    pub max_bytes_per_photo: Option<u64>,
}

#[derive(Default, Serialize)]
#[repr(C)]
/// Style payload shared by checkbox-like controls.
pub struct CheckboxStyleMessage {
    //pub fill: Option<ColorMessage>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize)]
#[repr(C)]
/// Request for the chassis to navigate to a URL.
pub struct NavigationPatch {
    pub url: String,
    pub target: String,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize)]
#[repr(C)]
/// Request for the chassis to update cursor style.
pub struct SetCursorPatch {
    pub cursor: String,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize, Clone)]
#[repr(C)]
/// Create/update patch for native or browser-managed text.
pub struct TextPatch {
    pub id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_frame: Option<Option<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z_index: Option<i32>,
    pub content: Option<String>,
    pub editable: Option<bool>,
    pub selectable: Option<bool>,
    pub clip: Option<bool>,
    pub markdown: Option<bool>,
    pub transform: Option<Vec<f64>>,
    pub size_x: Option<f64>,
    pub size_y: Option<f64>,
    pub opacity: Option<f64>,
    pub style: Option<TextStyleMessage>,
    pub style_link: Option<TextStyleMessage>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize, Clone, PartialEq)]
#[repr(C)]
/// Serializable text style payload shared with chassis text renderers.
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
/// Image-load request sent to the chassis.
pub struct ImagePatch {
    pub id: u32,
    pub path: Option<String>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Serialize, Clone, PartialEq)]
#[repr(C)]
/// Serializable color payload for native/chassis messages.
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
/// Serializable horizontal text alignment.
pub enum TextAlignHorizontalMessage {
    #[default]
    Left,
    Center,
    Right,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize, Clone, PartialEq)]
#[repr(C)]
/// Serializable vertical text alignment.
pub enum TextAlignVerticalMessage {
    #[default]
    Top,
    Center,
    Bottom,
}

#[derive(Serialize)]
#[repr(C)]
/// Serializable style payload for link text.
pub struct LinkStyleMessage {
    pub font: Option<FontPatch>,
    pub fill: Option<ColorMessage>,
    pub underline: Option<bool>,
    pub size: Option<f64>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize)]
#[repr(C)]
/// Create/update patch for a native/browser-owned scroller.
pub struct ScrollerPatch {
    pub id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_frame: Option<Option<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z_index: Option<i32>,
    pub transform: Option<Vec<f64>>,
    pub size_x: Option<f64>,
    pub size_y: Option<f64>,
    pub opacity: Option<f64>,
    pub clip_content: Option<bool>,
    pub border_radius: Option<f64>,
    pub size_inner_pane_x: Option<f64>,
    pub size_inner_pane_y: Option<f64>,
    pub snap_points_x: Option<Vec<f64>>,
    pub snap_points_y: Option<Vec<f64>>,
    pub scroll_x: Option<f64>,
    pub scroll_y: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presentation_scroll_x: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presentation_scroll_y: Option<f64>,
    pub scroll_enabled_x: Option<bool>,
    pub scroll_enabled_y: Option<bool>,
    pub content_layer_id: Option<u32>,
    pub presented_bounds: Option<[f64; 4]>,
    pub presented_clip_bounds: Option<[f64; 4]>,
    pub subtree_depth: u32,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Serialize)]
#[repr(C)]
/// Common creation payload shared by native element families.
pub struct AnyCreatePatch {
    pub id: u32,
    pub parent_frame: Option<u32>,
    pub occlusion_layer_id: u32,
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
/// Serializable font selection payload.
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
/// System font family payload.
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
/// Web font payload, including family and source URL.
pub struct WebFontMessage {
    pub family: Option<String>,
    pub url: Option<String>,
    pub style: Option<FontStyleMessage>,
    pub weight: Option<FontWeightMessage>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Serialize, Clone, PartialEq)]
#[repr(C)]
/// Local bundled font payload.
pub struct LocalFontMessage {
    pub family: Option<String>,
    pub path: Option<String>,
    pub style: Option<FontStyleMessage>,
    pub weight: Option<FontWeightMessage>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Clone, Serialize, PartialEq)]
#[repr(C)]
/// Serializable font-style value.
pub enum FontStyleMessage {
    Normal,
    Italic,
    Oblique,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Clone, Serialize, PartialEq)]
#[repr(C)]
/// Serializable font-weight value.
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
/// Request to allocate additional native/canvas layers.
pub struct LayerAddPatch {
    pub num_layers_to_add: usize,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Serialize)]
#[repr(C)]
/// Request to capture a screenshot from the chassis.
pub struct ScreenshotPatch {
    pub id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
}

#[derive(Serialize, Deserialize)]
/// Completed screenshot bytes returned by the chassis.
pub struct ScreenshotData {
    pub id: u32,
    pub data: Vec<u8>,
    pub width: usize,
    pub height: usize,
}

#[derive(Serialize, Deserialize)]
/// Completed screenshot bytes for one physical surface within a logical canvas layer.
pub struct LayerSurfaceScreenshotData {
    pub id: u32,
    pub key: String,
    pub data: Vec<u8>,
    pub width: usize,
    pub height: usize,
    pub origin_x: f32,
    pub origin_y: f32,
    pub logical_width: f32,
    pub logical_height: f32,
}
