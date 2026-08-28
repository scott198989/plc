use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet, VecDeque},
    string::{String, ToString},
    vec::Vec,
};
use core::{error::Error, fmt};

use plc_commissioning::CommissionedScanReceipt;
use plc_runtime::{CanonicalValue, CpuState, Hash32, RunOutcome, SCAN_QUANTUM_MS};
use plc_types::{CanonicalF32, CanonicalF64};

use crate::{
    DiagnosticEvent, DiagnosticEventKind, DiagnosticId, DiagnosticRegistry, ForceProvenance,
    ObservationContext, OccurrenceId, ProbeCatalog, ProbeLayer, PublishedTargetValue, Quality,
    SampleFreshness, StableTargetId, TargetReference, ValueType, canonical::CanonicalHasher,
    monitor::published_layer_value,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TraceConfigId(pub u128);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TraceCaptureId(pub u128);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TraceChannelId(pub u128);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TraceTriggerId(pub u128);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TraceSavedResultId(pub u128);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TraceProbeKind {
    LoadedTarget {
        target: TargetReference,
        layer: ProbeLayer,
    },
    ScanQuantumMs,
    ScanWorkUnits,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum TraceProbeIdentity {
    LoadedTarget(StableTargetId) = 1,
    ScanQuantumMs = 2,
    ScanWorkUnits = 3,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceChannel {
    pub id: TraceChannelId,
    pub alias: String,
    pub probe: TraceProbeKind,
    pub display_unit: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraceRuntimePublication {
    universe_id: u128,
    universe_epoch: u64,
    controller_id: u128,
    controller_epoch: u64,
    artifact_fingerprint: Hash32,
    scan_sequence: u64,
    event_sequence: u64,
    virtual_timestamp_ms: u64,
    scan_quantum_ms: u64,
    scan_work_units: u32,
    controller_state_hash: Hash32,
}

impl TraceRuntimePublication {
    pub fn from_commissioned_scan(
        context: ObservationContext,
        receipt: &CommissionedScanReceipt,
    ) -> Result<Self, TraceError> {
        let RunOutcome::Completed(report) = &receipt.runtime.outcome else {
            return Err(TraceError::RuntimeMetricUnavailable);
        };
        if context.cpu_state != CpuState::Run
            || context.publication_boundary != crate::PublicationBoundary::ScanEnd
            || context.scan_sequence != report.scan_sequence
            || context.event_sequence != report.output_event_sequence
            || context.virtual_timestamp_ms != report.completed_time_ms
            || context.target_state_hash != receipt.controller_state_hash
        {
            return Err(TraceError::RuntimeMetricBindingMismatch);
        }
        Ok(Self {
            universe_id: context.universe_id.0,
            universe_epoch: context.universe_epoch,
            controller_id: context.controller_id.0,
            controller_epoch: context.controller_epoch,
            artifact_fingerprint: context.artifact_fingerprint,
            scan_sequence: report.scan_sequence,
            event_sequence: report.output_event_sequence,
            virtual_timestamp_ms: report.completed_time_ms,
            scan_quantum_ms: SCAN_QUANTUM_MS,
            scan_work_units: report.work_units,
            controller_state_hash: receipt.controller_state_hash,
        })
    }

    pub const fn scan_quantum_ms(self) -> u64 {
        self.scan_quantum_ms
    }

    pub const fn scan_work_units(self) -> u32 {
        self.scan_work_units
    }

    pub const fn scan_sequence(self) -> u64 {
        self.scan_sequence
    }

    pub const fn event_sequence(self) -> u64 {
        self.event_sequence
    }

    fn validate_context(self, context: ObservationContext) -> Result<(), TraceError> {
        if self.universe_id != context.universe_id.0
            || self.universe_epoch != context.universe_epoch
            || self.controller_id != context.controller_id.0
            || self.controller_epoch != context.controller_epoch
            || self.artifact_fingerprint != context.artifact_fingerprint
            || self.scan_sequence != context.scan_sequence
            || self.event_sequence != context.event_sequence
            || self.virtual_timestamp_ms != context.virtual_timestamp_ms
            || self.scan_quantum_ms != SCAN_QUANTUM_MS
            || self.controller_state_hash != context.target_state_hash
        {
            return Err(TraceError::RuntimeMetricBindingMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericValue {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    TimeMs(i64),
    F32(CanonicalF32),
    F64(CanonicalF64),
}

impl NumericValue {
    fn from_canonical(value: CanonicalValue) -> Option<Self> {
        match value {
            CanonicalValue::I8(value) => Some(Self::I8(value)),
            CanonicalValue::I16(value) => Some(Self::I16(value)),
            CanonicalValue::I32(value) => Some(Self::I32(value)),
            CanonicalValue::I64(value) => Some(Self::I64(value)),
            CanonicalValue::U8(value) => Some(Self::U8(value)),
            CanonicalValue::U16(value) => Some(Self::U16(value)),
            CanonicalValue::U32(value) => Some(Self::U32(value)),
            CanonicalValue::U64(value) => Some(Self::U64(value)),
            CanonicalValue::TimeMs(value) => Some(Self::TimeMs(value)),
            CanonicalValue::F32(value) => Some(Self::F32(value)),
            CanonicalValue::F64(value) => Some(Self::F64(value)),
            CanonicalValue::Bool(_)
            | CanonicalValue::Bits8(_)
            | CanonicalValue::Bits16(_)
            | CanonicalValue::Bits32(_)
            | CanonicalValue::Bits64(_)
            | CanonicalValue::Char(_) => None,
        }
    }

    const fn value_type(self) -> ValueType {
        match self {
            Self::I8(_) => ValueType::I8,
            Self::I16(_) => ValueType::I16,
            Self::I32(_) => ValueType::I32,
            Self::I64(_) => ValueType::I64,
            Self::U8(_) => ValueType::U8,
            Self::U16(_) => ValueType::U16,
            Self::U32(_) => ValueType::U32,
            Self::U64(_) => ValueType::U64,
            Self::TimeMs(_) => ValueType::TimeMs,
            Self::F32(_) => ValueType::F32,
            Self::F64(_) => ValueType::F64,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ComparisonOperator {
    Equal = 1,
    NotEqual = 2,
    Less = 3,
    LessOrEqual = 4,
    Greater = 5,
    GreaterOrEqual = 6,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExpressionNode {
    BooleanChannel(TraceChannelId),
    NumericComparison {
        channel: TraceChannelId,
        operator: ComparisonOperator,
        threshold: NumericValue,
    },
    Not(Box<ExpressionNode>),
    All(Vec<ExpressionNode>),
    Any(Vec<ExpressionNode>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiagnosticEventTrigger {
    pub definition_id: DiagnosticId,
    pub code_version: u64,
    pub lifecycle: DiagnosticEventKind,
    pub primary_target_id: Option<StableTargetId>,
    pub root_occurrence_id: Option<OccurrenceId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TraceEventKey {
    pub universe_epoch: u64,
    pub event_sequence: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraceDiagnosticEvent {
    pub definition_id: DiagnosticId,
    pub code_version: u64,
    pub lifecycle: DiagnosticEventKind,
    pub primary_target_id: Option<StableTargetId>,
    pub root_occurrence_id: OccurrenceId,
    pub occurrence_id: OccurrenceId,
    pub key: TraceEventKey,
}

impl TraceDiagnosticEvent {
    pub fn from_authoritative(
        event: &DiagnosticEvent,
        registry: &DiagnosticRegistry,
    ) -> Option<Self> {
        let definition = registry.definition(event.definition_id)?;
        Some(Self {
            definition_id: event.definition_id,
            code_version: definition.code_version,
            lifecycle: event.kind,
            primary_target_id: event
                .condition_key
                .map(|key| StableTargetId(key.subject_identity)),
            root_occurrence_id: event.root_occurrence_id,
            occurrence_id: event.occurrence_id,
            key: TraceEventKey {
                universe_epoch: event.universe_epoch,
                event_sequence: event.event_sequence,
            },
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TraceTrigger {
    Immediate,
    BooleanRising(TraceChannelId),
    BooleanFalling(TraceChannelId),
    NumericCrossing {
        channel: TraceChannelId,
        operator: ComparisonOperator,
        threshold: NumericValue,
    },
    Expression(ExpressionNode),
    DiagnosticEvent(DiagnosticEventTrigger),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraceCadence {
    EveryScans(u32),
    VirtualIntervalMs(u64),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceConfig {
    pub id: TraceConfigId,
    pub trigger_id: TraceTriggerId,
    pub name: String,
    pub channels: Vec<TraceChannel>,
    pub cadence: TraceCadence,
    pub trigger: TraceTrigger,
    pub pre_trigger_samples: usize,
    pub post_trigger_samples: usize,
    pub post_trigger_duration_ms: Option<u64>,
    pub maximum_duration_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraceLimits {
    pub configurations_per_project: usize,
    pub channels_per_configuration: usize,
    pub samples_per_capture: usize,
    pub concurrent_captures_per_controller: usize,
    pub minimum_virtual_cadence_ms: u64,
    pub maximum_virtual_duration_ms: u64,
    pub trigger_depth: usize,
    pub trigger_nodes: usize,
}

impl TraceLimits {
    pub const fn edu21() -> Self {
        Self {
            configurations_per_project: 64,
            channels_per_configuration: 64,
            samples_per_capture: 1_000_000,
            concurrent_captures_per_controller: 4,
            minimum_virtual_cadence_ms: 10,
            maximum_virtual_duration_ms: 24 * 60 * 60 * 1_000,
            trigger_depth: 32,
            trigger_nodes: 256,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TraceState {
    Idle = 1,
    Validating = 2,
    Armed = 3,
    Triggered = 4,
    Completed = 5,
    Aborted = 6,
    Error = 7,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum GapReason {
    PublicationMissing = 1,
    QualityUnavailable = 2,
    BufferCompaction = 3,
    ReplayDiscontinuity = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TraceAbortReason {
    User = 1,
    SessionLost = 2,
    ControllerEpochChanged = 3,
    ArtifactChanged = 4,
    CpuReset = 5,
    CpuFault = 6,
    DurationLimit = 7,
    BufferLimit = 8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceSample {
    pub sample_ordinal: u64,
    pub virtual_timestamp_ms: u64,
    pub scan_sequence: u64,
    pub event_sequence: u64,
    pub boundary: crate::PublicationBoundary,
    pub values: Vec<Option<CanonicalValue>>,
    pub channel_values: Vec<TraceChannelSample>,
    pub diagnostic_occurrence_ids: Vec<OccurrenceId>,
    pub gap: Option<GapReason>,
    pub sample_hash: Hash32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraceChannelSample {
    pub channel_id: TraceChannelId,
    pub probe_identity: TraceProbeIdentity,
    pub target_id: Option<StableTargetId>,
    pub value_type: ValueType,
    pub layer: ProbeLayer,
    pub value: Option<CanonicalValue>,
    pub quality: Option<Quality>,
    pub freshness: SampleFreshness,
    pub force: Option<ForceProvenance>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceCapture {
    pub schema_version: u32,
    pub id: TraceCaptureId,
    pub config_id: TraceConfigId,
    pub config_hash: Hash32,
    pub artifact_fingerprint: Hash32,
    pub profile_fingerprint: Hash32,
    pub universe_id: u128,
    pub universe_epoch: u64,
    pub controller_id: u128,
    pub controller_epoch: u64,
    pub session_epoch: u64,
    pub trigger_sample_ordinal: Option<u64>,
    pub trigger_virtual_timestamp_ms: Option<u64>,
    pub armed_event_key: TraceEventKey,
    pub live_comparison_baseline: TraceEventKey,
    pub matched_occurrence_id: Option<OccurrenceId>,
    pub trigger_boundary: Option<crate::PublicationBoundary>,
    pub samples: Vec<TraceSample>,
    pub aborted: Option<TraceAbortReason>,
    pub content_hash: Hash32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TraceExportFormat {
    CanonicalJson = 1,
    Csv = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraceExportOptions {
    pub format: TraceExportFormat,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceExport {
    pub format: TraceExportFormat,
    pub media_type: &'static str,
    pub bytes: Vec<u8>,
    pub content_hash: Hash32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveTraceResultCommand {
    pub command_id: u128,
    pub idempotency_key: u128,
    pub result_id: TraceSavedResultId,
    pub capture_id: TraceCaptureId,
    pub actor_identity: u128,
    pub audit_context_hash: Hash32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceSavedResult {
    pub schema_version: u32,
    pub id: TraceSavedResultId,
    pub command_id: u128,
    pub capture_id: TraceCaptureId,
    pub config_id: TraceConfigId,
    pub config_hash: Hash32,
    pub capture_content_hash: Hash32,
    pub artifact_fingerprint: Hash32,
    pub universe_id: u128,
    pub universe_epoch: u64,
    pub controller_id: u128,
    pub controller_epoch: u64,
    pub saved_virtual_timestamp_ms: u64,
    pub saved_event_sequence: u64,
    pub actor_identity: u128,
    pub audit_context_hash: Hash32,
    pub capture: TraceCapture,
    pub content_hash: Hash32,
}

impl TraceSavedResult {
    pub fn verify(&self) -> bool {
        self.schema_version == 1
            && self.capture.verify()
            && self.capture.content_hash == self.capture_content_hash
            && self.content_hash == hash_saved_result(self)
    }
}

#[derive(Clone, Debug)]
pub struct TraceEngineSnapshot {
    pub schema_version: u32,
    pub universe_id: u128,
    pub captured_universe_epoch: u64,
    pub controller_id: u128,
    pub captured_controller_epoch: u64,
    pub artifact_fingerprint: Hash32,
    pub profile_fingerprint: Hash32,
    state: TraceEngine,
    pub content_hash: Hash32,
}

impl TraceEngineSnapshot {
    pub fn verify(&self) -> bool {
        self.schema_version == 1 && self.content_hash == hash_engine_snapshot(self)
    }
}

impl TraceCapture {
    pub fn verify(&self) -> bool {
        self.schema_version == 1 && self.content_hash == hash_capture(self)
    }

    pub fn replay_hash(&self) -> Result<Hash32, TraceError> {
        if !self.verify() {
            return Err(TraceError::CaptureIntegrityMismatch);
        }
        let mut hasher = CanonicalHasher::new("PES-TRACE-REPLAY-1");
        hasher.hash(self.content_hash);
        for sample in &self.samples {
            if sample.sample_hash != hash_sample(sample) {
                return Err(TraceError::SampleIntegrityMismatch(sample.sample_ordinal));
            }
            hasher.hash(sample.sample_hash);
        }
        Ok(hasher.finish())
    }

    pub fn export(&self, options: TraceExportOptions) -> Result<TraceExport, TraceError> {
        if !self.verify() {
            return Err(TraceError::CaptureIntegrityMismatch);
        }
        for sample in &self.samples {
            if sample.sample_hash != hash_sample(sample) {
                return Err(TraceError::SampleIntegrityMismatch(sample.sample_ordinal));
            }
        }
        let bytes = match options.format {
            TraceExportFormat::CanonicalJson => export_json(self).into_bytes(),
            TraceExportFormat::Csv => export_csv(self).into_bytes(),
        };
        let content_hash = plc_runtime::Sha256::digest(&bytes);
        Ok(TraceExport {
            format: options.format,
            media_type: match options.format {
                TraceExportFormat::CanonicalJson => "application/json",
                TraceExportFormat::Csv => "text/csv",
            },
            bytes,
            content_hash,
        })
    }
}

#[derive(Clone, Debug)]
struct ActiveTrace {
    state: TraceState,
    context: ObservationContext,
    config: TraceConfig,
    config_hash: Hash32,
    resolved_channels: Vec<ResolvedTraceChannel>,
    previous_values: BTreeMap<TraceChannelId, CanonicalValue>,
    pre_buffer: VecDeque<TraceSample>,
    captured: Vec<TraceSample>,
    last_sample_virtual_ms: Option<u64>,
    next_due_virtual_ms: Option<u64>,
    trigger_sample_ordinal: Option<u64>,
    trigger_virtual_timestamp_ms: Option<u64>,
    armed_event_key: TraceEventKey,
    live_comparison_baseline: TraceEventKey,
    matched_occurrence_id: Option<OccurrenceId>,
    trigger_boundary: Option<crate::PublicationBoundary>,
    next_sample_ordinal: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ResolvedTraceChannel {
    channel_id: TraceChannelId,
    probe_identity: TraceProbeIdentity,
    value_type: ValueType,
    layer: ProbeLayer,
}

#[derive(Clone, Debug)]
pub struct TraceEngine {
    limits: TraceLimits,
    configs: BTreeMap<TraceConfigId, TraceConfig>,
    active: BTreeMap<TraceConfigId, ActiveTrace>,
    captures: BTreeMap<TraceCaptureId, TraceCapture>,
    terminal_states: BTreeMap<TraceConfigId, TraceState>,
    saved_results: BTreeMap<TraceSavedResultId, TraceSavedResult>,
    save_receipts: BTreeMap<u128, (Hash32, TraceSavedResultId)>,
}

impl TraceEngine {
    pub fn new(limits: TraceLimits) -> Result<Self, TraceError> {
        if limits.configurations_per_project == 0
            || limits.channels_per_configuration == 0
            || limits.samples_per_capture == 0
            || limits.concurrent_captures_per_controller == 0
            || limits.minimum_virtual_cadence_ms == 0
            || limits.maximum_virtual_duration_ms == 0
            || limits.trigger_depth == 0
            || limits.trigger_nodes == 0
        {
            return Err(TraceError::InvalidLimits);
        }
        Ok(Self {
            limits,
            configs: BTreeMap::new(),
            active: BTreeMap::new(),
            captures: BTreeMap::new(),
            terminal_states: BTreeMap::new(),
            saved_results: BTreeMap::new(),
            save_receipts: BTreeMap::new(),
        })
    }

    pub fn upsert_config(&mut self, config: TraceConfig) -> Result<(), TraceError> {
        if self.active.contains_key(&config.id) {
            return Err(TraceError::ConfigurationActive(config.id));
        }
        if !self.configs.contains_key(&config.id)
            && self.configs.len() == self.limits.configurations_per_project
        {
            return Err(TraceError::ConfigurationLimitExceeded);
        }
        validate_config(&config, self.limits)?;
        self.configs.insert(config.id, config);
        Ok(())
    }

    pub fn state(&self, id: TraceConfigId) -> TraceState {
        self.active.get(&id).map_or_else(
            || {
                self.terminal_states
                    .get(&id)
                    .copied()
                    .unwrap_or(TraceState::Idle)
            },
            |trace| trace.state,
        )
    }

    pub fn arm(
        &mut self,
        id: TraceConfigId,
        context: ObservationContext,
        catalog: &ProbeCatalog,
    ) -> Result<(), TraceError> {
        self.arm_inner(id, context, catalog, None)
    }

    pub fn arm_with_diagnostics(
        &mut self,
        id: TraceConfigId,
        context: ObservationContext,
        catalog: &ProbeCatalog,
        registry: &DiagnosticRegistry,
    ) -> Result<(), TraceError> {
        self.arm_inner(id, context, catalog, Some(registry))
    }

    fn arm_inner(
        &mut self,
        id: TraceConfigId,
        context: ObservationContext,
        catalog: &ProbeCatalog,
        diagnostic_registry: Option<&DiagnosticRegistry>,
    ) -> Result<(), TraceError> {
        if self.active.len() == self.limits.concurrent_captures_per_controller {
            return Err(TraceError::ConcurrentCaptureLimitExceeded);
        }
        if self.active.contains_key(&id) {
            return Err(TraceError::IllegalTransition {
                from: self.state(id),
                action: "Arm",
            });
        }
        if matches!(
            context.cpu_state,
            CpuState::PoweredOff | CpuState::Startup | CpuState::Resetting | CpuState::Faulted
        ) {
            return Err(TraceError::CpuStateDisallowed(context.cpu_state));
        }
        let config = self
            .configs
            .get(&id)
            .cloned()
            .ok_or(TraceError::UnknownConfiguration(id))?;
        let mut resolved_channels = Vec::with_capacity(config.channels.len());
        let mut unique = BTreeSet::new();
        let mut channel_identity = BTreeSet::new();
        let mut aliases = BTreeSet::new();
        for channel in &config.channels {
            if channel.alias.is_empty()
                || !channel.alias.is_ascii()
                || !channel_identity.insert(channel.id)
                || !aliases.insert(channel.alias.to_ascii_lowercase())
            {
                return Err(TraceError::DuplicateOrInvalidChannelIdentity);
            }
            let resolved = resolve_trace_channel(channel, context, catalog)?;
            if !unique.insert((resolved.probe_identity, resolved.layer)) {
                return Err(TraceError::DuplicateProbe(resolved.probe_identity));
            }
            resolved_channels.push(resolved);
        }
        validate_trigger_channels(&config.trigger, &channel_identity)?;
        validate_trigger_types(&config.trigger, &resolved_channels)?;
        validate_diagnostic_trigger(&config.trigger, diagnostic_registry, catalog, context)?;
        let config_hash = hash_config(&config);
        let armed_event_key = TraceEventKey {
            universe_epoch: context.universe_epoch,
            event_sequence: context.event_sequence,
        };
        self.active.insert(
            id,
            ActiveTrace {
                state: TraceState::Armed,
                context,
                config,
                config_hash,
                resolved_channels,
                previous_values: BTreeMap::new(),
                pre_buffer: VecDeque::new(),
                captured: Vec::new(),
                last_sample_virtual_ms: None,
                next_due_virtual_ms: None,
                trigger_sample_ordinal: None,
                trigger_virtual_timestamp_ms: None,
                armed_event_key,
                live_comparison_baseline: armed_event_key,
                matched_occurrence_id: None,
                trigger_boundary: None,
                next_sample_ordinal: 0,
            },
        );
        self.terminal_states.remove(&id);
        Ok(())
    }

    pub fn publish(
        &mut self,
        context: ObservationContext,
        values: &[PublishedTargetValue],
        diagnostic_events: &[TraceDiagnosticEvent],
    ) -> Result<Vec<TraceCaptureId>, TraceError> {
        self.publish_inner(context, values, diagnostic_events, None)
    }

    pub fn publish_with_runtime(
        &mut self,
        context: ObservationContext,
        values: &[PublishedTargetValue],
        diagnostic_events: &[TraceDiagnosticEvent],
        runtime_publication: &TraceRuntimePublication,
    ) -> Result<Vec<TraceCaptureId>, TraceError> {
        runtime_publication.validate_context(context)?;
        self.publish_inner(
            context,
            values,
            diagnostic_events,
            Some(runtime_publication),
        )
    }

    fn publish_inner(
        &mut self,
        context: ObservationContext,
        values: &[PublishedTargetValue],
        diagnostic_events: &[TraceDiagnosticEvent],
        runtime_publication: Option<&TraceRuntimePublication>,
    ) -> Result<Vec<TraceCaptureId>, TraceError> {
        let ids = self.active.keys().copied().collect::<Vec<_>>();
        let mut completed = Vec::new();
        for id in ids {
            let stale_reason = {
                let trace = self
                    .active
                    .get(&id)
                    .expect("active trace identity is stable");
                if trace.context.universe_epoch != context.universe_epoch
                    || trace.context.controller_id != context.controller_id
                    || trace.context.session_epoch != context.session_epoch
                {
                    Some(TraceAbortReason::SessionLost)
                } else if trace.context.controller_epoch != context.controller_epoch {
                    Some(TraceAbortReason::ControllerEpochChanged)
                } else if trace.context.artifact_fingerprint != context.artifact_fingerprint {
                    Some(TraceAbortReason::ArtifactChanged)
                } else {
                    None
                }
            };
            if let Some(reason) = stale_reason {
                completed.push(self.abort_internal(id, reason)?);
                continue;
            }
            match context.cpu_state {
                CpuState::Stop | CpuState::PausedEducational => continue,
                CpuState::Startup => continue,
                CpuState::Resetting => {
                    completed.push(self.abort_internal(id, TraceAbortReason::CpuReset)?);
                    continue;
                }
                CpuState::Faulted => {
                    completed.push(self.abort_internal(id, TraceAbortReason::CpuFault)?);
                    continue;
                }
                CpuState::PoweredOff => {
                    completed.push(self.abort_internal(id, TraceAbortReason::SessionLost)?);
                    continue;
                }
                CpuState::Run => {}
            }
            let (should_sample, skipped_diagnostic_baseline) = {
                let trace = self
                    .active
                    .get(&id)
                    .expect("active trace identity is stable");
                let diagnostic = diagnostic_batch_match(
                    &trace.config.trigger,
                    diagnostic_events,
                    trace.live_comparison_baseline,
                );
                (
                    cadence_due(trace, context)
                        || diagnostic.is_some_and(|(matched, _)| matched.is_some()),
                    diagnostic.and_then(|(_, latest)| latest),
                )
            };
            if !should_sample {
                if let Some(latest) = skipped_diagnostic_baseline {
                    let trace = self
                        .active
                        .get_mut(&id)
                        .expect("active trace identity is stable");
                    if latest > trace.live_comparison_baseline {
                        trace.live_comparison_baseline = latest;
                    }
                }
                continue;
            }
            if runtime_publication.is_none()
                && self.active.get(&id).is_some_and(|trace| {
                    trace.resolved_channels.iter().any(|channel| {
                        !matches!(channel.probe_identity, TraceProbeIdentity::LoadedTarget(_))
                    })
                })
            {
                return Err(TraceError::RuntimeMetricPublicationRequired);
            }
            let publications = values
                .iter()
                .map(|value| (value.target_id, *value))
                .collect::<BTreeMap<_, _>>();
            let finished = self.sample_trace(
                id,
                context,
                &publications,
                diagnostic_events,
                runtime_publication,
            )?;
            if let Some(capture_id) = finished {
                completed.push(capture_id);
            }
        }
        Ok(completed)
    }

    pub fn abort(
        &mut self,
        id: TraceConfigId,
        reason: TraceAbortReason,
    ) -> Result<TraceCaptureId, TraceError> {
        self.abort_internal(id, reason)
    }

    pub fn capture(&self, id: TraceCaptureId) -> Option<&TraceCapture> {
        self.captures.get(&id)
    }

    pub fn save_result(
        &mut self,
        command: SaveTraceResultCommand,
        context: ObservationContext,
    ) -> Result<TraceSavedResult, TraceError> {
        let payload_hash = hash_save_command(command);
        if let Some((stored_hash, result_id)) = self.save_receipts.get(&command.idempotency_key) {
            if *stored_hash != payload_hash {
                return Err(TraceError::SaveIdempotencyCollision);
            }
            return self
                .saved_results
                .get(result_id)
                .cloned()
                .ok_or(TraceError::RuntimeInvariant);
        }
        if self.saved_results.contains_key(&command.result_id) {
            return Err(TraceError::DuplicateSavedResult(command.result_id));
        }
        let capture = self
            .captures
            .get(&command.capture_id)
            .cloned()
            .ok_or(TraceError::UnknownCapture(command.capture_id))?;
        if !capture.verify() {
            return Err(TraceError::CaptureIntegrityMismatch);
        }
        if capture.universe_id != context.universe_id.0
            || capture.controller_id != context.controller_id.0
        {
            return Err(TraceError::SaveBindingMismatch);
        }
        let mut result = TraceSavedResult {
            schema_version: 1,
            id: command.result_id,
            command_id: command.command_id,
            capture_id: capture.id,
            config_id: capture.config_id,
            config_hash: capture.config_hash,
            capture_content_hash: capture.content_hash,
            artifact_fingerprint: capture.artifact_fingerprint,
            universe_id: capture.universe_id,
            universe_epoch: capture.universe_epoch,
            controller_id: capture.controller_id,
            controller_epoch: capture.controller_epoch,
            saved_virtual_timestamp_ms: context.virtual_timestamp_ms,
            saved_event_sequence: context.event_sequence,
            actor_identity: command.actor_identity,
            audit_context_hash: command.audit_context_hash,
            capture,
            content_hash: Hash32::ZERO,
        };
        result.content_hash = hash_saved_result(&result);
        self.saved_results.insert(command.result_id, result.clone());
        self.save_receipts
            .insert(command.idempotency_key, (payload_hash, command.result_id));
        Ok(result)
    }

    pub fn saved_result(&self, id: TraceSavedResultId) -> Option<&TraceSavedResult> {
        self.saved_results.get(&id)
    }

    pub fn capture_snapshot(&self, context: ObservationContext) -> TraceEngineSnapshot {
        let mut snapshot = TraceEngineSnapshot {
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
        snapshot.content_hash = hash_engine_snapshot(&snapshot);
        snapshot
    }

    pub fn restore_snapshot(
        snapshot: &TraceEngineSnapshot,
        context: ObservationContext,
        catalog: &ProbeCatalog,
        restore_event_key: TraceEventKey,
    ) -> Result<Self, TraceError> {
        if !snapshot.verify() {
            return Err(TraceError::SnapshotIntegrityMismatch);
        }
        if snapshot.universe_id != context.universe_id.0
            || snapshot.controller_id != context.controller_id.0
            || snapshot.artifact_fingerprint != context.artifact_fingerprint
            || snapshot.profile_fingerprint != context.profile_fingerprint
            || context.universe_epoch <= snapshot.captured_universe_epoch
            || context.controller_epoch <= snapshot.captured_controller_epoch
            || restore_event_key.universe_epoch != context.universe_epoch
            || restore_event_key.event_sequence != context.event_sequence
        {
            return Err(TraceError::SnapshotBindingMismatch);
        }
        if !matches!(
            context.cpu_state,
            CpuState::Stop | CpuState::PausedEducational
        ) {
            return Err(TraceError::SnapshotRestoreStateDisallowed(
                context.cpu_state,
            ));
        }
        let mut restored = snapshot.state.clone();
        for trace in restored.active.values_mut() {
            if trace.context.universe_id != context.universe_id
                || trace.context.controller_id != context.controller_id
                || trace.context.artifact_fingerprint != context.artifact_fingerprint
                || trace.context.profile_fingerprint != context.profile_fingerprint
                || trace.config_hash != hash_config(&trace.config)
            {
                return Err(TraceError::SnapshotBindingMismatch);
            }
            let mut resolved_channels = Vec::with_capacity(trace.config.channels.len());
            for channel in &trace.config.channels {
                resolved_channels.push(
                    resolve_trace_channel(channel, context, catalog)
                        .map_err(|_| TraceError::SnapshotTargetMismatch)?,
                );
            }
            if resolved_channels != trace.resolved_channels {
                return Err(TraceError::SnapshotTargetMismatch);
            }
            trace.context = context;
            if matches!(trace.config.trigger, TraceTrigger::DiagnosticEvent(_)) {
                trace.live_comparison_baseline = restore_event_key;
            }
        }
        for capture in restored.captures.values() {
            if !capture.verify() {
                return Err(TraceError::CaptureIntegrityMismatch);
            }
        }
        for result in restored.saved_results.values() {
            if !result.verify() {
                return Err(TraceError::SavedResultIntegrityMismatch(result.id));
            }
        }
        Ok(restored)
    }

    pub fn reset_terminal_state(&mut self, id: TraceConfigId) -> Result<(), TraceError> {
        if self.active.contains_key(&id) {
            return Err(TraceError::ConfigurationActive(id));
        }
        match self.terminal_states.get(&id).copied() {
            Some(TraceState::Completed | TraceState::Aborted | TraceState::Error) => {
                self.terminal_states.remove(&id);
                Ok(())
            }
            state => Err(TraceError::IllegalTransition {
                from: state.unwrap_or(TraceState::Idle),
                action: "Reset",
            }),
        }
    }

    fn sample_trace(
        &mut self,
        id: TraceConfigId,
        context: ObservationContext,
        values: &BTreeMap<StableTargetId, PublishedTargetValue>,
        diagnostic_events: &[TraceDiagnosticEvent],
        runtime_publication: Option<&TraceRuntimePublication>,
    ) -> Result<Option<TraceCaptureId>, TraceError> {
        let trace = self
            .active
            .get_mut(&id)
            .ok_or(TraceError::UnknownActiveCapture(id))?;
        let mut sample_values = Vec::with_capacity(trace.resolved_channels.len());
        let mut channel_values = Vec::with_capacity(trace.resolved_channels.len());
        let mut trigger_values = BTreeMap::new();
        let mut gap = None;
        for resolved in &trace.resolved_channels {
            let (target_id, value, quality, freshness, force) = match resolved.probe_identity {
                TraceProbeIdentity::LoadedTarget(target_id) => {
                    let published = values.get(&target_id);
                    let value = published
                        .and_then(|published| published_layer_value(*published, resolved.layer));
                    (
                        Some(target_id),
                        value,
                        published.map(|published| published.quality),
                        if value.is_some() {
                            SampleFreshness::Current
                        } else {
                            SampleFreshness::Unknown
                        },
                        published.and_then(|published| published.force),
                    )
                }
                TraceProbeIdentity::ScanQuantumMs => (
                    None,
                    runtime_publication.and_then(|publication| {
                        i64::try_from(publication.scan_quantum_ms)
                            .ok()
                            .map(CanonicalValue::TimeMs)
                    }),
                    Some(Quality::Good),
                    SampleFreshness::Current,
                    None,
                ),
                TraceProbeIdentity::ScanWorkUnits => (
                    None,
                    runtime_publication
                        .map(|publication| CanonicalValue::U32(publication.scan_work_units)),
                    Some(Quality::Good),
                    SampleFreshness::Current,
                    None,
                ),
            };
            if value.is_none() {
                gap = Some(GapReason::PublicationMissing);
            } else if let Some(value) = value {
                trigger_values.insert(resolved.channel_id, value);
            }
            sample_values.push(value);
            channel_values.push(TraceChannelSample {
                channel_id: resolved.channel_id,
                probe_identity: resolved.probe_identity,
                target_id,
                value_type: resolved.value_type,
                layer: resolved.layer,
                value,
                quality,
                freshness,
                force,
            });
        }
        let mut sample = TraceSample {
            sample_ordinal: trace.next_sample_ordinal,
            virtual_timestamp_ms: context.virtual_timestamp_ms,
            scan_sequence: context.scan_sequence,
            event_sequence: context.event_sequence,
            boundary: context.publication_boundary,
            values: sample_values,
            channel_values,
            diagnostic_occurrence_ids: {
                let mut ids = diagnostic_events
                    .iter()
                    .map(|event| (event.key, event.occurrence_id))
                    .collect::<Vec<_>>();
                ids.sort_unstable();
                ids.into_iter().map(|(_, id)| id).collect()
            },
            gap,
            sample_hash: Hash32::ZERO,
        };
        sample.sample_hash = hash_sample(&sample);
        trace.next_sample_ordinal = trace.next_sample_ordinal.saturating_add(1);
        trace.last_sample_virtual_ms = Some(context.virtual_timestamp_ms);
        if let TraceCadence::VirtualIntervalMs(interval) = trace.config.cadence {
            let mut next_due = trace
                .next_due_virtual_ms
                .unwrap_or(context.virtual_timestamp_ms);
            while next_due <= context.virtual_timestamp_ms {
                next_due = next_due.saturating_add(interval);
            }
            trace.next_due_virtual_ms = Some(next_due);
        }

        let (trigger_matches, matched_occurrence_id, latest_diagnostic_key) = evaluate_trigger(
            &trace.config.trigger,
            &trace.previous_values,
            &trigger_values,
            diagnostic_events,
            trace.live_comparison_baseline,
        )?;
        if let Some(latest) = latest_diagnostic_key
            && latest > trace.live_comparison_baseline
        {
            trace.live_comparison_baseline = latest;
        }
        let triggered_now = trace.state == TraceState::Armed && trigger_matches;
        for (channel, value) in trigger_values {
            trace.previous_values.insert(channel, value);
        }
        if triggered_now {
            trace.state = TraceState::Triggered;
            trace.trigger_sample_ordinal = Some(sample.sample_ordinal);
            trace.trigger_virtual_timestamp_ms = Some(context.virtual_timestamp_ms);
            trace.matched_occurrence_id = matched_occurrence_id;
            trace.trigger_boundary = Some(context.publication_boundary);
            trace.captured.extend(trace.pre_buffer.drain(..));
            trace.captured.push(sample);
        } else if trace.state == TraceState::Armed {
            if trace.pre_buffer.len() == trace.config.pre_trigger_samples {
                trace.pre_buffer.pop_front();
            }
            if trace.config.pre_trigger_samples != 0 {
                trace.pre_buffer.push_back(sample);
            }
        } else {
            trace.captured.push(sample);
        }
        if trace.captured.len() > self.limits.samples_per_capture {
            return self
                .abort_internal(id, TraceAbortReason::BufferLimit)
                .map(Some);
        }
        if context
            .virtual_timestamp_ms
            .saturating_sub(trace.context.virtual_timestamp_ms)
            > trace.config.maximum_duration_ms
        {
            return self
                .abort_internal(id, TraceAbortReason::DurationLimit)
                .map(Some);
        }
        let completed = if trace.state == TraceState::Triggered {
            let trigger_ordinal = trace
                .trigger_sample_ordinal
                .expect("triggered trace must retain its trigger ordinal");
            let post_samples = trace.captured.last().map_or(0, |last| {
                last.sample_ordinal.saturating_sub(trigger_ordinal)
            });
            let sample_complete = post_samples >= trace.config.post_trigger_samples as u64;
            let duration_complete = trace
                .config
                .post_trigger_duration_ms
                .is_some_and(|duration| {
                    context.virtual_timestamp_ms.saturating_sub(
                        trace
                            .trigger_virtual_timestamp_ms
                            .expect("triggered trace must retain trigger time"),
                    ) >= duration
                });
            sample_complete || duration_complete
        } else {
            false
        };
        if completed {
            return self.complete_internal(id, None).map(Some);
        }
        Ok(None)
    }

    fn abort_internal(
        &mut self,
        id: TraceConfigId,
        reason: TraceAbortReason,
    ) -> Result<TraceCaptureId, TraceError> {
        self.complete_internal(id, Some(reason))
    }

    fn complete_internal(
        &mut self,
        id: TraceConfigId,
        aborted: Option<TraceAbortReason>,
    ) -> Result<TraceCaptureId, TraceError> {
        let trace = self
            .active
            .remove(&id)
            .ok_or(TraceError::UnknownActiveCapture(id))?;
        let terminal = if aborted.is_some() {
            TraceState::Aborted
        } else {
            TraceState::Completed
        };
        self.terminal_states.insert(id, terminal);
        let mut id_hasher = CanonicalHasher::new("PES-TRACE-CAPTURE-ID-1");
        id_hasher.u128(id.0);
        id_hasher.hash(trace.config_hash);
        id_hasher.u64(trace.context.controller_epoch);
        id_hasher.u64(trace.context.session_epoch);
        id_hasher.u64(trace.next_sample_ordinal);
        let capture_id = TraceCaptureId(crate::canonical::id128(id_hasher.finish()));
        let mut capture = TraceCapture {
            schema_version: 1,
            id: capture_id,
            config_id: id,
            config_hash: trace.config_hash,
            artifact_fingerprint: trace.context.artifact_fingerprint,
            profile_fingerprint: trace.context.profile_fingerprint,
            universe_id: trace.context.universe_id.0,
            universe_epoch: trace.context.universe_epoch,
            controller_id: trace.context.controller_id.0,
            controller_epoch: trace.context.controller_epoch,
            session_epoch: trace.context.session_epoch,
            trigger_sample_ordinal: trace.trigger_sample_ordinal,
            trigger_virtual_timestamp_ms: trace.trigger_virtual_timestamp_ms,
            armed_event_key: trace.armed_event_key,
            live_comparison_baseline: trace.live_comparison_baseline,
            matched_occurrence_id: trace.matched_occurrence_id,
            trigger_boundary: trace.trigger_boundary,
            samples: trace.captured,
            aborted,
            content_hash: Hash32::ZERO,
        };
        capture.content_hash = hash_capture(&capture);
        self.captures.insert(capture_id, capture);
        Ok(capture_id)
    }
}

fn validate_config(config: &TraceConfig, limits: TraceLimits) -> Result<(), TraceError> {
    if config.name.is_empty() {
        return Err(TraceError::EmptyName);
    }
    if config.trigger_id == TraceTriggerId::default() {
        return Err(TraceError::InvalidTriggerIdentity);
    }
    if config.channels.is_empty() || config.channels.len() > limits.channels_per_configuration {
        return Err(TraceError::ChannelLimitExceeded);
    }
    let mut channel_ids = BTreeSet::new();
    let mut aliases = BTreeSet::new();
    for channel in &config.channels {
        if channel.alias.is_empty()
            || !channel.alias.is_ascii()
            || !channel_ids.insert(channel.id)
            || !aliases.insert(channel.alias.to_ascii_lowercase())
        {
            return Err(TraceError::DuplicateOrInvalidChannelIdentity);
        }
    }
    if config.pre_trigger_samples + config.post_trigger_samples + 1 > limits.samples_per_capture {
        return Err(TraceError::SampleLimitExceeded);
    }
    if config.maximum_duration_ms == 0
        || config.maximum_duration_ms > limits.maximum_virtual_duration_ms
    {
        return Err(TraceError::DurationLimitInvalid);
    }
    match config.cadence {
        TraceCadence::EveryScans(0) => return Err(TraceError::CadenceInvalid),
        TraceCadence::VirtualIntervalMs(ms) if ms < limits.minimum_virtual_cadence_ms => {
            return Err(TraceError::CadenceInvalid);
        }
        _ => {}
    }
    let (depth, nodes) = trigger_complexity(&config.trigger);
    if depth > limits.trigger_depth || nodes > limits.trigger_nodes {
        return Err(TraceError::TriggerComplexityExceeded { depth, nodes });
    }
    Ok(())
}

fn resolve_trace_channel(
    channel: &TraceChannel,
    context: ObservationContext,
    catalog: &ProbeCatalog,
) -> Result<ResolvedTraceChannel, TraceError> {
    match &channel.probe {
        TraceProbeKind::LoadedTarget { target, layer } => {
            let resolved = catalog
                .resolve(
                    target,
                    *layer,
                    context.artifact_fingerprint,
                    context.profile_fingerprint,
                )
                .map_err(|_| TraceError::TargetUnavailable)?;
            if !catalog
                .definition(resolved.id)
                .is_some_and(|definition| definition.capabilities.trace)
            {
                return Err(TraceError::CapabilityDenied(resolved.id));
            }
            Ok(ResolvedTraceChannel {
                channel_id: channel.id,
                probe_identity: TraceProbeIdentity::LoadedTarget(resolved.id),
                value_type: resolved.value_type,
                layer: *layer,
            })
        }
        TraceProbeKind::ScanQuantumMs => Ok(ResolvedTraceChannel {
            channel_id: channel.id,
            probe_identity: TraceProbeIdentity::ScanQuantumMs,
            value_type: ValueType::TimeMs,
            layer: ProbeLayer::Natural,
        }),
        TraceProbeKind::ScanWorkUnits => Ok(ResolvedTraceChannel {
            channel_id: channel.id,
            probe_identity: TraceProbeIdentity::ScanWorkUnits,
            value_type: ValueType::U32,
            layer: ProbeLayer::Natural,
        }),
    }
}

fn validate_diagnostic_trigger(
    trigger: &TraceTrigger,
    registry: Option<&DiagnosticRegistry>,
    catalog: &ProbeCatalog,
    _context: ObservationContext,
) -> Result<(), TraceError> {
    let TraceTrigger::DiagnosticEvent(trigger) = trigger else {
        return Ok(());
    };
    let registry = registry.ok_or(TraceError::DiagnosticRegistryRequired)?;
    let definition = registry
        .definition(trigger.definition_id)
        .ok_or(TraceError::DiagnosticDefinitionUnavailable)?;
    if definition.code_version != trigger.code_version {
        return Err(TraceError::DiagnosticDefinitionVersionMismatch);
    }
    let lifecycle_legal = match definition.lifecycle {
        crate::DiagnosticLifecycle::Condition => matches!(
            trigger.lifecycle,
            DiagnosticEventKind::Incoming
                | DiagnosticEventKind::Acknowledged
                | DiagnosticEventKind::Cleared
        ),
        crate::DiagnosticLifecycle::OneShot => trigger.lifecycle == DiagnosticEventKind::OneShot,
        crate::DiagnosticLifecycle::Compaction => false,
    };
    if !lifecycle_legal || trigger.lifecycle == DiagnosticEventKind::Compaction {
        return Err(TraceError::DiagnosticLifecycleUnavailable);
    }
    if let Some(target) = trigger.primary_target_id
        && catalog.definition(target).is_none()
    {
        return Err(TraceError::DiagnosticTargetUnavailable(target));
    }
    Ok(())
}

fn trigger_complexity(trigger: &TraceTrigger) -> (usize, usize) {
    match trigger {
        TraceTrigger::Expression(expression) => expression_complexity(expression),
        _ => (1, 1),
    }
}

fn expression_complexity(expression: &ExpressionNode) -> (usize, usize) {
    match expression {
        ExpressionNode::BooleanChannel(_) | ExpressionNode::NumericComparison { .. } => (1, 1),
        ExpressionNode::Not(child) => {
            let (depth, nodes) = expression_complexity(child);
            (depth.saturating_add(1), nodes.saturating_add(1))
        }
        ExpressionNode::All(children) | ExpressionNode::Any(children) => {
            let mut max_depth = 0;
            let mut nodes = 1_usize;
            for child in children {
                let (depth, child_nodes) = expression_complexity(child);
                max_depth = max_depth.max(depth);
                nodes = nodes.saturating_add(child_nodes);
            }
            (max_depth.saturating_add(1), nodes)
        }
    }
}

fn validate_trigger_channels(
    trigger: &TraceTrigger,
    channels: &BTreeSet<TraceChannelId>,
) -> Result<(), TraceError> {
    let mut referenced = BTreeSet::new();
    collect_trigger_channels(trigger, &mut referenced);
    if referenced.iter().all(|channel| channels.contains(channel)) {
        Ok(())
    } else {
        Err(TraceError::TriggerChannelNotCaptured)
    }
}

fn validate_trigger_types(
    trigger: &TraceTrigger,
    channels: &[ResolvedTraceChannel],
) -> Result<(), TraceError> {
    match trigger {
        TraceTrigger::BooleanRising(channel) | TraceTrigger::BooleanFalling(channel) => {
            validate_channel_type(*channel, ValueType::Bool, channels)
        }
        TraceTrigger::NumericCrossing {
            channel, threshold, ..
        } => validate_channel_type(*channel, threshold.value_type(), channels),
        TraceTrigger::Expression(expression) => validate_expression_types(expression, channels),
        TraceTrigger::Immediate | TraceTrigger::DiagnosticEvent(_) => Ok(()),
    }
}

fn validate_expression_types(
    expression: &ExpressionNode,
    channels: &[ResolvedTraceChannel],
) -> Result<(), TraceError> {
    match expression {
        ExpressionNode::BooleanChannel(channel) => {
            validate_channel_type(*channel, ValueType::Bool, channels)
        }
        ExpressionNode::NumericComparison {
            channel, threshold, ..
        } => validate_channel_type(*channel, threshold.value_type(), channels),
        ExpressionNode::Not(child) => validate_expression_types(child, channels),
        ExpressionNode::All(children) | ExpressionNode::Any(children) => {
            for child in children {
                validate_expression_types(child, channels)?;
            }
            Ok(())
        }
    }
}

fn validate_channel_type(
    channel: TraceChannelId,
    expected: ValueType,
    channels: &[ResolvedTraceChannel],
) -> Result<(), TraceError> {
    if channels
        .iter()
        .any(|resolved| resolved.channel_id == channel && resolved.value_type == expected)
    {
        Ok(())
    } else {
        Err(TraceError::TriggerTypeMismatch(channel))
    }
}

fn collect_trigger_channels(trigger: &TraceTrigger, channels: &mut BTreeSet<TraceChannelId>) {
    match trigger {
        TraceTrigger::BooleanRising(channel)
        | TraceTrigger::BooleanFalling(channel)
        | TraceTrigger::NumericCrossing { channel, .. } => {
            channels.insert(*channel);
        }
        TraceTrigger::Expression(expression) => collect_expression_channels(expression, channels),
        TraceTrigger::Immediate | TraceTrigger::DiagnosticEvent(_) => {}
    }
}

fn collect_expression_channels(
    expression: &ExpressionNode,
    channels: &mut BTreeSet<TraceChannelId>,
) {
    match expression {
        ExpressionNode::BooleanChannel(channel)
        | ExpressionNode::NumericComparison { channel, .. } => {
            channels.insert(*channel);
        }
        ExpressionNode::Not(child) => collect_expression_channels(child, channels),
        ExpressionNode::All(children) | ExpressionNode::Any(children) => {
            for child in children {
                collect_expression_channels(child, channels);
            }
        }
    }
}

fn cadence_due(trace: &ActiveTrace, context: ObservationContext) -> bool {
    match trace.config.cadence {
        TraceCadence::EveryScans(scans) => context.scan_sequence.is_multiple_of(u64::from(scans)),
        TraceCadence::VirtualIntervalMs(_) => trace
            .next_due_virtual_ms
            .is_none_or(|due| context.virtual_timestamp_ms >= due),
    }
}

fn evaluate_trigger(
    trigger: &TraceTrigger,
    previous: &BTreeMap<TraceChannelId, CanonicalValue>,
    current: &BTreeMap<TraceChannelId, CanonicalValue>,
    diagnostic_events: &[TraceDiagnosticEvent],
    diagnostic_baseline: TraceEventKey,
) -> Result<(bool, Option<OccurrenceId>, Option<TraceEventKey>), TraceError> {
    let matched = match trigger {
        TraceTrigger::Immediate => true,
        TraceTrigger::BooleanRising(channel) => {
            let old = previous
                .get(channel)
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            let new = current
                .get(channel)
                .and_then(|value| value.as_bool())
                .ok_or(TraceError::TriggerTypeMismatch(*channel))?;
            !old && new
        }
        TraceTrigger::BooleanFalling(channel) => {
            let old = previous
                .get(channel)
                .and_then(|value| value.as_bool())
                .unwrap_or(true);
            let new = current
                .get(channel)
                .and_then(|value| value.as_bool())
                .ok_or(TraceError::TriggerTypeMismatch(*channel))?;
            old && !new
        }
        TraceTrigger::NumericCrossing {
            channel,
            operator,
            threshold,
        } => {
            let old = previous
                .get(channel)
                .and_then(|value| NumericValue::from_canonical(*value));
            let new = current
                .get(channel)
                .and_then(|value| NumericValue::from_canonical(*value))
                .ok_or(TraceError::TriggerTypeMismatch(*channel))?;
            let old_matches = old
                .map(|old| {
                    compare(old, *operator, *threshold)
                        .ok_or(TraceError::TriggerTypeMismatch(*channel))
                })
                .transpose()?;
            let new_matches = compare(new, *operator, *threshold)
                .ok_or(TraceError::TriggerTypeMismatch(*channel))?;
            old_matches == Some(false) && new_matches
        }
        TraceTrigger::Expression(expression) => evaluate_expression(expression, current)?,
        TraceTrigger::DiagnosticEvent(expected) => {
            let (occurrence, latest) = diagnostic_batch_match(
                &TraceTrigger::DiagnosticEvent(*expected),
                diagnostic_events,
                diagnostic_baseline,
            )
            .expect("diagnostic trigger produces a diagnostic evaluation");
            return Ok((occurrence.is_some(), occurrence, latest));
        }
    };
    Ok((matched, None, None))
}

fn diagnostic_batch_match(
    trigger: &TraceTrigger,
    diagnostic_events: &[TraceDiagnosticEvent],
    diagnostic_baseline: TraceEventKey,
) -> Option<(Option<OccurrenceId>, Option<TraceEventKey>)> {
    let TraceTrigger::DiagnosticEvent(expected) = trigger else {
        return None;
    };
    let mut ordered = diagnostic_events
        .iter()
        .filter(|event| {
            event.key.universe_epoch == diagnostic_baseline.universe_epoch
                && event.key > diagnostic_baseline
        })
        .copied()
        .collect::<Vec<_>>();
    ordered.sort_by_key(|event| (event.key, event.occurrence_id));
    let latest = ordered.last().map(|event| event.key);
    let occurrence = ordered
        .iter()
        .find(|event| {
            event.definition_id == expected.definition_id
                && event.code_version == expected.code_version
                && event.lifecycle == expected.lifecycle
                && expected
                    .primary_target_id
                    .is_none_or(|target| event.primary_target_id == Some(target))
                && expected
                    .root_occurrence_id
                    .is_none_or(|root| event.root_occurrence_id == root)
        })
        .map(|event| event.occurrence_id);
    Some((occurrence, latest))
}

fn evaluate_expression(
    expression: &ExpressionNode,
    current: &BTreeMap<TraceChannelId, CanonicalValue>,
) -> Result<bool, TraceError> {
    match expression {
        ExpressionNode::BooleanChannel(channel) => current
            .get(channel)
            .and_then(|value| value.as_bool())
            .ok_or(TraceError::TriggerTypeMismatch(*channel)),
        ExpressionNode::NumericComparison {
            channel,
            operator,
            threshold,
        } => {
            let value = current
                .get(channel)
                .and_then(|value| NumericValue::from_canonical(*value))
                .ok_or(TraceError::TriggerTypeMismatch(*channel))?;
            compare(value, *operator, *threshold).ok_or(TraceError::TriggerTypeMismatch(*channel))
        }
        ExpressionNode::Not(child) => Ok(!evaluate_expression(child, current)?),
        ExpressionNode::All(children) => {
            for child in children {
                if !evaluate_expression(child, current)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        ExpressionNode::Any(children) => {
            for child in children {
                if evaluate_expression(child, current)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
    }
}

fn compare(left: NumericValue, operator: ComparisonOperator, right: NumericValue) -> Option<bool> {
    Some(match (left, right) {
        (NumericValue::I8(left), NumericValue::I8(right)) => compare_values(left, operator, right),
        (NumericValue::I16(left), NumericValue::I16(right)) => {
            compare_values(left, operator, right)
        }
        (NumericValue::I32(left), NumericValue::I32(right)) => {
            compare_values(left, operator, right)
        }
        (NumericValue::I64(left), NumericValue::I64(right))
        | (NumericValue::TimeMs(left), NumericValue::TimeMs(right)) => {
            compare_values(left, operator, right)
        }
        (NumericValue::U8(left), NumericValue::U8(right)) => compare_values(left, operator, right),
        (NumericValue::U16(left), NumericValue::U16(right)) => {
            compare_values(left, operator, right)
        }
        (NumericValue::U32(left), NumericValue::U32(right)) => {
            compare_values(left, operator, right)
        }
        (NumericValue::U64(left), NumericValue::U64(right)) => {
            compare_values(left, operator, right)
        }
        (NumericValue::F32(left), NumericValue::F32(right)) => {
            compare_values(left.get(), operator, right.get())
        }
        (NumericValue::F64(left), NumericValue::F64(right)) => {
            compare_values(left.get(), operator, right.get())
        }
        _ => return None,
    })
}

fn compare_values<T: PartialEq + PartialOrd>(
    left: T,
    operator: ComparisonOperator,
    right: T,
) -> bool {
    match operator {
        ComparisonOperator::Equal => left == right,
        ComparisonOperator::NotEqual => left != right,
        ComparisonOperator::Less => left < right,
        ComparisonOperator::LessOrEqual => left <= right,
        ComparisonOperator::Greater => left > right,
        ComparisonOperator::GreaterOrEqual => left >= right,
    }
}

fn hash_config(config: &TraceConfig) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-TRACE-CONFIG-1");
    hasher.u128(config.id.0);
    hasher.u128(config.trigger_id.0);
    hasher.string(&config.name);
    hasher.u64(config.channels.len() as u64);
    for channel in &config.channels {
        hasher.u128(channel.id.0);
        hasher.string(&channel.alias);
        encode_trace_probe_kind(&channel.probe, &mut hasher);
        match &channel.display_unit {
            Some(unit) => {
                hasher.bool(true);
                hasher.string(unit);
            }
            None => hasher.bool(false),
        }
    }
    match config.cadence {
        TraceCadence::EveryScans(scans) => {
            hasher.u8(1);
            hasher.u32(scans);
        }
        TraceCadence::VirtualIntervalMs(ms) => {
            hasher.u8(2);
            hasher.u64(ms);
        }
    }
    encode_trigger(&config.trigger, &mut hasher);
    hasher.u64(config.pre_trigger_samples as u64);
    hasher.u64(config.post_trigger_samples as u64);
    match config.post_trigger_duration_ms {
        Some(value) => {
            hasher.bool(true);
            hasher.u64(value);
        }
        None => hasher.bool(false),
    }
    hasher.u64(config.maximum_duration_ms);
    hasher.finish()
}

fn encode_trace_probe_kind(kind: &TraceProbeKind, hasher: &mut CanonicalHasher) {
    match kind {
        TraceProbeKind::LoadedTarget { target, layer } => {
            hasher.u8(1);
            match target {
                TargetReference::Stable(target_id) => {
                    hasher.u8(1);
                    hasher.u128(target_id.0);
                }
                TargetReference::SourceOnly(source_identity) => {
                    hasher.u8(2);
                    crate::target::encode_source_anchor(source_identity, hasher);
                }
            }
            hasher.u8(*layer as u8);
        }
        TraceProbeKind::ScanQuantumMs => hasher.u8(2),
        TraceProbeKind::ScanWorkUnits => hasher.u8(3),
    }
}

fn encode_trace_probe_identity(identity: TraceProbeIdentity, hasher: &mut CanonicalHasher) {
    match identity {
        TraceProbeIdentity::LoadedTarget(target_id) => {
            hasher.u8(1);
            hasher.u128(target_id.0);
        }
        TraceProbeIdentity::ScanQuantumMs => hasher.u8(2),
        TraceProbeIdentity::ScanWorkUnits => hasher.u8(3),
    }
}

fn probe_token(identity: TraceProbeIdentity) -> &'static str {
    match identity {
        TraceProbeIdentity::LoadedTarget(_) => "LOADED_TARGET",
        TraceProbeIdentity::ScanQuantumMs => "SCAN_QUANTUM_MS",
        TraceProbeIdentity::ScanWorkUnits => "SCAN_WORK_UNITS",
    }
}

fn encode_trigger(trigger: &TraceTrigger, hasher: &mut CanonicalHasher) {
    match trigger {
        TraceTrigger::Immediate => hasher.u8(1),
        TraceTrigger::BooleanRising(channel) => {
            hasher.u8(2);
            hasher.u128(channel.0);
        }
        TraceTrigger::BooleanFalling(channel) => {
            hasher.u8(3);
            hasher.u128(channel.0);
        }
        TraceTrigger::NumericCrossing {
            channel,
            operator,
            threshold,
        } => {
            hasher.u8(4);
            hasher.u128(channel.0);
            hasher.u8(*operator as u8);
            encode_numeric(*threshold, hasher);
        }
        TraceTrigger::Expression(expression) => {
            hasher.u8(5);
            encode_expression(expression, hasher);
        }
        TraceTrigger::DiagnosticEvent(trigger) => {
            hasher.u8(6);
            hasher.u128(trigger.definition_id.0);
            hasher.u64(trigger.code_version);
            hasher.u8(trigger.lifecycle as u8);
            match trigger.primary_target_id {
                Some(value) => {
                    hasher.bool(true);
                    hasher.u128(value.0);
                }
                None => hasher.bool(false),
            }
            match trigger.root_occurrence_id {
                Some(value) => {
                    hasher.bool(true);
                    hasher.u128(value.0);
                }
                None => hasher.bool(false),
            }
        }
    }
}

fn encode_expression(expression: &ExpressionNode, hasher: &mut CanonicalHasher) {
    match expression {
        ExpressionNode::BooleanChannel(channel) => {
            hasher.u8(1);
            hasher.u128(channel.0);
        }
        ExpressionNode::NumericComparison {
            channel,
            operator,
            threshold,
        } => {
            hasher.u8(2);
            hasher.u128(channel.0);
            hasher.u8(*operator as u8);
            encode_numeric(*threshold, hasher);
        }
        ExpressionNode::Not(child) => {
            hasher.u8(3);
            encode_expression(child, hasher);
        }
        ExpressionNode::All(children) | ExpressionNode::Any(children) => {
            hasher.u8(if matches!(expression, ExpressionNode::All(_)) {
                4
            } else {
                5
            });
            hasher.u64(children.len() as u64);
            for child in children {
                encode_expression(child, hasher);
            }
        }
    }
}

fn encode_numeric(value: NumericValue, hasher: &mut CanonicalHasher) {
    match value {
        NumericValue::I8(value) => {
            hasher.u8(1);
            hasher.i32(i32::from(value));
        }
        NumericValue::I16(value) => {
            hasher.u8(2);
            hasher.i32(i32::from(value));
        }
        NumericValue::I32(value) => {
            hasher.u8(3);
            hasher.i32(value);
        }
        NumericValue::I64(value) => {
            hasher.u8(4);
            hasher.i64(value);
        }
        NumericValue::U8(value) => {
            hasher.u8(5);
            hasher.u8(value);
        }
        NumericValue::U16(value) => {
            hasher.u8(6);
            hasher.u16(value);
        }
        NumericValue::U32(value) => {
            hasher.u8(7);
            hasher.u32(value);
        }
        NumericValue::U64(value) => {
            hasher.u8(8);
            hasher.u64(value);
        }
        NumericValue::TimeMs(value) => {
            hasher.u8(9);
            hasher.i64(value);
        }
        NumericValue::F32(value) => {
            hasher.u8(10);
            hasher.u32(value.bits());
        }
        NumericValue::F64(value) => {
            hasher.u8(11);
            hasher.u64(value.bits());
        }
    }
}

fn hash_sample(sample: &TraceSample) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-TRACE-SAMPLE-1");
    hasher.u64(sample.sample_ordinal);
    hasher.u64(sample.virtual_timestamp_ms);
    hasher.u64(sample.scan_sequence);
    hasher.u64(sample.event_sequence);
    hasher.u8(sample.boundary as u8);
    hasher.u64(sample.values.len() as u64);
    for value in &sample.values {
        match value {
            Some(value) => {
                hasher.bool(true);
                hasher.value(*value);
            }
            None => hasher.bool(false),
        }
    }
    hasher.u64(sample.channel_values.len() as u64);
    for value in &sample.channel_values {
        hasher.u128(value.channel_id.0);
        encode_trace_probe_identity(value.probe_identity, &mut hasher);
        match value.target_id {
            Some(target_id) => {
                hasher.bool(true);
                hasher.u128(target_id.0);
            }
            None => hasher.bool(false),
        }
        hasher.u8(value.value_type as u8);
        hasher.u8(value.layer as u8);
        match value.value {
            Some(value) => {
                hasher.bool(true);
                hasher.value(value);
            }
            None => hasher.bool(false),
        }
        match value.quality {
            Some(quality) => {
                hasher.bool(true);
                hasher.u8(quality as u8);
            }
            None => hasher.bool(false),
        }
        hasher.u8(value.freshness as u8);
        match value.force {
            Some(force) => {
                hasher.bool(true);
                hasher.u128(force.force_id);
                hasher.u64(force.registry_version);
            }
            None => hasher.bool(false),
        }
    }
    hasher.u64(sample.diagnostic_occurrence_ids.len() as u64);
    for occurrence in &sample.diagnostic_occurrence_ids {
        hasher.u128(occurrence.0);
    }
    match sample.gap {
        Some(reason) => {
            hasher.bool(true);
            hasher.u8(reason as u8);
        }
        None => hasher.bool(false),
    }
    hasher.finish()
}

fn hash_capture(capture: &TraceCapture) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-TRACE-CAPTURE-1");
    hasher.u32(capture.schema_version);
    hasher.u128(capture.id.0);
    hasher.u128(capture.config_id.0);
    hasher.hash(capture.config_hash);
    hasher.hash(capture.artifact_fingerprint);
    hasher.hash(capture.profile_fingerprint);
    hasher.u128(capture.universe_id);
    hasher.u64(capture.universe_epoch);
    hasher.u128(capture.controller_id);
    hasher.u64(capture.controller_epoch);
    hasher.u64(capture.session_epoch);
    encode_optional_u64(capture.trigger_sample_ordinal, &mut hasher);
    encode_optional_u64(capture.trigger_virtual_timestamp_ms, &mut hasher);
    encode_trace_event_key(capture.armed_event_key, &mut hasher);
    encode_trace_event_key(capture.live_comparison_baseline, &mut hasher);
    match capture.matched_occurrence_id {
        Some(value) => {
            hasher.bool(true);
            hasher.u128(value.0);
        }
        None => hasher.bool(false),
    }
    match capture.trigger_boundary {
        Some(value) => {
            hasher.bool(true);
            hasher.u8(value as u8);
        }
        None => hasher.bool(false),
    }
    hasher.u64(capture.samples.len() as u64);
    for sample in &capture.samples {
        hasher.hash(sample.sample_hash);
    }
    match capture.aborted {
        Some(reason) => {
            hasher.bool(true);
            hasher.u8(reason as u8);
        }
        None => hasher.bool(false),
    }
    hasher.finish()
}

fn encode_trace_event_key(key: TraceEventKey, hasher: &mut CanonicalHasher) {
    hasher.u64(key.universe_epoch);
    hasher.u64(key.event_sequence);
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

fn hash_save_command(command: SaveTraceResultCommand) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-TRACE-SAVE-COMMAND-1");
    hasher.u128(command.command_id);
    hasher.u128(command.idempotency_key);
    hasher.u128(command.result_id.0);
    hasher.u128(command.capture_id.0);
    hasher.u128(command.actor_identity);
    hasher.hash(command.audit_context_hash);
    hasher.finish()
}

fn export_json(capture: &TraceCapture) -> String {
    let mut output = alloc::format!(
        "{{\"schemaVersion\":1,\"captureId\":\"{:032x}\",\"configId\":\"{:032x}\",\"artifactFingerprint\":\"{}\",\"rows\":[",
        capture.id.0,
        capture.config_id.0,
        hash_text(capture.artifact_fingerprint)
    );
    let mut first = true;
    for sample in &capture.samples {
        for channel in &sample.channel_values {
            if !first {
                output.push(',');
            }
            first = false;
            let value = channel
                .value
                .map(|value| alloc::format!("\"{}\"", canonical_value_text(value)))
                .unwrap_or_else(|| "null".into());
            let quality = channel
                .quality
                .map(|quality| alloc::format!("\"{}\"", quality_token(quality)))
                .unwrap_or_else(|| "null".into());
            let force_id = channel
                .force
                .map(|force| alloc::format!("\"{:032x}\"", force.force_id))
                .unwrap_or_else(|| "null".into());
            let force_version = channel
                .force
                .map(|force| force.registry_version.to_string())
                .unwrap_or_else(|| "null".into());
            let gap = sample
                .gap
                .map(|gap| alloc::format!("\"{}\"", gap_token(gap)))
                .unwrap_or_else(|| "null".into());
            let target_id = channel
                .target_id
                .map(|target| alloc::format!("\"{:032x}\"", target.0))
                .unwrap_or_else(|| "null".into());
            output.push_str(&alloc::format!(
                "{{\"sampleOrdinal\":{},\"virtualTimestampMs\":{},\"scanSequence\":{},\"eventSequence\":{},\"publicationBoundary\":\"{}\",\"channelId\":\"{:032x}\",\"probeKind\":\"{}\",\"targetId\":{},\"valueType\":\"{}\",\"layer\":\"{}\",\"value\":{},\"quality\":{},\"freshness\":\"{}\",\"forceId\":{},\"forceRegistryVersion\":{},\"gap\":{}}}",
                sample.sample_ordinal,
                sample.virtual_timestamp_ms,
                sample.scan_sequence,
                sample.event_sequence,
                boundary_token(sample.boundary),
                channel.channel_id.0,
                probe_token(channel.probe_identity),
                target_id,
                value_type_token(channel.value_type),
                layer_token(channel.layer),
                value,
                quality,
                freshness_token(channel.freshness),
                force_id,
                force_version,
                gap
            ));
        }
    }
    output.push_str("]}");
    output
}

fn export_csv(capture: &TraceCapture) -> String {
    let mut output = String::from(
        "schemaVersion,captureId,configId,artifactFingerprint,sampleOrdinal,virtualTimestampMs,scanSequence,eventSequence,publicationBoundary,channelId,probeKind,targetId,valueType,layer,value,quality,freshness,forceId,forceRegistryVersion,gap\n",
    );
    let artifact = hash_text(capture.artifact_fingerprint);
    for sample in &capture.samples {
        for channel in &sample.channel_values {
            let value = channel.value.map(canonical_value_text).unwrap_or_default();
            let quality = channel.quality.map(quality_token).unwrap_or("");
            let force_id = channel
                .force
                .map(|force| alloc::format!("{:032x}", force.force_id))
                .unwrap_or_default();
            let force_version = channel
                .force
                .map(|force| force.registry_version.to_string())
                .unwrap_or_default();
            let gap = sample.gap.map(gap_token).unwrap_or("");
            let target_id = channel
                .target_id
                .map(|target| alloc::format!("{:032x}", target.0))
                .unwrap_or_default();
            output.push_str(&alloc::format!(
                "1,{:032x},{:032x},{},{},{},{},{},{},{:032x},{},{},{},{},{},{},{},{},{},{}\n",
                capture.id.0,
                capture.config_id.0,
                artifact,
                sample.sample_ordinal,
                sample.virtual_timestamp_ms,
                sample.scan_sequence,
                sample.event_sequence,
                boundary_token(sample.boundary),
                channel.channel_id.0,
                probe_token(channel.probe_identity),
                target_id,
                value_type_token(channel.value_type),
                layer_token(channel.layer),
                value,
                quality,
                freshness_token(channel.freshness),
                force_id,
                force_version,
                gap
            ));
        }
    }
    output
}

fn hash_text(value: Hash32) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in value.as_bytes() {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    output
}

fn canonical_value_text(value: CanonicalValue) -> String {
    match value {
        CanonicalValue::Bool(value) => {
            alloc::format!("BOOL:{}", if value { "TRUE" } else { "FALSE" })
        }
        CanonicalValue::I32(value) => alloc::format!("I32:{value}"),
        CanonicalValue::I64(value) => alloc::format!("I64:{value}"),
        CanonicalValue::U32(value) => alloc::format!("U32:{value}"),
        CanonicalValue::TimeMs(value) => alloc::format!("TIME_MS:{value}"),
        CanonicalValue::I8(value) => alloc::format!("SINT:{value}"),
        CanonicalValue::I16(value) => alloc::format!("INT:{value}"),
        CanonicalValue::U8(value) => alloc::format!("USINT:{value}"),
        CanonicalValue::U16(value) => alloc::format!("UINT:{value}"),
        CanonicalValue::U64(value) => alloc::format!("ULINT:{value}"),
        CanonicalValue::Bits8(value) => alloc::format!("BYTE:16#{value:02X}"),
        CanonicalValue::Bits16(value) => alloc::format!("WORD:16#{value:04X}"),
        CanonicalValue::Bits32(value) => alloc::format!("DWORD:16#{value:08X}"),
        CanonicalValue::Bits64(value) => alloc::format!("LWORD:16#{value:016X}"),
        CanonicalValue::F32(value) => alloc::format!("REAL_BITS:16#{:08X}", value.bits()),
        CanonicalValue::F64(value) => alloc::format!("LREAL_BITS:16#{:016X}", value.bits()),
        CanonicalValue::Char(value) => alloc::format!("CHAR:{value}"),
    }
}

fn value_type_token(value: ValueType) -> &'static str {
    match value {
        ValueType::Bool => "BOOL",
        ValueType::I32 => "I32",
        ValueType::I64 => "I64",
        ValueType::U32 => "U32",
        ValueType::TimeMs => "TIME_MS",
        ValueType::I8 => "SINT",
        ValueType::I16 => "INT",
        ValueType::U8 => "USINT",
        ValueType::U16 => "UINT",
        ValueType::U64 => "ULINT",
        ValueType::Bits8 => "BYTE",
        ValueType::Bits16 => "WORD",
        ValueType::Bits32 => "DWORD",
        ValueType::Bits64 => "LWORD",
        ValueType::F32 => "REAL",
        ValueType::F64 => "LREAL",
        ValueType::Char => "CHAR",
    }
}

fn layer_token(value: ProbeLayer) -> &'static str {
    match value {
        ProbeLayer::Natural => "NATURAL",
        ProbeLayer::Effective => "EFFECTIVE",
        ProbeLayer::RawInput => "RAW_INPUT",
        ProbeLayer::CommittedOutput => "COMMITTED_OUTPUT",
        ProbeLayer::DeliveredOutput => "DELIVERED_OUTPUT",
    }
}

fn quality_token(value: Quality) -> &'static str {
    match value {
        Quality::Good => "GOOD",
        Quality::Uncertain => "UNCERTAIN",
        Quality::Bad => "BAD",
        Quality::NotPresent => "NOT_PRESENT",
    }
}

fn freshness_token(value: SampleFreshness) -> &'static str {
    match value {
        SampleFreshness::Current => "CURRENT",
        SampleFreshness::Stale => "STALE",
        SampleFreshness::Unknown => "UNKNOWN",
    }
}

fn boundary_token(value: crate::PublicationBoundary) -> &'static str {
    match value {
        crate::PublicationBoundary::ScanEnd => "SCAN_END",
        crate::PublicationBoundary::SerializedCommand => "SERIALIZED_COMMAND",
        crate::PublicationBoundary::FatalFault => "FATAL_FAULT",
        crate::PublicationBoundary::SnapshotReplay => "SNAPSHOT_REPLAY",
    }
}

fn gap_token(value: GapReason) -> &'static str {
    match value {
        GapReason::PublicationMissing => "PUBLICATION_MISSING",
        GapReason::QualityUnavailable => "QUALITY_UNAVAILABLE",
        GapReason::BufferCompaction => "BUFFER_COMPACTION",
        GapReason::ReplayDiscontinuity => "REPLAY_DISCONTINUITY",
    }
}

fn hash_saved_result(result: &TraceSavedResult) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-TRACE-SAVED-RESULT-1");
    hasher.u32(result.schema_version);
    hasher.u128(result.id.0);
    hasher.u128(result.command_id);
    hasher.u128(result.capture_id.0);
    hasher.u128(result.config_id.0);
    hasher.hash(result.config_hash);
    hasher.hash(result.capture_content_hash);
    hasher.hash(result.artifact_fingerprint);
    hasher.u128(result.universe_id);
    hasher.u64(result.universe_epoch);
    hasher.u128(result.controller_id);
    hasher.u64(result.controller_epoch);
    hasher.u64(result.saved_virtual_timestamp_ms);
    hasher.u64(result.saved_event_sequence);
    hasher.u128(result.actor_identity);
    hasher.hash(result.audit_context_hash);
    hasher.hash(result.capture.content_hash);
    hasher.finish()
}

fn hash_engine_snapshot(snapshot: &TraceEngineSnapshot) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-TRACE-ENGINE-SNAPSHOT-1");
    hasher.u32(snapshot.schema_version);
    hasher.u128(snapshot.universe_id);
    hasher.u64(snapshot.captured_universe_epoch);
    hasher.u128(snapshot.controller_id);
    hasher.u64(snapshot.captured_controller_epoch);
    hasher.hash(snapshot.artifact_fingerprint);
    hasher.hash(snapshot.profile_fingerprint);
    encode_trace_limits(snapshot.state.limits, &mut hasher);
    hasher.u64(snapshot.state.configs.len() as u64);
    for (id, config) in &snapshot.state.configs {
        hasher.u128(id.0);
        hasher.hash(hash_config(config));
    }
    hasher.u64(snapshot.state.active.len() as u64);
    for (id, trace) in &snapshot.state.active {
        hasher.u128(id.0);
        hasher.u8(trace.state as u8);
        encode_observation_context(trace.context, &mut hasher);
        hasher.hash(trace.config_hash);
        hasher.u64(trace.resolved_channels.len() as u64);
        for channel in &trace.resolved_channels {
            hasher.u128(channel.channel_id.0);
            encode_trace_probe_identity(channel.probe_identity, &mut hasher);
            hasher.u8(channel.value_type as u8);
            hasher.u8(channel.layer as u8);
        }
        hasher.u64(trace.previous_values.len() as u64);
        for (channel, value) in &trace.previous_values {
            hasher.u128(channel.0);
            hasher.value(*value);
        }
        encode_sample_hashes(trace.pre_buffer.iter(), &mut hasher);
        encode_sample_hashes(trace.captured.iter(), &mut hasher);
        encode_optional_u64(trace.last_sample_virtual_ms, &mut hasher);
        encode_optional_u64(trace.next_due_virtual_ms, &mut hasher);
        encode_optional_u64(trace.trigger_sample_ordinal, &mut hasher);
        encode_optional_u64(trace.trigger_virtual_timestamp_ms, &mut hasher);
        encode_trace_event_key(trace.armed_event_key, &mut hasher);
        encode_trace_event_key(trace.live_comparison_baseline, &mut hasher);
        match trace.matched_occurrence_id {
            Some(id) => {
                hasher.bool(true);
                hasher.u128(id.0);
            }
            None => hasher.bool(false),
        }
        match trace.trigger_boundary {
            Some(boundary) => {
                hasher.bool(true);
                hasher.u8(boundary as u8);
            }
            None => hasher.bool(false),
        }
        hasher.u64(trace.next_sample_ordinal);
    }
    hasher.u64(snapshot.state.captures.len() as u64);
    for (id, capture) in &snapshot.state.captures {
        hasher.u128(id.0);
        hasher.hash(capture.content_hash);
    }
    hasher.u64(snapshot.state.terminal_states.len() as u64);
    for (id, state) in &snapshot.state.terminal_states {
        hasher.u128(id.0);
        hasher.u8(*state as u8);
    }
    hasher.u64(snapshot.state.saved_results.len() as u64);
    for (id, result) in &snapshot.state.saved_results {
        hasher.u128(id.0);
        hasher.hash(result.content_hash);
    }
    hasher.u64(snapshot.state.save_receipts.len() as u64);
    for (key, (payload_hash, result_id)) in &snapshot.state.save_receipts {
        hasher.u128(*key);
        hasher.hash(*payload_hash);
        hasher.u128(result_id.0);
    }
    hasher.finish()
}

fn encode_trace_limits(limits: TraceLimits, hasher: &mut CanonicalHasher) {
    hasher.u64(limits.configurations_per_project as u64);
    hasher.u64(limits.channels_per_configuration as u64);
    hasher.u64(limits.samples_per_capture as u64);
    hasher.u64(limits.concurrent_captures_per_controller as u64);
    hasher.u64(limits.minimum_virtual_cadence_ms);
    hasher.u64(limits.maximum_virtual_duration_ms);
    hasher.u64(limits.trigger_depth as u64);
    hasher.u64(limits.trigger_nodes as u64);
}

fn encode_observation_context(context: ObservationContext, hasher: &mut CanonicalHasher) {
    hasher.u128(context.universe_id.0);
    hasher.u64(context.universe_epoch);
    hasher.u128(context.controller_id.0);
    hasher.u64(context.controller_epoch);
    hasher.u128(context.session_id.0);
    hasher.u64(context.session_epoch);
    hasher.hash(context.package_fingerprint);
    hasher.hash(context.artifact_fingerprint);
    hasher.hash(context.profile_fingerprint);
    hasher.hash(context.target_state_hash);
    hasher.u8(context.cpu_state as u8);
    hasher.u64(context.virtual_timestamp_ms);
    hasher.u64(context.scan_sequence);
    hasher.u64(context.event_sequence);
    hasher.u8(context.publication_boundary as u8);
}

fn encode_sample_hashes<'a>(
    samples: impl Iterator<Item = &'a TraceSample>,
    hasher: &mut CanonicalHasher,
) {
    let samples = samples.collect::<Vec<_>>();
    hasher.u64(samples.len() as u64);
    for sample in samples {
        hasher.hash(sample.sample_hash);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TraceError {
    InvalidLimits,
    EmptyName,
    ConfigurationLimitExceeded,
    ConfigurationActive(TraceConfigId),
    ConcurrentCaptureLimitExceeded,
    ChannelLimitExceeded,
    SampleLimitExceeded,
    DurationLimitInvalid,
    CadenceInvalid,
    TriggerComplexityExceeded {
        depth: usize,
        nodes: usize,
    },
    TriggerChannelNotCaptured,
    TriggerTypeMismatch(TraceChannelId),
    DuplicateProbe(TraceProbeIdentity),
    DuplicateOrInvalidChannelIdentity,
    InvalidTriggerIdentity,
    TargetUnavailable,
    CapabilityDenied(StableTargetId),
    UnknownConfiguration(TraceConfigId),
    UnknownActiveCapture(TraceConfigId),
    UnknownCapture(TraceCaptureId),
    CpuStateDisallowed(CpuState),
    DiagnosticRegistryRequired,
    DiagnosticDefinitionUnavailable,
    DiagnosticDefinitionVersionMismatch,
    DiagnosticLifecycleUnavailable,
    DiagnosticTargetUnavailable(StableTargetId),
    CaptureIntegrityMismatch,
    SampleIntegrityMismatch(u64),
    SaveIdempotencyCollision,
    DuplicateSavedResult(TraceSavedResultId),
    SaveBindingMismatch,
    SavedResultIntegrityMismatch(TraceSavedResultId),
    SnapshotIntegrityMismatch,
    SnapshotBindingMismatch,
    SnapshotTargetMismatch,
    SnapshotRestoreStateDisallowed(CpuState),
    RuntimeInvariant,
    RuntimeMetricUnavailable,
    RuntimeMetricBindingMismatch,
    RuntimeMetricPublicationRequired,
    IllegalTransition {
        from: TraceState,
        action: &'static str,
    },
}

impl fmt::Display for TraceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "trace action rejected: {self:?}")
    }
}

impl Error for TraceError {}
