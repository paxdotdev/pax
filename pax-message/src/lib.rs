#[macro_use]
extern crate serde;

pub mod reflection;

use std::ffi::CString;
use std::os::raw::c_char;

//FUTURE: feature-flag, only for Web builds
use wasm_bindgen::prelude::*;

use serde::{Serialize};

#[derive(Serialize)]
pub enum NativeMessage {
    TextCreate(AnyCreatePatch),
    TextUpdate(TextPatch),
    TextDelete(Vec<u64>), //node instance ID, "id_chain"
    FrameCreate(AnyCreatePatch),
    FrameUpdate(FramePatch),
    FrameDelete(Vec<u64>),
    ScrollerCreate(AnyCreatePatch),
    ScrollerUpdate(ScrollerPatch),
    ScrollerDelete(Vec<u64>),
    //FUTURE: native form controls

}

#[derive(Deserialize)]
#[repr(C)]
pub enum NativeInterrupt {
    Click(ClickInterruptArgs),
    Scroll(ScrollInterruptArgs),
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
pub struct ClickInterruptArgs {
    pub x: f64,
    pub y: f64,

    //FUTURE: right/middle/left click
}

#[derive(Deserialize)]
#[repr(C)]
pub struct ScrollInterruptArgs {
    pub x: f64,
    pub y: f64,
    pub delta_x: f64,
    pub delta_y: f64,
}


#[derive(Default, Serialize)]
#[repr(C)]
pub struct FramePatch {
    pub id_chain: Vec<u64>,
    pub size_x: Option<f64>,
    pub size_y: Option<f64>,
    pub transform: Option<Vec<f64>>,
}

#[derive(Default, Serialize)]
#[repr(C)]
pub struct TextPatch {
    pub id_chain: Vec<u64>,
    pub content: Option<String>, //See `TextContentMessage` for a sketched-out approach to rich text
    pub transform: Option<Vec<f64>>,
    pub size_x: Option<f64>,
    pub size_y: Option<f64>,
    pub font: FontPatch,
    pub fill: Option<ColorVariantMessage>, //FUTURE: more robust Fill support (multiple fills, ordering, gradients, opacity, etc.)
}

#[derive(Default, Serialize)]
#[repr(C)]
pub struct FontPatch {
    pub family:  Option<String>,
    pub variant:  Option<String>,
    pub size: Option<f64>,
}

#[derive(Serialize)]
#[repr(C)]
pub enum ColorVariantMessage {
    Hlca([f64; 4]),
    Rgba([f64; 4]),
    Rgb([f64; 3]),
}

impl Default for ColorVariantMessage {
    fn default() -> Self {
        ColorVariantMessage::Rgba([1.0,0.5,0.0,1.0])
    }
}





#[derive(Default, Serialize)]
#[repr(C)]
pub struct ScrollerPatch {
    pub id_chain: Vec<u64>,
    pub size_frame_x: Option<f64>,
    pub size_frame_y: Option<f64>,
    pub size_inner_pane_x: Option<f64>,
    pub size_inner_pane_y: Option<f64>,
    pub transform: Option<Vec<f64>>,
    pub scroll_x: Option<bool>,
    pub scroll_y: Option<bool>,
}


#[derive(Serialize)]
#[repr(C)]
pub struct AnyCreatePatch {
    pub id_chain: Vec<u64>,
    pub clipping_ids: Vec<Vec<u64>>,
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


