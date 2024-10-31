use std::pin::Pin;

use pax_runtime::api::RenderContext;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, Window};

#[cfg(not(feature = "gpu"))]
pub fn get_render_context(window: Window) -> impl RenderContext {
    use pax_runtime::piet_render_context::PietRenderer;
    use piet_web::WebRenderContext;
    PietRenderer::new(move |layer| {
        let dpr = window.device_pixel_ratio();
        let document = window.document().unwrap();
        let canvas = document
            .get_element_by_id(layer.to_string().as_str())
            .unwrap()
            .dyn_into::<HtmlCanvasElement>()
            .unwrap();
        let context: web_sys::CanvasRenderingContext2d = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();

        let width = canvas.offset_width() as f64 * dpr;
        let height = canvas.offset_height() as f64 * dpr;

        canvas.set_width(width as u32);
        canvas.set_height(height as u32);
        let _ = context.scale(dpr, dpr);

        (
            WebRenderContext::new(context.clone(), window.clone()),
            // clear fn
            Box::new({
                let context = context.clone();
                let canvas = canvas.clone();
                move || {
                    let w = canvas.width();
                    let h = canvas.height();
                    context.clear_rect(0.0, 0.0, w as f64, h as f64);
                }
            }),
            // resize fn
            Box::new({
                let window = window.clone();
                move || {
                    let dpr = window.device_pixel_ratio();
                    canvas.set_width((canvas.client_width() as f64 * dpr) as u32);
                    canvas.set_height((canvas.client_height() as f64 * dpr) as u32);
                    let _ = context.scale(dpr, dpr);
                }
            }),
        )
    })
}

#[cfg(feature = "gpu")]
pub fn get_render_context(window: Window) -> impl RenderContext {
    use pax_pixels::{
        render_backend::{RenderBackend, RenderConfig},
        WgpuRenderer,
    };
    use pax_runtime::pax_pixels_render_context::PaxPixelsRenderer;
    PaxPixelsRenderer::new(move |layer| {
        let window = window.clone();
        Box::pin(async move {
            let document = window.document().unwrap();
            let canvas = match document
                .get_element_by_id(layer.to_string().as_str())
                .and_then(|e| e.dyn_into::<HtmlCanvasElement>().ok())
            {
                Some(canvas) => canvas,
                None => {
                    log::warn!("failed to attach renderer: canvas doesn't exist yet");
                    return None;
                }
            };

            let width = canvas.offset_width() as f64;
            let height = canvas.offset_height() as f64;
            canvas.set_width(width as u32);
            canvas.set_height(height as u32);

            let res = WgpuRenderer::new(
                // NOTE: this exists when building for wasm32
                RenderBackend::to_canvas(
                    canvas.clone(),
                    RenderConfig::new(false, width as u32, height as u32, 1),
                )
                .await
                .ok()?,
            );
            Some((
                res,
                // resize fn
                Box::pin({
                    let window = window.clone();
                    move || {
                        let dpr = window.device_pixel_ratio();
                        canvas.set_width((canvas.client_width() as f64 * dpr) as u32);
                        canvas.set_height((canvas.client_height() as f64 * dpr) as u32);
                    }
                }) as Pin<Box<dyn Fn()>>,
            ))
        })
    })
}
