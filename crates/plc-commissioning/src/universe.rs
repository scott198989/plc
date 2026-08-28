use alloc::{collections::BTreeMap, vec::Vec};
use core::{error::Error, fmt};

use plc_runtime::{
    AtomicInstallError, CommandError as RuntimeCommandError, ControllerSnapshot, CpuState, Hash32,
    InputCommand, InputReceipt, MemoryId, RestartKind, RunOutcome, RuntimeBoundaryCommand,
    RuntimeBoundaryError, RuntimeBoundaryReceipt, RuntimeCloneReport, RuntimeForceResetApproval,
    RuntimeInstallDisposition, RuntimeLifecycleError, RuntimeReplacementReport, RuntimeScanCommand,
    RuntimeScanReceipt, RuntimeStateTransferPlan, SnapshotError, StateId, UniverseId,
    VirtualController, VirtualControllerId,
};

use crate::{
    canonical::{CanonicalHasher, id_from_hash},
    model::{
        ActualHardwareState, AvailabilityComparison, ComparisonVector, CompatibilityClass, ForceId,
        HardwareComparison, LoadBlocker, LoadPackageError, LoadPreview, LoadRequest,
        MemoryActionKind, MonitoringComparison, OfflineControllerId, OfflineEngineeringState,
        PackageComparison, PostLoadMode, PreviewApproval, ProfileComparison, SessionCommandBinding,
        SessionError, SessionState, StateActionKind, VirtualLoadPackage, VirtualOnlineSession,
        VirtualOnlineSessionId, build_memory_actions, build_state_actions, package_compatibility,
    },
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ControllerInstanceId(pub u128);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForceRegistryProjection {
    expected_previous_registry_hash: Hash32,
    next_registry_hash: Hash32,
    active_force_ids: Vec<ForceId>,
    expected_runtime_overlay_hash: Hash32,
}

impl ForceRegistryProjection {
    pub fn new(
        expected_previous_registry_hash: Hash32,
        next_registry_hash: Hash32,
        active_force_ids: Vec<ForceId>,
        expected_runtime_overlay_hash: Hash32,
    ) -> Result<Self, CommissioningError> {
        if active_force_ids.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(CommissioningError::ForceProjectionNotCanonical);
        }
        Ok(Self {
            expected_previous_registry_hash,
            next_registry_hash,
            active_force_ids,
            expected_runtime_overlay_hash,
        })
    }

    pub const fn expected_previous_registry_hash(&self) -> Hash32 {
        self.expected_previous_registry_hash
    }

    pub const fn next_registry_hash(&self) -> Hash32 {
        self.next_registry_hash
    }

    pub fn active_force_ids(&self) -> &[ForceId] {
        &self.active_force_ids
    }

    pub const fn expected_runtime_overlay_hash(&self) -> Hash32 {
        self.expected_runtime_overlay_hash
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommissionedBoundaryReceipt {
    pub runtime: RuntimeBoundaryReceipt,
    pub force_registry_hash: Hash32,
    pub controller_state_hash: Hash32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommissionedScanReceipt {
    pub runtime: RuntimeScanReceipt,
    pub force_registry_hash: Hash32,
    pub controller_state_hash: Hash32,
}

#[derive(Clone, Debug)]
pub struct ControllerInstance {
    instance_id: ControllerInstanceId,
    offline_controller_id: OfflineControllerId,
    runtime: VirtualController,
    loaded_package: Option<VirtualLoadPackage>,
    actual_hardware: ActualHardwareState,
    active_force_ids: Vec<ForceId>,
    force_registry_hash: Hash32,
}

impl ControllerInstance {
    pub const fn instance_id(&self) -> ControllerInstanceId {
        self.instance_id
    }

    pub const fn offline_controller_id(&self) -> OfflineControllerId {
        self.offline_controller_id
    }

    pub const fn runtime(&self) -> &VirtualController {
        &self.runtime
    }

    pub fn loaded_package(&self) -> Option<&VirtualLoadPackage> {
        self.loaded_package.as_ref()
    }

    pub const fn actual_hardware(&self) -> &ActualHardwareState {
        &self.actual_hardware
    }

    pub fn active_force_ids(&self) -> &[ForceId] {
        &self.active_force_ids
    }

    pub const fn force_registry_hash(&self) -> Hash32 {
        self.force_registry_hash
    }

    pub fn semantic_state_hash(&self) -> Hash32 {
        let mut hasher = CanonicalHasher::new("PES-COMMISSIONED-CONTROLLER-1");
        hasher.u128(self.instance_id.0);
        hasher.u128(self.offline_controller_id.0);
        hasher.hash(self.runtime.semantic_state_hash());
        match &self.loaded_package {
            Some(package) => {
                hasher.bool(true);
                hasher.hash(package.fingerprint());
            }
            None => hasher.bool(false),
        }
        hasher.hash(self.actual_hardware.fingerprint);
        hasher.bool(self.actual_hardware.present);
        hasher.hash(self.actual_hardware.fault_state_hash);
        hasher.hash(self.force_registry_hash);
        hasher.u64(self.active_force_ids.len() as u64);
        for force in &self.active_force_ids {
            hasher.u128(force.0);
        }
        hasher.finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CreateInstanceCommand {
    pub command_id: u128,
    pub instance_id: ControllerInstanceId,
    pub offline_controller_id: OfflineControllerId,
    pub controller_id: VirtualControllerId,
    pub deterministic_seed: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CloneInstanceCommand {
    pub command_id: u128,
    pub source_controller_id: VirtualControllerId,
    pub clone_instance_id: ControllerInstanceId,
    pub clone_controller_id: VirtualControllerId,
    pub expected_universe_epoch: u64,
    pub expected_source_controller_epoch: u64,
    pub expected_source_state_hash: Hash32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloneInstanceResult {
    pub source_controller_id: VirtualControllerId,
    pub clone_controller_id: VirtualControllerId,
    pub clone_instance_id: ControllerInstanceId,
    pub runtime: RuntimeCloneReport,
    pub clone_state_hash: Hash32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ResetInstanceKind {
    SimulatedPowerCycle = 1,
    MemoryReset = 2,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResetInstancePreview {
    pub preview_id: u128,
    pub target_controller_id: VirtualControllerId,
    pub universe_epoch: u64,
    pub controller_epoch: u64,
    pub target_state_hash: Hash32,
    pub kind: ResetInstanceKind,
    pub current_cpu_state: CpuState,
    pub final_cpu_state: CpuState,
    pub loaded_package_fingerprint: Hash32,
    pub reset_member_ids: Vec<u128>,
    pub preserved_member_ids: Vec<u128>,
    pub reset_state_ids: Vec<u128>,
    pub preserved_state_ids: Vec<u128>,
    pub cleared_force_ids: Vec<ForceId>,
    pub invalidated_session_ids: Vec<VirtualOnlineSessionId>,
    pub next_controller_epoch: u64,
    pub preview_hash: Hash32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResetInstanceApproval {
    pub preview_id: u128,
    pub preview_hash: Hash32,
    pub target_controller_id: VirtualControllerId,
    pub universe_epoch: u64,
    pub controller_epoch: u64,
    pub target_state_hash: Hash32,
    pub kind: ResetInstanceKind,
}

impl ResetInstanceApproval {
    pub fn approve(preview: &ResetInstancePreview) -> Self {
        Self {
            preview_id: preview.preview_id,
            preview_hash: preview.preview_hash,
            target_controller_id: preview.target_controller_id,
            universe_epoch: preview.universe_epoch,
            controller_epoch: preview.controller_epoch,
            target_state_hash: preview.target_state_hash,
            kind: preview.kind,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplaceInstanceCommand {
    pub command_id: u128,
    pub target_controller_id: VirtualControllerId,
    pub replacement_instance_id: ControllerInstanceId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplaceInstancePreview {
    pub preview_id: u128,
    pub target_controller_id: VirtualControllerId,
    pub current_instance_id: ControllerInstanceId,
    pub replacement_instance_id: ControllerInstanceId,
    pub universe_epoch: u64,
    pub controller_epoch: u64,
    pub target_state_hash: Hash32,
    pub loaded_package_fingerprint: Option<Hash32>,
    pub removed_member_ids: Vec<u128>,
    pub removed_state_ids: Vec<u128>,
    pub cleared_force_ids: Vec<ForceId>,
    pub invalidated_session_ids: Vec<VirtualOnlineSessionId>,
    pub next_controller_epoch: u64,
    pub final_cpu_state: CpuState,
    pub preview_hash: Hash32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplaceInstanceApproval {
    pub preview_id: u128,
    pub preview_hash: Hash32,
    pub target_controller_id: VirtualControllerId,
    pub current_instance_id: ControllerInstanceId,
    pub replacement_instance_id: ControllerInstanceId,
    pub universe_epoch: u64,
    pub controller_epoch: u64,
    pub target_state_hash: Hash32,
}

impl ReplaceInstanceApproval {
    pub fn approve(preview: &ReplaceInstancePreview) -> Self {
        Self {
            preview_id: preview.preview_id,
            preview_hash: preview.preview_hash,
            target_controller_id: preview.target_controller_id,
            current_instance_id: preview.current_instance_id,
            replacement_instance_id: preview.replacement_instance_id,
            universe_epoch: preview.universe_epoch,
            controller_epoch: preview.controller_epoch,
            target_state_hash: preview.target_state_hash,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleRollback {
    NotRequired,
    ExactPreStateRestored,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResetInstanceResult {
    pub preview_id: u128,
    pub kind: ResetInstanceKind,
    pub old_controller_epoch: u64,
    pub new_controller_epoch: u64,
    pub final_cpu_state: CpuState,
    pub cleared_force_ids: Vec<ForceId>,
    pub rollback: LifecycleRollback,
    pub pre_state_hash: Hash32,
    pub post_state_hash: Hash32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplaceInstanceResult {
    pub preview_id: u128,
    pub old_instance_id: ControllerInstanceId,
    pub new_instance_id: ControllerInstanceId,
    pub runtime: RuntimeReplacementReport,
    pub removed_package_fingerprint: Option<Hash32>,
    pub cleared_force_ids: Vec<ForceId>,
    pub rollback: LifecycleRollback,
    pub pre_state_hash: Hash32,
    pub post_state_hash: Hash32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActualHardwareFaultCommand {
    pub command_id: u128,
    pub target_controller_id: VirtualControllerId,
    pub expected_universe_epoch: u64,
    pub expected_controller_epoch: u64,
    pub expected_target_state_hash: Hash32,
    pub present: bool,
    pub fault_state_hash: Hash32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoveInstancePreview {
    pub preview_id: u128,
    pub target_controller_id: VirtualControllerId,
    pub controller_epoch: u64,
    pub target_state_hash: Hash32,
    pub loaded_package_fingerprint: Option<Hash32>,
    pub active_force_ids: Vec<ForceId>,
    pub invalidated_session_ids: Vec<VirtualOnlineSessionId>,
    pub preview_hash: Hash32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RemoveInstanceApproval {
    pub preview_id: u128,
    pub preview_hash: Hash32,
    pub target_controller_id: VirtualControllerId,
    pub controller_epoch: u64,
    pub target_state_hash: Hash32,
}

impl RemoveInstanceApproval {
    pub fn approve(preview: &RemoveInstancePreview) -> Self {
        Self {
            preview_id: preview.preview_id,
            preview_hash: preview.preview_hash,
            target_controller_id: preview.target_controller_id,
            controller_epoch: preview.controller_epoch,
            target_state_hash: preview.target_state_hash,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum InternalFailurePoint {
    None = 0,
    AfterValidation = 1,
    AfterRuntimeStage = 2,
    AfterCommitSwap = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoadExecution {
    pub failure_point: InternalFailurePoint,
}

pub type LifecycleExecution = LoadExecution;

impl Default for LoadExecution {
    fn default() -> Self {
        Self {
            failure_point: InternalFailurePoint::None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadRollback {
    NotRequired,
    ExactPreStateRestored,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadResult {
    pub preview_id: crate::PreviewId,
    pub old_package_fingerprint: Option<Hash32>,
    pub new_package_fingerprint: Hash32,
    pub compatibility: CompatibilityClass,
    pub preserved_member_ids: Vec<u128>,
    pub initialized_member_ids: Vec<u128>,
    pub removed_member_ids: Vec<u128>,
    pub preserved_state_ids: Vec<u128>,
    pub initialized_state_ids: Vec<u128>,
    pub removed_state_ids: Vec<u128>,
    pub final_cpu_state: CpuState,
    pub comparison: ComparisonVector,
    pub controller_epoch: u64,
    pub rollback: LoadRollback,
    pub pre_state_hash: Hash32,
    pub post_state_hash: Hash32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CommissioningAuditKind {
    InstanceCreated = 1,
    InstanceRemoved = 2,
    LoadCommitted = 3,
    LoadFailed = 4,
    SessionChanged = 5,
    CpuCommand = 6,
    InstanceCloned = 7,
    InstanceReplaced = 8,
    InstanceReset = 9,
    ActualHardwareStateChanged = 10,
    ObservationCommand = 11,
    VirtualInputChanged = 12,
    ControllerSnapshotRestored = 13,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommissioningAuditEvent {
    pub event_sequence: u64,
    pub kind: CommissioningAuditKind,
    pub controller_id: Option<VirtualControllerId>,
    pub preview_id: Option<crate::PreviewId>,
    pub success: bool,
    pub pre_state_hash: Option<Hash32>,
    pub post_state_hash: Option<Hash32>,
    pub internal_failure_point: InternalFailurePoint,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommissioningError {
    DuplicateOfflineController(OfflineControllerId),
    UnknownOfflineController(OfflineControllerId),
    DuplicateInstance(ControllerInstanceId),
    DuplicateController(VirtualControllerId),
    UnknownController(VirtualControllerId),
    Package(LoadPackageError),
    Runtime(RuntimeCommandError),
    AtomicRuntime(AtomicInstallError),
    RuntimeLifecycle(RuntimeLifecycleError),
    RuntimeBoundary(RuntimeBoundaryError),
    Snapshot(SnapshotError),
    ForceProjectionNotCanonical,
    ForceRegistryStateChanged {
        expected: Hash32,
        actual: Hash32,
    },
    ForceRuntimeProjectionMismatch {
        expected: Hash32,
        actual: Hash32,
    },
    PreviewBlocked(Vec<LoadBlocker>),
    PreviewExpired,
    ApprovalMismatch,
    PreviewStateChanged,
    CandidateChanged,
    InjectedInternalFailure(InternalFailurePoint),
    LoadRolledBack {
        failure_point: InternalFailurePoint,
        pre_state_hash: Hash32,
        restored_state_hash: Hash32,
    },
    LifecycleResetDisallowed {
        kind: ResetInstanceKind,
        cpu_state: CpuState,
    },
    LifecycleRequiresLoadedPackage,
    RequiredVirtualHardwareInvalid {
        controller_id: VirtualControllerId,
        configured_fingerprint: Hash32,
        actual_fingerprint: Hash32,
        present: bool,
        fault_state_hash: Hash32,
    },
    ReplacementInstanceIdentityUnchanged,
    ControllerEpochExhausted,
    LifecycleRolledBack {
        failure_point: InternalFailurePoint,
        pre_state_hash: Hash32,
        restored_state_hash: Hash32,
    },
    Session(SessionError),
}

impl fmt::Display for CommissioningError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "commissioning action rejected: {self:?}")
    }
}

impl Error for CommissioningError {}

impl From<LoadPackageError> for CommissioningError {
    fn from(value: LoadPackageError) -> Self {
        Self::Package(value)
    }
}

impl From<RuntimeCommandError> for CommissioningError {
    fn from(value: RuntimeCommandError) -> Self {
        Self::Runtime(value)
    }
}

impl From<AtomicInstallError> for CommissioningError {
    fn from(value: AtomicInstallError) -> Self {
        Self::AtomicRuntime(value)
    }
}

impl From<RuntimeLifecycleError> for CommissioningError {
    fn from(value: RuntimeLifecycleError) -> Self {
        Self::RuntimeLifecycle(value)
    }
}

impl From<RuntimeBoundaryError> for CommissioningError {
    fn from(value: RuntimeBoundaryError) -> Self {
        Self::RuntimeBoundary(value)
    }
}

impl From<SnapshotError> for CommissioningError {
    fn from(value: SnapshotError) -> Self {
        Self::Snapshot(value)
    }
}

impl From<SessionError> for CommissioningError {
    fn from(value: SessionError) -> Self {
        Self::Session(value)
    }
}

#[derive(Clone, Debug)]
pub struct VirtualUniverse {
    universe_id: UniverseId,
    universe_epoch: u64,
    event_sequence: u64,
    offline: BTreeMap<OfflineControllerId, OfflineEngineeringState>,
    controllers: BTreeMap<VirtualControllerId, ControllerInstance>,
    sessions: BTreeMap<VirtualOnlineSessionId, VirtualOnlineSession>,
    session_offline: BTreeMap<VirtualOnlineSessionId, OfflineControllerId>,
    audit: Vec<CommissioningAuditEvent>,
}

impl VirtualUniverse {
    pub fn new(universe_id: UniverseId) -> Self {
        Self {
            universe_id,
            universe_epoch: 1,
            event_sequence: 0,
            offline: BTreeMap::new(),
            controllers: BTreeMap::new(),
            sessions: BTreeMap::new(),
            session_offline: BTreeMap::new(),
            audit: Vec::new(),
        }
    }

    pub const fn universe_id(&self) -> UniverseId {
        self.universe_id
    }

    pub const fn universe_epoch(&self) -> u64 {
        self.universe_epoch
    }

    pub const fn event_sequence(&self) -> u64 {
        self.event_sequence
    }

    pub fn audit(&self) -> &[CommissioningAuditEvent] {
        &self.audit
    }

    pub fn semantic_state_hash(&self) -> Hash32 {
        let mut hasher = CanonicalHasher::new("PES-VIRTUAL-UNIVERSE-1");
        hasher.u128(self.universe_id.0);
        hasher.u64(self.universe_epoch);
        hasher.u64(self.event_sequence);
        hasher.u64(self.offline.len() as u64);
        for (id, offline) in &self.offline {
            hasher.u128(id.0);
            hasher.hash(offline.configured.configured_hardware_fingerprint);
            hasher.hash(offline.configured.profile_fingerprint);
            hasher.hash(offline.source_revision_hash);
            match offline.build_snapshot_hash {
                Some(value) => {
                    hasher.bool(true);
                    hasher.hash(value);
                }
                None => hasher.bool(false),
            }
            hasher.bool(offline.project_saved);
            hasher.u8(offline.source_to_build as u8);
            hasher.bool(offline.software_build_current);
            hasher.bool(offline.hardware_build_current);
            match offline.current_package_fingerprint {
                Some(value) => {
                    hasher.bool(true);
                    hasher.hash(value);
                }
                None => hasher.bool(false),
            }
        }
        hasher.u64(self.controllers.len() as u64);
        for (id, controller) in &self.controllers {
            hasher.u128(id.0);
            hasher.hash(controller.semantic_state_hash());
        }
        hasher.u64(self.sessions.len() as u64);
        for (id, session) in &self.sessions {
            hasher.u128(id.0);
            hasher.u8(session.state as u8);
            hasher.u64(session.universe_epoch);
            hasher.u128(session.controller_id.0);
            match session.controller_epoch {
                Some(value) => {
                    hasher.bool(true);
                    hasher.u64(value);
                }
                None => hasher.bool(false),
            }
            hasher.u64(session.session_epoch);
            session.comparison.encode(&mut hasher);
        }
        hasher.finish()
    }

    pub fn register_offline_controller(
        &mut self,
        state: OfflineEngineeringState,
    ) -> Result<(), CommissioningError> {
        if self.offline.contains_key(&state.configured.id) {
            return Err(CommissioningError::DuplicateOfflineController(
                state.configured.id,
            ));
        }
        self.offline.insert(state.configured.id, state);
        Ok(())
    }

    pub fn offline_controller(&self, id: OfflineControllerId) -> Option<&OfflineEngineeringState> {
        self.offline.get(&id)
    }

    pub fn offline_controller_mut(
        &mut self,
        id: OfflineControllerId,
    ) -> Option<&mut OfflineEngineeringState> {
        self.offline.get_mut(&id)
    }

    pub fn create_instance(
        &mut self,
        command: CreateInstanceCommand,
    ) -> Result<(), CommissioningError> {
        let offline = self.offline.get(&command.offline_controller_id).ok_or(
            CommissioningError::UnknownOfflineController(command.offline_controller_id),
        )?;
        if self
            .controllers
            .values()
            .any(|instance| instance.instance_id == command.instance_id)
        {
            return Err(CommissioningError::DuplicateInstance(command.instance_id));
        }
        if self.controllers.contains_key(&command.controller_id) {
            return Err(CommissioningError::DuplicateController(
                command.controller_id,
            ));
        }
        let instance = ControllerInstance {
            instance_id: command.instance_id,
            offline_controller_id: command.offline_controller_id,
            runtime: VirtualController::new(
                self.universe_id,
                command.controller_id,
                command.deterministic_seed,
            ),
            loaded_package: None,
            actual_hardware: ActualHardwareState {
                fingerprint: offline.configured.configured_hardware_fingerprint,
                present: true,
                fault_state_hash: Hash32::ZERO,
            },
            active_force_ids: Vec::new(),
            force_registry_hash: empty_force_registry_hash(),
        };
        self.controllers.insert(command.controller_id, instance);
        let sequence = self.next_event();
        self.audit.push(CommissioningAuditEvent {
            event_sequence: sequence,
            kind: CommissioningAuditKind::InstanceCreated,
            controller_id: Some(command.controller_id),
            preview_id: None,
            success: true,
            pre_state_hash: None,
            post_state_hash: self
                .controllers
                .get(&command.controller_id)
                .map(ControllerInstance::semantic_state_hash),
            internal_failure_point: InternalFailurePoint::None,
        });
        Ok(())
    }

    pub fn clone_instance(
        &mut self,
        command: CloneInstanceCommand,
    ) -> Result<CloneInstanceResult, CommissioningError> {
        if command.expected_universe_epoch != self.universe_epoch {
            return Err(CommissioningError::PreviewStateChanged);
        }
        if self
            .controllers
            .values()
            .any(|instance| instance.instance_id == command.clone_instance_id)
        {
            return Err(CommissioningError::DuplicateInstance(
                command.clone_instance_id,
            ));
        }
        if self.controllers.contains_key(&command.clone_controller_id) {
            return Err(CommissioningError::DuplicateController(
                command.clone_controller_id,
            ));
        }

        let source = self.controllers.get(&command.source_controller_id).ok_or(
            CommissioningError::UnknownController(command.source_controller_id),
        )?;
        if source.runtime.controller_epoch() != command.expected_source_controller_epoch
            || source.semantic_state_hash() != command.expected_source_state_hash
        {
            return Err(CommissioningError::PreviewStateChanged);
        }
        let runtime_stage = source
            .runtime
            .stage_reidentified_clone(command.clone_controller_id)?;
        let (runtime, runtime_report) = runtime_stage.commit(&source.runtime)?;
        let clone = ControllerInstance {
            instance_id: command.clone_instance_id,
            offline_controller_id: source.offline_controller_id,
            runtime,
            loaded_package: source.loaded_package.clone(),
            actual_hardware: source.actual_hardware.clone(),
            active_force_ids: source.active_force_ids.clone(),
            force_registry_hash: source.force_registry_hash,
        };
        let clone_state_hash = clone.semantic_state_hash();
        self.controllers.insert(command.clone_controller_id, clone);
        let sequence = self.next_event();
        self.audit.push(CommissioningAuditEvent {
            event_sequence: sequence,
            kind: CommissioningAuditKind::InstanceCloned,
            controller_id: Some(command.clone_controller_id),
            preview_id: None,
            success: true,
            pre_state_hash: Some(command.expected_source_state_hash),
            post_state_hash: Some(clone_state_hash),
            internal_failure_point: InternalFailurePoint::None,
        });
        Ok(CloneInstanceResult {
            source_controller_id: command.source_controller_id,
            clone_controller_id: command.clone_controller_id,
            clone_instance_id: command.clone_instance_id,
            runtime: runtime_report,
            clone_state_hash,
        })
    }

    pub fn controller(&self, id: VirtualControllerId) -> Option<&ControllerInstance> {
        self.controllers.get(&id)
    }

    pub fn actual_hardware_matches_configured(
        &self,
        id: VirtualControllerId,
    ) -> Result<bool, CommissioningError> {
        let instance = self
            .controllers
            .get(&id)
            .ok_or(CommissioningError::UnknownController(id))?;
        let offline = self.offline.get(&instance.offline_controller_id).ok_or(
            CommissioningError::UnknownOfflineController(instance.offline_controller_id),
        )?;
        Ok(instance.actual_hardware.present
            && instance.actual_hardware.fault_state_hash == Hash32::ZERO
            && instance.actual_hardware.fingerprint
                == offline.configured.configured_hardware_fingerprint)
    }

    pub fn apply_actual_hardware_fault(
        &mut self,
        command: ActualHardwareFaultCommand,
    ) -> Result<(), CommissioningError> {
        if command.expected_universe_epoch != self.universe_epoch {
            return Err(CommissioningError::PreviewStateChanged);
        }
        let instance = self
            .controllers
            .get_mut(&command.target_controller_id)
            .ok_or(CommissioningError::UnknownController(
                command.target_controller_id,
            ))?;
        if instance.runtime.controller_epoch() != command.expected_controller_epoch
            || instance.semantic_state_hash() != command.expected_target_state_hash
        {
            return Err(CommissioningError::PreviewStateChanged);
        }
        let pre_state_hash = instance.semantic_state_hash();
        instance.actual_hardware.present = command.present;
        instance.actual_hardware.fault_state_hash = command.fault_state_hash;
        let post_state_hash = instance.semantic_state_hash();
        self.refresh_sessions_for_target(command.target_controller_id);
        let sequence = self.next_event();
        self.audit.push(CommissioningAuditEvent {
            event_sequence: sequence,
            kind: CommissioningAuditKind::ActualHardwareStateChanged,
            controller_id: Some(command.target_controller_id),
            preview_id: None,
            success: true,
            pre_state_hash: Some(pre_state_hash),
            post_state_hash: Some(post_state_hash),
            internal_failure_point: InternalFailurePoint::None,
        });
        Ok(())
    }

    pub fn power_on(&mut self, id: VirtualControllerId) -> Result<(), CommissioningError> {
        let instance = self
            .controllers
            .get_mut(&id)
            .ok_or(CommissioningError::UnknownController(id))?;
        instance.runtime.power_on()?;
        Ok(())
    }

    pub fn power_off(&mut self, id: VirtualControllerId) -> Result<(), CommissioningError> {
        let instance = self
            .controllers
            .get_mut(&id)
            .ok_or(CommissioningError::UnknownController(id))?;
        instance.runtime.power_off()?;
        self.refresh_sessions_for_target(id);
        self.audit_cpu_command(id);
        Ok(())
    }

    pub fn prepare_reset_instance(
        &self,
        id: VirtualControllerId,
        kind: ResetInstanceKind,
    ) -> Result<ResetInstancePreview, CommissioningError> {
        let instance = self
            .controllers
            .get(&id)
            .ok_or(CommissioningError::UnknownController(id))?;
        let cpu_state = instance.runtime.cpu_state();
        let allowed = match kind {
            ResetInstanceKind::SimulatedPowerCycle => matches!(
                cpu_state,
                CpuState::Stop | CpuState::Faulted | CpuState::PoweredOff
            ),
            ResetInstanceKind::MemoryReset => matches!(
                cpu_state,
                CpuState::Stop | CpuState::PausedEducational | CpuState::Faulted
            ),
        };
        if !allowed {
            return Err(CommissioningError::LifecycleResetDisallowed { kind, cpu_state });
        }
        let package = instance
            .loaded_package
            .as_ref()
            .ok_or(CommissioningError::LifecycleRequiresLoadedPackage)?;
        let next_controller_epoch = instance
            .runtime
            .controller_epoch()
            .checked_add(1)
            .ok_or(CommissioningError::ControllerEpochExhausted)?;

        let mut reset_member_ids = Vec::new();
        let mut preserved_member_ids = Vec::new();
        for member in package.memory_schema() {
            if kind == ResetInstanceKind::SimulatedPowerCycle && member.retentive {
                preserved_member_ids.push(member.member_id);
            } else {
                reset_member_ids.push(member.member_id);
            }
        }
        let mut reset_state_ids = Vec::new();
        let mut preserved_state_ids = Vec::new();
        for state in package.state_schema() {
            if kind == ResetInstanceKind::SimulatedPowerCycle && state.retentive {
                preserved_state_ids.push(state.state_member_id);
            } else {
                reset_state_ids.push(state.state_member_id);
            }
        }
        let invalidated_session_ids = self
            .sessions
            .values()
            .filter(|session| {
                session.controller_id == id
                    && session.state == SessionState::Online
                    && session.controller_epoch == Some(instance.runtime.controller_epoch())
            })
            .map(|session| session.id)
            .collect::<Vec<_>>();
        let mut preview = ResetInstancePreview {
            preview_id: 0,
            target_controller_id: id,
            universe_epoch: self.universe_epoch,
            controller_epoch: instance.runtime.controller_epoch(),
            target_state_hash: instance.semantic_state_hash(),
            kind,
            current_cpu_state: cpu_state,
            final_cpu_state: CpuState::Stop,
            loaded_package_fingerprint: package.fingerprint(),
            reset_member_ids,
            preserved_member_ids,
            reset_state_ids,
            preserved_state_ids,
            cleared_force_ids: instance.active_force_ids.clone(),
            invalidated_session_ids,
            next_controller_epoch,
            preview_hash: Hash32::ZERO,
        };
        preview.preview_hash = hash_reset_preview(&preview);
        preview.preview_id = id_from_hash(preview.preview_hash);
        preview.preview_hash = hash_reset_preview(&preview);
        Ok(preview)
    }

    pub fn reset_instance(
        &mut self,
        preview: &ResetInstancePreview,
        approval: ResetInstanceApproval,
        execution: LifecycleExecution,
    ) -> Result<ResetInstanceResult, CommissioningError> {
        if preview.preview_hash != hash_reset_preview(preview)
            || approval.preview_id != preview.preview_id
            || approval.preview_hash != preview.preview_hash
            || approval.target_controller_id != preview.target_controller_id
            || approval.universe_epoch != preview.universe_epoch
            || approval.controller_epoch != preview.controller_epoch
            || approval.target_state_hash != preview.target_state_hash
            || approval.kind != preview.kind
        {
            return Err(CommissioningError::ApprovalMismatch);
        }
        let instance = self.controllers.get(&preview.target_controller_id).ok_or(
            CommissioningError::UnknownController(preview.target_controller_id),
        )?;
        if self.universe_epoch != preview.universe_epoch
            || instance.runtime.controller_epoch() != preview.controller_epoch
            || instance.runtime.cpu_state() != preview.current_cpu_state
            || instance.semantic_state_hash() != preview.target_state_hash
            || instance
                .loaded_package
                .as_ref()
                .map(VirtualLoadPackage::fingerprint)
                != Some(preview.loaded_package_fingerprint)
            || instance.active_force_ids != preview.cleared_force_ids
        {
            return Err(CommissioningError::PreviewStateChanged);
        }
        let pre_state_hash = instance.semantic_state_hash();
        if execution.failure_point == InternalFailurePoint::AfterValidation {
            return self.record_lifecycle_rollback(
                preview.target_controller_id,
                CommissioningAuditKind::InstanceReset,
                execution.failure_point,
                pre_state_hash,
            );
        }

        let backup = instance.clone();
        let old_controller_epoch = backup.runtime.controller_epoch();
        let mut staged = backup.clone();
        if !staged.runtime.force_overlays().is_empty() {
            let approval = RuntimeForceResetApproval {
                controller_id: staged.runtime.controller_id(),
                expected_controller_epoch: staged.runtime.controller_epoch(),
                expected_artifact_fingerprint: staged
                    .runtime
                    .loaded_fingerprint()
                    .ok_or(CommissioningError::LifecycleRequiresLoadedPackage)?,
                expected_force_overlay_hash: staged.runtime.force_overlay_hash(),
            };
            staged.runtime.clear_force_overlays_for_reset(approval)?;
        }
        staged.active_force_ids.clear();
        staged.force_registry_hash = empty_force_registry_hash();
        match preview.kind {
            ResetInstanceKind::SimulatedPowerCycle => staged.runtime.simulated_power_cycle()?,
            ResetInstanceKind::MemoryReset => staged.runtime.memory_reset()?,
        }
        if execution.failure_point == InternalFailurePoint::AfterRuntimeStage {
            return self.record_lifecycle_rollback(
                preview.target_controller_id,
                CommissioningAuditKind::InstanceReset,
                execution.failure_point,
                pre_state_hash,
            );
        }

        let new_controller_epoch = staged.runtime.controller_epoch();
        let final_cpu_state = staged.runtime.cpu_state();
        let post_state_hash = staged.semantic_state_hash();
        self.controllers
            .insert(preview.target_controller_id, staged);
        if execution.failure_point == InternalFailurePoint::AfterCommitSwap {
            self.controllers
                .insert(preview.target_controller_id, backup);
            return self.record_lifecycle_rollback(
                preview.target_controller_id,
                CommissioningAuditKind::InstanceReset,
                execution.failure_point,
                pre_state_hash,
            );
        }

        self.mark_sessions_lost_for_epoch_change(
            preview.target_controller_id,
            old_controller_epoch,
        );
        let sequence = self.next_event();
        self.audit.push(CommissioningAuditEvent {
            event_sequence: sequence,
            kind: CommissioningAuditKind::InstanceReset,
            controller_id: Some(preview.target_controller_id),
            preview_id: None,
            success: true,
            pre_state_hash: Some(pre_state_hash),
            post_state_hash: Some(post_state_hash),
            internal_failure_point: InternalFailurePoint::None,
        });
        Ok(ResetInstanceResult {
            preview_id: preview.preview_id,
            kind: preview.kind,
            old_controller_epoch,
            new_controller_epoch,
            final_cpu_state,
            cleared_force_ids: preview.cleared_force_ids.clone(),
            rollback: LifecycleRollback::NotRequired,
            pre_state_hash,
            post_state_hash,
        })
    }

    pub fn prepare_replace_instance(
        &self,
        command: ReplaceInstanceCommand,
    ) -> Result<ReplaceInstancePreview, CommissioningError> {
        let instance = self.controllers.get(&command.target_controller_id).ok_or(
            CommissioningError::UnknownController(command.target_controller_id),
        )?;
        if command.replacement_instance_id == instance.instance_id {
            return Err(CommissioningError::ReplacementInstanceIdentityUnchanged);
        }
        if self.controllers.values().any(|candidate| {
            candidate.instance_id == command.replacement_instance_id
                && candidate.instance_id != instance.instance_id
        }) {
            return Err(CommissioningError::DuplicateInstance(
                command.replacement_instance_id,
            ));
        }
        let runtime_stage = instance.runtime.stage_blank_replacement()?;
        let loaded_package_fingerprint = instance
            .loaded_package
            .as_ref()
            .map(VirtualLoadPackage::fingerprint);
        let removed_member_ids = instance
            .loaded_package
            .as_ref()
            .map(|package| {
                package
                    .memory_schema()
                    .iter()
                    .map(|member| member.member_id)
                    .collect()
            })
            .unwrap_or_default();
        let removed_state_ids = instance
            .loaded_package
            .as_ref()
            .map(|package| {
                package
                    .state_schema()
                    .iter()
                    .map(|state| state.state_member_id)
                    .collect()
            })
            .unwrap_or_default();
        let invalidated_session_ids = self
            .sessions
            .values()
            .filter(|session| {
                session.controller_id == command.target_controller_id
                    && session.state == SessionState::Online
                    && session.controller_epoch == Some(instance.runtime.controller_epoch())
            })
            .map(|session| session.id)
            .collect();
        let mut preview = ReplaceInstancePreview {
            preview_id: 0,
            target_controller_id: command.target_controller_id,
            current_instance_id: instance.instance_id,
            replacement_instance_id: command.replacement_instance_id,
            universe_epoch: self.universe_epoch,
            controller_epoch: instance.runtime.controller_epoch(),
            target_state_hash: instance.semantic_state_hash(),
            loaded_package_fingerprint,
            removed_member_ids,
            removed_state_ids,
            cleared_force_ids: instance.active_force_ids.clone(),
            invalidated_session_ids,
            next_controller_epoch: runtime_stage.report().new_controller_epoch,
            final_cpu_state: CpuState::PoweredOff,
            preview_hash: Hash32::ZERO,
        };
        preview.preview_hash = hash_replace_preview(&preview);
        preview.preview_id = id_from_hash(preview.preview_hash);
        preview.preview_hash = hash_replace_preview(&preview);
        Ok(preview)
    }

    pub fn replace_instance(
        &mut self,
        preview: &ReplaceInstancePreview,
        approval: ReplaceInstanceApproval,
        execution: LifecycleExecution,
    ) -> Result<ReplaceInstanceResult, CommissioningError> {
        if preview.preview_hash != hash_replace_preview(preview)
            || approval.preview_id != preview.preview_id
            || approval.preview_hash != preview.preview_hash
            || approval.target_controller_id != preview.target_controller_id
            || approval.current_instance_id != preview.current_instance_id
            || approval.replacement_instance_id != preview.replacement_instance_id
            || approval.universe_epoch != preview.universe_epoch
            || approval.controller_epoch != preview.controller_epoch
            || approval.target_state_hash != preview.target_state_hash
        {
            return Err(CommissioningError::ApprovalMismatch);
        }
        let instance = self.controllers.get(&preview.target_controller_id).ok_or(
            CommissioningError::UnknownController(preview.target_controller_id),
        )?;
        if self.universe_epoch != preview.universe_epoch
            || instance.instance_id != preview.current_instance_id
            || instance.runtime.controller_epoch() != preview.controller_epoch
            || instance.semantic_state_hash() != preview.target_state_hash
            || instance.active_force_ids != preview.cleared_force_ids
        {
            return Err(CommissioningError::PreviewStateChanged);
        }
        if self.controllers.values().any(|candidate| {
            candidate.instance_id == preview.replacement_instance_id
                && candidate.instance_id != preview.current_instance_id
        }) {
            return Err(CommissioningError::DuplicateInstance(
                preview.replacement_instance_id,
            ));
        }
        let pre_state_hash = instance.semantic_state_hash();
        if execution.failure_point == InternalFailurePoint::AfterValidation {
            return self.record_lifecycle_rollback(
                preview.target_controller_id,
                CommissioningAuditKind::InstanceReplaced,
                execution.failure_point,
                pre_state_hash,
            );
        }

        let backup = instance.clone();
        let old_controller_epoch = backup.runtime.controller_epoch();
        let offline_id = backup.offline_controller_id;
        let configured_hardware_fingerprint = self
            .offline
            .get(&offline_id)
            .ok_or(CommissioningError::UnknownOfflineController(offline_id))?
            .configured
            .configured_hardware_fingerprint;
        let mut staged = backup.clone();
        let runtime_stage = staged.runtime.stage_blank_replacement()?;
        let runtime_report = runtime_stage.commit(&mut staged.runtime)?;
        staged.instance_id = preview.replacement_instance_id;
        staged.loaded_package = None;
        staged.actual_hardware = ActualHardwareState {
            fingerprint: configured_hardware_fingerprint,
            present: true,
            fault_state_hash: Hash32::ZERO,
        };
        staged.active_force_ids.clear();
        staged.force_registry_hash = empty_force_registry_hash();
        if execution.failure_point == InternalFailurePoint::AfterRuntimeStage {
            return self.record_lifecycle_rollback(
                preview.target_controller_id,
                CommissioningAuditKind::InstanceReplaced,
                execution.failure_point,
                pre_state_hash,
            );
        }

        let post_state_hash = staged.semantic_state_hash();
        self.controllers
            .insert(preview.target_controller_id, staged);
        if execution.failure_point == InternalFailurePoint::AfterCommitSwap {
            self.controllers
                .insert(preview.target_controller_id, backup);
            return self.record_lifecycle_rollback(
                preview.target_controller_id,
                CommissioningAuditKind::InstanceReplaced,
                execution.failure_point,
                pre_state_hash,
            );
        }
        self.mark_sessions_lost_for_epoch_change(
            preview.target_controller_id,
            old_controller_epoch,
        );
        let sequence = self.next_event();
        self.audit.push(CommissioningAuditEvent {
            event_sequence: sequence,
            kind: CommissioningAuditKind::InstanceReplaced,
            controller_id: Some(preview.target_controller_id),
            preview_id: None,
            success: true,
            pre_state_hash: Some(pre_state_hash),
            post_state_hash: Some(post_state_hash),
            internal_failure_point: InternalFailurePoint::None,
        });
        Ok(ReplaceInstanceResult {
            preview_id: preview.preview_id,
            old_instance_id: preview.current_instance_id,
            new_instance_id: preview.replacement_instance_id,
            runtime: runtime_report,
            removed_package_fingerprint: preview.loaded_package_fingerprint,
            cleared_force_ids: preview.cleared_force_ids.clone(),
            rollback: LifecycleRollback::NotRequired,
            pre_state_hash,
            post_state_hash,
        })
    }

    pub fn prepare_remove_instance(
        &self,
        id: VirtualControllerId,
    ) -> Result<RemoveInstancePreview, CommissioningError> {
        let instance = self
            .controllers
            .get(&id)
            .ok_or(CommissioningError::UnknownController(id))?;
        let target_state_hash = instance.semantic_state_hash();
        let invalidated_session_ids = self
            .sessions
            .values()
            .filter(|session| session.controller_id == id && session.state != SessionState::Closed)
            .map(|session| session.id)
            .collect::<Vec<_>>();
        let mut preview = RemoveInstancePreview {
            preview_id: 0,
            target_controller_id: id,
            controller_epoch: instance.runtime.controller_epoch(),
            target_state_hash,
            loaded_package_fingerprint: instance
                .loaded_package
                .as_ref()
                .map(VirtualLoadPackage::fingerprint),
            active_force_ids: instance.active_force_ids.clone(),
            invalidated_session_ids,
            preview_hash: Hash32::ZERO,
        };
        preview.preview_hash = hash_remove_preview(&preview);
        preview.preview_id = id_from_hash(preview.preview_hash);
        Ok(preview)
    }

    pub fn remove_instance(
        &mut self,
        preview: &RemoveInstancePreview,
        approval: RemoveInstanceApproval,
    ) -> Result<ControllerInstance, CommissioningError> {
        if preview.preview_hash != hash_remove_preview(preview)
            || approval.preview_id != preview.preview_id
            || approval.preview_hash != preview.preview_hash
            || approval.target_controller_id != preview.target_controller_id
            || approval.controller_epoch != preview.controller_epoch
            || approval.target_state_hash != preview.target_state_hash
        {
            return Err(CommissioningError::ApprovalMismatch);
        }
        let instance = self.controllers.get(&preview.target_controller_id).ok_or(
            CommissioningError::UnknownController(preview.target_controller_id),
        )?;
        if instance.runtime.controller_epoch() != preview.controller_epoch
            || instance.semantic_state_hash() != preview.target_state_hash
        {
            return Err(CommissioningError::PreviewStateChanged);
        }
        let removed = self
            .controllers
            .remove(&preview.target_controller_id)
            .ok_or(CommissioningError::UnknownController(
                preview.target_controller_id,
            ))?;
        for session in self.sessions.values_mut() {
            if session.controller_id == preview.target_controller_id
                && session.state != SessionState::Closed
            {
                session.state = SessionState::VirtualLinkLost;
                session.session_epoch = session.session_epoch.saturating_add(1);
                session.comparison.availability = AvailabilityComparison::Lost;
                session.comparison.monitoring = MonitoringComparison::Stale;
            }
        }
        let sequence = self.next_event();
        self.audit.push(CommissioningAuditEvent {
            event_sequence: sequence,
            kind: CommissioningAuditKind::InstanceRemoved,
            controller_id: Some(preview.target_controller_id),
            preview_id: None,
            success: true,
            pre_state_hash: Some(preview.target_state_hash),
            post_state_hash: None,
            internal_failure_point: InternalFailurePoint::None,
        });
        Ok(removed)
    }

    pub fn prepare_load(
        &self,
        target: VirtualControllerId,
        candidate: &VirtualLoadPackage,
        request: LoadRequest,
    ) -> Result<LoadPreview, CommissioningError> {
        let instance = self
            .controllers
            .get(&target)
            .ok_or(CommissioningError::UnknownController(target))?;
        let offline = self.offline.get(&instance.offline_controller_id).ok_or(
            CommissioningError::UnknownOfflineController(instance.offline_controller_id),
        )?;
        let package_valid = candidate.validate().is_ok();
        let compatibility = if package_valid {
            package_compatibility(instance.loaded_package.as_ref(), candidate)
        } else {
            CompatibilityClass::Incompatible
        };
        let memory_actions = if package_valid {
            build_memory_actions(
                instance.loaded_package.as_ref(),
                candidate,
                request.initialize_compatible_members,
            )
        } else {
            Vec::new()
        };
        let state_actions = if package_valid {
            build_state_actions(instance.loaded_package.as_ref(), candidate)
        } else {
            Vec::new()
        };
        let artifact_change = compatibility != CompatibilityClass::Identical;
        let hardware_replacement = matches!(compatibility, CompatibilityClass::HardwareChanging);
        let requires_stop = artifact_change
            && (request.initialize_compatible_members
                || request.requested_post_load_mode == PostLoadMode::Stop
                || matches!(
                    compatibility,
                    CompatibilityClass::Initial
                        | CompatibilityClass::MemorySchemaChanging
                        | CompatibilityClass::StatefulSchemaChanging
                        | CompatibilityClass::HardwareChanging
                        | CompatibilityClass::Incompatible
                ));
        let mut blockers = Vec::new();
        if !package_valid {
            blockers.push(LoadBlocker::CandidateIntegrity);
        }
        if !candidate.build_is_current()
            || !offline.software_build_current
            || offline.source_to_build != crate::OfflineSourceBuild::Current
        {
            blockers.push(LoadBlocker::CandidateBuildStale);
        }
        if candidate.blocking_diagnostic_count() != 0 {
            blockers.push(LoadBlocker::CandidateHasBlockingDiagnostics(
                candidate.blocking_diagnostic_count(),
            ));
        }
        if candidate.build_snapshot_hash() != request.expected_build_snapshot_hash
            || offline.build_snapshot_hash != Some(candidate.build_snapshot_hash())
            || offline.current_package_fingerprint != Some(candidate.fingerprint())
        {
            blockers.push(LoadBlocker::BuildSnapshotMismatch);
        }
        if candidate.profile_fingerprint() != offline.configured.profile_fingerprint {
            blockers.push(LoadBlocker::IncompatibleProfile);
        }
        if compatibility == CompatibilityClass::Incompatible {
            blockers.push(LoadBlocker::IncompatibleRuntimeContract);
        }
        if !matches!(
            instance.runtime.cpu_state(),
            CpuState::Stop | CpuState::Run | CpuState::PausedEducational
        ) {
            blockers.push(LoadBlocker::CpuMode(instance.runtime.cpu_state()));
        }
        if artifact_change && !instance.active_force_ids.is_empty() {
            blockers.push(LoadBlocker::ActiveForces(instance.active_force_ids.clone()));
        }

        let mut preview = LoadPreview {
            id: crate::PreviewId(0),
            preview_hash: Hash32::ZERO,
            target_controller_id: target,
            current_package_fingerprint: instance
                .loaded_package
                .as_ref()
                .map(VirtualLoadPackage::fingerprint),
            candidate_package_fingerprint: candidate.fingerprint(),
            current_runtime_fingerprint: instance.runtime.loaded_fingerprint(),
            candidate_runtime_fingerprint: candidate.runtime_artifact().fingerprint(),
            target_state_hash: instance.semantic_state_hash(),
            controller_epoch: instance.runtime.controller_epoch(),
            universe_epoch: self.universe_epoch,
            cpu_state: instance.runtime.cpu_state(),
            compatibility,
            requires_stop,
            memory_actions,
            state_actions,
            hardware_replacement,
            force_registry_hash: instance.force_registry_hash,
            actual_hardware_hash: actual_hardware_hash(&instance.actual_hardware),
            offline_source_revision_hash: offline.source_revision_hash,
            offline_build_snapshot_hash: offline.build_snapshot_hash,
            snapshot_invalidation: artifact_change,
            opens_new_replay_segment: artifact_change,
            blockers,
            warnings: Vec::new(),
            requested_post_load_mode: request.requested_post_load_mode,
            initialize_compatible_members: request.initialize_compatible_members,
            valid_through_event_sequence: request.valid_through_event_sequence,
        };
        preview.finalize_hash();
        Ok(preview)
    }

    pub fn commit_load(
        &mut self,
        preview: &LoadPreview,
        approval: PreviewApproval,
        candidate: &VirtualLoadPackage,
        execution: LoadExecution,
    ) -> Result<LoadResult, CommissioningError> {
        candidate.validate()?;
        if !preview.blockers.is_empty() {
            return Err(CommissioningError::PreviewBlocked(preview.blockers.clone()));
        }
        if preview.recompute_hash() != preview.preview_hash {
            return Err(CommissioningError::ApprovalMismatch);
        }
        if approval.preview_id != preview.id
            || approval.preview_hash != preview.preview_hash
            || approval.target_controller_id != preview.target_controller_id
            || approval.controller_epoch != preview.controller_epoch
            || approval.target_state_hash != preview.target_state_hash
            || approval.candidate_package_fingerprint != preview.candidate_package_fingerprint
            || approval.force_registry_hash != preview.force_registry_hash
            || approval.requested_post_load_mode != preview.requested_post_load_mode
        {
            return Err(CommissioningError::ApprovalMismatch);
        }
        if candidate.fingerprint() != preview.candidate_package_fingerprint
            || candidate.runtime_artifact().fingerprint() != preview.candidate_runtime_fingerprint
        {
            return Err(CommissioningError::CandidateChanged);
        }
        if self.event_sequence > preview.valid_through_event_sequence {
            return Err(CommissioningError::PreviewExpired);
        }

        let instance = self.controllers.get(&preview.target_controller_id).ok_or(
            CommissioningError::UnknownController(preview.target_controller_id),
        )?;
        let offline = self.offline.get(&instance.offline_controller_id).ok_or(
            CommissioningError::UnknownOfflineController(instance.offline_controller_id),
        )?;
        if self.universe_epoch != preview.universe_epoch
            || instance.runtime.controller_epoch() != preview.controller_epoch
            || instance.runtime.cpu_state() != preview.cpu_state
            || instance.semantic_state_hash() != preview.target_state_hash
            || instance.force_registry_hash != preview.force_registry_hash
            || actual_hardware_hash(&instance.actual_hardware) != preview.actual_hardware_hash
            || instance
                .loaded_package
                .as_ref()
                .map(VirtualLoadPackage::fingerprint)
                != preview.current_package_fingerprint
            || instance.runtime.loaded_fingerprint() != preview.current_runtime_fingerprint
            || offline.source_revision_hash != preview.offline_source_revision_hash
            || offline.build_snapshot_hash != preview.offline_build_snapshot_hash
        {
            return Err(CommissioningError::PreviewStateChanged);
        }

        let pre_state_hash = instance.semantic_state_hash();
        if execution.failure_point == InternalFailurePoint::AfterValidation {
            return self.record_load_rollback(preview, execution.failure_point, pre_state_hash);
        }
        if preview.compatibility == CompatibilityClass::Identical {
            let comparison = comparison_for(
                offline,
                Some(instance),
                AvailabilityComparison::Available,
                MonitoringComparison::Inactive,
            );
            let result = LoadResult {
                preview_id: preview.id,
                old_package_fingerprint: preview.current_package_fingerprint,
                new_package_fingerprint: candidate.fingerprint(),
                compatibility: preview.compatibility,
                preserved_member_ids: preview
                    .memory_actions
                    .iter()
                    .filter(|action| action.kind == MemoryActionKind::Preserve)
                    .map(|action| action.member_id)
                    .collect(),
                initialized_member_ids: Vec::new(),
                removed_member_ids: Vec::new(),
                preserved_state_ids: preview
                    .state_actions
                    .iter()
                    .filter(|action| action.kind == StateActionKind::Preserve)
                    .map(|action| action.state_member_id)
                    .collect(),
                initialized_state_ids: Vec::new(),
                removed_state_ids: Vec::new(),
                final_cpu_state: instance.runtime.cpu_state(),
                comparison,
                controller_epoch: instance.runtime.controller_epoch(),
                rollback: LoadRollback::NotRequired,
                pre_state_hash,
                post_state_hash: pre_state_hash,
            };
            return Ok(result);
        }

        let backup = instance.clone();
        let old_controller_epoch = backup.runtime.controller_epoch();
        let mut staged = backup.clone();
        if preview.requires_stop
            && matches!(
                staged.runtime.cpu_state(),
                CpuState::Run | CpuState::PausedEducational
            )
        {
            staged.runtime.request_stop()?;
        }

        if let Some(current_package) = backup.loaded_package.as_ref() {
            let preserve_memory: Vec<MemoryId> = preview
                .memory_actions
                .iter()
                .filter(|action| action.kind == MemoryActionKind::Preserve)
                .map(|action| action.runtime_memory_id)
                .collect();
            let preserve_states: Vec<StateId> = preview
                .state_actions
                .iter()
                .filter(|action| action.kind == StateActionKind::Preserve)
                .map(|action| action.runtime_state_id)
                .collect();
            let disposition = if preview.compatibility == CompatibilityClass::PackageIdentityOnly {
                RuntimeInstallDisposition::PackageIdentityOnly
            } else {
                RuntimeInstallDisposition::ArtifactReplacement
            };
            let preserve_io = !preview.hardware_replacement
                && current_package.runtime_artifact().spec().channels
                    == candidate.runtime_artifact().spec().channels;
            let preserve_cpu_mode =
                !preview.requires_stop && preview.requested_post_load_mode != PostLoadMode::Stop;
            let transfer = RuntimeStateTransferPlan::new(
                current_package.runtime_artifact().fingerprint(),
                candidate.runtime_artifact().fingerprint(),
                disposition,
                preserve_memory,
                preserve_states,
                preserve_io,
                preserve_cpu_mode,
            )?;
            let runtime_stage = staged
                .runtime
                .stage_atomic_install(candidate.runtime_artifact(), &transfer)?;
            runtime_stage.commit(&mut staged.runtime)?;
        } else {
            staged
                .runtime
                .install_verified_artifact(candidate.runtime_artifact())?;
        }
        staged.loaded_package = Some(candidate.clone());
        staged.actual_hardware = ActualHardwareState {
            fingerprint: candidate.hardware_fingerprint(),
            present: true,
            fault_state_hash: Hash32::ZERO,
        };

        if execution.failure_point == InternalFailurePoint::AfterRuntimeStage {
            return self.record_load_rollback(preview, execution.failure_point, pre_state_hash);
        }

        match preview.requested_post_load_mode {
            PostLoadMode::Preserve => {}
            PostLoadMode::Stop => {
                if matches!(
                    staged.runtime.cpu_state(),
                    CpuState::Run | CpuState::PausedEducational
                ) {
                    staged.runtime.request_stop()?;
                }
            }
            PostLoadMode::Run
                if !matches!(
                    staged.runtime.cpu_state(),
                    CpuState::Stop | CpuState::PausedEducational | CpuState::Run
                ) =>
            {
                return Err(CommissioningError::Runtime(
                    RuntimeCommandError::IllegalCpuTransition {
                        from: staged.runtime.cpu_state(),
                        action: "Post-load RUN",
                    },
                ));
            }
            PostLoadMode::Run => {}
        }

        let offline_id = staged.offline_controller_id;
        let controller_epoch = staged.runtime.controller_epoch();
        self.controllers
            .insert(preview.target_controller_id, staged);

        if execution.failure_point == InternalFailurePoint::AfterCommitSwap {
            self.controllers
                .insert(preview.target_controller_id, backup.clone());
            return self.record_load_rollback(preview, execution.failure_point, pre_state_hash);
        }

        if preview.requested_post_load_mode == PostLoadMode::Run {
            let committed = self
                .controllers
                .get_mut(&preview.target_controller_id)
                .ok_or(CommissioningError::UnknownController(
                    preview.target_controller_id,
                ))?;
            match committed.runtime.cpu_state() {
                CpuState::Stop => committed.runtime.request_run(RestartKind::Resume)?,
                CpuState::PausedEducational => committed.runtime.resume_educational()?,
                CpuState::Run => {}
                _ => unreachable!("post-load RUN state was validated before atomic commit"),
            }
        }

        let committed = self.controllers.get(&preview.target_controller_id).ok_or(
            CommissioningError::UnknownController(preview.target_controller_id),
        )?;
        let post_state_hash = committed.semantic_state_hash();
        let final_cpu_state = committed.runtime.cpu_state();

        if controller_epoch != old_controller_epoch {
            self.mark_sessions_lost_for_epoch_change(
                preview.target_controller_id,
                old_controller_epoch,
            );
        }
        let offline = self
            .offline
            .get(&offline_id)
            .ok_or(CommissioningError::UnknownOfflineController(offline_id))?;
        let committed = self.controllers.get(&preview.target_controller_id).ok_or(
            CommissioningError::UnknownController(preview.target_controller_id),
        )?;
        let comparison = comparison_for(
            offline,
            Some(committed),
            AvailabilityComparison::Available,
            MonitoringComparison::Inactive,
        );
        let result = build_load_result(
            preview,
            candidate,
            final_cpu_state,
            comparison,
            controller_epoch,
            pre_state_hash,
            post_state_hash,
        );
        let sequence = self.next_event();
        self.audit.push(CommissioningAuditEvent {
            event_sequence: sequence,
            kind: CommissioningAuditKind::LoadCommitted,
            controller_id: Some(preview.target_controller_id),
            preview_id: Some(preview.id),
            success: true,
            pre_state_hash: Some(pre_state_hash),
            post_state_hash: Some(post_state_hash),
            internal_failure_point: InternalFailurePoint::None,
        });
        Ok(result)
    }

    pub fn begin_go_online(
        &mut self,
        session_id: VirtualOnlineSessionId,
        offline_controller_id: OfflineControllerId,
        target_controller_id: VirtualControllerId,
    ) -> Result<(), CommissioningError> {
        if self.sessions.contains_key(&session_id) {
            return Err(SessionError::DuplicateSession(session_id).into());
        }
        let offline = self.offline.get(&offline_controller_id).ok_or(
            CommissioningError::UnknownOfflineController(offline_controller_id),
        )?;
        let comparison = comparison_for(
            offline,
            self.controllers.get(&target_controller_id),
            if self.controllers.contains_key(&target_controller_id) {
                AvailabilityComparison::Available
            } else {
                AvailabilityComparison::Unavailable
            },
            MonitoringComparison::Inactive,
        );
        self.sessions.insert(
            session_id,
            VirtualOnlineSession {
                id: session_id,
                state: SessionState::Opening,
                universe_id: self.universe_id,
                universe_epoch: self.universe_epoch,
                controller_id: target_controller_id,
                controller_epoch: None,
                session_epoch: 1,
                loaded_package_fingerprint: None,
                comparison,
            },
        );
        self.session_offline
            .insert(session_id, offline_controller_id);
        Ok(())
    }

    pub fn complete_go_online(
        &mut self,
        session_id: VirtualOnlineSessionId,
    ) -> Result<(), CommissioningError> {
        let state = self
            .sessions
            .get(&session_id)
            .ok_or(SessionError::UnknownSession(session_id))?
            .state;
        if state != SessionState::Opening {
            return Err(SessionError::IllegalTransition {
                from: state,
                action: "Complete Go Online",
            }
            .into());
        }
        self.bind_session(session_id, false)?;
        self.audit_session_change(session_id);
        Ok(())
    }

    pub fn session(&self, session_id: VirtualOnlineSessionId) -> Option<&VirtualOnlineSession> {
        self.sessions.get(&session_id)
    }

    pub fn observe_session(
        &mut self,
        session_id: VirtualOnlineSessionId,
    ) -> Result<(), CommissioningError> {
        let session = self
            .sessions
            .get(&session_id)
            .ok_or(SessionError::UnknownSession(session_id))?;
        if session.state != SessionState::Online {
            return Err(SessionError::NotOnline(session.state).into());
        }
        let lost = session.universe_epoch != self.universe_epoch
            || self
                .controllers
                .get(&session.controller_id)
                .map(|instance| {
                    Some(instance.runtime.controller_epoch()) != session.controller_epoch
                })
                .unwrap_or(true);
        if lost {
            let session = self
                .sessions
                .get_mut(&session_id)
                .ok_or(SessionError::UnknownSession(session_id))?;
            session.state = SessionState::VirtualLinkLost;
            session.session_epoch = session.session_epoch.saturating_add(1);
            session.comparison.availability = AvailabilityComparison::Lost;
            session.comparison.monitoring = MonitoringComparison::Stale;
        } else {
            self.refresh_session_comparison(session_id)?;
        }
        Ok(())
    }

    pub fn begin_reconnect(
        &mut self,
        session_id: VirtualOnlineSessionId,
    ) -> Result<(), CommissioningError> {
        let session = self
            .sessions
            .get_mut(&session_id)
            .ok_or(SessionError::UnknownSession(session_id))?;
        if !matches!(
            session.state,
            SessionState::VirtualLinkLost | SessionState::VirtualUnavailable
        ) {
            return Err(SessionError::IllegalTransition {
                from: session.state,
                action: "Begin Reconnect",
            }
            .into());
        }
        session.state = SessionState::Reconnecting;
        session.session_epoch = session.session_epoch.saturating_add(1);
        session.comparison.monitoring = MonitoringComparison::Inactive;
        Ok(())
    }

    pub fn complete_reconnect(
        &mut self,
        session_id: VirtualOnlineSessionId,
    ) -> Result<(), CommissioningError> {
        let state = self
            .sessions
            .get(&session_id)
            .ok_or(SessionError::UnknownSession(session_id))?
            .state;
        if state != SessionState::Reconnecting {
            return Err(SessionError::IllegalTransition {
                from: state,
                action: "Complete Reconnect",
            }
            .into());
        }
        self.bind_session(session_id, true)?;
        self.audit_session_change(session_id);
        Ok(())
    }

    pub fn begin_disconnect(
        &mut self,
        session_id: VirtualOnlineSessionId,
    ) -> Result<(), CommissioningError> {
        let session = self
            .sessions
            .get_mut(&session_id)
            .ok_or(SessionError::UnknownSession(session_id))?;
        if matches!(session.state, SessionState::Closed | SessionState::Closing) {
            return Err(SessionError::IllegalTransition {
                from: session.state,
                action: "Begin Disconnect",
            }
            .into());
        }
        session.state = SessionState::Closing;
        session.comparison.monitoring = MonitoringComparison::Inactive;
        Ok(())
    }

    pub fn complete_disconnect(
        &mut self,
        session_id: VirtualOnlineSessionId,
    ) -> Result<(), CommissioningError> {
        let session = self
            .sessions
            .get_mut(&session_id)
            .ok_or(SessionError::UnknownSession(session_id))?;
        if session.state != SessionState::Closing {
            return Err(SessionError::IllegalTransition {
                from: session.state,
                action: "Complete Disconnect",
            }
            .into());
        }
        session.state = SessionState::Closed;
        session.session_epoch = session.session_epoch.saturating_add(1);
        session.controller_epoch = None;
        session.loaded_package_fingerprint = None;
        self.audit_session_change(session_id);
        Ok(())
    }

    pub fn session_command_binding(
        &self,
        session_id: VirtualOnlineSessionId,
    ) -> Result<SessionCommandBinding, CommissioningError> {
        let session = self
            .sessions
            .get(&session_id)
            .ok_or(SessionError::UnknownSession(session_id))?;
        if session.state != SessionState::Online {
            return Err(SessionError::NotOnline(session.state).into());
        }
        let instance = self
            .controllers
            .get(&session.controller_id)
            .ok_or(SessionError::TargetUnavailable)?;
        Ok(SessionCommandBinding {
            session_id,
            expected_universe_epoch: session.universe_epoch,
            expected_controller_epoch: session
                .controller_epoch
                .ok_or(SessionError::StaleControllerEpoch)?,
            expected_session_epoch: session.session_epoch,
            expected_target_state_hash: instance.semantic_state_hash(),
        })
    }

    pub fn request_run(
        &mut self,
        binding: SessionCommandBinding,
        restart: RestartKind,
    ) -> Result<(), CommissioningError> {
        let target = self.validate_session_binding(binding)?;
        {
            let instance = self
                .controllers
                .get(&target)
                .ok_or(SessionError::TargetUnavailable)?;
            let offline = self.offline.get(&instance.offline_controller_id).ok_or(
                CommissioningError::UnknownOfflineController(instance.offline_controller_id),
            )?;
            let actual = instance.actual_hardware();
            let configured_fingerprint = offline.configured.configured_hardware_fingerprint;
            if !actual.present
                || actual.fault_state_hash != Hash32::ZERO
                || actual.fingerprint != configured_fingerprint
            {
                return Err(CommissioningError::RequiredVirtualHardwareInvalid {
                    controller_id: target,
                    configured_fingerprint,
                    actual_fingerprint: actual.fingerprint,
                    present: actual.present,
                    fault_state_hash: actual.fault_state_hash,
                });
            }
        }
        let instance = self
            .controllers
            .get_mut(&target)
            .ok_or(SessionError::TargetUnavailable)?;
        instance.runtime.request_run(restart)?;
        self.refresh_sessions_for_target(target);
        self.audit_cpu_command(target);
        Ok(())
    }

    pub fn request_stop(
        &mut self,
        binding: SessionCommandBinding,
    ) -> Result<(), CommissioningError> {
        let target = self.validate_session_binding(binding)?;
        let instance = self
            .controllers
            .get_mut(&target)
            .ok_or(SessionError::TargetUnavailable)?;
        instance.runtime.request_stop()?;
        self.refresh_sessions_for_target(target);
        self.audit_cpu_command(target);
        Ok(())
    }

    pub fn restore_controller_snapshot(
        &mut self,
        binding: SessionCommandBinding,
        snapshot: &ControllerSnapshot,
    ) -> Result<Hash32, CommissioningError> {
        let target = self.validate_session_binding(binding)?;
        let instance = self
            .controllers
            .get(&target)
            .ok_or(SessionError::TargetUnavailable)?;
        let pre_state_hash = instance.semantic_state_hash();
        let old_universe_epoch = self.universe_epoch;
        let mut staged = instance.clone();
        let approval = match staged.runtime.prepare_restore(snapshot) {
            Ok(approval) => approval,
            Err(error) => {
                self.audit_snapshot_restore(target, false, pre_state_hash, pre_state_hash);
                return Err(error.into());
            }
        };
        let restored_runtime_hash = match staged.runtime.restore_snapshot(snapshot, approval) {
            Ok(hash) => hash,
            Err(error) => {
                self.audit_snapshot_restore(target, false, pre_state_hash, pre_state_hash);
                return Err(error.into());
            }
        };
        let post_state_hash = staged.semantic_state_hash();
        self.controllers.insert(target, staged);
        self.universe_epoch = self.universe_epoch.saturating_add(1);
        self.mark_sessions_lost_for_universe_epoch_change(old_universe_epoch);
        self.refresh_sessions_for_target(target);
        self.audit_snapshot_restore(target, true, pre_state_hash, post_state_hash);
        Ok(restored_runtime_hash)
    }

    pub fn run_scan(
        &mut self,
        binding: SessionCommandBinding,
    ) -> Result<RunOutcome, CommissioningError> {
        let target = self.validate_session_binding(binding)?;
        let outcome = self
            .controllers
            .get_mut(&target)
            .ok_or(SessionError::TargetUnavailable)?
            .runtime
            .run_scan()?;
        self.refresh_sessions_for_target(target);
        Ok(outcome)
    }

    pub fn set_virtual_input_raw(
        &mut self,
        binding: SessionCommandBinding,
        command: InputCommand,
    ) -> Result<InputReceipt, CommissioningError> {
        let target = self.validate_session_binding(binding)?;
        let instance = self
            .controllers
            .get_mut(&target)
            .ok_or(SessionError::TargetUnavailable)?;
        let pre_state_hash = instance.semantic_state_hash();
        let receipt = instance.runtime.set_virtual_input_raw(command)?;
        if receipt.duplicate {
            return Ok(receipt);
        }
        let post_state_hash = instance.semantic_state_hash();
        self.refresh_sessions_for_target(target);
        self.audit_virtual_input_change(target, pre_state_hash, post_state_hash);
        Ok(receipt)
    }

    pub fn apply_observation_boundary(
        &mut self,
        binding: SessionCommandBinding,
        command: &RuntimeBoundaryCommand,
        projection: &ForceRegistryProjection,
    ) -> Result<CommissionedBoundaryReceipt, CommissioningError> {
        let target = self.validate_session_binding(binding)?;
        let instance = self
            .controllers
            .get(&target)
            .ok_or(SessionError::TargetUnavailable)?;
        if projection.expected_previous_registry_hash != instance.force_registry_hash {
            return Err(CommissioningError::ForceRegistryStateChanged {
                expected: projection.expected_previous_registry_hash,
                actual: instance.force_registry_hash,
            });
        }

        let pre_state_hash = instance.semantic_state_hash();
        let mut staged = instance.clone();
        let runtime = staged.runtime.apply_observation_boundary(command)?;
        if runtime.force_overlay_hash != projection.expected_runtime_overlay_hash {
            return Err(CommissioningError::ForceRuntimeProjectionMismatch {
                expected: projection.expected_runtime_overlay_hash,
                actual: runtime.force_overlay_hash,
            });
        }
        staged.active_force_ids = projection.active_force_ids.clone();
        staged.force_registry_hash = projection.next_registry_hash;
        let controller_state_hash = staged.semantic_state_hash();
        self.controllers.insert(target, staged);
        self.refresh_sessions_for_target(target);
        self.audit_observation_command(target, pre_state_hash, controller_state_hash);
        Ok(CommissionedBoundaryReceipt {
            runtime,
            force_registry_hash: projection.next_registry_hash,
            controller_state_hash,
        })
    }

    pub fn run_scan_with_observation(
        &mut self,
        binding: SessionCommandBinding,
        command: &RuntimeScanCommand,
        projection: &ForceRegistryProjection,
    ) -> Result<CommissionedScanReceipt, CommissioningError> {
        let target = self.validate_session_binding(binding)?;
        let instance = self
            .controllers
            .get(&target)
            .ok_or(SessionError::TargetUnavailable)?;
        if projection.expected_previous_registry_hash != instance.force_registry_hash {
            return Err(CommissioningError::ForceRegistryStateChanged {
                expected: projection.expected_previous_registry_hash,
                actual: instance.force_registry_hash,
            });
        }

        let pre_state_hash = instance.semantic_state_hash();
        let mut staged = instance.clone();
        let runtime = staged.runtime.run_scan_with_observation(command)?;
        if runtime.force_overlay_hash != projection.expected_runtime_overlay_hash {
            return Err(CommissioningError::ForceRuntimeProjectionMismatch {
                expected: projection.expected_runtime_overlay_hash,
                actual: runtime.force_overlay_hash,
            });
        }
        staged.active_force_ids = projection.active_force_ids.clone();
        staged.force_registry_hash = projection.next_registry_hash;
        let controller_state_hash = staged.semantic_state_hash();
        self.controllers.insert(target, staged);
        self.refresh_sessions_for_target(target);
        self.audit_observation_command(target, pre_state_hash, controller_state_hash);
        Ok(CommissionedScanReceipt {
            runtime,
            force_registry_hash: projection.next_registry_hash,
            controller_state_hash,
        })
    }

    pub fn refresh_session_comparison(
        &mut self,
        session_id: VirtualOnlineSessionId,
    ) -> Result<(), CommissioningError> {
        let session = self
            .sessions
            .get(&session_id)
            .ok_or(SessionError::UnknownSession(session_id))?;
        let offline_id = *self
            .session_offline
            .get(&session_id)
            .ok_or(SessionError::UnknownSession(session_id))?;
        let offline = self
            .offline
            .get(&offline_id)
            .ok_or(CommissioningError::UnknownOfflineController(offline_id))?;
        let availability = match session.state {
            SessionState::Online => AvailabilityComparison::Available,
            SessionState::VirtualLinkLost => AvailabilityComparison::Lost,
            _ => AvailabilityComparison::Unavailable,
        };
        let monitoring = session.comparison.monitoring;
        let comparison = comparison_for(
            offline,
            self.controllers.get(&session.controller_id),
            availability,
            monitoring,
        );
        self.sessions
            .get_mut(&session_id)
            .ok_or(SessionError::UnknownSession(session_id))?
            .comparison = comparison;
        Ok(())
    }

    fn bind_session(
        &mut self,
        session_id: VirtualOnlineSessionId,
        reconnect: bool,
    ) -> Result<(), CommissioningError> {
        let (target, offline_id) = {
            let session = self
                .sessions
                .get(&session_id)
                .ok_or(SessionError::UnknownSession(session_id))?;
            let offline_id = *self
                .session_offline
                .get(&session_id)
                .ok_or(SessionError::UnknownSession(session_id))?;
            (session.controller_id, offline_id)
        };
        let offline = self
            .offline
            .get(&offline_id)
            .ok_or(CommissioningError::UnknownOfflineController(offline_id))?;
        let instance = self.controllers.get(&target);
        let comparison = comparison_for(
            offline,
            instance,
            if instance.is_some() {
                AvailabilityComparison::Available
            } else {
                AvailabilityComparison::Unavailable
            },
            MonitoringComparison::Inactive,
        );
        let session = self
            .sessions
            .get_mut(&session_id)
            .ok_or(SessionError::UnknownSession(session_id))?;
        if reconnect {
            session.session_epoch = session.session_epoch.saturating_add(1);
        }
        session.universe_epoch = self.universe_epoch;
        session.comparison = comparison;
        if let Some(instance) = instance {
            session.state = SessionState::Online;
            session.controller_epoch = Some(instance.runtime.controller_epoch());
            session.loaded_package_fingerprint = instance
                .loaded_package
                .as_ref()
                .map(VirtualLoadPackage::fingerprint);
        } else {
            session.state = SessionState::VirtualUnavailable;
            session.controller_epoch = None;
            session.loaded_package_fingerprint = None;
        }
        Ok(())
    }

    fn validate_session_binding(
        &self,
        binding: SessionCommandBinding,
    ) -> Result<VirtualControllerId, CommissioningError> {
        let session = self
            .sessions
            .get(&binding.session_id)
            .ok_or(SessionError::UnknownSession(binding.session_id))?;
        if session.state != SessionState::Online {
            return Err(SessionError::NotOnline(session.state).into());
        }
        if binding.expected_universe_epoch != self.universe_epoch
            || binding.expected_universe_epoch != session.universe_epoch
        {
            return Err(SessionError::StaleUniverseEpoch.into());
        }
        if binding.expected_session_epoch != session.session_epoch {
            return Err(SessionError::StaleSessionEpoch.into());
        }
        let instance = self
            .controllers
            .get(&session.controller_id)
            .ok_or(SessionError::TargetUnavailable)?;
        if binding.expected_controller_epoch != instance.runtime.controller_epoch()
            || session.controller_epoch != Some(instance.runtime.controller_epoch())
        {
            return Err(SessionError::StaleControllerEpoch.into());
        }
        if binding.expected_target_state_hash != instance.semantic_state_hash() {
            return Err(SessionError::StaleTargetState.into());
        }
        Ok(session.controller_id)
    }

    fn mark_sessions_lost_for_epoch_change(
        &mut self,
        target: VirtualControllerId,
        old_controller_epoch: u64,
    ) {
        for session in self.sessions.values_mut() {
            if session.controller_id == target
                && session.state == SessionState::Online
                && session.controller_epoch == Some(old_controller_epoch)
            {
                session.state = SessionState::VirtualLinkLost;
                session.session_epoch = session.session_epoch.saturating_add(1);
                session.comparison.availability = AvailabilityComparison::Lost;
                session.comparison.monitoring = MonitoringComparison::Stale;
            }
        }
    }

    fn mark_sessions_lost_for_universe_epoch_change(&mut self, old_universe_epoch: u64) {
        for session in self.sessions.values_mut() {
            if session.state == SessionState::Online && session.universe_epoch == old_universe_epoch
            {
                session.state = SessionState::VirtualLinkLost;
                session.session_epoch = session.session_epoch.saturating_add(1);
                session.comparison.availability = AvailabilityComparison::Lost;
                session.comparison.monitoring = MonitoringComparison::Stale;
            }
        }
    }

    fn refresh_sessions_for_target(&mut self, target: VirtualControllerId) {
        let ids = self
            .sessions
            .values()
            .filter(|session| session.controller_id == target)
            .map(|session| session.id)
            .collect::<Vec<_>>();
        for id in ids {
            let _ = self.refresh_session_comparison(id);
        }
    }

    fn record_load_rollback<T>(
        &mut self,
        preview: &LoadPreview,
        failure_point: InternalFailurePoint,
        pre_state_hash: Hash32,
    ) -> Result<T, CommissioningError> {
        let restored_state_hash = self
            .controllers
            .get(&preview.target_controller_id)
            .ok_or(CommissioningError::UnknownController(
                preview.target_controller_id,
            ))?
            .semantic_state_hash();
        let sequence = self.next_event();
        self.audit.push(CommissioningAuditEvent {
            event_sequence: sequence,
            kind: CommissioningAuditKind::LoadFailed,
            controller_id: Some(preview.target_controller_id),
            preview_id: Some(preview.id),
            success: false,
            pre_state_hash: Some(pre_state_hash),
            post_state_hash: Some(restored_state_hash),
            internal_failure_point: failure_point,
        });
        Err(CommissioningError::LoadRolledBack {
            failure_point,
            pre_state_hash,
            restored_state_hash,
        })
    }

    fn record_lifecycle_rollback<T>(
        &mut self,
        target: VirtualControllerId,
        kind: CommissioningAuditKind,
        failure_point: InternalFailurePoint,
        pre_state_hash: Hash32,
    ) -> Result<T, CommissioningError> {
        let restored_state_hash = self
            .controllers
            .get(&target)
            .ok_or(CommissioningError::UnknownController(target))?
            .semantic_state_hash();
        let sequence = self.next_event();
        self.audit.push(CommissioningAuditEvent {
            event_sequence: sequence,
            kind,
            controller_id: Some(target),
            preview_id: None,
            success: false,
            pre_state_hash: Some(pre_state_hash),
            post_state_hash: Some(restored_state_hash),
            internal_failure_point: failure_point,
        });
        Err(CommissioningError::LifecycleRolledBack {
            failure_point,
            pre_state_hash,
            restored_state_hash,
        })
    }

    fn audit_session_change(&mut self, session_id: VirtualOnlineSessionId) {
        let controller_id = self
            .sessions
            .get(&session_id)
            .map(|session| session.controller_id);
        let sequence = self.next_event();
        self.audit.push(CommissioningAuditEvent {
            event_sequence: sequence,
            kind: CommissioningAuditKind::SessionChanged,
            controller_id,
            preview_id: None,
            success: true,
            pre_state_hash: None,
            post_state_hash: None,
            internal_failure_point: InternalFailurePoint::None,
        });
    }

    fn audit_cpu_command(&mut self, target: VirtualControllerId) {
        let post_state_hash = self
            .controllers
            .get(&target)
            .map(ControllerInstance::semantic_state_hash);
        let sequence = self.next_event();
        self.audit.push(CommissioningAuditEvent {
            event_sequence: sequence,
            kind: CommissioningAuditKind::CpuCommand,
            controller_id: Some(target),
            preview_id: None,
            success: true,
            pre_state_hash: None,
            post_state_hash,
            internal_failure_point: InternalFailurePoint::None,
        });
    }

    fn audit_observation_command(
        &mut self,
        target: VirtualControllerId,
        pre_state_hash: Hash32,
        post_state_hash: Hash32,
    ) {
        let sequence = self.next_event();
        self.audit.push(CommissioningAuditEvent {
            event_sequence: sequence,
            kind: CommissioningAuditKind::ObservationCommand,
            controller_id: Some(target),
            preview_id: None,
            success: true,
            pre_state_hash: Some(pre_state_hash),
            post_state_hash: Some(post_state_hash),
            internal_failure_point: InternalFailurePoint::None,
        });
    }

    fn audit_virtual_input_change(
        &mut self,
        target: VirtualControllerId,
        pre_state_hash: Hash32,
        post_state_hash: Hash32,
    ) {
        let sequence = self.next_event();
        self.audit.push(CommissioningAuditEvent {
            event_sequence: sequence,
            kind: CommissioningAuditKind::VirtualInputChanged,
            controller_id: Some(target),
            preview_id: None,
            success: true,
            pre_state_hash: Some(pre_state_hash),
            post_state_hash: Some(post_state_hash),
            internal_failure_point: InternalFailurePoint::None,
        });
    }

    fn audit_snapshot_restore(
        &mut self,
        target: VirtualControllerId,
        success: bool,
        pre_state_hash: Hash32,
        post_state_hash: Hash32,
    ) {
        let sequence = self.next_event();
        self.audit.push(CommissioningAuditEvent {
            event_sequence: sequence,
            kind: CommissioningAuditKind::ControllerSnapshotRestored,
            controller_id: Some(target),
            preview_id: None,
            success,
            pre_state_hash: Some(pre_state_hash),
            post_state_hash: Some(post_state_hash),
            internal_failure_point: InternalFailurePoint::None,
        });
    }

    fn next_event(&mut self) -> u64 {
        self.event_sequence = self.event_sequence.saturating_add(1);
        self.event_sequence
    }
}

fn hash_reset_preview(preview: &ResetInstancePreview) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-RESET-INSTANCE-PREVIEW-1");
    hasher.u128(preview.target_controller_id.0);
    hasher.u64(preview.universe_epoch);
    hasher.u64(preview.controller_epoch);
    hasher.hash(preview.target_state_hash);
    hasher.u8(preview.kind as u8);
    hasher.u8(preview.current_cpu_state as u8);
    hasher.u8(preview.final_cpu_state as u8);
    hasher.hash(preview.loaded_package_fingerprint);
    encode_identity_list(&preview.reset_member_ids, &mut hasher);
    encode_identity_list(&preview.preserved_member_ids, &mut hasher);
    encode_identity_list(&preview.reset_state_ids, &mut hasher);
    encode_identity_list(&preview.preserved_state_ids, &mut hasher);
    encode_force_list(&preview.cleared_force_ids, &mut hasher);
    encode_session_list(&preview.invalidated_session_ids, &mut hasher);
    hasher.u64(preview.next_controller_epoch);
    hasher.finish()
}

fn hash_replace_preview(preview: &ReplaceInstancePreview) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-REPLACE-INSTANCE-PREVIEW-1");
    hasher.u128(preview.target_controller_id.0);
    hasher.u128(preview.current_instance_id.0);
    hasher.u128(preview.replacement_instance_id.0);
    hasher.u64(preview.universe_epoch);
    hasher.u64(preview.controller_epoch);
    hasher.hash(preview.target_state_hash);
    match preview.loaded_package_fingerprint {
        Some(value) => {
            hasher.bool(true);
            hasher.hash(value);
        }
        None => hasher.bool(false),
    }
    encode_identity_list(&preview.removed_member_ids, &mut hasher);
    encode_identity_list(&preview.removed_state_ids, &mut hasher);
    encode_force_list(&preview.cleared_force_ids, &mut hasher);
    encode_session_list(&preview.invalidated_session_ids, &mut hasher);
    hasher.u64(preview.next_controller_epoch);
    hasher.u8(preview.final_cpu_state as u8);
    hasher.finish()
}

fn hash_remove_preview(preview: &RemoveInstancePreview) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-REMOVE-INSTANCE-PREVIEW-1");
    hasher.u128(preview.target_controller_id.0);
    hasher.u64(preview.controller_epoch);
    hasher.hash(preview.target_state_hash);
    match preview.loaded_package_fingerprint {
        Some(value) => {
            hasher.bool(true);
            hasher.hash(value);
        }
        None => hasher.bool(false),
    }
    encode_force_list(&preview.active_force_ids, &mut hasher);
    encode_session_list(&preview.invalidated_session_ids, &mut hasher);
    hasher.finish()
}

fn encode_identity_list(values: &[u128], hasher: &mut CanonicalHasher) {
    hasher.u64(values.len() as u64);
    for value in values {
        hasher.u128(*value);
    }
}

fn encode_force_list(values: &[ForceId], hasher: &mut CanonicalHasher) {
    hasher.u64(values.len() as u64);
    for value in values {
        hasher.u128(value.0);
    }
}

fn encode_session_list(values: &[VirtualOnlineSessionId], hasher: &mut CanonicalHasher) {
    hasher.u64(values.len() as u64);
    for value in values {
        hasher.u128(value.0);
    }
}

fn build_load_result(
    preview: &LoadPreview,
    candidate: &VirtualLoadPackage,
    final_cpu_state: CpuState,
    comparison: ComparisonVector,
    controller_epoch: u64,
    pre_state_hash: Hash32,
    post_state_hash: Hash32,
) -> LoadResult {
    LoadResult {
        preview_id: preview.id,
        old_package_fingerprint: preview.current_package_fingerprint,
        new_package_fingerprint: candidate.fingerprint(),
        compatibility: preview.compatibility,
        preserved_member_ids: preview
            .memory_actions
            .iter()
            .filter(|action| action.kind == MemoryActionKind::Preserve)
            .map(|action| action.member_id)
            .collect(),
        initialized_member_ids: preview
            .memory_actions
            .iter()
            .filter(|action| action.kind == MemoryActionKind::Initialize)
            .map(|action| action.member_id)
            .collect(),
        removed_member_ids: preview
            .memory_actions
            .iter()
            .filter(|action| action.kind == MemoryActionKind::Remove)
            .map(|action| action.member_id)
            .collect(),
        preserved_state_ids: preview
            .state_actions
            .iter()
            .filter(|action| action.kind == StateActionKind::Preserve)
            .map(|action| action.state_member_id)
            .collect(),
        initialized_state_ids: preview
            .state_actions
            .iter()
            .filter(|action| action.kind == StateActionKind::Initialize)
            .map(|action| action.state_member_id)
            .collect(),
        removed_state_ids: preview
            .state_actions
            .iter()
            .filter(|action| action.kind == StateActionKind::Remove)
            .map(|action| action.state_member_id)
            .collect(),
        final_cpu_state,
        comparison,
        controller_epoch,
        rollback: LoadRollback::NotRequired,
        pre_state_hash,
        post_state_hash,
    }
}

fn comparison_for(
    offline: &OfflineEngineeringState,
    instance: Option<&ControllerInstance>,
    availability: AvailabilityComparison,
    monitoring: MonitoringComparison,
) -> ComparisonVector {
    let loaded = instance.and_then(|instance| instance.loaded_package.as_ref());
    let profile = match instance {
        Some(_) => match loaded {
            Some(package)
                if package.profile_fingerprint() == offline.configured.profile_fingerprint =>
            {
                ProfileComparison::Match
            }
            Some(_) => ProfileComparison::Mismatch,
            None => ProfileComparison::Unknown,
        },
        None => ProfileComparison::Unknown,
    };
    let comparable = profile != ProfileComparison::Mismatch;
    let software_to_loaded = match (offline.current_package_fingerprint, loaded) {
        (_, None) => PackageComparison::NotLoaded,
        (None, Some(_)) => PackageComparison::NotComparable,
        (Some(_), Some(_)) if !comparable => PackageComparison::NotComparable,
        (Some(offline), Some(loaded)) if offline == loaded.fingerprint() => {
            PackageComparison::Match
        }
        (Some(_), Some(_)) => PackageComparison::Mismatch,
    };
    let hardware_to_loaded = match (offline.built_hardware.as_ref(), loaded) {
        (_, None) => HardwareComparison::NotLoaded,
        (None, Some(_)) => HardwareComparison::NotComparable,
        (Some(_), Some(_)) if !comparable => HardwareComparison::NotComparable,
        (Some(hardware), Some(loaded)) if hardware.fingerprint == loaded.hardware_fingerprint() => {
            HardwareComparison::Match
        }
        (Some(_), Some(_)) => HardwareComparison::Mismatch,
    };
    ComparisonVector {
        project_saved: offline.project_saved,
        hardware_build_current: offline.hardware_build_current,
        software_build_current: offline.software_build_current,
        source_to_build: offline.source_to_build,
        hardware_to_loaded,
        software_to_loaded,
        profile,
        availability,
        monitoring,
        force_active: instance
            .map(|instance| !instance.active_force_ids.is_empty())
            .unwrap_or(false),
    }
}

fn actual_hardware_hash(state: &ActualHardwareState) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-ACTUAL-HARDWARE-1");
    hasher.hash(state.fingerprint);
    hasher.bool(state.present);
    hasher.hash(state.fault_state_hash);
    hasher.finish()
}

fn empty_force_registry_hash() -> Hash32 {
    CanonicalHasher::new("PES-FORCE-REGISTRY-1").finish()
}
