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
    self, write_session_request_json, DevLookRequest, DevReloadLogicRequest, DevSession,
};
use crate::helpers::PAX_BADGE;
use crate::{RunContext, RunTarget};
use notify::{Error, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use pax_designtime::messages::{AgentMessage, ReloadAppRequest};
use pax_manifest::PaxManifest;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use websocket::PrivilegedAgentWebSocket;

#[allow(unused)]
mod llm;
pub mod static_server;
pub mod websocket;

pub(crate) const DEFAULT_BIND_HOST: &str = "0.0.0.0";
const LOOPBACK_DISPLAY_HOST: &str = "127.0.0.1";

pub(crate) fn display_addresses(port: u16) -> Vec<String> {
    let mut addresses = vec![format!("http://{LOOPBACK_DISPLAY_HOST}:{port}")];
    if let Some(ip) = local_network_ip() {
        addresses.push(format!("http://{ip}:{port}"));
    }
    addresses
}

fn local_network_ip() -> Option<IpAddr> {
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
    rebuild_pending: bool,
}

pub struct AppState {
    serve_dir: Mutex<PathBuf>,
    userland_project_root: Mutex<PathBuf>,
    active_websocket_client: Mutex<Option<ActiveWebsocketClient>>,
    websocket_client_counter: Mutex<usize>,
    request_id_counter: Mutex<usize>,
    in_flight_dev_requests: Mutex<HashSet<String>>,
    manifest: Mutex<Option<PaxManifest>>,
    last_written_timestamp: Mutex<SystemTime>,
    dev_session: Mutex<Option<DevSession>>,
    pending_dev_look_requests: Mutex<HashMap<String, DevLookRequest>>,
    logic_reload: Mutex<Option<LogicReloadState>>,
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
            manifest: Mutex::new(None),
            last_written_timestamp: Mutex::new(UNIX_EPOCH),
            dev_session: Mutex::new(None),
            pending_dev_look_requests: Mutex::new(HashMap::new()),
            logic_reload: Mutex::new(None),
        }
    }
    pub fn new(
        serve_dir: PathBuf,
        project_root: PathBuf,
        manifest: PaxManifest,
        dev_session: Option<DevSession>,
        logic_reload: Option<LogicReloadConfig>,
    ) -> Self {
        AppState {
            serve_dir: Mutex::new(serve_dir),
            userland_project_root: Mutex::new(project_root),
            active_websocket_client: Mutex::new(None),
            websocket_client_counter: Mutex::new(0),
            request_id_counter: Mutex::new(0),
            in_flight_dev_requests: Mutex::new(HashSet::new()),
            manifest: Mutex::new(Some(manifest)),
            last_written_timestamp: Mutex::new(SystemTime::now()),
            dev_session: Mutex::new(dev_session),
            pending_dev_look_requests: Mutex::new(HashMap::new()),
            logic_reload: Mutex::new(logic_reload.map(|config| LogicReloadState {
                config,
                build_in_progress: false,
                rebuild_pending: false,
            })),
        }
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

    pub fn update_last_written_timestamp(&self) {
        let mut last_written = self.last_written_timestamp.lock().unwrap();
        *last_written = SystemTime::now();
    }
}

pub fn schedule_logic_reload(state: Data<AppState>) {
    let mut logic_reload = state.logic_reload.lock().unwrap();
    let Some(reload_state) = logic_reload.as_mut() else {
        return;
    };

    if reload_state.build_in_progress {
        reload_state.rebuild_pending = true;
        return;
    }

    reload_state.build_in_progress = true;
    reload_state.rebuild_pending = false;
    drop(logic_reload);

    std::thread::spawn(move || run_logic_reload_loop(state));
}

pub fn run_logic_reload_loop(state: Data<AppState>) {
    loop {
        std::thread::sleep(std::time::Duration::from_millis(150));

        let config = {
            let logic_reload = state.logic_reload.lock().unwrap();
            logic_reload.as_ref().map(|reload| reload.config.clone())
        };
        let Some(config) = config else {
            return;
        };
        let project_root = state.userland_project_root.lock().unwrap().clone();

        if let Err(err) = perform_logic_reload(&state, &project_root, &config) {
            eprintln!("failed to reload logic for hot reload: {err}");
        }

        let should_repeat = {
            let mut logic_reload = state.logic_reload.lock().unwrap();
            let Some(reload_state) = logic_reload.as_mut() else {
                return;
            };
            if reload_state.rebuild_pending {
                reload_state.rebuild_pending = false;
                true
            } else {
                reload_state.build_in_progress = false;
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
            let build = rebuild_staged_macos_logic_dylib(
                project_root,
                &config.session_dir,
                config.should_run_designer,
            )?;
            *state.manifest.lock().unwrap() = Some(build.manifest);
            enqueue_native_logic_reload_request(state, config, build.dylib_path.as_path())?;
        }
        LogicReloadConfig::Web(config) => {
            let build = rebuild_staged_web_cartridge(
                project_root,
                &config.serve_dir,
                config.should_run_designer,
            )?;
            *state.manifest.lock().unwrap() = Some(build.manifest);
            send_agent_message_to_active_client(
                state,
                AgentMessage::ReloadAppRequest(ReloadAppRequest {
                    request_id: format!("reload-app-{}", state.generate_request_id()),
                    build_id: build.build_id,
                    // Keep the host protocol artifact-oriented so future
                    // runtimes can extend this without inheriting dylib-shaped
                    // assumptions from the native path.
                    artifact_kind: "web-cartridge".to_string(),
                    artifact_location: build.extensionless_url,
                }),
            );
        }
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
    dylib_path: &Path,
) -> Result<(), color_eyre::eyre::Report> {
    let request_id = format!("reload-logic-{}", state.generate_request_id());
    let request_file_id = request_id.clone();
    write_session_request_json(
        &config.session_dir,
        &request_file_id,
        &DevReloadLogicRequest {
            request_id,
            kind: "reload-logic".to_string(),
            dylib_path: dylib_path.to_string_lossy().into_owned(),
        },
    )
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
pub fn start_server(
    static_file_path: &str,
    src_folder_to_watch: &str,
    manifest: PaxManifest,
    requested_port: Option<u16>,
    ready_file: Option<PathBuf>,
    show_address_log: bool,
    dev_session: Option<DevSession>,
    logic_reload: Option<LogicReloadConfig>,
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
    );
    let fs_path = initial_state.serve_dir.lock().unwrap().clone();
    let state = Data::new(initial_state);
    let _watcher = setup_file_watcher(state.clone(), src_folder_to_watch)
        .expect("Failed to setup file watcher");

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
            let address_msg = display_addresses(port).join(" or ").blue();
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

#[derive(Default)]
pub enum FileContent {
    Pax(String),
    Rust(String),
    #[default]
    Unknown,
}

#[derive(Default)]
struct WatcherFileChanged {
    pub contents: FileContent,
    pub path: String,
}

impl actix::Message for WatcherFileChanged {
    type Result = ();
}

pub fn setup_file_watcher(state: Data<AppState>, path: &str) -> Result<RecommendedWatcher, Error> {
    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, Error>| match res {
            Ok(e) => {
                if let Some(addr) = &*state.active_websocket_client.lock().unwrap() {
                    let now = SystemTime::now();
                    // check last written time so we don't spam file changes when we serialize
                    let last_written = *state.last_written_timestamp.lock().unwrap();
                    if now
                        .duration_since(last_written)
                        .unwrap_or_default()
                        .as_millis()
                        > 1000
                    {
                        if let EventKind::Modify(_) = e.kind {
                            if let Some(path) = e.paths.first() {
                                if should_ignore_watcher_path(path) {
                                    return;
                                }
                                match fs::read_to_string(path) {
                                    Ok(contents) => {
                                        let extension = path.extension();
                                        let msg = WatcherFileChanged {
                                            contents: match extension.and_then(|e| e.to_str()) {
                                                Some("pax") => FileContent::Pax(contents),
                                                Some("rs") => FileContent::Rust(contents),
                                                _ => FileContent::Unknown,
                                            },
                                            path: path.to_str().unwrap().to_string(),
                                        };
                                        addr.addr.do_send(msg);
                                    }
                                    Err(_) => (),
                                }
                            }
                        }
                    }
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

fn should_ignore_watcher_path(path: &Path) -> bool {
    path.components().any(|component| {
        let component = component.as_os_str();
        component == ".pax" || component == "target"
    })
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
    *state.manifest.lock().unwrap() = Some(manifest);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::static_files_service;
    use actix_web::http::{header, StatusCode};
    use actix_web::{test, App};
    use std::fs;
    use tempfile::tempdir;

    #[actix_web::test]
    async fn deep_html_routes_fall_back_to_index_html() {
        let dir = tempdir().expect("failed to create temp dir");
        fs::write(
            dir.path().join("index.html"),
            "<html><head></head><body>router playground</body></html>",
        )
        .expect("failed to write index");

        let app =
            test::init_service(App::new().service(static_files_service(dir.path().to_path_buf())))
                .await;

        let req = test::TestRequest::get()
            .uri("/guide/topic/router")
            .insert_header((header::ACCEPT, "text/html"))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);
        let body = test::read_body(resp).await;
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

        let app =
            test::init_service(App::new().service(static_files_service(dir.path().to_path_buf())))
                .await;

        let req = test::TestRequest::get()
            .uri("/missing.js")
            .insert_header((header::ACCEPT, "*/*"))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
