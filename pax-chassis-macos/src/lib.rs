#![allow(non_snake_case)]
//Non-snake-case is used to help denote foreign structs, e.g. from Swift via C

use std::rc::Rc;
use std::cell::RefCell;
use std::mem::ManuallyDrop;
use std::os::raw::c_char;

use core_graphics::context::CGContext;
use piet_coregraphics::{CoreGraphicsContext};

use pax_core::{InstanceRegistry, PaxEngine};
use pax_cartridge;

#[repr(C)] //Exposed to Swift via paxchassismacos.h
pub struct PaxChassisMacosBridgeContainer {
    _engine: *mut PaxEngine<CoreGraphicsContext<'static>>,
}

#[no_mangle] //Exposed to Swift via paxchassismacos.h
pub extern "C" fn pax_init(logger: extern "C" fn(*const c_char)) -> *mut PaxChassisMacosBridgeContainer {

    //Initialize a ManuallyDrop-contained PaxEngine, so that a pointer to that
    //engine can be passed back to Swift via the C (FFI) bridge
    //This could presumably be cleaned up but for now the engine will exist
    //on the heap for the lifetime of the containing process.
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

    let container = ManuallyDrop::new(Box::new(PaxChassisMacosBridgeContainer {
        _engine: Box::into_raw(ManuallyDrop::into_inner(engine)),
    }));

    Box::into_raw(ManuallyDrop::into_inner(container))
}


#[no_mangle] //Exposed to Swift via paxchassismacos.h
pub extern fn pax_tick(bridge_container: *mut PaxChassisMacosBridgeContainer, cgContext: *mut CGContext, width: f32, height: f32) { // note that f32 is essentially `CFloat`, per: https://doc.rust-lang.org/std/os/raw/type.c_float.html
    let mut engine = unsafe { Box::from_raw((*bridge_container)._engine) };
    let ctx = unsafe { &mut *cgContext };
    let mut render_context = CoreGraphicsContext::new_y_up(ctx, height as f64, None);
    (*engine).set_viewport_size((width as f64, height as f64));
    (*engine).tick(&mut render_context);

    //`Box::into_raw` is our necessary manual clean-up for engine, e.g. to drop all of the RefCell::borrow_mut's throughout the tick lifecycle
    unsafe {(*bridge_container)._engine=  Box::into_raw(engine)}
}
