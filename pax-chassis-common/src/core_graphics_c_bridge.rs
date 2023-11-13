#![allow(non_snake_case)] //Non-snake-case is used here to help denote foreign structs, e.g. from Swift via C

extern crate core;

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::rc::Rc;

use std::mem::{transmute, ManuallyDrop};
use std::os::raw::c_char;

use core_graphics::context::CGContext;
use piet_coregraphics::CoreGraphicsContext;

use flexbuffers;
use flexbuffers::DeserializationError;
use serde::Serialize;

use pax_cartridge;
use pax_core::{NodeRegistry, PaxEngine};

//Re-export all native message types; used by Swift via FFI.
//Note that any types exposed by pax_message must ALSO be added to `PaxCartridge.h`
//in order to be visible to Swift
pub use pax_message::*;
use pax_runtime_api::{ArgsClick, ArgsScroll, ModifierKey, MouseButton, MouseEventArgs};

/// Container data structure for PaxEngine, aggregated to support passing across C bridge
#[repr(C)] //Exposed to Swift via PaxCartridge.h
pub struct PaxEngineContainer {
    _engine: *mut PaxEngine<CoreGraphicsContext<'static>>,
    //NOTE: since that has become a single field, this data structure may be be retired and `*mut PaxEngine` could be passed directly.
}

/// Allocate an instance of the Pax engine, with a specified root/main component from the loaded `pax_cartridge`.
#[no_mangle] //Exposed to Swift via PaxCartridge.h
pub extern "C" fn pax_init(logger: extern "C" fn(*const c_char)) -> *mut PaxEngineContainer {
    //Initialize a ManuallyDrop-contained PaxEngine, so that a pointer to that
    //engine can be passed back to Swift via the C (FFI) bridge
    //This could presumably be cleaned up -- see `pax_dealloc_engine`
    let node_registry: Rc<RefCell<NodeRegistry<CoreGraphicsContext<'static>>>> =
        Rc::new(RefCell::new(NodeRegistry::new()));
    let main_component_instance =
        pax_cartridge::instantiate_main_component(Rc::clone(&node_registry));
    let expression_table = pax_cartridge::instantiate_expression_table();

    let engine: ManuallyDrop<Box<PaxEngine<CoreGraphicsContext<'static>>>> =
        ManuallyDrop::new(Box::new(PaxEngine::new(
            main_component_instance,
            expression_table,
            pax_runtime_api::PlatformSpecificLogger::MacOS(logger),
            (1.0, 1.0),
            node_registry,
        )));

    let container = ManuallyDrop::new(Box::new(PaxEngineContainer {
        _engine: Box::into_raw(ManuallyDrop::into_inner(engine)),
    }));

    Box::into_raw(ManuallyDrop::into_inner(container))
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
    let mut engine = unsafe { Box::from_raw((*engine_container)._engine) };
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

    let interrupt_wrapped: Result<NativeInterrupt, DeserializationError> =
        flexbuffers::from_slice(slice);
    let interrupt = interrupt_wrapped.unwrap();
    match interrupt {
        NativeInterrupt::Click(args) => {
            let prospective_hit = engine.get_topmost_element_beneath_ray((args.x, args.y));
            match prospective_hit {
                Some(topmost_node) => {
                    let modifiers = args
                        .modifiers
                        .iter()
                        .map(|x| ModifierKey::from(x))
                        .collect();
                    let args_click = ArgsClick {
                        mouse: MouseEventArgs {
                            x: args.x,
                            y: args.y,
                            button: MouseButton::from(args.button),
                            modifiers,
                        },
                    };
                    let tnb = topmost_node.borrow_mut();
                    tnb.dispatch_click(args_click);
                }
                _ => {}
            };
        }
        NativeInterrupt::Scroll(args) => {
            let prospective_hit = engine.get_focused_element();
            match prospective_hit {
                Some(topmost_node) => {
                    let args_scroll = ArgsScroll {
                        delta_x: args.delta_x,
                        delta_y: args.delta_y,
                    };
                    let tnb = topmost_node.borrow_mut();
                    tnb.dispatch_scroll(args_scroll);
                }
                _ => {}
            };
        }
        NativeInterrupt::Image(args) => match args {
            ImageLoadInterruptArgs::Reference(ref_args) => {
                let ptr = ref_args.image_data as *const u8;
                let slice = unsafe { std::slice::from_raw_parts(ptr, ref_args.image_data_length) };
                let owned_data: Vec<u8> = slice.to_vec();
                engine.load_image(
                    ref_args.id_chain,
                    owned_data,
                    ref_args.width,
                    ref_args.height,
                );
            }
            ImageLoadInterruptArgs::Data(_) => {}
        },
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
    let render_context = CoreGraphicsContext::new_y_up(ctx, height as f64, None);
    (*engine).set_viewport_size((width as f64, height as f64));

    let mut render_contexts = HashMap::new();
    render_contexts.insert(format!("{}", 0), render_context);

    let messages = (*engine).tick(&mut render_contexts);

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
