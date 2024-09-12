use std::net::SocketAddr;

use crate::{
    messages::{
        AgentMessage, ComponentSerializationRequest, LLMHelpRequest, LoadFileToStaticDirRequest,
    },
    orm::{template::NodeAction, PaxManifestORM},
};
use anyhow::{anyhow, Result};
use ewebsock::{WsEvent, WsMessage};
use pax_manifest::{ComponentDefinition, PaxManifest};

pub struct PrivilegedAgentConnection {
    sender: ewebsock::WsSender,
    recver: ewebsock::WsReceiver,
    pub alive: bool,
}

impl PrivilegedAgentConnection {
    pub fn new(addr: SocketAddr) -> Result<Self> {
        let url = format!("ws://{}/ws", addr);
        let (sender, recver) =
            ewebsock::connect(url).map_err(|_| anyhow!("couldn't create socket connection"))?;
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
                                    .replace_template(resp.type_id, resp.new_template)
                                    .map_err(|e| anyhow!(e))?;
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
