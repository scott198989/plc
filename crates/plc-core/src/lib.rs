#![forbid(unsafe_code)]

//! Deterministic, capability-free PLC project kernel and logical package codec.
//!
//! The crate accepts and returns ordinary values and byte slices. It never opens
//! files, resolves endpoints, reads clocks, starts processes, or accesses host
//! devices, which keeps the same API usable from native tests and `wasm32`.

mod engine;
mod hash;
mod journal;
mod json;
mod migration;
mod model;
mod package;
mod protocol;

pub use engine::{CopyPreview, Engine, EngineError};
pub use hash::{Sha256Digest, sha256};
pub use journal::{Journal, JournalError, JournalLimits, JournalRecord, recover_from_journal};
pub use migration::{
    MigrationBackup, MigrationChange, MigrationDefault, MigrationError, MigrationIdentityMapping,
    MigrationReport, MigrationStep, MigrationStepOutput, migrate_project,
};
pub use model::{
    CommandContext, CommandEnvelope, CommandOutcome, Comparison, ComparisonKind, DependencyEdge,
    DependencyReason, DerivedIndexes, Diagnostic, DomainCommand, DomainCommandResult, DomainEvent,
    Lifecycle, NewObject, ObjectId, Payload, PayloadValue, ProfilePin, Project, ProjectObject,
    ProjectObjectKind, ReferenceEdge, ReferenceKind, ResolutionState, SimulatorExtension,
    SimulatorExtensionError, TransactionId, UndoToken, Uuid,
};
pub use package::{
    ArchiveReceipt, ArchiveReference, DecodeLimits, LogicalPackage, Manifest, ManifestInspection,
    NativeImportPreview, PackageError, PackageInventoryEntry, ProjectArchive,
    ReadOnlyProjectReference, decode_project_package, encode_project_package,
    inspect_package_manifest, preview_native_import,
};
pub use protocol::{KernelSession, ProtocolError};
