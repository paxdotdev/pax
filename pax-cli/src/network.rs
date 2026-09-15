//! Best-effort CLI networking. Public commands never wait for HTTP completion.
//!
//! Updates are opportunistic in-process work; telemetry uses a bounded one-shot
//! copy of this executable so it can finish after the invoking command exits.

use pax_message::http_api::{
    HostArch, HostOs, LatestReleaseResponse, TelemetryEvent, TelemetryRequest,
    CLI_LATEST_RELEASE_PATH, CLI_TELEMETRY_PATH,
};
use reqwest::blocking::Client;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

const DEFAULT_API_BASE_URL: &str = "https://pub.pax.dev";
const WORKER_ARGUMENT: &str = "--pax-telemetry-worker";
const MAX_MESSAGE_BYTES: u64 = 4096;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(1);
pub(crate) const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);
// Also bounds stdin reads, client initialization and local lock contention.
const WORKER_LIFETIME: Duration = Duration::from_secs(5);

#[cfg(windows)]
static BACKGROUND_READY: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

pub(crate) fn initialize() {
    #[cfg(windows)]
    let _ = BACKGROUND_READY.set(prepare_standard_handles());
}

#[cfg(windows)]
fn prepare_standard_handles() -> bool {
    use windows_sys::Win32::Foundation::{
        SetHandleInformation, HANDLE_FLAG_INHERIT, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::System::Console::{
        GetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
    };

    // Stable Rust inherits all inheritable Windows handles, even with the
    // worker's stdout/stderr set to NUL. Clear the original streams' flags
    // once, before threads start, so no accidental aliases keep caller pipes
    // open. Command's explicit Stdio::inherit still duplicates them as needed.
    for stream in [STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
        // SAFETY: These borrowed process handles remain owned by std/the OS.
        // Only the inheritance bit changes; no handle is closed or replaced.
        let succeeded = unsafe {
            let handle = GetStdHandle(stream);
            handle.is_null()
                || handle == INVALID_HANDLE_VALUE
                || SetHandleInformation(handle, HANDLE_FLAG_INHERIT, 0) != 0
        };
        if !succeeded {
            return false;
        }
    }
    true
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TelemetryDelivery {
    installation_id: String,
    event: TelemetryEvent,
}

#[derive(Clone)]
pub(crate) struct TelemetrySender {
    installation_id: String,
    environment: Vec<(&'static str, Option<OsString>)>,
}

impl TelemetrySender {
    pub(crate) fn new(installation_id: String) -> Self {
        // Builds may load project .env files. Telemetry must keep the caller's
        // original endpoint, consent directory and environment policy instead.
        let environment = crate::telemetry::CONFIG_ENV_KEYS
            .iter()
            .copied()
            .chain(std::iter::once("PAX_API_BASE_URL"))
            .map(|key| (key, std::env::var_os(key)))
            .collect();
        Self {
            installation_id,
            environment,
        }
    }

    pub(crate) fn send(&self, event: TelemetryEvent) {
        let delivery = TelemetryDelivery {
            installation_id: self.installation_id.clone(),
            event,
        };
        let _ = spawn_delivery(&delivery, &self.environment);
    }
}

pub(crate) fn start_update_check(new_version_info: Arc<Mutex<Option<String>>>) {
    let base_url = api_base_url();
    let _ = thread::Builder::new()
        .name("pax-update".into())
        .spawn(move || {
            let _ = std::panic::catch_unwind(|| {
                if let Some(version) = latest_version(&base_url) {
                    if let Ok(mut result) = new_version_info.lock() {
                        *result = Some(version);
                    }
                }
            });
        });
}

fn spawn_delivery(
    delivery: &TelemetryDelivery,
    environment: &[(&str, Option<OsString>)],
) -> io::Result<()> {
    #[cfg(windows)]
    if BACKGROUND_READY.get() != Some(&true) {
        return Err(io::Error::other("background handles could not be isolated"));
    }
    let message = serde_json::to_vec(delivery)?;
    if message.len() as u64 > MAX_MESSAGE_BYTES {
        return Err(io::Error::other("telemetry handoff exceeds its size limit"));
    }

    // Start the reaper before the child so thread exhaustion cannot leave a
    // zombie in a long-running `run` session. It is never joined on CLI exit.
    let (sender, receiver) = mpsc::sync_channel::<Child>(1);
    thread::Builder::new()
        .name("pax-telemetry-reaper".into())
        .spawn(move || {
            if let Ok(mut child) = receiver.recv() {
                let _ = child.wait();
            }
        })?;

    let mut command = Command::new(std::env::current_exe()?);
    command
        .arg(WORKER_ARGUMENT)
        // Do not keep the user's project directory or terminal pipes alive.
        .current_dir(std::env::temp_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    apply_environment(&mut command, environment);
    detach(&mut command);
    let mut child = command.spawn()?;
    let handoff = match child.stdin.take() {
        Some(mut stdin) => stdin.write_all(&message),
        None => Err(io::Error::other("telemetry handoff pipe unavailable")),
    };
    // The fixed, small message fits into the pipe; EOF tells the worker it has
    // received the complete event. No request result is sent back to the CLI.
    if handoff.is_err() {
        let _ = child.kill();
    }
    if let Err(error) = sender.send(child) {
        let mut child = error.0;
        let _ = child.kill();
        let _ = child.wait();
    }
    handoff
}

fn apply_environment(command: &mut Command, environment: &[(&str, Option<OsString>)]) {
    for (key, value) in environment {
        match value {
            Some(value) => command.env(key, value),
            None => command.env_remove(key),
        };
    }
}

#[cfg(unix)]
fn detach(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(windows)]
fn detach(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    // No inherited console, console-control events, or flashing worker window.
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(any(unix, windows)))]
fn detach(_command: &mut Command) {}

pub(crate) fn run_worker_if_requested() -> bool {
    let mut arguments = std::env::args_os().skip(1);
    if arguments.next().as_deref() != Some(std::ffi::OsStr::new(WORKER_ARGUMENT)) {
        return false;
    }
    if arguments.next().is_some() {
        return true;
    }
    std::panic::set_hook(Box::new(|_| {}));
    // A private worker is expendable: malformed stdin or a stuck local/network
    // subsystem must not leave an orphan running indefinitely.
    if thread::Builder::new()
        .name("pax-network-deadline".into())
        .spawn(|| {
            thread::sleep(WORKER_LIFETIME);
            std::process::exit(0);
        })
        .is_err()
    {
        return true;
    }
    let _ = std::panic::catch_unwind(|| {
        let mut message = Vec::new();
        std::io::stdin()
            .take(MAX_MESSAGE_BYTES + 1)
            .read_to_end(&mut message)?;
        if message.len() as u64 > MAX_MESSAGE_BYTES {
            return Ok::<_, io::Error>(());
        }
        let delivery: TelemetryDelivery = serde_json::from_slice(&message)?;
        crate::telemetry::with_delivery_permission(&delivery.installation_id, || {
            let request = TelemetryRequest {
                installation_id: delivery.installation_id.clone(),
                cli_version: env!("CARGO_PKG_VERSION").to_owned(),
                host_os: host_os(),
                host_arch: host_arch(),
                event: delivery.event,
            };
            if let Ok(client) = client() {
                let _ = client
                    .post(api_url(&api_base_url(), CLI_TELEMETRY_PATH))
                    .json(&request)
                    .send();
            }
        });
        Ok(())
    });
    true
}

fn client() -> reqwest::Result<Client> {
    Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .user_agent(user_agent())
        .build()
}

fn latest_version(base_url: &str) -> Option<String> {
    let response = client()
        .ok()?
        .get(api_url(base_url, CLI_LATEST_RELEASE_PATH))
        .send()
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let mut bytes = Vec::new();
    response
        .take(MAX_MESSAGE_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_MESSAGE_BYTES {
        return None;
    }
    let latest: LatestReleaseResponse = serde_json::from_slice(&bytes).ok()?;
    let current = Version::parse(env!("CARGO_PKG_VERSION")).ok()?;
    let candidate = Version::parse(&latest.latest_version).ok()?;
    (candidate > current).then_some(latest.latest_version)
}

fn api_base_url() -> String {
    std::env::var("PAX_API_BASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_API_BASE_URL.to_owned())
}

fn api_url(base: &str, path: &str) -> String {
    format!("{}{}", base.trim_end_matches('/'), path)
}

fn user_agent() -> String {
    format!("pax-cli/{}", env!("CARGO_PKG_VERSION"))
}

fn host_os() -> HostOs {
    match std::env::consts::OS {
        "macos" => HostOs::Macos,
        "linux" => HostOs::Linux,
        "windows" => HostOs::Windows,
        _ => HostOs::Other,
    }
}

fn host_arch() -> HostArch {
    match std::env::consts::ARCH {
        "x86_64" => HostArch::X86_64,
        "aarch64" => HostArch::Aarch64,
        _ => HostArch::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_url_has_one_separator() {
        assert_eq!(
            api_url("https://pub.pax.dev/", CLI_LATEST_RELEASE_PATH),
            "https://pub.pax.dev/v1/cli/releases/latest"
        );
    }

    #[test]
    fn user_agent_has_no_locale() {
        assert_eq!(
            user_agent(),
            format!("pax-cli/{}", env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn worker_restores_caller_settings_over_project_environment() {
        let sender = TelemetrySender::new("3f028ebe-ea4d-4fd3-9c1a-f722f736991b".into());
        let mut command = Command::new("unused-test-command");
        for (key, _) in &sender.environment {
            command.env(key, "project-override");
        }
        apply_environment(&mut command, &sender.environment);
        for (key, expected) in &sender.environment {
            let configured = command
                .get_envs()
                .find(|(name, _)| *name == std::ffi::OsStr::new(key))
                .unwrap()
                .1;
            assert_eq!(configured, expected.as_deref(), "{key}");
        }
        assert_eq!(
            sender.environment.len(),
            crate::telemetry::CONFIG_ENV_KEYS.len() + 1
        );
    }

    #[test]
    fn worker_handoff_is_closed_and_has_no_project_fields() {
        let event = TelemetryEvent::RunReady {
            target: pax_message::http_api::Target::Web,
        };
        let message = TelemetryDelivery {
            installation_id: "3f028ebe-ea4d-4fd3-9c1a-f722f736991b".into(),
            event,
        };
        let mut json = serde_json::to_value(&message).unwrap();
        assert!(serde_json::to_vec(&message).unwrap().len() < 1024);
        json["project_path"] = "/private/project".into();
        assert!(serde_json::from_value::<TelemetryDelivery>(json).is_err());
    }
}
