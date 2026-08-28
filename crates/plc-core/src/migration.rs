#![allow(clippy::missing_errors_doc)]

//! Sequential, transactional project migration support.

use std::collections::BTreeSet;

use crate::{ObjectId, Project, Sha256Digest};

/// A registered adjacent schema migration. The callback must be deterministic
/// and idempotent; the runner checks both properties before committing.
#[derive(Clone, Copy)]
pub struct MigrationStep {
    pub from_version: u32,
    pub to_version: u32,
    pub name: &'static str,
    pub apply: fn(&mut Project) -> Result<MigrationStepOutput, String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MigrationStepOutput {
    pub affected_object_ids: Vec<ObjectId>,
    pub identity_mappings: Vec<MigrationIdentityMapping>,
    pub defaults_introduced: Vec<MigrationDefault>,
    pub warnings: Vec<String>,
    pub unsupported_features: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MigrationIdentityMapping {
    pub source_ids: Vec<ObjectId>,
    pub target_ids: Vec<ObjectId>,
    pub rationale: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MigrationDefault {
    pub object_id: ObjectId,
    pub field_path: String,
    pub canonical_value: String,
}

impl core::fmt::Debug for MigrationStep {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("MigrationStep")
            .field("from_version", &self.from_version)
            .field("to_version", &self.to_version)
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MigrationChange {
    pub from_version: u32,
    pub to_version: u32,
    pub name: String,
    pub before_hash: Sha256Digest,
    pub after_hash: Sha256Digest,
    pub affected_object_ids: Vec<ObjectId>,
    pub identity_mappings: Vec<MigrationIdentityMapping>,
    pub defaults_introduced: Vec<MigrationDefault>,
    pub warnings: Vec<String>,
    pub unsupported_features: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MigrationReport {
    pub source_version: u32,
    pub target_version: u32,
    /// Digest of the untouched source model. A host persists the encoded source
    /// package as its immutable backup before replacing any destination bytes.
    pub backup_hash: Sha256Digest,
    pub resulting_hash: Sha256Digest,
    pub changes: Vec<MigrationChange>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MigrationError {
    BackwardMigration,
    MissingStep(u32),
    AmbiguousStep(u32),
    NonAdjacentStep { from: u32, to: u32 },
    StepFailed { name: String, message: String },
    NonIdempotent(String),
    InvalidResult(String),
    IdentityChanged(String),
}

/// Applies a complete adjacent migration chain to a clone of `source`.
/// Failures leave `source` untouched and return no partially migrated model.
pub fn migrate_project(
    source: &Project,
    source_version: u32,
    target_version: u32,
    registry: &[MigrationStep],
) -> Result<(Project, MigrationReport), MigrationError> {
    if target_version < source_version {
        return Err(MigrationError::BackwardMigration);
    }
    let backup_hash = source.document_hash();
    let source_document_id = source.document_id();
    let source_root_id = source.root_id();
    let mut candidate = source.clone();
    let mut current = source_version;
    let mut changes = Vec::new();
    while current < target_version {
        let matching: Vec<_> = registry
            .iter()
            .filter(|step| step.from_version == current)
            .copied()
            .collect();
        let step = match matching.as_slice() {
            [] => return Err(MigrationError::MissingStep(current)),
            [step] => *step,
            _ => return Err(MigrationError::AmbiguousStep(current)),
        };
        if step.to_version != current.saturating_add(1) {
            return Err(MigrationError::NonAdjacentStep {
                from: step.from_version,
                to: step.to_version,
            });
        }
        let before_hash = candidate.document_hash();
        let mut migrated = candidate.clone();
        let mut output =
            (step.apply)(&mut migrated).map_err(|message| MigrationError::StepFailed {
                name: step.name.to_owned(),
                message,
            })?;
        migrated
            .validate()
            .map_err(|error| MigrationError::InvalidResult(format!("{error:?}")))?;
        if migrated.document_id() != source_document_id || migrated.root_id() != source_root_id {
            return Err(MigrationError::IdentityChanged(step.name.to_owned()));
        }
        validate_identity_changes(&candidate, &migrated, &output.identity_mappings, step.name)?;
        let mut idempotence_probe = migrated.clone();
        (step.apply)(&mut idempotence_probe).map_err(|message| MigrationError::StepFailed {
            name: step.name.to_owned(),
            message,
        })?;
        if idempotence_probe.document_hash() != migrated.document_hash() {
            return Err(MigrationError::NonIdempotent(step.name.to_owned()));
        }
        output.affected_object_ids.sort_unstable();
        output.affected_object_ids.dedup();
        let known: BTreeSet<_> = migrated.objects().map(|object| object.id).collect();
        if output
            .affected_object_ids
            .iter()
            .any(|id| !known.contains(id) && candidate.object(*id).is_none())
        {
            return Err(MigrationError::InvalidResult(
                "affected object list contains an unknown identity".to_owned(),
            ));
        }
        let after_hash = migrated.document_hash();
        changes.push(MigrationChange {
            from_version: current,
            to_version: step.to_version,
            name: step.name.to_owned(),
            before_hash,
            after_hash,
            affected_object_ids: output.affected_object_ids,
            identity_mappings: output.identity_mappings,
            defaults_introduced: output.defaults_introduced,
            warnings: output.warnings,
            unsupported_features: output.unsupported_features,
        });
        candidate = migrated;
        current = step.to_version;
    }
    let resulting_hash = candidate.document_hash();
    Ok((
        candidate,
        MigrationReport {
            source_version,
            target_version,
            backup_hash,
            resulting_hash,
            changes,
        },
    ))
}

fn validate_identity_changes(
    before: &Project,
    after: &Project,
    mappings: &[MigrationIdentityMapping],
    step_name: &str,
) -> Result<(), MigrationError> {
    let before_ids: BTreeSet<_> = before.objects().map(|object| object.id).collect();
    let after_ids: BTreeSet<_> = after.objects().map(|object| object.id).collect();
    let removed: BTreeSet<_> = before_ids.difference(&after_ids).copied().collect();
    let added: BTreeSet<_> = after_ids.difference(&before_ids).copied().collect();
    let mapped_sources: BTreeSet<_> = mappings
        .iter()
        .flat_map(|mapping| mapping.source_ids.iter().copied())
        .collect();
    let mapped_targets: BTreeSet<_> = mappings
        .iter()
        .flat_map(|mapping| mapping.target_ids.iter().copied())
        .collect();
    let valid_mappings = mappings.iter().all(|mapping| {
        !mapping.source_ids.is_empty()
            && !mapping.target_ids.is_empty()
            && !mapping.rationale.is_empty()
    });
    if !valid_mappings || removed != mapped_sources || added != mapped_targets {
        return Err(MigrationError::IdentityChanged(step_name.to_owned()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{ObjectId, ProfilePin, Project, Sha256Digest, Uuid};

    use super::{MigrationError, MigrationStep, MigrationStepOutput, migrate_project};

    #[allow(clippy::unnecessary_wraps)]
    fn no_op(_: &mut Project) -> Result<MigrationStepOutput, String> {
        Ok(MigrationStepOutput::default())
    }

    #[allow(clippy::unnecessary_wraps)]
    fn non_idempotent(project: &mut Project) -> Result<MigrationStepOutput, String> {
        project.document_revision = project
            .document_revision
            .checked_add(1)
            .expect("test revision space");
        Ok(MigrationStepOutput::default())
    }

    fn fixture() -> Project {
        Project::new(
            Uuid::deterministic_v4(b"migration-document", 1),
            ObjectId(Uuid::deterministic_v4(b"migration-root", 1)),
            "Migration",
            ProfilePin {
                id: "training".to_owned(),
                version: "1".to_owned(),
                manifest_hash: Sha256Digest([7; 32]),
            },
        )
    }

    #[test]
    fn requires_a_complete_sequential_chain() {
        let source = fixture();
        let registry = [MigrationStep {
            from_version: 1,
            to_version: 2,
            name: "one-to-two",
            apply: no_op,
        }];
        let (migrated, report) = migrate_project(&source, 1, 2, &registry).expect("migrate");
        assert_eq!(migrated, source);
        assert_eq!(report.backup_hash, source.document_hash());
        assert_eq!(report.changes.len(), 1);
        assert_eq!(
            migrate_project(&source, 1, 3, &registry),
            Err(MigrationError::MissingStep(2))
        );
    }

    #[test]
    fn rejects_non_idempotent_steps_without_mutating_source() {
        let source = fixture();
        let before = source.document_hash();
        let registry = [MigrationStep {
            from_version: 1,
            to_version: 2,
            name: "bad-step",
            apply: non_idempotent,
        }];
        assert_eq!(
            migrate_project(&source, 1, 2, &registry),
            Err(MigrationError::NonIdempotent("bad-step".to_owned()))
        );
        assert_eq!(source.document_hash(), before);
    }
}
