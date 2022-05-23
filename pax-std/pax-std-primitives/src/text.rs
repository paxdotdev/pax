use std::cell::RefCell;
use std::ffi::CString;
use std::rc::Rc;
use std::collections::HashMap;

use piet::{RenderContext};

use pax_std::primitives::{Text};
use pax_core::{ComputableTransform, TabCache, HandlerRegistry, InstantiationArgs, RenderNode, RenderNodePtr, RenderNodePtrList, RenderTreeContext};
use pax_core::pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};
use pax_message::{AnyCreatePatch, TextPatch};
use pax_runtime_api::{PropertyInstance, Transform2D, Size2D, PropertyLiteral};

pub struct TextInstance<R: 'static + RenderContext> {
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry<R>>>>,
    pub instance_id: u64,
    pub properties: Rc<RefCell<Text>>,

    pub size: Size2D,
    pub transform: Rc<RefCell<dyn PropertyInstance<Transform2D>>>,



    //Used as a cache of last-sent values, for crude dirty-checking.
    //Hopefully, this will by obviated by the built-in expression dirty-checking mechanism.
    //Note: must build in awareness of id_chain, since each virtual instance if this single `Text` instance
    //      shares this last_patches cache
    last_patches: HashMap<Vec<u64>, pax_message::TextPatch>,
}

impl<R: 'static + RenderContext>  RenderNode<R> for TextInstance<R> {




    fn get_instance_id(&self) -> u64 {
        self.instance_id
    }

    fn instantiate(mut args: InstantiationArgs<R>) -> Rc<RefCell<Self>> where Self: Sized {
        let properties = if let PropertiesCoproduct::Text(p) = args.properties { p } else {unreachable!("Wrong properties type")};

        let mut instance_registry = (*args.instance_registry).borrow_mut();
        let instance_id = instance_registry.mint_id();
        let ret = Rc::new(RefCell::new(TextInstance {
            instance_id,
            transform: args.transform,
            properties: Rc::new(RefCell::new(properties)),
            size: Rc::new(RefCell::new(args.size.expect("Text requires a size"))),
            handler_registry: args.handler_registry,
            last_patches: Default::default(),

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


        let mut properties = &mut *self.properties.as_ref().borrow_mut();
        if let Some(content) = rtc.compute_vtable_value(properties.content._get_vtable_id()) {
            let new_value = if let TypesCoproduct::String(v) = content { v } else { unreachable!() };
            properties.content.set(new_value);
        }

        if let Some(size) = rtc.compute_vtable_value(properties.font.size._get_vtable_id()) {
            let new_value = if let TypesCoproduct::SizePixels(v) = size { v } else { unreachable!() };
            properties.font.size.set(new_value);
        }

        if let Some(family) = rtc.compute_vtable_value(properties.font.family._get_vtable_id()) {
            let new_value = if let TypesCoproduct::String(v) = family { v } else { unreachable!() };
            properties.font.family.set(new_value);
        }

        if let Some(variant) = rtc.compute_vtable_value(properties.font.variant._get_vtable_id()) {
            let new_value = if let TypesCoproduct::String(v) = variant { v } else { unreachable!() };
            properties.font.variant.set(new_value);
        }

        let mut size = &mut *self.size.as_ref().borrow_mut();

        if let Some(new_size) = rtc.compute_vtable_value(size[0]._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Size(v) = new_size { v } else { unreachable!() };

            size[0].set(new_value);
        }

        if let Some(new_size) = rtc.compute_vtable_value(size[1]._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Size(v) = new_size { v } else { unreachable!() };

            size[1].set(new_value);
        }

        let mut transform = &mut *self.transform.as_ref().borrow_mut();
        if let Some(new_transform) = rtc.compute_vtable_value(transform._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Transform2D(v) = new_transform { v } else { unreachable!() };

            transform.set(new_value);
        }



    }

    fn compute_native_patches(&mut self, rtc: &mut RenderTreeContext<R>, computed_size: (f64, f64), transform_coeffs: Vec<f64>) {

        let mut new_message : TextPatch = Default::default();
        new_message.id_chain = rtc.get_id_chain(self.instance_id);
        if ! self.last_patches.contains_key(&new_message.id_chain) {
            let mut patch = TextPatch::default();
            patch.id_chain = new_message.id_chain.clone();
            self.last_patches.insert(new_message.id_chain.clone(), patch);
        }
        let last_patch = self.last_patches.get_mut( &new_message.id_chain).unwrap();
        let mut has_any_updates = false;

        let mut properties = &mut *self.properties.as_ref().borrow_mut();
        let val = properties.content.get();
        let is_new_value = match &last_patch.content {
            Some(cached_value) => {
                !val.eq(cached_value)
            },
            None => {
                true
            },
        };
        if is_new_value {
            new_message.content = Some(val.clone());
            last_patch.content = Some(val.clone());
            has_any_updates = true;
        }

        let val = properties.font.size.get();
        let is_new_value = match &last_patch.font.size {
            Some(cached_value) => {
                !val.eq(cached_value)
            },
            None => {
                true
            },
        };
        if is_new_value {
            new_message.font.size = Some(val.into());
            last_patch.font.size = Some(val.into());
            has_any_updates = true;
        }

        let val = properties.font.family.get();
        let is_new_value = match &last_patch.font.family {
            Some(cached_value) => {
                !val.eq(cached_value)
            },
            None => {
                true
            },
        };
        if is_new_value {
            new_message.font.family = Some(val.into());
            last_patch.font.family = Some(val.into());
            has_any_updates = true;
        }

        let val = properties.font.variant.get();
        let is_new_value = match &last_patch.font.variant {
            Some(cached_value) => {
                !val.eq(cached_value)
            },
            None => {
                true
            },
        };
        if is_new_value {
            new_message.font.variant = Some(val.into());
            last_patch.font.variant = Some(val.into());
            has_any_updates = true;
        }


        let val = properties.fill.get();
        let is_new_value = match &last_patch.fill {
            Some(cached_value) => {
                !val.eq(cached_value)
            },
            None => {
                true
            },
        };
        if is_new_value {
            new_message.fill = Some(val.into());
            last_patch.fill = Some(val.into());
            has_any_updates = true;
        }

        let val = computed_size.0;
        let is_new_value = match &last_patch.size_x {
            Some(cached_value) => {
                !val.eq(cached_value)
            },
            None => {
                true
            },
        };
        if is_new_value {
            new_message.size_x = Some(val.clone());
            last_patch.size_x = Some(val.clone());
            has_any_updates = true;
        }

        let val = computed_size.1;
        let is_new_value = match &last_patch.size_y {
            Some(cached_value) => {
                !val.eq(cached_value)
            },
            None => {
                true
            },
        };
        if is_new_value {
            new_message.size_y = Some(val.clone());
            last_patch.size_y = Some(val.clone());
            has_any_updates = true;
        }

        let latest_transform = transform_coeffs;
        let is_new_transform = match &last_patch.transform {
            Some(cached_transform) => {
                latest_transform.iter().enumerate().any(|(i,elem)|{
                    *elem != cached_transform[i]
                })
            },
            None => {
                true
            },
        };
        if is_new_transform {
            new_message.transform = Some(latest_transform.clone());
            last_patch.transform = Some(latest_transform.clone());
            has_any_updates = true;
        }

        if has_any_updates {
            (*rtc.engine.runtime).borrow_mut().enqueue_native_message(
                pax_message::NativeMessage::TextUpdate(new_message)
            );
        }
    }

    fn handle_render(&self, rtc: &mut RenderTreeContext<R>, rc: &mut R) {
        //no-op -- only native rendering for Text (unless/until we support rasterizing text, which Piet should be able to handle!)
    }

    fn handle_post_mount(&mut self, rtc: &mut RenderTreeContext<R>) {

        let clipping_ids = rtc.runtime.borrow().get_current_clipping_ids();

        let id_chain = rtc.get_id_chain(self.instance_id);
        (*rtc.engine.runtime).borrow_mut().enqueue_native_message(
            pax_message::NativeMessage::TextCreate(AnyCreatePatch{
                id_chain,
                clipping_ids,
            })
        );
    }

    fn handle_pre_unmount(&mut self, rtc: &mut RenderTreeContext<R>) {

        // unplugged in desperation, search codebase for "unplugged in desperation"

        // let id_chain = rtc.get_id_chain(self.instance_id);
        // (*rtc.engine.runtime).borrow_mut().enqueue_native_message(
        //     pax_message::NativeMessage::TextDelete(id_chain)
        // );
    }
}
