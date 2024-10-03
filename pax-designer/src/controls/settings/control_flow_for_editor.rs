use anyhow::anyhow;
use pax_engine::api::*;
use pax_engine::pax_manifest::{TemplateNodeId, TypeId, UniqueTemplateNodeIdentifier};
use pax_engine::*;
use pax_std::*;

use crate::model;

#[pax]
#[engine_import_path("pax_engine")]
#[file("controls/settings/control_flow_for_editor.pax")]
pub struct ControlFlowForEditor {
    pub stid: Property<TypeId>,
    pub snid: Property<TemplateNodeId>,
    pub for_source: Property<String>,
    pub for_predicate: Property<String>,
}

impl ControlFlowForEditor {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        // TODO hook up for source/predicate to reactively be read on manifest change
        let dt = borrow!(ctx.designtime);
        let manifest_ver = dt.get_last_rendered_manifest_version();
        let deps = [manifest_ver.untyped()];
        let ctx = ctx.clone();
        let stid = self.stid.clone();
        let snid = self.snid.clone();
        let control_flow_parts = Property::computed(
            move || {
                let mut dt = borrow_mut!(ctx.designtime);
                let uni = UniqueTemplateNodeIdentifier::build(stid.get(), snid.get());
                if let Some(control_flow_data) = dt
                    .get_orm_mut()
                    .get_node(uni, false)
                    .and_then(|mut n| n.get_control_flow_properties())
                {
                    (
                        control_flow_data
                            .repeat_predicate_definition
                            .map(|v| match v {
                                pax_manifest::ControlFlowRepeatPredicateDefinition::ElemId(ind) => ind.to_string(),
                                pax_manifest::ControlFlowRepeatPredicateDefinition::ElemIdIndexId(elem, ind) => format!("({},{})", elem, ind),
                            }).unwrap_or_else(|| "<error>".to_string()),
                        control_flow_data
                            .repeat_source_expression
                            .map(|v| v.to_string()).unwrap_or_else(|| "<error>".to_string()),
                    )
                } else {
                    log::warn!("couldn't fetch if definition");
                    ("<error>".to_string(), "<error>".to_string())
                }
            },
            &deps,
        );
        let deps = [control_flow_parts.untyped()];
        let control_flow_parts_cp = control_flow_parts.clone();
        self.for_source.replace_with(Property::computed(
            move || control_flow_parts_cp.get().1,
            &deps,
        ));
        self.for_predicate.replace_with(Property::computed(
            move || control_flow_parts.get().0,
            &deps,
        ));
    }

    pub fn commit_source(&mut self, ctx: &NodeContext, event: Event<TextboxChange>) {
        let t = model::with_action_context(ctx, |ac| ac.transaction("writing if expression"));
        let _ = t.run(|| {
            let uid = UniqueTemplateNodeIdentifier::build(self.stid.get(), self.snid.get());
            let mut dt = borrow_mut!(ctx.designtime);
            let orm = dt.get_orm_mut();
            let mut node = orm
                .get_node(uid, true)
                .ok_or_else(|| anyhow!("failed to get node"))?;
            node.set_repeat_source(&format!("{{{}}}", event.text))
                .map_err(|e| anyhow!("failed to set repeat source: {e}"))?;
            node.save()
                .map_err(|e| anyhow!("failed to save repeat source: {e}"))?;
            Ok(())
        });
    }

    pub fn commit_predicate(&mut self, ctx: &NodeContext, event: Event<TextboxChange>) {
        let t = model::with_action_context(ctx, |ac| ac.transaction("writing if expression"));
        let _ = t.run(|| {
            let uid = UniqueTemplateNodeIdentifier::build(self.stid.get(), self.snid.get());
            let mut dt = borrow_mut!(ctx.designtime);
            let orm = dt.get_orm_mut();
            let mut node = orm
                .get_node(uid, true)
                .ok_or_else(|| anyhow!("failed to get node"))?;
            node.set_repeat_predicate(&format!("{}", event.text))
                .map_err(|e| anyhow!("failed to set repeat predicate: {e}"))?;
            node.save()
                .map_err(|e| anyhow!("failed to save repeat predicate: {e}"))?;
            Ok(())
        });
    }
}
