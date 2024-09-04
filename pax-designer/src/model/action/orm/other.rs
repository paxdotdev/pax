use crate::controls::settings::property_editor::fill_property_editor::color_to_str;
use crate::controls::settings::property_editor::stringify_value_definition;
use crate::controls::settings::property_editor::stroke_property_editor::stroke_as_str;
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
                    .get_node(
                        uid,
                        ctx.app_state
                            .modifiers
                            .get()
                            .contains(&ModifierKey::Control),
                    )
                    .unwrap();
                let props = node.get_all_properties();
                let stroke = props
                    .iter()
                    .find_map(|(p, v)| (p.name == "stroke").then_some(v));
                let fill = props
                    .iter()
                    .find_map(|(p, v)| (p.name == "fill").then_some(v));
                let (Some(Some(stroke)), Some(Some(fill))) = (stroke, fill) else {
                    return Err(anyhow!("object doesn't have stroke and fill"));
                };
                let stroke = pax_engine::pax_lang::from_pax(&stringify_value_definition(stroke))
                    .map(|v| Stroke::try_coerce(v))?
                    .map_err(|e| anyhow!("failed to get stroke {e}"))?;
                let fill = pax_engine::pax_lang::from_pax(&stringify_value_definition(fill))
                    .map(|v| Fill::try_coerce(v))?
                    .map_err(|e| anyhow!("failed to get fill {e}"))?;

                let new_stroke = Stroke {
                    color: Property::new(match fill {
                        Fill::Solid(color) => color,
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
                let stroke_str = stroke_as_str(
                    new_stroke.color.get(),
                    new_stroke.width.get().expect_pixels().to_float(),
                );
                let fill_str = color_to_str(stroke.color.get());
                node.set_property("stroke", &stroke_str)?;
                node.set_property("fill", &fill_str)?;
                node.save()
                    .map_err(|e| anyhow!("failed to swap fill/stroke: {e}"))?;
            }
            Ok(())
        })
    }
}
