//! Basic example of rendering in the browser

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlCanvasElement};
use std::rc::Rc;
use std::cell::RefCell;

use piet_web::WebRenderContext;

use pax::PaxEngine;

fn browser_window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

// fn request_animation_frame(f: &Closure<dyn FnMut()>) {
//     browser_window()
//         .request_animation_frame(f.as_ref().unchecked_ref())
//         .expect("should register `requestAnimationFrame` OK");
// }

// Console.log support
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
    // #[wasm_bindgen(skip)]
    engine: Rc<RefCell<PaxEngine>>,
    drawing_context: WebRenderContext<'static>,
}

#[wasm_bindgen]
impl PaxChassisWeb {
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
        let context : web_sys::CanvasRenderingContext2d = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();

        let dpr = window.device_pixel_ratio();
        let width = canvas.offset_width() as f64 * dpr;
        let height = canvas.offset_height() as f64 * dpr;
        //TODO:  update these values on window resize
        //       future:  update these values on _element_ resize
        canvas.set_width(width as u32);
        canvas.set_height(height as u32);

        let _ = context.scale(dpr, dpr);

        let piet_context  = WebRenderContext::new(context, window);
        // piet_context.
        let engine = pax::get_engine(log_wrapper, (width / dpr, height / dpr));

        let engine_container : Rc<RefCell<PaxEngine>> = Rc::new(RefCell::new(engine));

        //see web-sys docs for handling browser events with closures
        //https://rustwasm.github.io/docs/wasm-bindgen/examples/closures.html
        {
            let engine_rc_pointer = engine_container.clone();
            let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                let mut engine = engine_rc_pointer.borrow_mut();

                //TODO:  can probably tackle this more elegantly by reusing / capturing / Rc-ing
                //       previously declared window / canvas / context / etc.
                let inner_window = web_sys::window().unwrap();
                // let inner_canvas = inner_window
                //     .document()
                //     .unwrap()
                //     .get_element_by_id("canvas")
                //     .unwrap()
                //     .dyn_into::<HtmlCanvasElement>()
                //     .unwrap();
                // let inner_context = inner_canvas
                //     .get_context("2d")
                //     .unwrap()
                //     .unwrap()
                //     .dyn_into::<web_sys::CanvasRenderingContext2d>()
                //     .unwrap();

                //inner_width and inner_height already account for device pixel ratio.
                let width = inner_window.inner_width().unwrap().as_f64().unwrap();
                let height = inner_window.inner_height().unwrap().as_f64().unwrap();
                let _ = canvas.set_attribute("width", format!("{}",width).as_str());
                let _ = canvas.set_attribute("height", format!("{}",height).as_str());
                engine.set_viewport_size((width, height));
            }) as Box<dyn FnMut(_)>);
            let inner_window = web_sys::window().unwrap();
            let _ = inner_window.add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref());
            closure.forget();
        }

        PaxChassisWeb {
            engine: engine_container,
            drawing_context: piet_context,
        }
    }

    //TODO: accept array of input messages, e.g. representing changes in user/input state
    pub fn tick(&mut self) -> Vec<JsValue> {
        self.engine.borrow_mut().tick(&mut self.drawing_context)
    }
}
