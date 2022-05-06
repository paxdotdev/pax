use std::ffi::CString;
use std::os::raw::c_char;

// REGISTER ANY NEW STRUCTS BELOW!
// this hacky function is not meant to be called; it acts as a manifest for `cbindgen`, which would not
// otherwise discover "orphan structs" https://github.com/eqrion/cbindgen/issues/596
// (cbindgen treats only functions as roots when statically crawling codebase)
// Alternatively, it may be possible to register `NativeMessage` as a "custom root" in a cbindgen.toml
#[no_mangle]
pub extern "C" fn __pax_message_manifest<T>(
    a: NativeMessage,
    b: NativeArgsClick,
    c: ClippingPatch,
    d: TextSize,
    e: Affine,
    f: TextCommand,
    g: NativeMessageQueue,
    h: COption<T>,
    // ^ New structs must be registered here for cbindgen ^
) { }

#[repr(C)]
pub enum NativeMessage {
    TextCreate(u64), //node instance ID
    TextUpdate(u64, TextPatch),
    TextDelete(u64),
    ClippingCreate(u64),
    ClippingUpdate(u64, ClippingPatch),
    ClippingDelete(u64),
    //TODO: form controls
    //TODO: scroll containers
    NativeEventClick(NativeArgsClick)
}

#[repr(C)]
pub struct NativeMessageQueue {
    pub msg_ptr: *const NativeMessage,
    pub length: u64,
}

#[repr(C)]
pub struct NativeArgsClick {
    pub x: f64,
    pub y: f64,
    //TODO: probably native element id (in case of native element click), offset
    //TODO: right/middle/left click
}

#[repr(C)]
pub struct ClippingPatch {
    pub size_x: *const TextSize,
    pub size_y: *const TextSize,
    pub transform: *const Affine,
}

#[repr(C)]
pub enum TextSize {
    Auto(),
    Pixels(f64),
}

#[repr(C)]
pub struct Affine {
    pub coefficients: [f64;6],
}

#[repr(C)]
pub enum COption<T> {
    Some(T),
    None
}

impl<T> Default for COption<T> {
    fn default() -> Self {
        COption::None
    }
}

#[derive(Default)]
#[repr(C)]
pub struct TextPatch {
    pub content: COption<CString>, //See `TextContentMessage` for a sketched-out approach to rich text
    pub transform: COption<Affine>,
    pub size_x: COption<TextSize>,
    pub size_y: COption<TextSize>,
}
//
// impl Into<TextPatchMessage> for TextPatch {
//     fn into(self) -> TextPatchMessage {
//         TextPatchMessage {
//             content: (),
//             transform: (),
//             size_x: (),
//             size_y: ()
//         }
//     }
// }

//TODO: write into() logic from Patch > PatchMessage
//
// #[repr(C)]
// pub struct TextPatchMessage {
//     pub content: *const c_char, //See `TextContentMessage` for a sketched-out approach to rich text
//     pub transform: *const Affine,
//     pub size_x: *const TextSize,
//     pub size_y: *const TextSize,
// }
//
// impl Default for TextPatch {
//     fn default() -> Self {
//         Self {
//             content: None,
//             transform: None,
//             size: [None, None],
//         }
//     }
// }

//
// #[repr(C)]
// pub struct TextContentMessage {
//     /// C-friendly `Vec<CString>`, along with explicit length.
//     /// In other renderers, these sorts of `spans` are sometimes referred to as `runs`
//     spans: *mut CString, //
//     spans_len: u64,
//     commands: *mut TextCommand, //C-friendly `Vec<MessageTextPropertiesCommand>`, along with explicit length
//     commands_len: u64,
// }

#[repr(C)]
pub struct TextCommand {
    pub set_font: *const c_char,
    pub set_weight: *const c_char,
    pub set_fill_color: *const c_char,
    pub set_stroke_color: *const c_char,
    pub set_decoration: *const c_char,
}


