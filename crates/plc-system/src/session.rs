use std::collections::{BTreeMap, BTreeSet};

use plc_commissioning::{
    ActualHardwareFaultCommand, CommissionedHardwareBoundaryReceipt, CommissionedScanReceipt,
    CommissioningError, ConfiguredController, ControllerInstanceId, CreateInstanceCommand,
    ForceId as CommissioningForceId, ForceRegistryProjection, LoadExecution, LoadPreview,
    LoadRequest, LoadResult, MatchComparison, OfflineControllerId, OfflineEngineeringState,
    OfflineSourceBuild, PostLoadMode, PreviewApproval, SessionCommandBinding, SessionState,
    VirtualLoadPackage, VirtualOnlineSessionId, VirtualUniverse,
};
use plc_core::{Lifecycle, ObjectId, Project, Sha256Digest, Uuid, sha256};
use plc_hardware::{
    ChannelConditionProjection, ChannelDirection as HardwareChannelDirection,
    ChannelId as HardwareChannelId, ChannelQuality as HardwareChannelQuality, ChannelRawValue,
    ConditionLifecycle, HardwareConditionEngine, HardwareConditionKey, HardwareConditionSnapshot,
    HardwareFaultAction, HardwareFaultCommand, HardwareFaultReceipt, InstalledOccupant,
    ModuleRuntimeState, NaturalChannelSample, RuntimeDeviceRole, RuntimeHardwareConfiguration,
    RuntimeModuleConfiguration,
};
use plc_observability::{
    ActiveCondition, ArtifactSide, DiagnosticEvent as ObservationDiagnosticEvent, DiagnosticLedger,
    DiagnosticLedgerSnapshot, DiagnosticLimits, DiagnosticRegistry, DisplayBase, ForceAuditRecord,
    ForceCommand, ForceCommandKind, ForceEntry, ForceExecutionReceipt, ForceId, ForceProvenance,
    ForceRegistry, ForceRegistrySnapshot, GlobalForceProjection, HardwareDiagnosticBridge,
    LoadedArtifactBinding, ModifyCommand, ModifyExecutionReceipt, ModifyItem, ModifyScheduler,
    MonitorSample, MonitorState, MonitoringEngine, MonitoringLimits, MonitoringPersistence,
    NavigationAnchor, NavigationDomainProjection, NavigationIndex, NavigationIndexBuilder,
    NavigationKind, NavigationRelationshipKind, NavigationResult, NavigationValidity,
    ObservationContext, ProbeLayer, PublicationBoundary, PublishedTargetValue, Quality,
    RuntimeDiagnosticBridge, RuntimeIoState, RuntimeTarget, SemanticIdentity, StableTargetId,
    TargetReference, TraceCadence, TraceCapture, TraceCaptureId, TraceChannel, TraceChannelId,
    TraceConfig, TraceConfigId, TraceDiagnosticEvent, TraceEngine, TraceEngineSnapshot,
    TraceEventKey, TraceLimits, TraceProbeKind, TraceRuntimePublication, TraceState, TraceTrigger,
    TraceTriggerId, WatchRow, WatchRowId, WatchTable, WatchTableId,
    execute_force_command_with_io_state, publish_modify_plan,
};
use plc_runtime::{
    CanonicalValue, CommandId, ControllerSnapshot, CpuState, Hash32, InputCommand, InputReceipt,
    RestartKind, RunOutcome, RuntimeBoundaryCommand, RuntimeHardwareBoundaryCommand,
    RuntimeOutputDeliveryOverride, RuntimeScanCommand, RuntimeValueTarget, ValueType,
    VirtualControllerId, canonical_force_overlay_hash,
};

use crate::software_projection::object_u128;
use crate::{
    CanonicalDisplayBase, CanonicalHardwareProjection, CanonicalProbeLayer, ProjectDiagnostic,
    SystemBuildError, SystemBuildProduct, build_project_controller, project_hardware,
    project_software,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SystemCommandIdentity {
    pub command_id: u128,
    pub idempotency_key: u128,
    pub author_identity: u128,
}

#[derive(Clone, Debug)]
pub enum SystemError {
    Build(SystemBuildError),
    Projection(Vec<ProjectDiagnostic>),
    NoCurrentBuild,
    CurrentBuildStale,
    NoLoadedArtifact,
    UnknownTarget(StableTargetId),
    TargetIsNotInput(StableTargetId),
    TargetValueUnavailable(StableTargetId),
    TraceConfigurationUnavailable(TraceConfigId),
    NavigationUnavailable,
    Commissioning(String),
    Monitoring(String),
    Modify(String),
    Force(String),
    Trace(String),
    Diagnostics(String),
    Hardware(String),
    Navigation(String),
    Snapshot(String),
}

impl From<SystemBuildError> for SystemError {
    fn from(value: SystemBuildError) -> Self {
        Self::Build(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct ProjectRefresh {
    pub semantic_changed: bool,
    pub document_changed: bool,
    pub build_invalidated: bool,
    pub loaded_runtime_preserved: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct EngineeringStatus {
    pub document_hash: Sha256Digest,
    pub semantic_fingerprint: Sha256Digest,
    pub document_dirty: bool,
    pub semantic_dirty: bool,
    pub build_current: bool,
    pub loaded: bool,
    pub online: bool,
    pub session_state: Option<SessionState>,
    pub cpu_state: CpuState,
    pub software_to_loaded: Option<MatchComparison>,
    pub hardware_to_loaded: Option<MatchComparison>,
    pub force_count: usize,
    pub monitor_state: MonitorState,
    pub projection_blocked: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineeringSnapshotHashes {
    pub document_hash: Sha256Digest,
    pub semantic_fingerprint: Sha256Digest,
    pub universe_state_hash: Hash32,
    pub controller_state_hash: Hash32,
    pub runtime_snapshot_hash: Hash32,
    pub runtime_replay_hash: Hash32,
    pub monitoring_hash: Hash32,
    pub force_snapshot_hash: Hash32,
    pub trace_snapshot_hash: Hash32,
    pub diagnostic_snapshot_hash: Hash32,
    pub diagnostic_replay_hash: Hash32,
    pub diagnostic_bridge_replay_hash: Hash32,
    pub hardware_condition_hash: Hash32,
    pub hardware_command_hash: Hash32,
    pub hardware_input_hash: Hash32,
    pub hardware_diagnostic_bridge_hash: Hash32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbeReadModel {
    pub identity: StableTargetId,
    pub display_name: String,
    pub value_type: ValueType,
    pub runtime_target: RuntimeTarget,
    pub natural_value: Option<CanonicalValue>,
    pub effective_value: Option<CanonicalValue>,
    pub raw_input_value: Option<CanonicalValue>,
    pub committed_output_value: Option<CanonicalValue>,
    pub delivered_output_value: Option<CanonicalValue>,
    pub quality: Quality,
    pub forced_value: Option<CanonicalValue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WatchTableReadModel {
    pub table: WatchTable,
    pub latest_samples: Vec<(WatchRowId, Option<MonitorSample>)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceReadModel {
    pub config: TraceConfig,
    pub state: TraceState,
    pub captures: Vec<TraceCapture>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticReadModel {
    pub active: Vec<ActiveCondition>,
    pub retained: Vec<ObservationDiagnosticEvent>,
    pub navigation: Vec<(u128, Option<NavigationResult>)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineeringReadModel {
    pub controller_object_id: ObjectId,
    pub universe_id: plc_runtime::UniverseId,
    pub universe_epoch: u64,
    pub runtime_controller_id: VirtualControllerId,
    pub controller_epoch: u64,
    pub cpu_state: CpuState,
    pub scan_sequence: u64,
    pub virtual_time_ms: u64,
    pub status: EngineeringStatus,
    pub build_semantic_fingerprint: Option<Sha256Digest>,
    pub build_snapshot_hash: Option<Hash32>,
    pub loaded_artifact_fingerprint: Option<Hash32>,
    pub profile_fingerprint: Option<Hash32>,
    pub probes: Vec<ProbeReadModel>,
    pub watches: Vec<WatchTableReadModel>,
    pub forces: GlobalForceProjection,
    pub force_audit: Vec<ForceAuditRecord>,
    pub traces: Vec<TraceReadModel>,
    pub diagnostics: DiagnosticReadModel,
    pub snapshot_hashes: Option<EngineeringSnapshotHashes>,
    pub runtime_replay_hash: Hash32,
    pub diagnostic_replay_hash: Hash32,
    pub diagnostic_bridge_replay_hash: Hash32,
    pub projection_diagnostics: Vec<ProjectDiagnostic>,
}

/// Integrity-bound aggregate snapshot. Opaque state is retained only so the
/// owning runtime and observability restore APIs can validate and rebind it;
/// callers can inspect or deliberately corrupt the declared aggregate hash.
#[derive(Clone, Debug)]
pub struct EngineeringSessionSnapshot {
    pub schema_version: u32,
    pub document_hash: Sha256Digest,
    pub semantic_fingerprint: Sha256Digest,
    pub controller_object_id: ObjectId,
    pub build_snapshot_hash: Hash32,
    pub loaded_artifact_fingerprint: Hash32,
    pub runtime_snapshot_hash: Hash32,
    pub monitoring_hash: Hash32,
    pub monitor_state: MonitorState,
    pub force_snapshot_hash: Hash32,
    pub trace_snapshot_hash: Hash32,
    pub diagnostic_snapshot_hash: Hash32,
    pub runtime_bridge_hash: Hash32,
    pub hardware_condition_hash: Hash32,
    pub hardware_command_hash: Hash32,
    pub hardware_input_hash: Hash32,
    pub hardware_diagnostic_bridge_hash: Hash32,
    pub content_hash: Hash32,
    runtime: ControllerSnapshot,
    monitoring_persistence: MonitoringPersistence,
    forces: ForceRegistrySnapshot,
    traces: TraceEngineSnapshot,
    diagnostics: DiagnosticLedgerSnapshot,
    runtime_diagnostics: RuntimeDiagnosticBridge,
    hardware_conditions: HardwareConditionSnapshot,
    hardware_commands: Vec<HardwareFaultCommand>,
    hardware_natural_inputs: BTreeMap<HardwareChannelId, ChannelRawValue>,
    hardware_diagnostics: HardwareDiagnosticBridge,
    trace_capture_ids: BTreeSet<TraceCaptureId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum ForceRestoreClassification {
    Retained = 1,
    Added = 2,
    Removed = 3,
    Replaced = 4,
    Reordered = 5,
    EpochRebound = 6,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForceRestoreDelta {
    pub force_id: ForceId,
    pub classifications: Vec<ForceRestoreClassification>,
    pub before_ordinal: Option<u64>,
    pub after_ordinal: Option<u64>,
    pub before: Option<ForceEntry>,
    pub after: Option<ForceEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestorePreview {
    pub snapshot_content_hash: Hash32,
    pub expected_universe_epoch: u64,
    pub expected_controller_epoch: u64,
    pub expected_current_state_hash: Hash32,
    pub current_force_registry_hash: Hash32,
    pub snapshot_force_registry_hash: Hash32,
    pub planned_force_registry_hash: Hash32,
    pub force_delta_hash: Hash32,
    pub force_deltas: Vec<ForceRestoreDelta>,
    pub planned_universe_epoch: u64,
    pub planned_controller_epoch: u64,
    pub safe_mode: CpuState,
    pub preview_hash: Hash32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RestoreApproval {
    snapshot_content_hash: Hash32,
    expected_universe_epoch: u64,
    expected_controller_epoch: u64,
    expected_current_state_hash: Hash32,
    current_force_registry_hash: Hash32,
    snapshot_force_registry_hash: Hash32,
    planned_force_registry_hash: Hash32,
    force_delta_hash: Hash32,
    preview_hash: Hash32,
}

impl RestoreApproval {
    #[must_use]
    pub const fn approve(preview: &RestorePreview) -> Self {
        Self {
            snapshot_content_hash: preview.snapshot_content_hash,
            expected_universe_epoch: preview.expected_universe_epoch,
            expected_controller_epoch: preview.expected_controller_epoch,
            expected_current_state_hash: preview.expected_current_state_hash,
            current_force_registry_hash: preview.current_force_registry_hash,
            snapshot_force_registry_hash: preview.snapshot_force_registry_hash,
            planned_force_registry_hash: preview.planned_force_registry_hash,
            force_delta_hash: preview.force_delta_hash,
            preview_hash: preview.preview_hash,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct SessionIds {
    universe: plc_runtime::UniverseId,
    offline: OfflineControllerId,
    instance: ControllerInstanceId,
    controller: VirtualControllerId,
    online: VirtualOnlineSessionId,
}

impl SessionIds {
    fn derive(project: &Project, controller: ObjectId) -> Self {
        let root = object_u128(project.root_id());
        let controller = object_u128(controller);
        Self {
            universe: plc_runtime::UniverseId(derived_identity(root, b"universe")),
            offline: OfflineControllerId(derived_identity(controller, b"offline")),
            instance: ControllerInstanceId(derived_identity(controller, b"instance")),
            controller: VirtualControllerId(derived_identity(controller, b"runtime")),
            online: VirtualOnlineSessionId(derived_identity(controller, b"online")),
        }
    }
}

#[derive(Clone, Debug)]
pub struct EngineeringSession {
    project: Project,
    controller_object_id: ObjectId,
    hardware: CanonicalHardwareProjection,
    projection_diagnostics: Vec<ProjectDiagnostic>,
    ids: SessionIds,
    universe: VirtualUniverse,
    build: Option<SystemBuildProduct>,
    monitoring: MonitoringEngine,
    modifies: ModifyScheduler,
    forces: ForceRegistry,
    traces: TraceEngine,
    diagnostics: DiagnosticLedger,
    runtime_diagnostics: RuntimeDiagnosticBridge,
    hardware_conditions: HardwareConditionEngine,
    hardware_diagnostics: HardwareDiagnosticBridge,
    hardware_commands: Vec<HardwareFaultCommand>,
    hardware_natural_inputs: BTreeMap<HardwareChannelId, ChannelRawValue>,
    trace_capture_ids: BTreeSet<TraceCaptureId>,
    navigation: Option<NavigationIndex>,
    navigation_revision: u64,
}

impl EngineeringSession {
    /// Creates a deterministic internal engineering universe from canonical
    /// project truth. It does not build, load, power, or go online implicitly.
    ///
    /// # Errors
    ///
    /// Returns canonical hardware, commissioning, or observability
    /// initialization defects without publishing a partial session.
    pub fn new(project: Project, controller_object_id: ObjectId) -> Result<Self, SystemError> {
        let hardware = project_hardware(&project);
        if !hardware.can_build() {
            return Err(SystemError::Projection(hardware.diagnostics().to_vec()));
        }
        let ids = SessionIds::derive(&project, controller_object_id);
        let hardware_fingerprint = hardware
            .artifact()
            .ok_or_else(|| SystemError::Projection(hardware.diagnostics().to_vec()))?
            .hardware_fingerprint;
        let profile_manifest_hash = Hash32::from_bytes(hardware.profile().manifest_hash().0);
        let offline = OfflineEngineeringState {
            configured: ConfiguredController {
                id: ids.offline,
                configured_hardware_fingerprint: hash32(hardware_fingerprint),
                profile_fingerprint: profile_manifest_hash,
            },
            source_revision_hash: hash32(project.semantic_fingerprint()),
            build_snapshot_hash: None,
            project_saved: !project.is_document_dirty(),
            source_to_build: OfflineSourceBuild::Absent,
            software_build_current: false,
            hardware_build_current: true,
            current_package_fingerprint: None,
            built_hardware: None,
        };
        let mut universe = VirtualUniverse::new(ids.universe);
        universe
            .register_offline_controller(offline)
            .map_err(commissioning_error)?;
        universe
            .create_instance(CreateInstanceCommand {
                command_id: derived_identity(object_u128(controller_object_id), b"create"),
                instance_id: ids.instance,
                offline_controller_id: ids.offline,
                controller_id: ids.controller,
                deterministic_seed: identity_seed(object_u128(controller_object_id)),
            })
            .map_err(commissioning_error)?;
        let controller_epoch = universe
            .controller(ids.controller)
            .ok_or_else(|| {
                SystemError::Commissioning("runtime controller was not created".to_owned())
            })?
            .runtime()
            .controller_epoch();
        let hardware_conditions = build_hardware_condition_engine(
            &hardware,
            controller_object_id,
            controller_epoch,
            &[],
        )?;
        let hardware_natural_inputs = initial_hardware_inputs(&hardware_conditions);
        Ok(Self {
            project,
            controller_object_id,
            hardware,
            projection_diagnostics: Vec::new(),
            ids,
            universe,
            build: None,
            monitoring: MonitoringEngine::new(MonitoringLimits::edu21())
                .map_err(|error| SystemError::Monitoring(format!("{error:?}")))?,
            modifies: ModifyScheduler::default(),
            forces: ForceRegistry::new(),
            traces: TraceEngine::new(TraceLimits::edu21())
                .map_err(|error| SystemError::Trace(format!("{error:?}")))?,
            diagnostics: DiagnosticLedger::new(
                DiagnosticRegistry::edu21_runtime(),
                DiagnosticLimits::edu21(),
            )
            .map_err(|error| SystemError::Diagnostics(format!("{error:?}")))?,
            runtime_diagnostics: RuntimeDiagnosticBridge::default(),
            hardware_conditions,
            hardware_diagnostics: HardwareDiagnosticBridge::default(),
            hardware_commands: Vec::new(),
            hardware_natural_inputs,
            trace_capture_ids: BTreeSet::new(),
            navigation: None,
            navigation_revision: 0,
        })
    }

    #[must_use]
    pub const fn project(&self) -> &Project {
        &self.project
    }

    #[must_use]
    pub const fn current_build(&self) -> Option<&SystemBuildProduct> {
        self.build.as_ref()
    }

    #[must_use]
    pub const fn universe(&self) -> &VirtualUniverse {
        &self.universe
    }

    #[must_use]
    pub const fn probe_catalog(&self) -> Option<&plc_observability::ProbeCatalog> {
        match &self.build {
            Some(build) => Some(build.probe_catalog()),
            None => None,
        }
    }

    #[must_use]
    pub const fn force_registry(&self) -> &ForceRegistry {
        &self.forces
    }

    #[must_use]
    pub const fn diagnostic_ledger(&self) -> &DiagnosticLedger {
        &self.diagnostics
    }

    #[must_use]
    pub const fn navigation_index(&self) -> Option<&NavigationIndex> {
        self.navigation.as_ref()
    }

    #[must_use]
    pub fn projection_diagnostics(&self) -> &[ProjectDiagnostic] {
        &self.projection_diagnostics
    }

    pub fn refresh_project(&mut self, project: Project) -> Result<ProjectRefresh, SystemError> {
        let document_changed = project.document_hash() != self.project.document_hash();
        let semantic_changed =
            project.semantic_fingerprint() != self.project.semantic_fingerprint();
        if !document_changed {
            return Ok(ProjectRefresh {
                semantic_changed: false,
                document_changed: false,
                build_invalidated: false,
                loaded_runtime_preserved: self.loaded(),
            });
        }
        let new_hardware = project_hardware(&project);
        let new_software = project_software(&project, self.controller_object_id);
        let mut projection_diagnostics = new_hardware.diagnostics().to_vec();
        projection_diagnostics.extend_from_slice(new_software.diagnostics());
        projection_diagnostics.sort();
        projection_diagnostics.dedup();
        let loaded_runtime_preserved = self.loaded();
        let offline = self
            .universe
            .offline_controller_mut(self.ids.offline)
            .ok_or_else(|| {
                SystemError::Commissioning("offline controller disappeared".to_owned())
            })?;
        offline.project_saved = !project.is_document_dirty();
        if semantic_changed {
            if let Some(artifact) = new_hardware.artifact() {
                offline.configured.configured_hardware_fingerprint =
                    hash32(artifact.hardware_fingerprint);
            }
            offline.mark_source_edited(hash32(project.semantic_fingerprint()));
            offline.hardware_build_current = false;
            self.monitoring.mark_stale();
        }
        self.project = project;
        self.hardware = new_hardware;
        self.projection_diagnostics = projection_diagnostics;
        self.refresh_online_comparison()?;
        if !semantic_changed && self.build.is_some() {
            self.rebuild_navigation()?;
        }
        Ok(ProjectRefresh {
            semantic_changed,
            document_changed,
            build_invalidated: semantic_changed,
            loaded_runtime_preserved,
        })
    }

    pub fn build(&mut self) -> Result<&SystemBuildProduct, SystemError> {
        let product = build_project_controller(&self.project, self.controller_object_id)?;
        let mut projection_diagnostics = product.hardware().diagnostics().to_vec();
        projection_diagnostics.extend_from_slice(product.software().diagnostics());
        projection_diagnostics.sort();
        projection_diagnostics.dedup();
        self.universe
            .offline_controller_mut(self.ids.offline)
            .ok_or_else(|| SystemError::Commissioning("offline controller disappeared".to_owned()))?
            .record_build(product.load_package());
        self.configure_observability(&product)?;
        self.build = Some(product);
        self.projection_diagnostics = projection_diagnostics;
        self.refresh_online_comparison()?;
        self.rebuild_navigation()?;
        self.build.as_ref().ok_or(SystemError::NoCurrentBuild)
    }

    pub fn power_on(&mut self) -> Result<(), SystemError> {
        self.universe
            .power_on(self.ids.controller)
            .map_err(commissioning_error)
    }

    pub fn power_off(&mut self) -> Result<(), SystemError> {
        self.universe
            .power_off(self.ids.controller)
            .map_err(commissioning_error)
    }

    pub fn preview_load(&self, post_load_mode: PostLoadMode) -> Result<LoadPreview, SystemError> {
        let build = self.build.as_ref().ok_or(SystemError::NoCurrentBuild)?;
        if build.source_semantic_fingerprint() != self.project.semantic_fingerprint() {
            return Err(SystemError::CurrentBuildStale);
        }
        self.universe
            .prepare_load(
                self.ids.controller,
                build.load_package(),
                LoadRequest {
                    expected_build_snapshot_hash: build.load_package().build_snapshot_hash(),
                    requested_post_load_mode: post_load_mode,
                    initialize_compatible_members: false,
                    valid_through_event_sequence: self.universe.event_sequence(),
                },
            )
            .map_err(commissioning_error)
    }

    pub fn commit_load(&mut self, preview: &LoadPreview) -> Result<LoadResult, SystemError> {
        let candidate: VirtualLoadPackage = self
            .build
            .as_ref()
            .ok_or(SystemError::NoCurrentBuild)?
            .load_package()
            .clone();
        let mut staged = self.clone();
        let result = staged
            .universe
            .commit_load(
                preview,
                PreviewApproval::approve(preview),
                &candidate,
                LoadExecution::default(),
            )
            .map_err(commissioning_error)?;
        staged.synchronize_hardware_epoch()?;
        staged.refresh_online_comparison()?;
        staged.rebuild_navigation()?;
        *self = staged;
        Ok(result)
    }

    pub fn go_online(&mut self) -> Result<(), SystemError> {
        let mut candidate = self.clone();
        match candidate
            .universe
            .session(candidate.ids.online)
            .map(plc_commissioning::VirtualOnlineSession::state)
        {
            None => {
                candidate
                    .universe
                    .begin_go_online(
                        candidate.ids.online,
                        candidate.ids.offline,
                        candidate.ids.controller,
                    )
                    .map_err(commissioning_error)?;
                candidate
                    .universe
                    .complete_go_online(candidate.ids.online)
                    .map_err(commissioning_error)?;
            }
            Some(SessionState::VirtualLinkLost | SessionState::VirtualUnavailable) => {
                candidate
                    .universe
                    .begin_reconnect(candidate.ids.online)
                    .map_err(commissioning_error)?;
                candidate
                    .universe
                    .complete_reconnect(candidate.ids.online)
                    .map_err(commissioning_error)?;
            }
            Some(state) => {
                return Err(SystemError::Commissioning(format!(
                    "session cannot go online from {state:?}"
                )));
            }
        }
        candidate.synchronize_hardware_epoch()?;
        let boundary = u128::from(candidate.universe.event_sequence());
        candidate.project_actual_hardware_state(derived_identity(
            boundary,
            b"go-online-hardware-state",
        ))?;
        candidate.apply_hardware_input_projection(derived_identity(
            boundary,
            b"go-online-hardware-inputs",
        ))?;
        let _hardware_receipt = candidate.apply_hardware_output_projection(derived_identity(
            boundary,
            b"go-online-hardware-outputs",
        ))?;
        *self = candidate;
        Ok(())
    }

    pub fn request_run(&mut self) -> Result<(), SystemError> {
        let mut candidate = self.clone();
        let binding = candidate.binding()?;
        candidate
            .universe
            .request_run(binding, RestartKind::Resume)
            .map_err(commissioning_error)?;
        let _hardware_receipt = candidate.apply_hardware_output_projection(derived_identity(
            u128::from(candidate.universe.event_sequence()),
            b"run-hardware-outputs",
        ))?;
        *self = candidate;
        Ok(())
    }

    pub fn request_stop(&mut self) -> Result<(), SystemError> {
        let mut candidate = self.clone();
        let binding = candidate.binding()?;
        candidate
            .universe
            .request_stop(binding)
            .map_err(commissioning_error)?;
        let _hardware_receipt = candidate.apply_hardware_output_projection(derived_identity(
            u128::from(candidate.universe.event_sequence()),
            b"stop-hardware-outputs",
        ))?;
        *self = candidate;
        Ok(())
    }

    pub fn set_raw_virtual_input(
        &mut self,
        identity: SystemCommandIdentity,
        target: StableTargetId,
        value: CanonicalValue,
    ) -> Result<InputReceipt, SystemError> {
        let mut candidate = self.clone();
        candidate.synchronize_hardware_epoch()?;
        let (channel_id, hardware_channel_id) = {
            let build = candidate
                .build
                .as_ref()
                .ok_or(SystemError::NoCurrentBuild)?;
            let definition = build
                .probe_catalog()
                .definition(target)
                .ok_or(SystemError::UnknownTarget(target))?;
            let RuntimeTarget::Input(channel_id) = definition.runtime_target else {
                return Err(SystemError::TargetIsNotInput(target));
            };
            let hardware_channel_id = build
                .channel_bindings()
                .iter()
                .find(|binding| binding.runtime_channel_id == channel_id)
                .map(|binding| binding.hardware_channel_id)
                .ok_or_else(|| {
                    SystemError::Hardware(
                        "runtime input has no canonical hardware binding".to_owned(),
                    )
                })?;
            (channel_id, hardware_channel_id)
        };
        let raw = hardware_raw_from_runtime(value).ok_or_else(|| {
            SystemError::Hardware("raw input value is not an EDU-21 channel type".to_owned())
        })?;
        candidate
            .hardware_natural_inputs
            .insert(hardware_channel_id, raw);
        let projected = candidate
            .hardware_conditions
            .project_channel(
                hardware_channel_id,
                NaturalChannelSample {
                    raw_value: raw,
                    provider_quality: HardwareChannelQuality::Good,
                    force_overlay_active: candidate
                        .runtime()?
                        .force_overlay(RuntimeValueTarget::Input(channel_id))
                        .is_some(),
                },
            )
            .map_err(|error| SystemError::Hardware(format!("{error:?}")))?;
        if projected.direction != HardwareChannelDirection::Input {
            return Err(SystemError::TargetIsNotInput(target));
        }
        let binding = candidate.binding()?;
        let command = InputCommand {
            command_id: CommandId(identity.command_id),
            idempotency_key: identity.idempotency_key,
            controller_id: candidate.ids.controller,
            expected_controller_epoch: binding.expected_controller_epoch,
            channel_id,
            value: runtime_value_from_hardware(projected.cpu_value),
            audit_provenance_hash: audit_hash(identity, b"raw-input"),
        };
        let receipt = candidate
            .universe
            .set_virtual_input_raw(binding, command)
            .map_err(commissioning_error)?;
        *self = candidate;
        Ok(receipt)
    }

    pub fn apply_hardware_fault(
        &mut self,
        identity: SystemCommandIdentity,
        action: HardwareFaultAction,
    ) -> Result<HardwareFaultReceipt, SystemError> {
        let mut candidate = self.clone();
        candidate.synchronize_hardware_epoch()?;
        let command = HardwareFaultCommand {
            idempotency_key: Uuid::from_bytes(identity.idempotency_key.to_be_bytes()),
            expected_controller_epoch: candidate.runtime()?.controller_epoch(),
            action,
        };
        let already_applied = candidate
            .hardware_commands
            .iter()
            .any(|prior| prior.idempotency_key == command.idempotency_key);
        let receipt = candidate
            .hardware_conditions
            .apply(command.clone())
            .map_err(|error| SystemError::Hardware(format!("{error:?}")))?;
        if already_applied {
            return Ok(receipt);
        }
        candidate.hardware_commands.push(command);
        candidate.project_actual_hardware_state(identity.command_id)?;
        candidate.apply_hardware_input_projection(identity.command_id)?;
        let _hardware_receipt = candidate.apply_hardware_output_projection(identity.command_id)?;
        let context = candidate.context(PublicationBoundary::SerializedCommand)?;
        let diagnostic_receipts = candidate
            .hardware_diagnostics
            .ingest_events(&mut candidate.diagnostics, context, &receipt.events)
            .map_err(|error| SystemError::Diagnostics(format!("{error:?}")))?;
        let mut diagnostic_events = Vec::new();
        for diagnostic_receipt in diagnostic_receipts
            .iter()
            .filter(|diagnostic_receipt| !diagnostic_receipt.duplicate)
        {
            let event = candidate
                .diagnostics
                .retained_events()
                .into_iter()
                .find(|event| event.occurrence_id == diagnostic_receipt.ledger_occurrence_id)
                .ok_or_else(|| {
                    SystemError::Diagnostics(
                        "hardware diagnostic receipt does not resolve to a retained event"
                            .to_owned(),
                    )
                })?;
            diagnostic_events.push(
                TraceDiagnosticEvent::from_authoritative(event, candidate.diagnostics.registry())
                    .ok_or_else(|| {
                    SystemError::Diagnostics(
                        "hardware diagnostic event does not resolve through the fixed registry"
                            .to_owned(),
                    )
                })?,
            );
        }
        candidate.publish_current_with_diagnostics(
            PublicationBoundary::SerializedCommand,
            diagnostic_events,
        )?;
        *self = candidate;
        Ok(receipt)
    }

    pub fn run_scan(
        &mut self,
        identity: SystemCommandIdentity,
    ) -> Result<CommissionedScanReceipt, SystemError> {
        let binding = self.binding()?;
        let (runtime_state_hash, artifact_fingerprint) = {
            let runtime = self.runtime()?;
            (
                runtime.semantic_state_hash(),
                runtime
                    .loaded_fingerprint()
                    .ok_or(SystemError::NoLoadedArtifact)?,
            )
        };
        let projection = self.force_projection()?;
        let command = RuntimeScanCommand {
            command_id: identity.command_id,
            controller_id: self.ids.controller,
            expected_controller_epoch: binding.expected_controller_epoch,
            expected_artifact_fingerprint: artifact_fingerprint,
            expected_state_hash: runtime_state_hash,
            pre_program_writes: Vec::new(),
            post_program_writes: Vec::new(),
            force_deltas: Vec::new(),
            audit_context_hash: audit_hash(identity, b"scan"),
        };
        let receipt = self
            .universe
            .run_scan_with_observation(binding, &command, &projection)
            .map_err(commissioning_error)?;
        let hardware_receipt = self
            .apply_hardware_output_projection(derived_identity(
                identity.command_id,
                b"scan-hardware-boundary",
            ))?
            .ok_or_else(|| {
                SystemError::Hardware(
                    "RUN scan did not produce a virtual-hardware delivery boundary".to_owned(),
                )
            })?;
        self.publish_after_scan(&receipt, &hardware_receipt)?;
        Ok(receipt)
    }

    pub fn start_monitoring(&mut self) -> Result<(), SystemError> {
        let context = self.context(PublicationBoundary::SerializedCommand)?;
        let catalog = self
            .build
            .as_ref()
            .ok_or(SystemError::NoCurrentBuild)?
            .probe_catalog();
        self.monitoring
            .start(context, catalog)
            .map_err(|error| SystemError::Monitoring(format!("{error:?}")))
    }

    #[must_use]
    pub fn latest_watch_sample(&self, row: WatchRowId) -> Option<&MonitorSample> {
        self.monitoring.latest(row)
    }

    pub fn modify_once(
        &mut self,
        identity: SystemCommandIdentity,
        target: StableTargetId,
        value: CanonicalValue,
    ) -> Result<ModifyExecutionReceipt, SystemError> {
        let boundary = self.command_boundary()?;
        let context = self.context(boundary)?;
        let catalog = self
            .build
            .as_ref()
            .ok_or(SystemError::NoCurrentBuild)?
            .probe_catalog();
        let definition = catalog
            .definition(target)
            .ok_or(SystemError::UnknownTarget(target))?;
        let command = ModifyCommand {
            command_id: identity.command_id,
            idempotency_key: identity.idempotency_key,
            session_id: self.ids.online,
            controller_id: self.ids.controller,
            expected_universe_epoch: context.universe_epoch,
            expected_controller_epoch: context.controller_epoch,
            expected_session_epoch: context.session_epoch,
            expected_artifact_fingerprint: context.artifact_fingerprint,
            expected_target_state_hash: context.target_state_hash,
            expected_probe_catalog_hash: catalog.catalog_hash(),
            expected_force_registry_version: self.forces.version(),
            expected_force_registry_hash: self.forces.registry_hash(),
            allow_overwrite_queued: false,
            requested_boundary: boundary,
            author_identity: identity.author_identity,
            audit_context_hash: audit_hash(identity, b"modify"),
            items: vec![ModifyItem {
                target: TargetReference::Stable(target),
                expected_instance_path: definition.instance_path.clone(),
                expected_value_type: definition.value_type,
                value,
            }],
        };
        let io_states = self.runtime_io_states();
        self.modifies
            .submit_with_io_state(command, context, catalog, &self.forces, &io_states)
            .map_err(|error| SystemError::Modify(format!("{error:?}")))?;
        let plan = self
            .modifies
            .next_due(context, catalog, &self.forces)
            .map_err(|error| SystemError::Modify(format!("{error:?}")))?
            .ok_or_else(|| SystemError::Modify("accepted modify is not due".to_owned()))?;
        let binding = self.binding()?;
        let receipt = publish_modify_plan(
            &mut self.universe,
            binding,
            &mut self.modifies,
            &self.forces,
            &plan,
        )
        .map_err(|error| SystemError::Modify(format!("{error:?}")))?;
        let _hardware_receipt = self.apply_hardware_output_projection(derived_identity(
            identity.command_id,
            b"modify-hardware-outputs",
        ))?;
        self.publish_current(boundary)?;
        Ok(receipt)
    }

    pub fn create_force(
        &mut self,
        identity: SystemCommandIdentity,
        force_id: ForceId,
        target: StableTargetId,
        value: CanonicalValue,
        reason: impl Into<String>,
    ) -> Result<ForceExecutionReceipt, SystemError> {
        let boundary = self.command_boundary()?;
        let context = self.context(boundary)?;
        let catalog = self
            .build
            .as_ref()
            .ok_or(SystemError::NoCurrentBuild)?
            .probe_catalog();
        let definition = catalog
            .definition(target)
            .ok_or(SystemError::UnknownTarget(target))?;
        let natural = self
            .runtime()?
            .natural_value(runtime_value_target(definition.runtime_target))
            .ok_or(SystemError::TargetValueUnavailable(target))?;
        let command = ForceCommand {
            command_id: identity.command_id,
            idempotency_key: identity.idempotency_key,
            expected_universe_epoch: context.universe_epoch,
            expected_controller_epoch: context.controller_epoch,
            expected_session_epoch: context.session_epoch,
            expected_artifact_fingerprint: context.artifact_fingerprint,
            expected_target_state_hash: context.target_state_hash,
            expected_registry_version: self.forces.version(),
            expected_registry_hash: self.forces.registry_hash(),
            audit_context_hash: audit_hash(identity, b"force-create"),
            kind: ForceCommandKind::Create {
                force_id,
                target: TargetReference::Stable(target),
                value,
                natural_at_application: natural,
                actor_identity: identity.author_identity,
                reason: reason.into(),
            },
        };
        self.execute_force(&command, boundary)
    }

    pub fn remove_force(
        &mut self,
        identity: SystemCommandIdentity,
        force_id: ForceId,
        reason: impl Into<String>,
    ) -> Result<ForceExecutionReceipt, SystemError> {
        let boundary = self.command_boundary()?;
        let context = self.context(boundary)?;
        let entry_hash = self
            .forces
            .entry(force_id)
            .ok_or_else(|| SystemError::Force("unknown force identity".to_owned()))?
            .entry_hash;
        let command = ForceCommand {
            command_id: identity.command_id,
            idempotency_key: identity.idempotency_key,
            expected_universe_epoch: context.universe_epoch,
            expected_controller_epoch: context.controller_epoch,
            expected_session_epoch: context.session_epoch,
            expected_artifact_fingerprint: context.artifact_fingerprint,
            expected_target_state_hash: context.target_state_hash,
            expected_registry_version: self.forces.version(),
            expected_registry_hash: self.forces.registry_hash(),
            audit_context_hash: audit_hash(identity, b"force-remove"),
            kind: ForceCommandKind::Remove {
                force_id,
                expected_entry_hash: entry_hash,
                actor_identity: identity.author_identity,
                reason: reason.into(),
            },
        };
        self.execute_force(&command, boundary)
    }

    pub fn arm_trace(&mut self, config: TraceConfigId) -> Result<(), SystemError> {
        let context = self.context(PublicationBoundary::SerializedCommand)?;
        let catalog = self
            .build
            .as_ref()
            .ok_or(SystemError::NoCurrentBuild)?
            .probe_catalog();
        if self.traces.state(config) == plc_observability::TraceState::Idle {
            self.traces
                .arm(config, context, catalog)
                .map_err(|error| SystemError::Trace(format!("{error:?}")))
        } else {
            Err(SystemError::TraceConfigurationUnavailable(config))
        }
    }

    pub fn resolve_navigation(
        &self,
        identity: SemanticIdentity,
        side: ArtifactSide,
    ) -> Result<NavigationResult, SystemError> {
        self.navigation
            .as_ref()
            .ok_or(SystemError::NavigationUnavailable)?
            .resolve(identity, side)
            .map_err(|error| SystemError::Navigation(format!("{error:?}")))
    }

    pub fn snapshot_hashes(&self) -> Result<EngineeringSnapshotHashes, SystemError> {
        let context = self.context(PublicationBoundary::SerializedCommand)?;
        let runtime = self.runtime()?;
        let runtime_snapshot = runtime
            .capture_snapshot()
            .map_err(|error| SystemError::Snapshot(format!("{error:?}")))?;
        let monitoring = self
            .monitoring
            .persistence()
            .map_err(|error| SystemError::Snapshot(format!("{error:?}")))?;
        let forces = self.forces.snapshot(context);
        let traces = self.traces.capture_snapshot(context);
        let diagnostics = self.diagnostics.capture_snapshot(context);
        let hardware_conditions = self.hardware_conditions.snapshot();
        Ok(EngineeringSnapshotHashes {
            document_hash: self.project.document_hash(),
            semantic_fingerprint: self.project.semantic_fingerprint(),
            universe_state_hash: self.universe.semantic_state_hash(),
            controller_state_hash: self.controller_state_hash()?,
            runtime_snapshot_hash: runtime_snapshot.content_hash(),
            runtime_replay_hash: runtime.replay_hash(),
            monitoring_hash: monitoring.content_hash,
            force_snapshot_hash: forces.content_hash,
            trace_snapshot_hash: traces.content_hash,
            diagnostic_snapshot_hash: diagnostics.content_hash,
            diagnostic_replay_hash: self
                .diagnostics
                .replay_hash()
                .map_err(|error| SystemError::Snapshot(format!("{error:?}")))?,
            diagnostic_bridge_replay_hash: self
                .runtime_diagnostics
                .replay_hash()
                .map_err(|error| SystemError::Snapshot(format!("{error:?}")))?,
            hardware_condition_hash: hardware_condition_snapshot_hash(&hardware_conditions),
            hardware_command_hash: hardware_command_log_hash(&self.hardware_commands),
            hardware_input_hash: hardware_natural_input_hash(&self.hardware_natural_inputs),
            hardware_diagnostic_bridge_hash: self
                .hardware_diagnostics
                .replay_hash()
                .map_err(|error| SystemError::Snapshot(format!("{error:?}")))?,
        })
    }

    /// Captures every authoritative runtime/observation subsystem behind one
    /// canonical project and build identity.
    pub fn capture_snapshot(&self) -> Result<EngineeringSessionSnapshot, SystemError> {
        let build = self.build.as_ref().ok_or(SystemError::NoCurrentBuild)?;
        let context = self.context(PublicationBoundary::SerializedCommand)?;
        let runtime = self
            .runtime()?
            .capture_snapshot()
            .map_err(|error| SystemError::Snapshot(format!("{error:?}")))?;
        let monitoring_persistence = self
            .monitoring
            .persistence()
            .map_err(|error| SystemError::Snapshot(format!("{error:?}")))?;
        let forces = self.forces.snapshot(context);
        let traces = self.traces.capture_snapshot(context);
        let diagnostics = self.diagnostics.capture_snapshot(context);
        let runtime_bridge_hash = self
            .runtime_diagnostics
            .replay_hash()
            .map_err(|error| SystemError::Snapshot(format!("{error:?}")))?;
        let hardware_conditions = self.hardware_conditions.snapshot();
        let hardware_condition_hash = hardware_condition_snapshot_hash(&hardware_conditions);
        let hardware_command_hash = hardware_command_log_hash(&self.hardware_commands);
        let hardware_input_hash = hardware_natural_input_hash(&self.hardware_natural_inputs);
        let hardware_diagnostic_bridge_hash = self
            .hardware_diagnostics
            .replay_hash()
            .map_err(|error| SystemError::Snapshot(format!("{error:?}")))?;
        let mut snapshot = EngineeringSessionSnapshot {
            schema_version: 1,
            document_hash: self.project.document_hash(),
            semantic_fingerprint: self.project.semantic_fingerprint(),
            controller_object_id: self.controller_object_id,
            build_snapshot_hash: build.load_package().build_snapshot_hash(),
            loaded_artifact_fingerprint: self
                .runtime()?
                .loaded_fingerprint()
                .ok_or(SystemError::NoLoadedArtifact)?,
            runtime_snapshot_hash: runtime.content_hash(),
            monitoring_hash: monitoring_persistence.content_hash,
            monitor_state: self.monitoring.state(),
            force_snapshot_hash: forces.content_hash,
            trace_snapshot_hash: traces.content_hash,
            diagnostic_snapshot_hash: diagnostics.content_hash,
            runtime_bridge_hash,
            hardware_condition_hash,
            hardware_command_hash,
            hardware_input_hash,
            hardware_diagnostic_bridge_hash,
            content_hash: Hash32::ZERO,
            runtime,
            monitoring_persistence,
            forces,
            traces,
            diagnostics,
            runtime_diagnostics: self.runtime_diagnostics.clone(),
            hardware_conditions,
            hardware_commands: self.hardware_commands.clone(),
            hardware_natural_inputs: self.hardware_natural_inputs.clone(),
            hardware_diagnostics: self.hardware_diagnostics.clone(),
            trace_capture_ids: self.trace_capture_ids.clone(),
        };
        snapshot.content_hash = engineering_snapshot_hash(&snapshot);
        Ok(snapshot)
    }

    /// Produces an immutable restore plan without changing the live session.
    /// The plan binds the complete current/snapshot force registries, their
    /// exact per-force delta, current epochs/state, and the planned rebound.
    pub fn preview_restore(
        &self,
        snapshot: &EngineeringSessionSnapshot,
    ) -> Result<RestorePreview, SystemError> {
        let safe_mode = self.runtime()?.cpu_state();
        if !matches!(safe_mode, CpuState::Stop | CpuState::PausedEducational) {
            return Err(SystemError::Snapshot(format!(
                "restore requires STOP or PAUSED_EDUCATIONAL, found {safe_mode:?}"
            )));
        }
        let expected_universe_epoch = self.universe.universe_epoch();
        let expected_controller_epoch = self.runtime()?.controller_epoch();
        let expected_current_state_hash = self.universe.semantic_state_hash();
        let current_force_registry_hash = self.forces.registry_hash();
        let snapshot_force_registry_hash = snapshot.forces.registry_hash;

        // Exercise the complete restore path on an isolated clone. This makes
        // preview a real validation/rebind pass rather than an optimistic diff.
        let mut simulated = self.clone();
        simulated.apply_snapshot(snapshot)?;
        if simulated.runtime()?.cpu_state() != safe_mode {
            return Err(SystemError::Snapshot(
                "snapshot restore changed the approved safe CPU mode".to_owned(),
            ));
        }
        let planned_universe_epoch = simulated.universe.universe_epoch();
        let planned_controller_epoch = simulated.runtime()?.controller_epoch();
        let planned_force_registry_hash = simulated.forces.registry_hash();
        let force_deltas = force_restore_deltas(&self.forces, &simulated.forces);
        let force_delta_hash = hash_force_restore_deltas(
            current_force_registry_hash,
            snapshot_force_registry_hash,
            planned_force_registry_hash,
            &force_deltas,
        );
        let mut preview = RestorePreview {
            snapshot_content_hash: snapshot.content_hash,
            expected_universe_epoch,
            expected_controller_epoch,
            expected_current_state_hash,
            current_force_registry_hash,
            snapshot_force_registry_hash,
            planned_force_registry_hash,
            force_delta_hash,
            force_deltas,
            planned_universe_epoch,
            planned_controller_epoch,
            safe_mode,
            preview_hash: Hash32::ZERO,
        };
        preview.preview_hash = hash_restore_preview(&preview);
        Ok(preview)
    }

    /// Atomically restores the runtime and every dependent observation model
    /// only after an exact, still-current preview is explicitly approved.
    pub fn commit_restore(
        &mut self,
        snapshot: &EngineeringSessionSnapshot,
        preview: &RestorePreview,
        approval: RestoreApproval,
    ) -> Result<Hash32, SystemError> {
        let current = self.preview_restore(snapshot)?;
        if &current != preview || approval != RestoreApproval::approve(&current) {
            return Err(SystemError::Snapshot(
                "restore approval no longer matches the exact current preview".to_owned(),
            ));
        }
        let mut candidate = self.clone();
        let restored_hash = candidate.apply_snapshot(snapshot)?;
        if candidate.universe.universe_epoch() != preview.planned_universe_epoch
            || candidate.runtime()?.controller_epoch() != preview.planned_controller_epoch
            || candidate.runtime()?.cpu_state() != preview.safe_mode
            || candidate.forces.registry_hash() != preview.planned_force_registry_hash
        {
            return Err(SystemError::Snapshot(
                "restore result diverged from its approved plan".to_owned(),
            ));
        }
        *self = candidate;
        Ok(restored_hash)
    }

    /// Applies to an isolated candidate only. Callers must use the public
    /// preview/approval/commit protocol so no live restore bypass exists.
    #[allow(clippy::too_many_lines)]
    fn apply_snapshot(
        &mut self,
        snapshot: &EngineeringSessionSnapshot,
    ) -> Result<Hash32, SystemError> {
        if snapshot.schema_version != 1
            || snapshot.content_hash != engineering_snapshot_hash(snapshot)
            || snapshot.runtime_snapshot_hash != snapshot.runtime.content_hash()
            || snapshot.monitoring_hash != snapshot.monitoring_persistence.content_hash
            || snapshot.force_snapshot_hash != snapshot.forces.content_hash
            || snapshot.trace_snapshot_hash != snapshot.traces.content_hash
            || snapshot.diagnostic_snapshot_hash != snapshot.diagnostics.content_hash
            || !snapshot.monitoring_persistence.verify()
            || !snapshot.traces.verify()
            || !snapshot.diagnostics.verify()
            || snapshot.runtime_diagnostics.replay_hash().ok() != Some(snapshot.runtime_bridge_hash)
            || snapshot.hardware_condition_hash
                != hardware_condition_snapshot_hash(&snapshot.hardware_conditions)
            || snapshot.hardware_command_hash
                != hardware_command_log_hash(&snapshot.hardware_commands)
            || snapshot.hardware_input_hash
                != hardware_natural_input_hash(&snapshot.hardware_natural_inputs)
            || snapshot.hardware_diagnostics.replay_hash().ok()
                != Some(snapshot.hardware_diagnostic_bridge_hash)
        {
            return Err(SystemError::Snapshot(
                "aggregate snapshot integrity verification failed".to_owned(),
            ));
        }
        if snapshot.controller_object_id != self.controller_object_id
            || snapshot.semantic_fingerprint != self.project.semantic_fingerprint()
        {
            return Err(SystemError::Snapshot(
                "snapshot canonical project identity does not match this session".to_owned(),
            ));
        }
        let captured_hardware = build_hardware_condition_engine(
            &self.hardware,
            self.controller_object_id,
            snapshot.hardware_conditions.controller_epoch,
            &snapshot.hardware_commands,
        )
        .map_err(|error| SystemError::Snapshot(format!("hardware replay failed: {error:?}")))?;
        if captured_hardware.snapshot() != snapshot.hardware_conditions
            || !hardware_input_shape_valid(&captured_hardware, &snapshot.hardware_natural_inputs)
        {
            return Err(SystemError::Snapshot(
                "snapshot hardware condition replay or input shape is inconsistent".to_owned(),
            ));
        }
        let build = self.build.as_ref().ok_or(SystemError::NoCurrentBuild)?;
        if snapshot.build_snapshot_hash != build.load_package().build_snapshot_hash()
            || snapshot.loaded_artifact_fingerprint != build.runtime_artifact().fingerprint()
            || self.runtime()?.loaded_fingerprint() != Some(snapshot.loaded_artifact_fingerprint)
        {
            return Err(SystemError::Snapshot(
                "snapshot build or loaded artifact identity does not match".to_owned(),
            ));
        }
        let catalog = build.probe_catalog();
        let mut candidate = self.clone();
        let binding = candidate.binding()?;
        let restored_runtime_hash = candidate
            .universe
            .restore_controller_snapshot(binding, &snapshot.runtime)
            .map_err(commissioning_error)?;
        candidate
            .universe
            .begin_reconnect(candidate.ids.online)
            .map_err(commissioning_error)?;
        candidate
            .universe
            .complete_reconnect(candidate.ids.online)
            .map_err(commissioning_error)?;
        let rebound_hardware_epoch = candidate.runtime()?.controller_epoch();
        let rebound_hardware_commands = snapshot
            .hardware_commands
            .iter()
            .cloned()
            .map(|mut command| {
                command.expected_controller_epoch = rebound_hardware_epoch;
                command
            })
            .collect::<Vec<_>>();
        candidate.hardware_conditions = build_hardware_condition_engine(
            &candidate.hardware,
            candidate.controller_object_id,
            rebound_hardware_epoch,
            &rebound_hardware_commands,
        )?;
        candidate.hardware_commands = rebound_hardware_commands;
        candidate
            .hardware_natural_inputs
            .clone_from(&snapshot.hardware_natural_inputs);
        candidate.hardware_diagnostics = snapshot.hardware_diagnostics.clone();
        candidate.project_actual_hardware_state(derived_identity(
            identity_from_hash(snapshot.content_hash),
            b"snapshot-hardware-state",
        ))?;
        candidate.apply_hardware_input_projection(derived_identity(
            identity_from_hash(snapshot.content_hash),
            b"snapshot-hardware-inputs",
        ))?;

        let rebound_context = candidate.context(PublicationBoundary::SnapshotReplay)?;
        let (forces, _) =
            ForceRegistry::rebind_snapshot(&snapshot.forces, rebound_context, catalog)
                .map_err(|error| SystemError::Snapshot(format!("{error:?}")))?;
        let expected_overlay = canonical_force_overlay_hash(
            &forces
                .entries()
                .map(|entry| (runtime_value_target(entry.runtime_target), entry.value))
                .collect::<Vec<_>>(),
        )
        .map_err(|error| SystemError::Snapshot(format!("{error:?}")))?;
        if candidate.runtime()?.force_overlay_hash() != expected_overlay {
            return Err(SystemError::Snapshot(
                "runtime snapshot and force snapshot disagree".to_owned(),
            ));
        }
        let prior_force_hash = candidate
            .universe
            .controller(candidate.ids.controller)
            .ok_or_else(|| SystemError::Snapshot("restored controller is unavailable".to_owned()))?
            .force_registry_hash();
        let projection = ForceRegistryProjection::new(
            prior_force_hash,
            forces.registry_hash(),
            forces.active_ids(),
            expected_overlay,
        )
        .map_err(commissioning_error)?;
        let binding = candidate.binding()?;
        let runtime = candidate.runtime()?;
        let synchronization = RuntimeBoundaryCommand {
            command_id: identity_from_hash(snapshot.content_hash),
            controller_id: candidate.ids.controller,
            expected_controller_epoch: binding.expected_controller_epoch,
            expected_artifact_fingerprint: snapshot.loaded_artifact_fingerprint,
            expected_state_hash: runtime.semantic_state_hash(),
            natural_writes: Vec::new(),
            force_deltas: Vec::new(),
            audit_context_hash: snapshot.content_hash,
        };
        candidate
            .universe
            .apply_observation_boundary(binding, &synchronization, &projection)
            .map_err(commissioning_error)?;
        let _hardware_receipt = candidate.apply_hardware_output_projection(derived_identity(
            identity_from_hash(snapshot.content_hash),
            b"snapshot-hardware-outputs",
        ))?;
        let context = candidate.context(PublicationBoundary::SnapshotReplay)?;

        let mut monitoring = MonitoringEngine::new(MonitoringLimits::edu21())
            .map_err(|error| SystemError::Snapshot(format!("{error:?}")))?;
        monitoring
            .restore_persistence(&snapshot.monitoring_persistence)
            .map_err(|error| SystemError::Snapshot(format!("{error:?}")))?;
        if matches!(
            snapshot.monitor_state,
            MonitorState::Active | MonitorState::Degraded
        ) {
            monitoring
                .start(context, catalog)
                .map_err(|error| SystemError::Snapshot(format!("{error:?}")))?;
        }
        let traces = TraceEngine::restore_snapshot(
            &snapshot.traces,
            context,
            catalog,
            TraceEventKey {
                universe_epoch: context.universe_epoch,
                event_sequence: context.event_sequence,
            },
        )
        .map_err(|error| SystemError::Snapshot(format!("{error:?}")))?;
        if snapshot
            .trace_capture_ids
            .iter()
            .any(|id| traces.capture(*id).is_none())
        {
            return Err(SystemError::Snapshot(
                "snapshot trace capture index is inconsistent".to_owned(),
            ));
        }
        let diagnostics = DiagnosticLedger::restore_snapshot(&snapshot.diagnostics, context)
            .map_err(|error| SystemError::Snapshot(format!("{error:?}")))?;

        candidate.monitoring = monitoring;
        candidate.modifies = ModifyScheduler::default();
        candidate.forces = forces;
        candidate.traces = traces;
        candidate.diagnostics = diagnostics;
        candidate.runtime_diagnostics = snapshot.runtime_diagnostics.clone();
        candidate
            .trace_capture_ids
            .clone_from(&snapshot.trace_capture_ids);
        candidate.rebuild_navigation()?;
        *self = candidate;
        Ok(restored_runtime_hash)
    }

    #[must_use]
    pub fn status(&self) -> EngineeringStatus {
        let instance = self.universe.controller(self.ids.controller);
        let session = self.universe.session(self.ids.online);
        let loaded = self.loaded();
        let build_current = self.build.as_ref().is_some_and(|build| {
            build.source_semantic_fingerprint() == self.project.semantic_fingerprint()
        });
        EngineeringStatus {
            document_hash: self.project.document_hash(),
            semantic_fingerprint: self.project.semantic_fingerprint(),
            document_dirty: self.project.is_document_dirty(),
            semantic_dirty: self.project.is_semantic_dirty(),
            build_current,
            loaded,
            online: session.is_some_and(|value| value.state() == SessionState::Online),
            session_state: session.map(plc_commissioning::VirtualOnlineSession::state),
            cpu_state: instance.map_or(CpuState::PoweredOff, |value| value.runtime().cpu_state()),
            software_to_loaded: session.map(|value| {
                if loaded && !build_current {
                    MatchComparison::Mismatch
                } else {
                    value.comparison().software_to_loaded
                }
            }),
            hardware_to_loaded: session.map(|value| {
                if loaded && !self.hardware.can_build() {
                    MatchComparison::Mismatch
                } else {
                    value.comparison().hardware_to_loaded
                }
            }),
            force_count: self.forces.entries().len(),
            monitor_state: self.monitoring.state(),
            projection_blocked: self
                .projection_diagnostics
                .iter()
                .any(|diagnostic| diagnostic.blocking),
        }
    }

    /// Returns one immutable, deterministic projection suitable for a thin
    /// WASM/UI adapter. All runtime values are read from the authoritative
    /// controller and all authored identities come from the current build.
    #[allow(clippy::too_many_lines)]
    pub fn read_model(&self) -> Result<EngineeringReadModel, SystemError> {
        let runtime = self.runtime()?;
        let build = self.build.as_ref();
        let publications = if self.loaded() && build.is_some() {
            self.published_values()?
        } else {
            Vec::new()
        };
        let publication_by_id = publications
            .into_iter()
            .map(|value| (value.target_id, value))
            .collect::<std::collections::BTreeMap<_, _>>();
        let probes = build
            .into_iter()
            .flat_map(|build| build.probe_catalog().definitions())
            .map(|definition| {
                let publication = publication_by_id.get(&definition.id);
                let forced_value = self
                    .forces
                    .entries()
                    .find(|entry| entry.target_id == definition.id)
                    .map(|entry| entry.value);
                ProbeReadModel {
                    identity: definition.id,
                    display_name: definition.display_name.clone(),
                    value_type: definition.value_type,
                    runtime_target: definition.runtime_target,
                    natural_value: publication.map(|value| value.natural_value),
                    effective_value: publication.map(|value| value.effective_value),
                    raw_input_value: publication.and_then(|value| value.raw_input_value),
                    committed_output_value: publication
                        .and_then(|value| value.committed_output_value),
                    delivered_output_value: publication
                        .and_then(|value| value.delivered_output_value),
                    quality: publication.map_or(Quality::NotPresent, |value| value.quality),
                    forced_value,
                }
            })
            .collect();

        let watches = self
            .monitoring
            .persistence()
            .map_err(|error| SystemError::Monitoring(format!("{error:?}")))?
            .tables
            .into_iter()
            .map(|table| WatchTableReadModel {
                latest_samples: table
                    .rows
                    .iter()
                    .map(|row| (row.id, self.monitoring.latest(row.id).copied()))
                    .collect(),
                table,
            })
            .collect();

        let traces = build
            .into_iter()
            .flat_map(|build| build.software().trace_configs())
            .map(|canonical| {
                let config = projected_trace_config(canonical);
                let captures = self
                    .trace_capture_ids
                    .iter()
                    .filter_map(|id| self.traces.capture(*id))
                    .filter(|capture| capture.config_id == config.id)
                    .cloned()
                    .collect();
                TraceReadModel {
                    state: self.traces.state(config.id),
                    config,
                    captures,
                }
            })
            .collect();

        let active = self.diagnostics.active_conditions().cloned().collect();
        let retained = self
            .diagnostics
            .retained_events()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        let navigation = retained
            .iter()
            .map(|event| {
                let value = self.navigation.as_ref().and_then(|index| {
                    index
                        .resolve_diagnostic(event.occurrence_id.0, ArtifactSide::Loaded)
                        .ok()
                });
                (event.occurrence_id.0, value)
            })
            .collect();
        let snapshot_hashes = self.snapshot_hashes().ok();
        Ok(EngineeringReadModel {
            controller_object_id: self.controller_object_id,
            universe_id: self.ids.universe,
            universe_epoch: self.universe.universe_epoch(),
            runtime_controller_id: self.ids.controller,
            controller_epoch: runtime.controller_epoch(),
            cpu_state: runtime.cpu_state(),
            scan_sequence: runtime.scan_sequence(),
            virtual_time_ms: runtime.virtual_time_ms(),
            status: self.status(),
            build_semantic_fingerprint: build.map(SystemBuildProduct::source_semantic_fingerprint),
            build_snapshot_hash: build.map(|value| value.load_package().build_snapshot_hash()),
            loaded_artifact_fingerprint: runtime.loaded_fingerprint(),
            profile_fingerprint: build.map(|value| value.compiler_artifact().profile_fingerprint()),
            probes,
            watches,
            forces: self.forces.global_projection(),
            force_audit: self.forces.audit_records().cloned().collect(),
            traces,
            diagnostics: DiagnosticReadModel {
                active,
                retained,
                navigation,
            },
            snapshot_hashes,
            runtime_replay_hash: runtime.replay_hash(),
            diagnostic_replay_hash: self
                .diagnostics
                .replay_hash()
                .map_err(|error| SystemError::Diagnostics(format!("{error:?}")))?,
            diagnostic_bridge_replay_hash: self
                .runtime_diagnostics
                .replay_hash()
                .map_err(|error| SystemError::Diagnostics(format!("{error:?}")))?,
            projection_diagnostics: self.projection_diagnostics.clone(),
        })
    }

    fn configure_observability(&mut self, product: &SystemBuildProduct) -> Result<(), SystemError> {
        let mut monitoring = MonitoringEngine::new(MonitoringLimits::edu21())
            .map_err(|error| SystemError::Monitoring(format!("{error:?}")))?;
        for table in product.software().watch_tables() {
            monitoring
                .upsert_table(WatchTable {
                    id: WatchTableId(object_u128(table.object_id)),
                    name: table.name.clone(),
                    rows: table
                        .rows
                        .iter()
                        .map(|row| WatchRow {
                            id: WatchRowId(row.id),
                            target: TargetReference::Stable(StableTargetId(object_u128(
                                row.target_tag,
                            ))),
                            layer: probe_layer(row.layer),
                            display_base: display_base(row.display_base),
                            unit: None,
                            format: None,
                            note: None,
                            order: row.order,
                        })
                        .collect(),
                })
                .map_err(|error| SystemError::Monitoring(format!("{error:?}")))?;
        }
        let mut traces = TraceEngine::new(TraceLimits::edu21())
            .map_err(|error| SystemError::Trace(format!("{error:?}")))?;
        for config in product.software().trace_configs() {
            if config.channels.is_empty() {
                continue;
            }
            traces
                .upsert_config(projected_trace_config(config))
                .map_err(|error| SystemError::Trace(format!("{error:?}")))?;
        }
        self.monitoring = monitoring;
        self.traces = traces;
        self.trace_capture_ids.clear();
        self.modifies = ModifyScheduler::default();
        Ok(())
    }

    fn execute_force(
        &mut self,
        command: &ForceCommand,
        boundary: PublicationBoundary,
    ) -> Result<ForceExecutionReceipt, SystemError> {
        let binding = self.binding()?;
        let catalog = self
            .build
            .as_ref()
            .ok_or(SystemError::NoCurrentBuild)?
            .probe_catalog();
        let io_states = self.runtime_io_states();
        let receipt = execute_force_command_with_io_state(
            &mut self.universe,
            binding,
            &mut self.forces,
            &self.modifies,
            catalog,
            command,
            &io_states,
        )
        .map_err(|error| SystemError::Force(format!("{error:?}")))?;
        let _hardware_receipt = self.apply_hardware_output_projection(derived_identity(
            command.command_id,
            b"force-hardware-outputs",
        ))?;
        self.publish_current(boundary)?;
        Ok(receipt)
    }

    fn synchronize_hardware_epoch(&mut self) -> Result<(), SystemError> {
        let epoch = self.runtime()?.controller_epoch();
        if self.hardware_conditions.controller_epoch() != epoch {
            self.hardware_conditions = build_hardware_condition_engine(
                &self.hardware,
                self.controller_object_id,
                epoch,
                &self.hardware_commands,
            )?;
            let defaults = initial_hardware_inputs(&self.hardware_conditions);
            self.hardware_natural_inputs
                .retain(|id, _| defaults.contains_key(id));
            for (id, value) in defaults {
                self.hardware_natural_inputs.entry(id).or_insert(value);
            }
        }
        Ok(())
    }

    fn project_actual_hardware_state(&mut self, command_id: u128) -> Result<(), SystemError> {
        let snapshot = self.hardware_conditions.snapshot();
        let present = !snapshot
            .active_conditions
            .iter()
            .any(|condition| matches!(condition, HardwareConditionKey::ControllerUnpowered(_)));
        let fault_state_hash = if snapshot.active_conditions.is_empty() {
            Hash32::ZERO
        } else {
            hash32(snapshot.state_fingerprint)
        };
        let instance = self
            .universe
            .controller(self.ids.controller)
            .ok_or_else(|| {
                SystemError::Commissioning("runtime controller disappeared".to_owned())
            })?;
        self.universe
            .apply_actual_hardware_fault(ActualHardwareFaultCommand {
                command_id,
                target_controller_id: self.ids.controller,
                expected_universe_epoch: self.universe.universe_epoch(),
                expected_controller_epoch: instance.runtime().controller_epoch(),
                expected_target_state_hash: instance.semantic_state_hash(),
                present,
                fault_state_hash,
            })
            .map_err(commissioning_error)
    }

    fn apply_hardware_input_projection(&mut self, command_id: u128) -> Result<(), SystemError> {
        let mappings = self
            .build
            .as_ref()
            .ok_or(SystemError::NoCurrentBuild)?
            .channel_bindings()
            .iter()
            .filter(|binding| binding.hardware.direction == HardwareChannelDirection::Input)
            .cloned()
            .collect::<Vec<_>>();
        for mapping in mappings {
            let natural = self
                .hardware_natural_inputs
                .get(&mapping.hardware_channel_id)
                .copied()
                .or_else(|| ChannelRawValue::canonical_default(mapping.hardware.raw_type))
                .ok_or_else(|| {
                    SystemError::Hardware(
                        "configured input has no canonical raw default".to_owned(),
                    )
                })?;
            let runtime_target = RuntimeValueTarget::Input(mapping.runtime_channel_id);
            let projection = self
                .hardware_conditions
                .project_channel(
                    mapping.hardware_channel_id,
                    NaturalChannelSample {
                        raw_value: natural,
                        provider_quality: HardwareChannelQuality::Good,
                        force_overlay_active: self
                            .runtime()?
                            .force_overlay(runtime_target)
                            .is_some(),
                    },
                )
                .map_err(|error| SystemError::Hardware(format!("{error:?}")))?;
            let binding = self.binding()?;
            let projected_identity =
                hardware_projection_identity(command_id, mapping.hardware_channel_id, b"input");
            self.universe
                .set_virtual_input_raw(
                    binding,
                    InputCommand {
                        command_id: CommandId(projected_identity),
                        idempotency_key: projected_identity,
                        controller_id: self.ids.controller,
                        expected_controller_epoch: binding.expected_controller_epoch,
                        channel_id: mapping.runtime_channel_id,
                        value: runtime_value_from_hardware(projection.cpu_value),
                        audit_provenance_hash: hash32(projection.causal_fingerprint),
                    },
                )
                .map_err(commissioning_error)?;
        }
        Ok(())
    }

    fn apply_hardware_output_projection(
        &mut self,
        command_id: u128,
    ) -> Result<Option<CommissionedHardwareBoundaryReceipt>, SystemError> {
        if !matches!(
            self.runtime()?.cpu_state(),
            CpuState::Run | CpuState::Stop | CpuState::PausedEducational | CpuState::Faulted
        ) {
            return Ok(None);
        }
        let mappings = self
            .build
            .as_ref()
            .ok_or(SystemError::NoCurrentBuild)?
            .channel_bindings()
            .iter()
            .filter(|binding| binding.hardware.direction == HardwareChannelDirection::Output)
            .cloned()
            .collect::<Vec<_>>();
        let mut overrides = Vec::new();
        let mut causal_hashes = Vec::new();
        for mapping in mappings {
            let target = RuntimeValueTarget::Output(mapping.runtime_channel_id);
            let value = self.runtime()?.effective_value(target).ok_or(
                SystemError::TargetValueUnavailable(StableTargetId(u128::from_be_bytes(
                    mapping.hardware_channel_id.uuid().into_bytes(),
                ))),
            )?;
            let raw = hardware_raw_from_runtime(value).ok_or_else(|| {
                SystemError::Hardware(
                    "configured output value is not an EDU-21 channel type".to_owned(),
                )
            })?;
            let projection = self
                .hardware_conditions
                .project_channel(
                    mapping.hardware_channel_id,
                    NaturalChannelSample {
                        raw_value: raw,
                        provider_quality: HardwareChannelQuality::Good,
                        force_overlay_active: self.runtime()?.force_overlay(target).is_some(),
                    },
                )
                .map_err(|error| SystemError::Hardware(format!("{error:?}")))?;
            causal_hashes.push(projection.causal_fingerprint);
            if projection.delivery_suppressed || projection.quality != HardwareChannelQuality::Good
            {
                overrides.push(RuntimeOutputDeliveryOverride {
                    channel_id: mapping.runtime_channel_id,
                    delivered_value: runtime_value_from_hardware(projection.delivered_value),
                    quality: runtime_quality_from_hardware(projection.quality),
                    suppressed: projection.delivery_suppressed,
                });
            }
        }
        let binding = self.binding()?;
        let runtime = self.runtime()?;
        let artifact = runtime
            .loaded_fingerprint()
            .ok_or(SystemError::NoLoadedArtifact)?;
        let audit_context_hash = hash_hardware_projection_set(&causal_hashes);
        let command = RuntimeHardwareBoundaryCommand {
            command_id,
            controller_id: self.ids.controller,
            expected_universe_epoch: runtime.universe_epoch(),
            expected_controller_epoch: runtime.controller_epoch(),
            expected_artifact_fingerprint: artifact,
            expected_state_hash: runtime.semantic_state_hash(),
            output_overrides: overrides,
            audit_context_hash,
        };
        let receipt = self
            .universe
            .apply_hardware_boundary(binding, &command)
            .map_err(commissioning_error)?;
        Ok(Some(receipt))
    }

    fn hardware_projection_for_target(
        &self,
        target: RuntimeTarget,
    ) -> Option<ChannelConditionProjection> {
        let build = self.build.as_ref()?;
        let runtime_channel = runtime_channel_id(target)?;
        let mapping = build
            .channel_bindings()
            .iter()
            .find(|binding| binding.runtime_channel_id == runtime_channel)?;
        let runtime_target = runtime_value_target(target);
        let raw_value = match target {
            RuntimeTarget::Input(_) => self
                .hardware_natural_inputs
                .get(&mapping.hardware_channel_id)
                .copied()?,
            RuntimeTarget::Output(_) => {
                hardware_raw_from_runtime(self.runtime().ok()?.effective_value(runtime_target)?)?
            }
            RuntimeTarget::Memory(_) => return None,
        };
        self.hardware_conditions
            .project_channel(
                mapping.hardware_channel_id,
                NaturalChannelSample {
                    raw_value,
                    provider_quality: HardwareChannelQuality::Good,
                    force_overlay_active: self
                        .runtime()
                        .ok()?
                        .force_overlay(runtime_target)
                        .is_some(),
                },
            )
            .ok()
    }

    fn publish_after_scan(
        &mut self,
        receipt: &CommissionedScanReceipt,
        hardware_receipt: &CommissionedHardwareBoundaryReceipt,
    ) -> Result<(), SystemError> {
        let context = self.context(PublicationBoundary::ScanEnd)?;
        let values = self.published_values()?;
        let diagnostic_events = self.ingest_runtime_diagnostics()?;
        if matches!(
            self.monitoring.state(),
            MonitorState::Active | MonitorState::Degraded
        ) {
            self.monitoring
                .publish(context, &values)
                .map_err(|error| SystemError::Monitoring(format!("{error:?}")))?;
        }
        let captures = match &receipt.runtime.outcome {
            RunOutcome::Completed(_) => {
                let runtime_publication =
                    TraceRuntimePublication::from_commissioned_scan_after_hardware_boundary(
                        context,
                        receipt,
                        hardware_receipt,
                    )
                    .map_err(|error| SystemError::Trace(format!("{error:?}")))?;
                self.traces.publish_with_runtime(
                    context,
                    &values,
                    &diagnostic_events,
                    &runtime_publication,
                )
            }
            RunOutcome::Faulted(_) => self.traces.publish(context, &values, &diagnostic_events),
        }
        .map_err(|error| SystemError::Trace(format!("{error:?}")))?;
        self.trace_capture_ids.extend(captures);
        if !diagnostic_events.is_empty() {
            self.rebuild_navigation()?;
        }
        Ok(())
    }

    fn publish_current(&mut self, boundary: PublicationBoundary) -> Result<(), SystemError> {
        self.publish_current_with_diagnostics(boundary, Vec::new())
    }

    fn publish_current_with_diagnostics(
        &mut self,
        boundary: PublicationBoundary,
        mut diagnostic_events: Vec<TraceDiagnosticEvent>,
    ) -> Result<(), SystemError> {
        let context = self.context(boundary)?;
        let values = self.published_values()?;
        diagnostic_events.extend(self.ingest_runtime_diagnostics()?);
        diagnostic_events.sort_by_key(|event| (event.key, event.occurrence_id));
        diagnostic_events.dedup_by_key(|event| event.occurrence_id);
        if matches!(
            self.monitoring.state(),
            MonitorState::Active | MonitorState::Degraded
        ) {
            self.monitoring
                .publish(context, &values)
                .map_err(|error| SystemError::Monitoring(format!("{error:?}")))?;
        }
        let captures = self
            .traces
            .publish(context, &values, &diagnostic_events)
            .map_err(|error| SystemError::Trace(format!("{error:?}")))?;
        self.trace_capture_ids.extend(captures);
        Ok(())
    }

    fn ingest_runtime_diagnostics(&mut self) -> Result<Vec<TraceDiagnosticEvent>, SystemError> {
        let binding = self.binding()?;
        let receipts = self
            .runtime_diagnostics
            .ingest_from_virtual_universe(&mut self.diagnostics, &self.universe, binding)
            .map_err(|error| SystemError::Diagnostics(format!("{error:?}")))?;
        let mut events = Vec::new();
        for receipt in receipts.iter().filter(|receipt| !receipt.duplicate) {
            let event = self
                .diagnostics
                .retained_events()
                .into_iter()
                .find(|event| event.occurrence_id == receipt.ledger_occurrence_id)
                .ok_or_else(|| {
                    SystemError::Diagnostics(
                        "runtime diagnostic receipt does not resolve to a retained event"
                            .to_owned(),
                    )
                })?;
            events.push(
                TraceDiagnosticEvent::from_authoritative(event, self.diagnostics.registry())
                    .ok_or_else(|| {
                        SystemError::Diagnostics(
                            "runtime diagnostic event does not resolve through the fixed registry"
                                .to_owned(),
                        )
                    })?,
            );
        }
        Ok(events)
    }

    fn published_values(&self) -> Result<Vec<PublishedTargetValue>, SystemError> {
        let build = self.build.as_ref().ok_or(SystemError::NoCurrentBuild)?;
        let runtime = self.runtime()?;
        let mut values = Vec::new();
        for definition in build.probe_catalog().definitions() {
            let target = runtime_value_target(definition.runtime_target);
            let hardware_projection =
                self.hardware_projection_for_target(definition.runtime_target);
            let natural = runtime
                .natural_value(target)
                .ok_or(SystemError::TargetValueUnavailable(definition.id))?;
            let effective = runtime
                .effective_value(target)
                .ok_or(SystemError::TargetValueUnavailable(definition.id))?;
            let force = self
                .forces
                .entries()
                .find(|entry| entry.runtime_target == definition.runtime_target)
                .map(|entry| ForceProvenance {
                    force_id: entry.id.0,
                    registry_version: self.forces.version(),
                });
            let (raw_input_value, committed_output_value, delivered_output_value) =
                match definition.runtime_target {
                    RuntimeTarget::Memory(_) => (None, None, None),
                    RuntimeTarget::Input(id) => (
                        build
                            .channel_bindings()
                            .iter()
                            .find(|binding| binding.runtime_channel_id == id)
                            .and_then(|binding| {
                                self.hardware_natural_inputs
                                    .get(&binding.hardware_channel_id)
                                    .copied()
                            })
                            .map(runtime_value_from_hardware),
                        None,
                        None,
                    ),
                    RuntimeTarget::Output(id) => (
                        None,
                        runtime.natural_output(id),
                        runtime
                            .boundary()
                            .delivered_output(id)
                            .map(|output| output.canonical_value),
                    ),
                };
            values.push(PublishedTargetValue {
                target_id: definition.id,
                value_type: definition.value_type,
                natural_value: natural,
                effective_value: effective,
                raw_input_value,
                committed_output_value,
                delivered_output_value,
                quality: hardware_projection
                    .as_ref()
                    .map_or(Quality::Good, |projection| {
                        observation_quality_from_hardware(projection.quality)
                    }),
                force,
            });
        }
        Ok(values)
    }

    fn runtime_io_states(&self) -> Vec<RuntimeIoState> {
        self.build
            .as_ref()
            .map(|build| {
                build
                    .probe_catalog()
                    .definitions()
                    .filter(|definition| {
                        matches!(
                            definition.runtime_target,
                            RuntimeTarget::Input(_) | RuntimeTarget::Output(_)
                        )
                    })
                    .map(|definition| {
                        let projection =
                            self.hardware_projection_for_target(definition.runtime_target);
                        RuntimeIoState {
                            target_id: definition.id,
                            runtime_present: projection.as_ref().is_none_or(|projection| {
                                projection.quality != HardwareChannelQuality::NotPresent
                            }),
                            quality: projection.as_ref().map_or(Quality::Good, |projection| {
                                observation_quality_from_hardware(projection.quality)
                            }),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn force_projection(&self) -> Result<ForceRegistryProjection, SystemError> {
        let instance = self
            .universe
            .controller(self.ids.controller)
            .ok_or_else(|| {
                SystemError::Commissioning("runtime controller disappeared".to_owned())
            })?;
        ForceRegistryProjection::new(
            instance.force_registry_hash(),
            self.forces.registry_hash(),
            self.forces
                .active_ids()
                .into_iter()
                .map(|id| CommissioningForceId(id.0))
                .collect(),
            instance.runtime().force_overlay_hash(),
        )
        .map_err(commissioning_error)
    }

    fn context(&self, boundary: PublicationBoundary) -> Result<ObservationContext, SystemError> {
        ObservationContext::from_virtual_universe(&self.universe, self.binding()?, boundary)
            .map_err(|error| SystemError::Monitoring(format!("{error:?}")))
    }

    fn binding(&self) -> Result<SessionCommandBinding, SystemError> {
        self.universe
            .session_command_binding(self.ids.online)
            .map_err(commissioning_error)
    }

    fn runtime(&self) -> Result<&plc_runtime::VirtualController, SystemError> {
        self.universe
            .controller(self.ids.controller)
            .map(plc_commissioning::ControllerInstance::runtime)
            .ok_or_else(|| SystemError::Commissioning("runtime controller disappeared".to_owned()))
    }

    fn controller_state_hash(&self) -> Result<Hash32, SystemError> {
        self.universe
            .controller(self.ids.controller)
            .map(plc_commissioning::ControllerInstance::semantic_state_hash)
            .ok_or_else(|| SystemError::Commissioning("runtime controller disappeared".to_owned()))
    }

    fn command_boundary(&self) -> Result<PublicationBoundary, SystemError> {
        match self.runtime()?.cpu_state() {
            CpuState::Run => Ok(PublicationBoundary::ScanEnd),
            CpuState::Stop | CpuState::PausedEducational | CpuState::Faulted => {
                Ok(PublicationBoundary::SerializedCommand)
            }
            state => Err(SystemError::Monitoring(format!(
                "CPU state {state:?} does not admit an observation command"
            ))),
        }
    }

    fn refresh_online_comparison(&mut self) -> Result<(), SystemError> {
        if self
            .universe
            .session(self.ids.online)
            .is_some_and(|session| session.state() == SessionState::Online)
        {
            self.universe
                .observe_session(self.ids.online)
                .map_err(commissioning_error)?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn rebuild_navigation(&mut self) -> Result<(), SystemError> {
        let Some(build) = &self.build else {
            self.navigation = None;
            return Ok(());
        };
        self.navigation_revision = self.navigation_revision.saturating_add(1).max(1);
        let offline_fingerprint = build.runtime_artifact().fingerprint();
        let loaded_artifact =
            self.universe
                .controller(self.ids.controller)
                .and_then(|instance| {
                    instance.runtime().loaded_fingerprint().map(|fingerprint| {
                        LoadedArtifactBinding {
                            fingerprint,
                            controller_epoch: instance.runtime().controller_epoch(),
                        }
                    })
                });
        let mut builder = NavigationIndexBuilder::new(
            self.navigation_revision,
            offline_fingerprint,
            loaded_artifact,
        )
        .map_err(|error| SystemError::Navigation(format!("{error:?}")))?;
        let active_objects = self
            .project
            .objects()
            .filter(|object| object.lifecycle == Lifecycle::Active)
            .collect::<Vec<_>>();
        let known_identities = active_objects
            .iter()
            .map(|object| SemanticIdentity(object_u128(object.id)))
            .collect::<BTreeSet<_>>();
        for object in active_objects {
            let identity = SemanticIdentity(object_u128(object.id));
            let probe_target = build
                .probe_catalog()
                .definition(StableTargetId(identity.0))
                .map(|_| StableTargetId(identity.0));
            let domain_projection = probe_target.map_or(
                NavigationDomainProjection::ProjectObject {
                    object_identity: identity.0,
                },
                |target| NavigationDomainProjection::ProbeTarget { target },
            );
            builder
                .insert_anchor(NavigationAnchor {
                    identity,
                    kind: if probe_target.is_some() {
                        NavigationKind::ProbeTarget
                    } else {
                        NavigationKind::ProjectObject
                    },
                    side: ArtifactSide::CurrentOffline,
                    artifact_fingerprint: offline_fingerprint,
                    controller_epoch: None,
                    source: None,
                    domain_projection: Some(domain_projection),
                    probe_target,
                    relationship_kind: NavigationRelationshipKind::Selected,
                    validity: NavigationValidity::Valid,
                    tombstone_reason_hash: None,
                })
                .map_err(|error| SystemError::Navigation(format!("{error:?}")))?;
            if let Some(loaded) = loaded_artifact {
                builder
                    .insert_anchor(NavigationAnchor {
                        identity,
                        kind: if probe_target.is_some() {
                            NavigationKind::ProbeTarget
                        } else {
                            NavigationKind::ProjectObject
                        },
                        side: ArtifactSide::Loaded,
                        artifact_fingerprint: loaded.fingerprint,
                        controller_epoch: Some(loaded.controller_epoch),
                        source: None,
                        domain_projection: Some(domain_projection),
                        probe_target,
                        relationship_kind: NavigationRelationshipKind::Selected,
                        validity: NavigationValidity::Valid,
                        tombstone_reason_hash: None,
                    })
                    .map_err(|error| SystemError::Navigation(format!("{error:?}")))?;
            }
        }
        let fallback = SemanticIdentity(object_u128(self.controller_object_id));
        let resolve_runtime_source_identity = |candidate: u128| {
            let direct = SemanticIdentity(candidate);
            if known_identities.contains(&direct) {
                return Some(direct);
            }
            let source = build.runtime_projection().source_for(candidate)?;
            let mut owners = source
                .anchors
                .iter()
                .filter_map(|anchor| build.software().block_origin(anchor.owner_object_id))
                .map(|owner| SemanticIdentity(object_u128(owner)))
                .filter(|identity| known_identities.contains(identity))
                .collect::<BTreeSet<_>>()
                .into_iter();
            let owner = owners.next()?;
            owners.next().is_none().then_some(owner)
        };
        for event in self.diagnostics.retained_events() {
            let primary = event
                .condition_key
                .and_then(|key| resolve_runtime_source_identity(key.subject_identity))
                .unwrap_or(fallback);
            let mut related = event
                .related_identities
                .iter()
                .copied()
                .filter_map(resolve_runtime_source_identity)
                .filter(|identity| *identity != primary && known_identities.contains(identity))
                .collect::<Vec<_>>();
            related.sort_unstable();
            related.dedup();
            builder
                .route_diagnostic(event.occurrence_id.0, primary, related)
                .map_err(|error| SystemError::Navigation(format!("{error:?}")))?;
        }
        self.navigation = Some(
            builder
                .commit()
                .map_err(|error| SystemError::Navigation(format!("{error:?}")))?,
        );
        Ok(())
    }

    fn loaded(&self) -> bool {
        self.universe
            .controller(self.ids.controller)
            .is_some_and(|instance| instance.runtime().loaded_fingerprint().is_some())
    }
}

const fn runtime_value_target(target: RuntimeTarget) -> RuntimeValueTarget {
    match target {
        RuntimeTarget::Memory(id) => RuntimeValueTarget::Memory(id),
        RuntimeTarget::Input(id) => RuntimeValueTarget::Input(id),
        RuntimeTarget::Output(id) => RuntimeValueTarget::Output(id),
    }
}

const fn probe_layer(layer: CanonicalProbeLayer) -> ProbeLayer {
    match layer {
        CanonicalProbeLayer::Natural => ProbeLayer::Natural,
        CanonicalProbeLayer::Effective => ProbeLayer::Effective,
        CanonicalProbeLayer::RawInput => ProbeLayer::RawInput,
        CanonicalProbeLayer::CommittedOutput => ProbeLayer::CommittedOutput,
        CanonicalProbeLayer::DeliveredOutput => ProbeLayer::DeliveredOutput,
    }
}

const fn display_base(value: CanonicalDisplayBase) -> DisplayBase {
    match value {
        CanonicalDisplayBase::Automatic => DisplayBase::Automatic,
        CanonicalDisplayBase::Binary => DisplayBase::Binary,
        CanonicalDisplayBase::Decimal => DisplayBase::Decimal,
        CanonicalDisplayBase::Hexadecimal => DisplayBase::Hexadecimal,
    }
}

fn build_hardware_condition_engine(
    hardware: &CanonicalHardwareProjection,
    controller_object_id: ObjectId,
    controller_epoch: u64,
    commands: &[HardwareFaultCommand],
) -> Result<HardwareConditionEngine, SystemError> {
    let project = hardware.hardware_project();
    let controller = project
        .controllers()
        .values()
        .find(|controller| hardware.origin_for(controller.id.uuid()) == Some(controller_object_id))
        .ok_or_else(|| {
            SystemError::Hardware(
                "canonical controller is absent from runtime hardware configuration".to_owned(),
            )
        })?;
    let mut configuration = RuntimeHardwareConfiguration::new();
    configuration
        .add_device(controller.virtual_device_id, RuntimeDeviceRole::Controller)
        .map_err(|error| SystemError::Hardware(format!("{error:?}")))?;
    add_runtime_rack_modules(
        &mut configuration,
        &controller.local_rack,
        controller.virtual_device_id,
        None,
    )?;
    for station in project
        .stations()
        .values()
        .filter(|station| station.controller_id == controller.id)
    {
        configuration
            .add_device(station.virtual_device_id, RuntimeDeviceRole::Station)
            .map_err(|error| SystemError::Hardware(format!("{error:?}")))?;
        let required_link = link_for_device(project.network(), station.virtual_device_id);
        add_runtime_rack_modules(
            &mut configuration,
            &station.rack,
            station.virtual_device_id,
            required_link,
        )?;
    }
    let rebound = commands.iter().cloned().map(|mut command| {
        command.expected_controller_epoch = controller_epoch;
        command
    });
    HardwareConditionEngine::replay(
        hardware.profile().pin(),
        configuration,
        project.network().clone(),
        controller_epoch,
        rebound,
    )
    .map_err(|error| SystemError::Hardware(format!("{error:?}")))
}

fn add_runtime_rack_modules(
    configuration: &mut RuntimeHardwareConfiguration,
    rack: &plc_hardware::RackConfig,
    owner_device_id: plc_hardware::VirtualDeviceId,
    required_link_id: Option<plc_hardware::VirtualLinkId>,
) -> Result<(), SystemError> {
    for module in rack
        .slots
        .values()
        .filter_map(|slot| match &slot.installed {
            Some(InstalledOccupant::Module(module)) => Some(module),
            Some(InstalledOccupant::ControllerCore(_)) | None => None,
        })
    {
        // VLINK-2 is represented by the canonical interface itself. Current
        // Phase 2 project projection has no station authoring surface that can
        // bind its provider-module identity, so it is not duplicated here.
        if module.catalog_id == plc_hardware::ModuleCatalogId::Vlink2 {
            continue;
        }
        configuration
            .add_module(RuntimeModuleConfiguration {
                configured_module: module.clone(),
                owner_device_id,
                required_link_id,
            })
            .map_err(|error| SystemError::Hardware(format!("{error:?}")))?;
    }
    Ok(())
}

fn link_for_device(
    network: &plc_hardware::VirtualNetwork,
    device_id: plc_hardware::VirtualDeviceId,
) -> Option<plc_hardware::VirtualLinkId> {
    let interface_ids = network
        .interfaces()
        .values()
        .filter(|interface| interface.owner_device_id == device_id)
        .map(|interface| interface.id)
        .collect::<BTreeSet<_>>();
    let port_ids = network
        .ports()
        .values()
        .filter(|port| interface_ids.contains(&port.owner_interface_id))
        .map(|port| port.id)
        .collect::<BTreeSet<_>>();
    network
        .links()
        .values()
        .find(|link| {
            link.endpoint_port_ids
                .iter()
                .any(|port| port_ids.contains(port))
        })
        .map(|link| link.id)
}

fn initial_hardware_inputs(
    engine: &HardwareConditionEngine,
) -> BTreeMap<HardwareChannelId, ChannelRawValue> {
    engine
        .configuration()
        .modules()
        .values()
        .flat_map(|module| module.configured_module.channels.iter())
        .filter(|channel| channel.direction == HardwareChannelDirection::Input)
        .filter_map(|channel| {
            ChannelRawValue::canonical_default(channel.raw_type).map(|value| (channel.id, value))
        })
        .collect()
}

fn hardware_input_shape_valid(
    engine: &HardwareConditionEngine,
    inputs: &BTreeMap<HardwareChannelId, ChannelRawValue>,
) -> bool {
    let expected = engine
        .configuration()
        .modules()
        .values()
        .flat_map(|module| module.configured_module.channels.iter())
        .filter(|channel| channel.direction == HardwareChannelDirection::Input)
        .map(|channel| (channel.id, channel.raw_type))
        .collect::<BTreeMap<_, _>>();
    inputs.len() == expected.len()
        && inputs.iter().all(|(id, value)| {
            expected
                .get(id)
                .is_some_and(|raw_type| value.matches(*raw_type))
        })
}

fn hardware_condition_snapshot_hash(snapshot: &HardwareConditionSnapshot) -> Hash32 {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"PES-HARDWARE-CONDITION-SNAPSHOT-1\0");
    append_bytes(&mut bytes, snapshot.profile_pin.id.as_bytes());
    append_bytes(&mut bytes, snapshot.profile_pin.version.as_bytes());
    bytes.extend_from_slice(&snapshot.profile_pin.manifest_hash.0);
    bytes.extend_from_slice(&snapshot.controller_epoch.to_be_bytes());
    bytes.extend_from_slice(&snapshot.command_boundary.to_be_bytes());
    bytes.extend_from_slice(
        &u64::try_from(snapshot.module_states.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    for (id, state) in &snapshot.module_states {
        append_uuid(&mut bytes, id.uuid());
        bytes.push(match state {
            ModuleRuntimeState::ConfiguredPresent => 1,
            ModuleRuntimeState::Pulled => 2,
            ModuleRuntimeState::WrongCatalogInstalled => 3,
        });
    }
    append_uuid_set(
        &mut bytes,
        snapshot.channel_faults.iter().map(|id| id.uuid()),
    );
    append_uuid_set(&mut bytes, snapshot.wire_breaks.iter().map(|id| id.uuid()));
    append_bool_map(
        &mut bytes,
        snapshot
            .controller_powered
            .iter()
            .map(|(id, value)| (id.uuid(), *value)),
    );
    append_bool_map(
        &mut bytes,
        snapshot
            .station_available
            .iter()
            .map(|(id, value)| (id.uuid(), *value)),
    );
    append_bool_map(
        &mut bytes,
        snapshot
            .link_available
            .iter()
            .map(|(id, value)| (id.uuid(), *value)),
    );
    bytes.extend_from_slice(
        &u64::try_from(snapshot.active_conditions.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    for condition in &snapshot.active_conditions {
        append_hardware_condition(&mut bytes, *condition);
    }
    bytes.extend_from_slice(
        &u64::try_from(snapshot.condition_events.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    for event in &snapshot.condition_events {
        bytes.extend_from_slice(&event.sequence.to_be_bytes());
        bytes.extend_from_slice(&event.command_boundary.to_be_bytes());
        append_hardware_condition(&mut bytes, event.condition);
        bytes.push(match event.lifecycle {
            ConditionLifecycle::Activated => 1,
            ConditionLifecycle::Cleared => 2,
        });
        append_bytes(&mut bytes, event.diagnostic_code.stable_code().as_bytes());
    }
    bytes.extend_from_slice(&snapshot.network_state_fingerprint.0);
    bytes.extend_from_slice(&snapshot.state_fingerprint.0);
    hash32(sha256(&bytes))
}

fn hardware_command_log_hash(commands: &[HardwareFaultCommand]) -> Hash32 {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"PES-HARDWARE-COMMAND-LOG-1\0");
    bytes.extend_from_slice(
        &u64::try_from(commands.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    for command in commands {
        append_uuid(&mut bytes, command.idempotency_key);
        bytes.extend_from_slice(&command.expected_controller_epoch.to_be_bytes());
        append_hardware_action(&mut bytes, command.action);
    }
    hash32(sha256(&bytes))
}

fn hardware_natural_input_hash(inputs: &BTreeMap<HardwareChannelId, ChannelRawValue>) -> Hash32 {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"PES-HARDWARE-NATURAL-INPUTS-1\0");
    bytes.extend_from_slice(
        &u64::try_from(inputs.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    for (id, value) in inputs {
        append_uuid(&mut bytes, id.uuid());
        match value {
            ChannelRawValue::Bool(value) => {
                bytes.push(1);
                bytes.push(u8::from(*value));
            }
            ChannelRawValue::Int(value) => {
                bytes.push(2);
                bytes.extend_from_slice(&value.to_be_bytes());
            }
        }
    }
    hash32(sha256(&bytes))
}

fn append_hardware_condition(bytes: &mut Vec<u8>, condition: HardwareConditionKey) {
    let (tag, id) = match condition {
        HardwareConditionKey::ModuleNotPresent(id) => (1, id.uuid()),
        HardwareConditionKey::WrongModule(id) => (2, id.uuid()),
        HardwareConditionKey::ChannelFault(id) => (3, id.uuid()),
        HardwareConditionKey::WireBreak(id) => (4, id.uuid()),
        HardwareConditionKey::ControllerUnpowered(id) => (5, id.uuid()),
        HardwareConditionKey::StationUnavailable(id) => (6, id.uuid()),
        HardwareConditionKey::LinkUnavailable(id) => (7, id.uuid()),
    };
    bytes.push(tag);
    append_uuid(bytes, id);
}

fn append_hardware_action(bytes: &mut Vec<u8>, action: HardwareFaultAction) {
    match action {
        HardwareFaultAction::PullModule(id) => append_tagged_uuid(bytes, 1, id.uuid()),
        HardwareFaultAction::RestoreModule(id) => append_tagged_uuid(bytes, 2, id.uuid()),
        HardwareFaultAction::InstallWrongModule {
            module_id,
            installed_catalog,
        } => {
            append_tagged_uuid(bytes, 3, module_id.uuid());
            append_bytes(bytes, installed_catalog.as_str().as_bytes());
        }
        HardwareFaultAction::RestoreConfiguredModule(id) => {
            append_tagged_uuid(bytes, 4, id.uuid());
        }
        HardwareFaultAction::SetChannelFault(id) => append_tagged_uuid(bytes, 5, id.uuid()),
        HardwareFaultAction::ClearChannelFault(id) => append_tagged_uuid(bytes, 6, id.uuid()),
        HardwareFaultAction::SetWireBreak(id) => append_tagged_uuid(bytes, 7, id.uuid()),
        HardwareFaultAction::ClearWireBreak(id) => append_tagged_uuid(bytes, 8, id.uuid()),
        HardwareFaultAction::SetControllerPowered { device_id, powered } => {
            append_tagged_uuid(bytes, 9, device_id.uuid());
            bytes.push(u8::from(powered));
        }
        HardwareFaultAction::SetStationAvailable {
            device_id,
            available,
        } => {
            append_tagged_uuid(bytes, 10, device_id.uuid());
            bytes.push(u8::from(available));
        }
        HardwareFaultAction::SetVirtualLinkAvailable { link_id, available } => {
            append_tagged_uuid(bytes, 11, link_id.uuid());
            bytes.push(u8::from(available));
        }
    }
}

fn append_tagged_uuid(bytes: &mut Vec<u8>, tag: u8, id: Uuid) {
    bytes.push(tag);
    append_uuid(bytes, id);
}

fn append_uuid(bytes: &mut Vec<u8>, id: Uuid) {
    bytes.extend_from_slice(&id.into_bytes());
}

fn append_uuid_set(bytes: &mut Vec<u8>, ids: impl ExactSizeIterator<Item = Uuid>) {
    bytes.extend_from_slice(&u64::try_from(ids.len()).unwrap_or(u64::MAX).to_be_bytes());
    for id in ids {
        append_uuid(bytes, id);
    }
}

fn append_bool_map(bytes: &mut Vec<u8>, values: impl ExactSizeIterator<Item = (Uuid, bool)>) {
    bytes.extend_from_slice(
        &u64::try_from(values.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    for (id, value) in values {
        append_uuid(bytes, id);
        bytes.push(u8::from(value));
    }
}

fn append_bytes(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(&u64::try_from(value.len()).unwrap_or(u64::MAX).to_be_bytes());
    bytes.extend_from_slice(value);
}

const fn hardware_raw_from_runtime(value: CanonicalValue) -> Option<ChannelRawValue> {
    match value {
        CanonicalValue::Bool(value) => Some(ChannelRawValue::Bool(value)),
        CanonicalValue::I16(value) => Some(ChannelRawValue::Int(value)),
        _ => None,
    }
}

const fn runtime_value_from_hardware(value: ChannelRawValue) -> CanonicalValue {
    match value {
        ChannelRawValue::Bool(value) => CanonicalValue::Bool(value),
        ChannelRawValue::Int(value) => CanonicalValue::I16(value),
    }
}

const fn runtime_quality_from_hardware(quality: HardwareChannelQuality) -> plc_runtime::Quality {
    match quality {
        HardwareChannelQuality::Good => plc_runtime::Quality::Good,
        HardwareChannelQuality::Uncertain => plc_runtime::Quality::Uncertain,
        HardwareChannelQuality::Bad => plc_runtime::Quality::Bad,
        HardwareChannelQuality::NotPresent => plc_runtime::Quality::NotPresent,
    }
}

const fn observation_quality_from_hardware(quality: HardwareChannelQuality) -> Quality {
    match quality {
        HardwareChannelQuality::Good => Quality::Good,
        HardwareChannelQuality::Uncertain => Quality::Uncertain,
        HardwareChannelQuality::Bad => Quality::Bad,
        HardwareChannelQuality::NotPresent => Quality::NotPresent,
    }
}

const fn runtime_channel_id(target: RuntimeTarget) -> Option<plc_runtime::ChannelId> {
    match target {
        RuntimeTarget::Input(id) | RuntimeTarget::Output(id) => Some(id),
        RuntimeTarget::Memory(_) => None,
    }
}

fn hardware_projection_identity(
    command_id: u128,
    channel_id: HardwareChannelId,
    domain: &[u8],
) -> u128 {
    let mut bytes = Vec::with_capacity(domain.len() + 32);
    bytes.extend_from_slice(domain);
    bytes.extend_from_slice(&command_id.to_be_bytes());
    bytes.extend_from_slice(&channel_id.uuid().into_bytes());
    identity_from_hash(hash32(sha256(&bytes)))
}

fn hash_hardware_projection_set(causal_hashes: &[Sha256Digest]) -> Hash32 {
    let mut bytes = Vec::with_capacity(causal_hashes.len() * 32 + 40);
    bytes.extend_from_slice(b"PES-HARDWARE-PROJECTION-SET-1\0");
    bytes.extend_from_slice(
        &u64::try_from(causal_hashes.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    for hash in causal_hashes {
        bytes.extend_from_slice(&hash.0);
    }
    hash32(sha256(&bytes))
}

fn projected_trace_config(config: &crate::CanonicalTraceConfig) -> TraceConfig {
    TraceConfig {
        id: TraceConfigId(object_u128(config.object_id)),
        trigger_id: TraceTriggerId(derived_identity(object_u128(config.object_id), b"trigger")),
        name: config.name.clone(),
        channels: config
            .channels
            .iter()
            .map(|channel| TraceChannel {
                id: TraceChannelId(channel.id),
                alias: channel.alias.clone(),
                probe: TraceProbeKind::LoadedTarget {
                    target: TargetReference::Stable(StableTargetId(object_u128(
                        channel.target_tag,
                    ))),
                    layer: probe_layer(channel.layer),
                },
                display_unit: None,
            })
            .collect(),
        cadence: TraceCadence::EveryScans(config.every_scans),
        trigger: TraceTrigger::Immediate,
        pre_trigger_samples: config.pre_trigger_samples,
        post_trigger_samples: config.post_trigger_samples,
        post_trigger_duration_ms: None,
        maximum_duration_ms: config.maximum_duration_ms,
    }
}

#[allow(clippy::needless_pass_by_value)]
fn commissioning_error(error: CommissioningError) -> SystemError {
    SystemError::Commissioning(format!("{error:?}"))
}

const fn hash32(value: Sha256Digest) -> Hash32 {
    Hash32::from_bytes(value.0)
}

fn audit_hash(identity: SystemCommandIdentity, domain: &[u8]) -> Hash32 {
    let mut bytes = Vec::with_capacity(domain.len() + 48);
    bytes.extend_from_slice(domain);
    bytes.extend_from_slice(&identity.command_id.to_be_bytes());
    bytes.extend_from_slice(&identity.idempotency_key.to_be_bytes());
    bytes.extend_from_slice(&identity.author_identity.to_be_bytes());
    hash32(sha256(&bytes))
}

fn derived_identity(base: u128, domain: &[u8]) -> u128 {
    let mut bytes = Vec::with_capacity(domain.len() + 16);
    bytes.extend_from_slice(domain);
    bytes.extend_from_slice(&base.to_be_bytes());
    let hash = sha256(&bytes);
    let mut identity = [0_u8; 16];
    identity.copy_from_slice(&hash.0[..16]);
    u128::from_be_bytes(identity)
}

fn engineering_snapshot_hash(snapshot: &EngineeringSessionSnapshot) -> Hash32 {
    let mut bytes = Vec::with_capacity(32 * 11 + 128);
    bytes.extend_from_slice(b"PES-ENGINEERING-SNAPSHOT-1\0");
    bytes.extend_from_slice(&snapshot.schema_version.to_be_bytes());
    bytes.extend_from_slice(&snapshot.document_hash.0);
    bytes.extend_from_slice(&snapshot.semantic_fingerprint.0);
    bytes.extend_from_slice(&object_u128(snapshot.controller_object_id).to_be_bytes());
    bytes.extend_from_slice(snapshot.build_snapshot_hash.as_bytes());
    bytes.extend_from_slice(snapshot.loaded_artifact_fingerprint.as_bytes());
    bytes.extend_from_slice(snapshot.runtime_snapshot_hash.as_bytes());
    bytes.extend_from_slice(snapshot.monitoring_hash.as_bytes());
    bytes.push(snapshot.monitor_state as u8);
    bytes.extend_from_slice(snapshot.force_snapshot_hash.as_bytes());
    bytes.extend_from_slice(snapshot.trace_snapshot_hash.as_bytes());
    bytes.extend_from_slice(snapshot.diagnostic_snapshot_hash.as_bytes());
    bytes.extend_from_slice(snapshot.runtime_bridge_hash.as_bytes());
    bytes.extend_from_slice(snapshot.hardware_condition_hash.as_bytes());
    bytes.extend_from_slice(snapshot.hardware_command_hash.as_bytes());
    bytes.extend_from_slice(snapshot.hardware_input_hash.as_bytes());
    bytes.extend_from_slice(snapshot.hardware_diagnostic_bridge_hash.as_bytes());
    bytes.extend_from_slice(
        &u64::try_from(snapshot.trace_capture_ids.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    for id in &snapshot.trace_capture_ids {
        bytes.extend_from_slice(&id.0.to_be_bytes());
    }
    hash32(sha256(&bytes))
}

fn force_restore_deltas(before: &ForceRegistry, after: &ForceRegistry) -> Vec<ForceRestoreDelta> {
    let before = before
        .entries()
        .enumerate()
        .map(|(index, entry)| {
            (
                entry.id,
                (
                    u64::try_from(index).unwrap_or(u64::MAX).saturating_add(1),
                    entry,
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let after = after
        .entries()
        .enumerate()
        .map(|(index, entry)| {
            (
                entry.id,
                (
                    u64::try_from(index).unwrap_or(u64::MAX).saturating_add(1),
                    entry,
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let identities = before
        .keys()
        .chain(after.keys())
        .copied()
        .collect::<BTreeSet<_>>();
    identities
        .into_iter()
        .map(|force_id| {
            let old = before.get(&force_id).copied();
            let new = after.get(&force_id).copied();
            let mut classifications = match (old, new) {
                (Some((_, old)), Some((_, new))) if same_force_semantics(old, new) => {
                    vec![ForceRestoreClassification::Retained]
                }
                (Some(_), Some(_)) => vec![ForceRestoreClassification::Replaced],
                (None, Some(_)) => vec![ForceRestoreClassification::Added],
                (Some(_), None) => vec![ForceRestoreClassification::Removed],
                (None, None) => unreachable!("identity comes from one registry"),
            };
            if old.map(|(ordinal, _)| ordinal) != new.map(|(ordinal, _)| ordinal)
                && old.is_some()
                && new.is_some()
            {
                classifications.push(ForceRestoreClassification::Reordered);
            }
            if old.zip(new).is_some_and(|((_, old), (_, new))| {
                old.bound_universe_epoch != new.bound_universe_epoch
                    || old.bound_controller_epoch != new.bound_controller_epoch
            }) {
                classifications.push(ForceRestoreClassification::EpochRebound);
            }
            classifications.sort_unstable();
            ForceRestoreDelta {
                force_id,
                classifications,
                before_ordinal: old.map(|(ordinal, _)| ordinal),
                after_ordinal: new.map(|(ordinal, _)| ordinal),
                before: old.map(|(_, entry)| entry.clone()),
                after: new.map(|(_, entry)| entry.clone()),
            }
        })
        .collect()
}

fn same_force_semantics(left: &ForceEntry, right: &ForceEntry) -> bool {
    left.id == right.id
        && left.controller_id == right.controller_id
        && left.target_id == right.target_id
        && left.runtime_target == right.runtime_target
        && left.instance_path == right.instance_path
        && left.bit_range == right.bit_range
        && left.value_type == right.value_type
        && left.value == right.value
        && left.natural_at_application == right.natural_at_application
        && left.target_layer == right.target_layer
        && left.underlying_quality == right.underlying_quality
        && left.quality_warning == right.quality_warning
        && left.activation_boundary == right.activation_boundary
        && left.status == right.status
        && left.created_universe_epoch == right.created_universe_epoch
        && left.created_controller_epoch == right.created_controller_epoch
        && left.created_session_epoch == right.created_session_epoch
        && left.artifact_fingerprint == right.artifact_fingerprint
        && left.actor_identity == right.actor_identity
        && left.reason == right.reason
        && left.audit_context_hash == right.audit_context_hash
}

fn hash_force_restore_deltas(
    current_registry_hash: Hash32,
    snapshot_registry_hash: Hash32,
    planned_registry_hash: Hash32,
    deltas: &[ForceRestoreDelta],
) -> Hash32 {
    let mut bytes = Vec::with_capacity(32 * 3 + deltas.len() * 96 + 32);
    bytes.extend_from_slice(b"PES-FORCE-RESTORE-DELTA-1\0");
    bytes.extend_from_slice(current_registry_hash.as_bytes());
    bytes.extend_from_slice(snapshot_registry_hash.as_bytes());
    bytes.extend_from_slice(planned_registry_hash.as_bytes());
    bytes.extend_from_slice(
        &u64::try_from(deltas.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    for delta in deltas {
        bytes.extend_from_slice(&delta.force_id.0.to_be_bytes());
        bytes.extend_from_slice(
            &u64::try_from(delta.classifications.len())
                .unwrap_or(u64::MAX)
                .to_be_bytes(),
        );
        bytes.extend(delta.classifications.iter().map(|value| *value as u8));
        encode_optional_ordinal(delta.before_ordinal, &mut bytes);
        encode_optional_ordinal(delta.after_ordinal, &mut bytes);
        encode_optional_force_hash(delta.before.as_ref(), &mut bytes);
        encode_optional_force_hash(delta.after.as_ref(), &mut bytes);
    }
    hash32(sha256(&bytes))
}

fn encode_optional_ordinal(value: Option<u64>, output: &mut Vec<u8>) {
    output.push(u8::from(value.is_some()));
    if let Some(value) = value {
        output.extend_from_slice(&value.to_be_bytes());
    }
}

fn encode_optional_force_hash(value: Option<&ForceEntry>, output: &mut Vec<u8>) {
    output.push(u8::from(value.is_some()));
    if let Some(value) = value {
        output.extend_from_slice(value.entry_hash.as_bytes());
    }
}

fn hash_restore_preview(preview: &RestorePreview) -> Hash32 {
    let mut bytes = Vec::with_capacity(32 * 7 + 64);
    bytes.extend_from_slice(b"PES-RESTORE-PREVIEW-1\0");
    bytes.extend_from_slice(preview.snapshot_content_hash.as_bytes());
    bytes.extend_from_slice(&preview.expected_universe_epoch.to_be_bytes());
    bytes.extend_from_slice(&preview.expected_controller_epoch.to_be_bytes());
    bytes.extend_from_slice(preview.expected_current_state_hash.as_bytes());
    bytes.extend_from_slice(preview.current_force_registry_hash.as_bytes());
    bytes.extend_from_slice(preview.snapshot_force_registry_hash.as_bytes());
    bytes.extend_from_slice(preview.planned_force_registry_hash.as_bytes());
    bytes.extend_from_slice(preview.force_delta_hash.as_bytes());
    bytes.extend_from_slice(&preview.planned_universe_epoch.to_be_bytes());
    bytes.extend_from_slice(&preview.planned_controller_epoch.to_be_bytes());
    bytes.push(preview.safe_mode as u8);
    hash32(sha256(&bytes))
}

fn identity_from_hash(hash: Hash32) -> u128 {
    let mut identity = [0_u8; 16];
    identity.copy_from_slice(&hash.as_bytes()[..16]);
    u128::from_be_bytes(identity)
}

const fn identity_seed(identity: u128) -> u64 {
    let bytes = identity.to_be_bytes();
    u64::from_be_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ])
}
