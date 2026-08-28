#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc, clippy::module_name_repetitions)]

//! Integrated, capability-free projections of the canonical project graph.
//!
//! This crate never owns a second editable PLC model. Every projection is
//! rebuilt from an immutable [`plc_core::Project`] snapshot, and every emitted
//! diagnostic retains a canonical project-object anchor.

mod build_product;
mod graph_projection;
mod hardware_projection;
mod replay_executor;
mod replay_package;
mod session;
mod software_projection;

pub use build_product::{
    RuntimeChannelBinding, SystemBuildError, SystemBuildProduct, SystemCompilerArtifact,
    build_project_controller,
};
pub use graph_projection::{
    CANONICAL_FBD_GRAPH_SCHEMA, CANONICAL_LAD_GRAPH_SCHEMA, DecodedGraphicalBody, GraphDecodeError,
    decode_graphical_body,
};
pub use session::{
    DiagnosticReadModel, EngineeringReadModel, EngineeringSession, EngineeringSessionSnapshot,
    EngineeringSnapshotHashes, EngineeringStatus, ProbeReadModel, ProjectRefresh, RestoreApproval,
    RestorePreview, SystemCommandIdentity, SystemError, TraceReadModel, WatchTableReadModel,
};

pub use hardware_projection::{
    CanonicalHardwareProjection, ProjectDiagnostic, ProjectDiagnosticPhase, project_hardware,
};
pub use replay_executor::{
    ENGINEERING_REPLAY_ALGORITHM, EngineeringReplayError, EngineeringReplayExecution,
    EngineeringReplayExecutor, engineering_replay_state_regions,
};
pub use replay_package::{
    ActorKind, CanonicalReplayPlcValue, ReplayActorProvenance, ReplayBoundaryHash,
    ReplayBoundaryKind, ReplayCommandResult, ReplayDecodeLimits, ReplayDivergence, ReplayMemberRef,
    ReplayPackage, ReplayPackageError, ReplayPackageEvent, ReplayPackageSpec, ReplayPayloadValue,
    ReplayPriorityClass, ReplayResultStatus, ReplayStateRegion, ReplayTypedPayload,
};
pub use software_projection::{
    AuthoredLanguage, CanonicalAddressArea, CanonicalAddressIntent, CanonicalDisplayBase,
    CanonicalHardwareAddress, CanonicalNamedType, CanonicalProbeLayer, CanonicalSoftwareProjection,
    CanonicalTag, CanonicalTagTarget, CanonicalTraceChannel, CanonicalTraceConfig,
    CanonicalWatchRow, CanonicalWatchTable, DATA_BLOCK_PAYLOAD_SCHEMA, GraphicalBodyHook,
    NAMED_TYPE_PAYLOAD_SCHEMA, PROGRAM_BLOCK_PAYLOAD_SCHEMA, TAG_PAYLOAD_SCHEMA,
    TRACE_CONFIG_PAYLOAD_SCHEMA, WATCH_TABLE_PAYLOAD_SCHEMA, project_software,
};

/// Schema version for the integrated Phase 2 canonical projection boundary.
pub const PLC_SYSTEM_SCHEMA_VERSION: &str = "EDU-SYSTEM-1";
