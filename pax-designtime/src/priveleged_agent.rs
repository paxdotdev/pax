use std::net::SocketAddr;

use crate::messages::{AgentMessage, ManifestSerializationRequest};
use anyhow::{anyhow, Result};
use pax_manifest::PaxManifest;
pub struct PrivilegedAgentConnection {
    sender: ewebsock::WsSender,
    recver: ewebsock::WsReceiver,
}

impl PrivilegedAgentConnection {
    pub fn new(addr: SocketAddr) -> Result<Self> {
        let (sender, recver) = ewebsock::connect(format!("ws://{}/ws", addr))
            .map_err(|_| anyhow!("couldn't create socket connection"))?;
        Ok(Self { sender, recver })
    }

    pub fn send_manifest_update(&mut self, manifest: &PaxManifest) -> Result<()> {
        let manifest_bytes = rmp_serde::to_vec(&manifest)?;
        let msg_bytes = rmp_serde::to_vec(&AgentMessage::ManifestSerializationRequest(
            ManifestSerializationRequest {
                manifest: manifest_bytes,
            },
        ))?;

        self.sender.send(ewebsock::WsMessage::Binary(msg_bytes));
        Ok(())
    }
}
