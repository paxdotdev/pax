use crate::messages::{
    ActivateAppRevision, AgentMessage, LoadManifestRequest, PrepareAppRevision, RevisionStamp,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManifestAdmission {
    Apply,
    Ignore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TemplateAdmission {
    Apply,
    Ignore,
    RequestSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppRevisionActivationStatus {
    Unknown,
    Preparing,
    Prepared,
    Committing,
    Committed,
    Rejected,
}

/// Runtime-side admission state for designtime program revisions.
///
/// Socket connectivity is intentionally not represented here. The gate owns
/// revision coherence while `WebSocketConnection` remains a transport. An old
/// active revision can therefore continue accepting template-only updates while
/// the host prepares a replacement executable revision.
#[derive(Debug, Default)]
pub(crate) struct RevisionGate {
    active: Option<RevisionStamp>,
    prepared: Option<PrepareAppRevision>,
    /// Revision identity supplied directly by a host which has already loaded
    /// a fresh artifact. Native candidates are delivered out-of-band, so the
    /// new engine cannot observe their websocket prepare envelope first.
    primed: Option<RevisionStamp>,
    activation_request: Option<ActivateAppRevision>,
    prepared_activation: Option<RevisionStamp>,
    pending_activation: Option<ActivateAppRevision>,
    committed_activation: Option<String>,
    rejected_activation: Option<String>,
    manifest_compatible: bool,
    snapshot_request_in_flight: bool,
}

impl RevisionGate {
    #[cfg(test)]
    pub(crate) fn active_stamp(&self) -> Option<&RevisionStamp> {
        self.active.as_ref()
    }

    /// Records a candidate without disturbing admission for the active logic
    /// revision. Returns true when the host should be notified.
    pub(crate) fn prepare(&mut self, prepared: PrepareAppRevision) -> bool {
        if self.prepared.as_ref() == Some(&prepared) {
            return false;
        }
        if self
            .pending_activation
            .as_ref()
            .is_some_and(|activation| activation.logic_revision_id != prepared.logic_revision_id)
        {
            // Final commit is an irrevocable boundary: retain and replay it
            // until the server returns the matching authoritative manifest.
            return false;
        }
        if self
            .activation_request
            .as_ref()
            .is_some_and(|request| request.logic_revision_id != prepared.logic_revision_id)
        {
            return false;
        }
        if self
            .prepared_activation
            .as_ref()
            .is_some_and(|revision| revision.logic_revision_id != prepared.logic_revision_id)
        {
            return false;
        }
        if self
            .primed
            .as_ref()
            .is_some_and(|primed| primed.logic_revision_id != prepared.logic_revision_id)
        {
            return false;
        }
        self.prepared = Some(prepared);
        true
    }

    /// Primes a freshly loaded host-managed artifact for strict activation.
    /// This is intentionally narrower than accepting an arbitrary activation:
    /// a different live or prepared revision cannot be replaced through this
    /// path.
    pub(crate) fn prime(&mut self, logic_revision_id: &str) -> bool {
        if logic_revision_id.is_empty() {
            return false;
        }
        let already_known = self
            .prepared
            .as_ref()
            .is_some_and(|prepared| prepared.logic_revision_id == logic_revision_id)
            || self
                .active
                .as_ref()
                .is_some_and(|active| active.logic_revision_id == logic_revision_id)
            || self
                .primed
                .as_ref()
                .is_some_and(|primed| primed.logic_revision_id == logic_revision_id);
        if already_known {
            return true;
        }
        if self.active.is_some() || self.prepared.is_some() || self.primed.is_some() {
            return false;
        }

        self.primed = Some(RevisionStamp {
            logic_revision_id: logic_revision_id.to_string(),
            template_version: 0,
        });
        true
    }

    /// Requests server-side validation while the previous host runtime remains
    /// mounted. The server returns a revision-tagged manifest only after replay
    /// and ABI validation succeed.
    pub(crate) fn request_activation(
        &mut self,
        logic_revision_id: &str,
    ) -> Option<ActivateAppRevision> {
        if logic_revision_id.is_empty() {
            return None;
        }
        let known = self
            .prepared
            .as_ref()
            .is_some_and(|prepared| prepared.logic_revision_id == logic_revision_id)
            || self
                .primed
                .as_ref()
                .is_some_and(|primed| primed.logic_revision_id == logic_revision_id);
        if !known {
            return None;
        }
        let request = ActivateAppRevision {
            logic_revision_id: logic_revision_id.to_string(),
        };
        self.activation_request = Some(request.clone());
        self.prepared_activation = None;
        self.committed_activation = None;
        self.rejected_activation = None;
        Some(request)
    }

    pub(crate) fn admit_prepared_activation_manifest(&self, revision: &RevisionStamp) -> bool {
        self.activation_request
            .as_ref()
            .is_some_and(|request| request.logic_revision_id == revision.logic_revision_id)
    }

    pub(crate) fn commit_prepared_activation_manifest(&mut self, revision: RevisionStamp) {
        self.activation_request = None;
        self.prepared_activation = Some(revision);
        self.manifest_compatible = true;
        self.snapshot_request_in_flight = false;
    }

    /// Starts the final commit only after the candidate accepted the server's
    /// validated manifest. The old host remains mounted until a matching final
    /// manifest proves that the server committed this reservation.
    pub(crate) fn activate(&mut self, logic_revision_id: &str) -> Option<ActivateAppRevision> {
        if logic_revision_id.is_empty() {
            return None;
        }

        let active_matches = self
            .active
            .as_ref()
            .is_some_and(|active| active.logic_revision_id == logic_revision_id);
        let prepared_activation_matches = self
            .prepared_activation
            .as_ref()
            .is_some_and(|prepared| prepared.logic_revision_id == logic_revision_id);
        if !prepared_activation_matches && !active_matches {
            return None;
        }

        if !active_matches {
            self.active = self.prepared_activation.clone();
        }
        self.prepared = None;
        self.primed = None;
        self.prepared_activation = None;
        // Require the server's post-commit snapshot even when it has the same
        // template version as the preflight snapshot.
        self.manifest_compatible = false;
        self.snapshot_request_in_flight = false;

        let activation = ActivateAppRevision {
            logic_revision_id: logic_revision_id.to_string(),
        };
        self.pending_activation = Some(activation.clone());
        self.committed_activation = None;
        self.rejected_activation = None;
        Some(activation)
    }

    pub(crate) fn admit_manifest(&self, revision: &RevisionStamp) -> ManifestAdmission {
        match &self.active {
            Some(active)
                if active.logic_revision_id == revision.logic_revision_id
                    && (revision.template_version > active.template_version
                        || (revision.template_version == active.template_version
                            && !self.manifest_compatible)) =>
            {
                ManifestAdmission::Apply
            }
            // The first manifest establishes the initial server revision only
            // when this is not an unactivated candidate runtime.
            None if self.prepared.is_none()
                && self.primed.is_none()
                && self.activation_request.is_none()
                && self.prepared_activation.is_none() =>
            {
                ManifestAdmission::Apply
            }
            _ => ManifestAdmission::Ignore,
        }
    }

    pub(crate) fn commit_manifest(&mut self, revision: RevisionStamp) {
        let committed_activation = self
            .pending_activation
            .as_ref()
            .filter(|activation| activation.logic_revision_id == revision.logic_revision_id)
            .map(|activation| activation.logic_revision_id.clone());
        if committed_activation.is_some() {
            self.pending_activation = None;
        }
        if let Some(committed_activation) = committed_activation {
            self.committed_activation = Some(committed_activation);
        } else if self.committed_activation.as_deref() != Some(&revision.logic_revision_id) {
            self.committed_activation = None;
        }
        self.active = Some(revision);
        self.manifest_compatible = true;
        self.snapshot_request_in_flight = false;
    }

    pub(crate) fn reject_manifest(&mut self) {
        self.manifest_compatible = false;
        self.snapshot_request_in_flight = false;
    }

    /// Clears a matching wire acknowledgement after the server reports that it
    /// is stale or unknown. The executable remains mounted; a following exact
    /// snapshot or a newer Prepare decides the next safe transition.
    pub(crate) fn reject_activation(&mut self, logic_revision_id: &str) -> bool {
        let matches_pending = self
            .pending_activation
            .as_ref()
            .is_some_and(|activation| activation.logic_revision_id == logic_revision_id);
        let matches_request = self
            .activation_request
            .as_ref()
            .is_some_and(|request| request.logic_revision_id == logic_revision_id);
        if matches_pending || matches_request {
            self.pending_activation = None;
            self.activation_request = None;
            self.prepared_activation = None;
            self.rejected_activation = Some(logic_revision_id.to_string());
            self.manifest_compatible = false;
            self.snapshot_request_in_flight = false;
            true
        } else {
            false
        }
    }

    pub(crate) fn cancel_activation(&mut self, logic_revision_id: &str) -> bool {
        let cancellable = self
            .activation_request
            .as_ref()
            .is_some_and(|request| request.logic_revision_id == logic_revision_id)
            || self
                .prepared_activation
                .as_ref()
                .is_some_and(|revision| revision.logic_revision_id == logic_revision_id);
        if cancellable {
            self.activation_request = None;
            self.prepared_activation = None;
            self.rejected_activation = None;
        }
        cancellable
    }

    pub(crate) fn activation_status(&self, logic_revision_id: &str) -> AppRevisionActivationStatus {
        if self.rejected_activation.as_deref() == Some(logic_revision_id) {
            AppRevisionActivationStatus::Rejected
        } else if self.committed_activation.as_deref() == Some(logic_revision_id) {
            AppRevisionActivationStatus::Committed
        } else if self
            .pending_activation
            .as_ref()
            .is_some_and(|activation| activation.logic_revision_id == logic_revision_id)
        {
            AppRevisionActivationStatus::Committing
        } else if self
            .prepared_activation
            .as_ref()
            .is_some_and(|revision| revision.logic_revision_id == logic_revision_id)
        {
            AppRevisionActivationStatus::Prepared
        } else if self
            .activation_request
            .as_ref()
            .is_some_and(|request| request.logic_revision_id == logic_revision_id)
        {
            AppRevisionActivationStatus::Preparing
        } else {
            AppRevisionActivationStatus::Unknown
        }
    }

    pub(crate) fn admit_template(&mut self, revision: &RevisionStamp) -> TemplateAdmission {
        let Some(active) = &self.active else {
            return self.request_snapshot_once();
        };
        if active.logic_revision_id != revision.logic_revision_id {
            return TemplateAdmission::Ignore;
        }
        if !self.manifest_compatible {
            return self.request_snapshot_once();
        }
        if revision.template_version <= active.template_version {
            return TemplateAdmission::Ignore;
        }
        if revision.template_version != active.template_version.saturating_add(1) {
            return self.request_snapshot_once();
        }
        TemplateAdmission::Apply
    }

    pub(crate) fn commit_template(&mut self, revision: RevisionStamp) {
        self.active = Some(revision);
    }

    fn request_snapshot_once(&mut self) -> TemplateAdmission {
        if self.snapshot_request_in_flight {
            TemplateAdmission::Ignore
        } else {
            self.snapshot_request_in_flight = true;
            TemplateAdmission::RequestSnapshot
        }
    }

    /// A disconnect invalidates trust in the server snapshot, but not the
    /// identity of the executable revision that remains mounted. A prepare
    /// envelope is process-local server state and must be replayed after reconnect.
    /// Host priming, by contrast, describes the artifact already mounted in this
    /// process and therefore remains valid across a transport interruption.
    pub(crate) fn disconnected(&mut self) {
        self.prepared = None;
        if self.pending_activation.is_none() {
            if let Some(prepared) = self.prepared_activation.take() {
                self.activation_request = Some(ActivateAppRevision {
                    logic_revision_id: prepared.logic_revision_id,
                });
            }
        }
        self.manifest_compatible = false;
        self.snapshot_request_in_flight = false;
    }

    pub(crate) fn connection_messages(&mut self) -> Vec<AgentMessage> {
        let mut messages = Vec::with_capacity(1);
        if let Some(activation) = self.pending_activation.clone() {
            messages.push(AgentMessage::ActivateAppRevision(activation));
        } else if let Some(request) = self.activation_request.clone() {
            messages.push(AgentMessage::RequestAppRevisionActivation(request));
        } else if self.active.is_none()
            && (self.prepared.is_some()
                || self.primed.is_some()
                || self.prepared_activation.is_some())
        {
            // A standby candidate must never advertise itself as an ordinary
            // survivor. Doing so could promote its socket and evict the still-
            // mounted last-known-good runtime during a reconnect gap.
        } else {
            messages.push(self.manifest_request());
            self.snapshot_request_in_flight = true;
        }
        messages
    }

    pub(crate) fn manifest_request(&self) -> AgentMessage {
        AgentMessage::LoadManifestRequest(LoadManifestRequest {
            active_revision: self.active.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{AppRevisionActivationStatus, ManifestAdmission, RevisionGate, TemplateAdmission};
    use crate::messages::{
        DebugArtifact, DebugLogicExecutionMode, PrepareAppRevision, RevisionStamp,
    };

    fn stamp(logic: &str, templates: u64) -> RevisionStamp {
        RevisionStamp {
            logic_revision_id: logic.to_string(),
            template_version: templates,
        }
    }

    fn prepare(logic: &str) -> PrepareAppRevision {
        PrepareAppRevision {
            logic_revision_id: logic.to_string(),
            execution_mode: DebugLogicExecutionMode::CompiledArtifact,
            artifact: DebugArtifact {
                kind: "web-cartridge".to_string(),
                location: format!("/__reloads__/{logic}/pax-cartridge"),
            },
        }
    }

    fn admit_preflight(gate: &mut RevisionGate, logic: &str, templates: u64) {
        assert!(gate.request_activation(logic).is_some());
        assert!(matches!(
            gate.connection_messages().as_slice(),
            [crate::messages::AgentMessage::RequestAppRevisionActivation(request)]
                if request.logic_revision_id == logic
        ));
        let revision = stamp(logic, templates);
        assert!(gate.admit_prepared_activation_manifest(&revision));
        gate.commit_prepared_activation_manifest(revision);
    }

    #[test]
    fn active_templates_continue_while_candidate_is_prepared() {
        let mut gate = RevisionGate::default();
        gate.commit_manifest(stamp("active", 2));
        assert!(gate.prepare(prepare("candidate")));

        assert_eq!(
            gate.admit_template(&stamp("active", 3)),
            TemplateAdmission::Apply
        );
        gate.commit_template(stamp("active", 3));
        assert_eq!(gate.active_stamp(), Some(&stamp("active", 3)));
    }

    #[test]
    fn candidate_runtime_rejects_old_manifest_until_activation() {
        let mut gate = RevisionGate::default();
        gate.prepare(prepare("candidate"));

        assert_eq!(
            gate.admit_manifest(&stamp("active", 8)),
            ManifestAdmission::Ignore
        );
        admit_preflight(&mut gate, "candidate", 0);
        assert!(gate.activate("candidate").is_some());
        assert_eq!(
            gate.admit_manifest(&stamp("candidate", 0)),
            ManifestAdmission::Apply
        );
    }

    #[test]
    fn a_template_gap_requests_one_full_snapshot() {
        let mut gate = RevisionGate::default();
        gate.commit_manifest(stamp("active", 2));

        assert_eq!(
            gate.admit_template(&stamp("active", 4)),
            TemplateAdmission::RequestSnapshot
        );
        assert_eq!(
            gate.admit_template(&stamp("active", 5)),
            TemplateAdmission::Ignore
        );
    }

    #[test]
    fn equal_snapshot_is_only_reapplied_to_restore_disconnect_trust() {
        let mut gate = RevisionGate::default();
        gate.commit_manifest(stamp("active", 2));

        assert_eq!(
            gate.admit_manifest(&stamp("active", 2)),
            ManifestAdmission::Ignore
        );
        gate.disconnected();
        assert!(matches!(
            gate.manifest_request(),
            crate::messages::AgentMessage::LoadManifestRequest(request)
                if request.active_revision == Some(stamp("active", 2))
        ));
        assert_eq!(
            gate.admit_manifest(&stamp("active", 2)),
            ManifestAdmission::Apply
        );
    }

    #[test]
    fn disconnect_preserves_active_and_queued_activation_but_clears_prepare() {
        let mut gate = RevisionGate::default();
        gate.commit_manifest(stamp("active", 7));
        gate.prepare(prepare("candidate"));
        admit_preflight(&mut gate, "candidate", 0);
        gate.activate("candidate").unwrap();

        gate.disconnected();
        let messages = gate.connection_messages();

        assert_eq!(gate.active_stamp(), Some(&stamp("candidate", 0)));
        assert!(matches!(
            messages.as_slice(),
            [crate::messages::AgentMessage::ActivateAppRevision(activation)]
                if activation.logic_revision_id == "candidate"
        ));
    }

    #[test]
    fn activation_requires_a_prepared_or_active_revision() {
        let mut gate = RevisionGate::default();
        gate.commit_manifest(stamp("active", 1));
        gate.prepare(prepare("candidate"));

        assert!(gate.activate("host-selected").is_none());
        assert_eq!(gate.active_stamp(), Some(&stamp("active", 1)));
        assert!(gate.activate("candidate").is_none());
        admit_preflight(&mut gate, "candidate", 0);
        assert!(gate.activate("candidate").is_some());
        assert!(gate.activate("").is_none());
    }

    #[test]
    fn fresh_native_artifact_can_be_primed_without_relaxing_activation() {
        let mut gate = RevisionGate::default();

        assert!(gate.activate("candidate").is_none());
        assert!(gate.prime("candidate"));
        assert_eq!(
            gate.admit_manifest(&stamp("old-active", 9)),
            ManifestAdmission::Ignore
        );
        assert!(gate.activate("different").is_none());
        admit_preflight(&mut gate, "candidate", 0);
        assert!(gate.activate("candidate").is_some());
        assert_eq!(
            gate.admit_manifest(&stamp("candidate", 0)),
            ManifestAdmission::Apply
        );
    }

    #[test]
    fn priming_cannot_replace_a_live_revision() {
        let mut gate = RevisionGate::default();
        gate.commit_manifest(stamp("active", 3));

        assert!(gate.prime("active"));
        assert!(!gate.prime("candidate"));
        assert!(gate.activate("candidate").is_none());
    }

    #[test]
    fn newer_prepare_cannot_supersede_commit_in_flight() {
        let mut gate = RevisionGate::default();
        gate.commit_manifest(stamp("active", 1));
        gate.prepare(prepare("candidate-b"));
        admit_preflight(&mut gate, "candidate-b", 0);
        gate.activate("candidate-b").unwrap();

        assert!(!gate.prepare(prepare("candidate-c")));
        let messages = gate.connection_messages();

        assert!(matches!(
            messages.as_slice(),
            [crate::messages::AgentMessage::ActivateAppRevision(activation)]
                if activation.logic_revision_id == "candidate-b"
        ));
        assert!(gate.activate("candidate-b").is_some());
    }

    #[test]
    fn explicit_rejection_clears_only_the_matching_activation() {
        let mut gate = RevisionGate::default();
        gate.commit_manifest(stamp("active", 1));
        gate.prepare(prepare("candidate-b"));
        assert!(gate.request_activation("candidate-b").is_some());

        assert!(!gate.reject_activation("candidate-a"));
        assert!(matches!(
            gate.connection_messages().as_slice(),
            [crate::messages::AgentMessage::RequestAppRevisionActivation(
                _
            )]
        ));
        assert!(gate.reject_activation("candidate-b"));
        assert_eq!(gate.active_stamp(), Some(&stamp("active", 1)));
        assert_eq!(
            gate.activation_status("candidate-b"),
            AppRevisionActivationStatus::Rejected
        );
        assert!(matches!(
            gate.connection_messages().as_slice(),
            [crate::messages::AgentMessage::LoadManifestRequest(_)]
        ));
    }

    #[test]
    fn unrelated_prepare_does_not_erase_terminal_rejection() {
        let mut gate = RevisionGate::default();
        gate.prepare(prepare("candidate-b"));
        admit_preflight(&mut gate, "candidate-b", 0);
        gate.activate("candidate-b").unwrap();
        assert!(gate.reject_activation("candidate-b"));

        assert!(gate.prepare(prepare("active-a")));

        assert_eq!(
            gate.activation_status("candidate-b"),
            AppRevisionActivationStatus::Rejected
        );
    }

    #[test]
    fn unrelated_prepare_does_not_erase_terminal_commit() {
        let mut gate = RevisionGate::default();
        gate.prepare(prepare("candidate-b"));
        admit_preflight(&mut gate, "candidate-b", 0);
        gate.activate("candidate-b").unwrap();
        gate.commit_manifest(stamp("candidate-b", 0));

        assert!(gate.prepare(prepare("candidate-c")));

        assert_eq!(
            gate.activation_status("candidate-b"),
            AppRevisionActivationStatus::Committed
        );
    }

    #[test]
    fn final_commit_cannot_be_cancelled_and_terminal_ack_survives_snapshots() {
        let mut gate = RevisionGate::default();
        gate.prepare(prepare("candidate"));
        admit_preflight(&mut gate, "candidate", 0);
        assert!(gate.activate("candidate").is_some());
        assert_eq!(
            gate.activation_status("candidate"),
            AppRevisionActivationStatus::Committing
        );
        assert!(!gate.cancel_activation("candidate"));

        gate.commit_manifest(stamp("candidate", 0));
        assert_eq!(
            gate.activation_status("candidate"),
            AppRevisionActivationStatus::Committed
        );
        assert!(!gate.cancel_activation("candidate"));

        gate.disconnected();
        gate.commit_manifest(stamp("candidate", 0));
        assert_eq!(
            gate.activation_status("candidate"),
            AppRevisionActivationStatus::Committed
        );
    }

    #[test]
    fn preflight_can_be_cancelled_without_changing_the_active_revision() {
        let mut gate = RevisionGate::default();
        gate.commit_manifest(stamp("active", 4));
        gate.prepare(prepare("candidate"));
        admit_preflight(&mut gate, "candidate", 0);

        assert!(gate.cancel_activation("candidate"));
        assert_eq!(gate.active_stamp(), Some(&stamp("active", 4)));
        assert_eq!(
            gate.activation_status("candidate"),
            AppRevisionActivationStatus::Unknown
        );
    }

    #[test]
    fn reconnect_replays_preflight_without_requesting_the_active_snapshot() {
        let mut gate = RevisionGate::default();
        gate.prepare(prepare("candidate"));
        admit_preflight(&mut gate, "candidate", 0);
        assert_eq!(
            gate.activation_status("candidate"),
            AppRevisionActivationStatus::Prepared
        );

        gate.disconnected();

        assert_eq!(
            gate.activation_status("candidate"),
            AppRevisionActivationStatus::Preparing
        );
        assert!(matches!(
            gate.connection_messages().as_slice(),
            [crate::messages::AgentMessage::RequestAppRevisionActivation(request)]
                if request.logic_revision_id == "candidate"
        ));
    }

    #[test]
    fn standby_candidate_never_requests_an_ordinary_manifest() {
        let mut primed = RevisionGate::default();
        assert!(primed.prime("candidate"));
        assert!(primed.connection_messages().is_empty());

        let mut prepared = RevisionGate::default();
        assert!(prepared.prepare(prepare("candidate")));
        assert!(prepared.connection_messages().is_empty());
        admit_preflight(&mut prepared, "candidate", 0);
        assert!(prepared.connection_messages().is_empty());
    }
}
