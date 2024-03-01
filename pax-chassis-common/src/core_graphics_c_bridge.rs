#![allow(non_snake_case)] //Non-snake-case is used here to help denote foreign structs, e.g. from Swift via C

extern crate core;

use std::cell::RefCell;
use std::ffi::c_void;

use std::mem::{transmute, ManuallyDrop};
use std::rc::Rc;

use core_graphics::context::CGContext;
use pax_runtime::api::math::Point2;
use piet_coregraphics::CoreGraphicsContext;

use flexbuffers;
use flexbuffers::DeserializationError;
use serde::Serialize;

use pax_cartridge;
use pax_runtime::{ExpressionTable, PaxEngine, Renderer};

//Re-export all native message types; used by Swift via FFI.
//Note that any types exposed by pax_message must ALSO be added to `PaxCartridge.h`
//in order to be visible to Swift
pub use pax_message::*;
use pax_runtime::api::{Click, ModifierKey, MouseButton, MouseEventArgs, RenderContext};

/// Container data structure for PaxEngine, aggregated to support passing across C bridge
#[repr(C)] //Exposed to Swift via PaxCartridge.h
pub struct PaxEngineContainer {
    _engine: *mut PaxEngine,
    //NOTE: since that has become a single field, this data structure may be be retired and `*mut PaxEngine` could be passed directly.
}

/// Allocate an instance of the Pax engine, with a specified root/main component from the loaded `pax_cartridge`.
#[no_mangle] //Exposed to Swift via PaxCartridge.h
pub extern "C" fn pax_init() -> *mut PaxEngineContainer {
    env_logger::init();

    let mut definition_to_instance_traverser =
        pax_cartridge::DefinitionToInstanceTraverser::new();
    let main_component_instance = definition_to_instance_traverser.get_main_component();
    let expression_table = ExpressionTable {
        table: pax_cartridge::instantiate_expression_table(),
    };


    //Initialize a ManuallyDrop-contained PaxEngine, so that a pointer to that
    //engine can be passed back to Swift via the C (FFI) bridge
    //This could presumably be cleaned up -- see `pax_dealloc_engine`
    let engine: ManuallyDrop<Box<PaxEngine>> = ManuallyDrop::new(Box::new(PaxEngine::new(
        main_component_instance,
        expression_table,
        (1.0, 1.0),
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

    let interrupt_wrapped: Result<NativeInterrupt, DeserializationError> =
        flexbuffers::from_slice(slice);
    let interrupt = interrupt_wrapped.unwrap();
    match interrupt {
        NativeInterrupt::Click(args) => {
            let prospective_hit = engine
                .runtime_context
                .get_topmost_element_beneath_ray(Point2::new(args.x, args.y));
            match prospective_hit {
                Some(topmost_node) => {
                    let modifiers = args
                        .modifiers
                        .iter()
                        .map(|x| ModifierKey::from(x))
                        .collect();
                    let args_click = Click {
                        mouse: MouseEventArgs {
                            x: args.x,
                            y: args.y,
                            button: MouseButton::from(args.button),
                            modifiers,
                        },
                    };
                    topmost_node.dispatch_click(
                        args_click,
                        engine.runtime_context.globals(),
                        &engine.runtime_context,
                    );
                }
                _ => {}
            };
        }
        NativeInterrupt::Scroll(_args) => {}
        NativeInterrupt::Image(args) => match args {
            ImageLoadInterruptArgs::Reference(_ref_args) => {
                // TODO this needs to be redone since image_map now lives in the
                // Renderer. Move renderer into the engine???
                // let ptr = ref_args.image_data as *const u8;
                // let slice = unsafe { std::slice::from_raw_parts(ptr, ref_args.image_data_length) };
                // let owned_data: Vec<u8> = slice.to_vec();
                // (&ref_args.path, owned_data, ref_args.width, ref_args.height);
                todo!();
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
    let mut render_context = Renderer::new();

    (*engine).set_viewport_size((width as f64, height as f64));
    render_context.add_context("0", CoreGraphicsContext::new_y_up(ctx, height as f64, None));

    let messages = (*engine).tick();
    engine.render(&mut render_context as &mut dyn RenderContext);

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
