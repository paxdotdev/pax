use clap::{App, ArgMatches, SubCommand};
use color_eyre::eyre::{eyre, Report, Result};
use pax_compiler::{RunLifecycleObserver, RunTarget};
use pax_message::http_api::{CommandFamily, CommandOutcome, Target, TelemetryEvent};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use uuid::Uuid;

const NOTICE_VERSION: u32 = 2;
const STATE_LOCK_TIMEOUT: Duration = Duration::from_millis(250);
const DELIVERY_STATE_LOCK_TIMEOUT: Duration = Duration::from_millis(25);
const STATE_LOCK_RETRY: Duration = Duration::from_millis(2);
// An explicit opt-out must be able to outwait an already-authorized delivery.
const MANAGEMENT_LOCK_TIMEOUT: Duration = Duration::from_millis(2500);

// Keep worker consent/state selection tied to the invoking shell, even if a
// compiler later loads a project's .env file into the foreground process.
pub(crate) const CONFIG_ENV_KEYS: &[&str] = &[
    "HOME",
    "USERPROFILE",
    "LOCALAPPDATA",
    "APPDATA",
    "XDG_STATE_HOME",
    "HOMEDRIVE",
    "HOMEPATH",
    "PAX_TELEMETRY",
    "DO_NOT_TRACK",
    "CI",
];

const NOTICE: &str = "Pax collects minimal CLI telemetry after this command to guide aggregate product, content, outreach, and marketing investment: the command, target, success or failure, CLI version, OS, architecture, and approximate city, region, and country derived from the connection IP. A random installation ID connects events over time and is not tied to an account, hardware, or project. This command sends no telemetry. The CLI sends no location or IP field; the telemetry service passes the connection IP to Mixpanel only to derive coarse location, and the raw IP is not retained. Pax never collects source, filenames, project paths, project content, command arguments, or error text. Run `pax-cli telemetry off` at any time. Learn more: https://docs.pax.dev/cli-telemetry";

const STATE_DIR_NAME: &str = "telemetry";
const LOCK_FILE_NAME: &str = "state.lock";
const DELIVERY_LOCK_FILE_NAME: &str = "delivery.lock";
const ENABLED_FILE_NAME: &str = "installation.json";
const DISABLED_FILE_NAME: &str = "disabled";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PublicCommand {
    pub family: CommandFamily,
    pub target: Option<Target>,
}

pub fn public_command(matches: &ArgMatches<'_>) -> Option<PublicCommand> {
    let command = match matches.subcommand() {
        ("create", Some(_)) => PublicCommand {
            family: CommandFamily::Create,
            target: None,
        },
        ("run", Some(args)) => PublicCommand {
            family: CommandFamily::Run,
            target: target_from_str(args.value_of("target")?),
        },
        ("build", Some(args)) => PublicCommand {
            family: CommandFamily::Build,
            target: target_from_str(args.value_of("target")?),
        },
        ("clean", Some(_)) => PublicCommand {
            family: CommandFamily::Clean,
            target: None,
        },
        ("eject", Some(args)) => PublicCommand {
            family: CommandFamily::Eject,
            target: target_from_str(args.value_of("target")?),
        },
        ("format", Some(_)) => PublicCommand {
            family: CommandFamily::Format,
            target: None,
        },
        ("lsp", Some(_)) => PublicCommand {
            family: CommandFamily::Lsp,
            target: None,
        },
        ("docs", Some(_)) => PublicCommand {
            family: CommandFamily::Docs,
            target: None,
        },
        ("dev", Some(_)) => PublicCommand {
            family: CommandFamily::Dev,
            target: None,
        },
        ("svg-import", Some(_)) => PublicCommand {
            family: CommandFamily::SvgImport,
            target: None,
        },
        _ => return None,
    };
    Some(command)
}

fn target_from_str(value: &str) -> Option<Target> {
    match value.to_ascii_lowercase().as_str() {
        "web" => Some(Target::Web),
        "macos" => Some(Target::Macos),
        "ios" => Some(Target::Ios),
        "ipados" | "ipad" => Some(Target::Ipados),
        _ => None,
    }
}

fn target_from_run_target(value: RunTarget) -> Target {
    match value {
        RunTarget::Web => Target::Web,
        RunTarget::macOS => Target::Macos,
        RunTarget::iOS => Target::Ios,
        RunTarget::iPadOS => Target::Ipados,
    }
}

pub fn command() -> App<'static, 'static> {
    SubCommand::with_name("telemetry")
        .about("View or change CLI telemetry")
        .subcommand(SubCommand::with_name("off").about("Disable CLI telemetry"))
        .subcommand(
            SubCommand::with_name("on").about("Enable CLI telemetry with a new installation ID"),
        )
        .subcommand(
            SubCommand::with_name("status").about("Show the effective CLI telemetry setting"),
        )
}

pub fn handle(args: &ArgMatches<'_>) -> Result<(), Report> {
    let store = StateStore::global().map_err(|error| eyre!(error))?;
    match args.subcommand() {
        ("off", Some(_)) => {
            store.disable().map_err(|error| eyre!(error))?;
            println!("Pax CLI telemetry is off.");
        }
        ("on", Some(_)) => {
            store.enable_new().map_err(|error| eyre!(error))?;
            println!("Pax CLI telemetry is on with a new random installation ID.");
        }
        ("status", Some(_)) | ("", None) => print_status(&store)?,
        _ => return Err(eyre!("expected `pax-cli telemetry on`, `off`, or `status`")),
    }
    Ok(())
}

fn print_status(store: &StateStore) -> Result<(), Report> {
    let state = store.read().map_err(|error| eyre!(error))?;
    if let Some(reason) = EnvironmentPolicy::current().suppression_reason() {
        println!("Pax CLI telemetry is off for this process ({reason}).");
        return Ok(());
    }
    match state {
        StoredState::Absent => println!("Pax CLI telemetry is not initialized. The next public command will show the privacy notice and send no telemetry; later commands default on."),
        StoredState::Enabled(_) => println!("Pax CLI telemetry is on."),
        StoredState::Disabled => println!("Pax CLI telemetry is off."),
    }
    Ok(())
}

#[derive(Clone)]
pub struct TelemetrySession {
    sender: Option<crate::network::TelemetrySender>,
    run_ready_observed: Arc<AtomicBool>,
}

impl TelemetrySession {
    pub fn disabled() -> Self {
        Self {
            sender: None,
            run_ready_observed: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn for_public_command() -> Self {
        if EnvironmentPolicy::current().suppression_reason().is_some() {
            return Self::disabled();
        }

        let Ok(store) = StateStore::global() else {
            return Self::disabled();
        };
        match store.prepare_public_command(|| eprintln!("\n{NOTICE}\n")) {
            Ok(Preparation::Enabled(installation_id)) => Self {
                sender: Some(crate::network::TelemetrySender::new(installation_id)),
                run_ready_observed: Arc::new(AtomicBool::new(false)),
            },
            Ok(Preparation::FirstCommandNoSend | Preparation::Disabled) | Err(_) => {
                Self::disabled()
            }
        }
    }

    pub fn command_finished(&self, command: PublicCommand, succeeded: bool) {
        let event = terminal_event(
            command,
            succeeded,
            self.run_ready_observed.load(AtomicOrdering::Acquire),
        );
        if let (Some(sender), Some(event)) = (&self.sender, event) {
            sender.send(event);
        }
    }

    pub fn lifecycle_observer(&self) -> Option<Arc<dyn RunLifecycleObserver>> {
        self.sender.as_ref().map(|sender| {
            Arc::new(CliRunLifecycleObserver {
                sender: sender.clone(),
                run_ready_observed: Arc::clone(&self.run_ready_observed),
            }) as Arc<dyn RunLifecycleObserver>
        })
    }
}

fn terminal_event(
    command: PublicCommand,
    succeeded: bool,
    run_ready_observed: bool,
) -> Option<TelemetryEvent> {
    if command.family == CommandFamily::Run && run_ready_observed {
        return None;
    }
    Some(TelemetryEvent::CommandOutcome {
        command: command.family,
        target: command.target,
        outcome: if succeeded {
            CommandOutcome::Succeeded
        } else {
            CommandOutcome::Failed
        },
    })
}

struct CliRunLifecycleObserver {
    sender: crate::network::TelemetrySender,
    run_ready_observed: Arc<AtomicBool>,
}

impl RunLifecycleObserver for CliRunLifecycleObserver {
    fn run_ready(&self, target: RunTarget) {
        if self.run_ready_observed.swap(true, AtomicOrdering::AcqRel) {
            return;
        }
        self.sender.send(TelemetryEvent::RunReady {
            target: target_from_run_target(target),
        });
    }
}

pub(crate) fn with_delivery_permission(installation_id: &str, deliver: impl FnOnce()) {
    if EnvironmentPolicy::current().suppression_reason().is_some() {
        return;
    }
    if let Ok(store) = StateStore::global() {
        let _ = store.deliver_if_enabled(installation_id, deliver);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum StoredState {
    Absent,
    Enabled(EnabledState),
    Disabled,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EnabledState {
    schema_version: u32,
    notice_version: u32,
    installation_id: String,
}

enum Preparation {
    FirstCommandNoSend,
    Enabled(String),
    Disabled,
}

#[derive(Clone)]
struct StateStore {
    root: PathBuf,
}

impl StateStore {
    fn global() -> io::Result<Self> {
        Ok(Self {
            root: global_state_root()?.join(STATE_DIR_NAME),
        })
    }

    #[cfg(test)]
    fn at(root: PathBuf) -> Self {
        Self { root }
    }

    fn read(&self) -> io::Result<StoredState> {
        self.with_lock(|store| store.read_locked())
    }

    fn prepare_public_command(&self, show_notice: impl FnOnce()) -> io::Result<Preparation> {
        self.with_lock(|store| match store.read_locked()? {
            StoredState::Disabled => Ok(Preparation::Disabled),
            StoredState::Enabled(state) if state.notice_version == NOTICE_VERSION => {
                Ok(Preparation::Enabled(state.installation_id))
            }
            StoredState::Enabled(mut state) => {
                show_notice();
                state.notice_version = NOTICE_VERSION;
                store.write_enabled_locked(&state)?;
                Ok(Preparation::FirstCommandNoSend)
            }
            StoredState::Absent => {
                show_notice();
                store.write_enabled_locked(&EnabledState {
                    schema_version: 1,
                    notice_version: NOTICE_VERSION,
                    installation_id: Uuid::new_v4().to_string(),
                })?;
                Ok(Preparation::FirstCommandNoSend)
            }
        })
    }

    fn disable(&self) -> io::Result<()> {
        self.with_lock(|store| {
            store.write_disabled_locked()?;
            remove_if_exists(&store.root.join(ENABLED_FILE_NAME))
        })?;
        self.drain_deliveries()
    }

    fn enable_new(&self) -> io::Result<()> {
        self.with_lock(|store| {
            store.write_enabled_locked(&EnabledState {
                schema_version: 1,
                notice_version: NOTICE_VERSION,
                installation_id: Uuid::new_v4().to_string(),
            })
        })?;
        self.drain_deliveries()
    }

    fn deliver_if_enabled(
        &self,
        expected_installation_id: &str,
        deliver: impl FnOnce(),
    ) -> io::Result<bool> {
        // Shared delivery locks allow independent workers to send concurrently.
        // Only management commands take this lock exclusively; ordinary state
        // reads/first-run preparation never wait for the network.
        let gate = self.open_lock(DELIVERY_LOCK_FILE_NAME)?;
        lock_bounded(&gate, DELIVERY_STATE_LOCK_TIMEOUT, true)?;
        let authorized = self.with_lock(|store| {
            Ok(matches!(
                store.read_locked()?,
                StoredState::Enabled(state)
                    if state.notice_version == NOTICE_VERSION
                        && state.installation_id == expected_installation_id
            ))
        })?;
        if authorized {
            deliver();
        }
        Ok(authorized)
    }

    fn drain_deliveries(&self) -> io::Result<()> {
        // Consent was already changed under the state lock. Release that lock
        // before taking the gate exclusively, so workers cannot deadlock with
        // management and no stale worker can authorize a new send.
        let gate = self.open_lock(DELIVERY_LOCK_FILE_NAME)?;
        lock_bounded(&gate, MANAGEMENT_LOCK_TIMEOUT, false)
    }

    fn with_lock<T>(&self, action: impl FnOnce(&Self) -> io::Result<T>) -> io::Result<T> {
        self.with_lock_timeout(STATE_LOCK_TIMEOUT, action)
    }

    fn with_lock_timeout<T>(
        &self,
        timeout: Duration,
        action: impl FnOnce(&Self) -> io::Result<T>,
    ) -> io::Result<T> {
        let lock = self.open_lock(LOCK_FILE_NAME)?;
        lock_bounded(&lock, timeout, false)?;
        action(self)
    }

    fn open_lock(&self, name: &str) -> io::Result<File> {
        fs::create_dir_all(&self.root)?;
        secure_directory(&self.root)?;
        secure_open_options()
            .create(true)
            .read(true)
            .write(true)
            .open(self.root.join(name))
    }

    fn read_locked(&self) -> io::Result<StoredState> {
        if self.root.join(DISABLED_FILE_NAME).exists() {
            return Ok(StoredState::Disabled);
        }
        let enabled_path = self.root.join(ENABLED_FILE_NAME);
        if !enabled_path.exists() {
            return Ok(StoredState::Absent);
        }
        let bytes = fs::read(enabled_path)?;
        let state: EnabledState = match serde_json::from_slice(&bytes) {
            Ok(state) => state,
            Err(_) => return Ok(StoredState::Absent),
        };
        if state.schema_version != 1 || Uuid::parse_str(&state.installation_id).is_err() {
            return Ok(StoredState::Absent);
        }
        Ok(StoredState::Enabled(state))
    }

    fn write_enabled_locked(&self, state: &EnabledState) -> io::Result<()> {
        let temporary_path = self
            .root
            .join(format!(".installation-{}.tmp", Uuid::new_v4()));
        let mut temporary = secure_open_options()
            .create_new(true)
            .write(true)
            .open(&temporary_path)?;
        serde_json::to_writer(&mut temporary, state).map_err(io::Error::other)?;
        temporary.write_all(b"\n")?;
        temporary.sync_all()?;

        let enabled_path = self.root.join(ENABLED_FILE_NAME);
        remove_if_exists(&enabled_path)?;
        if let Err(error) = fs::rename(&temporary_path, &enabled_path) {
            let _ = fs::remove_file(&temporary_path);
            return Err(error);
        }
        remove_if_exists(&self.root.join(DISABLED_FILE_NAME))
    }

    fn write_disabled_locked(&self) -> io::Result<()> {
        let temporary_path = self.root.join(format!(".disabled-{}.tmp", Uuid::new_v4()));
        let mut temporary = secure_open_options()
            .create_new(true)
            .write(true)
            .open(&temporary_path)?;
        temporary.write_all(b"disabled\n")?;
        temporary.sync_all()?;

        let disabled_path = self.root.join(DISABLED_FILE_NAME);
        remove_if_exists(&disabled_path)?;
        if let Err(error) = fs::rename(&temporary_path, &disabled_path) {
            let _ = fs::remove_file(&temporary_path);
            return Err(error);
        }
        Ok(())
    }
}

fn lock_bounded(lock: &File, timeout: Duration, shared: bool) -> io::Result<()> {
    let started = Instant::now();
    loop {
        match if shared {
            lock.try_lock_shared()
        } else {
            lock.try_lock()
        } {
            Ok(()) => return Ok(()),
            Err(std::fs::TryLockError::WouldBlock) => {
                let elapsed = started.elapsed();
                if elapsed >= timeout {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "timed out waiting for the telemetry state lock",
                    ));
                }
                thread::sleep(STATE_LOCK_RETRY.min(timeout.saturating_sub(elapsed)));
            }
            Err(std::fs::TryLockError::Error(error)) => return Err(error),
        }
    }
}

fn remove_if_exists(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(unix)]
fn secure_open_options() -> OpenOptions {
    use std::os::unix::fs::OpenOptionsExt;

    let mut options = OpenOptions::new();
    options.mode(0o600);
    options
}

#[cfg(not(unix))]
fn secure_open_options() -> OpenOptions {
    OpenOptions::new()
}

#[cfg(unix)]
fn secure_directory(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

#[cfg(not(unix))]
fn secure_directory(_path: &Path) -> io::Result<()> {
    Ok(())
}

fn global_state_root() -> io::Result<PathBuf> {
    if cfg!(target_os = "macos") {
        return Ok(user_home_dir()?
            .join("Library")
            .join("Application Support")
            .join("Pax"));
    }
    if cfg!(target_os = "windows") {
        if let Some(local_app_data) = absolute_env_path("LOCALAPPDATA")? {
            return Ok(local_app_data.join("Pax"));
        }
        if let Some(app_data) = absolute_env_path("APPDATA")? {
            return Ok(app_data.join("Pax"));
        }
        return Ok(user_home_dir()?.join("AppData").join("Local").join("Pax"));
    }
    if let Some(xdg_state_home) = absolute_env_path("XDG_STATE_HOME")? {
        return Ok(xdg_state_home.join("pax"));
    }
    Ok(user_home_dir()?.join(".local").join("state").join("pax"))
}

fn user_home_dir() -> io::Result<PathBuf> {
    if let Some(home) = absolute_env_path("HOME")? {
        return Ok(home);
    }
    if let Some(user_profile) = absolute_env_path("USERPROFILE")? {
        return Ok(user_profile);
    }
    if let (Some(drive), Some(path)) = (std::env::var_os("HOMEDRIVE"), std::env::var_os("HOMEPATH"))
    {
        return validated_absolute_state_root(
            PathBuf::from(drive).join(path),
            "HOMEDRIVE/HOMEPATH",
        );
    }
    Err(io::Error::other(
        "could not determine the current user's home directory",
    ))
}

fn absolute_env_path(variable: &str) -> io::Result<Option<PathBuf>> {
    std::env::var_os(variable)
        .map(|value| validated_absolute_state_root(PathBuf::from(value), variable))
        .transpose()
}

fn validated_absolute_state_root(path: PathBuf, variable: &str) -> io::Result<PathBuf> {
    if path.as_os_str().is_empty() || !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{variable} must contain a non-empty absolute path"),
        ));
    }
    Ok(path)
}

struct EnvironmentPolicy {
    pax_telemetry_off: bool,
    do_not_track: bool,
    continuous_integration: bool,
}

impl EnvironmentPolicy {
    fn current() -> Self {
        Self {
            pax_telemetry_off: std::env::var("PAX_TELEMETRY")
                .is_ok_and(|value| value.eq_ignore_ascii_case("off")),
            do_not_track: std::env::var("DO_NOT_TRACK").is_ok_and(|value| is_truthy(&value)),
            continuous_integration: std::env::var("CI").is_ok_and(|value| is_truthy(&value)),
        }
    }

    fn suppression_reason(&self) -> Option<&'static str> {
        if self.do_not_track {
            Some("DO_NOT_TRACK")
        } else if self.pax_telemetry_off {
            Some("PAX_TELEMETRY=off")
        } else if self.continuous_integration {
            Some("CI environment")
        } else {
            None
        }
    }
}

fn is_truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Arg;
    use pax_message::http_api::{HostArch, HostOs, TelemetryRequest};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{mpsc, Barrier};

    #[test]
    fn first_public_command_enables_state_but_sends_nothing() {
        let directory = tempfile::tempdir().unwrap();
        let store = StateStore::at(directory.path().join("telemetry"));
        let notices = AtomicUsize::new(0);

        let first = store
            .prepare_public_command(|| {
                notices.fetch_add(1, Ordering::SeqCst);
            })
            .unwrap();
        assert!(matches!(first, Preparation::FirstCommandNoSend));
        assert_eq!(notices.load(Ordering::SeqCst), 1);

        let second = store
            .prepare_public_command(|| panic!("notice repeated"))
            .unwrap();
        let Preparation::Enabled(installation_id) = second else {
            panic!("second public command should be enabled");
        };
        assert!(Uuid::parse_str(&installation_id).is_ok());
    }

    #[test]
    fn materially_expanded_notice_refreshes_without_sending() {
        let directory = tempfile::tempdir().unwrap();
        let store = StateStore::at(directory.path().join("telemetry"));
        let installation_id = "3f028ebe-ea4d-4fd3-9c1a-f722f736991b".to_owned();
        store
            .with_lock(|store| {
                store.write_enabled_locked(&EnabledState {
                    schema_version: 1,
                    notice_version: NOTICE_VERSION - 1,
                    installation_id: installation_id.clone(),
                })
            })
            .unwrap();

        let notices = AtomicUsize::new(0);
        assert!(matches!(
            store
                .prepare_public_command(|| {
                    notices.fetch_add(1, Ordering::SeqCst);
                })
                .unwrap(),
            Preparation::FirstCommandNoSend
        ));
        assert_eq!(notices.load(Ordering::SeqCst), 1);

        let StoredState::Enabled(refreshed) = store.read().unwrap() else {
            panic!("notice refresh should preserve the enabled installation");
        };
        assert_eq!(refreshed.notice_version, NOTICE_VERSION);
        assert_eq!(refreshed.installation_id, installation_id);
        assert!(matches!(
            store
                .prepare_public_command(|| panic!("refreshed notice repeated"))
                .unwrap(),
            Preparation::Enabled(id) if id == installation_id
        ));

        assert!(NOTICE.contains("approximate city, region, and country"));
        assert!(NOTICE.contains("marketing investment"));
        assert!(NOTICE.contains("raw IP is not retained"));
    }

    #[test]
    fn concurrent_first_commands_have_one_notice_only_winner() {
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(StateStore::at(directory.path().join("telemetry")));
        let notices = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(Barrier::new(2));

        let mut threads = Vec::new();
        for _ in 0..2 {
            let store = Arc::clone(&store);
            let notices = Arc::clone(&notices);
            let barrier = Arc::clone(&barrier);
            threads.push(thread::spawn(move || {
                barrier.wait();
                store
                    .prepare_public_command(|| {
                        notices.fetch_add(1, Ordering::SeqCst);
                    })
                    .unwrap()
            }));
        }

        let results: Vec<_> = threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .collect();
        assert_eq!(notices.load(Ordering::SeqCst), 1);
        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(result, Preparation::FirstCommandNoSend))
                .count(),
            1
        );
        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(result, Preparation::Enabled(_)))
                .count(),
            1
        );
    }

    #[test]
    fn explicit_on_creates_a_fresh_enabled_installation() {
        let directory = tempfile::tempdir().unwrap();
        let store = StateStore::at(directory.path().join("telemetry"));
        store.prepare_public_command(|| {}).unwrap();
        let StoredState::Enabled(first) = store.read().unwrap() else {
            panic!("first command should persist an enabled installation");
        };
        store.disable().unwrap();
        assert_eq!(store.read().unwrap(), StoredState::Disabled);

        store.enable_new().unwrap();
        let StoredState::Enabled(second) = store.read().unwrap() else {
            panic!("explicit on should immediately enable a new installation");
        };
        assert_ne!(first.installation_id, second.installation_id);
        assert!(matches!(
            store
                .prepare_public_command(|| panic!("explicit on already records notice consent"))
                .unwrap(),
            Preparation::Enabled(_)
        ));
    }

    #[test]
    fn disabled_marker_wins_over_stale_installation_state() {
        let directory = tempfile::tempdir().unwrap();
        let store = StateStore::at(directory.path().join("telemetry"));
        store.prepare_public_command(|| {}).unwrap();
        store.disable().unwrap();
        fs::write(store.root.join(ENABLED_FILE_NAME), b"not valid json").unwrap();
        assert_eq!(store.read().unwrap(), StoredState::Disabled);
    }

    #[cfg(unix)]
    #[test]
    fn disabling_replaces_a_sentinel_symlink_without_following_it() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().unwrap();
        let store = StateStore::at(directory.path().join("telemetry"));
        fs::create_dir_all(&store.root).unwrap();
        let unrelated_file = directory.path().join("unrelated.txt");
        fs::write(&unrelated_file, b"preserve me").unwrap();
        symlink(&unrelated_file, store.root.join(DISABLED_FILE_NAME)).unwrap();

        store.disable().unwrap();

        assert_eq!(fs::read(&unrelated_file).unwrap(), b"preserve me");
        assert_eq!(
            fs::read(store.root.join(DISABLED_FILE_NAME)).unwrap(),
            b"disabled\n"
        );
        assert!(!fs::symlink_metadata(store.root.join(DISABLED_FILE_NAME))
            .unwrap()
            .file_type()
            .is_symlink());
    }

    #[test]
    fn later_opt_out_revokes_an_existing_delivery_session() {
        let directory = tempfile::tempdir().unwrap();
        let store = StateStore::at(directory.path().join("telemetry"));
        store.prepare_public_command(|| {}).unwrap();
        let Preparation::Enabled(installation_id) = store.prepare_public_command(|| {}).unwrap()
        else {
            panic!("the second command should have an enabled installation");
        };
        let delivered = AtomicBool::new(false);
        assert!(store
            .deliver_if_enabled(&installation_id, || {
                delivered.store(true, Ordering::SeqCst);
            })
            .unwrap());
        assert!(delivered.load(Ordering::SeqCst));

        store.disable().unwrap();
        assert!(!store
            .deliver_if_enabled(&installation_id, || panic!("revoked ID delivered"))
            .unwrap());

        store.enable_new().unwrap();
        assert!(!store
            .deliver_if_enabled(&installation_id, || panic!("rotated ID delivered"))
            .unwrap());
    }

    #[test]
    fn opt_out_cannot_return_before_an_authorized_delivery_completes() {
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(StateStore::at(directory.path().join("telemetry")));
        store.prepare_public_command(|| {}).unwrap();
        let Preparation::Enabled(installation_id) = store.prepare_public_command(|| {}).unwrap()
        else {
            panic!("the second command should have an enabled installation");
        };

        let (delivery_started_sender, delivery_started_receiver) = mpsc::sync_channel(1);
        let (release_delivery_sender, release_delivery_receiver) = mpsc::sync_channel(1);
        let (delivery_finished_sender, delivery_finished_receiver) = mpsc::sync_channel(1);
        let delivery_store = Arc::clone(&store);
        let delivery_id = installation_id.clone();
        let delivery_thread = thread::spawn(move || {
            assert!(delivery_store
                .deliver_if_enabled(&delivery_id, || {
                    delivery_started_sender.send(()).unwrap();
                    release_delivery_receiver.recv().unwrap();
                    delivery_finished_sender.send(()).unwrap();
                })
                .unwrap());
        });
        delivery_started_receiver.recv().unwrap();

        // Network delivery does not own the state lock or serialize other
        // workers. A rapid create→run sequence must retain both events.
        assert!(matches!(store.read().unwrap(), StoredState::Enabled(state)
            if state.installation_id == installation_id));
        assert!(matches!(
            store
                .prepare_public_command(|| panic!("notice repeated"))
                .unwrap(),
            Preparation::Enabled(_)
        ));
        assert!(store.deliver_if_enabled(&installation_id, || {}).unwrap());

        let (off_attempting_sender, off_attempting_receiver) = mpsc::sync_channel(1);
        let (off_returned_sender, off_returned_receiver) = mpsc::sync_channel(1);
        let off_store = Arc::clone(&store);
        let off_thread = thread::spawn(move || {
            off_attempting_sender.send(()).unwrap();
            off_store.disable().unwrap();
            assert!(delivery_finished_receiver.try_recv().is_ok());
            off_returned_sender.send(()).unwrap();
        });
        off_attempting_receiver.recv().unwrap();
        assert_eq!(
            // Only the explicit management command waits for old requests.
            off_returned_receiver.recv_timeout(Duration::from_millis(400)),
            Err(mpsc::RecvTimeoutError::Timeout)
        );
        assert_eq!(store.read().unwrap(), StoredState::Disabled);

        release_delivery_sender.send(()).unwrap();
        off_returned_receiver
            .recv_timeout(Duration::from_secs(1))
            .unwrap();
        delivery_thread.join().unwrap();
        off_thread.join().unwrap();

        assert!(!store
            .deliver_if_enabled(&installation_id, || panic!(
                "delivery occurred after opt-out"
            ))
            .unwrap());
    }

    #[test]
    fn telemetry_state_lock_contention_times_out() {
        let directory = tempfile::tempdir().unwrap();
        let store = StateStore::at(directory.path().join("telemetry"));
        fs::create_dir_all(&store.root).unwrap();
        let held_lock = secure_open_options()
            .create(true)
            .read(true)
            .write(true)
            .open(store.root.join(LOCK_FILE_NAME))
            .unwrap();
        held_lock.lock().unwrap();

        let timeout = Duration::from_millis(20);
        let started = Instant::now();
        let error = store.with_lock_timeout(timeout, |_| Ok(())).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert!(started.elapsed() >= timeout);
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn state_root_paths_must_be_non_empty_and_absolute() {
        assert_eq!(
            validated_absolute_state_root(PathBuf::new(), "TEST_STATE_ROOT")
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(
            validated_absolute_state_root(PathBuf::from("relative/state"), "TEST_STATE_ROOT")
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );

        let absolute = std::env::current_dir().unwrap().join("telemetry-state");
        assert_eq!(
            validated_absolute_state_root(absolute.clone(), "TEST_STATE_ROOT").unwrap(),
            absolute
        );
    }

    #[test]
    fn environment_policy_is_strict_and_ci_defaults_off() {
        assert!(is_truthy("1"));
        assert!(is_truthy("TRUE"));
        assert!(!is_truthy("0"));
        assert!(!is_truthy("sometimes"));
        assert_eq!(
            EnvironmentPolicy {
                pax_telemetry_off: false,
                do_not_track: false,
                continuous_integration: true,
            }
            .suppression_reason(),
            Some("CI environment")
        );
    }

    #[test]
    fn target_alias_is_coarsened_without_project_data() {
        assert_eq!(target_from_str("ipad"), Some(Target::Ipados));
        assert_eq!(target_from_str("web"), Some(Target::Web));
        assert_eq!(target_from_str("project/private"), None);
    }

    #[test]
    fn run_outcome_does_not_invent_or_duplicate_readiness() {
        let run = PublicCommand {
            family: CommandFamily::Run,
            target: Some(Target::Web),
        };
        assert_eq!(terminal_event(run, true, true), None);
        assert_eq!(terminal_event(run, false, true), None);
        assert_eq!(
            terminal_event(run, true, false),
            Some(TelemetryEvent::CommandOutcome {
                command: CommandFamily::Run,
                target: Some(Target::Web),
                outcome: CommandOutcome::Succeeded,
            })
        );
        assert_eq!(
            terminal_event(run, false, false),
            Some(TelemetryEvent::CommandOutcome {
                command: CommandFamily::Run,
                target: Some(Target::Web),
                outcome: CommandOutcome::Failed,
            })
        );

        let build = PublicCommand {
            family: CommandFamily::Build,
            target: Some(Target::Web),
        };
        assert_eq!(
            terminal_event(build, true, false),
            Some(TelemetryEvent::CommandOutcome {
                command: CommandFamily::Build,
                target: Some(Target::Web),
                outcome: CommandOutcome::Succeeded,
            })
        );
    }

    #[test]
    fn only_public_command_families_are_eligible() {
        let app = App::new("pax")
            .subcommand(App::new("create").alias("new"))
            .subcommand(
                App::new("run").arg(
                    Arg::with_name("target")
                        .long("target")
                        .takes_value(true)
                        .default_value("web"),
                ),
            )
            .subcommand(App::new("telemetry"))
            .subcommand(App::new("libdev"))
            .subcommand(App::new("designtime-server"));

        let create = app.clone().get_matches_from(vec!["pax", "new"]);
        assert_eq!(
            public_command(&create),
            Some(PublicCommand {
                family: CommandFamily::Create,
                target: None,
            })
        );
        let run = app
            .clone()
            .get_matches_from(vec!["pax", "run", "--target", "web"]);
        assert_eq!(
            public_command(&run),
            Some(PublicCommand {
                family: CommandFamily::Run,
                target: Some(Target::Web),
            })
        );
        for name in ["telemetry", "libdev", "designtime-server"] {
            let matches = app.clone().get_matches_from(vec!["pax", name]);
            assert_eq!(public_command(&matches), None, "{name} must be excluded");
        }
    }

    #[test]
    fn command_delivery_contains_only_the_closed_contract() {
        let request = TelemetryRequest {
            installation_id: "3f028ebe-ea4d-4fd3-9c1a-f722f736991b".to_owned(),
            cli_version: "0.38.3".to_owned(),
            host_os: HostOs::Macos,
            host_arch: HostArch::Aarch64,
            event: TelemetryEvent::CommandOutcome {
                command: CommandFamily::Build,
                target: Some(Target::Web),
                outcome: CommandOutcome::Failed,
            },
        };

        let body = serde_json::to_string(&request).unwrap();
        assert!(body.contains("\"command\":\"build\""));
        assert!(body.contains("\"outcome\":\"failed\""));
        assert!(!body.contains("/private/should-never-leak"));
    }

    #[test]
    fn opt_out_can_outwait_a_network_request() {
        assert!(crate::network::REQUEST_TIMEOUT < MANAGEMENT_LOCK_TIMEOUT);
    }
}
