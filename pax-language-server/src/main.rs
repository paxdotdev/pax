use core::panic;
use std::fs;
use std::path::PathBuf;

use dashmap::DashMap;
use lsp_types::request::Request;
use serde::*;
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
                    eprintln!("Error sending request: {:?}", e);
                }
            }
        }
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
        let language_id = &did_open_params.text_document.language_id;
        let uri = &did_open_params.text_document.uri;
        self.client
            .log_message(MessageType::INFO, format!("Received URI: {}", uri))
            .await;

        if language_id == "rust" {
            //thread::sleep(Duration::from_secs(10));
            let mut rust_file_opened_guard = self.rust_file_opened.lock().unwrap();
            *rust_file_opened_guard = true;
        } else if language_id == "pax" {
            let rust_file_opened = { self.rust_file_opened.lock().unwrap().clone() };
            let path = uri.to_file_path().expect("Failed to convert URI to path");
            let path_str = path.to_string_lossy().to_string();
            if !rust_file_opened || self.pax_map.contains_key(&path_str) {
                return;
            }

            let directory = path.parent().expect("Failed to get parent directory");

            if let Some((rust_file_path, component_name)) =
                find_rust_file_with_macro(directory, &path_str)
            {
                // Create a backend clone and use it within tokio::spawn to handle the indexing
                let backend_clone = self.clone();
                tokio::spawn(async move {
                    backend_clone
                        .index_file(&path_str, rust_file_path, component_name)
                        .await;
                });
            } else {
                eprintln!("No matching Rust file found for {}", path.display());
            }
        }
    }

    // async fn did_opens(&self, did_open_params: DidOpenTextDocumentParams) {
    //     let language_id = &did_open_params.text_document.language_id;
    //     if language_id != "pax" {
    //         return;
    //     }

    //     let uri = &did_open_params.text_document.uri;
    //     let path = uri.to_file_path().expect("Failed to convert URI to path");
    //     let directory = path.parent().expect("Failed to get parent directory");
    //     let path_str = path.to_string_lossy().to_string();

    //     if let Some((rust_file_path, component_name)) =
    //         find_rust_file_with_macro(directory, &path_str)
    //     {
    //         self.client
    //             .log_message(MessageType::INFO, format!("{} is new. indexing", &path_str))
    //             .await;

    //         // Create an empty identifier_map for the new component
    //         let identifier_map: DashMap<String, IdentifierInfo> = DashMap::new();

    //         // Index the found Rust file and populate the identifier_map
    //         let rust_file_path_str = rust_file_path.to_string_lossy().to_string();
    //         index_rust_file(&rust_file_path_str, &identifier_map)
    //             .expect("Failed to index the Rust file");

    //         // Update the pax_map with the new component
    //         self.pax_map.insert(
    //             path_str.clone(),
    //             PaxComponent {
    //                 component_name,
    //                 identifier_map,
    //             },
    //         );

    //         self.rs_to_pax_map
    //             .insert(rust_file_path_str, path_str.clone());

    //         // Extract import positions
    //         let positions = extract_import_positions(&rust_file_path);

    //         for position in positions {
    //             let symbol_data = SymbolData {
    //                 uri: rust_file_path.to_string_lossy().to_string(),
    //                 position,
    //             };

    //             let params = SymbolLocationParams {
    //                 symbol: symbol_data,
    //             };

    //             // Send the request and log the response
    //             match self
    //                 .client
    //                 .send_request::<GetDefinitionRequest>(params)
    //                 .await
    //             {
    //                 Ok(response) => {
    //                     for location_link in response.locations {
    //                         let target_uri = location_link.target_uri;
    //                         let target_file = target_uri
    //                             .to_file_path()
    //                             .expect("Failed to convert URI to path")
    //                             .to_string_lossy()
    //                             .to_string();
    //                         let path = path_str.clone();
    //                         let backend_clone = self.clone();
    //                         tokio::spawn(async move {
    //                             backend_clone.handle_file(path, target_file).await;
    //                         });
    //                     }
    //                 }
    //                 Err(e) => {
    //                     eprintln!("Error sending request: {:?}", e);
    //                 }
    //             }
    //         }
    //     } else {
    //         eprintln!("No matching Rust file found for {}", path.display());
    //     }
    // }

    async fn did_change(&self, did: DidChangeTextDocumentParams) {
        // self.client
        //     .log_message(MessageType::INFO, "file changed!")
        //     .await;
    }

    async fn did_save(&self, did_save_params: DidSaveTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "File saved!")
            .await;

        let uri = &did_save_params.text_document.uri;
        let path_str = uri
            .to_file_path()
            .expect("Failed to convert URI to path")
            .to_string_lossy()
            .to_string();
        for e in self.pax_map.iter() {
            let component = e.value();
            let key = e.key();
            self.client
                .log_message(
                    MessageType::INFO,
                    format!("Fetching data for file: {}", key),
                )
                .await;
            for entry in component.identifier_map.iter() {
                let identifier = entry.key();
                let info = entry.value();
                self.client
                    .log_message(
                        MessageType::INFO,
                        format!("Identifier: {}\nInfo: {:?}", identifier, info),
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
        rust_file_opened: Arc::new(Mutex::new(false)), // Initialize as false
    })
    .finish();

    Server::new(stdin, stdout, socket).serve(service).await;
}
