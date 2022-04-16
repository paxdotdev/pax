use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::env::Args;
use std::f64::EPSILON;
use std::rc::Rc;




extern crate wee_alloc;

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;



use kurbo::{
    BezPath,
    Point,
    Vec2,
};
use piet_common::RenderContext;
use piet_web::WebRenderContext;

use crate::{Affine, ComponentInstance, Color, Error, ComputableTransform, RenderNodePtr, StrokeInstance, StrokeStyle, RenderNode, ExpressionContext, PropertyExpression, RenderNodePtrList};
use crate::runtime::{Runtime};
//TODO: make the JsValue render_message_queue platform agnostic and remove this dep —
//      (probably translate to JsValue at the pax-chassis-web layer instead of here.)
use wasm_bindgen::JsValue;
use pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};

use pax_runtime_api::{ArgsClick, ArgsRender, Interpolatable, PropertyInstance, TransitionManager, TransitionQueueEntry};

pub enum EventMessage {
    Tick(ArgsRender),
    Click(ArgsClick),
}

pub struct PaxEngine<R: 'static + RenderContext> {
    pub frames_elapsed: usize,
    pub instance_map: Rc<RefCell<InstanceMap<R>>>, //If the Rc<RefCell<>> is problematic for perf, could revisit this.  Is only a Rc<RefCell<>> to ease the mutability constraints of passing of the instance_map during RIL node instantiation
    pub event_message_queue: Vec<(String, EventMessage)>, //(element id, args)
    pub expression_table: HashMap<String, Box<dyn Fn(ExpressionContext<R>) -> TypesCoproduct> >,
    pub root_component: Rc<RefCell<ComponentInstance<R>>>,
    //NOTE: to support multiple concurrent "root components," e.g. for multi-stage authoring, this could simply be made an array of `root_components`
    pub runtime: Rc<RefCell<Runtime<R>>>,
    viewport_size: (f64, f64),
}


// #[derive(Clone)]
pub struct RenderTreeContext<'a, R: 'static + RenderContext>
{
    pub engine: &'a PaxEngine<R>,
    pub transform: Affine,
    pub bounds: (f64, f64),
    pub runtime: Rc<RefCell<Runtime<R>>>,
    pub node: RenderNodePtr<R>,
    pub timeline_playhead_position: usize,
    pub inherited_adoptees: Option<RenderNodePtrList<R>>,
}

impl<'a, R: 'static + RenderContext> Clone for RenderTreeContext<'a, R> {
    fn clone(&self) -> Self {
        RenderTreeContext {
            engine: &self.engine,
            transform: self.transform.clone(),
            bounds: self.bounds.clone(),
            runtime: Rc::clone(&self.runtime),
            node: Rc::clone(&self.node),
            timeline_playhead_position: self.timeline_playhead_position.clone(),
            inherited_adoptees: self.inherited_adoptees.clone()
        }
    }
}

impl<'a, R: RenderContext> Into<ArgsRender> for RenderTreeContext<'a, R> {

    fn into(self) -> ArgsRender {
        // possible approach to enabling "auto cell count" in `Spread`, for example:
        // let adoptee_count = {
        //     let stack_frame = (*(*self.runtime).borrow().peek_stack_frame().expect("Component required")).borrow();
        //     if stack_frame.has_adoptees() {
        //         (*stack_frame.get_adoptees()).borrow().len()
        //     } else {
        //         0
        //     }
        // };

        ArgsRender {
            frames_elapsed: self.engine.frames_elapsed,
            bounds: self.bounds.clone(),
            // adoptee_count,
        }
    }
}


impl<'a, R: RenderContext> RenderTreeContext<'a, R> {
    pub fn compute_eased_value<T: Clone + Interpolatable>(&self, transition_manager: Option<&mut TransitionManager<T>>) -> Option<T> {
        if let Some(mut tm) = transition_manager {
            if tm.queue.len() > 0 {
                let mut current_transition = tm.queue.get_mut(0).unwrap();
                if let None = current_transition.global_frame_started {
                    current_transition.global_frame_started = Some(self.engine.frames_elapsed);
                }
                //[0,1]
                let progress = (self.engine.frames_elapsed as f64 - current_transition.global_frame_started.unwrap() as f64) / (current_transition.duration_frames as f64);
                return if progress >= 1.0 { //TODO -- minus some epsilon for float imprecision?
                    tm.queue.pop_front();
                    self.compute_eased_value(Some(tm))
                } else {

                    let new_value = current_transition.curve.interpolate(&current_transition.starting_value, &current_transition.ending_value, progress);
                    tm.value = Some(new_value.clone());
                    tm.value.clone()
                };
            } else {
                return tm.value.clone();
            }
        }
        pax_runtime_api::log(&format!("Returning None for eased value"));
        None
    }

    //both Expressions and Timelines store their evaluators in the same vtable
    pub fn compute_vtable_value(&self, vtable_id: Option<&str>) -> Option<TypesCoproduct> {

        if let Some(id) = vtable_id {
            if let Some(evaluator) = self.engine.expression_table.borrow().get(id) {
                let ec = ExpressionContext {
                    engine: self.engine,
                    stack_frame: Rc::clone(&(*self.runtime).borrow_mut().peek_stack_frame().unwrap()),
                };
                return Some((**evaluator)(ec));
            }
        } //TODO: else if present in timeline vtable...

        None
    }
}

//
// pub struct HostPlatformContext<'a, 'b>
// {
//     pub drawing_context: Box<dyn RenderContext>,
//     pub render_message_queue: Vec<JsValue>, //TODO: platform polyfill
// }


#[derive(Default)]
pub struct HandlerRegistry {
    pub click_handlers: Vec<fn(Rc<RefCell<PropertiesCoproduct>>, ArgsClick)>,
    pub pre_render_handlers: Vec<fn(Rc<RefCell<PropertiesCoproduct>>, ArgsRender)>,
}

pub type InstanceMap<R: 'static + RenderContext> = HashMap<String, RenderNodePtr<R>>;

impl<R: 'static + RenderContext> PaxEngine<R> {
    pub fn new(
        root_component_instance: Rc<RefCell<ComponentInstance<R>>>,
        expression_table: HashMap<String, Box<dyn Fn(ExpressionContext<R>)->TypesCoproduct>>,
        logger: fn(&str),
        viewport_size: (f64, f64),
        instance_map: Rc<RefCell<InstanceMap<R>>>,
    ) -> Self {
        pax_runtime_api::register_logger(logger);
        PaxEngine {
            frames_elapsed: 0,
            instance_map,
            event_message_queue: vec![],
            expression_table,
            runtime: Rc::new(RefCell::new(Runtime::new(logger))),
            root_component: root_component_instance,
            viewport_size,
        }
    }

    // #[cfg(feature="designtime")]
    // fn get_root_component(&self) -> Rc<RefCell<Component>> {
    //     //For development, retrieve dynamic render tree from dev server
    //     designtime.get_root_component()
    // }
    // #[cfg(not(feature="designtime"))]
    // fn get_root_component(&self) -> Rc<RefCell<ComponentInstance>> {
    //     //For production, retrieve "baked in" render tree
    //     Rc::clone(&self.root_component)
    // }

    //TODO: use piet-common and `dyn`-ize WebRenderContext
    fn traverse_render_tree(&self, rc: &mut R) -> Vec<u8> {
        // Broadly:
        // 1. compute properties
        // 2. find lowest node (last child of last node), accumulating transform along the way
        // 3. start rendering, from lowest node on-up


        // let mut hpc = HostPlatformContext {
        //     drawing_context: rc,
        //     render_message_queue: Vec::new(),
        // };

        let cast_component_rc : RenderNodePtr<R> = self.root_component.clone();

        let mut rtc = RenderTreeContext {
            engine: &self,
            transform: Affine::default(),
            bounds: self.viewport_size,
            runtime: self.runtime.clone(),
            node: Rc::clone(&cast_component_rc),
            timeline_playhead_position: self.frames_elapsed,
            inherited_adoptees: None,
        };

        &self.recurse_traverse_render_tree(&mut rtc, rc, Rc::clone(&cast_component_rc));
        vec![]
        // hpc.render_message_queue
    }

    fn recurse_traverse_render_tree(&self, rtc: &mut RenderTreeContext<R>, rc: &mut R, node: RenderNodePtr<R>)  {
        // Recurse:
        //  - compute properties for this node
        //  - iterate backwards over children (lowest first); recurse until there are no more descendants.  track transform matrix & bounding dimensions along the way.
        //  - we now have the back-most leaf node.  Render it.  Return.
        //  - we're now at the second back-most leaf node.  Render it.  Return ...
        //  - done with this frame


        //populate a pointer to this (current) `RenderNode` onto `rtc`
        rtc.node = Rc::clone(&node);

        //TODO: double-check that this logic should be happening here, vs. after `compute_properties` (where
        //the "current component" will actually push its stack frame.)
        //peek at the current stack frame and set a scoped playhead position as needed
        match rtc.runtime.borrow_mut().peek_stack_frame() {
            Some(stack_frame) => {
                rtc.timeline_playhead_position = stack_frame.borrow_mut().get_timeline_playhead_position().clone();
            },
            None => ()
        }

        //lifecycle: init_and_calc happens before anything else and
        //           calculates
        //
        node.borrow_mut().compute_properties(rtc);
        let accumulated_transform = rtc.transform;
        let accumulated_bounds = rtc.bounds;

        //get the size of this node (calc'd or otherwise) and use
        //it as the new accumulated bounds: both for this nodes children (their parent container bounds)
        //and for this node itself (e.g. for specifying the size of a Rectangle node)
        let new_accumulated_bounds = node.borrow_mut().get_size_calc(accumulated_bounds);

        let node_computed_transform = {
            let mut node_borrowed = rtc.node.borrow_mut();
            let node_size = node_borrowed.get_size_calc(accumulated_bounds);
            let components = node_borrowed.get_transform().borrow_mut().get()
            .compute_transform_matrix(
                node_size,
                accumulated_bounds,
            );
            //combine align transformation exactly once per element per frame
            components.1 * components.0
        };

        let new_accumulated_transform = accumulated_transform * node_computed_transform;

        rtc.bounds = new_accumulated_bounds;
        rtc.transform = new_accumulated_transform;

        //lifecycle: pre_render for primitives
        node.borrow_mut().pre_render(rtc, rc);

        //lifecycle: pre_render for userland components
        let registry = (*node).borrow().get_handler_registry();
        if let Some(registry) = registry {
            //grab Rc of properties from stack frame; pass to type-specific handler
            //on instance in order to dispatch cartridge method
            match rtc.runtime.borrow_mut().peek_stack_frame() {
                Some(stack_frame) => {
                    for handler in (*registry).borrow().pre_render_handlers.iter() {
                        let args = ArgsRender { bounds: rtc.bounds.clone(), frames_elapsed: rtc.engine.frames_elapsed };
                        handler(stack_frame.borrow_mut().get_properties(), args);
                    }
                },
                None => {
                    panic!("can't bind events without a component")
                },
            }
        }


        let children = node.borrow_mut().get_rendering_children();

        //keep recursing through children
        children.borrow_mut().iter().rev().for_each(|child| {
            //note that we're iterating starting from the last child, for z-index (.rev())
            let mut new_rtc = rtc.clone();
            &self.recurse_traverse_render_tree(&mut new_rtc, rc, Rc::clone(child));
            //TODO: for dependency management, return computed values from subtree above
        });

        // lifecycle: `render`
        // this is this node's time to do its own rendering, aside
        // from its children.  Its children have already been rendered.
        node.borrow_mut().render(rtc, rc);

        // lifecycle: post_render
        node.borrow_mut().post_render(rtc, rc);
    }

    pub fn set_viewport_size(&mut self, new_viewport_size: (f64, f64)) {
        self.viewport_size = new_viewport_size;
    }

    pub fn tick(&mut self, rc: &mut R) {
        rc.clear(None, Color::rgb8(0, 0, 0));
        let native_render_queue = self.traverse_render_tree(rc);
        self.frames_elapsed = self.frames_elapsed + 1;
    }

}
