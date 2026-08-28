use alloc::{collections::BTreeMap, string::String, vec::Vec};
use core::{error::Error, fmt};

pub use plc_commissioning::ForceId;
use plc_runtime::{CanonicalValue, CpuState, Hash32, SCAN_QUANTUM_MS, ValueType};

use crate::{
    BitRange, ObservationContext, ProbeCatalog, ProbeLayer, PublicationBoundary, ResolvedTarget,
    RuntimeIoState, RuntimeTarget, StableTargetId, TargetReference,
    canonical::{CanonicalHasher, id128},
    target::encode_runtime_target,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ForceStatus {
    Active = 1,
    Removed = 2,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForceEntry {
    pub id: ForceId,
    pub controller_id: u128,
    pub target_id: StableTargetId,
    pub runtime_target: RuntimeTarget,
    pub instance_path: Vec<u128>,
    pub bit_range: BitRange,
    pub value_type: ValueType,
    pub value: CanonicalValue,
    pub natural_at_application: CanonicalValue,
    pub target_layer: ProbeLayer,
    pub underlying_quality: crate::Quality,
    pub quality_warning: bool,
    pub activation_boundary: PublicationBoundary,
    pub status: ForceStatus,
    pub created_universe_epoch: u64,
    pub created_controller_epoch: u64,
    pub created_session_epoch: u64,
    pub bound_universe_epoch: u64,
    pub bound_controller_epoch: u64,
    pub artifact_fingerprint: Hash32,
    pub actor_identity: u128,
    pub reason: String,
    pub audit_context_hash: Hash32,
    pub revision: u64,
    pub entry_hash: Hash32,
}

impl ForceEntry {
    fn calculate_hash(&self) -> Hash32 {
        let mut hasher = CanonicalHasher::new("PES-FORCE-ENTRY-1");
        hasher.u128(self.id.0);
        hasher.u128(self.controller_id);
        hasher.u128(self.target_id.0);
        encode_runtime_target(self.runtime_target, &mut hasher);
        hasher.u64(self.instance_path.len() as u64);
        for identity in &self.instance_path {
            hasher.u128(*identity);
        }
        hasher.u16(self.bit_range.offset);
        hasher.u16(self.bit_range.width);
        hasher.u8(self.value_type as u8);
        hasher.value(self.value);
        hasher.value(self.natural_at_application);
        hasher.u8(self.target_layer as u8);
        hasher.u8(self.underlying_quality as u8);
        hasher.bool(self.quality_warning);
        hasher.u8(self.activation_boundary as u8);
        hasher.u8(self.status as u8);
        hasher.u64(self.created_universe_epoch);
        hasher.u64(self.created_controller_epoch);
        hasher.u64(self.created_session_epoch);
        hasher.u64(self.bound_universe_epoch);
        hasher.u64(self.bound_controller_epoch);
        hasher.hash(self.artifact_fingerprint);
        hasher.u128(self.actor_identity);
        hasher.string(&self.reason);
        hasher.hash(self.audit_context_hash);
        hasher.u64(self.revision);
        hasher.finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveForceSummaryEntry {
    pub force_id: ForceId,
    pub controller_id: u128,
    pub target_id: StableTargetId,
    pub navigation_identity: StableTargetId,
    pub instance_path: Vec<u128>,
    pub value_type: ValueType,
    pub value: CanonicalValue,
    pub target_layer: ProbeLayer,
    pub status: ForceStatus,
    pub entry_hash: Hash32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveForceSummary {
    pub count: usize,
    pub registry_version: u64,
    pub registry_hash: Hash32,
    pub entries: Vec<ActiveForceSummaryEntry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ForceAuditAction {
    Create = 1,
    Replace = 2,
    Remove = 3,
    RemoveAll = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ForceAuditResult {
    Applied = 1,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForceAuditTarget {
    pub force_id: ForceId,
    pub controller_id: u128,
    pub target_id: StableTargetId,
    pub navigation_identity: StableTargetId,
    pub runtime_target: RuntimeTarget,
    pub instance_path: Vec<u128>,
    pub bit_range: BitRange,
    pub value_type: ValueType,
    pub typed_value: CanonicalValue,
    pub target_layer: ProbeLayer,
    pub entry_hash: Hash32,
}

impl ForceAuditTarget {
    fn from_entry(entry: &ForceEntry) -> Self {
        Self {
            force_id: entry.id,
            controller_id: entry.controller_id,
            target_id: entry.target_id,
            navigation_identity: entry.target_id,
            runtime_target: entry.runtime_target,
            instance_path: entry.instance_path.clone(),
            bit_range: entry.bit_range,
            value_type: entry.value_type,
            typed_value: entry.value,
            target_layer: entry.target_layer,
            entry_hash: entry.entry_hash,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForceAuditRecord {
    pub ordinal: u64,
    pub action: ForceAuditAction,
    pub result: ForceAuditResult,
    pub command_id: u128,
    pub idempotency_key: u128,
    pub force_id: ForceId,
    pub requester_identity: u128,
    pub reason: String,
    pub before: Option<ForceAuditTarget>,
    pub after: Option<ForceAuditTarget>,
    pub old_registry_version: u64,
    pub new_registry_version: u64,
    pub old_registry_hash: Hash32,
    pub new_registry_hash: Hash32,
    pub universe_epoch: u64,
    pub controller_id: u128,
    pub controller_epoch: u64,
    pub session_epoch: u64,
    pub artifact_fingerprint: Hash32,
    pub applied_boundary: PublicationBoundary,
    pub first_affected_scan_sequence: u64,
    pub first_affected_event_sequence: u64,
    pub first_affected_virtual_timestamp_ms: u64,
    pub audit_context_hash: Hash32,
    pub prior_record_hash: Hash32,
    pub record_hash: Hash32,
}

impl ForceAuditRecord {
    pub fn verify(&self) -> bool {
        self.record_hash == hash_force_audit_record(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlobalForceProjectionEntry {
    pub registry_ordinal: u64,
    pub navigation_identity: StableTargetId,
    pub force: ForceEntry,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlobalForceProjection {
    pub schema_version: u32,
    pub count: usize,
    pub controller_ids: Vec<u128>,
    pub registry_version: u64,
    pub registry_hash: Hash32,
    pub audit_record_count: usize,
    pub audit_head_hash: Hash32,
    pub entries: Vec<GlobalForceProjectionEntry>,
    pub projection_hash: Hash32,
}

impl GlobalForceProjection {
    pub fn verify(&self) -> bool {
        let mut controller_ids = self
            .entries
            .iter()
            .map(|entry| entry.force.controller_id)
            .collect::<Vec<_>>();
        controller_ids.sort_unstable();
        controller_ids.dedup();
        self.schema_version == 1
            && self.count == self.entries.len()
            && controller_ids == self.controller_ids
            && self.entries.iter().enumerate().all(|(ordinal, entry)| {
                entry.registry_ordinal == ordinal as u64
                    && entry.navigation_identity == entry.force.target_id
                    && entry.force.entry_hash == entry.force.calculate_hash()
            })
            && self.registry_hash
                == hash_projected_force_registry(self.registry_version, &self.entries)
            && self.projection_hash == hash_global_force_projection(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ForceCommandKind {
    Create {
        force_id: ForceId,
        target: TargetReference,
        value: CanonicalValue,
        natural_at_application: CanonicalValue,
        actor_identity: u128,
        reason: String,
    },
    Replace {
        force_id: ForceId,
        expected_entry_hash: Hash32,
        value: CanonicalValue,
        actor_identity: u128,
        reason: String,
    },
    Remove {
        force_id: ForceId,
        expected_entry_hash: Hash32,
        actor_identity: u128,
        reason: String,
    },
    RemoveAll {
        approval: RemoveAllApproval,
        actor_identity: u128,
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForceCommand {
    pub command_id: u128,
    pub idempotency_key: u128,
    pub expected_universe_epoch: u64,
    pub expected_controller_epoch: u64,
    pub expected_session_epoch: u64,
    pub expected_artifact_fingerprint: Hash32,
    pub expected_target_state_hash: Hash32,
    pub expected_registry_version: u64,
    pub expected_registry_hash: Hash32,
    pub audit_context_hash: Hash32,
    pub kind: ForceCommandKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForceWritePlan {
    pub command_id: u128,
    pub boundary: PublicationBoundary,
    pub expected_target_state_hash: Hash32,
    pub expected_registry_hash: Hash32,
    pub next_registry_hash: Hash32,
    pub set_values: Vec<(RuntimeTarget, CanonicalValue)>,
    pub remove_targets: Vec<RuntimeTarget>,
    pub resulting_force_ids: Vec<ForceId>,
    pub plan_hash: Hash32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForceReceipt {
    pub command_id: u128,
    pub idempotency_key: u128,
    pub duplicate: bool,
    pub applied_boundary: PublicationBoundary,
    pub registry_version: u64,
    pub registry_hash: Hash32,
    pub plan_hash: Hash32,
    pub affected_force_ids: Vec<ForceId>,
    pub affected_targets: Vec<ForceAuditTarget>,
    pub first_affected_scan_sequence: u64,
    pub first_affected_event_sequence: u64,
    pub first_affected_virtual_timestamp_ms: u64,
    pub audit_record_hashes: Vec<Hash32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StoredReceipt {
    payload_hash: Hash32,
    receipt: ForceReceipt,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoveAllPreview {
    pub preview_id: u128,
    pub controller_epoch: u64,
    pub target_state_hash: Hash32,
    pub registry_version: u64,
    pub registry_hash: Hash32,
    pub force_ids: Vec<ForceId>,
    pub preview_hash: Hash32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RemoveAllApproval {
    pub preview_id: u128,
    pub preview_hash: Hash32,
    pub controller_epoch: u64,
    pub target_state_hash: Hash32,
    pub registry_version: u64,
    pub registry_hash: Hash32,
}

impl RemoveAllApproval {
    pub fn approve(preview: &RemoveAllPreview) -> Self {
        Self {
            preview_id: preview.preview_id,
            preview_hash: preview.preview_hash,
            controller_epoch: preview.controller_epoch,
            target_state_hash: preview.target_state_hash,
            registry_version: preview.registry_version,
            registry_hash: preview.registry_hash,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForceRegistrySnapshot {
    pub schema_version: u32,
    pub universe_id: u128,
    pub controller_id: u128,
    pub controller_epoch: u64,
    pub artifact_fingerprint: Hash32,
    pub profile_fingerprint: Hash32,
    pub registry_version: u64,
    pub entries: Vec<ForceEntry>,
    pub audit_records: Vec<ForceAuditRecord>,
    pub audit_head_hash: Hash32,
    pub registry_hash: Hash32,
    pub content_hash: Hash32,
}

#[derive(Clone, Debug)]
pub struct ForceRegistry {
    entries: BTreeMap<ForceId, ForceEntry>,
    receipts: BTreeMap<u128, StoredReceipt>,
    audit_records: Vec<ForceAuditRecord>,
    audit_head_hash: Hash32,
    registry_version: u64,
    registry_hash: Hash32,
}

impl Default for ForceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ForceRegistry {
    pub fn new() -> Self {
        let mut value = Self {
            entries: BTreeMap::new(),
            receipts: BTreeMap::new(),
            audit_records: Vec::new(),
            audit_head_hash: Hash32::ZERO,
            registry_version: 0,
            registry_hash: Hash32::ZERO,
        };
        value.registry_hash = value.calculate_hash();
        value
    }

    pub const fn version(&self) -> u64 {
        self.registry_version
    }

    pub const fn registry_hash(&self) -> Hash32 {
        self.registry_hash
    }

    pub fn entries(&self) -> impl ExactSizeIterator<Item = &ForceEntry> {
        self.entries.values()
    }

    pub fn entry(&self, id: ForceId) -> Option<&ForceEntry> {
        self.entries.get(&id)
    }

    pub fn active_ids(&self) -> Vec<ForceId> {
        self.entries.keys().copied().collect()
    }

    pub fn active_summary(&self) -> ActiveForceSummary {
        let entries = self
            .entries
            .values()
            .map(|entry| ActiveForceSummaryEntry {
                force_id: entry.id,
                controller_id: entry.controller_id,
                target_id: entry.target_id,
                navigation_identity: entry.target_id,
                instance_path: entry.instance_path.clone(),
                value_type: entry.value_type,
                value: entry.value,
                target_layer: entry.target_layer,
                status: entry.status,
                entry_hash: entry.entry_hash,
            })
            .collect::<Vec<_>>();
        ActiveForceSummary {
            count: entries.len(),
            registry_version: self.registry_version,
            registry_hash: self.registry_hash,
            entries,
        }
    }

    pub fn audit_records(&self) -> impl ExactSizeIterator<Item = &ForceAuditRecord> {
        self.audit_records.iter()
    }

    pub const fn audit_head_hash(&self) -> Hash32 {
        self.audit_head_hash
    }

    pub fn global_projection(&self) -> GlobalForceProjection {
        let entries = self
            .entries
            .values()
            .enumerate()
            .map(|(ordinal, entry)| GlobalForceProjectionEntry {
                registry_ordinal: ordinal as u64,
                navigation_identity: entry.target_id,
                force: entry.clone(),
            })
            .collect::<Vec<_>>();
        let mut controller_ids = entries
            .iter()
            .map(|entry| entry.force.controller_id)
            .collect::<Vec<_>>();
        controller_ids.sort_unstable();
        controller_ids.dedup();
        let mut projection = GlobalForceProjection {
            schema_version: 1,
            count: entries.len(),
            controller_ids,
            registry_version: self.registry_version,
            registry_hash: self.registry_hash,
            audit_record_count: self.audit_records.len(),
            audit_head_hash: self.audit_head_hash,
            entries,
            projection_hash: Hash32::ZERO,
        };
        projection.projection_hash = hash_global_force_projection(&projection);
        projection
    }

    pub fn verify_audit_stream(&self) -> bool {
        verify_force_audit_stream(&self.audit_records, self.audit_head_hash)
    }

    pub fn overlaps(&self, runtime_target: RuntimeTarget, bit_range: BitRange) -> bool {
        self.entries.values().any(|entry| {
            entry.runtime_target == runtime_target && entry.bit_range.overlaps(bit_range)
        })
    }

    pub fn effective_value(
        &self,
        runtime_target: RuntimeTarget,
        natural: CanonicalValue,
    ) -> (CanonicalValue, Option<ForceId>) {
        self.entries
            .values()
            .find(|entry| {
                entry.runtime_target == runtime_target && entry.bit_range == BitRange::whole_value()
            })
            .map_or((natural, None), |entry| (entry.value, Some(entry.id)))
    }

    pub fn preview_remove_all(&self, context: ObservationContext) -> RemoveAllPreview {
        let force_ids = self.active_ids();
        let mut preview = RemoveAllPreview {
            preview_id: 0,
            controller_epoch: context.controller_epoch,
            target_state_hash: context.target_state_hash,
            registry_version: self.registry_version,
            registry_hash: self.registry_hash,
            force_ids,
            preview_hash: Hash32::ZERO,
        };
        preview.preview_hash = hash_remove_all_preview(&preview);
        preview.preview_id = id128(preview.preview_hash);
        preview.preview_hash = hash_remove_all_preview(&preview);
        preview
    }

    pub fn apply_at_boundary(
        &mut self,
        command: &ForceCommand,
        context: ObservationContext,
        catalog: &ProbeCatalog,
    ) -> Result<(ForceReceipt, ForceWritePlan), ForceError> {
        self.apply_at_boundary_with_io_state(command, context, catalog, &[])
    }

    pub fn apply_at_boundary_with_io_state(
        &mut self,
        command: &ForceCommand,
        context: ObservationContext,
        catalog: &ProbeCatalog,
        io_states: &[RuntimeIoState],
    ) -> Result<(ForceReceipt, ForceWritePlan), ForceError> {
        let payload_hash = hash_force_command(command);
        if let Some(stored) = self.receipts.get(&command.idempotency_key) {
            if stored.payload_hash != payload_hash {
                return Err(ForceError::IdempotencyCollision);
            }
            let mut receipt = stored.receipt.clone();
            receipt.duplicate = true;
            return Ok((
                receipt.clone(),
                ForceWritePlan {
                    command_id: receipt.command_id,
                    boundary: receipt.applied_boundary,
                    expected_target_state_hash: command.expected_target_state_hash,
                    expected_registry_hash: command.expected_registry_hash,
                    next_registry_hash: receipt.registry_hash,
                    set_values: Vec::new(),
                    remove_targets: Vec::new(),
                    resulting_force_ids: self.active_ids(),
                    plan_hash: receipt.plan_hash,
                },
            ));
        }
        validate_command_context(command, context)?;
        if command.expected_registry_version != self.registry_version
            || command.expected_registry_hash != self.registry_hash
        {
            return Err(ForceError::RegistryCompareAndSwapFailed);
        }
        let boundary = force_boundary(context.cpu_state, &command.kind)?;
        if boundary != context.publication_boundary {
            return Err(ForceError::WrongPublicationBoundary {
                required: boundary,
                actual: context.publication_boundary,
            });
        }

        let mut candidate = self.clone();
        let old_registry_version = self.registry_version;
        let old_registry_hash = self.registry_hash;
        let mut set_values = Vec::new();
        let mut remove_targets = Vec::new();
        let mut affected = Vec::new();
        match &command.kind {
            ForceCommandKind::Create {
                force_id,
                target,
                value,
                natural_at_application,
                actor_identity,
                reason,
            } => {
                if reason.is_empty() {
                    return Err(ForceError::ReasonRequired);
                }
                if candidate.entries.contains_key(force_id) {
                    return Err(ForceError::DuplicateForceId(*force_id));
                }
                let resolved = resolve_force_target(catalog, target, context)?;
                if value.value_type() != resolved.value_type
                    || natural_at_application.value_type() != resolved.value_type
                {
                    return Err(ForceError::TypeMismatch);
                }
                if candidate.overlaps(resolved.runtime_target, resolved.bit_range) {
                    return Err(ForceError::TargetOverlap);
                }
                let (underlying_quality, quality_warning) =
                    validate_force_io_quality(resolved.id, resolved.runtime_target, io_states)?;
                let mut entry = ForceEntry {
                    id: *force_id,
                    controller_id: context.controller_id.0,
                    target_id: resolved.id,
                    runtime_target: resolved.runtime_target,
                    instance_path: resolved.instance_path,
                    bit_range: resolved.bit_range,
                    value_type: resolved.value_type,
                    value: *value,
                    natural_at_application: *natural_at_application,
                    target_layer: ProbeLayer::Effective,
                    underlying_quality,
                    quality_warning,
                    activation_boundary: boundary,
                    status: ForceStatus::Active,
                    created_universe_epoch: context.universe_epoch,
                    created_controller_epoch: context.controller_epoch,
                    created_session_epoch: context.session_epoch,
                    bound_universe_epoch: context.universe_epoch,
                    bound_controller_epoch: context.controller_epoch,
                    artifact_fingerprint: context.artifact_fingerprint,
                    actor_identity: *actor_identity,
                    reason: reason.clone(),
                    audit_context_hash: command.audit_context_hash,
                    revision: 1,
                    entry_hash: Hash32::ZERO,
                };
                entry.entry_hash = entry.calculate_hash();
                candidate.entries.insert(*force_id, entry);
                set_values.push((resolved.runtime_target, *value));
                affected.push(*force_id);
            }
            ForceCommandKind::Replace {
                force_id,
                expected_entry_hash,
                value,
                actor_identity,
                reason,
            } => {
                if reason.is_empty() {
                    return Err(ForceError::ReasonRequired);
                }
                let entry = candidate
                    .entries
                    .get_mut(force_id)
                    .ok_or(ForceError::UnknownForce(*force_id))?;
                if entry.entry_hash != *expected_entry_hash {
                    return Err(ForceError::EntryCompareAndSwapFailed);
                }
                if value.value_type() != entry.value.value_type() {
                    return Err(ForceError::TypeMismatch);
                }
                let (underlying_quality, quality_warning) =
                    validate_force_io_quality(entry.target_id, entry.runtime_target, io_states)?;
                entry.value = *value;
                entry.underlying_quality = underlying_quality;
                entry.quality_warning = quality_warning;
                entry.actor_identity = *actor_identity;
                entry.reason = reason.clone();
                entry.audit_context_hash = command.audit_context_hash;
                entry.revision = entry.revision.saturating_add(1);
                entry.entry_hash = entry.calculate_hash();
                set_values.push((entry.runtime_target, *value));
                affected.push(*force_id);
            }
            ForceCommandKind::Remove {
                force_id,
                expected_entry_hash,
                actor_identity: _,
                reason,
            } => {
                if reason.is_empty() {
                    return Err(ForceError::ReasonRequired);
                }
                let entry = candidate
                    .entries
                    .get(force_id)
                    .ok_or(ForceError::UnknownForce(*force_id))?;
                if entry.entry_hash != *expected_entry_hash {
                    return Err(ForceError::EntryCompareAndSwapFailed);
                }
                remove_targets.push(entry.runtime_target);
                candidate.entries.remove(force_id);
                affected.push(*force_id);
            }
            ForceCommandKind::RemoveAll {
                approval,
                actor_identity: _,
                reason,
            } => {
                if reason.is_empty() {
                    return Err(ForceError::ReasonRequired);
                }
                let preview = candidate.preview_remove_all(context);
                if *approval != RemoveAllApproval::approve(&preview) {
                    return Err(ForceError::RemoveAllApprovalMismatch);
                }
                for entry in candidate.entries.values() {
                    remove_targets.push(entry.runtime_target);
                    affected.push(entry.id);
                }
                candidate.entries.clear();
            }
        }

        candidate.registry_version = candidate.registry_version.saturating_add(1);
        candidate.registry_hash = candidate.calculate_hash();
        let audit_records = build_force_audit_records(
            self,
            &candidate,
            command,
            context,
            boundary,
            &affected,
            old_registry_version,
            old_registry_hash,
        );
        if let Some(last) = audit_records.last() {
            candidate.audit_head_hash = last.record_hash;
        }
        candidate.audit_records.extend(audit_records.clone());
        let mut plan = ForceWritePlan {
            command_id: command.command_id,
            boundary,
            expected_target_state_hash: context.target_state_hash,
            expected_registry_hash: self.registry_hash,
            next_registry_hash: candidate.registry_hash,
            set_values,
            remove_targets,
            resulting_force_ids: candidate.active_ids(),
            plan_hash: Hash32::ZERO,
        };
        plan.plan_hash = hash_force_plan(&plan);
        let (
            first_affected_scan_sequence,
            first_affected_event_sequence,
            first_affected_virtual_timestamp_ms,
        ) = predicted_force_boundary(context, boundary);
        let receipt = ForceReceipt {
            command_id: command.command_id,
            idempotency_key: command.idempotency_key,
            duplicate: false,
            applied_boundary: boundary,
            registry_version: candidate.registry_version,
            registry_hash: candidate.registry_hash,
            plan_hash: plan.plan_hash,
            affected_force_ids: affected,
            affected_targets: audit_records
                .iter()
                .filter_map(|record| record.after.clone().or_else(|| record.before.clone()))
                .collect(),
            first_affected_scan_sequence,
            first_affected_event_sequence,
            first_affected_virtual_timestamp_ms,
            audit_record_hashes: audit_records
                .iter()
                .map(|record| record.record_hash)
                .collect(),
        };
        candidate.receipts.insert(
            command.idempotency_key,
            StoredReceipt {
                payload_hash,
                receipt: receipt.clone(),
            },
        );
        *self = candidate;
        Ok((receipt, plan))
    }

    pub fn snapshot(&self, context: ObservationContext) -> ForceRegistrySnapshot {
        let mut snapshot = ForceRegistrySnapshot {
            schema_version: 2,
            universe_id: context.universe_id.0,
            controller_id: context.controller_id.0,
            controller_epoch: context.controller_epoch,
            artifact_fingerprint: context.artifact_fingerprint,
            profile_fingerprint: context.profile_fingerprint,
            registry_version: self.registry_version,
            entries: self.entries.values().cloned().collect(),
            audit_records: self.audit_records.clone(),
            audit_head_hash: self.audit_head_hash,
            registry_hash: self.registry_hash,
            content_hash: Hash32::ZERO,
        };
        snapshot.content_hash = hash_force_snapshot(&snapshot);
        snapshot
    }

    pub fn rebind_snapshot(
        snapshot: &ForceRegistrySnapshot,
        context: ObservationContext,
        catalog: &ProbeCatalog,
    ) -> Result<(Self, ForceWritePlan), ForceError> {
        if snapshot.schema_version != 2 || snapshot.content_hash != hash_force_snapshot(snapshot) {
            return Err(ForceError::SnapshotIntegrityMismatch);
        }
        if snapshot.universe_id != context.universe_id.0
            || snapshot.controller_id != context.controller_id.0
            || snapshot.artifact_fingerprint != context.artifact_fingerprint
            || snapshot.profile_fingerprint != context.profile_fingerprint
        {
            return Err(ForceError::SnapshotBindingMismatch);
        }
        if !verify_force_audit_stream(&snapshot.audit_records, snapshot.audit_head_hash) {
            return Err(ForceError::SnapshotIntegrityMismatch);
        }
        let mut registry = Self::new();
        registry.audit_records = snapshot.audit_records.clone();
        registry.audit_head_hash = snapshot.audit_head_hash;
        let mut set_values = Vec::new();
        for old in &snapshot.entries {
            if old.entry_hash != old.calculate_hash() {
                return Err(ForceError::SnapshotIntegrityMismatch);
            }
            let definition = catalog
                .definition(old.target_id)
                .ok_or(ForceError::TargetUnavailable)?;
            if !definition.capabilities.force
                || old.controller_id != context.controller_id.0
                || definition.runtime_target != old.runtime_target
                || definition.instance_path != old.instance_path
                || definition.value_type != old.value_type
                || old.value_type != old.value.value_type()
                || old.target_layer != ProbeLayer::Effective
                || old.status != ForceStatus::Active
            {
                return Err(ForceError::TargetUnavailable);
            }
            if registry.overlaps(old.runtime_target, old.bit_range) {
                return Err(ForceError::TargetOverlap);
            }
            let mut rebound = old.clone();
            rebound.bound_universe_epoch = context.universe_epoch;
            rebound.bound_controller_epoch = context.controller_epoch;
            rebound.revision = rebound.revision.saturating_add(1);
            rebound.entry_hash = rebound.calculate_hash();
            set_values.push((rebound.runtime_target, rebound.value));
            registry.entries.insert(rebound.id, rebound);
        }
        registry.registry_version = snapshot.registry_version.saturating_add(1);
        registry.registry_hash = registry.calculate_hash();
        let mut plan = ForceWritePlan {
            command_id: id128(snapshot.content_hash),
            boundary: force_creation_boundary(context.cpu_state)?,
            expected_target_state_hash: context.target_state_hash,
            expected_registry_hash: ForceRegistry::new().registry_hash,
            next_registry_hash: registry.registry_hash,
            set_values,
            remove_targets: Vec::new(),
            resulting_force_ids: registry.active_ids(),
            plan_hash: Hash32::ZERO,
        };
        plan.plan_hash = hash_force_plan(&plan);
        Ok((registry, plan))
    }

    fn calculate_hash(&self) -> Hash32 {
        let mut hasher = CanonicalHasher::new("PES-FORCE-REGISTRY-1");
        if self.registry_version == 0 && self.entries.is_empty() {
            return hasher.finish();
        }
        hasher.u64(self.registry_version);
        hasher.u64(self.entries.len() as u64);
        for entry in self.entries.values() {
            hasher.hash(entry.entry_hash);
        }
        hasher.finish()
    }
}

#[allow(clippy::too_many_arguments)]
fn build_force_audit_records(
    before: &ForceRegistry,
    after: &ForceRegistry,
    command: &ForceCommand,
    context: ObservationContext,
    boundary: PublicationBoundary,
    affected: &[ForceId],
    old_registry_version: u64,
    old_registry_hash: Hash32,
) -> Vec<ForceAuditRecord> {
    let (action, requester_identity, reason) = force_audit_command(&command.kind);
    let (
        first_affected_scan_sequence,
        first_affected_event_sequence,
        first_affected_virtual_timestamp_ms,
    ) = predicted_force_boundary(context, boundary);
    let mut prior_record_hash = before.audit_head_hash;
    let mut records = Vec::with_capacity(affected.len());
    for force_id in affected {
        let mut record = ForceAuditRecord {
            ordinal: before.audit_records.len() as u64 + records.len() as u64 + 1,
            action,
            result: ForceAuditResult::Applied,
            command_id: command.command_id,
            idempotency_key: command.idempotency_key,
            force_id: *force_id,
            requester_identity,
            reason: reason.into(),
            before: before
                .entries
                .get(force_id)
                .map(ForceAuditTarget::from_entry),
            after: after
                .entries
                .get(force_id)
                .map(ForceAuditTarget::from_entry),
            old_registry_version,
            new_registry_version: after.registry_version,
            old_registry_hash,
            new_registry_hash: after.registry_hash,
            universe_epoch: context.universe_epoch,
            controller_id: context.controller_id.0,
            controller_epoch: context.controller_epoch,
            session_epoch: context.session_epoch,
            artifact_fingerprint: context.artifact_fingerprint,
            applied_boundary: boundary,
            first_affected_scan_sequence,
            first_affected_event_sequence,
            first_affected_virtual_timestamp_ms,
            audit_context_hash: command.audit_context_hash,
            prior_record_hash,
            record_hash: Hash32::ZERO,
        };
        record.record_hash = hash_force_audit_record(&record);
        prior_record_hash = record.record_hash;
        records.push(record);
    }
    records
}

fn force_audit_command(kind: &ForceCommandKind) -> (ForceAuditAction, u128, &str) {
    match kind {
        ForceCommandKind::Create {
            actor_identity,
            reason,
            ..
        } => (ForceAuditAction::Create, *actor_identity, reason),
        ForceCommandKind::Replace {
            actor_identity,
            reason,
            ..
        } => (ForceAuditAction::Replace, *actor_identity, reason),
        ForceCommandKind::Remove {
            actor_identity,
            reason,
            ..
        } => (ForceAuditAction::Remove, *actor_identity, reason),
        ForceCommandKind::RemoveAll {
            actor_identity,
            reason,
            ..
        } => (ForceAuditAction::RemoveAll, *actor_identity, reason),
    }
}

fn predicted_force_boundary(
    context: ObservationContext,
    boundary: PublicationBoundary,
) -> (u64, u64, u64) {
    let scan_delta = u64::from(boundary == PublicationBoundary::ScanEnd);
    let time_delta = if boundary == PublicationBoundary::ScanEnd {
        SCAN_QUANTUM_MS
    } else {
        0
    };
    (
        context.scan_sequence.saturating_add(scan_delta),
        context.event_sequence.saturating_add(1),
        context.virtual_timestamp_ms.saturating_add(time_delta),
    )
}

fn verify_force_audit_stream(records: &[ForceAuditRecord], expected_head: Hash32) -> bool {
    let mut prior = Hash32::ZERO;
    for (index, record) in records.iter().enumerate() {
        if record.ordinal != index as u64 + 1
            || record.prior_record_hash != prior
            || !record.verify()
        {
            return false;
        }
        prior = record.record_hash;
    }
    prior == expected_head
}

fn hash_force_audit_target(target: &ForceAuditTarget, hasher: &mut CanonicalHasher) {
    hasher.u128(target.force_id.0);
    hasher.u128(target.controller_id);
    hasher.u128(target.target_id.0);
    hasher.u128(target.navigation_identity.0);
    encode_runtime_target(target.runtime_target, hasher);
    hasher.u64(target.instance_path.len() as u64);
    for identity in &target.instance_path {
        hasher.u128(*identity);
    }
    hasher.u16(target.bit_range.offset);
    hasher.u16(target.bit_range.width);
    hasher.u8(target.value_type as u8);
    hasher.value(target.typed_value);
    hasher.u8(target.target_layer as u8);
    hasher.hash(target.entry_hash);
}

fn hash_force_audit_record(record: &ForceAuditRecord) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-FORCE-AUDIT-RECORD-1");
    hasher.u64(record.ordinal);
    hasher.u8(record.action as u8);
    hasher.u8(record.result as u8);
    hasher.u128(record.command_id);
    hasher.u128(record.idempotency_key);
    hasher.u128(record.force_id.0);
    hasher.u128(record.requester_identity);
    hasher.string(&record.reason);
    match &record.before {
        Some(target) => {
            hasher.bool(true);
            hash_force_audit_target(target, &mut hasher);
        }
        None => hasher.bool(false),
    }
    match &record.after {
        Some(target) => {
            hasher.bool(true);
            hash_force_audit_target(target, &mut hasher);
        }
        None => hasher.bool(false),
    }
    hasher.u64(record.old_registry_version);
    hasher.u64(record.new_registry_version);
    hasher.hash(record.old_registry_hash);
    hasher.hash(record.new_registry_hash);
    hasher.u64(record.universe_epoch);
    hasher.u128(record.controller_id);
    hasher.u64(record.controller_epoch);
    hasher.u64(record.session_epoch);
    hasher.hash(record.artifact_fingerprint);
    hasher.u8(record.applied_boundary as u8);
    hasher.u64(record.first_affected_scan_sequence);
    hasher.u64(record.first_affected_event_sequence);
    hasher.u64(record.first_affected_virtual_timestamp_ms);
    hasher.hash(record.audit_context_hash);
    hasher.hash(record.prior_record_hash);
    hasher.finish()
}

fn hash_global_force_projection(projection: &GlobalForceProjection) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-GLOBAL-FORCE-PROJECTION-1");
    hasher.u32(projection.schema_version);
    hasher.u64(projection.count as u64);
    hasher.u64(projection.controller_ids.len() as u64);
    for controller_id in &projection.controller_ids {
        hasher.u128(*controller_id);
    }
    hasher.u64(projection.registry_version);
    hasher.hash(projection.registry_hash);
    hasher.u64(projection.audit_record_count as u64);
    hasher.hash(projection.audit_head_hash);
    hasher.u64(projection.entries.len() as u64);
    for entry in &projection.entries {
        hasher.u64(entry.registry_ordinal);
        hasher.u128(entry.navigation_identity.0);
        hasher.hash(entry.force.entry_hash);
    }
    hasher.finish()
}

fn hash_projected_force_registry(
    registry_version: u64,
    entries: &[GlobalForceProjectionEntry],
) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-FORCE-REGISTRY-1");
    if registry_version == 0 && entries.is_empty() {
        return hasher.finish();
    }
    hasher.u64(registry_version);
    hasher.u64(entries.len() as u64);
    for entry in entries {
        hasher.hash(entry.force.entry_hash);
    }
    hasher.finish()
}

fn validate_force_io_quality(
    target_id: StableTargetId,
    runtime_target: RuntimeTarget,
    io_states: &[RuntimeIoState],
) -> Result<(crate::Quality, bool), ForceError> {
    if matches!(runtime_target, RuntimeTarget::Memory(_)) {
        return Ok((crate::Quality::Good, false));
    }
    let state = io_states
        .iter()
        .find(|state| state.target_id == target_id)
        .ok_or(ForceError::QualityUnavailable(target_id))?;
    if !state.runtime_present || state.quality == crate::Quality::NotPresent {
        return Err(ForceError::IoNotPresent(target_id));
    }
    Ok((state.quality, state.quality == crate::Quality::Bad))
}

fn resolve_force_target(
    catalog: &ProbeCatalog,
    target: &TargetReference,
    context: ObservationContext,
) -> Result<ResolvedTarget, ForceError> {
    let resolved = catalog
        .resolve(
            target,
            ProbeLayer::Effective,
            context.artifact_fingerprint,
            context.profile_fingerprint,
        )
        .map_err(|_| ForceError::TargetUnavailable)?;
    if !catalog
        .definition(resolved.id)
        .is_some_and(|definition| definition.capabilities.force)
    {
        return Err(ForceError::CapabilityDenied);
    }
    Ok(resolved)
}

fn validate_command_context(
    command: &ForceCommand,
    context: ObservationContext,
) -> Result<(), ForceError> {
    if command.expected_universe_epoch != context.universe_epoch {
        return Err(ForceError::StaleUniverseEpoch);
    }
    if command.expected_controller_epoch != context.controller_epoch {
        return Err(ForceError::StaleControllerEpoch);
    }
    if command.expected_session_epoch != context.session_epoch {
        return Err(ForceError::StaleSessionEpoch);
    }
    if command.expected_artifact_fingerprint != context.artifact_fingerprint {
        return Err(ForceError::StaleArtifact);
    }
    if command.expected_target_state_hash != context.target_state_hash {
        return Err(ForceError::StaleTargetState);
    }
    Ok(())
}

fn force_boundary(
    cpu_state: CpuState,
    kind: &ForceCommandKind,
) -> Result<PublicationBoundary, ForceError> {
    let removal = matches!(
        kind,
        ForceCommandKind::Remove { .. } | ForceCommandKind::RemoveAll { .. }
    );
    match cpu_state {
        CpuState::Run => Ok(PublicationBoundary::ScanEnd),
        CpuState::Stop | CpuState::PausedEducational => Ok(PublicationBoundary::SerializedCommand),
        CpuState::Faulted if removal => Ok(PublicationBoundary::SerializedCommand),
        state => Err(ForceError::CpuStateDisallowed(state)),
    }
}

fn force_creation_boundary(cpu_state: CpuState) -> Result<PublicationBoundary, ForceError> {
    match cpu_state {
        CpuState::Run => Ok(PublicationBoundary::ScanEnd),
        CpuState::Stop | CpuState::PausedEducational => Ok(PublicationBoundary::SerializedCommand),
        state => Err(ForceError::CpuStateDisallowed(state)),
    }
}

fn hash_remove_all_preview(preview: &RemoveAllPreview) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-FORCE-REMOVE-ALL-PREVIEW-1");
    hasher.u128(preview.preview_id);
    hasher.u64(preview.controller_epoch);
    hasher.hash(preview.target_state_hash);
    hasher.u64(preview.registry_version);
    hasher.hash(preview.registry_hash);
    hasher.u64(preview.force_ids.len() as u64);
    for id in &preview.force_ids {
        hasher.u128(id.0);
    }
    hasher.finish()
}

fn hash_force_command(command: &ForceCommand) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-FORCE-COMMAND-1");
    hasher.u128(command.command_id);
    hasher.u128(command.idempotency_key);
    hasher.u64(command.expected_universe_epoch);
    hasher.u64(command.expected_controller_epoch);
    hasher.u64(command.expected_session_epoch);
    hasher.hash(command.expected_artifact_fingerprint);
    hasher.hash(command.expected_target_state_hash);
    hasher.u64(command.expected_registry_version);
    hasher.hash(command.expected_registry_hash);
    hasher.hash(command.audit_context_hash);
    match &command.kind {
        ForceCommandKind::Create {
            force_id,
            target,
            value,
            natural_at_application,
            actor_identity,
            reason,
        } => {
            hasher.u8(1);
            hasher.u128(force_id.0);
            encode_target_reference(target, &mut hasher);
            hasher.value(*value);
            hasher.value(*natural_at_application);
            hasher.u128(*actor_identity);
            hasher.string(reason);
        }
        ForceCommandKind::Replace {
            force_id,
            expected_entry_hash,
            value,
            actor_identity,
            reason,
        } => {
            hasher.u8(2);
            hasher.u128(force_id.0);
            hasher.hash(*expected_entry_hash);
            hasher.value(*value);
            hasher.u128(*actor_identity);
            hasher.string(reason);
        }
        ForceCommandKind::Remove {
            force_id,
            expected_entry_hash,
            actor_identity,
            reason,
        } => {
            hasher.u8(3);
            hasher.u128(force_id.0);
            hasher.hash(*expected_entry_hash);
            hasher.u128(*actor_identity);
            hasher.string(reason);
        }
        ForceCommandKind::RemoveAll {
            approval,
            actor_identity,
            reason,
        } => {
            hasher.u8(4);
            hasher.u128(approval.preview_id);
            hasher.hash(approval.preview_hash);
            hasher.u64(approval.controller_epoch);
            hasher.hash(approval.target_state_hash);
            hasher.u64(approval.registry_version);
            hasher.hash(approval.registry_hash);
            hasher.u128(*actor_identity);
            hasher.string(reason);
        }
    }
    hasher.finish()
}

fn encode_target_reference(target: &TargetReference, hasher: &mut CanonicalHasher) {
    match target {
        TargetReference::Stable(id) => {
            hasher.u8(1);
            hasher.u128(id.0);
        }
        TargetReference::SourceOnly(source) => {
            hasher.u8(2);
            crate::target::encode_source_anchor(source, hasher);
        }
    }
}

fn hash_force_plan(plan: &ForceWritePlan) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-FORCE-WRITE-PLAN-1");
    hasher.u128(plan.command_id);
    hasher.u8(plan.boundary as u8);
    hasher.hash(plan.expected_target_state_hash);
    hasher.hash(plan.expected_registry_hash);
    hasher.hash(plan.next_registry_hash);
    hasher.u64(plan.set_values.len() as u64);
    for (target, value) in &plan.set_values {
        encode_runtime_target(*target, &mut hasher);
        hasher.value(*value);
    }
    hasher.u64(plan.remove_targets.len() as u64);
    for target in &plan.remove_targets {
        encode_runtime_target(*target, &mut hasher);
    }
    hasher.u64(plan.resulting_force_ids.len() as u64);
    for id in &plan.resulting_force_ids {
        hasher.u128(id.0);
    }
    hasher.finish()
}

fn hash_force_snapshot(snapshot: &ForceRegistrySnapshot) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-FORCE-SNAPSHOT-1");
    hasher.u32(snapshot.schema_version);
    hasher.u128(snapshot.universe_id);
    hasher.u128(snapshot.controller_id);
    hasher.u64(snapshot.controller_epoch);
    hasher.hash(snapshot.artifact_fingerprint);
    hasher.hash(snapshot.profile_fingerprint);
    hasher.u64(snapshot.registry_version);
    hasher.u64(snapshot.entries.len() as u64);
    for entry in &snapshot.entries {
        hasher.hash(entry.entry_hash);
    }
    hasher.u64(snapshot.audit_records.len() as u64);
    for record in &snapshot.audit_records {
        hasher.hash(record.record_hash);
    }
    hasher.hash(snapshot.audit_head_hash);
    hasher.hash(snapshot.registry_hash);
    hasher.finish()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ForceError {
    IdempotencyCollision,
    StaleUniverseEpoch,
    StaleControllerEpoch,
    StaleSessionEpoch,
    StaleArtifact,
    StaleTargetState,
    RegistryCompareAndSwapFailed,
    EntryCompareAndSwapFailed,
    CpuStateDisallowed(CpuState),
    WrongPublicationBoundary {
        required: PublicationBoundary,
        actual: PublicationBoundary,
    },
    DuplicateForceId(ForceId),
    UnknownForce(ForceId),
    TargetUnavailable,
    CapabilityDenied,
    TypeMismatch,
    QualityUnavailable(StableTargetId),
    IoNotPresent(StableTargetId),
    TargetOverlap,
    ReasonRequired,
    RemoveAllApprovalMismatch,
    SnapshotIntegrityMismatch,
    SnapshotBindingMismatch,
}

impl fmt::Display for ForceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "force registry action rejected: {self:?}")
    }
}

impl Error for ForceError {}
