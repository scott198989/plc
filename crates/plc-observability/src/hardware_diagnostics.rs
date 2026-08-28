use alloc::{collections::BTreeMap, vec, vec::Vec};
use core::{error::Error, fmt};

use plc_hardware::{
    ConditionLifecycle, HardwareConditionEvent, HardwareConditionKey, HardwareDiagnosticCode,
};
use plc_runtime::Hash32;

use crate::{
    CausalReference, ConditionKey, DiagnosticError, DiagnosticEvent, DiagnosticLedger,
    DiagnosticLifecycle, DiagnosticTransition, ObservationContext, PublicationBoundary,
    canonical::{CanonicalHasher, id128},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HardwareDiagnosticProviderKey {
    pub universe_epoch: u64,
    pub controller_epoch: u64,
    pub provider_event_sequence: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HardwareDiagnosticReceipt {
    pub provider_key: HardwareDiagnosticProviderKey,
    pub condition: HardwareConditionKey,
    pub lifecycle: ConditionLifecycle,
    pub provider_code: HardwareDiagnosticCode,
    pub provider_command_boundary: u64,
    pub provider_payload_hash: Hash32,
    pub ledger_occurrence_id: crate::OccurrenceId,
    pub ledger_event_hash: Hash32,
    pub duplicate: bool,
    pub receipt_hash: Hash32,
}

impl HardwareDiagnosticReceipt {
    #[must_use]
    pub fn verify(&self) -> bool {
        self.receipt_hash == hash_hardware_receipt(self)
    }
}

#[derive(Clone, Debug)]
pub struct HardwareDiagnosticBridge {
    universe_id: Option<u128>,
    controller_id: Option<u128>,
    receipts: BTreeMap<HardwareDiagnosticProviderKey, HardwareDiagnosticReceipt>,
    bridge_hash: Hash32,
}

impl Default for HardwareDiagnosticBridge {
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

impl HardwareDiagnosticBridge {
    #[must_use]
    pub const fn bridge_hash(&self) -> Hash32 {
        self.bridge_hash
    }

    pub fn receipts(&self) -> impl ExactSizeIterator<Item = &HardwareDiagnosticReceipt> {
        self.receipts.values()
    }

    pub fn replay_hash(&self) -> Result<Hash32, HardwareDiagnosticBridgeError> {
        for receipt in self.receipts.values() {
            if !receipt.verify() {
                return Err(HardwareDiagnosticBridgeError::ReceiptIntegrityMismatch(
                    receipt.provider_key,
                ));
            }
        }
        let calculated = self.calculate_hash();
        if calculated != self.bridge_hash {
            return Err(HardwareDiagnosticBridgeError::BridgeIntegrityMismatch);
        }
        Ok(calculated)
    }

    /// Atomically publishes newly accepted hardware-condition events through
    /// the canonical diagnostic ledger. The caller supplies the authoritative
    /// current runtime context; events never acquire host or UI causality.
    pub fn ingest_events(
        &mut self,
        ledger: &mut DiagnosticLedger,
        mut context: ObservationContext,
        events: &[HardwareConditionEvent],
    ) -> Result<Vec<HardwareDiagnosticReceipt>, HardwareDiagnosticBridgeError> {
        context.publication_boundary = PublicationBoundary::SerializedCommand;
        let mut candidate = self.clone();
        let mut ledger_candidate = ledger.clone();
        candidate.bind(context)?;
        validate_provider_stream(events)?;

        let mut output = Vec::with_capacity(events.len());
        for provider_event in events {
            let key = HardwareDiagnosticProviderKey {
                universe_epoch: context.universe_epoch,
                controller_epoch: context.controller_epoch,
                provider_event_sequence: provider_event.sequence,
            };
            let payload_hash = hash_hardware_provider_event(provider_event);
            if let Some(stored) = candidate.receipts.get(&key) {
                if stored.provider_payload_hash != payload_hash {
                    return Err(HardwareDiagnosticBridgeError::ProviderSequenceCollision(
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
                context,
            )?;
            let mut receipt = HardwareDiagnosticReceipt {
                provider_key: key,
                condition: provider_event.condition,
                lifecycle: provider_event.lifecycle,
                provider_code: provider_event.diagnostic_code,
                provider_command_boundary: provider_event.command_boundary,
                provider_payload_hash: payload_hash,
                ledger_occurrence_id: event.occurrence_id,
                ledger_event_hash: event.event_hash,
                duplicate: false,
                receipt_hash: Hash32::ZERO,
            };
            receipt.receipt_hash = hash_hardware_receipt(&receipt);
            candidate.receipts.insert(key, receipt.clone());
            output.push(receipt);
        }

        candidate.bridge_hash = candidate.calculate_hash();
        *ledger = ledger_candidate;
        *self = candidate;
        Ok(output)
    }

    fn bind(&mut self, context: ObservationContext) -> Result<(), HardwareDiagnosticBridgeError> {
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
            _ => Err(HardwareDiagnosticBridgeError::ProviderBindingMismatch),
        }
    }

    fn publish_provider_event(
        &self,
        ledger: &mut DiagnosticLedger,
        provider_event: &HardwareConditionEvent,
        payload_hash: Hash32,
        context: ObservationContext,
    ) -> Result<DiagnosticEvent, HardwareDiagnosticBridgeError> {
        let registry_code = provider_event.diagnostic_code.stable_code();
        let definition = ledger.registry().by_code(registry_code).ok_or(
            HardwareDiagnosticBridgeError::RegistryMappingUnavailable(
                provider_event.diagnostic_code,
            ),
        )?;
        if definition.lifecycle != DiagnosticLifecycle::Condition {
            return Err(HardwareDiagnosticBridgeError::RegistryLifecycleMismatch(
                provider_event.diagnostic_code,
            ));
        }
        let subject_identity = hardware_subject_identity(provider_event.condition);
        let condition_key = ConditionKey {
            definition_id: definition.id,
            subject_identity,
            provider_instance_identity: context.controller_id.0,
            discriminator_hash: hash_hardware_condition(provider_event.condition),
        };
        let transition = match provider_event.lifecycle {
            ConditionLifecycle::Activated => DiagnosticTransition::ActivateCondition {
                transition_id: id128(payload_hash),
                key: condition_key,
                severity_override: None,
                payload_hash,
                related_identities: vec![context.controller_id.0, subject_identity],
                causal: CausalReference::root(),
                rejectable: true,
            },
            ConditionLifecycle::Cleared => DiagnosticTransition::ClearCondition {
                transition_id: id128(payload_hash),
                key: condition_key,
                payload_hash,
                causal: self.activation_causality(provider_event.condition, context)?,
            },
        };
        Ok(ledger.apply_provider_transition(transition, context)?)
    }

    fn activation_causality(
        &self,
        condition: HardwareConditionKey,
        context: ObservationContext,
    ) -> Result<CausalReference, HardwareDiagnosticBridgeError> {
        let occurrence = self
            .receipts
            .values()
            .rev()
            .find(|receipt| {
                receipt.provider_key.universe_epoch == context.universe_epoch
                    && receipt.provider_key.controller_epoch == context.controller_epoch
                    && receipt.condition == condition
                    && receipt.lifecycle == ConditionLifecycle::Activated
            })
            .map(|receipt| receipt.ledger_occurrence_id)
            .ok_or(HardwareDiagnosticBridgeError::MissingActivation(condition))?;
        Ok(CausalReference {
            parent_occurrence_id: Some(occurrence),
            root_occurrence_id: Some(occurrence),
        })
    }

    fn calculate_hash(&self) -> Hash32 {
        let mut hasher = CanonicalHasher::new("PES-HARDWARE-DIAGNOSTIC-BRIDGE-1");
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

fn validate_provider_stream(
    events: &[HardwareConditionEvent],
) -> Result<(), HardwareDiagnosticBridgeError> {
    let mut prior_sequence = None;
    for event in events {
        if event.sequence == 0
            || event.command_boundary == 0
            || prior_sequence.is_some_and(|prior| event.sequence <= prior)
        {
            return Err(HardwareDiagnosticBridgeError::ProviderOrderInvalid);
        }
        if event.diagnostic_code != event.condition.diagnostic_code() {
            return Err(HardwareDiagnosticBridgeError::ProviderCodeMismatch {
                condition: event.condition,
                code: event.diagnostic_code,
            });
        }
        prior_sequence = Some(event.sequence);
    }
    Ok(())
}

fn hardware_subject_identity(condition: HardwareConditionKey) -> u128 {
    let uuid = match condition {
        HardwareConditionKey::ModuleNotPresent(id) | HardwareConditionKey::WrongModule(id) => {
            id.uuid()
        }
        HardwareConditionKey::ChannelFault(id) | HardwareConditionKey::WireBreak(id) => id.uuid(),
        HardwareConditionKey::ControllerUnpowered(id)
        | HardwareConditionKey::StationUnavailable(id) => id.uuid(),
        HardwareConditionKey::LinkUnavailable(id) => id.uuid(),
    };
    u128::from_be_bytes(uuid.into_bytes())
}

fn hash_hardware_condition(condition: HardwareConditionKey) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-HARDWARE-DIAGNOSTIC-CONDITION-1");
    hasher.string(condition.diagnostic_code().stable_code());
    hasher.u128(hardware_subject_identity(condition));
    hasher.finish()
}

fn hash_hardware_provider_event(event: &HardwareConditionEvent) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-HARDWARE-DIAGNOSTIC-PROVIDER-EVENT-1");
    hasher.u64(event.sequence);
    hasher.u64(event.command_boundary);
    hasher.hash(hash_hardware_condition(event.condition));
    hasher.u8(match event.lifecycle {
        ConditionLifecycle::Activated => 1,
        ConditionLifecycle::Cleared => 2,
    });
    hasher.string(event.diagnostic_code.stable_code());
    hasher.finish()
}

fn hash_hardware_receipt(receipt: &HardwareDiagnosticReceipt) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-HARDWARE-DIAGNOSTIC-RECEIPT-1");
    hasher.u64(receipt.provider_key.universe_epoch);
    hasher.u64(receipt.provider_key.controller_epoch);
    hasher.u64(receipt.provider_key.provider_event_sequence);
    hasher.hash(hash_hardware_condition(receipt.condition));
    hasher.u8(match receipt.lifecycle {
        ConditionLifecycle::Activated => 1,
        ConditionLifecycle::Cleared => 2,
    });
    hasher.string(receipt.provider_code.stable_code());
    hasher.u64(receipt.provider_command_boundary);
    hasher.hash(receipt.provider_payload_hash);
    hasher.u128(receipt.ledger_occurrence_id.0);
    hasher.hash(receipt.ledger_event_hash);
    hasher.finish()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HardwareDiagnosticBridgeError {
    Ledger(DiagnosticError),
    ProviderBindingMismatch,
    ProviderOrderInvalid,
    ProviderSequenceCollision(HardwareDiagnosticProviderKey),
    ProviderCodeMismatch {
        condition: HardwareConditionKey,
        code: HardwareDiagnosticCode,
    },
    RegistryMappingUnavailable(HardwareDiagnosticCode),
    RegistryLifecycleMismatch(HardwareDiagnosticCode),
    MissingActivation(HardwareConditionKey),
    ReceiptIntegrityMismatch(HardwareDiagnosticProviderKey),
    BridgeIntegrityMismatch,
}

impl From<DiagnosticError> for HardwareDiagnosticBridgeError {
    fn from(value: DiagnosticError) -> Self {
        Self::Ledger(value)
    }
}

impl fmt::Display for HardwareDiagnosticBridgeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "hardware diagnostic bridge rejected publication: {self:?}"
        )
    }
}

impl Error for HardwareDiagnosticBridgeError {}
