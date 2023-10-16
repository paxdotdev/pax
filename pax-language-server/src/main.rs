use completion::{
    get_all_root_component_member_completions, get_block_declaration_completions,
    get_class_completions, get_common_properties_setting_completions,
    get_common_property_type_completion, get_id_completions, get_root_component_methods,
    get_struct_property_setting_completions, get_struct_property_type_completion,
    get_struct_static_member_completions,
};
use completion::{get_event_completions, get_struct_completion, get_type_completion};
use core::panic;
use dashmap::DashMap;
use lsp_types::request::Request;
use pax_compiler::parsing::{self, PaxParser, Rule};
use pest::Parser;
use positional::is_inside_handlers_block;
use positional::is_inside_selector_block;
use positional::is_inside_settings_block;
use positional::{
    extract_positional_nodes, find_nodes_at_position, find_priority_node, find_relevant_ident,
    find_relevant_tag, has_attribute_error, NodeType, PositionalNode,
};
use regex::Captures;
use regex::Regex;
use serde::*;
use std::collections::HashSet;
use std::path::PathBuf;

use tower_lsp::jsonrpc::Error;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

mod index;
use index::{
    extract_import_positions, find_rust_file_with_macro, index_rust_file, IdentifierInfo,
    IdentifierType, Info, InfoRequest,
};

mod positional;

mod completion;

use std::sync::{Arc, Mutex};

extern crate pest;

use tokio::time::Duration;

use ropey::Rope;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SymbolLocationParams {
    symbol: SymbolData,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct SymbolData {
    uri: String,
    position: Position,
}

pub enum GetDefinitionRequest {}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GetDefinitionResult {
    locations: Vec<LocationLink>,
}

impl Request for GetDefinitionRequest {
    type Params = SymbolLocationParams;
    type Result = GetDefinitionResult;
    const METHOD: &'static str = "pax/getDefinition";
}

pub enum GetHoverRequest {}
pub type GetHoverResult = String;

impl Request for GetHoverRequest {
    type Params = SymbolLocationParams;
    type Result = GetHoverResult;
    const METHOD: &'static str = "pax/getHover";
}

pub enum EnrichRequest {}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EnrichParams {
    symbol: SymbolData,
    originatingPaxFile: String,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EnrichResult {
    getDefinition: usize,
    getHover: usize,
}

impl Request for EnrichRequest {
    type Params = EnrichParams;
    type Result = EnrichResult;
    const METHOD: &'static str = "pax/enrich";
}

#[derive(Debug)]
pub struct PaxComponent {
    component_name: String,
    identifier_map: DashMap<String, IdentifierInfo>,
}

#[derive(Debug)]
pub struct SelectorData {
    ids: HashSet<String>,
    classes: HashSet<String>,
}

#[derive(Debug, Clone)]
struct Backend {
    client: Arc<Client>,
    pax_map: Arc<DashMap<String, PaxComponent>>,
    rs_to_pax_map: Arc<DashMap<String, String>>,
    workspace_root: Arc<Mutex<Option<Url>>>,
    pax_ast_cache: Arc<DashMap<String, Vec<PositionalNode>>>,
    pax_selector_map: Arc<DashMap<String, SelectorData>>,
    pending_changes: Arc<DashMap<String, DidChangeTextDocumentParams>>,
    debounce_last_save: Arc<Mutex<std::time::Instant>>,
    document_content: Arc<DashMap<String, Rope>>,
}

impl Backend {
    pub async fn set_root(&self, url: Option<Url>) {
        let mut root_guard = self.workspace_root.lock().unwrap();
        *root_guard = url;
    }

    pub async fn handle_file(&self, pax_file: String, file_to_index: String) {
        if let Some(component) = self.pax_map.get(&pax_file) {
            let requests = match index_rust_file(&file_to_index, &component.identifier_map) {
                Ok(reqs) => reqs,
                Err(err) => {
                    eprintln!("Error indexing file {}: {:?}", file_to_index, err);
                    return;
                }
            };

            for request in requests {
                let backend_clone = self.clone();
                let pax_file_clone = pax_file.clone();

                tokio::spawn(async move {
                    backend_clone
                        .handle_info_request(pax_file_clone, request)
                        .await;
                });
            }
        }
    }

    pub async fn handle_info_request(&self, pax_file: String, info_request: InfoRequest) {
        if let Some(component) = self.pax_map.get(&pax_file) {
            let symbol_data = SymbolData {
                uri: info_request.info.path.clone(),
                position: info_request.info.position,
            };

            let params = EnrichParams {
                symbol: symbol_data,
                originatingPaxFile: pax_file.clone(),
            };

            let mut new_info = info_request.info.clone();

            match self.client.send_request::<EnrichRequest>(params).await {
                Ok(response) => {
                    new_info.definition_id = Some(response.getDefinition);
                    new_info.hover_id = Some(response.getHover);
                }
                Err(e) => {
                    eprintln!("Error sending EnrichRequest: {:?}", e);
                }
            }

            match &info_request.identifier_type {
                IdentifierType::Component | IdentifierType::PaxType | IdentifierType::Enum => {
                    if let Some(mut identifier_info) =
                        component.identifier_map.remove(&info_request.identifier)
                    {
                        identifier_info.1.info = new_info;
                        component
                            .identifier_map
                            .insert((&info_request).identifier.clone(), identifier_info.1);
                    }
                }
                IdentifierType::Property => {
                    if let Some(mut struct_info) = component
                        .identifier_map
                        .remove(&(&info_request).owner_identifier.clone().unwrap())
                    {
                        if let Some(property) = struct_info
                            .1
                            .properties
                            .iter_mut()
                            .find(|prop| prop.identifier == info_request.identifier)
                        {
                            property.info = new_info;
                        }
                        component.identifier_map.insert(
                            (&info_request).owner_identifier.clone().unwrap().clone(),
                            struct_info.1,
                        );
                    }
                }
                IdentifierType::Method => {
                    if let Some(mut struct_info) = component
                        .identifier_map
                        .remove(&(&info_request).owner_identifier.clone().unwrap())
                    {
                        if let Some(method) = struct_info
                            .1
                            .methods
                            .iter_mut()
                            .find(|m| m.identifier == info_request.identifier)
                        {
                            method.info = new_info;
                        }
                        component.identifier_map.insert(
                            (&info_request).owner_identifier.clone().unwrap().clone(),
                            struct_info.1,
                        );
                    }
                }
                IdentifierType::EnumVariant => {
                    if let Some(mut enum_info) = component
                        .identifier_map
                        .remove(&(&info_request).owner_identifier.clone().unwrap())
                    {
                        if let Some(variant) = enum_info
                            .1
                            .variants
                            .iter_mut()
                            .find(|prop| prop.identifier == info_request.identifier)
                        {
                            variant.info = new_info;
                        }
                        component.identifier_map.insert(
                            (&info_request).owner_identifier.clone().unwrap().clone(),
                            enum_info.1,
                        );
                    }
                }
            }
        }
    }

    pub async fn index_file(
        &self,
        pax_file: &str,
        rust_file_path: PathBuf,
        component_name: String,
    ) {
        let identifier_map: DashMap<String, IdentifierInfo> = DashMap::new();

        let rust_file_path_str = rust_file_path.to_string_lossy().to_string();
        let info_requests = match index_rust_file(&rust_file_path_str, &identifier_map) {
            Ok(reqs) => reqs,
            Err(err) => {
                eprintln!("Error indexing file {}: {:?}", rust_file_path_str, err);
                return;
            }
        };

        for info_request in info_requests.clone() {
            let backend_clone = self.clone();
            let pax_file_clone = pax_file.to_string();
            tokio::spawn(async move {
                backend_clone
                    .handle_info_request(pax_file_clone, info_request)
                    .await;
            });
        }

        self.pax_map.insert(
            pax_file.to_string(),
            PaxComponent {
                component_name,
                identifier_map,
            },
        );

        self.rs_to_pax_map
            .insert(rust_file_path_str, pax_file.to_string());

        let positions = extract_import_positions(&rust_file_path);
        for position in positions {
            let symbol_data = SymbolData {
                uri: rust_file_path.to_string_lossy().to_string(),
                position,
            };

            let params = SymbolLocationParams {
                symbol: symbol_data,
            };

            match self
                .client
                .send_request::<GetDefinitionRequest>(params)
                .await
            {
                Ok(response) => {
                    for location_link in response.locations {
                        let target_uri = location_link.target_uri;
                        let _path = target_uri.clone().path().to_string();
                        let target_file = target_uri
                            .to_file_path()
                            .expect("Failed to convert URI to path")
                            .to_string_lossy()
                            .to_string();

                        let path = pax_file.to_string();
                        let backend_clone = self.clone();
                        tokio::spawn(async move {
                            backend_clone.handle_file(path, target_file).await;
                        });
                    }
                }
                Err(e) => {
                    self.client
                        .log_message(MessageType::INFO, format!("Error couldnt look up imports"))
                        .await;
                    eprintln!("Error sending request: {:?}", e);
                }
            }
        }
    }

    fn parse_and_cache_pax_file(&self, pax: &str, uri: Url) -> Vec<Diagnostic> {
        let parse_result = PaxParser::parse(Rule::pax_component_definition, pax);

        let path_str = uri.path();

        match parse_result {
            Ok(pax_component_definition) => {
                let mut nodes = Vec::new();
                let mut ids = HashSet::new();
                let mut classes = HashSet::new();

                extract_positional_nodes(
                    pax_component_definition.clone().next().unwrap(),
                    &mut nodes,
                    &mut ids,
                    &mut classes,
                );

                self.pax_ast_cache
                    .insert(path_str.clone().to_string(), nodes.clone());

                self.pax_selector_map
                    .insert(path_str.clone().to_string(), SelectorData { ids, classes });

                let errors = parsing::extract_errors(
                    pax_component_definition
                        .clone()
                        .next()
                        .unwrap()
                        .into_inner(),
                );

                // If there are errors, publish them as diagnostics
                if !errors.is_empty() {
                    let diagnostics: Vec<Diagnostic> = errors
                        .into_iter()
                        .map(|err| Diagnostic {
                            range: Range {
                                start: Position {
                                    line: (err.start.0 - 1) as u32,
                                    character: (err.start.1 - 1) as u32,
                                },
                                end: Position {
                                    line: (err.end.0 - 1) as u32,
                                    character: (err.end.1 - 1) as u32,
                                },
                            },
                            message: err.error_message,
                            severity: Some(DiagnosticSeverity::ERROR),
                            code: None,
                            source: None,
                            related_information: None,
                            code_description: None,
                            tags: None,
                            data: None,
                        })
                        .collect();
                    diagnostics
                } else {
                    Vec::new()
                }
            }
            Err(e) => {
                eprintln!("Failed to parse {}: {:?}", path_str, e);
                Vec::new()
            }
        }
    }
    async fn hover_id(&self, params: HoverParams) -> Result<Option<u32>> {
        let uri_obj = &params.text_document_position_params.text_document.uri;
        let uri_path = uri_obj.path();
        let pos = &params.text_document_position_params.position;

        if uri_path.ends_with(".pax") && !self.pax_map.contains_key(uri_path) {
            self.process_pax_file(uri_obj).await;

            let file_path = uri_obj
                .to_file_path()
                .map_err(|_| Error::invalid_params(format!("Invalid URI: {}", uri_obj)))?;
            let file_content = std::fs::read_to_string(&file_path).map_err(|err| {
                Error::invalid_params(format!("Failed to read {}: {}", file_path.display(), err))
            })?;

            let _ = self.parse_and_cache_pax_file(&file_content, uri_obj.clone());
        }

        if let Some(info) = self.get_info(uri_path, pos) {
            if let Some(id) = info.hover_id {
                return Ok(Some(id as u32));
            }
        }

        Ok(None)
    }

    fn get_info(&self, uri: &str, pos: &Position) -> Option<Info> {
        if let Some(component) = self.pax_map.get(uri) {
            if let Some(cached_nodes) = self.pax_ast_cache.get(uri) {
                let relevant_nodes = find_nodes_at_position(pos.clone(), &cached_nodes);
                let priority_node = find_priority_node(&relevant_nodes);
                let tag_node = find_relevant_tag(&relevant_nodes);
                let relevant_ident = find_relevant_ident(&relevant_nodes);
                if let Some(node) = priority_node {
                    let struct_name = if let Some(tag) = tag_node {
                        if let NodeType::Tag(tag_data) = &tag.node_type {
                            Some(tag_data.pascal_identifier.clone())
                        } else {
                            panic!("Expected NodeType::Tag, found {:?}", tag.node_type);
                        }
                    } else {
                        None
                    };

                    if let Some(ident) = relevant_ident {
                        if let NodeType::Identifier(ident_data) = &ident.node_type {
                            let ident_name = ident_data.identifier.clone();
                            match &node.node_type {
                                NodeType::Identifier(data) => {
                                    let ident = data.identifier.clone();
                                    if let Some(ident_info) =
                                        component.identifier_map.get(ident.as_str())
                                    {
                                        return Some(ident_info.info.clone());
                                    }
                                }
                                NodeType::LiteralFunction(data) => {
                                    if let Some(ident_info) = component
                                        .identifier_map
                                        .get(component.component_name.clone().as_str())
                                    {
                                        if let Some(method) = ident_info
                                            .methods
                                            .iter()
                                            .find(|m| m.identifier == data.function_name)
                                        {
                                            return Some(method.info.clone());
                                        }
                                    }
                                }
                                NodeType::LiteralEnumValue(data) => {
                                    let mut struct_id = data.enum_name.clone();
                                    if &struct_id == "Self" {
                                        struct_id = component.component_name.clone();
                                    }
                                    if let Some(ident_info) =
                                        component.identifier_map.get(struct_id.as_str())
                                    {
                                        if ident_name == data.enum_name {
                                            return Some(ident_info.info.clone());
                                        }
                                        if let Some(variant) = ident_info
                                            .variants
                                            .iter()
                                            .find(|p| p.identifier == data.property_name)
                                        {
                                            return Some(variant.info.clone());
                                        }
                                    }
                                }
                                NodeType::XoFunctionCall(data) => {
                                    let mut struct_id = data.struct_name.clone();
                                    if struct_id == "Self" {
                                        struct_id = component.component_name.clone();
                                    }
                                    if let Some(ident_info) =
                                        component.identifier_map.get(data.struct_name.as_str())
                                    {
                                        if ident_name == data.struct_name {
                                            return Some(ident_info.info.clone());
                                        }
                                        if let Some(method) = ident_info
                                            .methods
                                            .iter()
                                            .find(|m| m.identifier == data.function_name)
                                        {
                                            return Some(method.info.clone());
                                        }
                                    }
                                }
                                NodeType::AttributeKeyValuePair(data) => {
                                    let property_names = vec![
                                        "x",
                                        "y",
                                        "scale_x",
                                        "scale_y",
                                        "skew_x",
                                        "skew_y",
                                        "rotate",
                                        "anchor_x",
                                        "anchor_y",
                                        "transform",
                                        "width",
                                        "height",
                                    ];

                                    if let Some(struct_ident) = struct_name {
                                        let mut struct_id = struct_ident.clone();
                                        if property_names.contains(&data.identifier.as_str()) {
                                            struct_id = "CommonProperties".to_string();
                                        }
                                        if let Some(ident_info) =
                                            component.identifier_map.get(struct_id.as_str())
                                        {
                                            if let Some(property) = ident_info
                                                .properties
                                                .iter()
                                                .find(|p| p.identifier == data.identifier)
                                            {
                                                return Some(property.info.clone());
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            };
                        }
                    }
                }
            }
        }
        return None;
    }

    async fn definition_id(&self, params: GotoDefinitionParams) -> Result<Option<u32>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .path();
        let pos = &params.text_document_position_params.position;

        if let Some(info) = self.get_info(uri, pos) {
            if let Some(id) = info.definition_id {
                return Ok(Some(id as u32));
            }
        }

        Ok(None)
    }

    async fn process_pax_file(&self, uri: &Url) {
        let file_path = uri.path();

        if self.pax_map.contains_key(file_path) {
            return;
        }

        let path = uri.clone().to_file_path().expect("Failed to get file path");
        let directory = path.parent().expect("Failed to get parent directory");

        if let Some((rust_file_path, component_name)) =
            find_rust_file_with_macro(directory, &file_path)
        {
            let backend_clone = self.clone();
            let path_str = file_path.to_string();
            tokio::spawn(async move {
                backend_clone
                    .index_file(&path_str, rust_file_path, component_name)
                    .await;
            });
        } else {
            eprintln!("No matching Rust file found for {}", file_path);
        }
    }

    async fn process_changes(&self, text: &str, uri: Url) {
        let diagnostics = self.parse_and_cache_pax_file(text, uri.clone());

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    fn get_valid_setter(&self, uri: &str, pos: &Position) -> Option<String> {
        if let Some(rope) = self.document_content.get(uri) {
            let char_pos = rope.line_to_char(pos.line as usize) + pos.character as usize;

            let start = if char_pos >= 50 { char_pos - 50 } else { 0 };
            let text_before_pos = rope.slice(start..char_pos).to_string();

            let pattern = r"(@?[A-Za-z_\d]+(=|::|:|\.| )*)";

            let re = Regex::new(pattern).unwrap();

            let mut largest_match: Option<String> = None;

            let captures: Vec<Captures> = re.captures_iter(&text_before_pos).collect();

            for captures in captures.into_iter().rev() {
                if let Some(matched) = captures.get(0) {
                    let matched_str = matched.as_str().to_string();
                    if matched.end() == text_before_pos.len()
                        && (largest_match.is_none()
                            || matched_str.len() > largest_match.as_ref().unwrap().len())
                    {
                        largest_match = Some(matched_str.trim_end().to_string());
                    }
                }
            }

            return largest_match;
        }

        None
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        self.set_root(params.root_uri).await;
        self.client
            .log_message(
                MessageType::INFO,
                format!("workspace root: {:?}", self.workspace_root),
            )
            .await;

        let pending_changes_clone = self.pending_changes.clone();
        let self_clone = self.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;

                let mut processed_keys = Vec::new();

                for entry in pending_changes_clone.iter() {
                    let uri_path = entry.key().clone();
                    let change_params = entry.value().clone();

                    let uri = change_params.text_document.uri.clone();

                    if uri_path.ends_with(".pax") {
                        self_clone.process_pax_file(&uri).await;
                        if !change_params.content_changes.is_empty() {
                            self_clone
                                .process_changes(
                                    &change_params.content_changes[0].text,
                                    change_params.text_document.uri.clone(),
                                )
                                .await;
                        }
                    }
                    processed_keys.push(uri_path);
                }

                // Remove processed entries
                for key in processed_keys {
                    pending_changes_clone.remove(&key);
                }
            }
        });

        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities::default(),
            offset_encoding: None,
        })
    }

    async fn initialized(&self, _: tower_lsp::lsp_types::InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, did_open_params: DidOpenTextDocumentParams) {
        let uri = did_open_params.text_document.uri.clone();
        let language_id = &did_open_params.text_document.language_id;
        if language_id == "pax" {
            self.process_pax_file(&uri).await;
            let diagnostics = self
                .parse_and_cache_pax_file(did_open_params.text_document.text.as_str(), uri.clone());
            self.client
                .publish_diagnostics(uri, diagnostics, None)
                .await;
        }
    }

    async fn did_change(&self, did_change_params: DidChangeTextDocumentParams) {
        let uri = did_change_params.text_document.uri.clone();
        let uri_path = uri.path().to_string();
        let new_content = did_change_params.content_changes[0].text.clone();
        if uri_path.ends_with(".pax") {
            self.process_pax_file(&uri).await;
            self.document_content
                .insert(uri_path.clone(), Rope::from_str(&new_content));
        }
        self.pending_changes.insert(uri_path, did_change_params);
    }

    async fn did_save(&self, did_save_params: DidSaveTextDocumentParams) {
        let uri_path = did_save_params.text_document.uri.path();

        if uri_path.ends_with(".rs") {
            if self.debounce_last_save.lock().unwrap().elapsed() < Duration::from_secs(10) {
                return;
            }
            *self.debounce_last_save.lock().unwrap() = std::time::Instant::now();

            if let Some(pax_file_path) = self.rs_to_pax_map.get(uri_path) {
                self.pax_map.remove(pax_file_path.value());

                let pax_uri = Url::from_file_path(pax_file_path.value()).unwrap();
                self.process_pax_file(&pax_uri).await;
            }
        } else if uri_path.ends_with(".pax") {
            self.pax_map.remove(uri_path);
            self.process_pax_file(&did_save_params.text_document.uri)
                .await;
        }
    }

    async fn completion(
        &self,
        completion_params: CompletionParams,
    ) -> Result<Option<CompletionResponse>> {
        let uri = &completion_params.text_document_position.text_document.uri;
        let pos = &completion_params.text_document_position.position;
        let prior_identifier = self.get_valid_setter(uri.clone().path(), pos);
        let selector_info = self.pax_selector_map.get(&uri.path().to_string());

        let mut completions = Vec::new();
        if let Some(cached_nodes) = self.pax_ast_cache.get(&uri.path().to_string()) {
            let relevant_nodes = find_nodes_at_position(pos.clone(), &cached_nodes);
            let tag_node = find_relevant_tag(&relevant_nodes);
            let has_attribute_error = has_attribute_error(&relevant_nodes);
            let is_inside_settings_block = is_inside_settings_block(&relevant_nodes);
            let is_inside_handlers_block = is_inside_handlers_block(&relevant_nodes);
            let is_inside_selector_block = is_inside_selector_block(&relevant_nodes);
            if let Some(component) = self.pax_map.get(&uri.path().to_string()) {
                if let Some(trigger_char) = &completion_params
                    .context
                    .and_then(|ctx| ctx.trigger_character)
                {
                    if trigger_char == "<" {
                        for entry in component.identifier_map.iter() {
                            self.client
                                .log_message(MessageType::INFO, format!("entry: {:?}", entry.key()))
                                .await;
                            if entry.ty == IdentifierType::Component
                                && entry.identifier != component.component_name
                            {
                                let mut completion = CompletionItem::new_simple(
                                    entry.identifier.clone(),
                                    String::from(""),
                                );
                                completion.kind = Some(CompletionItemKind::CLASS);
                                completion.insert_text =
                                    Some(format!("{} $0 />", entry.identifier.clone()));
                                completion.insert_text_format =
                                    Some(lsp_types::InsertTextFormat::SNIPPET);
                                if let Some(prepared_completion) =
                                    get_struct_completion(&entry.identifier)
                                {
                                    completion = prepared_completion;
                                }
                                completions.push(completion);
                            }
                        }
                        return Ok(Some(CompletionResponse::Array(completions)));
                    } else if trigger_char == "@" {
                        if let Some(_tag) = tag_node {
                            completions.extend(get_event_completions("="));
                            return Ok(Some(CompletionResponse::Array(completions)));
                        } else if !is_inside_settings_block && !is_inside_handlers_block {
                            completions.extend(get_block_declaration_completions());
                            return Ok(Some(CompletionResponse::Array(completions)));
                        }
                    } else if trigger_char == "=" {
                        if let Some(setter) = prior_identifier {
                            if setter.contains("@") {
                                completions.extend(get_root_component_methods(&component));
                                return Ok(Some(CompletionResponse::Array(completions)));
                            } else {
                                let requested_property = setter.clone().replace("=", "");
                                if let Some(tag) = tag_node {
                                    if let NodeType::Tag(tag_data) = &tag.node_type {
                                        if requested_property == "class" {
                                            completions.extend(get_class_completions(
                                                &selector_info,
                                                false,
                                                false,
                                            ));
                                            return Ok(Some(CompletionResponse::Array(
                                                completions,
                                            )));
                                        } else if requested_property == "id" {
                                            completions.extend(get_id_completions(
                                                &selector_info,
                                                false,
                                                false,
                                            ));
                                            return Ok(Some(CompletionResponse::Array(
                                                completions,
                                            )));
                                        } else {
                                            completions.extend(
                                                get_struct_property_type_completion(
                                                    &component,
                                                    tag_data.pascal_identifier.clone(),
                                                    requested_property.clone(),
                                                ),
                                            );
                                        }
                                    }
                                }
                                completions.extend(get_common_property_type_completion(
                                    &component,
                                    requested_property.clone(),
                                ));
                                return Ok(Some(CompletionResponse::Array(completions)));
                            }
                        }
                    } else if trigger_char == ":" {
                        if let Some(word) = prior_identifier {
                            if word.contains("::") {
                                let requested_struct = word.clone().replace("::", "");
                                completions.extend(get_struct_static_member_completions(
                                    &component,
                                    requested_struct,
                                ));
                                return Ok(Some(CompletionResponse::Array(completions)));
                            } else {
                                if is_inside_handlers_block {
                                    completions.extend(get_root_component_methods(&component));
                                    return Ok(Some(CompletionResponse::Array(completions)));
                                } else {
                                    let requested_property = word.clone().replace(":", "");
                                    completions.extend(get_common_property_type_completion(
                                        &component,
                                        requested_property.clone(),
                                    ));
                                    return Ok(Some(CompletionResponse::Array(completions)));
                                }
                            }
                        }
                    } else if trigger_char == "." {
                        if let Some(word) = prior_identifier {
                            if word.contains("self") {
                                completions
                                    .extend(get_all_root_component_member_completions(&component));
                                return Ok(Some(CompletionResponse::Array(completions)));
                            }
                        }
                        if is_inside_settings_block {
                            completions.extend(get_class_completions(&selector_info, true, false));
                            return Ok(Some(CompletionResponse::Array(completions)));
                        }
                    } else if trigger_char == "#" {
                        if is_inside_settings_block {
                            completions.extend(get_id_completions(&selector_info, true, false));
                            return Ok(Some(CompletionResponse::Array(completions)));
                        }
                    }
                } else {
                    if let Some(tag) = tag_node {
                        if let NodeType::Tag(tag_data) = &tag.node_type {
                            if let Some(word) = prior_identifier.clone() {
                                if word.contains("@") {
                                    completions.extend(get_root_component_methods(&component));
                                    return Ok(Some(CompletionResponse::Array(completions)));
                                } else if word.contains("=") {
                                    let requested_property = word.clone().replace("=", "");
                                    self.client
                                        .log_message(
                                            MessageType::INFO,
                                            format!("requested_property: {}", requested_property),
                                        )
                                        .await;
                                    completions.extend(get_struct_property_type_completion(
                                        &component,
                                        tag_data.pascal_identifier.clone(),
                                        requested_property.clone(),
                                    ));
                                    completions.extend(get_common_property_type_completion(
                                        &component,
                                        requested_property.clone(),
                                    ));
                                    return Ok(Some(CompletionResponse::Array(completions)));
                                }
                            }
                            if !has_attribute_error {
                                completions.extend(get_struct_property_setting_completions(
                                    &component,
                                    tag_data.clone().pascal_identifier,
                                ));
                                completions.extend(get_common_properties_setting_completions(
                                    &component, "=",
                                ));
                                return Ok(Some(CompletionResponse::Array(completions)));
                            }
                        }
                    }
                    if is_inside_settings_block {
                        if is_inside_selector_block {
                            completions
                                .extend(get_common_properties_setting_completions(&component, ":"));
                            return Ok(Some(CompletionResponse::Array(completions)));
                        }
                        if let Some(word) = prior_identifier.clone() {
                            let requested_property = word.clone().replace(":", "").replace("=", "");
                            completions.extend(get_common_property_type_completion(
                                &component,
                                requested_property.clone(),
                            ));
                            return Ok(Some(CompletionResponse::Array(completions)));
                        } else {
                            completions.extend(get_class_completions(&selector_info, true, true));
                            completions.extend(get_id_completions(&selector_info, true, true));
                            return Ok(Some(CompletionResponse::Array(completions)));
                        }
                    }
                    if is_inside_handlers_block {
                        if let Some(_) = prior_identifier.clone() {
                            completions.extend(get_root_component_methods(&component));
                            return Ok(Some(CompletionResponse::Array(completions)));
                        } else {
                            completions.extend(get_event_completions(":"));
                            return Ok(Some(CompletionResponse::Array(completions)));
                        }
                    }
                }
            }
        }
        return Ok(Some(CompletionResponse::Array(completions)));
    }
}

// pub async fn start_server() {
//     let stdin = tokio::io::stdin();
//     let stdout = tokio::io::stdout();

//     let (service, socket) = LspService::build(|client| Backend {
//         client: Arc::new(client),
//         pax_map: Arc::new(DashMap::new()),
//         rs_to_pax_map: Arc::new(DashMap::new()),
//         workspace_root: Arc::new(Mutex::new(None)),
//         pax_ast_cache: Arc::new(DashMap::new()),
//         pending_changes: Arc::new(DashMap::new()),
//         debounce_last_save: Arc::new(Mutex::new(std::time::Instant::now())),
//         document_content: Arc::new(DashMap::new()),
//     })
//     .custom_method("pax/getHoverId", Backend::hover_id)
//     .custom_method("pax/getDefinitionId", Backend::definition_id)
//     .finish();

//     Server::new(stdin, stdout, socket).serve(service).await;
// }

#[tokio::main]
pub async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::build(|client| Backend {
        client: Arc::new(client),
        pax_map: Arc::new(DashMap::new()),
        rs_to_pax_map: Arc::new(DashMap::new()),
        workspace_root: Arc::new(Mutex::new(None)),
        pax_ast_cache: Arc::new(DashMap::new()),
        pax_selector_map: Arc::new(DashMap::new()),
        pending_changes: Arc::new(DashMap::new()),
        debounce_last_save: Arc::new(Mutex::new(std::time::Instant::now())),
        document_content: Arc::new(DashMap::new()),
    })
    .custom_method("pax/getHoverId", Backend::hover_id)
    .custom_method("pax/getDefinitionId", Backend::definition_id)
    .finish();

    Server::new(stdin, stdout, socket).serve(service).await;
}
