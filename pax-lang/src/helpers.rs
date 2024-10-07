use std::fs;
use std::path::Path;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{parse_file, ItemStruct};

#[derive(Debug)]
pub struct InlinedTemplate {
    pub struct_name: String,
    pub start: (usize, usize),
    pub end: (usize, usize),
    pub template: String,
}

#[derive(Debug)]
pub struct InlinedTemplateFinder {
    pub file_contents: String,
    pub templates: Vec<InlinedTemplate>,
}

impl InlinedTemplateFinder {
    pub fn new(file_contents: String) -> Self {
        InlinedTemplateFinder {
            file_contents,
            templates: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for InlinedTemplateFinder {
    fn visit_item_struct(&mut self, i: &'ast ItemStruct) {
        let mut has_pax = false;
        let struct_name = i.ident.to_string();
        for attr in &i.attrs {
            // Check for #[pax]
            if attr.path.is_ident("pax") {
                has_pax = true;
            }

            if attr.path.is_ident("inlined") {
                let start = attr.tokens.span().start();
                let start_tuple = (start.line, start.column + 1);
                let end = attr.tokens.span().end();
                let end_tuple = (end.line, end.column + 1);
                let content =
                    get_substring_by_line_column(&self.file_contents, start_tuple, end_tuple)
                        .unwrap()
                        .trim_start_matches("(")
                        .trim_end_matches(")")
                        .to_string();
                if has_pax {
                    let found_template = InlinedTemplate {
                        struct_name: struct_name.clone(),
                        start: start_tuple,
                        end: end_tuple,
                        template: content,
                    };
                    self.templates.insert(0, found_template);
                }
            }
        }
    }
}

fn find_start_end_bytes(
    input: &str,
    start: (usize, usize),
    end: (usize, usize),
) -> (Option<usize>, Option<usize>) {
    let mut current_line = 1;
    let mut current_column = 1;
    let mut start_byte = None;
    let mut end_byte = None;

    for (i, c) in input.char_indices() {
        if current_line == start.0 && current_column == start.1 {
            start_byte = Some(i);
        }
        if current_line == end.0 && current_column == end.1 {
            end_byte = Some(i);
            break;
        }

        if c == '\n' {
            current_line += 1;
            current_column = 1;
        } else {
            current_column += 1;
        }
    }

    (start_byte, end_byte)
}

pub fn replace_by_line_column(
    input: &str,
    start: (usize, usize),
    end: (usize, usize),
    replacement: String,
) -> Option<String> {
    let (start_byte, end_byte) = find_start_end_bytes(input, start, end);

    match (start_byte, end_byte) {
        (Some(start_byte), Some(end_byte)) => {
            let mut result = String::new();
            result.push_str(&input[..start_byte]);
            result.push_str(&replacement);
            result.push_str(&input[end_byte..]);
            Some(result)
        }
        _ => None,
    }
}

pub fn get_substring_by_line_column(
    input: &str,
    start: (usize, usize),
    end: (usize, usize),
) -> Option<String> {
    let (start_byte, end_byte) = find_start_end_bytes(input, start, end);

    match (start_byte, end_byte) {
        (Some(start_byte), Some(end_byte)) => Some(input[start_byte..end_byte].to_string()),
        _ => None,
    }
}

pub fn clear_inlined_template(file_path: &str, pascal_identifier: &str) {
    let path = Path::new(file_path);
    let content = fs::read_to_string(path).expect("Failed to read file");
    let ast = parse_file(&content).expect("Failed to parse file");

    let mut finder = InlinedTemplateFinder::new(content.clone());
    finder.visit_file(&ast);

    let mut modified_content = content;
    for template in finder.templates {
        if template.struct_name == pascal_identifier {
            let blank_template = format!("()");
            modified_content = replace_by_line_column(
                &modified_content,
                template.start,
                template.end,
                blank_template,
            )
            .unwrap();
        }
    }
    fs::write(path, modified_content).expect("Failed to write to file");
}
