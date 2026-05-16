use pax_engine::api::{Axis, Property};
use pax_engine::pax;
use pax_runtime::api::{borrow, Color, Layer, NativeLiquidGlassScope, Numeric, Size};
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use std::iter;
use std::rc::Rc;

/// Applies an Apple liquid-glass native effect to supported descendant native surfaces.
#[pax]
#[engine_import_path("pax_engine")]
#[custom(Default)]
#[primitive("pax_std::core::liquid_glass::LiquidGlassInstance")]
pub struct LiquidGlass {
    /// Whether this liquid-glass scope is active. When false, descendants opt out.
    pub enabled: Property<bool>,
    /// Desired spacing between grouped glass surfaces, in Pax units.
    pub spacing: Property<Size>,
    /// Whether supported Apple glass surfaces should use the interactive effect.
    pub interactive: Property<bool>,
    /// Optional tint for supported Apple glass surfaces.
    pub tint: Property<Option<Color>>,
    /// Apple glass style. Supported values are currently "regular" and "clear".
    pub variant: Property<String>,
}

impl Default for LiquidGlass {
    fn default() -> Self {
        Self {
            enabled: Property::new(true),
            spacing: Property::new(Size::Pixels(Numeric::F64(0.0))),
            interactive: Property::new(false),
            tint: Property::new(None),
            variant: Property::new("regular".to_string()),
        }
    }
}

pub struct LiquidGlassInstance {
    base: BaseInstance,
}

impl LiquidGlassInstance {
    fn scope_property(
        expanded_node: &Rc<ExpandedNode>,
    ) -> Property<Option<NativeLiquidGlassScope>> {
        let (enabled, spacing, interactive, tint, variant) = expanded_node
            .with_properties_unwrapped(|liquid_glass: &mut LiquidGlass| {
                (
                    liquid_glass.enabled.clone(),
                    liquid_glass.spacing.clone(),
                    liquid_glass.interactive.clone(),
                    liquid_glass.tint.clone(),
                    liquid_glass.variant.clone(),
                )
            });
        let transform_and_bounds = expanded_node.transform_and_bounds.clone();
        let group_id = expanded_node.id.to_u32();
        let deps = [
            enabled.untyped(),
            spacing.untyped(),
            interactive.untyped(),
            tint.untyped(),
            variant.untyped(),
            transform_and_bounds.untyped(),
        ];

        Property::computed(
            move || {
                if !enabled.get() {
                    return None;
                }
                let bounds = transform_and_bounds.get().bounds;
                let spacing = spacing.get().evaluate(bounds, Axis::X).max(0.0);
                Some(NativeLiquidGlassScope {
                    group_id,
                    spacing,
                    interactive: interactive.get(),
                    tint: tint.get(),
                    variant: variant.get(),
                })
            },
            &deps,
        )
    }

    fn mount_children_with_scope(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
        recurse_control_flow: bool,
    ) {
        let env = Rc::clone(&expanded_node.stack);
        let children = borrow!(self.base().get_instance_children());
        let children_with_envs = children.iter().cloned().zip(iter::repeat(env));
        let scope = Self::scope_property(expanded_node);
        let children = expanded_node.generate_children_with_liquid_glass_scope(
            children_with_envs,
            context,
            &expanded_node.parent_frame,
            &scope,
            true,
        );
        if recurse_control_flow {
            for child in children.iter() {
                child.recurse_control_flow_expansion(context);
            }
        }
        expanded_node.children.set(children);
    }
}

impl InstanceNode for LiquidGlassInstance {
    fn instantiate(args: InstantiationArgs) -> Rc<Self>
    where
        Self: Sized,
    {
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

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        self.mount_children_with_scope(expanded_node, context, false);
    }

    fn handle_control_flow_node_expansion(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        self.mount_children_with_scope(expanded_node, context, true);
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        match expanded_node {
            Some(expanded_node) => {
                expanded_node.with_properties_unwrapped(|_g: &mut LiquidGlass| {
                    f.debug_struct("LiquidGlass").finish()
                })
            }
            None => f.debug_struct("LiquidGlass").finish_non_exhaustive(),
        }
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}
