use anyhow::anyhow;
use pax_engine::api::*;
use pax_engine::pax_manifest::{TemplateNodeId, TypeId, UniqueTemplateNodeIdentifier};
use pax_engine::*;
use pax_std::*;

use crate::granular_change_store::GranularManifestChangeStore;
use crate::model;

#[pax]
#[engine_import_path("pax_engine")]
#[file("controls/settings/control_flow_if_editor.pax")]
pub struct ControlFlowIfEditor {
    pub stid: Property<TypeId>,
    pub snid: Property<TemplateNodeId>,
    pub if_source: Property<String>,
}

impl ControlFlowIfEditor {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        // TODO hook up if_source to reactively be read on manifest change
        let manifest_changed_notifier = ctx
            .peek_local_store(
                |change_notification_store: &mut GranularManifestChangeStore| {
                    change_notification_store.get_manifest_any_change_notifier()
                },
            )
            .expect("should be inserted at designer root");
        let deps = [manifest_changed_notifier];
        let ctx = ctx.clone();
        let stid = self.stid.clone();
        let snid = self.snid.clone();
        self.if_source.replace_with(Property::computed(
            move || {
                let mut dt = borrow_mut!(ctx.designtime);
                let uni = UniqueTemplateNodeIdentifier::build(stid.get(), snid.get());
                if let Some(conditional_expression) = dt
                    .get_orm_mut()
                    .get_node_builder(uni, false)
                    .and_then(|mut n| n.get_control_flow_properties())
                    .and_then(|cf| cf.condition_expression)
                {
                    conditional_expression.to_string()
                } else {
                    log::warn!("couldn't fetch if definition");
                    "<error>".to_string()
                }
            },
            &deps,
        ));
    }

    pub fn commit(&mut self, ctx: &NodeContext, event: Event<TextboxChange>) {
        let t = model::with_action_context(ctx, |ac| ac.transaction("writing if expression"));
        let _ = t.run(|| {
            let uid = UniqueTemplateNodeIdentifier::build(self.stid.get(), self.snid.get());
            let mut dt = borrow_mut!(ctx.designtime);
            let orm = dt.get_orm_mut();
            let mut node = orm
                .get_node_builder(uid, true)
                .ok_or_else(|| anyhow!("failed to get node"))?;
            node.set_conditional_source(&format!("{{{}}}", event.text))
                .map_err(|e| anyhow!("failed to set conditional source: {e}"))?;
            node.save()
                .map_err(|e| anyhow!("failed to save conditional source: {e}"))?;
            Ok(())
        });
    }
}
