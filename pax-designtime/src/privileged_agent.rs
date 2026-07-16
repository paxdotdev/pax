use crate::messages::{
    AgentMessage, ComponentSerializationRequest, DevClientResponse, LLMRequest,
    LoadFileToStaticDirRequest, UserlandSourceUpdateRequest,
};
use anyhow::{anyhow, Result};
use ewebsock::{WsEvent, WsMessage};
use pax_manifest::ComponentDefinition;
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
use url::Url;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

const WEBSOCKET_RECONNECT_INITIAL_DELAY: Duration = Duration::from_millis(500);
const WEBSOCKET_RECONNECT_MAX_DELAY: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub(crate) enum ConnectionEvent {
    Opened,
    Message(AgentMessage),
    Disconnected,
}

pub struct WebSocketConnection {
    url: String,
    sender: Option<ewebsock::WsSender>,
    recver: Option<ewebsock::WsReceiver>,
    label: String,
    pub alive: bool,
    allow_reconnect: bool,
    connecting: bool,
    next_reconnect_at: Option<Instant>,
    reconnect_delay: Duration,
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
            // The browser WebSocket API rejects sends while CONNECTING. Revision messages are
            // retained by RevisionGate and replayed after ConnectionEvent::Opened.
            alive: false,
            allow_reconnect: true,
            connecting: true,
            next_reconnect_at: None,
            reconnect_delay: WEBSOCKET_RECONNECT_INITIAL_DELAY,
        })
    }

    /// Creates an explicitly disconnected transport. Native debug cartridges
    /// use this when no design-server address was supplied instead of probing a
    /// magical localhost endpoint forever.
    pub fn offline(label: impl Into<String>) -> Self {
        Self {
            url: String::new(),
            sender: None,
            recver: None,
            label: label.into(),
            alive: false,
            allow_reconnect: false,
            connecting: false,
            next_reconnect_at: None,
            reconnect_delay: WEBSOCKET_RECONNECT_INITIAL_DELAY,
        }
    }

    pub fn send_agent_message(&mut self, message: &AgentMessage) -> Result<()> {
        if !self.alive || self.connecting {
            return Err(anyhow!("design-server socket is not open"));
        }
        let msg_bytes = rmp_serde::to_vec(message)?;
        self.sender()
            .ok_or_else(|| anyhow!("design-server socket is not connected"))?
            .send(ewebsock::WsMessage::Binary(msg_bytes));
        Ok(())
    }

    pub fn send_component_update(&mut self, component: &ComponentDefinition) -> Result<()> {
        let component_bytes = rmp_serde::to_vec(component)?;
        self.send_connected(
            AgentMessage::ComponentSerializationRequest(ComponentSerializationRequest {
                component_bytes,
            }),
            "couldn't send component update: connection to design-server was lost",
        )
    }

    pub fn send_llm_request(&mut self, llm_request: LLMRequest) -> Result<()> {
        self.send_connected(
            AgentMessage::LLMRequest(llm_request),
            "couldn't send LLM request: connection to pub pax was lost",
        )
    }

    pub fn send_file_to_static_dir(&mut self, name: &str, data: Vec<u8>) -> Result<()> {
        self.send_connected(
            AgentMessage::LoadFileToStaticDirRequest(LoadFileToStaticDirRequest {
                name: name.to_owned(),
                data,
            }),
            "couldn't send file: connection to design-server was lost",
        )
    }

    pub fn send_dev_client_response(&mut self, response: DevClientResponse) -> Result<()> {
        self.send_connected(
            AgentMessage::DevClientResponse(response),
            "couldn't send dev response: connection to design-server was lost",
        )
    }

    pub fn send_userland_source_update_request(
        &mut self,
        request: UserlandSourceUpdateRequest,
    ) -> Result<()> {
        self.send_connected(
            AgentMessage::UserlandSourceUpdateRequest(request),
            "couldn't send userland source update: connection to design-server was lost",
        )
    }

    fn send_connected(&mut self, message: AgentMessage, disconnected_error: &str) -> Result<()> {
        if !self.alive {
            return Err(anyhow!(disconnected_error.to_owned()));
        }
        self.send_agent_message(&message)
    }

    pub(crate) fn handle_recv(&mut self) -> Result<Vec<ConnectionEvent>> {
        self.reconnect_if_needed();

        let mut connection_events = vec![];
        while let Some(event) = self.recver.as_ref().and_then(|recver| recver.try_recv()) {
            match event {
                WsEvent::Opened => {
                    self.alive = true;
                    self.connecting = false;
                    self.next_reconnect_at = None;
                    self.reconnect_delay = WEBSOCKET_RECONNECT_INITIAL_DELAY;
                    connection_events.push(ConnectionEvent::Opened);
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
                        if let AgentMessage::DisconnectNotification(notification) = &msg {
                            self.allow_reconnect = notification.allow_reconnect;
                            if !notification.allow_reconnect {
                                log::info!(
                                    "{} reconnect disabled by server: {}",
                                    self.label,
                                    notification.reason
                                );
                            }
                        }
                        connection_events.push(ConnectionEvent::Message(msg));
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
                    connection_events.push(ConnectionEvent::Disconnected);
                }
                WsEvent::Closed => {
                    log::warn!("{} web socket was closed", self.label);
                    self.schedule_reconnect();
                    connection_events.push(ConnectionEvent::Disconnected);
                }
            }
        }
        Ok(connection_events)
    }

    fn sender(&mut self) -> Option<&mut ewebsock::WsSender> {
        self.sender.as_mut()
    }

    fn schedule_reconnect(&mut self) {
        self.sender = None;
        self.recver = None;
        self.alive = false;
        self.connecting = false;
        if !self.allow_reconnect {
            self.next_reconnect_at = None;
            return;
        }
        if self.next_reconnect_at.is_none() {
            self.next_reconnect_at = Some(Instant::now() + self.reconnect_delay);
            self.reconnect_delay = self
                .reconnect_delay
                .saturating_mul(2)
                .min(WEBSOCKET_RECONNECT_MAX_DELAY);
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

        self.next_reconnect_at = None;
        match connect_socket(&self.url) {
            Ok((sender, recver)) => {
                self.sender = Some(sender);
                self.recver = Some(recver);
                self.alive = false;
                self.connecting = true;
                self.next_reconnect_at = None;
                log::info!("{} reconnecting to {}", self.label, self.url);
            }
            Err(err) => {
                log::warn!("{} reconnect failed: {err}", self.label);
                self.schedule_reconnect();
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
    use super::{
        build_socket_url, ConnectionEvent, WebSocketConnection, WEBSOCKET_RECONNECT_INITIAL_DELAY,
        WEBSOCKET_RECONNECT_MAX_DELAY,
    };
    use crate::messages::{AgentMessage, FileChangedNotification};
    use ewebsock::{WsEvent, WsMessage};

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
            reconnect_delay: WEBSOCKET_RECONNECT_INITIAL_DELAY,
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
        let messages = connection.handle_recv().unwrap();

        assert!(messages.is_empty());
        assert!(connection.alive);
    }

    #[test]
    fn forwards_decoded_messages_without_applying_policy() {
        let (recver, on_event) = ewebsock::WsReceiver::new();
        let message = AgentMessage::ProjectFileChangedNotification(FileChangedNotification {});
        let _ = on_event(WsEvent::Message(WsMessage::Binary(
            rmp_serde::to_vec(&message).unwrap(),
        )));
        let mut connection = test_connection(recver);

        let messages = connection.handle_recv().unwrap();

        assert!(matches!(
            messages.as_slice(),
            [ConnectionEvent::Message(
                AgentMessage::ProjectFileChangedNotification(_)
            )]
        ));
    }

    #[test]
    fn schedules_reconnect_after_close_without_error() {
        let (recver, on_event) = ewebsock::WsReceiver::new();
        let _ = on_event(WsEvent::Closed);
        let mut connection = test_connection(recver);

        let messages = connection.handle_recv().unwrap();

        assert!(matches!(
            messages.as_slice(),
            [ConnectionEvent::Disconnected]
        ));
        assert!(!connection.alive);
        assert!(!connection.connecting);
        assert!(connection.next_reconnect_at.is_some());
    }

    #[test]
    fn schedules_reconnect_after_error_without_error() {
        let (recver, on_event) = ewebsock::WsReceiver::new();
        let _ = on_event(WsEvent::Error(String::new()));
        let mut connection = test_connection(recver);

        let messages = connection.handle_recv().unwrap();

        assert!(matches!(
            messages.as_slice(),
            [ConnectionEvent::Disconnected]
        ));
        assert!(!connection.alive);
        assert!(!connection.connecting);
        assert!(connection.next_reconnect_at.is_some());
    }

    #[test]
    fn reconnect_attempts_use_capped_backoff() {
        let (recver, _on_event) = ewebsock::WsReceiver::new();
        let mut connection = test_connection(recver);

        connection.schedule_reconnect();
        assert_eq!(
            connection.reconnect_delay,
            WEBSOCKET_RECONNECT_INITIAL_DELAY.saturating_mul(2)
        );

        for _ in 0..8 {
            connection.next_reconnect_at = None;
            connection.schedule_reconnect();
        }
        assert_eq!(connection.reconnect_delay, WEBSOCKET_RECONNECT_MAX_DELAY);
    }

    #[test]
    fn offline_transport_never_reconnects() {
        let mut connection = WebSocketConnection::offline("test-offline");

        assert!(connection.handle_recv().unwrap().is_empty());
        assert!(!connection.alive);
        assert!(!connection.allow_reconnect);
        assert!(connection.next_reconnect_at.is_none());
    }

    #[test]
    fn connecting_transport_rejects_sends_until_opened() {
        let (recver, on_event) = ewebsock::WsReceiver::new();
        let mut connection = test_connection(recver);
        connection.alive = false;
        connection.connecting = true;

        let err = connection
            .send_agent_message(&AgentMessage::ProjectFileChangedNotification(
                FileChangedNotification {},
            ))
            .unwrap_err();
        assert_eq!(err.to_string(), "design-server socket is not open");

        let _ = on_event(WsEvent::Opened);
        let events = connection.handle_recv().unwrap();
        assert!(matches!(events.as_slice(), [ConnectionEvent::Opened]));
        assert!(connection.alive);
        assert!(!connection.connecting);
    }
}
