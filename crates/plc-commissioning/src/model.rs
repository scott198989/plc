use alloc::{collections::BTreeSet, string::String, vec::Vec};
use core::{error::Error, fmt};

use plc_runtime::{
    ArtifactError, ArtifactPackage, CanonicalValue, CpuState, Hash32, MemoryId, StateId,
    UniverseId, ValueType, VerifiedArtifact, VirtualControllerId,
};

use crate::canonical::{CanonicalHasher, encode_value, id_from_hash};

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub u128);
    };
}

id_type!(OfflineControllerId);
id_type!(VirtualOnlineSessionId);
id_type!(PreviewId);
id_type!(ForceId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum MemoryRole {
    Marker = 1,
    GlobalDb = 2,
    InstanceDb = 3,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryMemberSchema {
    pub member_id: u128,
    pub runtime_memory_id: MemoryId,
    pub value_type: ValueType,
    pub role: MemoryRole,
    pub instance_path: Vec<u128>,
    pub retentive: bool,
    pub loaded_start: CanonicalValue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum StateKind {
    Edge = 1,
    Timer = 2,
    Counter = 3,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateMemberSchema {
    pub state_member_id: u128,
    pub runtime_state_id: StateId,
    pub kind: StateKind,
    pub owner_member_id: u128,
    pub instance_path: Vec<u128>,
    pub retentive: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadPackageParts {
    pub runtime_artifact: ArtifactPackage,
    pub semantic_build_fingerprint: Hash32,
    pub verified_ir_fingerprint: Hash32,
    pub schedule_fingerprint: Hash32,
    pub hardware_fingerprint: Hash32,
    pub source_map_fingerprint: Hash32,
    pub probe_identity_fingerprint: Hash32,
    pub capability_fingerprint: Hash32,
    pub build_snapshot_hash: Hash32,
    pub build_is_current: bool,
    pub blocking_diagnostic_count: u32,
    pub memory_schema: Vec<MemoryMemberSchema>,
    pub state_schema: Vec<StateMemberSchema>,
}

impl LoadPackageParts {
    fn normalize(&mut self) {
        self.memory_schema.sort_by_key(|member| member.member_id);
        self.state_schema
            .sort_by_key(|member| member.state_member_id);
    }

    fn fingerprint(&self) -> Hash32 {
        let mut hasher = CanonicalHasher::new("PES-VIRTUAL-LOAD-PACKAGE-1");
        hasher.hash(self.runtime_artifact.fingerprint());
        hasher.hash(self.semantic_build_fingerprint);
        hasher.hash(self.verified_ir_fingerprint);
        hasher.hash(self.schedule_fingerprint);
        hasher.hash(self.hardware_fingerprint);
        hasher.hash(self.source_map_fingerprint);
        hasher.hash(self.probe_identity_fingerprint);
        hasher.hash(self.capability_fingerprint);
        hasher.hash(self.build_snapshot_hash);
        hasher.bool(self.build_is_current);
        hasher.u32(self.blocking_diagnostic_count);
        hasher.u64(self.memory_schema.len() as u64);
        for member in &self.memory_schema {
            encode_memory_schema(member, &mut hasher);
        }
        hasher.u64(self.state_schema.len() as u64);
        for member in &self.state_schema {
            encode_state_schema(member, &mut hasher);
        }
        hasher.finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VirtualLoadPackage {
    parts: LoadPackageParts,
    package_fingerprint: Hash32,
    integrity_verified: bool,
}

impl VirtualLoadPackage {
    pub fn seal_verified(mut parts: LoadPackageParts) -> Result<Self, LoadPackageError> {
        parts.normalize();
        validate_package_parts(&parts)?;
        let package_fingerprint = parts.fingerprint();
        Ok(Self {
            parts,
            package_fingerprint,
            integrity_verified: true,
        })
    }

    pub fn from_untrusted_package(
        parts: LoadPackageParts,
        package_fingerprint: Hash32,
        integrity_verified: bool,
    ) -> Self {
        Self {
            parts,
            package_fingerprint,
            integrity_verified,
        }
    }

    pub fn validate(&self) -> Result<(), LoadPackageError> {
        if !self.integrity_verified {
            return Err(LoadPackageError::IntegrityNotVerified);
        }
        validate_package_parts(&self.parts)?;
        let actual = self.parts.fingerprint();
        if actual != self.package_fingerprint {
            return Err(LoadPackageError::FingerprintMismatch {
                declared: self.package_fingerprint,
                actual,
            });
        }
        Ok(())
    }

    pub const fn fingerprint(&self) -> Hash32 {
        self.package_fingerprint
    }

    pub const fn runtime_artifact(&self) -> &ArtifactPackage {
        &self.parts.runtime_artifact
    }

    pub const fn semantic_build_fingerprint(&self) -> Hash32 {
        self.parts.semantic_build_fingerprint
    }

    pub const fn verified_ir_fingerprint(&self) -> Hash32 {
        self.parts.verified_ir_fingerprint
    }

    pub const fn schedule_fingerprint(&self) -> Hash32 {
        self.parts.schedule_fingerprint
    }

    pub const fn hardware_fingerprint(&self) -> Hash32 {
        self.parts.hardware_fingerprint
    }

    pub const fn source_map_fingerprint(&self) -> Hash32 {
        self.parts.source_map_fingerprint
    }

    pub const fn probe_identity_fingerprint(&self) -> Hash32 {
        self.parts.probe_identity_fingerprint
    }

    pub const fn capability_fingerprint(&self) -> Hash32 {
        self.parts.capability_fingerprint
    }

    pub const fn build_snapshot_hash(&self) -> Hash32 {
        self.parts.build_snapshot_hash
    }

    pub const fn build_is_current(&self) -> bool {
        self.parts.build_is_current
    }

    pub const fn blocking_diagnostic_count(&self) -> u32 {
        self.parts.blocking_diagnostic_count
    }

    pub fn memory_schema(&self) -> &[MemoryMemberSchema] {
        &self.parts.memory_schema
    }

    pub fn state_schema(&self) -> &[StateMemberSchema] {
        &self.parts.state_schema
    }

    pub fn profile_fingerprint(&self) -> Hash32 {
        self.parts.runtime_artifact.spec().profile_fingerprint
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadPackageError {
    IntegrityNotVerified,
    FingerprintMismatch { declared: Hash32, actual: Hash32 },
    RuntimeArtifact(ArtifactError),
    DuplicateMemberId(u128),
    DuplicateRuntimeMemoryId(MemoryId),
    DuplicateStateMemberId(u128),
    DuplicateRuntimeStateId(StateId),
    MemorySchemaDoesNotMatchRuntime(MemoryId),
    StateSchemaDoesNotMatchRuntime(StateId),
    OrphanRuntimeMemory(MemoryId),
    OrphanRuntimeState(StateId),
}

impl fmt::Display for LoadPackageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "virtual load package rejected: {self:?}")
    }
}

impl Error for LoadPackageError {}

impl From<ArtifactError> for LoadPackageError {
    fn from(value: ArtifactError) -> Self {
        Self::RuntimeArtifact(value)
    }
}

fn validate_package_parts(parts: &LoadPackageParts) -> Result<(), LoadPackageError> {
    let runtime = VerifiedArtifact::accept(&parts.runtime_artifact)?;
    let mut member_ids = BTreeSet::new();
    let mut runtime_memory_ids = BTreeSet::new();
    for member in &parts.memory_schema {
        if !member_ids.insert(member.member_id) {
            return Err(LoadPackageError::DuplicateMemberId(member.member_id));
        }
        if !runtime_memory_ids.insert(member.runtime_memory_id) {
            return Err(LoadPackageError::DuplicateRuntimeMemoryId(
                member.runtime_memory_id,
            ));
        }
        let Some(definition) = runtime
            .spec()
            .memory
            .iter()
            .find(|definition| definition.id == member.runtime_memory_id)
        else {
            return Err(LoadPackageError::MemorySchemaDoesNotMatchRuntime(
                member.runtime_memory_id,
            ));
        };
        if definition.value_type != member.value_type
            || definition.retentive != member.retentive
            || definition.loaded_start != member.loaded_start
        {
            return Err(LoadPackageError::MemorySchemaDoesNotMatchRuntime(
                member.runtime_memory_id,
            ));
        }
    }
    for definition in &runtime.spec().memory {
        if !runtime_memory_ids.contains(&definition.id) {
            return Err(LoadPackageError::OrphanRuntimeMemory(definition.id));
        }
    }

    let mut state_member_ids = BTreeSet::new();
    let mut runtime_state_ids = BTreeSet::new();
    for member in &parts.state_schema {
        if !state_member_ids.insert(member.state_member_id) {
            return Err(LoadPackageError::DuplicateStateMemberId(
                member.state_member_id,
            ));
        }
        if !runtime_state_ids.insert(member.runtime_state_id) {
            return Err(LoadPackageError::DuplicateRuntimeStateId(
                member.runtime_state_id,
            ));
        }
        let Some(definition) = runtime
            .spec()
            .states
            .iter()
            .find(|definition| definition.id == member.runtime_state_id)
        else {
            return Err(LoadPackageError::StateSchemaDoesNotMatchRuntime(
                member.runtime_state_id,
            ));
        };
        let kind = match definition.loaded_start {
            plc_runtime::StateStart::Edge { .. } => StateKind::Edge,
            plc_runtime::StateStart::Timer { .. } => StateKind::Timer,
            plc_runtime::StateStart::Counter { .. } => StateKind::Counter,
        };
        if kind != member.kind || definition.retentive != member.retentive {
            return Err(LoadPackageError::StateSchemaDoesNotMatchRuntime(
                member.runtime_state_id,
            ));
        }
    }
    for definition in &runtime.spec().states {
        if !runtime_state_ids.contains(&definition.id) {
            return Err(LoadPackageError::OrphanRuntimeState(definition.id));
        }
    }
    Ok(())
}

fn encode_memory_schema(member: &MemoryMemberSchema, hasher: &mut CanonicalHasher) {
    hasher.u128(member.member_id);
    hasher.u32(member.runtime_memory_id.0);
    hasher.u8(member.value_type as u8);
    hasher.u8(member.role as u8);
    hasher.u64(member.instance_path.len() as u64);
    for id in &member.instance_path {
        hasher.u128(*id);
    }
    hasher.bool(member.retentive);
    encode_value(member.loaded_start, hasher);
}

fn encode_state_schema(member: &StateMemberSchema, hasher: &mut CanonicalHasher) {
    hasher.u128(member.state_member_id);
    hasher.u32(member.runtime_state_id.0);
    hasher.u8(member.kind as u8);
    hasher.u128(member.owner_member_id);
    hasher.u64(member.instance_path.len() as u64);
    for id in &member.instance_path {
        hasher.u128(*id);
    }
    hasher.bool(member.retentive);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfiguredController {
    pub id: OfflineControllerId,
    pub configured_hardware_fingerprint: Hash32,
    pub profile_fingerprint: Hash32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuiltHardwareState {
    pub fingerprint: Hash32,
    pub configured_hardware_fingerprint: Hash32,
    pub current: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActualHardwareState {
    pub fingerprint: Hash32,
    pub present: bool,
    pub fault_state_hash: Hash32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum OfflineSourceBuild {
    Current = 1,
    Stale = 2,
    Absent = 3,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfflineEngineeringState {
    pub configured: ConfiguredController,
    pub source_revision_hash: Hash32,
    pub build_snapshot_hash: Option<Hash32>,
    pub project_saved: bool,
    pub source_to_build: OfflineSourceBuild,
    pub software_build_current: bool,
    pub hardware_build_current: bool,
    pub current_package_fingerprint: Option<Hash32>,
    pub built_hardware: Option<BuiltHardwareState>,
}

impl OfflineEngineeringState {
    pub fn mark_source_edited(&mut self, new_source_revision_hash: Hash32) {
        self.source_revision_hash = new_source_revision_hash;
        self.project_saved = false;
        self.source_to_build = OfflineSourceBuild::Stale;
        self.software_build_current = false;
    }

    pub fn mark_saved(&mut self) {
        self.project_saved = true;
    }

    pub fn record_build(&mut self, package: &VirtualLoadPackage) {
        self.build_snapshot_hash = Some(package.build_snapshot_hash());
        self.current_package_fingerprint = Some(package.fingerprint());
        self.source_to_build = OfflineSourceBuild::Current;
        self.software_build_current = package.build_is_current();
        self.hardware_build_current = true;
        self.built_hardware = Some(BuiltHardwareState {
            fingerprint: package.hardware_fingerprint(),
            configured_hardware_fingerprint: self.configured.configured_hardware_fingerprint,
            current: true,
        });
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum MatchComparison {
    Match = 1,
    Mismatch = 2,
    NotLoaded = 3,
    NotComparable = 4,
}

pub type HardwareComparison = MatchComparison;
pub type PackageComparison = MatchComparison;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ProfileComparison {
    Match = 1,
    Mismatch = 2,
    Unknown = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AvailabilityComparison {
    Available = 1,
    Unavailable = 2,
    Lost = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum MonitoringComparison {
    Inactive = 1,
    Active = 2,
    Stale = 3,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComparisonVector {
    pub project_saved: bool,
    pub hardware_build_current: bool,
    pub software_build_current: bool,
    pub source_to_build: OfflineSourceBuild,
    pub hardware_to_loaded: HardwareComparison,
    pub software_to_loaded: PackageComparison,
    pub profile: ProfileComparison,
    pub availability: AvailabilityComparison,
    pub monitoring: MonitoringComparison,
    pub force_active: bool,
}

impl ComparisonVector {
    pub(crate) fn encode(&self, hasher: &mut CanonicalHasher) {
        hasher.bool(self.project_saved);
        hasher.bool(self.hardware_build_current);
        hasher.bool(self.software_build_current);
        hasher.u8(self.source_to_build as u8);
        hasher.u8(self.hardware_to_loaded as u8);
        hasher.u8(self.software_to_loaded as u8);
        hasher.u8(self.profile as u8);
        hasher.u8(self.availability as u8);
        hasher.u8(self.monitoring as u8);
        hasher.bool(self.force_active);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SessionState {
    Closed = 1,
    Opening = 2,
    Online = 3,
    VirtualUnavailable = 4,
    VirtualLinkLost = 5,
    Reconnecting = 6,
    Closing = 7,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VirtualOnlineSession {
    pub(crate) id: VirtualOnlineSessionId,
    pub(crate) state: SessionState,
    pub(crate) universe_id: UniverseId,
    pub(crate) universe_epoch: u64,
    pub(crate) controller_id: VirtualControllerId,
    pub(crate) controller_epoch: Option<u64>,
    pub(crate) session_epoch: u64,
    pub(crate) loaded_package_fingerprint: Option<Hash32>,
    pub(crate) comparison: ComparisonVector,
}

impl VirtualOnlineSession {
    pub const fn id(&self) -> VirtualOnlineSessionId {
        self.id
    }

    pub const fn state(&self) -> SessionState {
        self.state
    }

    pub const fn universe_epoch(&self) -> u64 {
        self.universe_epoch
    }

    pub const fn controller_id(&self) -> VirtualControllerId {
        self.controller_id
    }

    pub const fn controller_epoch(&self) -> Option<u64> {
        self.controller_epoch
    }

    pub const fn session_epoch(&self) -> u64 {
        self.session_epoch
    }

    pub const fn loaded_package_fingerprint(&self) -> Option<Hash32> {
        self.loaded_package_fingerprint
    }

    pub const fn comparison(&self) -> &ComparisonVector {
        &self.comparison
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionCommandBinding {
    pub session_id: VirtualOnlineSessionId,
    pub expected_universe_epoch: u64,
    pub expected_controller_epoch: u64,
    pub expected_session_epoch: u64,
    pub expected_target_state_hash: Hash32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionError {
    UnknownSession(VirtualOnlineSessionId),
    DuplicateSession(VirtualOnlineSessionId),
    IllegalTransition {
        from: SessionState,
        action: &'static str,
    },
    NotOnline(SessionState),
    StaleUniverseEpoch,
    StaleControllerEpoch,
    StaleSessionEpoch,
    StaleTargetState,
    TargetUnavailable,
}

impl fmt::Display for SessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "virtual online session rejected: {self:?}")
    }
}

impl Error for SessionError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CompatibilityClass {
    Initial = 1,
    Identical = 2,
    PackageIdentityOnly = 3,
    CodeOnly = 4,
    StartValueOnly = 5,
    MemorySchemaChanging = 6,
    StatefulSchemaChanging = 7,
    HardwareChanging = 8,
    Incompatible = 9,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum MemoryActionKind {
    Preserve = 1,
    Initialize = 2,
    Remove = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum MemoryActionReason {
    StableCompatibleIdentity = 1,
    NewIdentity = 2,
    RemovedIdentity = 3,
    TypeChanged = 4,
    StorageRoleChanged = 5,
    InstancePathChanged = 6,
    ExplicitInitialization = 7,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryAction {
    pub member_id: u128,
    pub runtime_memory_id: MemoryId,
    pub kind: MemoryActionKind,
    pub reason: MemoryActionReason,
    pub old_type: Option<ValueType>,
    pub new_type: Option<ValueType>,
    pub candidate_loaded_start: Option<CanonicalValue>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum StateActionKind {
    Preserve = 1,
    Initialize = 2,
    Remove = 3,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateAction {
    pub state_member_id: u128,
    pub runtime_state_id: StateId,
    pub kind: StateActionKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PostLoadMode {
    Preserve = 1,
    Stop = 2,
    Run = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoadRequest {
    pub expected_build_snapshot_hash: Hash32,
    pub requested_post_load_mode: PostLoadMode,
    pub initialize_compatible_members: bool,
    pub valid_through_event_sequence: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadBlocker {
    CandidateIntegrity,
    CandidateBuildStale,
    CandidateHasBlockingDiagnostics(u32),
    BuildSnapshotMismatch,
    IncompatibleProfile,
    IncompatibleRuntimeContract,
    CpuMode(CpuState),
    ActiveForces(Vec<ForceId>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadPreview {
    pub(crate) id: PreviewId,
    pub(crate) preview_hash: Hash32,
    pub(crate) target_controller_id: VirtualControllerId,
    pub(crate) current_package_fingerprint: Option<Hash32>,
    pub(crate) candidate_package_fingerprint: Hash32,
    pub(crate) current_runtime_fingerprint: Option<Hash32>,
    pub(crate) candidate_runtime_fingerprint: Hash32,
    pub(crate) target_state_hash: Hash32,
    pub(crate) controller_epoch: u64,
    pub(crate) universe_epoch: u64,
    pub(crate) cpu_state: CpuState,
    pub(crate) compatibility: CompatibilityClass,
    pub(crate) requires_stop: bool,
    pub(crate) memory_actions: Vec<MemoryAction>,
    pub(crate) state_actions: Vec<StateAction>,
    pub(crate) hardware_replacement: bool,
    pub(crate) force_registry_hash: Hash32,
    pub(crate) actual_hardware_hash: Hash32,
    pub(crate) offline_source_revision_hash: Hash32,
    pub(crate) offline_build_snapshot_hash: Option<Hash32>,
    pub(crate) snapshot_invalidation: bool,
    pub(crate) opens_new_replay_segment: bool,
    pub(crate) blockers: Vec<LoadBlocker>,
    pub(crate) warnings: Vec<String>,
    pub(crate) requested_post_load_mode: PostLoadMode,
    pub(crate) initialize_compatible_members: bool,
    pub(crate) valid_through_event_sequence: u64,
}

impl LoadPreview {
    pub const fn id(&self) -> PreviewId {
        self.id
    }

    pub const fn hash(&self) -> Hash32 {
        self.preview_hash
    }

    pub const fn target_controller_id(&self) -> VirtualControllerId {
        self.target_controller_id
    }

    pub const fn compatibility(&self) -> CompatibilityClass {
        self.compatibility
    }

    pub const fn requires_stop(&self) -> bool {
        self.requires_stop
    }

    pub fn memory_actions(&self) -> &[MemoryAction] {
        &self.memory_actions
    }

    pub fn state_actions(&self) -> &[StateAction] {
        &self.state_actions
    }

    pub fn blockers(&self) -> &[LoadBlocker] {
        &self.blockers
    }

    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    pub const fn candidate_package_fingerprint(&self) -> Hash32 {
        self.candidate_package_fingerprint
    }

    pub(crate) fn finalize_hash(&mut self) {
        self.preview_hash = hash_preview(self);
        self.id = PreviewId(id_from_hash(self.preview_hash));
    }

    pub(crate) fn recompute_hash(&self) -> Hash32 {
        hash_preview(self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreviewApproval {
    pub preview_id: PreviewId,
    pub preview_hash: Hash32,
    pub target_controller_id: VirtualControllerId,
    pub controller_epoch: u64,
    pub target_state_hash: Hash32,
    pub candidate_package_fingerprint: Hash32,
    pub force_registry_hash: Hash32,
    pub requested_post_load_mode: PostLoadMode,
}

impl PreviewApproval {
    pub fn approve(preview: &LoadPreview) -> Self {
        Self {
            preview_id: preview.id,
            preview_hash: preview.preview_hash,
            target_controller_id: preview.target_controller_id,
            controller_epoch: preview.controller_epoch,
            target_state_hash: preview.target_state_hash,
            candidate_package_fingerprint: preview.candidate_package_fingerprint,
            force_registry_hash: preview.force_registry_hash,
            requested_post_load_mode: preview.requested_post_load_mode,
        }
    }
}

pub(crate) fn package_compatibility(
    current: Option<&VirtualLoadPackage>,
    candidate: &VirtualLoadPackage,
) -> CompatibilityClass {
    let Some(current) = current else {
        return CompatibilityClass::Initial;
    };
    if current.fingerprint() == candidate.fingerprint() {
        return CompatibilityClass::Identical;
    }
    if current.profile_fingerprint() != candidate.profile_fingerprint()
        || current.capability_fingerprint() != candidate.capability_fingerprint()
        || current.runtime_artifact().spec().runtime_version
            != candidate.runtime_artifact().spec().runtime_version
        || current.runtime_artifact().spec().scheduler_version
            != candidate.runtime_artifact().spec().scheduler_version
    {
        return CompatibilityClass::Incompatible;
    }
    let memory_shape_equal = memory_shape_equal(current.memory_schema(), candidate.memory_schema());
    let memory_starts_equal =
        memory_starts_equal(current.memory_schema(), candidate.memory_schema());
    let state_shape_equal = state_shape_equal(current.state_schema(), candidate.state_schema());
    let hardware_equal = current.hardware_fingerprint() == candidate.hardware_fingerprint();
    let schedule_equal = current.schedule_fingerprint() == candidate.schedule_fingerprint();
    let probes_equal =
        current.probe_identity_fingerprint() == candidate.probe_identity_fingerprint();
    let ir_equal = current.verified_ir_fingerprint() == candidate.verified_ir_fingerprint();

    if current.semantic_build_fingerprint() == candidate.semantic_build_fingerprint()
        && current.runtime_artifact().fingerprint() == candidate.runtime_artifact().fingerprint()
        && hardware_equal
        && schedule_equal
        && memory_shape_equal
        && memory_starts_equal
        && state_shape_equal
        && probes_equal
    {
        CompatibilityClass::PackageIdentityOnly
    } else if !hardware_equal {
        CompatibilityClass::HardwareChanging
    } else if !state_shape_equal {
        CompatibilityClass::StatefulSchemaChanging
    } else if !memory_shape_equal {
        CompatibilityClass::MemorySchemaChanging
    } else if ir_equal && schedule_equal && !memory_starts_equal {
        CompatibilityClass::StartValueOnly
    } else if schedule_equal {
        CompatibilityClass::CodeOnly
    } else {
        CompatibilityClass::Incompatible
    }
}

pub(crate) fn build_memory_actions(
    current: Option<&VirtualLoadPackage>,
    candidate: &VirtualLoadPackage,
    initialize_compatible: bool,
) -> Vec<MemoryAction> {
    let Some(current) = current else {
        return candidate
            .memory_schema()
            .iter()
            .map(|member| initialize_action(member, MemoryActionReason::NewIdentity))
            .collect();
    };
    let mut actions = Vec::new();
    let mut old_index = 0;
    let mut new_index = 0;
    while old_index < current.memory_schema().len() || new_index < candidate.memory_schema().len() {
        let old = current.memory_schema().get(old_index);
        let new = candidate.memory_schema().get(new_index);
        match (old, new) {
            (Some(old), Some(new)) if old.member_id == new.member_id => {
                let reason = incompatibility_reason(old, new);
                if let Some(reason) = reason {
                    actions.push(remove_action(old, reason));
                    actions.push(initialize_action(new, reason));
                } else if initialize_compatible {
                    actions.push(initialize_action(
                        new,
                        MemoryActionReason::ExplicitInitialization,
                    ));
                } else {
                    actions.push(MemoryAction {
                        member_id: new.member_id,
                        runtime_memory_id: new.runtime_memory_id,
                        kind: MemoryActionKind::Preserve,
                        reason: MemoryActionReason::StableCompatibleIdentity,
                        old_type: Some(old.value_type),
                        new_type: Some(new.value_type),
                        candidate_loaded_start: Some(new.loaded_start),
                    });
                }
                old_index += 1;
                new_index += 1;
            }
            (Some(old), Some(new)) if old.member_id < new.member_id => {
                actions.push(remove_action(old, MemoryActionReason::RemovedIdentity));
                old_index += 1;
            }
            (Some(_), Some(new)) => {
                actions.push(initialize_action(new, MemoryActionReason::NewIdentity));
                new_index += 1;
            }
            (Some(old), None) => {
                actions.push(remove_action(old, MemoryActionReason::RemovedIdentity));
                old_index += 1;
            }
            (None, Some(new)) => {
                actions.push(initialize_action(new, MemoryActionReason::NewIdentity));
                new_index += 1;
            }
            (None, None) => break,
        }
    }
    actions
}

pub(crate) fn build_state_actions(
    current: Option<&VirtualLoadPackage>,
    candidate: &VirtualLoadPackage,
) -> Vec<StateAction> {
    let Some(current) = current else {
        return candidate
            .state_schema()
            .iter()
            .map(|state| StateAction {
                state_member_id: state.state_member_id,
                runtime_state_id: state.runtime_state_id,
                kind: StateActionKind::Initialize,
            })
            .collect();
    };
    let mut actions = Vec::new();
    let mut old_index = 0;
    let mut new_index = 0;
    while old_index < current.state_schema().len() || new_index < candidate.state_schema().len() {
        let old = current.state_schema().get(old_index);
        let new = candidate.state_schema().get(new_index);
        match (old, new) {
            (Some(old), Some(new)) if old.state_member_id == new.state_member_id => {
                if old.kind == new.kind
                    && old.owner_member_id == new.owner_member_id
                    && old.instance_path == new.instance_path
                    && old.retentive == new.retentive
                {
                    actions.push(StateAction {
                        state_member_id: new.state_member_id,
                        runtime_state_id: new.runtime_state_id,
                        kind: StateActionKind::Preserve,
                    });
                } else {
                    actions.push(StateAction {
                        state_member_id: old.state_member_id,
                        runtime_state_id: old.runtime_state_id,
                        kind: StateActionKind::Remove,
                    });
                    actions.push(StateAction {
                        state_member_id: new.state_member_id,
                        runtime_state_id: new.runtime_state_id,
                        kind: StateActionKind::Initialize,
                    });
                }
                old_index += 1;
                new_index += 1;
            }
            (Some(old), Some(new)) if old.state_member_id < new.state_member_id => {
                actions.push(StateAction {
                    state_member_id: old.state_member_id,
                    runtime_state_id: old.runtime_state_id,
                    kind: StateActionKind::Remove,
                });
                old_index += 1;
            }
            (Some(_), Some(new)) => {
                actions.push(StateAction {
                    state_member_id: new.state_member_id,
                    runtime_state_id: new.runtime_state_id,
                    kind: StateActionKind::Initialize,
                });
                new_index += 1;
            }
            (Some(old), None) => {
                actions.push(StateAction {
                    state_member_id: old.state_member_id,
                    runtime_state_id: old.runtime_state_id,
                    kind: StateActionKind::Remove,
                });
                old_index += 1;
            }
            (None, Some(new)) => {
                actions.push(StateAction {
                    state_member_id: new.state_member_id,
                    runtime_state_id: new.runtime_state_id,
                    kind: StateActionKind::Initialize,
                });
                new_index += 1;
            }
            (None, None) => break,
        }
    }
    actions
}

fn initialize_action(member: &MemoryMemberSchema, reason: MemoryActionReason) -> MemoryAction {
    MemoryAction {
        member_id: member.member_id,
        runtime_memory_id: member.runtime_memory_id,
        kind: MemoryActionKind::Initialize,
        reason,
        old_type: None,
        new_type: Some(member.value_type),
        candidate_loaded_start: Some(member.loaded_start),
    }
}

fn remove_action(member: &MemoryMemberSchema, reason: MemoryActionReason) -> MemoryAction {
    MemoryAction {
        member_id: member.member_id,
        runtime_memory_id: member.runtime_memory_id,
        kind: MemoryActionKind::Remove,
        reason,
        old_type: Some(member.value_type),
        new_type: None,
        candidate_loaded_start: None,
    }
}

fn incompatibility_reason(
    old: &MemoryMemberSchema,
    new: &MemoryMemberSchema,
) -> Option<MemoryActionReason> {
    if old.value_type != new.value_type || old.runtime_memory_id != new.runtime_memory_id {
        Some(MemoryActionReason::TypeChanged)
    } else if old.role != new.role || old.retentive != new.retentive {
        Some(MemoryActionReason::StorageRoleChanged)
    } else if old.instance_path != new.instance_path {
        Some(MemoryActionReason::InstancePathChanged)
    } else {
        None
    }
}

fn memory_shape_equal(old: &[MemoryMemberSchema], new: &[MemoryMemberSchema]) -> bool {
    old.len() == new.len()
        && old.iter().zip(new).all(|(old, new)| {
            old.member_id == new.member_id
                && old.runtime_memory_id == new.runtime_memory_id
                && old.value_type == new.value_type
                && old.role == new.role
                && old.instance_path == new.instance_path
                && old.retentive == new.retentive
        })
}

fn memory_starts_equal(old: &[MemoryMemberSchema], new: &[MemoryMemberSchema]) -> bool {
    old.len() == new.len()
        && old.iter().zip(new).all(|(old, new)| {
            old.member_id == new.member_id && old.loaded_start == new.loaded_start
        })
}

fn state_shape_equal(old: &[StateMemberSchema], new: &[StateMemberSchema]) -> bool {
    old.len() == new.len()
        && old.iter().zip(new).all(|(old, new)| {
            old.state_member_id == new.state_member_id
                && old.runtime_state_id == new.runtime_state_id
                && old.kind == new.kind
                && old.owner_member_id == new.owner_member_id
                && old.instance_path == new.instance_path
                && old.retentive == new.retentive
        })
}

fn hash_preview(preview: &LoadPreview) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-LOAD-PREVIEW-1");
    hasher.u128(preview.target_controller_id.0);
    encode_optional_hash(preview.current_package_fingerprint, &mut hasher);
    hasher.hash(preview.candidate_package_fingerprint);
    encode_optional_hash(preview.current_runtime_fingerprint, &mut hasher);
    hasher.hash(preview.candidate_runtime_fingerprint);
    hasher.hash(preview.target_state_hash);
    hasher.u64(preview.controller_epoch);
    hasher.u64(preview.universe_epoch);
    hasher.u8(preview.cpu_state as u8);
    hasher.u8(preview.compatibility as u8);
    hasher.bool(preview.requires_stop);
    hasher.u64(preview.memory_actions.len() as u64);
    for action in &preview.memory_actions {
        hasher.u128(action.member_id);
        hasher.u32(action.runtime_memory_id.0);
        hasher.u8(action.kind as u8);
        hasher.u8(action.reason as u8);
        encode_optional_value_type(action.old_type, &mut hasher);
        encode_optional_value_type(action.new_type, &mut hasher);
        match action.candidate_loaded_start {
            Some(value) => {
                hasher.bool(true);
                encode_value(value, &mut hasher);
            }
            None => hasher.bool(false),
        }
    }
    hasher.u64(preview.state_actions.len() as u64);
    for action in &preview.state_actions {
        hasher.u128(action.state_member_id);
        hasher.u32(action.runtime_state_id.0);
        hasher.u8(action.kind as u8);
    }
    hasher.bool(preview.hardware_replacement);
    hasher.hash(preview.force_registry_hash);
    hasher.hash(preview.actual_hardware_hash);
    hasher.hash(preview.offline_source_revision_hash);
    encode_optional_hash(preview.offline_build_snapshot_hash, &mut hasher);
    hasher.bool(preview.snapshot_invalidation);
    hasher.bool(preview.opens_new_replay_segment);
    hasher.u64(preview.blockers.len() as u64);
    for blocker in &preview.blockers {
        encode_blocker(blocker, &mut hasher);
    }
    hasher.u64(preview.warnings.len() as u64);
    for warning in &preview.warnings {
        hasher.string(warning);
    }
    hasher.u8(preview.requested_post_load_mode as u8);
    hasher.bool(preview.initialize_compatible_members);
    hasher.u64(preview.valid_through_event_sequence);
    hasher.finish()
}

fn encode_blocker(blocker: &LoadBlocker, hasher: &mut CanonicalHasher) {
    match blocker {
        LoadBlocker::CandidateIntegrity => hasher.u8(1),
        LoadBlocker::CandidateBuildStale => hasher.u8(2),
        LoadBlocker::CandidateHasBlockingDiagnostics(count) => {
            hasher.u8(3);
            hasher.u32(*count);
        }
        LoadBlocker::BuildSnapshotMismatch => hasher.u8(4),
        LoadBlocker::IncompatibleProfile => hasher.u8(5),
        LoadBlocker::IncompatibleRuntimeContract => hasher.u8(6),
        LoadBlocker::CpuMode(state) => {
            hasher.u8(7);
            hasher.u8(*state as u8);
        }
        LoadBlocker::ActiveForces(forces) => {
            hasher.u8(8);
            hasher.u64(forces.len() as u64);
            for force in forces {
                hasher.u128(force.0);
            }
        }
    }
}

fn encode_optional_hash(value: Option<Hash32>, hasher: &mut CanonicalHasher) {
    match value {
        Some(value) => {
            hasher.bool(true);
            hasher.hash(value);
        }
        None => hasher.bool(false),
    }
}

fn encode_optional_value_type(value: Option<ValueType>, hasher: &mut CanonicalHasher) {
    match value {
        Some(value) => {
            hasher.bool(true);
            hasher.u8(value as u8);
        }
        None => hasher.bool(false),
    }
}
