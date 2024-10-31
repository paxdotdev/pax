use crate::{
    messages::{
        AgentMessage, ChangeType, ComponentSerializationRequest, LLMRequest,
        LoadFileToStaticDirRequest,
    },
    orm::PaxManifestORM,
};
use anyhow::{anyhow, Result};
use ewebsock::{WsEvent, WsMessage};
use pax_manifest::{ComponentDefinition, PaxManifest};
use url::Url;

pub struct WebSocketConnection {
    sender: ewebsock::WsSender,
    recver: ewebsock::WsReceiver,
    pub alive: bool,
}

impl WebSocketConnection {
    pub fn new(addr: &str, versioning_prefix: Option<&str>) -> Result<Self> {
        // Parse the address as a URL
        let mut url = Url::parse(addr).map_err(|e| anyhow!("Invalid URL: {}", e))?;

        // Change the scheme to 'ws' or 'wss' depending on the original scheme
        let ws_scheme = match url.scheme() {
            "http" => "ws",
            "https" => "wss",
            "ws" | "wss" => url.scheme(),
            _ => return Err(anyhow!("Unsupported URL scheme: {}", url.scheme())),
        }
        .to_owned();

        url.set_scheme(&ws_scheme)
            .map_err(|_| anyhow!("Failed to set URL scheme"))?;

        // Append the versioning prefix and '/ws' to the path
        let versioning_prefix = versioning_prefix.unwrap_or("");
        let new_path = format!("{}/ws", versioning_prefix);
        url.set_path(&new_path);

        let url_str = url.to_string();

        // Connect using ewebsock
        let (sender, recver) =
            ewebsock::connect(url_str).map_err(|_| anyhow!("Couldn't create socket connection"))?;

        Ok(Self {
            sender,
            recver,
            alive: true,
        })
    }

    pub fn send_manifest_load_request(&mut self) -> Result<()> {
        let msg_bytes = rmp_serde::to_vec(&AgentMessage::LoadManifestRequest)?;
        self.sender.send(ewebsock::WsMessage::Binary(msg_bytes));
        Ok(())
    }

    pub fn send_component_update(&mut self, component: &ComponentDefinition) -> Result<()> {
        if self.alive {
            let component_bytes = rmp_serde::to_vec(&component)?;
            let msg_bytes = rmp_serde::to_vec(&AgentMessage::ComponentSerializationRequest(
                ComponentSerializationRequest { component_bytes },
            ))?;
            self.sender.send(ewebsock::WsMessage::Binary(msg_bytes));
            Ok(())
        } else {
            Err(anyhow!(
                "couldn't send component update: connection to design-server was lost"
            ))
        }
    }

    pub fn send_updated_files(&mut self, files: Vec<(String, String)>) -> Result<()> {
        if self.alive {
            let msg_bytes = rmp_serde::to_vec(&AgentMessage::WriteNewFilesRequest(files))?;
            self.sender.send(ewebsock::WsMessage::Binary(msg_bytes));
            Ok(())
        } else {
            Err(anyhow!(
                "couldn't send updated files: connection to design-server was lost"
            ))
        }
    }

    pub fn send_llm_request(&mut self, llm_request: LLMRequest) -> Result<()> {
        if self.alive {
            let msg_bytes = rmp_serde::to_vec(&AgentMessage::LLMRequest(llm_request))?;
            self.sender.send(ewebsock::WsMessage::Binary(msg_bytes));
            Ok(())
        } else {
            Err(anyhow!(
                "couldn't send LLM request: connection to pub pax was lost"
            ))
        }
    }

    pub fn send_file_to_static_dir(&mut self, name: &str, data: Vec<u8>) -> Result<()> {
        if self.alive {
            let msg_bytes = rmp_serde::to_vec(&AgentMessage::LoadFileToStaticDirRequest(
                LoadFileToStaticDirRequest {
                    name: name.to_owned(),
                    data,
                },
            ))?;
            self.sender.send(ewebsock::WsMessage::Binary(msg_bytes));
            Ok(())
        } else {
            Err(anyhow!(
                "couldn't send file: connection to design-server was lost"
            ))
        }
    }

    pub fn handle_recv(&mut self, manager: &mut PaxManifestORM) -> Result<()> {
        while let Some(event) = self.recver.try_recv() {
            match event {
                WsEvent::Opened => {
                    self.send_manifest_load_request()?;
                }
                WsEvent::Message(message) => {
                    if let WsMessage::Binary(msg_bytes) = message {
                        let msg: AgentMessage = rmp_serde::from_slice(&msg_bytes)?;
                        match msg {
                            AgentMessage::LoadManifestResponse(resp) => {
                                let manifest: PaxManifest = rmp_serde::from_slice(&resp.manifest)?;
                                manager.set_manifest(manifest);
                            }
                            AgentMessage::UpdateTemplateRequest(resp) => {
                                manager
                                    .replace_template(
                                        resp.type_id,
                                        resp.new_template,
                                        resp.settings_block,
                                    )
                                    .map_err(|e| anyhow!(e))?;
                            }
                            AgentMessage::LLMPartialResponse(partial) => {
                                manager.add_new_message(
                                    partial.request_id,
                                    partial.message,
                                    vec![],
                                );
                            }
                            AgentMessage::LLMFinalResponse(final_response) => {
                                if let ChangeType::PaxOnly(components) = final_response.changes {
                                    manager.add_new_message(
                                        final_response.request_id,
                                        final_response.message,
                                        components,
                                    );
                                } else if let ChangeType::FullReload(project_files) =
                                    final_response.changes
                                {
                                    let updated_files: Vec<(String, String)> = project_files
                                        .iter()
                                        .filter(|(f, _)| f.ends_with(".rs") | f.ends_with(".pax"))
                                        .cloned()
                                        .collect();
                                    manager.set_updated_project_files(updated_files);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                WsEvent::Error(e) => log::warn!("web socket error: {e}"),
                WsEvent::Closed => {
                    self.alive = false;
                    log::warn!("web socket was closed")
                }
            }
        }
        Ok(())
    }
}
