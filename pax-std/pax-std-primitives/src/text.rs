use std::cell::RefCell;
use std::ffi::CString;
use std::rc::Rc;

use piet::{RenderContext};

use pax_std::primitives::{Text};
use pax_core::{HandlerRegistry, InstantiationArgs, RenderNode, RenderNodePtr, RenderNodePtrList, RenderTreeContext};
use pax_core::pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};
use pax_message::TextPatch;
use pax_runtime_api::{PropertyInstance, Transform2D, Size2D, PropertyLiteral};

pub struct TextInstance {
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry>>>,
    pub instance_id: u64,
    pub properties: Rc<RefCell<Text>>,

    pub size: Size2D,
    pub transform: Rc<RefCell<dyn PropertyInstance<Transform2D>>>,

    //Used as a cache of last-sent values, for crude dirty-checking.
    //Hopefully, this will by obviated by the built-in expression dirty-checking mechanism.
    last_patches: pax_message::TextPatch,
}

impl<R: 'static + RenderContext>  RenderNode<R> for TextInstance {
    fn get_instance_id(&self) -> u64 {
        self.instance_id
    }

    fn instantiate(args: InstantiationArgs<R>) -> Rc<RefCell<Self>> where Self: Sized {
        let properties = if let PropertiesCoproduct::Text(p) = args.properties { p } else {unreachable!("Wrong properties type")};

        let mut instance_registry = (*args.instance_registry).borrow_mut();
        let instance_id = instance_registry.mint_id();
        let ret = Rc::new(RefCell::new(TextInstance {
            instance_id,
            transform: args.transform,
            properties: Rc::new(RefCell::new(properties)),
            size: Rc::new(RefCell::new(args.size.expect("Text requires a size"))),
            handler_registry: args.handler_registry,
            last_patches: Default::default()
        }));

        instance_registry.register(instance_id, Rc::clone(&ret) as RenderNodePtr<R>);
        ret
    }
    
    fn get_rendering_children(&self) -> RenderNodePtrList<R> {
        Rc::new(RefCell::new(vec![]))
    }
    fn get_size(&self) -> Option<Size2D> { Some(Rc::clone(&self.size)) }
    fn get_transform(&mut self) -> Rc<RefCell<dyn PropertyInstance<Transform2D>>> { Rc::clone(&self.transform) }

    fn compute_properties(&mut self, rtc: &mut RenderTreeContext<R>) {
        let mut new_message : TextPatch = Default::default();
        let mut has_any_updates = false;

        let mut properties = &mut *self.properties.as_ref().borrow_mut();
        if let Some(content) = rtc.compute_vtable_value(properties.content._get_vtable_id()) {
            let new_value = if let TypesCoproduct::String(v) = content { v } else { unreachable!() };
            new_message.content = pax_message::COption::Some(CString::new(new_value.clone()).unwrap()); //TODO: better error handling?
            has_any_updates = true;
            properties.content.set(new_value);
        }

        let mut size = &mut *self.size.as_ref().borrow_mut();

        if let Some(new_size) = rtc.compute_vtable_value(size[0]._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Size(v) = new_size { v } else { unreachable!() };

            has_any_updates = true;
            size[0].set(new_value);
        }

        if let Some(new_size) = rtc.compute_vtable_value(size[1]._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Size(v) = new_size { v } else { unreachable!() };

            has_any_updates = true;
            size[1].set(new_value);
        }

        let mut transform = &mut *self.transform.as_ref().borrow_mut();
        if let Some(new_transform) = rtc.compute_vtable_value(transform._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Transform2D(v) = new_transform { v } else { unreachable!() };

            has_any_updates = true;
            transform.set(new_value);
        }

        if has_any_updates {
            (*rtc.engine.runtime).borrow_mut().enqueue_native_message(
                pax_message::NativeMessage::TextUpdate(self.instance_id, new_message)
            );
        }

    }

    fn handle_render(&self, rtc: &mut RenderTreeContext<R>, rc: &mut R) {
        //no-op -- only native rendering for Text (unless/until we support rasterizing text, which Piet should be able to handle!)
    }

    fn handle_post_mount(&mut self, rtc: &mut RenderTreeContext<R>) {
        (*rtc.engine.runtime).borrow_mut().enqueue_native_message(
            pax_message::NativeMessage::TextCreate(self.instance_id)
        );
    }

    fn handle_pre_unmount(&mut self, rtc: &mut RenderTreeContext<R>) {
        (*rtc.engine.runtime).borrow_mut().enqueue_native_message(
            pax_message::NativeMessage::TextDelete(self.instance_id)
        );
    }
}
