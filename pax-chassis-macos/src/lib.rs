#![allow(non_snake_case)]

use std::rc::Rc;
use std::cell::RefCell;
use std::mem::ManuallyDrop;

use pax_cartridge_runtime;

use core_graphics::context::CGContext;
use piet_coregraphics::{CoreGraphicsContext};

use pax_core::{InstanceMap, PaxEngine};

#[repr(C)]
pub struct PaxChassisMacosBridgeContainer {
    _engine: *mut Rc<RefCell<PaxEngine<CoreGraphicsContext<'static>>>>,
}

#[no_mangle]
pub extern fn pax_init() -> *mut PaxChassisMacosBridgeContainer {

    //Initialize a ManuallyDrop-contained PaxEngine, so that a pointer to that
    //engine can be passed back to Swift via the C (FFI) bridge
    //This could presumably be cleaned up but currently the engine will exist
    //on the heap for the lifetime of the containing process.

    let instance_map : Rc<RefCell<InstanceMap<CoreGraphicsContext<'static>>>> = Rc::new(RefCell::new(std::collections::HashMap::new()));
    let root_component_instance = pax_cartridge_runtime::instantiate_root_component(Rc::clone(&instance_map));
    let expression_table = pax_cartridge_runtime::instantiate_expression_table();

    let engine : ManuallyDrop<Box<Rc<RefCell<PaxEngine<CoreGraphicsContext<'static>>>>>> = ManuallyDrop::new(
        Box::new(
            Rc::new(RefCell::new(PaxEngine::new(
                root_component_instance,
                expression_table,
                |msg:&str| {  }, //TODO
                (400.0, 400.0), //TODO
                Rc::new(RefCell::new(Default::default()))
            )))
        )
    );

    let container = ManuallyDrop::new(Box::new(PaxChassisMacosBridgeContainer {
        _engine: Box::into_raw(ManuallyDrop::into_inner(engine)),
    }));

    Box::into_raw(ManuallyDrop::into_inner(container))
}

#[no_mangle]
pub extern fn pax_tick(bridge_container: *mut PaxChassisMacosBridgeContainer, cgContext: *mut CGContext) {
    let engine = unsafe { Box::from_raw((*bridge_container)._engine) };
    let ctx = unsafe { &mut *cgContext };
    let mut render_context = CoreGraphicsContext::new_y_up(ctx, 400.0, None);
    if let Ok(mut engine_borrowed) = (**engine).try_borrow_mut() {
        engine_borrowed.tick(&mut render_context);
    };
    //This step is necessary to clean up engine, e.g. to drop all of the RefCell::borrow_mut's throughout
    unsafe {(*bridge_container)._engine=  Box::into_raw(engine)}

}
