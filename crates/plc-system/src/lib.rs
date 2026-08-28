#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc, clippy::module_name_repetitions)]

//! Integrated, capability-free projections of the canonical project graph.
//!
//! This crate never owns a second editable PLC model. Every projection is
//! rebuilt from an immutable [`plc_core::Project`] snapshot, and every emitted
//! diagnostic retains a canonical project-object anchor.

mod hardware_projection;

pub use hardware_projection::{
    CanonicalHardwareProjection, ProjectDiagnostic, ProjectDiagnosticPhase, project_hardware,
};

/// Schema version for the integrated Phase 2 canonical projection boundary.
pub const PLC_SYSTEM_SCHEMA_VERSION: &str = "EDU-SYSTEM-1";
