
use std::ffi::CString;

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
pub struct NativeArgsClick {
    pub x: f64,
    pub y: f64,
    //TODO: probably native element id (in case of native element click), offset
    //TODO: right/middle/left click
}

#[repr(C)]
pub struct ClippingPatch {
    pub size_x: Option<TextSize>,
    pub size_y: Option<TextSize>,
    pub transform: Option<Affine>,
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

#[derive(Default)]
#[repr(C)]
pub struct TextPatch {
    pub content: Option<CString>, //See `TextContentMessage` for a sketched-out approach to rich text
    pub transform: Option<Affine>,
    pub size_x: Option<TextSize>,
    pub size_y: Option<TextSize>,
}
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
    pub set_font: Option<CString>,
    pub set_weight: Option<CString>,
    pub set_fill_color: Option<CString>,
    pub set_stroke_color: Option<CString>,
    pub set_decoration: Option<CString>,
}





