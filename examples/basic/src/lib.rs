//! Basic example of rendering in the browser

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlCanvasElement};

use piet::RenderContext;
use piet_web::WebRenderContext;

use carbon;

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

    let mut piet_context = WebRenderContext::new(context, window);

    // sample.draw(&mut piet_context).unwrap();
    // piet_context.finish().unwrap();
    let engine = carbon::get_engine();
    drawingLoop(engine,piet_context);
}
//
//
// fn request_animation_frame(f: &Closure<dyn FnMut()>) {
//     window()
//         .request_animation_frame.as_ref().unchecked_ref()
//         .expect("should register `requestAnimationFrame` OK");
// }

pub fn drawingLoop(CarbonEngine: engine, piet_context: WebRenderContext) -> Result<(), JsValue> {
    engine.tick_and_render(&mut piet_context).unwrap();
    window
}
