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
    pub fx_button_at_far_left: Property<bool>,

    // internal repr, always set to collection of above
    pub prop_type_ident_id: Property<usize>,
    pub data: Property<PropertyEditorData>,
    pub is_literal: Property<bool>,
    pub fx_text_color: Property<Color>,
    pub fx_background_color: Property<Color>,
}

impl PropertyEditor {
    pub fn on_mount(&mut self, ctxs: &NodeContext) {
        let stid = self.stid.clone();
        let snid = self.snid.clone();
        let name = self.name.clone();
        let ind = self.ind.clone();
        let manifest_ver = borrow!(ctxs.designtime).get_manifest_version();
        let deps = [
            stid.untyped(),
            snid.untyped(),
            name.untyped(),
            ind.untyped(),
            manifest_ver.untyped(),
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
        let ctx = ctxs.clone();
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

        let data = self.data.clone();
        let ctx = ctxs.clone();
        self.is_literal.replace_with(Property::computed(
            move || {
                let val = data.get().get_value(&ctx);
                !matches!(val, Some(ValueDefinition::Expression(_)))
            },
            &deps,
        ));

        let is_literal = self.is_literal.clone();
        let deps = [is_literal.untyped()];
        self.fx_text_color.replace_with(Property::computed(
            move || {
                match is_literal.get() {
                true => Color::WHITE,
                false => Color::rgb(207.into(), 31.into(), 201.into()),
            }},
            &deps,
        ));
        let is_literal = self.is_literal.clone();
        self.fx_background_color.replace_with(Property::computed(
            move || match is_literal.get() {
                true => Color::rgb(50.into(), 50.into(), 50.into()),
                false => Color::TEAL,
            },
            &deps,
        ));
    }

    pub fn toggle_literal(&mut self, ctx: &NodeContext, _event: Event<Click>) {
        let data = self.data.get();
        let val = data.get_value(&ctx);
        let res = if matches!(val, Some(ValueDefinition::Expression(_))) {
            data.set_value(&ctx, "")
        } else {
            let str_val = data.get_value_as_str(ctx);
            if str_val.is_empty() {
                // don't convert to expression if embty
                Ok(())
            } else {
                let str_val_expr = format!("{{{str_val}}}");
                data.set_value(&ctx, &str_val_expr)
            }
        };
        if let Err(e) = res {
            log::warn!("couldn't toggle expr: {e}");
        }
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
                ValueDefinition::LiteralValue(v) => v.to_string(),
                ValueDefinition::Expression(e) => format!("{{{}}}", e),
                ValueDefinition::DoubleBinding(i) | ValueDefinition::Identifier(i) => i.to_string(),
                ValueDefinition::Block(LiteralBlockDefinition { elements, .. }) => {
                    let mut block = String::new();
                    write!(block, "{{").unwrap();
                    for e in elements {
                        match e {
                            SettingElement::Setting(Token { token_value, .. }, value) => {
                                write!(block, "{}: {} ", token_value, stringify(value)).unwrap();
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
        let t = model::with_action_context(ctx, |ac| ac.transaction("updating property"));
        t.run(|| {
            match self.with_node_def(ctx, |mut node| {
                node.set_property(&self.name, val.trim())?;
                node.save().map_err(|e| anyhow!("{:?}", e)).map(|_| ())
            }) {
                Some(res) => res,
                None => Err(anyhow!("has no definition")),
            }
        })
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
            true,
        )?;
        Some(f(node_definition))
    }
}
