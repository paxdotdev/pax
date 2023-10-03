use core::panic;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use pest::Parser;
use dashmap::DashMap;
use lsp_types::request::Request;
use pax_compiler::parsing::{PaxParser, Rule, self};
use serde::*;
use syn::parse;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

mod index;
use index::{
    extract_import_positions, find_rust_file_with_macro, index_rust_file, IdentifierInfo,
    IdentifierType, InfoRequest,
};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

extern crate pest;
use pest::iterators::{Pair, Pairs};
use pest_derive::Parser;

use pest::pratt_parser::{Assoc, Op, PrattParser};


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
struct PaxComponent {
    component_name: String,
    identifier_map: DashMap<String, IdentifierInfo>,
}

#[derive(Debug, Clone)]
struct Backend {
    client: Arc<Client>,
    pax_map: Arc<DashMap<String, PaxComponent>>,
    rs_to_pax_map: Arc<DashMap<String, String>>,
    workspace_root: Arc<Mutex<Option<Url>>>,
    rust_file_opened: Arc<Mutex<bool>>,
    pax_ast_cache: Arc<DashMap<String, Vec<PositionalNode>>>,
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
                IdentifierType::Component | IdentifierType::PaxType => {
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
                        .remove(&(&info_request).struct_identifier.clone().unwrap())
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
                            (&info_request).struct_identifier.clone().unwrap().clone(),
                            struct_info.1,
                        );
                    }
                }
                IdentifierType::Method => {
                    if let Some(mut struct_info) = component
                        .identifier_map
                        .remove(&(&info_request).struct_identifier.clone().unwrap())
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
                            (&info_request).struct_identifier.clone().unwrap().clone(),
                            struct_info.1,
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
        self.client
            .log_message(MessageType::INFO, format!("{} is new. indexing", pax_file))
            .await;

        // Create an empty identifier_map for the new component
        let identifier_map: DashMap<String, IdentifierInfo> = DashMap::new();

        // Index the found Rust file and populate the identifier_map
        let rust_file_path_str = rust_file_path.to_string_lossy().to_string();
        let info_requests = match index_rust_file(&rust_file_path_str, &identifier_map) {
            Ok(reqs) => reqs,
            Err(err) => {
                eprintln!("Error indexing file {}: {:?}", rust_file_path_str, err);
                return;
            }
        };

        // Handle info requests for the initial file
        for info_request in info_requests.clone() {
            let backend_clone = self.clone();
            let pax_file_clone = pax_file.to_string();
            tokio::spawn(async move {
                backend_clone
                    .handle_info_request(pax_file_clone, info_request)
                    .await;
            });
        }

        // Update the pax_map with the new component
        self.pax_map.insert(
            pax_file.to_string(),
            PaxComponent {
                component_name,
                identifier_map,
            },
        );

        self.rs_to_pax_map
            .insert(rust_file_path_str, pax_file.to_string());

        // Extract import positions
        let positions = extract_import_positions(&rust_file_path);

        for position in positions {
            let symbol_data = SymbolData {
                uri: rust_file_path.to_string_lossy().to_string(),
                position,
            };

            let params = SymbolLocationParams {
                symbol: symbol_data,
            };

            // Send the request and log the response
            match self
                .client
                .send_request::<GetDefinitionRequest>(params)
                .await
            {
                Ok(response) => {
                    for location_link in response.locations {
                        let target_uri = location_link.target_uri;
                        let target_file = target_uri
                            .to_file_path()
                            .expect("Failed to convert URI to path chill")
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
                    eprintln!("Error sending request: {:?}", e);
                }
            }
        }
    }

    fn parse_and_cache_pax_file(&self, pax: &str, uri: Url) -> Vec<Diagnostic> {
        let parse_result = PaxParser::parse(Rule::pax_component_definition, pax);

        
        // Convert URI to a file path string
        let path_str = uri.path();

        //let mut cache_guard = self.pax_ast_cache.lock().unwrap();

        match parse_result {
            Ok(pax_component_definition) => {
                //cache_guard.insert(path_str.clone(), Some(parsed_ast.clone()));
                let mut nodes = Vec::new();
                for pair in pax_component_definition.clone() {
                    extract_positional_nodes(pair.clone(), &mut nodes);
                }
                self.pax_ast_cache.insert(path_str.clone().to_string(), nodes.clone());
                let errors = parsing::extract_errors(pax_component_definition.clone().next().unwrap().into_inner());

                // If there are errors, publish them as diagnostics
                if !errors.is_empty() {
                    let diagnostics: Vec<Diagnostic> = errors.into_iter().map(|err| {
                        Diagnostic {
                            range: Range {
                                start: Position {
                                    line: (err.start.0 - 1) as u32,
                                    character: (err.start.1-1) as u32,
                                },
                                end: Position {
                                    line: (err.end.0 - 1) as u32,
                                    character: (err.end.1-1) as u32,
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
                        }
                    }).collect();
                    diagnostics
                } else {
                    Vec::new()
                }
            },
            Err(e) => {
                // Handle the case when the pax file fails to parse completely
                eprintln!("Failed to parse {}: {:?}", path_str, e);
                Vec::new()
                // Clear the specific pax file cache
                //cache_guard.insert(path_str, None);
            }
        }
    }

    async fn completion_id(&self, _: CompletionParams) -> Result<String> {
        self.client
        .log_message(
            MessageType::INFO,
            "fired completion_id",
        )
        .await;
        Ok("test".to_string())
    }

    async fn definition_id(&self, _: GotoDefinitionParams) -> Result<String> {
        self.client
        .log_message(
            MessageType::INFO,
            "fired definition",
        )
        .await;
        Ok("test".to_string())
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
        self.client
        .log_message(MessageType::INFO, "file opened!")
        .await;
        let uri = did_open_params.text_document.uri.clone();
        self.client
        .log_message(MessageType::INFO, format!("{:?}", uri.path()))
        .await;
        let language_id = &did_open_params.text_document.language_id;
        if language_id == "rust" {
            //thread::sleep(Duration::from_secs(10));
            let mut rust_file_opened_guard = self.rust_file_opened.lock().unwrap();
            *rust_file_opened_guard = true;
        } else if language_id == "pax" {
            let rust_file_opened = { self.rust_file_opened.lock().unwrap().clone() };
            let file_path = uri.path();
            if !rust_file_opened || self.pax_map.contains_key(file_path) {
                return;
            }
            let path = uri.clone().to_file_path().expect("Failed to get file path");
            let directory = path.parent().expect("Failed to get parent directory");

            if let Some((rust_file_path, component_name)) =
                find_rust_file_with_macro(directory, &file_path)
            {
                // Create a backend clone and use it within tokio::spawn to handle the indexing
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

            let diagnostics = self.parse_and_cache_pax_file(did_open_params.text_document.text.as_str(), uri.clone());
            self.client.publish_diagnostics(uri, diagnostics, None).await;
        }
    }

    async fn did_change(&self, did_change_params: DidChangeTextDocumentParams) {
        if !did_change_params.content_changes.is_empty() && did_change_params.text_document.uri.path().ends_with(".pax"){
            let uri = did_change_params.text_document.uri.clone();
            let diagnostics = self.parse_and_cache_pax_file(did_change_params.content_changes[0].text.as_str(), uri.clone());
            self.client.publish_diagnostics(uri, diagnostics, None).await;
        }
    }

    // async fn did_save(&self, did_save_params: DidSaveTextDocumentParams) {
    //     self.client
    //         .log_message(MessageType::INFO, "File saved!")
    //         .await;
    
    //     let uri = &did_save_params.text_document.uri;
    //     let saved_file_path = uri
    //         .to_file_path()
    //         .expect("Failed to convert URI to path")
    //         .to_string_lossy()
    //         .to_string();
    
    //     // Check if the file from pax_map matches the saved file
    //     if let Some(component) = self.pax_map.get(&saved_file_path) {
    //         self.client
    //             .log_message(
    //                 MessageType::INFO,
    //                 format!("Fetching data for file: {}", saved_file_path),
    //             )
    //             .await;
    //         for entry in component.identifier_map.iter() {
    //             let identifier = entry.key();
    //             let info = entry.value();
    //             self.client
    //                 .log_message(
    //                     MessageType::INFO,
    //                     format!("Identifier: {}\nInfo: {:?}", identifier, info),
    //                 )
    //                 .await;
    //         }
    //     }
    // }
    
    async fn did_save(&self, did_save_params: DidSaveTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "File saved!")
            .await;
    
        let uri = &did_save_params.text_document.uri;
        let saved_file_path = uri
            .to_file_path()
            .expect("Failed to convert URI to path")
            .to_string_lossy()
            .to_string();
    
        // Check if the file from pax_ast_cache matches the saved file
        if let Some(ast_nodes) = self.pax_ast_cache.get(&saved_file_path) {
            self.client
                .log_message(
                    MessageType::INFO,
                    format!("Fetching AST nodes for file: {}", saved_file_path),
                )
                .await;
    
            for node in ast_nodes.iter() {
                self.client
                    .log_message(
                        MessageType::INFO,
                        format!("AST Node: {:?}", node),
                    )
                    .await;
            }
        }
    }    


    async fn goto_definition(
        &self,
        _: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        Ok(None)
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        Ok(None)
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::build(|client| Backend {
        client: Arc::new(client),
        pax_map: Arc::new(DashMap::new()),
        rs_to_pax_map: Arc::new(DashMap::new()),
        workspace_root: Arc::new(Mutex::new(None)),
        rust_file_opened: Arc::new(Mutex::new(false)),
        pax_ast_cache: Arc::new(DashMap::new()),
    }).custom_method("pax/getCompletionId", Backend::completion_id)
    .custom_method("pax/getDefinitionId",  Backend::definition_id)
    .finish();

    Server::new(stdin, stdout, socket).serve(service).await;
}


#[derive(Debug, Clone)]
struct PositionalNode {
    start: Position,
    end: Position,
    node_type: NodeType,
}

#[derive(Debug, Clone)]
enum NodeType {
    Identifier(IdentifierData),
    Tag(TagData),
    Handlers,
    Settings,
}

#[derive(Debug, Clone)]
struct TagData {
    pascal_identifier: String,
}

#[derive(Debug, Clone)]
struct IdentifierData {
    identifier: String,
    is_pascal_identifier: bool,
}

fn pair_to_positions(pair: Pair<Rule>) -> (Position, Position){
    let span = pair.as_span();
    let start = Position {
        line: (span.start_pos().line_col().0 - 1) as u32,
        character: (span.start_pos().line_col().1 - 1) as u32,
    };
    let end = Position {
        line: (span.end_pos().line_col().0 - 1) as u32,
        character: (span.end_pos().line_col().1 - 1) as u32,
    };
    (start, end)
}

fn extract_positional_nodes(pair: Pair<'_, Rule>, nodes: &mut Vec<PositionalNode>){
        let (start, end) = pair_to_positions(pair.clone());
        match pair.as_rule() {
            Rule::handlers_block_declaration => {
                nodes.push(PositionalNode {
                    start,
                    end,
                    node_type: NodeType::Handlers,
                });
            },
            Rule::settings_block_declaration => {
                nodes.push(PositionalNode {
                    start,
                    end,
                    node_type: NodeType::Settings,
                });
            },
            Rule::open_tag | Rule::open_tag_error | Rule::tag_error | Rule::closing_tag => {
                if let Some(inner_pair) = pair.clone().into_inner().find(|p| p.as_rule() == Rule::pascal_identifier) {
                    let identifier = inner_pair.as_str().to_string();
                    nodes.push(PositionalNode {
                        start,
                        end,
                        node_type: NodeType::Tag(TagData {
                            pascal_identifier: identifier,
                        }),
                    });
                }
            },
            Rule::pascal_identifier => {
                let identifier = pair.as_str().to_string();
                nodes.push(PositionalNode {
                    start,
                    end,
                    node_type: NodeType::Identifier(IdentifierData {
                        identifier,
                        is_pascal_identifier: true,
                    }),
                });
            },
            Rule::identifier => {
                let identifier = pair.as_str().to_string();
                nodes.push(PositionalNode {
                    start,
                    end,
                    node_type: NodeType::Identifier(IdentifierData {
                        identifier,
                        is_pascal_identifier: false,
                    }),
                });
            },
            _ => {}
        }

        // Recursively process inner nodes
        for inner_pair in pair.into_inner() {
            extract_positional_nodes(inner_pair, nodes);
        }
    }

fn find_nodes_at_position(pos: Position, nodes: &Vec<PositionalNode>) -> Vec<PositionalNode> {
    nodes.iter()
         .filter(|&node| is_position_within_node(&pos, node))
         .cloned()
         .collect()
}

fn is_position_within_node(pos: &Position, node: &PositionalNode) -> bool {
    // Check if the given position lies within the start and end of the node
    (node.start.line < pos.line || (node.start.line == pos.line && node.start.character <= pos.character))
    &&
    (node.end.line > pos.line || (node.end.line == pos.line && node.end.character >= pos.character))
}