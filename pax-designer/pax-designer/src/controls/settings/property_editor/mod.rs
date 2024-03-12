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

pub mod fill_property_editor;
pub mod text_property_editor;

use fill_property_editor::FillPropertyEditor;
use text_property_editor::TextPropertyEditor;

#[pax]
#[file("controls/settings/property_editor/mod.pax")]
pub struct PropertyEditor {
    pub name: Property<StringBox>,
    pub ind: Property<Option<Numeric>>,
    pub stid: Property<TypeId>,
    pub snid: Property<TemplateNodeId>,

    // internal repr, always set to collection of above
    pub prop_type_ident_id: Property<usize>,
    pub data: Property<PropertyEditorData>,
}

impl PropertyEditor {
    pub fn tick(&mut self, ctx: &NodeContext) {
        let prop = self.data.get();
        if &self.name.get().string != &prop.name
            || self.stid.get() != &prop.stid
            || self.snid.get() != &prop.snid
        {
            let prop = PropertyEditorData {
                editor_index: self.ind.get().map(|v| v.to_int() as usize),
                name: self.name.get().string.clone(),
                stid: self.stid.get().clone(),
                snid: self.snid.get().clone(),
            };
            let prop_type_ident = prop
                .get_prop_type_id(ctx)
                .unwrap_or_default()
                .get_unique_identifier();

            // Id corresponds to a given editor type in the pax file
            // 1 = general text editor (without any special features)
            // 2 = color picker
            let prop_type_ident_id = match prop_type_ident.as_str() {
                "pax_engine::api::Size" => 1,
                "pax_engine::api::Rotation" => 1,
                "pax_engine::api::Numeric" => 1,
                "String" => 1,
                "pax_engine::api::Transform2D" => 1,
                "pax_designer::pax_reexports::pax_std::types::Stroke" => 1,
                "pax_designer::pax_reexports::pax_std::types::Fill" => 2,
                "pax_designer::pax_reexports::pax_std::types::RectangleCornerRadii" => 1,
                "pax_engine::api::Transform" => 1,
                _ => 1,
            };
            self.prop_type_ident_id.set(prop_type_ident_id);
            self.data.set(prop);
        }
    }
}

#[pax]
pub struct PropertyEditorData {
    // this is used  by the custom properties to communicate back to the
    // settings editor to set its height
    pub editor_index: Option<usize>,
    pub name: String,
    pub stid: TypeId,
    pub snid: TemplateNodeId,
}

impl PropertyEditorData {
    pub fn get_prop_type_id(&self, ctx: &NodeContext) -> Option<TypeId> {
        let dt = ctx.designtime.borrow();
        dt.get_orm().get_property_type(
            &UniqueTemplateNodeIdentifier::build(self.stid.clone(), self.snid.clone()),
            self.name.as_str(),
        )
    }

    pub fn get_value(&self, ctx: &NodeContext) -> Option<ValueDefinition> {
        let dt = ctx.designtime.borrow();
        dt.get_orm().get_property(
            &UniqueTemplateNodeIdentifier::build(self.stid.clone(), self.snid.clone()),
            self.name.as_str(),
        )
    }

    pub fn get_value_as_str(&self, ctx: &NodeContext) -> String {
        match self.get_value(ctx) {
            Some(
                ValueDefinition::LiteralValue(Token { raw_value, .. })
                | ValueDefinition::Expression(Token { raw_value, .. }, _)
                | ValueDefinition::Identifier(Token { raw_value, .. }, _),
            ) => raw_value,
            Some(_) => "ERROR: UNSUPPORTED BINDING TYPE".to_owned(),
            None => "".to_owned(),
        }
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
        let mut dt = ctx.designtime.borrow_mut();
        let node_definition = dt
            .get_orm_mut()
            .get_node(UniqueTemplateNodeIdentifier::build(
                self.stid.clone(),
                self.snid.clone(),
            ))?;
        Some(f(node_definition))
    }
}
