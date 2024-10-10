use crate::controls::settings::property_editor::fill_property_editor::color_to_str;
use crate::model::action::{Action, ActionContext};
use crate::model::input::ModifierKey;
use anyhow::anyhow;
use anyhow::Result;
use pax_engine::api::{borrow_mut, Fill, Stroke};
use pax_engine::pax_manifest::{UniqueTemplateNodeIdentifier, ValueDefinition};
use pax_engine::{log, CoercionRules, Property};

pub struct SwapFillStrokeAction;

impl Action for SwapFillStrokeAction {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let selected = ctx.app_state.selected_template_node_ids.get();
        if selected.len() != 1 {
            // Maybe could do all?
            return Err(anyhow!("can only swap color/fill on a single node"));
        }
        let t = ctx.transaction("swapping fill and stroke");
        t.run(|| {
            if selected.len() == 1 {
                let cid = ctx.app_state.selected_component_id.get();
                let tnid = selected.into_iter().next().unwrap();
                let uid = UniqueTemplateNodeIdentifier::build(cid, tnid);
                // TODO transaction
                let mut dt = borrow_mut!(ctx.engine_context.designtime);
                let mut node = dt
                    .get_orm_mut()
                    .get_node_builder(
                        uid,
                        ctx.app_state
                            .modifiers
                            .get()
                            .contains(&ModifierKey::Control),
                    )
                    .unwrap();
                let props = node.get_all_property_definitions();
                let stroke = props
                    .iter()
                    .find_map(|(p, v)| (p.name == "stroke").then_some(v));
                let fill = props
                    .iter()
                    .find_map(|(p, v)| (p.name == "fill").then_some(v));
                let (
                    Some(Some(ValueDefinition::LiteralValue(stroke))),
                    Some(Some(ValueDefinition::LiteralValue(fill))),
                ) = (stroke, fill)
                else {
                    return Err(anyhow!(
                        "object doesn't have stroke and fill properties where both are literals"
                    ));
                };

                let (Ok(stroke), Ok(fill)) = (
                    Stroke::try_coerce(stroke.clone()),
                    Fill::try_coerce(fill.clone()),
                ) else {
                    return Err(anyhow!("stroke or fill property type was unexpected"));
                };

                let new_stroke = Stroke {
                    color: Property::new(match &fill {
                        Fill::Solid(color) => color.clone(),
                        // TODO when stroke supports gradient
                        Fill::LinearGradient(l) => l
                            .stops
                            .first()
                            .map(|gs| gs.color.clone())
                            .unwrap_or_default(),
                        Fill::RadialGradient(r) => r
                            .stops
                            .first()
                            .map(|gs| gs.color.clone())
                            .unwrap_or_default(),
                    }),
                    ..stroke
                };
                node.set_property_from_typed("stroke", Some(new_stroke))?;
                node.set_property_from_typed("fill", Some(stroke.color.get()))?;
                node.save()
                    .map_err(|e| anyhow!("failed to swap fill/stroke: {e}"))?;
            }
            Ok(())
        })
    }
}
