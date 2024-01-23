use std::net::SocketAddr;

use crate::messages::{AgentMessage, ManifestSerializationRequest};
use anyhow::{anyhow, Result};
use pax_manifest::PaxManifest;
pub struct PrivilegedAgentConnection {
    sender: ewebsock::WsSender,
    recver: ewebsock::WsReceiver,
}
// "ws://127.0.0.1:8252/ws"
impl PrivilegedAgentConnection {
    pub fn new(addr: SocketAddr) -> Result<Self> {
        //Static, for now
        // const PRIV_ADDR: &'static str = "ws://127.0.0.1:8252/ws";
        let (sender, recver) = ewebsock::connect(format!("ws://{}/ws", addr))
            .map_err(|_| anyhow!("couldn't create socket connection"))?;
        Ok(Self { sender, recver })
    }

    pub fn send_manifest_update(&mut self, manifest: &PaxManifest) -> Result<()> {
        let msg_bytes = rmp_serde::to_vec(&AgentMessage::ManifestSerializationRequest(
            ManifestSerializationRequest {
                manifest: manifest.clone(),
            },
        ))?;

        self.sender.send(ewebsock::WsMessage::Binary(msg_bytes));
        Ok(())
    }

    // Something similar to this can eventually be used to recieve wasm reload requests
    pub fn _tick_recv(&mut self) {
        if let Some(value) = self.recver.try_recv() {
            match value {
                ewebsock::WsEvent::Message(ewebsock::WsMessage::Binary(bytes)) => {
                    let Ok(res) = rmp_serde::from_slice(&bytes) else {
                        return;
                    };
                    match res {
                        AgentMessage::ManifestSerializationAcknowledgement(ack) => {
                            log::info!("manifest serialization acc id: {}", ack.id)
                        }
                        _ => {
                            log::warn!("recieved unexpected acc after sending manifest update")
                        }
                    }
                }
                ewebsock::WsEvent::Opened => log::info!("opened????"),
                ewebsock::WsEvent::Error(_) => log::info!("unexpected: error"),
                ewebsock::WsEvent::Closed => log::info!("unexpected: closed connection"),
                _ => log::warn!("whaat!!"),
            }
        } else {
            log::warn!("no message!!");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
