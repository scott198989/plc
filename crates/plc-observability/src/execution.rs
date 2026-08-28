use alloc::{boxed::Box, vec::Vec};
use core::{error::Error, fmt};

use plc_commissioning::{
    CommissionedBoundaryReceipt, CommissionedScanReceipt, CommissioningError,
    ForceRegistryProjection, SessionCommandBinding, VirtualUniverse,
};
use plc_runtime::{
    CanonicalValue, CpuState, DiagnosticEvent as RuntimeDiagnosticEvent, Hash32,
    RuntimeBoundaryCommand, RuntimeForceDelta, RuntimeNaturalWrite, RuntimeScanCommand,
    RuntimeValueTarget, canonical_force_overlay_hash,
};

use crate::{
    BitRange, ContextError, ForceCommand, ForceError, ForceReceipt, ForceRegistry, ModifyError,
    ModifyReceipt, ModifyScheduler, ObservationContext, ProbeCatalog, PublicationBoundary,
    PublicationPlan, RuntimeIoState, RuntimeTarget,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimePublicationReceipt {
    Serialized(Box<CommissionedBoundaryReceipt>),
    Scan(Box<CommissionedScanReceipt>),
}

impl RuntimePublicationReceipt {
    pub fn controller_state_hash(&self) -> Hash32 {
        match self {
            Self::Serialized(receipt) => receipt.controller_state_hash,
            Self::Scan(receipt) => receipt.controller_state_hash,
        }
    }

    pub fn event_sequence(&self) -> u64 {
        match self {
            Self::Serialized(receipt) => receipt.runtime.event_sequence,
            Self::Scan(receipt) => match &receipt.runtime.outcome {
                plc_runtime::RunOutcome::Completed(report) => report.output_event_sequence,
                plc_runtime::RunOutcome::Faulted(RuntimeDiagnosticEvent {
                    event_sequence, ..
                }) => *event_sequence,
            },
        }
    }

    pub fn applied_write_count(&self) -> usize {
        match self {
            Self::Serialized(receipt) => receipt.runtime.writes.len(),
            Self::Scan(receipt) => {
                receipt.runtime.applied_pre_program_writes.len()
                    + receipt.runtime.applied_post_program_writes.len()
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModifyExecutionReceipt {
    pub modify: ModifyReceipt,
    pub publication: RuntimePublicationReceipt,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForceExecutionReceipt {
    pub force: ForceReceipt,
    pub publication: Option<RuntimePublicationReceipt>,
}

pub fn publish_modify_plan(
    universe: &mut VirtualUniverse,
    binding: SessionCommandBinding,
    scheduler: &mut ModifyScheduler,
    forces: &ForceRegistry,
    plan: &PublicationPlan,
) -> Result<ModifyExecutionReceipt, ExecutionError> {
    scheduler.validate_commit_plan(plan)?;
    if plan.session_id != binding.session_id
        || plan.expected_universe_epoch != binding.expected_universe_epoch
        || plan.expected_controller_epoch != binding.expected_controller_epoch
        || plan.expected_session_epoch != binding.expected_session_epoch
        || plan.expected_target_state_hash != binding.expected_target_state_hash
        || plan.expected_force_registry_hash != forces.registry_hash()
    {
        return Err(ExecutionError::StalePublicationPlan);
    }
    validate_whole_value_writes(plan)?;

    let mut universe_candidate = universe.clone();
    let instance = controller_for_binding(&universe_candidate, binding)?;
    if plan.controller_id != instance.runtime().controller_id() {
        return Err(ExecutionError::StalePublicationPlan);
    }
    let runtime = instance.runtime();
    let projection = projection_for_registry(instance.force_registry_hash(), forces)?;
    let publication = match plan_boundary(plan)? {
        PublicationBoundary::SerializedCommand => {
            let command = RuntimeBoundaryCommand {
                command_id: plan.command_id,
                controller_id: runtime.controller_id(),
                expected_controller_epoch: runtime.controller_epoch(),
                expected_artifact_fingerprint: runtime
                    .loaded_fingerprint()
                    .ok_or(ExecutionError::NoLoadedArtifact)?,
                expected_state_hash: runtime.semantic_state_hash(),
                natural_writes: plan
                    .writes
                    .iter()
                    .map(|write| RuntimeNaturalWrite {
                        target: to_runtime_target(write.runtime_target),
                        value: write.value,
                    })
                    .collect(),
                force_deltas: Vec::new(),
                audit_context_hash: plan.audit_context_hash,
            };
            RuntimePublicationReceipt::Serialized(Box::new(
                universe_candidate.apply_observation_boundary(binding, &command, &projection)?,
            ))
        }
        PublicationBoundary::ScanEnd => {
            let mut pre_program_writes = Vec::new();
            let mut post_program_writes = Vec::new();
            for write in &plan.writes {
                let write = RuntimeNaturalWrite {
                    target: to_runtime_target(write.runtime_target),
                    value: write.value,
                };
                if matches!(write.target, RuntimeValueTarget::Output(_)) {
                    post_program_writes.push(write);
                } else {
                    pre_program_writes.push(write);
                }
            }
            let command = RuntimeScanCommand {
                command_id: plan.command_id,
                controller_id: runtime.controller_id(),
                expected_controller_epoch: runtime.controller_epoch(),
                expected_artifact_fingerprint: runtime
                    .loaded_fingerprint()
                    .ok_or(ExecutionError::NoLoadedArtifact)?,
                expected_state_hash: runtime.semantic_state_hash(),
                pre_program_writes,
                post_program_writes,
                force_deltas: Vec::new(),
                audit_context_hash: plan.audit_context_hash,
            };
            RuntimePublicationReceipt::Scan(Box::new(
                universe_candidate.run_scan_with_observation(binding, &command, &projection)?,
            ))
        }
        boundary => return Err(ExecutionError::UnsupportedPublicationBoundary(boundary)),
    };

    let mut scheduler_candidate = scheduler.clone();
    let modify = if publication.applied_write_count() == plan.writes.len() {
        scheduler_candidate.commit(
            plan,
            publication.controller_state_hash(),
            publication.event_sequence(),
        )?
    } else {
        scheduler_candidate.cancel_publication(plan, "SCAN_FAULT_BEFORE_MODIFY_BOUNDARY")?
    };
    *universe = universe_candidate;
    *scheduler = scheduler_candidate;
    Ok(ModifyExecutionReceipt {
        modify,
        publication,
    })
}

pub fn execute_force_command(
    universe: &mut VirtualUniverse,
    binding: SessionCommandBinding,
    registry: &mut ForceRegistry,
    pending_modifies: &ModifyScheduler,
    catalog: &ProbeCatalog,
    command: &ForceCommand,
) -> Result<ForceExecutionReceipt, ExecutionError> {
    execute_force_command_with_io_state(
        universe,
        binding,
        registry,
        pending_modifies,
        catalog,
        command,
        &[],
    )
}

pub fn execute_force_command_with_io_state(
    universe: &mut VirtualUniverse,
    binding: SessionCommandBinding,
    registry: &mut ForceRegistry,
    pending_modifies: &ModifyScheduler,
    catalog: &ProbeCatalog,
    command: &ForceCommand,
    io_states: &[RuntimeIoState],
) -> Result<ForceExecutionReceipt, ExecutionError> {
    let instance = controller_for_binding(universe, binding)?;
    let runtime = instance.runtime();
    let publication_boundary = match runtime.cpu_state() {
        CpuState::Run => PublicationBoundary::ScanEnd,
        CpuState::Stop | CpuState::PausedEducational | CpuState::Faulted => {
            PublicationBoundary::SerializedCommand
        }
        state => return Err(ExecutionError::CpuStateDisallowed(state)),
    };
    let context =
        ObservationContext::from_virtual_universe(universe, binding, publication_boundary)?;
    let mut registry_candidate = registry.clone();
    let (force, plan) =
        registry_candidate.apply_at_boundary_with_io_state(command, context, catalog, io_states)?;
    if force.duplicate {
        return Ok(ForceExecutionReceipt {
            force,
            publication: None,
        });
    }
    for entry in registry_candidate.entries() {
        if plan
            .set_values
            .iter()
            .any(|(target, _)| *target == entry.runtime_target)
            && pending_modifies.overlaps_pending(entry.runtime_target, entry.bit_range)
        {
            return Err(ExecutionError::PendingModifyConflict(entry.runtime_target));
        }
    }

    let force_deltas = force_deltas(&plan);
    let expected_overlay_hash = registry_overlay_hash(&registry_candidate)?;
    let projection = ForceRegistryProjection::new(
        plan.expected_registry_hash,
        plan.next_registry_hash,
        plan.resulting_force_ids.clone(),
        expected_overlay_hash,
    )?;
    let mut universe_candidate = universe.clone();
    let runtime = controller_for_binding(&universe_candidate, binding)?.runtime();
    let publication = match plan.boundary {
        PublicationBoundary::SerializedCommand => {
            let runtime_command = RuntimeBoundaryCommand {
                command_id: plan.command_id,
                controller_id: runtime.controller_id(),
                expected_controller_epoch: runtime.controller_epoch(),
                expected_artifact_fingerprint: runtime
                    .loaded_fingerprint()
                    .ok_or(ExecutionError::NoLoadedArtifact)?,
                expected_state_hash: runtime.semantic_state_hash(),
                natural_writes: Vec::new(),
                force_deltas,
                audit_context_hash: command.audit_context_hash,
            };
            RuntimePublicationReceipt::Serialized(Box::new(
                universe_candidate.apply_observation_boundary(
                    binding,
                    &runtime_command,
                    &projection,
                )?,
            ))
        }
        PublicationBoundary::ScanEnd => {
            let runtime_command = RuntimeScanCommand {
                command_id: plan.command_id,
                controller_id: runtime.controller_id(),
                expected_controller_epoch: runtime.controller_epoch(),
                expected_artifact_fingerprint: runtime
                    .loaded_fingerprint()
                    .ok_or(ExecutionError::NoLoadedArtifact)?,
                expected_state_hash: runtime.semantic_state_hash(),
                pre_program_writes: Vec::new(),
                post_program_writes: Vec::new(),
                force_deltas,
                audit_context_hash: command.audit_context_hash,
            };
            RuntimePublicationReceipt::Scan(Box::new(
                universe_candidate.run_scan_with_observation(
                    binding,
                    &runtime_command,
                    &projection,
                )?,
            ))
        }
        boundary => return Err(ExecutionError::UnsupportedPublicationBoundary(boundary)),
    };

    *universe = universe_candidate;
    *registry = registry_candidate;
    Ok(ForceExecutionReceipt {
        force,
        publication: Some(publication),
    })
}

fn controller_for_binding(
    universe: &VirtualUniverse,
    binding: SessionCommandBinding,
) -> Result<&plc_commissioning::ControllerInstance, ExecutionError> {
    let session = universe
        .session(binding.session_id)
        .ok_or(ExecutionError::TargetUnavailable)?;
    universe
        .controller(session.controller_id())
        .ok_or(ExecutionError::TargetUnavailable)
}

fn plan_boundary(plan: &PublicationPlan) -> Result<PublicationBoundary, ExecutionError> {
    let Some(first) = plan.writes.first() else {
        return Err(ExecutionError::EmptyPublicationPlan);
    };
    if plan
        .writes
        .iter()
        .any(|write| write.boundary != first.boundary)
    {
        return Err(ExecutionError::MixedPublicationBoundaries);
    }
    Ok(first.boundary)
}

fn validate_whole_value_writes(plan: &PublicationPlan) -> Result<(), ExecutionError> {
    if let Some(write) = plan
        .writes
        .iter()
        .find(|write| write.bit_range != BitRange::whole_value())
    {
        return Err(ExecutionError::PartialValueUnsupported(
            write.runtime_target,
        ));
    }
    Ok(())
}

fn force_deltas(plan: &crate::ForceWritePlan) -> Vec<RuntimeForceDelta> {
    let mut values = plan
        .set_values
        .iter()
        .map(|(target, value)| RuntimeForceDelta {
            target: to_runtime_target(*target),
            value: Some(*value),
        })
        .collect::<Vec<_>>();
    values.extend(plan.remove_targets.iter().map(|target| RuntimeForceDelta {
        target: to_runtime_target(*target),
        value: None,
    }));
    values
}

fn projection_for_registry(
    expected_commissioning_hash: Hash32,
    registry: &ForceRegistry,
) -> Result<ForceRegistryProjection, ExecutionError> {
    ForceRegistryProjection::new(
        expected_commissioning_hash,
        registry.registry_hash(),
        registry.active_ids(),
        registry_overlay_hash(registry)?,
    )
    .map_err(Into::into)
}

fn registry_overlay_hash(registry: &ForceRegistry) -> Result<Hash32, ExecutionError> {
    let values = registry
        .entries()
        .map(|entry| (to_runtime_target(entry.runtime_target), entry.value))
        .collect::<Vec<(RuntimeValueTarget, CanonicalValue)>>();
    canonical_force_overlay_hash(&values).map_err(|_| ExecutionError::ForceProjectionNotCanonical)
}

fn to_runtime_target(target: RuntimeTarget) -> RuntimeValueTarget {
    match target {
        RuntimeTarget::Memory(id) => RuntimeValueTarget::Memory(id),
        RuntimeTarget::Input(id) => RuntimeValueTarget::Input(id),
        RuntimeTarget::Output(id) => RuntimeValueTarget::Output(id),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutionError {
    Commissioning(CommissioningError),
    Context(ContextError),
    Modify(ModifyError),
    Force(ForceError),
    StalePublicationPlan,
    TargetUnavailable,
    NoLoadedArtifact,
    EmptyPublicationPlan,
    MixedPublicationBoundaries,
    UnsupportedPublicationBoundary(PublicationBoundary),
    PartialValueUnsupported(RuntimeTarget),
    ForceProjectionNotCanonical,
    PendingModifyConflict(RuntimeTarget),
    CpuStateDisallowed(CpuState),
}

impl From<CommissioningError> for ExecutionError {
    fn from(value: CommissioningError) -> Self {
        Self::Commissioning(value)
    }
}

impl From<ContextError> for ExecutionError {
    fn from(value: ContextError) -> Self {
        Self::Context(value)
    }
}

impl From<ModifyError> for ExecutionError {
    fn from(value: ModifyError) -> Self {
        Self::Modify(value)
    }
}

impl From<ForceError> for ExecutionError {
    fn from(value: ForceError) -> Self {
        Self::Force(value)
    }
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "observation publication rejected: {self:?}")
    }
}

impl Error for ExecutionError {}
