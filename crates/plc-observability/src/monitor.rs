use alloc::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    string::String,
    vec::Vec,
};
use core::{error::Error, fmt};

use plc_runtime::{CanonicalValue, CpuState, Hash32, ValueType};

use crate::{
    ObservationContext, ProbeCatalog, ProbeLayer, PublicationBoundary, ResolvedTarget,
    StableTargetId, TargetError, TargetReference, canonical::CanonicalHasher,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WatchTableId(pub u128);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WatchRowId(pub u128);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DisplayBase {
    Automatic = 1,
    Binary = 2,
    Decimal = 3,
    Hexadecimal = 4,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WatchRow {
    pub id: WatchRowId,
    pub target: TargetReference,
    pub layer: ProbeLayer,
    pub display_base: DisplayBase,
    pub unit: Option<String>,
    pub format: Option<String>,
    pub note: Option<String>,
    pub order: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WatchTable {
    pub id: WatchTableId,
    pub name: String,
    pub rows: Vec<WatchRow>,
}

impl WatchTable {
    pub fn normalize(&mut self) -> Result<(), MonitorError> {
        if self.name.is_empty() {
            return Err(MonitorError::EmptyTableName);
        }
        self.rows.sort_by_key(|row| (row.order, row.id));
        let mut ids = BTreeSet::new();
        for row in &self.rows {
            if !ids.insert(row.id) {
                return Err(MonitorError::DuplicateRow(row.id));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonitoringLimits {
    pub tables_per_project: usize,
    pub rows_per_table: usize,
    pub active_subscriptions_per_controller: usize,
    pub retained_samples_per_row: usize,
}

impl MonitoringLimits {
    pub const fn edu21() -> Self {
        Self {
            tables_per_project: 64,
            rows_per_table: 512,
            active_subscriptions_per_controller: 2_048,
            retained_samples_per_row: 1_024,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum MonitorState {
    Stopped = 1,
    Starting = 2,
    Active = 3,
    Degraded = 4,
    Stopping = 5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Quality {
    Good = 1,
    Uncertain = 2,
    Bad = 3,
    NotPresent = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeIoState {
    pub target_id: StableTargetId,
    pub runtime_present: bool,
    pub quality: Quality,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SampleFreshness {
    Current = 1,
    Stale = 2,
    Unknown = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ForceProvenance {
    pub force_id: u128,
    pub registry_version: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PublishedTargetValue {
    pub target_id: StableTargetId,
    pub value_type: ValueType,
    pub natural_value: CanonicalValue,
    pub effective_value: CanonicalValue,
    pub raw_input_value: Option<CanonicalValue>,
    pub committed_output_value: Option<CanonicalValue>,
    pub delivered_output_value: Option<CanonicalValue>,
    pub quality: Quality,
    pub force: Option<ForceProvenance>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonitorSample {
    pub row_id: WatchRowId,
    pub target_id: StableTargetId,
    pub value: CanonicalValue,
    pub layer: ProbeLayer,
    pub natural_value: CanonicalValue,
    pub effective_value: CanonicalValue,
    pub quality: Quality,
    pub freshness: SampleFreshness,
    pub force: Option<ForceProvenance>,
    pub virtual_timestamp_ms: u64,
    pub scan_sequence: u64,
    pub event_sequence: u64,
    pub boundary: PublicationBoundary,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MonitorFailure {
    TargetAbsent,
    SourceOnlyTarget,
    ArtifactMismatch,
    ProfileMismatch,
    LayerUnavailable,
    CapabilityDenied,
    ValueTypeMismatch,
    PublicationMissing,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonitoringPersistence {
    pub schema_version: u32,
    pub tables: Vec<WatchTable>,
    pub content_hash: Hash32,
}

impl MonitoringPersistence {
    pub fn new(mut tables: Vec<WatchTable>) -> Result<Self, MonitorError> {
        tables.sort_by_key(|table| table.id);
        let mut ids = BTreeSet::new();
        for table in &mut tables {
            table.normalize()?;
            if !ids.insert(table.id) {
                return Err(MonitorError::DuplicateTable(table.id));
            }
        }
        let mut value = Self {
            schema_version: 1,
            tables,
            content_hash: Hash32::ZERO,
        };
        value.content_hash = value.calculate_hash();
        Ok(value)
    }

    pub fn verify(&self) -> bool {
        self.schema_version == 1 && self.content_hash == self.calculate_hash()
    }

    fn calculate_hash(&self) -> Hash32 {
        let mut hasher = CanonicalHasher::new("PES-WATCH-PERSISTENCE-1");
        hasher.u32(self.schema_version);
        hasher.u64(self.tables.len() as u64);
        for table in &self.tables {
            encode_table(table, &mut hasher);
        }
        hasher.finish()
    }
}

#[derive(Clone, Debug)]
pub struct MonitoringEngine {
    limits: MonitoringLimits,
    tables: BTreeMap<WatchTableId, WatchTable>,
    state: MonitorState,
    context: Option<ObservationContext>,
    resolved: BTreeMap<WatchRowId, ResolvedTarget>,
    failures: BTreeMap<WatchRowId, MonitorFailure>,
    samples: BTreeMap<WatchRowId, VecDeque<MonitorSample>>,
}

impl MonitoringEngine {
    pub fn new(limits: MonitoringLimits) -> Result<Self, MonitorError> {
        if limits.tables_per_project == 0
            || limits.rows_per_table == 0
            || limits.active_subscriptions_per_controller == 0
            || limits.retained_samples_per_row == 0
        {
            return Err(MonitorError::InvalidLimits);
        }
        Ok(Self {
            limits,
            tables: BTreeMap::new(),
            state: MonitorState::Stopped,
            context: None,
            resolved: BTreeMap::new(),
            failures: BTreeMap::new(),
            samples: BTreeMap::new(),
        })
    }

    pub const fn state(&self) -> MonitorState {
        self.state
    }

    pub fn upsert_table(&mut self, mut table: WatchTable) -> Result<(), MonitorError> {
        if self.state != MonitorState::Stopped {
            return Err(MonitorError::ConfigurationWhileRunning);
        }
        table.normalize()?;
        if table.rows.len() > self.limits.rows_per_table {
            return Err(MonitorError::RowsPerTableExceeded);
        }
        if !self.tables.contains_key(&table.id)
            && self.tables.len() == self.limits.tables_per_project
        {
            return Err(MonitorError::TableLimitExceeded);
        }
        self.tables.insert(table.id, table);
        Ok(())
    }

    pub fn remove_table(&mut self, id: WatchTableId) -> Result<bool, MonitorError> {
        if self.state != MonitorState::Stopped {
            return Err(MonitorError::ConfigurationWhileRunning);
        }
        Ok(self.tables.remove(&id).is_some())
    }

    pub fn persistence(&self) -> Result<MonitoringPersistence, MonitorError> {
        MonitoringPersistence::new(self.tables.values().cloned().collect())
    }

    pub fn restore_persistence(
        &mut self,
        persistence: &MonitoringPersistence,
    ) -> Result<(), MonitorError> {
        if self.state != MonitorState::Stopped {
            return Err(MonitorError::ConfigurationWhileRunning);
        }
        if !persistence.verify() {
            return Err(MonitorError::PersistenceIntegrityMismatch);
        }
        if persistence.tables.len() > self.limits.tables_per_project {
            return Err(MonitorError::TableLimitExceeded);
        }
        for table in &persistence.tables {
            if table.rows.len() > self.limits.rows_per_table {
                return Err(MonitorError::RowsPerTableExceeded);
            }
        }
        self.tables = persistence
            .tables
            .iter()
            .cloned()
            .map(|table| (table.id, table))
            .collect();
        Ok(())
    }

    pub fn start(
        &mut self,
        context: ObservationContext,
        catalog: &ProbeCatalog,
    ) -> Result<(), MonitorError> {
        if self.state != MonitorState::Stopped {
            return Err(MonitorError::IllegalTransition {
                from: self.state,
                action: "Start",
            });
        }
        self.state = MonitorState::Starting;
        self.resolved.clear();
        self.failures.clear();
        self.samples.clear();
        let rows = self
            .tables
            .values()
            .flat_map(|table| table.rows.iter())
            .collect::<Vec<_>>();
        if rows.len() > self.limits.active_subscriptions_per_controller {
            self.state = MonitorState::Stopped;
            return Err(MonitorError::ActiveSubscriptionLimitExceeded);
        }
        for row in rows {
            match catalog.resolve(
                &row.target,
                row.layer,
                context.artifact_fingerprint,
                context.profile_fingerprint,
            ) {
                Ok(target) if target_capability(catalog, target.id, |caps| caps.monitor) => {
                    self.resolved.insert(row.id, target);
                    self.samples.insert(row.id, VecDeque::new());
                }
                Ok(_) => {
                    self.failures
                        .insert(row.id, MonitorFailure::CapabilityDenied);
                }
                Err(error) => {
                    self.failures.insert(row.id, map_target_error(&error));
                }
            }
        }
        self.context = Some(context);
        self.state = if self.failures.is_empty() {
            MonitorState::Active
        } else {
            MonitorState::Degraded
        };
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), MonitorError> {
        if !matches!(self.state, MonitorState::Active | MonitorState::Degraded) {
            return Err(MonitorError::IllegalTransition {
                from: self.state,
                action: "Stop",
            });
        }
        self.state = MonitorState::Stopping;
        for samples in self.samples.values_mut() {
            if let Some(last) = samples.back_mut() {
                last.freshness = SampleFreshness::Stale;
            }
        }
        self.context = None;
        self.resolved.clear();
        self.failures.clear();
        self.state = MonitorState::Stopped;
        Ok(())
    }

    pub fn mark_stale(&mut self) {
        if matches!(self.state, MonitorState::Active | MonitorState::Degraded) {
            for samples in self.samples.values_mut() {
                if let Some(last) = samples.back_mut() {
                    last.freshness = SampleFreshness::Stale;
                }
            }
            self.state = MonitorState::Degraded;
        }
    }

    pub fn publish(
        &mut self,
        context: ObservationContext,
        values: &[PublishedTargetValue],
    ) -> Result<usize, MonitorError> {
        if !matches!(self.state, MonitorState::Active | MonitorState::Degraded) {
            return Err(MonitorError::IllegalTransition {
                from: self.state,
                action: "Publish",
            });
        }
        let bound = self.context.ok_or(MonitorError::MissingContext)?;
        if !bound.same_runtime_epoch(context) {
            self.mark_stale();
            return Err(MonitorError::EpochChanged);
        }
        if !publication_allowed(context) {
            self.mark_stale();
            return Ok(0);
        }
        let publications = values
            .iter()
            .map(|value| (value.target_id, *value))
            .collect::<BTreeMap<_, _>>();
        let mut appended = 0;
        for (row_id, target) in &self.resolved {
            let Some(publication) = publications.get(&target.id) else {
                self.failures
                    .insert(*row_id, MonitorFailure::PublicationMissing);
                continue;
            };
            if publication.value_type != target.value_type {
                self.failures
                    .insert(*row_id, MonitorFailure::ValueTypeMismatch);
                continue;
            }
            let Some(value) = published_layer_value(*publication, target.layer) else {
                self.failures
                    .insert(*row_id, MonitorFailure::PublicationMissing);
                continue;
            };
            let sample = MonitorSample {
                row_id: *row_id,
                target_id: target.id,
                value,
                layer: target.layer,
                natural_value: publication.natural_value,
                effective_value: publication.effective_value,
                quality: publication.quality,
                freshness: SampleFreshness::Current,
                force: publication.force,
                virtual_timestamp_ms: context.virtual_timestamp_ms,
                scan_sequence: context.scan_sequence,
                event_sequence: context.event_sequence,
                boundary: context.publication_boundary,
            };
            let row_samples = self
                .samples
                .get_mut(row_id)
                .expect("every resolved row owns a bounded sample queue");
            if row_samples.len() == self.limits.retained_samples_per_row {
                row_samples.pop_front();
            }
            row_samples.push_back(sample);
            self.failures.remove(row_id);
            appended += 1;
        }
        self.context = Some(context);
        self.state = if self.failures.is_empty() {
            MonitorState::Active
        } else {
            MonitorState::Degraded
        };
        Ok(appended)
    }

    pub fn latest(&self, row_id: WatchRowId) -> Option<&MonitorSample> {
        self.samples.get(&row_id)?.back()
    }

    pub fn history(&self, row_id: WatchRowId) -> Option<&VecDeque<MonitorSample>> {
        self.samples.get(&row_id)
    }

    pub fn failure(&self, row_id: WatchRowId) -> Option<&MonitorFailure> {
        self.failures.get(&row_id)
    }
}

pub(crate) fn published_layer_value(
    publication: PublishedTargetValue,
    layer: ProbeLayer,
) -> Option<CanonicalValue> {
    match layer {
        ProbeLayer::Natural => Some(publication.natural_value),
        ProbeLayer::Effective => Some(publication.effective_value),
        ProbeLayer::RawInput => publication.raw_input_value,
        ProbeLayer::CommittedOutput => publication.committed_output_value,
        ProbeLayer::DeliveredOutput => publication.delivered_output_value,
    }
}

fn publication_allowed(context: ObservationContext) -> bool {
    match context.cpu_state {
        CpuState::Run => context.publication_boundary == PublicationBoundary::ScanEnd,
        CpuState::Stop | CpuState::PausedEducational => {
            context.publication_boundary == PublicationBoundary::SerializedCommand
        }
        CpuState::Faulted => matches!(
            context.publication_boundary,
            PublicationBoundary::FatalFault | PublicationBoundary::SerializedCommand
        ),
        CpuState::PoweredOff | CpuState::Startup | CpuState::Resetting => false,
    }
}

fn target_capability(
    catalog: &ProbeCatalog,
    id: StableTargetId,
    selector: impl FnOnce(crate::AccessCapabilities) -> bool,
) -> bool {
    catalog
        .definition(id)
        .is_some_and(|definition| selector(definition.capabilities))
}

fn map_target_error(error: &TargetError) -> MonitorFailure {
    match error {
        TargetError::SourceOnlyReference => MonitorFailure::SourceOnlyTarget,
        TargetError::ArtifactMismatch | TargetError::SourceArtifactMismatch => {
            MonitorFailure::ArtifactMismatch
        }
        TargetError::ProfileMismatch => MonitorFailure::ProfileMismatch,
        TargetError::LayerUnavailable { .. } => MonitorFailure::LayerUnavailable,
        _ => MonitorFailure::TargetAbsent,
    }
}

fn encode_table(table: &WatchTable, hasher: &mut CanonicalHasher) {
    hasher.u128(table.id.0);
    hasher.string(&table.name);
    hasher.u64(table.rows.len() as u64);
    for row in &table.rows {
        hasher.u128(row.id.0);
        match &row.target {
            TargetReference::Stable(id) => {
                hasher.u8(1);
                hasher.u128(id.0);
            }
            TargetReference::SourceOnly(source) => {
                hasher.u8(2);
                crate::target::encode_source_anchor(source, hasher);
            }
        }
        hasher.u8(row.layer as u8);
        hasher.u8(row.display_base as u8);
        encode_optional_string(row.unit.as_deref(), hasher);
        encode_optional_string(row.format.as_deref(), hasher);
        encode_optional_string(row.note.as_deref(), hasher);
        hasher.u32(row.order);
    }
}

fn encode_optional_string(value: Option<&str>, hasher: &mut CanonicalHasher) {
    match value {
        Some(value) => {
            hasher.bool(true);
            hasher.string(value);
        }
        None => hasher.bool(false),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MonitorError {
    InvalidLimits,
    EmptyTableName,
    DuplicateTable(WatchTableId),
    DuplicateRow(WatchRowId),
    TableLimitExceeded,
    RowsPerTableExceeded,
    ActiveSubscriptionLimitExceeded,
    ConfigurationWhileRunning,
    PersistenceIntegrityMismatch,
    MissingContext,
    EpochChanged,
    IllegalTransition {
        from: MonitorState,
        action: &'static str,
    },
}

impl fmt::Display for MonitorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "monitoring action rejected: {self:?}")
    }
}

impl Error for MonitorError {}
