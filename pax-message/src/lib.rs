#[macro_use]
extern crate serde;


use std::ffi::CString;
use std::os::raw::c_char;

use serde::{Serialize};

#[derive(Serialize)]
pub enum NativeMessage {
    TextCreate(u64), //node instance ID
    TextUpdate(TextPatch),
    TextDelete(u64),
    // ClippingCreate(u64),
    // ClippingUpdate(u64, ClippingPatch),
    // ClippingDelete(u64),
    //TODO: form controls
    //TODO: scroll containers

    // TODO: perhaps handle input events in a separate struct, to minimize
    //       de/serialization boilerplate
    // NativeEventClick(NativeArgsClick)
}

#[repr(C)]
pub struct NativeMessageQueue {
    pub data_ptr: *mut [u8], //JSON
    // pub msg_ptr: *const NativeMessage,
    pub length: u64,
}

#[derive(Serialize)]
pub struct MessageQueue {
    pub messages: Vec<NativeMessage>,
}

#[repr(C)]
pub struct NativeArgsClick {
    pub x: f64,
    pub y: f64,
    //TODO: probably native element id (in case of native element click), offset
    //TODO: right/middle/left click
}
//
// #[repr(C)]
// pub struct ClippingPatch {
//     pub size_x: Option<TextSize>,
//     pub size_y: Option<TextSize>,
//     pub transform: Option<Affine>,
// }

#[derive(Serialize)]
pub enum TextSize {
    Auto(),
    Pixels(f64),
}

#[derive(Default, Serialize)]
#[repr(C)]
pub struct TextPatch {
    pub id: u64,
    pub content: Option<String>, //See `TextContentMessage` for a sketched-out approach to rich text
    pub transform: Option<[f64; 6]>,
    pub size_x: Option<TextSize>,
    pub size_y: Option<TextSize>,
}


#[repr(C)]
pub struct TextCommand {
    pub set_font: Option<String>,
    pub set_weight: Option<String>,
    pub set_fill_color: Option<String>,
    pub set_stroke_color: Option<String>,
    pub set_decoration: Option<String>,
}


