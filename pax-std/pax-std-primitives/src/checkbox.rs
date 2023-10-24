use std::cell::RefCell;

use pax_core::form_event::FormEvent;
use pax_core::pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};
use pax_core::{
    unsafe_unwrap, HandlerRegistry, InstantiationArgs, PropertiesComputable, RenderNode,
    RenderNodePtr, RenderNodePtrList, RenderTreeContext,
};
use pax_message::{AnyCreatePatch, CheckboxPatch};
use pax_runtime_api::{CommonProperties, Layer};
use pax_std::primitives::Checkbox;
use piet::RenderContext;
use std::collections::HashMap;
use std::rc::Rc;

pub struct CheckboxInstance<R: 'static + RenderContext> {
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry<R>>>>,
    pub instance_id: u32,
    pub properties: Rc<RefCell<Checkbox>>,
    pub common_properties: CommonProperties,
    //Used as a cache of last-sent values, for crude dirty-checking.
    //Hopefully, this will by obviated by the built-in expression dirty-checking mechanism.
    //Note: must build in awareness of id_chain, since each virtual instance if this single `Text` instance
    //      shares this last_patches cache
    last_patches: HashMap<Vec<u32>, pax_message::CheckboxPatch>,
}

impl<R: 'static + RenderContext> RenderNode<R> for CheckboxInstance<R> {
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
        let properties = unsafe_unwrap!(args.properties, PropertiesCoproduct, Checkbox);

        let mut instance_registry = (*args.instance_registry).borrow_mut();
        let instance_id = instance_registry.mint_id();
        let ret = Rc::new(RefCell::new(CheckboxInstance {
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

        if let Some(checked) =
            rtc.compute_vtable_value(properties.checked._get_vtable_id())
        {
            let new_value = unsafe_unwrap!(checked, TypesCoproduct, bool);
            properties.checked.set(new_value);
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
        
        let id_chain = rtc.get_id_chain(self.instance_id);
        let mut patch = CheckboxPatch {id_chain: id_chain.clone(), ..Default::default()};
        let old_state = self.last_patches.entry(id_chain.clone()).or_insert(
            CheckboxPatch {id_chain, ..Default::default()}
        );
        let properties = &mut *self.properties.as_ref().borrow_mut();
        let update_needed = 
            crate::patch_if_needed(&mut old_state.checked, &mut patch.checked, *properties.checked.get()) ||
            crate::patch_if_needed(&mut old_state.size_x, &mut patch.size_x, computed_size.0) ||
            crate::patch_if_needed(&mut old_state.size_y, &mut patch.size_y, computed_size.1) ||
            crate::patch_if_needed(&mut old_state.transform, &mut patch.transform, transform_coeffs);
        if update_needed {
            (*rtc.engine.runtime)
                .borrow_mut()
                .enqueue_native_message(pax_message::NativeMessage::CheckboxUpdate(patch));
        }
    }

    fn handle_render(&mut self, _rtc: &mut RenderTreeContext<R>, _rc: &mut R) {
        //no-op -- only native rendering for Text (unless/until we support rasterizing text, which Piet should be able to handle!)
    }

    fn handle_did_mount(&mut self, rtc: &mut RenderTreeContext<R>, z_index: u32) {
        let id_chain = rtc.get_id_chain(self.instance_id);

        //though macOS and iOS don't need this ancestry chain for clipping, Web does
        let clipping_ids = (*rtc.runtime).borrow().get_current_clipping_ids();

        let scroller_ids = (*rtc.runtime).borrow().get_current_scroller_ids();

        (*rtc.engine.runtime).borrow_mut().enqueue_native_message(
            pax_message::NativeMessage::CheckboxCreate(AnyCreatePatch {
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
            .enqueue_native_message(pax_message::NativeMessage::CheckboxDelete(id_chain));
    }

    fn get_layer_type(&mut self) -> Layer {
        Layer::Native
    }

    fn handle_form_event(&mut self, event: FormEvent) {
        match event {
            FormEvent::Toggle { state } => self.properties.borrow_mut().checked.set(state),
            _ => panic!("checkbox received non-compatible form event: {:?}", event)
        }
    }
}
