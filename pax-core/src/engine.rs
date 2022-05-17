use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::env::Args;
use std::f64::EPSILON;
use std::rc::Rc;

use pax_message::NativeMessage;

extern crate wee_alloc;

// Use `wee_alloc` as the global allocator, to reduce runtime disk footprint.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

use piet_common::RenderContext;

use crate::{Affine, ComponentInstance, Color, ComputableTransform, RenderNodePtr, ExpressionContext, RenderNodePtrList};
use crate::runtime::{Runtime};
use pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};

use pax_runtime_api::{ArgsClick, ArgsJab, ArgsRender, Interpolatable, TransitionManager};

pub enum EventMessage {
    Tick(ArgsRender),
    Click(ArgsClick),
    Jab(ArgsJab),
}

pub struct PaxEngine<R: 'static + RenderContext> {
    pub frames_elapsed: usize,
    pub instance_registry: Rc<RefCell<InstanceRegistry<R>>>,
    pub expression_table: HashMap<String, Box<dyn Fn(ExpressionContext<R>) -> TypesCoproduct> >,
    pub root_component: Rc<RefCell<ComponentInstance<R>>>,
    pub runtime: Rc<RefCell<Runtime<R>>>,
    viewport_size: (f64, f64),
}


pub struct ExpressionVTable<R: RenderContext + 'static> {
    inner_map: HashMap<u64, Box<dyn Fn(ExpressionContext<R>) -> TypesCoproduct>>,
    dependency_graph: HashMap<u64, Vec<u64>>,
}

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
        None
    }

    /// Get an `id_chain` for this element, an array of `u64` used collectively as a single unique ID across native bridges.
    /// Specifically, the ID chain represents not only the instance ID, but the indices of each RepeatItem found by a traversal
    /// of the runtime stack.
    ///
    /// The need for this emerges from the fact that `Repeat`ed elements share a single underlying
    /// `instance`, where that instantiation happens once at init-time — specifically, it does not happen
    /// when `Repeat`ed elements are added and removed to the render tree.  10 apparent rendered elements may share the same `instance_id` -- which doesn't work as a unique key for native renderers
    /// that are expected to render and update 10 distinct elements.
    ///
    /// Thus, the `id_chain` is used as a unique key, first the `instance_id` (which will increase monotonically through the lifetime of the program),
    /// then each RepeatItem index through a traversal of the stack frame.  Thus, each virtually `Repeat`ed element
    /// gets its own unique ID in the form of an "address" through any nested `Repeat`-ancestors.
    pub fn get_id_chain(&self, id: u64) -> Vec<u64> {
        let mut indices = (*self.runtime).borrow().get_list_of_repeat_indicies_from_stack();
        indices.insert(0, id);
        indices
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

#[derive(Default)]
pub struct HandlerRegistry {
    pub click_handlers: Vec<fn(Rc<RefCell<PropertiesCoproduct>>, ArgsClick)>,
    pub pre_render_handlers: Vec<fn(Rc<RefCell<PropertiesCoproduct>>, ArgsRender)>,
}

pub struct InstanceRegistry<R: 'static + RenderContext> {
    ///look up RenderNodePtr by id
    instance_map: HashMap<u64, RenderNodePtr<R>>,

    ///track which elements are currently mounted -- if id is present in set, is mounted
    mounted_set: HashSet<(u64, Vec<u64>)>,

    ///register holding the next value to mint as an id
    next_id: u64,
}

impl<R: 'static + RenderContext> InstanceRegistry<R> {
    pub fn new() -> Self {
        Self {
            mounted_set: HashSet::new(),
            instance_map: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn mint_id(&mut self) -> u64 {
        let new_id = self.next_id;
        self.next_id = self.next_id + 1;
        new_id
    }

    pub fn register(&mut self, instance_id: u64, node: RenderNodePtr<R>) {
        self.instance_map.insert(instance_id, node);
    }

    pub fn deregister(&mut self, instance_id: u64) {
        self.instance_map.remove(&instance_id);
    }

    pub fn mark_mounted(&mut self, id: u64, repeat_indices: Vec<u64>) {
        self.mounted_set.insert((id, repeat_indices));
    }

    pub fn is_mounted(&self, id: u64, repeat_indices: Vec<u64>) -> bool {
        self.mounted_set.contains(&(id, repeat_indices))
    }

    pub fn mark_unmounted(&mut self, id: u64, repeat_indices: Vec<u64>) {
        self.mounted_set.remove(&(id, repeat_indices));
    }

}


impl<R: 'static + RenderContext> PaxEngine<R> {
    pub fn new(
        root_component_instance: Rc<RefCell<ComponentInstance<R>>>,
        expression_table: HashMap<String, Box<dyn Fn(ExpressionContext<R>)->TypesCoproduct>>,
        logger: pax_runtime_api::PlatformSpecificLogger,
        viewport_size: (f64, f64),
        instance_registry: Rc<RefCell<InstanceRegistry<R>>>,
    ) -> Self {
        pax_runtime_api::register_logger(logger);
        PaxEngine {
            frames_elapsed: 0,
            instance_registry,
            expression_table,
            runtime: Rc::new(RefCell::new(Runtime::new())),
            root_component: root_component_instance,
            viewport_size,
        }
    }

    fn traverse_render_tree(&self, rc: &mut R) -> Vec<pax_message::NativeMessage> {
        //Broadly:
        // 1. compute properties
        // 2. find lowest node (last child of last node), accumulating transform along the way
        // 3. start rendering, from lowest node on-up

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

        self.recurse_traverse_render_tree(&mut rtc, rc, Rc::clone(&cast_component_rc));

        let native_render_queue = (*self.runtime).borrow_mut().swap_native_message_queue();
        native_render_queue.into()
    }

    fn recurse_traverse_render_tree(&self, rtc: &mut RenderTreeContext<R>, rc: &mut R, node: RenderNodePtr<R>)  {
        //Recurse:
        //  - compute properties for this node
        //  - fire lifecycle events for this node
        //  - iterate backwards over children (lowest first); recurse until there are no more descendants.  track transform matrix & bounding dimensions along the way.
        //  - we now have the back-most leaf node.  Render it.  Return.
        //  - we're now at the second back-most leaf node.  Render it.  Return ...
        //  - done with this frame

        //populate a pointer to this (current) `RenderNode` onto `rtc`
        rtc.node = Rc::clone(&node);

        //fire mount event if this is this node's first frame
        {
            let id = (*rtc.node).borrow().get_instance_id();
            let mut instance_registry = (*rtc.engine.instance_registry).borrow_mut();

            //Due to Repeat, an effective unique instance ID is the tuple: `(instance_id, [list_of_RepeatItem_indices])`
            let repeat_indices = (*rtc.engine.runtime).borrow().get_list_of_repeat_indicies_from_stack();
            if !instance_registry.is_mounted(id, repeat_indices.clone()) { //TODO: make more efficient
                node.borrow_mut().handle_post_mount(rtc);
                instance_registry.mark_mounted(id, repeat_indices.clone());
            }
        }

        //TODO: double-check that this logic should be happening here, vs. after `compute_properties`
        //the "current component" will actually push its stack frame.)
        //peek at the current stack frame and set a scoped playhead position as needed
        match rtc.runtime.borrow_mut().peek_stack_frame() {
            Some(stack_frame) => {
                rtc.timeline_playhead_position = stack_frame.borrow_mut().get_timeline_playhead_position().clone();
            },
            None => ()
        }

        //lifecycle: compute_properties happens before rendering
        node.borrow_mut().compute_properties(rtc);
        let accumulated_transform = rtc.transform;
        let accumulated_bounds = rtc.bounds;

        //get the size of this node (calc'd or otherwise) and use
        //it as the new accumulated bounds: both for this nodes children (their parent container bounds)
        //and for this node itself (e.g. for specifying the size of a Rectangle node)
        let new_accumulated_bounds = node.borrow_mut().compute_size_within_bounds(accumulated_bounds);

        let node_computed_transform = {
            let mut node_borrowed = rtc.node.borrow_mut();
            let node_size = node_borrowed.compute_size_within_bounds(accumulated_bounds);
            let components = node_borrowed.get_transform().borrow_mut().get()
            .compute_transform_matrix(
                node_size,
                accumulated_bounds,
            );
            //combine align transformation exactly once per element per frame
            components.1 * components.0
        };

        let new_accumulated_transform = accumulated_transform * node_computed_transform;
        rtc.bounds = new_accumulated_bounds.clone();
        rtc.transform = new_accumulated_transform.clone();

        //lifecycle: compute_native_patches — for elements with native components (for example Text, Frame, and form control elements),
        //certain native-bridge events must be triggered when changes occur, and some of those events require pre-computed `size` and `transform`.
        node.borrow_mut().compute_native_patches(rtc, new_accumulated_bounds, new_accumulated_transform.as_coeffs().to_vec());

        //lifecycle: pre_render for primitives
        node.borrow_mut().handle_pre_render(rtc, rc);

        //Fire userland `pax_on(PreRender)` handlers
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

        //lifecycle: render
        //this is this node's time to do its own rendering, aside
        //from its children.  Its children have already been rendered.
        node.borrow_mut().handle_render(rtc, rc);

        //lifecycle: post_render
        node.borrow_mut().handle_post_render(rtc, rc);

    }

    pub fn set_viewport_size(&mut self, new_viewport_size: (f64, f64)) {
        self.viewport_size = new_viewport_size;
    }

    pub fn tick(&mut self, rc: &mut R) -> Vec<NativeMessage> {
        rc.clear(None, Color::rgb(1.0, 1.0, 1.0));
        let native_render_queue = self.traverse_render_tree(rc);
        self.frames_elapsed = self.frames_elapsed + 1;
        native_render_queue
    }

}
