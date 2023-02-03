use core::option::Option;
use core::option::Option::Some;
use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;
use std::borrow::Borrow;

use kurbo::BezPath;
use piet::RenderContext;

use pax_core::{RenderNode, TabCache, RenderNodePtrList, RenderTreeContext, RenderNodePtr, InstantiationArgs, HandlerRegistry};
use pax_core::pax_properties_coproduct::TypesCoproduct;
use pax_runtime_api::{Transform2D, Size, PropertyInstance, PropertyLiteral, Size2D};
use pax_message::{AnyCreatePatch, ScrollerPatch};

/// A combination of a clipping area (nearly identical to a `Frame`,) and an
/// inner panel that can be scrolled on zero or more axes.  `Scroller` coordinates with each chassis to
/// create native scrolling containers, which pass native scroll events back to Engine.  In turn,
/// `Scroller` translates its children to reflect the current scroll position.
/// When both scrolling axes are disabled, `Scroller` acts exactly like a `Frame`, with a possibly-
/// transformed `Group` surrounding its contents.
pub struct ScrollerInstance<R: 'static + RenderContext> {
    pub instance_id: u64,
    pub children: RenderNodePtrList<R>,
    //Note that size_inner_pane must be a float value -- no percentages supported
    //because the inner pane is independent of any container, thus must have a concrete pixel size
    pub size_inner_pane: Rc<RefCell<[Box<dyn PropertyInstance<f64>>;2]>>,
    pub size_frame: Size2D,
    pub transform: Rc<RefCell<dyn PropertyInstance<Transform2D>>>,
    pub scroll_enabled_x: Box<dyn PropertyInstance<bool>>,
    pub scroll_enabled_y: Box<dyn PropertyInstance<bool>>,

    last_patches: HashMap<Vec<u64>, ScrollerPatch>,
}

impl<R: 'static + RenderContext> RenderNode<R> for ScrollerInstance<R> {

    fn get_instance_id(&self) -> u64 {
        self.instance_id
    }

    fn instantiate(mut args: InstantiationArgs<R>) -> Rc<RefCell<Self>> where Self: Sized {

        //      instantiate a `Group`, store it as a private field on the instance struct; attach the provided
        //      children (here, in Inst.Args) to that `Group`.  Finally, `set` the `transform` of that Group to
        //      update the `translation` mandated by scroll events.

        let mut scroller_args = args.scroller_args.unwrap(); // Scroller args required
        let mut size_inner_pane = scroller_args.size_inner_pane;
        let mut axes_enabled = scroller_args.axes_enabled;
        let scroll_enabled_x = std::mem::replace(&mut axes_enabled[0], Box::new(PropertyLiteral::new(false)));
        let scroll_enabled_y = std::mem::replace(&mut axes_enabled[1], Box::new(PropertyLiteral::new(false)));

        let mut instance_registry = args.instance_registry.borrow_mut();
        let instance_id = instance_registry.mint_id();
        let ret = Rc::new(RefCell::new(
            Self {
                instance_id,
                children: args.children.expect("Scroller expects primitive_children, even if empty Vec"),
                size_inner_pane: Rc::new(RefCell::new(size_inner_pane)),
                size_frame: args.size.expect("Scroller requires size_frame"),
                transform: args.transform,
                scroll_enabled_x,
                scroll_enabled_y,
                last_patches: HashMap::new(),

            }
        ));

        instance_registry.register(instance_id, Rc::clone(&ret) as RenderNodePtr<R>);
        ret
    }

    fn is_clipping(&self) -> bool {
        true
    }

    fn compute_native_patches(&mut self, rtc: &mut RenderTreeContext<R>, computed_size: (f64, f64), transform_coeffs: Vec<f64>) {

        let mut new_message : ScrollerPatch = Default::default();
        new_message.id_chain = rtc.get_id_chain(self.instance_id);
        if ! self.last_patches.contains_key(&new_message.id_chain) {
            let mut patch = ScrollerPatch::default();
            patch.id_chain = new_message.id_chain.clone();
            self.last_patches.insert(new_message.id_chain.clone(),patch);
        }
        let last_patch = self.last_patches.get_mut( &new_message.id_chain).unwrap();
        let mut has_any_updates = false;

        let val = computed_size.0;
        let is_new_value = match &last_patch.size_frame_x {
            Some(cached_value) => {
                !val.eq(cached_value)
            },
            None => {
                true
            },
        };
        if is_new_value {
            new_message.size_frame_x = Some(val.clone());
            last_patch.size_frame_x = Some(val.clone());
            has_any_updates = true;
        }

        let val = computed_size.1;
        let is_new_value = match &last_patch.size_frame_y {
            Some(cached_value) => {
                !val.eq(cached_value)
            },
            None => {
                true
            },
        };
        if is_new_value {
            new_message.size_frame_y = Some(val.clone());
            last_patch.size_frame_y = Some(val.clone());
            has_any_updates = true;
        }

        let val = *(*self.size_inner_pane).borrow()[0].get();
        let is_new_value = match &last_patch.size_inner_pane_x {
            Some(cached_value) => {
                !val.eq(cached_value)
            },
            None => {
                true
            },
        };
        if is_new_value {
            new_message.size_inner_pane_x = Some(val.clone());
            last_patch.size_inner_pane_x = Some(val.clone());
            has_any_updates = true;
        }

        let val = *(*self.size_inner_pane).borrow()[1].get();
        let is_new_value = match &last_patch.size_inner_pane_y {
            Some(cached_value) => {
                !val.eq(cached_value)
            },
            None => {
                true
            },
        };
        if is_new_value {
            new_message.size_inner_pane_y = Some(val.clone());
            last_patch.size_inner_pane_y = Some(val.clone());
            has_any_updates = true;
        }

        let val = *(*self.scroll_enabled_x).borrow().get();
        let is_new_value = match &last_patch.scroll_x {
            Some(cached_value) => {
                !val.eq(cached_value)
            },
            None => {
                true
            },
        };
        if is_new_value {
            new_message.scroll_x = Some(val.clone());
            last_patch.scroll_x = Some(val.clone());
            has_any_updates = true;
        }

        let val = *(*self.scroll_enabled_y).borrow().get();
        let is_new_value = match &last_patch.scroll_y {
            Some(cached_value) => {
                !val.eq(cached_value)
            },
            None => {
                true
            },
        };
        if is_new_value {
            new_message.scroll_y = Some(val.clone());
            last_patch.scroll_y = Some(val.clone());
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
                pax_message::NativeMessage::ScrollerUpdate(new_message)
            );
        }
    }

    fn get_rendering_children(&self) -> RenderNodePtrList<R> {
        Rc::clone(&self.children)
    }

    fn get_size(&self) -> Option<Size2D> {
        Some(Rc::clone(&self.size_frame))
    }

    fn get_transform(&mut self) -> Rc<RefCell<dyn PropertyInstance<Transform2D>>> { Rc::clone(&self.transform) }

    fn compute_properties(&mut self, rtc: &mut RenderTreeContext<R>) {
        let mut size = &mut *self.size_frame.as_ref().borrow_mut();

        if let Some(new_size) = rtc.compute_vtable_value(size[0]._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Size(v) = new_size { v } else { unreachable!() };
            size[0].set(new_value);
        }

        if let Some(new_size) = rtc.compute_vtable_value(size[1]._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Size(v) = new_size { v } else { unreachable!() };
            size[1].set(new_value);
        }

        let mut size = &mut *self.size_inner_pane.as_ref().borrow_mut();

        if let Some(new_size) = rtc.compute_vtable_value(size[0]._get_vtable_id()) {
            let new_value = if let TypesCoproduct::f64(v) = new_size { v } else { unreachable!() };
            size[0].set(new_value);
        }

        if let Some(new_size) = rtc.compute_vtable_value(size[1]._get_vtable_id()) {
            let new_value = if let TypesCoproduct::f64(v) = new_size { v } else { unreachable!() };
            size[1].set(new_value);
        }

        let mut transform = &mut *self.transform.as_ref().borrow_mut();
        if let Some(new_transform) = rtc.compute_vtable_value(transform._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Transform2D(v) = new_transform { v } else { unreachable!() };
            transform.set(new_value);
        }

        let mut transform = &mut *self.transform.as_ref().borrow_mut();
        if let Some(new_transform) = rtc.compute_vtable_value(transform._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Transform2D(v) = new_transform { v } else { unreachable!() };
            transform.set(new_value);
        }
    }

    fn handle_will_render(&mut self, rtc: &mut RenderTreeContext<R>, rc: &mut R) {
        let transform = rtc.transform;
        let bounding_dimens = rtc.bounds;

        let width: f64 =  bounding_dimens.0;
        let height: f64 =  bounding_dimens.1;

        let mut bez_path = BezPath::new();
        bez_path.move_to((0.0, 0.0));
        bez_path.line_to((width , 0.0));
        bez_path.line_to((width , height ));
        bez_path.line_to((0.0, height));
        bez_path.line_to((0.0,0.0));
        bez_path.close_path();

        let transformed_bez_path = transform * bez_path;
        rc.save().unwrap(); //our "save point" before clipping — restored to in the did_render
        rc.clip(transformed_bez_path);

        let id_chain = rtc.get_id_chain(self.instance_id);
        (*rtc.runtime).borrow_mut().push_clipping_stack_id(id_chain);
    }
    fn handle_did_render(&mut self, rtc: &mut RenderTreeContext<R>, rc: &mut R) {
        //pop the clipping context from the stack
        rc.restore().unwrap();

        (*rtc.runtime).borrow_mut().pop_clipping_stack_id();
    }

    fn handle_did_mount(&mut self, rtc: &mut RenderTreeContext<R>) {
        let id_chain = rtc.get_id_chain(self.instance_id);

        //though macOS and iOS don't need this ancestry chain for clipping, Web does
        let clipping_ids = (*rtc.runtime).borrow().get_current_clipping_ids();

        (*rtc.engine.runtime).borrow_mut().enqueue_native_message(
            pax_message::NativeMessage::ScrollerCreate(AnyCreatePatch {
                id_chain: id_chain.clone(),
                clipping_ids,
            })
        );

    }

    fn handle_will_unmount(&mut self, rtc: &mut RenderTreeContext<R>) {
        unimplemented!("Scroller unmount not yet handled")
    }

}
