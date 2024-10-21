use anyhow::{anyhow, Result};
use pax_designtime::orm::template::node_builder::NodeBuilder;
use pax_engine::{
    api::{borrow_mut, Rotation},
    log,
    math::Space,
    pax_manifest::{PaxType, UniqueTemplateNodeIdentifier, ValueDefinition},
    pax_runtime::{LayoutProperties, TransformAndBounds},
    serde::Serialize,
    NodeLocal, ToPaxValue,
};
use pax_std::Size;

use crate::{
    math::{self, approx::ApproxEq, DecompositionConfiguration},
    model::{
        action::{Action, ActionContext},
        input::ModifierKey,
    },
};

pub enum NodeLayoutSettings<'a, S> {
    Fill,
    KeepScreenBounds {
        node_transform_and_bounds: &'a TransformAndBounds<NodeLocal, S>,
        parent_transform_and_bounds: &'a TransformAndBounds<NodeLocal, S>,
        node_decomposition_config: &'a DecompositionConfiguration,
    },
    WithProperties(LayoutProperties),
}

impl<S: Space> Clone for NodeLayoutSettings<'_, S> {
    fn clone(&self) -> Self {
        match self {
            Self::Fill => Self::Fill,
            Self::KeepScreenBounds {
                node_transform_and_bounds,
                node_decomposition_config,
                parent_transform_and_bounds,
            } => Self::KeepScreenBounds {
                node_transform_and_bounds,
                node_decomposition_config,
                parent_transform_and_bounds,
            },
            Self::WithProperties(props) => Self::WithProperties(props.clone()),
        }
    }
}

pub struct SetNodeLayout<'a, S> {
    pub id: &'a UniqueTemplateNodeIdentifier,
    pub node_layout: &'a NodeLayoutSettings<'a, S>,
}

impl<S: Space> Action for SetNodeLayout<'_, S> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        match &self.node_layout {
            NodeLayoutSettings::KeepScreenBounds {
                node_transform_and_bounds,
                parent_transform_and_bounds: new_parent_transform_and_bounds,
                node_decomposition_config: node_inv_config,
            } => SetNodeLayoutPropertiesFromTransform {
                id: &self.id,
                transform_and_bounds: node_transform_and_bounds,
                parent_transform_and_bounds: new_parent_transform_and_bounds,
                decomposition_config: node_inv_config,
            }
            .perform(ctx),
            NodeLayoutSettings::Fill => SetNodeLayoutProperties {
                id: &self.id,
                properties: &LayoutProperties::fill(),
                reset_anchor: true,
            }
            .perform(ctx),
            NodeLayoutSettings::WithProperties(props) => SetNodeLayoutProperties {
                id: &self.id,
                properties: props,
                reset_anchor: false,
            }
            .perform(ctx),
        }
    }
}

struct SetNodeLayoutProperties<'a> {
    id: &'a UniqueTemplateNodeIdentifier,
    properties: &'a LayoutProperties,
    // anchor doesn't have a default value (becomes "reactive" in the None case), and so needs
    // to be manually specified to be reset
    reset_anchor: bool,
}

impl Action for SetNodeLayoutProperties<'_> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let mut dt = borrow_mut!(ctx.engine_context.designtime);
        let Some(mut builder) = dt.get_orm_mut().get_node_builder(
            self.id.clone(),
            ctx.app_state
                .modifiers
                .get()
                .contains(&ModifierKey::Control),
        ) else {
            return Err(anyhow!("can't move: node doesn't exist in orm"));
        };

        if !matches!(
            builder.get_type_id().get_pax_type(),
            PaxType::Singleton { .. } | PaxType::BlankComponent { .. }
        ) {
            return Ok(());
        };

        let LayoutProperties {
            x,
            y,
            width,
            height,
            rotate,
            scale_x,
            scale_y,
            anchor_x,
            anchor_y,
            skew_x,
            skew_y,
        } = self.properties.clone();

        // compare with the values for the current node in the engine, and
        // only try to write if different (we don't want to try to overwrite a
        // rotation expression if we are only moving an object and not affecting
        // rotation)
        let old_props = ctx
            .engine_context
            .get_nodes_by_global_id(self.id.clone())
            .into_iter()
            .next()
            .map(|n| n.layout_properties())
            .unwrap_or_default();

        write_to_orm(&mut builder, "x", x, old_props.x.as_ref(), Size::ZERO())
            .map_err(|e| anyhow!("couldn't set x to {x:?}: {e}"))?;
        write_to_orm(&mut builder, "y", y, old_props.y.as_ref(), Size::ZERO())
            .map_err(|e| anyhow!("couldn't set y to {y:?}: {e}"))?;

        write_to_orm(
            &mut builder,
            "width",
            width,
            old_props.width.as_ref(),
            Size::default(),
        )
        .map_err(|e| anyhow!("couldn't set width to {width:?}: {e}"))?;

        write_to_orm(
            &mut builder,
            "height",
            height,
            old_props.height.as_ref(),
            Size::default(),
        )
        .map_err(|e| anyhow!("couldn't set height to {height:?}: {e}"))?;

        write_to_orm(
            &mut builder,
            "scale_x",
            scale_x.map(|v| Size::Percent(v.0)),
            old_props
                .scale_x
                .as_ref()
                .map(|v| Size::Percent(v.0))
                .as_ref(),
            Size::default(),
        )
        .map_err(|e| anyhow!("couldn't set scale_x: {e}"))?;

        write_to_orm(
            &mut builder,
            "scale_y",
            scale_y.map(|v| Size::Percent(v.0)),
            old_props
                .scale_y
                .as_ref()
                .map(|v| Size::Percent(v.0))
                .as_ref(),
            Size::default(),
        )
        .map_err(|e| anyhow!("couldn't set scale_y: {e}"))?;

        write_to_orm(
            &mut builder,
            "rotate",
            rotate,
            old_props.rotate.as_ref(),
            Rotation::default(),
        )
        .map_err(|e| anyhow!("couldn't set rotation to: {e}"))?;

        write_to_orm(
            &mut builder,
            "skew_x",
            skew_x,
            old_props.skew_x.as_ref(),
            Rotation::default(),
        )
        .map_err(|e| anyhow!("couldn't set skew_x to {skew_x:?}: {e}"))?;

        write_to_orm(
            &mut builder,
            "skew_y",
            skew_y,
            old_props.skew_y.as_ref(),
            Rotation::default(),
        )
        .map_err(|e| anyhow!("couldn't set skew_y to {skew_y:?}: {e}"))?;

        if self.reset_anchor {
            builder.set_property_from_value_definition("anchor_x", None)?;
            builder.set_property_from_value_definition("anchor_y", None)?;
        } else {
            write_to_orm(
                &mut builder,
                "anchor_x",
                anchor_x,
                old_props.anchor_x.as_ref(),
                // never assume default
                Size::Combined(f64::MAX.into(), f64::MAX.into()),
            )
            .map_err(|e| anyhow!("couldn't set anchor_x to {anchor_x:?}: {e}"))?;
            write_to_orm(
                &mut builder,
                "anchor_y",
                anchor_y,
                old_props.anchor_y.as_ref(),
                // never assume default
                Size::Combined(f64::MAX.into(), f64::MAX.into()),
            )
            .map_err(|e| anyhow!("couldn't set anchor_y to {anchor_y:?}: {e}"))?;
        }

        builder
            .save()
            .map_err(|e| anyhow!("could not move: {}", e))?;

        Ok(())
    }
}

struct SetNodeLayoutPropertiesFromTransform<'a, T> {
    id: &'a UniqueTemplateNodeIdentifier,
    transform_and_bounds: &'a TransformAndBounds<NodeLocal, T>,
    parent_transform_and_bounds: &'a TransformAndBounds<NodeLocal, T>,
    decomposition_config: &'a DecompositionConfiguration,
}

impl<T: Space> Action for SetNodeLayoutPropertiesFromTransform<'_, T> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let new_props: LayoutProperties = math::transform_and_bounds_decomposition(
            self.decomposition_config,
            self.parent_transform_and_bounds,
            self.transform_and_bounds,
        );

        SetNodeLayoutProperties {
            id: self.id,
            properties: &new_props,
            reset_anchor: false,
        }
        .perform(ctx)?;
        Ok(())
    }
}

fn write_to_orm<T: ToPaxValue + ApproxEq>(
    builder: &mut NodeBuilder,
    name: &str,
    value: Option<T>,
    old_value: Option<&T>,
    default_value: T,
) -> Result<()> {
    if old_value.approx_eq(&value.as_ref()) {
        return Ok(());
    }
    let value = value.filter(|v| !v.approx_eq(&default_value));
    builder.set_property_from_typed(name, value)?;
    Ok(())
}
