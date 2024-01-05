use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use pax_message::{NativeMessage, OcclusionPatch};

use pax_runtime_api::{
    ArgsCheckboxChange, ArgsClap, ArgsClick, ArgsContextMenu, ArgsDoubleClick, ArgsKeyDown,
    ArgsKeyPress, ArgsKeyUp, ArgsMouseDown, ArgsMouseMove, ArgsMouseOut, ArgsMouseOver,
    ArgsMouseUp, ArgsScroll, ArgsTouchEnd, ArgsTouchMove, ArgsTouchStart, ArgsWheel,
    CommonProperties, Interpolatable, Layer, NodeContext, OcclusionLayerGen, RenderContext,
    TransitionManager,
};

use crate::declarative_macros::{handle_vtable_update, handle_vtable_update_optional};
use crate::{
    Affine, ComponentInstance, ExpressionContext, RuntimeContext, RuntimePropertiesStackFrame,
    TransformAndBounds,
};

/// The atomic unit of rendering; also the container for each unique tuple of computed properties.
/// Represents an expanded node, that is "expanded" in the context of computed properties and repeat expansion.
/// For example, a Rectangle inside `for i in 0..3` and a `for j in 0..4` would have 12 expanded nodes representing the 12 virtual Rectangles in the
/// rendered scene graph. These nodes are addressed uniquely by id_chain (see documentation for `get_id_chain`.)
/// `ExpandedNode`s are architecturally "type-blind" — while they store typed data e.g. inside `computed_properties` and `computed_common_properties`,
/// they require coordinating with their "type-aware" [`InstanceNode`] to perform operations on those properties.
mod expanded_node;
pub use expanded_node::ExpandedNode;

pub struct Globals {
    pub frames_elapsed: usize,
    pub viewport: TransformAndBounds,
}

/// Singleton struct storing everything related to properties computation & rendering
pub struct PaxEngine {
    pub runtime_context: RuntimeContext,
    pub root_node: Rc<ExpandedNode>,
    pub z_index_node_cache: Vec<Rc<ExpandedNode>>,
    pub image_map: HashMap<Vec<u32>, (Box<Vec<u8>>, usize, usize)>,
}

//This trait is used strictly to side-load the `compute_properties` function onto CommonProperties,
//so that it can use the type RenderTreeContext (defined in pax_core, which depends on pax_runtime_api, which
//defines CommonProperties, and which can thus not depend on pax_core due to a would-be circular dependency.)
pub trait PropertiesComputable {
    fn compute_properties(
        &mut self,
        stack: &Rc<RuntimePropertiesStackFrame>,
        table: &ExpressionTable,
    );
}

impl PropertiesComputable for CommonProperties {
    fn compute_properties(
        &mut self,
        stack: &Rc<RuntimePropertiesStackFrame>,
        table: &ExpressionTable,
    ) {
        handle_vtable_update(table, stack, &mut self.width);
        handle_vtable_update(table, stack, &mut self.height);
        handle_vtable_update(table, stack, &mut self.transform);
        handle_vtable_update_optional(table, stack, self.rotate.as_mut());
        handle_vtable_update_optional(table, stack, self.scale_x.as_mut());
        handle_vtable_update_optional(table, stack, self.scale_y.as_mut());
        handle_vtable_update_optional(table, stack, self.skew_x.as_mut());
        handle_vtable_update_optional(table, stack, self.skew_y.as_mut());
        handle_vtable_update_optional(table, stack, self.anchor_x.as_mut());
        handle_vtable_update_optional(table, stack, self.anchor_y.as_mut());
        handle_vtable_update_optional(table, stack, self.x.as_mut());
        handle_vtable_update_optional(table, stack, self.y.as_mut());
    }
}

pub struct HandlerRegistry {
    pub scroll_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsScroll)>,
    pub clap_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsClap)>,
    pub touch_start_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsTouchStart)>,
    pub touch_move_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsTouchMove)>,
    pub touch_end_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsTouchEnd)>,
    pub key_down_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsKeyDown)>,
    pub key_up_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsKeyUp)>,
    pub key_press_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsKeyPress)>,
    pub checkbox_change_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsCheckboxChange)>,
    pub click_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsClick)>,
    pub mouse_down_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsMouseDown)>,
    pub mouse_up_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsMouseUp)>,
    pub mouse_move_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsMouseMove)>,
    pub mouse_over_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsMouseOver)>,
    pub mouse_out_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsMouseOut)>,
    pub double_click_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsDoubleClick)>,
    pub context_menu_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsContextMenu)>,
    pub wheel_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext, ArgsWheel)>,
    pub pre_render_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext)>,
    pub mount_handlers: Vec<fn(Rc<RefCell<dyn Any>>, &NodeContext)>,
}

impl Default for HandlerRegistry {
    fn default() -> Self {
        HandlerRegistry {
            scroll_handlers: Vec::new(),
            clap_handlers: Vec::new(),
            touch_start_handlers: Vec::new(),
            touch_move_handlers: Vec::new(),
            touch_end_handlers: Vec::new(),
            key_down_handlers: Vec::new(),
            key_up_handlers: Vec::new(),
            key_press_handlers: Vec::new(),
            click_handlers: Vec::new(),
            mouse_down_handlers: Vec::new(),
            mouse_up_handlers: Vec::new(),
            mouse_move_handlers: Vec::new(),
            mouse_over_handlers: Vec::new(),
            mouse_out_handlers: Vec::new(),
            double_click_handlers: Vec::new(),
            context_menu_handlers: Vec::new(),
            wheel_handlers: Vec::new(),
            pre_render_handlers: Vec::new(),
            mount_handlers: Vec::new(),
            checkbox_change_handlers: Vec::new(),
        }
    }
}

pub struct Renderer<R> {
    pub backend: R,
}

impl<R: piet::RenderContext> pax_runtime_api::RenderContext for Renderer<R> {
    fn fill(&mut self, path: kurbo::BezPath, brush: &piet_common::PaintBrush) {
        self.backend.fill(path, brush);
    }

    fn stroke(&mut self, path: kurbo::BezPath, brush: &piet_common::PaintBrush, width: f64) {
        self.backend.stroke(path, brush, width);
    }

    fn save(&mut self) {
        self.backend.save().expect("failed to save piet state");
    }

    fn clip(&mut self, path: kurbo::BezPath) {
        self.backend.clip(path);
    }

    fn restore(&mut self) {
        self.backend
            .restore()
            .expect("failed to restore piet state");
    }
}

pub struct ExpressionTable {
    pub table: HashMap<usize, Box<dyn Fn(ExpressionContext) -> Box<dyn Any>>>,
}

impl ExpressionTable {
    pub fn compute_vtable_value(
        &self,
        stack: &Rc<RuntimePropertiesStackFrame>,
        vtable_id: usize,
    ) -> Box<dyn Any> {
        if let Some(evaluator) = self.table.get(&vtable_id) {
            let stack_frame = Rc::clone(stack);
            let ec = ExpressionContext { stack_frame };
            (**evaluator)(ec)
        } else {
            panic!() //unhandled error if an invalid id is passed or if vtable is incorrectly initialized
        }
    }

    pub fn compute_eased_value<T: Clone + Interpolatable>(
        &self,
        transition_manager: Option<&mut TransitionManager<T>>,
        globals: &Globals,
    ) -> Option<T> {
        if let Some(tm) = transition_manager {
            if tm.queue.len() > 0 {
                let current_transition = tm.queue.get_mut(0).unwrap();
                if let None = current_transition.global_frame_started {
                    current_transition.global_frame_started = Some(globals.frames_elapsed);
                }
                let progress = (1.0 + globals.frames_elapsed as f64
                    - current_transition.global_frame_started.unwrap() as f64)
                    / (current_transition.duration_frames as f64);
                return if progress >= 1.0 {
                    //NOTE: we may encounter float imprecision here, consider `progress >= 1.0 - EPSILON` for some `EPSILON`
                    let new_value = current_transition.curve.interpolate(
                        &current_transition.starting_value,
                        &current_transition.ending_value,
                        progress,
                    );
                    tm.value = Some(new_value.clone());

                    tm.queue.pop_front();
                    self.compute_eased_value(Some(tm), globals)
                } else {
                    let new_value = current_transition.curve.interpolate(
                        &current_transition.starting_value,
                        &current_transition.ending_value,
                        progress,
                    );
                    tm.value = Some(new_value.clone());
                    tm.value.clone()
                };
            } else {
                return tm.value.clone();
            }
        }
        None
    }
}

/// Central instance of the PaxEngine and runtime, intended to be created by a particular chassis.
/// Contains all rendering and runtime logic.
///
impl PaxEngine {
    pub fn new(
        main_component_instance: Rc<ComponentInstance>,
        expression_table: ExpressionTable,
        logger: pax_runtime_api::PlatformSpecificLogger,
        viewport_size: (f64, f64),
    ) -> Self {
        pax_runtime_api::register_logger(logger);

        let globals = Globals {
            frames_elapsed: 0,
            viewport: TransformAndBounds {
                transform: Affine::default(),
                bounds: viewport_size,
            },
        };

        let mut runtime_context = RuntimeContext::new(expression_table, globals);

        let root_node = ExpandedNode::root(main_component_instance, &mut runtime_context);

        PaxEngine {
            runtime_context,
            root_node,
            image_map: HashMap::new(),
            z_index_node_cache: Vec::new(),
        }
    }

    // NOTES: this is the order of different things being computed in recurse-expand-nodes
    // - expanded_node instantiated from instance_node.

    /// Workhorse methods of every tick.  Will be executed up to 240 Hz.
    /// Three phases:
    /// 1. Expand nodes & compute properties; recurse entire instance tree and evaluate ExpandedNodes, stitching
    ///    together parent/child relationships between ExpandedNodes along the way.
    /// 2. Compute layout (z-index & TransformAndBounds) by visiting ExpandedNode tree
    ///    in rendering order, writing computed rendering-specific values to ExpandedNodes
    /// 3. Render:
    ///     a. find lowest node (last child of last node)
    ///     b. start rendering, from lowest node on-up, throughout tree
    pub fn tick(
        &mut self,
        rcs: &mut HashMap<String, Box<dyn RenderContext>>,
    ) -> Vec<NativeMessage> {
        //
        // 1. UPDATE NODES (properties, etc.). This part we should be able to
        // completely remove once reactive properties dirty-dag is a thing.
        //
        self.root_node.recurse_update(&mut self.runtime_context);

        // Z-INDEX (just render order, for now. used for hit testing)
        {
            self.z_index_node_cache.clear();
            fn assign_z_indicies(n: &Rc<ExpandedNode>, state: &mut Vec<Rc<ExpandedNode>>) {
                state.push(Rc::clone(&n));
            }

            self.root_node
                .recurse_visit_postorder(&assign_z_indicies, &mut self.z_index_node_cache);
        }

        // This is pretty useful during debugging - left it here since I use it often. /Sam
        // pax_runtime_api::log(&format!("tree: {:#?}", self.root_node));

        self.root_node
            .recurse_render(&mut self.runtime_context, rcs);

        // TODO canvas layer ids, see notes in lab-journal-ss line 95.
        // 2. LAYER-IDS, z-index list creation Will always be recomputed each
        // frame. Nothing intensive is to be done here.
        let mut occlusion_ind = OcclusionLayerGen::new(None);
        for node in self.z_index_node_cache.iter() {
            let layer = node.instance_template.base().flags().layer;
            occlusion_ind.update_z_index(layer);
            let new_occlusion_ind = occlusion_ind.get_level();
            let mut curr_occlusion_ind = node.occlusion_id.borrow_mut();
            if layer == Layer::Native && *curr_occlusion_ind != new_occlusion_ind {
                self.runtime_context.enqueue_native_message(
                    pax_message::NativeMessage::OcclusionUpdate(OcclusionPatch {
                        id_chain: node.id_chain.clone(),
                        z_index: new_occlusion_ind,
                    }),
                );
            }
            *curr_occlusion_ind = new_occlusion_ind;
        }
        // Next steps:
        // - Make occlusion updates actually do something chassi side.
        // - what nodes need to send occlusion updates? last canvas might still
        //   need to create next layer.

        self.runtime_context.take_native_messages()
    }

    /// Simple 2D raycasting: the coordinates of the ray represent a
    /// ray running orthogonally to the view plane, intersecting at
    /// the specified point `ray`.  Areas outside of clipping bounds will
    /// not register a `hit`, nor will elements that suppress input events.
    pub fn get_topmost_element_beneath_ray(&self, ray: (f64, f64)) -> Option<Rc<ExpandedNode>> {
        //Traverse all elements in render tree sorted by z-index (highest-to-lowest)
        //First: check whether events are suppressed
        //Next: check whether ancestral clipping bounds (hit_test) are satisfied
        //Finally: check whether element itself satisfies hit_test(ray)

        //Instead of storing a pointer to `last_rtc`, we should store a custom
        //struct with exactly the fields we need for ray-casting

        let mut ret: Option<Rc<ExpandedNode>> = None;
        for node in self.z_index_node_cache.iter().rev().skip(1) {
            if node.ray_cast_test(&ray) {
                //We only care about the topmost node getting hit, and the element
                //pool is ordered by z-index so we can just resolve the whole
                //calculation when we find the first matching node

                let mut ancestral_clipping_bounds_are_satisfied = true;
                let mut parent: Option<Rc<ExpandedNode>> =
                    node.parent_expanded_node.borrow().upgrade();

                loop {
                    if let Some(unwrapped_parent) = parent {
                        if let Some(_) = unwrapped_parent.get_clipping_size() {
                            ancestral_clipping_bounds_are_satisfied =
                            //clew
                                (*unwrapped_parent).ray_cast_test(&ray);
                            break;
                        }
                        parent = unwrapped_parent.parent_expanded_node.borrow().upgrade();
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

    pub fn get_focused_element(&self) -> Option<Rc<ExpandedNode>> {
        let (x, y) = self.runtime_context.globals().viewport.bounds;
        self.get_topmost_element_beneath_ray((x / 2.0, y / 2.0))
    }

    /// Called by chassis when viewport size changes, e.g. with native window resizes
    pub fn set_viewport_size(&mut self, new_viewport_size: (f64, f64)) {
        self.runtime_context.globals_mut().viewport.bounds = new_viewport_size;
    }

    pub fn load_image(
        &mut self,
        id_chain: Vec<u32>,
        image_data: Vec<u8>,
        width: usize,
        height: usize,
    ) {
        self.image_map
            .insert(id_chain, (Box::new(image_data), width, height));
    }
}
