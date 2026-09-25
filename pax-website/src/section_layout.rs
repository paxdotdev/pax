use pax_kit::*;

// The content stack is intrinsically measured; the outer ExampleHost needs a
// concrete height because its source drawer and preview are percentage-sized.
pub fn publish_height(ctx: &NodeContext, id: &str, output: &Property<f64>) {
    let Some(node) = ctx.expanded_node.upgrade() else {
        return;
    };
    // Inspect this component's shallow content tree, not the debug-only global
    // ID registry. This same measurement runs in the release cartridge.
    if let Some(height) = find_height(node.into(), id, 3) {
        if height.is_finite() && height > 0.0 {
            output.set_if_neq(height.ceil() + 160.0);
        }
    }
}

fn find_height(node: NodeInterface, id: &str, depth: usize) -> Option<f64> {
    if node.has_id(id) {
        return Some(node.transform_and_bounds().get().bounds.1);
    }
    if depth == 0 {
        return None;
    }
    node.children()
        .into_iter()
        .find_map(|child| find_height(child, id, depth - 1))
}
