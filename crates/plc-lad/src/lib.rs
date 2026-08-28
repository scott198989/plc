#![no_std]
#![forbid(unsafe_code)]

//! Coordinate-independent Ladder Diagram semantics for Phase 2.
//!
//! This crate owns editable LAD graphs, atomic structural edits, deterministic
//! legality/type analysis, and lowering into the existing compiler IR data
//! model. It deliberately contains no renderer, runtime, host I/O, clock,
//! entropy, filesystem, network, process, device, FFI, or Phase 3 behavior.

extern crate alloc;

mod edit;
mod hash;
mod ids;
mod lowering;
mod model;
mod validate;

pub use edit::{LadEdit, LadEditError, LadUndo, apply_lad_edits_atomically};
pub use ids::{
    LadBranchId, LadBranchPathId, LadDocumentId, LadEdgeId, LadNetworkId, LadNodeId, LadOperandId,
    LadPortId, LadStateInstanceId,
};
pub use lowering::{
    LadIrArtifact, LadLowerError, SharedIrGap, SharedIrRequirement, lower_lad_to_ir,
};
pub use model::{
    CoilMode, ContactMode, LadBox, LadBranch, LadBranchPath, LadCall, LadDocument, LadFormalRef,
    LadInstance, LadLayout, LadNetwork, LadNode, LadNodeKind, LadOperand, LadOperandRef, LadPin,
    LadPinDirection, LadPortStatus, LadPowerEdge, LadPowerPort, LadPowerPortDirection,
    LadStateBinding, NodeLayout, RoutePoint,
};
pub use validate::{
    LadDiagnostic, LadDiagnosticReason, LadGraphReason, LadLimits, LadLocation, LadNetworkAnalysis,
    LadValidationReport, validate_lad,
};

/// Canonical editable LAD schema identity.
pub const LAD_SCHEMA_VERSION: &str = "EDU-LAD-GRAPH-1";

/// Limits from the Phase 2 compiler/resource contract.
pub const MAX_NETWORKS_PER_BLOCK: usize = 10_000;
pub const MAX_NODES_PER_NETWORK: usize = 10_000;
pub const MAX_EDGES_PER_NETWORK: usize = 20_000;
