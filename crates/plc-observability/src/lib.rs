#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod canonical;
mod context;
mod diagnostics;
mod execution;
mod force;
mod layers;
mod modify;
mod monitor;
mod navigation;
mod runtime_diagnostics;
mod target;
mod trace;

pub use context::{ContextError, ObservationContext, PublicationBoundary};
pub use diagnostics::{
    ActiveCondition, CausalReference, CausalResolution, CompactedDiagnosticReference, ConditionId,
    ConditionKey, DiagnosticCode, DiagnosticCpuResponse, DiagnosticDefinition, DiagnosticError,
    DiagnosticEvent, DiagnosticEventKind, DiagnosticId, DiagnosticLedger, DiagnosticLedgerSnapshot,
    DiagnosticLifecycle, DiagnosticLimits, DiagnosticOrderKey, DiagnosticRegistry,
    DiagnosticSeverity, DiagnosticSource, DiagnosticTransition, OccurrenceId,
};
pub use execution::{
    ExecutionError, ForceExecutionReceipt, ModifyExecutionReceipt, RuntimePublicationReceipt,
    execute_force_command, execute_force_command_with_io_state, publish_modify_plan,
};
pub use force::{
    ActiveForceSummary, ActiveForceSummaryEntry, ForceAuditAction, ForceAuditRecord,
    ForceAuditResult, ForceAuditTarget, ForceCommand, ForceCommandKind, ForceEntry, ForceError,
    ForceId, ForceReceipt, ForceRegistry, ForceRegistrySnapshot, ForceStatus, ForceWritePlan,
    GlobalForceProjection, GlobalForceProjectionEntry, RemoveAllApproval, RemoveAllPreview,
};
pub use layers::{
    CanonicalLayerBundle, CanonicalLayerSnapshot, EngineeringValueLayers, LayerCodecLimits,
    LayerError, LayerForce, LayerTargetKind, RuntimeValueLayers, ScalarEngineeringValueLayers,
    ScalarRuntimeValueLayers, scalar_layer_snapshot_from_publication,
};
pub use modify::{
    ModifyCommand, ModifyError, ModifyItem, ModifyReceipt, ModifyReceiptState, ModifyScheduler,
    ModifyWritePlan, PublicationPlan,
};
pub use monitor::{
    DisplayBase, ForceProvenance, MonitorError, MonitorFailure, MonitorSample, MonitorState,
    MonitoringEngine, MonitoringLimits, MonitoringPersistence, PublishedTargetValue, Quality,
    RuntimeIoState, SampleFreshness, WatchRow, WatchRowId, WatchTable, WatchTableId,
};
pub use navigation::{
    ArtifactSide, NavigationAnchor, NavigationError, NavigationIndex, NavigationIndexBuilder,
    NavigationKind, NavigationResult, SemanticIdentity,
};
pub use runtime_diagnostics::{
    RuntimeDiagnosticBridge, RuntimeDiagnosticBridgeError, RuntimeDiagnosticProviderKey,
    RuntimeDiagnosticReceipt,
};
pub use target::{
    AccessCapabilities, BitRange, ProbeCatalog, ProbeDefinition, ProbeLayer, ResolvedTarget,
    RuntimeTarget, SourceAnchor, StableTargetId, TargetError, TargetReference,
};
pub use trace::{
    ComparisonOperator, DiagnosticEventTrigger, ExpressionNode, GapReason, NumericValue,
    SaveTraceResultCommand, TraceAbortReason, TraceCadence, TraceCapture, TraceCaptureId,
    TraceChannel, TraceChannelId, TraceChannelSample, TraceConfig, TraceConfigId,
    TraceDiagnosticEvent, TraceEngine, TraceEngineSnapshot, TraceError, TraceEventKey, TraceExport,
    TraceExportFormat, TraceExportOptions, TraceLimits, TraceProbeIdentity, TraceProbeKind,
    TraceRuntimePublication, TraceSample, TraceSavedResult, TraceSavedResultId, TraceState,
    TraceTrigger, TraceTriggerId,
};

pub use plc_commissioning::{SessionCommandBinding, VirtualOnlineSessionId, VirtualUniverse};
pub use plc_runtime::{
    CanonicalValue, ChannelId, CpuState, Hash32, MemoryId, UniverseId, ValueType,
    VirtualControllerId,
};

pub const OBSERVABILITY_SEMANTICS_VERSION: &str = "EDU-OBS-1";
