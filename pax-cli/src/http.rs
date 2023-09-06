use std::sync::{Arc, Mutex};

pub fn check_for_update(current_version: &str, new_version_info: Arc<Mutex<Option<String>>>) {
    let url = match option_env!("PAX_UPDATE_SERVER") {
        Some(server) => {
            format!("{}/pax-cli/{}", server, current_version)
        },
        None => {
            format!("https://update.pax.dev/pax-cli/{}", current_version)
        }
    };
    let user_agent = get_user_agent();

    let client = reqwest::blocking::Client::new();
    if let Ok(response) = client.get(&url)
        .header(reqwest::header::USER_AGENT, user_agent)
        .send() {
        if response.status().is_success() {
            match response.text() {
                Ok(body) => {
                    if !body.is_empty() {
                        // Store the new version info — this mutex is how the "nominal action"
                        // will decide, upon completion, whether an update is available.
                        let mut lock = new_version_info.lock().unwrap();
                        *lock = Some(body);
                    }
                },
                Err(e) => {
                    panic!("{:?}",e);
                }
            }
        } else {
            //error returned by remote, e.g. by a malformed request.  Silently proceed as if no update is available.
        }
    } else {
        //error connecting to remote.  Silently proceed as if no update is available.
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