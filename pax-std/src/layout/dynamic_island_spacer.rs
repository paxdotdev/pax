use pax_engine::*;
use pax_runtime::api::{Layer, Platform, Property, SafeAreaInsets};
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use std::rc::Rc;

/// Opt-in space for UIKit's current safe area; does not draw or intercept input.
///
/// `<DynamicIslandSpacer />` fills its container's width and measures its height
/// from the top safe-area inset. Use it in content-sized layout, or bind `inset`
/// when positioning other content explicitly. `edge` also supports the home
/// indicator and landscape side insets. Insets update on rotation and window
/// changes, in debug and release. Other chassis measure zero on the inset axis.
///
/// This reserves the platform's entire safe inset, including status bars on
/// devices without a Dynamic Island. It does not automatically move siblings,
/// inset ancestors, or detect an ancestor that already avoids the safe area.
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::layout::dynamic_island_spacer::DynamicIslandSpacerInstance")]
pub struct DynamicIslandSpacer {
    /// Window edge to reserve. Defaults to Top. Left/Right measure width and
    /// fill the container's height; Top/Bottom measure height and fill width.
    pub edge: Property<SafeAreaEdge>,
    /// Computed inset in logical pixels. Bind to observe it; the spacer owns
    /// this output and overwrites authored values when its measurement changes.
    pub inset: Property<f64>,
}

/// The window safe-area edge reserved by a DynamicIslandSpacer.
#[pax]
#[engine_import_path("pax_engine")]
pub enum SafeAreaEdge {
    /// Space below the status bar or top cutout.
    #[default]
    Top,
    /// Space left of a right-side cutout in landscape.
    Right,
    /// Space above the home indicator.
    Bottom,
    /// Space right of a left-side cutout in landscape.
    Left,
}

impl SafeAreaEdge {
    fn measurement(&self, insets: SafeAreaInsets, parent: (f64, f64)) -> (f64, (f64, f64)) {
        match self {
            Self::Top => (insets.top, (parent.0, insets.top)),
            Self::Right => (insets.right, (insets.right, parent.1)),
            Self::Bottom => (insets.bottom, (parent.0, insets.bottom)),
            Self::Left => (insets.left, (insets.left, parent.1)),
        }
    }
}

pub struct DynamicIslandSpacerInstance {
    base: BaseInstance,
}

#[cfg(test)]
mod tests;

impl InstanceNode for DynamicIslandSpacerInstance {
    fn instantiate(args: InstantiationArgs) -> Rc<Self> {
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    invisible_to_slot: false,
                    invisible_to_raycasting: true,
                    layer: Layer::DontCare,
                    is_component: false,
                    is_slot: false,
                },
            ),
        })
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("DynamicIslandSpacer").finish()
    }

    fn handle_mount(self: Rc<Self>, node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        let globals = context.globals();
        let supported = globals.platform == Platform::Native && globals.os.is_ios();
        let insets = context.safe_area_insets();
        let parent = node.get_node_context(context).bounds_parent;
        let (edge, inset) = node.with_properties_unwrapped(|p: &mut DynamicIslandSpacer| {
            (p.edge.clone(), p.inset.clone())
        });
        let deps = [insets.untyped(), parent.untyped(), edge.untyped()];
        let weak = Rc::downgrade(node);
        node.changed_listener
            .replace_with(Property::computed_with_name(
                move || {
                    let Some(node) = weak.upgrade() else {
                        return;
                    };
                    let insets = if supported {
                        insets.get()
                    } else {
                        SafeAreaInsets::default()
                    };
                    let (extent, measured) = edge.get().measurement(insets, parent.get());
                    inset.set_if_neq(extent);
                    node.measured_size.set_if_neq(Some(measured));
                },
                &deps,
                "dynamic island spacer",
            ));
    }

    fn handle_unmount(&self, node: &Rc<ExpandedNode>, _: &Rc<RuntimeContext>) {
        node.changed_listener.replace_with(Property::default());
    }
}
