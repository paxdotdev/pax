use anyhow::Result;
use pax_engine::api::*;
use pax_engine::pax_manifest::{TemplateNodeId, TypeId, UniqueTemplateNodeIdentifier};
use pax_engine::*;
use pax_std::*;

use crate::model;
use crate::model::action::{Action, ActionContext};

#[pax]
#[engine_import_path("pax_engine")]
#[file("controls/settings/class_selector.pax")]
pub struct ClassSelector {
    pub stid: Property<TypeId>,
    pub snid: Property<TemplateNodeId>,
    pub current_classes: Property<Vec<String>>,
    // available classes to choose from in combobox
    pub available_classes: Property<Vec<String>>,
    pub new_class_text: Property<String>,
}

impl ClassSelector {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        self.bind_available_classes(ctx);
        self.bind_current_classes(ctx);
    }

    fn bind_available_classes(&self, ctx: &NodeContext) {
        let stid = self.stid.clone();
        let manifest_ver = borrow!(ctx.designtime).get_manifest_version();
        let deps = [stid.untyped(), manifest_ver.untyped()];
        let ctx = ctx.clone();
        self.available_classes.replace_with(Property::computed(
            move || {
                let dt = borrow!(ctx.designtime);
                let orm = dt.get_orm();
                let available_classes = orm.get_classes(&stid.get()).unwrap_or_else(|e| {
                    log::warn!("couldn't find classes (error: {e}) for {:?}", stid.get());
                    Default::default()
                });
                available_classes
            },
            &deps,
        ));
    }

    fn bind_current_classes(&self, ctx: &NodeContext) {
        let stid = self.stid.clone();
        let snid = self.snid.clone();
        let manifest_ver = borrow!(ctx.designtime).get_manifest_version();
        let deps = [stid.untyped(), manifest_ver.untyped(), snid.untyped()];
        let ctx = ctx.clone();
        self.current_classes.replace_with(Property::computed(
            move || {
                log::debug!(
                    "vals: {:?}, {:?}",
                    stid.get().import_path(),
                    snid.get().as_usize()
                );
                let mut dt = borrow_mut!(ctx.designtime);
                let orm = dt.get_orm_mut();
                let Some(mut properties) = orm.get_node_builder(
                    UniqueTemplateNodeIdentifier::build(stid.get(), snid.get()),
                    false,
                ) else {
                    log::debug!("failed to get node classes");
                    return vec![];
                };
                let out = properties.get_all_applied_classes();
                log::debug!("out: {:?}", out);
                out
            },
            &deps,
        ));
    }

    pub fn add_class(&mut self, ctx: &NodeContext, event: Event<Click>) {
        log::debug!("add class: {}", self.new_class_text.get());
        // let dt = borrow!(ctx.designtime);
        // let orm = dt.get_orm_mut();
    }
}

// TODO next steps:
// - hook up this to be used in list view
// - enable removal by clicking x
// - enable editing by double clicking
// - make adding a class to a node work (need to be able to write mutliple with same property name without overwriting)

#[pax]
#[engine_import_path("pax_engine")]
#[inlined(
    <Text x=3px class=input text=class width={100%-3px} height=100%/>
    <Rectangle fill=black/>

    @settings {
        @mouse_down: on_click
    }
)]
pub struct ListItem {
    pub class: Property<String>,
}

impl ListItem {
    pub fn on_name_click(&mut self, ctx: &NodeContext, _event: Event<Click>) {
        model::perform_action(&EditClass(self.class.get()), ctx);
    }

    pub fn remove(&mut self, ctx: &NodeContext, event: Event<Click>) {}
}

struct EditClass(String);

impl Action for EditClass {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        ctx.app_state
            .current_editor_class_name
            .set(Some(self.0.clone()));
        Ok(())
    }
}
