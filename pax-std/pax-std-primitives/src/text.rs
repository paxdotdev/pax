use std::cell::RefCell;
use std::rc::Rc;


use pax_core::{Property, RenderNode, RenderNodePtrList, RenderTreeContext, Size2D, Transform, HostPlatformContext};
use wasm_bindgen::prelude::*;

pub struct Text {
    pub content: Box<dyn Property<String>>,
    pub transform: Rc<RefCell<Transform>>,
    pub size: Size2D,
    pub id: String,
}




impl RenderNode for Text {
    fn get_rendering_children(&self) -> RenderNodePtrList {
        Rc::new(RefCell::new(vec![]))
    }
    fn get_size(&self) -> Option<Size2D> { Some(Rc::clone(&self.size)) }
    fn get_transform(&mut self) -> Rc<RefCell<Transform>> { Rc::clone(&self.transform) }
    fn compute_properties(&mut self, rtc: &mut RenderTreeContext) {

        self.size.borrow_mut().0.compute_in_place(rtc);
        self.size.borrow_mut().1.compute_in_place(rtc);
        self.transform.borrow_mut().compute_in_place(rtc);
    }
    fn render(&self, rtc: &mut RenderTreeContext, hpc: &mut HostPlatformContext) {
        //no-op -- all native for Text
    }


    fn handle_post_mount(&mut self, rtc: &mut RenderTreeContext<R>) {
        (*rtc.engine.runtime).borrow_mut().enqueue_native_message();
        // todo!(construct message and attach to native_render_queue)
    }


    fn handle_pre_unmount(&mut self, _rtc: &mut RenderTreeContext<R>) {
        todo!(construct message and attach to native_render_queue)
    }
}
