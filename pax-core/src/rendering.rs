use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use kurbo::{Affine, Point};
use piet::{Color, StrokeStyle};
use piet_common::RenderContext;
use pax_properties_coproduct::PropertiesCoproduct;

use pax_runtime_api::{Size, Size2D};

use crate::{RenderTreeContext, HandlerRegistry, InstanceRegistry};

use pax_runtime_api::{PropertyInstance, PropertyLiteral};

/// Type aliases to make it easier to work with nested Rcs and
/// RefCells for rendernodes.
pub type RenderNodePtr<R> = Rc<RefCell<dyn RenderNode<R>>>;
pub type RenderNodePtrList<R> = Rc<RefCell<Vec<RenderNodePtr<R>>>>;

pub struct ScrollerArgs {
    pub size_inner_pane: [Box<dyn PropertyInstance<f64>>;2],
    pub axes_enabled: [Box<dyn PropertyInstance<bool>>;2],
}


pub struct InstantiationArgs<R: 'static + RenderContext> {
    pub properties: PropertiesCoproduct,
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry<R>>>>,
    pub instance_registry: Rc<RefCell<InstanceRegistry<R>>>,
    pub transform: Rc<RefCell<dyn PropertyInstance<Transform2D>>>,
    pub size: Option<Size2D>,
    pub children: Option<RenderNodePtrList<R>>,
    pub component_template: Option<RenderNodePtrList<R>>,

    pub scroller_args: Option<ScrollerArgs>,

    /// used by Slot
    pub slot_index: Option<Box<dyn PropertyInstance<usize>>>,

    ///used by Repeat — the _vec and _range variants are modal, describing whether the source
    ///is encoded as a Vec<T> or as a Range<...>
    pub repeat_source_expression_vec: Option<Box<dyn PropertyInstance<Vec<Rc<PropertiesCoproduct>>>>>,
    pub repeat_source_expression_range: Option<Box<dyn PropertyInstance<std::ops::Range<isize>>>>,

    ///used by Conditional
    pub conditional_boolean_expression: Option<Box<dyn PropertyInstance<bool>>>,

    ///used by Component instances, specifically to unwrap type-specific PropertiesCoproducts
    ///and recurse into descendant property computation
    pub compute_properties_fn: Option<Box<dyn FnMut(Rc<RefCell<PropertiesCoproduct>>,&mut RenderTreeContext<R>)>>,
}

fn recurse_get_rendering_subtree_flattened<R: 'static + RenderContext>(children: RenderNodePtrList<R>) -> Vec<RenderNodePtr<R>> {
    //For each node:
    // - push self to list
    // - push children (recursively) to list,
    // - (next)
    let mut ret = vec![];

    (*children).borrow().iter().for_each(|child|{
        ret.push(Rc::clone(child));

        let new_children = (**child).borrow().get_rendering_children();
        ret.append(&mut recurse_get_rendering_subtree_flattened(new_children));
    });

    ret
}

/// Stores the computed transform and the pre-transform bounding box (where the
/// other corner is the origin).  Useful for ray-casting, along with
pub struct TransformAndBounds {
    pub transform: Affine,
    pub bounds: (f64, f64),
}

/// "Transform And Bounds" — a helper struct for storing necessary data for event propagation and ray casting
pub struct TabCache<R: 'static + RenderContext> {
    pub tabs: HashMap<Vec<u64>, TransformAndBounds>,
    pub parents: HashMap<Vec<u64>, Option<RenderNodePtr<R>>>,
}

impl<R: 'static + RenderContext> TabCache<R> {
    pub fn new() -> Self {
        Self {
            tabs: HashMap::new(),
            parents: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.tabs = HashMap::new();
        self.parents = HashMap::new();
    }
}

/// The base trait for a RenderNode, representing any node that can
/// be rendered by the engine.
/// T: a member of PropertiesCoproduct, representing the type of the set of properites
/// associated with this node.
pub trait RenderNode<R: 'static + RenderContext>
{

    fn instantiate(args: InstantiationArgs<R>) -> Rc<RefCell<Self>> where Self: Sized;

    /// Return the list of nodes that are children of this node at render-time.
    /// Note that "children" is somewhat overloaded, hence "rendering_children" here.
    /// "Children" may indicate a.) a template root, b.) adoptees, c.) primitive children
    /// Each RenderNode is responsible for determining at render-time which of these concepts
    /// to pass to the engine for rendering, and that distinction occurs inside `get_rendering_children`
    fn get_rendering_children(&self) -> RenderNodePtrList<R>;


    /// For this element and its subtree of rendering elements, mark as unmounted in InstanceRegistry
    /// If `permanent` is passed (namely, if this is not a "transient" unmount such as for `Conditional`), then
    /// the instance is permanently removed from the instance_registry
    fn unmount_recursive(&mut self, rtc: &mut RenderTreeContext<R>, permanent: bool) {
        {
            let repeat_indices = (*rtc.engine.runtime).borrow().get_list_of_repeat_indicies_from_stack();
            let mut instance_registry = (*rtc.engine.instance_registry).borrow_mut();
            if instance_registry.is_mounted(self.get_instance_id(), repeat_indices.clone()) {
                instance_registry.mark_unmounted(self.get_instance_id(), repeat_indices);
            }

            self.handle_will_unmount(rtc);

            if permanent {
                //cleans up memory, otherwise leads to runaway allocations in instance_registry
                instance_registry.deregister(self.get_instance_id());
            }
        }

        for child in (*self.get_rendering_children()).borrow().iter() {
            (*(*child)).borrow_mut().unmount_recursive(rtc, permanent);
        }
    }

    // /// Recursively collects all nodes from subtree into a single
    // /// list, **exclusive of the current node.** (This is due to Rc<RefCell<>>/ownership constraints)
    // /// This list is ordered descending by z-index and is appropriate at least for 2D raycasting.
    // fn get_rendering_subtree_flattened(&self) -> RenderNodePtrList<R> {
    //
    //     //push each child to the front of the running list, bottom-first
    //     //(so that top-most elements are always at the front)
    //
    //     let children = self.get_rendering_children();
    //     let ret = recurse_get_rendering_subtree_flattened(children);
    //
    //     Rc::new(RefCell::new(ret))
    // }

    ///Determines whether the provided ray, orthogonal to the view plane,
    ///intersects this rendernode. `tab` must also be passed because these are specific
    ///to a RepeatExpandedNode
    fn ray_cast_test(&self, ray: &(f64, f64), tab: &TransformAndBounds) -> bool {

        //short-circuit fail for Group and other size-None elements.
        //This doesn't preclude event handlers on Groups and size-None elements --
        //it just requires the event to "bubble".  otherwise, `Component A > Component B` will
        //never allow events to be bound to `B` — they will be vacuously intercepted by `A`
        if let None = self.get_size() {
            return false
        }

        let inverted_transform = tab.transform.inverse();
        let transformed_ray = inverted_transform * Point {x:ray.0,y:ray.1};

        //Default implementation: rectilinear bounding hull
        transformed_ray.x > 0.0 && transformed_ray.y > 0.0
            && transformed_ray.x < tab.bounds.0 && transformed_ray.y < tab.bounds.1
    }

    fn get_handler_registry(&self) -> Option<Rc<RefCell<HandlerRegistry<R>>>> {
        None //default no-op
    }

    /// Used at least by ray-casting; only nodes that clip content (and thus should
    /// not allow outside content to respond to ray-casting) should return true
    fn is_clipping(&self) -> bool {
        false
    }

    /// Returns the size of this node, or `None` if this node
    /// doesn't have a size (e.g. `Group`)
    fn get_size(&self) -> Option<Size2D>;

    /// Returns unique integer ID of this RenderNode instance.  Note that
    /// individual rendered elements may share an instance_id, for example
    /// inside of `Repeat`.  See also `RenderTreeContext::get_id_chain`, which enables globally
    /// unique node addressing in the context of an in-progress render tree traversal.
    fn get_instance_id(&self) -> u64;

    /// Used for exotic tree traversals, e.g. for `Stacker` > `Repeat` > `Rectangle`
    /// where the repeated `Rectangle`s need to be be considered direct children of `Stacker`.
    /// `Repeat` overrides `should_flatten` to return true, which `Engine` interprets to mean "ignore this
    /// node and consume its children" during traversal.
    ///
    /// This may also be useful as a check during slot -> adoptee
    /// searching via stackframes — currently slots will recurse
    /// up the stackframe looking for adoptees, but it may be the case that
    /// checking should_flatten and NOT recursing is better behavior.  TBD
    /// as more use-cases are vetted.
    fn should_flatten(&self) -> bool {
        false
    }

    /// Returns the size of this node in pixels, requiring
    /// parent bounds for calculation of `Percent` values
    fn compute_size_within_bounds(&self, bounds: (f64, f64)) -> (f64, f64) {
        match self.get_size() {
            None => bounds,
            Some(size_raw) => {
                (
                    match size_raw.borrow()[0].get() {
                        Size::Pixels(width) => {
                            width.get_as_float()
                        },
                        Size::Percent(width) => {
                            bounds.0 * (*width / 100.0)
                        }
                    },
                    match size_raw.borrow()[1].get() {
                        Size::Pixels(height) => {
                            height.get_as_float()
                        },
                        Size::Percent(height) => {
                            bounds.1 * (*height / 100.0)
                        }
                    }
                )
            }
        }
    }

    fn get_transform(&mut self) -> Rc<RefCell<dyn PropertyInstance<Transform2D>>>;

    /// First lifecycle method during each render loop, used to compute
    /// properties in advance of rendering.
    /// Occurs in a pre-order traversal of the render tree.
    fn compute_properties(&mut self, _rtc: &mut RenderTreeContext<R>) {
        //no-op default implementation
    }

    /// Used by elements that need to communicate across native rendering bridge (for example: Text, Clipping masks, scroll containers)
    /// Called by engine after `compute_properties`, passed calculated size and transform matrix coefficients for convenience
    /// Expected to induce side-effects (if appropriate) via enqueueing messages to the native message queue
    ///
    /// An implementor of `compute_native_patches` is responsible for determining which properties if any have changed
    /// (e.g. by keeping a local patch object as a cache of last known values.)
    fn compute_native_patches(&mut self, rtc: &mut RenderTreeContext<R>, computed_size: (f64, f64), transform_coeffs: Vec<f64>) {
        //no-op default implementation
    }

    /// Second lifecycle method during each render loop, occurs AFTER
    /// properties have been computed, but BEFORE rendering
    /// Example use-case: perform side-effects to the drawing context.
    /// This is how [`Frame`] performs clipping, for example.
    /// Occurs in a pre-order traversal of the render tree.
    fn handle_will_render(&mut self, _rtc: &mut RenderTreeContext<R>, _rc: &mut R) {
        //no-op default implementation
    }

    /// Third lifecycle method during each render loop, occurs
    /// AFTER all descendents have been rendered.
    /// Occurs in a post-order traversal of the render tree. Most primitives
    /// are expected to draw their contents to the rendering context during this event.
    fn handle_render(&self, _rtc: &mut RenderTreeContext<R>, _rc: &mut R) {
        //no-op default implementation
    }

    /// Fourth and final lifecycle method during each render loop, occurs
    /// AFTER all descendents have been rendered AND the current node has been rendered.
    /// Useful for clean-up, e.g. this is where `Frame` cleans up the drawing context
    /// to stop clipping.
    /// Occurs in a post-order traversal of the render tree.
    fn handle_did_render(&mut self, _rtc: &mut RenderTreeContext<R>, _rc: &mut R) {
        //no-op default implementation
    }


    /// Fires during the tick when a node is first attached to the render tree.  For example,
    /// this event fires by all nodes on the global first tick, and by all nodes in a subtree
    /// when a `Conditional` subsequently turns on a subtree (i.e. when the `Conditional`s criterion becomes `true` after being `false` through the end of at least 1 frame.)
    /// A use-case: send a message to native renderers that a `Text` element should be rendered and tracked
    fn handle_did_mount(&mut self, _rtc: &mut RenderTreeContext<R>) {
        //no-op default implementation
    }

    /// Fires during element unmount, when an element is about to be removed from the render tree (e.g. by a `Conditional`)
    /// A use-case: send a message to native renderers that a `Text` element should be removed
    fn handle_will_unmount(&mut self, _rtc: &mut RenderTreeContext<R>) {
        //no-op default implementation
    }

}

pub trait LifecycleNode {


}

use pax_runtime_api::Transform2D;



pub trait ComputableTransform {
    fn compute_transform_matrix(&self, node_size: (f64, f64), container_bounds: (f64, f64)) -> (Affine,Affine);
}

impl ComputableTransform for Transform2D {
    //Distinction of note: scale, translate, rotate, anchor, and align are all AUTHOR-TIME properties
    //                     node_size and container_bounds are (computed) RUNTIME properties
    //Returns (Base affine transform, align component)
    fn compute_transform_matrix(&self, node_size: (f64, f64), container_bounds: (f64, f64)) -> (Affine,Affine)  {
        let anchor_transform = match &self.anchor {
            Some(anchor) => {
                Affine::translate(
                    (
                        match anchor[0] {
                            Size::Pixels(x) => {
                                -x.get_as_float()
                            },
                            Size::Percent(x) => {
                                -node_size.0 * (x / 100.0)
                            },
                        },
                        match anchor[1] {
                            Size::Pixels(y) => {
                                -y.get_as_float()
                            },
                            Size::Percent(y) => {
                                -node_size.1 * (y / 100.0)
                            },
                        }
                    )
                )
            },
            //No anchor applied: treat as 0,0; identity matrix
            None => {Affine::default()}
        };

        let mut transform = Affine::default();
        if let Some(rotate) = &self.rotate {
            transform = transform * Affine::rotate(*rotate);
        }
        if let Some(scale) = &self.scale {
            transform = transform * Affine::scale_non_uniform(scale[0], scale[1]);
        }
        if let Some(translate) = &self.translate {
            transform = transform * Affine::translate((translate[0], translate[1]));
        }

        //if this has an align component, return it.else {if previous has an align component, return it }



        let (previous_transform, previous_align_component) = match &self.previous {
            Some(previous) => {(*previous).compute_transform_matrix(node_size, container_bounds)},
            None => {(Affine::default(), Affine::default())},
        };

        let align_component = match &self.align {
            Some(align) => {
                let x_percent = if let Size::Percent(x) = align[0] {x/100.0} else {panic!("Align requires a Size::Percent value")};
                let y_percent = if let Size::Percent(y) = align[1] {y/100.0} else {panic!("Align requires a Size::Percent value")};
                Affine::translate((x_percent * container_bounds.0, y_percent * container_bounds.1))},
            None => {
                previous_align_component //which defaults to identity
            }
        };

        //align component is passed separately because it is global for a given sequence of Transform operations
        (anchor_transform * transform * previous_transform, align_component)
    }

}

/// Represents the outer stroke of a drawable element
pub struct StrokeInstance {
    pub color: Color,
    pub width: f64,
    pub style: StrokeStyle,
    //FUTURE: stroke alignment, inner/outer/center?
}

