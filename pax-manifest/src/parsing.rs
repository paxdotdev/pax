use crate::*;
use pax_lang::interpreter::parse_pax_expression_from_pair;
use pax_lang::{
    from_pax, get_pax_pratt_parser, parse_pax_expression, parse_pax_str, Pair, Pairs, PaxParser,
    PrattParser, Rule, Span,
};
use pax_runtime_api::{Color, Fill, Size, Stroke};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

pub fn parse_template_from_component_definition_string(
    ctx: &mut TemplateNodeParseContext,
    pax: &str,
    pax_component_definition: Pair<Rule>,
) {
    pax_component_definition
        .into_inner()
        .for_each(|pair| match pair.as_rule() {
            Rule::root_tag_pair => {
                recurse_visit_tag_pairs_for_template(
                    ctx,
                    pair.into_inner().next().unwrap(),
                    pax,
                    TreeLocation::Root,
                );
            }
            _ => {}
        });
}

pub struct TemplateNodeParseContext {
    pub template: ComponentTemplate,
    pub pascal_identifier_to_type_id_map: HashMap<String, TypeId>,
}

fn recurse_visit_tag_pairs_for_template(
    ctx: &mut TemplateNodeParseContext,
    any_tag_pair: Pair<Rule>,
    pax: &str,
    location: TreeLocation,
) {
    match any_tag_pair.as_rule() {
        Rule::matched_tag => {
            //matched_tag => open_tag > pascal_identifier
            let matched_tag = any_tag_pair;
            let mut open_tag = matched_tag
                .clone()
                .into_inner()
                .next()
                .unwrap()
                .into_inner();
            let pascal_identifier = open_tag.next().unwrap().as_str();

            let template_node = TemplateNodeDefinition {
                type_id: TypeId::build_singleton(
                    &ctx.pascal_identifier_to_type_id_map
                        .get(pascal_identifier)
                        .expect(&format!("Template key not found {}", &pascal_identifier))
                        .to_string(),
                    Some(&pascal_identifier.to_string()),
                ),
                settings: parse_inline_attribute_from_final_pairs_of_tag(open_tag, pax),
                raw_comment_string: None,
                control_flow_settings: None,
            };

            let id = match location {
                TreeLocation::Root => ctx.template.add_root_node_back(template_node),
                TreeLocation::Parent(id) => ctx.template.add_child_back(id, template_node),
            };

            //recurse into inner_nodes
            let prospective_inner_nodes = matched_tag.into_inner().nth(1).unwrap();
            match prospective_inner_nodes.as_rule() {
                Rule::inner_nodes => {
                    let inner_nodes = prospective_inner_nodes;
                    inner_nodes.into_inner().for_each(|sub_tag_pair| {
                        recurse_visit_tag_pairs_for_template(
                            ctx,
                            sub_tag_pair,
                            pax,
                            TreeLocation::Parent(id.clone().get_template_node_id()),
                        );
                    })
                }
                _ => {
                    panic!("wrong prospective inner nodes (or nth)")
                }
            }
        }
        Rule::self_closing_tag => {
            let mut tag_pairs = any_tag_pair.into_inner();
            let pascal_identifier = tag_pairs.next().unwrap().as_str();

            let type_id = if let Some(type_id) =
                ctx.pascal_identifier_to_type_id_map.get(pascal_identifier)
            {
                type_id.clone()
            } else {
                TypeId::build_blank_component(pascal_identifier)
            };
            let template_node = TemplateNodeDefinition {
                type_id,
                settings: parse_inline_attribute_from_final_pairs_of_tag(tag_pairs, pax),
                raw_comment_string: None,
                control_flow_settings: None,
            };
            let _ = match location {
                TreeLocation::Root => ctx.template.add_root_node_back(template_node),
                TreeLocation::Parent(id) => ctx.template.add_child_back(id, template_node),
            };
        }
        Rule::statement_control_flow => {
            /* statement_control_flow = {(statement_if | statement_for | statement_slot)} */

            let any_tag_pair = any_tag_pair.into_inner().next().unwrap();
            let _template_node_definition = match any_tag_pair.as_rule() {
                Rule::statement_if => {
                    let mut statement_if = any_tag_pair.into_inner();
                    let expression_body = statement_if.next().unwrap();
                    let condition_expression =
                        parse_pax_expression(expression_body.as_str()).unwrap();
                    let expression_info = ExpressionInfo::new(condition_expression);

                    //`if` TemplateNodeDefinition
                    let template_node = TemplateNodeDefinition {
                        control_flow_settings: Some(ControlFlowSettingsDefinition {
                            condition_expression: Some(expression_info),
                            slot_index_expression: None,
                            repeat_predicate_definition: None,
                            repeat_source_expression: None,
                        }),
                        type_id: TypeId::build_if(),
                        settings: None,
                        raw_comment_string: None,
                    };

                    let id = match location {
                        TreeLocation::Root => ctx.template.add_root_node_back(template_node),
                        TreeLocation::Parent(id) => ctx.template.add_child_back(id, template_node),
                    };

                    let prospective_inner_nodes = statement_if.next();

                    if let Some(inner_nodes) = prospective_inner_nodes {
                        inner_nodes.into_inner().for_each(|sub_tag_pair| {
                            recurse_visit_tag_pairs_for_template(
                                ctx,
                                sub_tag_pair,
                                pax,
                                TreeLocation::Parent(id.clone().get_template_node_id()),
                            );
                        })
                    }
                }
                Rule::statement_for => {
                    let mut cfavd = ControlFlowSettingsDefinition::default();
                    let mut for_statement = any_tag_pair.clone().into_inner();
                    let mut predicate_declaration = for_statement.next().unwrap().into_inner();
                    let source = for_statement.next().unwrap();

                    let prospective_inner_nodes = for_statement.next();

                    if predicate_declaration.clone().count() > 1 {
                        //tuple, like the `elem, i` in `for (elem, i) in self.some_list`
                        let elem = predicate_declaration.next().unwrap();
                        let index = predicate_declaration.next().unwrap();
                        cfavd.repeat_predicate_definition =
                            Some(ControlFlowRepeatPredicateDefinition::ElemIdIndexId(
                                elem.as_str().to_owned(),
                                index.as_str().to_owned(),
                            ));
                    } else {
                        let elem = predicate_declaration.next().unwrap();
                        //single identifier, like the `elem` in `for elem in self.some_list`
                        cfavd.repeat_predicate_definition = Some(
                            ControlFlowRepeatPredicateDefinition::ElemId(elem.as_str().to_owned()),
                        );
                    }

                    let inner_source = source.into_inner().next().unwrap();
                    /* statement_for_source = { xo_range | xo_symbol } */
                    let repeat_source_definition =
                        ExpressionInfo::new(parse_pax_expression(inner_source.as_str()).unwrap());
                    cfavd.repeat_source_expression = Some(repeat_source_definition);

                    //`for` TemplateNodeDefinition
                    let template_node = TemplateNodeDefinition {
                        type_id: TypeId::build_repeat(),
                        control_flow_settings: Some(cfavd),
                        settings: None,
                        raw_comment_string: None,
                    };

                    let id = match location {
                        TreeLocation::Root => ctx.template.add_root_node_back(template_node),
                        TreeLocation::Parent(id) => ctx.template.add_child_back(id, template_node),
                    };

                    if let Some(inner_nodes) = prospective_inner_nodes {
                        inner_nodes.into_inner().for_each(|sub_tag_pair| {
                            recurse_visit_tag_pairs_for_template(
                                ctx,
                                sub_tag_pair,
                                pax,
                                TreeLocation::Parent(id.clone().get_template_node_id()),
                            );
                        })
                    }
                }
                Rule::statement_slot => {
                    let mut statement_slot = any_tag_pair.into_inner();
                    let expression_body = statement_slot.next().unwrap();
                    let slot_expression = ExpressionInfo::new(
                        parse_pax_expression(expression_body.as_str()).unwrap(),
                    );
                    let template_node = TemplateNodeDefinition {
                        control_flow_settings: Some(ControlFlowSettingsDefinition {
                            condition_expression: None,
                            slot_index_expression: Some(slot_expression),
                            repeat_predicate_definition: None,
                            repeat_source_expression: None,
                        }),
                        type_id: TypeId::build_slot(),
                        settings: None,
                        raw_comment_string: None,
                    };

                    let _ = match location {
                        TreeLocation::Root => ctx.template.add_root_node_back(template_node),
                        TreeLocation::Parent(id) => ctx.template.add_child_back(id, template_node),
                    };
                }
                _ => {
                    unreachable!("Parsing error: {:?}", any_tag_pair.as_rule());
                }
            };
        }
        Rule::comment => {
            let template_node = TemplateNodeDefinition {
                control_flow_settings: None,
                type_id: TypeId::build_comment(),
                settings: None,
                raw_comment_string: Some(any_tag_pair.as_str().to_string()),
            };
            let _ = match location {
                TreeLocation::Root => ctx.template.add_root_node_back(template_node),
                TreeLocation::Parent(id) => ctx.template.add_child_back(id, template_node),
            };
        }
        Rule::node_inner_content => {
            //For example:  `<Text>"I am inner content"</Text>`
            unimplemented!("Inner content not yet supported");
        }
        _ => {
            unreachable!("Parsing error: {:?}", any_tag_pair.as_rule());
        }
    }
}

fn parse_literal_function(literal_function_full: Pair<Rule>) -> Token {
    let literal_function = literal_function_full.clone().into_inner().next().unwrap();

    let location_info = span_to_location(&literal_function.as_span());
    let literal_function_token = Token::new(literal_function.as_str().to_string(), location_info);
    literal_function_token
}

fn parse_event_id(event_id_full: Pair<Rule>, pax: &str) -> Token {
    let event_id = event_id_full.clone().into_inner().next().unwrap();

    let event_id_location = span_to_location(&event_id.as_span());
    let event_id_token = Token::new(event_id.as_str().to_string(), event_id_location);
    event_id_token
}

fn parse_inline_attribute_from_final_pairs_of_tag(
    final_pairs_of_tag: Pairs<Rule>,
    pax: &str,
) -> Option<Vec<SettingElement>> {
    let vec: Vec<SettingElement> = final_pairs_of_tag
        .map(|attribute_key_value_pair| {
            match attribute_key_value_pair
                .clone()
                .into_inner()
                .next()
                .unwrap()
                .as_rule()
            {
                Rule::double_binding => {
                    let mut kv = attribute_key_value_pair.into_inner();
                    let mut double_binding = kv.next().unwrap().into_inner();

                    let setting: Pair<Rule> = double_binding.next().unwrap();
                    let property = double_binding.next().unwrap();
                    let setting_location = span_to_location(&setting.as_span());
                    let setting_token = Token::new(setting.as_str().to_string(), setting_location);

                    SettingElement::Setting(
                        setting_token,
                        ValueDefinition::DoubleBinding(PaxIdentifier::new(property.as_str())),
                    )
                }
                Rule::attribute_event_binding => {
                    // attribute_event_binding = {event_id ~ "=" ~ literal_function}
                    let mut kv = attribute_key_value_pair.into_inner();
                    let mut attribute_event_binding = kv.next().unwrap().into_inner();

                    let event_id_token =
                        parse_event_id(attribute_event_binding.next().unwrap(), pax);

                    let literal_function = attribute_event_binding.next().unwrap().as_str();
                    SettingElement::Setting(
                        event_id_token,
                        ValueDefinition::EventBindingTarget(PaxIdentifier::new(literal_function)),
                    )
                }
                _ => {
                    //Vanilla `key=value` setting pair

                    let mut kv = attribute_key_value_pair.into_inner();
                    let key = kv.next().unwrap();
                    let key_location = span_to_location(&key.as_span());
                    let key_token = Token::new(key.as_str().to_string(), key_location);
                    let value_outer =
                        kv.next()
                            .expect(&format!("key: {}, kvs: {}", key.as_str(), kv));
                    let value = value_outer.clone().into_inner().next().expect(&format!(
                        "key: {}, value: {}",
                        key.as_str(),
                        value_outer.as_str()
                    ));
                    let value_definition = parse_value_definition(value);
                    SettingElement::Setting(key_token, value_definition)
                }
            }
        })
        .collect();

    if vec.len() > 0 {
        Some(vec)
    } else {
        None
    }
}

pub fn parse_value_definition(value: Pair<Rule>) -> ValueDefinition {
    match value.as_rule() {
        Rule::literal_value => {
            let inner = value.into_inner().next().unwrap();
            match inner.as_rule() {
                Rule::literal_object =>     {
                    ValueDefinition::Block(derive_value_definition_from_literal_object_pair(inner))
                },
                _ => {
                    let literal = from_pax(inner.as_str()).expect(&format!("Unable to parse literal: {:?}", inner));
                    ValueDefinition::LiteralValue(literal)
                }
            }
        }
        Rule::expression_body => {
            let expression =
                parse_pax_expression_from_pair(value).expect("Unable to parse expression");
            ValueDefinition::Expression(ExpressionInfo::new(expression))
        }
        Rule::identifier => {
            let identifier = PaxIdentifier::new(value.as_str());
            ValueDefinition::Identifier(identifier)
        }
        _ => {
            unreachable!(
                "Unexpected attribute value pair rule: {:?}",
                value.as_rule()
            );
        }
    }
}

fn derive_value_definition_from_literal_object_pair(
    literal_object: Pair<Rule>,
) -> LiteralBlockDefinition {
    let mut literal_object_pairs = literal_object.into_inner();

    if let None = literal_object_pairs.peek() {
        return LiteralBlockDefinition {
            explicit_type_pascal_identifier: None,
            elements: vec![],
        };
    }

    let explicit_type_pascal_identifier = match literal_object_pairs.peek().unwrap().as_rule() {
        Rule::pascal_identifier => {
            let raw_value = literal_object_pairs.next().unwrap();
            let raw_value_location = span_to_location(&raw_value.as_span());
            let token = Token::new(raw_value.as_str().to_string(), raw_value_location);
            Some(token)
        }
        _ => None,
    };

    LiteralBlockDefinition {
        explicit_type_pascal_identifier,
        elements: literal_object_pairs
            .map(
                |settings_key_value_pair| match settings_key_value_pair.as_rule() {
                    Rule::settings_key_value_pair => {
                        let mut pairs = settings_key_value_pair.into_inner();

                        let setting_key = pairs.next().unwrap().into_inner().next().unwrap();
                        let setting_key_location = span_to_location(&setting_key.as_span());
                        let setting_key_token =
                            Token::new(setting_key.as_str().to_string(), setting_key_location);
                        let value = pairs.next().unwrap().into_inner().next().unwrap();
                        let setting_value_definition = parse_value_definition(value);

                        SettingElement::Setting(setting_key_token, setting_value_definition)
                    }
                    Rule::comment => {
                        let comment = settings_key_value_pair.as_str().to_string();
                        SettingElement::Comment(comment)
                    }
                    _ => {
                        unreachable!(
                            "Parsing error 2314314145: {:?}",
                            settings_key_value_pair.as_rule()
                        );
                    }
                },
            )
            .collect(),
    }
}

pub fn parse_settings_from_component_definition_string(
    pax: &str,
    pax_component_definition: Pair<Rule>,
) -> Vec<SettingsBlockElement> {
    let mut settings: Vec<SettingsBlockElement> = vec![];

    pax_component_definition
        .into_inner()
        .for_each(|top_level_pair| {
            match top_level_pair.as_rule() {
                Rule::settings_block_declaration => {
                    top_level_pair
                        .into_inner()
                        .for_each(|top_level_settings_block_entity| {
                            match top_level_settings_block_entity.as_rule() {
                                Rule::settings_event_binding => {
                                    //event handler binding in the form of `@pre_render: handle_pre_render`
                                    let mut settings_event_binding_pairs =
                                        top_level_settings_block_entity.into_inner();
                                    let event_id_token = parse_event_id(
                                        settings_event_binding_pairs.next().unwrap(),
                                        pax,
                                    );
                                    let literal_function_token = parse_literal_function(
                                        settings_event_binding_pairs.next().unwrap(),
                                    );
                                    let handler_element: SettingsBlockElement =
                                        SettingsBlockElement::Handler(
                                            event_id_token,
                                            vec![literal_function_token],
                                        );
                                    settings.push(handler_element);
                                }
                                Rule::selector_block => {
                                    //selector_block => settings_key_value_pair where v is a ValueDefinition
                                    let mut selector_block_pairs =
                                        top_level_settings_block_entity.into_inner();
                                    //first pair is the selector itself
                                    let raw_selector = selector_block_pairs.next().unwrap();
                                    let raw_value_location =
                                        span_to_location(&raw_selector.as_span());
                                    let selector: String = raw_selector
                                        .as_str()
                                        .chars()
                                        .filter(|c| !c.is_whitespace())
                                        .collect();
                                    let token = Token::new(selector, raw_value_location);
                                    let literal_object = selector_block_pairs.next().unwrap();

                                    settings.push(SettingsBlockElement::SelectorBlock(
                                        token,
                                        derive_value_definition_from_literal_object_pair(
                                            literal_object,
                                        ),
                                    ));
                                }
                                Rule::comment => {
                                    let comment =
                                        top_level_settings_block_entity.as_str().to_string();
                                    settings.push(SettingsBlockElement::Comment(comment));
                                }
                                _ => {
                                    unreachable!(
                                        "Parsing error: {:?}",
                                        top_level_settings_block_entity.as_rule()
                                    );
                                }
                            }
                        });
                }
                _ => {}
            }
        });
    settings
}

pub struct ParsingContext {
    /// Used to track which files/sources have been visited during parsing,
    /// to prevent duplicate parsing
    pub visited_type_ids: HashSet<TypeId>,

    pub main_component_type_id: TypeId,

    pub component_definitions: BTreeMap<TypeId, ComponentDefinition>,

    pub template_map: HashMap<String, TypeId>,

    pub template_node_definitions: ComponentTemplate,

    pub type_table: TypeTable,
}

impl Default for ParsingContext {
    fn default() -> Self {
        Self {
            main_component_type_id: TypeId::default(),
            visited_type_ids: HashSet::new(),
            component_definitions: BTreeMap::new(),
            template_map: HashMap::new(),
            type_table: get_primitive_type_table(),
            template_node_definitions: ComponentTemplate::default(),
        }
    }
}

#[derive(Debug)]
pub struct ParsingError {
    pub error_name: String,
    pub error_message: String,
    pub matched_string: String,
    pub start: (usize, usize),
    pub end: (usize, usize),
}

/// From a raw string of Pax representing a single component, parse a complete ComponentDefinition
pub fn assemble_component_definition(
    mut ctx: ParsingContext,
    pax: &str,
    is_main_component: bool,
    template_map: HashMap<String, TypeId>,
    module_path: &str,
    self_type_id: TypeId,
    component_source_file_path: &str,
) -> (ParsingContext, ComponentDefinition) {
    let mut tpc = TemplateNodeParseContext {
        pascal_identifier_to_type_id_map: template_map,
        template: ComponentTemplate::new(
            self_type_id.clone(),
            Some(component_source_file_path.to_owned()),
        ),
    };

    let ast = parse_pax_str(Rule::pax_component_definition, pax).expect("Unsuccessful parse");

    parse_template_from_component_definition_string(&mut tpc, pax, ast.clone());
    let modified_module_path = if module_path.starts_with("parser") {
        module_path.replacen("parser", "crate", 1)
    } else {
        module_path.to_string()
    };

    //populate template_node_definitions vec, needed for traversing node tree at codegen-time
    ctx.template_node_definitions = tpc.template.clone();

    let settings = parse_settings_from_component_definition_string(pax, ast);

    let new_def = ComponentDefinition {
        is_primitive: false,
        is_struct_only_component: false,
        is_main_component,
        primitive_instance_import_path: None,
        type_id: self_type_id,
        template: Some(tpc.template),
        settings: Some(settings),
        module_path: modified_module_path,
    };

    (ctx, new_def)
}

pub fn clean_module_path(module_path: &str) -> String {
    if module_path.starts_with("parser") {
        module_path.replacen("parser", "crate", 1)
    } else {
        module_path.to_string()
    }
}

pub fn assemble_struct_only_component_definition(
    ctx: ParsingContext,
    module_path: &str,
    self_type_id: TypeId,
) -> (ParsingContext, ComponentDefinition) {
    let modified_module_path = clean_module_path(module_path);

    let new_def = ComponentDefinition {
        type_id: self_type_id,
        is_main_component: false,
        is_primitive: false,
        is_struct_only_component: true,
        module_path: modified_module_path,
        primitive_instance_import_path: None,
        template: None,
        settings: None,
    };
    (ctx, new_def)
}

pub fn assemble_primitive_definition(
    module_path: &str,
    primitive_instance_import_path: String,
    self_type_id: TypeId,
) -> ComponentDefinition {
    let modified_module_path = clean_module_path(module_path);

    ComponentDefinition {
        is_primitive: true,
        is_struct_only_component: false,
        primitive_instance_import_path: Some(primitive_instance_import_path),
        is_main_component: false,
        type_id: self_type_id,
        template: None,
        settings: None,
        module_path: modified_module_path,
    }
}

pub fn assemble_type_definition(
    mut ctx: ParsingContext,
    property_definitions: Vec<PropertyDefinition>,
    inner_iterable_type_id: Option<TypeId>,
    self_type_id: TypeId,
) -> (ParsingContext, TypeDefinition) {
    let new_def = TypeDefinition {
        type_id: self_type_id.clone(),
        inner_iterable_type_id,
        property_definitions,
    };

    ctx.type_table.insert(self_type_id, new_def.clone());

    (ctx, new_def)
}

/// Given a Pest Span returns starting and ending (line,col)
fn span_to_location(span: &Span) -> LocationInfo {
    let start = (
        span.start_pos().line_col().0 - 1,
        span.start_pos().line_col().1 - 1,
    );
    let end = (
        span.end_pos().line_col().0 - 1,
        span.end_pos().line_col().1 - 1,
    );
    LocationInfo {
        start_line_col: start,
        end_line_col: end,
    }
}

/// This trait is used only to extend primitives like u64
/// with the parser-time method `parse_to_manifest`.  This
/// allows the parser binary to codegen calls to `::parse_to_manifest()` even
/// on primitive types
pub trait Reflectable {
    fn parse_to_manifest(mut ctx: ParsingContext) -> (ParsingContext, Vec<PropertyDefinition>) {
        //Default impl for primitives and pax_runtime_api
        let type_id = Self::get_type_id();
        let td = TypeDefinition {
            type_id: type_id.clone(),
            inner_iterable_type_id: None,
            property_definitions: vec![],
        };

        if !ctx.type_table.contains_key(&type_id) {
            ctx.type_table.insert(type_id, td);
        }

        (ctx, vec![])
    }

    ///The import path is the fully namespace-qualified path for a type, like `std::vec::Vec`
    ///This is distinct from type_id ONLY when the type has generics, like Vec, where
    ///the type_id is distinct across a Vec<Foo> and a Vec<Bar>.  In both cases of Vec,
    ///the import_path will remain the same.
    fn get_import_path() -> String {
        //This default is used by primitives but expected to
        //be overridden by userland Pax components / primitives
        Self::get_self_pascal_identifier()
    }

    fn get_self_pascal_identifier() -> String;

    fn get_type_id() -> TypeId;

    fn get_iterable_type_id() -> Option<TypeId> {
        //Most types do not have an iterable type (e.g. the T in Vec<T>) —
        //it is the responsibility of iterable types to override this fn
        None
    }
}

impl Reflectable for () {
    fn get_self_pascal_identifier() -> String {
        "()".to_string()
    }

    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&Self::get_self_pascal_identifier())
    }
}

impl Reflectable for usize {
    fn get_self_pascal_identifier() -> String {
        "usize".to_string()
    }

    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&Self::get_self_pascal_identifier())
    }
}
impl Reflectable for isize {
    fn get_self_pascal_identifier() -> String {
        "isize".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&Self::get_self_pascal_identifier())
    }
}
impl Reflectable for i128 {
    fn get_self_pascal_identifier() -> String {
        "i128".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&Self::get_self_pascal_identifier())
    }
}
impl Reflectable for u128 {
    fn get_self_pascal_identifier() -> String {
        "u128".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&Self::get_self_pascal_identifier())
    }
}
impl Reflectable for i64 {
    fn get_self_pascal_identifier() -> String {
        "i64".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&Self::get_self_pascal_identifier())
    }
}
impl Reflectable for u64 {
    fn get_self_pascal_identifier() -> String {
        "u64".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&Self::get_self_pascal_identifier())
    }
}
impl Reflectable for i32 {
    fn get_self_pascal_identifier() -> String {
        "i32".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&Self::get_self_pascal_identifier())
    }
}
impl Reflectable for u32 {
    fn get_self_pascal_identifier() -> String {
        "u32".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&Self::get_self_pascal_identifier())
    }
}
impl Reflectable for i8 {
    fn get_self_pascal_identifier() -> String {
        "i8".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&Self::get_self_pascal_identifier())
    }
}
impl Reflectable for u8 {
    fn get_self_pascal_identifier() -> String {
        "u8".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&Self::get_self_pascal_identifier())
    }
}
impl Reflectable for f64 {
    fn get_self_pascal_identifier() -> String {
        "f64".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&&Self::get_self_pascal_identifier())
    }
}
impl Reflectable for f32 {
    fn get_self_pascal_identifier() -> String {
        "f32".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&Self::get_self_pascal_identifier())
    }
}
impl Reflectable for bool {
    fn get_self_pascal_identifier() -> String {
        "bool".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&Self::get_self_pascal_identifier())
    }
}

impl Reflectable for char {
    fn get_self_pascal_identifier() -> String {
        "char".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&Self::get_self_pascal_identifier())
    }
}

impl Reflectable for std::string::String {
    fn get_import_path() -> String {
        "std::string::String".to_string()
    }
    fn get_self_pascal_identifier() -> String {
        "String".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_singleton(
            &Self::get_import_path(),
            Some(&Self::get_self_pascal_identifier()),
        )
    }
}

impl<T> Reflectable for std::rc::Rc<T> {
    fn get_import_path() -> String {
        "std::rc::Rc".to_string()
    }
    fn get_self_pascal_identifier() -> String {
        "Rc".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_singleton(
            &Self::get_import_path(),
            Some(&Self::get_self_pascal_identifier()),
        )
    }
}

impl<T: Reflectable> Reflectable for std::option::Option<T> {
    fn parse_to_manifest(mut ctx: ParsingContext) -> (ParsingContext, Vec<PropertyDefinition>) {
        let type_id = Self::get_type_id();
        let td = TypeDefinition {
            type_id: type_id.clone(),
            inner_iterable_type_id: None,
            property_definitions: vec![],
        };

        if !ctx.type_table.contains_key(&type_id) {
            ctx.type_table.insert(type_id, td);
        }

        let (ctx, _) = T::parse_to_manifest(ctx);
        (ctx, vec![]) //Option itself has no PAXEL-addressable properties
    }
    fn get_import_path() -> String {
        "std::option::Option".to_string()
    }
    fn get_self_pascal_identifier() -> String {
        "Option".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_option(&format!("{}", &T::get_type_id()))
    }
}

impl Reflectable for TypeId {
    fn get_import_path() -> String {
        "pax_manifest::TypeId".to_string()
    }

    fn get_self_pascal_identifier() -> String {
        "TypeId".to_string()
    }

    fn get_type_id() -> TypeId {
        TypeId::build_singleton(
            &Self::get_import_path(),
            Some(&Self::get_self_pascal_identifier()),
        )
    }
}

impl Reflectable for TemplateNodeId {
    fn get_import_path() -> String {
        "pax_manifest::TemplateNodeId".to_string()
    }

    fn get_self_pascal_identifier() -> String {
        "TemplateNodeId".to_string()
    }

    fn get_type_id() -> TypeId {
        TypeId::build_singleton(
            &Self::get_import_path(),
            Some(&Self::get_self_pascal_identifier()),
        )
    }
}

impl Reflectable for Fill {
    fn get_import_path() -> String {
        "pax_engine::api::Fill".to_string()
    }

    fn get_self_pascal_identifier() -> String {
        "Fill".to_string()
    }

    fn get_type_id() -> TypeId {
        TypeId::build_singleton(
            &Self::get_import_path(),
            Some(&Self::get_self_pascal_identifier()),
        )
    }
}

impl Reflectable for Stroke {
    fn parse_to_manifest(mut ctx: ParsingContext) -> (ParsingContext, Vec<PropertyDefinition>) {
        let type_id = Self::get_type_id();
        let mut flags = PropertyDefinitionFlags::default();
        flags.is_property_wrapped = true;
        let td = TypeDefinition {
            type_id: type_id.clone(),
            inner_iterable_type_id: None,
            property_definitions: vec![
                PropertyDefinition {
                    name: "color".to_string(),
                    flags: flags.clone(),
                    type_id: Color::get_type_id(),
                },
                PropertyDefinition {
                    name: "width".to_string(),
                    flags: flags,
                    type_id: Size::get_type_id(),
                },
            ],
        };

        if !ctx.type_table.contains_key(&type_id) {
            ctx.type_table.insert(type_id, td);
        }
        let color_type_id = Color::get_type_id();
        if !ctx.type_table.contains_key(&color_type_id) {
            ctx.type_table.insert(
                color_type_id.clone(),
                TypeDefinition {
                    type_id: color_type_id,
                    inner_iterable_type_id: None,
                    property_definitions: vec![],
                },
            );
        }
        let size_type_id = Size::get_type_id();
        if !ctx.type_table.contains_key(&size_type_id) {
            ctx.type_table.insert(
                size_type_id.clone(),
                TypeDefinition {
                    type_id: size_type_id,
                    inner_iterable_type_id: None,
                    property_definitions: vec![],
                },
            );
        }

        (ctx, vec![])
    }

    fn get_import_path() -> String {
        "pax_engine::api::Stroke".to_string()
    }

    fn get_self_pascal_identifier() -> String {
        "Stroke".to_string()
    }

    fn get_type_id() -> TypeId {
        TypeId::build_singleton(
            &Self::get_import_path(),
            Some(&Self::get_self_pascal_identifier()),
        )
    }
}

impl Reflectable for pax_runtime_api::Size {
    fn get_import_path() -> String {
        "pax_engine::api::Size".to_string()
    }

    fn get_self_pascal_identifier() -> String {
        "Size".to_string()
    }

    fn get_type_id() -> TypeId {
        TypeId::build_singleton(
            &Self::get_import_path(),
            Some(&Self::get_self_pascal_identifier()),
        )
    }
}

impl Reflectable for pax_runtime_api::Color {
    fn get_import_path() -> String {
        "pax_engine::api::Color".to_string()
    }

    fn get_self_pascal_identifier() -> String {
        "Color".to_string()
    }

    fn get_type_id() -> TypeId {
        TypeId::build_singleton(
            &Self::get_import_path(),
            Some(&Self::get_self_pascal_identifier()),
        )
    }
}

impl Reflectable for pax_runtime_api::ColorChannel {
    fn get_import_path() -> String {
        "pax_engine::api::ColorChannel".to_string()
    }

    fn get_self_pascal_identifier() -> String {
        "ColorChannel".to_string()
    }

    fn get_type_id() -> TypeId {
        TypeId::build_singleton(
            &Self::get_import_path(),
            Some(&Self::get_self_pascal_identifier()),
        )
    }
}

impl Reflectable for pax_runtime_api::Rotation {
    fn get_import_path() -> String {
        "pax_engine::api::Rotation".to_string()
    }

    fn get_self_pascal_identifier() -> String {
        "Rotation".to_string()
    }
    fn get_type_id() -> TypeId {
        let type_id = TypeId::build_singleton(
            &Self::get_import_path(),
            Some(&Self::get_self_pascal_identifier()),
        );

        type_id
    }
}

impl Reflectable for pax_runtime_api::Numeric {
    fn get_import_path() -> String {
        "pax_engine::api::Numeric".to_string()
    }

    fn get_self_pascal_identifier() -> String {
        "Numeric".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_singleton(
            &Self::get_import_path(),
            Some(&Self::get_self_pascal_identifier()),
        )
    }
}

impl Reflectable for kurbo::Point {
    fn get_import_path() -> String {
        "kurbo::Point".to_string()
    }

    fn get_self_pascal_identifier() -> String {
        "Point".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_singleton(
            &Self::get_import_path(),
            Some(&Self::get_self_pascal_identifier()),
        )
    }
}

impl Reflectable for pax_runtime_api::Transform2D {
    fn get_import_path() -> String {
        "pax_engine::api::Transform2D".to_string()
    }

    fn get_self_pascal_identifier() -> String {
        "Transform2D".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_singleton(
            &Self::get_import_path(),
            Some(&Self::get_self_pascal_identifier()),
        )
    }
}

impl<T: Reflectable> Reflectable for std::vec::Vec<T> {
    fn parse_to_manifest(mut ctx: ParsingContext) -> (ParsingContext, Vec<PropertyDefinition>) {
        let type_id = Self::get_type_id();
        let td = TypeDefinition {
            type_id: type_id.clone(),
            inner_iterable_type_id: Self::get_iterable_type_id(),
            property_definitions: vec![],
        };

        if !ctx.type_table.contains_key(&type_id) {
            ctx.type_table.insert(type_id, td);
        }

        // Also parse iterable type
        T::parse_to_manifest(ctx)
    }
    fn get_import_path() -> String {
        "std::vec::Vec".to_string()
    }
    fn get_self_pascal_identifier() -> String {
        "Vec".to_string()
    }
    fn get_type_id() -> TypeId {
        //Need to encode generics contents as part of unique id for iterables
        TypeId::build_vector(&format!("{}", &Self::get_iterable_type_id().unwrap()))
    }
    fn get_iterable_type_id() -> Option<TypeId> {
        Some(T::get_type_id())
    }
}

impl<T: Reflectable> Reflectable for VecDeque<T> {
    fn parse_to_manifest(mut ctx: ParsingContext) -> (ParsingContext, Vec<PropertyDefinition>) {
        let type_id = Self::get_type_id();
        let td = TypeDefinition {
            type_id: type_id.clone(),
            inner_iterable_type_id: Self::get_iterable_type_id(),
            property_definitions: vec![],
        };

        if !ctx.type_table.contains_key(&type_id) {
            ctx.type_table.insert(type_id, td);
        }

        // Also parse iterable type
        T::parse_to_manifest(ctx)
    }
    fn get_import_path() -> String {
        "std::collections::VecDeque".to_string()
    }
    fn get_self_pascal_identifier() -> String {
        "VecDeque".to_string()
    }
    fn get_type_id() -> TypeId {
        //Need to encode generics contents as part of unique id for iterables
        TypeId::build_vector(&format!("{}", &Self::get_iterable_type_id().unwrap()))
    }
    fn get_iterable_type_id() -> Option<TypeId> {
        Some(T::get_type_id())
    }
}

pub fn clean_and_split_symbols(possibly_nested_symbols: &str) -> Vec<String> {
    let entire_symbol = if possibly_nested_symbols.starts_with("self.") {
        possibly_nested_symbols.replacen("self.", "", 1)
    } else if possibly_nested_symbols.starts_with("this.") {
        possibly_nested_symbols.replacen("this.", "", 1)
    } else {
        possibly_nested_symbols.to_string()
    };

    let trimmed_symbol = entire_symbol.trim();

    trimmed_symbol
        .split(".")
        .map(|atomic_symbol| atomic_symbol.to_string())
        .collect::<Vec<_>>()
}
