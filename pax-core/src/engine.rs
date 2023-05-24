use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::env::Args;
use std::f64::EPSILON;
use std::rc::Rc;
use std::thread::sleep;
use std::time::Duration;
use kurbo::Point;



use pax_message::NativeMessage;

use piet_common::RenderContext;

use crate::{Affine, ComponentInstance, Color, ComputableTransform, RenderNodePtr, ExpressionContext, RenderNodePtrList, RenderNode, TabCache, TransformAndBounds, StackFrame, ScrollerArgs};
use crate::runtime::{Runtime};
use pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};

use pax_runtime_api::{ArgsClick, ArgsJab, ArgsRender, ArgsScroll, Interpolatable, TransitionManager};

pub enum EventMessage {
    Tick(ArgsRender),
    Click(ArgsClick),
    Jab(ArgsJab),
}

pub struct PaxEngine<R: 'static + RenderContext> {
    pub frames_elapsed: usize,
    pub instance_registry: Rc<RefCell<InstanceRegistry<R>>>,
    pub expression_table: HashMap<usize, Box<dyn Fn(ExpressionContext<R>) -> TypesCoproduct> >,
    pub root_component: Rc<RefCell<ComponentInstance<R>>>,
    pub runtime: Rc<RefCell<Runtime<R>>>,
    pub image_map: HashMap<Vec<u64>, (Box<Vec<u8>>, usize, usize)>,
    viewport_size: (f64, f64),
}


pub struct ExpressionVTable<R: 'static + RenderContext> {
    inner_map: HashMap<usize, Box<dyn Fn(ExpressionContext<R>) -> TypesCoproduct>>,
    dependency_graph: HashMap<u64, Vec<u64>>,
}

pub struct RenderTreeContext<'a, R: 'static + RenderContext>
{
    pub engine: &'a PaxEngine<R>,
    pub transform: Affine,
    pub bounds: (f64, f64),
    pub runtime: Rc<RefCell<Runtime<R>>>,
    pub node: RenderNodePtr<R>,
    pub parent_hydrated_node: Option<Rc<HydratedNode<R>>>,
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
            parent_hydrated_node: self.parent_hydrated_node.clone(),
            timeline_playhead_position: self.timeline_playhead_position.clone(),
            inherited_adoptees: self.inherited_adoptees.clone(),
        }
    }
}

impl<'a, R: RenderContext> Into<ArgsRender> for RenderTreeContext<'a, R> {


    fn into(self) -> ArgsRender {
        // possible approach to enabling "auto cell count" in `Stacker`, for example:
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
                return if progress >= 1.0 { //NOTE: we may encounter float imprecision here, consider `progress >= 1.0 - EPSILON` for some `EPSILON`
                    let new_value = current_transition.curve.interpolate(&current_transition.starting_value, &current_transition.ending_value, progress);
                    tm.value = Some(new_value.clone());

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

    pub fn compute_vtable_value(&self, vtable_id: Option<usize>) -> Option<TypesCoproduct> {

        if let Some(id) = vtable_id {
            if let Some(evaluator) = self.engine.expression_table.get(&id) {
                let ec = ExpressionContext {
                    engine: self.engine,
                    stack_frame: Rc::clone(&(*self.runtime).borrow_mut().peek_stack_frame().unwrap()),
                };
                return Some((**evaluator)(ec));
            }
        } //FUTURE: for timelines: else if present in timeline vtable...

        None
    }
}

#[derive(Default)]
pub struct HandlerRegistry<R: 'static + RenderContext> {
    pub click_handlers: Vec<fn(Rc<RefCell<StackFrame<R>>>, ArgsClick)>,
    pub will_render_handlers: Vec<fn(Rc<RefCell<PropertiesCoproduct>>, ArgsRender)>,
    pub did_mount_handlers: Vec<fn(Rc<RefCell<PropertiesCoproduct>>)>,
    pub scroll_handlers: Vec<fn(Rc<RefCell<StackFrame<R>>>, ArgsScroll)>,
}

/// Represents a repeat-expanded node.  For example, a Rectangle inside `for i in 0..3` and
/// a `for j in 0..4` would have 12 hydrated nodes representing the 12 virtual Rectangles in the
/// rendered scene graph. These nodes are addressed uniquely by id_chain (see documentation for `get_id_chain`.)
pub struct HydratedNode<R: 'static + RenderContext> {
    id_chain: Vec<u64>,
    parent_hydrated_node: Option<Rc<HydratedNode<R>>>,
    instance_node: RenderNodePtr<R>,
    stack_frame: Rc<RefCell<crate::StackFrame<R>>>,
    tab: TransformAndBounds,
}

impl<R: 'static + RenderContext> HydratedNode<R> {
    pub fn dispatch_click(&self, args_click: ArgsClick) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            (*registry).borrow().click_handlers.iter().for_each(|handler|{
                handler(Rc::clone(&self.stack_frame), args_click.clone());
            })
        }
        if let Some(parent) = &self.parent_hydrated_node {
            parent.dispatch_click(args_click);
        }
    }
    pub fn dispatch_scroll(&self, args_scroll: ArgsScroll) {
        if let Some(registry) = (*self.instance_node).borrow().get_handler_registry() {
            (*registry).borrow().scroll_handlers.iter().for_each(|handler|{
                handler(Rc::clone(&self.stack_frame), args_scroll.clone());
            })
        }
        if let Some(parent) = &self.parent_hydrated_node {
            parent.dispatch_scroll(args_scroll);
        }
    }
}

pub struct InstanceRegistry<R: 'static + RenderContext> {
    ///look up RenderNodePtr by id
    instance_map: HashMap<u64, RenderNodePtr<R>>,

    ///a cache of "actual elements" visited by rendertree traversal,
    ///intended to be cleared at the beginning of each frame and populated
    ///with each node visited.  This enables post-facto operations on nodes with
    ///otherwise ephemeral calculations, e.g. the descendants of `Repeat` instances.
    hydrated_node_cache: Vec<Rc<HydratedNode<R>>>,

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
            hydrated_node_cache: vec![],
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

    pub fn reset_hydrated_node_cache(&mut self) {
        self.hydrated_node_cache = vec![];
    }

    pub fn add_to_hydrated_node_cache(&mut self, hydrated_node: Rc<HydratedNode<R>>) {
        //Note: ray-casting requires that these nodes are sorted by z-index
        self.hydrated_node_cache.push(hydrated_node);
    }


}


impl<R: 'static + RenderContext> PaxEngine<R> {
    pub fn new(
        root_component_instance: Rc<RefCell<ComponentInstance<R>>>,
        expression_table: HashMap<usize, Box<dyn Fn(ExpressionContext<R>)->TypesCoproduct>>,
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
            image_map: HashMap::new(),
        }
    }

    fn hydrate_render_tree(&self, rc: &mut R) -> Vec<pax_message::NativeMessage> {
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
            parent_hydrated_node: None,
            timeline_playhead_position: self.frames_elapsed,
            inherited_adoptees: None,
        };

        self.recurse_hydrate_render_tree(&mut rtc, rc, Rc::clone(&cast_component_rc));

        let native_render_queue = (*self.runtime).borrow_mut().swap_native_message_queue();
        native_render_queue.into()
    }

    fn recurse_hydrate_render_tree(&self, rtc: &mut RenderTreeContext<R>, rc: &mut R, node: RenderNodePtr<R>)  {
        //Recurse:
        //  - compute properties for this node
        //  - fire lifecycle events for this node
        //  - iterate backwards over children (lowest first); recurse until there are no more descendants.  track transform matrix & bounding dimensions along the way.
        //  - we now have the back-most leaf node.  Render it.  Return.
        //  - we're now at the second back-most leaf node.  Render it.  Return ...
        //  - done with this frame

        //populate a pointer to this (current) `RenderNode` onto `rtc`
        rtc.node = Rc::clone(&node);


        //lifecycle: compute_properties happens before rendering
        node.borrow_mut().compute_properties(rtc);
        let accumulated_transform = rtc.transform;
        let accumulated_bounds = rtc.bounds;


        //fire `did_mount` event if this is this node's first frame
        //Note that this must happen after initial `compute_properties`, which performs the
        //necessary side-effect of creating the `self` that must be passed to handlers
        {
            let id = (*rtc.node).borrow().get_instance_id();
            let mut instance_registry = (*rtc.engine.instance_registry).borrow_mut();

            //Due to Repeat, an effective unique instance ID is the tuple: `(instance_id, [list_of_RepeatItem_indices])`
            let repeat_indices = (*rtc.engine.runtime).borrow().get_list_of_repeat_indicies_from_stack();
            if !instance_registry.is_mounted(id, repeat_indices.clone()) {
                //Fire primitive-level did_mount lifecycle method
                node.borrow_mut().handle_did_mount(rtc);

                //Fire registered did_mount events
                let registry = (*node).borrow().get_handler_registry();
                if let Some(registry) = registry {
                    //grab Rc of properties from stack frame; pass to type-specific handler
                    //on instance in order to dispatch cartridge method
                    match rtc.runtime.borrow_mut().peek_stack_frame() {
                        Some(stack_frame) => {
                            for handler in (*registry).borrow().did_mount_handlers.iter() {
                                // let args = ArgsRender { bounds: rtc.bounds.clone(), frames_elapsed: rtc.engine.frames_elapsed };
                                handler(stack_frame.borrow_mut().get_properties());
                            }
                        },
                        None => {

                        },
                    }
                }
                instance_registry.mark_mounted(id, repeat_indices.clone());
            }
        }

        //peek at the current stack frame and set a scoped playhead position as needed
        match rtc.runtime.borrow_mut().peek_stack_frame() {
            Some(stack_frame) => {
                rtc.timeline_playhead_position = stack_frame.borrow_mut().get_timeline_playhead_position().clone();
            },
            None => ()
        }

        //get the size of this node (calc'd or otherwise) and use
        //it as the new accumulated bounds: both for this nodes children (their parent container bounds)
        //and for this node itself (e.g. for specifying the size of a Rectangle node)
        let new_accumulated_bounds = node.borrow_mut().compute_size_within_bounds(accumulated_bounds);
        let mut node_size : (f64, f64) = (0.0, 0.0);
        let node_computed_transform = {
            let mut node_borrowed = rtc.node.borrow_mut();
            node_size = node_borrowed.compute_size_within_bounds(accumulated_bounds);
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

        //lifecycle: will_render for primitives
        node.borrow_mut().handle_will_render(rtc, rc);

        //fire `will_render` handlers
        let registry = (*node).borrow().get_handler_registry();
        if let Some(registry) = registry {
            //grab Rc of properties from stack frame; pass to type-specific handler
            //on instance in order to dispatch cartridge method
            match rtc.runtime.borrow_mut().peek_stack_frame() {
                Some(stack_frame) => {
                    for handler in (*registry).borrow().will_render_handlers.iter() {
                        let args = ArgsRender { bounds: rtc.bounds.clone(), frames_elapsed: rtc.engine.frames_elapsed };
                        handler(stack_frame.borrow_mut().get_properties(), args);
                    }
                },
                None => {
                    panic!("can't bind events without a component")
                },
            }
        }

        //create the `hydrated_node` for the current node
        let children = node.borrow_mut().get_rendering_children();
        let id_chain = rtc.get_id_chain(node.borrow().get_instance_id());
        let hydrated_node = Rc::new(HydratedNode {
            stack_frame: rtc.runtime.borrow_mut().peek_stack_frame().unwrap(),
            tab: TransformAndBounds {
                bounds: node_size,
                transform: new_accumulated_transform.clone(),
            },
            id_chain: id_chain.clone(),
            instance_node: Rc::clone(&node),
            parent_hydrated_node: rtc.parent_hydrated_node.clone(),
        });

        //keep recursing through children
        children.borrow_mut().iter().rev().for_each(|child| {
            //note that we're iterating starting from the last child, for z-index (.rev())
            let mut new_rtc = rtc.clone();
            new_rtc.parent_hydrated_node = Some(Rc::clone(&hydrated_node));
            &self.recurse_hydrate_render_tree(&mut new_rtc, rc, Rc::clone(child));
            //FUTURE: for dependency management, return computed values from subtree above
        });

        //Note: ray-casting requires that the hydrated_node_cache is sorted by z-index,
        //so the order in which `add_to_hydrated_node_cache` is invoked vs. descendants is important
        (*rtc.engine.instance_registry).borrow_mut().add_to_hydrated_node_cache(Rc::clone(&hydrated_node));

        //lifecycle: render
        //this is this node's time to do its own rendering, aside
        //from the rendering of its children. Its children have already been rendered.
        node.borrow_mut().handle_render(rtc, rc);

        //lifecycle: did_render
        node.borrow_mut().handle_did_render(rtc, rc);
    }

    /// Simple 2D raycasting: the coordinates of the ray represent a
    /// ray running orthogonally to the view plane, intersecting at
    /// the specified point `ray`.  Areas outside of clipping bounds will
    /// not register a `hit`, nor will elements that suppress input events.
    pub fn get_topmost_hydrated_element_beneath_ray(&self, ray: (f64, f64)) -> Option<Rc<HydratedNode<R>>> {
        //Traverse all elements in render tree sorted by z-index (highest-to-lowest)
        //First: check whether events are suppressed
        //Next: check whether ancestral clipping bounds (hit_test) are satisfied
        //Finally: check whether element itself satisfies hit_test(ray)

        //Instead of storing a pointer to `last_rtc`, we should store a custom
        //struct with exactly the fields we need for ray-casting

        //Need:
        // - Cached computed transform `: Affine`
        // - Pointer to parent:
        //     for bubbling, i.e. propagating event
        //     for finding ancestral clipping containers
        //

        // reverse nodes to get top-most first (rendered in reverse order)
        let mut nodes_ordered : Vec<Rc<HydratedNode<R>>> = (*self.instance_registry).borrow()
            .hydrated_node_cache.iter().rev()
            .map(|rc|{
                Rc::clone(rc)
            }).collect();

        // remove root element that is moved to top during reversal
        nodes_ordered.remove(0);

        // let ray = Point {x: ray.0,y: ray.1};
        let mut ret : Option<Rc<HydratedNode<R>>> = None;
        for node in nodes_ordered {
            // pax_runtime_api::log(&(**node).borrow().get_instance_id().to_string())


            if (*node.instance_node).borrow().ray_cast_test(&ray, &node.tab) {

                //We only care about the topmost node getting hit, and the element
                //pool is ordered by z-index so we can just resolve the whole
                //calculation when we find the first matching node

                let mut ancestral_clipping_bounds_are_satisfied = true;
                let mut parent : Option<Rc<HydratedNode<R>>> = node.parent_hydrated_node.clone();

                loop {
                    if let Some(unwrapped_parent) = parent {
                        if (*unwrapped_parent.instance_node).borrow().is_clipping() && !(*unwrapped_parent.instance_node).borrow().ray_cast_test(&ray, &unwrapped_parent.tab) {
                            ancestral_clipping_bounds_are_satisfied = false;
                            break;
                        }
                        parent = unwrapped_parent.parent_hydrated_node.clone();
                    } else {
                        break;
                    }
                }

                if ancestral_clipping_bounds_are_satisfied {
                    ret = Some(Rc::clone(&node));
                    break;
                }
            }
        }

        ret
    }

    /// Called by chassis when viewport size changes, e.g. with native window resizes
    pub fn set_viewport_size(&mut self, new_viewport_size: (f64, f64)) {
        self.viewport_size = new_viewport_size;
    }

    /// Workhorse method to advance rendering and property calculation by one discrete tick
    /// Expected to be called up to 60-120 times/second.
    pub fn tick(&mut self, rc: &mut R) -> Vec<NativeMessage> {
        rc.clear(None, Color::rgb(1.0, 1.0, 1.0));
        (*self.instance_registry).borrow_mut().reset_hydrated_node_cache();
        let native_render_queue = self.hydrate_render_tree(rc);
        self.frames_elapsed = self.frames_elapsed + 1;
        native_render_queue
    }

    pub fn loadImage(&mut self, id_chain: Vec<u64>, image_data: Vec<u8>, width: usize, height: usize) {
        self.image_map.insert(id_chain, (Box::new(image_data), width, height));
    }
}
