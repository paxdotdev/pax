use std::sync::Arc;
use tokio::sync::Mutex;
use rustc_version::version;

pub async fn check_for_update(current_version: &str, new_version_info: Arc<Mutex<Option<String>>>) {
    let url = match option_env!("PAX_UPDATE_SERVER") {
        Some(server) => {
            format!("{}/pax-cli/{}", server, current_version)
        },
        None => {
            format!("https://update.pax.dev/pax-cli/{}", current_version)
        }
    };
    let user_agent = get_user_agent();

    let client = reqwest::Client::new();
    if let Ok(response) = client.get(&url)
        .header(reqwest::header::USER_AGENT, user_agent)
        .send()
        .await {
        if response.status().is_success() {
            match response.text().await {
                Ok(body) => {
                    if !body.is_empty() {
                        // Store the new version info — this mutex is how the "nominal action"
                        // will decide, upon completion, whether an update is available.
                        let mut lock = new_version_info.lock().await;
                        *lock = Some(body);
                    }
                },
                Err(e) => {
                    panic!("{:?}",e);
                }
            }
        } else {
            panic!();
        }
    }
}

const TOOL_NAME : &str = "pax-cli";
fn get_user_agent() -> String {
    let os = if cfg!(target_os = "windows") {
        "Windows"
    } else if cfg!(target_os = "macos") {
        "macOS"
    } else if cfg!(target_os = "linux") {
        "Linux"
    } else {
        "Unknown-OS"
    };

    let locale = option_env!("LANG").unwrap_or_else(|| "unknown");

    let arch = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "x86") {
        "x86"
    } else if cfg!(target_arch = "arm") {
        "arm"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "unknown"
    };

    format!("{}/{} ({}; {})", TOOL_NAME, os, locale, arch)
}