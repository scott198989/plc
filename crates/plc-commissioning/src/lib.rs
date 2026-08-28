#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod canonical;
mod model;
mod universe;

pub use model::{
    ActualHardwareState, AvailabilityComparison, BuiltHardwareState, ComparisonVector,
    CompatibilityClass, ConfiguredController, ForceId, HardwareComparison, LoadBlocker,
    LoadPackageError, LoadPackageParts, LoadPreview, LoadRequest, MatchComparison, MemoryAction,
    MemoryActionKind, MemoryActionReason, MemoryMemberSchema, MemoryRole, MonitoringComparison,
    OfflineControllerId, OfflineEngineeringState, OfflineSourceBuild, PackageComparison,
    PostLoadMode, PreviewApproval, PreviewId, ProfileComparison, SessionCommandBinding,
    SessionError, SessionState, StateAction, StateActionKind, StateKind, StateMemberSchema,
    VirtualLoadPackage, VirtualOnlineSession, VirtualOnlineSessionId,
};
pub use universe::{
    ActualHardwareFaultCommand, CloneInstanceCommand, CloneInstanceResult,
    CommissionedBoundaryReceipt, CommissionedScanReceipt, CommissioningAuditEvent,
    CommissioningAuditKind, CommissioningError, ControllerInstance, ControllerInstanceId,
    CreateInstanceCommand, ForceRegistryProjection, InternalFailurePoint, LifecycleExecution,
    LifecycleRollback, LoadExecution, LoadResult, LoadRollback, RemoveInstanceApproval,
    RemoveInstancePreview, ReplaceInstanceApproval, ReplaceInstanceCommand, ReplaceInstancePreview,
    ReplaceInstanceResult, ResetInstanceApproval, ResetInstanceKind, ResetInstancePreview,
    ResetInstanceResult, VirtualUniverse,
};

pub use plc_runtime::{
    ArtifactPackage, CanonicalValue, ChannelId, CommandId, CpuState, Hash32, InputCommand,
    InputReceipt, MemoryId, RunOutcome, RuntimeBoundaryCommand, RuntimeBoundaryReceipt,
    RuntimeForceDelta, RuntimeNaturalWrite, RuntimeScanCommand, RuntimeScanReceipt,
    RuntimeValueTarget, StateId, UniverseId, ValueType, VirtualControllerId,
};
