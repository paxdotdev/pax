use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use color_eyre::eyre::{eyre, Report, Result};
use pax_designtime::messages::PrepareAppRevision;
use serde::{Deserialize, Serialize};

pub const DEV_DIR_NAME: &str = "dev";
pub const DEV_SESSION_STALE_AFTER_MS: u128 = 15_000;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DevRequestEnvelope {
    pub request_id: String,
    pub kind: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DevLookRequest {
    pub request_id: String,
    pub kind: String,
    pub output_dir: PathBuf,
    pub scale: f64,
    pub period_ms: u64,
    pub duration_ms: u64,
    pub format: String,
    pub quality: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DevCapture {
    pub path: PathBuf,
    pub width: usize,
    pub height: usize,
    pub captured_at_ms: u128,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DevLookResponse {
    pub request_id: String,
    pub status: String,
    pub captures: Vec<DevCapture>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DevInspectTreeRequest {
    pub request_id: String,
    pub kind: String,
    pub max_depth: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DevInspectTreeResponse {
    pub request_id: String,
    pub status: String,
    pub node_count: Option<usize>,
    pub tree_json: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DevRayCastRequest {
    pub request_id: String,
    pub kind: String,
    pub x: f64,
    pub y: f64,
    pub hit_invisible: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DevRayCastResponse {
    pub request_id: String,
    pub status: String,
    pub x: f64,
    pub y: f64,
    pub hit_invisible: bool,
    pub node_count: Option<usize>,
    pub nodes_json: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DevSelectorQueryRequest {
    pub request_id: String,
    pub kind: String,
    pub selector: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DevSelectorQueryResponse {
    pub request_id: String,
    pub status: String,
    pub selector: String,
    pub node_count: Option<usize>,
    pub nodes_json: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DevReplaceNodeRequest {
    pub request_id: String,
    pub kind: String,
    pub component_type_id: String,
    pub template_node_id: usize,
    pub subtemplate: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DevReplaceNodeResponse {
    pub request_id: String,
    pub status: String,
    pub component_type_id: String,
    pub template_node_id: usize,
    pub reload_scope: String,
    pub reloaded_template_node_id: Option<usize>,
    pub source_path: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DevReloadLogicRequest {
    pub request_id: String,
    pub kind: String,
    pub revision: PrepareAppRevision,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DevReloadLogicResponse {
    pub request_id: String,
    pub status: String,
    pub logic_revision_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DevLogsRequest {
    pub request_id: String,
    pub kind: String,
    pub since_seq: Option<u64>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DevLogEntry {
    pub seq: u64,
    pub level: String,
    pub message: String,
    pub timestamp_ms: u128,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DevLogsResponse {
    pub request_id: String,
    pub status: String,
    pub entries: Vec<DevLogEntry>,
    pub next_seq: u64,
    pub oldest_seq: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DevSession {
    pub session_id: String,
    pub platform: String,
    pub designtime: bool,
    pub project_root: Option<PathBuf>,
    pub session_dir: Option<PathBuf>,
    pub app_pid: Option<u32>,
    pub design_server_addr: Option<String>,
    pub control_kind: String,
    pub location: Option<String>,
    pub started_at_ms: u128,
    pub last_seen_ms: u128,
}

/// A connected runtime and its last acknowledged template revision.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RuntimeInstance {
    pub id: String,
    pub target: String,
    pub cartridge_abi: String,
    pub capabilities: Vec<String>,
    pub applied: Option<pax_designtime::messages::RevisionStamp>,
    pub rejection: Option<String>,
}

/// Latest build attempt and the last successfully published artifact for a target.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetBuild {
    pub generation: u64,
    pub artifact: Option<PathBuf>,
    pub building: bool,
    pub error: Option<String>,
}

/// A web development server that can host additional native preview clients.
/// Its registration lives independently of the browser's inspection session.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectServer {
    pub id: String,
    pub project_root: PathBuf,
    pub port: u16,
    pub cartridge_abi: String,
    pub template_reload: bool,
    #[serde(default)]
    pub provides_instances: bool,
    /// Supports project-scoped Apple mobile preview connections.
    #[serde(default)] pub mobile_previews:bool,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub builds: std::collections::BTreeMap<String, TargetBuild>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub instances: Vec<RuntimeInstance>,
    /// Optional identity supplied by the process supervisor that launched this server.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch_id: Option<String>,
}

pub(crate) fn cartridge_abi(manifest: &pax_manifest::PaxManifest) -> Result<String, Report> {
    pax_designtime::orm::runtime_abi_digest(manifest).map_err(|error| eyre!("{error}"))
}

pub(crate) struct ProjectServerRegistration(PathBuf);

impl ProjectServerRegistration {
    pub fn publish(server: &ProjectServer) -> Result<Self, Report> {
        Ok(Self(Self::update(server)?))
    }

    pub(super) fn update(server: &ProjectServer) -> Result<PathBuf, Report> {
        let path = global_dev_dir()?
            .join("servers")
            .join(format!("{}.json", server.id));
        atomic_write_json(&path, server)?;
        Ok(path)
    }
}

impl Drop for ProjectServerRegistration {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// List Web servers whose registry identity matches their live loopback endpoint.
/// An optional project path restricts discovery to that canonical project root.
pub fn live_project_servers(root: Option<&Path>) -> Result<Vec<ProjectServer>, Report> {
    let root = root.map(Path::canonicalize).transpose()?;
    if let Some(root) = &root {
        if !root.is_dir() {
            return Err(eyre!("Project path {} is not a directory", root.display()));
        }
    }
    let dir = global_dev_dir()?.join("servers");
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(2))
        .build()?;
    let mut found = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Some(mut server) = read_json_if_exists::<ProjectServer>(&path).ok().flatten() else {
            continue;
        };
        if root
            .as_ref()
            .is_some_and(|root| root != &server.project_root)
        {
            continue;
        }
        // A stale record's port can already belong to a different process.
        let identity = client
            .get(format!("http://127.0.0.1:{}/_pax/server-id", server.port))
            .send()
            .and_then(|response| response.error_for_status()?.text());
        if identity.ok().as_deref() != Some(&server.id) {
            continue;
        }
        if server.provides_instances || !server.builds.is_empty() {
            server.instances = client
                .get(format!("http://127.0.0.1:{}/_pax/instances", server.port))
                .send()
                .and_then(|r| r.error_for_status()?.bytes())
                .ok()
                .and_then(|bytes| serde_json::from_slice(&bytes).ok())
                .unwrap_or_default();
        }
        found.push(server);
    }
    found.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(found)
}

pub(crate) fn live_project_server(root: &Path) -> Result<Option<ProjectServer>, Report> {
    let mut servers = live_project_servers(Some(root))?;
    if servers.len() > 1 {
        return Err(eyre!("Multiple web development servers are serving {}. Stop the duplicate before attaching a native preview.", root.display()));
    }
    Ok(servers.pop())
}

impl DevSession {
    pub fn is_stale(&self, now_ms: u128) -> bool {
        now_ms.saturating_sub(self.last_seen_ms) > DEV_SESSION_STALE_AFTER_MS
    }
}

pub fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

pub fn project_dev_dir(pax_dir: &Path) -> PathBuf {
    pax_dir.join(DEV_DIR_NAME)
}

pub fn project_active_session_file(pax_dir: &Path) -> PathBuf {
    project_dev_dir(pax_dir).join("active-session.json")
}

pub fn project_designtime_manifest_file(pax_dir: &Path) -> PathBuf {
    project_dev_dir(pax_dir).join("designtime-manifest.json")
}

pub fn global_dev_dir() -> Result<PathBuf, Report> {
    let root = if cfg!(target_os = "macos") {
        user_home_dir()?
            .join("Library")
            .join("Application Support")
            .join("Pax")
            .join(DEV_DIR_NAME)
    } else if cfg!(target_os = "windows") {
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            PathBuf::from(local_app_data).join("Pax").join(DEV_DIR_NAME)
        } else if let Some(app_data) = std::env::var_os("APPDATA") {
            PathBuf::from(app_data).join("Pax").join(DEV_DIR_NAME)
        } else {
            user_home_dir()?
                .join("AppData")
                .join("Local")
                .join("Pax")
                .join(DEV_DIR_NAME)
        }
    } else if let Some(xdg_state_home) = std::env::var_os("XDG_STATE_HOME") {
        PathBuf::from(xdg_state_home).join("pax").join(DEV_DIR_NAME)
    } else if let Some(xdg_data_home) = std::env::var_os("XDG_DATA_HOME") {
        PathBuf::from(xdg_data_home).join("pax").join(DEV_DIR_NAME)
    } else {
        user_home_dir()?
            .join(".local")
            .join("state")
            .join("pax")
            .join(DEV_DIR_NAME)
    };

    fs::create_dir_all(&root)?;
    Ok(root)
}

pub fn global_session_registry_dir() -> Result<PathBuf, Report> {
    let dir = global_dev_dir()?.join("sessions");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn global_session_registry_file(session_id: &str) -> Result<PathBuf, Report> {
    Ok(global_session_registry_dir()?.join(format!("{session_id}.json")))
}

pub fn write_project_active_session(pax_dir: &Path, session: &DevSession) -> Result<(), Report> {
    atomic_write_json(&project_active_session_file(pax_dir), session)
}

pub fn read_project_active_session(pax_dir: &Path) -> Result<Option<DevSession>, Report> {
    read_json_if_exists(&project_active_session_file(pax_dir))
}

pub fn remove_project_active_session(pax_dir: &Path, session_id: &str) -> Result<(), Report> {
    let path = project_active_session_file(pax_dir);
    if let Some(existing) = read_json_if_exists::<DevSession>(&path)? {
        if existing.session_id == session_id {
            let _ = fs::remove_file(path);
        }
    }
    Ok(())
}

pub fn write_registered_session(session: &DevSession) -> Result<(), Report> {
    atomic_write_json(&global_session_registry_file(&session.session_id)?, session)
}

pub fn remove_registered_session(session_id: &str) -> Result<(), Report> {
    let path = global_session_registry_file(session_id)?;
    if path.exists() {
        let _ = fs::remove_file(path);
    }
    Ok(())
}

pub fn list_registered_sessions() -> Result<Vec<DevSession>, Report> {
    let registry_dir = global_session_registry_dir()?;
    let mut sessions = vec![];
    for entry in fs::read_dir(registry_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        match fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<DevSession>(&bytes).ok())
        {
            Some(session) => sessions.push(session),
            None => {
                let _ = fs::remove_file(path);
            }
        }
    }
    sessions.sort_by(|a, b| b.started_at_ms.cmp(&a.started_at_ms));
    Ok(sessions)
}

pub fn session_request_dir(session: &DevSession) -> Result<PathBuf, Report> {
    let session_dir = session.session_dir.as_ref().ok_or_else(|| {
        eyre!(
            "session {} does not expose a local request directory",
            session.session_id
        )
    })?;
    Ok(session_dir.join("requests"))
}

pub fn session_response_dir(session: &DevSession) -> Result<PathBuf, Report> {
    let session_dir = session.session_dir.as_ref().ok_or_else(|| {
        eyre!(
            "session {} does not expose a local response directory",
            session.session_id
        )
    })?;
    Ok(session_dir.join("responses"))
}

pub fn write_session_request_json<T: Serialize>(
    session_dir: &Path,
    request_id: &str,
    value: &T,
) -> Result<(), Report> {
    atomic_write_json(
        &session_dir
            .join("requests")
            .join(format!("{request_id}.json")),
        value,
    )
}

fn read_json_if_exists<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<Option<T>, Report> {
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_slice(&fs::read(path)?)?))
}

fn atomic_write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), Report> {
    let bytes = serde_json::to_vec_pretty(value)?;
    let tmp_path = path.with_extension("tmp");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&tmp_path, bytes)?;
    fs::rename(tmp_path, path)?;
    Ok(())
}

fn user_home_dir() -> Result<PathBuf, Report> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
        .or_else(
            || match (std::env::var_os("HOMEDRIVE"), std::env::var_os("HOMEPATH")) {
                (Some(drive), Some(path)) => Some(PathBuf::from(drive).join(path)),
                _ => None,
            },
        )
        .ok_or_else(|| eyre!("could not determine the current user's home directory"))
}

#[cfg(test)]
mod tests {
    use super::{DevReloadLogicRequest, DevReloadLogicResponse};
    use pax_designtime::messages::{DebugArtifact, DebugLogicExecutionMode, PrepareAppRevision};

    #[test]
    fn native_reload_request_round_trips_revision_envelope() {
        let request = DevReloadLogicRequest {
            request_id: "reload-logic-7".to_string(),
            kind: "reload-logic".to_string(),
            revision: PrepareAppRevision {
                logic_revision_id: "logic-7".to_string(),
                execution_mode: DebugLogicExecutionMode::CompiledArtifact,
                artifact: DebugArtifact {
                    kind: "macos-dylib".to_string(),
                    location: "/tmp/PaxCartridge-logic-7.dylib".to_string(),
                },
            },
        };

        let encoded = serde_json::to_vec(&request).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(
            json["revision"]["execution_mode"],
            serde_json::Value::String("compiled-artifact".to_string())
        );
        let decoded: DevReloadLogicRequest = serde_json::from_slice(&encoded).unwrap();

        assert_eq!(decoded.request_id, "reload-logic-7");
        assert_eq!(decoded.kind, "reload-logic");
        assert_eq!(decoded.revision.logic_revision_id, "logic-7");
        assert_eq!(decoded.revision.artifact.kind, "macos-dylib");
        assert_eq!(
            decoded.revision.artifact.location,
            "/tmp/PaxCartridge-logic-7.dylib"
        );
        assert!(matches!(
            decoded.revision.execution_mode,
            DebugLogicExecutionMode::CompiledArtifact
        ));
    }

    #[test]
    fn native_reload_response_round_trips_revision_identity() {
        let response = DevReloadLogicResponse {
            request_id: "reload-logic-7".to_string(),
            status: "ok".to_string(),
            logic_revision_id: Some("logic-7".to_string()),
            error: None,
        };

        let encoded = serde_json::to_vec(&response).unwrap();
        let decoded: DevReloadLogicResponse = serde_json::from_slice(&encoded).unwrap();

        assert_eq!(decoded.request_id, "reload-logic-7");
        assert_eq!(decoded.status, "ok");
        assert_eq!(decoded.logic_revision_id.as_deref(), Some("logic-7"));
        assert!(decoded.error.is_none());
    }
}
