use crate::{
    messages::{
        AgentMessage, ComponentSerializationRequest, DevClientResponse, LLMRequest,
        LoadFileToStaticDirRequest, UserlandSourceUpdateRequest,
    },
    orm::PaxManifestORM,
};
use anyhow::{anyhow, Result};
use ewebsock::{WsEvent, WsMessage};
use pax_manifest::{ComponentDefinition, PaxManifest};
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
use url::Url;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

const WEBSOCKET_RECONNECT_DELAY: Duration = Duration::from_millis(500);

pub struct WebSocketConnection {
    url: String,
    sender: Option<ewebsock::WsSender>,
    recver: Option<ewebsock::WsReceiver>,
    label: String,
    pub alive: bool,
    allow_reconnect: bool,
    connecting: bool,
    next_reconnect_at: Option<Instant>,
}

impl WebSocketConnection {
    pub fn new(
        addr: &str,
        versioning_prefix: Option<&str>,
        label: impl Into<String>,
    ) -> Result<Self> {
        let url = build_socket_url(addr, versioning_prefix)?;
        let (sender, recver) = connect_socket(&url)?;

        Ok(Self {
            url,
            sender: Some(sender),
            recver: Some(recver),
            label: label.into(),
            // ewebsock can buffer outbound frames before the Opened event arrives, and existing
            // designtime flows rely on being able to send immediately after constructing the socket.
            alive: true,
            allow_reconnect: true,
            connecting: true,
            next_reconnect_at: None,
        })
    }

    pub fn send_manifest_load_request(&mut self) -> Result<()> {
        let msg_bytes = rmp_serde::to_vec(&AgentMessage::LoadManifestRequest)?;
        self.sender()
            .ok_or_else(|| anyhow!("design-server socket is not connected"))?
            .send(ewebsock::WsMessage::Binary(msg_bytes));
        Ok(())
    }

    pub fn send_component_update(&mut self, component: &ComponentDefinition) -> Result<()> {
        if self.alive {
            let component_bytes = rmp_serde::to_vec(&component)?;
            let msg_bytes = rmp_serde::to_vec(&AgentMessage::ComponentSerializationRequest(
                ComponentSerializationRequest { component_bytes },
            ))?;
            self.sender()
                .ok_or_else(|| anyhow!("design-server socket is not connected"))?
                .send(ewebsock::WsMessage::Binary(msg_bytes));
            Ok(())
        } else {
            Err(anyhow!(
                "couldn't send component update: connection to design-server was lost"
            ))
        }
    }

    pub fn send_llm_request(&mut self, llm_request: LLMRequest) -> Result<()> {
        if self.alive {
            let msg_bytes = rmp_serde::to_vec(&AgentMessage::LLMRequest(llm_request))?;
            self.sender()
                .ok_or_else(|| anyhow!("pub pax socket is not connected"))?
                .send(ewebsock::WsMessage::Binary(msg_bytes));
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
            self.sender()
                .ok_or_else(|| anyhow!("design-server socket is not connected"))?
                .send(ewebsock::WsMessage::Binary(msg_bytes));
            Ok(())
        } else {
            Err(anyhow!(
                "couldn't send file: connection to design-server was lost"
            ))
        }
    }

    pub fn send_dev_client_response(&mut self, response: DevClientResponse) -> Result<()> {
        if self.alive {
            let msg_bytes = rmp_serde::to_vec(&AgentMessage::DevClientResponse(response))?;
            self.sender()
                .ok_or_else(|| anyhow!("design-server socket is not connected"))?
                .send(ewebsock::WsMessage::Binary(msg_bytes));
            Ok(())
        } else {
            Err(anyhow!(
                "couldn't send dev response: connection to design-server was lost"
            ))
        }
    }

    pub fn send_userland_source_update_request(
        &mut self,
        request: UserlandSourceUpdateRequest,
    ) -> Result<()> {
        if self.alive {
            let msg_bytes = rmp_serde::to_vec(&AgentMessage::UserlandSourceUpdateRequest(request))?;
            self.sender()
                .ok_or_else(|| anyhow!("design-server socket is not connected"))?
                .send(ewebsock::WsMessage::Binary(msg_bytes));
            Ok(())
        } else {
            Err(anyhow!(
                "couldn't send userland source update: connection to design-server was lost"
            ))
        }
    }

    pub fn handle_recv(&mut self, manager: &mut PaxManifestORM) -> Result<Vec<AgentMessage>> {
        self.reconnect_if_needed();

        let mut passthrough_messages = vec![];
        while let Some(event) = self.recver.as_ref().and_then(|recver| recver.try_recv()) {
            match event {
                WsEvent::Opened => {
                    self.alive = true;
                    self.connecting = false;
                    self.next_reconnect_at = None;
                    if let Err(err) = self.send_manifest_load_request() {
                        log::warn!("{} failed to request manifest: {err}", self.label);
                        self.schedule_reconnect();
                    }
                }
                WsEvent::Message(message) => match message {
                    WsMessage::Binary(msg_bytes) => {
                        let msg: AgentMessage = match rmp_serde::from_slice(&msg_bytes) {
                            Ok(msg) => msg,
                            Err(err) => {
                                log::warn!(
                                    "{} received invalid websocket message: {err}",
                                    self.label
                                );
                                continue;
                            }
                        };
                        match msg {
                            AgentMessage::LoadManifestResponse(resp) => {
                                match rmp_serde::from_slice::<PaxManifest>(&resp.manifest) {
                                    Ok(manifest) => manager.set_initial_server_manifest(manifest),
                                    Err(err) => log::warn!(
                                        "{} received invalid manifest payload: {err}",
                                        self.label
                                    ),
                                }
                            }
                            AgentMessage::UpdateTemplateRequest(resp) => {
                                if let Err(err) = manager.replace_template(
                                    resp.type_id,
                                    resp.new_template,
                                    resp.settings_block,
                                ) {
                                    log::warn!(
                                        "{} failed to apply template update from design-server: {}",
                                        self.label,
                                        err
                                    );
                                }
                            }
                            AgentMessage::LLMPartialResponse(partial) => {
                                manager.add_new_message(partial.request_id, partial.message, None);
                            }
                            AgentMessage::LLMFinalResponse(final_response) => {
                                manager.add_new_message(
                                    final_response.request_id,
                                    final_response.message,
                                    Some(final_response.component_definition),
                                );
                            }
                            AgentMessage::DisconnectNotification(notification) => {
                                self.allow_reconnect = notification.allow_reconnect;
                                if !notification.allow_reconnect {
                                    log::info!(
                                        "{} reconnect disabled by server: {}",
                                        self.label,
                                        notification.reason
                                    );
                                }
                            }
                            other => passthrough_messages.push(other),
                        }
                    }
                    WsMessage::Ping(data) => {
                        if let Some(sender) = self.sender() {
                            sender.send(WsMessage::Pong(data));
                        }
                    }
                    WsMessage::Pong(_) | WsMessage::Text(_) | WsMessage::Unknown(_) => {}
                },
                WsEvent::Error(e) => {
                    log::warn!("{} web socket error: {e}", self.label);
                    self.schedule_reconnect();
                }
                WsEvent::Closed => {
                    log::warn!("{} web socket was closed", self.label);
                    self.schedule_reconnect();
                }
            }
        }
        Ok(passthrough_messages)
    }

    fn sender(&mut self) -> Option<&mut ewebsock::WsSender> {
        self.sender.as_mut()
    }

    fn schedule_reconnect(&mut self) {
        if !self.allow_reconnect {
            return;
        }
        self.sender = None;
        self.recver = None;
        self.alive = false;
        self.connecting = false;
        if !self.allow_reconnect {
            self.next_reconnect_at = None;
            return;
        }
        if self.next_reconnect_at.is_none() {
            self.next_reconnect_at = Some(Instant::now() + WEBSOCKET_RECONNECT_DELAY);
        }
    }

    fn reconnect_if_needed(&mut self) {
        if !self.allow_reconnect {
            return;
        }
        if self.alive || self.connecting {
            return;
        }
        let Some(next_reconnect_at) = self.next_reconnect_at else {
            return;
        };
        if Instant::now() < next_reconnect_at {
            return;
        }

        match connect_socket(&self.url) {
            Ok((sender, recver)) => {
                self.sender = Some(sender);
                self.recver = Some(recver);
                // Preserve immediate-send behavior across reconnect attempts as well.
                self.alive = true;
                self.connecting = true;
                self.next_reconnect_at = None;
                log::info!("{} reconnecting to {}", self.label, self.url);
            }
            Err(err) => {
                log::warn!("{} reconnect failed: {err}", self.label);
                self.next_reconnect_at = Some(Instant::now() + WEBSOCKET_RECONNECT_DELAY);
            }
        }
    }

    pub fn shutdown_permanently(&mut self) {
        self.allow_reconnect = false;
        if let Some(sender) = self.sender.as_mut() {
            let _ = sender.close();
        }
        self.sender = None;
        self.recver = None;
        self.alive = false;
        self.connecting = false;
        self.next_reconnect_at = None;
    }
}

fn build_socket_url(addr: &str, versioning_prefix: Option<&str>) -> Result<String> {
    let mut url = Url::parse(addr).map_err(|e| anyhow!("Invalid URL: {}", e))?;

    let ws_scheme = match url.scheme() {
        "http" => "ws",
        "https" => "wss",
        "ws" | "wss" => url.scheme(),
        _ => return Err(anyhow!("Unsupported URL scheme: {}", url.scheme())),
    }
    .to_owned();

    url.set_scheme(&ws_scheme)
        .map_err(|_| anyhow!("Failed to set URL scheme"))?;

    let versioning_prefix = versioning_prefix.unwrap_or("");
    let new_path = format!("{}/ws", versioning_prefix);
    url.set_path(&new_path);

    Ok(url.to_string())
}

fn connect_socket(url: &str) -> Result<(ewebsock::WsSender, ewebsock::WsReceiver)> {
    ewebsock::connect_with_wakeup(
        url.to_owned(),
        ewebsock::Options::default(),
        wake_designtime_loop,
    )
    .map_err(|err| anyhow!("Couldn't create socket connection: {err}"))
}

#[cfg(target_arch = "wasm32")]
fn wake_designtime_loop() {
    if let Some(window) = web_sys::window() {
        if let Ok(event) = web_sys::Event::new("pax-designtime-wakeup") {
            let _ = window.dispatch_event(&event);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn wake_designtime_loop() {}

#[cfg(test)]
mod tests {
    use super::{build_socket_url, WebSocketConnection};
    use crate::{
        messages::{AgentMessage, LoadManifestResponse},
        orm::PaxManifestORM,
    };
    use ewebsock::{WsEvent, WsMessage};
    use pax_manifest::{PaxManifest, TypeId};
    use std::collections::{BTreeMap, HashMap};

    fn empty_manifest() -> PaxManifest {
        PaxManifest {
            components: BTreeMap::new(),
            main_component_type_id: TypeId::build_singleton("TestComponent", Some("TestComponent")),
            type_table: HashMap::new(),
            assets_dirs: vec![],
            engine_import_path: String::new(),
        }
    }

    fn test_connection(recver: ewebsock::WsReceiver) -> WebSocketConnection {
        WebSocketConnection {
            url: "ws://127.0.0.1:1/ws".to_string(),
            sender: None,
            recver: Some(recver),
            label: "test".to_string(),
            alive: true,
            allow_reconnect: true,
            connecting: false,
            next_reconnect_at: None,
        }
    }

    #[test]
    fn builds_websocket_url_from_http_origin() {
        assert_eq!(
            build_socket_url("http://127.0.0.1:8080", None).unwrap(),
            "ws://127.0.0.1:8080/ws"
        );
    }

    #[test]
    fn preserves_version_prefix_when_building_websocket_url() {
        assert_eq!(
            build_socket_url("https://pub.pax.dev", Some("/v0")).unwrap(),
            "wss://pub.pax.dev/v0/ws"
        );
    }

    #[test]
    fn ignores_invalid_websocket_messages() {
        let (recver, on_event) = ewebsock::WsReceiver::new();
        let _ = on_event(WsEvent::Message(WsMessage::Binary(vec![0xc1])));
        let mut connection = test_connection(recver);
        let mut orm = PaxManifestORM::new(empty_manifest());

        let messages = connection.handle_recv(&mut orm).unwrap();

        assert!(messages.is_empty());
        assert!(connection.alive);
    }

    #[test]
    fn ignores_invalid_manifest_payloads() {
        let (recver, on_event) = ewebsock::WsReceiver::new();
        let message = AgentMessage::LoadManifestResponse(LoadManifestResponse {
            manifest: vec![0xc1],
        });
        let _ = on_event(WsEvent::Message(WsMessage::Binary(
            rmp_serde::to_vec(&message).unwrap(),
        )));
        let mut connection = test_connection(recver);
        let mut orm = PaxManifestORM::new(empty_manifest());

        let messages = connection.handle_recv(&mut orm).unwrap();

        assert!(messages.is_empty());
        assert!(connection.alive);
    }

    #[test]
    fn schedules_reconnect_after_close_without_error() {
        let (recver, on_event) = ewebsock::WsReceiver::new();
        let _ = on_event(WsEvent::Closed);
        let mut connection = test_connection(recver);
        let mut orm = PaxManifestORM::new(empty_manifest());

        let messages = connection.handle_recv(&mut orm).unwrap();

        assert!(messages.is_empty());
        assert!(!connection.alive);
        assert!(!connection.connecting);
        assert!(connection.next_reconnect_at.is_some());
    }

    #[test]
    fn schedules_reconnect_after_error_without_error() {
        let (recver, on_event) = ewebsock::WsReceiver::new();
        let _ = on_event(WsEvent::Error(String::new()));
        let mut connection = test_connection(recver);
        let mut orm = PaxManifestORM::new(empty_manifest());

        let messages = connection.handle_recv(&mut orm).unwrap();

        assert!(messages.is_empty());
        assert!(!connection.alive);
        assert!(!connection.connecting);
        assert!(connection.next_reconnect_at.is_some());
    }
}
