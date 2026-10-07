// The browser reconciliation tests compile this fixture with rustc, so the
// production allocation helper is exercised without a browser/GPU dependency.
#[path = "../../pax-runtime/src/engine/surface_metrics.rs"]
mod surface_metrics;

fn main() {
    let mut first = true;
    print!("[");
    for (width, height, cap) in [
        (500.0, 250.0, u32::MAX),
        (73.8181, 534.25, 4096),
        (0.6, 0.6, 4096),
        (3000.0, 600.0, 2048),
    ] {
        for dpr in [0.5, 1.0, 1.25, 1.5, 2.0, 3.0] {
            let metrics = surface_metrics::projected_surface_metrics(width, height, [dpr; 2], cap);
            if !first {
                print!(",");
            }
            first = false;
            print!("{{\"width\":{width},\"height\":{height},\"dpr\":{dpr},\"backingWidth\":{},\"backingHeight\":{},\"densityClamped\":{}}}",
                metrics.surface_width, metrics.surface_height, metrics.density_clamped);
        }
    }
    println!("]");
}
