use pax_message::http_api::{LatestReleaseResponse, CLI_LATEST_RELEASE_PATH};
use semver::Version;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const DEFAULT_API_BASE_URL: &str = "https://pub.pax.dev";
const UPDATE_CONNECT_TIMEOUT: Duration = Duration::from_millis(200);
const UPDATE_REQUEST_TIMEOUT: Duration = Duration::from_millis(750);

pub fn check_for_update(new_version_info: Arc<Mutex<Option<String>>>) {
    let _ = std::panic::catch_unwind(|| {
        let current_version = env!("CARGO_PKG_VERSION");
        let url = api_url(&api_base_url(), CLI_LATEST_RELEASE_PATH);
        let Ok(client) = reqwest::blocking::Client::builder()
            .no_proxy()
            .connect_timeout(UPDATE_CONNECT_TIMEOUT)
            .timeout(UPDATE_REQUEST_TIMEOUT)
            .user_agent(user_agent())
            .build()
        else {
            return;
        };
        let Ok(response) = client.get(url).send() else {
            return;
        };
        if !response.status().is_success() {
            return;
        }
        let Ok(latest) = response.json::<LatestReleaseResponse>() else {
            return;
        };
        let (Ok(current), Ok(candidate)) = (
            Version::parse(current_version),
            Version::parse(&latest.latest_version),
        ) else {
            return;
        };
        if candidate <= current {
            return;
        }
        if let Ok(mut lock) = new_version_info.lock() {
            *lock = Some(latest.latest_version);
        }
    });
}

pub(crate) fn api_base_url() -> String {
    std::env::var("PAX_API_BASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_API_BASE_URL.to_owned())
}

pub(crate) fn api_url(base: &str, path: &str) -> String {
    format!("{}{}", base.trim_end_matches('/'), path)
}

pub(crate) fn user_agent() -> String {
    format!("pax-cli/{}", env!("CARGO_PKG_VERSION"))
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
        let user_agent = user_agent();
        assert_eq!(user_agent, format!("pax-cli/{}", env!("CARGO_PKG_VERSION")));
        assert!(!user_agent.contains(std::env::consts::OS));
        assert!(!user_agent.contains(std::env::consts::ARCH));
    }
}
