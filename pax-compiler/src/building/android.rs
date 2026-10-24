//! Android packages retain Activity lifecycle ownership and a separate cartridge.
use crate::{project_layout, BuildTimings, RunContext};
use color_eyre::eyre::{eyre, Result};
use pax_manifest::PaxManifest;
use std::time::Duration;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) const TRIPLE: &str = "aarch64-linux-android";

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(default, deny_unknown_fields)]
struct AndroidSettings {
    application_id: Option<String>,
    version_code: u32,
    min_sdk: u32,
    target_sdk: u32,
    compile_sdk: u32,
    build_tools: String,
    permissions: Vec<String>,
    icon: Option<PathBuf>,
    system_bar_icons: Option<String>,
    format: String,
}
impl Default for AndroidSettings {
    fn default() -> Self {
        Self {
            application_id: None,
            version_code: 1,
            min_sdk: 29,
            target_sdk: 36,
            compile_sdk: 36,
            build_tools: "36.0.0".into(),
            permissions: vec!["android.permission.INTERNET".into()],
            icon: None,
            system_bar_icons: None,
            format: "apk".into(),
        }
    }
}
impl AndroidSettings {
    fn parse(metadata: &serde_json::Value) -> Result<Self> {
        let settings: Self = serde_json::from_value(
            metadata
                .pointer("/pax/android")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({})),
        )
        .map_err(|error| eyre!("Invalid package.metadata.pax.android: {error}"))?;
        if let Some(id) = &settings.application_id {
            validate_application_id(id)?;
        }
        if !(1..=2_100_000_000).contains(&settings.version_code) {
            return Err(eyre!(
                "android.version_code must be between 1 and 2100000000"
            ));
        }
        if settings.min_sdk < 29
            || settings.min_sdk > settings.target_sdk
            || settings.target_sdk > settings.compile_sdk
        {
            return Err(eyre!(
                "Android SDK versions must satisfy 29 <= min_sdk <= target_sdk <= compile_sdk"
            ));
        }
        if settings.build_tools.split('.').count() != 3
            || !settings
                .build_tools
                .split('.')
                .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err(eyre!(
                "android.build_tools must be an installed version such as 36.0.0"
            ));
        }
        if !["apk", "aab", "both"].contains(&settings.format.as_str()) {
            return Err(eyre!("android.format must be apk, aab, or both"));
        }
        if settings
            .system_bar_icons
            .as_deref()
            .is_some_and(|icons| !["light", "dark"].contains(&icons))
        {
            return Err(eyre!("android.system_bar_icons must be light or dark"));
        }
        for permission in &settings.permissions {
            validate_application_id(permission)?;
        }
        Ok(settings)
    }
}

fn sdk() -> Result<PathBuf> {
    env::var_os("ANDROID_HOME")
        .or_else(|| env::var_os("ANDROID_SDK_ROOT"))
        .map(PathBuf::from)
        .ok_or_else(|| eyre!("Set ANDROID_HOME to an installed Android SDK"))
}

/// Validate packaging tools before spending time compiling the cartridge.
pub(crate) fn preflight(project: &Path, release: bool) -> Result<()> {
    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(project.join("Cargo.toml"))
        .no_deps()
        .exec()?;
    let package_manifest = project.join("Cargo.toml").canonicalize()?;
    let package = metadata
        .packages
        .iter()
        .find(|p| {
            p.manifest_path.as_std_path().canonicalize().ok().as_deref()
                == Some(package_manifest.as_path())
        })
        .ok_or_else(|| eyre!("Select a Pax application package"))?;
    let settings = AndroidSettings::parse(&package.metadata)?;
    let signing = [
        "PAX_ANDROID_KEYSTORE",
        "PAX_ANDROID_KEY_ALIAS",
        "PAX_ANDROID_STORE_PASSWORD",
        "PAX_ANDROID_KEY_PASSWORD",
    ];
    if release && signing.iter().any(|name| env::var_os(name).is_some()) {
        for name in signing {
            if env::var_os(name).is_none_or(|value| value.is_empty()) {
                return Err(eyre!("Android Release signing requires {name}"));
            }
        }
        if !PathBuf::from(env::var_os("PAX_ANDROID_KEYSTORE").unwrap()).is_file() {
            return Err(eyre!(
                "PAX_ANDROID_KEYSTORE must point to an existing keystore"
            ));
        }
    }
    toolchain()?;
    let sdk = sdk()?;
    let java = env::var_os("JAVA_HOME")
        .map(PathBuf::from)
        .ok_or_else(|| eyre!("Set JAVA_HOME to JDK 17 or newer"))?;
    for path in [
        sdk.join(format!(
            "platforms/android-{}/android.jar",
            settings.compile_sdk
        )),
        sdk.join(format!("build-tools/{}/aapt2", settings.build_tools)),
        sdk.join(format!("build-tools/{}/d8", settings.build_tools)),
        sdk.join(format!("build-tools/{}/apksigner", settings.build_tools)),
        sdk.join(format!("build-tools/{}/zipalign", settings.build_tools)),
        java.join("bin/javac"),
        java.join("bin/java"),
        java.join("bin/keytool"),
        java.join("bin/jarsigner"),
    ] {
        if !path.is_file() {
            return Err(eyre!(
                "Missing Android build prerequisite: {}",
                path.display()
            ));
        }
    }
    if settings.format != "apk"
        && !env::var_os("PAX_ANDROID_BUNDLETOOL")
            .map(PathBuf::from)
            .is_some_and(|p| p.is_file())
    {
        return Err(eyre!(
            "Android App Bundles require PAX_ANDROID_BUNDLETOOL pointing to bundletool-all.jar"
        ));
    }
    if let Some(icon) = settings.icon {
        let bytes = fs::read(project.join(icon))?;
        if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err(eyre!("android.icon must be a PNG file"));
        }
    }
    Ok(())
}

fn ndk() -> Result<PathBuf> {
    let path = env::var_os("ANDROID_NDK_HOME")
        .or_else(|| env::var_os("ANDROID_NDK_ROOT"))
        .ok_or_else(|| {
            eyre!("Android requires ANDROID_NDK_HOME pointing to an installed NDK (r28 or newer)")
        })?;
    let path = PathBuf::from(path);
    let properties = fs::read_to_string(path.join("source.properties"))?;
    let major = properties.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        (key.trim() == "Pkg.Revision")
            .then(|| value.trim().split('.').next()?.parse::<u32>().ok())
            .flatten()
    });
    if !major.is_some_and(|version| version >= 28) {
        return Err(eyre!(
            "Android requires NDK r28 or newer for 16 KiB page alignment"
        ));
    }
    Ok(path)
}

fn toolchain() -> Result<PathBuf> {
    let host = match env::consts::OS {
        "macos" => "darwin-x86_64",
        "linux" => "linux-x86_64",
        _ => {
            return Err(eyre!(
                "Android packaging is currently supported from macOS or Linux"
            ))
        }
    };
    let bin = ndk()?
        .join("toolchains/llvm/prebuilt")
        .join(host)
        .join("bin");
    if !bin.join("aarch64-linux-android29-clang").is_file() {
        return Err(eyre!(
            "Android NDK compiler is missing from {}",
            bin.display()
        ));
    }
    Ok(bin)
}

pub(crate) fn configure_cargo(command: &mut Command) -> Result<()> {
    let bin = toolchain()?;
    command
        .env(
            "CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER",
            bin.join("aarch64-linux-android29-clang"),
        )
        .env(
            "CC_aarch64_linux_android",
            bin.join("aarch64-linux-android29-clang"),
        )
        .env("AR_aarch64_linux_android", bin.join("llvm-ar"));
    Ok(())
}

pub(crate) fn build(
    ctx: &RunContext,
    pax_dir: &Path,
    assets: Vec<String>,
    manifest: PaxManifest,
    timings: &mut BuildTimings,
) -> Result<PathBuf> {
    let layout = project_layout::ProjectLayout::resolve(&ctx.project_path)?;
    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(ctx.project_path.join("Cargo.toml"))
        .no_deps()
        .exec()?;
    let package_manifest = ctx.project_path.join("Cargo.toml").canonicalize()?;
    let package = metadata
        .packages
        .iter()
        .find(|p| {
            p.manifest_path.as_std_path().canonicalize().ok().as_deref()
                == Some(package_manifest.as_path())
        })
        .ok_or_else(|| eyre!("Select a Pax application package"))?;
    let android = AndroidSettings::parse(&package.metadata)?;
    let default_id = format!("dev.pax.{}", package.name.replace(['-', '.'], "_"));
    let application_id = android.application_id.as_deref().unwrap_or(&default_id);
    validate_application_id(application_id)?;
    let mode = if ctx.is_release { "release" } else { "debug" };
    let destination = pax_dir.join("build").join("android").join(mode);
    fs::create_dir_all(&destination)?;
    let sdk = sdk()?;
    let library = format!("lib{}.so", layout.library_name.replace('-', "_"));
    let mut development = if ctx.should_also_run && ctx.should_run_designtime {
        Some(AndroidDevelopment::start(ctx, pax_dir, &manifest, &sdk)?)
    } else {
        None
    };
    let configuration = serde_json::json!({
        "sdk":sdk,"ndkBin":toolchain()?,"android":android,
        "project":ctx.project_path.canonicalize()?,
        "library":layout.target_dir.join(TRIPLE).join(mode).join(&library),
        "assets":assets,"nativeExports":layout.native_exports,
        "destination":destination,"applicationId":application_id,
        "title":package.metadata.get("pax").and_then(|v|v.get("title")).and_then(|v|v.as_str()).unwrap_or(&package.name),
        "version":package.version.to_string(),"release":ctx.is_release,"run":ctx.should_also_run,
        "baseUrl":package.metadata.pointer("/pax/web/site_url").and_then(|v|v.as_str()).unwrap_or(""),
        "keystore":pax_dir.join("android-development.keystore"),
        "designServer":development.as_ref().map(|run| &run.address)
    });
    let config = pax_dir.join("android-build.json");
    fs::write(&config, serde_json::to_vec_pretty(&configuration)?)?;
    timings.record("Android package", || -> Result<()> {
        let status = Command::new("python3")
            .arg(pax_dir.join("interface/android/build.py"))
            .arg(&config)
            .status()?;
        if !status.success() {
            return Err(eyre!("Android packaging failed"));
        }
        Ok(())
    })?;
    println!("Android package: {}", destination.display());
    if let Some(run) = development.as_mut() {
        run.wait_for_app(ctx, application_id)?;
    }
    Ok(destination)
}
struct AndroidDevelopment {
    server: std::process::Child,
    process_child_ids: std::sync::Arc<std::sync::Mutex<Vec<u64>>>,
    adb: PathBuf,
    reverse: String,
    address: String,
    pax_dir: PathBuf,
    session: crate::dev_session::DevSession,
}
impl AndroidDevelopment {
    fn start(ctx: &RunContext, pax_dir: &Path, manifest: &PaxManifest, sdk: &Path) -> Result<Self> {
        use crate::dev_session::*;
        let started_at_ms = now_ms();
        let session_id = format!("android-{started_at_ms}-{}", std::process::id());
        let session_dir = project_dev_dir(pax_dir).join("sessions").join(&session_id);
        for name in ["requests", "responses", "captures"] {
            fs::create_dir_all(session_dir.join(name))?;
        }
        let mut session = DevSession {
            session_id,
            platform: "android".into(),
            designtime: true,
            project_root: Some(ctx.project_path.canonicalize()?),
            session_dir: Some(session_dir),
            app_pid: None,
            design_server_addr: None,
            control_kind: "filesystem".into(),
            location: None,
            started_at_ms,
            last_seen_ms: started_at_ms,
        };
        let session_file = pax_dir.join("android-dev-session.json");
        fs::write(&session_file, serde_json::to_vec(&session)?)?;
        let manifest_file = project_designtime_manifest_file(pax_dir);
        fs::write(&manifest_file, serde_json::to_vec(manifest)?)?;
        let ready = pax_dir.join("android-design-server-url.txt");
        if ready.exists() {
            fs::remove_file(&ready)?;
        }
        let mut command = super::apple::designtime_server_command(
            &std::env::current_exe()?,
            pax_dir,
            &ctx.project_path,
            &manifest_file,
            None,
            ctx.hot_reload.unwrap_or_default(),
            &ready,
        );
        command.args(["--dev-session-file"]).arg(session_file);
        #[cfg(unix)]
        unsafe {
            use std::os::unix::process::CommandExt;
            command.pre_exec(crate::pre_exec_hook);
        }
        let mut server = command.spawn()?;
        ctx.process_child_ids
            .lock()
            .unwrap()
            .push(server.id() as u64);
        let setup = (|| -> Result<(String, String)> {
            let address = super::apple::wait_for_designtime_ready(&ready, Duration::from_secs(10))?;
            let port = address
                .strip_prefix("ws://127.0.0.1:")
                .ok_or_else(|| eyre!("Invalid Android design-server address"))?
                .parse::<u16>()?;
            let reverse = format!("tcp:{port}");
            if !Command::new(sdk.join("platform-tools/adb"))
                .args(["reverse", &reverse, &reverse])
                .status()?
                .success()
            {
                return Err(eyre!("Could not forward the Android design server; select a connected device with ANDROID_SERIAL"));
            }
            Ok((address, reverse))
        })();
        let (address, reverse) = match setup {
            Ok(result) => result,
            Err(error) => {
                super::apple::stop_designtime_server_process(&mut server, &ctx.process_child_ids);
                return Err(error);
            }
        };
        session.design_server_addr = Some(address.clone());
        session.location = Some("adb".into());
        let run = Self {
            server,
            process_child_ids: ctx.process_child_ids.clone(),
            adb: sdk.join("platform-tools/adb"),
            reverse,
            address,
            pax_dir: pax_dir.to_path_buf(),
            session,
        };
        write_project_active_session(pax_dir, &run.session)?;
        Ok(run)
    }
    fn wait_for_app(&mut self, ctx: &RunContext, application_id: &str) -> Result<()> {
        let marker = crate::design_server::app_ready_marker_path(
            &self.pax_dir.join("android-design-server-url.txt"),
        );
        let notifier = crate::RunLifecycleNotifier::new(ctx);
        let _readiness =
            super::apple::watch_app_ready_marker(&marker, Duration::from_secs(30), move || {
                notifier.notify()
            });
        println!(
            "Android development session {} (Ctrl-C to stop)",
            self.session.session_id
        );
        loop {
            if let Some(status) = self.server.try_wait()? {
                return Err(eyre!("Android design server exited: {status}"));
            }
            let process = Command::new(&self.adb)
                .args(["shell", "pidof", application_id])
                .output()?;
            if !process.status.success() || process.stdout.is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
        Ok(())
    }
}
impl Drop for AndroidDevelopment {
    fn drop(&mut self) {
        super::apple::stop_designtime_server_process(&mut self.server, &self.process_child_ids);
        let _ = Command::new(&self.adb)
            .args(["reverse", "--remove", &self.reverse])
            .status();
        let _ = crate::dev_session::remove_project_active_session(
            &self.pax_dir,
            &self.session.session_id,
        );
        let _ = crate::dev_session::remove_registered_session(&self.session.session_id);
    }
}

fn validate_application_id(value: &str) -> Result<()> {
    if !value.contains('.')
        || !value.split('.').all(|part| {
            let mut chars = part.chars();
            chars.next().is_some_and(|c| c.is_ascii_alphabetic())
                && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
    {
        return Err(eyre!(
            "android.application_id must contain dot-separated Java identifiers"
        ));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn android_distribution_settings_validate_versions_and_capabilities() {
        let defaults = AndroidSettings::parse(&serde_json::json!({})).unwrap();
        assert_eq!(defaults.system_bar_icons, None);
        assert_eq!(
            (defaults.min_sdk, defaults.target_sdk, defaults.compile_sdk),
            (29, 36, 36)
        );
        for settings in [
            serde_json::json!({"version_code": 0}),
            serde_json::json!({"version_code": 2_100_000_001u64}),
            serde_json::json!({"min_sdk": 28}),
            serde_json::json!({"target_sdk": 37}),
            serde_json::json!({"build_tools": "../34.0.0"}),
            serde_json::json!({"build_tools": ".."}),
            serde_json::json!({"build_tools": "36..0"}),
            serde_json::json!({"permissions": ["android.permission.INTERNET\"/>"]}),
            serde_json::json!({"format": "app-bundle"}),
            serde_json::json!({"system_bar_icons": "auto"}),
            serde_json::json!({"versionCode": 2}),
        ] {
            assert!(
                AndroidSettings::parse(&serde_json::json!({"pax": {"android": settings}})).is_err()
            );
        }
        assert!(
            AndroidSettings::parse(&serde_json::json!({"pax": {"android": {
                "version_code": 42, "min_sdk": 31, "target_sdk": 36, "format": "both",
                "permissions": [], "application_id": "org.example.app", "system_bar_icons": "dark"
            }}}))
            .is_ok()
        );
    }
    #[test]
    fn package_identity_rejects_manifest_injection_and_invalid_components() {
        for value in ["dev.pax.demo", "com.example.my_app"] {
            assert!(validate_application_id(value).is_ok());
        }
        for value in ["demo", "a..b", "a.2b", "a.b\"/>", "a.b/c"] {
            assert!(validate_application_id(value).is_err());
        }
    }
}
