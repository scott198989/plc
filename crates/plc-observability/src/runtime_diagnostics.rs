use alloc::{collections::BTreeMap, vec, vec::Vec};
use core::{error::Error, fmt};

use plc_commissioning::{SessionCommandBinding, VirtualUniverse};
use plc_runtime::{
    DiagnosticCode as RuntimeDiagnosticCode, DiagnosticEvent as RuntimeDiagnosticEvent,
    DiagnosticSeverity as RuntimeDiagnosticSeverity, Hash32,
};

use crate::{
    CausalReference, ConditionKey, ContextError, DiagnosticError, DiagnosticEvent,
    DiagnosticLedger, DiagnosticLifecycle, DiagnosticSeverity, DiagnosticTransition,
    ObservationContext, PublicationBoundary, canonical::CanonicalHasher,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeDiagnosticProviderKey {
    pub universe_epoch: u64,
    pub controller_epoch: u64,
    pub occurrence_id: u128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeDiagnosticReceipt {
    pub provider_key: RuntimeDiagnosticProviderKey,
    pub provider_code: RuntimeDiagnosticCode,
    pub provider_event_sequence: u64,
    pub provider_virtual_timestamp_ms: u64,
    pub provider_payload_hash: Hash32,
    pub ledger_occurrence_id: crate::OccurrenceId,
    pub ledger_event_hash: Hash32,
    pub duplicate: bool,
    pub receipt_hash: Hash32,
}

impl RuntimeDiagnosticReceipt {
    pub fn verify(&self) -> bool {
        self.receipt_hash == hash_runtime_receipt(self)
    }
}

#[derive(Clone, Debug)]
pub struct RuntimeDiagnosticBridge {
    universe_id: Option<u128>,
    controller_id: Option<u128>,
    receipts: BTreeMap<RuntimeDiagnosticProviderKey, RuntimeDiagnosticReceipt>,
    bridge_hash: Hash32,
}

impl Default for RuntimeDiagnosticBridge {
    fn default() -> Self {
        let mut value = Self {
            universe_id: None,
            controller_id: None,
            receipts: BTreeMap::new(),
            bridge_hash: Hash32::ZERO,
        };
        value.bridge_hash = value.calculate_hash();
        value
    }
}

impl RuntimeDiagnosticBridge {
    pub const fn bridge_hash(&self) -> Hash32 {
        self.bridge_hash
    }

    pub fn receipts(&self) -> impl ExactSizeIterator<Item = &RuntimeDiagnosticReceipt> {
        self.receipts.values()
    }

    pub fn replay_hash(&self) -> Result<Hash32, RuntimeDiagnosticBridgeError> {
        for receipt in self.receipts.values() {
            if !receipt.verify() {
                return Err(RuntimeDiagnosticBridgeError::ReceiptIntegrityMismatch(
                    receipt.provider_key,
                ));
            }
        }
        let calculated = self.calculate_hash();
        if calculated != self.bridge_hash {
            return Err(RuntimeDiagnosticBridgeError::BridgeIntegrityMismatch);
        }
        Ok(calculated)
    }

    pub fn ingest_from_virtual_universe(
        &mut self,
        ledger: &mut DiagnosticLedger,
        universe: &VirtualUniverse,
        binding: SessionCommandBinding,
    ) -> Result<Vec<RuntimeDiagnosticReceipt>, RuntimeDiagnosticBridgeError> {
        let base_context = ObservationContext::from_virtual_universe(
            universe,
            binding,
            PublicationBoundary::SerializedCommand,
        )?;
        let instance = universe
            .controller(base_context.controller_id)
            .ok_or(RuntimeDiagnosticBridgeError::TargetUnavailable)?;
        let runtime = instance.runtime();
        let provider_events = runtime.diagnostics().to_vec();

        let mut candidate = self.clone();
        let mut ledger_candidate = ledger.clone();
        candidate.bind(base_context)?;
        candidate.validate_provider_stream(&provider_events, base_context)?;

        let mut output = Vec::new();
        for provider_event in &provider_events {
            let key = provider_key(provider_event);
            let payload_hash = hash_runtime_provider_event(provider_event);
            if provider_event.universe_epoch != base_context.universe_epoch
                || provider_event.controller_epoch != base_context.controller_epoch
            {
                match candidate.receipts.get(&key) {
                    Some(stored) if stored.provider_payload_hash == payload_hash => continue,
                    Some(_) => {
                        return Err(RuntimeDiagnosticBridgeError::ProviderOccurrenceCollision(
                            key,
                        ));
                    }
                    None => {
                        return Err(
                            RuntimeDiagnosticBridgeError::HistoricalProviderEventRequiresBridgeState(
                                key,
                            ),
                        );
                    }
                }
            }
            if let Some(stored) = candidate.receipts.get(&key) {
                if stored.provider_payload_hash != payload_hash {
                    return Err(RuntimeDiagnosticBridgeError::ProviderOccurrenceCollision(
                        key,
                    ));
                }
                let mut duplicate = stored.clone();
                duplicate.duplicate = true;
                output.push(duplicate);
                continue;
            }

            let event = candidate.publish_provider_event(
                &mut ledger_candidate,
                provider_event,
                payload_hash,
                base_context,
            )?;
            let mut receipt = RuntimeDiagnosticReceipt {
                provider_key: key,
                provider_code: provider_event.code,
                provider_event_sequence: provider_event.event_sequence,
                provider_virtual_timestamp_ms: provider_event.virtual_timestamp_ms,
                provider_payload_hash: payload_hash,
                ledger_occurrence_id: event.occurrence_id,
                ledger_event_hash: event.event_hash,
                duplicate: false,
                receipt_hash: Hash32::ZERO,
            };
            receipt.receipt_hash = hash_runtime_receipt(&receipt);
            candidate.receipts.insert(key, receipt.clone());
            output.push(receipt);
        }

        candidate.bridge_hash = candidate.calculate_hash();
        *ledger = ledger_candidate;
        *self = candidate;
        Ok(output)
    }

    fn bind(&mut self, context: ObservationContext) -> Result<(), RuntimeDiagnosticBridgeError> {
        match (self.universe_id, self.controller_id) {
            (None, None) => {
                self.universe_id = Some(context.universe_id.0);
                self.controller_id = Some(context.controller_id.0);
                Ok(())
            }
            (Some(universe_id), Some(controller_id))
                if universe_id == context.universe_id.0
                    && controller_id == context.controller_id.0 =>
            {
                Ok(())
            }
            _ => Err(RuntimeDiagnosticBridgeError::ProviderBindingMismatch),
        }
    }

    fn validate_provider_stream(
        &self,
        events: &[RuntimeDiagnosticEvent],
        context: ObservationContext,
    ) -> Result<(), RuntimeDiagnosticBridgeError> {
        let mut prior_sequence = None;
        let mut occurrences = BTreeMap::new();
        for event in events {
            if prior_sequence.is_some_and(|prior| event.event_sequence <= prior) {
                return Err(RuntimeDiagnosticBridgeError::ProviderOrderInvalid);
            }
            prior_sequence = Some(event.event_sequence);
            let key = provider_key(event);
            let payload_hash = hash_runtime_provider_event(event);
            if occurrences
                .insert(event.occurrence_id, payload_hash)
                .is_some()
            {
                return Err(RuntimeDiagnosticBridgeError::ProviderOccurrenceCollision(
                    key,
                ));
            }
            if event.universe_epoch == context.universe_epoch
                && event.controller_epoch == context.controller_epoch
                && (event.event_sequence > context.event_sequence
                    || event.virtual_timestamp_ms > context.virtual_timestamp_ms)
            {
                return Err(RuntimeDiagnosticBridgeError::ProviderEventAheadOfRuntime(
                    key,
                ));
            }
            if event.universe_epoch == context.universe_epoch
                && event.controller_epoch == context.controller_epoch
                && let Some(fault) = &event.fault_context
                && (fault.controller_epoch != event.controller_epoch
                    || fault.virtual_timestamp_ms != event.virtual_timestamp_ms
                    || fault.artifact_fingerprint != context.artifact_fingerprint)
            {
                return Err(RuntimeDiagnosticBridgeError::FaultContextBindingMismatch(
                    key,
                ));
            }
        }
        Ok(())
    }

    fn publish_provider_event(
        &self,
        ledger: &mut DiagnosticLedger,
        provider_event: &RuntimeDiagnosticEvent,
        payload_hash: Hash32,
        base_context: ObservationContext,
    ) -> Result<DiagnosticEvent, RuntimeDiagnosticBridgeError> {
        let mapping = runtime_mapping(provider_event.code);
        let definition = ledger.registry().by_code(mapping.registry_code).ok_or(
            RuntimeDiagnosticBridgeError::RegistryMappingUnavailable(provider_event.code),
        )?;
        if definition.lifecycle != mapping.lifecycle {
            return Err(RuntimeDiagnosticBridgeError::RegistryLifecycleMismatch(
                provider_event.code,
            ));
        }
        let causal = self.resolve_causality(provider_event)?;
        let mut context = base_context;
        context.event_sequence = provider_event.event_sequence;
        context.virtual_timestamp_ms = provider_event.virtual_timestamp_ms;
        context.publication_boundary = if provider_event.fault_context.is_some() {
            PublicationBoundary::FatalFault
        } else {
            PublicationBoundary::SerializedCommand
        };
        let severity = map_runtime_severity(provider_event.severity);
        let related_identities =
            runtime_related_identities(provider_event, context.controller_id.0);
        let transition = match mapping.lifecycle {
            DiagnosticLifecycle::Condition => DiagnosticTransition::ActivateCondition {
                transition_id: provider_event.occurrence_id,
                key: ConditionKey {
                    definition_id: definition.id,
                    subject_identity: provider_event
                        .fault_context
                        .as_ref()
                        .map_or(context.controller_id.0, |fault| fault.source_identity),
                    provider_instance_identity: context.controller_id.0,
                    discriminator_hash: hash_runtime_condition_identity(provider_event),
                },
                severity_override: Some(severity),
                payload_hash,
                related_identities,
                causal,
                rejectable: false,
            },
            DiagnosticLifecycle::OneShot => DiagnosticTransition::EmitOneShot {
                transition_id: provider_event.occurrence_id,
                definition_id: definition.id,
                severity_override: Some(severity),
                payload_hash,
                related_identities,
                causal,
            },
            DiagnosticLifecycle::Compaction => {
                return Err(RuntimeDiagnosticBridgeError::RegistryLifecycleMismatch(
                    provider_event.code,
                ));
            }
        };
        Ok(ledger.apply_provider_transition(transition, context)?)
    }

    fn resolve_causality(
        &self,
        event: &RuntimeDiagnosticEvent,
    ) -> Result<CausalReference, RuntimeDiagnosticBridgeError> {
        let parent_occurrence_id = event
            .parent_occurrence_id
            .map(|id| self.ledger_occurrence_for_provider(id))
            .transpose()?
            .flatten();
        let root_occurrence_id = if event.root_occurrence_id == event.occurrence_id {
            None
        } else {
            self.ledger_occurrence_for_provider(event.root_occurrence_id)?
        };
        Ok(CausalReference {
            parent_occurrence_id,
            root_occurrence_id,
        })
    }

    fn ledger_occurrence_for_provider(
        &self,
        provider_occurrence_id: u128,
    ) -> Result<Option<crate::OccurrenceId>, RuntimeDiagnosticBridgeError> {
        self.receipts
            .values()
            .find(|receipt| receipt.provider_key.occurrence_id == provider_occurrence_id)
            .map(|receipt| Some(receipt.ledger_occurrence_id))
            .ok_or(
                RuntimeDiagnosticBridgeError::UnknownProviderCausalReference(
                    provider_occurrence_id,
                ),
            )
    }

    fn calculate_hash(&self) -> Hash32 {
        let mut hasher = CanonicalHasher::new("PES-RUNTIME-DIAGNOSTIC-BRIDGE-1");
        match self.universe_id {
            Some(value) => {
                hasher.bool(true);
                hasher.u128(value);
            }
            None => hasher.bool(false),
        }
        match self.controller_id {
            Some(value) => {
                hasher.bool(true);
                hasher.u128(value);
            }
            None => hasher.bool(false),
        }
        hasher.u64(self.receipts.len() as u64);
        for receipt in self.receipts.values() {
            hasher.hash(receipt.receipt_hash);
        }
        hasher.finish()
    }
}

#[derive(Clone, Copy)]
struct RuntimeMapping {
    registry_code: &'static str,
    lifecycle: DiagnosticLifecycle,
}

fn runtime_mapping(code: RuntimeDiagnosticCode) -> RuntimeMapping {
    match code {
        RuntimeDiagnosticCode::IllegalCpuTransition => RuntimeMapping {
            registry_code: "EDU-CPU-0002",
            lifecycle: DiagnosticLifecycle::OneShot,
        },
        RuntimeDiagnosticCode::ArtifactRejected => RuntimeMapping {
            registry_code: "EDU-COM-0003",
            lifecycle: DiagnosticLifecycle::OneShot,
        },
        RuntimeDiagnosticCode::ArithmeticDivideByZero
        | RuntimeDiagnosticCode::ArithmeticOverflow => RuntimeMapping {
            registry_code: "EDU-RTM-0001",
            lifecycle: DiagnosticLifecycle::Condition,
        },
        RuntimeDiagnosticCode::TimerOverflow => RuntimeMapping {
            registry_code: "EDU-RTM-0003",
            lifecycle: DiagnosticLifecycle::Condition,
        },
        RuntimeDiagnosticCode::WorkUnitBudgetExceeded => RuntimeMapping {
            registry_code: "EDU-RTM-0004",
            lifecycle: DiagnosticLifecycle::Condition,
        },
        RuntimeDiagnosticCode::RuntimeInvariantFailure => RuntimeMapping {
            registry_code: "EDU-RTM-0006",
            lifecycle: DiagnosticLifecycle::Condition,
        },
        RuntimeDiagnosticCode::SnapshotRejected => RuntimeMapping {
            registry_code: "EDU-SNP-0001",
            lifecycle: DiagnosticLifecycle::OneShot,
        },
    }
}

fn provider_key(event: &RuntimeDiagnosticEvent) -> RuntimeDiagnosticProviderKey {
    RuntimeDiagnosticProviderKey {
        universe_epoch: event.universe_epoch,
        controller_epoch: event.controller_epoch,
        occurrence_id: event.occurrence_id,
    }
}

fn map_runtime_severity(value: RuntimeDiagnosticSeverity) -> DiagnosticSeverity {
    match value {
        RuntimeDiagnosticSeverity::Information => DiagnosticSeverity::Info,
        RuntimeDiagnosticSeverity::Warning => DiagnosticSeverity::Warning,
        RuntimeDiagnosticSeverity::Error => DiagnosticSeverity::Error,
        RuntimeDiagnosticSeverity::Fatal => DiagnosticSeverity::Fatal,
    }
}

fn runtime_related_identities(event: &RuntimeDiagnosticEvent, controller_id: u128) -> Vec<u128> {
    let mut identities = vec![controller_id];
    if let Some(fault) = &event.fault_context
        && fault.source_identity != 0
    {
        identities.push(fault.source_identity);
    }
    identities.sort_unstable();
    identities.dedup();
    identities
}

fn hash_runtime_condition_identity(event: &RuntimeDiagnosticEvent) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-RUNTIME-DIAGNOSTIC-CONDITION-1");
    hasher.u16(event.code as u16);
    match &event.fault_context {
        Some(fault) => {
            hasher.bool(true);
            hasher.hash(fault.artifact_fingerprint);
            hasher.u32(fault.block_id.0);
            hasher.u32(fault.operation_id);
            hasher.u128(fault.source_identity);
        }
        None => hasher.bool(false),
    }
    hasher.finish()
}

fn hash_runtime_provider_event(event: &RuntimeDiagnosticEvent) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-RUNTIME-DIAGNOSTIC-PROVIDER-EVENT-1");
    hasher.u128(event.occurrence_id);
    match event.parent_occurrence_id {
        Some(value) => {
            hasher.bool(true);
            hasher.u128(value);
        }
        None => hasher.bool(false),
    }
    hasher.u128(event.root_occurrence_id);
    hasher.u16(event.code as u16);
    hasher.u8(event.severity as u8);
    hasher.u64(event.universe_epoch);
    hasher.u64(event.controller_epoch);
    hasher.u64(event.event_sequence);
    hasher.u64(event.virtual_timestamp_ms);
    match &event.fault_context {
        Some(fault) => {
            hasher.bool(true);
            hasher.hash(fault.artifact_fingerprint);
            hasher.u32(fault.block_id.0);
            hasher.u32(fault.operation_id);
            hasher.u128(fault.source_identity);
            hasher.u64(fault.scan_sequence);
            hasher.u64(fault.controller_epoch);
            hasher.u64(fault.virtual_timestamp_ms);
            hasher.u32(fault.work_units_before_operation);
        }
        None => hasher.bool(false),
    }
    match event.fault_boundary_state_hash {
        Some(value) => {
            hasher.bool(true);
            hasher.hash(value);
        }
        None => hasher.bool(false),
    }
    hasher.finish()
}

fn hash_runtime_receipt(receipt: &RuntimeDiagnosticReceipt) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-RUNTIME-DIAGNOSTIC-RECEIPT-1");
    hasher.u64(receipt.provider_key.universe_epoch);
    hasher.u64(receipt.provider_key.controller_epoch);
    hasher.u128(receipt.provider_key.occurrence_id);
    hasher.u16(receipt.provider_code as u16);
    hasher.u64(receipt.provider_event_sequence);
    hasher.u64(receipt.provider_virtual_timestamp_ms);
    hasher.hash(receipt.provider_payload_hash);
    hasher.u128(receipt.ledger_occurrence_id.0);
    hasher.hash(receipt.ledger_event_hash);
    hasher.finish()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeDiagnosticBridgeError {
    Context(ContextError),
    Ledger(DiagnosticError),
    TargetUnavailable,
    ProviderBindingMismatch,
    ProviderOrderInvalid,
    ProviderOccurrenceCollision(RuntimeDiagnosticProviderKey),
    HistoricalProviderEventRequiresBridgeState(RuntimeDiagnosticProviderKey),
    ProviderEventAheadOfRuntime(RuntimeDiagnosticProviderKey),
    FaultContextBindingMismatch(RuntimeDiagnosticProviderKey),
    RegistryMappingUnavailable(RuntimeDiagnosticCode),
    RegistryLifecycleMismatch(RuntimeDiagnosticCode),
    UnknownProviderCausalReference(u128),
    ReceiptIntegrityMismatch(RuntimeDiagnosticProviderKey),
    BridgeIntegrityMismatch,
}

impl From<ContextError> for RuntimeDiagnosticBridgeError {
    fn from(value: ContextError) -> Self {
        Self::Context(value)
    }
}

impl From<DiagnosticError> for RuntimeDiagnosticBridgeError {
    fn from(value: DiagnosticError) -> Self {
        Self::Ledger(value)
    }
}

impl fmt::Display for RuntimeDiagnosticBridgeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "runtime diagnostic bridge rejected publication: {self:?}"
        )
    }
}

impl Error for RuntimeDiagnosticBridgeError {}
