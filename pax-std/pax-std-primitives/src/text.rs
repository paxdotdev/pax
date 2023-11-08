use std::cell::RefCell;

use pax_core::pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};
use pax_core::{
    unsafe_unwrap, HandlerRegistry, InstantiationArgs, PropertiesComputable, RenderNode,
    RenderNodePtr, RenderNodePtrList, RenderTreeContext,
};
use pax_message::{AnyCreatePatch, TextPatch};
use pax_runtime_api::{CommonProperties, Layer, SizePixels, StringBox};
use pax_std::primitives::Text;
use std::collections::HashMap;
use std::rc::Rc;
use pax_pixels::RenderContext;
use pax_std::types::text::{Font, TextAlignHorizontal, TextAlignVertical, TextStyle};

use pax_std::types::Color;

pub struct TextInstance<R: 'static + RenderContext> {
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry<R>>>>,
    pub instance_id: u32,
    pub properties: Rc<RefCell<Text>>,
    pub common_properties: CommonProperties,
    //Used as a cache of last-sent values, for crude dirty-checking.
    //Hopefully, this will by obviated by the built-in expression dirty-checking mechanism.
    //Note: must build in awareness of id_chain, since each virtual instance if this single `Text` instance
    //      shares this last_patches cache
    last_patches: HashMap<Vec<u32>, pax_message::TextPatch>,
}

impl<R: 'static + RenderContext> RenderNode<R> for TextInstance<R> {
    fn get_common_properties(&self) -> &CommonProperties {
        &self.common_properties
    }

    fn get_instance_id(&self) -> u32 {
        self.instance_id
    }

    fn instantiate(args: InstantiationArgs<R>) -> Rc<RefCell<Self>>
    where
        Self: Sized,
    {
        let properties = unsafe_unwrap!(args.properties, PropertiesCoproduct, Text);

        let mut instance_registry = (*args.instance_registry).borrow_mut();
        let instance_id = instance_registry.mint_id();
        let ret = Rc::new(RefCell::new(TextInstance {
            instance_id,
            properties: Rc::new(RefCell::new(properties)),
            common_properties: args.common_properties,
            handler_registry: args.handler_registry,
            last_patches: Default::default(),
        }));

        instance_registry.register(instance_id, Rc::clone(&ret) as RenderNodePtr<R>);
        ret
    }

    fn get_rendering_children(&self) -> RenderNodePtrList<R> {
        Rc::new(RefCell::new(vec![]))
    }
    fn compute_properties(&mut self, rtc: &mut RenderTreeContext<R>) {
        let properties = &mut *self.properties.as_ref().borrow_mut();

        if let Some(text) = rtc.compute_vtable_value(properties.text._get_vtable_id()) {
            let new_value = unsafe_unwrap!(text, TypesCoproduct, StringBox);
            properties.text.set(new_value);
        }

        if let Some(style_font) =
            rtc.compute_vtable_value(properties.style.get().font._get_vtable_id())
        {
            let new_value = unsafe_unwrap!(style_font, TypesCoproduct, Font);
            properties.style.get_mut().font.set(new_value);
        }

        if let Some(style_font_size) =
            rtc.compute_vtable_value(properties.style.get().font_size._get_vtable_id())
        {
            let new_value = unsafe_unwrap!(style_font_size, TypesCoproduct, SizePixels);
            properties.style.get_mut().font_size.set(new_value);
        }

        if let Some(style_fill) =
            rtc.compute_vtable_value(properties.style.get().fill._get_vtable_id())
        {
            let new_value = unsafe_unwrap!(style_fill, TypesCoproduct, Color);
            properties.style.get_mut().fill.set(new_value);
        }

        if let Some(style_underline) =
            rtc.compute_vtable_value(properties.style.get().underline._get_vtable_id())
        {
            let new_value = unsafe_unwrap!(style_underline, TypesCoproduct, bool);
            properties.style.get_mut().underline.set(new_value);
        }

        if let Some(style_align_multiline) =
            rtc.compute_vtable_value(properties.style.get().align_multiline._get_vtable_id())
        {
            let new_value =
                unsafe_unwrap!(style_align_multiline, TypesCoproduct, TextAlignHorizontal);
            properties.style.get_mut().align_multiline.set(new_value);
        }

        if let Some(style_align_vertical) =
            rtc.compute_vtable_value(properties.style.get().align_vertical._get_vtable_id())
        {
            let new_value = unsafe_unwrap!(style_align_vertical, TypesCoproduct, TextAlignVertical);
            properties.style.get_mut().align_vertical.set(new_value);
        }

        if let Some(style_align_horizontal) =
            rtc.compute_vtable_value(properties.style.get().align_horizontal._get_vtable_id())
        {
            let new_value =
                unsafe_unwrap!(style_align_horizontal, TypesCoproduct, TextAlignHorizontal);
            properties.style.get_mut().align_horizontal.set(new_value);
        }

        if let Some(style_link) = rtc.compute_vtable_value(properties.style_link._get_vtable_id()) {
            let new_value = unsafe_unwrap!(style_link, TypesCoproduct, TextStyle);
            properties.style_link.set(new_value);
        }

        let style_link = properties.style_link.get_mut();
        if let Some(style_font) = rtc.compute_vtable_value(style_link.font._get_vtable_id()) {
            let new_value = unsafe_unwrap!(style_font, TypesCoproduct, Font);
            style_link.font.set(new_value);
        }

        if let Some(style_font_size) =
            rtc.compute_vtable_value(style_link.font_size._get_vtable_id())
        {
            let new_value = unsafe_unwrap!(style_font_size, TypesCoproduct, SizePixels);
            style_link.font_size.set(new_value);
        }

        if let Some(style_fill) = rtc.compute_vtable_value(style_link.fill._get_vtable_id()) {
            let new_value = unsafe_unwrap!(style_fill, TypesCoproduct, Color);
            style_link.fill.set(new_value);
        }

        if let Some(style_underline) =
            rtc.compute_vtable_value(style_link.underline._get_vtable_id())
        {
            let new_value = unsafe_unwrap!(style_underline, TypesCoproduct, bool);
            style_link.underline.set(new_value);
        }

        if let Some(style_align_multiline) =
            rtc.compute_vtable_value(style_link.align_multiline._get_vtable_id())
        {
            let new_value =
                unsafe_unwrap!(style_align_multiline, TypesCoproduct, TextAlignHorizontal);
            style_link.align_multiline.set(new_value);
        }

        if let Some(style_align_vertical) =
            rtc.compute_vtable_value(style_link.align_vertical._get_vtable_id())
        {
            let new_value = unsafe_unwrap!(style_align_vertical, TypesCoproduct, TextAlignVertical);
            style_link.align_vertical.set(new_value);
        }

        if let Some(style_align_horizontal) =
            rtc.compute_vtable_value(style_link.align_horizontal._get_vtable_id())
        {
            let new_value =
                unsafe_unwrap!(style_align_horizontal, TypesCoproduct, TextAlignHorizontal);
            style_link.align_horizontal.set(new_value);
        }

        self.common_properties.compute_properties(rtc);
    }

    fn compute_native_patches(
        &mut self,
        rtc: &mut RenderTreeContext<R>,
        computed_size: (f64, f64),
        transform_coeffs: Vec<f64>,
        _z_index: u32,
        _subtree_depth: u32,
    ) {
        let mut new_message: TextPatch = Default::default();
        new_message.id_chain = rtc.get_id_chain(self.instance_id);
        if !self.last_patches.contains_key(&new_message.id_chain) {
            let mut patch = TextPatch::default();
            patch.id_chain = new_message.id_chain.clone();
            self.last_patches
                .insert(new_message.id_chain.clone(), patch);
        }
        let last_patch = self.last_patches.get_mut(&new_message.id_chain).unwrap();
        let mut has_any_updates = false;

        let properties = &mut *self.properties.as_ref().borrow_mut();

        let val = properties.text.get().string.clone();
        let is_new_value = match &last_patch.content {
            Some(cached_value) => !val.eq(cached_value),
            None => true,
        };
        if is_new_value {
            //pax_runtime_api::log(format!("Text update {:?}{:?}", new_message.id_chain, val.clone()).as_str());
            new_message.content = Some(val.clone());
            last_patch.content = Some(val.clone());
            has_any_updates = true;
        }

        let val = properties.style.get();
        let _is_new_val = match &last_patch.style {
            Some(cached_value) => !val.eq(cached_value),
            None => true,
        };

        if is_new_value {
            new_message.style = Some(val.into());
            last_patch.style = Some(val.into());
            has_any_updates = true;
        }

        let val = properties.style_link.get();
        let _is_new_val = match &last_patch.style_link {
            Some(cached_value) => !val.eq(cached_value),
            None => true,
        };

        if is_new_value {
            new_message.style_link = Some(val.into());
            last_patch.style_link = Some(val.into());
            has_any_updates = true;
        }

        let val = computed_size.0;
        let is_new_value = match &last_patch.size_x {
            Some(cached_value) => !val.eq(cached_value),
            None => true,
        };
        if is_new_value {
            new_message.size_x = Some(val);
            last_patch.size_x = Some(val);
            has_any_updates = true;
        }

        let val = computed_size.1;
        let is_new_value = match &last_patch.size_y {
            Some(cached_value) => !val.eq(cached_value),
            None => true,
        };
        if is_new_value {
            new_message.size_y = Some(val);
            last_patch.size_y = Some(val);
            has_any_updates = true;
        }

        let latest_transform = transform_coeffs;
        let is_new_transform = match &last_patch.transform {
            Some(cached_transform) => latest_transform
                .iter()
                .enumerate()
                .any(|(i, elem)| *elem != cached_transform[i]),
            None => true,
        };
        if is_new_transform {
            new_message.transform = Some(latest_transform.clone());
            last_patch.transform = Some(latest_transform.clone());
            has_any_updates = true;
        }

        if has_any_updates {
            (*rtc.engine.runtime)
                .borrow_mut()
                .enqueue_native_message(pax_message::NativeMessage::TextUpdate(new_message));
        }
    }

    fn handle_render(&mut self, _rtc: &mut RenderTreeContext<R>, _rc: &mut R) {
        //no-op -- only native rendering for Text (unless/until we support rasterizing text)
    }

    fn handle_did_mount(&mut self, rtc: &mut RenderTreeContext<R>, z_index: u32) {
        let id_chain = rtc.get_id_chain(self.instance_id);

        //though macOS and iOS don't need this ancestry chain for clipping, Web does
        let clipping_ids = (*rtc.runtime).borrow().get_current_clipping_ids();

        let scroller_ids = (*rtc.runtime).borrow().get_current_scroller_ids();

        (*rtc.engine.runtime).borrow_mut().enqueue_native_message(
            pax_message::NativeMessage::TextCreate(AnyCreatePatch {
                id_chain: id_chain.clone(),
                clipping_ids,
                scroller_ids,
                z_index,
            }),
        );
    }

    fn handle_will_unmount(&mut self, _rtc: &mut RenderTreeContext<R>) {
        let id_chain = _rtc.get_id_chain(self.instance_id);
        self.last_patches.remove(&id_chain);
        (*_rtc.engine.runtime)
            .borrow_mut()
            .enqueue_native_message(pax_message::NativeMessage::TextDelete(id_chain));
    }

    fn get_layer_type(&mut self) -> Layer {
        Layer::Native
    }
}
