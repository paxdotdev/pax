// completions.rs

use dashmap::mapref::one::Ref;
use lazy_static::lazy_static;
use lsp_types::{CompletionItem, CompletionItemKind, InsertTextFormat};
use std::collections::HashMap;
use std::sync::RwLock;

use crate::{PaxComponent, SelectorData};

lazy_static! {
    static ref STRUCT_COMPLETIONS: RwLock<HashMap<String, CompletionItem>> = {
        let mut map = HashMap::new();

        let components = ["Scroller", "Stacker", "Frame", "Group"];

        for component in &components {
            map.insert(
                component.to_string(),
                CompletionItem {
                    label: component.to_string(),
                    kind: Some(CompletionItemKind::CLASS),
                    insert_text: Some(format!("{}>\n\t$0\n</{}>", component, component)),
                    insert_text_format: Some(lsp_types::InsertTextFormat::SNIPPET),
                    ..Default::default()
                },
            );
        }

        RwLock::new(map)
    };
    static ref TYPE_COMPLETIONS: RwLock<HashMap<String, Vec<CompletionItem>>> = {
        let mut type_map = HashMap::new();
        type_map.insert(
            "Size".to_string(),
            vec![
                CompletionItem {
                    label: "px".to_string(),
                    kind: Some(CompletionItemKind::CONSTANT),
                    insert_text: Some("$0px".to_string()),
                    insert_text_format: Some(lsp_types::InsertTextFormat::SNIPPET),
                    detail: Some("Pixels (e.g. 100px)".to_string()),
                    sort_text: Some("2".to_string()),
                    ..Default::default()
                },
                CompletionItem {
                    label: "%".to_string(),
                    kind: Some(CompletionItemKind::CONSTANT),
                    insert_text: Some("$0%".to_string()),
                    insert_text_format: Some(lsp_types::InsertTextFormat::SNIPPET),
                    detail: Some("% of parent (e.g. 100%)".to_string()),
                    sort_text: Some("1".to_string()),
                    ..Default::default()
                },
                CompletionItem {
                    label: "Expression".to_string(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    insert_text: Some("{$0% + 0px}".to_string()),
                    insert_text_format: Some(lsp_types::InsertTextFormat::SNIPPET),
                    detail: Some("e.g. {50% + 50px}".to_string()),
                    sort_text: Some("3".to_string()),
                    ..Default::default()
                },
            ],
        );
        type_map.insert(
            "Numeric".to_string(),
            vec![
                CompletionItem {
                    label: "Integer".to_string(),
                    kind: Some(CompletionItemKind::CONSTANT),
                    insert_text: Some("0".to_string()),
                    insert_text_format: Some(lsp_types::InsertTextFormat::PLAIN_TEXT),
                    detail: Some("".to_string()),
                    sort_text: Some("1".to_string()),
                    ..Default::default()
                },
                CompletionItem {
                    label: "Float".to_string(),
                    kind: Some(CompletionItemKind::CONSTANT),
                    insert_text: Some("0.0".to_string()),
                    insert_text_format: Some(lsp_types::InsertTextFormat::PLAIN_TEXT),
                    detail: Some("".to_string()),
                    sort_text: Some("2".to_string()),
                    ..Default::default()
                },
            ],
        );
        type_map.insert(
            "Rotation".to_string(),
            vec![
                CompletionItem {
                    label: "degrees".to_string(),
                    kind: Some(CompletionItemKind::CONSTANT),
                    insert_text: Some("$0deg".to_string()),
                    insert_text_format: Some(lsp_types::InsertTextFormat::SNIPPET),
                    detail: Some("".to_string()),
                    sort_text: Some("1".to_string()),
                    ..Default::default()
                },
                CompletionItem {
                    label: "radians".to_string(),
                    kind: Some(CompletionItemKind::CONSTANT),
                    insert_text: Some("$0rad".to_string()),
                    insert_text_format: Some(lsp_types::InsertTextFormat::SNIPPET),
                    detail: Some("".to_string()),
                    sort_text: Some("2".to_string()),
                    ..Default::default()
                },
            ],
        );
        type_map.insert(
            "Transform2D".to_string(),
            vec![CompletionItem {
                label: "Transform Expression".to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                insert_text: Some(
                    "{Transform2D::Scale(50%, 50%) * Transform2D::Rotation(50deg)}$0".to_string(),
                ),
                insert_text_format: Some(lsp_types::InsertTextFormat::SNIPPET),
                detail: Some("see Transform2D api".to_string()),
                sort_text: Some("1".to_string()),
                ..Default::default()
            }],
        );
        type_map.insert(
            "Fill".to_string(),
            vec![
                CompletionItem {
                    label: "Solid Black".to_string(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    insert_text: Some("{Fill::Solid(Color::rgba(0.0,0.0,0.0,1.0))}$0".to_string()),
                    insert_text_format: Some(lsp_types::InsertTextFormat::SNIPPET),
                    detail: Some("See Color Api".to_string()),
                    sort_text: Some("1".to_string()),
                    ..Default::default()
                },
                CompletionItem {
                    label: "Linear Gradient".to_string(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    insert_text: Some("{Fill::linearGradient(
                        (0%, 50%),
                        (100%, 50%),
                        [GradientStop::get(Color::rgba(0.0,0.0,0.0,1.0), 0%), GradientStop::get(Color::rgba(0.0,0.0,0.0,0.5), 100%)])}$0".to_string()),
                    insert_text_format: Some(lsp_types::InsertTextFormat::SNIPPET),
                    detail: Some("See Fill api".to_string()),
                    sort_text: Some("2".to_string()),
                    ..Default::default()
                },
            ]
        );
        type_map.insert(
            "crate::types::Fill".to_string(),
            vec![
                CompletionItem {
                    label: "Solid Black".to_string(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    insert_text: Some("{Fill::Solid(Color::rgba(0.0,0.0,0.0,1.0))}$0".to_string()),
                    insert_text_format: Some(lsp_types::InsertTextFormat::SNIPPET),
                    detail: Some("See Color Api".to_string()),
                    sort_text: Some("1".to_string()),
                    ..Default::default()
                },
                CompletionItem {
                    label: "Linear Gradient".to_string(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    insert_text: Some("{Fill::linearGradient(
                        (0%, 50%),
                        (100%, 50%),
                        [GradientStop::get(Color::rgba(0.0,0.0,0.0,1.0), 0%), GradientStop::get(Color::rgba(0.0,0.0,0.0,0.5), 100%)])}$0".to_string()),
                    insert_text_format: Some(lsp_types::InsertTextFormat::SNIPPET),
                    detail: Some("See Fill api".to_string()),
                    sort_text: Some("2".to_string()),
                    ..Default::default()
                },
            ]
        );
        type_map.insert(
            "Color".to_string(),
            vec![CompletionItem {
                label: "Solid Black".to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                insert_text: Some("{Color::rgba(0.0,0.0,0.0,1.0)}$0".to_string()),
                insert_text_format: Some(lsp_types::InsertTextFormat::SNIPPET),
                detail: Some("See Color Api".to_string()),
                sort_text: Some("1".to_string()),
                ..Default::default()
            }],
        );
        type_map.insert(
            "crate::types::Color".to_string(),
            vec![CompletionItem {
                label: "Solid Black".to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                insert_text: Some("{Color::rgba(0.0,0.0,0.0,1.0)}$0".to_string()),
                insert_text_format: Some(lsp_types::InsertTextFormat::SNIPPET),
                detail: Some("See Color Api".to_string()),
                sort_text: Some("1".to_string()),
                ..Default::default()
            }],
        );
        type_map.insert(
            "TextStyle".to_string(),
            vec![CompletionItem {
                label: "Text Styling".to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                insert_text: Some(
                    "{
                        font: Font::system(\"Helvetica\", FontStyle::Normal, FontWeight::Bold),
                        font_size: 24px,
                        fill: Color::rgba(0.0, 0.0, 0.0, 0.0),
                        align_vertical: TextAlignVertical::Center,
                        align_horizontal: TextAlignHorizontal::Center,
                        underline: true
                    }$0"
                    .to_string(),
                ),
                insert_text_format: Some(lsp_types::InsertTextFormat::SNIPPET),
                detail: Some("See Text Api".to_string()),
                sort_text: Some("1".to_string()),
                ..Default::default()
            }],
        );
        type_map.insert(
            "crate::types::Stroke".to_string(),
            vec![CompletionItem {
                label: "Black Stroke".to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                insert_text: Some(
                    "{color:{Color::rgba(0.0,0.0,0.0,1.0)}, width: 10px}$0".to_string(),
                ),
                insert_text_format: Some(lsp_types::InsertTextFormat::SNIPPET),
                detail: Some("See Stroke Api".to_string()),
                sort_text: Some("1".to_string()),
                ..Default::default()
            }],
        );
        type_map.insert(
            "Stroke".to_string(),
            vec![CompletionItem {
                label: "Black Stroke".to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                insert_text: Some(
                    "{color:{Color::rgba(0.0,0.0,0.0,1.0)}, width: 10px}$0".to_string(),
                ),
                insert_text_format: Some(lsp_types::InsertTextFormat::SNIPPET),
                detail: Some("See Stroke Api".to_string()),
                sort_text: Some("1".to_string()),
                ..Default::default()
            }],
        );
        type_map.insert(
            "crate::types::RectangleCornerRadii".to_string(),
            vec![CompletionItem {
                label: "5px Rounded Corners".to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                insert_text: Some("{RectangleCornerRadii::radii(5.0,5.0,5.0,5.0)}$0".to_string()),
                insert_text_format: Some(lsp_types::InsertTextFormat::SNIPPET),
                detail: Some("See pax-std Api".to_string()),
                sort_text: Some("1".to_string()),
                ..Default::default()
            }],
        );
        type_map.insert(
            "RectangleCornerRadii".to_string(),
            vec![CompletionItem {
                label: "5px Rounded Corners".to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                insert_text: Some("{RectangleCornerRadii::radii(5.0,5.0,5.0,5.0)}$0".to_string()),
                insert_text_format: Some(lsp_types::InsertTextFormat::SNIPPET),
                detail: Some("See pax-std Api".to_string()),
                sort_text: Some("1".to_string()),
                ..Default::default()
            }],
        );
        RwLock::new(type_map)
    };
    static ref EVENT_COMPLETIONS: RwLock<Vec<CompletionItem>> = {
        let mut completions = Vec::new();

        let events = [
            ("scroll", "Set Scroll event handler"),
            ("checkbox_change", "Set Changed event handler"),
            ("clap", "Set Clap event handler"),
            ("touch_start", "Set Touch Start event handler"),
            ("touch_move", "Set Touch Move event handler"),
            ("touch_end", "Set Touch End event handler"),
            ("key_down", "Set Key Down event handler"),
            ("key_up", "Set Key Up event handler"),
            ("key_press", "Set Key Press event handler"),
            ("click", "Set Click event handler"),
            ("mousedown", "Set Mouse Down event handler"),
            ("mouseup", "Set Mouse Up event handler"),
            ("mousemove", "Set Mouse Move event handler"),
            ("mouse_over", "Set Mouse Over event handler"),
            ("mouse_out", "Set Mouse Out event handler"),
            ("double_click", "Set Double Click event handler"),
            ("context_menu", "Set Context Menu event handler"),
            ("wheel", "Set Wheel event handler"),
            ("tick", "Set Tick event handler"),
            ("pre_render", "Set Will Render event handler"),
            ("mount", "Set Did Mount event handler"),
        ];

        for (event, description) in &events {
            completions.push(CompletionItem {
                label: event.to_string(),
                detail: Some(description.to_string()),
                kind: Some(CompletionItemKind::FIELD),
                insert_text: Some(format!("{}", event)),
                ..Default::default()
            });
        }
        RwLock::new(completions)
    };
}

pub fn get_struct_completion(identifier: &str) -> Option<CompletionItem> {
    let map = STRUCT_COMPLETIONS.read().unwrap();
    map.get(identifier).cloned()
}

pub fn get_type_completion(type_identifier: &str) -> Option<Vec<CompletionItem>> {
    let map = TYPE_COMPLETIONS.read().unwrap();
    map.get(&type_identifier.to_string()).cloned()
}

pub fn get_event_completions(delim: &str) -> Vec<CompletionItem> {
    let base_event_completions = EVENT_COMPLETIONS.read().unwrap().clone();
    base_event_completions
        .iter()
        .map(|c| {
            let mut c = c.clone();
            c.insert_text = Some(format!("{}{}", c.insert_text.unwrap(), delim));
            c
        })
        .collect()
}

pub fn get_block_declaration_completions() -> Vec<CompletionItem> {
    let mut completions = Vec::new();

    let mut completion = CompletionItem::new_simple(
        String::from("settings"),
        String::from("Define classes and id selectors"),
    );
    completion.kind = Some(CompletionItemKind::CLASS);
    completion.insert_text = Some("settings {\n\t $0 \n}".to_string());
    completion.insert_text_format = Some(InsertTextFormat::SNIPPET);
    completions.push(completion);

    let mut completion = CompletionItem::new_simple(
        String::from("handlers"),
        String::from("Define root component event handlers"),
    );
    completion.kind = Some(CompletionItemKind::CLASS);
    completion.insert_text_format = Some(InsertTextFormat::SNIPPET);
    completion.insert_text = Some("handlers {\n\t $0 \n}".to_string());
    completions.push(completion);

    completions
}

pub fn get_root_component_methods(component: &PaxComponent) -> Vec<CompletionItem> {
    let mut completions = Vec::new();
    if let Some(c) = component.identifier_map.get(&component.component_name) {
        for entry in &c.methods {
            let mut completion =
                CompletionItem::new_simple(entry.identifier.clone(), entry.identifier.clone());
            completion.kind = Some(CompletionItemKind::METHOD);
            completion.insert_text = Some(format!("{}", entry.identifier.clone()));
            completions.push(completion);
        }
    }
    return completions;
}

pub fn get_common_property_type_completion(
    component: &PaxComponent,
    requested_property: String,
) -> Vec<CompletionItem> {
    let mut completions = Vec::new();
    if let Some(struct_ident) = component.identifier_map.get("CommonProperties") {
        if let Some(property) = struct_ident
            .properties
            .iter()
            .find(|p| p.identifier == requested_property)
        {
            if let Some(type_completions) = get_type_completion(&property.rust_type) {
                completions.extend(type_completions.clone());
            }
        }
    }
    return completions;
}

pub fn get_struct_property_type_completion(
    component: &PaxComponent,
    tag_struct: String,
    requested_property: String,
) -> Vec<CompletionItem> {
    let mut completions = Vec::new();
    if let Some(struct_ident) = component.identifier_map.get(&tag_struct) {
        if let Some(property) = struct_ident
            .properties
            .iter()
            .find(|p| p.identifier == requested_property)
        {
            if let Some(type_completions) = get_type_completion(&property.rust_type) {
                completions.extend(type_completions.clone());
            }
        }
    }
    return completions;
}

pub fn get_class_completions(
    selector_info: &Option<Ref<String, SelectorData>>,
    full_formatting: bool,
    prefix: bool,
) -> Vec<CompletionItem> {
    let mut completions = Vec::new();
    let prefix_string = if prefix { "." } else { "" };

    if let Some(info) = selector_info {
        for entry in &info.classes {
            let mut completion = CompletionItem::new_simple(entry.clone(), "Class".to_string());
            completion.kind = Some(CompletionItemKind::CLASS);
            completion.sort_text = Some("0".to_string());
            if full_formatting {
                completion.insert_text =
                    Some(format!("{}{} {{\n\t$0\n}}", prefix_string, entry.clone()));
                completion.insert_text_format = Some(InsertTextFormat::SNIPPET);
            } else {
                completion.insert_text = Some(format!("{}", entry.clone()));
            }
            completions.push(completion);
        }
    }
    return completions;
}

pub fn get_id_completions(
    selector_info: &Option<Ref<String, SelectorData>>,
    full_formatting: bool,
    prefix: bool,
) -> Vec<CompletionItem> {
    let mut completions = Vec::new();
    let prefix_string = if prefix { "#" } else { "" };
    if let Some(info) = selector_info {
        for entry in &info.ids {
            let mut completion = CompletionItem::new_simple(entry.clone(), "Id".to_string());
            completion.kind = Some(CompletionItemKind::CLASS);
            completion.sort_text = Some("1".to_string());
            if full_formatting {
                completion.insert_text =
                    Some(format!("{}{} {{\n\t$0\n}}", prefix_string, entry.clone()));
                completion.insert_text_format = Some(InsertTextFormat::SNIPPET);
            } else {
                completion.insert_text = Some(format!("{}", entry.clone()));
            }
            completions.push(completion);
        }
    }
    return completions;
}

pub fn get_struct_static_member_completions(
    component: &PaxComponent,
    requested_struct: String,
) -> Vec<CompletionItem> {
    let mut completions = Vec::new();
    if let Some(struct_ident) = component.identifier_map.get(&requested_struct) {
        for entry in &struct_ident.variants {
            let mut completion =
                CompletionItem::new_simple(entry.identifier.clone(), "Variant".to_string());
            completion.sort_text = Some("0".to_string());
            completion.kind = Some(CompletionItemKind::ENUM_MEMBER);
            if entry.has_fields {
                completion.insert_text = Some(format!("{}($0)", entry.identifier.clone()));
                completion.insert_text_format = Some(InsertTextFormat::SNIPPET);
            } else {
                completion.insert_text = Some(format!("{}", entry.identifier.clone()));
            }
            completions.push(completion);
        }
        for entry in &struct_ident.methods {
            let mut completion =
                CompletionItem::new_simple(entry.identifier.clone(), "Function".to_string());
            completion.kind = Some(CompletionItemKind::METHOD);
            completion.sort_text = Some("1".to_string());
            completion.insert_text = Some(format!("{}($0)", entry.identifier.clone()));
            completion.insert_text_format = Some(InsertTextFormat::SNIPPET);
            completions.push(completion);
        }
    }
    return completions;
}

pub fn get_all_root_component_member_completions(component: &PaxComponent) -> Vec<CompletionItem> {
    let mut completions = Vec::new();
    if let Some(c) = component.identifier_map.get(&component.component_name) {
        for entry in &c.properties {
            let mut completion =
                CompletionItem::new_simple(entry.identifier.clone(), entry.identifier.clone());
            completion.kind = Some(CompletionItemKind::FIELD);
            completion.insert_text = Some(format!("{}", entry.identifier.clone()));
            completions.push(completion);
        }
        for entry in &c.methods {
            let mut completion =
                CompletionItem::new_simple(entry.identifier.clone(), entry.identifier.clone());
            completion.kind = Some(CompletionItemKind::METHOD);
            completion.insert_text = Some(format!("{}", entry.identifier.clone()));
            completions.push(completion);
        }
    }
    return completions;
}

pub fn get_struct_property_setting_completions(
    component: &PaxComponent,
    requested_struct: String,
) -> Vec<CompletionItem> {
    let mut completions = Vec::new();
    if let Some(struct_ident) = component.identifier_map.get(&requested_struct) {
        for entry in struct_ident.properties.iter() {
            let mut completion =
                CompletionItem::new_simple(entry.identifier.clone(), entry.identifier.clone());
            completion.kind = Some(CompletionItemKind::FIELD);
            completion.insert_text = Some(format!("{}=", entry.identifier.clone()));
            completions.push(completion);
        }
    }
    return completions;
}

pub fn get_common_properties_setting_completions(
    component: &PaxComponent,
    delim: &str,
) -> Vec<CompletionItem> {
    let mut completions = Vec::new();
    if let Some(info) = component.identifier_map.get("CommonProperties") {
        for entry in info.properties.iter() {
            let mut completion =
                CompletionItem::new_simple(entry.identifier.clone(), entry.identifier.clone());
            completion.kind = Some(CompletionItemKind::FIELD);
            completion.insert_text = Some(format!("{}{}", entry.identifier.clone(), delim));
            completions.push(completion);
        }
    }
    return completions;
}
