use pax_kit::*;

// The content stack is unconditional in each section. Bind its measurement as
// soon as it exists, so later mounting/resize can settle without a lifecycle tick.
// The outer ExampleHost needs this concrete extent for its percentage-sized preview.
pub fn bind_height(ctx: &NodeContext, id: &str, output: &Property<f64>) -> bool {
    let Some(node) = ctx.expanded_node.upgrade() else {
        return false;
    };
    // Inspect this component's shallow content tree, not the debug-only global
    // ID registry. This same measurement runs in the release cartridge.
    let Some(content) = find_content(node.into(), id, 3) else {
        return false;
    };
    let measured = content.measured_size_property();
    let deps = [measured.untyped()];
    let output = output.local();
    // A deferred stack still inherits provisional bounds from this section.
    // Feeding those bounds back with padding grows the parent forever. Only
    // publish an actual autosize measurement, including later partial results.
    let update = move || {
        if let Some((_, height)) = measured.get() {
            if height.is_finite() && height > 0.0 {
                output.set_if_neq(height.ceil() + 160.0);
            }
        }
    };
    update();
    ctx.subscribe(&deps, update);
    true
}

pub fn bind_compact(ctx: &NodeContext, output: &Property<bool>, breakpoint: f64) {
    let bounds = ctx.bounds_self.clone();
    let deps = [bounds.untyped()];
    let output = output.local();
    let update = move || {
        output.set_if_neq(bounds.get().0 < breakpoint);
    };
    update();
    ctx.subscribe(&deps, update);
}

fn find_content(node: NodeInterface, id: &str, depth: usize) -> Option<NodeInterface> {
    if node.has_id(id) {
        return Some(node);
    }
    if depth == 0 {
        return None;
    }
    node.children()
        .into_iter()
        .find_map(|child| find_content(child, id, depth - 1))
}
