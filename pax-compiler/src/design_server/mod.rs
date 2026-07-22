use actix::Addr;
use actix_files::Files;
use actix_web::middleware::Logger;

use actix_web::dev::{fn_service, ServiceRequest, ServiceResponse};
use actix_web::http::{header, Method};
use actix_web::web::Data;
use actix_web::{get, web, App, HttpRequest, HttpServer, Responder};
use actix_web::{HttpResponse, Result};
use actix_web_actors::ws;
use colored::Colorize;
use std::fs;
use std::net::{IpAddr, TcpListener, UdpSocket};

use env_logger;
use std::io::Write;

use crate::building::apple::rebuild_staged_macos_logic_dylib;
use crate::building::web::rebuild_staged_web_cartridge;
use crate::dev_session::{
    self, write_session_request_json, DevLookRequest, DevReloadLogicRequest,
    DevReloadLogicResponse, DevSession,
};
use crate::helpers::PAX_BADGE;
use crate::{HotReloadMode, RunContext, RunTarget};
use notify::{Error, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use pax_designtime::messages::{
    AgentMessage, DebugArtifact, DebugLogicExecutionMode, PrepareAppRevision, RevisionStamp,
    UpdateTemplateRequest,
};
use pax_manifest::PaxManifest;

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use websocket::PrivilegedAgentWebSocket;

#[allow(unused)]
mod llm;
mod revision;
pub mod static_server;
pub mod websocket;

pub use revision::{ActivationChannel, DebugRestartSnapshot};
use revision::{ActivationOutcome, DebugRevisionCoordinator};

pub(crate) const DEFAULT_BIND_HOST: &str = "0.0.0.0";
const LOOPBACK_DISPLAY_HOST: &str = "127.0.0.1";

pub(crate) fn display_addresses(port: u16) -> Vec<String> {
    let mut addresses = vec![format!("http://{LOOPBACK_DISPLAY_HOST}:{port}")];
    if let Some(ip) = local_network_ip() {
        addresses.push(format!("http://{ip}:{port}"));
    }
    addresses
}

pub(crate) fn display_address_links(port: u16) -> String {
    display_addresses(port)
        .into_iter()
        .map(|address| address.blue().to_string())
        .collect::<Vec<_>>()
        .join(" or ")
}

pub(crate) fn local_network_ip() -> Option<IpAddr> {
    let socket = UdpSocket::bind((DEFAULT_BIND_HOST, 0)).ok()?;
    socket.connect(("8.8.8.8", 80)).ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_loopback()).then_some(ip)
}

fn static_files_service(fs_path: PathBuf) -> Files {
    let index_path = fs_path.join("index.html");
    Files::new("/*", fs_path)
        .index_file("index.html")
        .default_handler(fn_service(move |req: ServiceRequest| {
            let index_path = index_path.clone();
            async move { history_api_fallback(req, index_path).await }
        }))
}

async fn history_api_fallback(
    req: ServiceRequest,
    index_path: PathBuf,
) -> Result<ServiceResponse, actix_web::Error> {
    if should_serve_history_api_fallback(&req) {
        let (req, _) = req.into_parts();
        let html = tokio::fs::read_to_string(index_path).await?;
        let response = HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(inject_base_href(&html, "/"));
        Ok(ServiceResponse::new(req, response))
    } else {
        Ok(req.into_response(HttpResponse::NotFound().finish()))
    }
}

fn should_serve_history_api_fallback(req: &ServiceRequest) -> bool {
    matches!(req.method(), &Method::GET | &Method::HEAD)
        && request_accepts_html(req)
        && !request_targets_static_asset(req.path())
}

fn request_accepts_html(req: &ServiceRequest) -> bool {
    req.headers()
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.contains("text/html") || value.contains("application/xhtml+xml"))
        .unwrap_or(false)
}

fn request_targets_static_asset(path: &str) -> bool {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(|segment| segment.contains('.'))
        .unwrap_or(false)
}

fn inject_base_href(index_html: &str, href: &str) -> String {
    if index_html.contains("<base ") {
        return index_html.to_string();
    }

    let base_tag = format!(r#"<base href="{href}">"#);
    if let Some(head_end) = index_html.find("</head>") {
        let mut output = String::with_capacity(index_html.len() + base_tag.len() + 9);
        output.push_str(&index_html[..head_end]);
        output.push_str("        ");
        output.push_str(&base_tag);
        output.push('\n');
        output.push_str(&index_html[head_end..]);
        output
    } else {
        format!("{base_tag}\n{index_html}")
    }
}

#[derive(Clone)]
pub struct ActiveWebsocketClient {
    pub connection_id: usize,
    pub addr: Addr<PrivilegedAgentWebSocket>,
}

#[derive(Clone)]
pub struct NativeLogicReloadConfig {
    pub session_dir: PathBuf,
    pub manifest_path: PathBuf,
    pub should_run_designer: bool,
}

#[derive(Clone)]
pub struct WebLogicReloadConfig {
    pub serve_dir: PathBuf,
    pub should_run_designer: bool,
}

#[derive(Clone)]
pub enum LogicReloadConfig {
    Native(NativeLogicReloadConfig),
    Web(WebLogicReloadConfig),
}

struct LogicReloadState {
    config: LogicReloadConfig,
    build_in_progress: bool,
    build_started: bool,
    rebuild_pending: bool,
}

#[derive(Default)]
struct WatcherState {
    expected_writes: HashMap<PathBuf, String>,
    observed_contents: HashMap<PathBuf, String>,
}

pub struct AppState {
    serve_dir: Mutex<PathBuf>,
    userland_project_root: Mutex<PathBuf>,
    active_websocket_client: Mutex<Option<ActiveWebsocketClient>>,
    websocket_client_counter: Mutex<usize>,
    request_id_counter: Mutex<usize>,
    in_flight_dev_requests: Mutex<HashSet<String>>,
    revisions: Mutex<DebugRevisionCoordinator>,
    watcher_state: Mutex<WatcherState>,
    dev_session: Mutex<Option<DevSession>>,
    pending_dev_look_requests: Mutex<HashMap<String, DevLookRequest>>,
    hot_reload: HotReloadMode,
    logic_reload: Mutex<Option<LogicReloadState>>,
    pax_restart_notice_emitted: Mutex<bool>,
    logic_restart_notice_emitted: Mutex<bool>,
    source_update_lock: Mutex<()>,
    restart_manifest_path: Option<PathBuf>,
    startup_recovery_rebuild: Mutex<bool>,
}

impl AppState {
    pub fn new_empty() -> Self {
        Self {
            serve_dir: Mutex::new(PathBuf::new()),
            userland_project_root: Mutex::new(PathBuf::new()),
            active_websocket_client: Mutex::new(None),
            websocket_client_counter: Mutex::new(0),
            request_id_counter: Mutex::new(0),
            in_flight_dev_requests: Mutex::new(HashSet::new()),
            revisions: Mutex::new(DebugRevisionCoordinator::empty()),
            watcher_state: Mutex::new(WatcherState::default()),
            dev_session: Mutex::new(None),
            pending_dev_look_requests: Mutex::new(HashMap::new()),
            hot_reload: HotReloadMode::All,
            logic_reload: Mutex::new(None),
            pax_restart_notice_emitted: Mutex::new(false),
            logic_restart_notice_emitted: Mutex::new(false),
            source_update_lock: Mutex::new(()),
            restart_manifest_path: None,
            startup_recovery_rebuild: Mutex::new(false),
        }
    }
    pub fn new(
        serve_dir: PathBuf,
        project_root: PathBuf,
        manifest: PaxManifest,
        dev_session: Option<DevSession>,
        logic_reload: Option<LogicReloadConfig>,
        hot_reload: HotReloadMode,
        restart_snapshot: Option<DebugRestartSnapshot>,
        restart_manifest_path: Option<PathBuf>,
    ) -> Result<Self, String> {
        let restored_snapshot = restart_snapshot.is_some();
        let restored_web_artifact = restart_snapshot.as_ref().is_some_and(|snapshot| {
            snapshot
                .active_artifact
                .as_ref()
                .is_some_and(|artifact| artifact.channel == ActivationChannel::WebSocket)
        });
        let logic_reload = logic_reload.or_else(|| {
            restored_web_artifact.then(|| {
                LogicReloadConfig::Web(WebLogicReloadConfig {
                    serve_dir: serve_dir.clone(),
                    should_run_designer: false,
                })
            })
        });
        let revisions = if let Some(snapshot) = restart_snapshot {
            DebugRevisionCoordinator::from_restart_snapshot(snapshot)
        } else {
            DebugRevisionCoordinator::new(
                format!("server-start-{}", dev_session::now_ms()),
                manifest,
            )
        };
        let startup_recovery_rebuild =
            restored_snapshot && hot_reload.reloads_logic() && logic_reload.is_some();
        let state = AppState {
            serve_dir: Mutex::new(serve_dir),
            userland_project_root: Mutex::new(project_root),
            active_websocket_client: Mutex::new(None),
            websocket_client_counter: Mutex::new(0),
            request_id_counter: Mutex::new(0),
            in_flight_dev_requests: Mutex::new(HashSet::new()),
            revisions: Mutex::new(revisions),
            watcher_state: Mutex::new(WatcherState::default()),
            dev_session: Mutex::new(dev_session),
            pending_dev_look_requests: Mutex::new(HashMap::new()),
            hot_reload,
            logic_reload: Mutex::new(logic_reload.map(|config| LogicReloadState {
                config,
                build_in_progress: false,
                build_started: false,
                rebuild_pending: false,
            })),
            pax_restart_notice_emitted: Mutex::new(false),
            logic_restart_notice_emitted: Mutex::new(false),
            source_update_lock: Mutex::new(()),
            restart_manifest_path,
            startup_recovery_rebuild: Mutex::new(startup_recovery_rebuild),
        };
        if restored_snapshot && hot_reload.reloads_pax() {
            state.reconcile_restored_pax_sources();
        } else {
            let revisions = state.revisions.lock().unwrap();
            state.persist_active_restart_snapshot_locked(&revisions, "server startup")?;
        }
        Ok(state)
    }

    fn generate_request_id(&self) -> usize {
        let mut counter = self.request_id_counter.lock().unwrap();
        *counter += 1;
        *counter
    }

    fn generate_websocket_client_id(&self) -> usize {
        let mut counter = self.websocket_client_counter.lock().unwrap();
        *counter += 1;
        *counter
    }

    pub(crate) fn expect_watcher_write(&self, path: &Path, contents: &str) {
        self.watcher_state
            .lock()
            .unwrap()
            .expected_writes
            .insert(normalized_watcher_path(path), contents.to_string());
    }

    pub(crate) fn cancel_expected_watcher_write(&self, path: &Path) {
        self.watcher_state
            .lock()
            .unwrap()
            .expected_writes
            .remove(&normalized_watcher_path(path));
    }

    fn should_process_watcher_contents(&self, path: &Path, contents: &str) -> bool {
        let path = normalized_watcher_path(path);
        let mut watcher = self.watcher_state.lock().unwrap();

        if watcher
            .expected_writes
            .get(&path)
            .is_some_and(|expected| expected == contents)
        {
            watcher.expected_writes.remove(&path);
            watcher.observed_contents.insert(path, contents.to_string());
            return false;
        }
        watcher.expected_writes.remove(&path);

        if watcher
            .observed_contents
            .get(&path)
            .is_some_and(|observed| observed == contents)
        {
            return false;
        }
        watcher.observed_contents.insert(path, contents.to_string());
        true
    }

    fn take_startup_recovery_rebuild(&self) -> bool {
        std::mem::take(&mut *self.startup_recovery_rebuild.lock().unwrap())
    }

    fn pax_hot_reload_enabled(&self) -> bool {
        self.hot_reload.reloads_pax()
    }

    fn logic_hot_reload_available(&self) -> bool {
        self.hot_reload.reloads_logic() && self.logic_reload.lock().unwrap().is_some()
    }

    fn note_pax_restart_required(&self) {
        let mut emitted = self.pax_restart_notice_emitted.lock().unwrap();
        if std::mem::replace(&mut *emitted, true) {
            return;
        }
        let next_activation = if self.hot_reload.reloads_logic() {
            "the next logic reload or app restart"
        } else {
            "the next app restart"
        };
        eprintln!(
            "Pax hot reload is disabled; saved `.pax` changes will apply on {next_activation}."
        );
    }

    fn note_logic_restart_required(&self) {
        let mut emitted = self.logic_restart_notice_emitted.lock().unwrap();
        if std::mem::replace(&mut *emitted, true) {
            return;
        }
        if self.hot_reload.reloads_logic() {
            eprintln!(
                "Logic hot reload is unavailable for this target; saved logic changes require rebuilding and restarting the app."
            );
        } else {
            eprintln!(
                "Logic hot reload is disabled; saved logic changes will apply after rebuilding and restarting the app."
            );
        }
    }

    fn websocket_client_is_active(&self, connection_id: usize) -> bool {
        self.active_websocket_client
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|active| active.connection_id == connection_id)
    }

    fn source_update_guard_for_client(
        &self,
        connection_id: usize,
    ) -> Option<std::sync::MutexGuard<'_, ()>> {
        self.source_update_guard_for_client_after(connection_id, || {})
    }

    fn source_update_guard_for_client_after<F>(
        &self,
        connection_id: usize,
        after_initial_check: F,
    ) -> Option<std::sync::MutexGuard<'_, ()>>
    where
        F: FnOnce(),
    {
        if !self.websocket_client_is_active(connection_id) {
            return None;
        }
        after_initial_check();
        let source_update = self.source_update_lock.lock().unwrap();
        if !self.websocket_client_is_active(connection_id) {
            return None;
        }
        Some(source_update)
    }

    fn reconcile_and_promote_websocket_client(
        &self,
        connection_id: usize,
        addr: Addr<PrivilegedAgentWebSocket>,
        client_stamp: Option<&RevisionStamp>,
    ) -> (bool, Option<ActiveWebsocketClient>) {
        // Canonical mutation/route order: source barrier -> revisions -> active
        // websocket. A superseding runtime therefore cannot slip between a
        // mutation handler's post-barrier ownership check and its disk commit.
        let _source_update = self.source_update_lock.lock().unwrap();
        let mut revisions = self.revisions.lock().unwrap();
        let mut active_client = self.active_websocket_client.lock().unwrap();
        let adoption_was_allowed = revisions.survivor_adoption_allowed();
        let mut next_revisions = revisions.clone();
        let client_revision = client_stamp.map(|revision| revision.logic_revision_id.as_str());
        let adopted_surviving_revision = active_client.is_none()
            && client_stamp
                .is_some_and(|stamp| next_revisions.adopt_surviving_client_revision(stamp));
        let eligible = next_revisions.client_matches_active(client_revision)
            || adopted_surviving_revision
            || (client_stamp.is_none()
                && !next_revisions.has_candidate()
                && next_revisions
                    .prepare_for_web_client(client_revision)
                    .is_none());
        if !eligible || !next_revisions.claim_active_connection(connection_id) {
            return (false, None);
        }
        if adoption_was_allowed {
            if let Err(err) = self.persist_active_restart_snapshot_locked(
                &next_revisions,
                "surviving runtime adoption",
            ) {
                *revisions = next_revisions;
                self.invalidate_restart_snapshot_after_live_install(
                    "surviving runtime adoption",
                    &err,
                );
            } else {
                *revisions = next_revisions;
            }
        } else {
            *revisions = next_revisions;
        }
        if active_client
            .as_ref()
            .is_some_and(|active| active.connection_id == connection_id)
        {
            return (true, None);
        }
        (
            true,
            active_client.replace(ActiveWebsocketClient {
                connection_id,
                addr,
            }),
        )
    }

    fn prepare_for_web_client(
        &self,
        client_logic_revision_id: Option<&str>,
    ) -> Option<PrepareAppRevision> {
        self.revisions
            .lock()
            .unwrap()
            .prepare_for_web_client(client_logic_revision_id)
    }

    fn active_manifest_response(
        &self,
    ) -> Result<Option<pax_designtime::messages::LoadManifestResponse>, String> {
        self.revisions.lock().unwrap().active_snapshot()
    }

    /// Persist the proposed coordinator state while the caller owns the live
    /// revision lock. Logic activation treats this snapshot as its commit
    /// record; authoring mutations remain live on failure and invalidate the
    /// stale record instead.
    fn persist_active_restart_snapshot_locked(
        &self,
        revisions: &DebugRevisionCoordinator,
        action: &str,
    ) -> Result<(), String> {
        let Some(manifest_path) = &self.restart_manifest_path else {
            return Ok(());
        };
        let Some(snapshot) = revisions.restart_snapshot() else {
            return Ok(());
        };
        if let Err(err) = publish_restart_manifest_with_retry(manifest_path, &snapshot) {
            return Err(format!(
                "{action} restart snapshot could not be published: {err}"
            ));
        }
        Ok(())
    }

    fn invalidate_restart_snapshot_after_live_install(&self, action: &str, error: &str) {
        let Some(manifest_path) = &self.restart_manifest_path else {
            eprintln!("{action} was installed without a restart snapshot: {error}");
            return;
        };
        let removal = match fs::remove_file(manifest_path) {
            Ok(()) => "the stale restart snapshot was removed".to_string(),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                "no stale restart snapshot remained".to_string()
            }
            Err(remove_err) => match fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(manifest_path)
            {
                Ok(file) => {
                    let _ = file.sync_all();
                    "the stale restart snapshot was truncated".to_string()
                }
                Err(truncate_err) => format!(
                    "the stale restart snapshot could not be removed ({remove_err}) or truncated ({truncate_err})"
                ),
            },
        };
        eprintln!(
            "{action} was installed for live coherence even though persistence failed: {error}; {removal}"
        );
    }

    fn reconcile_restored_pax_sources(&self) {
        let _source_update = self.source_update_lock.lock().unwrap();
        let project_root = self.userland_project_root.lock().unwrap().clone();
        let mut revisions = self.revisions.lock().unwrap();
        let mut next_revisions = revisions.clone();
        let source_paths: BTreeSet<String> = next_revisions
            .active_manifest_clone()
            .into_iter()
            .flat_map(|manifest| manifest.components.into_values())
            .filter_map(|component| component.template)
            .filter_map(|template| template.get_file_path())
            .filter(|path| Path::new(path).extension().and_then(|ext| ext.to_str()) == Some("pax"))
            .map(|path| resolve_restored_pax_source_path(&project_root, &path))
            .map(|path| {
                websocket::normalized_pax_mutation_path(&path.to_string_lossy(), &project_root)
            })
            .collect();
        let mut manifest_changed = false;

        for normalized_path in source_paths {
            let disk_path = PathBuf::from(&normalized_path);
            let content = match fs::read_to_string(&disk_path) {
                Ok(content) => content,
                Err(err) => {
                    eprintln!(
                        "could not reconcile restored Pax source {}: {err}",
                        disk_path.display()
                    );
                    continue;
                }
            };
            let mutation_generation =
                next_revisions.record_pax_source(normalized_path.clone(), content.clone());
            let Some((expected_revision, active_manifest)) =
                next_revisions.active_manifest_snapshot()
            else {
                break;
            };
            let update = match websocket::parse_pax_source_update(
                &active_manifest,
                &disk_path.to_string_lossy(),
                &content,
                &project_root,
            ) {
                Ok(update) => update,
                Err(err) => {
                    eprintln!(
                        "could not parse restored Pax source {}: {err}",
                        disk_path.display()
                    );
                    continue;
                }
            };
            match next_revisions.commit_template_update(
                &expected_revision,
                Some((&normalized_path, mutation_generation)),
                update,
            ) {
                Ok(committed) => {
                    manifest_changed |= committed.revision != expected_revision;
                }
                Err(err) => eprintln!(
                    "restored Pax source {} needs the recovery logic rebuild: {err}",
                    disk_path.display()
                ),
            }
        }

        if manifest_changed {
            let persistence = self.persist_active_restart_snapshot_locked(
                &next_revisions,
                "restored Pax source reconciliation",
            );
            *revisions = next_revisions;
            if let Err(err) = persistence {
                self.invalidate_restart_snapshot_after_live_install(
                    "restored Pax source reconciliation",
                    &err,
                );
            }
        } else {
            // The source journal is still useful to the unconditional recovery
            // build even when the persisted active manifest needed no changes.
            *revisions = next_revisions;
        }
    }

    fn install_live_revision_mutation<F>(&self, action: &str, mutation: F) -> Result<(), String>
    where
        F: FnOnce(&mut DebugRevisionCoordinator) -> Result<(), String>,
    {
        let mut revisions = self.revisions.lock().unwrap();
        let mut next_revisions = revisions.clone();
        mutation(&mut next_revisions)?;
        let persistence = self.persist_active_restart_snapshot_locked(&next_revisions, action);
        *revisions = next_revisions;
        if let Err(err) = persistence {
            self.invalidate_restart_snapshot_after_live_install(action, &err);
        }
        Ok(())
    }

    fn apply_pax_source_mutation<F>(&self, action: &str, mutation: F) -> Result<(), String>
    where
        F: FnOnce(&mut DebugRevisionCoordinator) -> Result<(), String>,
    {
        if self.pax_hot_reload_enabled() {
            self.install_live_revision_mutation(action, mutation)
        } else {
            let mut scratch = self.revisions.lock().unwrap().clone();
            mutation(&mut scratch)?;
            self.note_pax_restart_required();
            Ok(())
        }
    }

    fn commit_template_update_and_route(
        &self,
        expected_revision: &RevisionStamp,
        source_mutation: Option<(&str, u64)>,
        update: UpdateTemplateRequest,
    ) -> Result<UpdateTemplateRequest, String> {
        if !self.pax_hot_reload_enabled() {
            return Err("Pax hot reload is disabled for this session".to_string());
        }
        let mut revisions = self.revisions.lock().unwrap();
        let mut next_revisions = revisions.clone();
        let committed =
            next_revisions.commit_template_update(expected_revision, source_mutation, update)?;
        let persistence =
            self.persist_active_restart_snapshot_locked(&next_revisions, "Pax template update");
        *revisions = next_revisions;
        if let Err(err) = persistence {
            self.invalidate_restart_snapshot_after_live_install("Pax template update", &err);
        }
        let active_addr = self
            .active_websocket_client
            .lock()
            .unwrap()
            .as_ref()
            .map(|active| active.addr.clone());
        drop(revisions);

        if let Some(addr) = active_addr {
            addr.do_send(websocket::SendAgentMessage {
                message: AgentMessage::UpdateTemplateRequest(Box::new(committed.clone())),
            });
        }
        Ok(committed)
    }

    #[cfg(test)]
    fn activate_logic_revision(
        &self,
        logic_revision_id: &str,
        activation_owner: usize,
    ) -> Result<ActivationOutcome, String> {
        let mut revisions = self.revisions.lock().unwrap();
        let mut next_revisions = revisions.clone();
        let outcome =
            next_revisions.activate_logic_revision(logic_revision_id, activation_owner)?;
        if let Err(err) =
            self.persist_active_restart_snapshot_locked(&next_revisions, "logic activation")
        {
            if !outcome.already_active {
                return Err(format!("logic activation was not committed: {err}"));
            }
            eprintln!(
                "duplicate logic activation remains committed despite republish failure: {err}"
            );
        }
        *revisions = next_revisions;
        Ok(outcome)
    }

    fn activate_logic_revision_and_promote(
        &self,
        logic_revision_id: &str,
        connection_id: usize,
        addr: Addr<PrivilegedAgentWebSocket>,
    ) -> Result<(ActivationOutcome, bool, Option<ActiveWebsocketClient>), String> {
        let _source_update = self.source_update_lock.lock().unwrap();
        let mut revisions = self.revisions.lock().unwrap();
        let mut next_revisions = revisions.clone();
        let outcome = next_revisions.activate_logic_revision(logic_revision_id, connection_id)?;
        if let Err(err) =
            self.persist_active_restart_snapshot_locked(&next_revisions, "logic activation")
        {
            if !outcome.already_active {
                return Err(format!("logic activation was not committed: {err}"));
            }
            eprintln!(
                "duplicate logic activation remains committed despite republish failure: {err}"
            );
        }
        *revisions = next_revisions;
        let should_promote = outcome.promote_connection
            && revisions.active_connection_matches(logic_revision_id, connection_id);
        if !should_promote {
            return Ok((outcome, false, None));
        }

        let mut active_client = self.active_websocket_client.lock().unwrap();
        if active_client
            .as_ref()
            .is_some_and(|active| active.connection_id == connection_id)
        {
            return Ok((outcome, true, None));
        }
        let previous = active_client.replace(ActiveWebsocketClient {
            connection_id,
            addr,
        });
        Ok((outcome, true, previous))
    }

    fn prepare_logic_activation(
        &self,
        logic_revision_id: &str,
        activation_owner: usize,
    ) -> Result<pax_designtime::messages::LoadManifestResponse, String> {
        let project_root = self.userland_project_root.lock().unwrap().clone();
        self.revisions.lock().unwrap().prepare_logic_activation(
            logic_revision_id,
            activation_owner,
            |manifest, path, contents| {
                websocket::parse_pax_source_update(manifest, path, contents, &project_root)
            },
        )
    }
}

fn publish_restart_manifest_with_retry(
    manifest_path: &Path,
    snapshot: &DebugRestartSnapshot,
) -> Result<(), String> {
    const ATTEMPTS: usize = 3;
    let mut last_error = None;
    for attempt in 1..=ATTEMPTS {
        match publish_restart_manifest(manifest_path, snapshot) {
            Ok(()) => return Ok(()),
            Err(err) => last_error = Some(err),
        }
        if attempt < ATTEMPTS {
            std::thread::sleep(Duration::from_millis(25));
        }
    }
    Err(last_error.unwrap_or_else(|| "restart manifest publication failed".to_string()))
}

fn publish_restart_manifest(
    manifest_path: &Path,
    snapshot: &DebugRestartSnapshot,
) -> Result<(), String> {
    let bytes = serde_json::to_vec(snapshot)
        .map_err(|err| format!("failed to serialize restart manifest: {err}"))?;
    let parent = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let mut pending = tempfile::NamedTempFile::new_in(parent)
        .map_err(|err| format!("failed to stage restart manifest: {err}"))?;
    pending
        .write_all(&bytes)
        .map_err(|err| format!("failed to write staged restart manifest: {err}"))?;
    pending
        .as_file()
        .sync_all()
        .map_err(|err| format!("failed to flush staged restart manifest: {err}"))?;
    pending
        .persist(manifest_path)
        .map_err(|err| format!("failed to publish restart manifest: {}", err.error))?;
    Ok(())
}

fn resolve_restored_pax_source_path(project_root: &Path, source_path: &str) -> PathBuf {
    let source_path = Path::new(source_path);
    if source_path.is_absolute() {
        return source_path.to_path_buf();
    }
    let project_candidate = project_root.join(source_path);
    if project_candidate.exists() {
        return project_candidate;
    }
    let cwd_candidate = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::new())
        .join(source_path);
    if cwd_candidate.exists() {
        cwd_candidate
    } else {
        project_candidate
    }
}

fn normalized_watcher_path(path: &Path) -> PathBuf {
    if let Ok(canonical) = fs::canonicalize(path) {
        return canonical;
    }

    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::new())
            .join(path)
    };
    let Some(parent) = absolute.parent() else {
        return absolute;
    };
    let Some(file_name) = absolute.file_name() else {
        return absolute;
    };
    fs::canonicalize(parent)
        .map(|parent| parent.join(file_name))
        .unwrap_or(absolute)
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RestartManifestFile {
    Snapshot(DebugRestartSnapshot),
    Legacy(PaxManifest),
}

pub fn decode_restart_manifest(
    bytes: &[u8],
) -> Result<(PaxManifest, Option<DebugRestartSnapshot>), serde_json::Error> {
    match serde_json::from_slice(bytes)? {
        RestartManifestFile::Snapshot(snapshot) => Ok((snapshot.manifest.clone(), Some(snapshot))),
        RestartManifestFile::Legacy(manifest) => Ok((manifest, None)),
    }
}

pub fn schedule_logic_reload(state: Data<AppState>) {
    if request_logic_reload(&state) {
        std::thread::spawn(move || run_logic_reload_loop(state));
    }
}

fn request_logic_reload(state: &Data<AppState>) -> bool {
    if !state.logic_hot_reload_available() {
        state.note_logic_restart_required();
        return false;
    }
    let candidate_reserved = state.revisions.lock().unwrap().has_reserved_candidate();
    let mut logic_reload = state.logic_reload.lock().unwrap();
    let Some(reload_state) = logic_reload.as_mut() else {
        return false;
    };

    if candidate_reserved {
        reload_state.rebuild_pending = true;
        return false;
    }

    if reload_state.build_in_progress {
        if reload_state.build_started {
            reload_state.rebuild_pending = true;
        }
        return false;
    }

    reload_state.build_in_progress = true;
    reload_state.build_started = false;
    reload_state.rebuild_pending = false;
    true
}

pub(crate) fn resume_deferred_logic_reload(state: Data<AppState>) {
    if !state.logic_hot_reload_available() {
        return;
    }
    if state.revisions.lock().unwrap().has_reserved_candidate() {
        return;
    }
    let mut logic_reload = state.logic_reload.lock().unwrap();
    let Some(reload_state) = logic_reload.as_mut() else {
        return;
    };
    if !reload_state.rebuild_pending || reload_state.build_in_progress {
        return;
    }
    reload_state.rebuild_pending = false;
    reload_state.build_in_progress = true;
    reload_state.build_started = false;
    drop(logic_reload);
    std::thread::spawn(move || run_logic_reload_loop(state));
}

pub fn run_logic_reload_loop(state: Data<AppState>) {
    run_logic_reload_loop_with(state, Duration::from_millis(150), perform_logic_reload);
}

fn run_logic_reload_loop_with<F>(state: Data<AppState>, debounce: Duration, mut perform_reload: F)
where
    F: FnMut(&Data<AppState>, &PathBuf, &LogicReloadConfig) -> Result<(), color_eyre::eyre::Report>,
{
    loop {
        std::thread::sleep(debounce);

        let config = {
            let mut logic_reload = state.logic_reload.lock().unwrap();
            logic_reload.as_mut().map(|reload| {
                reload.build_started = true;
                reload.config.clone()
            })
        };
        let Some(config) = config else {
            return;
        };
        let project_root = state.userland_project_root.lock().unwrap().clone();

        match perform_reload(&state, &project_root, &config) {
            Ok(()) => {}
            Err(err) => {
                eprintln!("failed to reload logic for hot reload: {err}");
            }
        }

        let should_repeat = {
            let candidate_reserved = state.revisions.lock().unwrap().has_reserved_candidate();
            let mut logic_reload = state.logic_reload.lock().unwrap();
            let Some(reload_state) = logic_reload.as_mut() else {
                return;
            };
            if reload_state.rebuild_pending && !candidate_reserved {
                reload_state.rebuild_pending = false;
                reload_state.build_started = false;
                true
            } else {
                reload_state.build_in_progress = false;
                reload_state.build_started = false;
                false
            }
        };

        if !should_repeat {
            break;
        }
    }
}

pub(crate) fn perform_logic_reload(
    state: &Data<AppState>,
    project_root: &PathBuf,
    config: &LogicReloadConfig,
) -> Result<(), color_eyre::eyre::Report> {
    match config {
        LogicReloadConfig::Native(config) => {
            let build_start_mutation_generation =
                state.revisions.lock().unwrap().mutation_generation();
            let build = rebuild_staged_macos_logic_dylib(
                project_root,
                &config.session_dir,
                config.should_run_designer,
            )?;
            let prepare = PrepareAppRevision {
                logic_revision_id: build.build_id,
                execution_mode: DebugLogicExecutionMode::CompiledArtifact,
                artifact: DebugArtifact {
                    kind: "macos-dylib".to_string(),
                    location: build.dylib_path.to_string_lossy().into_owned(),
                },
            };
            stage_logic_candidate_or_defer(
                state,
                build.manifest,
                prepare.clone(),
                ActivationChannel::NativeSession,
                build_start_mutation_generation,
            )?;
            let request_id =
                match enqueue_native_logic_reload_request(state, config, prepare.clone()) {
                    Ok(request_id) => request_id,
                    Err(err) => {
                        state
                            .revisions
                            .lock()
                            .unwrap()
                            .discard_logic_candidate(&prepare.logic_revision_id);
                        return Err(err);
                    }
                };
            if let Err(failure) =
                wait_for_native_logic_reload(state, config, &request_id, &prepare.logic_revision_id)
            {
                if failure.safe_to_discard_candidate {
                    state
                        .revisions
                        .lock()
                        .unwrap()
                        .discard_logic_candidate(&prepare.logic_revision_id);
                }
                return Err(failure.report);
            }
        }
        LogicReloadConfig::Web(config) => {
            let build_start_mutation_generation =
                state.revisions.lock().unwrap().mutation_generation();
            let build = rebuild_staged_web_cartridge(
                project_root,
                &config.serve_dir,
                config.should_run_designer,
            )?;
            let request = PrepareAppRevision {
                logic_revision_id: build.build_id,
                execution_mode: DebugLogicExecutionMode::CompiledArtifact,
                artifact: DebugArtifact {
                    kind: "web-cartridge".to_string(),
                    location: build.extensionless_url,
                },
            };
            stage_logic_candidate_or_defer(
                state,
                build.manifest,
                request.clone(),
                ActivationChannel::WebSocket,
                build_start_mutation_generation,
            )?;
            send_agent_message_to_active_client(state, AgentMessage::PrepareAppRevision(request));
        }
    }
    Ok(())
}

fn stage_logic_candidate_or_defer(
    state: &Data<AppState>,
    manifest: PaxManifest,
    prepare: PrepareAppRevision,
    channel: ActivationChannel,
    build_start_mutation_generation: u64,
) -> Result<(), color_eyre::eyre::Report> {
    let result = state
        .revisions
        .lock()
        .unwrap()
        .stage_logic_candidate_built_after_generation(
            manifest,
            prepare,
            channel,
            build_start_mutation_generation,
        );
    if let Err(err) = result {
        if state.revisions.lock().unwrap().has_reserved_candidate() {
            if let Some(reload) = state.logic_reload.lock().unwrap().as_mut() {
                reload.rebuild_pending = true;
            }
        }
        return Err(color_eyre::eyre::eyre!(err));
    }
    Ok(())
}

pub(crate) fn send_agent_message_to_active_client(state: &Data<AppState>, message: AgentMessage) {
    let active_client = state.active_websocket_client.lock().unwrap().clone();
    if let Some(active_client) = active_client {
        active_client
            .addr
            .do_send(websocket::SendAgentMessage { message });
    }
}

fn enqueue_native_logic_reload_request(
    state: &AppState,
    config: &NativeLogicReloadConfig,
    revision: PrepareAppRevision,
) -> Result<String, color_eyre::eyre::Report> {
    let request_id = format!("reload-logic-{}", state.generate_request_id());
    let request_file_id = request_id.clone();
    write_session_request_json(
        &config.session_dir,
        &request_file_id,
        &DevReloadLogicRequest {
            request_id,
            kind: "reload-logic".to_string(),
            revision,
        },
    )?;
    Ok(request_file_id)
}

struct NativeLogicReloadFailure {
    report: color_eyre::eyre::Report,
    safe_to_discard_candidate: bool,
}

fn wait_for_native_logic_reload(
    state: &AppState,
    config: &NativeLogicReloadConfig,
    request_id: &str,
    logic_revision_id: &str,
) -> Result<(), NativeLogicReloadFailure> {
    const HOST_RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);
    const ACTIVATION_TIMEOUT: Duration = Duration::from_secs(15);
    const POLL_INTERVAL: Duration = Duration::from_millis(50);

    let response_path = config
        .session_dir
        .join("responses")
        .join(format!("{request_id}.json"));
    let request_path = config
        .session_dir
        .join("requests")
        .join(format!("{request_id}.json"));
    let response_deadline = Instant::now() + HOST_RESPONSE_TIMEOUT;
    let response = loop {
        if response_path.exists() {
            let bytes = fs::read(&response_path).map_err(|err| NativeLogicReloadFailure {
                report: err.into(),
                safe_to_discard_candidate: false,
            })?;
            let response: DevReloadLogicResponse =
                serde_json::from_slice(&bytes).map_err(|err| NativeLogicReloadFailure {
                    report: err.into(),
                    safe_to_discard_candidate: false,
                })?;
            let _ = fs::remove_file(&response_path);
            break response;
        }
        if Instant::now() >= response_deadline && request_path.exists() {
            // Only an unclaimed inbox request has a safe timeout. Once Swift
            // removes the request it may enter irreversible final commit, so
            // keep waiting for its post-swap response without a deadline.
            let safe_to_discard_candidate = true;
            if request_path.exists() {
                let _ = fs::remove_file(&request_path);
            }
            return Err(NativeLogicReloadFailure {
                report: color_eyre::eyre::eyre!(
                    "timed out waiting for the native host to load logic revision {logic_revision_id}"
                ),
                safe_to_discard_candidate,
            });
        }
        std::thread::sleep(POLL_INTERVAL);
    };

    if response.request_id != request_id {
        return Err(NativeLogicReloadFailure {
            report: color_eyre::eyre::eyre!(
                "native host replied to {request_id} with mismatched request id {}",
                response.request_id
            ),
            safe_to_discard_candidate: false,
        });
    }
    if response.logic_revision_id.as_deref() != Some(logic_revision_id) {
        return Err(NativeLogicReloadFailure {
            report: color_eyre::eyre::eyre!(
                "native host replied to {request_id} with mismatched logic revision {:?}",
                response.logic_revision_id
            ),
            safe_to_discard_candidate: false,
        });
    }
    if response.status != "ok" {
        return Err(NativeLogicReloadFailure {
            report: color_eyre::eyre::eyre!(
                "native host rejected logic revision {logic_revision_id}: {}",
                response.error.as_deref().unwrap_or("unknown error")
            ),
            // Swift reports an error only after abandoning the candidate and
            // resuming the previous engine.
            safe_to_discard_candidate: true,
        });
    }

    // The host writes its success response after asking the new engine to
    // activate. Wait for the websocket acknowledgement before permitting a
    // later build to supersede the sole coordinator candidate.
    let activation_deadline = Instant::now() + ACTIVATION_TIMEOUT;
    loop {
        if state
            .revisions
            .lock()
            .unwrap()
            .is_active_logic_revision(logic_revision_id)
        {
            return Ok(());
        }
        if Instant::now() >= activation_deadline {
            return Err(NativeLogicReloadFailure {
                report: color_eyre::eyre::eyre!(
                    "native host loaded logic revision {logic_revision_id}, but its activation acknowledgement did not arrive"
                ),
                safe_to_discard_candidate: false,
            });
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

#[get("/ws")]
pub async fn web_socket(
    req: HttpRequest,
    stream: web::Payload,
    state: web::Data<AppState>,
) -> impl Responder {
    ws::WsResponseBuilder::new(PrivilegedAgentWebSocket::new(state), &req, stream)
        .frame_size(2_000_000)
        .start()
}

#[allow(unused_assignments)]
/// Starts a debug design server with independently selectable Pax and logic
/// hot-reload lanes.
///
/// [`HotReloadMode::Off`] keeps websocket-backed inspection available while
/// preventing source changes from updating the mounted app.
pub fn start_server(
    static_file_path: &str,
    src_folder_to_watch: &str,
    manifest: PaxManifest,
    requested_port: Option<u16>,
    ready_file: Option<PathBuf>,
    show_address_log: bool,
    dev_session: Option<DevSession>,
    logic_reload: Option<LogicReloadConfig>,
    hot_reload: HotReloadMode,
    restart_snapshot: Option<DebugRestartSnapshot>,
    restart_manifest_path: Option<PathBuf>,
) -> std::io::Result<()> {
    // Initialize logging
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::Builder::from_env(env_logger::Env::default())
        .format(|buf, record| writeln!(buf, "{} 🍱 Served {}", *PAX_BADGE, record.args()))
        .init();

    let initial_state = AppState::new(
        PathBuf::from(static_file_path),
        PathBuf::from_str(src_folder_to_watch).unwrap(),
        manifest,
        dev_session,
        logic_reload,
        hot_reload,
        restart_snapshot,
        restart_manifest_path,
    )
    .map_err(std::io::Error::other)?;
    let fs_path = initial_state.serve_dir.lock().unwrap().clone();
    let state = Data::new(initial_state);
    let _watcher = setup_file_watcher(state.clone(), src_folder_to_watch)
        .expect("Failed to setup file watcher");
    if state.take_startup_recovery_rebuild() {
        // Source reconciliation above preserves ABI-compatible edits made while
        // the server was down. One unconditional rebuild recovers Rust edits,
        // incompatible Pax changes, and a crash after a frozen preflight.
        schedule_logic_reload(state.clone());
    }

    // Create a Runtime
    let runtime = actix_web::rt::System::new().block_on(async {
        let listener = TcpListener::bind((DEFAULT_BIND_HOST, requested_port.unwrap_or(0)))?;
        let port = listener.local_addr()?.port();
        if let Some(session) = state.dev_session.lock().unwrap().as_mut() {
            session.design_server_addr = Some(format!("ws://{LOOPBACK_DISPLAY_HOST}:{port}"));
            session.location = Some(format!("http://{LOOPBACK_DISPLAY_HOST}:{port}"));
            session.last_seen_ms = dev_session::now_ms();
        }

        println!(
            "{} 🗂️  Serving static files from {}",
            *PAX_BADGE,
            &fs_path.to_str().unwrap()
        );
        if show_address_log {
            let address_msg = display_address_links(port);
            let server_running_at_msg = format!("Server running at {}", address_msg).bold();
            println!("{} 📠 {}", *PAX_BADGE, server_running_at_msg);
        }

        if let Some(ready_file) = ready_file {
            if let Some(parent) = ready_file.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&ready_file, format!("ws://{LOOPBACK_DISPLAY_HOST}:{port}"))?;
        }

        let server = HttpServer::new(move || {
            App::new()
                .wrap(Logger::new("| %s | %U"))
                .app_data(state.clone())
                .service(web_socket)
                .service(static_files_service(fs_path.clone()))
        })
        .listen(listener)?
        .workers(2);

        server.run().await
    });

    runtime
}

pub enum FileContent {
    Pax(String),
    Rust(String),
}

struct WatcherFileChanged {
    pub contents: FileContent,
    pub path: String,
}

pub fn setup_file_watcher(state: Data<AppState>, path: &str) -> Result<RecommendedWatcher, Error> {
    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, Error>| match res {
            Ok(e) => {
                if !matches!(e.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                    return;
                }
                for path in e.paths {
                    if should_ignore_watcher_path(&path) {
                        continue;
                    }
                    let Some(is_pax) = watcher_source_kind(&path) else {
                        // Non-source project files still invalidate dev-tool views,
                        // but they never enter source parsing or reload scheduling.
                        send_project_file_changed_notification(&state);
                        continue;
                    };
                    let change = {
                        // Server-authored writes own this barrier until their exact
                        // path/content echo has been registered. External writes use
                        // the same barrier as parsing and revision commit.
                        let _source_update = state.source_update_lock.lock().unwrap();
                        let contents = match fs::read_to_string(&path) {
                            Ok(contents) => contents,
                            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
                            Err(err) => {
                                eprintln!("failed to read changed file {}: {err}", path.display());
                                continue;
                            }
                        };
                        if !state.should_process_watcher_contents(&path, &contents) {
                            continue;
                        }
                        WatcherFileChanged {
                            contents: if is_pax {
                                FileContent::Pax(contents)
                            } else {
                                FileContent::Rust(contents)
                            },
                            path: path.to_string_lossy().into_owned(),
                        }
                    };
                    handle_watcher_file_changed(state.clone(), change);
                }
            }
            Err(e) => {
                println!("File system watch error: {:?}", e);
            }
        },
        Default::default(),
    )?;
    watcher.watch(Path::new(path), RecursiveMode::Recursive)?;
    Ok(watcher)
}

fn handle_watcher_file_changed(state: Data<AppState>, change: WatcherFileChanged) {
    let WatcherFileChanged { contents, path } = change;
    println!("File changed: {path:?}");

    match contents {
        FileContent::Pax(content) => {
            if !state.pax_hot_reload_enabled() {
                state.note_pax_restart_required();
                send_project_file_changed_notification(&state);
                return;
            }
            match websocket::apply_pax_source_update(&state, &path, &content) {
                Ok(()) => {}
                Err(err) => {
                    if websocket::update_requires_logic_reload(&err) {
                        schedule_logic_reload(state.clone());
                    }
                    eprintln!("ignoring invalid Pax watcher update for {path}: {err}");
                }
            }
        }
        FileContent::Rust(_) => schedule_logic_reload(state.clone()),
    }

    send_project_file_changed_notification(&state);
}

fn send_project_file_changed_notification(state: &Data<AppState>) {
    send_agent_message_to_active_client(
        state,
        AgentMessage::ProjectFileChangedNotification(
            pax_designtime::messages::FileChangedNotification {},
        ),
    );
}

fn should_ignore_watcher_path(path: &Path) -> bool {
    let ignored_directory = path.components().any(|component| {
        let component = component.as_os_str();
        component == ".pax" || component == "target"
    });
    let transient_file = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with('~') || name.starts_with(".#"));
    ignored_directory || transient_file
}

fn watcher_source_kind(path: &Path) -> Option<bool> {
    if should_ignore_watcher_path(path) {
        return None;
    }
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("pax") => Some(true),
        Some("rs") => Some(false),
        _ => None,
    }
}

#[get("/ai")]
async fn ai_page() -> Result<HttpResponse> {
    let html_content = fs::read_to_string("static/ai_chat.html")?;
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(html_content))
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct AiMessage {
    message: String,
}

#[allow(dead_code)]
fn create_designer_run_context() -> RunContext {
    RunContext {
        target: RunTarget::Web,
        project_path: PathBuf::from("../pax-designer".to_string()),
        verbose: false,
        should_also_run: false,
        is_libdev_mode: true,
        should_run_designtime: true,
        should_run_designer: true,
        hot_reload: None,
        process_child_ids: Arc::new(Mutex::new(vec![])),
        is_release: false,
        profile_wasm_size: false,
        webgl: false,
        ios_device: None,
        ios_development_team: None,
    }
}

#[allow(dead_code)]
fn perform_build() -> std::io::Result<(PaxManifest, Option<PathBuf>)> {
    let ctx = create_designer_run_context();
    crate::perform_build(&ctx).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

#[allow(dead_code)]
fn perform_build_and_update_state(state: &AppState, folder_to_watch: &str) -> std::io::Result<()> {
    let (manifest, fs_path) = perform_build()?;

    // Update the state
    *state.serve_dir.lock().unwrap() = fs_path.expect("serve directory should exist");
    *state.userland_project_root.lock().unwrap() = PathBuf::from_str(folder_to_watch).unwrap();
    let mut revisions = state.revisions.lock().unwrap();
    let mut next_revisions = revisions.clone();
    next_revisions
        .replace_initial_manifest(format!("server-build-{}", dev_session::now_ms()), manifest);
    state
        .persist_active_restart_snapshot_locked(&next_revisions, "full server build")
        .map_err(std::io::Error::other)?;
    *revisions = next_revisions;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::revision::DebugRevisionCoordinator;
    use super::{
        decode_restart_manifest, handle_watcher_file_changed, request_logic_reload,
        run_logic_reload_loop_with, static_files_service, watcher_source_kind, ActivationChannel,
        AppState, FileContent, HotReloadMode, LogicReloadConfig, NativeLogicReloadConfig,
        WatcherFileChanged, WebLogicReloadConfig,
    };
    use actix_web::http::{header, StatusCode};
    use actix_web::{test as actix_test, web::Data, App};
    use pax_designtime::messages::{
        DebugArtifact, DebugLogicExecutionMode, PrepareAppRevision, RevisionStamp,
    };
    use pax_manifest::{
        ComponentDefinition, ComponentTemplate, PaxManifest, SettingsBlockElement, TypeId,
    };
    use std::collections::{BTreeMap, HashMap};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::Duration;
    use tempfile::tempdir;

    fn watcher_manifest() -> PaxManifest {
        let example_type_id = TypeId::build_singleton("crate::Example", Some("Example"));
        let group_type_id = TypeId::build_singleton("Group", Some("Group"));
        let mut components = BTreeMap::new();
        components.insert(
            example_type_id.clone(),
            ComponentDefinition {
                type_id: example_type_id.clone(),
                is_main_component: true,
                is_primitive: false,
                is_struct_only_component: false,
                module_path: "crate".to_string(),
                primitive_instance_import_path: None,
                template: Some(ComponentTemplate::new(
                    example_type_id.clone(),
                    Some("src/lib.pax".to_string()),
                )),
                settings: Some(vec![]),
                timelines: vec![],
                route_branch: None,
            },
        );
        components.insert(
            group_type_id.clone(),
            ComponentDefinition {
                type_id: group_type_id,
                is_main_component: false,
                is_primitive: true,
                is_struct_only_component: false,
                module_path: "pax_kit".to_string(),
                primitive_instance_import_path: None,
                template: None,
                settings: None,
                timelines: vec![],
                route_branch: None,
            },
        );
        PaxManifest {
            components,
            main_component_type_id: example_type_id,
            type_table: HashMap::new(),
            assets_dirs: vec![],
            engine_import_path: "pax_kit".to_string(),
        }
    }

    #[test]
    fn pax_watcher_commits_revision_without_an_active_websocket() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("src/lib.pax");
        fs::create_dir_all(source_path.parent().unwrap()).unwrap();
        fs::write(&source_path, "<Group />").unwrap();
        let state = Data::new(
            AppState::new(
                PathBuf::new(),
                dir.path().to_path_buf(),
                watcher_manifest(),
                None,
                None,
                HotReloadMode::All,
                None,
                None,
            )
            .unwrap(),
        );
        assert!(state.active_websocket_client.lock().unwrap().is_none());
        let initial_stamp = state.revisions.lock().unwrap().active_stamp().unwrap();

        handle_watcher_file_changed(
            state.clone(),
            WatcherFileChanged {
                contents: FileContent::Pax("<Group />".to_string()),
                path: source_path.to_string_lossy().into_owned(),
            },
        );

        let revisions = state.revisions.lock().unwrap();
        let updated_stamp = revisions.active_stamp().unwrap();
        assert_eq!(
            updated_stamp.logic_revision_id,
            initial_stamp.logic_revision_id
        );
        assert_eq!(updated_stamp.template_version, 1);
        let manifest = revisions.active_manifest().unwrap();
        assert_eq!(
            manifest
                .components
                .get(&manifest.main_component_type_id)
                .unwrap()
                .template
                .as_ref()
                .unwrap()
                .get_nodes()
                .len(),
            1
        );
    }

    #[test]
    fn rust_watcher_queues_logic_rebuild_without_an_active_websocket() {
        let state = Data::new(
            AppState::new(
                PathBuf::new(),
                PathBuf::new(),
                watcher_manifest(),
                None,
                Some(LogicReloadConfig::Web(WebLogicReloadConfig {
                    serve_dir: PathBuf::new(),
                    should_run_designer: false,
                })),
                HotReloadMode::All,
                None,
                None,
            )
            .unwrap(),
        );
        assert!(state.active_websocket_client.lock().unwrap().is_none());
        {
            let mut logic_reload = state.logic_reload.lock().unwrap();
            let logic_reload = logic_reload.as_mut().unwrap();
            logic_reload.build_in_progress = true;
            logic_reload.build_started = true;
        }

        handle_watcher_file_changed(
            state.clone(),
            WatcherFileChanged {
                contents: FileContent::Rust("pub struct Changed;".to_string()),
                path: "src/lib.rs".to_string(),
            },
        );

        let logic_reload = state.logic_reload.lock().unwrap();
        let logic_reload = logic_reload.as_ref().unwrap();
        assert!(logic_reload.build_in_progress);
        assert!(logic_reload.rebuild_pending);
    }

    #[test]
    fn logic_events_during_debounce_do_not_queue_a_duplicate_build() {
        let state = Data::new(
            AppState::new(
                PathBuf::new(),
                PathBuf::new(),
                watcher_manifest(),
                None,
                Some(LogicReloadConfig::Web(WebLogicReloadConfig {
                    serve_dir: PathBuf::new(),
                    should_run_designer: false,
                })),
                HotReloadMode::All,
                None,
                None,
            )
            .unwrap(),
        );
        {
            let mut logic_reload = state.logic_reload.lock().unwrap();
            let logic_reload = logic_reload.as_mut().unwrap();
            logic_reload.build_in_progress = true;
            logic_reload.build_started = false;
        }

        handle_watcher_file_changed(
            state.clone(),
            WatcherFileChanged {
                contents: FileContent::Rust("pub struct Latest;".to_string()),
                path: "src/lib.rs".to_string(),
            },
        );

        let logic_reload = state.logic_reload.lock().unwrap();
        let logic_reload = logic_reload.as_ref().unwrap();
        assert!(logic_reload.build_in_progress);
        assert!(!logic_reload.build_started);
        assert!(!logic_reload.rebuild_pending);
    }

    #[test]
    fn watcher_filters_transient_files_and_duplicate_contents() {
        assert_eq!(watcher_source_kind(Path::new("src/lib.pax")), Some(true));
        assert_eq!(watcher_source_kind(Path::new("src/lib.rs")), Some(false));
        assert_eq!(watcher_source_kind(Path::new("src/lib.pax~")), None);
        assert_eq!(watcher_source_kind(Path::new("src/.#lib.pax")), None);
        assert_eq!(watcher_source_kind(Path::new("src/lib.pax.swp")), None);
        assert_eq!(watcher_source_kind(Path::new("target/lib.rs")), None);

        let dir = tempdir().unwrap();
        let source_path = dir.path().join("src/lib.pax");
        fs::create_dir_all(source_path.parent().unwrap()).unwrap();
        let state = AppState::new_empty();

        state.expect_watcher_write(&source_path, "server-authored");
        assert!(!state.should_process_watcher_contents(&source_path, "server-authored"));
        assert!(!state.should_process_watcher_contents(&source_path, "server-authored"));
        assert!(state.should_process_watcher_contents(&source_path, "external edit"));
        assert!(!state.should_process_watcher_contents(&source_path, "external edit"));
        assert!(state.should_process_watcher_contents(&source_path, "next external edit"));
    }

    #[test]
    fn failed_logic_build_preserves_active_revision_and_next_success_can_activate() {
        let state = Data::new(
            AppState::new(
                PathBuf::new(),
                PathBuf::new(),
                watcher_manifest(),
                None,
                Some(LogicReloadConfig::Web(WebLogicReloadConfig {
                    serve_dir: PathBuf::new(),
                    should_run_designer: false,
                })),
                HotReloadMode::All,
                None,
                None,
            )
            .unwrap(),
        );
        let initial_stamp = state.revisions.lock().unwrap().active_stamp().unwrap();

        assert!(request_logic_reload(&state));
        run_logic_reload_loop_with(state.clone(), Duration::ZERO, |_, _, _| {
            Err(color_eyre::eyre::eyre!(
                "simulated Rust compilation failure"
            ))
        });

        {
            let revisions = state.revisions.lock().unwrap();
            assert_eq!(revisions.active_stamp().unwrap(), initial_stamp);
            assert!(!revisions.has_candidate());
        }
        {
            let logic_reload = state.logic_reload.lock().unwrap();
            let logic_reload = logic_reload.as_ref().unwrap();
            assert!(!logic_reload.build_in_progress);
            assert!(!logic_reload.build_started);
            assert!(!logic_reload.rebuild_pending);
        }

        assert!(request_logic_reload(&state));
        run_logic_reload_loop_with(state.clone(), Duration::ZERO, |state, _, _| {
            state
                .revisions
                .lock()
                .unwrap()
                .stage_logic_candidate(
                    watcher_manifest(),
                    PrepareAppRevision {
                        logic_revision_id: "fixed-logic".to_string(),
                        execution_mode: DebugLogicExecutionMode::CompiledArtifact,
                        artifact: DebugArtifact {
                            kind: "web-cartridge".to_string(),
                            location: "/__reloads__/fixed/pax-cartridge".to_string(),
                        },
                    },
                    ActivationChannel::WebSocket,
                )
                .map_err(color_eyre::eyre::Report::msg)
        });

        assert_eq!(
            state.revisions.lock().unwrap().active_stamp().unwrap(),
            initial_stamp
        );
        state
            .prepare_logic_activation("fixed-logic", 1)
            .expect("successful rebuild should prepare");
        state
            .activate_logic_revision("fixed-logic", 1)
            .expect("successful rebuild should activate");
        assert_eq!(
            state
                .revisions
                .lock()
                .unwrap()
                .active_stamp()
                .unwrap()
                .logic_revision_id,
            "fixed-logic"
        );
    }

    #[test]
    fn logic_only_mode_leaves_pax_changes_on_disk_without_mutating_the_live_revision() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("src/lib.pax");
        fs::create_dir_all(source_path.parent().unwrap()).unwrap();
        fs::write(&source_path, "<Group />").unwrap();
        let state = Data::new(
            AppState::new(
                PathBuf::new(),
                dir.path().to_path_buf(),
                watcher_manifest(),
                None,
                Some(LogicReloadConfig::Web(WebLogicReloadConfig {
                    serve_dir: PathBuf::new(),
                    should_run_designer: false,
                })),
                HotReloadMode::Logic,
                None,
                None,
            )
            .unwrap(),
        );
        let initial_stamp = state.revisions.lock().unwrap().active_stamp().unwrap();

        handle_watcher_file_changed(
            state.clone(),
            WatcherFileChanged {
                contents: FileContent::Pax("<Group />".to_string()),
                path: source_path.to_string_lossy().into_owned(),
            },
        );

        assert_eq!(
            state.revisions.lock().unwrap().active_stamp().unwrap(),
            initial_stamp
        );
        assert!(*state.pax_restart_notice_emitted.lock().unwrap());
        assert_eq!(fs::read_to_string(source_path).unwrap(), "<Group />");
    }

    #[test]
    fn pax_only_mode_does_not_queue_logic_builds() {
        let state = Data::new(
            AppState::new(
                PathBuf::new(),
                PathBuf::new(),
                watcher_manifest(),
                None,
                Some(LogicReloadConfig::Web(WebLogicReloadConfig {
                    serve_dir: PathBuf::new(),
                    should_run_designer: false,
                })),
                HotReloadMode::Pax,
                None,
                None,
            )
            .unwrap(),
        );

        for _ in 0..2 {
            handle_watcher_file_changed(
                state.clone(),
                WatcherFileChanged {
                    contents: FileContent::Rust("pub struct Changed;".to_string()),
                    path: "src/lib.rs".to_string(),
                },
            );
        }

        let logic_reload = state.logic_reload.lock().unwrap();
        let logic_reload = logic_reload.as_ref().unwrap();
        assert!(!logic_reload.build_in_progress);
        assert!(!logic_reload.rebuild_pending);
        assert!(*state.logic_restart_notice_emitted.lock().unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn failed_restart_snapshot_publication_rejects_activation_before_commit() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().unwrap();
        let manifest_path = dir.path().join("active-manifest.json");
        let state = AppState::new(
            PathBuf::new(),
            dir.path().to_path_buf(),
            watcher_manifest(),
            None,
            Some(LogicReloadConfig::Native(NativeLogicReloadConfig {
                session_dir: dir.path().join("session"),
                manifest_path: manifest_path.clone(),
                should_run_designer: false,
            })),
            HotReloadMode::All,
            None,
            Some(manifest_path.clone()),
        )
        .unwrap();
        state
            .revisions
            .lock()
            .unwrap()
            .stage_logic_candidate(
                watcher_manifest(),
                PrepareAppRevision {
                    logic_revision_id: "candidate-native".to_string(),
                    execution_mode: DebugLogicExecutionMode::CompiledArtifact,
                    artifact: DebugArtifact {
                        kind: "macos-dylib".to_string(),
                        location: "/tmp/candidate.dylib".to_string(),
                    },
                },
                ActivationChannel::NativeSession,
            )
            .unwrap();

        state
            .prepare_logic_activation("candidate-native", 1)
            .expect("candidate preflight should succeed");
        let committed_snapshot = fs::read(&manifest_path).unwrap();
        let mut read_only = fs::metadata(dir.path()).unwrap().permissions();
        read_only.set_mode(0o555);
        fs::set_permissions(dir.path(), read_only).unwrap();
        let activation = state.activate_logic_revision("candidate-native", 1);
        let mut writable = fs::metadata(dir.path()).unwrap().permissions();
        writable.set_mode(0o755);
        fs::set_permissions(dir.path(), writable).unwrap();

        let error = match activation {
            Ok(_) => panic!("durability failure must reject activation before commit"),
            Err(error) => error,
        };
        assert!(error.contains("was not committed"));
        let revisions = state.revisions.lock().unwrap();
        assert_ne!(
            revisions.active_stamp().unwrap().logic_revision_id,
            "candidate-native"
        );
        assert!(revisions.has_candidate());
        assert_eq!(fs::read(&manifest_path).unwrap(), committed_snapshot);
    }

    #[cfg(unix)]
    #[test]
    fn designer_mutation_stays_live_and_invalidates_snapshot_on_publish_failure() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().unwrap();
        let manifest_path = dir.path().join("active-manifest.json");
        let state = AppState::new(
            PathBuf::new(),
            dir.path().to_path_buf(),
            watcher_manifest(),
            None,
            None,
            HotReloadMode::All,
            None,
            Some(manifest_path.clone()),
        )
        .unwrap();
        let initial_stamp = state.revisions.lock().unwrap().active_stamp().unwrap();
        let mut component = state
            .revisions
            .lock()
            .unwrap()
            .active_manifest_clone()
            .unwrap()
            .components
            .get(&TypeId::build_singleton("crate::Example", Some("Example")))
            .unwrap()
            .clone();
        component.settings = Some(vec![SettingsBlockElement::Comment(
            "designer edit".to_string(),
        )]);

        let mut read_only = fs::metadata(dir.path()).unwrap().permissions();
        read_only.set_mode(0o555);
        fs::set_permissions(dir.path(), read_only).unwrap();
        let mutation = state
            .install_live_revision_mutation("component serialization", |revisions| {
                revisions.replace_active_component(component)
            });
        let mut writable = fs::metadata(dir.path()).unwrap().permissions();
        writable.set_mode(0o755);
        fs::set_permissions(dir.path(), writable).unwrap();

        mutation.expect("designer mutation should remain live after persistence failure");
        let revisions = state.revisions.lock().unwrap();
        assert_eq!(
            revisions.active_stamp().unwrap().template_version,
            initial_stamp.template_version + 1
        );
        let active = revisions.active_manifest_clone().unwrap();
        let settings = active.components[&active.main_component_type_id]
            .settings
            .as_ref()
            .unwrap();
        assert!(settings.iter().any(
            |setting| matches!(setting, SettingsBlockElement::Comment(comment) if comment == "designer edit")
        ));
        assert_eq!(fs::metadata(&manifest_path).unwrap().len(), 0);
    }

    #[cfg(unix)]
    #[test]
    fn duplicate_active_activation_acks_when_snapshot_republish_fails() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().unwrap();
        let manifest_path = dir.path().join("active-manifest.json");
        let state = AppState::new(
            PathBuf::new(),
            dir.path().to_path_buf(),
            watcher_manifest(),
            None,
            None,
            HotReloadMode::All,
            None,
            Some(manifest_path.clone()),
        )
        .unwrap();
        state
            .revisions
            .lock()
            .unwrap()
            .stage_logic_candidate(
                watcher_manifest(),
                PrepareAppRevision {
                    logic_revision_id: "active-b".to_string(),
                    execution_mode: DebugLogicExecutionMode::CompiledArtifact,
                    artifact: DebugArtifact {
                        kind: "web-cartridge".to_string(),
                        location: "/active-b/pax-cartridge".to_string(),
                    },
                },
                ActivationChannel::WebSocket,
            )
            .unwrap();
        state
            .prepare_logic_activation("active-b", 1)
            .expect("candidate preflight should succeed");
        state
            .activate_logic_revision("active-b", 1)
            .expect("first activation should persist");
        let committed_snapshot = fs::read(&manifest_path).unwrap();
        let (_, persisted) = decode_restart_manifest(&committed_snapshot).unwrap();
        let persisted = persisted.expect("activation should replace the startup envelope");
        assert_eq!(persisted.stamp.logic_revision_id, "active-b");
        assert_eq!(
            persisted.active_artifact.unwrap().prepare.logic_revision_id,
            "active-b"
        );

        let mut read_only = fs::metadata(dir.path()).unwrap().permissions();
        read_only.set_mode(0o555);
        fs::set_permissions(dir.path(), read_only).unwrap();
        let duplicate = state.activate_logic_revision("active-b", 2);
        let mut writable = fs::metadata(dir.path()).unwrap().permissions();
        writable.set_mode(0o755);
        fs::set_permissions(dir.path(), writable).unwrap();

        let duplicate = duplicate.expect("duplicate final must remain idempotent");
        assert!(duplicate.already_active);
        assert!(duplicate.promote_connection);
        assert_eq!(fs::read(&manifest_path).unwrap(), committed_snapshot);
    }

    #[test]
    fn restored_snapshot_reconciles_source_and_queues_one_web_recovery_build() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("src/lib.pax");
        fs::create_dir_all(source_path.parent().unwrap()).unwrap();
        fs::write(&source_path, "<Group />").unwrap();

        let mut coordinator = DebugRevisionCoordinator::new("a".to_string(), watcher_manifest());
        coordinator
            .stage_logic_candidate(
                watcher_manifest(),
                PrepareAppRevision {
                    logic_revision_id: "active-b".to_string(),
                    execution_mode: DebugLogicExecutionMode::CompiledArtifact,
                    artifact: DebugArtifact {
                        kind: "web-cartridge".to_string(),
                        location: "/active-b/pax-cartridge".to_string(),
                    },
                },
                ActivationChannel::WebSocket,
            )
            .unwrap();
        coordinator
            .prepare_logic_activation("active-b", 1, |_, _, _| unreachable!())
            .unwrap();
        coordinator.activate_logic_revision("active-b", 1).unwrap();
        let snapshot = coordinator.restart_snapshot().unwrap();

        let state = AppState::new(
            PathBuf::from("served-web"),
            dir.path().to_path_buf(),
            watcher_manifest(),
            None,
            None,
            HotReloadMode::All,
            Some(snapshot.clone()),
            None,
        )
        .unwrap();

        assert_eq!(
            state.revisions.lock().unwrap().active_stamp().unwrap(),
            RevisionStamp {
                logic_revision_id: "active-b".to_string(),
                template_version: 1,
            }
        );
        assert!(matches!(
            state
                .logic_reload
                .lock()
                .unwrap()
                .as_ref()
                .map(|reload| &reload.config),
            Some(LogicReloadConfig::Web(_))
        ));
        assert!(state.take_startup_recovery_rebuild());
        assert!(!state.take_startup_recovery_rebuild());
        assert_eq!(
            state
                .prepare_for_web_client(Some("a"))
                .unwrap()
                .logic_revision_id,
            "active-b"
        );

        let pax_only = AppState::new(
            PathBuf::from("served-web"),
            dir.path().to_path_buf(),
            watcher_manifest(),
            None,
            None,
            HotReloadMode::Pax,
            Some(snapshot.clone()),
            None,
        )
        .unwrap();
        assert_eq!(
            pax_only
                .revisions
                .lock()
                .unwrap()
                .active_stamp()
                .unwrap()
                .template_version,
            1
        );
        assert!(!pax_only.take_startup_recovery_rebuild());

        let logic_only = AppState::new(
            PathBuf::from("served-web"),
            dir.path().to_path_buf(),
            watcher_manifest(),
            None,
            None,
            HotReloadMode::Logic,
            Some(snapshot.clone()),
            None,
        )
        .unwrap();
        assert_eq!(
            logic_only
                .revisions
                .lock()
                .unwrap()
                .active_stamp()
                .unwrap()
                .template_version,
            0
        );
        assert!(logic_only.take_startup_recovery_rebuild());

        let off = AppState::new(
            PathBuf::from("served-web"),
            dir.path().to_path_buf(),
            watcher_manifest(),
            None,
            None,
            HotReloadMode::Off,
            Some(snapshot),
            None,
        )
        .unwrap();
        assert_eq!(
            off.revisions
                .lock()
                .unwrap()
                .active_stamp()
                .unwrap()
                .template_version,
            0
        );
        assert!(!off.take_startup_recovery_rebuild());
    }

    #[actix_web::test]
    async fn deep_html_routes_fall_back_to_index_html() {
        let dir = tempdir().expect("failed to create temp dir");
        fs::write(
            dir.path().join("index.html"),
            "<html><head></head><body>router playground</body></html>",
        )
        .expect("failed to write index");

        let app = actix_test::init_service(
            App::new().service(static_files_service(dir.path().to_path_buf())),
        )
        .await;

        let req = actix_test::TestRequest::get()
            .uri("/guide/topic/router")
            .insert_header((header::ACCEPT, "text/html"))
            .to_request();
        let resp = actix_test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);
        let body = actix_test::read_body(resp).await;
        let body = String::from_utf8(body.to_vec()).expect("body should be utf-8");
        assert!(body.contains(r#"<base href="/">"#));
        assert!(body.contains("router playground"));
    }

    #[actix_web::test]
    async fn missing_assets_still_return_404() {
        let dir = tempdir().expect("failed to create temp dir");
        fs::write(
            dir.path().join("index.html"),
            "<html>router playground</html>",
        )
        .expect("failed to write index");

        let app = actix_test::init_service(
            App::new().service(static_files_service(dir.path().to_path_buf())),
        )
        .await;

        let req = actix_test::TestRequest::get()
            .uri("/missing.js")
            .insert_header((header::ACCEPT, "*/*"))
            .to_request();
        let resp = actix_test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
