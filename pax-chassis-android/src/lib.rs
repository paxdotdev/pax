//! Android Activity-owned sessions and native-window GPU presentation.
//!
//! The Activity explicitly initializes its cartridge on the UI thread. Surface
//! loss suspends presentation independently of the application's retained root.
//! Native Android Views remain responsible for controls, accessibility and IME.

#[cfg(any(target_os = "android", test))]
mod rich_text;

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "android")]
pub use android::*;

#[cfg(any(target_os = "android", test))]
fn tiling_policy() -> pax_runtime::engine::layer_tiling::ScrollerTilingPolicy {
    let mut policy = pax_chassis_common::desktop::native_scroller_tiling_policy();
    // TextureView presentation retains swapchain copies in addition to renderer
    // backings. A desktop-sized warm region is too costly on a phone; preserve
    // device-pixel fidelity and reduce only offscreen scroll preparation.
    policy.prewarm_viewport_pad_x_multiplier = 0.25;
    policy.prewarm_viewport_pad_y_multiplier = 0.25;
    policy.prewarm_viewport_pad_min_x = 128.0;
    policy.prewarm_viewport_pad_min_y = 128.0;
    policy
}

#[cfg(test)]
mod tests {
    use pax_runtime::engine::layer_tiling::scroller_canvas_plan_with_policy;

    #[test]
    fn website_phone_tiles_cover_the_viewport_with_less_offscreen_backing() {
        for scroll in [0.0, 800.0, 4000.0, 8670.0] {
            let plan = |policy| {
                scroller_canvas_plan_with_policy(
                    0,
                    "page".into(),
                    393.0,
                    9365.0,
                    393.0,
                    695.0,
                    0.0,
                    scroll,
                    2.75,
                    policy,
                )
            };
            let mobile = plan(super::tiling_policy());
            let desktop = plan(pax_chassis_common::desktop::native_scroller_tiling_policy());
            let area = |plan: &pax_runtime::engine::layer_tiling::LayerCanvasPlan| {
                plan.surfaces
                    .iter()
                    .map(|tile| tile.width * tile.height)
                    .sum::<f64>()
            };
            assert!(
                area(&mobile) < area(&desktop),
                "phone warm region must reduce backing area"
            );
            for row in 0..695 {
                let y = scroll + row as f64;
                assert!(
                    mobile.surfaces.iter().any(|tile| tile.left <= 0.0
                        && tile.left + tile.width >= 393.0
                        && tile.top <= y
                        && tile.top + tile.height >= y + 1.0),
                    "visible row {y} must remain fully covered"
                );
            }
        }
    }
}
