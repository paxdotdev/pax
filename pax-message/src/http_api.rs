//! Shared request and response types for Pax's public HTTP service.
//!
//! This module deliberately contains only the closed wire contract. HTTP clients,
//! provider mappings, persistence, and policy belong at the respective edges.

use serde::{Deserialize, Serialize};

/// Path for querying the latest published `pax-cli` release.
pub const CLI_LATEST_RELEASE_PATH: &str = "/v1/cli/releases/latest";

/// Path for submitting a single CLI telemetry event.
pub const CLI_TELEMETRY_PATH: &str = "/v1/cli/telemetry";

/// Response returned by [`CLI_LATEST_RELEASE_PATH`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LatestReleaseResponse {
    pub latest_version: String,
}

/// One privacy-bounded CLI telemetry event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TelemetryRequest {
    /// Random UUID v4 representing one OS-user CLI installation.
    pub installation_id: String,
    pub cli_version: String,
    pub host_os: HostOs,
    pub host_arch: HostArch,
    pub event: TelemetryEvent,
}

/// Closed set of telemetry events accepted by the launch API.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum TelemetryEvent {
    CommandOutcome {
        command: CommandFamily,
        target: Option<Target>,
        outcome: CommandOutcome,
    },
    RunReady {
        target: Target,
    },
}

/// Public top-level CLI command families eligible for telemetry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandFamily {
    Create,
    Run,
    Build,
    Clean,
    Eject,
    Format,
    Lsp,
    Docs,
    Dev,
    SvgImport,
}

/// Coarse command result. Error details never cross the HTTP boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandOutcome {
    Succeeded,
    Failed,
}

/// Supported Pax build or run targets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    Web,
    Macos,
    Ios,
    Ipados,
}

/// Coarse host operating-system family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostOs {
    Macos,
    Linux,
    Windows,
    Other,
}

/// Coarse host CPU architecture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostArch {
    X86_64,
    Aarch64,
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telemetry_wire_format_is_stable() {
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

        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            json,
            r#"{"installation_id":"3f028ebe-ea4d-4fd3-9c1a-f722f736991b","cli_version":"0.38.3","host_os":"macos","host_arch":"aarch64","event":{"type":"command_outcome","command":"build","target":"web","outcome":"failed"}}"#
        );
        assert_eq!(
            serde_json::from_str::<TelemetryRequest>(&json).unwrap(),
            request
        );
    }

    #[test]
    fn run_ready_wire_format_is_stable() {
        let event = TelemetryEvent::RunReady {
            target: Target::Ipados,
        };

        assert_eq!(
            serde_json::to_string(&event).unwrap(),
            r#"{"type":"run_ready","target":"ipados"}"#
        );
    }

    #[test]
    fn unknown_request_fields_are_rejected() {
        let json = r#"{
            "installation_id":"3f028ebe-ea4d-4fd3-9c1a-f722f736991b",
            "cli_version":"0.38.3",
            "host_os":"linux",
            "host_arch":"x86_64",
            "project_path":"/private/example",
            "event":{"type":"run_ready","target":"web"}
        }"#;

        assert!(serde_json::from_str::<TelemetryRequest>(json).is_err());

        let event_with_unknown_field =
            r#"{"type":"run_ready","target":"web","project_name":"private"}"#;
        assert!(serde_json::from_str::<TelemetryEvent>(event_with_unknown_field).is_err());
    }

    #[test]
    fn latest_release_response_is_forward_compatible() {
        let response = LatestReleaseResponse {
            latest_version: "0.38.3".to_owned(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert_eq!(json, r#"{"latest_version":"0.38.3"}"#);
        assert_eq!(
            serde_json::from_str::<LatestReleaseResponse>(&json).unwrap(),
            response
        );
        assert_eq!(
            serde_json::from_str::<LatestReleaseResponse>(
                r#"{"latest_version":"0.38.3","release_notes_url":"https://pax.dev/releases/0.38.3"}"#
            )
            .unwrap(),
            response
        );
    }
}
