use dashmap::DashMap;
use lsp_types::Position;
use proc_macro2::Span;
use quote::ToTokens;
use std::fs;
use std::path::{Path, PathBuf};
use syn::visit::Visit;
use syn::{
    parse_file, spanned::Spanned, Attribute, ImplItem, Item, ItemImpl, ItemStruct, Meta,
    NestedMeta, Type,
};
use syn::{Fields, ItemEnum, ItemUse, UseTree};

fn contains_pax_file_macro(attrs: &[Attribute], target_file_path: &str) -> bool {
    let has_pax_derive = attrs
        .iter()
        .any(|attr| attr.path.is_ident("derive") && attr.tokens.to_string().contains("Pax"));

    let has_file_attr = attrs.iter().any(|attr| {
        attr.path.is_ident("file")
            && match attr.parse_meta() {
                Ok(Meta::List(meta_list)) => meta_list.nested.iter().any(|nested_meta| {
                    if let NestedMeta::Lit(syn::Lit::Str(lit_str)) = nested_meta {
                        let lit_str_value = lit_str.value();
                        target_file_path.ends_with(&lit_str_value)
                    } else {
                        false
                    }
                }),
                _ => false,
            }
    });

    has_pax_derive && has_file_attr
}

pub fn find_rust_file_with_macro<P: AsRef<Path>>(
    dir: P,
    file_path: &str,
) -> Option<(PathBuf, String)> {
    for entry in fs::read_dir(&dir).expect("Failed to read dir") {
        let path = entry.expect("Failed to read entry").path();
        if path.is_dir() {
            // Skip the .cargo directory
            if path.ends_with(".cargo") {
                continue;
            }

            if let Some((matching_file, component_name)) =
                find_rust_file_with_macro(&path, file_path)
            {
                return Some((matching_file, component_name));
            }
        } else if path.extension().map_or(false, |ext| ext == "rs") {
            let content = fs::read_to_string(&path).expect("Failed to read file");
            let parsed = parse_file(&content).expect("Failed to parse file");
            for item in &parsed.items {
                if let Item::Struct(item_struct) = item {
                    if contains_pax_file_macro(&item_struct.attrs, file_path) {
                        return Some((path.clone(), item_struct.ident.to_string()));
                    }
                }
            }
        }
    }
    None
}

pub fn extract_import_positions<P: AsRef<Path>>(file_path: P) -> Vec<Position> {
    let src = fs::read_to_string(&file_path).expect("Failed to read the Rust file");
    let parsed_file = syn::parse_file(&src).expect("Failed to parse the source code");

    let mut visitor = ImportVisitor {
        positions: Vec::new(),
    };
    visitor.visit_file(&parsed_file);

    visitor.positions
}

struct ImportVisitor {
    positions: Vec<Position>,
}

impl<'ast> Visit<'ast> for ImportVisitor {
    fn visit_item_use(&mut self, i: &'ast ItemUse) {
        self.visit_use_tree(&i.tree);
    }
}

impl ImportVisitor {
    fn visit_use_tree(&mut self, tree: &UseTree) {
        match tree {
            UseTree::Path(p) => {
                if let UseTree::Glob(_) = *p.tree {
                    self.positions.push(span_to_position(p.ident.span()));
                }
                self.visit_use_tree(&*p.tree);
            }
            UseTree::Name(n) => self.positions.push(span_to_position(n.ident.span())),
            UseTree::Group(g) => {
                for tree in &g.items {
                    self.visit_use_tree(tree);
                }
            }
            UseTree::Rename(r) => self.positions.push(span_to_position(r.rename.span())),
            _ => {}
        }
    }
}

fn span_to_position(span: Span) -> Position {
    let start = span.start();
    Position {
        line: (start.line - 1) as u32,
        character: (start.column) as u32,
    }
}

#[derive(Debug, Clone)]
pub struct Info {
    pub path: String,
    pub position: Position,
    pub definition_id: Option<usize>,
    pub hover_id: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IdentifierType {
    Component,
    PaxType,
    Method,
    Property,
    Enum,
    EnumVariant,
}

#[derive(Debug, Clone)]
pub struct StructProperty {
    pub identifier: String,
    pub rust_type: String,
    pub info: Info,
}

#[derive(Debug, Clone)]
pub struct Method {
    pub identifier: String,
    pub info: Info,
}

#[derive(Debug, Clone)]
pub struct VariantData {
    pub identifier: String,
    pub info: Info,
    pub has_fields: bool,
}

#[derive(Debug, Clone)]
pub struct IdentifierInfo {
    pub ty: IdentifierType,
    pub identifier: String,
    pub info: Info,
    pub properties: Vec<StructProperty>,
    pub methods: Vec<Method>,
    pub variants: Vec<VariantData>,
}

#[derive(Debug, Clone)]
pub struct InfoRequest {
    pub identifier_type: IdentifierType,
    pub identifier: String,
    pub owner_identifier: Option<String>,
    pub info: Info,
}

fn extract_rust_type(ty: &Type) -> String {
    let ty_str = ty.to_token_stream().to_string();

    if let Some(inner) = extract_between(&ty_str, "Property <", ">") {
        return inner.replace(" ", "");
    } else if let Some(inner) = extract_between(&ty_str, "PropertyInstance <", ">") {
        return inner.replace(" ", "");
    }

    ty_str
}

fn extract_between(source: &str, start: &str, _end: &str) -> Option<String> {
    let mut start_idx = source.find(start)?;

    start_idx += start.len();

    let mut end_idx = start_idx;
    let mut bracket_count = 1;
    while end_idx < source.len() {
        match &source[end_idx..=end_idx] {
            "<" => bracket_count += 1,
            ">" => bracket_count -= 1,
            _ => {}
        }
        if bracket_count == 0 {
            break;
        }
        end_idx += 1;
    }

    if bracket_count == 0 {
        Some(source[start_idx..end_idx].to_string())
    } else {
        None
    }
}

struct IndexVisitor<'a> {
    index: &'a DashMap<String, IdentifierInfo>,
    file_path: String,
    requests: Vec<InfoRequest>,
}

impl<'ast, 'a> Visit<'ast> for IndexVisitor<'a> {
    fn visit_item_struct(&mut self, i: &'ast ItemStruct) {
        let attributes = &i.attrs;

        let ty = if attributes.iter().any(|attr| {
            attr.path.is_ident("primitive")
                || attr.path.is_ident("inlined")
                || attr.path.is_ident("file")
        }) {
            IdentifierType::Component
        } else {
            IdentifierType::PaxType
        };

        let properties = i
            .fields
            .iter()
            .map(|f| {
                let rust_type_string = extract_rust_type(&(f.clone()).ty);
                if let Some(_) = f.ident.clone() {
                    let prop_info = Info {
                        path: self.file_path.clone(),
                        position: span_to_position(f.ident.clone().unwrap().span()),
                        definition_id: None,
                        hover_id: None,
                    };

                    self.requests.push(InfoRequest {
                        identifier_type: IdentifierType::Property,
                        identifier: f.ident.clone().unwrap().to_string(),
                        owner_identifier: Some(i.ident.to_string()),
                        info: prop_info.clone(),
                    });

                    StructProperty {
                        identifier: f.ident.clone().unwrap().to_string(),
                        rust_type: rust_type_string,
                        info: prop_info,
                    }
                } else {
                    StructProperty {
                        identifier: rust_type_string.clone(),
                        rust_type: rust_type_string.clone(),
                        info: Info {
                            path: self.file_path.clone(),
                            position: span_to_position(f.ty.span()),
                            definition_id: None,
                            hover_id: None,
                        },
                    }
                }
            })
            .collect::<Vec<_>>();

        let struct_name = i.ident.to_string();
        self.index.insert(
            struct_name.clone(),
            IdentifierInfo {
                ty: ty.clone(),
                identifier: struct_name.clone(),
                info: Info {
                    path: self.file_path.clone(),
                    position: span_to_position(i.span()),
                    definition_id: None,
                    hover_id: None,
                },
                properties,
                methods: Vec::new(),
                variants: Vec::new(),
            },
        );

        self.requests.push(InfoRequest {
            identifier_type: ty,
            identifier: struct_name,
            owner_identifier: None,
            info: Info {
                path: self.file_path.clone(),
                position: span_to_position(i.ident.span()),
                definition_id: None,
                hover_id: None,
            },
        });
    }

    fn visit_item_impl(&mut self, i: &'ast ItemImpl) {
        let self_ty = &i.self_ty;
        let struct_name;

        if let syn::Type::Path(tp) = self_ty.as_ref() {
            struct_name = tp.path.segments.last().unwrap().ident.to_string();
        } else {
            return;
        }

        if let Some(mut info) = self.index.get_mut(&struct_name) {
            for item in &i.items {
                if let ImplItem::Method(method) = item {
                    let method_name = method.sig.ident.to_string();
                    let method_info = Method {
                        identifier: method_name.clone(),
                        info: Info {
                            path: self.file_path.clone(),
                            position: span_to_position(method.sig.ident.span()),
                            definition_id: None,
                            hover_id: None,
                        },
                    };
                    info.methods.push(method_info.clone());

                    self.requests.push(InfoRequest {
                        identifier_type: IdentifierType::Method,
                        identifier: method_name,
                        owner_identifier: Some(struct_name.clone()),
                        info: method_info.info,
                    });
                }
            }
        }
    }

    fn visit_item_enum(&mut self, i: &'ast ItemEnum) {
        let enum_name = i.ident.to_string();

        let variants = i
            .variants
            .iter()
            .map(|variant| {
                let has_fields = match &variant.fields {
                    Fields::Unit => false,
                    Fields::Unnamed(_) | Fields::Named(_) => true,
                };

                let variant_info = VariantData {
                    identifier: variant.ident.to_string(),
                    has_fields,
                    info: Info {
                        path: self.file_path.clone(),
                        position: span_to_position(variant.ident.span()),
                        definition_id: None,
                        hover_id: None,
                    },
                };

                self.requests.push(InfoRequest {
                    identifier_type: IdentifierType::EnumVariant,
                    identifier: variant_info.identifier.clone(),
                    owner_identifier: Some(enum_name.clone()),
                    info: variant_info.info.clone(),
                });

                variant_info
            })
            .collect();

        self.index.insert(
            enum_name.clone(),
            IdentifierInfo {
                ty: IdentifierType::Enum,
                identifier: enum_name.clone(),
                info: Info {
                    path: self.file_path.clone(),
                    position: span_to_position(i.ident.span()),
                    definition_id: None,
                    hover_id: None,
                },
                properties: Vec::new(),
                methods: Vec::new(),
                variants,
            },
        );

        self.requests.push(InfoRequest {
            identifier_type: IdentifierType::Enum,
            identifier: enum_name,
            owner_identifier: None,
            info: Info {
                path: self.file_path.clone(),
                position: span_to_position(i.ident.span()),
                definition_id: None,
                hover_id: None,
            },
        });
    }
}

pub fn index_rust_file(
    file_path: &str,
    map: &DashMap<String, IdentifierInfo>,
) -> Result<Vec<InfoRequest>, Box<dyn std::error::Error>> {
    let file_content = std::fs::read_to_string(file_path)?;

    let parsed_file = parse_file(&file_content)?;

    let mut visitor = IndexVisitor {
        index: map,
        file_path: file_path.to_string(),
        requests: Vec::new(),
    };

    visitor.visit_file(&parsed_file);
    Ok(visitor.requests)
}
