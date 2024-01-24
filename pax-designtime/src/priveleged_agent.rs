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

    /// not used atm.
    /// Something similar to this could in the future be responsible for
    /// handling a "files changed" notification to let designer know when to reload wasm.
    pub fn _tick_recv(&self) -> Result<()> {
        if let Some(value) = self.recver.try_recv() {
            match value {
                ewebsock::WsEvent::Message(ewebsock::WsMessage::Binary(bytes)) => {
                    let res: AgentMessage = rmp_serde::from_slice(&bytes)?;
                    match res {
                        AgentMessage::ManifestSerializationAcknowledgement(ack) => {
                            log::trace!("manifest serialization acc id: {}", ack.id)
                        }
                        _ => {
                            log::trace!("recieved unexpected acc after sending manifest update")
                        }
                    }
                }
                ewebsock::WsEvent::Opened => log::info!("opened connection to priv-agent"),
                ewebsock::WsEvent::Error(e) => log::warn!("recieved priv-agent socket error {}", e),
                ewebsock::WsEvent::Closed => log::info!("closed connection to priv-agent"),
                ewebsock::WsEvent::Message(_m) => {
                    log::warn!("recieved non-binary message from priv agent")
                }
            }
        } else {
            log::warn!("no message!!");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Ok(())
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
