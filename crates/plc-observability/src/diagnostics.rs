use alloc::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    vec,
    vec::Vec,
};
use core::{error::Error, fmt};

use plc_runtime::{Hash32, Sha256};

use crate::{
    ObservationContext,
    canonical::{CanonicalHasher, id128},
};

const DIAGNOSTIC_UUID_NAMESPACE: [u8; 16] = [
    0x7e, 0x97, 0xc4, 0xb2, 0x8c, 0xc0, 0x5d, 0x01, 0xbf, 0x68, 0x4b, 0x76, 0xc0, 0xc8, 0x0d, 0x21,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DiagnosticId(pub u128);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OccurrenceId(pub u128);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConditionId(pub u128);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DiagnosticCode(pub &'static str);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DiagnosticSeverity {
    Info = 1,
    Warning = 2,
    Error = 3,
    Fatal = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DiagnosticLifecycle {
    Condition = 1,
    OneShot = 2,
    Compaction = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DiagnosticCpuResponse {
    None = 1,
    Faulted = 2,
    Rollback = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DiagnosticSource {
    Runtime = 1,
    Cpu = 2,
    VirtualIo = 3,
    Commissioning = 4,
    Session = 5,
    Monitoring = 6,
    Modify = 7,
    Force = 8,
    Trace = 9,
    Snapshot = 10,
    Diagnostics = 11,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticDefinition {
    pub id: DiagnosticId,
    pub code: DiagnosticCode,
    pub code_version: u64,
    pub mnemonic: &'static str,
    pub lifecycle: DiagnosticLifecycle,
    pub default_severity: DiagnosticSeverity,
    pub acknowledgeable: bool,
    pub cpu_response: DiagnosticCpuResponse,
    pub source: DiagnosticSource,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticRegistry {
    definitions: BTreeMap<DiagnosticId, DiagnosticDefinition>,
    by_code: BTreeMap<&'static str, DiagnosticId>,
    registry_hash: Hash32,
}

impl DiagnosticRegistry {
    pub fn edu21_runtime() -> Self {
        let rows = [
            (
                "EDU-RTM-0001",
                "ARITHMETIC_FAULT",
                DiagnosticLifecycle::Condition,
                DiagnosticSeverity::Fatal,
                false,
                DiagnosticCpuResponse::Faulted,
                DiagnosticSource::Runtime,
            ),
            (
                "EDU-RTM-0002",
                "BOUNDS_OR_STRING_FAULT",
                DiagnosticLifecycle::Condition,
                DiagnosticSeverity::Fatal,
                false,
                DiagnosticCpuResponse::Faulted,
                DiagnosticSource::Runtime,
            ),
            (
                "EDU-RTM-0003",
                "TIMER_PRESET_OR_OVERFLOW",
                DiagnosticLifecycle::Condition,
                DiagnosticSeverity::Fatal,
                false,
                DiagnosticCpuResponse::Faulted,
                DiagnosticSource::Runtime,
            ),
            (
                "EDU-RTM-0004",
                "WORK_BUDGET_EXCEEDED",
                DiagnosticLifecycle::Condition,
                DiagnosticSeverity::Fatal,
                false,
                DiagnosticCpuResponse::Faulted,
                DiagnosticSource::Runtime,
            ),
            (
                "EDU-RTM-0005",
                "CALL_DEPTH_EXCEEDED",
                DiagnosticLifecycle::Condition,
                DiagnosticSeverity::Fatal,
                false,
                DiagnosticCpuResponse::Faulted,
                DiagnosticSource::Runtime,
            ),
            (
                "EDU-RTM-0006",
                "RUNTIME_INVARIANT_FAILED",
                DiagnosticLifecycle::Condition,
                DiagnosticSeverity::Fatal,
                false,
                DiagnosticCpuResponse::Faulted,
                DiagnosticSource::Runtime,
            ),
            (
                "EDU-RTM-0007",
                "COUNTER_SATURATION",
                DiagnosticLifecycle::Condition,
                DiagnosticSeverity::Warning,
                true,
                DiagnosticCpuResponse::None,
                DiagnosticSource::Runtime,
            ),
            (
                "EDU-CPU-0001",
                "STARTUP_VALIDATION_FAILED",
                DiagnosticLifecycle::Condition,
                DiagnosticSeverity::Fatal,
                false,
                DiagnosticCpuResponse::Faulted,
                DiagnosticSource::Cpu,
            ),
            (
                "EDU-CPU-0002",
                "MODE_TRANSITION_REJECTED",
                DiagnosticLifecycle::OneShot,
                DiagnosticSeverity::Warning,
                false,
                DiagnosticCpuResponse::None,
                DiagnosticSource::Cpu,
            ),
            (
                "EDU-IO-0001",
                "MODULE_NOT_PRESENT",
                DiagnosticLifecycle::Condition,
                DiagnosticSeverity::Warning,
                true,
                DiagnosticCpuResponse::None,
                DiagnosticSource::VirtualIo,
            ),
            (
                "EDU-IO-0002",
                "WRONG_MODULE",
                DiagnosticLifecycle::Condition,
                DiagnosticSeverity::Error,
                true,
                DiagnosticCpuResponse::None,
                DiagnosticSource::VirtualIo,
            ),
            (
                "EDU-IO-0003",
                "CHANNEL_FAULT",
                DiagnosticLifecycle::Condition,
                DiagnosticSeverity::Warning,
                true,
                DiagnosticCpuResponse::None,
                DiagnosticSource::VirtualIo,
            ),
            (
                "EDU-IO-0004",
                "WIRE_BREAK",
                DiagnosticLifecycle::Condition,
                DiagnosticSeverity::Warning,
                true,
                DiagnosticCpuResponse::None,
                DiagnosticSource::VirtualIo,
            ),
            (
                "EDU-COM-0001",
                "STATION_UNAVAILABLE",
                DiagnosticLifecycle::Condition,
                DiagnosticSeverity::Warning,
                true,
                DiagnosticCpuResponse::None,
                DiagnosticSource::Commissioning,
            ),
            (
                "EDU-COM-0002",
                "LINK_UNAVAILABLE",
                DiagnosticLifecycle::Condition,
                DiagnosticSeverity::Warning,
                true,
                DiagnosticCpuResponse::None,
                DiagnosticSource::Commissioning,
            ),
            (
                "EDU-COM-0003",
                "LOAD_COMMIT_FAILED",
                DiagnosticLifecycle::OneShot,
                DiagnosticSeverity::Error,
                false,
                DiagnosticCpuResponse::Rollback,
                DiagnosticSource::Commissioning,
            ),
            (
                "EDU-SES-0001",
                "ONLINE_SESSION_LOST",
                DiagnosticLifecycle::OneShot,
                DiagnosticSeverity::Warning,
                false,
                DiagnosticCpuResponse::None,
                DiagnosticSource::Session,
            ),
            (
                "EDU-MON-0001",
                "MONITOR_MAPPING_LOST",
                DiagnosticLifecycle::OneShot,
                DiagnosticSeverity::Warning,
                false,
                DiagnosticCpuResponse::None,
                DiagnosticSource::Monitoring,
            ),
            (
                "EDU-MOD-0001",
                "MODIFY_REJECTED_OR_CANCELED",
                DiagnosticLifecycle::OneShot,
                DiagnosticSeverity::Warning,
                false,
                DiagnosticCpuResponse::None,
                DiagnosticSource::Modify,
            ),
            (
                "EDU-FRC-0001",
                "FORCE_ACTIVATED_OR_REPLACED",
                DiagnosticLifecycle::OneShot,
                DiagnosticSeverity::Warning,
                false,
                DiagnosticCpuResponse::None,
                DiagnosticSource::Force,
            ),
            (
                "EDU-FRC-0002",
                "FORCE_REMOVED",
                DiagnosticLifecycle::OneShot,
                DiagnosticSeverity::Info,
                false,
                DiagnosticCpuResponse::None,
                DiagnosticSource::Force,
            ),
            (
                "EDU-TRC-0001",
                "TRACE_ABORTED",
                DiagnosticLifecycle::OneShot,
                DiagnosticSeverity::Warning,
                false,
                DiagnosticCpuResponse::None,
                DiagnosticSource::Trace,
            ),
            (
                "EDU-TRC-0002",
                "TRACE_GAP_OR_LIMIT",
                DiagnosticLifecycle::OneShot,
                DiagnosticSeverity::Warning,
                false,
                DiagnosticCpuResponse::None,
                DiagnosticSource::Trace,
            ),
            (
                "EDU-SNP-0001",
                "SNAPSHOT_RESTORE_FAILED",
                DiagnosticLifecycle::OneShot,
                DiagnosticSeverity::Error,
                false,
                DiagnosticCpuResponse::Rollback,
                DiagnosticSource::Snapshot,
            ),
            (
                "EDU-DIA-0001",
                "EVENT_GAP",
                DiagnosticLifecycle::Compaction,
                DiagnosticSeverity::Warning,
                false,
                DiagnosticCpuResponse::None,
                DiagnosticSource::Diagnostics,
            ),
            (
                "EDU-DIA-0002",
                "DIAGNOSTIC_CAPACITY_REJECTED",
                DiagnosticLifecycle::OneShot,
                DiagnosticSeverity::Error,
                false,
                DiagnosticCpuResponse::None,
                DiagnosticSource::Diagnostics,
            ),
        ];
        let mut definitions = BTreeMap::new();
        let mut by_code = BTreeMap::new();
        for (code, mnemonic, lifecycle, severity, acknowledgeable, cpu_response, source) in rows {
            let id = diagnostic_id(code);
            definitions.insert(
                id,
                DiagnosticDefinition {
                    id,
                    code: DiagnosticCode(code),
                    code_version: 1,
                    mnemonic,
                    lifecycle,
                    default_severity: severity,
                    acknowledgeable,
                    cpu_response,
                    source,
                },
            );
            by_code.insert(code, id);
        }
        let registry_hash = hash_registry(&definitions);
        Self {
            definitions,
            by_code,
            registry_hash,
        }
    }

    pub fn definition(&self, id: DiagnosticId) -> Option<&DiagnosticDefinition> {
        self.definitions.get(&id)
    }

    pub fn by_code(&self, code: &str) -> Option<&DiagnosticDefinition> {
        self.by_code
            .get(code)
            .and_then(|id| self.definitions.get(id))
    }

    pub fn definitions(&self) -> impl ExactSizeIterator<Item = &DiagnosticDefinition> {
        self.definitions.values()
    }

    pub const fn registry_hash(&self) -> Hash32 {
        self.registry_hash
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConditionKey {
    pub definition_id: DiagnosticId,
    pub subject_identity: u128,
    pub provider_instance_identity: u128,
    pub discriminator_hash: Hash32,
}

impl ConditionKey {
    pub fn canonical_hash(self) -> Hash32 {
        let provider = if self.provider_instance_identity == 0 {
            dia_null()
        } else {
            dia_uuid(self.provider_instance_identity)
        };
        let primary_target =
            dia_tuple(&[dia_enum("STABLE_TARGET"), dia_uuid(self.subject_identity)]);
        let discriminator = dia_tuple(&[
            dia_string("discriminator"),
            dia_string("HASH32"),
            dia_hash(self.discriminator_hash),
        ]);
        Sha256::digest(&dia_tuple(&[
            dia_uuid(self.definition_id.0),
            provider,
            primary_target,
            discriminator,
        ]))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CausalReference {
    pub parent_occurrence_id: Option<OccurrenceId>,
    pub root_occurrence_id: Option<OccurrenceId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CausalResolution {
    Retained = 1,
    Compacted = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiagnosticOrderKey {
    pub universe_epoch: u64,
    pub controller_epoch: u64,
    pub event_sequence: u64,
    pub occurrence_id: OccurrenceId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompactedDiagnosticReference {
    pub occurrence_id: OccurrenceId,
    pub definition_id: DiagnosticId,
    pub code_version: u64,
    pub lifecycle: DiagnosticEventKind,
    pub primary_target_id: Option<u128>,
    pub canonical_order_key: DiagnosticOrderKey,
    pub root_occurrence_id: OccurrenceId,
}

impl CausalReference {
    pub const fn root() -> Self {
        Self {
            parent_occurrence_id: None,
            root_occurrence_id: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DiagnosticEventKind {
    Incoming = 1,
    Acknowledged = 2,
    Cleared = 3,
    OneShot = 4,
    Compaction = 5,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticEvent {
    pub occurrence_id: OccurrenceId,
    pub condition_id: Option<ConditionId>,
    pub definition_id: DiagnosticId,
    pub kind: DiagnosticEventKind,
    pub severity: DiagnosticSeverity,
    pub condition_key: Option<ConditionKey>,
    pub lifecycle_episode: u64,
    pub parent_occurrence_id: Option<OccurrenceId>,
    pub parent_resolution: Option<CausalResolution>,
    pub root_occurrence_id: OccurrenceId,
    pub root_resolution: CausalResolution,
    pub universe_epoch: u64,
    pub controller_epoch: u64,
    pub session_epoch: u64,
    pub event_sequence: u64,
    pub virtual_timestamp_ms: u64,
    pub payload_hash: Hash32,
    pub related_identities: Vec<u128>,
    pub compacted_count: u64,
    pub compacted_first_sequence: Option<u64>,
    pub compacted_last_sequence: Option<u64>,
    pub compacted_first_key: Option<DiagnosticOrderKey>,
    pub compacted_last_key: Option<DiagnosticOrderKey>,
    pub compacted_references: Vec<CompactedDiagnosticReference>,
    pub event_hash: Hash32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveCondition {
    pub key: ConditionKey,
    pub condition_id: ConditionId,
    pub incoming_occurrence_id: OccurrenceId,
    pub root_occurrence_id: OccurrenceId,
    pub severity: DiagnosticSeverity,
    pub acknowledgeable: bool,
    pub acknowledged: bool,
    pub lifecycle_episode: u64,
    pub payload_hash: Hash32,
    pub related_identities: Vec<u128>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiagnosticTransition {
    ActivateCondition {
        transition_id: u128,
        key: ConditionKey,
        severity_override: Option<DiagnosticSeverity>,
        payload_hash: Hash32,
        related_identities: Vec<u128>,
        causal: CausalReference,
        rejectable: bool,
    },
    AcknowledgeCondition {
        transition_id: u128,
        key: ConditionKey,
        actor_identity: u128,
        causal: CausalReference,
    },
    ClearCondition {
        transition_id: u128,
        key: ConditionKey,
        payload_hash: Hash32,
        causal: CausalReference,
    },
    EmitOneShot {
        transition_id: u128,
        definition_id: DiagnosticId,
        severity_override: Option<DiagnosticSeverity>,
        payload_hash: Hash32,
        related_identities: Vec<u128>,
        causal: CausalReference,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiagnosticLimits {
    pub ordinary_nonfatal_active: usize,
    pub total_active: usize,
    pub retained_events: usize,
}

#[derive(Clone, Debug)]
pub struct DiagnosticLedgerSnapshot {
    pub schema_version: u32,
    pub universe_id: u128,
    pub captured_universe_epoch: u64,
    pub controller_id: u128,
    pub captured_controller_epoch: u64,
    pub artifact_fingerprint: Hash32,
    pub profile_fingerprint: Hash32,
    state: DiagnosticLedger,
    pub content_hash: Hash32,
}

impl DiagnosticLedgerSnapshot {
    pub fn verify(&self) -> bool {
        self.schema_version == 1 && self.content_hash == hash_ledger_snapshot(self)
    }

    /// Returns whether the integrity-bound snapshot retains one authoritative
    /// occurrence. Aggregate snapshot owners use this to prove that a provider
    /// receipt resolves inside the captured ledger rather than only in live
    /// mutable state.
    #[must_use]
    pub fn contains_occurrence(&self, occurrence_id: OccurrenceId) -> bool {
        self.verify()
            && self
                .state
                .retained_events()
                .into_iter()
                .any(|event| event.occurrence_id == occurrence_id)
    }
}

impl DiagnosticLimits {
    pub const fn edu21() -> Self {
        Self {
            ordinary_nonfatal_active: 9_999,
            total_active: 10_000,
            retained_events: 100_000,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DiagnosticLedger {
    registry: DiagnosticRegistry,
    limits: DiagnosticLimits,
    active: BTreeMap<ConditionKey, ActiveCondition>,
    episodes: BTreeMap<ConditionKey, u64>,
    events: VecDeque<DiagnosticEvent>,
    gap: Option<DiagnosticEvent>,
    compacted_references: BTreeMap<OccurrenceId, CompactedDiagnosticReference>,
    next_event_sequence: u64,
    ledger_hash: Hash32,
}

impl DiagnosticLedger {
    pub fn new(
        registry: DiagnosticRegistry,
        limits: DiagnosticLimits,
    ) -> Result<Self, DiagnosticError> {
        if limits.ordinary_nonfatal_active == 0
            || limits.total_active <= limits.ordinary_nonfatal_active
            || limits.retained_events < 2
        {
            return Err(DiagnosticError::InvalidLimits);
        }
        let mut value = Self {
            registry,
            limits,
            active: BTreeMap::new(),
            episodes: BTreeMap::new(),
            events: VecDeque::new(),
            gap: None,
            compacted_references: BTreeMap::new(),
            next_event_sequence: 0,
            ledger_hash: Hash32::ZERO,
        };
        value.ledger_hash = value.calculate_hash();
        Ok(value)
    }

    pub fn registry(&self) -> &DiagnosticRegistry {
        &self.registry
    }

    pub const fn ledger_hash(&self) -> Hash32 {
        self.ledger_hash
    }

    pub fn active_conditions(&self) -> impl ExactSizeIterator<Item = &ActiveCondition> {
        self.active.values()
    }

    pub fn retained_events(&self) -> Vec<&DiagnosticEvent> {
        self.gap.iter().chain(self.events.iter()).collect()
    }

    pub fn replay_hash(&self) -> Result<Hash32, DiagnosticError> {
        self.validate_integrity()?;
        Ok(self.calculate_hash())
    }

    pub fn capture_snapshot(&self, context: ObservationContext) -> DiagnosticLedgerSnapshot {
        let mut snapshot = DiagnosticLedgerSnapshot {
            schema_version: 1,
            universe_id: context.universe_id.0,
            captured_universe_epoch: context.universe_epoch,
            controller_id: context.controller_id.0,
            captured_controller_epoch: context.controller_epoch,
            artifact_fingerprint: context.artifact_fingerprint,
            profile_fingerprint: context.profile_fingerprint,
            state: self.clone(),
            content_hash: Hash32::ZERO,
        };
        snapshot.content_hash = hash_ledger_snapshot(&snapshot);
        snapshot
    }

    pub fn restore_snapshot(
        snapshot: &DiagnosticLedgerSnapshot,
        context: ObservationContext,
    ) -> Result<Self, DiagnosticError> {
        if !snapshot.verify() {
            return Err(DiagnosticError::SnapshotIntegrityMismatch);
        }
        if snapshot.universe_id != context.universe_id.0
            || snapshot.controller_id != context.controller_id.0
            || snapshot.artifact_fingerprint != context.artifact_fingerprint
            || snapshot.profile_fingerprint != context.profile_fingerprint
            || context.universe_epoch <= snapshot.captured_universe_epoch
            || context.controller_epoch <= snapshot.captured_controller_epoch
        {
            return Err(DiagnosticError::SnapshotBindingMismatch);
        }
        let mut restored = snapshot.state.clone();
        restored.validate_integrity()?;
        restored.next_event_sequence = context.event_sequence;
        restored.ledger_hash = restored.calculate_hash();
        Ok(restored)
    }

    pub fn apply_provider_transition(
        &mut self,
        transition: DiagnosticTransition,
        context: ObservationContext,
    ) -> Result<DiagnosticEvent, DiagnosticError> {
        let mut candidate = self.clone();
        let result = candidate.apply_transition_inner(transition, context);
        match result {
            Ok(event) => {
                candidate.compact(context)?;
                candidate.ledger_hash = candidate.calculate_hash();
                *self = candidate;
                Ok(event)
            }
            Err(DiagnosticError::CapacityRejected {
                proposed_nonfatal,
                proposed_total,
                proposed_keys,
            }) => {
                let capacity = self
                    .registry
                    .by_code("EDU-DIA-0002")
                    .expect("EDU-21 registry includes capacity rejection")
                    .id;
                let mut payload = CanonicalHasher::new("PES-DIAGNOSTIC-CAPACITY-REJECTION-1");
                payload.u64(proposed_nonfatal as u64);
                payload.u64(proposed_total as u64);
                for key in &proposed_keys {
                    payload.hash(key.canonical_hash());
                }
                let payload_hash = payload.finish();
                let _ = self.apply_provider_transition(
                    DiagnosticTransition::EmitOneShot {
                        transition_id: id128(payload_hash),
                        definition_id: capacity,
                        severity_override: None,
                        payload_hash,
                        related_identities: Vec::new(),
                        causal: CausalReference::root(),
                    },
                    context,
                );
                Err(DiagnosticError::CapacityRejected {
                    proposed_nonfatal,
                    proposed_total,
                    proposed_keys,
                })
            }
            Err(error) => Err(error),
        }
    }

    fn apply_transition_inner(
        &mut self,
        transition: DiagnosticTransition,
        context: ObservationContext,
    ) -> Result<DiagnosticEvent, DiagnosticError> {
        match transition {
            DiagnosticTransition::ActivateCondition {
                transition_id: _,
                key,
                severity_override,
                payload_hash,
                mut related_identities,
                causal,
                rejectable,
            } => {
                let (lifecycle, default_severity, acknowledgeable) = self
                    .registry
                    .definition(key.definition_id)
                    .map(|definition| {
                        (
                            definition.lifecycle,
                            definition.default_severity,
                            definition.acknowledgeable,
                        )
                    })
                    .ok_or(DiagnosticError::UnknownDefinition(key.definition_id))?;
                if lifecycle != DiagnosticLifecycle::Condition {
                    return Err(DiagnosticError::LifecycleMismatch);
                }
                if let Some(active) = self.active.get(&key) {
                    return self
                        .events
                        .iter()
                        .find(|event| event.occurrence_id == active.incoming_occurrence_id)
                        .cloned()
                        .ok_or(DiagnosticError::RuntimeInvariant);
                }
                let severity = severity_override.unwrap_or(default_severity);
                let mut proposed_keys = self.active.keys().copied().collect::<Vec<_>>();
                proposed_keys.push(key);
                proposed_keys.sort_unstable();
                let proposed_nonfatal = self
                    .active
                    .values()
                    .filter(|condition| condition.severity != DiagnosticSeverity::Fatal)
                    .count()
                    + usize::from(severity != DiagnosticSeverity::Fatal);
                let proposed_total = proposed_keys.len();
                let fatal_count = self
                    .active
                    .values()
                    .filter(|condition| condition.severity == DiagnosticSeverity::Fatal)
                    .count()
                    + usize::from(severity == DiagnosticSeverity::Fatal);
                if proposed_nonfatal > self.limits.ordinary_nonfatal_active
                    || proposed_total > self.limits.total_active
                    || fatal_count > 1
                {
                    if rejectable {
                        return Err(DiagnosticError::CapacityRejected {
                            proposed_nonfatal,
                            proposed_total,
                            proposed_keys,
                        });
                    }
                    return self.activate_invariant_for_suppressed(
                        key,
                        payload_hash,
                        context,
                        causal,
                    );
                }
                related_identities.sort_unstable();
                related_identities.dedup();
                let episode = self
                    .episodes
                    .get(&key)
                    .copied()
                    .unwrap_or(0)
                    .saturating_add(1);
                self.episodes.insert(key, episode);
                let event = self.build_event(
                    key.definition_id,
                    DiagnosticEventKind::Incoming,
                    severity,
                    Some(key),
                    episode,
                    None,
                    payload_hash,
                    related_identities.clone(),
                    causal,
                    context,
                )?;
                self.active.insert(
                    key,
                    ActiveCondition {
                        key,
                        condition_id: event
                            .condition_id
                            .expect("incoming condition event has a ConditionId"),
                        incoming_occurrence_id: event.occurrence_id,
                        root_occurrence_id: event.root_occurrence_id,
                        severity,
                        acknowledgeable,
                        acknowledged: false,
                        lifecycle_episode: episode,
                        payload_hash,
                        related_identities,
                    },
                );
                self.push_event(event.clone());
                Ok(event)
            }
            DiagnosticTransition::AcknowledgeCondition {
                transition_id: _,
                key,
                actor_identity,
                causal,
            } => {
                if !matches!(
                    context.cpu_state,
                    plc_runtime::CpuState::Stop
                        | plc_runtime::CpuState::Run
                        | plc_runtime::CpuState::PausedEducational
                        | plc_runtime::CpuState::Faulted
                ) {
                    return Err(DiagnosticError::CpuStateDisallowed(context.cpu_state));
                }
                if context.publication_boundary != crate::PublicationBoundary::SerializedCommand {
                    return Err(DiagnosticError::WrongPublicationBoundary);
                }
                let (severity, lifecycle_episode, condition_id, related_identities) = {
                    let active = self
                        .active
                        .get_mut(&key)
                        .ok_or(DiagnosticError::UnknownCondition(key))?;
                    if !active.acknowledgeable {
                        return Err(DiagnosticError::NotAcknowledgeable(key));
                    }
                    if active.acknowledged {
                        return Err(DiagnosticError::AlreadyAcknowledged(key));
                    }
                    active.acknowledged = true;
                    (
                        active.severity,
                        active.lifecycle_episode,
                        active.condition_id,
                        active.related_identities.clone(),
                    )
                };
                let mut payload = CanonicalHasher::new("PES-DIAGNOSTIC-ACK-1");
                payload.u128(actor_identity);
                let event = self.build_event(
                    key.definition_id,
                    DiagnosticEventKind::Acknowledged,
                    severity,
                    Some(key),
                    lifecycle_episode,
                    Some(condition_id),
                    payload.finish(),
                    related_identities,
                    causal,
                    context,
                )?;
                self.push_event(event.clone());
                Ok(event)
            }
            DiagnosticTransition::ClearCondition {
                transition_id: _,
                key,
                payload_hash,
                causal,
            } => {
                let active = self
                    .active
                    .remove(&key)
                    .ok_or(DiagnosticError::UnknownCondition(key))?;
                let event = self.build_event(
                    key.definition_id,
                    DiagnosticEventKind::Cleared,
                    active.severity,
                    Some(key),
                    active.lifecycle_episode,
                    Some(active.condition_id),
                    payload_hash,
                    active.related_identities,
                    causal,
                    context,
                )?;
                self.push_event(event.clone());
                Ok(event)
            }
            DiagnosticTransition::EmitOneShot {
                transition_id: _,
                definition_id,
                severity_override,
                payload_hash,
                mut related_identities,
                causal,
            } => {
                let definition = self
                    .registry
                    .definition(definition_id)
                    .ok_or(DiagnosticError::UnknownDefinition(definition_id))?;
                if definition.lifecycle != DiagnosticLifecycle::OneShot {
                    return Err(DiagnosticError::LifecycleMismatch);
                }
                related_identities.sort_unstable();
                related_identities.dedup();
                let event = self.build_event(
                    definition_id,
                    DiagnosticEventKind::OneShot,
                    severity_override.unwrap_or(definition.default_severity),
                    None,
                    0,
                    None,
                    payload_hash,
                    related_identities,
                    causal,
                    context,
                )?;
                self.push_event(event.clone());
                Ok(event)
            }
        }
    }

    fn activate_invariant_for_suppressed(
        &mut self,
        suppressed: ConditionKey,
        cause: Hash32,
        context: ObservationContext,
        causal: CausalReference,
    ) -> Result<DiagnosticEvent, DiagnosticError> {
        let invariant = self
            .registry
            .by_code("EDU-RTM-0006")
            .expect("EDU-21 registry includes invariant failure")
            .id;
        let key = ConditionKey {
            definition_id: invariant,
            subject_identity: suppressed.subject_identity,
            provider_instance_identity: suppressed.provider_instance_identity,
            discriminator_hash: suppressed.canonical_hash(),
        };
        if self
            .active
            .values()
            .any(|condition| condition.severity == DiagnosticSeverity::Fatal)
            || self.active.len() >= self.limits.total_active
        {
            return Err(DiagnosticError::FatalCapacityExhausted);
        }
        let episode = self
            .episodes
            .get(&key)
            .copied()
            .unwrap_or(0)
            .saturating_add(1);
        self.episodes.insert(key, episode);
        let event = self.build_event(
            invariant,
            DiagnosticEventKind::Incoming,
            DiagnosticSeverity::Fatal,
            Some(key),
            episode,
            None,
            cause,
            vec![suppressed.subject_identity],
            causal,
            context,
        )?;
        self.active.insert(
            key,
            ActiveCondition {
                key,
                condition_id: event
                    .condition_id
                    .expect("incoming invariant event has a ConditionId"),
                incoming_occurrence_id: event.occurrence_id,
                root_occurrence_id: event.root_occurrence_id,
                severity: DiagnosticSeverity::Fatal,
                acknowledgeable: false,
                acknowledged: false,
                lifecycle_episode: episode,
                payload_hash: cause,
                related_identities: vec![suppressed.subject_identity],
            },
        );
        self.push_event(event.clone());
        Ok(event)
    }

    #[allow(clippy::too_many_arguments)]
    fn build_event(
        &mut self,
        definition_id: DiagnosticId,
        kind: DiagnosticEventKind,
        severity: DiagnosticSeverity,
        condition_key: Option<ConditionKey>,
        lifecycle_episode: u64,
        existing_condition_id: Option<ConditionId>,
        payload_hash: Hash32,
        related_identities: Vec<u128>,
        causal: CausalReference,
        context: ObservationContext,
    ) -> Result<DiagnosticEvent, DiagnosticError> {
        self.next_event_sequence = self.next_event_sequence.saturating_add(1);
        let parent = self.resolve_reference(causal.parent_occurrence_id)?;
        let explicit_root = self.resolve_reference(causal.root_occurrence_id)?;
        let occurrence_id = occurrence_id(context, definition_id, kind, self.next_event_sequence);
        let root = explicit_root
            .map(|resolved| resolved.0)
            .or_else(|| parent.map(|resolved| resolved.2))
            .unwrap_or(occurrence_id);
        let root_resolution = if root == occurrence_id {
            CausalResolution::Retained
        } else if let Some(resolved) = explicit_root {
            resolved.1
        } else if self.find_event(root).is_some() {
            CausalResolution::Retained
        } else if self.compacted_references.contains_key(&root) {
            CausalResolution::Compacted
        } else {
            return Err(DiagnosticError::UnknownCausalReference(root));
        };
        let condition_id = match (kind, condition_key, existing_condition_id) {
            (DiagnosticEventKind::Incoming, Some(key), None) => Some(condition_id(
                context,
                definition_id,
                key.canonical_hash(),
                self.next_event_sequence,
            )),
            (DiagnosticEventKind::Acknowledged | DiagnosticEventKind::Cleared, _, Some(id)) => {
                Some(id)
            }
            (DiagnosticEventKind::OneShot, None, None) => None,
            _ => return Err(DiagnosticError::RuntimeInvariant),
        };
        let mut event = DiagnosticEvent {
            occurrence_id,
            condition_id,
            definition_id,
            kind,
            severity,
            condition_key,
            lifecycle_episode,
            parent_occurrence_id: parent.map(|resolved| resolved.0),
            parent_resolution: parent.map(|resolved| resolved.1),
            root_occurrence_id: root,
            root_resolution,
            universe_epoch: context.universe_epoch,
            controller_epoch: context.controller_epoch,
            session_epoch: context.session_epoch,
            event_sequence: self.next_event_sequence,
            virtual_timestamp_ms: context.virtual_timestamp_ms,
            payload_hash,
            related_identities,
            compacted_count: 0,
            compacted_first_sequence: None,
            compacted_last_sequence: None,
            compacted_first_key: None,
            compacted_last_key: None,
            compacted_references: Vec::new(),
            event_hash: Hash32::ZERO,
        };
        event.event_hash = hash_event(&event);
        Ok(event)
    }

    fn resolve_reference(
        &self,
        value: Option<OccurrenceId>,
    ) -> Result<Option<(OccurrenceId, CausalResolution, OccurrenceId)>, DiagnosticError> {
        let Some(value) = value else {
            return Ok(None);
        };
        if let Some(event) = self.find_event(value) {
            return Ok(Some((
                value,
                CausalResolution::Retained,
                event.root_occurrence_id,
            )));
        }
        self.compacted_references
            .get(&value)
            .map(|summary| {
                Some((
                    value,
                    CausalResolution::Compacted,
                    summary.root_occurrence_id,
                ))
            })
            .ok_or(DiagnosticError::UnknownCausalReference(value))
    }

    fn find_event(&self, id: OccurrenceId) -> Option<&DiagnosticEvent> {
        self.events.iter().find(|event| event.occurrence_id == id)
    }

    fn push_event(&mut self, event: DiagnosticEvent) {
        self.events.push_back(event);
    }

    fn compact(&mut self, context: ObservationContext) -> Result<(), DiagnosticError> {
        let reserved_for_gap = 1;
        if self.events.len() + usize::from(self.gap.is_some()) <= self.limits.retained_events {
            return Ok(());
        }
        let keep = self.limits.retained_events.saturating_sub(reserved_for_gap);
        let remove_count = self.events.len().saturating_sub(keep);
        let mut removed = Vec::with_capacity(remove_count);
        for _ in 0..remove_count {
            if let Some(event) = self.events.pop_front() {
                removed.push(event);
            }
        }
        if removed.is_empty() {
            return Ok(());
        }
        let first = removed.first().expect("nonempty compacted prefix");
        let last = removed.last().expect("nonempty compacted prefix");
        let prior_count = self.gap.as_ref().map_or(0, |gap| gap.compacted_count);
        let gap_definition = self
            .registry
            .by_code("EDU-DIA-0001")
            .expect("EDU-21 registry includes event gap");
        let first_key = self
            .gap
            .as_ref()
            .and_then(|gap| gap.compacted_first_key)
            .unwrap_or_else(|| diagnostic_order_key(first));
        let last_key = diagnostic_order_key(last);
        let occurrence = gap_occurrence_id(context, gap_definition.id, first_key);

        let mut available = self.compacted_references.clone();
        for event in removed {
            let definition = self
                .registry
                .definition(event.definition_id)
                .ok_or(DiagnosticError::RuntimeInvariant)?;
            available.insert(
                event.occurrence_id,
                CompactedDiagnosticReference {
                    occurrence_id: event.occurrence_id,
                    definition_id: event.definition_id,
                    code_version: definition.code_version,
                    lifecycle: event.kind,
                    primary_target_id: event.condition_key.map(|key| key.subject_identity),
                    canonical_order_key: diagnostic_order_key(&event),
                    root_occurrence_id: event.root_occurrence_id,
                },
            );
        }
        let retained_ids = self
            .events
            .iter()
            .map(|event| event.occurrence_id)
            .collect::<BTreeSet<_>>();
        let mut required = BTreeSet::new();
        for event in &self.events {
            if let Some(parent) = event.parent_occurrence_id
                && !retained_ids.contains(&parent)
            {
                required.insert(parent);
            }
            if event.root_occurrence_id != event.occurrence_id
                && !retained_ids.contains(&event.root_occurrence_id)
            {
                required.insert(event.root_occurrence_id);
            }
        }
        for condition in self.active.values() {
            if !retained_ids.contains(&condition.incoming_occurrence_id) {
                required.insert(condition.incoming_occurrence_id);
            }
            if !retained_ids.contains(&condition.root_occurrence_id) {
                required.insert(condition.root_occurrence_id);
            }
        }
        let mut summaries = required
            .into_iter()
            .map(|id| {
                available
                    .get(&id)
                    .cloned()
                    .ok_or(DiagnosticError::RuntimeInvariant)
            })
            .collect::<Result<Vec<_>, _>>()?;
        summaries.sort_by_key(|summary| (summary.canonical_order_key, summary.occurrence_id));
        self.compacted_references = summaries
            .iter()
            .cloned()
            .map(|summary| (summary.occurrence_id, summary))
            .collect();
        for event in &mut self.events {
            if event
                .parent_occurrence_id
                .is_some_and(|id| self.compacted_references.contains_key(&id))
            {
                event.parent_resolution = Some(CausalResolution::Compacted);
            }
            if self
                .compacted_references
                .contains_key(&event.root_occurrence_id)
            {
                event.root_resolution = CausalResolution::Compacted;
            }
            event.event_hash = hash_event(event);
        }
        let mut gap = DiagnosticEvent {
            occurrence_id: occurrence,
            condition_id: None,
            definition_id: gap_definition.id,
            kind: DiagnosticEventKind::Compaction,
            severity: DiagnosticSeverity::Warning,
            condition_key: None,
            lifecycle_episode: 0,
            parent_occurrence_id: None,
            parent_resolution: None,
            root_occurrence_id: occurrence,
            root_resolution: CausalResolution::Retained,
            universe_epoch: context.universe_epoch,
            controller_epoch: context.controller_epoch,
            session_epoch: context.session_epoch,
            event_sequence: 0,
            virtual_timestamp_ms: context.virtual_timestamp_ms,
            payload_hash: Hash32::ZERO,
            related_identities: Vec::new(),
            compacted_count: prior_count + remove_count as u64,
            compacted_first_sequence: Some(first_key.event_sequence),
            compacted_last_sequence: Some(last_key.event_sequence),
            compacted_first_key: Some(first_key),
            compacted_last_key: Some(last_key),
            compacted_references: summaries,
            event_hash: Hash32::ZERO,
        };
        gap.event_hash = hash_event(&gap);
        self.gap = Some(gap);
        Ok(())
    }

    fn calculate_hash(&self) -> Hash32 {
        let mut hasher = CanonicalHasher::new("PES-DIAGNOSTIC-LEDGER-1");
        hasher.hash(self.registry.registry_hash());
        hasher.u64(self.limits.ordinary_nonfatal_active as u64);
        hasher.u64(self.limits.total_active as u64);
        hasher.u64(self.limits.retained_events as u64);
        hasher.u64(self.next_event_sequence);
        hasher.u64(self.active.len() as u64);
        for condition in self.active.values() {
            hasher.hash(condition.key.canonical_hash());
            hasher.u128(condition.condition_id.0);
            hasher.u128(condition.incoming_occurrence_id.0);
            hasher.u128(condition.root_occurrence_id.0);
            hasher.u8(condition.severity as u8);
            hasher.bool(condition.acknowledgeable);
            hasher.bool(condition.acknowledged);
            hasher.u64(condition.lifecycle_episode);
            hasher.hash(condition.payload_hash);
            hasher.u64(condition.related_identities.len() as u64);
            for identity in &condition.related_identities {
                hasher.u128(*identity);
            }
        }
        hasher.u64(self.episodes.len() as u64);
        for (key, episode) in &self.episodes {
            hasher.hash(key.canonical_hash());
            hasher.u64(*episode);
        }
        match &self.gap {
            Some(gap) => {
                hasher.bool(true);
                hasher.hash(gap.event_hash);
            }
            None => hasher.bool(false),
        }
        hasher.u64(self.events.len() as u64);
        for event in &self.events {
            hasher.hash(event.event_hash);
        }
        hasher.finish()
    }

    fn validate_integrity(&self) -> Result<(), DiagnosticError> {
        if self.registry.registry_hash() != hash_registry(&self.registry.definitions) {
            return Err(DiagnosticError::RegistryIntegrityMismatch);
        }
        if let Some(gap) = &self.gap
            && gap.event_hash != hash_event(gap)
        {
            return Err(DiagnosticError::EventIntegrityMismatch(gap.occurrence_id));
        }
        for event in &self.events {
            if event.event_hash != hash_event(event) {
                return Err(DiagnosticError::EventIntegrityMismatch(event.occurrence_id));
            }
        }
        if self.ledger_hash != self.calculate_hash() {
            return Err(DiagnosticError::LedgerIntegrityMismatch);
        }
        Ok(())
    }
}

fn hash_ledger_snapshot(snapshot: &DiagnosticLedgerSnapshot) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-DIAGNOSTIC-LEDGER-SNAPSHOT-1");
    hasher.u32(snapshot.schema_version);
    hasher.u128(snapshot.universe_id);
    hasher.u64(snapshot.captured_universe_epoch);
    hasher.u128(snapshot.controller_id);
    hasher.u64(snapshot.captured_controller_epoch);
    hasher.hash(snapshot.artifact_fingerprint);
    hasher.hash(snapshot.profile_fingerprint);
    hasher.hash(snapshot.state.calculate_hash());
    hasher.finish()
}

fn diagnostic_id(code: &str) -> DiagnosticId {
    let mut hasher = CanonicalHasher::new("PES-DIAGNOSTIC-DEFINITION-ID-1");
    hasher.string(code);
    DiagnosticId(id128(hasher.finish()))
}

fn hash_registry(definitions: &BTreeMap<DiagnosticId, DiagnosticDefinition>) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-DIAGNOSTIC-REGISTRY-1");
    hasher.u64(definitions.len() as u64);
    for definition in definitions.values() {
        hasher.u128(definition.id.0);
        hasher.string(definition.code.0);
        hasher.u64(definition.code_version);
        hasher.string(definition.mnemonic);
        hasher.u8(definition.lifecycle as u8);
        hasher.u8(definition.default_severity as u8);
        hasher.bool(definition.acknowledgeable);
        hasher.u8(definition.cpu_response as u8);
        hasher.u8(definition.source as u8);
    }
    hasher.finish()
}

fn occurrence_id(
    context: ObservationContext,
    definition_id: DiagnosticId,
    kind: DiagnosticEventKind,
    event_sequence: u64,
) -> OccurrenceId {
    let name = dia_name(&[
        dia_enum("OCCURRENCE"),
        dia_uuid(context.universe_id.0),
        dia_number(context.universe_epoch),
        dia_uuid(context.controller_id.0),
        dia_number(context.controller_epoch),
        dia_uuid(definition_id.0),
        dia_enum(lifecycle_token(kind)),
        dia_number(event_sequence),
    ]);
    OccurrenceId(u128::from_be_bytes(uuid_v5(
        DIAGNOSTIC_UUID_NAMESPACE,
        &name,
    )))
}

fn condition_id(
    context: ObservationContext,
    definition_id: DiagnosticId,
    condition_key_hash: Hash32,
    incoming_event_sequence: u64,
) -> ConditionId {
    let name = dia_name(&[
        dia_enum("CONDITION"),
        dia_uuid(context.universe_id.0),
        dia_number(context.universe_epoch),
        dia_uuid(context.controller_id.0),
        dia_number(context.controller_epoch),
        dia_uuid(definition_id.0),
        dia_hash(condition_key_hash),
        dia_number(incoming_event_sequence),
    ]);
    ConditionId(u128::from_be_bytes(uuid_v5(
        DIAGNOSTIC_UUID_NAMESPACE,
        &name,
    )))
}

fn gap_occurrence_id(
    context: ObservationContext,
    gap_definition_id: DiagnosticId,
    first_lost: DiagnosticOrderKey,
) -> OccurrenceId {
    let name = dia_name(&[
        dia_enum("GAP"),
        dia_uuid(context.universe_id.0),
        dia_uuid(context.controller_id.0),
        dia_uuid(gap_definition_id.0),
        dia_number(first_lost.universe_epoch),
        dia_number(first_lost.controller_epoch),
        dia_number(first_lost.event_sequence),
        dia_uuid(first_lost.occurrence_id.0),
    ]);
    OccurrenceId(u128::from_be_bytes(uuid_v5(
        DIAGNOSTIC_UUID_NAMESPACE,
        &name,
    )))
}

fn lifecycle_token(kind: DiagnosticEventKind) -> &'static str {
    match kind {
        DiagnosticEventKind::Incoming => "INCOMING",
        DiagnosticEventKind::Acknowledged => "ACKNOWLEDGED",
        DiagnosticEventKind::Cleared => "CLEARED",
        DiagnosticEventKind::OneShot => "ONE_SHOT",
        DiagnosticEventKind::Compaction => "COMPACTION",
    }
}

fn dia_name(members: &[Vec<u8>]) -> Vec<u8> {
    let mut name = b"PES-DIA-ID-TLV-1\0".to_vec();
    name.extend_from_slice(&dia_tuple(members));
    name
}

fn dia_value(tag: u8, value: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(5 + value.len());
    output.push(tag);
    output.extend_from_slice(&(value.len() as u32).to_be_bytes());
    output.extend_from_slice(value);
    output
}

fn dia_uuid(value: u128) -> Vec<u8> {
    dia_value(b'U', &value.to_be_bytes())
}

fn dia_number(value: u64) -> Vec<u8> {
    dia_value(b'N', &value.to_be_bytes())
}

fn dia_string(value: &str) -> Vec<u8> {
    dia_value(b'S', value.as_bytes())
}

fn dia_enum(value: &str) -> Vec<u8> {
    debug_assert!(value.bytes().all(|byte| !byte.is_ascii_lowercase()));
    dia_value(b'E', value.as_bytes())
}

fn dia_hash(value: Hash32) -> Vec<u8> {
    dia_value(b'H', value.as_bytes())
}

fn dia_tuple(members: &[Vec<u8>]) -> Vec<u8> {
    let payload_len = members.iter().map(Vec::len).sum();
    let mut payload = Vec::with_capacity(payload_len);
    for member in members {
        payload.extend_from_slice(member);
    }
    dia_value(b'T', &payload)
}

fn dia_null() -> Vec<u8> {
    dia_value(b'Z', &[])
}

fn uuid_v5(namespace: [u8; 16], name: &[u8]) -> [u8; 16] {
    let mut bytes = Vec::with_capacity(namespace.len() + name.len());
    bytes.extend_from_slice(&namespace);
    bytes.extend_from_slice(name);
    let digest = sha1(&bytes);
    let mut uuid: [u8; 16] = digest[..16]
        .try_into()
        .expect("SHA-1 contains sixteen UUID bytes");
    uuid[6] = (uuid[6] & 0x0f) | 0x50;
    uuid[8] = (uuid[8] & 0x3f) | 0x80;
    uuid
}

fn sha1(input: &[u8]) -> [u8; 20] {
    let mut message = input.to_vec();
    let bit_len = (message.len() as u64).saturating_mul(8);
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());
    let mut state = [
        0x67452301_u32,
        0xefcdab89,
        0x98badcfe,
        0x10325476,
        0xc3d2e1f0,
    ];
    for block in message.chunks_exact(64) {
        let mut words = [0_u32; 80];
        for (index, word) in words.iter_mut().take(16).enumerate() {
            *word = u32::from_be_bytes(
                block[index * 4..index * 4 + 4]
                    .try_into()
                    .expect("SHA-1 word"),
            );
        }
        for index in 16..80 {
            words[index] =
                (words[index - 3] ^ words[index - 8] ^ words[index - 14] ^ words[index - 16])
                    .rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = state;
        for (index, word) in words.iter().enumerate() {
            let (function, constant) = match index {
                0..=19 => ((b & c) | ((!b) & d), 0x5a827999),
                20..=39 => (b ^ c ^ d, 0x6ed9eba1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8f1bbcdc),
                _ => (b ^ c ^ d, 0xca62c1d6),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(function)
                .wrapping_add(e)
                .wrapping_add(constant)
                .wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
    }
    let mut output = [0_u8; 20];
    for (index, word) in state.iter().enumerate() {
        output[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    output
}

fn hash_event(event: &DiagnosticEvent) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-DIAGNOSTIC-EVENT-1");
    hasher.u128(event.occurrence_id.0);
    match event.condition_id {
        Some(id) => {
            hasher.bool(true);
            hasher.u128(id.0);
        }
        None => hasher.bool(false),
    }
    hasher.u128(event.definition_id.0);
    hasher.u8(event.kind as u8);
    hasher.u8(event.severity as u8);
    match event.condition_key {
        Some(key) => {
            hasher.bool(true);
            hasher.hash(key.canonical_hash());
        }
        None => hasher.bool(false),
    }
    hasher.u64(event.lifecycle_episode);
    match event.parent_occurrence_id {
        Some(id) => {
            hasher.bool(true);
            hasher.u128(id.0);
        }
        None => hasher.bool(false),
    }
    match event.parent_resolution {
        Some(resolution) => {
            hasher.bool(true);
            hasher.u8(resolution as u8);
        }
        None => hasher.bool(false),
    }
    hasher.u128(event.root_occurrence_id.0);
    hasher.u8(event.root_resolution as u8);
    hasher.u64(event.universe_epoch);
    hasher.u64(event.controller_epoch);
    hasher.u64(event.session_epoch);
    hasher.u64(event.event_sequence);
    hasher.u64(event.virtual_timestamp_ms);
    hasher.hash(event.payload_hash);
    hasher.u64(event.related_identities.len() as u64);
    for id in &event.related_identities {
        hasher.u128(*id);
    }
    hasher.u64(event.compacted_count);
    encode_optional_u64(event.compacted_first_sequence, &mut hasher);
    encode_optional_u64(event.compacted_last_sequence, &mut hasher);
    encode_optional_order_key(event.compacted_first_key, &mut hasher);
    encode_optional_order_key(event.compacted_last_key, &mut hasher);
    hasher.u64(event.compacted_references.len() as u64);
    for summary in &event.compacted_references {
        encode_compacted_reference(summary, &mut hasher);
    }
    hasher.finish()
}

fn diagnostic_order_key(event: &DiagnosticEvent) -> DiagnosticOrderKey {
    DiagnosticOrderKey {
        universe_epoch: event.universe_epoch,
        controller_epoch: event.controller_epoch,
        event_sequence: event.event_sequence,
        occurrence_id: event.occurrence_id,
    }
}

fn encode_order_key(key: DiagnosticOrderKey, hasher: &mut CanonicalHasher) {
    hasher.u64(key.universe_epoch);
    hasher.u64(key.controller_epoch);
    hasher.u64(key.event_sequence);
    hasher.u128(key.occurrence_id.0);
}

fn encode_optional_order_key(value: Option<DiagnosticOrderKey>, hasher: &mut CanonicalHasher) {
    match value {
        Some(value) => {
            hasher.bool(true);
            encode_order_key(value, hasher);
        }
        None => hasher.bool(false),
    }
}

fn encode_compacted_reference(
    summary: &CompactedDiagnosticReference,
    hasher: &mut CanonicalHasher,
) {
    hasher.u128(summary.occurrence_id.0);
    hasher.u128(summary.definition_id.0);
    hasher.u64(summary.code_version);
    hasher.u8(summary.lifecycle as u8);
    match summary.primary_target_id {
        Some(value) => {
            hasher.bool(true);
            hasher.u128(value);
        }
        None => hasher.bool(false),
    }
    encode_order_key(summary.canonical_order_key, hasher);
    hasher.u128(summary.root_occurrence_id.0);
}

fn encode_optional_u64(value: Option<u64>, hasher: &mut CanonicalHasher) {
    match value {
        Some(value) => {
            hasher.bool(true);
            hasher.u64(value);
        }
        None => hasher.bool(false),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiagnosticError {
    InvalidLimits,
    UnknownDefinition(DiagnosticId),
    UnknownCondition(ConditionKey),
    UnknownCausalReference(OccurrenceId),
    LifecycleMismatch,
    NotAcknowledgeable(ConditionKey),
    AlreadyAcknowledged(ConditionKey),
    CpuStateDisallowed(plc_runtime::CpuState),
    WrongPublicationBoundary,
    CapacityRejected {
        proposed_nonfatal: usize,
        proposed_total: usize,
        proposed_keys: Vec<ConditionKey>,
    },
    FatalCapacityExhausted,
    RegistryIntegrityMismatch,
    EventIntegrityMismatch(OccurrenceId),
    LedgerIntegrityMismatch,
    SnapshotIntegrityMismatch,
    SnapshotBindingMismatch,
    RuntimeInvariant,
}

impl fmt::Display for DiagnosticError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "diagnostic transition rejected: {self:?}")
    }
}

impl Error for DiagnosticError {}

#[cfg(test)]
mod sha1_vectors {
    use super::sha1;

    #[test]
    fn sha1_known_vector() {
        assert_eq!(
            sha1(b"abc"),
            [
                0xa9, 0x99, 0x3e, 0x36, 0x47, 0x06, 0x81, 0x6a, 0xba, 0x3e, 0x25, 0x71, 0x78, 0x50,
                0xc2, 0x6c, 0x9c, 0xd0, 0xd8, 0x9d
            ]
        );
    }
}
