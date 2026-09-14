use pax_message::ImageDataArgs;
use pax_runtime::{api::RenderContext, ExpandedNodeIdentifier, PaxEngine};

pub(crate) fn complete_image_load(
    engine: &PaxEngine,
    renderer: &mut dyn RenderContext,
    args: &ImageDataArgs,
    rgba: &[u8],
) {
    renderer.load_image(&args.path, rgba, args.width, args.height);

    // Loading only updates the renderer's resource cache. A pending Image may
    // still be node-dirty after its first render, but the layer has gone idle.
    // Wake both gates, using the node's current layer rather than its old load
    // location. A late response may populate the cache after its node unmounts.
    if let Some(node) = engine.get_expanded_node(ExpandedNodeIdentifier(args.id)) {
        engine.runtime_context.mark_canvas_node_dirty(node.id);
        engine
            .runtime_context
            .set_canvas_dirty(node.occlusion.get().render_layer_id);
    }
}

#[cfg(test)]
mod tests;
