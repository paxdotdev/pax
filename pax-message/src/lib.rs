#[macro_use]
extern crate serde;


use std::ffi::CString;
use std::os::raw::c_char;

use serde::{Serialize};

#[derive(Serialize)]
pub enum NativeMessage {
    TextCreate(Vec<u64>), //node instance ID, "id_chain"
    TextUpdate(TextPatch),
    TextDelete(Vec<u64>),
    FrameCreate(Vec<u64>),
    FrameUpdate(FramePatch),
    FrameDelete(Vec<u64>),
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
}


pub struct AnyCreatePatch {
    pub id_chain: Vec<u64>,
    pub clipping_ids: Vec<Vec<u64>>,
}


#[repr(C)]
pub struct TextCommand {
    pub set_font: Option<String>,
    pub set_weight: Option<String>,
    pub set_fill_color: Option<String>,
    pub set_stroke_color: Option<String>,
    pub set_decoration: Option<String>,
}


