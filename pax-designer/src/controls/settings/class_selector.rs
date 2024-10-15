use anyhow::anyhow;
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
        let manifest_ver = borrow!(ctx.designtime).get_last_written_manifest_version();
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
        let manifest_ver = borrow!(ctx.designtime).get_last_written_manifest_version();
        let deps = [stid.untyped(), manifest_ver.untyped(), snid.untyped()];
        let ctx = ctx.clone();
        self.current_classes.replace_with(Property::computed(
            move || {
                let mut dt = borrow_mut!(ctx.designtime);
                let orm = dt.get_orm_mut();
                let Some(mut properties) = orm.get_node_builder(
                    UniqueTemplateNodeIdentifier::build(stid.get(), snid.get()),
                    false,
                ) else {
                    return vec![];
                };
                let out = properties.get_all_applied_classes();
                out
            },
            &deps,
        ));
    }

    pub fn add_class(&mut self, ctx: &NodeContext, _event: Event<Click>) {
        let uni = UniqueTemplateNodeIdentifier::build(self.stid.get(), self.snid.get());
        let t = model::with_action_context(ctx, |ac| ac.transaction("add class"));
        let _ = t.run(|| {
            let mut dt = borrow_mut!(ctx.designtime);
            let orm = dt.get_orm_mut();
            let Some(mut node_builder) = orm.get_node_builder(uni.clone(), false) else {
                log::warn!("no node: {uni:?}");
                return Err(anyhow!("failed to retrieve node"));
            };
            node_builder.add_class(&self.new_class_text.get())?;
            node_builder
                .save()
                .map_err(|e| anyhow!("failed to save node: {e}"))?;
            self.new_class_text.set("".to_string());
            Ok(())
        });
    }
}

#[pax]
#[engine_import_path("pax_engine")]
#[inlined(
    <Group  @click=self.remove x=100% width=20px>
        <EventBlocker/>
        <Path class=x_symbol x=50% y=50% width=9px height=9px/>
    </Group>
    <EventBlocker @double_click=on_name_click/>
    <Text x=3px class=input text=class width={100%-3px} height=100%/>
    <Rectangle fill={rgb(12.5%, 12.5%, 12.5%)}
            stroke={
                color: rgb(48, 56, 62),
                width: 1px,
            }
    />

    @settings {
        .input {
            style: {
                font: {Font::Web(
                    "ff-real-headline-pro",
                    "https://use.typekit.net/ivu7epf.css",
                    FontStyle::Normal,
                    FontWeight::ExtraLight,
                )},
                font_size: 11px,
                align_vertical: TextAlignVertical::Center,
                align_multiline: TextAlignHorizontal::Center
                fill: WHITE,
            }
        }
        .x_symbol {
            elements: {[
                PathElement::Point(0%, 10%),
                PathElement::Line,
                PathElement::Point(50% - 10%, 50%),
                PathElement::Line,
                PathElement::Point(0%, 100% - 10%),
                PathElement::Line,
                PathElement::Point(10%, 100%),
                PathElement::Line,
                PathElement::Point(50%, 50% + 10%),
                PathElement::Line,
                PathElement::Point(100% - 10%, 100%),
                PathElement::Line,
                PathElement::Point(100%, 100% - 10%),
                PathElement::Line,
                PathElement::Point(50% + 10%, 50%),
                PathElement::Line,
                PathElement::Point(100%, 10%),
                PathElement::Line,
                PathElement::Point(100% - 10%, 0%),
                PathElement::Line,
                PathElement::Point(50%, 50% - 10%),
                PathElement::Line,
                PathElement::Point(10%, 0%),
                PathElement::Close
            ]},
            stroke: {
                width: 0
            },
            fill: rgb(200, 200, 200)
        }
    }
)]
pub struct ListItem {
    pub stid: Property<TypeId>,
    pub snid: Property<TemplateNodeId>,
    pub text: Property<String>,
}

impl ListItem {
    pub fn on_name_click(&mut self, ctx: &NodeContext, _event: Event<DoubleClick>) {
        model::perform_action(&EditClass(self.text.get()), ctx);
    }

    pub fn remove(&mut self, ctx: &NodeContext, _event: Event<Click>) {
        let uni = UniqueTemplateNodeIdentifier::build(self.stid.get(), self.snid.get());
        let t = model::with_action_context(ctx, |ac| ac.transaction("remove class"));
        let _ = t.run(|| {
            let mut dt = borrow_mut!(ctx.designtime);
            let orm = dt.get_orm_mut();
            let Some(mut node_builder) = orm.get_node_builder(uni.clone(), false) else {
                log::warn!("no node: {uni:?}");
                return Err(anyhow!("failed to retrieve node"));
            };
            node_builder.remove_class(&self.text.get())?;
            node_builder
                .save()
                .map_err(|e| anyhow!("failed to save node: {e}"))?;
            Ok(())
        });
    }
}

struct EditClass(String);

impl Action for EditClass {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        ctx.app_state
            .current_editor_class_name
            .set(Some(format!("{}", self.0)));
        Ok(())
    }
}
