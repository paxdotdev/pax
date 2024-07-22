use anyhow::anyhow;
use pax_designtime::orm::template::builder::NodeBuilder;
use pax_designtime::orm::template::NodeAction;
use pax_engine::api::*;
use pax_engine::*;
use pax_manifest::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;
use std::fmt::Write;

pub mod border_radius_property_editor;
pub mod color_property_editor;
pub mod direction_property_editor;
pub mod fill_property_editor;
pub mod stroke_property_editor;
pub mod text_property_editor;

use border_radius_property_editor::BorderRadiusPropertyEditor;
use color_property_editor::ColorPropertyEditor;
use direction_property_editor::DirectionPropertyEditor;
use fill_property_editor::FillPropertyEditor;
use stroke_property_editor::StrokePropertyEditor;
use text_property_editor::TextPropertyEditor;

#[pax]
#[file("controls/settings/property_editor/mod.pax")]
pub struct PropertyEditor {
    pub name: Property<String>,
    pub ind: Property<usize>,
    pub stid: Property<TypeId>,
    pub snid: Property<TemplateNodeId>,

    // internal repr, always set to collection of above
    pub prop_type_ident_id: Property<usize>,
    pub data: Property<PropertyEditorData>,
}

impl PropertyEditor {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let stid = self.stid.clone();
        let snid = self.snid.clone();
        let name = self.name.clone();
        let ind = self.ind.clone();
        let deps = [
            stid.untyped(),
            snid.untyped(),
            name.untyped(),
            ind.untyped(),
        ];
        self.data.replace_with(Property::computed(
            move || PropertyEditorData {
                editor_index: ind.get(),
                name: name.get(),
                stid: stid.get(),
                snid: snid.get(),
            },
            &deps,
        ));

        let data = self.data.clone();
        let ctx = ctx.clone();
        let deps = [data.untyped()];
        self.prop_type_ident_id.replace_with(Property::computed(
            move || {
                let data = data.get();
                let prop_type_ident = data
                    .get_prop_type_id(&ctx)
                    .unwrap_or_default()
                    .get_unique_identifier();

                // log::info!("settings data: ({}, {})", data.name, prop_type_ident);
                match (data.name.as_str(), prop_type_ident.as_str()) {
                    // TODO make this unique type (4 corner values) instead of matching on name
                    ("border_radius", "f64") => {
                        log::debug!("matched border radius");
                        6
                    }
                    (_, "pax_designer::pax_reexports::pax_engine::api::Color") => 5,
                    (_, "pax_designer::pax_reexports::pax_engine::api::Fill") => 2,
                    (_, "pax_designer::pax_reexports::pax_engine::api::Stroke") => 3,
                    (_, "pax_designer::pax_reexports::pax_std::types::StackerDirection") => 4,
                    _ => 1,
                }
            },
            &deps,
        ));
    }
}

#[pax]
pub struct PropertyEditorData {
    // this is used  by the custom properties to communicate back to the
    // settings editor to set its height
    pub editor_index: usize,
    pub name: String,
    pub stid: TypeId,
    pub snid: TemplateNodeId,
}

impl PropertyEditorData {
    pub fn get_prop_type_id(&self, ctx: &NodeContext) -> Option<TypeId> {
        let dt = borrow!(ctx.designtime);
        dt.get_orm().get_property_type(
            &UniqueTemplateNodeIdentifier::build(self.stid.clone(), self.snid.clone()),
            self.name.as_str(),
        )
    }

    pub fn get_value(&self, ctx: &NodeContext) -> Option<ValueDefinition> {
        let dt = borrow!(ctx.designtime);
        dt.get_orm().get_property(
            &UniqueTemplateNodeIdentifier::build(self.stid.clone(), self.snid.clone()),
            self.name.as_str(),
        )
    }

    pub fn get_value_as_str(&self, ctx: &NodeContext) -> String {
        fn stringify(value: &ValueDefinition) -> String {
            match value {
                ValueDefinition::LiteralValue(Token { raw_value, .. })
                | ValueDefinition::Expression(Token { raw_value, .. }, _)
                | ValueDefinition::Identifier(Token { raw_value, .. }, _) => raw_value.to_owned(),
                ValueDefinition::Block(LiteralBlockDefinition { elements, .. }) => {
                    let mut block = String::new();
                    write!(block, "{{").unwrap();
                    for e in elements {
                        match e {
                            SettingElement::Setting(Token { raw_value, .. }, value) => {
                                write!(block, "{}: {} ", raw_value, stringify(value)).unwrap();
                            }
                            SettingElement::Comment(_) => (),
                        }
                    }
                    write!(block, "}}").unwrap();
                    block
                }
                _ => "(UNSUPPORTED BINDING TYPE)".to_owned(),
            }
        }

        self.get_value(ctx)
            .map(|v| stringify(&v))
            .unwrap_or_default()
    }

    pub fn set_value(&self, ctx: &NodeContext, val: &str) -> anyhow::Result<()> {
        match self.with_node_def(ctx, |mut node| {
            node.set_property(&self.name, val)?;
            node.save().map_err(|e| anyhow!("{:?}", e)).map(|_| ())
        }) {
            Some(res) => res,
            None => Err(anyhow!("has no definition")),
        }
    }

    pub fn with_node_def<T>(
        &self,
        ctx: &NodeContext,
        f: impl FnOnce(NodeBuilder<'_>) -> T,
    ) -> Option<T> {
        let mut dt = borrow_mut!(ctx.designtime);
        let node_definition = dt
            .get_orm_mut()
            .get_node(UniqueTemplateNodeIdentifier::build(
                self.stid.clone(),
                self.snid.clone(),
            ))?;
        Some(f(node_definition))
    }
}
