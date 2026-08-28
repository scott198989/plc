use std::{collections::BTreeSet, error::Error, fmt};

use plc_hardware::{
    ConditionLifecycle, HardwareConditionEngine, HardwareConditionKey, HardwareConditionSnapshot,
    HardwareFaultAction, HardwareFaultCommand, ModuleRuntimeState, ProfilePin,
    RuntimeHardwareConfiguration, Uuid, VirtualNetwork,
};
use plc_observability::{
    DiagnosticLedger, DiagnosticLedgerSnapshot, HardwareDiagnosticBridge, MonitorSample,
    MonitoringEngine, MonitoringPersistence, ObservationContext, OccurrenceId, TraceCaptureId,
    TraceEngine, TraceEngineSnapshot, WatchRowId,
};
use plc_runtime::{Hash32, Sha256};

/// One explicit causal join across the condition provider, monitoring,
/// diagnostics, and trace boundaries. The aggregate constructor admits a link
/// only after every referenced production record resolves exactly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HardwareObservationLinkRequest {
    pub provider_event_sequence: u64,
    pub diagnostic_occurrence_id: OccurrenceId,
    pub monitor_row_id: WatchRowId,
    pub publication_event_sequence: u64,
    pub trace_capture_id: TraceCaptureId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HardwareObservationLink {
    provider_event_sequence: u64,
    diagnostic_occurrence_id: OccurrenceId,
    monitor_row_id: WatchRowId,
    publication_event_sequence: u64,
    trace_capture_id: TraceCaptureId,
}

impl From<HardwareObservationLinkRequest> for HardwareObservationLink {
    fn from(value: HardwareObservationLinkRequest) -> Self {
        Self {
            provider_event_sequence: value.provider_event_sequence,
            diagnostic_occurrence_id: value.diagnostic_occurrence_id,
            monitor_row_id: value.monitor_row_id,
            publication_event_sequence: value.publication_event_sequence,
            trace_capture_id: value.trace_capture_id,
        }
    }
}

/// Borrowed production state admitted into one immutable hardware-observation
/// snapshot. This keeps the capture API bounded without hiding its inputs.
#[derive(Clone, Copy)]
pub struct HardwareObservationCapture<'a> {
    pub conditions: &'a HardwareConditionEngine,
    pub commands: &'a [HardwareFaultCommand],
    pub monitoring: &'a MonitoringEngine,
    pub traces: &'a TraceEngine,
    pub diagnostics: &'a DiagnosticLedger,
    pub diagnostic_bridge: &'a HardwareDiagnosticBridge,
    pub context: ObservationContext,
    pub links: &'a [HardwareObservationLinkRequest],
}

/// Integrity-bound aggregate of the physical-condition observation path.
/// Mutable engines remain owned by their domains; this value retains their
/// verified snapshots, causal joins, and exact typed command log for replay.
#[derive(Clone, Debug)]
pub struct HardwareObservationSnapshot {
    pub schema_version: u32,
    pub context: ObservationContext,
    pub conditions: HardwareConditionSnapshot,
    pub monitoring_hash: Hash32,
    pub trace_hash: Hash32,
    pub diagnostic_hash: Hash32,
    pub diagnostic_bridge_hash: Hash32,
    pub content_hash: Hash32,
    commands: Vec<HardwareFaultCommand>,
    monitoring: MonitoringPersistence,
    traces: TraceEngineSnapshot,
    diagnostics: DiagnosticLedgerSnapshot,
    diagnostic_bridge: HardwareDiagnosticBridge,
    links: Vec<HardwareObservationLink>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HardwareObservationReplayReceipt {
    pub snapshot_content_hash: Hash32,
    pub replayed_condition_fingerprint: plc_hardware::Sha256Digest,
    pub replayed_command_count: usize,
    pub replayed_causal_link_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HardwareObservationError {
    MonitoringPersistenceUnavailable,
    ComponentIntegrityMismatch,
    SnapshotBindingMismatch,
    DuplicateOrIncompleteCausalLinks,
    CausalConditionEventMissing,
    CausalDiagnosticReceiptMissing,
    CausalDiagnosticSnapshotMissing,
    CausalMonitorSampleMissing,
    CausalTraceSampleMissing,
    ReplayFailed,
    ReplayMismatch,
}

impl fmt::Display for HardwareObservationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "hardware observation rejected: {self:?}")
    }
}

impl Error for HardwareObservationError {}

impl HardwareObservationSnapshot {
    pub fn capture(
        input: HardwareObservationCapture<'_>,
    ) -> Result<Self, HardwareObservationError> {
        let monitoring = input
            .monitoring
            .persistence()
            .map_err(|_| HardwareObservationError::MonitoringPersistenceUnavailable)?;
        let traces = input.traces.capture_snapshot(input.context);
        let diagnostics = input.diagnostics.capture_snapshot(input.context);
        let conditions = input.conditions.snapshot();
        let diagnostic_bridge_hash = input
            .diagnostic_bridge
            .replay_hash()
            .map_err(|_| HardwareObservationError::ComponentIntegrityMismatch)?;
        let mut links = input
            .links
            .iter()
            .copied()
            .map(HardwareObservationLink::from)
            .collect::<Vec<_>>();
        links.sort_by_key(|link| link.provider_event_sequence);
        validate_component_binding(
            input.context,
            &conditions,
            &monitoring,
            &traces,
            &diagnostics,
            input.diagnostic_bridge,
            diagnostic_bridge_hash,
        )?;
        validate_causal_links(
            &conditions,
            input.monitoring,
            &traces,
            &diagnostics,
            input.diagnostic_bridge,
            &links,
        )?;
        let mut snapshot = Self {
            schema_version: 1,
            context: input.context,
            conditions,
            monitoring_hash: monitoring.content_hash,
            trace_hash: traces.content_hash,
            diagnostic_hash: diagnostics.content_hash,
            diagnostic_bridge_hash,
            content_hash: Hash32::ZERO,
            commands: input.commands.to_vec(),
            monitoring,
            traces,
            diagnostics,
            diagnostic_bridge: input.diagnostic_bridge.clone(),
            links,
        };
        snapshot.content_hash = snapshot.calculate_hash();
        Ok(snapshot)
    }

    #[must_use]
    pub fn causal_link_count(&self) -> usize {
        self.links.len()
    }

    #[must_use]
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }

    #[must_use]
    pub fn verify(&self) -> bool {
        self.schema_version == 1
            && self.monitoring_hash == self.monitoring.content_hash
            && self.trace_hash == self.traces.content_hash
            && self.diagnostic_hash == self.diagnostics.content_hash
            && self.diagnostic_bridge.replay_hash().ok() == Some(self.diagnostic_bridge_hash)
            && validate_component_binding(
                self.context,
                &self.conditions,
                &self.monitoring,
                &self.traces,
                &self.diagnostics,
                &self.diagnostic_bridge,
                self.diagnostic_bridge_hash,
            )
            .is_ok()
            && validate_snapshot_links(
                &self.conditions,
                &self.traces,
                &self.diagnostics,
                &self.diagnostic_bridge,
                &self.links,
            )
            .is_ok()
            && self.content_hash == self.calculate_hash()
    }

    pub fn replay(
        &self,
        profile_pin: ProfilePin,
        configuration: RuntimeHardwareConfiguration,
        network: VirtualNetwork,
    ) -> Result<HardwareObservationReplayReceipt, HardwareObservationError> {
        if !self.verify() || profile_pin != self.conditions.profile_pin {
            return Err(HardwareObservationError::ComponentIntegrityMismatch);
        }
        let replayed = HardwareConditionEngine::replay(
            profile_pin,
            configuration,
            network,
            self.conditions.controller_epoch,
            self.commands.clone(),
        )
        .map_err(|_| HardwareObservationError::ReplayFailed)?;
        let replayed_snapshot = replayed.snapshot();
        if replayed_snapshot != self.conditions {
            return Err(HardwareObservationError::ReplayMismatch);
        }
        Ok(HardwareObservationReplayReceipt {
            snapshot_content_hash: self.content_hash,
            replayed_condition_fingerprint: replayed_snapshot.state_fingerprint,
            replayed_command_count: self.commands.len(),
            replayed_causal_link_count: self.links.len(),
        })
    }

    fn calculate_hash(&self) -> Hash32 {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"PES-HARDWARE-OBSERVATION-SNAPSHOT-1\0");
        bytes.extend_from_slice(&self.schema_version.to_be_bytes());
        encode_context(&mut bytes, self.context);
        encode_condition_snapshot(&mut bytes, &self.conditions);
        encode_commands(&mut bytes, &self.commands);
        bytes.extend_from_slice(&self.monitoring_hash.0);
        bytes.extend_from_slice(&self.trace_hash.0);
        bytes.extend_from_slice(&self.diagnostic_hash.0);
        bytes.extend_from_slice(&self.diagnostic_bridge_hash.0);
        bytes.extend_from_slice(&usize_u64(self.links.len()).to_be_bytes());
        for link in &self.links {
            bytes.extend_from_slice(&link.provider_event_sequence.to_be_bytes());
            bytes.extend_from_slice(&link.diagnostic_occurrence_id.0.to_be_bytes());
            bytes.extend_from_slice(&link.monitor_row_id.0.to_be_bytes());
            bytes.extend_from_slice(&link.publication_event_sequence.to_be_bytes());
            bytes.extend_from_slice(&link.trace_capture_id.0.to_be_bytes());
        }
        Sha256::digest(&bytes)
    }
}

fn validate_component_binding(
    context: ObservationContext,
    conditions: &HardwareConditionSnapshot,
    monitoring: &MonitoringPersistence,
    traces: &TraceEngineSnapshot,
    diagnostics: &DiagnosticLedgerSnapshot,
    bridge: &HardwareDiagnosticBridge,
    bridge_hash: Hash32,
) -> Result<(), HardwareObservationError> {
    if !monitoring.verify()
        || !traces.verify()
        || !diagnostics.verify()
        || bridge.replay_hash().ok() != Some(bridge_hash)
    {
        return Err(HardwareObservationError::ComponentIntegrityMismatch);
    }
    if conditions.controller_epoch != context.controller_epoch
        || traces.universe_id != context.universe_id.0
        || traces.captured_universe_epoch != context.universe_epoch
        || traces.controller_id != context.controller_id.0
        || traces.captured_controller_epoch != context.controller_epoch
        || traces.artifact_fingerprint != context.artifact_fingerprint
        || traces.profile_fingerprint != context.profile_fingerprint
        || diagnostics.universe_id != context.universe_id.0
        || diagnostics.captured_universe_epoch != context.universe_epoch
        || diagnostics.controller_id != context.controller_id.0
        || diagnostics.captured_controller_epoch != context.controller_epoch
        || diagnostics.artifact_fingerprint != context.artifact_fingerprint
        || diagnostics.profile_fingerprint != context.profile_fingerprint
    {
        return Err(HardwareObservationError::SnapshotBindingMismatch);
    }
    Ok(())
}

fn validate_causal_links(
    conditions: &HardwareConditionSnapshot,
    monitoring: &MonitoringEngine,
    traces: &TraceEngineSnapshot,
    diagnostics: &DiagnosticLedgerSnapshot,
    bridge: &HardwareDiagnosticBridge,
    links: &[HardwareObservationLink],
) -> Result<(), HardwareObservationError> {
    validate_snapshot_links(conditions, traces, diagnostics, bridge, links)?;
    for link in links {
        let sample = monitoring
            .history(link.monitor_row_id)
            .and_then(|samples| {
                samples
                    .iter()
                    .find(|sample| sample.event_sequence == link.publication_event_sequence)
            })
            .ok_or(HardwareObservationError::CausalMonitorSampleMissing)?;
        validate_monitor_sample(sample, link)?;
    }
    Ok(())
}

fn validate_snapshot_links(
    conditions: &HardwareConditionSnapshot,
    traces: &TraceEngineSnapshot,
    diagnostics: &DiagnosticLedgerSnapshot,
    bridge: &HardwareDiagnosticBridge,
    links: &[HardwareObservationLink],
) -> Result<(), HardwareObservationError> {
    let linked = links
        .iter()
        .map(|link| link.provider_event_sequence)
        .collect::<BTreeSet<_>>();
    let condition_sequences = conditions
        .condition_events
        .iter()
        .map(|event| event.sequence)
        .collect::<BTreeSet<_>>();
    if linked.len() != links.len()
        || linked != condition_sequences
        || bridge.receipts().len() != conditions.condition_events.len()
    {
        return Err(HardwareObservationError::DuplicateOrIncompleteCausalLinks);
    }
    for link in links {
        let condition = conditions
            .condition_events
            .iter()
            .find(|event| event.sequence == link.provider_event_sequence)
            .ok_or(HardwareObservationError::CausalConditionEventMissing)?;
        let receipt = bridge
            .receipts()
            .find(|receipt| {
                receipt.provider_key.provider_event_sequence == link.provider_event_sequence
            })
            .ok_or(HardwareObservationError::CausalDiagnosticReceiptMissing)?;
        if !receipt.verify()
            || receipt.condition != condition.condition
            || receipt.lifecycle != condition.lifecycle
            || receipt.provider_command_boundary != condition.command_boundary
            || receipt.ledger_occurrence_id != link.diagnostic_occurrence_id
        {
            return Err(HardwareObservationError::CausalDiagnosticReceiptMissing);
        }
        if !diagnostics.contains_occurrence(link.diagnostic_occurrence_id) {
            return Err(HardwareObservationError::CausalDiagnosticSnapshotMissing);
        }
        if !traces.contains_diagnostic_occurrence(
            link.trace_capture_id,
            link.publication_event_sequence,
            link.diagnostic_occurrence_id,
        ) {
            return Err(HardwareObservationError::CausalTraceSampleMissing);
        }
    }
    Ok(())
}

fn validate_monitor_sample(
    sample: &MonitorSample,
    link: &HardwareObservationLink,
) -> Result<(), HardwareObservationError> {
    if sample.row_id == link.monitor_row_id
        && sample.event_sequence == link.publication_event_sequence
    {
        Ok(())
    } else {
        Err(HardwareObservationError::CausalMonitorSampleMissing)
    }
}

fn encode_context(bytes: &mut Vec<u8>, context: ObservationContext) {
    bytes.extend_from_slice(&context.universe_id.0.to_be_bytes());
    bytes.extend_from_slice(&context.universe_epoch.to_be_bytes());
    bytes.extend_from_slice(&context.controller_id.0.to_be_bytes());
    bytes.extend_from_slice(&context.controller_epoch.to_be_bytes());
    bytes.extend_from_slice(&context.session_id.0.to_be_bytes());
    bytes.extend_from_slice(&context.session_epoch.to_be_bytes());
    bytes.extend_from_slice(&context.package_fingerprint.0);
    bytes.extend_from_slice(&context.artifact_fingerprint.0);
    bytes.extend_from_slice(&context.profile_fingerprint.0);
    bytes.extend_from_slice(&context.target_state_hash.0);
    bytes.push(context.cpu_state as u8);
    bytes.extend_from_slice(&context.virtual_timestamp_ms.to_be_bytes());
    bytes.extend_from_slice(&context.scan_sequence.to_be_bytes());
    bytes.extend_from_slice(&context.event_sequence.to_be_bytes());
    bytes.push(context.publication_boundary as u8);
}

fn encode_commands(bytes: &mut Vec<u8>, commands: &[HardwareFaultCommand]) {
    bytes.extend_from_slice(&usize_u64(commands.len()).to_be_bytes());
    for command in commands {
        append_uuid(bytes, command.idempotency_key);
        bytes.extend_from_slice(&command.expected_controller_epoch.to_be_bytes());
        encode_action(bytes, command.action);
    }
}

fn encode_condition_snapshot(bytes: &mut Vec<u8>, snapshot: &HardwareConditionSnapshot) {
    append_bytes(bytes, snapshot.profile_pin.id.as_bytes());
    append_bytes(bytes, snapshot.profile_pin.version.as_bytes());
    bytes.extend_from_slice(&snapshot.profile_pin.manifest_hash.0);
    bytes.extend_from_slice(&snapshot.controller_epoch.to_be_bytes());
    bytes.extend_from_slice(&snapshot.command_boundary.to_be_bytes());
    bytes.extend_from_slice(&usize_u64(snapshot.module_states.len()).to_be_bytes());
    for (id, state) in &snapshot.module_states {
        append_uuid(bytes, id.uuid());
        bytes.push(match state {
            ModuleRuntimeState::ConfiguredPresent => 1,
            ModuleRuntimeState::Pulled => 2,
            ModuleRuntimeState::WrongCatalogInstalled => 3,
        });
    }
    encode_uuid_set(bytes, snapshot.channel_faults.iter().map(|id| id.uuid()));
    encode_uuid_set(bytes, snapshot.wire_breaks.iter().map(|id| id.uuid()));
    encode_bool_map(
        bytes,
        snapshot
            .controller_powered
            .iter()
            .map(|(id, value)| (id.uuid(), *value)),
    );
    encode_bool_map(
        bytes,
        snapshot
            .station_available
            .iter()
            .map(|(id, value)| (id.uuid(), *value)),
    );
    encode_bool_map(
        bytes,
        snapshot
            .link_available
            .iter()
            .map(|(id, value)| (id.uuid(), *value)),
    );
    bytes.extend_from_slice(&usize_u64(snapshot.active_conditions.len()).to_be_bytes());
    for condition in &snapshot.active_conditions {
        encode_condition(bytes, *condition);
    }
    bytes.extend_from_slice(&usize_u64(snapshot.condition_events.len()).to_be_bytes());
    for event in &snapshot.condition_events {
        bytes.extend_from_slice(&event.sequence.to_be_bytes());
        bytes.extend_from_slice(&event.command_boundary.to_be_bytes());
        encode_condition(bytes, event.condition);
        bytes.push(match event.lifecycle {
            ConditionLifecycle::Activated => 1,
            ConditionLifecycle::Cleared => 2,
        });
        append_bytes(bytes, event.diagnostic_code.stable_code().as_bytes());
    }
    bytes.extend_from_slice(&snapshot.network_state_fingerprint.0);
    bytes.extend_from_slice(&snapshot.state_fingerprint.0);
}

fn encode_condition(bytes: &mut Vec<u8>, condition: HardwareConditionKey) {
    match condition {
        HardwareConditionKey::ModuleNotPresent(id) => append_tagged_uuid(bytes, 1, id.uuid()),
        HardwareConditionKey::WrongModule(id) => append_tagged_uuid(bytes, 2, id.uuid()),
        HardwareConditionKey::ChannelFault(id) => append_tagged_uuid(bytes, 3, id.uuid()),
        HardwareConditionKey::WireBreak(id) => append_tagged_uuid(bytes, 4, id.uuid()),
        HardwareConditionKey::ControllerUnpowered(id) => {
            append_tagged_uuid(bytes, 5, id.uuid());
        }
        HardwareConditionKey::StationUnavailable(id) => {
            append_tagged_uuid(bytes, 6, id.uuid());
        }
        HardwareConditionKey::LinkUnavailable(id) => append_tagged_uuid(bytes, 7, id.uuid()),
    }
}

fn encode_uuid_set(bytes: &mut Vec<u8>, ids: impl ExactSizeIterator<Item = Uuid>) {
    bytes.extend_from_slice(&usize_u64(ids.len()).to_be_bytes());
    for id in ids {
        append_uuid(bytes, id);
    }
}

fn encode_bool_map(bytes: &mut Vec<u8>, values: impl ExactSizeIterator<Item = (Uuid, bool)>) {
    bytes.extend_from_slice(&usize_u64(values.len()).to_be_bytes());
    for (id, value) in values {
        append_uuid(bytes, id);
        bytes.push(u8::from(value));
    }
}

fn encode_action(bytes: &mut Vec<u8>, action: HardwareFaultAction) {
    match action {
        HardwareFaultAction::PullModule(id) => append_tagged_uuid(bytes, 1, id.uuid()),
        HardwareFaultAction::RestoreModule(id) => append_tagged_uuid(bytes, 2, id.uuid()),
        HardwareFaultAction::InstallWrongModule {
            module_id,
            installed_catalog,
        } => {
            append_tagged_uuid(bytes, 3, module_id.uuid());
            append_bytes(bytes, installed_catalog.as_str().as_bytes());
        }
        HardwareFaultAction::RestoreConfiguredModule(id) => {
            append_tagged_uuid(bytes, 4, id.uuid());
        }
        HardwareFaultAction::SetChannelFault(id) => append_tagged_uuid(bytes, 5, id.uuid()),
        HardwareFaultAction::ClearChannelFault(id) => append_tagged_uuid(bytes, 6, id.uuid()),
        HardwareFaultAction::SetWireBreak(id) => append_tagged_uuid(bytes, 7, id.uuid()),
        HardwareFaultAction::ClearWireBreak(id) => append_tagged_uuid(bytes, 8, id.uuid()),
        HardwareFaultAction::SetControllerPowered { device_id, powered } => {
            append_tagged_uuid(bytes, 9, device_id.uuid());
            bytes.push(u8::from(powered));
        }
        HardwareFaultAction::SetStationAvailable {
            device_id,
            available,
        } => {
            append_tagged_uuid(bytes, 10, device_id.uuid());
            bytes.push(u8::from(available));
        }
        HardwareFaultAction::SetVirtualLinkAvailable { link_id, available } => {
            append_tagged_uuid(bytes, 11, link_id.uuid());
            bytes.push(u8::from(available));
        }
    }
}

fn append_tagged_uuid(bytes: &mut Vec<u8>, tag: u8, id: Uuid) {
    bytes.push(tag);
    append_uuid(bytes, id);
}

fn append_uuid(bytes: &mut Vec<u8>, id: Uuid) {
    bytes.extend_from_slice(&id.into_bytes());
}

fn append_bytes(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(&usize_u64(value.len()).to_be_bytes());
    bytes.extend_from_slice(value);
}

fn usize_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
