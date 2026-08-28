#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod boundary;
mod controller;
mod hash;
mod model;

pub use boundary::{
    CommandId, DeliveredOutput, DeliveryReason, InputCommand, InputReceipt, Quality, RawInput,
    UniverseId, VirtualControllerId, VirtualIoBoundary,
};
pub use controller::{
    AtomicInstallError, AtomicInstallReport, BoundaryHash, CommandError, ControllerSnapshot,
    CpuState, DiagnosticCode, DiagnosticEvent, DiagnosticSeverity, FaultContext, InstallOutcome,
    ReplayEvent, ReplayEventKind, ReplaySegment, RestartKind, RestoreApproval, RunOutcome,
    RuntimeAppliedWrite, RuntimeBoundaryCommand, RuntimeBoundaryError, RuntimeBoundaryReceipt,
    RuntimeCloneReport, RuntimeForceDelta, RuntimeForceResetApproval, RuntimeInstallDisposition,
    RuntimeLifecycleError, RuntimeNaturalWrite, RuntimePublicationBoundary,
    RuntimeReplacementReport, RuntimeScanCommand, RuntimeScanReceipt, RuntimeStateTransferPlan,
    RuntimeValueTarget, ScanReport, SnapshotError, StagedAtomicInstall, StagedControllerClone,
    StagedControllerReplacement, VirtualController, canonical_force_overlay_hash,
};
pub use hash::{Hash32, Sha256};
pub use model::{
    ArtifactError, ArtifactPackage, ArtifactSpec, BlockId, CanonicalValue, ChannelDefinition,
    ChannelDirection, ChannelId, Instruction, MemoryDefinition, MemoryId, Operand, Operation,
    ProgramBlock, ProgramImage, StateDefinition, StateId, StateStart, TaskId, TimedTask, ValueType,
    VerifiedArtifact,
};

/// The EDU-21 Core 1.0 semantic scan quantum.
pub const SCAN_QUANTUM_MS: u64 = 10;

/// The deterministic `EDU-WORK-1` budget for one normal scan.
pub const MAX_WORK_UNITS_PER_SCAN: u32 = 100_000;

/// The maximum supported dynamic call depth. Calls are rejected by artifact
/// validation until the typed IR call instruction is introduced.
pub const MAX_DYNAMIC_CALL_DEPTH: u8 = 64;

/// Version identities participate in artifact, snapshot, state, and replay
/// hashes. Any semantic change requires a new value.
pub const RUNTIME_SEMANTICS_VERSION: &str = "EDU-RTM-1";
pub const SCHEDULER_VERSION: &str = "EDU-SCH-1";
pub const PRIORITY_TABLE_VERSION: &str = "EDU-PRIORITY-1";
pub const WORK_COST_VERSION: &str = "EDU-WORK-1";
