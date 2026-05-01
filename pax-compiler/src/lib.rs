//! # The Pax Compiler Library
//!
//! `pax-compiler` is a collection of utilities to facilitate compiling Pax templates into Rust code.
//!
//! This library is structured into several modules, each providing different
//! functionality:
//!
//! - `building`: Core structures and functions related to building management.
//! - `utilities`: Helper functions and common routines used across the library.
//!

#[macro_use]
extern crate serde;

extern crate core;
mod building;
mod cartridge_generation;
pub mod dev_session;
pub mod helpers;
pub mod static_analysis;

pub mod design_server;

use color_eyre::eyre;
use color_eyre::eyre::Report;
use eyre::eyre;
use fs_extra::dir::{self, CopyOptions};
use helpers::{copy_dir_recursively, wait_with_output, ERR_SPAWN};
use pax_manifest::{
    ComponentDefinition, ComponentTemplate, LiteralBlockDefinition, PaxExpression, PaxManifest,
    SettingElement, SettingsBlockElement, TemplateNodeDefinition, TypeId, ValueDefinition,
};
use reqwest::blocking::Client;
use reqwest::Url;
use serde_json::Value as JsonValue;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

use crate::building::build_project_with_cartridge;

use crate::cartridge_generation::generate_cartridge_partial_rs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};
use walkdir::WalkDir;

use crate::helpers::{
    get_or_create_pax_directory, update_pax_dependency_versions, INTERFACE_DIR_NAME, PAX_BADGE,
    PAX_CREATE_LIBDEV_TEMPLATE_DIR_NAME, PAX_CREATE_TEMPLATE, PAX_IOS_INTERFACE_TEMPLATE,
    PAX_MACOS_INTERFACE_TEMPLATE, PAX_SWIFT_CARTRIDGE_TEMPLATE, PAX_SWIFT_COMMON_TEMPLATE,
    PAX_WEB_INTERFACE_TEMPLATE,
};

pub struct RunContext {
    pub target: RunTarget,
    pub project_path: PathBuf,
    pub verbose: bool,
    pub should_also_run: bool,
    pub is_libdev_mode: bool,
    pub process_child_ids: Arc<Mutex<Vec<u64>>>,
    pub should_run_designtime: bool,
    pub should_run_designer: bool,
    pub is_release: bool,
    pub profile_wasm_size: bool,
    pub webgl: bool,
    pub ios_device: Option<String>,
    pub ios_development_team: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct WebFontSource {
    family: String,
    url: String,
}

#[derive(Debug, PartialEq)]
pub enum RunTarget {
    #[allow(non_camel_case_types)]
    macOS,
    Web,
    #[allow(non_camel_case_types)]
    iOS,
    #[allow(non_camel_case_types)]
    iPadOS,
}

const WEB_INTERFACE_BUNDLE_FILE: &str = "public/pax-interface-web.js";
const WEB_INTERFACE_FINGERPRINT_FILE: &str = ".pax-interface-web.fingerprint";
const WEB_INTERFACE_HASH_OFFSET: u64 = 0xcbf29ce484222325;
const WEB_INTERFACE_HASH_PRIME: u64 = 0x100000001b3;

pub(crate) struct BuildTimings {
    total_start: Instant,
    phases: Vec<BuildTimingPhase>,
}

struct BuildTimingPhase {
    label: &'static str,
    duration: Duration,
}

impl BuildTimings {
    fn start() -> Self {
        Self {
            total_start: Instant::now(),
            phases: Vec::new(),
        }
    }

    fn record<T>(&mut self, label: &'static str, operation: impl FnOnce() -> T) -> T {
        let start = Instant::now();
        let result = operation();
        self.phases.push(BuildTimingPhase {
            label,
            duration: start.elapsed(),
        });
        result
    }

    pub(crate) fn print_summary(&self) {
        let total = self.total_start.elapsed();
        let measured = self
            .phases
            .iter()
            .map(|phase| phase.duration)
            .fold(Duration::ZERO, |sum, duration| sum + duration);
        let other = total.saturating_sub(measured);

        println!(
            "{} ⏱️  Build completed in {:.2}s",
            *PAX_BADGE,
            seconds(total)
        );
        for phase in &self.phases {
            println!(
                "{}    {:<24} {:.2}s",
                *PAX_BADGE,
                phase.label,
                seconds(phase.duration)
            );
        }
        if seconds(other) >= 0.005 {
            println!("{}    {:<24} {:.2}s", *PAX_BADGE, "other", seconds(other));
        }
    }
}

fn seconds(duration: Duration) -> f64 {
    duration.as_secs_f64()
}

pub(crate) struct PreparedCartridgeSources {
    pub pax_dir: PathBuf,
    pub userland_manifest: PaxManifest,
    pub assets_dirs: Vec<String>,
}

/// For the specified file path or current working directory, first compile Pax project,
/// then run it with a patched build of the `chassis` appropriate for the specified platform
/// See: pax-compiler-sequence-diagram.png
pub fn perform_build(ctx: &RunContext) -> eyre::Result<(PaxManifest, Option<PathBuf>), Report> {
    let mut timings = BuildTimings::start();
    let prepared = prepare_cartridge_sources_with_timings(ctx, &mut timings)?;

    //7. Build full project from source
    println!("{} 🧱 Building project with `cargo`", *PAX_BADGE);
    let build_dir = build_project_with_cartridge(
        &prepared.pax_dir,
        &ctx,
        Arc::clone(&ctx.process_child_ids),
        prepared.assets_dirs,
        prepared.userland_manifest.clone(),
        &mut timings,
    )?;

    Ok((prepared.userland_manifest, build_dir))
}

pub(crate) fn prepare_cartridge_sources(
    ctx: &RunContext,
) -> eyre::Result<PreparedCartridgeSources, Report> {
    let mut timings = BuildTimings::start();
    prepare_cartridge_sources_with_timings(ctx, &mut timings)
}

fn prepare_cartridge_sources_with_timings(
    ctx: &RunContext,
    timings: &mut BuildTimings,
) -> eyre::Result<PreparedCartridgeSources, Report> {
    if ctx.is_release && (ctx.should_run_designtime || ctx.should_run_designer) {
        return Err(eyre!(
            "Release builds do not support designtime or designer features. Use a debug build for designtime sessions."
        ));
    }

    if ctx.target == RunTarget::Web {
        timings.record("web interface", || ensure_default_web_interface_bundle(ctx));
    }

    let pax_dir = get_or_create_pax_directory(&ctx.project_path);

    // Copy interface files for relevant path
    timings.record("copy interface", || {
        copy_interface_files_for_target(ctx, &pax_dir)
    });

    let mut manifests: Vec<PaxManifest> =
        timings.record("manifest", || -> eyre::Result<Vec<PaxManifest>, Report> {
            if ctx.should_run_designer {
                println!(
                    "{} 🔎 Static analysis disabled for designer builds; falling back to parser binary",
                    *PAX_BADGE
                );
                run_and_parse_parser_binary(ctx)
            } else {
                match static_analysis::build_manifest_with_options(
                    &ctx.project_path,
                    static_analysis::BuildManifestOptions {
                        is_designtime: ctx.should_run_designtime,
                    },
                ) {
                    Ok(manifest) => {
                        println!("{} 🔎 Built manifest via static analysis", *PAX_BADGE);
                        Ok(vec![manifest])
                    }
                    Err(err) => {
                        println!(
                            "{} 🔎 Static analysis fell back to parser binary: {}",
                            *PAX_BADGE, err
                        );
                        run_and_parse_parser_binary(ctx)
                    }
                }
            }
        })?;

    // Simple starting convention: first manifest is userland, second manifest is designer; other schemas are undefined
    let mut userland_manifest = manifests.remove(0);

    let mut merged_manifest = userland_manifest.clone();

    //Hack: add a wrapper component so UniqueTemplateNodeIdentifier is a suitable uniqueid, even for root nodes
    let wrapper_type_id = TypeId::build_singleton("ROOT_COMPONENT", Some("RootComponent"));
    let mut tnd = TemplateNodeDefinition::default();
    tnd.type_id = userland_manifest.main_component_type_id.clone();
    let mut wrapper_component_template = ComponentTemplate::new(wrapper_type_id.clone(), None);
    wrapper_component_template.add(tnd);
    userland_manifest.components.insert(
        wrapper_type_id.clone(),
        ComponentDefinition {
            type_id: wrapper_type_id.clone(),
            is_main_component: false,
            is_primitive: false,
            is_struct_only_component: false,
            module_path: "".to_string(),
            primitive_instance_import_path: None,
            template: Some(wrapper_component_template),
            settings: None,
            timelines: vec![],
        },
    );

    let designer_manifest = if ctx.should_run_designer {
        let designer_manifest = manifests.remove(0);
        merged_manifest.merge_in_place(&designer_manifest);

        userland_manifest
            .components
            .extend(designer_manifest.components.clone());
        userland_manifest
            .type_table
            .extend(designer_manifest.type_table.clone());

        Some(designer_manifest)
    } else {
        None
    };

    if matches!(
        ctx.target,
        RunTarget::macOS | RunTarget::iOS | RunTarget::iPadOS
    ) {
        timings.record("apple web fonts", || {
            vendor_apple_web_fonts(ctx, &pax_dir, &merged_manifest)
        })?;
    }

    println!("{} 🦀 Generating Rust", *PAX_BADGE);
    timings.record("generate rust", || {
        generate_cartridge_partial_rs(
            &pax_dir,
            &merged_manifest,
            &userland_manifest,
            designer_manifest,
            ctx.is_release && !ctx.should_run_designtime && !ctx.should_run_designer,
        );
    });
    Ok(PreparedCartridgeSources {
        pax_dir,
        userland_manifest,
        assets_dirs: merged_manifest.assets_dirs,
    })
}

fn run_and_parse_parser_binary(ctx: &RunContext) -> eyre::Result<Vec<PaxManifest>, Report> {
    println!("{} 🛠️  Building parser binary with `cargo`...", *PAX_BADGE);

    let output = run_parser_binary(
        &ctx.project_path,
        Arc::clone(&ctx.process_child_ids),
        ctx.should_run_designtime,
        ctx.should_run_designer,
    );

    std::io::stderr()
        .write_all(output.stderr.as_slice())
        .unwrap();

    if !output.status.success() {
        return Err(eyre!(
            "Parser build failed. See the Cargo output above; this can be caused by Pax syntax errors, dependency feature mismatches, or missing local path patches."
        ));
    }

    let out = String::from_utf8(output.stdout).unwrap();
    let manifests: Vec<PaxManifest> =
        serde_json::from_str(&out).expect(&format!("Malformed JSON from parser: {}", &out));
    Ok(manifests)
}

fn ensure_default_web_interface_bundle(ctx: &RunContext) {
    let pax_compiler_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let web_interface_root = pax_compiler_root
        .join("files")
        .join("interfaces")
        .join("web");
    if !web_interface_root.exists() {
        return;
    }
    if !web_interface_bundle_needs_rebuild(&web_interface_root, ctx.is_libdev_mode) {
        return;
    }

    let mut cmd = Command::new("bash");
    cmd.arg("./build-interface.sh")
        .current_dir(&web_interface_root)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(pre_exec_hook);
    }

    let child = cmd
        .spawn()
        .expect("failed to start web interface bundle build");
    let output = wait_with_output(&ctx.process_child_ids, child);
    if !output.status.success() {
        panic!(
            "failed to build the default Pax web interface at {:?}",
            web_interface_root
        );
    }

    if ctx.is_libdev_mode {
        if let Err(err) = write_web_interface_fingerprint(&web_interface_root) {
            eprintln!(
                "{} ⚠️  Failed to write web interface fingerprint: {}",
                *PAX_BADGE, err
            );
        }
    }
}

fn web_interface_bundle_needs_rebuild(web_interface_root: &Path, is_libdev_mode: bool) -> bool {
    if !web_interface_root.join(WEB_INTERFACE_BUNDLE_FILE).is_file() {
        return true;
    }
    if !is_libdev_mode {
        return false;
    }

    current_web_interface_fingerprint(web_interface_root)
        .and_then(|current| {
            fs::read_to_string(web_interface_fingerprint_path(web_interface_root))
                .map(|stored| stored.trim() != current)
        })
        .unwrap_or(true)
}

fn write_web_interface_fingerprint(web_interface_root: &Path) -> io::Result<()> {
    let fingerprint = current_web_interface_fingerprint(web_interface_root)?;
    fs::write(
        web_interface_fingerprint_path(web_interface_root),
        format!("{fingerprint}\n"),
    )
}

fn web_interface_fingerprint_path(web_interface_root: &Path) -> PathBuf {
    web_interface_root.join(WEB_INTERFACE_FINGERPRINT_FILE)
}

fn current_web_interface_fingerprint(web_interface_root: &Path) -> io::Result<String> {
    let mut hasher = WebInterfaceStableHasher::new();
    hasher.write(b"pax-web-interface-fingerprint-v1\0");

    for input in web_interface_fingerprint_inputs(web_interface_root)? {
        hasher.write(input.relative_path.as_bytes());
        hasher.write(b"\0");
        match input.path {
            Some(path) => {
                let bytes = fs::read(path)?;
                hasher.write(b"file\0");
                hasher.write(&(bytes.len() as u64).to_le_bytes());
                hasher.write(&bytes);
            }
            None => hasher.write(b"missing\0"),
        }
        hasher.write(b"\0");
    }

    Ok(format!("{:016x}", hasher.finish()))
}

struct WebInterfaceFingerprintInput {
    relative_path: String,
    path: Option<PathBuf>,
}

fn web_interface_fingerprint_inputs(
    web_interface_root: &Path,
) -> io::Result<Vec<WebInterfaceFingerprintInput>> {
    let mut inputs = Vec::new();

    for relative_path in ["build-interface.sh", "package.json", "tsconfig.json"] {
        let path = web_interface_root.join(relative_path);
        inputs.push(WebInterfaceFingerprintInput {
            relative_path: relative_path.to_string(),
            path: path.is_file().then_some(path),
        });
    }

    let src_path = web_interface_root.join("src");
    if src_path.is_dir() {
        for entry in WalkDir::new(&src_path) {
            let entry = entry.map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
            if !entry.file_type().is_file() {
                continue;
            }
            let relative_path = entry
                .path()
                .strip_prefix(web_interface_root)
                .map_err(|err| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "failed to make web interface path relative to {}: {err}",
                            web_interface_root.display()
                        ),
                    )
                })?;
            inputs.push(WebInterfaceFingerprintInput {
                relative_path: normalized_relative_path(relative_path),
                path: Some(entry.path().to_path_buf()),
            });
        }
    } else {
        inputs.push(WebInterfaceFingerprintInput {
            relative_path: "src".to_string(),
            path: None,
        });
    }

    inputs.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(inputs)
}

fn normalized_relative_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

struct WebInterfaceStableHasher {
    state: u64,
}

impl WebInterfaceStableHasher {
    fn new() -> Self {
        Self {
            state: WEB_INTERFACE_HASH_OFFSET,
        }
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.state ^= *byte as u64;
            self.state = self.state.wrapping_mul(WEB_INTERFACE_HASH_PRIME);
        }
    }

    fn finish(&self) -> u64 {
        self.state
    }
}

fn build_interface_dir_name(target: &RunTarget) -> &'static str {
    match target {
        RunTarget::Web => "web",
        RunTarget::macOS => "macos",
        RunTarget::iOS | RunTarget::iPadOS => "ios",
    }
}

fn custom_interface_dir_candidates(target: &RunTarget) -> &'static [&'static str] {
    match target {
        RunTarget::Web => &["web"],
        RunTarget::macOS => &["macos"],
        RunTarget::iOS => &["ios"],
        RunTarget::iPadOS => &["ipados", "ios"],
    }
}

fn copy_interface_files_for_target(ctx: &RunContext, pax_dir: &PathBuf) {
    let interface_path = pax_dir
        .join(INTERFACE_DIR_NAME)
        .join(build_interface_dir_name(&ctx.target));

    let _ = fs::remove_dir_all(&interface_path);
    let _ = fs::create_dir_all(&interface_path);

    let custom_interface = custom_interface_dir_candidates(&ctx.target)
        .iter()
        .map(|candidate| {
            let mut interface_path = pax_dir.parent().unwrap().join("interfaces").join(candidate);
            if ctx.target == RunTarget::Web {
                interface_path = interface_path.join("public");
            }
            interface_path
        })
        .find(|path| path.exists());

    if let Some(custom_interface) = custom_interface {
        copy_interface_files(&custom_interface, &interface_path);
    } else {
        copy_default_interface_files(&interface_path, ctx);
    }

    // Copy common files for macOS and iOS builds
    if matches!(
        ctx.target,
        RunTarget::macOS | RunTarget::iOS | RunTarget::iPadOS
    ) {
        let common_dest = pax_dir.join(INTERFACE_DIR_NAME).join("common");
        copy_common_swift_files(ctx, &common_dest);
    }
}

fn copy_interface_files(src: &Path, dest: &Path) {
    copy_dir_recursively(src, dest, &[]).expect("Failed to copy interface files");
}

fn copy_default_interface_files(interface_path: &Path, ctx: &RunContext) {
    let pax_compiler_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let interface_src = match ctx.target {
        RunTarget::Web => pax_compiler_root
            .join("files")
            .join("interfaces")
            .join("web")
            .join("public"),
        RunTarget::macOS => pax_compiler_root
            .join("files")
            .join("interfaces")
            .join("macos"),
        RunTarget::iOS | RunTarget::iPadOS => pax_compiler_root
            .join("files")
            .join("interfaces")
            .join("ios"),
    };

    if ctx.is_libdev_mode || interface_src.exists() {
        copy_dir_recursively(&interface_src, interface_path, &[])
            .expect("Failed to copy interface files");
    } else {
        // File src is include_dir — recursively extract files from include_dir into full_path
        match ctx.target {
            RunTarget::Web => PAX_WEB_INTERFACE_TEMPLATE
                .extract(interface_path)
                .expect("Failed to extract web interface files"),
            RunTarget::macOS => PAX_MACOS_INTERFACE_TEMPLATE
                .extract(interface_path)
                .expect("Failed to extract macos interface files"),
            RunTarget::iOS | RunTarget::iPadOS => PAX_IOS_INTERFACE_TEMPLATE
                .extract(interface_path)
                .expect("Failed to extract ios interface files"),
        }
    }
}

fn copy_common_swift_files(ctx: &RunContext, common_dest: &Path) {
    let _ = std::fs::remove_dir_all(common_dest);
    std::fs::create_dir_all(common_dest).expect("Failed to create swift common destination");
    let pax_compiler_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let common_swift_cartridge_src = pax_compiler_root
        .join("files")
        .join("swift")
        .join("pax-swift-cartridge");
    let common_swift_common_src = pax_compiler_root
        .join("files")
        .join("swift")
        .join("pax-swift-common");

    if ctx.is_libdev_mode
        || (common_swift_cartridge_src.exists() && common_swift_common_src.exists())
    {
        let common_swift_cartridge_dest = common_dest.join("pax-swift-cartridge");
        let common_swift_common_dest = common_dest.join("pax-swift-common");

        copy_dir_recursively(
            &common_swift_cartridge_src,
            &common_swift_cartridge_dest,
            &[".build"],
        )
        .expect("Failed to copy swift cartridge files");
        copy_dir_recursively(
            &common_swift_common_src,
            &common_swift_common_dest,
            &[".build"],
        )
        .expect("Failed to copy swift common files");
    } else {
        let common_swift_common_dest = common_dest.join("pax-swift-common");
        let common_swift_cartridge_dest = common_dest.join("pax-swift-cartridge");
        fs::create_dir_all(&common_swift_common_dest)
            .expect("Failed to create swift common destination");
        fs::create_dir_all(&common_swift_cartridge_dest)
            .expect("Failed to create swift cartridge destination");
        PAX_SWIFT_COMMON_TEMPLATE
            .extract(&common_swift_common_dest)
            .expect("Failed to extract swift common template files");
        PAX_SWIFT_CARTRIDGE_TEMPLATE
            .extract(&common_swift_cartridge_dest)
            .expect("Failed to extract swift cartridge template files");
    }
}

fn vendor_apple_web_fonts(
    ctx: &RunContext,
    pax_dir: &Path,
    manifest: &PaxManifest,
) -> eyre::Result<(), Report> {
    if !matches!(
        ctx.target,
        RunTarget::macOS | RunTarget::iOS | RunTarget::iPadOS
    ) {
        return Ok(());
    }

    let font_sources = collect_web_font_sources(manifest);
    if font_sources.is_empty() {
        return Ok(());
    }

    let resources_dir = pax_dir
        .join(INTERFACE_DIR_NAME)
        .join("common")
        .join("pax-swift-cartridge")
        .join("Sources")
        .join("PaxCartridgeAssets")
        .join("Resources");
    fs::create_dir_all(&resources_dir)?;
    if let Ok(existing_entries) = fs::read_dir(&resources_dir) {
        for entry in existing_entries.flatten() {
            let path = entry.path();
            let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if file_name.starts_with("pax-font-") {
                let _ = fs::remove_file(path);
            }
        }
    }

    let client = Client::builder().build()?;

    let mut vendored_assets: HashSet<String> = HashSet::new();
    let mut vendored_count = 0usize;

    for font_source in font_sources {
        if let Err(error) = vendor_web_font_source(
            &client,
            &font_source,
            &resources_dir,
            &mut vendored_assets,
            &mut vendored_count,
        ) {
            println!(
                "{} ⚠️  Failed to vendor Apple font '{}' from {}: {}",
                *PAX_BADGE, font_source.family, font_source.url, error
            );
        }
    }

    if vendored_count > 0 {
        println!(
            "{} 🔤 Vendored {} Apple font asset{} for bundled native builds",
            *PAX_BADGE,
            vendored_count,
            if vendored_count == 1 { "" } else { "s" }
        );
    }

    Ok(())
}

fn collect_web_font_sources(manifest: &PaxManifest) -> Vec<WebFontSource> {
    let mut seen = HashSet::new();
    let mut collected = Vec::new();

    for component in manifest.components.values() {
        if let Some(template) = &component.template {
            for node in template.get_nodes() {
                if let Some(settings) = &node.settings {
                    collect_setting_elements(settings, &mut seen, &mut collected);
                }
            }
        }

        if let Some(settings) = &component.settings {
            collect_settings_block_elements(settings, &mut seen, &mut collected);
        }

        for timeline in &component.timelines {
            if let Some(playhead) = &timeline.playhead {
                collect_value_definition(playhead, &mut seen, &mut collected);
            }
            for element in &timeline.elements {
                if let pax_manifest::TimelineBlockElement::SelectorBlock(_, selector_block) =
                    element
                {
                    for element in &selector_block.elements {
                        if let pax_manifest::TimelineSelectorElement::Track(_, track) = element {
                            if let Some(playhead) = &track.playhead {
                                collect_value_definition(playhead, &mut seen, &mut collected);
                            }
                            if let Some(starting_value) = &track.starting_value {
                                collect_value_definition(starting_value, &mut seen, &mut collected);
                            }
                            for element in &track.elements {
                                if let pax_manifest::TimelineTrackElement::Keyframe(keyframe) =
                                    element
                                {
                                    collect_value_definition(
                                        &keyframe.value,
                                        &mut seen,
                                        &mut collected,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    collected
}

fn collect_settings_block_elements(
    settings: &[SettingsBlockElement],
    seen: &mut HashSet<WebFontSource>,
    collected: &mut Vec<WebFontSource>,
) {
    for setting in settings {
        match setting {
            SettingsBlockElement::SelectorBlock(_, block) => {
                collect_literal_block_definition(block, seen, collected);
            }
            SettingsBlockElement::Handler(_, _)
            | SettingsBlockElement::Transition(_, _)
            | SettingsBlockElement::Comment(_) => {}
        }
    }
}

fn collect_setting_elements(
    settings: &[SettingElement],
    seen: &mut HashSet<WebFontSource>,
    collected: &mut Vec<WebFontSource>,
) {
    for setting in settings {
        match setting {
            SettingElement::Setting(_, value) => {
                collect_value_definition(value, seen, collected);
            }
            SettingElement::Comment(_) => {}
        }
    }
}

fn collect_literal_block_definition(
    block: &LiteralBlockDefinition,
    seen: &mut HashSet<WebFontSource>,
    collected: &mut Vec<WebFontSource>,
) {
    collect_setting_elements(&block.elements, seen, collected);
}

fn collect_value_definition(
    value: &ValueDefinition,
    seen: &mut HashSet<WebFontSource>,
    collected: &mut Vec<WebFontSource>,
) {
    match value {
        ValueDefinition::Block(block) => collect_literal_block_definition(block, seen, collected),
        ValueDefinition::Timeline(track) => {
            if let Some(duration) = &track.duration {
                collect_value_definition(duration, seen, collected);
            }
            if let Some(starting_value) = &track.starting_value {
                collect_value_definition(starting_value, seen, collected);
            }
            for element in &track.elements {
                if let pax_manifest::TimelineTrackElement::Keyframe(keyframe) = element {
                    collect_value_definition(&keyframe.value, seen, collected);
                }
            }
        }
        ValueDefinition::Transition(transition) => {
            if let Some(starting_value) = &transition.starting_value {
                collect_value_definition(starting_value, seen, collected);
            }
            for track in [&transition.enter, &transition.exit].into_iter().flatten() {
                if let Some(duration) = &track.duration {
                    collect_value_definition(duration, seen, collected);
                }
                if let Some(starting_value) = &track.starting_value {
                    collect_value_definition(starting_value, seen, collected);
                }
                for element in &track.elements {
                    if let pax_manifest::TimelineTrackElement::Keyframe(keyframe) = element {
                        collect_value_definition(&keyframe.value, seen, collected);
                    }
                }
            }
        }
        ValueDefinition::Expression(expression_info) => {
            collect_font_sources_from_expression(&expression_info.expression, seen, collected);
        }
        ValueDefinition::LiteralValue(literal_value) => {
            let Ok(serialized) = serde_json::to_value(literal_value) else {
                return;
            };
            collect_font_sources_from_serialized_json(&serialized, seen, collected);
        }
        ValueDefinition::Undefined
        | ValueDefinition::Identifier(_)
        | ValueDefinition::DoubleBinding(_)
        | ValueDefinition::EventBindingTarget(_) => {}
    }
}

fn collect_font_sources_from_expression(
    expression: &PaxExpression,
    seen: &mut HashSet<WebFontSource>,
    collected: &mut Vec<WebFontSource>,
) {
    let Ok(serialized) = serde_json::to_value(expression) else {
        return;
    };
    collect_font_sources_from_serialized_json(&serialized, seen, collected);
}

fn collect_font_sources_from_serialized_json(
    value: &JsonValue,
    seen: &mut HashSet<WebFontSource>,
    collected: &mut Vec<WebFontSource>,
) {
    match value {
        JsonValue::Object(map) => {
            if let Some(function_or_enum) = map.get("FunctionOrEnum") {
                if let Some(font_source) = parse_font_web_source(function_or_enum) {
                    if seen.insert(font_source.clone()) {
                        collected.push(font_source);
                    }
                }
            }
            if let Some(enum_value) = map.get("Enum") {
                if let Some(font_source) = parse_font_web_source(enum_value) {
                    if seen.insert(font_source.clone()) {
                        collected.push(font_source);
                    }
                }
            }

            for child in map.values() {
                collect_font_sources_from_serialized_json(child, seen, collected);
            }
        }
        JsonValue::Array(items) => {
            for item in items {
                collect_font_sources_from_serialized_json(item, seen, collected);
            }
        }
        JsonValue::Null | JsonValue::Bool(_) | JsonValue::Number(_) | JsonValue::String(_) => {}
    }
}

fn parse_font_web_source(value: &JsonValue) -> Option<WebFontSource> {
    let JsonValue::Array(parts) = value else {
        return None;
    };
    if parts.len() != 3 {
        return None;
    }

    let name = parts.first()?.as_str()?;
    let enum_variant = parts.get(1)?.as_str()?;
    if name != "Font" || enum_variant != "Web" {
        return None;
    }

    let JsonValue::Array(args) = parts.get(2)? else {
        return None;
    };
    if args.len() < 2 {
        return None;
    }

    let family = extract_string_literal(&args[0])?;
    let url = extract_string_literal(&args[1])?;

    Some(WebFontSource { family, url })
}

fn extract_string_literal(value: &JsonValue) -> Option<String> {
    match value {
        JsonValue::String(value) => Some(value.clone()),
        JsonValue::Object(map) => {
            if let Some(string_value) = map.get("String") {
                return string_value.as_str().map(ToString::to_string);
            }
            if let Some(primary) = map.get("Primary") {
                return extract_string_literal(primary);
            }
            if let Some(literal) = map.get("Literal") {
                return literal.as_str().map(ToString::to_string);
            }
            None
        }
        JsonValue::Array(items) => items.iter().find_map(extract_string_literal),
        JsonValue::Null | JsonValue::Bool(_) | JsonValue::Number(_) => None,
    }
}

fn vendor_web_font_source(
    client: &Client,
    font_source: &WebFontSource,
    resources_dir: &Path,
    vendored_assets: &mut HashSet<String>,
    vendored_count: &mut usize,
) -> eyre::Result<(), Report> {
    let url = Url::parse(&font_source.url)?;
    if url.as_str().contains("fonts.googleapis.com/css") {
        let css = client
            .get(url.clone())
            .header(reqwest::header::USER_AGENT, "curl/8.7.1")
            .send()?
            .error_for_status()?
            .text()?;
        let asset_urls = parse_css_font_urls(&css, &url);
        for asset_url in asset_urls {
            vendor_font_asset(
                client,
                &font_source.family,
                &asset_url,
                resources_dir,
                vendored_assets,
                vendored_count,
            )?;
        }
    } else {
        vendor_font_asset(
            client,
            &font_source.family,
            &url,
            resources_dir,
            vendored_assets,
            vendored_count,
        )?;
    }

    Ok(())
}

fn parse_css_font_urls(css: &str, base_url: &Url) -> Vec<Url> {
    let mut urls = Vec::new();
    let mut seen = HashSet::new();

    let mut remaining = css;
    while let Some(start) = remaining.find("url(") {
        let after_prefix = &remaining[start + 4..];
        let Some(end) = after_prefix.find(')') else {
            break;
        };
        let raw_value = after_prefix[..end]
            .trim()
            .trim_matches(|character| matches!(character, '"' | '\''));

        if let Ok(resolved_url) = base_url.join(raw_value) {
            if seen.insert(resolved_url.as_str().to_string()) {
                urls.push(resolved_url);
            }
        }

        remaining = &after_prefix[end + 1..];
    }

    urls
}

fn vendor_font_asset(
    client: &Client,
    family: &str,
    asset_url: &Url,
    resources_dir: &Path,
    vendored_assets: &mut HashSet<String>,
    vendored_count: &mut usize,
) -> eyre::Result<(), Report> {
    if !vendored_assets.insert(asset_url.as_str().to_string()) {
        return Ok(());
    }

    let response = client.get(asset_url.clone()).send()?.error_for_status()?;
    let bytes = response.bytes()?;
    let extension = asset_url
        .path_segments()
        .and_then(|segments| segments.last())
        .and_then(|segment| {
            segment
                .rsplit_once('.')
                .map(|(_, ext)| ext.to_ascii_lowercase())
        })
        .filter(|ext| !ext.is_empty())
        .unwrap_or_else(|| "font".to_string());

    let file_name = format!(
        "pax-font-{}-{}.{}",
        sanitize_file_stem(family),
        stable_hash(asset_url.as_str()),
        extension
    );
    let destination = resources_dir.join(file_name);
    fs::write(&destination, bytes)?;
    *vendored_count += 1;

    Ok(())
}

fn sanitize_file_stem(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    sanitized.trim_matches('-').to_string()
}

fn stable_hash(value: &str) -> String {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_font_web_literal_enum_shape() {
        let value = json!([
            "Font",
            "Web",
            [
                { "String": "Oxanium" },
                { "String": "https://fonts.googleapis.com/css2?family=Oxanium:wght@400;600;700;800&display=swap" },
                { "Enum": ["FontStyle", "Normal", []] },
                { "Enum": ["FontWeight", "Bold", []] }
            ]
        ]);

        let source = parse_font_web_source(&value).expect("expected font source");
        assert_eq!(source.family, "Oxanium");
        assert_eq!(
            source.url,
            "https://fonts.googleapis.com/css2?family=Oxanium:wght@400;600;700;800&display=swap"
        );
    }

    #[test]
    fn collects_font_web_sources_from_literal_value_json() {
        let value = json!({
            "LiteralValue": {
                "Object": [
                    [
                        "font",
                        {
                            "Enum": [
                                "Font",
                                "Web",
                                [
                                    { "String": "Space Mono" },
                                    { "String": "https://fonts.googleapis.com/css2?family=Space+Mono:wght@400;700&display=swap" }
                                ]
                            ]
                        }
                    ]
                ]
            }
        });

        let mut seen = HashSet::new();
        let mut collected = Vec::new();
        collect_font_sources_from_serialized_json(&value, &mut seen, &mut collected);

        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0].family, "Space Mono");
        assert_eq!(
            collected[0].url,
            "https://fonts.googleapis.com/css2?family=Space+Mono:wght@400;700&display=swap"
        );
    }

    #[test]
    fn parses_css_font_urls_from_google_fonts_stylesheet() {
        let css = "@font-face {\n  font-family: 'Oxanium';\n  src: url(https://fonts.gstatic.com/s/oxanium/v20/RrQQboN_4yJ0JmiMe2LE0Q.woff2) format('woff2');\n}\n@font-face {\n  src: url('https://fonts.gstatic.com/s/oxanium/v20/RrQQboN_4yJ0JmiMe2zE0Q.woff2') format('woff2');\n}";
        let base_url = Url::parse(
            "https://fonts.googleapis.com/css2?family=Oxanium:wght@400;600;700;800&display=swap",
        )
        .expect("expected valid base url");

        let urls = parse_css_font_urls(css, &base_url);
        assert_eq!(urls.len(), 2);
        assert_eq!(
            urls[0].as_str(),
            "https://fonts.gstatic.com/s/oxanium/v20/RrQQboN_4yJ0JmiMe2LE0Q.woff2"
        );
        assert_eq!(
            urls[1].as_str(),
            "https://fonts.gstatic.com/s/oxanium/v20/RrQQboN_4yJ0JmiMe2zE0Q.woff2"
        );
    }

    #[test]
    fn web_interface_bundle_rebuilds_when_bundle_is_missing() {
        let dir = web_interface_fixture();
        fs::remove_file(dir.path().join(WEB_INTERFACE_BUNDLE_FILE))
            .expect("bundle should be removable");

        assert!(web_interface_bundle_needs_rebuild(dir.path(), true));
    }

    #[test]
    fn web_interface_bundle_skips_fingerprint_check_for_non_libdev_cached_bundle() {
        let dir = web_interface_fixture();

        assert!(!web_interface_bundle_needs_rebuild(dir.path(), false));
    }

    #[test]
    fn web_interface_bundle_rebuilds_when_libdev_fingerprint_is_missing() {
        let dir = web_interface_fixture();

        assert!(web_interface_bundle_needs_rebuild(dir.path(), true));
    }

    #[test]
    fn web_interface_bundle_skips_rebuild_when_libdev_fingerprint_matches() {
        let dir = web_interface_fixture();
        write_web_interface_fingerprint(dir.path()).expect("fingerprint should be written");

        assert!(!web_interface_bundle_needs_rebuild(dir.path(), true));
    }

    #[test]
    fn web_interface_bundle_rebuilds_when_source_content_changes() {
        let dir = web_interface_fixture();
        write_web_interface_fingerprint(dir.path()).expect("fingerprint should be written");
        write_file(&dir.path().join("src/index.ts"), b"console.log('changed');");

        assert!(web_interface_bundle_needs_rebuild(dir.path(), true));
    }

    #[test]
    fn web_interface_bundle_rebuilds_when_source_file_is_added() {
        let dir = web_interface_fixture();
        write_web_interface_fingerprint(dir.path()).expect("fingerprint should be written");
        write_file(&dir.path().join("src/extra.ts"), b"console.log('extra');");

        assert!(web_interface_bundle_needs_rebuild(dir.path(), true));
    }

    #[test]
    fn web_interface_bundle_rebuilds_when_source_file_is_deleted() {
        let dir = web_interface_fixture();
        write_web_interface_fingerprint(dir.path()).expect("fingerprint should be written");
        fs::remove_file(dir.path().join("src/index.ts")).expect("source file should be removable");

        assert!(web_interface_bundle_needs_rebuild(dir.path(), true));
    }

    fn web_interface_fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir should be created");
        write_file(&dir.path().join("build-interface.sh"), b"#!/bin/sh\n");
        write_file(&dir.path().join("package.json"), b"{}");
        write_file(&dir.path().join("tsconfig.json"), b"{}");
        write_file(&dir.path().join("src/index.ts"), b"console.log('pax');");
        write_file(&dir.path().join(WEB_INTERFACE_BUNDLE_FILE), b"bundle");
        dir
    }

    fn write_file(path: &Path, bytes: &[u8]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("fixture parent should be created");
        }
        fs::write(path, bytes).expect("fixture file should be written");
    }

    #[test]
    fn ipad_alias_maps_to_ipados_target() {
        assert_eq!(RunTarget::parse("ipad"), Ok(RunTarget::iPadOS));
        assert_eq!(RunTarget::parse("ipados"), Ok(RunTarget::iPadOS));
    }

    #[test]
    fn invalid_target_returns_error_instead_of_unreachable() {
        let error = RunTarget::parse("fridge").expect_err("expected invalid target");
        assert!(error.contains("unsupported target `fridge`"));
    }

    fn release_context(should_run_designtime: bool, should_run_designer: bool) -> RunContext {
        RunContext {
            target: RunTarget::Web,
            project_path: PathBuf::from("."),
            verbose: false,
            should_also_run: false,
            is_libdev_mode: false,
            process_child_ids: Arc::new(Mutex::new(vec![])),
            should_run_designtime,
            should_run_designer,
            is_release: true,
            profile_wasm_size: false,
            webgl: false,
            ios_device: None,
            ios_development_team: None,
        }
    }

    #[test]
    fn release_build_rejects_devtime_features() {
        for ctx in [release_context(true, false), release_context(false, true)] {
            let error = match prepare_cartridge_sources(&ctx) {
                Ok(_) => panic!(
                    "release builds should reject designtime and designer cartridge contexts"
                ),
                Err(error) => error,
            };
            assert!(error
                .to_string()
                .contains("Release builds do not support designtime or designer features"));
        }
    }
}

/// Ejects the interface files for the specified target platform
/// Interface files will then be used to build the project
pub fn perform_eject(ctx: &RunContext) -> eyre::Result<(), Report> {
    let pax_dir = get_or_create_pax_directory(&ctx.project_path);
    eject_interface_files(ctx, &pax_dir);
    Ok(())
}

fn eject_interface_files(ctx: &RunContext, pax_dir: &PathBuf) {
    let target_str: &str = (&ctx.target).into();
    let target_str_lower = &target_str.to_lowercase();
    let custom_interfaces_dir = pax_dir.parent().unwrap().join("interfaces");
    let mut target_custom_interface_dir = custom_interfaces_dir.join(target_str_lower);
    if ctx.target == RunTarget::Web {
        target_custom_interface_dir = target_custom_interface_dir.join("public");
    }

    let _ = fs::create_dir_all(&target_custom_interface_dir);

    let src_path = get_libdev_interface_path(ctx);
    if ctx.is_libdev_mode || src_path.exists() {
        let _ = copy_dir_recursively(&src_path, &target_custom_interface_dir, &[]);
    } else {
        let _ = extract_interface_template(ctx, &target_custom_interface_dir);
    }

    println!(
        "Interface files ejected to: {}",
        target_custom_interface_dir.display()
    );
}

fn get_libdev_interface_path(ctx: &RunContext) -> PathBuf {
    let pax_compiler_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    match ctx.target {
        RunTarget::Web => pax_compiler_root
            .join("files")
            .join("interfaces")
            .join("web")
            .join("public"),
        RunTarget::macOS => pax_compiler_root
            .join("files")
            .join("interfaces")
            .join("macos")
            .join("pax-app-macos"),
        RunTarget::iOS | RunTarget::iPadOS => pax_compiler_root
            .join("files")
            .join("interfaces")
            .join("ios")
            .join("pax-app-ios"),
    }
}

fn extract_interface_template(ctx: &RunContext, dest: &Path) -> Result<(), std::io::Error> {
    match ctx.target {
        RunTarget::Web => PAX_WEB_INTERFACE_TEMPLATE.extract(dest)?,
        RunTarget::macOS => PAX_MACOS_INTERFACE_TEMPLATE.extract(dest)?,
        RunTarget::iOS | RunTarget::iPadOS => PAX_IOS_INTERFACE_TEMPLATE.extract(dest)?,
    }
    Ok(())
}

/// Clean all `.pax` temp files
pub fn perform_clean(path: &str) {
    let path = PathBuf::from(path);
    let pax_dir = path.join(".pax");
    fs::remove_dir_all(&pax_dir).ok();
}

pub struct CreateContext {
    pub path: String,
    pub is_libdev_mode: bool,
    pub version: String,
}

pub fn perform_create(ctx: &CreateContext) {
    let full_path = Path::new(&ctx.path);

    // Abort if directory already exists
    if full_path.exists() {
        panic!("Error: destination `{:?}` already exists", full_path);
    }
    let _ = fs::create_dir_all(&full_path);

    // clone template into full_path
    if ctx.is_libdev_mode {
        //For is_libdev_mode, we copy our monorepo @/pax-compiler/new-project-template directory
        //to the target directly.  This enables iterating on new-project-template during libdev
        //without the sticky caches associated with `include_dir`
        let pax_compiler_cargo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let template_src = pax_compiler_cargo_root
            .join("files")
            .join("new-project")
            .join(PAX_CREATE_LIBDEV_TEMPLATE_DIR_NAME);

        let mut options = CopyOptions::new();
        options.overwrite = true;

        for entry in std::fs::read_dir(&template_src).expect("Failed to read template directory") {
            let entry_path = entry.expect("Failed to read entry").path();
            if entry_path.is_dir() {
                dir::copy(&entry_path, &full_path, &options).expect("Failed to copy directory");
            } else {
                fs::copy(&entry_path, full_path.join(entry_path.file_name().unwrap()))
                    .expect("Failed to copy file");
            }
        }
    } else {
        // File src is include_dir — recursively extract files from include_dir into full_path
        PAX_CREATE_TEMPLATE
            .extract(&full_path)
            .expect("Failed to extract files");
    }

    //Patch Cargo.toml
    let cargo_template_path = full_path.join("Cargo.toml.template");
    let extracted_cargo_toml_path = full_path.join("Cargo.toml");
    let _ = fs::copy(&cargo_template_path, &extracted_cargo_toml_path);
    let _ = fs::remove_file(&cargo_template_path);

    let crate_name = full_path.file_name().unwrap().to_str().unwrap().to_string();

    // Read the Cargo.toml
    let mut doc = fs::read_to_string(&full_path.join("Cargo.toml"))
        .expect("Failed to read Cargo.toml")
        .parse::<toml_edit::Document>()
        .expect("Failed to parse Cargo.toml");

    // Update the `dependencies` section
    update_pax_dependency_versions(&mut doc, &ctx.version);

    // Update the `package` section
    if let Some(package) = doc
        .as_table_mut()
        .entry("package")
        .or_insert_with(toml_edit::table)
        .as_table_mut()
    {
        if let Some(name_item) = package.get_mut("name") {
            *name_item = toml_edit::Item::Value(crate_name.into());
        }
        if let Some(version_item) = package.get_mut("version") {
            *version_item = toml_edit::Item::Value(ctx.version.clone().into());
        }
    }

    // Write the modified Cargo.toml back to disk
    fs::write(&full_path.join("Cargo.toml"), doc.to_string())
        .expect("Failed to write modified Cargo.toml");

    ensure_claude_md_link(full_path);

    println!(
        "\nCreated new Pax project at {}.\nTo run:\n  `cd {} && pax-cli run --target=web`",
        full_path.to_str().unwrap(),
        full_path.to_str().unwrap()
    );
}

fn ensure_claude_md_link(project_root: &Path) {
    let claude_path = project_root.join("CLAUDE.md");
    if let Ok(metadata) = claude_path.symlink_metadata() {
        if metadata.file_type().is_symlink() {
            return;
        }
        if metadata.is_dir() {
            fs::remove_dir_all(&claude_path).expect("Failed to replace CLAUDE.md directory");
        } else {
            fs::remove_file(&claude_path).expect("Failed to replace CLAUDE.md file");
        }
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("AGENTS.md", &claude_path)
            .expect("Failed to create CLAUDE.md symlink");
    }

    #[cfg(not(unix))]
    {
        fs::copy(project_root.join("AGENTS.md"), &claude_path)
            .expect("Failed to copy CLAUDE.md from AGENTS.md");
    }
}

/// Executes a shell command to run the feature-flagged parser at the specified path
/// Returns an output object containing bytestreams of stdout/stderr as well as an exit code
pub fn run_parser_binary(
    project_path: &PathBuf,
    process_child_ids: Arc<Mutex<Vec<u64>>>,
    should_run_designtime: bool,
    should_run_designer: bool,
) -> Output {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(project_path)
        .arg("run")
        .arg("--bin")
        .arg("parser")
        .arg("--features")
        .arg("parser")
        .arg("--profile")
        .arg("parser")
        .arg("--color")
        .arg("always")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    if should_run_designer {
        cmd.arg("--features").arg("designer");
    } else if should_run_designtime {
        cmd.arg("--features").arg("designtime");
    }

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(pre_exec_hook);
    }

    let child = cmd.spawn().expect(ERR_SPAWN);

    // child.stdin.take().map(drop);
    let output = wait_with_output(&process_child_ids, child);
    output
}

impl RunTarget {
    pub fn parse(input: &str) -> Result<Self, String> {
        match input.to_lowercase().as_str() {
            "macos" => Ok(RunTarget::macOS),
            "web" => Ok(RunTarget::Web),
            "ios" => Ok(RunTarget::iOS),
            "ipados" | "ipad" => Ok(RunTarget::iPadOS),
            _ => Err(format!(
                "unsupported target `{input}`; expected one of: web, macos, ios, ipados"
            )),
        }
    }
}

impl From<&str> for RunTarget {
    fn from(input: &str) -> Self {
        Self::parse(input).unwrap_or_else(|error| panic!("{error}"))
    }
}

impl<'a> Into<&'a str> for &'a RunTarget {
    fn into(self) -> &'a str {
        match self {
            RunTarget::Web => "Web",
            RunTarget::macOS => "macOS",
            RunTarget::iOS => "iOS",
            RunTarget::iPadOS => "iPadOS",
        }
    }
}

#[cfg(unix)]
fn pre_exec_hook() -> Result<(), std::io::Error> {
    // Set a new process group for this command
    unsafe {
        libc::setpgid(0, 0);
    }
    Ok(())
}
