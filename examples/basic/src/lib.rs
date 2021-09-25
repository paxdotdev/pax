//! Basic example of rendering in the browser

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlCanvasElement};
use std::rc::Rc;
use std::cell::RefCell;

use piet_web::WebRenderContext;

use carbon::CarbonEngine;

fn browser_window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    browser_window()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK");
}

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

#[wasm_bindgen]
pub fn run() {
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
    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    // let sample = carbon SamplePicture::new(picture_0::SIZE, picture_0::draw);

    // let sample = samples::get::<WebRenderContext>(SAMPLE_PICTURE_NO).unwrap();
    let dpr = window.device_pixel_ratio();
    canvas.set_width((canvas.offset_width() as f64 * dpr) as u32);
    canvas.set_height((canvas.offset_height() as f64 * dpr) as u32);
    let _ = context.scale(dpr, dpr);

    let mut piet_context  = WebRenderContext::new(context, window);
    // sample.draw(&mut piet_context).unwrap();
    // piet_context.finish().unwrap();
    let engine = carbon::get_engine();
    console_log!("Hello!");
    drawing_loop(engine,piet_context);
}
//
//
// fn request_animation_frame(f: &Closure<dyn FnMut()>) {
//     window()
//         .request_animation_frame.as_ref().unchecked_ref()
//         .expect("should register `requestAnimationFrame` OK");
// }

pub fn drawing_loop(engine: CarbonEngine, mut piet_context: WebRenderContext) -> Result<(), JsValue> {
    engine.tick_and_render(piet_context).unwrap();
    // browser_window().request_animation_frame(Closure::wrap(drawing_loop.as_ref);




    let f = Rc::new(RefCell::new(None));
    let g = f.clone();

    let mut i = 0;
    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        if i > 300 {
            console_log!("All done!");

            // Drop our handle to this closure so that it will get cleaned
            // up once we return.
            let _ = f.borrow_mut().take();
            return;
        }

        // Set the body's text content to how many times this
        // requestAnimationFrame callback has fired.
        i += 1;
        let msg = format!("requestAnimationFrame has been called {} times.", i);
        console_log!("{}", msg);

        // Schedule ourself for another requestAnimationFrame callback.
        request_animation_frame(f.borrow().as_ref().unwrap());
    }) as Box<dyn FnMut()>));

    request_animation_frame(g.borrow().as_ref().unwrap());

    Ok(())
}
