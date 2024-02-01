use std::net::SocketAddr;

use crate::messages::{AgentMessage, ComponentSerializationRequest, ManifestSerializationRequest};
use anyhow::{anyhow, Result};
use pax_manifest::{ComponentDefinition, PaxManifest};

pub struct PrivilegedAgentConnection {
    sender: ewebsock::WsSender,
    _recver: ewebsock::WsReceiver,
}

impl PrivilegedAgentConnection {
    pub fn new(addr: SocketAddr) -> Result<Self> {
        let (sender, recver) = ewebsock::connect(format!("ws://{}/ws", addr))
            .map_err(|_| anyhow!("couldn't create socket connection"))?;
        Ok(Self {
            sender,
            _recver: recver,
        })
    }

    pub fn send_component_update(&mut self, component: &ComponentDefinition) -> Result<()> {
        let component_bytes = rmp_serde::to_vec(&component)?;
        let msg_bytes = rmp_serde::to_vec(&AgentMessage::ComponentSerializationRequest(
            ComponentSerializationRequest { component_bytes },
        ))?;

        self.sender.send(ewebsock::WsMessage::Binary(msg_bytes));
        Ok(())
    }
}
