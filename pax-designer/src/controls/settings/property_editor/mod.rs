use anyhow::anyhow;
use pax_designtime::orm::template::node_builder::NodeBuilder;
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
/// Used by containers of property editors (added to local store)
/// to let PropertyEditors communicate upwardly what size they want to take up (height)
pub struct PropertyAreas(pub Property<Vec<f64>>);
impl Store for PropertyAreas {}

#[pax]
#[engine_import_path("pax_engine")]
#[file("controls/settings/property_editor/mod.pax")]
pub struct PropertyEditor {
    pub write_target: Property<WriteTarget>,
    pub name: Property<String>,
    pub ind: Property<usize>,
    pub is_custom_property: Property<bool>,

    // internal repr, always set to collection of above
    pub prop_type_ident_id: Property<usize>,
    pub data: Property<PropertyEditorData>,
    pub is_literal: Property<bool>,
    pub fx_text_color: Property<Color>,
    pub fx_background_color: Property<Color>,
}

#[pax]
#[engine_import_path("pax_engine")]
pub enum WriteTarget {
    #[default]
    None,
    TemplateNode(TypeId, TemplateNodeId),
    Class(TypeId, String),
}

impl PropertyEditor {
    pub fn on_mount(&mut self, ctxs: &NodeContext) {
        let ind = self.ind.clone();
        let write_target = self.write_target.clone();
        let name = self.name.clone();
        let manifest_ver = borrow!(ctxs.designtime).get_last_written_manifest_version();
        let deps = [
            ind.untyped(),
            name.untyped(),
            manifest_ver.untyped(),
            write_target.untyped(),
        ];
        self.data.replace_with(Property::computed(
            move || PropertyEditorData {
                editor_index: ind.get(),
                write_target: write_target.get(),
                name: name.get(),
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
            move || match is_literal.get() {
                true => Color::WHITE,
                false => Color::rgb(207.into(), 31.into(), 201.into()),
            },
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
                // don't convert to expression if empty
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
#[engine_import_path("pax_engine")]
pub struct PropertyEditorData {
    // this is used  by the custom properties to communicate back to the
    // settings editor to set its height
    pub editor_index: usize,
    pub write_target: WriteTarget,
    pub name: String,
}

impl PropertyEditorData {
    pub fn get_prop_type_id(&self, ctx: &NodeContext) -> Option<TypeId> {
        let dt = borrow!(ctx.designtime);
        let orm = dt.get_orm();
        match &self.write_target {
            WriteTarget::None => None,
            WriteTarget::TemplateNode(stid, snid) => orm.get_property_type(
                &UniqueTemplateNodeIdentifier::build(stid.clone(), snid.clone()),
                self.name.as_str(),
            ),
            WriteTarget::Class(stid, class_ident) => {
                // TODO PERF: expensive?
                let class = orm.get_class(stid, class_ident).ok()?;
                class
                    .into_iter()
                    .find_map(|(v, _, type_id)| (v == self.name).then_some(type_id))
                    .flatten()
            }
        }
    }

    pub fn get_value(&self, ctx: &NodeContext) -> Option<ValueDefinition> {
        let dt = borrow!(ctx.designtime);
        let orm = dt.get_orm();
        match &self.write_target {
            WriteTarget::None => None,
            WriteTarget::TemplateNode(stid, snid) => orm.get_property(
                &UniqueTemplateNodeIdentifier::build(stid.clone(), snid.clone()),
                self.name.as_str(),
            ),
            WriteTarget::Class(stid, class_ident) => {
                // TODO PERF: expensive?
                let class = orm.get_class(stid, class_ident).ok()?;
                class
                    .into_iter()
                    .find_map(|(v, value, _)| (v == self.name).then_some(value))
            }
        }
    }

    pub fn get_value_as_str(&self, ctx: &NodeContext) -> String {
        self.get_value(ctx)
            .map(|v| stringify_value_definition(&v))
            .unwrap_or_default()
    }

    pub fn set_value(&self, ctx: &NodeContext, val: &str) -> anyhow::Result<()> {
        // save-point before property edit
        let t = model::with_action_context(ctx, |ac| ac.transaction("updating property"));
        t.run(|| {
            let mut dt = borrow_mut!(ctx.designtime);
            let orm = dt.get_orm_mut();
            match &self.write_target {
                WriteTarget::None => (),
                WriteTarget::TemplateNode(stid, snid) => {
                    let mut node = orm
                        .get_node_builder(
                            UniqueTemplateNodeIdentifier::build(stid.clone(), snid.clone()),
                            // TODO how to handle this? The UI should probably show in some way
                            // if this already contains an expression, and if so not show the normal editor
                            true,
                        )
                        .ok_or_else(|| anyhow!("couldn't get node builder"))?;
                    node.set_property(&self.name, val.trim())?;
                    node.save().map_err(|e| anyhow!("{:?}", e))?;
                }
                WriteTarget::Class(stid, class_ident) => {
                    let mut class = orm.get_class_builder(stid.clone(), class_ident);
                    class.set_property(&self.name, val)?;
                    class
                        .save()
                        .map_err(|e| anyhow!("failed to write class property: {e}"))?;
                }
            }
            Ok(())
        })
    }
}

pub fn stringify_value_definition(value: &ValueDefinition) -> String {
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
                        write!(
                            block,
                            "{}: {} ",
                            token_value,
                            stringify_value_definition(value)
                        )
                        .unwrap();
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
