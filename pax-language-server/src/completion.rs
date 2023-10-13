// completions.rs

use lazy_static::lazy_static;
use lsp_types::{CompletionItem, CompletionItemKind};
use std::collections::HashMap;
use std::sync::RwLock;

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
                    insert_text: Some("{Color::rgba(0.0,0.0,0.0,1.0)}$0".to_string()),
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
                    insert_text: Some("{Color::rgba(0.0,0.0,0.0,1.0)}$0".to_string()),
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
        RwLock::new(type_map)
    };
    static ref EVENT_COMPLETIONS: RwLock<Vec<CompletionItem>> = {
        let mut completions = Vec::new();

        let events = [
            ("scroll", "Set Scroll event handler"),
            ("jab", "Set Jab event handler"),
            ("touch_start", "Set Touch Start event handler"),
            ("touch_move", "Set Touch Move event handler"),
            ("touch_end", "Set Touch End event handler"),
            ("key_down", "Set Key Down event handler"),
            ("key_up", "Set Key Up event handler"),
            ("key_press", "Set Key Press event handler"),
            ("click", "Set Click event handler"),
            ("mouse_down", "Set Mouse Down event handler"),
            ("mouse_up", "Set Mouse Up event handler"),
            ("mouse_move", "Set Mouse Move event handler"),
            ("mouse_over", "Set Mouse Over event handler"),
            ("mouse_out", "Set Mouse Out event handler"),
            ("double_click", "Set Double Click event handler"),
            ("context_menu", "Set Context Menu event handler"),
            ("wheel", "Set Wheel event handler"),
            ("will_render", "Set Will Render event handler"),
            ("did_mount", "Set Did Mount event handler"),
        ];

        for (event, description) in &events {
            completions.push(CompletionItem {
                label: event.to_string(),
                detail: Some(description.to_string()),
                kind: Some(CompletionItemKind::FIELD),
                insert_text: Some(format!("{}=", event)),
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

pub fn get_event_completions() -> Vec<CompletionItem> {
    EVENT_COMPLETIONS.read().unwrap().clone()
}
