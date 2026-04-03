use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use color_eyre::eyre::{eyre, Report, Result};
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
