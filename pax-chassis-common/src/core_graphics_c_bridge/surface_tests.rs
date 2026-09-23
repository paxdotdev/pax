use super::*;

fn plan_at_scale(extent: f64, dpr: f64) -> pax_runtime::engine::layer_tiling::LayerCanvasPlan {
    scroller_canvas_plan_with_policy(
        1,
        "scroller:graph".into(),
        extent,
        extent,
        370.0,
        260.0,
        extent * 0.5 - 185.0,
        extent * 0.5 - 130.0,
        dpr,
        native_surface_tiling_policy(native_scroller_tiling_policy(), dpr),
    )
}

#[test]
fn graph_zoom_transitions_respect_native_backing_limits() {
    for dpr in [1.0, 2.0, 3.0] {
        // The first zoom-out used to replace the tiles with a 4096-point surface,
        // which the Swift host materialized as 12288 pixels square on a 3x phone.
        for extent in [8192.0, 4096.0, 2048.0, 4096.0, 8192.0, 131072.0] {
            let plan = plan_at_scale(extent, dpr);
            assert!(plan.active);
            assert!(!plan.surfaces.is_empty());
            assert!(plan.surfaces.len() <= 12);
            for surface in &plan.surfaces {
                let width = (surface.width * dpr).round();
                let height = (surface.height * dpr).round();
                assert!(width <= 4096.0, "{extent} points at {dpr}x: {width}px wide");
                assert!(
                    height <= 4096.0,
                    "{extent} points at {dpr}x: {height}px tall"
                );
                assert!(width * height <= 4096.0 * 4096.0);
            }
            // Both opposite viewport corners must stay covered when the tile
            // topology changes; a bounded plan cannot simply drop visible tiles.
            for (x, y) in [
                (extent * 0.5 - 185.0, extent * 0.5 - 130.0),
                (extent * 0.5 + 185.0, extent * 0.5 + 130.0),
            ] {
                assert!(plan.surfaces.iter().any(|surface| {
                    x >= surface.left
                        && x <= surface.left + surface.width
                        && y >= surface.top
                        && y <= surface.top + surface.height
                }));
            }
        }
    }
}

#[test]
fn small_native_content_keeps_a_single_surface() {
    for dpr in [1.0, 2.0, 3.0] {
        let plan = plan_at_scale(512.0, dpr);
        assert_eq!(plan.surfaces.len(), 1);
        assert_eq!(plan.surfaces[0].key, "single");
    }
}
