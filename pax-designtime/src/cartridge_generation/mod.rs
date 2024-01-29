use include_dir::{include_dir, Dir};
use pax_manifest::{HostCrateInfo, PaxManifest};
#[allow(unused_imports)]
use serde_derive::{Deserialize, Serialize};
#[allow(unused_imports)]
use serde_json;
use tera::{Context, Tera};

#[allow(unused)]
static TEMPLATE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates/cartridge_generation");
static DESIGN_CARTRIDGE_TEMPLATE: &str = "designtime-cartridge.tera";
static MACROS_TEMPLATE: &str = "macros.tera";

// {%- macro render_component_helper(component) -%}
// struct {{component.type_id_pascal_case}}Helper{}

// impl {{component.type_id_pascal_case}}Helper {

//     pub fn build_properties(defined_properties: HashMap<String,ValueDefinition>) -> Box<dyn Fn() -> Rc<RefCell<dyn Any>>> {
//         let mut properties = {{component.properties_struct}}::default();
//         {% for property in component.properties %}
//             if let Some(vd) = defined_properties.get("{{property.name}}") {
//                 properties.{{property.name}} =
//                     match vd {
//                         ValueDefinition::LiteralValue(lv) => {
//                             Box::new(PropertyLiteral::new(from_pax<{{property.type}}>(lv.raw_value)))
//                         },
//                         ValueDefinition::Expression(token, id) | ValueDefinition::Identifier(token,id) =>
//                         {
//                             Box::new(PropertyExpression::new(id.expect("Tried to use expression but it wasn't compiled")))
//                         },
//                         ValueDefinition::Block(block) => {
//                             Box::new(PropertyLiteral::new(from_pax<{{property.type}}>(block.raw_block_string)))
//                         }
//                     };
//             }
//         {% endfor %}
//         Box::new(move || Rc::new(RefCell::new(properties)))
//     }

//     pub fn build_handler(fn_name: String) -> fn(Rc<RefCell<dyn Any>>, &NodeContext, Option::<Box<dyn Any>>) {
//         match fn_name.as_str() {
//             {% for handler in component.handlers %}
//             "{{handler.name}}" => {
//                 |properties, ctx, args|{
//                     let properties = &mut *properties.as_ref().borrow_mut();
//                     if let Some(mut properties) = properties.downcast_mut::<{{component.properties_struct}}>() {
//                         // downcast args to handler.type
//                         if let Some(args) = args {
//                             if let Some(args) = args.downcast_ref::<{{handler.type}}>() {
//                                 {{component.type_id_pascal_case}}::{{handler.name}}(properties,ctx, args);
//                             } else {panic!("Failed to downcast args to {{handler.type}}")};
//                         } else {
//                             {{component.type_id_pascal_case}}::{{handler.name}}(properties,ctx);
//                         }
//                     } else {panic!("Failed to downcast properties to {{component.properties_struct}}")};
//                 }
//             },
//             {% endfor %}
//             _ => panic!("Unknown handler name {}", fn_name)
//         }
//     }

//     pub fn build_compute_properties_fn() -> Option<Box<dyn Fn(&ExpandedNode, &ExpressionTable, &Globals)>> {
//         Some(Box::new(|node, table, globals|{
//             let props = &node.properties;
//             let properties = &mut props.as_ref().borrow_mut();

//             if let Some(properties) = properties.downcast_mut::<{{component.properties_struct}}>() {

//                 {% for prop in component.properties %}
//                     if let Some(new_value) = table.compute_eased_value(properties.{{prop.name}}._get_transition_manager(), globals) {
//                         properties.{{ prop.name }}.set(new_value);
//                     } else if let Some(vtable_id) = properties.{{ prop.name }}._get_vtable_id() {
//                         let new_value_wrapped = table.compute_vtable_value(&node.stack, vtable_id);
//                         if let Ok(new_value) = new_value_wrapped.downcast::<{{prop.type}}>() {
//                             properties.{{ prop.name }}.set(*new_value);
//                         } else {
//                             panic!(
//                                 "generated code tried to downcast to incompatible type \"{{prop.type}}\" for property \"{{prop.name}}\" on {{component.properties_struct}}"
//                             );
//                         }
//                     }
//                 {% endfor %}

//             } else {
//                 panic!("Failed to downcast properties to {{component.properties_struct}}");
//             }
//         }))
//     }
// }
// {%- endmacro -%}


#[derive(Serialize)]
struct ComponentInfo {
    pub type_id: String,
    pub type_id_pascal_case: String,
    pub primitive_instance_import_path: Option<String>,
    pub properties_struct: String,
    pub properties: Vec<PropertyInfo>,
    pub handlers: Vec<HandlerInfo>,
}


#[derive(Serialize)]
struct PropertyInfo {
    pub name: String,
    pub property_type: String,
}


#[derive(Serialize)]
struct HandlerInfo {
    pub name: String,
    pub args_type: String,
}

#[derive(Serialize)]
struct CommonProperty {
    name: String,
    property_type: String,
    is_optional: bool,
}



#[derive(Serialize)]
pub struct TemplateArgsCodegenDesigntimeCartridge {
    components: Vec<ComponentInfo>,
    common_properties: Vec<CommonProperty>,
}


pub fn generate_designtime_cartridge(manifest: &PaxManifest, host_crate_info: &HostCrateInfo) -> String {
    // Make a test component info
    let t = ComponentInfo {
        type_id: "Rectangle".to_string(),
        type_id_pascal_case: "Rectangle".to_string(),
        properties_struct: "Rectangle".to_string(),
        properties: vec![
            PropertyInfo {
                name: "width".to_string(),
                property_type: "f32".to_string(),
            },
            PropertyInfo {
                name: "height".to_string(),
                property_type: "f32".to_string(),
            },
        ],
        handlers: vec![
            HandlerInfo {
                name: "on_click".to_string(),
                args_type: "ArgsClick".to_string(),
            },
            HandlerInfo {
                name: "on_hover".to_string(),
                args_type: "ArgsMouseMove".to_string(),
            },
        ],
        primitive_instance_import_path: Some("pax_std_primitives::rectangle::RectangleInstance".to_string()),
    };

    let mut common_properties = Vec::new();
    let test_common_property = CommonProperty {
        name: "width".to_string(),
        property_type: "Size".to_string(),
        is_optional: false,
    };
    let other_common_property = CommonProperty {
        name: "height".to_string(),
        property_type: "Size".to_string(),
        is_optional: false,
    };
    common_properties.push(test_common_property);


    let args = TemplateArgsCodegenDesigntimeCartridge { components: vec![t], common_properties };
    press_template_codegen_designtime_cartridge(args)
}




pub fn press_template_codegen_designtime_cartridge(
    args: TemplateArgsCodegenDesigntimeCartridge,
) -> String {
    let mut tera = Tera::default();

    tera.add_raw_template(
        MACROS_TEMPLATE,
        TEMPLATE_DIR
            .get_file(MACROS_TEMPLATE)
            .unwrap()
            .contents_utf8()
            .unwrap(),
    )
    .expect("Failed to add macros.tera");

    tera.add_raw_template(
        DESIGN_CARTRIDGE_TEMPLATE,
        TEMPLATE_DIR
            .get_file(DESIGN_CARTRIDGE_TEMPLATE)
            .unwrap()
            .contents_utf8()
            .unwrap(),
    )
    .expect("Failed to add designtime-cartridge.tera");

    tera.render(
        DESIGN_CARTRIDGE_TEMPLATE,
        &Context::from_serialize(args).unwrap(),
    )
    .expect("Failed to render template")
}
