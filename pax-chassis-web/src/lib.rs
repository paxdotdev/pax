//! Basic example of rendering in the browser

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, SvgElement, HtmlCanvasElement};
use std::rc::Rc;
use std::cell::RefCell;

use piet_web::WebRenderContext;
use serde::Serialize;

use pax_core::{InstanceRegistry, PaxEngine};
use pax_message::NativeMessage;

use serde_json;

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
    drawing_context: WebRenderContext<'static>,
}

#[wasm_bindgen]
impl PaxChassisWeb {
    //called from JS, this is essentially `main`
    pub fn new() -> Self {

        #[cfg(feature = "console_error_panic_hook")]
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));

        let window = window().unwrap();
        let canvas = window
            .document()
            .unwrap()
            .get_element_by_id("canvas")
            .unwrap()
            .dyn_into::<HtmlCanvasElement>()
            .unwrap();
        //
        let clipping_layer = window
            .document()
            .unwrap()
            .get_element_by_id("clipping-layer")
            .unwrap()
            .dyn_into::<SvgElement>()
            .unwrap();

        let context : web_sys::CanvasRenderingContext2d = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();

        let dpr = window.device_pixel_ratio();
        let width = canvas.offset_width() as f64 * dpr;
        let height = canvas.offset_height() as f64 * dpr;

        canvas.set_width(width as u32);
        canvas.set_height(height as u32);

        clipping_layer.set_attribute("width", &format!("{}", width));
        clipping_layer.set_attribute("height", &format!("{}", height));

        let _ = context.scale(dpr, dpr);

        let render_context = WebRenderContext::new(context, window);

        let instance_registry : Rc<RefCell<InstanceRegistry<WebRenderContext>>> = Rc::new(RefCell::new(InstanceRegistry::new()));
        let root_component_instance = pax_cartridge::instantiate_root_component(Rc::clone(&instance_registry));
        let expression_table = pax_cartridge::instantiate_expression_table();

        let engine = pax_core::PaxEngine::new(root_component_instance, expression_table, pax_runtime_api::PlatformSpecificLogger::Web(log_wrapper), (width / dpr, height / dpr), instance_registry);

        let engine_container : Rc<RefCell<PaxEngine<WebRenderContext>>> = Rc::new(RefCell::new(engine));

        let engine_cloned = Rc::clone(&engine_container);
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
                let _ = canvas.set_attribute("width", format!("{}",width).as_str());
                let _ = canvas.set_attribute("height", format!("{}",height).as_str());

                let _ = clipping_layer.set_attribute("width", format!("{}",width).as_str());
                let _ = clipping_layer.set_attribute("height", format!("{}",height).as_str());

                engine.set_viewport_size((width, height));
            }) as Box<dyn FnMut(_)>);
            let inner_window = web_sys::window().unwrap();

            //attach handler closure to DOM `window` `resize` event
            let _ = inner_window.add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref());
            closure.forget();
        }

        Self {
            engine: engine_container,
            drawing_context: render_context,
        }
    }

    pub fn tick(&mut self) -> String {
        let message_queue = self.engine.borrow_mut().tick(&mut self.drawing_context);
        //Note that this approach likely carries some CPU overhead, but may be suitable.
        //See zb lab journal `On robust message-passing to web` May 11 2022
        serde_json::to_string(&message_queue).unwrap()
    }
}
