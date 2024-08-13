use anyhow::anyhow;
use pax_designtime::orm::template::builder::NodeBuilder;
use pax_designtime::orm::template::NodeAction;
use pax_engine::api::*;
use pax_engine::*;
use pax_manifest::*;
use pax_std::*;
use std::fmt::Write;

pub mod border_radius_property_editor;
pub mod color_property_editor;
pub mod corner_radii_property_editor;
pub mod direction_property_editor;
pub mod fill_property_editor;
pub mod stroke_property_editor;
pub mod text_property_editor;
pub mod text_style_property_editor;

use border_radius_property_editor::BorderRadiusPropertyEditor;
use color_property_editor::ColorPropertyEditor;
use corner_radii_property_editor::CornerRadiiPropertyEditor;
use direction_property_editor::DirectionPropertyEditor;
use fill_property_editor::FillPropertyEditor;
use stroke_property_editor::StrokePropertyEditor;
use text_property_editor::TextPropertyEditor;
use text_style_property_editor::TextStylePropertyEditor;

use crate::model;

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

                // log useful for quickly checking what to match when adding new editor:
                // log::info!("type ident gen for: ({}, {})", data.name, prop_type_ident);
                match (data.name.as_str(), prop_type_ident.as_str()) {
                    // TODO rename RectangleCornerRadii to CornerRadii and use for button/textbox
                    // etc. as well.
                    ("border_radius", "f64") => 6,
                    (_, "pax_engine::api::Color") => 5,
                    (_, "pax_engine::api::Fill") => 2,
                    (_, "pax_engine::api::Stroke") => 3,
                    (_, "pax_std::layout::stacker::StackerDirection") => 4,
                    (_, "pax_std::core::text::TextStyle") => 7,
                    (_, "pax_std::drawing::rectangle::RectangleCornerRadii") => 8,
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
                | ValueDefinition::Expression(Token { raw_value, .. })
                | ValueDefinition::Identifier(Token { raw_value, .. }) => raw_value.to_owned(),
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
        // save-point before property edit
        let mut t = model::with_action_context(ctx, |ac| ac.transaction("updating property"));
        t.run(|| {
            match self.with_node_def(ctx, |mut node| {
                node.set_property(&self.name, val)?;
                node.save().map_err(|e| anyhow!("{:?}", e)).map(|_| ())
            }) {
                Some(res) => res,
                None => Err(anyhow!("has no definition")),
            }
        });
        model::with_action_context(ctx, |ac| t.finish(ac))
    }

    pub fn with_node_def<T>(
        &self,
        ctx: &NodeContext,
        f: impl FnOnce(NodeBuilder<'_>) -> T,
    ) -> Option<T> {
        let mut dt = borrow_mut!(ctx.designtime);
        let node_definition = dt.get_orm_mut().get_node(
            UniqueTemplateNodeIdentifier::build(self.stid.clone(), self.snid.clone()),
            // TODO how to handle this? The UI should probably show in some way
            // if this already contains an expression, and if so not show the normal editor
            false,
        )?;
        Some(f(node_definition))
    }
}
