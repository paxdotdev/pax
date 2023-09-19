use core::option::Option;
use core::option::Option::Some;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use kurbo::BezPath;
use piet::RenderContext;

use pax_core::pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};
use pax_core::{
    unsafe_unwrap, CommonProperties, HandlerRegistry, InstantiationArgs, RenderNode, RenderNodePtr,
    RenderNodePtrList, RenderTreeContext,
};
use pax_message::{AnyCreatePatch, ScrollerPatch};
use pax_runtime_api::{ArgsScroll, EasingCurve, Layer, PropertyInstance, PropertyLiteral, Size};
use pax_std::primitives::Scroller;

/// A combination of a clipping area (nearly identical to a `Frame`,) and an
/// inner panel that can be scrolled on zero or more axes.  `Scroller` coordinates with each chassis to
/// create native scrolling containers, which pass native scroll events back to Engine.  In turn,
/// `Scroller` translates its children to reflect the current scroll position.
/// When both scrolling axes are disabled, `Scroller` acts exactly like a `Frame`, with a possibly-
/// transformed `Group` surrounding its contents.
pub struct ScrollerInstance<R: 'static + RenderContext> {
    pub instance_id: u32,
    pub children: RenderNodePtrList<R>,
    pub common_properties: CommonProperties,
    pub properties: Rc<RefCell<Scroller>>,
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry<R>>>>,
    pub scroll_x: f64,
    pub scroll_y: f64,
    pub scroll_x_offset: Rc<RefCell<dyn PropertyInstance<f64>>>,
    pub scroll_y_offset: Rc<RefCell<dyn PropertyInstance<f64>>>,
    last_patches: HashMap<Vec<u32>, ScrollerPatch>,
}

impl<R: 'static + RenderContext> RenderNode<R> for ScrollerInstance<R> {
    fn get_common_properties(&self) -> &CommonProperties {
        &self.common_properties
    }

    fn get_instance_id(&self) -> u32 {
        self.instance_id
    }

    fn get_layer_type(&mut self) -> Layer {
        Layer::Scroller
    }

    fn instantiate(args: InstantiationArgs<R>) -> Rc<RefCell<Self>>
    where
        Self: Sized,
    {
        //      instantiate a `Group`, store it as a private field on the instance struct; attach the provided
        //      children (here, in Inst.Args) to that `Group`.  Finally, `set` the `transform` of that Group to
        //      update the `translation` mandated by scroll events.
        let properties = unsafe_unwrap!(args.properties, PropertiesCoproduct, Scroller);

        let mut instance_registry = args.instance_registry.borrow_mut();
        let instance_id = instance_registry.mint_id();

        let ret = Rc::new(RefCell::new(Self {
            instance_id,
            children: args
                .children
                .expect("Scroller expects primitive_children, even if empty Vec"),
            common_properties: args.common_properties,
            properties: Rc::new(RefCell::new(properties)),
            last_patches: HashMap::new(),
            handler_registry: args.handler_registry,
            scroll_x: 0.0,
            scroll_y: 0.0,
            scroll_x_offset: Rc::new(RefCell::new(PropertyLiteral::new(0.0))),
            scroll_y_offset: Rc::new(RefCell::new(PropertyLiteral::new(0.0))),
        }));

        instance_registry.register(instance_id, Rc::clone(&ret) as RenderNodePtr<R>);
        ret
    }

    fn handle_scroll(&mut self, args_scroll: ArgsScroll) {
        self.scroll_x -= args_scroll.delta_x;
        self.scroll_y -= args_scroll.delta_y;
        (*self.scroll_x_offset)
            .borrow_mut()
            .ease_to(self.scroll_x, 2, EasingCurve::Linear);
        (*self.scroll_y_offset)
            .borrow_mut()
            .ease_to(self.scroll_y, 2, EasingCurve::Linear);
    }

    fn get_scroll_offset(&mut self) -> (f64, f64) {
        (
            (*self.scroll_x_offset).borrow().get().clone(),
            (*self.scroll_y_offset).borrow().get().clone(),
        )
    }

    fn get_handler_registry(&self) -> Option<Rc<RefCell<HandlerRegistry<R>>>> {
        match &self.handler_registry {
            Some(registry) => Some(Rc::clone(&registry)),
            _ => None,
        }
    }

    fn compute_native_patches(
        &mut self,
        rtc: &mut RenderTreeContext<R>,
        computed_size: (f64, f64),
        transform_coeffs: Vec<f64>,
        _z_index: u32,
        subtree_depth: u32,
    ) {
        let mut new_message: ScrollerPatch = Default::default();
        new_message.id_chain = rtc.get_id_chain(self.instance_id);
        if !self.last_patches.contains_key(&new_message.id_chain) {
            let mut patch = ScrollerPatch::default();
            patch.id_chain = new_message.id_chain.clone();
            self.last_patches
                .insert(new_message.id_chain.clone(), patch);
        }
        let last_patch = self.last_patches.get_mut(&new_message.id_chain).unwrap();
        let mut has_any_updates = false;

        let properties = &mut *self.properties.as_ref().borrow_mut();

        let val = subtree_depth;
        let is_new_value = last_patch.subtree_depth != val;
        if is_new_value {
            new_message.subtree_depth = val;
            last_patch.subtree_depth = val;
            has_any_updates = true;
        }

        let val = computed_size.0;
        let is_new_value = match &last_patch.size_x {
            Some(cached_value) => !val.eq(cached_value),
            None => true,
        };
        if is_new_value {
            new_message.size_x = Some(val.clone());
            last_patch.size_x = Some(val.clone());
            has_any_updates = true;
        }

        let val = computed_size.1;
        let is_new_value = match &last_patch.size_y {
            Some(cached_value) => !val.eq(cached_value),
            None => true,
        };
        if is_new_value {
            new_message.size_y = Some(val.clone());
            last_patch.size_y = Some(val.clone());
            has_any_updates = true;
        }

        let val = Size::get_pixels(properties.size_inner_pane_x.get(), computed_size.0);
        let is_new_value = match &last_patch.size_inner_pane_x {
            Some(cached_value) => !val.eq(cached_value),
            None => true,
        };
        if is_new_value {
            new_message.size_inner_pane_x = Some(val.clone());
            last_patch.size_inner_pane_x = Some(val.clone());
            has_any_updates = true;
        }

        let val = Size::get_pixels(properties.size_inner_pane_y.get(), computed_size.1);
        let is_new_value = match &last_patch.size_inner_pane_y {
            Some(cached_value) => !val.eq(cached_value),
            None => true,
        };
        if is_new_value {
            new_message.size_inner_pane_y = Some(val.clone());
            last_patch.size_inner_pane_y = Some(val.clone());
            has_any_updates = true;
        }

        let val = properties.scroll_enabled_x.get();
        let is_new_value = match &last_patch.scroll_x {
            Some(cached_value) => !val.eq(cached_value),
            None => true,
        };
        if is_new_value {
            new_message.scroll_x = Some(val.clone());
            last_patch.scroll_x = Some(val.clone());
            has_any_updates = true;
        }

        let val = properties.scroll_enabled_y.get();
        let is_new_value = match &last_patch.scroll_y {
            Some(cached_value) => !val.eq(cached_value),
            None => true,
        };
        if is_new_value {
            new_message.scroll_y = Some(val.clone());
            last_patch.scroll_y = Some(val.clone());
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
                .enqueue_native_message(pax_message::NativeMessage::ScrollerUpdate(new_message));
        }
    }

    fn get_rendering_children(&self) -> RenderNodePtrList<R> {
        Rc::clone(&self.children)
    }

    fn get_clipping_bounds(&self) -> Option<(Size, Size)> {
        Some((
            self.common_properties
                .width
                .as_ref()
                .unwrap()
                .borrow()
                .get()
                .clone(),
            self.common_properties
                .height
                .as_ref()
                .unwrap()
                .borrow()
                .get()
                .clone(),
        ))
    }

    fn get_size(&self) -> Option<(Size, Size)> {
        Some((
            self.properties
                .as_ref()
                .borrow()
                .size_inner_pane_x
                .get()
                .clone(),
            self.properties
                .as_ref()
                .borrow()
                .size_inner_pane_y
                .get()
                .clone(),
        ))
    }

    fn compute_properties(&mut self, rtc: &mut RenderTreeContext<R>) {
        let mut scroll_x_offset_borrowed = (*self.scroll_x_offset).borrow_mut();
        if let Some(new_value) =
            rtc.compute_eased_value(scroll_x_offset_borrowed._get_transition_manager())
        {
            scroll_x_offset_borrowed.set(new_value);
        }

        let mut scroll_y_offset_borrowed = (*self.scroll_y_offset).borrow_mut();
        if let Some(new_value) =
            rtc.compute_eased_value(scroll_y_offset_borrowed._get_transition_manager())
        {
            scroll_y_offset_borrowed.set(new_value);
        }

        let properties = &mut *self.properties.as_ref().borrow_mut();

        let width = &mut *self.common_properties.width.as_ref().unwrap().borrow_mut();

        if let Some(new_size) = rtc.compute_vtable_value(width._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Size(v) = new_size {
                v
            } else {
                unreachable!()
            };
            width.set(new_value);
        }

        let height = &mut *self.common_properties.height.as_ref().unwrap().borrow_mut();
        if let Some(new_size) = rtc.compute_vtable_value(height._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Size(v) = new_size {
                v
            } else {
                unreachable!()
            };
            height.set(new_value);
        }

        if let Some(new_size) =
            rtc.compute_vtable_value(properties.size_inner_pane_x._get_vtable_id())
        {
            let new_value = if let TypesCoproduct::Size(v) = new_size {
                v
            } else {
                unreachable!()
            };
            properties.size_inner_pane_x.set(new_value);
        }

        if let Some(new_size) =
            rtc.compute_vtable_value(properties.size_inner_pane_y._get_vtable_id())
        {
            let new_value = if let TypesCoproduct::Size(v) = new_size {
                v
            } else {
                unreachable!()
            };
            properties.size_inner_pane_y.set(new_value);
        }

        if let Some(scroll_enabled_x) =
            rtc.compute_vtable_value(properties.scroll_enabled_x._get_vtable_id())
        {
            let new_value = if let TypesCoproduct::bool(v) = scroll_enabled_x {
                v
            } else {
                unreachable!()
            };
            properties.scroll_enabled_x.set(new_value);
        }

        if let Some(scroll_enabled_y) =
            rtc.compute_vtable_value(properties.scroll_enabled_y._get_vtable_id())
        {
            let new_value = if let TypesCoproduct::bool(v) = scroll_enabled_y {
                v
            } else {
                unreachable!()
            };
            properties.scroll_enabled_y.set(new_value);
        }

        let transform = &mut *self.common_properties.transform.as_ref().borrow_mut();
        if let Some(new_transform) = rtc.compute_vtable_value(transform._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Transform2D(v) = new_transform {
                v
            } else {
                unreachable!()
            };
            transform.set(new_value);
        }
    }

    fn handle_will_render(&mut self, rtc: &mut RenderTreeContext<R>, rcs: &mut HashMap<String, R>) {
        let transform = rtc.transform_global;
        let bounding_dimens = rtc.bounds;

        let width: f64 = bounding_dimens.0;
        let height: f64 = bounding_dimens.1;

        let mut bez_path = BezPath::new();
        bez_path.move_to((0.0, 0.0));
        bez_path.line_to((width, 0.0));
        bez_path.line_to((width, height));
        bez_path.line_to((0.0, height));
        bez_path.line_to((0.0, 0.0));
        bez_path.close_path();

        let transformed_bez_path = transform * bez_path;
        for (_key, rc) in rcs.iter_mut() {
            rc.save().unwrap(); //our "save point" before clipping — restored to in the did_render
            rc.clip(transformed_bez_path.clone());
        }
        let id_chain = rtc.get_id_chain(self.instance_id);
        (*rtc.runtime)
            .borrow_mut()
            .push_clipping_stack_id(id_chain.clone());
        (*rtc.runtime)
            .borrow_mut()
            .push_scroller_stack_id(id_chain.clone());
    }
    fn handle_did_render(&mut self, rtc: &mut RenderTreeContext<R>, _rcs: &mut HashMap<String, R>) {
        for (_key, rc) in _rcs.iter_mut() {
            //pop the clipping context from the stack
            rc.restore().unwrap();
        }

        (*rtc.runtime).borrow_mut().pop_clipping_stack_id();
        (*rtc.runtime).borrow_mut().pop_scroller_stack_id();
    }

    fn handle_did_mount(&mut self, rtc: &mut RenderTreeContext<R>, z_index: u32) {
        let id_chain = rtc.get_id_chain(self.instance_id);

        //though macOS and iOS don't need this ancestry chain for clipping, Web does
        let clipping_ids = (*rtc.runtime).borrow().get_current_clipping_ids();

        let scroller_ids = (*rtc.runtime).borrow().get_current_scroller_ids();

        (*rtc.engine.runtime).borrow_mut().enqueue_native_message(
            pax_message::NativeMessage::ScrollerCreate(AnyCreatePatch {
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
            .enqueue_native_message(pax_message::NativeMessage::ScrollerDelete(id_chain));
    }
}
