#![allow(non_snake_case)] //Non-snake-case is used here to help denote foreign structs, e.g. from Swift via C

extern crate core;

use std::ffi::c_void;
use std::collections::HashMap;

use std::mem::{transmute, ManuallyDrop};

use core_graphics::context::CGContext;
use pax_runtime::api::math::Point2;
use piet_coregraphics::CoreGraphicsContext;
use piet::{InterpolationMode, RenderContext as PietRenderContext};
use piet::kurbo;

use flexbuffers;
use flexbuffers::DeserializationError;
use serde::Serialize;

use pax_runtime::PaxEngine;

//Re-export all native message types; used by Swift via FFI.
//Note that any types exposed by pax_message must ALSO be added to `PaxCartridge.h`
//in order to be visible to Swift
pub use pax_message::*;
use pax_runtime::api::borrow;
use pax_runtime::api::{
    ButtonClick, Click, Event, Focus, ModifierKey, MouseButton, MouseEventArgs, RenderContext,
    SelectStart, TextboxChange,
};

struct ImgData<'a> {
    img: <CoreGraphicsContext<'a> as PietRenderContext>::Image,
    size: (usize, usize),
}

struct AppleRenderContext<'a> {
    backend: CoreGraphicsContext<'a>,
    image_map: HashMap<String, ImgData<'a>>,
}

impl<'a> AppleRenderContext<'a> {
    fn new(backend: CoreGraphicsContext<'a>) -> Self {
        Self {
            backend,
            image_map: HashMap::new(),
        }
    }
}

impl<'a> RenderContext for AppleRenderContext<'a> {
    fn fill(&mut self, _layer: usize, path: kurbo::BezPath, brush: &piet::PaintBrush) {
        self.backend.fill(path, brush);
    }

    fn stroke(
        &mut self,
        _layer: usize,
        path: kurbo::BezPath,
        brush: &piet::PaintBrush,
        width: f64,
    ) {
        self.backend.stroke(path, brush, width);
    }

    fn save(&mut self, _layer: usize) {
        let _ = self.backend.save();
    }

    fn restore(&mut self, _layer: usize) {
        let _ = self.backend.restore();
    }

    fn clip(&mut self, _layer: usize, path: kurbo::BezPath) {
        self.backend.clip(path);
    }

    fn load_image(&mut self, path: &str, buf: &[u8], width: usize, height: usize) {
        let img = self
            .backend
            .make_image(width, height, buf, piet::ImageFormat::RgbaSeparate)
            .expect("image creation successful");
        self.image_map.insert(
            path.to_owned(),
            ImgData {
                img,
                size: (width, height),
            },
        );
    }

    fn draw_image(&mut self, _layer: usize, image_path: &str, rect: kurbo::Rect) {
        let Some(data) = self.image_map.get(image_path) else {
            return;
        };
        self.backend
            .draw_image(&data.img, rect, InterpolationMode::Bilinear);
    }

    fn get_image_size(&mut self, image_path: &str) -> Option<(usize, usize)> {
        self.image_map.get(image_path).map(|img| img.size)
    }

    fn transform(&mut self, _layer: usize, affine: kurbo::Affine) {
        self.backend.transform(affine);
    }

    fn layers(&self) -> usize {
        1
    }
}

/// Container data structure for PaxEngine, aggregated to support passing across C bridge
#[repr(C)] //Exposed to Swift via PaxCartridge.h
pub struct PaxEngineContainer {
    pub _engine: *mut PaxEngine,
    //NOTE: since that has become a single field, this data structure may be be retired and `*mut PaxEngine` could be passed directly.
}

/// Destroy `engine` and clean up the `ManuallyDrop` container surround it.
#[no_mangle]
pub extern "C" fn pax_dealloc_engine(_container: *mut PaxEngineContainer) {
    //particularly for when we need to support elegant clean-up from attached harness
    unimplemented!();
}

/// Send `interrupt`s from the chassis, for example: user input
/// Note that in any single-threaded environment, these interrupts will happen
/// synchronously between engine ticks, allowing for safe unwrapping / borrowing
/// of engine and runtime here.
#[no_mangle]
pub extern "C" fn pax_interrupt(
    engine_container: *mut PaxEngineContainer,
    buffer: *const InterruptBuffer,
) {
    let engine = unsafe { Box::from_raw((*engine_container)._engine) };
    // let slice = unsafe { buffer.as_ref().unwrap() };

    let length: u64 = unsafe {
        (*buffer).length.try_into().unwrap() // length negative or overflowed
    };

    let slice = unsafe {
        if (*buffer).data_ptr.is_null() {
            &mut []
        } else {
            std::slice::from_raw_parts((*buffer).data_ptr, length.try_into().unwrap())
        }
    };

    let interrupt_wrapped: Result<NativeInterrupt, DeserializationError> = flexbuffers::from_slice(slice);
    let interrupt = interrupt_wrapped.unwrap();
    let globals = engine.runtime_context.globals();

    match &interrupt {
        NativeInterrupt::Focus(_args) => {
            engine.global_dispatch_focus(Focus {});
        }
        NativeInterrupt::SelectStart(_args) => {
            engine.global_dispatch_select_start(SelectStart {});
        }
        NativeInterrupt::ChassisResizeRequestCollection(collection) => {
            for args in collection {
                let node = engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id));
                if let Some(node) = node {
                    node.chassis_resize_request(args.width, args.height);
                }
            }
        }
        NativeInterrupt::Click(args) => {
            let topmost_node = engine
                .runtime_context
                .get_topmost_element_beneath_ray(Point2::new(args.x, args.y));
            let modifiers = args
                .modifiers
                .iter()
                .map(|x| ModifierKey::from(x))
                .collect();
            let args_click = Click {
                mouse: MouseEventArgs {
                    x: args.x,
                    y: args.y,
                    button: MouseButton::from(args.button.clone()),
                    modifiers,
                },
            };
            topmost_node.dispatch_click(
                Event::new(args_click),
                &globals,
                &engine.runtime_context,
            );
        }
        NativeInterrupt::FormRadioSetChange(args) => {
            let node = engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id));
            if let Some(node) = node {
                borrow!(node.instance_node).handle_native_interrupt(&node, &interrupt);
            }
        }
        NativeInterrupt::FormSliderChange(args) => {
            let node = engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id));
            if let Some(node) = node {
                borrow!(node.instance_node).handle_native_interrupt(&node, &interrupt);
            }
        }
        NativeInterrupt::FormDropdownChange(args) => {
            let node = engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id));
            if let Some(node) = node {
                borrow!(node.instance_node).handle_native_interrupt(&node, &interrupt);
            }
        }
        NativeInterrupt::FormButtonClick(args) => {
            if let Some(node) = engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id)) {
                node.dispatch_button_click(
                    Event::new(ButtonClick {}),
                    &globals,
                    &engine.runtime_context,
                );
            }
        }
        NativeInterrupt::FormTextboxInput(args) => {
            if let Some(node) = engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id)) {
                borrow!(node.instance_node).handle_native_interrupt(&node, &interrupt);
            }
        }
        NativeInterrupt::TextInput(args) => {
            if let Some(node) = engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id)) {
                borrow!(node.instance_node).handle_native_interrupt(&node, &interrupt);
            }
        }
        NativeInterrupt::FormTextboxChange(args) => {
            if let Some(node) = engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id)) {
                node.dispatch_textbox_change(
                    Event::new(TextboxChange {
                        text: args.text.clone(),
                    }),
                    &globals,
                    &engine.runtime_context,
                );
            }
        }
        NativeInterrupt::FormCheckboxToggle(args) => {
            if let Some(node) = engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id)) {
                borrow!(node.instance_node).handle_native_interrupt(&node, &interrupt);
            }
        }
        NativeInterrupt::Scrollbar(_args) => {}
        NativeInterrupt::Scroll(_args) => {}
        NativeInterrupt::Image(args) => match args {
            ImageLoadInterruptArgs::Reference(_ref_args) => {}
            ImageLoadInterruptArgs::Data(_) => {}
        },
        NativeInterrupt::AddedLayer(_args) => {}
        _ => {}
    }

    unsafe { (*engine_container)._engine = Box::into_raw(engine) };
}

/// Perform full tick of engine, including property computation, lifecycle event handling, and rendering side-effects.
/// Returns a message queue of native rendering actions encoded as a Flexbuffer via FFI to Swift.
/// The returned message queue requires explicit deallocation: `pax_deallocate_message_queue`
#[no_mangle] //Exposed to Swift via PaxCartridge.h
pub extern "C" fn pax_tick(
    engine_container: *mut PaxEngineContainer,
    cgContext: *mut c_void,
    width: f32,
    height: f32,
) -> *mut NativeMessageQueue {
    // note that f32 is essentially `CFloat`, per: https://doc.rust-lang.org/std/os/raw/type.c_float.html
    let mut engine = unsafe { Box::from_raw((*engine_container)._engine) };

    let will_cast_cgContext = cgContext as *mut CGContext;
    let ctx = unsafe { &mut *will_cast_cgContext };

    (*engine).set_viewport_size((width as f64, height as f64));
    let messages = (*engine).tick();

    // Native Apple chassis currently composites all vector layers into a single CoreGraphics
    // surface. Mirror the runtime's canvas bookkeeping so vector primitives render at all, even
    // though true multi-surface compositing is still future work.
    let max_native_layer = engine.runtime_context.layer_count.get();
    engine.runtime_context.add_canvas(max_native_layer);

    let mut render_context =
        AppleRenderContext::new(CoreGraphicsContext::new_y_up(ctx, height as f64, None));
    engine.render(&mut render_context as &mut dyn RenderContext);
    engine.runtime_context.clear_all_dirty_canvases();

    let wrapped_queue = MessageQueue { messages };
    let mut serializer = flexbuffers::FlexbufferSerializer::new();

    //side-effectfully serialize, mutating `serializer`
    wrapped_queue.serialize(&mut serializer).unwrap();

    let data_buffer = serializer.take_buffer();
    let length = data_buffer.len();

    let leaked_data: ManuallyDrop<Box<[u8]>> = ManuallyDrop::new(data_buffer.into_boxed_slice());

    let queue_container = unsafe {
        transmute(Box::new(NativeMessageQueue {
            data_ptr: Box::into_raw(ManuallyDrop::into_inner(leaked_data)),
            length: length as u64,
        }))
    };

    //`Box::into_raw` is our necessary manual clean-up, acting as a trigger to drop all of the RefCell::borrow_mut's throughout the tick lifecycle
    unsafe { (*engine_container)._engine = Box::into_raw(engine) };

    queue_container
}

/// Required manual cleanup callback from Swift after reading a frame's message queue.
/// If this is not called after `pax_tick` is invoked, we will have a memory leak.
#[no_mangle] //Exposed to Swift via PaxCartridge.h
pub extern "C" fn pax_dealloc_message_queue(queue: *mut NativeMessageQueue) {
    unsafe {
        let queue_container = Box::from_raw(queue);
        let data_buffer = Box::from_raw(queue_container.data_ptr);
        drop(data_buffer);
        drop(queue_container);
    }
}
