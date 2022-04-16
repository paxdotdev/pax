#[macro_use]
extern crate lazy_static;
extern crate mut_static;

use std::borrow::{Borrow, BorrowMut};
use std::cell::RefCell;
use std::os::raw::{c_char};
use std::ffi::{CString, CStr};
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::rc::Rc;
use lazy_static::lazy_static;
use std::sync::{Mutex, Arc};

use pax_cartridge_runtime;

use core_graphics::{
    context::CGContextRef,
};
use piet_coregraphics::{CoreGraphicsContext};

use pax_core::{ComponentInstance, InstanceMap, PaxEngine};

//hello world achieved with help from: https://mozilla.github.io/firefox-browser-architecture/experiments/2017-09-06-rust-on-ios.html
// as a backup plan, check out https://lib.rs/crates/swift-bridge

//TODO: expose `tick`, manage instance of Engine and Chassis,
//      accept CGContext and pass into CoreGraphicsRenderer
#[no_mangle]
pub extern fn pax_tick(container: *mut PaxChassisMacosBridgeContainer) {
    unsafe {
        let mut engine = Box::from_raw((*container)._engine);
        let mut render_context = Box::from_raw((*container)._render_context);
        engine.tick(&mut *render_context);
    }


}

// static mut ENGINE : PaxEngine<CoreGraphicsContext<'static>> = unsafe { *&0x0 } ;
// lazy_static! {
//     static ref ENGINE: *mut PaxEngine<CoreGraphicsContext<'static>> = Box::into_raw(Box::new_uninit());
// //     static ref ENGINE: Box<PaxEngine<CoreGraphicsContext<'static>>> = Box::;
//     // static ref ENGINE: MutStatic<PaxEngine> = MutStatic::new();
//     // static ref CONTEXT: MutStatic<CoreGraphicsContext<'static>> = MutStatic::new();
// }



#[repr(C)]
pub struct PaxChassisMacosBridgeContainer {
    _engine: *mut PaxEngine<CoreGraphicsContext<'static>>,
    _render_context: *mut CoreGraphicsContext<'static>,
}

#[no_mangle]
pub extern fn pax_init(cgContext: &'static mut CGContextRef) -> *mut PaxChassisMacosBridgeContainer {

    //Initialize a ManuallyDrop-contained PaxEngine, so that a pointer to that
    //engine can be passed back to Swift via the C (FFI) bridge
    //This could presumably be cleaned up but currently the engine will exist
    //on the heap for the lifetime of the containing process.

    let render_context : ManuallyDrop<Box<CoreGraphicsContext<'static>>> = ManuallyDrop::new(Box::new(CoreGraphicsContext::new_y_down(cgContext, None)));

    let instance_map : Rc<RefCell<InstanceMap<CoreGraphicsContext<'static>>>> = Rc::new(RefCell::new(std::collections::HashMap::new()));
    let root_component_instance = pax_cartridge_runtime::instantiate_root_component(Rc::clone(&instance_map));
    let expression_table = pax_cartridge_runtime::instantiate_expression_table();

    let engine : ManuallyDrop<Box<PaxEngine<CoreGraphicsContext<'static>>>> = ManuallyDrop::new(
        Box::new(
            PaxEngine::new(
                root_component_instance,
                expression_table,
                |msg:&str| {  }, //TODO
                (500.0, 500.0), //TODO
                Rc::new(RefCell::new(Default::default()))
            )
        )
    );

    let container = ManuallyDrop::new(Box::new(PaxChassisMacosBridgeContainer {
        _engine: Box::into_raw(ManuallyDrop::into_inner(engine)),
        _render_context: Box::into_raw(ManuallyDrop::into_inner(render_context))
    }));

    Box::into_raw(ManuallyDrop::into_inner(container))

}
