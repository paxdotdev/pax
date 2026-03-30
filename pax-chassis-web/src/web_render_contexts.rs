use std::pin::Pin;

use pax_runtime::api::RenderContext;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, Window};

struct SurfaceMetrics {
    logical_width: f32,
    logical_height: f32,
    surface_width: u32,
    surface_height: u32,
    dpr: f32,
}

fn compute_surface_metrics(
    logical_width: f64,
    logical_height: f64,
    desired_dpr: f64,
    max_surface_dimension: u32,
) -> SurfaceMetrics {
    let logical_width = logical_width.max(0.0);
    let logical_height = logical_height.max(0.0);
    let desired_dpr = desired_dpr.max(1.0);
    let max_surface_dimension = max_surface_dimension.max(1) as f64;

    let width_limit = if logical_width > 0.0 {
        max_surface_dimension / logical_width
    } else {
        desired_dpr
    };
    let height_limit = if logical_height > 0.0 {
        max_surface_dimension / logical_height
    } else {
        desired_dpr
    };
    let dpr = desired_dpr.min(width_limit).min(height_limit).max(1.0);

    if dpr + 0.01 < desired_dpr {
        log::warn!(
            "render backend: clamped device pixel ratio from {:.2} to {:.2} to fit max surface dimension {}",
            desired_dpr,
            dpr,
            max_surface_dimension as u32
        );
    }

    SurfaceMetrics {
        logical_width: logical_width as f32,
        logical_height: logical_height as f32,
        surface_width: ((logical_width * dpr).round() as u32).max(1),
        surface_height: ((logical_height * dpr).round() as u32).max(1),
        dpr: dpr as f32,
    }
}

#[cfg(feature = "piet")]
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

#[cfg(not(feature = "piet"))]
pub fn get_render_context(window: Window) -> impl RenderContext {
    use pax_pixels::{
        render_backend::{RenderBackend, RenderConfig},
        WgpuRenderer,
    };
    use pax_runtime::pax_pixels_render_context::{LayerSurfaceSize, PaxPixelsRenderer};
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
            canvas.set_width(1);
            canvas.set_height(1);

            let backend = match RenderBackend::to_canvas(
                canvas.clone(),
                RenderConfig::new(false, 1, 1, 1.0),
            )
            .await
            {
                Ok(backend) => backend,
                Err(err) => {
                    log::warn!(
                        "failed to create browser render backend for layer {}: {}",
                        layer,
                        err
                    );
                    return None;
                }
            };

            let mut res = WgpuRenderer::new(backend);
            let max_surface_dimension = res.max_surface_dimension();
            let surface = compute_surface_metrics(
                width,
                height,
                window.device_pixel_ratio(),
                max_surface_dimension,
            );
            canvas.set_width(surface.surface_width);
            canvas.set_height(surface.surface_height);
            res.resize_surface(surface.surface_width as f32, surface.surface_height as f32);
            res.set_viewport(surface.logical_width, surface.logical_height, surface.dpr);
            Some((
                res,
                // resize fn
                Box::pin({
                    let window = window.clone();
                    move || {
                        let surface = compute_surface_metrics(
                            canvas.client_width() as f64,
                            canvas.client_height() as f64,
                            window.device_pixel_ratio(),
                            max_surface_dimension,
                        );
                        canvas.set_width(surface.surface_width);
                        canvas.set_height(surface.surface_height);
                        LayerSurfaceSize {
                            surface_width: surface.surface_width,
                            surface_height: surface.surface_height,
                            dpr: surface.dpr,
                        }
                    }
                }) as Pin<Box<dyn Fn() -> LayerSurfaceSize>>,
            ))
        })
    })
}
