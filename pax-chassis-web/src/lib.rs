//! Basic example of rendering in the browser

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlCanvasElement};
use js_sys::Uint8Array;
use std::rc::Rc;
use std::cell::RefCell;

use piet_web::WebRenderContext;

use pax_core::{InstanceRegistry, PaxEngine};

use serde_json;
use pax_message::{ImageLoadInterruptArgs, NativeInterrupt};
use pax_runtime_api::{ArgsClick, ArgsScroll};

// Console.log support, piped from `pax::log`
#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    // The `console.log` is quite polymorphic, so we can bind it with multiple
    // signatures. Note that we need to use `js_name` to ensure we always call
    // `log` in JS.
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_u32(a: u32);

    // Multiple arguments too!
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_many(a: &str, b: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

pub fn log_wrapper(msg: &str) {
    console_log!("{}", msg);
}

#[wasm_bindgen]
pub struct PaxChassisWeb {
    engine: Rc<RefCell<PaxEngine<WebRenderContext<'static>>>>,
    drawing_contexts: Vec<WebRenderContext<'static>>,
}

#[wasm_bindgen]
impl PaxChassisWeb {
    //called from JS, this is essentially `main`
    pub fn new() -> Self {

        #[cfg(feature = "console_error_panic_hook")]
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        let window = window().unwrap();
        let dpr = window.device_pixel_ratio();
        let document = window.document().unwrap();
        let canvases = document.get_elements_by_class_name("canvas");
        let canvas = canvases.item(0).unwrap().dyn_into::<HtmlCanvasElement>().unwrap();
        let width = canvas.offset_width() as f64 * dpr;
        let height = canvas.offset_height() as f64 * dpr;

        let instance_registry : Rc<RefCell<InstanceRegistry<WebRenderContext>>> = Rc::new(RefCell::new(InstanceRegistry::new()));
        let root_component_instance = pax_cartridge::instantiate_root_component(Rc::clone(&instance_registry));
        let expression_table = pax_cartridge::instantiate_expression_table();

        let engine = pax_core::PaxEngine::new(root_component_instance, expression_table, pax_runtime_api::PlatformSpecificLogger::Web(log_wrapper), (width / dpr, height / dpr), instance_registry);

        let engine_container : Rc<RefCell<PaxEngine<WebRenderContext>>> = Rc::new(RefCell::new(engine));

        let render_contexts = PaxChassisWeb::initializeContexts(canvases.length(), &engine_container, &vec![]);

        Self {
            engine: engine_container,
            drawing_contexts: render_contexts,
        }
    }

    fn initializeContexts(num_canvases: u32, engine: &Rc<RefCell<PaxEngine<WebRenderContext>>>, current_contexts: &Vec<WebRenderContext>) -> Vec<WebRenderContext<'static>>{
        let mut new_render_contexts = Vec::new();
        let window = window().unwrap();
        let dpr = window.device_pixel_ratio();
        let document = window.document().unwrap();
        let canvases = document.get_elements_by_class_name("canvas");
        let mut width = 0.0;
        let mut height = 0.0;

        let starting_index: u32 = current_contexts.len() as u32;

        for i in 0..num_canvases {
            let index = starting_index + i;
            let canvas = canvases.item(index).unwrap().dyn_into::<HtmlCanvasElement>().unwrap();

            let context: web_sys::CanvasRenderingContext2d = canvas
                .get_context("2d")
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::CanvasRenderingContext2d>()
                .unwrap();

            width = canvas.offset_width() as f64 * dpr;
            height = canvas.offset_height() as f64 * dpr;

            canvas.set_width(width as u32);
            canvas.set_height(height as u32);

            let _ = context.scale(dpr, dpr);
            let render_context = WebRenderContext::new(context, window.clone());
            new_render_contexts.push(render_context);
        }

        let engine_cloned = Rc::clone(engine);
        //see web-sys docs for handling browser events with closures
        //https://rustwasm.github.io/docs/wasm-bindgen/examples/closures.html
        {
            let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                let mut engine = engine_cloned.borrow_mut();
                let inner_window = web_sys::window().unwrap();

                //inner_width and inner_height already account for device pixel ratio.
                let width = inner_window.inner_width().unwrap().as_f64().unwrap();
                let height = inner_window.inner_height().unwrap().as_f64().unwrap();

                //handle window resize
                for i in 0..canvases.length() {
                    let canvas = canvases.item(i).unwrap().dyn_into::<HtmlCanvasElement>().unwrap();
                    let _ = canvas.set_attribute("width", format!("{}",width).as_str());
                    let _ = canvas.set_attribute("height", format!("{}",height).as_str());

                }

                engine.set_viewport_size((width, height));
            }) as Box<dyn FnMut(_)>);
            let inner_window = web_sys::window().unwrap();

            //attach handler closure to DOM `window` `resize` event
            let _ = inner_window.add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref());
            closure.forget();
        }
        new_render_contexts
    }

    pub fn interrupt(&mut self, native_interrupt: String, additional_payload: &JsValue) {
        let x : NativeInterrupt = serde_json::from_str(&native_interrupt).unwrap();
        match x {
            NativeInterrupt::Click(args) => {
                let prospective_hit = (*self.engine).borrow().get_topmost_hydrated_element_beneath_ray((args.x, args.y));
                match prospective_hit {
                    Some(topmost_node) => {
                        let args_click = ArgsClick {x: args.x , y: args.y};
                        topmost_node.dispatch_click(args_click);
                    },
                    _ => {},
                };
            },
            NativeInterrupt::Scroll(args) => {
                let prospective_hit = (*self.engine).borrow().get_topmost_hydrated_element_beneath_ray((args.x, args.y));
                match prospective_hit {
                    Some(topmost_node) => {
                        let args_scroll = ArgsScroll {delta_x: args.delta_x, delta_y: args.delta_y};
                        topmost_node.dispatch_scroll(args_scroll);
                    },
                    _ => {},
                };
            }
            NativeInterrupt::Image(args) => {
                match args {
                    ImageLoadInterruptArgs::Reference(ref_args) => {}
                    ImageLoadInterruptArgs::Data(data_args) => {
                        let data = Uint8Array::new(additional_payload).to_vec();
                        (*self.engine).borrow_mut().loadImage(data_args.id_chain, data, data_args.width, data_args.height);
                    }
                }
            }
            NativeInterrupt::AddedLayer(args) => {
                let mut new_contexts = PaxChassisWeb::initializeContexts(args.num_layers_added, &self.engine, &self.drawing_contexts);
                self.drawing_contexts.append(&mut new_contexts);
            }
        }
    }

    pub fn tick(&mut self) -> String {
        let message_queue = self.engine.borrow_mut().tick(&mut self.drawing_contexts);
        //Note that this approach likely carries some CPU overhead, but may be suitable.
        //See zb lab journal `On robust message-passing to web` May 11 2022
        serde_json::to_string(&message_queue).unwrap()
    }

}
