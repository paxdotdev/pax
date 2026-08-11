use crate::helpers::INTERFACE_DIR_NAME;
use crate::{RunContext, RunTarget};
use color_eyre::eyre;
use eyre::{eyre, WrapErr};
use image::imageops::FilterType;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::{Document, Item, Table};

#[derive(Debug, Clone, Default)]
pub struct PaxProjectMetadata {
    manifest_dir: PathBuf,
    package_name: Option<String>,
    package_version: Option<String>,
    common: CommonMetadata,
    dev: DevMetadata,
    web: WebMetadata,
    ios: AppleMetadata,
    ipados: AppleMetadata,
    macos: AppleMetadata,
}

#[derive(Debug, Clone, Default)]
struct DevMetadata {
    hot_reload: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct CommonMetadata {
    title: Option<String>,
    icon: Option<String>,
    bundle_identifier: Option<String>,
    marketing_version: Option<String>,
    build_number: Option<String>,
    development_team: Option<String>,
    info_plist: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default)]
struct WebMetadata {
    title: Option<String>,
    icon: Option<String>,
    favicon: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct AppleMetadata {
    title: Option<String>,
    icon: Option<String>,
    bundle_identifier: Option<String>,
    marketing_version: Option<String>,
    build_number: Option<String>,
    development_team: Option<String>,
    info_plist: BTreeMap<String, String>,
}

pub fn load_project_metadata(project_path: &Path) -> Result<PaxProjectMetadata, eyre::Report> {
    let manifest_path = canonical_manifest_path(project_path)?;
    let manifest_dir = manifest_path
        .parent()
        .ok_or_else(|| eyre!("Project manifest has no parent directory"))?
        .to_path_buf();
    let contents = fs::read_to_string(&manifest_path)
        .wrap_err_with(|| format!("Failed to read `{}`", manifest_path.display()))?;
    load_project_metadata_from_toml(&manifest_dir, &contents).wrap_err_with(|| {
        format!(
            "Failed to parse Pax project metadata in `{}`",
            manifest_path.display()
        )
    })
}

pub fn apply_copied_interface_metadata(
    ctx: &RunContext,
    pax_dir: &Path,
    metadata: &PaxProjectMetadata,
) -> Result<(), eyre::Report> {
    match ctx.target {
        RunTarget::Web => apply_web_metadata(pax_dir, metadata),
        RunTarget::macOS | RunTarget::iOS | RunTarget::iPadOS => {
            apply_apple_icon_metadata(&ctx.target, pax_dir, metadata)
        }
    }
}

impl PaxProjectMetadata {
    pub(crate) fn manifest_dir(&self) -> &Path {
        &self.manifest_dir
    }

    pub(crate) fn configured_hot_reload(&self) -> Option<&str> {
        self.dev.hot_reload.as_deref()
    }

    pub fn apple_xcode_build_settings(&self, target: &RunTarget) -> Vec<(String, String)> {
        let mut settings = Vec::new();
        if let Some(title) = self.apple_title(target) {
            settings.push((
                "INFOPLIST_KEY_CFBundleDisplayName".to_string(),
                title.clone(),
            ));
            settings.push(("INFOPLIST_KEY_CFBundleName".to_string(), title));
        }
        if let Some(bundle_identifier) = self.apple_bundle_identifier(target) {
            settings.push(("PRODUCT_BUNDLE_IDENTIFIER".to_string(), bundle_identifier));
        }
        if let Some(marketing_version) = self.apple_marketing_version(target) {
            settings.push(("MARKETING_VERSION".to_string(), marketing_version));
        }
        if let Some(build_number) = self.apple_build_number(target) {
            settings.push(("CURRENT_PROJECT_VERSION".to_string(), build_number));
        }
        for (key, value) in self.apple_info_plist(target) {
            settings.push((format!("INFOPLIST_KEY_{key}"), value));
        }
        settings
    }

    pub fn apple_development_team(&self, target: &RunTarget) -> Option<String> {
        self.apple_target_metadata(target)
            .and_then(|metadata| metadata.development_team.clone())
            .or_else(|| {
                if matches!(target, RunTarget::iPadOS) {
                    self.ios.development_team.clone()
                } else {
                    None
                }
            })
            .or_else(|| self.common.development_team.clone())
    }

    fn web_title(&self) -> Option<String> {
        self.web
            .title
            .clone()
            .or_else(|| self.common.title.clone())
            .or_else(|| self.package_name.clone())
    }

    fn web_favicon_source(&self) -> Option<&str> {
        self.web.favicon.as_deref()
    }

    fn web_icon_source(&self) -> Option<&str> {
        self.web.icon.as_deref().or(self.common.icon.as_deref())
    }

    fn apple_title(&self, target: &RunTarget) -> Option<String> {
        self.apple_target_metadata(target)
            .and_then(|metadata| metadata.title.clone())
            .or_else(|| {
                if matches!(target, RunTarget::iPadOS) {
                    self.ios.title.clone()
                } else {
                    None
                }
            })
            .or_else(|| self.common.title.clone())
            .or_else(|| self.package_name.clone())
    }

    fn apple_icon_source(&self, target: &RunTarget) -> Option<&str> {
        self.apple_target_metadata(target)
            .and_then(|metadata| metadata.icon.as_deref())
            .or_else(|| {
                if matches!(target, RunTarget::iPadOS) {
                    self.ios.icon.as_deref()
                } else {
                    None
                }
            })
            .or(self.common.icon.as_deref())
    }

    fn apple_bundle_identifier(&self, target: &RunTarget) -> Option<String> {
        self.apple_target_metadata(target)
            .and_then(|metadata| metadata.bundle_identifier.clone())
            .or_else(|| {
                if matches!(target, RunTarget::iPadOS) {
                    self.ios.bundle_identifier.clone()
                } else {
                    None
                }
            })
            .or_else(|| self.common.bundle_identifier.clone())
    }

    fn apple_marketing_version(&self, target: &RunTarget) -> Option<String> {
        self.apple_target_metadata(target)
            .and_then(|metadata| metadata.marketing_version.clone())
            .or_else(|| {
                if matches!(target, RunTarget::iPadOS) {
                    self.ios.marketing_version.clone()
                } else {
                    None
                }
            })
            .or_else(|| self.common.marketing_version.clone())
            .or_else(|| self.package_version.clone())
    }

    fn apple_build_number(&self, target: &RunTarget) -> Option<String> {
        self.apple_target_metadata(target)
            .and_then(|metadata| metadata.build_number.clone())
            .or_else(|| {
                if matches!(target, RunTarget::iPadOS) {
                    self.ios.build_number.clone()
                } else {
                    None
                }
            })
            .or_else(|| self.common.build_number.clone())
    }

    fn apple_info_plist(&self, target: &RunTarget) -> BTreeMap<String, String> {
        let mut entries = self.common.info_plist.clone();
        if matches!(target, RunTarget::iPadOS) {
            entries.extend(self.ios.info_plist.clone());
        }
        if let Some(metadata) = self.apple_target_metadata(target) {
            entries.extend(metadata.info_plist.clone());
        }
        entries
    }

    fn apple_target_metadata(&self, target: &RunTarget) -> Option<&AppleMetadata> {
        match target {
            RunTarget::macOS => Some(&self.macos),
            RunTarget::iOS => Some(&self.ios),
            RunTarget::iPadOS => Some(&self.ipados),
            RunTarget::Web => None,
        }
    }

    fn resolve_project_path(&self, path: &str) -> PathBuf {
        let path = PathBuf::from(path);
        if path.is_absolute() {
            path
        } else {
            self.manifest_dir.join(path)
        }
    }
}

fn canonical_manifest_path(project_path: &Path) -> Result<PathBuf, eyre::Report> {
    let manifest_path = if project_path.is_dir() {
        project_path.join("Cargo.toml")
    } else {
        project_path.to_path_buf()
    };
    manifest_path.canonicalize().map_err(|err| {
        eyre!(
            "Failed to canonicalize `{}`: {err}",
            manifest_path.display()
        )
    })
}

fn load_project_metadata_from_toml(
    manifest_dir: &Path,
    contents: &str,
) -> Result<PaxProjectMetadata, eyre::Report> {
    let document = contents.parse::<Document>()?;
    let package = document
        .get("package")
        .and_then(Item::as_table)
        .ok_or_else(|| eyre!("Cargo.toml is missing a `[package]` table"))?;

    let package_name = string_field(package, "name", "package.name")?;
    let package_version = string_field(package, "version", "package.version")?;

    let mut metadata = PaxProjectMetadata {
        manifest_dir: manifest_dir.to_path_buf(),
        package_name,
        package_version,
        ..Default::default()
    };

    let Some(pax) = package
        .get("metadata")
        .and_then(Item::as_table)
        .and_then(|metadata| metadata.get("pax"))
        .and_then(Item::as_table)
    else {
        return Ok(metadata);
    };

    metadata.common = parse_common_metadata(pax, "package.metadata.pax")?;
    if let Some(dev) = child_table(pax, "dev", "package.metadata.pax.dev")? {
        metadata.dev = parse_dev_metadata(dev, "package.metadata.pax.dev")?;
    }
    if let Some(web) = child_table(pax, "web", "package.metadata.pax.web")? {
        metadata.web = parse_web_metadata(web, "package.metadata.pax.web")?;
    }
    if let Some(ios) = child_table(pax, "ios", "package.metadata.pax.ios")? {
        metadata.ios = parse_apple_metadata(ios, "package.metadata.pax.ios")?;
    }
    if let Some(ipados) = child_table(pax, "ipados", "package.metadata.pax.ipados")? {
        metadata.ipados = parse_apple_metadata(ipados, "package.metadata.pax.ipados")?;
    }
    if let Some(macos) = child_table(pax, "macos", "package.metadata.pax.macos")? {
        metadata.macos = parse_apple_metadata(macos, "package.metadata.pax.macos")?;
    }

    Ok(metadata)
}

fn parse_dev_metadata(table: &Table, path: &str) -> Result<DevMetadata, eyre::Report> {
    Ok(DevMetadata {
        hot_reload: string_field(table, "hot_reload", &format!("{path}.hot_reload"))?,
    })
}

fn child_table<'a>(
    table: &'a Table,
    key: &str,
    path: &str,
) -> Result<Option<&'a Table>, eyre::Report> {
    let Some(item) = table.get(key) else {
        return Ok(None);
    };
    item.as_table()
        .map(Some)
        .ok_or_else(|| eyre!("`{path}` must be a TOML table"))
}

fn parse_common_metadata(table: &Table, path: &str) -> Result<CommonMetadata, eyre::Report> {
    Ok(CommonMetadata {
        title: string_field(table, "title", &format!("{path}.title"))?,
        icon: string_field(table, "icon", &format!("{path}.icon"))?,
        bundle_identifier: string_field(
            table,
            "bundle_identifier",
            &format!("{path}.bundle_identifier"),
        )?,
        marketing_version: string_field(
            table,
            "marketing_version",
            &format!("{path}.marketing_version"),
        )?,
        build_number: string_field(table, "build_number", &format!("{path}.build_number"))?,
        development_team: string_field(
            table,
            "development_team",
            &format!("{path}.development_team"),
        )?,
        info_plist: string_table(table, "info_plist", &format!("{path}.info_plist"))?,
    })
}

fn parse_web_metadata(table: &Table, path: &str) -> Result<WebMetadata, eyre::Report> {
    Ok(WebMetadata {
        title: string_field(table, "title", &format!("{path}.title"))?,
        icon: string_field(table, "icon", &format!("{path}.icon"))?,
        favicon: string_field(table, "favicon", &format!("{path}.favicon"))?,
    })
}

fn parse_apple_metadata(table: &Table, path: &str) -> Result<AppleMetadata, eyre::Report> {
    Ok(AppleMetadata {
        title: string_field(table, "title", &format!("{path}.title"))?,
        icon: string_field(table, "icon", &format!("{path}.icon"))?,
        bundle_identifier: string_field(
            table,
            "bundle_identifier",
            &format!("{path}.bundle_identifier"),
        )?,
        marketing_version: string_field(
            table,
            "marketing_version",
            &format!("{path}.marketing_version"),
        )?,
        build_number: string_field(table, "build_number", &format!("{path}.build_number"))?,
        development_team: string_field(
            table,
            "development_team",
            &format!("{path}.development_team"),
        )?,
        info_plist: string_table(table, "info_plist", &format!("{path}.info_plist"))?,
    })
}

fn string_field(table: &Table, key: &str, path: &str) -> Result<Option<String>, eyre::Report> {
    let Some(item) = table.get(key) else {
        return Ok(None);
    };
    item.as_str()
        .map(|value| Some(value.to_string()))
        .ok_or_else(|| eyre!("`{path}` must be a string"))
}

fn string_table(
    table: &Table,
    key: &str,
    path: &str,
) -> Result<BTreeMap<String, String>, eyre::Report> {
    let Some(table) = child_table(table, key, path)? else {
        return Ok(BTreeMap::new());
    };
    let mut entries = BTreeMap::new();
    for (entry_key, item) in table.iter() {
        let Some(value) = item.as_str() else {
            return Err(eyre!("`{path}.{entry_key}` must be a string"));
        };
        entries.insert(entry_key.to_string(), value.to_string());
    }
    Ok(entries)
}

fn apply_web_metadata(pax_dir: &Path, metadata: &PaxProjectMetadata) -> Result<(), eyre::Report> {
    let interface_path = pax_dir.join(INTERFACE_DIR_NAME).join("web");
    let index_path = interface_path.join("index.html");
    if !index_path.exists() {
        return Ok(());
    }

    let mut html = fs::read_to_string(&index_path)?;
    if let Some(title) = metadata.web_title() {
        html = set_or_insert_html_title(&html, &title);
    }
    if let Some(favicon_href) = materialize_web_favicon(&interface_path, metadata)? {
        html = set_or_insert_favicon_link(&html, &favicon_href);
    }
    fs::write(index_path, html)?;
    Ok(())
}

fn materialize_web_favicon(
    interface_path: &Path,
    metadata: &PaxProjectMetadata,
) -> Result<Option<String>, eyre::Report> {
    if let Some(favicon_source) = metadata.web_favicon_source() {
        let source = metadata.resolve_project_path(favicon_source);
        ensure_asset_exists(&source)?;
        let extension = source
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("ico");
        let file_name = format!("pax-favicon.{extension}");
        fs::copy(&source, interface_path.join(&file_name))?;
        return Ok(Some(file_name));
    }

    let Some(icon_source) = metadata.web_icon_source() else {
        return Ok(None);
    };
    let source = metadata.resolve_project_path(icon_source);
    ensure_asset_exists(&source)?;
    write_resized_square_png(&source, &interface_path.join("pax-favicon.png"), 64, false)?;
    Ok(Some("pax-favicon.png".to_string()))
}

fn set_or_insert_html_title(html: &str, title: &str) -> String {
    let escaped_title = escape_html_text(title);
    let lower = html.to_lowercase();
    if let Some(start) = lower.find("<title>") {
        if let Some(relative_end) = lower[start..].find("</title>") {
            let content_start = start + "<title>".len();
            let end = start + relative_end;
            let mut output = String::new();
            output.push_str(&html[..content_start]);
            output.push_str(&escaped_title);
            output.push_str(&html[end..]);
            return output;
        }
    }

    if let Some(head_end) = lower.find("</head>") {
        let mut output = String::new();
        output.push_str(&html[..head_end]);
        output.push_str("        <title>");
        output.push_str(&escaped_title);
        output.push_str("</title>\n");
        output.push_str(&html[head_end..]);
        return output;
    }

    format!("<title>{escaped_title}</title>\n{html}")
}

fn set_or_insert_favicon_link(html: &str, href: &str) -> String {
    let without_existing_icon_links = html
        .lines()
        .filter(|line| {
            let lower = line.to_lowercase();
            !(lower.contains("rel=\"icon\"")
                || lower.contains("rel='icon'")
                || lower.contains("rel=icon"))
        })
        .collect::<Vec<_>>()
        .join("\n");
    let lower = without_existing_icon_links.to_lowercase();
    let link = format!(
        "        <link rel=\"icon\" href=\"{}\">\n",
        escape_html_attr(href)
    );

    if let Some(head_end) = lower.find("</head>") {
        let mut output = String::new();
        output.push_str(&without_existing_icon_links[..head_end]);
        output.push_str(&link);
        output.push_str(&without_existing_icon_links[head_end..]);
        return output;
    }

    format!("{link}{without_existing_icon_links}")
}

fn escape_html_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_html_attr(value: &str) -> String {
    escape_html_text(value).replace('"', "&quot;")
}

fn apply_apple_icon_metadata(
    target: &RunTarget,
    pax_dir: &Path,
    metadata: &PaxProjectMetadata,
) -> Result<(), eyre::Report> {
    let Some(icon_source) = metadata.apple_icon_source(target) else {
        return Ok(());
    };
    let source = metadata.resolve_project_path(icon_source);
    ensure_asset_exists(&source)?;

    match target {
        RunTarget::macOS => {
            let app_iconset_path = pax_dir
                .join(INTERFACE_DIR_NAME)
                .join("macos")
                .join("pax-app-macos")
                .join("pax-app-macos")
                .join("Assets.xcassets")
                .join("AppIcon.appiconset");
            write_macos_app_icon_set(&source, &app_iconset_path)
        }
        RunTarget::iOS | RunTarget::iPadOS => {
            let app_iconset_path = pax_dir
                .join(INTERFACE_DIR_NAME)
                .join("ios")
                .join("pax-app-ios")
                .join("pax-app-ios")
                .join("Assets.xcassets")
                .join("AppIcon.appiconset");
            write_ios_app_icon_set(&source, &app_iconset_path)
        }
        RunTarget::Web => Ok(()),
    }
}

fn ensure_asset_exists(path: &Path) -> Result<(), eyre::Report> {
    if path.is_file() {
        Ok(())
    } else {
        Err(eyre!(
            "Pax project metadata asset `{}` does not exist",
            path.display()
        ))
    }
}

fn write_ios_app_icon_set(source: &Path, app_iconset_path: &Path) -> Result<(), eyre::Report> {
    fs::create_dir_all(app_iconset_path)?;
    remove_pngs(app_iconset_path)?;
    let file_name = "pax-icon-ios-1024.png";
    write_resized_square_png(source, &app_iconset_path.join(file_name), 1024, true)?;
    let contents = json!({
        "images": [
            {
                "filename": file_name,
                "idiom": "universal",
                "platform": "ios",
                "size": "1024x1024"
            }
        ],
        "info": {
            "author": "xcode",
            "version": 1
        }
    });
    fs::write(
        app_iconset_path.join("Contents.json"),
        serde_json::to_string_pretty(&contents)?,
    )?;
    Ok(())
}

fn write_macos_app_icon_set(source: &Path, app_iconset_path: &Path) -> Result<(), eyre::Report> {
    fs::create_dir_all(app_iconset_path)?;
    remove_pngs(app_iconset_path)?;

    let variants = [
        ("pax-icon-macos-16.png", "16x16", "1x", 16u32),
        ("pax-icon-macos-16@2x.png", "16x16", "2x", 32u32),
        ("pax-icon-macos-32.png", "32x32", "1x", 32u32),
        ("pax-icon-macos-32@2x.png", "32x32", "2x", 64u32),
        ("pax-icon-macos-128.png", "128x128", "1x", 128u32),
        ("pax-icon-macos-128@2x.png", "128x128", "2x", 256u32),
        ("pax-icon-macos-256.png", "256x256", "1x", 256u32),
        ("pax-icon-macos-256@2x.png", "256x256", "2x", 512u32),
        ("pax-icon-macos-512.png", "512x512", "1x", 512u32),
        ("pax-icon-macos-512@2x.png", "512x512", "2x", 1024u32),
    ];

    for (file_name, _, _, size) in variants {
        write_resized_square_png(source, &app_iconset_path.join(file_name), size, false)?;
    }

    let images = variants
        .iter()
        .map(|(file_name, size, scale, _)| {
            json!({
                "filename": file_name,
                "idiom": "mac",
                "scale": scale,
                "size": size
            })
        })
        .collect::<Vec<_>>();
    let contents = json!({
        "images": images,
        "info": {
            "author": "xcode",
            "version": 1
        }
    });
    fs::write(
        app_iconset_path.join("Contents.json"),
        serde_json::to_string_pretty(&contents)?,
    )?;
    Ok(())
}

fn remove_pngs(dir: &Path) -> Result<(), eyre::Report> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.eq_ignore_ascii_case("png"))
            .unwrap_or(false)
        {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

fn write_resized_square_png(
    source: &Path,
    dest: &Path,
    size: u32,
    require_opaque: bool,
) -> Result<(), eyre::Report> {
    let image = image::open(source)
        .wrap_err_with(|| format!("Failed to read icon image `{}`", source.display()))?;
    if image.width() != image.height() {
        return Err(eyre!(
            "Pax project icon `{}` must be square, got {}x{}",
            source.display(),
            image.width(),
            image.height()
        ));
    }
    if require_opaque && image_has_transparency(&image) {
        return Err(eyre!(
            "Pax iOS app icon `{}` must be opaque; transparent pixels are not valid for App Store icons",
            source.display()
        ));
    }

    let resized = image.resize_exact(size, size, FilterType::Lanczos3);
    if require_opaque {
        resized.to_rgb8().save(dest)?;
    } else {
        resized.save(dest)?;
    }
    Ok(())
}

fn image_has_transparency(image: &image::DynamicImage) -> bool {
    image.to_rgba8().pixels().any(|pixel| pixel.0[3] < 255)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(contents: &str) -> PaxProjectMetadata {
        load_project_metadata_from_toml(Path::new("/tmp/example"), contents)
            .expect("metadata should parse")
    }

    #[test]
    fn uses_package_name_and_version_as_defaults() {
        let metadata = load(
            r#"
            [package]
            name = "example-app"
            version = "1.2.3"
            "#,
        );

        assert_eq!(metadata.web_title().as_deref(), Some("example-app"));
        assert_eq!(
            metadata.apple_xcode_build_settings(&RunTarget::iOS),
            vec![
                (
                    "INFOPLIST_KEY_CFBundleDisplayName".to_string(),
                    "example-app".to_string()
                ),
                (
                    "INFOPLIST_KEY_CFBundleName".to_string(),
                    "example-app".to_string()
                ),
                ("MARKETING_VERSION".to_string(), "1.2.3".to_string())
            ]
        );
    }

    #[test]
    fn reads_dev_hot_reload_without_conflating_it_with_target_metadata() {
        let metadata = load(
            r#"
            [package]
            name = "example-app"
            version = "1.2.3"

            [package.metadata.pax.dev]
            hot_reload = "pax"
            "#,
        );

        assert_eq!(metadata.configured_hot_reload(), Some("pax"));
    }

    #[test]
    fn dev_hot_reload_must_be_a_string() {
        let error = load_project_metadata_from_toml(
            Path::new("/tmp/example"),
            r#"
            [package]
            name = "example-app"

            [package.metadata.pax.dev]
            hot_reload = false
            "#,
        )
        .expect_err("non-string hot-reload metadata should fail");

        assert!(error
            .to_string()
            .contains("`package.metadata.pax.dev.hot_reload` must be a string"));
    }

    #[test]
    fn target_metadata_overrides_common_metadata() {
        let metadata = load(
            r#"
            [package]
            name = "example-app"
            version = "1.2.3"

            [package.metadata.pax]
            title = "Common"
            icon = "assets/common.png"
            bundle_identifier = "dev.pax.common"
            development_team = "COMMONTEAM"

            [package.metadata.pax.ios]
            title = "iOS Title"
            bundle_identifier = "dev.pax.ios"
            development_team = "IOSTEAM"
            "#,
        );

        assert_eq!(metadata.web_title().as_deref(), Some("Common"));
        assert_eq!(
            metadata.apple_development_team(&RunTarget::iOS).as_deref(),
            Some("IOSTEAM")
        );
        assert_eq!(
            metadata
                .apple_development_team(&RunTarget::macOS)
                .as_deref(),
            Some("COMMONTEAM")
        );
        assert_eq!(
            metadata.apple_bundle_identifier(&RunTarget::iOS).as_deref(),
            Some("dev.pax.ios")
        );
        assert_eq!(
            metadata
                .apple_bundle_identifier(&RunTarget::macOS)
                .as_deref(),
            Some("dev.pax.common")
        );
    }

    #[test]
    fn ipados_falls_back_to_ios_before_common() {
        let metadata = load(
            r#"
            [package]
            name = "example-app"
            version = "1.2.3"

            [package.metadata.pax]
            title = "Common"

            [package.metadata.pax.ios]
            title = "iOS"
            development_team = "TEAM"
            "#,
        );

        assert_eq!(
            metadata.apple_title(&RunTarget::iPadOS).as_deref(),
            Some("iOS")
        );
        assert_eq!(
            metadata
                .apple_development_team(&RunTarget::iPadOS)
                .as_deref(),
            Some("TEAM")
        );
    }

    #[test]
    fn apple_info_plist_metadata_maps_to_xcode_settings() {
        let metadata = load(
            r#"
            [package]
            name = "example-app"
            version = "1.2.3"

            [package.metadata.pax.info_plist]
            NSPhotoLibraryUsageDescription = "Common photo library access."

            [package.metadata.pax.ios.info_plist]
            NSCameraUsageDescription = "iOS camera access."
            NSPhotoLibraryUsageDescription = "iOS photo library access."

            [package.metadata.pax.ipados.info_plist]
            NSCameraUsageDescription = "iPadOS camera access."
            "#,
        );

        let ios_settings = metadata.apple_xcode_build_settings(&RunTarget::iOS);
        assert!(ios_settings.contains(&(
            "INFOPLIST_KEY_NSCameraUsageDescription".to_string(),
            "iOS camera access.".to_string()
        )));
        assert!(ios_settings.contains(&(
            "INFOPLIST_KEY_NSPhotoLibraryUsageDescription".to_string(),
            "iOS photo library access.".to_string()
        )));

        let ipados_settings = metadata.apple_xcode_build_settings(&RunTarget::iPadOS);
        assert!(ipados_settings.contains(&(
            "INFOPLIST_KEY_NSCameraUsageDescription".to_string(),
            "iPadOS camera access.".to_string()
        )));
        assert!(ipados_settings.contains(&(
            "INFOPLIST_KEY_NSPhotoLibraryUsageDescription".to_string(),
            "iOS photo library access.".to_string()
        )));

        let macos_settings = metadata.apple_xcode_build_settings(&RunTarget::macOS);
        assert!(macos_settings.contains(&(
            "INFOPLIST_KEY_NSPhotoLibraryUsageDescription".to_string(),
            "Common photo library access.".to_string()
        )));
        assert!(!macos_settings
            .iter()
            .any(|(key, _)| key == "INFOPLIST_KEY_NSCameraUsageDescription"));
    }

    #[test]
    fn ios_icon_writer_generates_single_opaque_1024_asset() {
        let dir = tempfile::tempdir().expect("tempdir should be created");
        let source = dir.path().join("source.png");
        let app_iconset = dir.path().join("AppIcon.appiconset");
        let image = image::RgbaImage::from_pixel(16, 16, image::Rgba([24, 48, 96, 255]));
        image.save(&source).expect("source image should be saved");

        write_ios_app_icon_set(&source, &app_iconset).expect("ios icon set should be written");

        let generated = app_iconset.join("pax-icon-ios-1024.png");
        assert!(generated.exists());
        let generated_image = image::open(&generated).expect("generated icon should be readable");
        assert_eq!(
            (generated_image.width(), generated_image.height()),
            (1024, 1024)
        );
        let contents =
            fs::read_to_string(app_iconset.join("Contents.json")).expect("contents should exist");
        assert!(contents.contains("\"platform\": \"ios\""));
        assert!(contents.contains("\"size\": \"1024x1024\""));
    }

    #[test]
    fn macos_icon_writer_generates_full_size_set() {
        let dir = tempfile::tempdir().expect("tempdir should be created");
        let source = dir.path().join("source.png");
        let app_iconset = dir.path().join("AppIcon.appiconset");
        let image = image::RgbaImage::from_pixel(16, 16, image::Rgba([24, 48, 96, 255]));
        image.save(&source).expect("source image should be saved");

        write_macos_app_icon_set(&source, &app_iconset).expect("macos icon set should be written");

        for (file_name, expected_size) in [
            ("pax-icon-macos-16.png", 16),
            ("pax-icon-macos-16@2x.png", 32),
            ("pax-icon-macos-32.png", 32),
            ("pax-icon-macos-32@2x.png", 64),
            ("pax-icon-macos-128.png", 128),
            ("pax-icon-macos-128@2x.png", 256),
            ("pax-icon-macos-256.png", 256),
            ("pax-icon-macos-256@2x.png", 512),
            ("pax-icon-macos-512.png", 512),
            ("pax-icon-macos-512@2x.png", 1024),
        ] {
            let generated = app_iconset.join(file_name);
            assert!(generated.exists(), "{file_name} should exist");
            let generated_image =
                image::open(generated).expect("generated macos icon should be readable");
            assert_eq!(
                (generated_image.width(), generated_image.height()),
                (expected_size, expected_size)
            );
        }
    }
}
