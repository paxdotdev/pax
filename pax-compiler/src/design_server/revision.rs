use pax_designtime::messages::{
    LoadManifestResponse, PrepareAppRevision, RevisionStamp, UpdateTemplateRequest,
};
use pax_designtime::orm::runtime_abi_identity;
use pax_manifest::{
    ComponentDefinition, ComponentTemplate, PaxManifest, SettingsBlockElement, TimelineDefinition,
    TypeId,
};
use std::collections::BTreeMap;

/// Host-specific artifact delivery stays outside this coordinator.  The channel
/// only determines whether a cartridge connection should receive the generic
/// prepare envelope over its websocket.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActivationChannel {
    WebSocket,
    NativeSession,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PersistedActiveArtifact {
    pub prepare: PrepareAppRevision,
    pub channel: ActivationChannel,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DebugRestartSnapshot {
    pub stamp: RevisionStamp,
    pub manifest: PaxManifest,
    pub active_artifact: Option<PersistedActiveArtifact>,
    #[serde(default)]
    pub survivor_adoption_allowed: bool,
}

#[derive(Clone)]
struct DebugRevision {
    stamp: RevisionStamp,
    manifest: PaxManifest,
    artifact: Option<(PrepareAppRevision, ActivationChannel)>,
    activation_owner: Option<usize>,
}

#[derive(Clone)]
struct CandidateRevision {
    logic_revision_id: String,
    manifest: PaxManifest,
    prepare: PrepareAppRevision,
    channel: ActivationChannel,
    build_start_mutation_generation: u64,
    validated_generation: Option<u64>,
    activation_owner: Option<usize>,
}

#[derive(Clone)]
enum LatestPaxMutation {
    Source(String),
    SerializedComponents(BTreeMap<TypeId, VersionedSerializedComponentMutation>),
}

#[derive(Clone)]
struct VersionedSerializedComponentMutation {
    generation: u64,
    mutation: SerializedComponentMutation,
}

#[derive(Clone)]
struct SerializedComponentMutation {
    type_id: TypeId,
    template: Option<ComponentTemplate>,
    settings: Option<Vec<SettingsBlockElement>>,
    timelines: Vec<TimelineDefinition>,
}

impl From<ComponentDefinition> for SerializedComponentMutation {
    fn from(component: ComponentDefinition) -> Self {
        Self {
            type_id: component.type_id,
            template: component.template,
            settings: component.settings,
            timelines: component.timelines,
        }
    }
}

pub struct ActivationOutcome {
    pub response: LoadManifestResponse,
    pub requires_followup_rebuild: bool,
    pub promote_connection: bool,
    pub already_active: bool,
}

/// Single debug-only authority for the manifest generation a running cartridge
/// is allowed to observe.
///
/// Logic artifacts are staged as candidates.  The active manifest remains
/// readable and writable by template-only operations until the matching host
/// explicitly activates the candidate.  Release cartridges never construct
/// this type.
#[derive(Clone)]
pub struct DebugRevisionCoordinator {
    active: Option<DebugRevision>,
    candidate: Option<CandidateRevision>,
    // Keep the latest Pax mutation for each source file for the lifetime of the
    // dev session. Each build captures a generation baseline, so preflight
    // replays only entries newer than that artifact's source snapshot.
    latest_pax_mutations: BTreeMap<String, LatestPaxMutation>,
    latest_pax_mutation_generations: BTreeMap<String, u64>,
    mutation_generation: u64,
    survivor_adoption_allowed: bool,
}

impl DebugRevisionCoordinator {
    pub fn empty() -> Self {
        Self {
            active: None,
            candidate: None,
            latest_pax_mutations: BTreeMap::new(),
            latest_pax_mutation_generations: BTreeMap::new(),
            mutation_generation: 0,
            survivor_adoption_allowed: true,
        }
    }

    pub fn new(initial_logic_revision_id: String, manifest: PaxManifest) -> Self {
        Self {
            active: Some(DebugRevision {
                stamp: RevisionStamp {
                    logic_revision_id: initial_logic_revision_id,
                    template_version: 0,
                },
                manifest,
                artifact: None,
                activation_owner: None,
            }),
            candidate: None,
            latest_pax_mutations: BTreeMap::new(),
            latest_pax_mutation_generations: BTreeMap::new(),
            mutation_generation: 0,
            survivor_adoption_allowed: true,
        }
    }

    pub fn from_restart_snapshot(snapshot: DebugRestartSnapshot) -> Self {
        let artifact = snapshot
            .active_artifact
            .map(|artifact| (artifact.prepare, artifact.channel));
        Self {
            active: Some(DebugRevision {
                stamp: snapshot.stamp,
                manifest: snapshot.manifest,
                artifact,
                activation_owner: None,
            }),
            candidate: None,
            latest_pax_mutations: BTreeMap::new(),
            latest_pax_mutation_generations: BTreeMap::new(),
            mutation_generation: 0,
            survivor_adoption_allowed: snapshot.survivor_adoption_allowed,
        }
    }

    pub fn restart_snapshot(&self) -> Option<DebugRestartSnapshot> {
        self.active.as_ref().map(|active| DebugRestartSnapshot {
            stamp: active.stamp.clone(),
            manifest: active.manifest.clone(),
            active_artifact: active
                .artifact
                .clone()
                .map(|(prepare, channel)| PersistedActiveArtifact { prepare, channel }),
            survivor_adoption_allowed: self.survivor_adoption_allowed,
        })
    }

    pub fn survivor_adoption_allowed(&self) -> bool {
        self.survivor_adoption_allowed
    }

    #[cfg(test)]
    pub fn active_stamp(&self) -> Option<RevisionStamp> {
        self.active.as_ref().map(|active| active.stamp.clone())
    }

    #[cfg(test)]
    pub fn active_manifest(&self) -> Option<&PaxManifest> {
        self.active.as_ref().map(|active| &active.manifest)
    }

    pub fn is_active_logic_revision(&self, logic_revision_id: &str) -> bool {
        self.active
            .as_ref()
            .is_some_and(|active| active.stamp.logic_revision_id == logic_revision_id)
    }

    pub fn active_snapshot(&self) -> Result<Option<LoadManifestResponse>, String> {
        self.active
            .as_ref()
            .map(|active| {
                rmp_serde::to_vec(&active.manifest)
                    .map(|manifest| LoadManifestResponse {
                        revision: active.stamp.clone(),
                        manifest,
                    })
                    .map_err(|err| format!("failed to serialize active manifest: {err}"))
            })
            .transpose()
    }

    pub fn active_manifest_clone(&self) -> Option<PaxManifest> {
        self.active.as_ref().map(|active| active.manifest.clone())
    }

    pub fn active_manifest_snapshot(&self) -> Option<(RevisionStamp, PaxManifest)> {
        self.active
            .as_ref()
            .map(|active| (active.stamp.clone(), active.manifest.clone()))
    }

    pub fn candidate_manifest_clone(&self) -> Option<PaxManifest> {
        self.candidate
            .as_ref()
            .map(|candidate| candidate.manifest.clone())
    }

    #[cfg(test)]
    pub fn stage_logic_candidate(
        &mut self,
        manifest: PaxManifest,
        prepare: PrepareAppRevision,
        channel: ActivationChannel,
    ) -> Result<(), String> {
        self.stage_logic_candidate_built_after_generation(manifest, prepare, channel, 0)
    }

    pub fn stage_logic_candidate_built_after_generation(
        &mut self,
        manifest: PaxManifest,
        prepare: PrepareAppRevision,
        channel: ActivationChannel,
        build_start_mutation_generation: u64,
    ) -> Result<(), String> {
        if self.has_reserved_candidate() {
            return Err(
                "a validated logic revision is awaiting final host commit; deferring the newer build"
                    .to_string(),
            );
        }
        if prepare.logic_revision_id.is_empty() {
            return Err("logic revision id cannot be empty".to_string());
        }
        // Validate that the candidate exposes a coherent descriptor capability
        // before any host is asked to load its artifact.
        runtime_abi_identity(&manifest).map_err(|err| err.to_string())?;
        self.candidate = Some(CandidateRevision {
            logic_revision_id: prepare.logic_revision_id.clone(),
            manifest,
            prepare,
            channel,
            build_start_mutation_generation,
            validated_generation: None,
            activation_owner: None,
        });
        Ok(())
    }

    pub fn discard_logic_candidate(&mut self, logic_revision_id: &str) -> bool {
        if self
            .candidate
            .as_ref()
            .is_some_and(|candidate| candidate.logic_revision_id == logic_revision_id)
        {
            self.candidate = None;
            true
        } else {
            false
        }
    }

    pub fn discard_logic_candidate_if_owner(
        &mut self,
        logic_revision_id: &str,
        activation_owner: usize,
    ) -> bool {
        if self.candidate.as_ref().is_some_and(|candidate| {
            candidate.logic_revision_id == logic_revision_id
                && candidate.activation_owner == Some(activation_owner)
        }) {
            self.candidate = None;
            true
        } else {
            false
        }
    }

    pub fn has_candidate(&self) -> bool {
        self.candidate.is_some()
    }

    pub fn has_reserved_candidate(&self) -> bool {
        self.candidate
            .as_ref()
            .is_some_and(|candidate| candidate.validated_generation.is_some())
    }

    pub fn record_pax_source(&mut self, path: String, contents: String) -> u64 {
        self.mutation_generation = self.mutation_generation.saturating_add(1);
        self.latest_pax_mutation_generations
            .insert(path.clone(), self.mutation_generation);
        self.latest_pax_mutations
            .insert(path, LatestPaxMutation::Source(contents));
        self.mutation_generation
    }

    pub fn mutation_generation(&self) -> u64 {
        self.mutation_generation
    }

    /// Records a designer serialization which has already been admitted into
    /// the active manifest and persisted to its source file. The watcher is
    /// intentionally suppressed for server-authored writes, so this journal is
    /// the only reliable way to replay the edit onto an in-flight logic build.
    pub fn record_serialized_component(
        &mut self,
        path: String,
        component: ComponentDefinition,
    ) -> u64 {
        self.mutation_generation = self.mutation_generation.saturating_add(1);
        self.latest_pax_mutation_generations
            .insert(path.clone(), self.mutation_generation);
        let mutation = SerializedComponentMutation::from(component);
        let type_id = mutation.type_id.clone();
        let mutation = VersionedSerializedComponentMutation {
            generation: self.mutation_generation,
            mutation,
        };
        match self.latest_pax_mutations.entry(path) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(LatestPaxMutation::SerializedComponents(BTreeMap::from([(
                    type_id, mutation,
                )])));
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => match entry.get_mut() {
                LatestPaxMutation::SerializedComponents(components) => {
                    components.insert(type_id, mutation);
                }
                LatestPaxMutation::Source(_) => {
                    entry.insert(LatestPaxMutation::SerializedComponents(BTreeMap::from([(
                        type_id, mutation,
                    )])));
                }
            },
        }
        self.mutation_generation
    }

    pub fn pax_mutation_is_current(&self, path: &str, generation: u64) -> bool {
        self.latest_pax_mutation_generations.get(path).copied() == Some(generation)
    }

    /// Returns the artifact envelope a websocket cartridge needs.  Candidate
    /// artifacts take precedence.  Once activated, a web artifact is retained
    /// so a fresh page that booted the root cartridge can catch up.
    pub fn prepare_for_web_client(
        &self,
        client_logic_revision_id: Option<&str>,
    ) -> Option<PrepareAppRevision> {
        if let Some(candidate) = &self.candidate {
            if candidate.channel == ActivationChannel::WebSocket {
                return Some(candidate.prepare.clone());
            }
        }

        let active = self.active.as_ref()?;
        if client_logic_revision_id == Some(active.stamp.logic_revision_id.as_str()) {
            return None;
        }
        match &active.artifact {
            Some((prepare, ActivationChannel::WebSocket)) => Some(prepare.clone()),
            _ => None,
        }
    }

    pub fn client_matches_active(&self, logic_revision_id: Option<&str>) -> bool {
        self.active.as_ref().is_some_and(|active| {
            logic_revision_id == Some(active.stamp.logic_revision_id.as_str())
        })
    }

    /// Claims the active revision for an eligible hello/reconnect. Connection
    /// ids are monotonic within a server process, so delayed older sockets can
    /// receive snapshots but cannot become the active mutation route again.
    pub fn claim_active_connection(&mut self, connection_id: usize) -> bool {
        let Some(active) = self.active.as_mut() else {
            return false;
        };
        if active
            .activation_owner
            .is_some_and(|current_owner| connection_id < current_owner)
        {
            return false;
        }
        active.activation_owner = Some(connection_id);
        self.survivor_adoption_allowed = false;
        true
    }

    pub fn active_connection_matches(&self, logic_revision_id: &str, connection_id: usize) -> bool {
        self.active.as_ref().is_some_and(|active| {
            active.stamp.logic_revision_id == logic_revision_id
                && active.activation_owner == Some(connection_id)
        })
    }

    /// Reconcile a restarted design server with the executable revision that
    /// survived it. This is only valid before the new server has an active
    /// socket, candidate, or retained artifact transition. The runtime still
    /// applies its embedded capability guard to the resulting full snapshot.
    pub fn adopt_surviving_client_revision(&mut self, revision: &RevisionStamp) -> bool {
        if revision.logic_revision_id.is_empty() || self.candidate.is_some() {
            return false;
        }
        let Some(active) = self.active.as_mut() else {
            return false;
        };
        if active.artifact.is_some() {
            return false;
        }
        if active.stamp.logic_revision_id == revision.logic_revision_id {
            // A live server may have accepted Pax edits while this socket was
            // away. Its higher template version remains authoritative.
            return true;
        }
        if !self.survivor_adoption_allowed {
            return false;
        }

        active.stamp = revision.clone();
        active.activation_owner = None;
        self.survivor_adoption_allowed = false;
        true
    }

    /// Commit a template-only mutation to the active logic revision.
    ///
    /// The descriptor capability is compared before and after the mutation.
    /// A template that introduces a new statically generated handler or another
    /// codegen requirement is therefore escalated to a logic rebuild instead of
    /// being streamed into an incompatible cartridge.
    pub fn commit_template_update(
        &mut self,
        expected_revision: &RevisionStamp,
        source_mutation: Option<(&str, u64)>,
        mut update: UpdateTemplateRequest,
    ) -> Result<UpdateTemplateRequest, String> {
        if source_mutation
            .is_some_and(|(path, generation)| !self.pax_mutation_is_current(path, generation))
        {
            return Err("Pax source mutation was superseded by a newer update".to_string());
        }
        let active = self
            .active
            .as_mut()
            .ok_or_else(|| "design server manifest is unavailable".to_string())?;
        if &active.stamp != expected_revision {
            return Err(format!(
                "active revision changed while parsing Pax source: expected {}@{}, found {}@{}",
                expected_revision.logic_revision_id,
                expected_revision.template_version,
                active.stamp.logic_revision_id,
                active.stamp.template_version,
            ));
        }
        let previous_identity =
            runtime_abi_identity(&active.manifest).map_err(|err| err.to_string())?;
        let mut next_manifest = active.manifest.clone();
        apply_template_to_manifest(&mut next_manifest, &update)?;
        let next_identity = runtime_abi_identity(&next_manifest).map_err(|err| err.to_string())?;
        if previous_identity != next_identity {
            return Err(
                "Pax source update changes the compiled cartridge capability; waiting for a logic rebuild"
                    .to_string(),
            );
        }

        active.manifest = next_manifest;
        active.stamp.template_version = active.stamp.template_version.saturating_add(1);
        update.revision = active.stamp.clone();
        Ok(update)
    }

    pub fn replace_active_component(
        &mut self,
        component: pax_manifest::ComponentDefinition,
    ) -> Result<(), String> {
        let active = self
            .active
            .as_mut()
            .ok_or_else(|| "design server manifest is unavailable".to_string())?;
        let previous_identity =
            runtime_abi_identity(&active.manifest).map_err(|err| err.to_string())?;
        let mut next_manifest = active.manifest.clone();
        let file_path = component
            .template
            .as_ref()
            .and_then(|template| template.get_file_path())
            .ok_or_else(|| "serialized component is missing its Pax source path".to_string())?
            .to_owned();
        let type_id = component.type_id.clone();
        let target = next_manifest
            .components
            .get_mut(&type_id)
            .ok_or_else(|| format!("active manifest has no component {type_id}"))?;
        let active_file_path = target
            .template
            .as_ref()
            .and_then(|template| template.get_file_path())
            .ok_or_else(|| format!("active component {type_id} has no Pax source path"))?;
        if active_file_path != file_path {
            return Err(format!(
                "serialized component {type_id} belongs to {file_path}, not {active_file_path}"
            ));
        }
        apply_serialized_component_mutation(target, component.into());
        let next_identity = runtime_abi_identity(&next_manifest).map_err(|err| err.to_string())?;
        if previous_identity != next_identity {
            return Err("component update requires a new logic artifact".to_string());
        }
        active.manifest = next_manifest;
        active.stamp.template_version = active.stamp.template_version.saturating_add(1);
        Ok(())
    }

    /// Admits only the authoring fields carried by whole-manifest designer
    /// serialization. Compiler/reflection fields and manifest-global state stay
    /// owned by the active logic revision.
    pub fn replace_active_authoring_manifest(
        &mut self,
        manifest: &PaxManifest,
    ) -> Result<(), String> {
        let active = self
            .active
            .as_mut()
            .ok_or_else(|| "design server manifest is unavailable".to_string())?;
        let previous_identity =
            runtime_abi_identity(&active.manifest).map_err(|err| err.to_string())?;
        if active
            .manifest
            .components
            .keys()
            .ne(manifest.components.keys())
        {
            return Err("serialized manifest changes the active component set".to_string());
        }
        let mut next_manifest = active.manifest.clone();
        for component in manifest.components.values() {
            let target = next_manifest
                .components
                .get_mut(&component.type_id)
                .ok_or_else(|| format!("active manifest has no component {}", component.type_id))?;
            apply_serialized_component_mutation(target, component.clone().into());
        }
        let next_identity = runtime_abi_identity(&next_manifest).map_err(|err| err.to_string())?;
        if previous_identity != next_identity {
            return Err("manifest update requires a new logic artifact".to_string());
        }
        active.manifest = next_manifest;
        active.stamp.template_version = active.stamp.template_version.saturating_add(1);
        Ok(())
    }

    pub fn replace_initial_manifest(&mut self, logic_revision_id: String, manifest: PaxManifest) {
        self.active = Some(DebugRevision {
            stamp: RevisionStamp {
                logic_revision_id,
                template_version: 0,
            },
            manifest,
            artifact: None,
            activation_owner: None,
        });
        self.candidate = None;
        self.survivor_adoption_allowed = true;
    }

    /// Validate and reserve a candidate while the last-known-good runtime stays
    /// mounted. The returned snapshot is the exact manifest the candidate must
    /// admit before requesting the final commit.
    pub fn prepare_logic_activation<F>(
        &mut self,
        logic_revision_id: &str,
        activation_owner: usize,
        mut parse_source: F,
    ) -> Result<LoadManifestResponse, String>
    where
        F: FnMut(&PaxManifest, &str, &str) -> Result<UpdateTemplateRequest, String>,
    {
        if self.is_active_logic_revision(logic_revision_id) {
            return self
                .active_snapshot()?
                .ok_or_else(|| "active manifest is unavailable".to_string());
        }

        let mut candidate = self
            .candidate
            .as_ref()
            .cloned()
            .ok_or_else(|| format!("no pending logic revision {logic_revision_id}"))?;
        if candidate.logic_revision_id != logic_revision_id {
            let pending_id = candidate.logic_revision_id.clone();
            return Err(format!(
                "stale logic activation {logic_revision_id}; pending revision is {pending_id}"
            ));
        }

        // A reconnecting candidate replays preflight before its queued final
        // commit. Transfer ownership without replaying newer mutations into the
        // already-admitted frozen snapshot.
        if candidate.validated_generation.is_some() {
            if candidate
                .activation_owner
                .is_some_and(|current_owner| activation_owner < current_owner)
            {
                return Err(format!(
                    "stale websocket owner for logic revision {logic_revision_id}"
                ));
            }
            candidate.activation_owner = Some(activation_owner);
            let response = serialize_candidate_manifest(&candidate)?;
            self.candidate = Some(candidate);
            return Ok(response);
        }

        let compiled_identity = runtime_abi_identity(&candidate.manifest)
            .map_err(|err| format!("candidate manifest is invalid: {err}"))?;
        let mut replay_errors = Vec::new();
        for (path, mutation) in &self.latest_pax_mutations {
            let mutation_generation = self
                .latest_pax_mutation_generations
                .get(path)
                .copied()
                .unwrap_or_default();
            if mutation_generation <= candidate.build_start_mutation_generation {
                continue;
            }
            let mut next_manifest = candidate.manifest.clone();
            match mutation {
                LatestPaxMutation::Source(contents) => {
                    let update = match parse_source(&candidate.manifest, path, contents) {
                        Ok(update) => update,
                        Err(err) => {
                            replay_errors.push(format!("{path}: {err}"));
                            continue;
                        }
                    };
                    if let Err(err) = apply_template_to_manifest(&mut next_manifest, &update) {
                        replay_errors.push(format!("{path}: {err}"));
                        continue;
                    }
                }
                LatestPaxMutation::SerializedComponents(components) => {
                    let mut missing_component = None;
                    for entry in components.values().filter(|entry| {
                        entry.generation > candidate.build_start_mutation_generation
                    }) {
                        let mutation = &entry.mutation;
                        let Some(target) = next_manifest.components.get_mut(&mutation.type_id)
                        else {
                            missing_component = Some(mutation.type_id.clone());
                            break;
                        };
                        apply_serialized_component_mutation(target, mutation.clone());
                    }
                    if let Some(type_id) = missing_component {
                        replay_errors.push(format!(
                            "{path}: serialized component {type_id} is missing from the candidate"
                        ));
                        continue;
                    }
                }
            }
            match runtime_abi_identity(&next_manifest) {
                Ok(identity) if identity == compiled_identity => {
                    candidate.manifest = next_manifest;
                }
                Ok(_) => replay_errors.push(format!(
                    "{path}: update changes the compiled cartridge capability"
                )),
                Err(err) => replay_errors.push(format!("{path}: {err}")),
            }
        }

        if !replay_errors.is_empty() {
            self.candidate = None;
            return Err(format!(
                "logic revision {logic_revision_id} could not replay the latest Pax source: {}",
                replay_errors.join("; ")
            ));
        }

        let response = serialize_candidate_manifest(&candidate)?;

        candidate.validated_generation = Some(self.mutation_generation);
        candidate.activation_owner = Some(activation_owner);
        self.candidate = Some(candidate);
        Ok(response)
    }

    /// Commit only a candidate which already passed replay and candidate-side
    /// ABI admission. No fallible parsing occurs here, so a watcher mutation or
    /// rebuild request cannot invalidate the transaction after commit begins.
    pub fn activate_logic_revision(
        &mut self,
        logic_revision_id: &str,
        activation_owner: usize,
    ) -> Result<ActivationOutcome, String> {
        if self.is_active_logic_revision(logic_revision_id) {
            let promote_connection = {
                let active = self.active.as_mut().expect("active revision disappeared");
                match active.activation_owner {
                    Some(current_owner) if activation_owner < current_owner => false,
                    _ => {
                        active.activation_owner = Some(activation_owner);
                        true
                    }
                }
            };
            if promote_connection {
                self.survivor_adoption_allowed = false;
            }
            return Ok(ActivationOutcome {
                response: self
                    .active_snapshot()?
                    .ok_or_else(|| "active manifest is unavailable".to_string())?,
                requires_followup_rebuild: false,
                promote_connection,
                already_active: true,
            });
        }

        let mut candidate = self
            .candidate
            .as_ref()
            .cloned()
            .ok_or_else(|| format!("no pending logic revision {logic_revision_id}"))?;
        if candidate.logic_revision_id != logic_revision_id {
            let pending_id = candidate.logic_revision_id.clone();
            return Err(format!(
                "stale logic activation {logic_revision_id}; pending revision is {pending_id}"
            ));
        }
        if candidate
            .activation_owner
            .is_some_and(|current_owner| activation_owner < current_owner)
        {
            return Err(format!(
                "stale websocket owner for logic revision {logic_revision_id}"
            ));
        }
        // The unique logic revision id is the final-commit transaction token.
        // Reconnect ids are monotonic, so a newer final replay atomically
        // transfers ownership while a delayed older socket cannot win it back.
        candidate.activation_owner = Some(activation_owner);
        let validated_generation = candidate.validated_generation.ok_or_else(|| {
            format!("logic revision {logic_revision_id} has not passed activation preflight")
        })?;
        let stamp = RevisionStamp {
            logic_revision_id: candidate.logic_revision_id.clone(),
            template_version: 0,
        };
        let response = rmp_serde::to_vec(&candidate.manifest)
            .map(|manifest| LoadManifestResponse {
                revision: stamp.clone(),
                manifest,
            })
            .map_err(|err| format!("failed to serialize activated manifest: {err}"))?;

        let channel = candidate.channel;
        self.active = Some(DebugRevision {
            stamp,
            manifest: candidate.manifest,
            artifact: Some((candidate.prepare, channel)),
            activation_owner: Some(activation_owner),
        });
        self.survivor_adoption_allowed = false;
        self.candidate = None;
        Ok(ActivationOutcome {
            response,
            requires_followup_rebuild: validated_generation != self.mutation_generation,
            promote_connection: true,
            already_active: false,
        })
    }
}

fn serialize_candidate_manifest(
    candidate: &CandidateRevision,
) -> Result<LoadManifestResponse, String> {
    let revision = RevisionStamp {
        logic_revision_id: candidate.logic_revision_id.clone(),
        template_version: 0,
    };
    rmp_serde::to_vec(&candidate.manifest)
        .map(|manifest| LoadManifestResponse { revision, manifest })
        .map_err(|err| format!("failed to serialize prepared manifest: {err}"))
}

fn apply_serialized_component_mutation(
    target: &mut ComponentDefinition,
    mutation: SerializedComponentMutation,
) {
    target.template = mutation.template;
    target.settings = mutation.settings;
    target.timelines = mutation.timelines;
}

fn apply_template_to_manifest(
    manifest: &mut PaxManifest,
    update: &UpdateTemplateRequest,
) -> Result<(), String> {
    let component = manifest
        .components
        .get_mut(&update.type_id)
        .ok_or_else(|| format!("missing component {}", update.type_id))?;
    component.template = Some(update.new_template.clone());
    component.settings = Some(update.settings_block.clone());
    component.timelines = update.timelines.clone();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ActivationChannel, DebugRevisionCoordinator};
    use pax_designtime::messages::{
        DebugArtifact, DebugLogicExecutionMode, PrepareAppRevision, RevisionStamp,
        UpdateTemplateRequest,
    };
    use pax_manifest::{
        ComponentDefinition, ComponentTemplate, PaxManifest, SettingsBlockElement,
        TimelineDefinition, Token, TypeId,
    };
    use std::collections::{BTreeMap, HashMap};

    fn manifest(component_name: &str) -> PaxManifest {
        let type_id = TypeId::build_singleton(component_name, Some(component_name));
        let mut components = BTreeMap::new();
        components.insert(
            type_id.clone(),
            ComponentDefinition {
                type_id: type_id.clone(),
                is_main_component: true,
                is_primitive: false,
                is_struct_only_component: false,
                module_path: component_name.to_string(),
                primitive_instance_import_path: None,
                template: None,
                settings: None,
                timelines: vec![],
                route_branch: None,
            },
        );
        PaxManifest {
            components,
            main_component_type_id: type_id,
            type_table: HashMap::new(),
            assets_dirs: vec![],
            engine_import_path: String::new(),
        }
    }

    fn prepare(id: &str) -> PrepareAppRevision {
        PrepareAppRevision {
            logic_revision_id: id.to_string(),
            execution_mode: DebugLogicExecutionMode::CompiledArtifact,
            artifact: DebugArtifact {
                kind: "web-cartridge".to_string(),
                location: format!("/__reloads__/{id}/pax-cartridge"),
            },
        }
    }

    fn template_update(manifest: &PaxManifest, label: &str) -> UpdateTemplateRequest {
        let type_id = manifest.main_component_type_id.clone();
        UpdateTemplateRequest {
            revision: RevisionStamp {
                logic_revision_id: String::new(),
                template_version: 0,
            },
            type_id: type_id.clone(),
            new_template: ComponentTemplate::new(type_id, None),
            settings_block: vec![SettingsBlockElement::Comment(label.to_string())],
            timelines: vec![],
        }
    }

    #[test]
    fn staging_does_not_replace_active_manifest() {
        let mut coordinator = DebugRevisionCoordinator::new("a".to_string(), manifest("A"));
        coordinator
            .stage_logic_candidate(manifest("B"), prepare("b"), ActivationChannel::WebSocket)
            .unwrap();

        assert_eq!(coordinator.active_stamp().unwrap().logic_revision_id, "a");
        assert_eq!(
            coordinator
                .active_manifest()
                .unwrap()
                .main_component_type_id,
            TypeId::build_singleton("A", Some("A"))
        );
        assert_eq!(
            coordinator
                .prepare_for_web_client(Some("a"))
                .unwrap()
                .logic_revision_id,
            "b"
        );
    }

    #[test]
    fn template_commit_rejects_intervening_same_logic_mutation() {
        let initial = manifest("A");
        let mut coordinator = DebugRevisionCoordinator::new("a".to_string(), initial.clone());
        let (expected, _) = coordinator.active_manifest_snapshot().unwrap();

        coordinator
            .commit_template_update(&expected, None, template_update(&initial, "first"))
            .unwrap();
        let error = coordinator
            .commit_template_update(&expected, None, template_update(&initial, "stale"))
            .unwrap_err();

        assert!(error.starts_with("active revision changed while parsing Pax source"));
        assert_eq!(coordinator.active_stamp().unwrap().template_version, 1);
    }

    #[test]
    fn template_commit_replaces_named_timelines_in_the_active_manifest() {
        let initial = manifest("A");
        let component_type_id = initial.main_component_type_id.clone();
        let mut coordinator = DebugRevisionCoordinator::new("a".to_string(), initial.clone());
        let (expected, _) = coordinator.active_manifest_snapshot().unwrap();
        let mut update = template_update(&initial, "timeline edit");
        update.timelines = vec![TimelineDefinition {
            name: Some(Token::new_without_location("entrance".to_string())),
            ..Default::default()
        }];

        let committed = coordinator
            .commit_template_update(&expected, None, update)
            .unwrap();

        assert_eq!(
            committed.timelines[0]
                .name
                .as_ref()
                .map(|name| name.token_value.as_str()),
            Some("entrance")
        );
        let active_component =
            &coordinator.active_manifest().unwrap().components[&component_type_id];
        assert_eq!(
            active_component.timelines[0]
                .name
                .as_ref()
                .map(|name| name.token_value.as_str()),
            Some("entrance")
        );
    }

    #[test]
    fn template_commit_rejects_snapshot_from_previous_logic_revision() {
        let initial = manifest("A");
        let mut coordinator = DebugRevisionCoordinator::new("a".to_string(), initial.clone());
        let (expected, _) = coordinator.active_manifest_snapshot().unwrap();
        coordinator
            .stage_logic_candidate(manifest("B"), prepare("b"), ActivationChannel::WebSocket)
            .unwrap();
        coordinator
            .prepare_logic_activation("b", 1, |_, _, _| unreachable!())
            .unwrap();
        coordinator.activate_logic_revision("b", 1).unwrap();

        let error = coordinator
            .commit_template_update(&expected, None, template_update(&initial, "stale-a"))
            .unwrap_err();

        assert!(error.starts_with("active revision changed while parsing Pax source"));
        assert_eq!(coordinator.active_stamp().unwrap().logic_revision_id, "b");
        assert_eq!(coordinator.active_stamp().unwrap().template_version, 0);
    }

    #[test]
    fn template_commit_rejects_superseded_mutation_for_same_source_path() {
        let initial = manifest("A");
        let mut coordinator = DebugRevisionCoordinator::new("a".to_string(), initial.clone());
        let (expected, _) = coordinator.active_manifest_snapshot().unwrap();
        let stale_generation =
            coordinator.record_pax_source("src/main.pax".to_string(), "first".to_string());
        let current_generation =
            coordinator.record_pax_source("src/main.pax".to_string(), "second".to_string());

        let error = coordinator
            .commit_template_update(
                &expected,
                Some(("src/main.pax", stale_generation)),
                template_update(&initial, "stale"),
            )
            .unwrap_err();

        assert!(error.starts_with("Pax source mutation was superseded by a newer update"));
        assert!(coordinator.pax_mutation_is_current("src/main.pax", current_generation));
        assert_eq!(coordinator.active_stamp().unwrap().template_version, 0);
    }

    #[test]
    fn mutation_to_another_source_path_does_not_supersede_template_commit() {
        let initial = manifest("A");
        let mut coordinator = DebugRevisionCoordinator::new("a".to_string(), initial.clone());
        let (expected, _) = coordinator.active_manifest_snapshot().unwrap();
        let main_generation =
            coordinator.record_pax_source("src/main.pax".to_string(), "main".to_string());
        coordinator.record_pax_source("src/other.pax".to_string(), "other".to_string());

        coordinator
            .commit_template_update(
                &expected,
                Some(("src/main.pax", main_generation)),
                template_update(&initial, "current"),
            )
            .unwrap();

        assert_eq!(coordinator.active_stamp().unwrap().template_version, 1);
    }

    #[test]
    fn serialized_component_supersedes_older_source_token_for_same_path() {
        let initial = manifest("A");
        let component = initial.components[&initial.main_component_type_id].clone();
        let mut coordinator = DebugRevisionCoordinator::new("a".to_string(), initial);
        let source_generation =
            coordinator.record_pax_source("src/main.pax".to_string(), "source".to_string());

        let serialized_generation =
            coordinator.record_serialized_component("src/main.pax".to_string(), component);

        assert!(!coordinator.pax_mutation_is_current("src/main.pax", source_generation));
        assert!(coordinator.pax_mutation_is_current("src/main.pax", serialized_generation));
    }

    #[test]
    fn only_matching_activation_promotes_candidate() {
        let mut coordinator = DebugRevisionCoordinator::new("a".to_string(), manifest("A"));
        coordinator
            .stage_logic_candidate(manifest("B"), prepare("b"), ActivationChannel::WebSocket)
            .unwrap();

        assert!(coordinator
            .prepare_logic_activation("stale", 1, |_, _, _| unreachable!())
            .is_err());
        assert_eq!(coordinator.active_stamp().unwrap().logic_revision_id, "a");

        coordinator
            .prepare_logic_activation("b", 1, |_, _, _| unreachable!())
            .unwrap();
        let outcome = coordinator.activate_logic_revision("b", 1).unwrap();
        assert_eq!(outcome.response.revision.logic_revision_id, "b");
        assert_eq!(
            coordinator
                .active_manifest()
                .unwrap()
                .main_component_type_id,
            TypeId::build_singleton("B", Some("B"))
        );
    }

    #[test]
    fn validated_candidate_reserves_its_transaction_until_final_commit() {
        let mut coordinator = DebugRevisionCoordinator::new("a".to_string(), manifest("A"));
        coordinator
            .stage_logic_candidate(manifest("B"), prepare("b"), ActivationChannel::WebSocket)
            .unwrap();
        coordinator
            .prepare_logic_activation("b", 1, |_, _, _| unreachable!())
            .unwrap();

        assert!(coordinator.has_reserved_candidate());
        assert!(coordinator
            .stage_logic_candidate(manifest("C"), prepare("c"), ActivationChannel::WebSocket)
            .is_err());
        assert_eq!(coordinator.active_stamp().unwrap().logic_revision_id, "a");

        coordinator.activate_logic_revision("b", 1).unwrap();
        assert_eq!(coordinator.active_stamp().unwrap().logic_revision_id, "b");
    }

    #[test]
    fn restart_snapshot_restores_active_artifact_and_rejects_old_adoption() {
        let mut coordinator = DebugRevisionCoordinator::new("a".to_string(), manifest("A"));
        coordinator
            .stage_logic_candidate(manifest("B"), prepare("b"), ActivationChannel::WebSocket)
            .unwrap();
        coordinator
            .prepare_logic_activation("b", 1, |_, _, _| unreachable!())
            .unwrap();
        coordinator.activate_logic_revision("b", 1).unwrap();

        let snapshot = coordinator.restart_snapshot().unwrap();
        let mut restored = DebugRevisionCoordinator::from_restart_snapshot(snapshot);

        assert_eq!(
            restored
                .prepare_for_web_client(Some("a"))
                .unwrap()
                .logic_revision_id,
            "b"
        );
        assert!(!restored.adopt_surviving_client_revision(&RevisionStamp {
            logic_revision_id: "a".to_string(),
            template_version: 9,
        }));
        assert_eq!(restored.active_stamp().unwrap().logic_revision_id, "b");
        let duplicate = restored.activate_logic_revision("b", 2).unwrap();
        assert!(duplicate.already_active);
        assert!(duplicate.promote_connection);
    }

    #[test]
    fn reconnect_preflight_transfers_reservation_before_stale_cleanup() {
        let mut coordinator = DebugRevisionCoordinator::new("a".to_string(), manifest("A"));
        coordinator
            .stage_logic_candidate(manifest("B"), prepare("b"), ActivationChannel::WebSocket)
            .unwrap();
        coordinator
            .prepare_logic_activation("b", 10, |_, _, _| unreachable!())
            .unwrap();
        coordinator
            .prepare_logic_activation("b", 11, |_, _, _| unreachable!())
            .unwrap();

        assert!(!coordinator.discard_logic_candidate_if_owner("b", 10));
        coordinator.activate_logic_revision("b", 11).unwrap();
        assert_eq!(coordinator.active_stamp().unwrap().logic_revision_id, "b");
    }

    #[test]
    fn reconnect_final_transfers_owner_and_late_old_duplicate_cannot_repromote() {
        let mut coordinator = DebugRevisionCoordinator::new("a".to_string(), manifest("A"));
        coordinator
            .stage_logic_candidate(manifest("B"), prepare("b"), ActivationChannel::WebSocket)
            .unwrap();
        coordinator
            .prepare_logic_activation("b", 10, |_, _, _| unreachable!())
            .unwrap();

        let reconnect = coordinator.activate_logic_revision("b", 11).unwrap();
        assert!(reconnect.promote_connection);
        assert!(!coordinator.discard_logic_candidate_if_owner("b", 10));

        let delayed_old = coordinator.activate_logic_revision("b", 10).unwrap();
        assert!(!delayed_old.promote_connection);
        let reconnect_retry = coordinator.activate_logic_revision("b", 11).unwrap();
        assert!(reconnect_retry.promote_connection);

        assert!(coordinator.claim_active_connection(12));
        let delayed_pre_hello_owner = coordinator.activate_logic_revision("b", 11).unwrap();
        assert!(!delayed_pre_hello_owner.promote_connection);
        let current_hello_owner = coordinator.activate_logic_revision("b", 12).unwrap();
        assert!(current_hello_owner.promote_connection);
    }

    #[test]
    fn newly_committed_logic_forces_promotion_over_newer_old_logic_socket() {
        let mut coordinator = DebugRevisionCoordinator::new("a".to_string(), manifest("A"));
        assert!(coordinator.claim_active_connection(11));
        coordinator
            .stage_logic_candidate(manifest("B"), prepare("b"), ActivationChannel::WebSocket)
            .unwrap();
        coordinator
            .prepare_logic_activation("b", 10, |_, _, _| unreachable!())
            .unwrap();

        let outcome = coordinator.activate_logic_revision("b", 10).unwrap();
        assert!(outcome.promote_connection);
        assert_eq!(coordinator.active_stamp().unwrap().logic_revision_id, "b");
    }

    #[test]
    fn transferred_preflight_rejects_delayed_older_final_without_discarding_candidate() {
        let mut coordinator = DebugRevisionCoordinator::new("a".to_string(), manifest("A"));
        coordinator
            .stage_logic_candidate(manifest("B"), prepare("b"), ActivationChannel::WebSocket)
            .unwrap();
        coordinator
            .prepare_logic_activation("b", 10, |_, _, _| unreachable!())
            .unwrap();
        coordinator
            .prepare_logic_activation("b", 11, |_, _, _| unreachable!())
            .unwrap();

        assert!(coordinator
            .prepare_logic_activation("b", 10, |_, _, _| unreachable!())
            .is_err());
        assert!(coordinator.activate_logic_revision("b", 10).is_err());
        assert!(coordinator.has_reserved_candidate());
        coordinator.activate_logic_revision("b", 11).unwrap();
    }

    #[test]
    fn mutation_after_preflight_commits_frozen_snapshot_and_survives_for_followup() {
        let mut coordinator = DebugRevisionCoordinator::new("a".to_string(), manifest("A"));
        coordinator
            .stage_logic_candidate(manifest("B"), prepare("b"), ActivationChannel::WebSocket)
            .unwrap();
        coordinator
            .prepare_logic_activation("b", 1, |_, _, _| unreachable!())
            .unwrap();

        let mut edited = manifest("B").components.into_values().next().unwrap();
        edited.settings = Some(vec![SettingsBlockElement::Comment(
            "edit after preflight".to_string(),
        )]);
        coordinator.record_serialized_component("src/main.pax".to_string(), edited);

        let outcome = coordinator.activate_logic_revision("b", 1).unwrap();
        assert!(outcome.requires_followup_rebuild);
        assert!(coordinator
            .active_manifest()
            .unwrap()
            .components
            .values()
            .next()
            .unwrap()
            .settings
            .is_none());

        coordinator
            .stage_logic_candidate(manifest("B"), prepare("c"), ActivationChannel::WebSocket)
            .unwrap();
        coordinator
            .prepare_logic_activation("c", 1, |_, _, _| unreachable!())
            .unwrap();
        coordinator.activate_logic_revision("c", 1).unwrap();
        assert!(matches!(
            coordinator
                .active_manifest()
                .unwrap()
                .components
                .values()
                .next()
                .unwrap()
                .settings
                .as_deref(),
            Some([SettingsBlockElement::Comment(comment)]) if comment == "edit after preflight"
        ));
    }

    #[test]
    fn activated_web_artifact_is_replayed_to_fresh_clients() {
        let mut coordinator = DebugRevisionCoordinator::new("a".to_string(), manifest("A"));
        coordinator
            .stage_logic_candidate(manifest("B"), prepare("b"), ActivationChannel::WebSocket)
            .unwrap();
        coordinator
            .prepare_logic_activation("b", 1, |_, _, _| unreachable!())
            .unwrap();
        coordinator.activate_logic_revision("b", 1).unwrap();

        assert!(coordinator.prepare_for_web_client(Some("b")).is_none());
        assert_eq!(
            coordinator
                .prepare_for_web_client(None)
                .unwrap()
                .logic_revision_id,
            "b"
        );
    }

    #[test]
    fn restarted_server_adopts_surviving_client_identity_without_an_artifact_transition() {
        let mut coordinator = DebugRevisionCoordinator::new("new-epoch".to_string(), manifest("A"));

        assert!(coordinator.adopt_surviving_client_revision(
            &pax_designtime::messages::RevisionStamp {
                logic_revision_id: "surviving-artifact".to_string(),
                template_version: 7,
            }
        ));
        assert_eq!(
            coordinator.active_stamp().unwrap(),
            pax_designtime::messages::RevisionStamp {
                logic_revision_id: "surviving-artifact".to_string(),
                template_version: 7,
            }
        );
    }

    #[test]
    fn reconnect_does_not_downgrade_server_template_progress() {
        let mut coordinator = DebugRevisionCoordinator::new("active".to_string(), manifest("A"));
        coordinator.active.as_mut().unwrap().stamp.template_version = 9;

        assert!(coordinator.adopt_surviving_client_revision(
            &pax_designtime::messages::RevisionStamp {
                logic_revision_id: "active".to_string(),
                template_version: 5,
            }
        ));
        assert_eq!(coordinator.active_stamp().unwrap().template_version, 9);
    }

    #[test]
    fn server_with_a_candidate_does_not_adopt_an_unrelated_client_identity() {
        let mut coordinator = DebugRevisionCoordinator::new("a".to_string(), manifest("A"));
        coordinator
            .stage_logic_candidate(manifest("B"), prepare("b"), ActivationChannel::WebSocket)
            .unwrap();

        assert!(!coordinator.adopt_surviving_client_revision(
            &pax_designtime::messages::RevisionStamp {
                logic_revision_id: "stale".to_string(),
                template_version: 4,
            }
        ));
        assert_eq!(coordinator.active_stamp().unwrap().logic_revision_id, "a");
    }

    #[test]
    fn replay_failure_does_not_promote_a_stale_candidate() {
        let mut coordinator = DebugRevisionCoordinator::new("a".to_string(), manifest("A"));
        coordinator.record_pax_source("src/main.pax".to_string(), "<Group />".to_string());
        coordinator
            .stage_logic_candidate(manifest("B"), prepare("b"), ActivationChannel::WebSocket)
            .unwrap();

        let error = match coordinator.prepare_logic_activation("b", 1, |_, path, _| {
            Err(format!("{path} no longer maps to a component"))
        }) {
            Ok(_) => panic!("stale candidate should not activate"),
            Err(error) => error,
        };

        assert!(error.contains("could not replay the latest Pax source"));
        assert_eq!(coordinator.active_stamp().unwrap().logic_revision_id, "a");
        assert!(!coordinator.has_candidate());
    }

    #[test]
    fn removed_source_mutation_that_predates_build_does_not_poison_candidate() {
        let mut coordinator = DebugRevisionCoordinator::new("a".to_string(), manifest("A"));
        coordinator.record_pax_source("src/removed.pax".to_string(), "<Group />".to_string());
        let build_start_mutation_generation = coordinator.mutation_generation();
        coordinator
            .stage_logic_candidate_built_after_generation(
                manifest("B"),
                prepare("b"),
                ActivationChannel::WebSocket,
                build_start_mutation_generation,
            )
            .unwrap();

        coordinator
            .prepare_logic_activation("b", 1, |_, path, _| {
                panic!("pre-build mutation for removed source {path} must not replay")
            })
            .unwrap();
        coordinator.activate_logic_revision("b", 1).unwrap();

        assert_eq!(coordinator.active_stamp().unwrap().logic_revision_id, "b");
    }

    #[test]
    fn removed_source_mutation_after_build_start_still_blocks_candidate() {
        let mut coordinator = DebugRevisionCoordinator::new("a".to_string(), manifest("A"));
        let build_start_mutation_generation = coordinator.mutation_generation();
        coordinator.record_pax_source("src/removed.pax".to_string(), "<Group />".to_string());
        coordinator
            .stage_logic_candidate_built_after_generation(
                manifest("B"),
                prepare("b"),
                ActivationChannel::WebSocket,
                build_start_mutation_generation,
            )
            .unwrap();

        let error = coordinator
            .prepare_logic_activation("b", 1, |_, path, _| {
                Err(format!("{path} no longer maps to a component"))
            })
            .unwrap_err();

        assert!(error.contains("could not replay the latest Pax source"));
        assert_eq!(coordinator.active_stamp().unwrap().logic_revision_id, "a");
        assert!(!coordinator.has_candidate());
    }

    #[test]
    fn shared_source_replay_filters_prebuild_component_mutations_individually() {
        let initial = manifest("A");
        let old_component = initial.components[&initial.main_component_type_id].clone();
        let mut coordinator = DebugRevisionCoordinator::new("a".to_string(), initial);
        coordinator.record_serialized_component("src/shared.pax".to_string(), old_component);
        let build_start_mutation_generation = coordinator.mutation_generation();

        let candidate = manifest("B");
        let candidate_type = candidate.main_component_type_id.clone();
        let mut edited_candidate = candidate.components[&candidate_type].clone();
        edited_candidate.settings = Some(vec![SettingsBlockElement::Comment(
            "edit during build".to_string(),
        )]);
        coordinator.record_serialized_component("src/shared.pax".to_string(), edited_candidate);
        coordinator
            .stage_logic_candidate_built_after_generation(
                candidate,
                prepare("b"),
                ActivationChannel::WebSocket,
                build_start_mutation_generation,
            )
            .unwrap();

        coordinator
            .prepare_logic_activation("b", 1, |_, _, _| unreachable!())
            .unwrap();
        coordinator.activate_logic_revision("b", 1).unwrap();

        assert!(matches!(
            coordinator.active_manifest().unwrap().components[&candidate_type]
                .settings
                .as_deref(),
            Some([SettingsBlockElement::Comment(comment)]) if comment == "edit during build"
        ));
    }

    #[test]
    fn designer_serialization_is_replayed_onto_an_in_flight_candidate() {
        let initial = manifest("A");
        let component_id = initial.main_component_type_id.clone();
        let mut serialized_component = initial.components[&component_id].clone();
        serialized_component.module_path = "stale-designer-module".to_string();
        serialized_component.template = Some(ComponentTemplate::new(
            component_id.clone(),
            Some("src/main.pax".to_string()),
        ));

        let mut coordinator = DebugRevisionCoordinator::new("a".to_string(), initial);
        coordinator
            .record_serialized_component("src/main.pax".to_string(), serialized_component.clone());
        coordinator
            .stage_logic_candidate(manifest("A"), prepare("b"), ActivationChannel::WebSocket)
            .unwrap();

        coordinator
            .prepare_logic_activation("b", 1, |_, _, _| unreachable!())
            .unwrap();
        coordinator.activate_logic_revision("b", 1).unwrap();

        assert_eq!(
            coordinator.active_manifest().unwrap().components[&component_id].module_path,
            "A"
        );
        assert!(
            coordinator.active_manifest().unwrap().components[&component_id]
                .template
                .is_some()
        );
    }

    #[test]
    fn designer_serialization_keeps_all_components_that_share_one_source_file() {
        let mut initial = manifest("A");
        let first_id = initial.main_component_type_id.clone();
        let second_id = TypeId::build_singleton("Second", Some("Second"));
        initial.components.insert(
            second_id.clone(),
            ComponentDefinition {
                type_id: second_id.clone(),
                is_main_component: false,
                is_primitive: false,
                is_struct_only_component: false,
                module_path: "second".to_string(),
                primitive_instance_import_path: None,
                template: None,
                settings: None,
                timelines: vec![],
                route_branch: None,
            },
        );
        let mut first = initial.components[&first_id].clone();
        first.template = Some(ComponentTemplate::new(
            first_id.clone(),
            Some("src/shared.rs".to_string()),
        ));
        let mut second = initial.components[&second_id].clone();
        second.template = Some(ComponentTemplate::new(
            second_id.clone(),
            Some("src/shared.rs".to_string()),
        ));

        let mut coordinator = DebugRevisionCoordinator::new("a".to_string(), initial.clone());
        coordinator.record_serialized_component("src/shared.rs".to_string(), first);
        coordinator.record_serialized_component("src/shared.rs".to_string(), second);
        coordinator
            .stage_logic_candidate(initial, prepare("b"), ActivationChannel::WebSocket)
            .unwrap();

        coordinator
            .prepare_logic_activation("b", 1, |_, _, _| unreachable!())
            .unwrap();
        coordinator.activate_logic_revision("b", 1).unwrap();
        let active = coordinator.active_manifest().unwrap();
        assert!(active.components[&first_id].template.is_some());
        assert!(active.components[&second_id].template.is_some());
    }

    #[test]
    fn whole_manifest_serialization_cannot_replace_logic_or_global_metadata() {
        let initial = manifest("A");
        let component_id = initial.main_component_type_id.clone();
        let mut serialized = initial.clone();
        serialized.main_component_type_id = TypeId::build_singleton("Wrong", Some("Wrong"));
        serialized
            .components
            .get_mut(&component_id)
            .unwrap()
            .module_path = "stale-designer-module".to_string();
        serialized
            .components
            .get_mut(&component_id)
            .unwrap()
            .template = Some(ComponentTemplate::new(
            component_id.clone(),
            Some("src/main.pax".to_string()),
        ));

        let mut coordinator = DebugRevisionCoordinator::new("a".to_string(), initial);
        coordinator
            .replace_active_authoring_manifest(&serialized)
            .unwrap();

        let active = coordinator.active_manifest().unwrap();
        assert_eq!(active.main_component_type_id, component_id);
        assert_eq!(active.components[&component_id].module_path, "A");
        assert!(active.components[&component_id].template.is_some());
    }

    #[test]
    fn component_serialization_selects_type_when_components_share_a_source_file() {
        let mut initial = manifest("A");
        let first_id = initial.main_component_type_id.clone();
        initial.components.get_mut(&first_id).unwrap().template = Some(ComponentTemplate::new(
            first_id.clone(),
            Some("src/shared.rs".to_string()),
        ));
        let second_id = TypeId::build_singleton("Second", Some("Second"));
        initial.components.insert(
            second_id.clone(),
            ComponentDefinition {
                type_id: second_id.clone(),
                is_main_component: false,
                is_primitive: false,
                is_struct_only_component: false,
                module_path: "second".to_string(),
                primitive_instance_import_path: None,
                template: Some(ComponentTemplate::new(
                    second_id.clone(),
                    Some("src/shared.rs".to_string()),
                )),
                settings: None,
                timelines: vec![],
                route_branch: None,
            },
        );
        let mut edited_second = initial.components[&second_id].clone();
        edited_second.settings = Some(vec![SettingsBlockElement::Comment(
            "second component edit".to_string(),
        )]);

        let mut coordinator = DebugRevisionCoordinator::new("a".to_string(), initial);
        coordinator.replace_active_component(edited_second).unwrap();

        let active = coordinator.active_manifest().unwrap();
        assert!(active.components[&first_id].settings.is_none());
        assert!(matches!(
            active.components[&second_id].settings.as_deref(),
            Some([SettingsBlockElement::Comment(comment)]) if comment == "second component edit"
        ));
    }
}
