#![no_std]
#![forbid(unsafe_code)]

//! Deterministic, capability-free Phase 2 PLC compiler.
//!
//! The crate consumes immutable canonical program/source snapshots and emits
//! immutable reports plus verified, non-host-executable typed IR. It contains
//! the initial SCL frontend, but deliberately contains no runtime executor,
//! filesystem, network, clock, entropy, process, device, FFI, UI, LAD, or FBD
//! capability.

extern crate alloc;

mod build;
mod composition;
mod diagnostic;
mod hash;
mod ids;
mod ir;
mod limits;
mod lowering;
mod runtime_adapter;
mod source;

pub mod scl;

pub use build::{
    ArtifactFreshness, ArtifactIntegrityHashes, BuildArtifact, BuildAttempt, BuildCache,
    BuildCompletion, BuildManifest, BuildMode, BuildOutcome, BuildPublicationState, BuildReport,
    BuildScope, BuildSnapshot, CacheLookup, Compiler, CompilerInitError, CompilerProfile,
    CompilerStage, DependencyRecord, ExpandedScope, ProfileError, PublicationDecision, ScopeError,
    SnapshotError, StageMetric,
};
pub use composition::{
    ComposedFrontendArtifact, CompositionError, CompositionIdentityKind, FrontendArtifact,
    SclFrontendError, SclLoweringFailure, compose_frontend_artifacts, lower_scl_frontend_artifact,
};
pub use diagnostic::{
    BuildDiagnostic, DiagnosticCode, DiagnosticDefinition, DiagnosticParameter,
    DiagnosticParameterKind, DiagnosticPhase, DiagnosticRegistry, DiagnosticSeverity,
    DiagnosticTarget, NavigationRole, RecoveryCategory, RegistryError, phase2_diagnostic_registry,
};
pub use ids::{
    BuildAttemptId, IrBasicBlockId, IrOperationId, IrValueId, ProbeId, SemanticNodeId, SourceMapId,
};
pub use ir::{
    BinaryOperator, IrActivation, IrAggregateSource, IrBasicBlock, IrBoundInput, IrDeclaredOutput,
    IrFormalRef, IrFunction, IrInstanceIdentity, IrOperation, IrOperationKind, IrTerminator,
    IrTerminatorKind, IrType, IrValue, ProbeDefinition, ProbeKind, ProbeTable,
    ResolvedSourceAnchor, RuntimeOperationId, SourceAnchorResolution,
    SourceAnchorUnavailableReason, SourceMapEntry, SourceMapSite, SourceMapTable, TypedIrProgram,
    UnaryOperator, VerificationError, VerifiedIr, verify_typed_ir,
};
pub use limits::{CancellationToken, ResourceLimit, ResourceLimits, ResourceProfileError};
pub use plc_runtime::Hash32;
pub use runtime_adapter::{
    RuntimeAdapterError, RuntimeAggregateMemoryBinding, RuntimeArtifactProjection,
    RuntimeBlockBinding, RuntimeMappedSite, RuntimeMemoryBinding, RuntimeSourceBinding,
    project_verified_ir_to_runtime,
};
pub use source::{
    GraphSourceIds, LineColumn, SclSource, SourceAnchor, SourceLanguage, StableSourceIdentity,
    TextRange,
};

/// Compiler semantics are immutable build input. Any semantic change requires
/// a new value.
pub const COMPILER_SEMANTICS_VERSION: &str = "EDU-CMP-2";

/// Canonical type and conversion identities pinned into every report/artifact.
pub const TYPE_SYSTEM_VERSION: &str = "EDU-TYPE-1";
pub const ARITHMETIC_POLICY_VERSION: &str = "EDU-ARITH-1";
pub const CONVERSION_POLICY_VERSION: &str = "EDU-CONVERT-1";

/// Initial typed IR and probe schema identities.
pub const TYPED_IR_VERSION: &str = "EDU-IR-2";
pub const PROBE_SCHEMA_VERSION: &str = "EDU-PROBE-1";
pub const BUILD_ARTIFACT_SCHEMA: &str = "EDU-BUILD-ARTIFACT-1";
