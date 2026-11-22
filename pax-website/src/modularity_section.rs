use crate::{circuit_board::CircuitBoard, SiteTheme};
use pax_kit::*;

#[pax]
#[file("modularity_section.pax")]
pub struct ModularitySection {
    pub compact: Property<bool>,
    pub content_height: Property<f64>,
}

impl ModularitySection {
    pub fn mount(&mut self, ctx: &NodeContext) {
        crate::section_layout::bind_compact(ctx, &self.compact, 1000.0);
    }

    pub fn bind_content_height(&mut self, ctx: &NodeContext) {
        let Some(board) = ctx.expanded_node.upgrade().map(NodeInterface::from) else {
            return;
        };
        let Some(stack) = board.render_parent() else {
            return;
        };
        let stack_size = stack.measured_size_property();
        let board_hull = board.subtree_layout_hull();
        let compact = self.compact.local();
        let output = self.content_height.local();
        let deps = [
            stack_size.untyped(),
            board_hull.untyped(),
            compact.untyped(),
        ];
        let update = move || {
            let Some((_, height)) = stack_size.get() else {
                return;
            };
            // In a wide layout both columns contribute to the section's height.
            // In a compact layout the board already occupies a row of the stack.
            let height = if compact.get() {
                height
            } else {
                height.max(board_hull.get().forward_extent_y().unwrap_or_default())
            };
            if height.is_finite() && height > 0.0 {
                output.set_if_neq(height.ceil() + 160.0);
            }
        };
        update();
        ctx.subscribe(&deps, update);
    }
}
