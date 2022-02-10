use std::cell::RefCell;
use std::rc::Rc;

use kurbo::{
    BezPath,
    Point,
    Vec2,
};
use piet::RenderContext;
use piet_web::WebRenderContext;

use crate::{Affine, ComponentInstance, Color, Error, ComputableTransform,  RenderNodePtr, StrokeInstance, StrokeStyle, RenderNode};
use crate::runtime::{Runtime};
//TODO: make the JsValue render_message_queue platform agnostic and remove this dep —
//      (probably translate to JsValue at the pax-chassis-web layer instead of here.)
use wasm_bindgen::JsValue;

use pax_runtime_api::StringReceiver;


// Public method for consumption by engine chassis, e.g. WebChassis
pub fn get_engine(root_component_instance: Rc<RefCell<ComponentInstance>>,logger: fn(&str), viewport_size: (f64, f64)) -> PaxEngine {
    PaxEngine::new(root_component_instance, logger,viewport_size)
}


pub struct PaxEngine {
    pub frames_elapsed: usize,
    //used to communicate between cartridge & runtime,
    //namely which property ID to calculate next

    pub root_component: Rc<RefCell<ComponentInstance>>, //NOTE: to support multiple concurrent "root components," e.g. for multi-stage authoring, this could simply be made an array of `root_components`
    pub runtime: Rc<RefCell<Runtime>>,
    viewport_size: (f64, f64),
}

#[derive(Clone)]
pub struct RenderTreeContext<'a>
{
    pub engine: &'a PaxEngine,
    pub transform: Affine,
    pub bounds: (f64, f64),
    pub runtime: Rc<RefCell<Runtime>>,
    pub node: RenderNodePtr,
    pub timeline_playhead_position: usize,
    pub property_id_register: Option<String>,

}


impl<'a> StringReceiver for RenderTreeContext<'a> {
    fn receive(&mut self, value: String) {
        self.property_id_register = Some(value);
    }
}

pub struct HostPlatformContext<'a, 'b>
{
    pub drawing_context: &'a mut WebRenderContext<'b>,
    pub render_message_queue: Vec<JsValue>, //TODO: platform polyfill
}

impl PaxEngine {
    fn new(root_component_instance: Rc<RefCell<ComponentInstance>>, logger: fn(&str), viewport_size: (f64, f64)) -> Self {
        PaxEngine {
            frames_elapsed: 0,
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

    fn traverse_render_tree(&self, rc: &mut WebRenderContext) -> Vec<JsValue> {
        // Broadly:
        // 1. compute properties
        // 2. find lowest node (last child of last node), accumulating transform along the way
        // 3. start rendering, from lowest node on-up


        let mut hpc = HostPlatformContext {
            drawing_context: rc,
            render_message_queue: Vec::new(),
        };

        let cast_component_rc : Rc<RefCell<dyn RenderNode>> = self.root_component.clone();

        let mut rtc = RenderTreeContext {
            engine: &self,
            transform: Affine::default(),
            bounds: self.viewport_size,
            runtime: self.runtime.clone(),
            node: Rc::clone(&cast_component_rc),
            timeline_playhead_position: self.frames_elapsed,
            property_id_register: None,
        };

        &self.recurse_traverse_render_tree(&mut rtc, &mut hpc, Rc::clone(&cast_component_rc));

        hpc.render_message_queue
    }

    fn recurse_traverse_render_tree(&self, rtc: &mut RenderTreeContext, hpc: &mut HostPlatformContext, node: RenderNodePtr)  {
        // Recurse:
        //  - compute properties for this node
        //  - iterate backwards over children (lowest first); recurse until there are no more descendants.  track transform matrix & bounding dimensions along the way.
        //  - we now have the back-most leaf node.  Render it.  Return.
        //  - we're now at the second back-most leaf node.  Render it.  Return ...
        //  - done with this frame

        //populate a pointer to this (current) `RenderNode` onto `rtc`
        rtc.node = Rc::clone(&node);

        //peek at the current stack frame and set a scoped playhead position as needed
        match rtc.runtime.borrow_mut().peek_stack_frame() {
            Some(stack_frame) => {
                rtc.timeline_playhead_position = stack_frame.borrow_mut().get_timeline_playhead_position();
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
            node_borrowed.get_transform().borrow_mut().get()
            .compute_transform_matrix(
                node_size,
                accumulated_bounds,
            )
        };

        let new_accumulated_transform = accumulated_transform * node_computed_transform;

        rtc.bounds = new_accumulated_bounds;
        rtc.transform = new_accumulated_transform;

        //lifecycle: pre_render
        node.borrow_mut().pre_render(rtc, hpc);

        let children = node.borrow_mut().get_rendering_children();

        //keep recursing through children
        children.borrow_mut().iter().rev().for_each(|child| {
            //note that we're iterating starting from the last child, for z-index (.rev())
            let mut new_rtc = rtc.clone();
            &self.recurse_traverse_render_tree(&mut new_rtc, hpc, Rc::clone(child));
            //TODO: for dependency management, return computed values from subtree above
        });

        // lifecycle: `render`
        // this is this node's time to do its own rendering, aside
        // from its children.  Its children have already been rendered.
        node.borrow_mut().render(rtc, hpc);

        // lifecycle: post_render
        node.borrow_mut().post_render(rtc, hpc);
    }

    pub fn set_viewport_size(&mut self, new_viewport_size: (f64, f64)) {
        self.viewport_size = new_viewport_size;
    }

    pub fn tick(&mut self, rc: &mut WebRenderContext) -> Vec<JsValue> {
        rc.clear(Color::rgb8(0, 0, 0));
        let render_queue = self.traverse_render_tree(rc);
        self.frames_elapsed = self.frames_elapsed + 1;
        render_queue
    }

    //keeping until this can be done via scene graph
    pub fn tick_and_render_disco_taps(&mut self, rc: &mut WebRenderContext) -> Result<(), Error> {
        let hue = (((self.frames_elapsed + 1) as f64 * 2.0) as i64 % 360) as f64;
        let current_color = Color::hlc(hue, 75.0, 127.0);
        rc.clear(current_color);

        for x in 0..20 {
            for y in 0..12 {
                let bp_width: f64 = 100.;
                let bp_height: f64 = 100.;
                let mut bez_path = BezPath::new();
                bez_path.move_to(Point::new(-bp_width / 2., -bp_height / 2.));
                bez_path.line_to(Point::new(bp_width / 2., -bp_height / 2.));
                bez_path.line_to(Point::new(bp_width / 2., bp_height / 2.));
                bez_path.line_to(Point::new(-bp_width / 2., bp_height / 2.));
                bez_path.line_to(Point::new(-bp_width / 2., -bp_height / 2.));
                bez_path.close_path();

                let theta = self.frames_elapsed as f64 * (0.04 + (x as f64 + y as f64 + 10.) / 64.) / 10.;
                let transform =
                    Affine::translate(Vec2::new(x as f64 * bp_width, y as f64 * bp_height)) *
                        Affine::rotate(theta) *
                        Affine::scale(theta.sin() * 1.2)
                    ;

                let transformed_bez_path = transform * bez_path;

                let phased_hue = ((hue + 180.) as i64 % 360) as f64;
                let phased_color = Color::hlc(phased_hue, 75., 127.);
                rc.fill(transformed_bez_path, &phased_color);
            }
        }

        self.frames_elapsed = self.frames_elapsed + 1;
        Ok(())
    }
}

/*****************************/
/* Codegen (macro) territory */

//OR: revisit this approach, without variadics.
//
// pub struct MyManualMacroExpression<T> {
//     pub variadic_evaluator: fn(engine: &PaxEngine) -> T,
// }
//
// //TODO:  should this hard-code the return type
// impl<T> MyManualMacroExpression<T> {
//
// }
//
// impl<T> Evaluator<T> for MyManualMacroExpression<T> {
//     fn inject_and_evaluate(&self, ic: &InjectionContext) -> T {
//         //TODO:CODEGEN
//         //       pull necessary data from `ic`,
//         //       map into the variadic args of elf.variadic_evaluator()
//         //       Perhaps this is a LUT of `String => (Fn(njectionContext) -> V)` for any variadic type (injection tream) V
//
//         let x = ||{};
//         let y = |x: i64| x + 6;
//
//
//         let engine = ic.engine;
//         (self.variadic_evaluator)(engine)
//
//
//     }
// }


/* End codegen (macro) territory */
/*********************************/
