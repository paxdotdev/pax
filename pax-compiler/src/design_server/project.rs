use super::{websocket::SendAgentMessage, AppState, DebugRevisionCoordinator};
use crate::dev_session::{self, ProjectServer, ProjectServerRegistration, TargetBuild};
use crate::{perform_build, RunContext, RunTarget};
use actix_web::web::Data;
use color_eyre::eyre::{eyre, Result};
use pax_designtime::messages::AgentMessage;
use pax_manifest::PaxManifest;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct Builds {
    context: RunContext,
    targets_file: Option<PathBuf>,
    directory: tempfile::TempDir,
    preview_key: String,
    first: String,
    source_generation: AtomicU64,
    targets: Mutex<BTreeMap<String, TargetBuild>>,
    // Keep the current and previous artifacts through a handover. Web requests
    // also hold their own artifact lease until the response has been opened.
    artifacts: Mutex<BTreeMap<String, Vec<Arc<tempfile::TempDir>>>>,
}

#[derive(Clone)]
pub(super) struct Managed(Arc<Builds>);

struct Product {
    manifest: PaxManifest,
    artifact: PathBuf,
    staging: Arc<tempfile::TempDir>,
}

impl Managed {
    pub fn builds(&self) -> BTreeMap<String, TargetBuild> {
        self.0.targets.lock().unwrap().clone()
    }

    pub fn invalidate(&self) {
        self.0.source_generation.fetch_add(1, Ordering::Release);
    }

    pub fn reject_template(&self, state: &AppState, error: &str) {
        for peer in state.peers.lock().unwrap().values_mut() {
            peer.rejection = Some(error.to_owned());
        }
    }

    pub(super) fn accepts_mobile_key(&self, key: Option<&String>) -> bool {
        key.is_some_and(|k| k == &self.0.preview_key)
    }

    fn requested(&self) -> Result<Vec<String>> {
        let mut targets = vec![self.0.first.clone()];
        if let Some(path) = &self.0.targets_file {
            let more: Vec<String> = serde_json::from_slice(&fs::read(path)?)?;
            for name in more {
                if !matches!(
                    name.as_str(),
                    "web"
                        | "macos"
                        | "ios-simulator"
                        | "ipados-simulator"
                        | "ios-device"
                        | "ipados-device"
                ) {
                    return Err(eyre!("Unsupported project target {name}"));
                }
                if !targets.contains(&name) {
                    targets.push(name);
                }
            }
        }
        Ok(targets)
    }

    fn build(&self, name: &str, generation: u64) -> Result<Product> {
        let mut ctx = self.0.context.clone();
        let (platform, sdk) = name
            .split_once('-')
            .map_or((name, None), |(p, s)| (p, Some(s)));
        ctx.target = RunTarget::parse(platform).map_err(|e| eyre!(e))?;
        ctx.ios_device = sdk.map(str::to_owned);
        ctx.should_also_run = false;
        // Provisioning profiles and device registrations are checked by Xcode
        // on each new server. Active servers still share a signed build.
        let cacheable = !name.ends_with("-device");
        let key = cacheable
            .then(|| super::build_cache::inputs(&ctx, name))
            .and_then(Result::ok);
        let cache = ctx.project_path.join(".pax/dev/build-cache").join(name);
        let source = if name == "web" {
            ctx.project_path.join(".pax/build/optimized-dev/web")
        } else {
            let directory = crate::building::apple::product_directory(
                &ctx.project_path.join(".pax"),
                "optimized-dev",
                platform,
                sdk,
            );
            directory.join("app").join(format!(
                "{}.app",
                if name == "macos" {
                    crate::building::apple::XCODE_MACOS_TARGET_DEBUG
                } else {
                    crate::building::apple::XCODE_IOS_TARGET
                }
            ))
        };
        if let Some(key) = &key {
            let cached = (|| -> Result<Product> {
                let stamp: super::build_cache::Receipt =
                    serde_json::from_slice(&fs::read(cache.join("receipt.json"))?)?;
                if stamp.inputs != *key {
                    return Err(eyre!("Build inputs changed"));
                }
                let manifest = fs::read(cache.join("manifest.json"))?;
                if stamp.manifest != super::build_cache::manifest_digest(&manifest) {
                    return Err(eyre!("Manifest changed"));
                }
                let manifest: PaxManifest = serde_json::from_slice(&manifest)?;
                if stamp.resources != super::build_cache::resources(&ctx.project_path, &manifest)? {
                    return Err(eyre!("Template or asset inputs changed"));
                }
                let staged = self.stage(name, generation, &source, manifest)?;
                if stamp.artifact != super::build_cache::artifact(&staged.artifact)? {
                    return Err(eyre!("Build product changed"));
                }
                Ok(staged)
            })();
            match cached {
                Ok(product) => {
                    println!("Reused validated {name} development artifact");
                    return Ok(product);
                }
                Err(error) if cache.join("receipt.json").exists() => {
                    println!("Rebuilding {name} development artifact: {error}");
                }
                Err(_) => {}
            }
        }
        let (manifest, web_dir) = perform_build(&ctx)?;
        let source = web_dir
            .as_deref()
            .filter(|_| name == "web")
            .unwrap_or(&source);
        let product = self.stage(name, generation, source, manifest)?;
        if let Some(inputs) = key {
            let save = (|| -> Result<()> {
                if super::build_cache::inputs(&ctx, name)? != inputs {
                    self.invalidate();
                    return Ok(());
                }
                fs::create_dir_all(&cache)?;
                let manifest = serde_json::to_vec(&product.manifest)?;
                let receipt = super::build_cache::Receipt {
                    resources: super::build_cache::resources(&ctx.project_path, &product.manifest)?,
                    inputs,
                    artifact: super::build_cache::artifact(&product.artifact)?,
                    manifest: super::build_cache::manifest_digest(&manifest),
                };
                for (file, data) in [
                    ("manifest.json", manifest),
                    ("receipt.json", serde_json::to_vec(&receipt)?),
                ] {
                    let mut stage = tempfile::NamedTempFile::new_in(&cache)?;
                    std::io::Write::write_all(&mut stage, &data)?;
                    stage.persist(cache.join(file))?;
                }
                Ok(())
            })();
            if let Err(e) = save {
                eprintln!("Development build cache: {e:#}")
            }
        }
        Ok(product)
    }

    fn stage(
        &self,
        name: &str,
        generation: u64,
        source: &std::path::Path,
        manifest: PaxManifest,
    ) -> Result<Product> {
        let staging = tempfile::Builder::new()
            .prefix(&format!("{name}-{generation}-"))
            .tempdir_in(self.0.directory.path())?;
        let dest = staging.path().join(if name == "web" {
            std::ffi::OsStr::new("web")
        } else {
            source
                .file_name()
                .ok_or_else(|| eyre!("Invalid Apple bundle"))?
        });
        if name == "web" {
            crate::copy_dir_recursively(source, &dest, &[]).map_err(|e| eyre!("{e}"))?;
            if !dest.join("index.html").is_file() {
                return Err(eyre!("Missing Web entry point"));
            }
        } else {
            let result = std::process::Command::new("/usr/bin/ditto")
                .arg(source)
                .arg(&dest)
                .status()?;
            if !result.success() {
                return Err(eyre!("Could not stage the Apple bundle ({result})"));
            }
            fs::write(
                staging.path().join("manifest.json"),
                serde_json::to_vec(&manifest)?,
            )?;
        }
        Ok(Product {
            manifest,
            artifact: dest,
            staging: Arc::new(staging),
        })
    }

    fn install(&self, target: String, product: Product, generation: u64) {
        let mut paths = self.0.artifacts.lock().unwrap();
        let versions = paths.entry(target.clone()).or_default();
        versions.push(product.staging);
        if versions.len() > 2 {
            versions.remove(0);
        }
        self.0.targets.lock().unwrap().insert(
            target,
            TargetBuild {
                generation,
                artifact: Some(product.artifact),
                building: false,
                error: None,
            },
        );
    }

    fn publish(&self, descriptor: &mut ProjectServer) -> Result<()> {
        descriptor.builds = self.builds();
        ProjectServerRegistration::update(descriptor)?;
        Ok(())
    }

    pub fn start(&self, state: Data<AppState>, mut descriptor: ProjectServer) {
        let managed = self.clone();
        let mut attempted: BTreeMap<_, _> = self
            .builds()
            .into_keys()
            .map(|t| (t, (1u64, 0u64)))
            .collect();
        let state = Arc::downgrade(&state.into_inner());
        std::thread::spawn(move || {
            while let Some(server) = state.upgrade() {
                std::thread::sleep(Duration::from_millis(250));
                let result = managed.tick(&Data::from(server), &mut descriptor, &mut attempted);
                if let Err(err) = result {
                    eprintln!("Project build coordinator: {err:#}");
                }
            }
        });
    }

    fn tick(
        &self,
        state: &Data<AppState>,
        descriptor: &mut ProjectServer,
        attempted: &mut BTreeMap<String, (u64, u64)>,
    ) -> Result<()> {
        let generation = self.0.source_generation.load(Ordering::Acquire);
        let retries: BTreeMap<String, u64> = match self
            .0
            .targets_file
            .as_deref()
            .map(|f| fs::read(f.with_extension("retry.json")))
        {
            Some(Ok(bytes)) => serde_json::from_slice(&bytes)?,
            Some(Err(e)) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.into()),
            _ => BTreeMap::new(),
        };
        let attempt = |target: &str| (generation, retries.get(target).copied().unwrap_or(0));
        let mut pending: Vec<_> = self
            .requested()?
            .into_iter()
            .filter(|t| attempted.get(t) != Some(&attempt(t)))
            .collect();
        // Additional selections can launch as soon as their SDK build is ready.
        // On common source changes, stage compatible replacements together.
        if descriptor
            .builds
            .values()
            .any(|b| b.generation == generation)
        {
            pending.truncate(1);
        }
        if pending.is_empty() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(150));
        if generation != self.0.source_generation.load(Ordering::Acquire) {
            return Ok(());
        }
        for target in &pending {
            attempted.insert(target.clone(), attempt(target));
            let mut targets = self.0.targets.lock().unwrap();
            let build = targets.entry(target.clone()).or_default();
            build.building = true;
            build.error = None;
        }
        self.publish(descriptor)?;
        let mut products = Vec::new();
        for target in &pending {
            println!("Building {target} development preview, generation {generation}");
            match self.build(target, generation) {
                Ok(product) => products.push((target.clone(), product)),
                Err(err) => {
                    let diagnostic =
                        format!("{target} build failed: {err:#}. See the operation log.");
                    eprintln!("{diagnostic}");
                    let mut targets = self.0.targets.lock().unwrap();
                    let build = targets.get_mut(target).unwrap();
                    build.building = false;
                    build.error = Some(diagnostic);
                }
            }
        }
        // A save during compilation queues a fresh batch. A stale completion
        // never replaces a working preview.
        if self.0.source_generation.load(Ordering::Acquire) != generation {
            return Ok(());
        }
        if products.is_empty() {
            return self.publish(descriptor);
        }
        let rebuild = generation
            > descriptor
                .builds
                .values()
                .map(|b| b.generation)
                .max()
                .unwrap_or(0);
        let abi = if rebuild {
            let primary = products
                .iter()
                .find(|(t, _)| t == &self.0.first)
                .unwrap_or(&products[0]);
            dev_session::cartridge_abi(&primary.1.manifest)?
        } else {
            descriptor.cartridge_abi.clone()
        };
        let mut compatible = Vec::new();
        for (target, product) in products {
            if dev_session::cartridge_abi(&product.manifest)? == abi {
                compatible.push((target, product))
            } else if let Some(b) = self.0.targets.lock().unwrap().get_mut(&target) {
                b.building = false;
                b.error = Some("Build has incompatible compiled components or handlers.".into());
            }
        }
        let products = compatible;
        if products.is_empty() {
            return self.publish(descriptor);
        }
        if rebuild {
            let _source = state.source_update_lock.lock().unwrap();
            *state.revisions.lock().unwrap() = DebugRevisionCoordinator::new(
                format!("{}-{generation}", descriptor.id),
                products[0].1.manifest.clone(),
            );
            if descriptor.cartridge_abi != abi {
                let peers = state.peers.lock().unwrap();
                state
                    .template_previews
                    .lock()
                    .unwrap()
                    .retain(|id, _| peers.get(id).is_some_and(|p| p.cartridge_abi == abi));
            }
            descriptor.cartridge_abi = abi;
        }
        let web_changed = products.iter().any(|(t, _)| t == "web");
        for (target, product) in products {
            self.install(target, product, generation);
        }
        if rebuild {
            state.reconcile_restored_pax_sources();
            for peer in state.peers.lock().unwrap().values_mut() {
                if peer.target != "web" {
                    peer.rejection = Some("New build available. Relaunch this preview.".into());
                }
            }
        }
        self.publish(descriptor)?;
        if rebuild && web_changed {
            let peers = state.peers.lock().unwrap();
            for (id, subscriber) in state.template_previews.lock().unwrap().iter() {
                if peers.get(id).is_some_and(|p| p.target == "web") {
                    subscriber.do_send(SendAgentMessage {
                        message: AgentMessage::ReloadPage,
                    });
                }
            }
        }
        Ok(())
    }
}

/// Serve one project's active Web and Apple builds. The optional local JSON file
/// selects additional targets. A source rebuild only publishes complete products.
pub fn serve_project(mut ctx: RunContext, targets_file: Option<PathBuf>) -> Result<()> {
    if !matches!(
        ctx.target,
        RunTarget::Web | RunTarget::macOS | RunTarget::iOS | RunTarget::iPadOS
    ) {
        return Err(eyre!("Project servers support Web and Apple previews"));
    }
    ctx.project_path = ctx.project_path.canonicalize()?;
    ctx.optimized_dev = true;
    let hot_reload = crate::development_reload_mode(&ctx)?;
    ctx.hot_reload = Some(hot_reload);
    let root = ctx.project_path.clone();
    let pax_dir = root.join(".pax");
    let session = crate::building::web::prepare_web_dev_session(&root, &pax_dir)?;
    let staged = session.session_dir.as_ref().unwrap().join("builds");
    fs::create_dir_all(&staged)?;
    let name: &str = (&ctx.target).into();
    let first = if matches!(ctx.target, RunTarget::iOS | RunTarget::iPadOS) {
        let sdk = crate::building::apple::project_mobile_sdk(&ctx)?;
        format!("{}-{sdk}", name.to_lowercase())
    } else {
        name.to_lowercase()
    };
    let key = format!(
        "{:032x}{:032x}",
        rand::random::<u128>(),
        rand::random::<u128>()
    );
    let mut secret = tempfile::NamedTempFile::new_in(session.session_dir.as_ref().unwrap())?;
    std::io::Write::write_all(&mut secret, key.as_bytes())?;
    secret.persist(
        session
            .session_dir
            .as_ref()
            .unwrap()
            .join("mobile-preview.key"),
    )?;
    let port = ctx.port;
    let managed = Managed(Arc::new(Builds {
        context: ctx,
        first: first.clone(),
        preview_key: key,
        targets_file,
        directory: tempfile::tempdir_in(staged)?,
        source_generation: AtomicU64::new(1),
        targets: Mutex::new(BTreeMap::new()),
        artifacts: Mutex::new(BTreeMap::new()),
    }));
    let mut attempted = std::collections::BTreeSet::new();
    let manifest = loop {
        let target = managed
            .requested()?
            .into_iter()
            .find(|target| !attempted.contains(target))
            .ok_or_else(|| {
                eyre!("Initial project build failed. See target diagnostics in the operation log.")
            })?;
        attempted.insert(target.clone());
        match managed.build(&target, 1) {
            Ok(product) => {
                let manifest = product.manifest.clone();
                managed.install(target, product, 1);
                break manifest;
            }
            Err(e) => {
                eprintln!("Initial {target} build failed: {e:#}");
                managed.0.targets.lock().unwrap().insert(
                    target,
                    TargetBuild {
                        error: Some(format!("{e:#}")),
                        ..Default::default()
                    },
                );
            }
        }
    };
    let dir = session.session_dir.as_ref().unwrap().join("site");
    fs::create_dir_all(&dir)?;
    super::serve(
        dir.to_str().unwrap(),
        root.to_str().unwrap(),
        manifest,
        port,
        None,
        true,
        Some(session),
        None,
        hot_reload,
        None,
        None,
        None,
        Some(managed),
    )?;
    Ok(())
}

pub(super) fn files(
    cfg: &mut actix_web::web::ServiceConfig,
    managed: Managed,
    public: Option<PathBuf>,
) {
    use actix_web::dev::{fn_service, Service, ServiceFactory, ServiceRequest};
    use actix_web::{http::header, HttpResponse};
    cfg.default_service(fn_service(move |req: ServiceRequest| {
        let lease = managed
            .0
            .artifacts
            .lock()
            .unwrap()
            .get("web")
            .and_then(|v| v.last().cloned());
        let public = public.clone();
        async move {
            let Some(lease) = lease else {
                return Ok(req.into_response(
                    HttpResponse::ServiceUnavailable()
                        .body("Run this project on Web to open a browser preview."),
                ));
            };
            let path = lease.path().join("web");
            let prefixes = crate::project_metadata::load_web_server_owned_prefixes(&path)
                .map_err(actix_web::error::ErrorInternalServerError)?;
            let files = super::static_files_service(path, public, prefixes);
            let service = files.new_service(()).await.map_err(|_| {
                actix_web::error::ErrorInternalServerError("Cannot serve Web build")
            })?;
            let mut response = service.call(req).await?;
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                header::HeaderValue::from_static("no-cache"),
            );
            Ok::<_, actix_web::Error>(response)
        }
    }));
}
