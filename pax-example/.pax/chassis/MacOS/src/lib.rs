#![allow(non_snake_case)] //Non-snake-case is used here to help denote foreign structs, e.g. from Swift via C

extern crate core;

use std::rc::Rc;
use std::cell::RefCell;
use std::ffi::c_void;
use std::mem::{ManuallyDrop, transmute};
use std::os::raw::{c_char};

use core_graphics::context::CGContext;
use piet_coregraphics::{CoreGraphicsContext};

use serde::Serialize;
use flexbuffers;
use flexbuffers::{Buffer, DeserializationError, Reader};

use pax_core::{InstanceRegistry, PaxEngine};
use pax_cartridge;

//Re-export all native message types; used by Swift via FFI.
//Note that any types exposed by pax_message must ALSO be added to `paxchassismacos.h`
//in order to be visible to Swift
pub use pax_message::*;
use pax_runtime_api::ArgsClick;

/// Container data structure for PaxEngine, aggregated to support passing across C bridge
#[repr(C)] //Exposed to Swift via paxchassismacos.h
pub struct PaxEngineContainer {
    _engine: *mut PaxEngine<CoreGraphicsContext<'static>>,
    //TODO: since that has become a single field, this data structure should be retired and `*mut PaxEngine` should be passed directly.
}

/// Allocate an instance of the Pax engine, with a specified root component from the loaded `pax_cartridge`.
#[no_mangle] //Exposed to Swift via paxchassismacos.h
pub extern "C" fn pax_init(logger: extern "C" fn(*const c_char)) -> *mut PaxEngineContainer {

    //Initialize a ManuallyDrop-contained PaxEngine, so that a pointer to that
    //engine can be passed back to Swift via the C (FFI) bridge
    //This could presumably be cleaned up -- see `pax_dealloc_engine`
    let instance_registry : Rc<RefCell<InstanceRegistry<CoreGraphicsContext<'static>>>> = Rc::new(RefCell::new(InstanceRegistry::new()));
    let root_component_instance = pax_cartridge::instantiate_root_component(Rc::clone(&instance_registry));
    let expression_table = pax_cartridge::instantiate_expression_table();

    let engine : ManuallyDrop<Box<PaxEngine<CoreGraphicsContext<'static>>>> = ManuallyDrop::new(
        Box::new(
           PaxEngine::new(
               root_component_instance,
               expression_table,
               pax_runtime_api::PlatformSpecificLogger::MacOS(logger),
               (1.0, 1.0),
               instance_registry,
           )
        )
    );

    let container = ManuallyDrop::new(Box::new(PaxEngineContainer {
        _engine: Box::into_raw(ManuallyDrop::into_inner(engine)),
    }));

    Box::into_raw(ManuallyDrop::into_inner(container))
}

/// Destroy `engine` and clean up the `ManuallyDrop` container surround it.
#[no_mangle]
pub extern "C" fn pax_dealloc_engine(_container: *mut PaxEngineContainer) {
    //TODO: support deallocing the engine container, particularly for when we need to support elegant clean-up from attached harness
}


/// Send `interrupt`s from the chassis, for example: user input
/// Note that in any single-threaded environemnt, these interrupts will happen
/// synchronously between engine ticks, allowing for safe unwrapping / borrowing
/// of engine and runtime here.  In short: this happens between ticks.
#[no_mangle]
pub extern "C" fn pax_interrupt(engine_container: *mut PaxEngineContainer, buffer: *const InterruptBuffer) {
    let mut engine = unsafe { Box::from_raw((*engine_container)._engine) };
    // let slice = unsafe { buffer.as_ref().unwrap() };

    let length : u64 = unsafe {
        (*buffer).length
            .try_into()
            .unwrap() // length negative or overflowed
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
    match interrupt {
        NativeInterrupt::Click(args) => {
            let prospective_hit = engine.get_topmost_hydrated_element_beneath_ray((args.x, args.y));
            match prospective_hit {
                Some(topmost_node) => {
                    let args_click = ArgsClick {x: args.x , y: args.y};
                    topmost_node.dispatch_click(args_click);
                },
                _ => {},
            };
        }
        NativeInterrupt::Scroll(_) => {}
    }

    unsafe {(*engine_container)._engine=  Box::into_raw(engine)};
}

/// Perform full tick of engine, including property computation, lifecycle event handling, and rendering side-effects.
/// Returns a message queue of native rendering actions encoded as a Flexbuffer via FFI to Swift.
/// The returned message queue requires explicit deallocation: `pax_deallocate_message_queue`
#[no_mangle] //Exposed to Swift via paxchassismacos.h
pub extern "C" fn pax_tick(engine_container: *mut PaxEngineContainer, cgContext: *mut c_void, width: f32, height: f32) -> *mut NativeMessageQueue { // note that f32 is essentially `CFloat`, per: https://doc.rust-lang.org/std/os/raw/type.c_float.html
    let mut engine = unsafe { Box::from_raw((*engine_container)._engine) };

    let pre_cast_cgContext = cgContext as *mut CGContext;
    let ctx = unsafe { &mut *pre_cast_cgContext };
    let mut render_context = CoreGraphicsContext::new_y_up(ctx, height as f64, None);
    (*engine).set_viewport_size((width as f64, height as f64));

    let messages = (*engine).tick(&mut render_context);

    let wrapped_queue = MessageQueue{messages,};
    let mut serializer = flexbuffers::FlexbufferSerializer::new();

    //side-effectfully serialize, mutating `serializer`
    wrapped_queue.serialize(&mut serializer).unwrap();

    let data_buffer = serializer.take_buffer();
    let length = data_buffer.len();

    let leaked_data : ManuallyDrop<Box<[u8]>> = ManuallyDrop::new(data_buffer.into_boxed_slice());

    let queue_container  = unsafe{ transmute(Box::new(NativeMessageQueue {
        data_ptr: Box::into_raw(ManuallyDrop::into_inner(leaked_data)),
        length: length as u64,
    }))};
    
    //`Box::into_raw` is our necessary manual clean-up, acting as a trigger to drop all of the RefCell::borrow_mut's throughout the tick lifecycle
    unsafe {(*engine_container)._engine=  Box::into_raw(engine)};

    queue_container
}

/// Required manual cleanup callback from Swift after reading a frame's message queue.
/// If this is not called after `pax_tick` is invoked, we will have a memory leak.
#[no_mangle] //Exposed to Swift via paxchassismacos.h
pub extern "C" fn pax_dealloc_message_queue(queue: *mut NativeMessageQueue)  {

    unsafe {
        let queue_container = Box::from_raw(queue);
        let data_buffer = Box::from_raw(queue_container.data_ptr);
        drop(data_buffer);
        drop(queue_container);
    }

}
