#![no_std]
#![forbid(unsafe_code)]
#![allow(
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::struct_excessive_bools
)]

//! Capability-free Phase 2 language tooling.
//!
//! FBD is represented as a typed semantic graph and lowered into the shared
//! `plc-compiler` IR. Geometry remains presentation-only. SCL services consume
//! compiler-owned lexical, syntactic, and semantic analysis; this crate never
//! parses or binds SCL independently and never executes either language.

extern crate alloc;

mod fbd;
mod fbd_diagnostic;
mod fbd_editor;
mod fbd_lowering;
mod ids;
mod scl_service;
mod type_adapter;

pub use fbd::{
    ActivationRole, ConnectionKind, DisabledOutputBehavior, EffectRole, FbdConnection, FbdDocument,
    FbdLayout, FbdNetwork, FbdNode, FbdPort, InstanceIdentity, NodeKind, NodeLayout, PortDirection,
    PortMultiplicity, PortStatus, RoutePoint, disabled_output_behavior,
};
pub use fbd_diagnostic::{
    DiagnosticSeverity, FbdDiagnostic, FbdDiagnosticCode, FbdValidationReport, validate_fbd,
    validate_fbd_with_program,
};
pub use fbd_editor::{FbdEdit, FbdEditError, apply_fbd_edits_atomically};
pub use fbd_lowering::{
    FbdLowerError, FbdLoweredProgram, FbdProbe, FbdProbeKind, FbdProbeTable, FbdSourceLocation,
    FbdSourceMapEntry, FbdSourceMapTable, VerifiedFbdProgram, lower_fbd_to_ir,
    lower_fbd_to_verified_ir,
};
pub use ids::{ConnectionId, FbdDocumentId, NetworkId, NodeId, PortId, StateInstanceId};
pub use scl_service::{
    CompletionItem, CompletionKind, HoverInfo, InterfaceRename, RenameError, RenamePlan,
    SclLanguageService, SclNavigationEntry, SclNavigationRelationship, SclNavigationTarget,
    SclNavigationValidity, SemanticToken, SemanticTokenKind, SignatureFormal, SignatureHelp,
    SourceEdit, SymbolDefinition,
};
pub use type_adapter::{TypeAdapterError, data_type_to_ir_type};

/// Canonical schema identity for semantic FBD documents in this Phase 2 slice.
pub const FBD_SCHEMA_VERSION: &str = "EDU-FBD-1";
