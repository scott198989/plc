use alloc::{collections::BTreeMap, vec::Vec};
use core::{error::Error, fmt};

use plc_runtime::{CanonicalValue, CpuState, Hash32, ValueType, VirtualControllerId};

use crate::{
    BitRange, ForceRegistry, ObservationContext, ProbeCatalog, ProbeLayer, PublicationBoundary,
    RuntimeIoState, RuntimeTarget, StableTargetId, TargetReference, canonical::CanonicalHasher,
    target::encode_runtime_target,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModifyItem {
    pub target: TargetReference,
    pub expected_instance_path: Vec<u128>,
    pub expected_value_type: ValueType,
    pub value: CanonicalValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModifyCommand {
    pub command_id: u128,
    pub idempotency_key: u128,
    pub session_id: crate::VirtualOnlineSessionId,
    pub controller_id: VirtualControllerId,
    pub expected_universe_epoch: u64,
    pub expected_controller_epoch: u64,
    pub expected_session_epoch: u64,
    pub expected_artifact_fingerprint: Hash32,
    pub expected_target_state_hash: Hash32,
    pub expected_probe_catalog_hash: Hash32,
    pub expected_force_registry_version: u64,
    pub expected_force_registry_hash: Hash32,
    pub allow_overwrite_queued: bool,
    pub requested_boundary: PublicationBoundary,
    pub author_identity: u128,
    pub audit_context_hash: Hash32,
    pub items: Vec<ModifyItem>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ModifyReceiptState {
    Queued = 1,
    Applied = 2,
    Canceled = 3,
    Rejected = 4,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModifyWritePlan {
    pub target_id: StableTargetId,
    pub runtime_target: RuntimeTarget,
    pub instance_path: Vec<u128>,
    pub bit_range: BitRange,
    pub value_type: ValueType,
    pub value: CanonicalValue,
    pub boundary: PublicationBoundary,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicationPlan {
    pub command_id: u128,
    pub idempotency_key: u128,
    pub session_id: crate::VirtualOnlineSessionId,
    pub controller_id: VirtualControllerId,
    pub expected_universe_epoch: u64,
    pub expected_controller_epoch: u64,
    pub expected_session_epoch: u64,
    pub expected_target_state_hash: Hash32,
    pub expected_probe_catalog_hash: Hash32,
    pub expected_force_registry_hash: Hash32,
    pub audit_context_hash: Hash32,
    pub author_identity: u128,
    pub writes: Vec<ModifyWritePlan>,
    pub plan_hash: Hash32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModifyReceipt {
    pub command_id: u128,
    pub idempotency_key: u128,
    pub duplicate: bool,
    pub state: ModifyReceiptState,
    pub scheduled_boundary: PublicationBoundary,
    pub writes: Vec<ModifyWritePlan>,
    pub accepted_state_hash: Hash32,
    pub applied_state_hash: Option<Hash32>,
    pub applied_event_sequence: Option<u64>,
    pub cancellation_code: Option<&'static str>,
    pub plan_hash: Hash32,
}

#[derive(Clone, Debug)]
struct PendingModify {
    context: ObservationContext,
    command: ModifyCommand,
    plan: PublicationPlan,
}

#[derive(Clone, Debug)]
struct StoredModifyReceipt {
    payload_hash: Hash32,
    receipt: ModifyReceipt,
}

#[derive(Clone, Debug, Default)]
pub struct ModifyScheduler {
    pending: BTreeMap<u128, PendingModify>,
    target_owners: BTreeMap<(RuntimeTarget, u16, u16), u128>,
    receipts: BTreeMap<u128, StoredModifyReceipt>,
    command_to_key: BTreeMap<u128, u128>,
}

impl ModifyScheduler {
    pub fn submit(
        &mut self,
        command: ModifyCommand,
        context: ObservationContext,
        catalog: &ProbeCatalog,
        forces: &ForceRegistry,
    ) -> Result<ModifyReceipt, ModifyError> {
        self.submit_with_io_state(command, context, catalog, forces, &[])
    }

    pub fn submit_with_io_state(
        &mut self,
        command: ModifyCommand,
        context: ObservationContext,
        catalog: &ProbeCatalog,
        forces: &ForceRegistry,
        io_states: &[RuntimeIoState],
    ) -> Result<ModifyReceipt, ModifyError> {
        let payload_hash = hash_modify_command(&command);
        if let Some(stored) = self.receipts.get(&command.idempotency_key) {
            if stored.payload_hash != payload_hash {
                return Err(ModifyError::IdempotencyCollision);
            }
            let mut receipt = stored.receipt.clone();
            receipt.duplicate = true;
            return Ok(receipt);
        }
        validate_context(&command, context, catalog, forces)?;
        if command.items.is_empty() {
            return Err(ModifyError::EmptyAggregate);
        }
        let scheduled_boundary = modify_boundary(context.cpu_state)?;
        let mut writes: Vec<ModifyWritePlan> = Vec::with_capacity(command.items.len());
        for item in &command.items {
            let resolved = catalog
                .resolve(
                    &item.target,
                    ProbeLayer::Natural,
                    context.artifact_fingerprint,
                    context.profile_fingerprint,
                )
                .map_err(|_| ModifyError::TargetUnavailable)?;
            let definition = catalog
                .definition(resolved.id)
                .ok_or(ModifyError::TargetUnavailable)?;
            if !definition.capabilities.modify {
                return Err(ModifyError::CapabilityDenied(resolved.id));
            }
            if item.value.value_type() != resolved.value_type {
                return Err(ModifyError::TypeMismatch(resolved.id));
            }
            if item.expected_value_type != resolved.value_type {
                return Err(ModifyError::TypeMismatch(resolved.id));
            }
            if item.expected_instance_path != resolved.instance_path {
                return Err(ModifyError::InstancePathMismatch(resolved.id));
            }
            validate_modify_io_quality(resolved.id, resolved.runtime_target, io_states)?;
            if writes.iter().any(|write| {
                write.runtime_target == resolved.runtime_target
                    && write.bit_range.overlaps(resolved.bit_range)
            }) {
                return Err(ModifyError::DuplicateTarget(resolved.id));
            }
            if forces.overlaps(resolved.runtime_target, resolved.bit_range) {
                return Err(ModifyError::ForceConflict(resolved.id));
            }
            writes.push(ModifyWritePlan {
                target_id: resolved.id,
                runtime_target: resolved.runtime_target,
                instance_path: resolved.instance_path,
                bit_range: resolved.bit_range,
                value_type: resolved.value_type,
                value: item.value,
                boundary: scheduled_boundary,
            });
        }
        writes.sort_by_key(|write| {
            (
                write.runtime_target,
                write.bit_range.offset,
                write.bit_range.width,
                write.target_id,
            )
        });
        if context.cpu_state == CpuState::Run {
            let has_output = writes
                .iter()
                .any(|write| matches!(write.runtime_target, RuntimeTarget::Output(_)));
            let has_pre_program = writes
                .iter()
                .any(|write| !matches!(write.runtime_target, RuntimeTarget::Output(_)));
            if has_output && has_pre_program {
                return Err(ModifyError::MixedApplicationStage);
            }
        }

        let overwritten = writes
            .iter()
            .flat_map(|write| {
                self.target_owners
                    .iter()
                    .filter(move |((target, offset, width), _)| {
                        *target == write.runtime_target
                            && BitRange {
                                offset: *offset,
                                width: *width,
                            }
                            .overlaps(write.bit_range)
                    })
                    .map(|(_, owner)| *owner)
            })
            .collect::<alloc::collections::BTreeSet<_>>();
        if !overwritten.is_empty() && !command.allow_overwrite_queued {
            return Err(ModifyError::QueuedWriteConflict);
        }
        for command_id in overwritten {
            self.cancel_internal(command_id, "OVERWRITTEN_BY_NEWER_MODIFY");
        }

        let mut plan = PublicationPlan {
            command_id: command.command_id,
            idempotency_key: command.idempotency_key,
            session_id: command.session_id,
            controller_id: command.controller_id,
            expected_universe_epoch: command.expected_universe_epoch,
            expected_controller_epoch: command.expected_controller_epoch,
            expected_session_epoch: command.expected_session_epoch,
            expected_target_state_hash: context.target_state_hash,
            expected_probe_catalog_hash: catalog.catalog_hash(),
            expected_force_registry_hash: forces.registry_hash(),
            audit_context_hash: command.audit_context_hash,
            author_identity: command.author_identity,
            writes: writes.clone(),
            plan_hash: Hash32::ZERO,
        };
        plan.plan_hash = hash_publication_plan(&plan);
        for write in &writes {
            self.target_owners.insert(
                (
                    write.runtime_target,
                    write.bit_range.offset,
                    write.bit_range.width,
                ),
                command.command_id,
            );
        }
        let receipt = ModifyReceipt {
            command_id: command.command_id,
            idempotency_key: command.idempotency_key,
            duplicate: false,
            state: ModifyReceiptState::Queued,
            scheduled_boundary,
            writes,
            accepted_state_hash: context.target_state_hash,
            applied_state_hash: None,
            applied_event_sequence: None,
            cancellation_code: None,
            plan_hash: plan.plan_hash,
        };
        self.pending.insert(
            command.command_id,
            PendingModify {
                context,
                command: command.clone(),
                plan,
            },
        );
        self.command_to_key
            .insert(command.command_id, command.idempotency_key);
        self.receipts.insert(
            command.idempotency_key,
            StoredModifyReceipt {
                payload_hash,
                receipt: receipt.clone(),
            },
        );
        Ok(receipt)
    }

    pub fn next_due(
        &mut self,
        context: ObservationContext,
        catalog: &ProbeCatalog,
        forces: &ForceRegistry,
    ) -> Result<Option<PublicationPlan>, ModifyError> {
        self.cancel_stale(context, catalog, forces);
        let Some(command_id) = self.pending.keys().next().copied() else {
            return Ok(None);
        };
        let pending = self
            .pending
            .get(&command_id)
            .expect("selected pending modify command must remain present");
        if pending
            .plan
            .writes
            .iter()
            .any(|write| write.boundary != context.publication_boundary)
        {
            return Ok(None);
        }
        Ok(Some(pending.plan.clone()))
    }

    pub fn commit(
        &mut self,
        plan: &PublicationPlan,
        resulting_state_hash: Hash32,
        applied_event_sequence: u64,
    ) -> Result<ModifyReceipt, ModifyError> {
        self.validate_commit_plan(plan)?;
        let pending = self
            .pending
            .get(&plan.command_id)
            .ok_or(ModifyError::UnknownPendingCommand(plan.command_id))?;
        let key = pending.command.idempotency_key;
        let writes = pending.plan.writes.clone();
        self.remove_pending_indexes(plan.command_id, &writes);
        let stored = self
            .receipts
            .get_mut(&key)
            .expect("accepted modify command must own a receipt");
        stored.receipt.state = ModifyReceiptState::Applied;
        stored.receipt.applied_state_hash = Some(resulting_state_hash);
        stored.receipt.applied_event_sequence = Some(applied_event_sequence);
        Ok(stored.receipt.clone())
    }

    pub fn cancel(
        &mut self,
        command_id: u128,
        context: ObservationContext,
    ) -> Result<ModifyReceipt, ModifyError> {
        let pending = self
            .pending
            .get(&command_id)
            .ok_or(ModifyError::UnknownPendingCommand(command_id))?;
        if !pending.context.same_runtime_epoch(context) {
            return Err(ModifyError::StaleCancellation);
        }
        self.cancel_internal(command_id, "USER_CANCELED")
            .ok_or(ModifyError::UnknownPendingCommand(command_id))
    }

    pub fn receipt_by_idempotency_key(&self, key: u128) -> Option<&ModifyReceipt> {
        self.receipts.get(&key).map(|stored| &stored.receipt)
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn overlaps_pending(&self, target: RuntimeTarget, bit_range: BitRange) -> bool {
        self.target_owners
            .keys()
            .any(|(pending_target, offset, width)| {
                *pending_target == target
                    && BitRange {
                        offset: *offset,
                        width: *width,
                    }
                    .overlaps(bit_range)
            })
    }

    pub(crate) fn validate_commit_plan(&self, plan: &PublicationPlan) -> Result<(), ModifyError> {
        if plan.plan_hash != hash_publication_plan(plan) {
            return Err(ModifyError::PlanIntegrityMismatch);
        }
        let pending = self
            .pending
            .get(&plan.command_id)
            .ok_or(ModifyError::UnknownPendingCommand(plan.command_id))?;
        if pending.plan != *plan {
            return Err(ModifyError::PlanChanged);
        }
        Ok(())
    }

    pub(crate) fn cancel_publication(
        &mut self,
        plan: &PublicationPlan,
        cancellation_code: &'static str,
    ) -> Result<ModifyReceipt, ModifyError> {
        self.validate_commit_plan(plan)?;
        self.cancel_internal(plan.command_id, cancellation_code)
            .ok_or(ModifyError::UnknownPendingCommand(plan.command_id))
    }

    fn cancel_stale(
        &mut self,
        context: ObservationContext,
        catalog: &ProbeCatalog,
        forces: &ForceRegistry,
    ) {
        let stale =
            self.pending
                .iter()
                .filter_map(|(id, pending)| {
                    let stale_epoch = !pending.context.same_runtime_epoch(context);
                    let stale_mapping =
                        pending.plan.expected_probe_catalog_hash != catalog.catalog_hash();
                    let stale_force =
                        pending.plan.expected_force_registry_hash != forces.registry_hash()
                            || pending.plan.writes.iter().any(|write| {
                                forces.overlaps(write.runtime_target, write.bit_range)
                            });
                    let illegal_cpu = modify_boundary(context.cpu_state).is_err();
                    (stale_epoch || stale_mapping || stale_force || illegal_cpu).then_some(*id)
                })
                .collect::<Vec<_>>();
        for id in stale {
            self.cancel_internal(id, "TARGET_CONTEXT_CHANGED");
        }
    }

    fn cancel_internal(
        &mut self,
        command_id: u128,
        cancellation_code: &'static str,
    ) -> Option<ModifyReceipt> {
        let pending = self.pending.remove(&command_id)?;
        self.remove_pending_indexes(command_id, &pending.plan.writes);
        let key = self.command_to_key.get(&command_id).copied()?;
        let stored = self.receipts.get_mut(&key)?;
        stored.receipt.state = ModifyReceiptState::Canceled;
        stored.receipt.cancellation_code = Some(cancellation_code);
        Some(stored.receipt.clone())
    }

    fn remove_pending_indexes(&mut self, command_id: u128, writes: &[ModifyWritePlan]) {
        self.pending.remove(&command_id);
        for write in writes {
            let key = (
                write.runtime_target,
                write.bit_range.offset,
                write.bit_range.width,
            );
            if self.target_owners.get(&key) == Some(&command_id) {
                self.target_owners.remove(&key);
            }
        }
    }
}

fn validate_modify_io_quality(
    target_id: StableTargetId,
    runtime_target: RuntimeTarget,
    io_states: &[RuntimeIoState],
) -> Result<(), ModifyError> {
    if matches!(runtime_target, RuntimeTarget::Memory(_)) {
        return Ok(());
    }
    let state = io_states
        .iter()
        .find(|state| state.target_id == target_id)
        .ok_or(ModifyError::QualityUnavailable(target_id))?;
    if !state.runtime_present
        || matches!(
            state.quality,
            crate::Quality::Bad | crate::Quality::NotPresent
        )
    {
        return Err(ModifyError::IoQualityRejected {
            target_id,
            quality: state.quality,
        });
    }
    Ok(())
}

fn validate_context(
    command: &ModifyCommand,
    context: ObservationContext,
    catalog: &ProbeCatalog,
    forces: &ForceRegistry,
) -> Result<(), ModifyError> {
    if command.expected_universe_epoch != context.universe_epoch {
        return Err(ModifyError::StaleUniverseEpoch);
    }
    if command.session_id != context.session_id {
        return Err(ModifyError::WrongSession);
    }
    if command.controller_id != context.controller_id {
        return Err(ModifyError::WrongController);
    }
    if command.expected_controller_epoch != context.controller_epoch {
        return Err(ModifyError::StaleControllerEpoch);
    }
    if command.expected_session_epoch != context.session_epoch {
        return Err(ModifyError::StaleSessionEpoch);
    }
    if command.expected_artifact_fingerprint != context.artifact_fingerprint {
        return Err(ModifyError::StaleArtifact);
    }
    if command.expected_target_state_hash != context.target_state_hash {
        return Err(ModifyError::StaleTargetState);
    }
    if command.expected_probe_catalog_hash != catalog.catalog_hash() {
        return Err(ModifyError::StaleProbeCatalog);
    }
    if command.expected_force_registry_version != forces.version()
        || command.expected_force_registry_hash != forces.registry_hash()
    {
        return Err(ModifyError::StaleForceRegistry);
    }
    let required_boundary = modify_boundary(context.cpu_state)?;
    if command.requested_boundary != required_boundary {
        return Err(ModifyError::WrongPublicationBoundary {
            required: required_boundary,
            actual: command.requested_boundary,
        });
    }
    Ok(())
}

fn modify_boundary(cpu_state: CpuState) -> Result<PublicationBoundary, ModifyError> {
    match cpu_state {
        CpuState::Run => Ok(PublicationBoundary::ScanEnd),
        CpuState::Stop | CpuState::PausedEducational => Ok(PublicationBoundary::SerializedCommand),
        state => Err(ModifyError::CpuStateDisallowed(state)),
    }
}

fn hash_modify_command(command: &ModifyCommand) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-MODIFY-COMMAND-1");
    hasher.u128(command.command_id);
    hasher.u128(command.idempotency_key);
    hasher.u128(command.session_id.0);
    hasher.u128(command.controller_id.0);
    hasher.u64(command.expected_universe_epoch);
    hasher.u64(command.expected_controller_epoch);
    hasher.u64(command.expected_session_epoch);
    hasher.hash(command.expected_artifact_fingerprint);
    hasher.hash(command.expected_target_state_hash);
    hasher.hash(command.expected_probe_catalog_hash);
    hasher.u64(command.expected_force_registry_version);
    hasher.hash(command.expected_force_registry_hash);
    hasher.bool(command.allow_overwrite_queued);
    hasher.u8(command.requested_boundary as u8);
    hasher.u128(command.author_identity);
    hasher.hash(command.audit_context_hash);
    hasher.u64(command.items.len() as u64);
    for item in &command.items {
        match &item.target {
            TargetReference::Stable(id) => {
                hasher.u8(1);
                hasher.u128(id.0);
            }
            TargetReference::SourceOnly(source) => {
                hasher.u8(2);
                crate::target::encode_source_anchor(source, &mut hasher);
            }
        }
        hasher.u64(item.expected_instance_path.len() as u64);
        for identity in &item.expected_instance_path {
            hasher.u128(*identity);
        }
        hasher.u8(item.expected_value_type as u8);
        hasher.value(item.value);
    }
    hasher.finish()
}

fn hash_publication_plan(plan: &PublicationPlan) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-MODIFY-PUBLICATION-PLAN-1");
    hasher.u128(plan.command_id);
    hasher.u128(plan.idempotency_key);
    hasher.u128(plan.session_id.0);
    hasher.u128(plan.controller_id.0);
    hasher.u64(plan.expected_universe_epoch);
    hasher.u64(plan.expected_controller_epoch);
    hasher.u64(plan.expected_session_epoch);
    hasher.hash(plan.expected_target_state_hash);
    hasher.hash(plan.expected_probe_catalog_hash);
    hasher.hash(plan.expected_force_registry_hash);
    hasher.hash(plan.audit_context_hash);
    hasher.u128(plan.author_identity);
    hasher.u64(plan.writes.len() as u64);
    for write in &plan.writes {
        hasher.u128(write.target_id.0);
        encode_runtime_target(write.runtime_target, &mut hasher);
        hasher.u64(write.instance_path.len() as u64);
        for identity in &write.instance_path {
            hasher.u128(*identity);
        }
        hasher.u16(write.bit_range.offset);
        hasher.u16(write.bit_range.width);
        hasher.u8(write.value_type as u8);
        hasher.value(write.value);
        hasher.u8(write.boundary as u8);
    }
    hasher.finish()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModifyError {
    IdempotencyCollision,
    EmptyAggregate,
    StaleUniverseEpoch,
    StaleControllerEpoch,
    StaleSessionEpoch,
    StaleArtifact,
    StaleTargetState,
    StaleProbeCatalog,
    StaleForceRegistry,
    WrongSession,
    WrongController,
    StaleCancellation,
    CpuStateDisallowed(CpuState),
    TargetUnavailable,
    CapabilityDenied(StableTargetId),
    TypeMismatch(StableTargetId),
    InstancePathMismatch(StableTargetId),
    DuplicateTarget(StableTargetId),
    ForceConflict(StableTargetId),
    QualityUnavailable(StableTargetId),
    IoQualityRejected {
        target_id: StableTargetId,
        quality: crate::Quality,
    },
    WrongPublicationBoundary {
        required: PublicationBoundary,
        actual: PublicationBoundary,
    },
    QueuedWriteConflict,
    MixedApplicationStage,
    UnknownPendingCommand(u128),
    PlanIntegrityMismatch,
    PlanChanged,
}

impl fmt::Display for ModifyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "modify action rejected: {self:?}")
    }
}

impl Error for ModifyError {}
