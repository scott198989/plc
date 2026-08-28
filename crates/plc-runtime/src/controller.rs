use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};
use core::{error::Error, fmt};

use crate::{
    ArtifactError, ArtifactPackage, BlockId, CanonicalValue, ChannelDirection, ChannelId,
    DeliveryReason, Hash32, InputCommand, InputReceipt, MAX_WORK_UNITS_PER_SCAN, MemoryId, Operand,
    Operation, ProgramBlock, RUNTIME_SEMANTICS_VERSION, RuntimeActivation, RuntimeBinaryOperator,
    RuntimeBlockCall, RuntimeCallKind, RuntimeDisabledBehavior, RuntimeFormalRef,
    RuntimeFunctionBlockInstance, RuntimeInstructionCode, RuntimeInstructionInstance,
    RuntimeInstructionInvocation, RuntimeInstructionStateKind, RuntimeUnaryOperator,
    SCAN_QUANTUM_MS, SCHEDULER_VERSION, StateId, StateStart, ValueType, VerifiedArtifact,
    WORK_COST_VERSION,
    boundary::{CommandId, UniverseId, VirtualControllerId, VirtualIoBoundary},
    hash::SemanticHasher,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CpuState {
    PoweredOff = 1,
    Stop = 2,
    Startup = 3,
    Run = 4,
    PausedEducational = 5,
    Faulted = 6,
    Resetting = 7,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum RuntimeValueTarget {
    Memory(MemoryId) = 1,
    Input(ChannelId) = 2,
    Output(ChannelId) = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum RuntimePublicationBoundary {
    SerializedCommand = 1,
    ScanBeforeProgram = 2,
    ScanAfterProgram = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeNaturalWrite {
    pub target: RuntimeValueTarget,
    pub value: CanonicalValue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeForceDelta {
    pub target: RuntimeValueTarget,
    pub value: Option<CanonicalValue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeBoundaryCommand {
    pub command_id: u128,
    pub controller_id: VirtualControllerId,
    pub expected_controller_epoch: u64,
    pub expected_artifact_fingerprint: Hash32,
    pub expected_state_hash: Hash32,
    pub natural_writes: Vec<RuntimeNaturalWrite>,
    pub force_deltas: Vec<RuntimeForceDelta>,
    pub audit_context_hash: Hash32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeScanCommand {
    pub command_id: u128,
    pub controller_id: VirtualControllerId,
    pub expected_controller_epoch: u64,
    pub expected_artifact_fingerprint: Hash32,
    pub expected_state_hash: Hash32,
    pub pre_program_writes: Vec<RuntimeNaturalWrite>,
    pub post_program_writes: Vec<RuntimeNaturalWrite>,
    pub force_deltas: Vec<RuntimeForceDelta>,
    pub audit_context_hash: Hash32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeAppliedWrite {
    pub target: RuntimeValueTarget,
    pub boundary: RuntimePublicationBoundary,
    pub prior_natural_value: CanonicalValue,
    pub written_natural_value: CanonicalValue,
    pub visible_effective_value: CanonicalValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeBoundaryReceipt {
    pub command_id: u128,
    pub event_sequence: u64,
    pub virtual_timestamp_ms: u64,
    pub scan_sequence: u64,
    pub cpu_state: CpuState,
    pub writes: Vec<RuntimeAppliedWrite>,
    pub force_overlay_hash: Hash32,
    pub state_hash: Hash32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeScanReceipt {
    pub command_id: u128,
    pub outcome: RunOutcome,
    pub applied_pre_program_writes: Vec<RuntimeAppliedWrite>,
    pub applied_post_program_writes: Vec<RuntimeAppliedWrite>,
    pub force_overlay_hash: Hash32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeForceResetApproval {
    pub controller_id: VirtualControllerId,
    pub expected_controller_epoch: u64,
    pub expected_artifact_fingerprint: Hash32,
    pub expected_force_overlay_hash: Hash32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeBoundaryError {
    WrongController,
    StaleControllerEpoch,
    StaleArtifact,
    StaleState,
    StaleForceOverlay,
    CpuStateDisallowed(CpuState),
    NoLoadedArtifact,
    DuplicateTarget(RuntimeValueTarget),
    UnknownTarget(RuntimeValueTarget),
    TypeMismatch {
        target: RuntimeValueTarget,
        expected: ValueType,
        actual: ValueType,
    },
    InvalidPreProgramTarget(RuntimeValueTarget),
    InvalidPostProgramTarget(RuntimeValueTarget),
}

impl fmt::Display for RuntimeBoundaryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "runtime observation boundary rejected: {self:?}")
    }
}

impl Error for RuntimeBoundaryError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestartKind {
    Resume,
    WarmRestart,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallOutcome {
    Installed {
        old_fingerprint: Option<Hash32>,
        new_fingerprint: Hash32,
    },
    IdenticalNoOp {
        fingerprint: Hash32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeInstallDisposition {
    ArtifactReplacement,
    PackageIdentityOnly,
}

/// A commissioning-approved transfer plan. Stable member and state identities
/// are explicit; every omitted candidate entry is initialized from its loaded
/// start and every omitted old-only entry is removed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeStateTransferPlan {
    expected_current_fingerprint: Hash32,
    candidate_fingerprint: Hash32,
    disposition: RuntimeInstallDisposition,
    preserve_memory: Vec<MemoryId>,
    preserve_states: Vec<StateId>,
    preserve_io: bool,
    preserve_cpu_mode: bool,
}

impl RuntimeStateTransferPlan {
    pub fn new(
        expected_current_fingerprint: Hash32,
        candidate_fingerprint: Hash32,
        disposition: RuntimeInstallDisposition,
        mut preserve_memory: Vec<MemoryId>,
        mut preserve_states: Vec<StateId>,
        preserve_io: bool,
        preserve_cpu_mode: bool,
    ) -> Result<Self, AtomicInstallError> {
        preserve_memory.sort_unstable();
        if preserve_memory.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(AtomicInstallError::DuplicateMemoryIdentity);
        }
        preserve_states.sort_unstable();
        if preserve_states.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(AtomicInstallError::DuplicateStateIdentity);
        }
        Ok(Self {
            expected_current_fingerprint,
            candidate_fingerprint,
            disposition,
            preserve_memory,
            preserve_states,
            preserve_io,
            preserve_cpu_mode,
        })
    }

    pub const fn disposition(&self) -> RuntimeInstallDisposition {
        self.disposition
    }

    pub fn preserved_memory(&self) -> &[MemoryId] {
        &self.preserve_memory
    }

    pub fn preserved_states(&self) -> &[StateId] {
        &self.preserve_states
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AtomicInstallError {
    NoLoadedArtifact,
    Artifact(ArtifactError),
    CurrentFingerprintMismatch,
    CandidateFingerprintMismatch,
    IdentityOnlyRequiresEqualRuntimeArtifact,
    ReplacementRequiresDifferentRuntimeArtifact,
    UnsafeCpuState(CpuState),
    StopRequired,
    DuplicateMemoryIdentity,
    DuplicateStateIdentity,
    IncompatibleMemoryIdentity(MemoryId),
    IncompatibleStateIdentity(StateId),
    IncompatibleIoSchema,
    StaleTarget,
}

impl fmt::Display for AtomicInstallError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "atomic runtime install rejected: {self:?}")
    }
}

impl Error for AtomicInstallError {}

impl From<ArtifactError> for AtomicInstallError {
    fn from(value: ArtifactError) -> Self {
        Self::Artifact(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AtomicInstallReport {
    pub old_fingerprint: Hash32,
    pub new_fingerprint: Hash32,
    pub disposition: RuntimeInstallDisposition,
    pub preserved_memory: Vec<MemoryId>,
    pub initialized_memory_count: usize,
    pub removed_memory_count: usize,
    pub preserved_states: Vec<StateId>,
    pub initialized_state_count: usize,
    pub removed_state_count: usize,
    pub final_cpu_state: CpuState,
    pub controller_epoch: u64,
    pub state_hash: Hash32,
}

/// An owned candidate controller produced entirely outside live target state.
/// Commit succeeds only when the exact target identity, epoch, mode, and state
/// hash still match the staging boundary.
#[derive(Clone, Debug)]
pub struct StagedAtomicInstall {
    controller_id: VirtualControllerId,
    expected_controller_epoch: u64,
    expected_cpu_state: CpuState,
    expected_state_hash: Hash32,
    candidate: VirtualController,
    report: AtomicInstallReport,
}

impl StagedAtomicInstall {
    pub const fn report(&self) -> &AtomicInstallReport {
        &self.report
    }

    pub fn commit(
        self,
        target: &mut VirtualController,
    ) -> Result<AtomicInstallReport, AtomicInstallError> {
        if target.controller_id != self.controller_id
            || target.controller_epoch != self.expected_controller_epoch
            || target.cpu_state != self.expected_cpu_state
            || target.semantic_state_hash() != self.expected_state_hash
        {
            return Err(AtomicInstallError::StaleTarget);
        }
        *target = self.candidate;
        Ok(self.report)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeLifecycleError {
    CloneIdentityUnchanged,
    ControllerEpochExhausted,
    StaleSource,
    StaleTarget,
}

impl fmt::Display for RuntimeLifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "runtime lifecycle action rejected: {self:?}")
    }
}

impl Error for RuntimeLifecycleError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeCloneReport {
    pub source_controller_id: VirtualControllerId,
    pub clone_controller_id: VirtualControllerId,
    pub source_controller_epoch: u64,
    pub clone_controller_epoch: u64,
    pub source_state_hash: Hash32,
    pub clone_state_hash: Hash32,
    pub cpu_state: CpuState,
    pub loaded_fingerprint: Option<Hash32>,
}

/// A complete candidate clone staged outside both the source registry entry
/// and the future clone registry entry. Historical replay and diagnostic
/// records retain their original identities; the clone begins a new segment
/// under its new controller identity.
#[derive(Clone, Debug)]
pub struct StagedControllerClone {
    source_controller_id: VirtualControllerId,
    expected_source_controller_epoch: u64,
    expected_source_state_hash: Hash32,
    candidate: VirtualController,
    report: RuntimeCloneReport,
}

impl StagedControllerClone {
    pub const fn report(&self) -> &RuntimeCloneReport {
        &self.report
    }

    pub fn commit(
        self,
        source: &VirtualController,
    ) -> Result<(VirtualController, RuntimeCloneReport), RuntimeLifecycleError> {
        if source.controller_id != self.source_controller_id
            || source.controller_epoch != self.expected_source_controller_epoch
            || source.semantic_state_hash() != self.expected_source_state_hash
        {
            return Err(RuntimeLifecycleError::StaleSource);
        }
        Ok((self.candidate, self.report))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeReplacementReport {
    pub controller_id: VirtualControllerId,
    pub old_controller_epoch: u64,
    pub new_controller_epoch: u64,
    pub old_state_hash: Hash32,
    pub new_state_hash: Hash32,
    pub old_loaded_fingerprint: Option<Hash32>,
}

/// A blank replacement runtime staged outside the live target. Commit is a
/// single guarded move and therefore cannot expose partially reset state.
#[derive(Clone, Debug)]
pub struct StagedControllerReplacement {
    controller_id: VirtualControllerId,
    expected_controller_epoch: u64,
    expected_state_hash: Hash32,
    candidate: VirtualController,
    report: RuntimeReplacementReport,
}

impl StagedControllerReplacement {
    pub const fn report(&self) -> &RuntimeReplacementReport {
        &self.report
    }

    pub fn commit(
        self,
        target: &mut VirtualController,
    ) -> Result<RuntimeReplacementReport, RuntimeLifecycleError> {
        if target.controller_id != self.controller_id
            || target.controller_epoch != self.expected_controller_epoch
            || target.semantic_state_hash() != self.expected_state_hash
        {
            return Err(RuntimeLifecycleError::StaleTarget);
        }
        *target = self.candidate;
        Ok(self.report)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DiagnosticSeverity {
    Information = 1,
    Warning = 2,
    Error = 3,
    Fatal = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum DiagnosticCode {
    IllegalCpuTransition = 1,
    ArtifactRejected = 2,
    ArithmeticDivideByZero = 100,
    ArithmeticOverflow = 101,
    TimerOverflow = 102,
    WorkUnitBudgetExceeded = 103,
    RuntimeInvariantFailure = 104,
    SnapshotRejected = 200,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaultContext {
    pub artifact_fingerprint: Hash32,
    pub block_id: BlockId,
    pub operation_id: u32,
    pub source_identity: u128,
    pub scan_sequence: u64,
    pub controller_epoch: u64,
    pub virtual_timestamp_ms: u64,
    pub work_units_before_operation: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticEvent {
    pub occurrence_id: u128,
    pub parent_occurrence_id: Option<u128>,
    pub root_occurrence_id: u128,
    pub code: DiagnosticCode,
    pub severity: DiagnosticSeverity,
    pub universe_epoch: u64,
    pub controller_epoch: u64,
    pub event_sequence: u64,
    pub virtual_timestamp_ms: u64,
    pub fault_context: Option<FaultContext>,
    pub fault_boundary_state_hash: Option<Hash32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ReplayEventKind {
    PowerOn = 1,
    PowerOff = 2,
    ArtifactInstalled = 3,
    RequestRun = 4,
    RequestStop = 5,
    Pause = 6,
    Resume = 7,
    RawInputAccepted = 8,
    ScanCompleted = 9,
    FatalFault = 10,
    FaultReset = 11,
    WarmRestart = 12,
    PowerCycle = 13,
    MemoryReset = 14,
    SnapshotRestored = 15,
    CommandRejected = 16,
    InstanceCloned = 17,
    InstanceReplaced = 18,
    ObservationBoundary = 19,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReplaySegment {
    pub universe_id: UniverseId,
    pub universe_epoch: u64,
    pub controller_id: VirtualControllerId,
    pub controller_epoch: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayEvent {
    pub segment: ReplaySegment,
    pub kind: ReplayEventKind,
    pub event_sequence: u64,
    pub virtual_timestamp_ms: u64,
    pub payload_hash: Hash32,
    pub result_hash: Hash32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
enum BoundaryKind {
    ScanEnd = 1,
    FatalFault = 2,
    SnapshotRestore = 3,
    Lifecycle = 4,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundaryHash {
    pub segment: ReplaySegment,
    pub scan_sequence: u64,
    pub virtual_timestamp_ms: u64,
    kind: BoundaryKind,
    pub state_hash: Hash32,
}

impl BoundaryHash {
    pub const fn is_scan_end(&self) -> bool {
        matches!(self.kind, BoundaryKind::ScanEnd)
    }

    pub const fn is_fatal_fault(&self) -> bool {
        matches!(self.kind, BoundaryKind::FatalFault)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanReport {
    pub scan_sequence: u64,
    pub scan_start_time_ms: u64,
    pub completed_time_ms: u64,
    pub work_units: u32,
    pub executed_blocks: Vec<BlockId>,
    pub call_boundaries: Vec<CallBoundaryEvent>,
    pub output_event_sequence: u64,
    pub state_hash: Hash32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallBoundaryKind {
    Enter,
    Return,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallBoundaryEvent {
    pub kind: CallBoundaryKind,
    pub caller_block: BlockId,
    pub callee_block: BlockId,
    pub call_operation_id: u32,
    pub source_identity: u128,
    pub call_site_identity: u128,
    pub dynamic_depth: u8,
    pub instance: Option<RuntimeFunctionBlockInstance>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunOutcome {
    Completed(ScanReport),
    Faulted(DiagnosticEvent),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandError {
    IllegalCpuTransition {
        from: CpuState,
        action: &'static str,
    },
    NoLoadedArtifact,
    Artifact(ArtifactError),
    WrongController,
    StaleControllerEpoch {
        expected: u64,
        actual: u64,
    },
    UnknownInputChannel(ChannelId),
    InputTypeMismatch {
        expected: ValueType,
        actual: ValueType,
    },
    IdempotencyCollision {
        key: u128,
    },
}

impl fmt::Display for CommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "runtime command rejected: {self:?}")
    }
}

impl Error for CommandError {}

impl From<ArtifactError> for CommandError {
    fn from(value: ArtifactError) -> Self {
        Self::Artifact(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SnapshotError {
    UnsafeCaptureState(CpuState),
    UnsafeRestoreState(CpuState),
    WrongController,
    IntegrityMismatch { declared: Hash32, actual: Hash32 },
    ArtifactUnavailable(Hash32),
    IncompatibleRuntime,
    ApprovalMismatch,
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "snapshot action rejected: {self:?}")
    }
}

impl Error for SnapshotError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RestoreApproval {
    pub snapshot_hash: Hash32,
    pub controller_id: VirtualControllerId,
    pub expected_universe_epoch: u64,
    pub expected_controller_epoch: u64,
    pub expected_current_state_hash: Hash32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeStateCell {
    Edge { previous: bool },
    Timer { elapsed_ms: u64, output: bool },
    Counter { count: i32, previous_input: bool },
}

impl From<StateStart> for RuntimeStateCell {
    fn from(value: StateStart) -> Self {
        match value {
            StateStart::Edge { previous } => Self::Edge { previous },
            StateStart::Timer { elapsed_ms, output } => Self::Timer { elapsed_ms, output },
            StateStart::Counter {
                count,
                previous_input,
            } => Self::Counter {
                count,
                previous_input,
            },
        }
    }
}

impl RuntimeStateCell {
    fn encode(self, hasher: &mut SemanticHasher) {
        match self {
            Self::Edge { previous } => {
                hasher.u8(1);
                hasher.bool(previous);
            }
            Self::Timer { elapsed_ms, output } => {
                hasher.u8(2);
                hasher.u64(elapsed_ms);
                hasher.bool(output);
            }
            Self::Counter {
                count,
                previous_input,
            } => {
                hasher.u8(3);
                hasher.i32(count);
                hasher.bool(previous_input);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct InvocationStateKey {
    scope: Vec<u128>,
    stable_id: u128,
    kind: RuntimeInstructionStateKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InvocationStateCell {
    Edge {
        previous: bool,
    },
    Timer {
        elapsed_ms: u64,
        output: bool,
        previous_input: bool,
    },
    Counter {
        count: i32,
        previous_up: bool,
        previous_down: bool,
        output_up: bool,
        output_down: bool,
    },
}

impl InvocationStateCell {
    const fn initial(kind: RuntimeInstructionStateKind) -> Self {
        match kind {
            RuntimeInstructionStateKind::Edge => Self::Edge { previous: false },
            RuntimeInstructionStateKind::Timer => Self::Timer {
                elapsed_ms: 0,
                output: false,
                previous_input: false,
            },
            RuntimeInstructionStateKind::Counter => Self::Counter {
                count: 0,
                previous_up: false,
                previous_down: false,
                output_up: false,
                output_down: true,
            },
        }
    }

    fn encode(self, hasher: &mut SemanticHasher) {
        match self {
            Self::Edge { previous } => {
                hasher.u8(1);
                hasher.bool(previous);
            }
            Self::Timer {
                elapsed_ms,
                output,
                previous_input,
            } => {
                hasher.u8(2);
                hasher.u64(elapsed_ms);
                hasher.bool(output);
                hasher.bool(previous_input);
            }
            Self::Counter {
                count,
                previous_up,
                previous_down,
                output_up,
                output_down,
            } => {
                hasher.u8(3);
                hasher.i32(count);
                hasher.bool(previous_up);
                hasher.bool(previous_down);
                hasher.bool(output_up);
                hasher.bool(output_down);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InvocationResult {
    Value(CanonicalValue),
    Suppressed,
}

#[derive(Clone, Debug, Default)]
struct BlockExecution {
    frame_memory: Option<BTreeMap<MemoryId, CanonicalValue>>,
    suppressed_memory: BTreeSet<MemoryId>,
    invocation_results: BTreeMap<(u32, RuntimeFormalRef), InvocationResult>,
    state_scope: Vec<u128>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeImage {
    actual_memory: BTreeMap<MemoryId, CanonicalValue>,
    retain_memory: BTreeMap<MemoryId, CanonicalValue>,
    natural_inputs: BTreeMap<ChannelId, CanonicalValue>,
    effective_inputs: BTreeMap<ChannelId, CanonicalValue>,
    natural_outputs: BTreeMap<ChannelId, CanonicalValue>,
    effective_outputs: BTreeMap<ChannelId, CanonicalValue>,
    force_overlays: BTreeMap<RuntimeValueTarget, CanonicalValue>,
    state_cells: BTreeMap<StateId, RuntimeStateCell>,
    retain_state_cells: BTreeMap<StateId, RuntimeStateCell>,
    invocation_state_cells: BTreeMap<InvocationStateKey, InvocationStateCell>,
    retain_invocation_state_cells: BTreeMap<InvocationStateKey, InvocationStateCell>,
    function_block_instances:
        BTreeMap<RuntimeFunctionBlockInstance, BTreeMap<MemoryId, CanonicalValue>>,
    retain_function_block_instances:
        BTreeMap<RuntimeFunctionBlockInstance, BTreeMap<MemoryId, CanonicalValue>>,
    invocation_ordinals: BTreeMap<BlockId, u64>,
}

impl RuntimeImage {
    fn empty() -> Self {
        Self {
            actual_memory: BTreeMap::new(),
            retain_memory: BTreeMap::new(),
            natural_inputs: BTreeMap::new(),
            effective_inputs: BTreeMap::new(),
            natural_outputs: BTreeMap::new(),
            effective_outputs: BTreeMap::new(),
            force_overlays: BTreeMap::new(),
            state_cells: BTreeMap::new(),
            retain_state_cells: BTreeMap::new(),
            invocation_state_cells: BTreeMap::new(),
            retain_invocation_state_cells: BTreeMap::new(),
            function_block_instances: BTreeMap::new(),
            retain_function_block_instances: BTreeMap::new(),
            invocation_ordinals: BTreeMap::new(),
        }
    }

    fn from_artifact(artifact: &VerifiedArtifact) -> Self {
        let spec = artifact.spec();
        let mut image = Self::empty();
        for definition in &spec.memory {
            image
                .actual_memory
                .insert(definition.id, definition.loaded_start);
            if definition.retentive {
                image
                    .retain_memory
                    .insert(definition.id, definition.loaded_start);
            }
        }
        for channel in &spec.channels {
            match channel.direction {
                ChannelDirection::Input => {
                    image
                        .natural_inputs
                        .insert(channel.id, channel.canonical_default);
                    image
                        .effective_inputs
                        .insert(channel.id, channel.canonical_default);
                }
                ChannelDirection::Output => {
                    image
                        .natural_outputs
                        .insert(channel.id, channel.canonical_default);
                    image
                        .effective_outputs
                        .insert(channel.id, channel.canonical_default);
                }
            }
        }
        for definition in &spec.states {
            let state = RuntimeStateCell::from(definition.loaded_start);
            image.state_cells.insert(definition.id, state);
            if definition.retentive {
                image.retain_state_cells.insert(definition.id, state);
            }
        }
        if let Some(block) = &spec.program.startup {
            collect_invocation_ordinals(block, &mut image.invocation_ordinals);
        }
        for task in &spec.program.timed {
            collect_invocation_ordinals(&task.block, &mut image.invocation_ordinals);
        }
        collect_invocation_ordinals(&spec.program.cyclic, &mut image.invocation_ordinals);
        image
    }

    fn encode(&self, hasher: &mut SemanticHasher) {
        encode_value_map(&self.actual_memory, hasher);
        encode_value_map(&self.retain_memory, hasher);
        encode_value_map(&self.natural_inputs, hasher);
        encode_value_map(&self.effective_inputs, hasher);
        encode_value_map(&self.natural_outputs, hasher);
        encode_value_map(&self.effective_outputs, hasher);
        if !self.force_overlays.is_empty() {
            hasher.string("PES-RUNTIME-FORCE-OVERLAYS-1");
            hasher.u64(self.force_overlays.len() as u64);
            for (target, value) in &self.force_overlays {
                encode_runtime_target(*target, hasher);
                value.encode(hasher);
            }
        }
        encode_state_map(&self.state_cells, hasher);
        encode_state_map(&self.retain_state_cells, hasher);
        if !self.invocation_state_cells.is_empty()
            || !self.retain_invocation_state_cells.is_empty()
            || !self.function_block_instances.is_empty()
            || !self.retain_function_block_instances.is_empty()
        {
            hasher.string("PES-RUNTIME-CALL-STATE-1");
            encode_invocation_state_map(&self.invocation_state_cells, hasher);
            encode_invocation_state_map(&self.retain_invocation_state_cells, hasher);
            encode_function_block_instances(&self.function_block_instances, hasher);
            encode_function_block_instances(&self.retain_function_block_instances, hasher);
        }
        hasher.u64(self.invocation_ordinals.len() as u64);
        for (id, ordinal) in &self.invocation_ordinals {
            hasher.u32(id.0);
            hasher.u64(*ordinal);
        }
    }
}

fn collect_invocation_ordinals(block: &ProgramBlock, ordinals: &mut BTreeMap<BlockId, u64>) {
    ordinals.entry(block.id).or_insert(0);
    for instruction in &block.instructions {
        if let Operation::CallBlock(call) = instruction.operation() {
            collect_invocation_ordinals(&call.callee, ordinals);
        }
    }
}

fn encode_invocation_state_map(
    map: &BTreeMap<InvocationStateKey, InvocationStateCell>,
    hasher: &mut SemanticHasher,
) {
    hasher.u64(map.len() as u64);
    for (key, value) in map {
        hasher.u64(key.scope.len() as u64);
        for item in &key.scope {
            hasher.u128(*item);
        }
        hasher.u128(key.stable_id);
        hasher.u8(match key.kind {
            RuntimeInstructionStateKind::Edge => 1,
            RuntimeInstructionStateKind::Timer => 2,
            RuntimeInstructionStateKind::Counter => 3,
        });
        value.encode(hasher);
    }
}

fn encode_function_block_instances(
    map: &BTreeMap<RuntimeFunctionBlockInstance, BTreeMap<MemoryId, CanonicalValue>>,
    hasher: &mut SemanticHasher,
) {
    hasher.u64(map.len() as u64);
    for (instance, values) in map {
        hasher.u128(instance.root_instance);
        hasher.u64(instance.multi_instance_slots.len() as u64);
        for slot in &instance.multi_instance_slots {
            hasher.u128(*slot);
        }
        encode_value_map(values, hasher);
    }
}

fn encode_value_map<K>(map: &BTreeMap<K, CanonicalValue>, hasher: &mut SemanticHasher)
where
    K: Copy + Ord + IntoU32,
{
    hasher.u64(map.len() as u64);
    for (id, value) in map {
        hasher.u32((*id).into_u32());
        value.encode(hasher);
    }
}

trait IntoU32 {
    fn into_u32(self) -> u32;
}

impl IntoU32 for MemoryId {
    fn into_u32(self) -> u32 {
        self.0
    }
}

impl IntoU32 for ChannelId {
    fn into_u32(self) -> u32 {
        self.0
    }
}

fn encode_runtime_target(target: RuntimeValueTarget, hasher: &mut SemanticHasher) {
    match target {
        RuntimeValueTarget::Memory(id) => {
            hasher.u8(1);
            hasher.u32(id.0);
        }
        RuntimeValueTarget::Input(id) => {
            hasher.u8(2);
            hasher.u32(id.0);
        }
        RuntimeValueTarget::Output(id) => {
            hasher.u8(3);
            hasher.u32(id.0);
        }
    }
}

fn encode_state_map(map: &BTreeMap<StateId, RuntimeStateCell>, hasher: &mut SemanticHasher) {
    hasher.u64(map.len() as u64);
    for (id, state) in map {
        hasher.u32(id.0);
        state.encode(hasher);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StoredInputReceipt {
    payload_hash: Hash32,
    receipt: InputReceipt,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SnapshotBody {
    universe_id: UniverseId,
    captured_universe_epoch: u64,
    controller_id: VirtualControllerId,
    captured_controller_epoch: u64,
    captured_cpu_state: CpuState,
    virtual_time_ms: u64,
    captured_scan_sequence: u64,
    captured_event_sequence: u64,
    artifact_fingerprint: Hash32,
    profile_fingerprint: Hash32,
    runtime_version: &'static str,
    scheduler_version: &'static str,
    work_cost_version: &'static str,
    deterministic_seed: u64,
    image: RuntimeImage,
    boundary: VirtualIoBoundary,
    diagnostics: Vec<DiagnosticEvent>,
    input_receipts: BTreeMap<u128, StoredInputReceipt>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControllerSnapshot {
    schema_version: u32,
    body: SnapshotBody,
    content_hash: Hash32,
}

impl ControllerSnapshot {
    pub const fn content_hash(&self) -> Hash32 {
        self.content_hash
    }

    pub const fn captured_cpu_state(&self) -> CpuState {
        self.body.captured_cpu_state
    }

    pub const fn captured_universe_epoch(&self) -> u64 {
        self.body.captured_universe_epoch
    }

    pub const fn captured_controller_epoch(&self) -> u64 {
        self.body.captured_controller_epoch
    }

    fn calculate_hash(&self) -> Hash32 {
        hash_snapshot(self.schema_version, &self.body)
    }
}

#[derive(Clone, Debug)]
pub struct VirtualController {
    universe_id: UniverseId,
    universe_epoch: u64,
    controller_id: VirtualControllerId,
    controller_epoch: u64,
    cpu_state: CpuState,
    virtual_time_ms: u64,
    scan_sequence: u64,
    event_sequence: u64,
    deterministic_seed: u64,
    loaded: Option<VerifiedArtifact>,
    image: RuntimeImage,
    boundary: VirtualIoBoundary,
    diagnostics: Vec<DiagnosticEvent>,
    replay_events: Vec<ReplayEvent>,
    boundary_hashes: Vec<BoundaryHash>,
    input_receipts: BTreeMap<u128, StoredInputReceipt>,
    last_state_hash: Hash32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExecutionFault {
    DivideByZero,
    ArithmeticOverflow,
    TimerOverflow,
    WorkUnitBudgetExceeded,
    RuntimeInvariant,
}

const FORMAL_INPUT: u16 = 0x0010;
const FORMAL_OUTPUT: u16 = 0x0011;
const FORMAL_LEFT: u16 = 0x0020;
const FORMAL_RIGHT: u16 = 0x0021;
const FORMAL_CLOCK: u16 = 0x0030;
const FORMAL_PRESET_TIME: u16 = 0x0031;
const FORMAL_ELAPSED_TIME: u16 = 0x0032;
const FORMAL_COUNT_UP: u16 = 0x0040;
const FORMAL_COUNT_DOWN: u16 = 0x0041;
const FORMAL_RESET: u16 = 0x0042;
const FORMAL_LOAD: u16 = 0x0043;
const FORMAL_PRESET_VALUE: u16 = 0x0044;
const FORMAL_CURRENT_VALUE: u16 = 0x0045;
const FORMAL_QU: u16 = 0x0046;
const FORMAL_QD: u16 = 0x0047;

impl VirtualController {
    pub fn new(
        universe_id: UniverseId,
        controller_id: VirtualControllerId,
        deterministic_seed: u64,
    ) -> Self {
        let mut controller = Self {
            universe_id,
            universe_epoch: 1,
            controller_id,
            controller_epoch: 1,
            cpu_state: CpuState::PoweredOff,
            virtual_time_ms: 0,
            scan_sequence: 0,
            event_sequence: 0,
            deterministic_seed,
            loaded: None,
            image: RuntimeImage::empty(),
            boundary: VirtualIoBoundary::empty(controller_id),
            diagnostics: Vec::new(),
            replay_events: Vec::new(),
            boundary_hashes: Vec::new(),
            input_receipts: BTreeMap::new(),
            last_state_hash: Hash32::ZERO,
        };
        controller.last_state_hash = controller.semantic_state_hash();
        controller
    }

    pub const fn universe_id(&self) -> UniverseId {
        self.universe_id
    }
    pub const fn universe_epoch(&self) -> u64 {
        self.universe_epoch
    }
    pub const fn controller_id(&self) -> VirtualControllerId {
        self.controller_id
    }
    pub const fn controller_epoch(&self) -> u64 {
        self.controller_epoch
    }
    pub const fn cpu_state(&self) -> CpuState {
        self.cpu_state
    }
    pub const fn virtual_time_ms(&self) -> u64 {
        self.virtual_time_ms
    }
    pub const fn scan_sequence(&self) -> u64 {
        self.scan_sequence
    }
    pub const fn event_sequence(&self) -> u64 {
        self.event_sequence
    }
    pub const fn deterministic_seed(&self) -> u64 {
        self.deterministic_seed
    }
    pub const fn last_state_hash(&self) -> Hash32 {
        self.last_state_hash
    }

    pub fn loaded_fingerprint(&self) -> Option<Hash32> {
        self.loaded.as_ref().map(VerifiedArtifact::fingerprint)
    }

    pub const fn boundary(&self) -> &VirtualIoBoundary {
        &self.boundary
    }
    pub fn diagnostics(&self) -> &[DiagnosticEvent] {
        &self.diagnostics
    }
    pub fn replay_events(&self) -> &[ReplayEvent] {
        &self.replay_events
    }
    pub fn boundary_hashes(&self) -> &[BoundaryHash] {
        &self.boundary_hashes
    }

    pub fn stage_reidentified_clone(
        &self,
        clone_controller_id: VirtualControllerId,
    ) -> Result<StagedControllerClone, RuntimeLifecycleError> {
        if clone_controller_id == self.controller_id {
            return Err(RuntimeLifecycleError::CloneIdentityUnchanged);
        }

        let source_state_hash = self.semantic_state_hash();
        let mut candidate = self.clone();
        candidate.controller_id = clone_controller_id;
        candidate.controller_epoch = 1;
        candidate.boundary.controller_id = clone_controller_id;
        candidate.input_receipts.clear();
        let sequence = candidate.next_event_sequence();
        let mut payload = SemanticHasher::new("PES-CONTROLLER-CLONE-1");
        payload.u128(self.controller_id.0);
        payload.u128(clone_controller_id.0);
        payload.u64(self.controller_epoch);
        payload.hash(source_state_hash);
        candidate.append_replay(
            ReplayEventKind::InstanceCloned,
            sequence,
            candidate.virtual_time_ms,
            payload.finish(),
            Hash32::ZERO,
        );
        candidate.finish_lifecycle_boundary();

        let report = RuntimeCloneReport {
            source_controller_id: self.controller_id,
            clone_controller_id,
            source_controller_epoch: self.controller_epoch,
            clone_controller_epoch: candidate.controller_epoch,
            source_state_hash,
            clone_state_hash: candidate.last_state_hash,
            cpu_state: candidate.cpu_state,
            loaded_fingerprint: candidate.loaded_fingerprint(),
        };
        Ok(StagedControllerClone {
            source_controller_id: self.controller_id,
            expected_source_controller_epoch: self.controller_epoch,
            expected_source_state_hash: source_state_hash,
            candidate,
            report,
        })
    }

    pub fn stage_blank_replacement(
        &self,
    ) -> Result<StagedControllerReplacement, RuntimeLifecycleError> {
        let new_controller_epoch = self
            .controller_epoch
            .checked_add(1)
            .ok_or(RuntimeLifecycleError::ControllerEpochExhausted)?;
        let old_state_hash = self.semantic_state_hash();
        let mut candidate = Self::new(
            self.universe_id,
            self.controller_id,
            self.deterministic_seed,
        );
        candidate.universe_epoch = self.universe_epoch;
        candidate.controller_epoch = new_controller_epoch;
        candidate.event_sequence = self.event_sequence;
        candidate.replay_events = self.replay_events.clone();
        candidate.boundary_hashes = self.boundary_hashes.clone();
        let sequence = candidate.next_event_sequence();
        let mut payload = SemanticHasher::new("PES-CONTROLLER-REPLACEMENT-1");
        payload.u128(self.controller_id.0);
        payload.u64(self.controller_epoch);
        payload.u64(new_controller_epoch);
        payload.hash(old_state_hash);
        encode_optional_hash(self.loaded_fingerprint(), &mut payload);
        candidate.append_replay(
            ReplayEventKind::InstanceReplaced,
            sequence,
            candidate.virtual_time_ms,
            payload.finish(),
            Hash32::ZERO,
        );
        candidate.finish_lifecycle_boundary();

        let report = RuntimeReplacementReport {
            controller_id: self.controller_id,
            old_controller_epoch: self.controller_epoch,
            new_controller_epoch,
            old_state_hash,
            new_state_hash: candidate.last_state_hash,
            old_loaded_fingerprint: self.loaded_fingerprint(),
        };
        Ok(StagedControllerReplacement {
            controller_id: self.controller_id,
            expected_controller_epoch: self.controller_epoch,
            expected_state_hash: old_state_hash,
            candidate,
            report,
        })
    }

    pub fn actual_memory(&self, id: MemoryId) -> Option<CanonicalValue> {
        self.image.actual_memory.get(&id).copied()
    }

    pub fn retained_memory(&self, id: MemoryId) -> Option<CanonicalValue> {
        self.image.retain_memory.get(&id).copied()
    }

    pub fn natural_input(&self, id: ChannelId) -> Option<CanonicalValue> {
        self.image.natural_inputs.get(&id).copied()
    }

    pub fn effective_input(&self, id: ChannelId) -> Option<CanonicalValue> {
        self.image.effective_inputs.get(&id).copied()
    }

    pub fn natural_output(&self, id: ChannelId) -> Option<CanonicalValue> {
        self.image.natural_outputs.get(&id).copied()
    }

    pub fn effective_output(&self, id: ChannelId) -> Option<CanonicalValue> {
        self.image.effective_outputs.get(&id).copied()
    }

    pub fn natural_value(&self, target: RuntimeValueTarget) -> Option<CanonicalValue> {
        match target {
            RuntimeValueTarget::Memory(id) => self.actual_memory(id),
            RuntimeValueTarget::Input(id) => self.natural_input(id),
            RuntimeValueTarget::Output(id) => self.natural_output(id),
        }
    }

    pub fn effective_value(&self, target: RuntimeValueTarget) -> Option<CanonicalValue> {
        if let Some(value) = self.image.force_overlays.get(&target) {
            return Some(*value);
        }
        self.natural_value(target)
    }

    pub fn target_value_type(&self, target: RuntimeValueTarget) -> Option<ValueType> {
        let artifact = self.loaded.as_ref()?;
        match target {
            RuntimeValueTarget::Memory(id) => artifact
                .spec()
                .memory
                .binary_search_by_key(&id, |definition| definition.id)
                .ok()
                .map(|index| artifact.spec().memory[index].value_type),
            RuntimeValueTarget::Input(id) => artifact
                .spec()
                .channels
                .binary_search_by_key(&id, |definition| definition.id)
                .ok()
                .and_then(|index| {
                    let channel = &artifact.spec().channels[index];
                    (channel.direction == ChannelDirection::Input).then_some(channel.value_type)
                }),
            RuntimeValueTarget::Output(id) => artifact
                .spec()
                .channels
                .binary_search_by_key(&id, |definition| definition.id)
                .ok()
                .and_then(|index| {
                    let channel = &artifact.spec().channels[index];
                    (channel.direction == ChannelDirection::Output).then_some(channel.value_type)
                }),
        }
    }

    pub fn force_overlay(&self, target: RuntimeValueTarget) -> Option<CanonicalValue> {
        self.image.force_overlays.get(&target).copied()
    }

    pub fn force_overlays(&self) -> &BTreeMap<RuntimeValueTarget, CanonicalValue> {
        &self.image.force_overlays
    }

    pub fn force_overlay_hash(&self) -> Hash32 {
        hash_force_overlays(&self.image.force_overlays)
    }

    pub fn projected_force_overlay_hash(
        &self,
        deltas: &[RuntimeForceDelta],
    ) -> Result<Hash32, RuntimeBoundaryError> {
        self.validate_force_deltas(deltas)?;
        let mut overlays = self.image.force_overlays.clone();
        for delta in deltas {
            if let Some(value) = delta.value {
                overlays.insert(delta.target, value);
            } else {
                overlays.remove(&delta.target);
            }
        }
        Ok(hash_force_overlays(&overlays))
    }

    pub fn apply_observation_boundary(
        &mut self,
        command: &RuntimeBoundaryCommand,
    ) -> Result<RuntimeBoundaryReceipt, RuntimeBoundaryError> {
        self.validate_observation_identity(
            command.controller_id,
            command.expected_controller_epoch,
            command.expected_artifact_fingerprint,
            command.expected_state_hash,
        )?;
        match self.cpu_state {
            CpuState::Stop | CpuState::PausedEducational => {}
            CpuState::Faulted
                if command.natural_writes.is_empty()
                    && command
                        .force_deltas
                        .iter()
                        .all(|delta| delta.value.is_none()) => {}
            state => return Err(RuntimeBoundaryError::CpuStateDisallowed(state)),
        }
        self.validate_natural_writes(&command.natural_writes)?;
        self.validate_force_deltas(&command.force_deltas)?;

        let mut candidate = self.clone();
        let prior_values: Vec<_> = command
            .natural_writes
            .iter()
            .map(|write| {
                (
                    write.target,
                    candidate
                        .natural_value(write.target)
                        .expect("validated target has a natural value"),
                )
            })
            .collect();
        candidate.apply_force_deltas(&command.force_deltas);
        for write in &command.natural_writes {
            candidate.apply_natural_write(*write);
        }
        candidate.refresh_effective_layers();

        let sequence = candidate.next_event_sequence();
        candidate.append_replay(
            ReplayEventKind::ObservationBoundary,
            sequence,
            candidate.virtual_time_ms,
            hash_runtime_boundary_command(command),
            Hash32::ZERO,
        );
        candidate.finish_lifecycle_boundary();
        let writes = command
            .natural_writes
            .iter()
            .zip(prior_values)
            .map(
                |(write, (target, prior_natural_value))| RuntimeAppliedWrite {
                    target,
                    boundary: RuntimePublicationBoundary::SerializedCommand,
                    prior_natural_value,
                    written_natural_value: write.value,
                    visible_effective_value: candidate
                        .effective_value(target)
                        .expect("validated target has an effective value"),
                },
            )
            .collect();
        let receipt = RuntimeBoundaryReceipt {
            command_id: command.command_id,
            event_sequence: sequence,
            virtual_timestamp_ms: candidate.virtual_time_ms,
            scan_sequence: candidate.scan_sequence,
            cpu_state: candidate.cpu_state,
            writes,
            force_overlay_hash: candidate.force_overlay_hash(),
            state_hash: candidate.last_state_hash,
        };
        *self = candidate;
        Ok(receipt)
    }

    pub fn clear_force_overlays_for_reset(
        &mut self,
        approval: RuntimeForceResetApproval,
    ) -> Result<Hash32, RuntimeBoundaryError> {
        if approval.controller_id != self.controller_id {
            return Err(RuntimeBoundaryError::WrongController);
        }
        if approval.expected_controller_epoch != self.controller_epoch {
            return Err(RuntimeBoundaryError::StaleControllerEpoch);
        }
        if self.loaded_fingerprint() != Some(approval.expected_artifact_fingerprint) {
            return Err(RuntimeBoundaryError::StaleArtifact);
        }
        if approval.expected_force_overlay_hash != self.force_overlay_hash() {
            return Err(RuntimeBoundaryError::StaleForceOverlay);
        }
        if !matches!(
            self.cpu_state,
            CpuState::Stop | CpuState::PausedEducational | CpuState::Faulted
        ) {
            return Err(RuntimeBoundaryError::CpuStateDisallowed(self.cpu_state));
        }

        self.image.force_overlays.clear();
        self.refresh_effective_layers();
        let sequence = self.next_event_sequence();
        let mut payload = SemanticHasher::new("PES-OBSERVATION-FORCE-RESET-1");
        payload.u128(self.controller_id.0);
        payload.u64(self.controller_epoch);
        payload.hash(approval.expected_artifact_fingerprint);
        payload.hash(approval.expected_force_overlay_hash);
        self.append_replay(
            ReplayEventKind::ObservationBoundary,
            sequence,
            self.virtual_time_ms,
            payload.finish(),
            Hash32::ZERO,
        );
        self.finish_lifecycle_boundary();
        Ok(self.last_state_hash)
    }

    pub fn invocation_ordinal(&self, block: BlockId) -> Option<u64> {
        self.image.invocation_ordinals.get(&block).copied()
    }

    pub fn power_on(&mut self) -> Result<(), CommandError> {
        if self.cpu_state != CpuState::PoweredOff {
            return self.reject_transition("Power On");
        }
        let sequence = self.next_event_sequence();
        self.cpu_state = CpuState::Stop;
        self.deliver_mode_defaults(sequence, DeliveryReason::CpuModeDefault);
        let payload = hash_lifecycle("POWER_ON", self.cpu_state, self.controller_epoch);
        self.append_replay(
            ReplayEventKind::PowerOn,
            sequence,
            self.virtual_time_ms,
            payload,
            Hash32::ZERO,
        );
        self.finish_lifecycle_boundary();
        Ok(())
    }

    pub fn power_off(&mut self) -> Result<(), CommandError> {
        if matches!(self.cpu_state, CpuState::Startup | CpuState::Resetting) {
            return self.reject_transition("Power Off");
        }
        if self.cpu_state == CpuState::PoweredOff {
            return Ok(());
        }
        let sequence = self.next_event_sequence();
        self.cpu_state = CpuState::PoweredOff;
        self.deliver_mode_defaults(sequence, DeliveryReason::CpuModeDefault);
        let payload = hash_lifecycle("POWER_OFF", self.cpu_state, self.controller_epoch);
        self.append_replay(
            ReplayEventKind::PowerOff,
            sequence,
            self.virtual_time_ms,
            payload,
            Hash32::ZERO,
        );
        self.finish_lifecycle_boundary();
        Ok(())
    }

    pub fn install_verified_artifact(
        &mut self,
        package: &ArtifactPackage,
    ) -> Result<InstallOutcome, CommandError> {
        if self.cpu_state != CpuState::Stop {
            return self.reject_transition("Install Artifact");
        }
        let candidate = match VerifiedArtifact::accept(package) {
            Ok(candidate) => candidate,
            Err(error) => {
                self.emit_nonfatal_diagnostic(DiagnosticCode::ArtifactRejected);
                return Err(CommandError::Artifact(error));
            }
        };
        if self.loaded_fingerprint() == Some(candidate.fingerprint()) {
            return Ok(InstallOutcome::IdenticalNoOp {
                fingerprint: candidate.fingerprint(),
            });
        }

        // All candidate construction and validation completed above. From this
        // point the replacement is one infallible in-memory commit.
        let old_fingerprint = self.loaded_fingerprint();
        let mut new_image = RuntimeImage::from_artifact(&candidate);
        let new_epoch = self.controller_epoch.saturating_add(1);
        let sequence = self.next_event_sequence();
        let new_boundary =
            VirtualIoBoundary::configured(self.controller_id, &candidate.spec().channels, sequence);
        for channel in &candidate.spec().channels {
            match channel.direction {
                ChannelDirection::Input => {
                    new_image
                        .natural_inputs
                        .insert(channel.id, channel.canonical_default);
                    new_image
                        .effective_inputs
                        .insert(channel.id, channel.canonical_default);
                }
                ChannelDirection::Output => {
                    new_image
                        .effective_outputs
                        .insert(channel.id, channel.canonical_default);
                }
            }
        }
        self.controller_epoch = new_epoch;
        self.scan_sequence = 0;
        self.loaded = Some(candidate);
        self.image = new_image;
        self.boundary = new_boundary;
        self.input_receipts.clear();

        let new_fingerprint = self.loaded_fingerprint().expect("artifact just installed");
        let mut payload = SemanticHasher::new("PES-INSTALL-1");
        encode_optional_hash(old_fingerprint, &mut payload);
        payload.hash(new_fingerprint);
        payload.u64(self.controller_epoch);
        self.append_replay(
            ReplayEventKind::ArtifactInstalled,
            sequence,
            self.virtual_time_ms,
            payload.finish(),
            new_fingerprint,
        );
        self.finish_lifecycle_boundary();
        Ok(InstallOutcome::Installed {
            old_fingerprint,
            new_fingerprint,
        })
    }

    pub fn stage_atomic_install(
        &self,
        package: &ArtifactPackage,
        plan: &RuntimeStateTransferPlan,
    ) -> Result<StagedAtomicInstall, AtomicInstallError> {
        let current = self
            .loaded
            .as_ref()
            .ok_or(AtomicInstallError::NoLoadedArtifact)?;
        let candidate = VerifiedArtifact::accept(package)?;
        if current.fingerprint() != plan.expected_current_fingerprint {
            return Err(AtomicInstallError::CurrentFingerprintMismatch);
        }
        if candidate.fingerprint() != plan.candidate_fingerprint {
            return Err(AtomicInstallError::CandidateFingerprintMismatch);
        }
        if matches!(
            self.cpu_state,
            CpuState::PoweredOff | CpuState::Startup | CpuState::Faulted | CpuState::Resetting
        ) {
            return Err(AtomicInstallError::UnsafeCpuState(self.cpu_state));
        }
        match plan.disposition {
            RuntimeInstallDisposition::PackageIdentityOnly
                if current.fingerprint() != candidate.fingerprint() =>
            {
                return Err(AtomicInstallError::IdentityOnlyRequiresEqualRuntimeArtifact);
            }
            RuntimeInstallDisposition::ArtifactReplacement
                if current.fingerprint() == candidate.fingerprint() =>
            {
                return Err(AtomicInstallError::ReplacementRequiresDifferentRuntimeArtifact);
            }
            _ => {}
        }
        if !plan.preserve_cpu_mode && self.cpu_state != CpuState::Stop {
            return Err(AtomicInstallError::StopRequired);
        }

        for id in &plan.preserve_memory {
            let old = current
                .spec()
                .memory
                .binary_search_by_key(id, |definition| definition.id)
                .ok()
                .map(|index| &current.spec().memory[index]);
            let new = candidate
                .spec()
                .memory
                .binary_search_by_key(id, |definition| definition.id)
                .ok()
                .map(|index| &candidate.spec().memory[index]);
            if !matches!((old, new), (Some(old), Some(new)) if old.value_type == new.value_type && old.retentive == new.retentive)
            {
                return Err(AtomicInstallError::IncompatibleMemoryIdentity(*id));
            }
        }
        for id in &plan.preserve_states {
            let old = current
                .spec()
                .states
                .binary_search_by_key(id, |definition| definition.id)
                .ok()
                .map(|index| &current.spec().states[index]);
            let new = candidate
                .spec()
                .states
                .binary_search_by_key(id, |definition| definition.id)
                .ok()
                .map(|index| &candidate.spec().states[index]);
            if !matches!((old, new), (Some(old), Some(new)) if old.loaded_start.kind_tag() == new.loaded_start.kind_tag() && old.retentive == new.retentive)
            {
                return Err(AtomicInstallError::IncompatibleStateIdentity(*id));
            }
        }
        if plan.preserve_io && current.spec().channels != candidate.spec().channels {
            return Err(AtomicInstallError::IncompatibleIoSchema);
        }

        if matches!(self.cpu_state, CpuState::Run | CpuState::PausedEducational)
            && plan.disposition == RuntimeInstallDisposition::ArtifactReplacement
            && (!plan.preserve_cpu_mode
                || !plan.preserve_io
                || plan.preserve_memory.len() != current.spec().memory.len()
                || plan.preserve_memory.len() != candidate.spec().memory.len()
                || plan.preserve_states.len() != current.spec().states.len()
                || plan.preserve_states.len() != candidate.spec().states.len())
        {
            return Err(AtomicInstallError::StopRequired);
        }

        let old_memory_count = current.spec().memory.len();
        let old_state_count = current.spec().states.len();
        let new_memory_count = candidate.spec().memory.len();
        let new_state_count = candidate.spec().states.len();
        let expected_state_hash = self.semantic_state_hash();
        let mut staged = self.clone();

        if plan.disposition == RuntimeInstallDisposition::ArtifactReplacement {
            let mut image = RuntimeImage::from_artifact(&candidate);
            for id in &plan.preserve_memory {
                let value = *self
                    .image
                    .actual_memory
                    .get(id)
                    .ok_or(AtomicInstallError::IncompatibleMemoryIdentity(*id))?;
                image.actual_memory.insert(*id, value);
                if let Some(retained) = self.image.retain_memory.get(id) {
                    image.retain_memory.insert(*id, *retained);
                }
            }
            for id in &plan.preserve_states {
                let value = *self
                    .image
                    .state_cells
                    .get(id)
                    .ok_or(AtomicInstallError::IncompatibleStateIdentity(*id))?;
                image.state_cells.insert(*id, value);
                if let Some(retained) = self.image.retain_state_cells.get(id) {
                    image.retain_state_cells.insert(*id, *retained);
                }
            }
            if plan.preserve_io {
                image.natural_inputs = self.image.natural_inputs.clone();
                image.effective_inputs = self.image.effective_inputs.clone();
                image.natural_outputs = self.image.natural_outputs.clone();
                image.effective_outputs = self.image.effective_outputs.clone();
            }
            staged.image = image;
        }

        staged.controller_epoch = staged.controller_epoch.saturating_add(1);
        staged.scan_sequence = 0;
        staged.loaded = Some(candidate);
        staged.input_receipts.clear();
        if !plan.preserve_cpu_mode {
            staged.cpu_state = CpuState::Stop;
        }
        let sequence = staged.next_event_sequence();
        if plan.disposition == RuntimeInstallDisposition::ArtifactReplacement && !plan.preserve_io {
            staged.boundary = VirtualIoBoundary::configured(
                staged.controller_id,
                &staged
                    .loaded
                    .as_ref()
                    .expect("staged artifact is present")
                    .spec()
                    .channels,
                sequence,
            );
        }
        if staged.cpu_state == CpuState::Stop {
            staged.deliver_mode_defaults(sequence, DeliveryReason::CpuModeDefault);
        }
        let new_fingerprint = staged
            .loaded_fingerprint()
            .expect("staged artifact is present");
        let mut payload = SemanticHasher::new("PES-ATOMIC-INSTALL-1");
        payload.hash(current.fingerprint());
        payload.hash(new_fingerprint);
        payload.u8(match plan.disposition {
            RuntimeInstallDisposition::ArtifactReplacement => 1,
            RuntimeInstallDisposition::PackageIdentityOnly => 2,
        });
        payload.u64(plan.preserve_memory.len() as u64);
        for id in &plan.preserve_memory {
            payload.u32(id.0);
        }
        payload.u64(plan.preserve_states.len() as u64);
        for id in &plan.preserve_states {
            payload.u32(id.0);
        }
        staged.append_replay(
            ReplayEventKind::ArtifactInstalled,
            sequence,
            staged.virtual_time_ms,
            payload.finish(),
            Hash32::ZERO,
        );
        staged.finish_lifecycle_boundary();

        let report = AtomicInstallReport {
            old_fingerprint: current.fingerprint(),
            new_fingerprint,
            disposition: plan.disposition,
            preserved_memory: plan.preserve_memory.clone(),
            initialized_memory_count: new_memory_count - plan.preserve_memory.len(),
            removed_memory_count: old_memory_count - plan.preserve_memory.len(),
            preserved_states: plan.preserve_states.clone(),
            initialized_state_count: new_state_count - plan.preserve_states.len(),
            removed_state_count: old_state_count - plan.preserve_states.len(),
            final_cpu_state: staged.cpu_state,
            controller_epoch: staged.controller_epoch,
            state_hash: staged.last_state_hash,
        };
        Ok(StagedAtomicInstall {
            controller_id: self.controller_id,
            expected_controller_epoch: self.controller_epoch,
            expected_cpu_state: self.cpu_state,
            expected_state_hash,
            candidate: staged,
            report,
        })
    }

    pub fn request_run(&mut self, restart: RestartKind) -> Result<(), CommandError> {
        if self.cpu_state != CpuState::Stop {
            return self.reject_transition("Request RUN");
        }
        let artifact = self.loaded.clone().ok_or(CommandError::NoLoadedArtifact)?;
        let sequence = self.next_event_sequence();
        self.cpu_state = CpuState::Startup;
        self.deliver_mode_defaults(sequence, DeliveryReason::CpuModeDefault);
        if restart == RestartKind::WarmRestart {
            self.apply_warm_initialization(&artifact);
        }
        let payload = hash_lifecycle(
            match restart {
                RestartKind::Resume => "REQUEST_RUN_RESUME",
                RestartKind::WarmRestart => "REQUEST_RUN_WARM",
            },
            CpuState::Startup,
            self.controller_epoch,
        );
        self.append_replay(
            match restart {
                RestartKind::Resume => ReplayEventKind::RequestRun,
                RestartKind::WarmRestart => ReplayEventKind::WarmRestart,
            },
            sequence,
            self.virtual_time_ms,
            payload,
            Hash32::ZERO,
        );

        if let Some(startup) = artifact.spec().program.startup.clone() {
            let mut work_units = 0;
            let mut executed_blocks = Vec::new();
            let mut call_boundaries = Vec::new();
            self.increment_invocation(startup.id);
            executed_blocks.push(startup.id);
            if let Err((fault, context)) = self.execute_block(
                &startup,
                self.scan_sequence,
                &mut work_units,
                &mut executed_blocks,
                &mut call_boundaries,
            ) {
                self.enter_fatal_fault(fault, context);
                return Ok(());
            }
        }
        self.cpu_state = CpuState::Run;
        self.finish_lifecycle_boundary();
        Ok(())
    }

    pub fn request_stop(&mut self) -> Result<(), CommandError> {
        if !matches!(self.cpu_state, CpuState::Run | CpuState::PausedEducational) {
            return self.reject_transition("Request STOP");
        }
        let sequence = self.next_event_sequence();
        self.cpu_state = CpuState::Stop;
        self.deliver_mode_defaults(sequence, DeliveryReason::CpuModeDefault);
        let payload = hash_lifecycle("REQUEST_STOP", self.cpu_state, self.controller_epoch);
        self.append_replay(
            ReplayEventKind::RequestStop,
            sequence,
            self.virtual_time_ms,
            payload,
            Hash32::ZERO,
        );
        self.finish_lifecycle_boundary();
        Ok(())
    }

    pub fn pause_educational(&mut self) -> Result<(), CommandError> {
        if self.cpu_state != CpuState::Run {
            return self.reject_transition("Pause");
        }
        let sequence = self.next_event_sequence();
        self.cpu_state = CpuState::PausedEducational;
        let payload = hash_lifecycle("PAUSE", self.cpu_state, self.controller_epoch);
        self.append_replay(
            ReplayEventKind::Pause,
            sequence,
            self.virtual_time_ms,
            payload,
            Hash32::ZERO,
        );
        self.finish_lifecycle_boundary();
        Ok(())
    }

    pub fn resume_educational(&mut self) -> Result<(), CommandError> {
        if self.cpu_state != CpuState::PausedEducational {
            return self.reject_transition("Resume");
        }
        let sequence = self.next_event_sequence();
        self.cpu_state = CpuState::Run;
        let payload = hash_lifecycle("RESUME", self.cpu_state, self.controller_epoch);
        self.append_replay(
            ReplayEventKind::Resume,
            sequence,
            self.virtual_time_ms,
            payload,
            Hash32::ZERO,
        );
        self.finish_lifecycle_boundary();
        Ok(())
    }

    pub fn reset_fault(&mut self) -> Result<(), CommandError> {
        if self.cpu_state != CpuState::Faulted {
            return self.reject_transition("Reset Fault");
        }
        let sequence = self.next_event_sequence();
        self.cpu_state = CpuState::Stop;
        self.deliver_mode_defaults(sequence, DeliveryReason::CpuModeDefault);
        let payload = hash_lifecycle("RESET_FAULT", self.cpu_state, self.controller_epoch);
        self.append_replay(
            ReplayEventKind::FaultReset,
            sequence,
            self.virtual_time_ms,
            payload,
            Hash32::ZERO,
        );
        self.finish_lifecycle_boundary();
        Ok(())
    }

    pub fn simulated_power_cycle(&mut self) -> Result<(), CommandError> {
        if !matches!(
            self.cpu_state,
            CpuState::Stop | CpuState::Faulted | CpuState::PoweredOff
        ) {
            return self.reject_transition("Simulated Power Cycle");
        }
        let artifact = self.loaded.clone().ok_or(CommandError::NoLoadedArtifact)?;
        self.cpu_state = CpuState::Resetting;
        self.controller_epoch = self.controller_epoch.saturating_add(1);
        self.scan_sequence = 0;
        let sequence = self.next_event_sequence();
        self.image.force_overlays.clear();
        self.apply_warm_initialization(&artifact);
        self.reset_invocation_ordinals();
        self.reset_io_layers(&artifact, sequence);
        self.cpu_state = CpuState::Stop;
        self.deliver_mode_defaults(sequence, DeliveryReason::CpuModeDefault);
        let payload = hash_lifecycle("POWER_CYCLE", self.cpu_state, self.controller_epoch);
        self.append_replay(
            ReplayEventKind::PowerCycle,
            sequence,
            self.virtual_time_ms,
            payload,
            Hash32::ZERO,
        );
        self.finish_lifecycle_boundary();
        Ok(())
    }

    pub fn memory_reset(&mut self) -> Result<(), CommandError> {
        if !matches!(
            self.cpu_state,
            CpuState::Stop | CpuState::PausedEducational | CpuState::Faulted
        ) {
            return self.reject_transition("Memory Reset");
        }
        let artifact = self.loaded.clone().ok_or(CommandError::NoLoadedArtifact)?;
        self.cpu_state = CpuState::Resetting;
        self.controller_epoch = self.controller_epoch.saturating_add(1);
        self.scan_sequence = 0;
        let sequence = self.next_event_sequence();
        self.image.force_overlays.clear();
        self.apply_memory_reset(&artifact);
        self.reset_io_layers(&artifact, sequence);
        self.cpu_state = CpuState::Stop;
        self.deliver_mode_defaults(sequence, DeliveryReason::CpuModeDefault);
        let payload = hash_lifecycle("MEMORY_RESET", self.cpu_state, self.controller_epoch);
        self.append_replay(
            ReplayEventKind::MemoryReset,
            sequence,
            self.virtual_time_ms,
            payload,
            Hash32::ZERO,
        );
        self.finish_lifecycle_boundary();
        Ok(())
    }

    pub fn set_virtual_input_raw(
        &mut self,
        command: InputCommand,
    ) -> Result<InputReceipt, CommandError> {
        let payload_hash = hash_input_command(&command);
        if let Some(stored) = self.input_receipts.get(&command.idempotency_key) {
            if stored.payload_hash != payload_hash {
                return Err(CommandError::IdempotencyCollision {
                    key: command.idempotency_key,
                });
            }
            let mut receipt = stored.receipt.clone();
            receipt.duplicate = true;
            return Ok(receipt);
        }
        if command.controller_id != self.controller_id {
            return Err(CommandError::WrongController);
        }
        if command.expected_controller_epoch != self.controller_epoch {
            return Err(CommandError::StaleControllerEpoch {
                expected: command.expected_controller_epoch,
                actual: self.controller_epoch,
            });
        }
        if matches!(self.cpu_state, CpuState::Startup | CpuState::Resetting) {
            return self.reject_transition("Set Virtual Input Raw");
        }
        let expected_type = self
            .boundary
            .input_value_type(command.channel_id)
            .ok_or(CommandError::UnknownInputChannel(command.channel_id))?;
        if command.value.value_type() != expected_type {
            return Err(CommandError::InputTypeMismatch {
                expected: expected_type,
                actual: command.value.value_type(),
            });
        }

        let sequence = self.next_event_sequence();
        self.boundary
            .set_raw(command.channel_id, command.value, sequence);
        let result_hash = hash_input_result(
            command.command_id,
            command.channel_id,
            command.value,
            sequence,
        );
        let receipt = InputReceipt {
            command_id: command.command_id,
            accepted_event_sequence: sequence,
            virtual_timestamp_ms: self.virtual_time_ms,
            duplicate: false,
            result_hash,
        };
        self.input_receipts.insert(
            command.idempotency_key,
            StoredInputReceipt {
                payload_hash,
                receipt: receipt.clone(),
            },
        );
        self.append_replay(
            ReplayEventKind::RawInputAccepted,
            sequence,
            self.virtual_time_ms,
            payload_hash,
            result_hash,
        );
        Ok(receipt)
    }

    fn validate_observation_identity(
        &self,
        controller_id: VirtualControllerId,
        expected_controller_epoch: u64,
        expected_artifact_fingerprint: Hash32,
        expected_state_hash: Hash32,
    ) -> Result<(), RuntimeBoundaryError> {
        if controller_id != self.controller_id {
            return Err(RuntimeBoundaryError::WrongController);
        }
        if expected_controller_epoch != self.controller_epoch {
            return Err(RuntimeBoundaryError::StaleControllerEpoch);
        }
        let fingerprint = self
            .loaded_fingerprint()
            .ok_or(RuntimeBoundaryError::NoLoadedArtifact)?;
        if expected_artifact_fingerprint != fingerprint {
            return Err(RuntimeBoundaryError::StaleArtifact);
        }
        if expected_state_hash != self.semantic_state_hash() {
            return Err(RuntimeBoundaryError::StaleState);
        }
        Ok(())
    }

    fn validate_natural_writes(
        &self,
        writes: &[RuntimeNaturalWrite],
    ) -> Result<(), RuntimeBoundaryError> {
        let mut targets = BTreeMap::new();
        for write in writes {
            if targets.insert(write.target, ()).is_some() {
                return Err(RuntimeBoundaryError::DuplicateTarget(write.target));
            }
            self.validate_target_value(write.target, Some(write.value))?;
        }
        Ok(())
    }

    fn validate_force_deltas(
        &self,
        deltas: &[RuntimeForceDelta],
    ) -> Result<(), RuntimeBoundaryError> {
        let mut targets = BTreeMap::new();
        for delta in deltas {
            if targets.insert(delta.target, ()).is_some() {
                return Err(RuntimeBoundaryError::DuplicateTarget(delta.target));
            }
            self.validate_target_value(delta.target, delta.value)?;
        }
        Ok(())
    }

    fn validate_target_value(
        &self,
        target: RuntimeValueTarget,
        value: Option<CanonicalValue>,
    ) -> Result<(), RuntimeBoundaryError> {
        let expected = self
            .target_value_type(target)
            .ok_or(RuntimeBoundaryError::UnknownTarget(target))?;
        if let Some(value) = value
            && value.value_type() != expected
        {
            return Err(RuntimeBoundaryError::TypeMismatch {
                target,
                expected,
                actual: value.value_type(),
            });
        }
        Ok(())
    }

    fn apply_natural_write(&mut self, write: RuntimeNaturalWrite) {
        match write.target {
            RuntimeValueTarget::Memory(id) => self.write_memory(id, write.value),
            RuntimeValueTarget::Input(id) => {
                self.image.natural_inputs.insert(id, write.value);
            }
            RuntimeValueTarget::Output(id) => {
                self.image.natural_outputs.insert(id, write.value);
            }
        }
    }

    fn apply_force_deltas(&mut self, deltas: &[RuntimeForceDelta]) {
        for delta in deltas {
            if let Some(value) = delta.value {
                self.image.force_overlays.insert(delta.target, value);
            } else {
                self.image.force_overlays.remove(&delta.target);
            }
        }
    }

    fn refresh_effective_inputs(&mut self) {
        self.image.effective_inputs = self.image.natural_inputs.clone();
        for (target, value) in &self.image.force_overlays {
            if let RuntimeValueTarget::Input(id) = target {
                self.image.effective_inputs.insert(*id, *value);
            }
        }
    }

    fn refresh_effective_outputs(&mut self) {
        self.image.effective_outputs = self.image.natural_outputs.clone();
        for (target, value) in &self.image.force_overlays {
            if let RuntimeValueTarget::Output(id) = target {
                self.image.effective_outputs.insert(*id, *value);
            }
        }
    }

    fn refresh_effective_layers(&mut self) {
        self.refresh_effective_inputs();
        self.refresh_effective_outputs();
    }

    pub fn run_scan(&mut self) -> Result<RunOutcome, CommandError> {
        self.run_scan_core(None).map(|(outcome, _, _)| outcome)
    }

    pub fn run_scan_with_observation(
        &mut self,
        command: &RuntimeScanCommand,
    ) -> Result<RuntimeScanReceipt, RuntimeBoundaryError> {
        self.validate_observation_identity(
            command.controller_id,
            command.expected_controller_epoch,
            command.expected_artifact_fingerprint,
            command.expected_state_hash,
        )?;
        if self.cpu_state != CpuState::Run {
            return Err(RuntimeBoundaryError::CpuStateDisallowed(self.cpu_state));
        }
        self.validate_natural_writes(&command.pre_program_writes)?;
        self.validate_natural_writes(&command.post_program_writes)?;
        self.validate_force_deltas(&command.force_deltas)?;
        for write in &command.pre_program_writes {
            if matches!(write.target, RuntimeValueTarget::Output(_)) {
                return Err(RuntimeBoundaryError::InvalidPreProgramTarget(write.target));
            }
        }
        for write in &command.post_program_writes {
            if !matches!(write.target, RuntimeValueTarget::Output(_)) {
                return Err(RuntimeBoundaryError::InvalidPostProgramTarget(write.target));
            }
        }

        let mut candidate = self.clone();
        let (outcome, applied_pre_program_writes, applied_post_program_writes) = candidate
            .run_scan_core(Some(command))
            .expect("validated RUN controller has a loaded artifact");
        let receipt = RuntimeScanReceipt {
            command_id: command.command_id,
            outcome,
            applied_pre_program_writes,
            applied_post_program_writes,
            force_overlay_hash: candidate.force_overlay_hash(),
        };
        *self = candidate;
        Ok(receipt)
    }

    fn run_scan_core(
        &mut self,
        observation: Option<&RuntimeScanCommand>,
    ) -> Result<
        (
            RunOutcome,
            Vec<RuntimeAppliedWrite>,
            Vec<RuntimeAppliedWrite>,
        ),
        CommandError,
    > {
        if self.cpu_state != CpuState::Run {
            return self.reject_transition("Run Scan");
        }
        let artifact = self.loaded.clone().ok_or(CommandError::NoLoadedArtifact)?;
        let scan_start_time_ms = self.virtual_time_ms;
        let active_scan_sequence = self.scan_sequence.saturating_add(1);
        self.scan_sequence = active_scan_sequence;

        // Input sampling is one atomic clone from the boundary. Program reads
        // only the frozen effective image until this invocation returns.
        for input in self.boundary.raw_inputs() {
            self.image
                .natural_inputs
                .insert(input.channel_id, input.canonical_value);
        }
        let mut pre_prior_values = Vec::new();
        let mut post_prior_values = Vec::new();
        let observation_replay_index = if let Some(command) = observation {
            for write in &command.pre_program_writes {
                pre_prior_values.push((
                    write.target,
                    self.natural_value(write.target)
                        .expect("validated target has a natural value"),
                ));
            }
            for write in &command.post_program_writes {
                post_prior_values.push((
                    write.target,
                    self.natural_value(write.target)
                        .expect("validated target has a natural value"),
                ));
            }
            self.apply_force_deltas(&command.force_deltas);
            for write in &command.pre_program_writes {
                self.apply_natural_write(*write);
            }
            let sequence = self.next_event_sequence();
            self.append_replay(
                ReplayEventKind::ObservationBoundary,
                sequence,
                scan_start_time_ms,
                hash_runtime_scan_command(command),
                Hash32::ZERO,
            );
            Some(self.replay_events.len() - 1)
        } else {
            None
        };
        self.refresh_effective_inputs();

        let applied_pre_program_writes = observation
            .map(|command| {
                command
                    .pre_program_writes
                    .iter()
                    .zip(pre_prior_values)
                    .map(
                        |(write, (target, prior_natural_value))| RuntimeAppliedWrite {
                            target,
                            boundary: RuntimePublicationBoundary::ScanBeforeProgram,
                            prior_natural_value,
                            written_natural_value: write.value,
                            visible_effective_value: self
                                .effective_value(target)
                                .expect("validated target has an effective value"),
                        },
                    )
                    .collect()
            })
            .unwrap_or_default();

        let mut work_units = 0_u32;
        let mut executed_blocks = Vec::new();
        let mut call_boundaries = Vec::new();
        let mut due_tasks: Vec<_> = artifact
            .spec()
            .program
            .timed
            .iter()
            .filter(|task| {
                scan_start_time_ms >= task.first_due_ms
                    && (scan_start_time_ms - task.first_due_ms).is_multiple_of(task.period_ms)
            })
            .cloned()
            .collect();
        due_tasks.sort_by_key(|task| (task.semantic_order, task.id, task.block.id));

        for task in due_tasks {
            self.increment_invocation(task.block.id);
            executed_blocks.push(task.block.id);
            if let Err((fault, context)) = self.execute_block(
                &task.block,
                active_scan_sequence,
                &mut work_units,
                &mut executed_blocks,
                &mut call_boundaries,
            ) {
                let event = self.enter_fatal_fault(fault, context);
                if let Some(index) = observation_replay_index {
                    self.replay_events[index].result_hash = self.last_state_hash;
                }
                return Ok((
                    RunOutcome::Faulted(event),
                    applied_pre_program_writes,
                    Vec::new(),
                ));
            }
        }

        let cyclic = artifact.spec().program.cyclic.clone();
        self.increment_invocation(cyclic.id);
        executed_blocks.push(cyclic.id);
        if let Err((fault, context)) = self.execute_block(
            &cyclic,
            active_scan_sequence,
            &mut work_units,
            &mut executed_blocks,
            &mut call_boundaries,
        ) {
            let event = self.enter_fatal_fault(fault, context);
            if let Some(index) = observation_replay_index {
                self.replay_events[index].result_hash = self.last_state_hash;
            }
            return Ok((
                RunOutcome::Faulted(event),
                applied_pre_program_writes,
                Vec::new(),
            ));
        }

        if let Some(command) = observation {
            for write in &command.post_program_writes {
                self.apply_natural_write(*write);
            }
        }
        self.refresh_effective_outputs();
        let applied_post_program_writes = observation
            .map(|command| {
                command
                    .post_program_writes
                    .iter()
                    .zip(post_prior_values)
                    .map(
                        |(write, (target, prior_natural_value))| RuntimeAppliedWrite {
                            target,
                            boundary: RuntimePublicationBoundary::ScanAfterProgram,
                            prior_natural_value,
                            written_natural_value: write.value,
                            visible_effective_value: self
                                .effective_value(target)
                                .expect("validated target has an effective value"),
                        },
                    )
                    .collect()
            })
            .unwrap_or_default();
        let output_event_sequence = self.next_event_sequence();
        self.boundary.commit_outputs(
            &self.image.effective_outputs,
            output_event_sequence,
            active_scan_sequence,
        );
        self.virtual_time_ms = self.virtual_time_ms.saturating_add(SCAN_QUANTUM_MS);
        self.last_state_hash = self.semantic_state_hash();
        self.boundary_hashes.push(BoundaryHash {
            segment: self.replay_segment(),
            scan_sequence: active_scan_sequence,
            virtual_timestamp_ms: self.virtual_time_ms,
            kind: BoundaryKind::ScanEnd,
            state_hash: self.last_state_hash,
        });
        let mut payload = SemanticHasher::new("PES-SCAN-1");
        payload.u64(active_scan_sequence);
        payload.u64(scan_start_time_ms);
        payload.u32(work_units);
        payload.u64(executed_blocks.len() as u64);
        for block in &executed_blocks {
            payload.u32(block.0);
        }
        if !call_boundaries.is_empty() {
            payload.string("PES-SCAN-CALL-BOUNDARIES-1");
            payload.u64(call_boundaries.len() as u64);
            for boundary in &call_boundaries {
                payload.u8(match boundary.kind {
                    CallBoundaryKind::Enter => 1,
                    CallBoundaryKind::Return => 2,
                });
                payload.u32(boundary.caller_block.0);
                payload.u32(boundary.callee_block.0);
                payload.u32(boundary.call_operation_id);
                payload.u128(boundary.source_identity);
                payload.u128(boundary.call_site_identity);
                payload.u8(boundary.dynamic_depth);
                match &boundary.instance {
                    Some(instance) => {
                        payload.bool(true);
                        payload.u128(instance.root_instance);
                        payload.u64(instance.multi_instance_slots.len() as u64);
                        for slot in &instance.multi_instance_slots {
                            payload.u128(*slot);
                        }
                    }
                    None => payload.bool(false),
                }
            }
        }
        self.append_replay(
            ReplayEventKind::ScanCompleted,
            output_event_sequence,
            scan_start_time_ms,
            payload.finish(),
            self.last_state_hash,
        );
        if let Some(index) = observation_replay_index {
            self.replay_events[index].result_hash = self.last_state_hash;
        }
        Ok((
            RunOutcome::Completed(ScanReport {
                scan_sequence: active_scan_sequence,
                scan_start_time_ms,
                completed_time_ms: self.virtual_time_ms,
                work_units,
                executed_blocks,
                call_boundaries,
                output_event_sequence,
                state_hash: self.last_state_hash,
            }),
            applied_pre_program_writes,
            applied_post_program_writes,
        ))
    }

    pub fn capture_snapshot(&self) -> Result<ControllerSnapshot, SnapshotError> {
        if !matches!(
            self.cpu_state,
            CpuState::Stop | CpuState::Run | CpuState::PausedEducational | CpuState::Faulted
        ) {
            return Err(SnapshotError::UnsafeCaptureState(self.cpu_state));
        }
        let artifact = self
            .loaded
            .as_ref()
            .ok_or(SnapshotError::ArtifactUnavailable(Hash32::ZERO))?;
        let body = SnapshotBody {
            universe_id: self.universe_id,
            captured_universe_epoch: self.universe_epoch,
            controller_id: self.controller_id,
            captured_controller_epoch: self.controller_epoch,
            captured_cpu_state: self.cpu_state,
            virtual_time_ms: self.virtual_time_ms,
            captured_scan_sequence: self.scan_sequence,
            captured_event_sequence: self.event_sequence,
            artifact_fingerprint: artifact.fingerprint(),
            profile_fingerprint: artifact.spec().profile_fingerprint,
            runtime_version: RUNTIME_SEMANTICS_VERSION,
            scheduler_version: SCHEDULER_VERSION,
            work_cost_version: WORK_COST_VERSION,
            deterministic_seed: self.deterministic_seed,
            image: self.image.clone(),
            boundary: self.boundary.clone(),
            diagnostics: self.diagnostics.clone(),
            input_receipts: self.input_receipts.clone(),
        };
        let content_hash = hash_snapshot(1, &body);
        Ok(ControllerSnapshot {
            schema_version: 1,
            body,
            content_hash,
        })
    }

    pub fn prepare_restore(
        &self,
        snapshot: &ControllerSnapshot,
    ) -> Result<RestoreApproval, SnapshotError> {
        self.validate_restore(snapshot)?;
        Ok(RestoreApproval {
            snapshot_hash: snapshot.content_hash,
            controller_id: self.controller_id,
            expected_universe_epoch: self.universe_epoch,
            expected_controller_epoch: self.controller_epoch,
            expected_current_state_hash: self.semantic_state_hash(),
        })
    }

    pub fn restore_snapshot(
        &mut self,
        snapshot: &ControllerSnapshot,
        approval: RestoreApproval,
    ) -> Result<Hash32, SnapshotError> {
        self.validate_restore(snapshot)?;
        let current_hash = self.semantic_state_hash();
        if approval.snapshot_hash != snapshot.content_hash
            || approval.controller_id != self.controller_id
            || approval.expected_universe_epoch != self.universe_epoch
            || approval.expected_controller_epoch != self.controller_epoch
            || approval.expected_current_state_hash != current_hash
        {
            return Err(SnapshotError::ApprovalMismatch);
        }

        // Candidate state is fully cloned before any live field is changed.
        let candidate_image = snapshot.body.image.clone();
        let candidate_boundary = snapshot.body.boundary.clone();
        let candidate_diagnostics = snapshot.body.diagnostics.clone();
        let candidate_receipts = snapshot.body.input_receipts.clone();
        let safe_mode = self.cpu_state;
        let new_universe_epoch = self.universe_epoch.saturating_add(1);
        let new_controller_epoch = self.controller_epoch.saturating_add(1);

        self.universe_epoch = new_universe_epoch;
        self.controller_epoch = new_controller_epoch;
        self.cpu_state = safe_mode;
        self.virtual_time_ms = snapshot.body.virtual_time_ms;
        self.scan_sequence = 0;
        self.event_sequence = 0;
        self.deterministic_seed = snapshot.body.deterministic_seed;
        self.image = candidate_image;
        self.reset_invocation_ordinals();
        self.boundary = candidate_boundary;
        self.boundary.controller_id = self.controller_id;
        self.diagnostics = candidate_diagnostics;
        self.input_receipts = candidate_receipts;

        let sequence = self.next_event_sequence();
        if safe_mode == CpuState::Stop {
            self.deliver_mode_defaults(sequence, DeliveryReason::CpuModeDefault);
        }
        let mut payload = SemanticHasher::new("PES-RESTORE-1");
        payload.hash(snapshot.content_hash);
        payload.u64(snapshot.body.captured_universe_epoch);
        payload.u64(snapshot.body.captured_controller_epoch);
        payload.u64(self.universe_epoch);
        payload.u64(self.controller_epoch);
        self.last_state_hash = self.semantic_state_hash();
        self.append_replay(
            ReplayEventKind::SnapshotRestored,
            sequence,
            self.virtual_time_ms,
            payload.finish(),
            self.last_state_hash,
        );
        self.boundary_hashes.push(BoundaryHash {
            segment: self.replay_segment(),
            scan_sequence: self.scan_sequence,
            virtual_timestamp_ms: self.virtual_time_ms,
            kind: BoundaryKind::SnapshotRestore,
            state_hash: self.last_state_hash,
        });
        Ok(self.last_state_hash)
    }

    pub fn replay_hash(&self) -> Hash32 {
        let mut hasher = SemanticHasher::new("PES-REPLAY-1");
        hasher.string(RUNTIME_SEMANTICS_VERSION);
        hasher.string(SCHEDULER_VERSION);
        hasher.string(WORK_COST_VERSION);
        hasher.u64(self.deterministic_seed);
        hasher.u64(self.replay_events.len() as u64);
        for event in &self.replay_events {
            encode_replay_event(event, &mut hasher);
        }
        hasher.u64(self.boundary_hashes.len() as u64);
        for boundary in &self.boundary_hashes {
            encode_segment(boundary.segment, &mut hasher);
            hasher.u64(boundary.scan_sequence);
            hasher.u64(boundary.virtual_timestamp_ms);
            hasher.u8(boundary.kind as u8);
            hasher.hash(boundary.state_hash);
        }
        hasher.finish()
    }

    fn replay_segment(&self) -> ReplaySegment {
        ReplaySegment {
            universe_id: self.universe_id,
            universe_epoch: self.universe_epoch,
            controller_id: self.controller_id,
            controller_epoch: self.controller_epoch,
        }
    }

    fn next_event_sequence(&mut self) -> u64 {
        self.event_sequence = self
            .event_sequence
            .checked_add(1)
            .expect("event sequence exhausted");
        self.event_sequence
    }

    fn append_replay(
        &mut self,
        kind: ReplayEventKind,
        event_sequence: u64,
        virtual_timestamp_ms: u64,
        payload_hash: Hash32,
        result_hash: Hash32,
    ) {
        self.replay_events.push(ReplayEvent {
            segment: self.replay_segment(),
            kind,
            event_sequence,
            virtual_timestamp_ms,
            payload_hash,
            result_hash,
        });
    }

    fn reject_transition<T>(&mut self, action: &'static str) -> Result<T, CommandError> {
        let from = self.cpu_state;
        self.emit_nonfatal_diagnostic(DiagnosticCode::IllegalCpuTransition);
        Err(CommandError::IllegalCpuTransition { from, action })
    }

    fn emit_nonfatal_diagnostic(&mut self, code: DiagnosticCode) -> DiagnosticEvent {
        let sequence = self.next_event_sequence();
        let occurrence_id = occurrence_id(
            self.universe_id,
            self.universe_epoch,
            self.controller_id,
            self.controller_epoch,
            code,
            sequence,
        );
        let event = DiagnosticEvent {
            occurrence_id,
            parent_occurrence_id: None,
            root_occurrence_id: occurrence_id,
            code,
            severity: DiagnosticSeverity::Error,
            universe_epoch: self.universe_epoch,
            controller_epoch: self.controller_epoch,
            event_sequence: sequence,
            virtual_timestamp_ms: self.virtual_time_ms,
            fault_context: None,
            fault_boundary_state_hash: None,
        };
        self.diagnostics.push(event.clone());
        let mut payload = SemanticHasher::new("PES-DIAGNOSTIC-COMMAND-1");
        payload.u16(code as u16);
        payload.u128(occurrence_id);
        self.append_replay(
            ReplayEventKind::CommandRejected,
            sequence,
            self.virtual_time_ms,
            payload.finish(),
            Hash32::ZERO,
        );
        self.finish_lifecycle_boundary();
        event
    }

    fn finish_lifecycle_boundary(&mut self) {
        self.last_state_hash = self.semantic_state_hash();
        if let Some(event) = self.replay_events.last_mut()
            && event.result_hash == Hash32::ZERO
        {
            event.result_hash = self.last_state_hash;
        }
        self.boundary_hashes.push(BoundaryHash {
            segment: self.replay_segment(),
            scan_sequence: self.scan_sequence,
            virtual_timestamp_ms: self.virtual_time_ms,
            kind: BoundaryKind::Lifecycle,
            state_hash: self.last_state_hash,
        });
    }

    fn deliver_mode_defaults(&mut self, sequence: u64, reason: DeliveryReason) {
        if let Some(artifact) = &self.loaded {
            for channel in &artifact.spec().channels {
                if channel.direction == ChannelDirection::Output {
                    let target = RuntimeValueTarget::Output(channel.id);
                    let effective = self
                        .image
                        .force_overlays
                        .get(&target)
                        .copied()
                        .unwrap_or(channel.canonical_default);
                    self.image.effective_outputs.insert(channel.id, effective);
                }
            }
            self.boundary
                .deliver_defaults(&artifact.spec().channels, sequence, reason);
        }
    }

    fn increment_invocation(&mut self, block: BlockId) {
        let ordinal = self
            .image
            .invocation_ordinals
            .get_mut(&block)
            .expect("verified block has an ordinal");
        *ordinal = ordinal.saturating_add(1);
    }

    fn execute_block(
        &mut self,
        block: &ProgramBlock,
        scan_sequence: u64,
        work_units: &mut u32,
        executed_blocks: &mut Vec<BlockId>,
        call_boundaries: &mut Vec<CallBoundaryEvent>,
    ) -> Result<(), (ExecutionFault, FaultContext)> {
        self.execute_block_inner(
            block,
            scan_sequence,
            work_units,
            0,
            BlockExecution::default(),
            executed_blocks,
            call_boundaries,
        )
        .map(|_| ())
    }

    #[allow(clippy::too_many_arguments)]
    fn execute_block_inner(
        &mut self,
        block: &ProgramBlock,
        scan_sequence: u64,
        work_units: &mut u32,
        dynamic_depth: u8,
        mut execution: BlockExecution,
        executed_blocks: &mut Vec<BlockId>,
        call_boundaries: &mut Vec<CallBoundaryEvent>,
    ) -> Result<BlockExecution, (ExecutionFault, FaultContext)> {
        let fingerprint = self.loaded_fingerprint().unwrap_or(Hash32::ZERO);
        for instruction in &block.instructions {
            let context = FaultContext {
                artifact_fingerprint: fingerprint,
                block_id: block.id,
                operation_id: instruction.operation_id,
                source_identity: instruction.source_identity,
                scan_sequence,
                controller_epoch: self.controller_epoch,
                virtual_timestamp_ms: self.virtual_time_ms,
                work_units_before_operation: *work_units,
            };
            self.charge_work(work_units, instruction.work_units(), &context)?;
            self.execute_operation(
                block.id,
                instruction.operation_id,
                instruction.source_identity,
                instruction.operation(),
                scan_sequence,
                work_units,
                dynamic_depth,
                &mut execution,
                executed_blocks,
                call_boundaries,
                &context,
            )?;
        }
        Ok(execution)
    }

    fn charge_work(
        &self,
        work_units: &mut u32,
        charge: u32,
        context: &FaultContext,
    ) -> Result<(), (ExecutionFault, FaultContext)> {
        let Some(charged) = work_units.checked_add(charge) else {
            return Err((ExecutionFault::WorkUnitBudgetExceeded, context.clone()));
        };
        if charged > MAX_WORK_UNITS_PER_SCAN {
            return Err((ExecutionFault::WorkUnitBudgetExceeded, context.clone()));
        }
        *work_units = charged;
        Ok(())
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn execute_operation(
        &mut self,
        caller_block: BlockId,
        operation_id: u32,
        source_identity: u128,
        operation: &Operation,
        scan_sequence: u64,
        work_units: &mut u32,
        dynamic_depth: u8,
        execution: &mut BlockExecution,
        executed_blocks: &mut Vec<BlockId>,
        call_boundaries: &mut Vec<CallBoundaryEvent>,
        context: &FaultContext,
    ) -> Result<(), (ExecutionFault, FaultContext)> {
        let local = |fault| (fault, context.clone());
        match operation {
            Operation::Noop => Ok(()),
            Operation::SetMemory { target, value } => {
                self.write_execution_memory(execution, *target, *value);
                Ok(())
            }
            Operation::Copy { source, target } => {
                match self
                    .read_execution_operand(execution, *source)
                    .map_err(local)?
                {
                    Some(value) => self.write_execution_memory(execution, *target, value),
                    None => Self::suppress_execution_memory(execution, *target),
                }
                Ok(())
            }
            Operation::AddI32 {
                left,
                right,
                target,
            } => self
                .execute_binary(
                    execution,
                    RuntimeBinaryOperator::Add,
                    *left,
                    *right,
                    *target,
                )
                .map_err(local),
            Operation::DivideI32 {
                numerator,
                denominator,
                target,
            } => self
                .execute_binary(
                    execution,
                    RuntimeBinaryOperator::Divide,
                    *numerator,
                    *denominator,
                    *target,
                )
                .map_err(local),
            Operation::LoadInput { channel, target } => {
                let value = *self
                    .image
                    .effective_inputs
                    .get(channel)
                    .ok_or_else(|| local(ExecutionFault::RuntimeInvariant))?;
                self.write_execution_memory(execution, *target, value);
                Ok(())
            }
            Operation::StoreOutput { source, channel } => {
                if let Some(value) = self
                    .read_execution_operand(execution, *source)
                    .map_err(local)?
                {
                    let output = self
                        .image
                        .natural_outputs
                        .get_mut(channel)
                        .ok_or_else(|| local(ExecutionFault::RuntimeInvariant))?;
                    *output = value;
                }
                Ok(())
            }
            Operation::RisingEdge {
                source,
                state,
                target,
            }
            | Operation::FallingEdge {
                source,
                state,
                target,
            } => {
                let Some(current) = self
                    .read_execution_bool(execution, *source)
                    .map_err(local)?
                else {
                    Self::suppress_execution_memory(execution, *target);
                    return Ok(());
                };
                let previous = match self.image.state_cells.get(state).copied() {
                    Some(RuntimeStateCell::Edge { previous }) => previous,
                    _ => return Err(local(ExecutionFault::RuntimeInvariant)),
                };
                let rising = matches!(operation, Operation::RisingEdge { .. });
                let value = if rising {
                    current && !previous
                } else {
                    !current && previous
                };
                self.write_execution_memory(execution, *target, CanonicalValue::Bool(value));
                self.write_state(*state, RuntimeStateCell::Edge { previous: current });
                Ok(())
            }
            Operation::TimerOnDelay {
                input,
                preset_ms,
                state,
                output,
                elapsed,
            } => {
                let Some(input) = self.read_execution_bool(execution, *input).map_err(local)?
                else {
                    Self::suppress_execution_memory(execution, *output);
                    Self::suppress_execution_memory(execution, *elapsed);
                    return Ok(());
                };
                let prior_elapsed = match self.image.state_cells.get(state).copied() {
                    Some(RuntimeStateCell::Timer { elapsed_ms, .. }) => elapsed_ms,
                    _ => return Err(local(ExecutionFault::RuntimeInvariant)),
                };
                let next_elapsed = if input {
                    prior_elapsed
                        .checked_add(SCAN_QUANTUM_MS)
                        .ok_or_else(|| local(ExecutionFault::TimerOverflow))?
                        .min(*preset_ms)
                } else {
                    0
                };
                let next_output = input && next_elapsed >= *preset_ms;
                self.write_state(
                    *state,
                    RuntimeStateCell::Timer {
                        elapsed_ms: next_elapsed,
                        output: next_output,
                    },
                );
                self.write_execution_memory(execution, *output, CanonicalValue::Bool(next_output));
                self.write_execution_memory(
                    execution,
                    *elapsed,
                    CanonicalValue::TimeMs(next_elapsed),
                );
                Ok(())
            }
            Operation::CounterUp {
                input,
                reset,
                preset,
                state,
                output,
                current,
            } => {
                let (Some(input), Some(reset)) = (
                    self.read_execution_bool(execution, *input).map_err(local)?,
                    self.read_execution_bool(execution, *reset).map_err(local)?,
                ) else {
                    Self::suppress_execution_memory(execution, *output);
                    Self::suppress_execution_memory(execution, *current);
                    return Ok(());
                };
                let (prior_count, prior_input) = match self.image.state_cells.get(state).copied() {
                    Some(RuntimeStateCell::Counter {
                        count,
                        previous_input,
                    }) => (count, previous_input),
                    _ => return Err(local(ExecutionFault::RuntimeInvariant)),
                };
                let count = if reset {
                    0
                } else if input && !prior_input {
                    prior_count.saturating_add(1)
                } else {
                    prior_count
                };
                self.write_state(
                    *state,
                    RuntimeStateCell::Counter {
                        count,
                        previous_input: input,
                    },
                );
                self.write_execution_memory(execution, *current, CanonicalValue::I32(count));
                self.write_execution_memory(
                    execution,
                    *output,
                    CanonicalValue::Bool(count >= *preset),
                );
                Ok(())
            }
            Operation::Unary {
                operator,
                operand,
                target,
            } => self
                .execute_unary(execution, *operator, *operand, *target)
                .map_err(local),
            Operation::Binary {
                operator,
                left,
                right,
                target,
            } => self
                .execute_binary(execution, *operator, *left, *right, *target)
                .map_err(local),
            Operation::InvokeInstruction(invocation) => self
                .execute_instruction_invocation(operation_id, invocation, execution)
                .map_err(local),
            Operation::CallBlock(call) => self.execute_block_call(
                caller_block,
                operation_id,
                source_identity,
                call,
                scan_sequence,
                work_units,
                dynamic_depth,
                execution,
                executed_blocks,
                call_boundaries,
                context,
            ),
            Operation::InvocationOutput {
                invocation_id,
                formal,
                target,
            } => {
                match execution
                    .invocation_results
                    .get(&(*invocation_id, *formal))
                    .copied()
                    .ok_or_else(|| local(ExecutionFault::RuntimeInvariant))?
                {
                    InvocationResult::Value(value) => {
                        self.write_execution_memory(execution, *target, value);
                    }
                    InvocationResult::Suppressed => {
                        Self::suppress_execution_memory(execution, *target);
                    }
                }
                Ok(())
            }
        }
    }

    fn execute_unary(
        &mut self,
        execution: &mut BlockExecution,
        operator: RuntimeUnaryOperator,
        operand: Operand,
        target: MemoryId,
    ) -> Result<(), ExecutionFault> {
        let Some(value) = self.read_execution_operand(execution, operand)? else {
            Self::suppress_execution_memory(execution, target);
            return Ok(());
        };
        let value = match (operator, value) {
            (RuntimeUnaryOperator::Plus, CanonicalValue::I32(value)) => CanonicalValue::I32(value),
            (RuntimeUnaryOperator::Negate, CanonicalValue::I32(value)) => CanonicalValue::I32(
                value
                    .checked_neg()
                    .ok_or(ExecutionFault::ArithmeticOverflow)?,
            ),
            (RuntimeUnaryOperator::Not, CanonicalValue::Bool(value)) => {
                CanonicalValue::Bool(!value)
            }
            _ => return Err(ExecutionFault::RuntimeInvariant),
        };
        self.write_execution_memory(execution, target, value);
        Ok(())
    }

    fn execute_binary(
        &mut self,
        execution: &mut BlockExecution,
        operator: RuntimeBinaryOperator,
        left: Operand,
        right: Operand,
        target: MemoryId,
    ) -> Result<(), ExecutionFault> {
        let (Some(left), Some(right)) = (
            self.read_execution_operand(execution, left)?,
            self.read_execution_operand(execution, right)?,
        ) else {
            Self::suppress_execution_memory(execution, target);
            return Ok(());
        };
        let value = evaluate_binary(operator, left, right)?;
        self.write_execution_memory(execution, target, value);
        Ok(())
    }

    fn read_execution_operand(
        &self,
        execution: &BlockExecution,
        operand: Operand,
    ) -> Result<Option<CanonicalValue>, ExecutionFault> {
        match operand {
            Operand::Constant(value) => Ok(Some(value)),
            Operand::Memory(id) => {
                if execution.suppressed_memory.contains(&id) {
                    return Ok(None);
                }
                if let Some(frame) = &execution.frame_memory
                    && let Some(value) = frame.get(&id)
                {
                    return Ok(Some(*value));
                }
                self.image
                    .force_overlays
                    .get(&RuntimeValueTarget::Memory(id))
                    .or_else(|| self.image.actual_memory.get(&id))
                    .copied()
                    .map(Some)
                    .ok_or(ExecutionFault::RuntimeInvariant)
            }
            Operand::Input(id) => self
                .image
                .effective_inputs
                .get(&id)
                .copied()
                .map(Some)
                .ok_or(ExecutionFault::RuntimeInvariant),
            Operand::Output(id) => self
                .image
                .natural_outputs
                .get(&id)
                .copied()
                .map(Some)
                .ok_or(ExecutionFault::RuntimeInvariant),
        }
    }

    fn read_execution_bool(
        &self,
        execution: &BlockExecution,
        operand: Operand,
    ) -> Result<Option<bool>, ExecutionFault> {
        self.read_execution_operand(execution, operand)?
            .map(|value| value.as_bool().ok_or(ExecutionFault::RuntimeInvariant))
            .transpose()
    }

    fn write_execution_memory(
        &mut self,
        execution: &mut BlockExecution,
        id: MemoryId,
        value: CanonicalValue,
    ) {
        execution.suppressed_memory.remove(&id);
        if let Some(frame) = &mut execution.frame_memory
            && frame.contains_key(&id)
        {
            frame.insert(id, value);
            return;
        }
        self.write_memory(id, value);
    }

    fn suppress_execution_memory(execution: &mut BlockExecution, id: MemoryId) {
        execution.suppressed_memory.insert(id);
    }

    fn execute_instruction_invocation(
        &mut self,
        operation_id: u32,
        invocation: &RuntimeInstructionInvocation,
        execution: &mut BlockExecution,
    ) -> Result<(), ExecutionFault> {
        let enabled = match invocation.activation {
            Some(activation) => self
                .read_execution_bool(execution, activation.enable)?
                .unwrap_or(false),
            None => true,
        };
        if !enabled {
            self.publish_disabled_instruction(operation_id, invocation, execution)?;
            return Ok(());
        }

        let mut inputs = BTreeMap::new();
        for input in &invocation.inputs {
            let RuntimeFormalRef::Instruction(formal) = input.formal else {
                return Err(ExecutionFault::RuntimeInvariant);
            };
            let Some(value) = self.read_execution_operand(execution, input.source)? else {
                self.suppress_invocation_outputs(operation_id, &invocation.outputs, execution);
                self.publish_status(operation_id, invocation.activation, false, execution);
                return Ok(());
            };
            inputs.insert(formal, value);
        }

        match self.compute_instruction_outputs(invocation, &inputs, execution) {
            Ok(outputs) => {
                for (formal, value) in outputs {
                    execution.invocation_results.insert(
                        (operation_id, RuntimeFormalRef::Instruction(formal)),
                        InvocationResult::Value(value),
                    );
                }
                self.publish_status(operation_id, invocation.activation, true, execution);
                Ok(())
            }
            Err(fault) => {
                self.publish_status(operation_id, invocation.activation, false, execution);
                Err(fault)
            }
        }
    }

    fn publish_disabled_instruction(
        &self,
        operation_id: u32,
        invocation: &RuntimeInstructionInvocation,
        execution: &mut BlockExecution,
    ) -> Result<(), ExecutionFault> {
        let behavior = invocation
            .activation
            .map(|activation| activation.when_disabled)
            .ok_or(ExecutionFault::RuntimeInvariant)?;
        match behavior {
            RuntimeDisabledBehavior::DefaultOutputsNoStateChange => {
                for output in &invocation.outputs {
                    if output.formal
                        != RuntimeFormalRef::Instruction(
                            invocation
                                .activation
                                .expect("disabled invocation has activation")
                                .status_formal,
                        )
                    {
                        execution.invocation_results.insert(
                            (operation_id, output.formal),
                            InvocationResult::Value(output.value_type.canonical_default()),
                        );
                    }
                }
            }
            RuntimeDisabledBehavior::PreserveOutputsNoStateChange => {
                let instance = invocation
                    .instance
                    .ok_or(ExecutionFault::RuntimeInvariant)?;
                let key = invocation_state_key(execution, instance);
                let state = self
                    .image
                    .invocation_state_cells
                    .get(&key)
                    .copied()
                    .unwrap_or_else(|| InvocationStateCell::initial(instance.kind));
                let preserved = preserved_instruction_outputs(invocation.instruction, state)?;
                for output in &invocation.outputs {
                    if let RuntimeFormalRef::Instruction(formal) = output.formal
                        && let Some(value) = preserved.get(&formal)
                    {
                        execution.invocation_results.insert(
                            (operation_id, output.formal),
                            InvocationResult::Value(*value),
                        );
                    }
                }
            }
            RuntimeDisabledBehavior::SuppressEffects => {
                self.suppress_invocation_outputs(operation_id, &invocation.outputs, execution);
            }
        }
        self.publish_status(operation_id, invocation.activation, false, execution);
        Ok(())
    }

    fn suppress_invocation_outputs(
        &self,
        operation_id: u32,
        outputs: &[crate::RuntimeDeclaredOutput],
        execution: &mut BlockExecution,
    ) {
        for output in outputs {
            execution
                .invocation_results
                .insert((operation_id, output.formal), InvocationResult::Suppressed);
        }
    }

    fn publish_status(
        &self,
        operation_id: u32,
        activation: Option<RuntimeActivation>,
        status: bool,
        execution: &mut BlockExecution,
    ) {
        if let Some(activation) = activation {
            execution.invocation_results.insert(
                (
                    operation_id,
                    RuntimeFormalRef::Instruction(activation.status_formal),
                ),
                InvocationResult::Value(CanonicalValue::Bool(status)),
            );
        }
    }

    #[allow(clippy::too_many_lines)]
    fn compute_instruction_outputs(
        &mut self,
        invocation: &RuntimeInstructionInvocation,
        inputs: &BTreeMap<u16, CanonicalValue>,
        execution: &BlockExecution,
    ) -> Result<BTreeMap<u16, CanonicalValue>, ExecutionFault> {
        let mut outputs = BTreeMap::new();
        match invocation.instruction {
            RuntimeInstructionCode::NoOp
            | RuntimeInstructionCode::Probe
            | RuntimeInstructionCode::TraceSample
            | RuntimeInstructionCode::BreakpointMarker => {}
            RuntimeInstructionCode::Move => {
                outputs.insert(FORMAL_OUTPUT, instruction_input(inputs, FORMAL_INPUT)?);
            }
            RuntimeInstructionCode::BoolNot => {
                outputs.insert(
                    FORMAL_OUTPUT,
                    CanonicalValue::Bool(!instruction_bool(inputs, FORMAL_INPUT)?),
                );
            }
            RuntimeInstructionCode::BoolAnd
            | RuntimeInstructionCode::BoolOr
            | RuntimeInstructionCode::BoolXor => {
                let left = instruction_bool(inputs, FORMAL_LEFT)?;
                let right = instruction_bool(inputs, FORMAL_RIGHT)?;
                let value = match invocation.instruction {
                    RuntimeInstructionCode::BoolAnd => left && right,
                    RuntimeInstructionCode::BoolOr => left || right,
                    RuntimeInstructionCode::BoolXor => left ^ right,
                    _ => unreachable!(),
                };
                outputs.insert(FORMAL_OUTPUT, CanonicalValue::Bool(value));
            }
            RuntimeInstructionCode::CompareEqual
            | RuntimeInstructionCode::CompareNotEqual
            | RuntimeInstructionCode::CompareLess
            | RuntimeInstructionCode::CompareLessEqual
            | RuntimeInstructionCode::CompareGreater
            | RuntimeInstructionCode::CompareGreaterEqual => {
                let operator = match invocation.instruction {
                    RuntimeInstructionCode::CompareEqual => RuntimeBinaryOperator::Equal,
                    RuntimeInstructionCode::CompareNotEqual => RuntimeBinaryOperator::NotEqual,
                    RuntimeInstructionCode::CompareLess => RuntimeBinaryOperator::Less,
                    RuntimeInstructionCode::CompareLessEqual => RuntimeBinaryOperator::LessEqual,
                    RuntimeInstructionCode::CompareGreater => RuntimeBinaryOperator::Greater,
                    RuntimeInstructionCode::CompareGreaterEqual => {
                        RuntimeBinaryOperator::GreaterEqual
                    }
                    _ => unreachable!(),
                };
                outputs.insert(
                    FORMAL_OUTPUT,
                    evaluate_binary(
                        operator,
                        instruction_input(inputs, FORMAL_LEFT)?,
                        instruction_input(inputs, FORMAL_RIGHT)?,
                    )?,
                );
            }
            RuntimeInstructionCode::Add
            | RuntimeInstructionCode::Subtract
            | RuntimeInstructionCode::Multiply
            | RuntimeInstructionCode::Divide
            | RuntimeInstructionCode::Modulo => {
                let operator = match invocation.instruction {
                    RuntimeInstructionCode::Add => RuntimeBinaryOperator::Add,
                    RuntimeInstructionCode::Subtract => RuntimeBinaryOperator::Subtract,
                    RuntimeInstructionCode::Multiply => RuntimeBinaryOperator::Multiply,
                    RuntimeInstructionCode::Divide => RuntimeBinaryOperator::Divide,
                    RuntimeInstructionCode::Modulo => RuntimeBinaryOperator::Modulo,
                    _ => unreachable!(),
                };
                outputs.insert(
                    FORMAL_OUTPUT,
                    evaluate_binary(
                        operator,
                        instruction_input(inputs, FORMAL_LEFT)?,
                        instruction_input(inputs, FORMAL_RIGHT)?,
                    )?,
                );
            }
            RuntimeInstructionCode::RisingEdge | RuntimeInstructionCode::FallingEdge => {
                let instance = invocation
                    .instance
                    .ok_or(ExecutionFault::RuntimeInvariant)?;
                let key = invocation_state_key(execution, instance);
                let previous = match self
                    .image
                    .invocation_state_cells
                    .get(&key)
                    .copied()
                    .unwrap_or_else(|| InvocationStateCell::initial(instance.kind))
                {
                    InvocationStateCell::Edge { previous } => previous,
                    _ => return Err(ExecutionFault::RuntimeInvariant),
                };
                let current = instruction_bool(inputs, FORMAL_CLOCK)?;
                let value = if invocation.instruction == RuntimeInstructionCode::RisingEdge {
                    current && !previous
                } else {
                    !current && previous
                };
                self.write_invocation_state(
                    key,
                    InvocationStateCell::Edge { previous: current },
                    instance.retentive,
                );
                outputs.insert(FORMAL_OUTPUT, CanonicalValue::Bool(value));
            }
            RuntimeInstructionCode::TimerOnDelay
            | RuntimeInstructionCode::TimerOffDelay
            | RuntimeInstructionCode::TimerPulse => {
                self.compute_timer_outputs(invocation, inputs, execution, &mut outputs)?;
            }
            RuntimeInstructionCode::CounterUp
            | RuntimeInstructionCode::CounterDown
            | RuntimeInstructionCode::CounterUpDown => {
                self.compute_counter_outputs(invocation, inputs, execution, &mut outputs)?;
            }
        }
        Ok(outputs)
    }

    fn compute_timer_outputs(
        &mut self,
        invocation: &RuntimeInstructionInvocation,
        inputs: &BTreeMap<u16, CanonicalValue>,
        execution: &BlockExecution,
        outputs: &mut BTreeMap<u16, CanonicalValue>,
    ) -> Result<(), ExecutionFault> {
        let instance = invocation
            .instance
            .ok_or(ExecutionFault::RuntimeInvariant)?;
        let key = invocation_state_key(execution, instance);
        let (prior_elapsed, prior_output, prior_input) = match self
            .image
            .invocation_state_cells
            .get(&key)
            .copied()
            .unwrap_or_else(|| InvocationStateCell::initial(instance.kind))
        {
            InvocationStateCell::Timer {
                elapsed_ms,
                output,
                previous_input,
            } => (elapsed_ms, output, previous_input),
            _ => return Err(ExecutionFault::RuntimeInvariant),
        };
        let input = instruction_bool(inputs, FORMAL_INPUT)?;
        let preset = instruction_time(inputs, FORMAL_PRESET_TIME)?;
        let (elapsed, output) = match invocation.instruction {
            RuntimeInstructionCode::TimerOnDelay => {
                let elapsed = if input {
                    prior_elapsed
                        .checked_add(SCAN_QUANTUM_MS)
                        .ok_or(ExecutionFault::TimerOverflow)?
                        .min(preset)
                } else {
                    0
                };
                (elapsed, input && elapsed >= preset)
            }
            RuntimeInstructionCode::TimerOffDelay => {
                if input {
                    (0, true)
                } else if prior_output {
                    let elapsed = prior_elapsed
                        .checked_add(SCAN_QUANTUM_MS)
                        .ok_or(ExecutionFault::TimerOverflow)?
                        .min(preset);
                    (elapsed, elapsed < preset)
                } else {
                    (0, false)
                }
            }
            RuntimeInstructionCode::TimerPulse => {
                let rising = input && !prior_input;
                if rising && !prior_output {
                    (0, true)
                } else if prior_output {
                    let elapsed = prior_elapsed
                        .checked_add(SCAN_QUANTUM_MS)
                        .ok_or(ExecutionFault::TimerOverflow)?
                        .min(preset);
                    (elapsed, elapsed < preset)
                } else {
                    (0, false)
                }
            }
            _ => return Err(ExecutionFault::RuntimeInvariant),
        };
        self.write_invocation_state(
            key,
            InvocationStateCell::Timer {
                elapsed_ms: elapsed,
                output,
                previous_input: input,
            },
            instance.retentive,
        );
        outputs.insert(FORMAL_OUTPUT, CanonicalValue::Bool(output));
        outputs.insert(FORMAL_ELAPSED_TIME, CanonicalValue::TimeMs(elapsed));
        Ok(())
    }

    fn compute_counter_outputs(
        &mut self,
        invocation: &RuntimeInstructionInvocation,
        inputs: &BTreeMap<u16, CanonicalValue>,
        execution: &BlockExecution,
        outputs: &mut BTreeMap<u16, CanonicalValue>,
    ) -> Result<(), ExecutionFault> {
        let instance = invocation
            .instance
            .ok_or(ExecutionFault::RuntimeInvariant)?;
        let key = invocation_state_key(execution, instance);
        let (prior_count, prior_up, prior_down) = match self
            .image
            .invocation_state_cells
            .get(&key)
            .copied()
            .unwrap_or_else(|| InvocationStateCell::initial(instance.kind))
        {
            InvocationStateCell::Counter {
                count,
                previous_up,
                previous_down,
                ..
            } => (count, previous_up, previous_down),
            _ => return Err(ExecutionFault::RuntimeInvariant),
        };
        let preset = instruction_i32(inputs, FORMAL_PRESET_VALUE)?;
        let (up, down, count) = match invocation.instruction {
            RuntimeInstructionCode::CounterUp => {
                let up = instruction_bool(inputs, FORMAL_COUNT_UP)?;
                let reset = instruction_bool(inputs, FORMAL_RESET)?;
                let count = if reset {
                    0
                } else if up && !prior_up {
                    prior_count.saturating_add(1)
                } else {
                    prior_count
                };
                (up, false, count)
            }
            RuntimeInstructionCode::CounterDown => {
                let down = instruction_bool(inputs, FORMAL_COUNT_DOWN)?;
                let load = instruction_bool(inputs, FORMAL_LOAD)?;
                let count = if load {
                    preset
                } else if down && !prior_down {
                    prior_count.saturating_sub(1)
                } else {
                    prior_count
                };
                (false, down, count)
            }
            RuntimeInstructionCode::CounterUpDown => {
                let up = instruction_bool(inputs, FORMAL_COUNT_UP)?;
                let down = instruction_bool(inputs, FORMAL_COUNT_DOWN)?;
                let reset = instruction_bool(inputs, FORMAL_RESET)?;
                let load = instruction_bool(inputs, FORMAL_LOAD)?;
                let count = if reset {
                    0
                } else if load {
                    preset
                } else {
                    match (up && !prior_up, down && !prior_down) {
                        (true, false) => prior_count.saturating_add(1),
                        (false, true) => prior_count.saturating_sub(1),
                        (false, false) | (true, true) => prior_count,
                    }
                };
                (up, down, count)
            }
            _ => return Err(ExecutionFault::RuntimeInvariant),
        };
        let output_up = count >= preset;
        let output_down = count <= 0;
        self.write_invocation_state(
            key,
            InvocationStateCell::Counter {
                count,
                previous_up: up,
                previous_down: down,
                output_up,
                output_down,
            },
            instance.retentive,
        );
        match invocation.instruction {
            RuntimeInstructionCode::CounterUp => {
                outputs.insert(FORMAL_OUTPUT, CanonicalValue::Bool(output_up));
            }
            RuntimeInstructionCode::CounterDown => {
                outputs.insert(FORMAL_OUTPUT, CanonicalValue::Bool(output_down));
            }
            RuntimeInstructionCode::CounterUpDown => {
                outputs.insert(FORMAL_QU, CanonicalValue::Bool(output_up));
                outputs.insert(FORMAL_QD, CanonicalValue::Bool(output_down));
            }
            _ => return Err(ExecutionFault::RuntimeInvariant),
        }
        outputs.insert(FORMAL_CURRENT_VALUE, CanonicalValue::I32(count));
        Ok(())
    }

    fn write_invocation_state(
        &mut self,
        key: InvocationStateKey,
        value: InvocationStateCell,
        retentive: bool,
    ) {
        self.image.invocation_state_cells.insert(key.clone(), value);
        if retentive {
            self.image.retain_invocation_state_cells.insert(key, value);
        }
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn execute_block_call(
        &mut self,
        caller_block: BlockId,
        operation_id: u32,
        source_identity: u128,
        call: &RuntimeBlockCall,
        scan_sequence: u64,
        work_units: &mut u32,
        dynamic_depth: u8,
        caller_execution: &mut BlockExecution,
        executed_blocks: &mut Vec<BlockId>,
        call_boundaries: &mut Vec<CallBoundaryEvent>,
        context: &FaultContext,
    ) -> Result<(), (ExecutionFault, FaultContext)> {
        let enabled = match call.activation {
            Some(activation) => self
                .read_execution_bool(caller_execution, activation.enable)
                .map_err(|fault| (fault, context.clone()))?
                .unwrap_or(false),
            None => true,
        };
        if !enabled {
            self.suppress_call_outputs(operation_id, call, caller_execution);
            self.publish_status(operation_id, call.activation, false, caller_execution);
            return Ok(());
        }
        if dynamic_depth >= crate::MAX_DYNAMIC_CALL_DEPTH {
            return Err((ExecutionFault::WorkUnitBudgetExceeded, context.clone()));
        }

        let mut copied_inputs = Vec::with_capacity(call.inputs.len());
        for input in &call.inputs {
            let RuntimeFormalRef::BlockMember(formal) = input.formal else {
                return Err((ExecutionFault::RuntimeInvariant, context.clone()));
            };
            let Some(value) = self
                .read_execution_operand(caller_execution, input.source)
                .map_err(|fault| (fault, context.clone()))?
            else {
                self.suppress_call_outputs(operation_id, call, caller_execution);
                self.publish_status(operation_id, call.activation, false, caller_execution);
                return Ok(());
            };
            copied_inputs.push((formal, value));
        }

        let artifact = self
            .loaded
            .as_ref()
            .ok_or_else(|| (ExecutionFault::RuntimeInvariant, context.clone()))?;
        let mut frame_memory = BTreeMap::new();
        let mut frame_ids = BTreeSet::new();
        collect_block_memory_ids(&call.callee, &mut frame_ids);
        for id in frame_ids {
            let index = artifact
                .spec()
                .memory
                .binary_search_by_key(&id, |definition| definition.id)
                .map_err(|_| (ExecutionFault::RuntimeInvariant, context.clone()))?;
            frame_memory.insert(id, artifact.spec().memory[index].loaded_start);
        }
        for member in &call.frame_members {
            frame_memory.insert(member.memory, member.initial_value);
        }
        if let Some(instance) = &call.instance
            && let Some(persisted) = self.image.function_block_instances.get(instance)
        {
            for (memory, value) in persisted {
                frame_memory.insert(*memory, *value);
            }
        }
        for (formal, value) in copied_inputs {
            let member = call
                .frame_members
                .iter()
                .find(|member| member.formal == formal)
                .ok_or_else(|| (ExecutionFault::RuntimeInvariant, context.clone()))?;
            frame_memory.insert(member.memory, value);
        }

        let mut state_scope = caller_execution.state_scope.clone();
        state_scope.push(match &call.instance {
            Some(instance) => instance_scope_identity(instance),
            None => call.call_site_identity,
        });
        let callee_execution = BlockExecution {
            frame_memory: Some(frame_memory),
            suppressed_memory: BTreeSet::new(),
            invocation_results: BTreeMap::new(),
            state_scope,
        };
        call_boundaries.push(CallBoundaryEvent {
            kind: CallBoundaryKind::Enter,
            caller_block,
            callee_block: call.callee.id,
            call_operation_id: operation_id,
            source_identity,
            call_site_identity: call.call_site_identity,
            dynamic_depth: dynamic_depth + 1,
            instance: call.instance.clone(),
        });
        self.increment_invocation(call.callee.id);
        executed_blocks.push(call.callee.id);
        let completed = self.execute_block_inner(
            &call.callee,
            scan_sequence,
            work_units,
            dynamic_depth + 1,
            callee_execution,
            executed_blocks,
            call_boundaries,
        )?;
        self.charge_work(work_units, 1, context)?;

        let frame = completed
            .frame_memory
            .as_ref()
            .ok_or_else(|| (ExecutionFault::RuntimeInvariant, context.clone()))?;
        if call.kind == RuntimeCallKind::FunctionBlock {
            let instance = call
                .instance
                .as_ref()
                .ok_or_else(|| (ExecutionFault::RuntimeInvariant, context.clone()))?;
            let mut persistent = BTreeMap::new();
            let mut retained = BTreeMap::new();
            for member in &call.frame_members {
                if member.role.persists_in_instance() {
                    let value = *frame
                        .get(&member.memory)
                        .ok_or_else(|| (ExecutionFault::RuntimeInvariant, context.clone()))?;
                    persistent.insert(member.memory, value);
                    if member.retentive {
                        retained.insert(member.memory, value);
                    }
                }
            }
            self.image
                .function_block_instances
                .insert(instance.clone(), persistent);
            if retained.is_empty() {
                self.image.retain_function_block_instances.remove(instance);
            } else {
                self.image
                    .retain_function_block_instances
                    .insert(instance.clone(), retained);
            }
        }
        for member in &call.frame_members {
            let formal = RuntimeFormalRef::BlockMember(member.formal);
            if call.outputs.iter().any(|output| output.formal == formal) {
                let value = *frame
                    .get(&member.memory)
                    .ok_or_else(|| (ExecutionFault::RuntimeInvariant, context.clone()))?;
                caller_execution
                    .invocation_results
                    .insert((operation_id, formal), InvocationResult::Value(value));
            }
        }
        self.publish_status(operation_id, call.activation, true, caller_execution);
        call_boundaries.push(CallBoundaryEvent {
            kind: CallBoundaryKind::Return,
            caller_block,
            callee_block: call.callee.id,
            call_operation_id: operation_id,
            source_identity,
            call_site_identity: call.call_site_identity,
            dynamic_depth: dynamic_depth + 1,
            instance: call.instance.clone(),
        });
        Ok(())
    }

    fn suppress_call_outputs(
        &self,
        operation_id: u32,
        call: &RuntimeBlockCall,
        execution: &mut BlockExecution,
    ) {
        for output in &call.outputs {
            execution
                .invocation_results
                .insert((operation_id, output.formal), InvocationResult::Suppressed);
        }
    }

    fn write_memory(&mut self, id: MemoryId, value: CanonicalValue) {
        self.image.actual_memory.insert(id, value);
        let retentive = self
            .loaded
            .as_ref()
            .and_then(|artifact| {
                artifact
                    .spec()
                    .memory
                    .binary_search_by_key(&id, |definition| definition.id)
                    .ok()
                    .map(|index| artifact.spec().memory[index].retentive)
            })
            .unwrap_or(false);
        if retentive {
            self.image.retain_memory.insert(id, value);
        }
    }

    fn write_state(&mut self, id: StateId, value: RuntimeStateCell) {
        self.image.state_cells.insert(id, value);
        let retentive = self
            .loaded
            .as_ref()
            .and_then(|artifact| {
                artifact
                    .spec()
                    .states
                    .binary_search_by_key(&id, |definition| definition.id)
                    .ok()
                    .map(|index| artifact.spec().states[index].retentive)
            })
            .unwrap_or(false);
        if retentive {
            self.image.retain_state_cells.insert(id, value);
        }
    }

    fn enter_fatal_fault(
        &mut self,
        fault: ExecutionFault,
        context: FaultContext,
    ) -> DiagnosticEvent {
        let code = match fault {
            ExecutionFault::DivideByZero => DiagnosticCode::ArithmeticDivideByZero,
            ExecutionFault::ArithmeticOverflow => DiagnosticCode::ArithmeticOverflow,
            ExecutionFault::TimerOverflow => DiagnosticCode::TimerOverflow,
            ExecutionFault::WorkUnitBudgetExceeded => DiagnosticCode::WorkUnitBudgetExceeded,
            ExecutionFault::RuntimeInvariant => DiagnosticCode::RuntimeInvariantFailure,
        };
        let sequence = self.next_event_sequence();
        let occurrence_id = occurrence_id(
            self.universe_id,
            self.universe_epoch,
            self.controller_id,
            self.controller_epoch,
            code,
            sequence,
        );
        self.cpu_state = CpuState::Faulted;
        self.deliver_mode_defaults(sequence, DeliveryReason::FatalFaultDefault);
        let event = DiagnosticEvent {
            occurrence_id,
            parent_occurrence_id: None,
            root_occurrence_id: occurrence_id,
            code,
            severity: DiagnosticSeverity::Fatal,
            universe_epoch: self.universe_epoch,
            controller_epoch: self.controller_epoch,
            event_sequence: sequence,
            virtual_timestamp_ms: self.virtual_time_ms,
            fault_context: Some(context),
            fault_boundary_state_hash: None,
        };
        self.diagnostics.push(event);
        let payload_hash = hash_diagnostic(
            self.diagnostics.last().expect("fatal event just appended"),
            false,
        );
        self.append_replay(
            ReplayEventKind::FatalFault,
            sequence,
            self.virtual_time_ms,
            payload_hash,
            Hash32::ZERO,
        );
        self.last_state_hash = self.semantic_state_hash();
        if let Some(replay) = self.replay_events.last_mut() {
            replay.result_hash = self.last_state_hash;
        }
        if let Some(diagnostic) = self.diagnostics.last_mut() {
            diagnostic.fault_boundary_state_hash = Some(self.last_state_hash);
        }
        self.boundary_hashes.push(BoundaryHash {
            segment: self.replay_segment(),
            scan_sequence: self.scan_sequence,
            virtual_timestamp_ms: self.virtual_time_ms,
            kind: BoundaryKind::FatalFault,
            state_hash: self.last_state_hash,
        });
        self.diagnostics
            .last()
            .expect("fatal event retained")
            .clone()
    }

    fn apply_warm_initialization(&mut self, artifact: &VerifiedArtifact) {
        for definition in &artifact.spec().memory {
            let value = if definition.retentive {
                self.image
                    .retain_memory
                    .get(&definition.id)
                    .copied()
                    .unwrap_or(definition.loaded_start)
            } else {
                definition.loaded_start
            };
            self.image.actual_memory.insert(definition.id, value);
        }
        for definition in &artifact.spec().states {
            let value = if definition.retentive {
                self.image
                    .retain_state_cells
                    .get(&definition.id)
                    .copied()
                    .unwrap_or_else(|| RuntimeStateCell::from(definition.loaded_start))
            } else {
                RuntimeStateCell::from(definition.loaded_start)
            };
            self.image.state_cells.insert(definition.id, value);
        }
        // Stateful instruction instances and FB instance frames follow the
        // same warm-restart policy as ordinary state: only the explicitly
        // retentive mirror is carried into the next execution.
        self.image.invocation_state_cells = self.image.retain_invocation_state_cells.clone();
        self.image.function_block_instances = self.image.retain_function_block_instances.clone();
        // A warm restart retains the boundary raw layer, but the CPU images are
        // reset and will be freshly sampled on the next normal scan.
        for channel in &artifact.spec().channels {
            match channel.direction {
                ChannelDirection::Input => {
                    self.image
                        .natural_inputs
                        .insert(channel.id, channel.canonical_default);
                    self.image
                        .effective_inputs
                        .insert(channel.id, channel.canonical_default);
                }
                ChannelDirection::Output => {
                    self.image
                        .natural_outputs
                        .insert(channel.id, channel.canonical_default);
                    self.image
                        .effective_outputs
                        .insert(channel.id, channel.canonical_default);
                }
            }
        }
    }

    fn apply_memory_reset(&mut self, artifact: &VerifiedArtifact) {
        self.image.actual_memory.clear();
        self.image.retain_memory.clear();
        for definition in &artifact.spec().memory {
            self.image
                .actual_memory
                .insert(definition.id, definition.loaded_start);
            if definition.retentive {
                self.image
                    .retain_memory
                    .insert(definition.id, definition.loaded_start);
            }
        }
        self.image.state_cells.clear();
        self.image.retain_state_cells.clear();
        for definition in &artifact.spec().states {
            let state = RuntimeStateCell::from(definition.loaded_start);
            self.image.state_cells.insert(definition.id, state);
            if definition.retentive {
                self.image.retain_state_cells.insert(definition.id, state);
            }
        }
        self.image.invocation_state_cells.clear();
        self.image.retain_invocation_state_cells.clear();
        self.image.function_block_instances.clear();
        self.image.retain_function_block_instances.clear();
        self.reset_invocation_ordinals();
    }

    fn reset_io_layers(&mut self, artifact: &VerifiedArtifact, sequence: u64) {
        self.boundary
            .reset_raw_defaults(&artifact.spec().channels, sequence);
        for channel in &artifact.spec().channels {
            match channel.direction {
                ChannelDirection::Input => {
                    self.image
                        .natural_inputs
                        .insert(channel.id, channel.canonical_default);
                    self.image
                        .effective_inputs
                        .insert(channel.id, channel.canonical_default);
                }
                ChannelDirection::Output => {
                    self.image
                        .natural_outputs
                        .insert(channel.id, channel.canonical_default);
                    self.image
                        .effective_outputs
                        .insert(channel.id, channel.canonical_default);
                }
            }
        }
    }

    fn reset_invocation_ordinals(&mut self) {
        for ordinal in self.image.invocation_ordinals.values_mut() {
            *ordinal = 0;
        }
    }

    fn validate_restore(&self, snapshot: &ControllerSnapshot) -> Result<(), SnapshotError> {
        if !matches!(self.cpu_state, CpuState::Stop | CpuState::PausedEducational) {
            return Err(SnapshotError::UnsafeRestoreState(self.cpu_state));
        }
        if snapshot.schema_version != 1
            || snapshot.body.runtime_version != RUNTIME_SEMANTICS_VERSION
            || snapshot.body.scheduler_version != SCHEDULER_VERSION
            || snapshot.body.work_cost_version != WORK_COST_VERSION
        {
            return Err(SnapshotError::IncompatibleRuntime);
        }
        if snapshot.body.universe_id != self.universe_id
            || snapshot.body.controller_id != self.controller_id
            || snapshot.body.boundary.controller_id != self.controller_id
        {
            return Err(SnapshotError::WrongController);
        }
        let actual = snapshot.calculate_hash();
        if actual != snapshot.content_hash {
            return Err(SnapshotError::IntegrityMismatch {
                declared: snapshot.content_hash,
                actual,
            });
        }
        let artifact = self
            .loaded
            .as_ref()
            .ok_or(SnapshotError::ArtifactUnavailable(
                snapshot.body.artifact_fingerprint,
            ))?;
        if artifact.fingerprint() != snapshot.body.artifact_fingerprint
            || artifact.spec().profile_fingerprint != snapshot.body.profile_fingerprint
        {
            return Err(SnapshotError::ArtifactUnavailable(
                snapshot.body.artifact_fingerprint,
            ));
        }
        Ok(())
    }

    pub fn semantic_state_hash(&self) -> Hash32 {
        let mut hasher = SemanticHasher::new("PES-RUNTIME-STATE-1");
        hasher.string(RUNTIME_SEMANTICS_VERSION);
        hasher.string(SCHEDULER_VERSION);
        hasher.string(WORK_COST_VERSION);
        hasher.u128(self.universe_id.0);
        hasher.u64(self.universe_epoch);
        hasher.u128(self.controller_id.0);
        hasher.u64(self.controller_epoch);
        hasher.u8(self.cpu_state as u8);
        hasher.u64(self.virtual_time_ms);
        hasher.u64(self.scan_sequence);
        hasher.u64(self.event_sequence);
        hasher.u64(self.deterministic_seed);
        match &self.loaded {
            Some(artifact) => {
                hasher.bool(true);
                hasher.hash(artifact.fingerprint());
                hasher.hash(artifact.spec().profile_fingerprint);
            }
            None => hasher.bool(false),
        }
        self.image.encode(&mut hasher);
        self.boundary.encode(&mut hasher);
        hasher.u64(self.diagnostics.len() as u64);
        for event in &self.diagnostics {
            encode_diagnostic(event, &mut hasher, false);
        }
        hasher.u64(self.input_receipts.len() as u64);
        for (key, stored) in &self.input_receipts {
            hasher.u128(*key);
            hasher.hash(stored.payload_hash);
            encode_input_receipt(&stored.receipt, &mut hasher);
        }
        hasher.finish()
    }
}

fn invocation_state_key(
    execution: &BlockExecution,
    instance: RuntimeInstructionInstance,
) -> InvocationStateKey {
    InvocationStateKey {
        scope: execution.state_scope.clone(),
        stable_id: instance.stable_id,
        kind: instance.kind,
    }
}

fn instance_scope_identity(instance: &RuntimeFunctionBlockInstance) -> u128 {
    let mut hasher = SemanticHasher::new("PES-RUNTIME-FB-INSTANCE-SCOPE-1");
    hasher.u128(instance.root_instance);
    hasher.u64(instance.multi_instance_slots.len() as u64);
    for slot in &instance.multi_instance_slots {
        hasher.u128(*slot);
    }
    let bytes = hasher.finish().0;
    u128::from_be_bytes(bytes[..16].try_into().expect("16-byte instance identity"))
}

fn instruction_input(
    inputs: &BTreeMap<u16, CanonicalValue>,
    formal: u16,
) -> Result<CanonicalValue, ExecutionFault> {
    inputs
        .get(&formal)
        .copied()
        .ok_or(ExecutionFault::RuntimeInvariant)
}

fn instruction_bool(
    inputs: &BTreeMap<u16, CanonicalValue>,
    formal: u16,
) -> Result<bool, ExecutionFault> {
    instruction_input(inputs, formal)?
        .as_bool()
        .ok_or(ExecutionFault::RuntimeInvariant)
}

fn instruction_i32(
    inputs: &BTreeMap<u16, CanonicalValue>,
    formal: u16,
) -> Result<i32, ExecutionFault> {
    instruction_input(inputs, formal)?
        .as_i32()
        .ok_or(ExecutionFault::RuntimeInvariant)
}

fn instruction_time(
    inputs: &BTreeMap<u16, CanonicalValue>,
    formal: u16,
) -> Result<u64, ExecutionFault> {
    match instruction_input(inputs, formal)? {
        CanonicalValue::TimeMs(value) => Ok(value),
        _ => Err(ExecutionFault::RuntimeInvariant),
    }
}

fn preserved_instruction_outputs(
    instruction: RuntimeInstructionCode,
    state: InvocationStateCell,
) -> Result<BTreeMap<u16, CanonicalValue>, ExecutionFault> {
    let mut outputs = BTreeMap::new();
    match (instruction, state) {
        (
            RuntimeInstructionCode::TimerOnDelay
            | RuntimeInstructionCode::TimerOffDelay
            | RuntimeInstructionCode::TimerPulse,
            InvocationStateCell::Timer {
                elapsed_ms, output, ..
            },
        ) => {
            outputs.insert(FORMAL_OUTPUT, CanonicalValue::Bool(output));
            outputs.insert(FORMAL_ELAPSED_TIME, CanonicalValue::TimeMs(elapsed_ms));
        }
        (
            RuntimeInstructionCode::CounterUp,
            InvocationStateCell::Counter {
                count, output_up, ..
            },
        ) => {
            outputs.insert(FORMAL_OUTPUT, CanonicalValue::Bool(output_up));
            outputs.insert(FORMAL_CURRENT_VALUE, CanonicalValue::I32(count));
        }
        (
            RuntimeInstructionCode::CounterDown,
            InvocationStateCell::Counter {
                count, output_down, ..
            },
        ) => {
            outputs.insert(FORMAL_OUTPUT, CanonicalValue::Bool(output_down));
            outputs.insert(FORMAL_CURRENT_VALUE, CanonicalValue::I32(count));
        }
        (
            RuntimeInstructionCode::CounterUpDown,
            InvocationStateCell::Counter {
                count,
                output_up,
                output_down,
                ..
            },
        ) => {
            outputs.insert(FORMAL_QU, CanonicalValue::Bool(output_up));
            outputs.insert(FORMAL_QD, CanonicalValue::Bool(output_down));
            outputs.insert(FORMAL_CURRENT_VALUE, CanonicalValue::I32(count));
        }
        _ => return Err(ExecutionFault::RuntimeInvariant),
    }
    Ok(outputs)
}

fn evaluate_binary(
    operator: RuntimeBinaryOperator,
    left: CanonicalValue,
    right: CanonicalValue,
) -> Result<CanonicalValue, ExecutionFault> {
    match operator {
        RuntimeBinaryOperator::Add
        | RuntimeBinaryOperator::Subtract
        | RuntimeBinaryOperator::Multiply
        | RuntimeBinaryOperator::Divide
        | RuntimeBinaryOperator::Modulo => {
            let (Some(left), Some(right)) = (left.as_i32(), right.as_i32()) else {
                return Err(ExecutionFault::RuntimeInvariant);
            };
            let value = match operator {
                RuntimeBinaryOperator::Add => left.checked_add(right),
                RuntimeBinaryOperator::Subtract => left.checked_sub(right),
                RuntimeBinaryOperator::Multiply => left.checked_mul(right),
                RuntimeBinaryOperator::Divide if right == 0 => {
                    return Err(ExecutionFault::DivideByZero);
                }
                RuntimeBinaryOperator::Divide => left.checked_div(right),
                RuntimeBinaryOperator::Modulo if right == 0 => {
                    return Err(ExecutionFault::DivideByZero);
                }
                RuntimeBinaryOperator::Modulo => left.checked_rem(right),
                _ => unreachable!(),
            }
            .ok_or(ExecutionFault::ArithmeticOverflow)?;
            Ok(CanonicalValue::I32(value))
        }
        RuntimeBinaryOperator::And | RuntimeBinaryOperator::Xor | RuntimeBinaryOperator::Or => {
            let (Some(left), Some(right)) = (left.as_bool(), right.as_bool()) else {
                return Err(ExecutionFault::RuntimeInvariant);
            };
            Ok(CanonicalValue::Bool(match operator {
                RuntimeBinaryOperator::And => left && right,
                RuntimeBinaryOperator::Xor => left ^ right,
                RuntimeBinaryOperator::Or => left || right,
                _ => unreachable!(),
            }))
        }
        RuntimeBinaryOperator::Equal | RuntimeBinaryOperator::NotEqual => {
            if left.value_type() != right.value_type() {
                return Err(ExecutionFault::RuntimeInvariant);
            }
            let equal = left == right;
            Ok(CanonicalValue::Bool(
                if operator == RuntimeBinaryOperator::Equal {
                    equal
                } else {
                    !equal
                },
            ))
        }
        RuntimeBinaryOperator::Less
        | RuntimeBinaryOperator::LessEqual
        | RuntimeBinaryOperator::Greater
        | RuntimeBinaryOperator::GreaterEqual => {
            let ordering = compare_canonical(left, right)?;
            Ok(CanonicalValue::Bool(match operator {
                RuntimeBinaryOperator::Less => ordering.is_lt(),
                RuntimeBinaryOperator::LessEqual => !ordering.is_gt(),
                RuntimeBinaryOperator::Greater => ordering.is_gt(),
                RuntimeBinaryOperator::GreaterEqual => !ordering.is_lt(),
                _ => unreachable!(),
            }))
        }
    }
}

fn compare_canonical(
    left: CanonicalValue,
    right: CanonicalValue,
) -> Result<core::cmp::Ordering, ExecutionFault> {
    match (left, right) {
        (CanonicalValue::Bool(left), CanonicalValue::Bool(right)) => Ok(left.cmp(&right)),
        (CanonicalValue::I32(left), CanonicalValue::I32(right)) => Ok(left.cmp(&right)),
        (CanonicalValue::I64(left), CanonicalValue::I64(right)) => Ok(left.cmp(&right)),
        (CanonicalValue::U32(left), CanonicalValue::U32(right)) => Ok(left.cmp(&right)),
        (CanonicalValue::TimeMs(left), CanonicalValue::TimeMs(right)) => Ok(left.cmp(&right)),
        _ => Err(ExecutionFault::RuntimeInvariant),
    }
}

fn collect_block_memory_ids(block: &ProgramBlock, target: &mut BTreeSet<MemoryId>) {
    for instruction in &block.instructions {
        collect_operation_memory_ids(instruction.operation(), target);
    }
}

fn collect_operand_memory(operand: Operand, target: &mut BTreeSet<MemoryId>) {
    if let Operand::Memory(memory) = operand {
        target.insert(memory);
    }
}

#[allow(clippy::too_many_lines)]
fn collect_operation_memory_ids(operation: &Operation, target: &mut BTreeSet<MemoryId>) {
    match operation {
        Operation::Noop => {}
        Operation::SetMemory { target: memory, .. }
        | Operation::LoadInput { target: memory, .. }
        | Operation::InvocationOutput { target: memory, .. } => {
            target.insert(*memory);
        }
        Operation::Copy {
            source,
            target: memory,
        }
        | Operation::Unary {
            operand: source,
            target: memory,
            ..
        } => {
            collect_operand_memory(*source, target);
            target.insert(*memory);
        }
        Operation::AddI32 {
            left,
            right,
            target: memory,
        }
        | Operation::Binary {
            left,
            right,
            target: memory,
            ..
        } => {
            collect_operand_memory(*left, target);
            collect_operand_memory(*right, target);
            target.insert(*memory);
        }
        Operation::DivideI32 {
            numerator,
            denominator,
            target: memory,
        } => {
            collect_operand_memory(*numerator, target);
            collect_operand_memory(*denominator, target);
            target.insert(*memory);
        }
        Operation::StoreOutput { source, .. } => collect_operand_memory(*source, target),
        Operation::RisingEdge {
            source,
            target: memory,
            ..
        }
        | Operation::FallingEdge {
            source,
            target: memory,
            ..
        } => {
            collect_operand_memory(*source, target);
            target.insert(*memory);
        }
        Operation::TimerOnDelay {
            input,
            output,
            elapsed,
            ..
        } => {
            collect_operand_memory(*input, target);
            target.insert(*output);
            target.insert(*elapsed);
        }
        Operation::CounterUp {
            input,
            reset,
            output,
            current,
            ..
        } => {
            collect_operand_memory(*input, target);
            collect_operand_memory(*reset, target);
            target.insert(*output);
            target.insert(*current);
        }
        Operation::InvokeInstruction(invocation) => {
            for input in &invocation.inputs {
                collect_operand_memory(input.source, target);
            }
        }
        Operation::CallBlock(call) => {
            for input in &call.inputs {
                collect_operand_memory(input.source, target);
            }
        }
    }
}

fn hash_lifecycle(action: &str, state: CpuState, controller_epoch: u64) -> Hash32 {
    let mut hasher = SemanticHasher::new("PES-LIFECYCLE-1");
    hasher.string(action);
    hasher.u8(state as u8);
    hasher.u64(controller_epoch);
    hasher.finish()
}

fn hash_force_overlays(overlays: &BTreeMap<RuntimeValueTarget, CanonicalValue>) -> Hash32 {
    let mut hasher = SemanticHasher::new("PES-RUNTIME-FORCE-OVERLAYS-1");
    hasher.u64(overlays.len() as u64);
    for (target, value) in overlays {
        encode_runtime_target(*target, &mut hasher);
        value.encode(&mut hasher);
    }
    hasher.finish()
}

pub fn canonical_force_overlay_hash(
    overlays: &[(RuntimeValueTarget, CanonicalValue)],
) -> Result<Hash32, RuntimeBoundaryError> {
    let mut canonical = BTreeMap::new();
    for (target, value) in overlays {
        if canonical.insert(*target, *value).is_some() {
            return Err(RuntimeBoundaryError::DuplicateTarget(*target));
        }
    }
    Ok(hash_force_overlays(&canonical))
}

fn encode_natural_writes(writes: &[RuntimeNaturalWrite], hasher: &mut SemanticHasher) {
    hasher.u64(writes.len() as u64);
    for write in writes {
        encode_runtime_target(write.target, hasher);
        write.value.encode(hasher);
    }
}

fn encode_force_deltas(deltas: &[RuntimeForceDelta], hasher: &mut SemanticHasher) {
    hasher.u64(deltas.len() as u64);
    for delta in deltas {
        encode_runtime_target(delta.target, hasher);
        match delta.value {
            Some(value) => {
                hasher.bool(true);
                value.encode(hasher);
            }
            None => hasher.bool(false),
        }
    }
}

fn hash_runtime_boundary_command(command: &RuntimeBoundaryCommand) -> Hash32 {
    let mut hasher = SemanticHasher::new("PES-RUNTIME-OBSERVATION-COMMAND-1");
    hasher.u128(command.command_id);
    hasher.u128(command.controller_id.0);
    hasher.u64(command.expected_controller_epoch);
    hasher.hash(command.expected_artifact_fingerprint);
    hasher.hash(command.expected_state_hash);
    encode_natural_writes(&command.natural_writes, &mut hasher);
    encode_force_deltas(&command.force_deltas, &mut hasher);
    hasher.hash(command.audit_context_hash);
    hasher.finish()
}

fn hash_runtime_scan_command(command: &RuntimeScanCommand) -> Hash32 {
    let mut hasher = SemanticHasher::new("PES-RUNTIME-OBSERVATION-SCAN-COMMAND-1");
    hasher.u128(command.command_id);
    hasher.u128(command.controller_id.0);
    hasher.u64(command.expected_controller_epoch);
    hasher.hash(command.expected_artifact_fingerprint);
    hasher.hash(command.expected_state_hash);
    encode_natural_writes(&command.pre_program_writes, &mut hasher);
    encode_natural_writes(&command.post_program_writes, &mut hasher);
    encode_force_deltas(&command.force_deltas, &mut hasher);
    hasher.hash(command.audit_context_hash);
    hasher.finish()
}

fn hash_input_command(command: &InputCommand) -> Hash32 {
    let mut hasher = SemanticHasher::new("PES-RAW-INPUT-COMMAND-1");
    hasher.u128(command.command_id.0);
    hasher.u128(command.idempotency_key);
    hasher.u128(command.controller_id.0);
    hasher.u64(command.expected_controller_epoch);
    hasher.u32(command.channel_id.0);
    command.value.encode(&mut hasher);
    hasher.hash(command.audit_provenance_hash);
    hasher.finish()
}

fn hash_input_result(
    command_id: CommandId,
    channel_id: ChannelId,
    value: CanonicalValue,
    event_sequence: u64,
) -> Hash32 {
    let mut hasher = SemanticHasher::new("PES-RAW-INPUT-RESULT-1");
    hasher.u128(command_id.0);
    hasher.u32(channel_id.0);
    value.encode(&mut hasher);
    hasher.u64(event_sequence);
    hasher.finish()
}

fn occurrence_id(
    universe_id: UniverseId,
    universe_epoch: u64,
    controller_id: VirtualControllerId,
    controller_epoch: u64,
    code: DiagnosticCode,
    event_sequence: u64,
) -> u128 {
    let mut hasher = SemanticHasher::new("PES-OCCURRENCE-ID-1");
    hasher.u128(universe_id.0);
    hasher.u64(universe_epoch);
    hasher.u128(controller_id.0);
    hasher.u64(controller_epoch);
    hasher.u16(code as u16);
    hasher.u64(event_sequence);
    let bytes = hasher.finish().0;
    u128::from_be_bytes(bytes[..16].try_into().expect("16-byte occurrence identity"))
}

fn encode_optional_hash(value: Option<Hash32>, hasher: &mut SemanticHasher) {
    match value {
        Some(value) => {
            hasher.bool(true);
            hasher.hash(value);
        }
        None => hasher.bool(false),
    }
}

fn encode_segment(segment: ReplaySegment, hasher: &mut SemanticHasher) {
    hasher.u128(segment.universe_id.0);
    hasher.u64(segment.universe_epoch);
    hasher.u128(segment.controller_id.0);
    hasher.u64(segment.controller_epoch);
}

fn encode_replay_event(event: &ReplayEvent, hasher: &mut SemanticHasher) {
    encode_segment(event.segment, hasher);
    hasher.u8(event.kind as u8);
    hasher.u64(event.event_sequence);
    hasher.u64(event.virtual_timestamp_ms);
    hasher.hash(event.payload_hash);
    hasher.hash(event.result_hash);
}

fn hash_diagnostic(event: &DiagnosticEvent, include_boundary_hash: bool) -> Hash32 {
    let mut hasher = SemanticHasher::new("PES-DIAGNOSTIC-EVENT-1");
    encode_diagnostic(event, &mut hasher, include_boundary_hash);
    hasher.finish()
}

fn encode_diagnostic(
    event: &DiagnosticEvent,
    hasher: &mut SemanticHasher,
    include_boundary_hash: bool,
) {
    hasher.u128(event.occurrence_id);
    match event.parent_occurrence_id {
        Some(value) => {
            hasher.bool(true);
            hasher.u128(value);
        }
        None => hasher.bool(false),
    }
    hasher.u128(event.root_occurrence_id);
    hasher.u16(event.code as u16);
    hasher.u8(event.severity as u8);
    hasher.u64(event.universe_epoch);
    hasher.u64(event.controller_epoch);
    hasher.u64(event.event_sequence);
    hasher.u64(event.virtual_timestamp_ms);
    match &event.fault_context {
        Some(context) => {
            hasher.bool(true);
            hasher.hash(context.artifact_fingerprint);
            hasher.u32(context.block_id.0);
            hasher.u32(context.operation_id);
            hasher.u128(context.source_identity);
            hasher.u64(context.scan_sequence);
            hasher.u64(context.controller_epoch);
            hasher.u64(context.virtual_timestamp_ms);
            hasher.u32(context.work_units_before_operation);
        }
        None => hasher.bool(false),
    }
    if include_boundary_hash {
        encode_optional_hash(event.fault_boundary_state_hash, hasher);
    }
}

fn encode_input_receipt(receipt: &InputReceipt, hasher: &mut SemanticHasher) {
    hasher.u128(receipt.command_id.0);
    hasher.u64(receipt.accepted_event_sequence);
    hasher.u64(receipt.virtual_timestamp_ms);
    // Duplicate is a response-local observation. The authoritative accepted
    // receipt is always encoded as non-duplicate in controller state.
    hasher.bool(false);
    hasher.hash(receipt.result_hash);
}

fn hash_snapshot(schema_version: u32, body: &SnapshotBody) -> Hash32 {
    let mut hasher = SemanticHasher::new("PES-SNAPSHOT-1");
    hasher.u32(schema_version);
    hasher.u128(body.universe_id.0);
    hasher.u64(body.captured_universe_epoch);
    hasher.u128(body.controller_id.0);
    hasher.u64(body.captured_controller_epoch);
    hasher.u8(body.captured_cpu_state as u8);
    hasher.u64(body.virtual_time_ms);
    hasher.u64(body.captured_scan_sequence);
    hasher.u64(body.captured_event_sequence);
    hasher.hash(body.artifact_fingerprint);
    hasher.hash(body.profile_fingerprint);
    hasher.string(body.runtime_version);
    hasher.string(body.scheduler_version);
    hasher.string(body.work_cost_version);
    hasher.u64(body.deterministic_seed);
    body.image.encode(&mut hasher);
    body.boundary.encode(&mut hasher);
    hasher.u64(body.diagnostics.len() as u64);
    for event in &body.diagnostics {
        encode_diagnostic(event, &mut hasher, true);
    }
    hasher.u64(body.input_receipts.len() as u64);
    for (key, stored) in &body.input_receipts {
        hasher.u128(*key);
        hasher.hash(stored.payload_hash);
        encode_input_receipt(&stored.receipt, &mut hasher);
    }
    hasher.finish()
}
