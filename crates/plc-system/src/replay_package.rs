//! Canonical, bounded, capability-free Phase 2 replay packages.
//!
//! The codec owns no clock, filesystem, transport, executable extension, or
//! replay side effect. It accepts already-captured semantic events and boundary
//! hashes, produces one deterministic byte string, and decodes only that exact
//! canonical representation.

#![allow(clippy::struct_field_names)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::session::EngineeringSessionSnapshot;
use plc_runtime::{
    BoundaryHash, CanonicalValue, Hash32, PRIORITY_TABLE_VERSION, RUNTIME_SEMANTICS_VERSION,
    ReplayEvent, ReplayEventKind, ReplaySegment, SCHEDULER_VERSION, Sha256, WORK_COST_VERSION,
};
use plc_types::{
    AggregateLimits, CanonicalType, PlcValue, PrimitiveType, ScalarValue, StableUuid, TypeError,
};

const MAGIC: &[u8; 8] = b"PESRPLY1";
const CONTAINER_VERSION: u32 = 1;
const SCHEMA_VERSION: u32 = 2;
const CANONICALIZATION_VERSION: u32 = 2;
const PACKAGE_KIND: &str = "plc-engineering-replay";
const MEMBER_NAMES: [&str; 4] = [
    "manifest.json",
    "initial-snapshot.ref",
    "events.jsonl",
    "boundary-hashes.jsonl",
];
const MAX_PACKAGE_BYTES: usize = 536_870_912;
const MAX_MEMBER_BYTES: usize = 268_435_456;
const MAX_TOTAL_MEMBER_BYTES: usize = 1_073_741_824;
const MAX_RECORDS: usize = 1_000_000;
const MAX_LINE_BYTES: usize = MAX_MEMBER_BYTES;
const MAX_JSON_DEPTH: usize = 64;
const MAX_JSON_STRING_BYTES: usize = 1024 * 1024;
const MAX_JSON_COLLECTION_ITEMS: usize = 1_000_000;
const MAX_JSON_VALUES: usize = 8_000_000;
const MAX_PAYLOAD_DEPTH: usize = 32;
const MAX_PAYLOAD_VALUES: usize = 1_000_000;
const MAX_TOKEN_BYTES: usize = 256;

/// Bounded decoder policy. Values above the shipped maxima are rejected so a
/// caller cannot silently weaken the fail-closed resource contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplayDecodeLimits {
    pub max_package_bytes: usize,
    pub max_member_bytes: usize,
    pub max_total_member_bytes: usize,
    pub max_events: usize,
    pub max_boundaries: usize,
    pub max_line_bytes: usize,
    pub max_json_depth: usize,
    pub max_json_string_bytes: usize,
    pub max_json_collection_items: usize,
    pub max_json_values: usize,
    pub max_payload_depth: usize,
    pub max_payload_values: usize,
}

impl ReplayDecodeLimits {
    #[must_use]
    pub const fn edu21() -> Self {
        Self {
            max_package_bytes: MAX_PACKAGE_BYTES,
            max_member_bytes: MAX_MEMBER_BYTES,
            max_total_member_bytes: MAX_TOTAL_MEMBER_BYTES,
            max_events: MAX_RECORDS,
            max_boundaries: MAX_RECORDS,
            max_line_bytes: MAX_LINE_BYTES,
            max_json_depth: MAX_JSON_DEPTH,
            max_json_string_bytes: MAX_JSON_STRING_BYTES,
            max_json_collection_items: MAX_JSON_COLLECTION_ITEMS,
            max_json_values: MAX_JSON_VALUES,
            max_payload_depth: MAX_PAYLOAD_DEPTH,
            max_payload_values: MAX_PAYLOAD_VALUES,
        }
    }

    fn validate(self) -> Result<(), ReplayPackageError> {
        let valid = self.max_package_bytes > 0
            && self.max_package_bytes <= MAX_PACKAGE_BYTES
            && self.max_member_bytes > 0
            && self.max_member_bytes <= MAX_MEMBER_BYTES
            && self.max_total_member_bytes > 0
            && self.max_total_member_bytes <= MAX_TOTAL_MEMBER_BYTES
            && self.max_events > 0
            && self.max_events <= MAX_RECORDS
            && self.max_boundaries > 0
            && self.max_boundaries <= MAX_RECORDS
            && self.max_line_bytes > 0
            && self.max_line_bytes <= MAX_LINE_BYTES
            && self.max_json_depth > 0
            && self.max_json_depth <= MAX_JSON_DEPTH
            && self.max_json_string_bytes > 0
            && self.max_json_string_bytes <= MAX_JSON_STRING_BYTES
            && self.max_json_collection_items > 0
            && self.max_json_collection_items <= MAX_JSON_COLLECTION_ITEMS
            && self.max_json_values > 0
            && self.max_json_values <= MAX_JSON_VALUES
            && self.max_payload_depth > 0
            && self.max_payload_depth <= MAX_PAYLOAD_DEPTH
            && self.max_payload_values > 0
            && self.max_payload_values <= MAX_PAYLOAD_VALUES;
        if valid {
            Ok(())
        } else {
            Err(ReplayPackageError::InvalidLimits)
        }
    }

    const fn json(self) -> JsonLimits {
        JsonLimits {
            max_depth: self.max_json_depth,
            max_string_bytes: self.max_json_string_bytes,
            max_collection_items: self.max_json_collection_items,
            max_total_values: self.max_json_values,
        }
    }
}

impl Default for ReplayDecodeLimits {
    fn default() -> Self {
        Self::edu21()
    }
}

/// Deterministic replay-package failure. No partial package or decoded event
/// collection is returned for any variant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplayPackageError {
    InvalidLimits,
    BadMagic,
    UnsupportedContainerVersion(u32),
    Truncated,
    TrailingData,
    IntegerOverflow,
    LimitExceeded(&'static str),
    ContainerIntegrityMismatch,
    MemberCount(usize),
    UnexpectedMember {
        index: usize,
        expected: &'static str,
        observed: String,
    },
    NonCanonicalJson(&'static str),
    InvalidJson(&'static str),
    InvalidManifest,
    InvalidSnapshotReference,
    InvalidEvent(usize),
    InvalidBoundary(usize),
    NonCanonicalEventOrder,
    NonCanonicalBoundaryOrder,
    InvalidSegmentStart(usize),
    InvalidSegmentPredecessor(usize),
    InvalidTimelineBranch(usize),
    MissingBoundaryForEvent(u64),
    OrphanBoundary(u64),
    PayloadHashMismatch(usize),
    ResultHashMismatch(usize),
    ContentFingerprintMismatch,
    NonCanonicalPackage,
    InvalidToken(&'static str),
    ZeroIdentity(&'static str),
    PlcType(String),
}

impl fmt::Display for ReplayPackageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimits => formatter.write_str("invalid replay decoder limits"),
            Self::BadMagic => formatter.write_str("invalid replay package magic"),
            Self::UnsupportedContainerVersion(version) => {
                write!(formatter, "unsupported replay container version {version}")
            }
            Self::Truncated => formatter.write_str("truncated replay package"),
            Self::TrailingData => formatter.write_str("trailing replay package data"),
            Self::IntegerOverflow => formatter.write_str("replay package integer overflow"),
            Self::LimitExceeded(name) => write!(formatter, "replay limit exceeded: {name}"),
            Self::ContainerIntegrityMismatch => {
                formatter.write_str("replay container integrity digest mismatch")
            }
            Self::MemberCount(count) => write!(formatter, "invalid replay member count {count}"),
            Self::UnexpectedMember {
                index,
                expected,
                observed,
            } => write!(
                formatter,
                "replay member {index} must be {expected}, observed {observed}"
            ),
            Self::NonCanonicalJson(member) => {
                write!(formatter, "noncanonical JSON in {member}")
            }
            Self::InvalidJson(member) => write!(formatter, "invalid JSON in {member}"),
            Self::InvalidManifest => formatter.write_str("invalid replay manifest"),
            Self::InvalidSnapshotReference => {
                formatter.write_str("invalid initial snapshot reference")
            }
            Self::InvalidEvent(index) => write!(formatter, "invalid replay event {index}"),
            Self::InvalidBoundary(index) => write!(formatter, "invalid boundary hash {index}"),
            Self::NonCanonicalEventOrder => {
                formatter.write_str("replay events are not in canonical order")
            }
            Self::NonCanonicalBoundaryOrder => {
                formatter.write_str("boundary hashes are not in canonical order")
            }
            Self::InvalidSegmentStart(index) => {
                write!(
                    formatter,
                    "replay segment {index} lacks its causal first event"
                )
            }
            Self::InvalidSegmentPredecessor(index) => {
                write!(
                    formatter,
                    "replay segment {index} has an invalid predecessor"
                )
            }
            Self::InvalidTimelineBranch(index) => {
                write!(
                    formatter,
                    "replay segment {index} has an invalid timeline branch"
                )
            }
            Self::MissingBoundaryForEvent(sequence) => {
                write!(
                    formatter,
                    "event {sequence} lacks its required boundary hash"
                )
            }
            Self::OrphanBoundary(sequence) => {
                write!(
                    formatter,
                    "boundary {sequence} lacks its causal replay event"
                )
            }
            Self::PayloadHashMismatch(index) => {
                write!(formatter, "typed payload hash mismatch at event {index}")
            }
            Self::ResultHashMismatch(index) => {
                write!(formatter, "typed result hash mismatch at event {index}")
            }
            Self::ContentFingerprintMismatch => {
                formatter.write_str("replay content fingerprint mismatch")
            }
            Self::NonCanonicalPackage => {
                formatter.write_str("replay package is not byte-canonical")
            }
            Self::InvalidToken(field) => write!(formatter, "invalid replay token: {field}"),
            Self::ZeroIdentity(field) => write!(formatter, "zero replay identity: {field}"),
            Self::PlcType(detail) => write!(formatter, "invalid canonical PLC value: {detail}"),
        }
    }
}

impl std::error::Error for ReplayPackageError {}

impl From<TypeError> for ReplayPackageError {
    fn from(value: TypeError) -> Self {
        Self::PlcType(value.to_string())
    }
}

/// Closed EDU-21 priority-table class in canonical scheduler order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReplayPriorityClass {
    ControllerLifecycle,
    ApprovedLoadSnapshot,
    VirtualHardwareNetworkFault,
    RawInput,
    ModifyForceBoundary,
    ScheduledProgram,
    OutputProcess,
    Publication,
}

impl ReplayPriorityClass {
    const fn token(self) -> &'static str {
        match self {
            Self::ControllerLifecycle => "CONTROLLER_LIFECYCLE",
            Self::ApprovedLoadSnapshot => "APPROVED_LOAD_SNAPSHOT",
            Self::VirtualHardwareNetworkFault => "VIRTUAL_HARDWARE_NETWORK_FAULT",
            Self::RawInput => "RAW_INPUT",
            Self::ModifyForceBoundary => "MODIFY_FORCE_BOUNDARY",
            Self::ScheduledProgram => "SCHEDULED_PROGRAM",
            Self::OutputProcess => "OUTPUT_PROCESS",
            Self::Publication => "PUBLICATION",
        }
    }

    fn from_token(value: &str) -> Result<Self, ReplayPackageError> {
        match value {
            "CONTROLLER_LIFECYCLE" => Ok(Self::ControllerLifecycle),
            "APPROVED_LOAD_SNAPSHOT" => Ok(Self::ApprovedLoadSnapshot),
            "VIRTUAL_HARDWARE_NETWORK_FAULT" => Ok(Self::VirtualHardwareNetworkFault),
            "RAW_INPUT" => Ok(Self::RawInput),
            "MODIFY_FORCE_BOUNDARY" => Ok(Self::ModifyForceBoundary),
            "SCHEDULED_PROGRAM" => Ok(Self::ScheduledProgram),
            "OUTPUT_PROCESS" => Ok(Self::OutputProcess),
            "PUBLICATION" => Ok(Self::Publication),
            _ => Err(ReplayPackageError::InvalidToken("event.priority")),
        }
    }
}

/// Stable actor class for a runtime-affecting replay command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActorKind {
    Operator,
    System,
    Replay,
}

impl ActorKind {
    const fn token(self) -> &'static str {
        match self {
            Self::Operator => "OPERATOR",
            Self::System => "SYSTEM",
            Self::Replay => "REPLAY",
        }
    }

    fn from_token(value: &str) -> Result<Self, ReplayPackageError> {
        match value {
            "OPERATOR" => Ok(Self::Operator),
            "SYSTEM" => Ok(Self::System),
            "REPLAY" => Ok(Self::Replay),
            _ => Err(ReplayPackageError::InvalidToken("actor.kind")),
        }
    }
}

/// Complete stable actor/command provenance retained with every event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplayActorProvenance {
    pub kind: ActorKind,
    pub actor_id: u128,
    pub command_id: u128,
    pub idempotency_key: u128,
}

/// Accepted/rejected result class for a runtime-affecting command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayResultStatus {
    Accepted,
    Rejected,
}

impl ReplayResultStatus {
    const fn token(self) -> &'static str {
        match self {
            Self::Accepted => "ACCEPTED",
            Self::Rejected => "REJECTED",
        }
    }

    fn from_token(value: &str) -> Result<Self, ReplayPackageError> {
        match value {
            "ACCEPTED" => Ok(Self::Accepted),
            "REJECTED" => Ok(Self::Rejected),
            _ => Err(ReplayPackageError::InvalidToken("result.status")),
        }
    }
}

/// Typed command result. `detail_hash` binds the ordinary runtime receipt or
/// rejection detail without permitting unbounded or executable prose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayCommandResult {
    pub status: ReplayResultStatus,
    pub code: String,
    pub detail: ReplayTypedPayload,
}

impl ReplayCommandResult {
    pub fn new(
        status: ReplayResultStatus,
        code: impl Into<String>,
        detail: ReplayTypedPayload,
    ) -> Result<Self, ReplayPackageError> {
        let value = Self {
            status,
            code: code.into(),
            detail,
        };
        validate_token(&value.code, "result.code")?;
        Ok(value)
    }

    /// Canonical typed receipt/rejection hash recorded independently from the
    /// runtime's native result hash.
    #[must_use]
    pub fn canonical_hash(&self) -> Hash32 {
        Sha256::digest(&canonical_json(&result_json(self)))
    }
}

/// Canonical PLC value encoded exactly as `{typeId,encoding,value}`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalReplayPlcValue {
    type_id: String,
    encoding: String,
    value: ReplayPlcNode,
}

impl CanonicalReplayPlcValue {
    /// Projects an existing runtime scalar through the canonical PLC type
    /// authority; no host-number JSON representation is admitted.
    pub fn from_runtime(value: CanonicalValue) -> Result<Self, ReplayPackageError> {
        let scalar = value
            .typed_scalar()
            .map_err(|error| ReplayPackageError::PlcType(format!("{error:?}")))?;
        Self::from_plc(
            &CanonicalType::Primitive(scalar.data_type()),
            &PlcValue::scalar(scalar),
            AggregateLimits::edu21(),
        )
    }

    /// Projects an existing canonical scalar/array/structure value in declared
    /// index/member order.
    pub fn from_plc(
        data_type: &CanonicalType,
        value: &PlcValue,
        limits: AggregateLimits,
    ) -> Result<Self, ReplayPackageError> {
        data_type.validate_value(value, limits)?;
        plc_value_from_existing(data_type, value, limits)
    }

    #[must_use]
    pub fn type_id(&self) -> &str {
        &self.type_id
    }

    #[must_use]
    pub fn encoding(&self) -> &str {
        &self.encoding
    }

    /// Returns the canonical standalone JSON object used inside payloads.
    #[must_use]
    pub fn canonical_json_bytes(&self) -> Vec<u8> {
        canonical_json(&plc_value_json(self))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ReplayPlcNode {
    Bool(bool),
    Text(String),
    CharBytes(Vec<u8>),
    Array(Vec<CanonicalReplayPlcValue>),
    Struct(Vec<ReplayStructField>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ReplayStructField {
    member_id: StableUuid,
    value: CanonicalReplayPlcValue,
}

/// Closed typed payload value. There is no code, host handle, endpoint, map
/// iteration, or UI-action variant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplayPayloadValue {
    Null,
    Bool(bool),
    Decimal(String),
    Hex(String),
    Text(String),
    Identity(u128),
    Hash(Hash32),
    Plc(CanonicalReplayPlcValue),
    Array(Vec<Self>),
    Object(BTreeMap<String, Self>),
}

/// Schema-bound typed payload for one runtime/lifecycle event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayTypedPayload {
    pub event_kind: ReplayEventKind,
    pub fields: BTreeMap<String, ReplayPayloadValue>,
}

impl ReplayTypedPayload {
    pub fn new(
        event_kind: ReplayEventKind,
        fields: BTreeMap<String, ReplayPayloadValue>,
    ) -> Result<Self, ReplayPackageError> {
        let value = Self { event_kind, fields };
        validate_payload(&value, ReplayDecodeLimits::edu21())?;
        Ok(value)
    }

    #[must_use]
    pub fn canonical_hash(&self) -> Hash32 {
        Sha256::digest(&canonical_json(&payload_json(self)))
    }
}

/// Full semantic event record. The existing runtime event hashes remain
/// provenance while the typed payload/result hashes are independently bound.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayPackageEvent {
    pub segment: ReplaySegment,
    /// Exact preceding segment for this controller when this record opens a
    /// new segment. Same-segment records and the first segment represented by
    /// a package use `None`.
    pub segment_predecessor: Option<ReplaySegment>,
    /// Last recorded event in `segment_predecessor`. This binds a segment
    /// transition to one causal record instead of merely naming an epoch.
    pub predecessor_event_sequence: Option<u64>,
    /// True only for the required non-restored-controller branch record when
    /// a universe timeline is replaced without changing controller epoch.
    pub universe_timeline_branch: bool,
    pub artifact_hash: Hash32,
    pub profile_hash: Hash32,
    pub kind: ReplayEventKind,
    pub event_sequence: u64,
    pub virtual_timestamp_ms: u64,
    pub priority: ReplayPriorityClass,
    pub actor: ReplayActorProvenance,
    pub payload: ReplayTypedPayload,
    pub result: ReplayCommandResult,
    pub runtime_payload_hash: Hash32,
    pub runtime_result_hash: Hash32,
}

impl ReplayPackageEvent {
    /// Binds an existing runtime replay event to the typed payload, actor, and
    /// result that must be captured at the command-ingress boundary.
    #[must_use]
    pub fn from_runtime(
        event: &ReplayEvent,
        artifact_hash: Hash32,
        profile_hash: Hash32,
        priority: ReplayPriorityClass,
        actor: ReplayActorProvenance,
        payload: ReplayTypedPayload,
        result: ReplayCommandResult,
    ) -> Self {
        Self {
            segment: event.segment,
            segment_predecessor: None,
            predecessor_event_sequence: None,
            universe_timeline_branch: false,
            artifact_hash,
            profile_hash,
            kind: event.kind,
            event_sequence: event.event_sequence,
            virtual_timestamp_ms: event.virtual_timestamp_ms,
            priority,
            actor,
            payload,
            result,
            runtime_payload_hash: event.payload_hash,
            runtime_result_hash: event.result_hash,
        }
    }

    /// Links the first event of a later segment to the exact last event of
    /// this controller's preceding segment. `universe_timeline_branch` is
    /// reserved for a controller whose epoch did not change while the
    /// universe epoch did.
    #[must_use]
    pub const fn linked_from(
        mut self,
        predecessor: ReplaySegment,
        predecessor_event_sequence: u64,
        universe_timeline_branch: bool,
    ) -> Self {
        self.segment_predecessor = Some(predecessor);
        self.predecessor_event_sequence = Some(predecessor_event_sequence);
        self.universe_timeline_branch = universe_timeline_branch;
        self
    }
}

/// Semantic state regions retained at every verified boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReplayStateRegion {
    /// Aggregate runtime state hash emitted by the runtime boundary itself.
    Runtime,
    Cpu,
    Memory,
    Io,
    TimersCountersEdges,
    Diagnostics,
    Forces,
    Trace,
    /// Hash of the exact canonical event JSONL prefix through this boundary.
    EventOrder,
}

impl ReplayStateRegion {
    const DETAILED: [Self; 7] = [
        Self::Cpu,
        Self::Memory,
        Self::Io,
        Self::TimersCountersEdges,
        Self::Diagnostics,
        Self::Forces,
        Self::Trace,
    ];

    const ALL: [Self; 9] = [
        Self::Runtime,
        Self::Cpu,
        Self::Memory,
        Self::Io,
        Self::TimersCountersEdges,
        Self::Diagnostics,
        Self::Forces,
        Self::Trace,
        Self::EventOrder,
    ];

    const fn token(self) -> &'static str {
        match self {
            Self::Runtime => "runtime",
            Self::Cpu => "cpu",
            Self::Memory => "memory",
            Self::Io => "io",
            Self::TimersCountersEdges => "timersCountersEdges",
            Self::Diagnostics => "diagnostics",
            Self::Forces => "forces",
            Self::Trace => "trace",
            Self::EventOrder => "eventOrder",
        }
    }

    fn from_token(value: &str) -> Result<Self, ReplayPackageError> {
        match value {
            "runtime" => Ok(Self::Runtime),
            "cpu" => Ok(Self::Cpu),
            "memory" => Ok(Self::Memory),
            "io" => Ok(Self::Io),
            "timersCountersEdges" => Ok(Self::TimersCountersEdges),
            "diagnostics" => Ok(Self::Diagnostics),
            "forces" => Ok(Self::Forces),
            "trace" => Ok(Self::Trace),
            "eventOrder" => Ok(Self::EventOrder),
            _ => Err(ReplayPackageError::InvalidToken("boundary.region")),
        }
    }
}

/// Only boundary classes required by PES-SNP-0012 are serializable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayBoundaryKind {
    ScanEnd,
    FatalFault,
}

impl ReplayBoundaryKind {
    const fn token(self) -> &'static str {
        match self {
            Self::ScanEnd => "SCAN_END",
            Self::FatalFault => "FATAL_FAULT",
        }
    }

    fn from_token(value: &str) -> Result<Self, ReplayPackageError> {
        match value {
            "SCAN_END" => Ok(Self::ScanEnd),
            "FATAL_FAULT" => Ok(Self::FatalFault),
            _ => Err(ReplayPackageError::InvalidToken("boundary.kind")),
        }
    }
}

/// Boundary hash plus exact per-region hashes and the causal event sequence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayBoundaryHash {
    pub segment: ReplaySegment,
    pub kind: ReplayBoundaryKind,
    pub event_sequence: u64,
    pub causal_input_event_sequence: u64,
    pub scan_sequence: u64,
    pub virtual_timestamp_ms: u64,
    pub runtime_state_hash: Hash32,
    pub semantic_state_hash: Hash32,
    pub region_hashes: BTreeMap<ReplayStateRegion, Hash32>,
}

impl ReplayBoundaryHash {
    /// Adapts a runtime scan/fatal boundary while requiring independently
    /// calculated complete state-region hashes, the canonical event prefix,
    /// and causal event identity. The caller supplies the seven independent
    /// detailed regions; runtime aggregate and event-order regions are bound
    /// here from their authoritative inputs.
    pub fn from_runtime(
        boundary: &BoundaryHash,
        event_sequence: u64,
        causal_input_event_sequence: u64,
        mut region_hashes: BTreeMap<ReplayStateRegion, Hash32>,
        events: &[ReplayPackageEvent],
    ) -> Result<Self, ReplayPackageError> {
        let kind = if boundary.is_scan_end() {
            ReplayBoundaryKind::ScanEnd
        } else if boundary.is_fatal_fault() {
            ReplayBoundaryKind::FatalFault
        } else {
            return Err(ReplayPackageError::InvalidToken("runtime.boundary.kind"));
        };
        validate_detailed_region_hashes(&region_hashes)?;
        region_hashes.insert(ReplayStateRegion::Runtime, boundary.state_hash);
        let mut value = Self {
            segment: boundary.segment,
            kind,
            event_sequence,
            causal_input_event_sequence,
            scan_sequence: boundary.scan_sequence,
            virtual_timestamp_ms: boundary.virtual_timestamp_ms,
            runtime_state_hash: boundary.state_hash,
            semantic_state_hash: Hash32::ZERO,
            region_hashes,
        };
        value.bind_event_order(events)?;
        Ok(value)
    }

    /// Binds the exact canonical event prefix through this boundary and
    /// recomputes the aggregate semantic hash. This is intentionally explicit:
    /// package encoding rejects caller-invented or stale event-order hashes.
    pub fn bind_event_order(
        &mut self,
        events: &[ReplayPackageEvent],
    ) -> Result<(), ReplayPackageError> {
        validate_detailed_region_hashes(&self.region_hashes)?;
        self.region_hashes
            .insert(ReplayStateRegion::Runtime, self.runtime_state_hash);
        self.region_hashes.insert(
            ReplayStateRegion::EventOrder,
            event_order_region_hash(events, self)?,
        );
        validate_region_hashes(&self.region_hashes)?;
        self.semantic_state_hash = semantic_region_hash(&self.region_hashes);
        Ok(())
    }
}

/// Complete canonical input required to create one replay package.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayPackageSpec {
    pub initial_snapshot_hash: Hash32,
    pub artifact_hash: Hash32,
    pub profile_hash: Hash32,
    pub deterministic_seed: u64,
    pub deterministic_algorithm: String,
    pub runtime_version: String,
    pub scheduler_version: String,
    pub events: Vec<ReplayPackageEvent>,
    pub boundaries: Vec<ReplayBoundaryHash>,
}

impl ReplayPackageSpec {
    #[must_use]
    pub fn edu21(
        snapshot: &EngineeringSessionSnapshot,
        artifact_hash: Hash32,
        profile_hash: Hash32,
        deterministic_seed: u64,
        deterministic_algorithm: impl Into<String>,
        events: Vec<ReplayPackageEvent>,
        boundaries: Vec<ReplayBoundaryHash>,
    ) -> Self {
        Self {
            initial_snapshot_hash: snapshot.content_hash,
            artifact_hash,
            profile_hash,
            deterministic_seed,
            deterministic_algorithm: deterministic_algorithm.into(),
            runtime_version: RUNTIME_SEMANTICS_VERSION.to_owned(),
            scheduler_version: SCHEDULER_VERSION.to_owned(),
            events,
            boundaries,
        }
    }

    /// Binds every boundary to the canonical event prefix in this spec. The
    /// returned spec is ready for fail-closed encoding; a missing causal
    /// boundary event is reported rather than repaired or guessed.
    pub fn bind_event_order(mut self) -> Result<Self, ReplayPackageError> {
        validate_event_order(&self.events)?;
        for boundary in &mut self.boundaries {
            boundary.bind_event_order(&self.events)?;
        }
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ReplayManifest {
    artifact_hash: Hash32,
    boundary_count: usize,
    content_fingerprint: Hash32,
    deterministic_algorithm: String,
    deterministic_seed: u64,
    event_count: usize,
    initial_snapshot_hash: Hash32,
    profile_hash: Hash32,
    priority_table_version: String,
    runtime_version: String,
    scheduler_version: String,
    work_cost_version: String,
}

/// Borrowed fixed-order logical member.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplayMemberRef<'a> {
    pub name: &'static str,
    pub bytes: &'a [u8],
}

/// Decoded canonical package. Its original bytes are retained so callers can
/// persist the exact verified representation without reserialization drift.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayPackage {
    manifest: ReplayManifest,
    members: [Vec<u8>; 4],
    events: Vec<ReplayPackageEvent>,
    boundaries: Vec<ReplayBoundaryHash>,
    encoded: Vec<u8>,
}

impl ReplayPackage {
    /// Encodes only after every event, boundary, segment, payload, and limit
    /// invariant has been validated.
    pub fn encode(spec: ReplayPackageSpec) -> Result<Self, ReplayPackageError> {
        encode_package(spec, ReplayDecodeLimits::edu21())
    }

    /// Decodes only the exact four-member canonical representation.
    pub fn decode(bytes: &[u8], limits: ReplayDecodeLimits) -> Result<Self, ReplayPackageError> {
        decode_package(bytes, limits)
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.encoded
    }

    #[must_use]
    pub const fn content_fingerprint(&self) -> Hash32 {
        self.manifest.content_fingerprint
    }

    #[must_use]
    pub const fn initial_snapshot_hash(&self) -> Hash32 {
        self.manifest.initial_snapshot_hash
    }

    #[must_use]
    pub fn events(&self) -> &[ReplayPackageEvent] {
        &self.events
    }

    #[must_use]
    pub fn boundaries(&self) -> &[ReplayBoundaryHash] {
        &self.boundaries
    }

    #[must_use]
    pub fn members(&self) -> [ReplayMemberRef<'_>; 4] {
        std::array::from_fn(|index| ReplayMemberRef {
            name: MEMBER_NAMES[index],
            bytes: &self.members[index],
        })
    }

    /// Compares in canonical boundary order and returns only the first
    /// divergence, its differing regions, and its causal event.
    pub fn first_divergence(
        &self,
        observed: &[ReplayBoundaryHash],
    ) -> Result<Option<ReplayDivergence>, ReplayPackageError> {
        let count = self.boundaries.len().max(observed.len());
        for index in 0..count {
            let expected = self.boundaries.get(index);
            let actual = observed.get(index);
            if expected == actual {
                continue;
            }
            return Ok(Some(self.divergence(index, expected, actual, None)));
        }
        Ok(None)
    }

    /// Drives a verification executor in canonical event order. The executor
    /// returns a boundary only when the applied event reaches a scan-end or
    /// fatal-fault boundary. Comparison happens immediately and execution
    /// stops before the event following the first divergence is dispatched.
    ///
    /// Host pacing is deliberately outside this capability-free driver. A
    /// caller proving host-speed equivalence must run this method under each
    /// scheduling/pacing mode and compare the resulting package boundaries.
    pub fn verify_with<F>(
        &self,
        mut execute: F,
    ) -> Result<Option<ReplayDivergence>, ReplayPackageError>
    where
        F: FnMut(&ReplayPackageEvent) -> Result<Option<ReplayBoundaryHash>, ReplayPackageError>,
    {
        let mut boundary_index = 0;
        for event in &self.events {
            let observed = execute(event)?;
            let expected = self.boundaries.get(boundary_index).filter(|boundary| {
                boundary.segment == event.segment && boundary.event_sequence == event.event_sequence
            });
            match (expected, observed.as_ref()) {
                (None, None) => {}
                (Some(expected), Some(observed)) if expected == observed => {
                    boundary_index += 1;
                }
                (Some(expected), actual) => {
                    return Ok(Some(self.divergence(
                        boundary_index,
                        Some(expected),
                        actual,
                        None,
                    )));
                }
                (None, Some(actual)) => {
                    return Ok(Some(self.divergence(
                        boundary_index,
                        None,
                        Some(actual),
                        Some(event),
                    )));
                }
            }
        }
        if boundary_index != self.boundaries.len() {
            let expected = self.boundaries.get(boundary_index);
            return Ok(Some(self.divergence(boundary_index, expected, None, None)));
        }
        Ok(None)
    }

    fn divergence(
        &self,
        boundary_index: usize,
        expected: Option<&ReplayBoundaryHash>,
        observed: Option<&ReplayBoundaryHash>,
        causal_override: Option<&ReplayPackageEvent>,
    ) -> ReplayDivergence {
        let causal_event = causal_override
            .or_else(|| expected.and_then(|boundary| event_for_boundary(&self.events, boundary)))
            .cloned();
        let differing_regions = match (expected, observed) {
            (Some(left), Some(right)) => {
                let mut regions = ReplayStateRegion::ALL
                    .into_iter()
                    .filter(|region| {
                        left.region_hashes.get(region) != right.region_hashes.get(region)
                    })
                    .collect::<Vec<_>>();
                if regions.is_empty() && left != right {
                    // Boundary identity/timing metadata diverged even though
                    // every supplied state region matched. No narrower state
                    // claim is truthful, so report all declared regions.
                    regions.extend(ReplayStateRegion::ALL);
                }
                regions
            }
            _ => ReplayStateRegion::ALL.to_vec(),
        };
        ReplayDivergence {
            boundary_index,
            expected_state_hash: expected.map(|value| value.semantic_state_hash),
            observed_state_hash: observed.map(|value| value.semantic_state_hash),
            differing_regions,
            causal_event,
        }
    }
}

/// First verification-replay divergence. Later boundaries are deliberately not
/// inspected once this record is produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayDivergence {
    pub boundary_index: usize,
    pub expected_state_hash: Option<Hash32>,
    pub observed_state_hash: Option<Hash32>,
    pub differing_regions: Vec<ReplayStateRegion>,
    pub causal_event: Option<ReplayPackageEvent>,
}

fn encode_package(
    spec: ReplayPackageSpec,
    limits: ReplayDecodeLimits,
) -> Result<ReplayPackage, ReplayPackageError> {
    limits.validate()?;
    validate_spec(&spec, limits)?;
    let events = encode_events(&spec.events);
    let boundaries = encode_boundaries(&spec.boundaries);
    let snapshot_reference = snapshot_reference(spec.initial_snapshot_hash);
    let mut manifest = ReplayManifest {
        artifact_hash: spec.artifact_hash,
        boundary_count: spec.boundaries.len(),
        content_fingerprint: Hash32::ZERO,
        deterministic_algorithm: spec.deterministic_algorithm,
        deterministic_seed: spec.deterministic_seed,
        event_count: spec.events.len(),
        initial_snapshot_hash: spec.initial_snapshot_hash,
        profile_hash: spec.profile_hash,
        priority_table_version: PRIORITY_TABLE_VERSION.to_owned(),
        runtime_version: spec.runtime_version,
        scheduler_version: spec.scheduler_version,
        work_cost_version: WORK_COST_VERSION.to_owned(),
    };
    let identity_manifest = canonical_json(&manifest_json(&manifest, false));
    let identity_members = [
        identity_manifest,
        snapshot_reference.clone(),
        events.clone(),
        boundaries.clone(),
    ];
    manifest.content_fingerprint = Sha256::digest(&container_payload(&identity_members)?);
    let members = [
        canonical_json(&manifest_json(&manifest, true)),
        snapshot_reference,
        events,
        boundaries,
    ];
    validate_member_sizes(&members, limits)?;
    let payload = container_payload(&members)?;
    let mut encoded = payload.clone();
    encoded.extend_from_slice(Sha256::digest(&payload).as_bytes());
    if encoded.len() > limits.max_package_bytes {
        return Err(ReplayPackageError::LimitExceeded("package bytes"));
    }
    Ok(ReplayPackage {
        manifest,
        members,
        events: spec.events,
        boundaries: spec.boundaries,
        encoded,
    })
}

fn decode_package(
    bytes: &[u8],
    limits: ReplayDecodeLimits,
) -> Result<ReplayPackage, ReplayPackageError> {
    limits.validate()?;
    let members = decode_container(bytes, limits)?;
    let manifest_value = parse_json(&members[0], limits.json())
        .map_err(|()| ReplayPackageError::InvalidJson(MEMBER_NAMES[0]))?;
    if canonical_json(&manifest_value) != members[0] {
        return Err(ReplayPackageError::NonCanonicalJson(MEMBER_NAMES[0]));
    }
    let manifest = manifest_from_json(&manifest_value)?;
    let snapshot_hash = parse_snapshot_reference(&members[1])?;
    if snapshot_hash != manifest.initial_snapshot_hash {
        return Err(ReplayPackageError::InvalidSnapshotReference);
    }
    let events = decode_events(&members[2], limits)?;
    let boundaries = decode_boundaries(&members[3], limits)?;
    if events.len() != manifest.event_count || boundaries.len() != manifest.boundary_count {
        return Err(ReplayPackageError::InvalidManifest);
    }
    let spec = ReplayPackageSpec {
        initial_snapshot_hash: manifest.initial_snapshot_hash,
        artifact_hash: manifest.artifact_hash,
        profile_hash: manifest.profile_hash,
        deterministic_seed: manifest.deterministic_seed,
        deterministic_algorithm: manifest.deterministic_algorithm.clone(),
        runtime_version: manifest.runtime_version.clone(),
        scheduler_version: manifest.scheduler_version.clone(),
        events: events.clone(),
        boundaries: boundaries.clone(),
    };
    validate_spec(&spec, limits)?;

    let identity_manifest = canonical_json(&manifest_json(&manifest, false));
    let identity_members = [
        identity_manifest,
        members[1].clone(),
        members[2].clone(),
        members[3].clone(),
    ];
    if Sha256::digest(&container_payload(&identity_members)?) != manifest.content_fingerprint {
        return Err(ReplayPackageError::ContentFingerprintMismatch);
    }
    let canonical = encode_package(spec, limits)?;
    if canonical.encoded != bytes {
        return Err(ReplayPackageError::NonCanonicalPackage);
    }
    Ok(ReplayPackage {
        manifest,
        members,
        events,
        boundaries,
        encoded: bytes.to_vec(),
    })
}

fn validate_spec(
    spec: &ReplayPackageSpec,
    limits: ReplayDecodeLimits,
) -> Result<(), ReplayPackageError> {
    for (name, hash) in [
        ("initialSnapshotHash", spec.initial_snapshot_hash),
        ("artifactHash", spec.artifact_hash),
        ("profileHash", spec.profile_hash),
    ] {
        if hash == Hash32::ZERO {
            return Err(ReplayPackageError::ZeroIdentity(name));
        }
    }
    validate_token(&spec.deterministic_algorithm, "manifest.algorithm")?;
    validate_token(&spec.runtime_version, "manifest.runtimeVersion")?;
    validate_token(&spec.scheduler_version, "manifest.schedulerVersion")?;
    if spec.runtime_version != RUNTIME_SEMANTICS_VERSION
        || spec.scheduler_version != SCHEDULER_VERSION
    {
        return Err(ReplayPackageError::InvalidManifest);
    }
    if spec.events.len() > limits.max_events {
        return Err(ReplayPackageError::LimitExceeded("event count"));
    }
    if spec.boundaries.len() > limits.max_boundaries {
        return Err(ReplayPackageError::LimitExceeded("boundary count"));
    }
    for event in &spec.events {
        if event.segment.universe_id.0 == 0
            || event.segment.universe_epoch == 0
            || event.segment.controller_id.0 == 0
            || event.segment.controller_epoch == 0
            || event.event_sequence == 0
            || event.actor.actor_id == 0
            || event.actor.command_id == 0
            || event.actor.idempotency_key == 0
            || event.runtime_payload_hash == Hash32::ZERO
            || event.runtime_result_hash == Hash32::ZERO
            || event.artifact_hash == Hash32::ZERO
            || event.profile_hash == Hash32::ZERO
        {
            return Err(ReplayPackageError::ZeroIdentity("event"));
        }
        validate_payload(&event.payload, limits)?;
        validate_payload(&event.result.detail, limits)?;
        if event.payload.event_kind != event.kind || event.result.detail.event_kind != event.kind {
            return Err(ReplayPackageError::InvalidToken("event.payload.schema"));
        }
        if matches!(event.kind, ReplayEventKind::CommandRejected)
            != matches!(event.result.status, ReplayResultStatus::Rejected)
        {
            return Err(ReplayPackageError::InvalidToken("event.result.status"));
        }
        validate_token(&event.result.code, "result.code")?;
    }
    if let Some(first) = spec.events.first()
        && (first.artifact_hash != spec.artifact_hash || first.profile_hash != spec.profile_hash)
    {
        return Err(ReplayPackageError::InvalidManifest);
    }
    validate_event_order(&spec.events)?;
    validate_boundary_order(&spec.boundaries)?;
    validate_boundary_coverage(&spec.events, &spec.boundaries)
}

#[derive(Clone, Copy)]
struct ReplayControllerCursor {
    segment: ReplaySegment,
    artifact_hash: Hash32,
    profile_hash: Hash32,
    event_sequence: u64,
    virtual_timestamp_ms: u64,
}

fn validate_event_order(events: &[ReplayPackageEvent]) -> Result<(), ReplayPackageError> {
    let mut previous_order: Option<(u64, u64)> = None;
    let mut universe_id = None;
    let mut controllers: BTreeMap<plc_runtime::VirtualControllerId, ReplayControllerCursor> =
        BTreeMap::new();
    for (index, event) in events.iter().enumerate() {
        if universe_id.is_some_and(|value| value != event.segment.universe_id) {
            return Err(ReplayPackageError::NonCanonicalEventOrder);
        }
        universe_id = Some(event.segment.universe_id);
        let order = (event.segment.universe_epoch, event.event_sequence);
        if let Some(previous) = previous_order {
            if previous.0 == order.0 {
                if previous.1.checked_add(1) != Some(order.1) {
                    return Err(ReplayPackageError::NonCanonicalEventOrder);
                }
            } else if previous.0.checked_add(1) != Some(order.0) || order.1 != 1 {
                return Err(ReplayPackageError::NonCanonicalEventOrder);
            }
        }
        let key = event.segment.controller_id;
        if let Some(previous) = controllers.get(&key).copied() {
            validate_controller_transition(index, event, previous)?;
        } else {
            validate_initial_controller_event(index, event)?;
        }
        controllers.insert(
            key,
            ReplayControllerCursor {
                segment: event.segment,
                artifact_hash: event.artifact_hash,
                profile_hash: event.profile_hash,
                event_sequence: event.event_sequence,
                virtual_timestamp_ms: event.virtual_timestamp_ms,
            },
        );
        previous_order = Some(order);
    }
    Ok(())
}

fn validate_controller_transition(
    index: usize,
    event: &ReplayPackageEvent,
    previous: ReplayControllerCursor,
) -> Result<(), ReplayPackageError> {
    let changed_segment = previous.segment.universe_epoch != event.segment.universe_epoch
        || previous.segment.controller_epoch != event.segment.controller_epoch;
    if !changed_segment {
        if event.segment_predecessor.is_some() || event.predecessor_event_sequence.is_some() {
            return Err(ReplayPackageError::InvalidSegmentPredecessor(index));
        }
        if event.universe_timeline_branch {
            return Err(ReplayPackageError::InvalidTimelineBranch(index));
        }
        if previous.artifact_hash != event.artifact_hash
            || previous.profile_hash != event.profile_hash
        {
            return Err(ReplayPackageError::InvalidSegmentStart(index));
        }
        if previous.virtual_timestamp_ms > event.virtual_timestamp_ms {
            return Err(ReplayPackageError::NonCanonicalEventOrder);
        }
        return Ok(());
    }
    if event.segment_predecessor != Some(previous.segment)
        || event.predecessor_event_sequence != Some(previous.event_sequence)
    {
        return Err(ReplayPackageError::InvalidSegmentPredecessor(index));
    }
    validate_changed_segment(index, event, previous)
}

fn validate_changed_segment(
    index: usize,
    event: &ReplayPackageEvent,
    previous: ReplayControllerCursor,
) -> Result<(), ReplayPackageError> {
    let universe_changed = previous.segment.universe_epoch != event.segment.universe_epoch;
    let controller_changed = previous.segment.controller_epoch != event.segment.controller_epoch;
    if universe_changed
        && previous.segment.universe_epoch.checked_add(1) != Some(event.segment.universe_epoch)
    {
        return Err(ReplayPackageError::InvalidSegmentStart(index));
    }
    if controller_changed
        && previous.segment.controller_epoch.checked_add(1) != Some(event.segment.controller_epoch)
    {
        return Err(ReplayPackageError::InvalidSegmentStart(index));
    }
    if universe_changed && !controller_changed {
        if !is_timeline_branch(event, previous.segment) {
            return Err(ReplayPackageError::InvalidTimelineBranch(index));
        }
    } else if event.universe_timeline_branch {
        return Err(ReplayPackageError::InvalidTimelineBranch(index));
    } else if !is_causal_segment_start(event.kind) {
        return Err(ReplayPackageError::InvalidSegmentStart(index));
    }
    if previous.profile_hash != event.profile_hash
        && !matches!(event.kind, ReplayEventKind::InstanceReplaced)
    {
        return Err(ReplayPackageError::InvalidSegmentStart(index));
    }
    if previous.artifact_hash != event.artifact_hash
        && !matches!(
            event.kind,
            ReplayEventKind::ArtifactInstalled | ReplayEventKind::InstanceReplaced
        )
    {
        return Err(ReplayPackageError::InvalidSegmentStart(index));
    }
    Ok(())
}

fn validate_initial_controller_event(
    index: usize,
    event: &ReplayPackageEvent,
) -> Result<(), ReplayPackageError> {
    match (event.segment_predecessor, event.predecessor_event_sequence) {
        (None, None) if !event.universe_timeline_branch => Ok(()),
        (Some(previous), Some(previous_sequence))
            if previous_sequence > 0 && is_timeline_branch(event, previous) =>
        {
            Ok(())
        }
        (Some(_), Some(_)) => Err(ReplayPackageError::InvalidTimelineBranch(index)),
        _ => Err(ReplayPackageError::InvalidSegmentPredecessor(index)),
    }
}

fn validate_boundary_order(boundaries: &[ReplayBoundaryHash]) -> Result<(), ReplayPackageError> {
    let mut previous: Option<(u64, u64)> = None;
    for (index, boundary) in boundaries.iter().enumerate() {
        validate_region_hashes(&boundary.region_hashes)?;
        if boundary.segment.universe_id.0 == 0
            || boundary.segment.universe_epoch == 0
            || boundary.segment.controller_id.0 == 0
            || boundary.segment.controller_epoch == 0
            || boundary.event_sequence == 0
            || boundary.causal_input_event_sequence == 0
            || boundary.runtime_state_hash == Hash32::ZERO
            || boundary.semantic_state_hash == Hash32::ZERO
            || boundary
                .region_hashes
                .values()
                .any(|hash| *hash == Hash32::ZERO)
        {
            return Err(ReplayPackageError::ZeroIdentity("boundary"));
        }
        if semantic_region_hash(&boundary.region_hashes) != boundary.semantic_state_hash
            || boundary.region_hashes.get(&ReplayStateRegion::Runtime)
                != Some(&boundary.runtime_state_hash)
        {
            return Err(ReplayPackageError::InvalidBoundary(index));
        }
        let order = (boundary.segment.universe_epoch, boundary.event_sequence);
        if previous.is_some_and(|value| value >= order) {
            return Err(ReplayPackageError::NonCanonicalBoundaryOrder);
        }
        previous = Some(order);
    }
    Ok(())
}

fn validate_boundary_coverage(
    events: &[ReplayPackageEvent],
    boundaries: &[ReplayBoundaryHash],
) -> Result<(), ReplayPackageError> {
    let mut expected = BTreeMap::new();
    for event in events {
        let kind = match event.kind {
            ReplayEventKind::ScanCompleted => Some(ReplayBoundaryKind::ScanEnd),
            ReplayEventKind::FatalFault => Some(ReplayBoundaryKind::FatalFault),
            _ => None,
        };
        if let Some(kind) = kind {
            expected.insert((event.segment, event.event_sequence), kind);
        }
    }
    for (boundary_index, boundary) in boundaries.iter().enumerate() {
        let key = (boundary.segment, boundary.event_sequence);
        let Some(kind) = expected.remove(&key) else {
            return Err(ReplayPackageError::OrphanBoundary(boundary.event_sequence));
        };
        if kind != boundary.kind {
            return Err(ReplayPackageError::OrphanBoundary(boundary.event_sequence));
        }
        let causal = events.iter().find(|event| {
            event.segment == boundary.segment
                && event.event_sequence == boundary.causal_input_event_sequence
        });
        if causal.is_none_or(|event| {
            event.result.status != ReplayResultStatus::Accepted
                || matches!(
                    event.kind,
                    ReplayEventKind::ScanCompleted
                        | ReplayEventKind::FatalFault
                        | ReplayEventKind::ObservationBoundary
                )
        }) {
            return Err(ReplayPackageError::OrphanBoundary(
                boundary.causal_input_event_sequence,
            ));
        }
        let later_ingress_exists = events.iter().any(|event| {
            event.segment == boundary.segment
                && event.event_sequence > boundary.causal_input_event_sequence
                && event.event_sequence < boundary.event_sequence
                && event.result.status == ReplayResultStatus::Accepted
                && !matches!(
                    event.kind,
                    ReplayEventKind::ScanCompleted
                        | ReplayEventKind::FatalFault
                        | ReplayEventKind::ObservationBoundary
                )
        });
        if later_ingress_exists {
            return Err(ReplayPackageError::OrphanBoundary(
                boundary.causal_input_event_sequence,
            ));
        }
        if boundary.region_hashes.get(&ReplayStateRegion::EventOrder)
            != Some(&event_order_region_hash(events, boundary)?)
        {
            return Err(ReplayPackageError::InvalidBoundary(boundary_index));
        }
    }
    if let Some(((_, sequence), _)) = expected.into_iter().next() {
        return Err(ReplayPackageError::MissingBoundaryForEvent(sequence));
    }
    Ok(())
}

fn validate_region_hashes(
    regions: &BTreeMap<ReplayStateRegion, Hash32>,
) -> Result<(), ReplayPackageError> {
    if regions.len() != ReplayStateRegion::ALL.len()
        || ReplayStateRegion::ALL
            .iter()
            .any(|region| !regions.contains_key(region))
    {
        return Err(ReplayPackageError::InvalidToken("boundary.regionHashes"));
    }
    Ok(())
}

fn validate_detailed_region_hashes(
    regions: &BTreeMap<ReplayStateRegion, Hash32>,
) -> Result<(), ReplayPackageError> {
    if ReplayStateRegion::DETAILED
        .iter()
        .any(|region| !regions.contains_key(region))
        || regions.keys().any(|region| {
            !ReplayStateRegion::DETAILED.contains(region)
                && !matches!(
                    region,
                    ReplayStateRegion::Runtime | ReplayStateRegion::EventOrder
                )
        })
    {
        return Err(ReplayPackageError::InvalidToken("boundary.regionHashes"));
    }
    Ok(())
}

fn is_causal_segment_start(kind: ReplayEventKind) -> bool {
    matches!(
        kind,
        ReplayEventKind::ArtifactInstalled
            | ReplayEventKind::PowerCycle
            | ReplayEventKind::MemoryReset
            | ReplayEventKind::SnapshotRestored
            | ReplayEventKind::InstanceCloned
            | ReplayEventKind::InstanceReplaced
    )
}

fn is_timeline_branch(event: &ReplayPackageEvent, predecessor: ReplaySegment) -> bool {
    predecessor.universe_epoch > 0
        && predecessor.controller_epoch > 0
        && event.universe_timeline_branch
        && matches!(event.kind, ReplayEventKind::ObservationBoundary)
        && event.priority == ReplayPriorityClass::ControllerLifecycle
        && event.actor.kind == ActorKind::System
        && predecessor.universe_id == event.segment.universe_id
        && predecessor.controller_id == event.segment.controller_id
        && predecessor.universe_epoch.checked_add(1) == Some(event.segment.universe_epoch)
        && predecessor.controller_epoch == event.segment.controller_epoch
}

fn event_for_boundary<'a>(
    events: &'a [ReplayPackageEvent],
    boundary: &ReplayBoundaryHash,
) -> Option<&'a ReplayPackageEvent> {
    events.iter().find(|event| {
        event.segment == boundary.segment
            && event.event_sequence == boundary.causal_input_event_sequence
    })
}

fn event_order_region_hash(
    events: &[ReplayPackageEvent],
    boundary: &ReplayBoundaryHash,
) -> Result<Hash32, ReplayPackageError> {
    let boundary_order = (boundary.segment.universe_epoch, boundary.event_sequence);
    let mut bytes = b"PES-REPLAY-EVENT-ORDER-1\0".to_vec();
    let mut found = false;
    for event in events {
        let order = (event.segment.universe_epoch, event.event_sequence);
        if order > boundary_order {
            break;
        }
        let json = canonical_json(&event_json(event));
        bytes.extend_from_slice(
            &u64::try_from(json.len())
                .map_err(|_| ReplayPackageError::IntegerOverflow)?
                .to_be_bytes(),
        );
        bytes.extend_from_slice(&json);
        if event.segment == boundary.segment && event.event_sequence == boundary.event_sequence {
            found = true;
        }
    }
    if !found {
        return Err(ReplayPackageError::OrphanBoundary(boundary.event_sequence));
    }
    Ok(Sha256::digest(&bytes))
}

fn validate_token(value: &str, field: &'static str) -> Result<(), ReplayPackageError> {
    if value.is_empty()
        || value.len() > MAX_TOKEN_BYTES
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'-' | b'_' | b'.' | b'/' | b'[' | b']' | b':')
        })
    {
        return Err(ReplayPackageError::InvalidToken(field));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum JsonValue {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    CharBytes(Vec<u8>),
    Array(Vec<Self>),
    Object(BTreeMap<String, Self>),
}

impl JsonValue {
    fn object(entries: impl IntoIterator<Item = (String, Self)>) -> Self {
        Self::Object(entries.into_iter().collect())
    }

    fn as_object(&self) -> Result<&BTreeMap<String, Self>, ReplayPackageError> {
        if let Self::Object(value) = self {
            Ok(value)
        } else {
            Err(ReplayPackageError::InvalidJson("type"))
        }
    }

    fn as_array(&self) -> Result<&[Self], ReplayPackageError> {
        if let Self::Array(value) = self {
            Ok(value)
        } else {
            Err(ReplayPackageError::InvalidJson("type"))
        }
    }

    fn as_str(&self) -> Result<&str, ReplayPackageError> {
        if let Self::String(value) = self {
            Ok(value)
        } else {
            Err(ReplayPackageError::InvalidJson("type"))
        }
    }

    fn as_bool(&self) -> Result<bool, ReplayPackageError> {
        if let Self::Bool(value) = self {
            Ok(*value)
        } else {
            Err(ReplayPackageError::InvalidJson("type"))
        }
    }

    fn as_u64(&self) -> Result<u64, ReplayPackageError> {
        match self {
            Self::String(value) if canonical_unsigned(value) => value
                .parse()
                .map_err(|_| ReplayPackageError::InvalidJson("unsigned")),
            Self::Number(value) => value
                .parse()
                .map_err(|_| ReplayPackageError::InvalidJson("number")),
            _ => Err(ReplayPackageError::InvalidJson("unsigned")),
        }
    }
}

impl From<&str> for JsonValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<String> for JsonValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<bool> for JsonValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

#[derive(Clone, Copy)]
struct JsonLimits {
    max_depth: usize,
    max_string_bytes: usize,
    max_collection_items: usize,
    max_total_values: usize,
}

fn canonical_json(value: &JsonValue) -> Vec<u8> {
    let mut output = String::new();
    write_json(value, &mut output);
    output.into_bytes()
}

fn write_json(value: &JsonValue, output: &mut String) {
    match value {
        JsonValue::Null => output.push_str("null"),
        JsonValue::Bool(true) => output.push_str("true"),
        JsonValue::Bool(false) => output.push_str("false"),
        JsonValue::Number(number) => output.push_str(number),
        JsonValue::String(string) => write_json_string(string, output),
        JsonValue::CharBytes(bytes) => {
            use fmt::Write;
            output.push('"');
            for byte in bytes {
                write!(output, "\\u{byte:04x}").expect("String writes cannot fail");
            }
            output.push('"');
        }
        JsonValue::Array(items) => {
            output.push('[');
            for (index, item) in items.iter().enumerate() {
                if index != 0 {
                    output.push(',');
                }
                write_json(item, output);
            }
            output.push(']');
        }
        JsonValue::Object(entries) => {
            output.push('{');
            for (index, (key, item)) in entries.iter().enumerate() {
                if index != 0 {
                    output.push(',');
                }
                write_json_string(key, output);
                output.push(':');
                write_json(item, output);
            }
            output.push('}');
        }
    }
}

fn write_json_string(value: &str, output: &mut String) {
    use fmt::Write;
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\u{00}'..='\u{1f}' => {
                write!(output, "\\u{:04x}", u32::from(character))
                    .expect("String writes cannot fail");
            }
            _ => output.push(character),
        }
    }
    output.push('"');
}

fn parse_json(input: &[u8], limits: JsonLimits) -> Result<JsonValue, ()> {
    if input.starts_with(&[0xef, 0xbb, 0xbf]) {
        return Err(());
    }
    let text = core::str::from_utf8(input).map_err(|_| ())?;
    if text.contains('\r') {
        return Err(());
    }
    let mut parser = JsonParser {
        bytes: text.as_bytes(),
        index: 0,
        limits,
        total_values: 0,
    };
    let value = parser.parse_value(0)?;
    parser.skip_whitespace();
    if parser.index == parser.bytes.len() {
        Ok(value)
    } else {
        Err(())
    }
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    index: usize,
    limits: JsonLimits,
    total_values: usize,
}

impl JsonParser<'_> {
    fn parse_value(&mut self, depth: usize) -> Result<JsonValue, ()> {
        if depth > self.limits.max_depth {
            return Err(());
        }
        self.total_values = self.total_values.checked_add(1).ok_or(())?;
        if self.total_values > self.limits.max_total_values {
            return Err(());
        }
        self.skip_whitespace();
        match self.peek() {
            Some(b'n') => {
                self.literal(b"null")?;
                Ok(JsonValue::Null)
            }
            Some(b't') => {
                self.literal(b"true")?;
                Ok(JsonValue::Bool(true))
            }
            Some(b'f') => {
                self.literal(b"false")?;
                Ok(JsonValue::Bool(false))
            }
            Some(b'"') => self.parse_string().map(JsonValue::String),
            Some(b'[') => self.parse_array(depth + 1),
            Some(b'{') => self.parse_object(depth + 1),
            Some(b'-' | b'0'..=b'9') => self.parse_number().map(JsonValue::Number),
            _ => Err(()),
        }
    }

    fn parse_array(&mut self, depth: usize) -> Result<JsonValue, ()> {
        self.expect(b'[')?;
        self.skip_whitespace();
        let mut values = Vec::new();
        if self.peek() == Some(b']') {
            self.index += 1;
            return Ok(JsonValue::Array(values));
        }
        loop {
            if values.len() >= self.limits.max_collection_items {
                return Err(());
            }
            values.push(self.parse_value(depth)?);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.index += 1,
                Some(b']') => {
                    self.index += 1;
                    break;
                }
                _ => return Err(()),
            }
        }
        Ok(JsonValue::Array(values))
    }

    fn parse_object(&mut self, depth: usize) -> Result<JsonValue, ()> {
        self.expect(b'{')?;
        self.skip_whitespace();
        let mut entries = BTreeMap::new();
        if self.peek() == Some(b'}') {
            self.index += 1;
            return Ok(JsonValue::Object(entries));
        }
        loop {
            if entries.len() >= self.limits.max_collection_items {
                return Err(());
            }
            self.skip_whitespace();
            let key = self.parse_string()?;
            if entries.contains_key(&key) {
                return Err(());
            }
            self.skip_whitespace();
            self.expect(b':')?;
            let value = self.parse_value(depth)?;
            entries.insert(key, value);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.index += 1,
                Some(b'}') => {
                    self.index += 1;
                    break;
                }
                _ => return Err(()),
            }
        }
        Ok(JsonValue::Object(entries))
    }

    fn parse_string(&mut self) -> Result<String, ()> {
        self.expect(b'"')?;
        let mut output = String::new();
        while let Some(byte) = self.peek() {
            self.index += 1;
            match byte {
                b'"' => {
                    return (output.len() <= self.limits.max_string_bytes)
                        .then_some(output)
                        .ok_or(());
                }
                b'\\' => self.parse_escape(&mut output)?,
                0x00..=0x1f => return Err(()),
                0x20..=0x7f => output.push(char::from(byte)),
                _ => {
                    self.index -= 1;
                    let remaining =
                        core::str::from_utf8(&self.bytes[self.index..]).map_err(|_| ())?;
                    let character = remaining.chars().next().ok_or(())?;
                    output.push(character);
                    self.index += character.len_utf8();
                }
            }
            if output.len() > self.limits.max_string_bytes {
                return Err(());
            }
        }
        Err(())
    }

    fn parse_escape(&mut self, output: &mut String) -> Result<(), ()> {
        let escaped = self.peek().ok_or(())?;
        self.index += 1;
        match escaped {
            b'"' => output.push('"'),
            b'\\' => output.push('\\'),
            b'/' => output.push('/'),
            b'b' => output.push('\u{08}'),
            b'f' => output.push('\u{0c}'),
            b'n' => output.push('\n'),
            b'r' => output.push('\r'),
            b't' => output.push('\t'),
            b'u' => {
                let code = self.hex_quad()?;
                if (0xd800..=0xdbff).contains(&code) {
                    if self.bytes.get(self.index..self.index + 2) != Some(b"\\u") {
                        return Err(());
                    }
                    self.index += 2;
                    let low = self.hex_quad()?;
                    if !(0xdc00..=0xdfff).contains(&low) {
                        return Err(());
                    }
                    let scalar =
                        0x1_0000 + ((u32::from(code) - 0xd800) << 10) + (u32::from(low) - 0xdc00);
                    output.push(char::from_u32(scalar).ok_or(())?);
                } else if (0xdc00..=0xdfff).contains(&code) {
                    return Err(());
                } else {
                    output.push(char::from_u32(u32::from(code)).ok_or(())?);
                }
            }
            _ => return Err(()),
        }
        Ok(())
    }

    fn hex_quad(&mut self) -> Result<u16, ()> {
        let bytes = self.bytes.get(self.index..self.index + 4).ok_or(())?;
        let mut value = 0_u16;
        for byte in bytes {
            value = value
                .checked_mul(16)
                .and_then(|v| hex_nibble(*byte).map(|n| v + u16::from(n)))
                .ok_or(())?;
        }
        self.index += 4;
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<String, ()> {
        let start = self.index;
        if self.peek() == Some(b'-') {
            self.index += 1;
        }
        match self.peek() {
            Some(b'0') => {
                self.index += 1;
                if matches!(self.peek(), Some(b'0'..=b'9')) {
                    return Err(());
                }
            }
            Some(b'1'..=b'9') => {
                self.index += 1;
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.index += 1;
                }
            }
            _ => return Err(()),
        }
        if matches!(self.peek(), Some(b'.' | b'e' | b'E')) {
            return Err(());
        }
        core::str::from_utf8(&self.bytes[start..self.index])
            .map(str::to_owned)
            .map_err(|_| ())
    }

    fn literal(&mut self, literal: &[u8]) -> Result<(), ()> {
        if self.bytes.get(self.index..self.index + literal.len()) != Some(literal) {
            return Err(());
        }
        self.index += literal.len();
        Ok(())
    }
    fn expect(&mut self, byte: u8) -> Result<(), ()> {
        if self.peek() != Some(byte) {
            return Err(());
        }
        self.index += 1;
        Ok(())
    }
    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\t')) {
            self.index += 1;
        }
    }
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.index).copied()
    }
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn required<'a>(
    object: &'a BTreeMap<String, JsonValue>,
    key: &'static str,
) -> Result<&'a JsonValue, ReplayPackageError> {
    object
        .get(key)
        .ok_or(ReplayPackageError::InvalidJson("missing field"))
}

fn only_fields(
    object: &BTreeMap<String, JsonValue>,
    allowed: &[&str],
) -> Result<(), ReplayPackageError> {
    if object.len() != allowed.len() || object.keys().any(|key| !allowed.contains(&key.as_str())) {
        Err(ReplayPackageError::InvalidJson("closed schema"))
    } else {
        Ok(())
    }
}

fn canonical_unsigned(value: &str) -> bool {
    value == "0" || (!value.starts_with('0') && value.bytes().all(|byte| byte.is_ascii_digit()))
}

fn canonical_signed(value: &str) -> bool {
    if let Some(rest) = value.strip_prefix('-') {
        !rest.is_empty() && rest != "0" && canonical_unsigned(rest)
    } else {
        canonical_unsigned(value)
    }
}

fn container_payload(members: &[Vec<u8>; 4]) -> Result<Vec<u8>, ReplayPackageError> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&CONTAINER_VERSION.to_be_bytes());
    bytes.extend_from_slice(&4_u32.to_be_bytes());
    for (name, member) in MEMBER_NAMES.iter().zip(members) {
        let name_len =
            u16::try_from(name.len()).map_err(|_| ReplayPackageError::IntegerOverflow)?;
        let member_len =
            u64::try_from(member.len()).map_err(|_| ReplayPackageError::IntegerOverflow)?;
        bytes.extend_from_slice(&name_len.to_be_bytes());
        bytes.extend_from_slice(&member_len.to_be_bytes());
        bytes.extend_from_slice(name.as_bytes());
        bytes.extend_from_slice(member);
    }
    Ok(bytes)
}

fn validate_member_sizes(
    members: &[Vec<u8>; 4],
    limits: ReplayDecodeLimits,
) -> Result<(), ReplayPackageError> {
    let mut total = 0_usize;
    for member in members {
        if member.len() > limits.max_member_bytes {
            return Err(ReplayPackageError::LimitExceeded("member bytes"));
        }
        total = total
            .checked_add(member.len())
            .ok_or(ReplayPackageError::IntegerOverflow)?;
    }
    if total > limits.max_total_member_bytes {
        return Err(ReplayPackageError::LimitExceeded("expanded member bytes"));
    }
    Ok(())
}

fn decode_container(
    bytes: &[u8],
    limits: ReplayDecodeLimits,
) -> Result<[Vec<u8>; 4], ReplayPackageError> {
    if bytes.len() > limits.max_package_bytes {
        return Err(ReplayPackageError::LimitExceeded("package bytes"));
    }
    if bytes.len() < MAGIC.len() + 8 + 32 {
        return Err(ReplayPackageError::Truncated);
    }
    let (payload, digest) = bytes.split_at(bytes.len() - 32);
    if Sha256::digest(payload).as_bytes() != digest {
        return Err(ReplayPackageError::ContainerIntegrityMismatch);
    }
    let mut reader = ByteReader {
        bytes: payload,
        index: 0,
    };
    if reader.take(MAGIC.len())? != MAGIC {
        return Err(ReplayPackageError::BadMagic);
    }
    let version = reader.u32()?;
    if version != CONTAINER_VERSION {
        return Err(ReplayPackageError::UnsupportedContainerVersion(version));
    }
    let count = usize::try_from(reader.u32()?).map_err(|_| ReplayPackageError::IntegerOverflow)?;
    if count != 4 {
        return Err(ReplayPackageError::MemberCount(count));
    }
    let mut members: [Vec<u8>; 4] = std::array::from_fn(|_| Vec::new());
    let mut total = 0_usize;
    for index in 0..4 {
        let name_len = usize::from(reader.u16()?);
        let data_len =
            usize::try_from(reader.u64()?).map_err(|_| ReplayPackageError::IntegerOverflow)?;
        let name = core::str::from_utf8(reader.take(name_len)?)
            .map_err(|_| ReplayPackageError::InvalidToken("member name"))?;
        if name != MEMBER_NAMES[index] {
            return Err(ReplayPackageError::UnexpectedMember {
                index,
                expected: MEMBER_NAMES[index],
                observed: name.to_owned(),
            });
        }
        if data_len > limits.max_member_bytes {
            return Err(ReplayPackageError::LimitExceeded("member bytes"));
        }
        total = total
            .checked_add(data_len)
            .ok_or(ReplayPackageError::IntegerOverflow)?;
        if total > limits.max_total_member_bytes {
            return Err(ReplayPackageError::LimitExceeded("expanded member bytes"));
        }
        members[index] = reader.take(data_len)?.to_vec();
    }
    if reader.index != payload.len() {
        return Err(ReplayPackageError::TrailingData);
    }
    Ok(members)
}

struct ByteReader<'a> {
    bytes: &'a [u8],
    index: usize,
}
impl ByteReader<'_> {
    fn take(&mut self, count: usize) -> Result<&[u8], ReplayPackageError> {
        let end = self
            .index
            .checked_add(count)
            .ok_or(ReplayPackageError::IntegerOverflow)?;
        let value = self
            .bytes
            .get(self.index..end)
            .ok_or(ReplayPackageError::Truncated)?;
        self.index = end;
        Ok(value)
    }
    fn u16(&mut self) -> Result<u16, ReplayPackageError> {
        Ok(u16::from_be_bytes(
            self.take(2)?
                .try_into()
                .map_err(|_| ReplayPackageError::Truncated)?,
        ))
    }
    fn u32(&mut self) -> Result<u32, ReplayPackageError> {
        Ok(u32::from_be_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| ReplayPackageError::Truncated)?,
        ))
    }
    fn u64(&mut self) -> Result<u64, ReplayPackageError> {
        Ok(u64::from_be_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| ReplayPackageError::Truncated)?,
        ))
    }
}

fn hash_json(hash: Hash32) -> JsonValue {
    JsonValue::String(hash.to_hex())
}
fn decimal_json(value: u64) -> JsonValue {
    JsonValue::String(value.to_string())
}
fn identity_json(value: u128) -> JsonValue {
    JsonValue::String(format!("{value:032x}"))
}

fn parse_hash(value: &JsonValue) -> Result<Hash32, ReplayPackageError> {
    let text = value.as_str()?;
    if text.len() != 64
        || !text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(ReplayPackageError::InvalidJson("hash"));
    }
    let mut bytes = [0_u8; 32];
    for (index, pair) in text.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] = (hex_nibble(pair[0]).ok_or(ReplayPackageError::InvalidJson("hash"))? << 4)
            | hex_nibble(pair[1]).ok_or(ReplayPackageError::InvalidJson("hash"))?;
    }
    Ok(Hash32::from_bytes(bytes))
}

fn parse_identity(value: &JsonValue) -> Result<u128, ReplayPackageError> {
    let text = value.as_str()?;
    if text.len() != 32
        || !text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(ReplayPackageError::InvalidJson("identity"));
    }
    u128::from_str_radix(text, 16).map_err(|_| ReplayPackageError::InvalidJson("identity"))
}

fn manifest_json(manifest: &ReplayManifest, fingerprint: bool) -> JsonValue {
    let mut fields = BTreeMap::from([
        ("artifactHash".into(), hash_json(manifest.artifact_hash)),
        (
            "boundaryCount".into(),
            decimal_json(manifest.boundary_count as u64),
        ),
        (
            "canonicalizationVersion".into(),
            JsonValue::Number(CANONICALIZATION_VERSION.to_string()),
        ),
        (
            "deterministicAlgorithm".into(),
            JsonValue::from(manifest.deterministic_algorithm.clone()),
        ),
        (
            "deterministicSeed".into(),
            decimal_json(manifest.deterministic_seed),
        ),
        (
            "eventCount".into(),
            decimal_json(manifest.event_count as u64),
        ),
        (
            "initialSnapshotHash".into(),
            hash_json(manifest.initial_snapshot_hash),
        ),
        ("packageKind".into(), JsonValue::from(PACKAGE_KIND)),
        (
            "priorityTableVersion".into(),
            JsonValue::from(manifest.priority_table_version.clone()),
        ),
        ("profileHash".into(), hash_json(manifest.profile_hash)),
        (
            "runtimeVersion".into(),
            JsonValue::from(manifest.runtime_version.clone()),
        ),
        (
            "schedulerVersion".into(),
            JsonValue::from(manifest.scheduler_version.clone()),
        ),
        (
            "schemaVersion".into(),
            JsonValue::Number(SCHEMA_VERSION.to_string()),
        ),
        (
            "workCostVersion".into(),
            JsonValue::from(manifest.work_cost_version.clone()),
        ),
    ]);
    if fingerprint {
        fields.insert(
            "contentFingerprint".into(),
            hash_json(manifest.content_fingerprint),
        );
    }
    JsonValue::Object(fields)
}

fn manifest_from_json(value: &JsonValue) -> Result<ReplayManifest, ReplayPackageError> {
    let object = value.as_object()?;
    only_fields(
        object,
        &[
            "artifactHash",
            "boundaryCount",
            "canonicalizationVersion",
            "contentFingerprint",
            "deterministicAlgorithm",
            "deterministicSeed",
            "eventCount",
            "initialSnapshotHash",
            "packageKind",
            "priorityTableVersion",
            "profileHash",
            "runtimeVersion",
            "schedulerVersion",
            "schemaVersion",
            "workCostVersion",
        ],
    )?;
    if required(object, "packageKind")?.as_str()? != PACKAGE_KIND
        || required(object, "schemaVersion")?.as_u64()? != u64::from(SCHEMA_VERSION)
        || required(object, "canonicalizationVersion")?.as_u64()?
            != u64::from(CANONICALIZATION_VERSION)
        || required(object, "priorityTableVersion")?.as_str()? != PRIORITY_TABLE_VERSION
        || required(object, "workCostVersion")?.as_str()? != WORK_COST_VERSION
    {
        return Err(ReplayPackageError::InvalidManifest);
    }
    Ok(ReplayManifest {
        artifact_hash: parse_hash(required(object, "artifactHash")?)?,
        boundary_count: usize::try_from(required(object, "boundaryCount")?.as_u64()?)
            .map_err(|_| ReplayPackageError::IntegerOverflow)?,
        content_fingerprint: parse_hash(required(object, "contentFingerprint")?)?,
        deterministic_algorithm: required(object, "deterministicAlgorithm")?
            .as_str()?
            .to_owned(),
        deterministic_seed: required(object, "deterministicSeed")?.as_u64()?,
        event_count: usize::try_from(required(object, "eventCount")?.as_u64()?)
            .map_err(|_| ReplayPackageError::IntegerOverflow)?,
        initial_snapshot_hash: parse_hash(required(object, "initialSnapshotHash")?)?,
        profile_hash: parse_hash(required(object, "profileHash")?)?,
        priority_table_version: required(object, "priorityTableVersion")?
            .as_str()?
            .to_owned(),
        runtime_version: required(object, "runtimeVersion")?.as_str()?.to_owned(),
        scheduler_version: required(object, "schedulerVersion")?.as_str()?.to_owned(),
        work_cost_version: required(object, "workCostVersion")?.as_str()?.to_owned(),
    })
}

fn snapshot_reference(hash: Hash32) -> Vec<u8> {
    format!("sha256:{}\n", hash.to_hex()).into_bytes()
}
fn parse_snapshot_reference(bytes: &[u8]) -> Result<Hash32, ReplayPackageError> {
    if bytes.len() != 72 || !bytes.starts_with(b"sha256:") || !bytes.ends_with(b"\n") {
        return Err(ReplayPackageError::InvalidSnapshotReference);
    }
    parse_hash(&JsonValue::String(
        core::str::from_utf8(&bytes[7..71])
            .map_err(|_| ReplayPackageError::InvalidSnapshotReference)?
            .to_owned(),
    ))
    .map_err(|_| ReplayPackageError::InvalidSnapshotReference)
}

fn event_kind_token(kind: ReplayEventKind) -> &'static str {
    match kind {
        ReplayEventKind::PowerOn => "POWER_ON",
        ReplayEventKind::PowerOff => "POWER_OFF",
        ReplayEventKind::ArtifactInstalled => "ARTIFACT_INSTALLED",
        ReplayEventKind::RequestRun => "REQUEST_RUN",
        ReplayEventKind::RequestStop => "REQUEST_STOP",
        ReplayEventKind::Pause => "PAUSE",
        ReplayEventKind::Resume => "RESUME",
        ReplayEventKind::RawInputAccepted => "RAW_INPUT_ACCEPTED",
        ReplayEventKind::ScanCompleted => "SCAN_COMPLETED",
        ReplayEventKind::FatalFault => "FATAL_FAULT",
        ReplayEventKind::FaultReset => "FAULT_RESET",
        ReplayEventKind::WarmRestart => "WARM_RESTART",
        ReplayEventKind::PowerCycle => "POWER_CYCLE",
        ReplayEventKind::MemoryReset => "MEMORY_RESET",
        ReplayEventKind::SnapshotRestored => "SNAPSHOT_RESTORED",
        ReplayEventKind::CommandRejected => "COMMAND_REJECTED",
        ReplayEventKind::InstanceCloned => "INSTANCE_CLONED",
        ReplayEventKind::InstanceReplaced => "INSTANCE_REPLACED",
        ReplayEventKind::ObservationBoundary => "OBSERVATION_BOUNDARY",
        ReplayEventKind::HardwareBoundary => "HARDWARE_BOUNDARY",
    }
}
fn parse_event_kind(value: &str) -> Result<ReplayEventKind, ReplayPackageError> {
    match value {
        "POWER_ON" => Ok(ReplayEventKind::PowerOn),
        "POWER_OFF" => Ok(ReplayEventKind::PowerOff),
        "ARTIFACT_INSTALLED" => Ok(ReplayEventKind::ArtifactInstalled),
        "REQUEST_RUN" => Ok(ReplayEventKind::RequestRun),
        "REQUEST_STOP" => Ok(ReplayEventKind::RequestStop),
        "PAUSE" => Ok(ReplayEventKind::Pause),
        "RESUME" => Ok(ReplayEventKind::Resume),
        "RAW_INPUT_ACCEPTED" => Ok(ReplayEventKind::RawInputAccepted),
        "SCAN_COMPLETED" => Ok(ReplayEventKind::ScanCompleted),
        "FATAL_FAULT" => Ok(ReplayEventKind::FatalFault),
        "FAULT_RESET" => Ok(ReplayEventKind::FaultReset),
        "WARM_RESTART" => Ok(ReplayEventKind::WarmRestart),
        "POWER_CYCLE" => Ok(ReplayEventKind::PowerCycle),
        "MEMORY_RESET" => Ok(ReplayEventKind::MemoryReset),
        "SNAPSHOT_RESTORED" => Ok(ReplayEventKind::SnapshotRestored),
        "COMMAND_REJECTED" => Ok(ReplayEventKind::CommandRejected),
        "INSTANCE_CLONED" => Ok(ReplayEventKind::InstanceCloned),
        "INSTANCE_REPLACED" => Ok(ReplayEventKind::InstanceReplaced),
        "OBSERVATION_BOUNDARY" => Ok(ReplayEventKind::ObservationBoundary),
        "HARDWARE_BOUNDARY" => Ok(ReplayEventKind::HardwareBoundary),
        _ => Err(ReplayPackageError::InvalidToken("event.kind")),
    }
}

fn plc_value_from_existing(
    data_type: &CanonicalType,
    value: &PlcValue,
    limits: AggregateLimits,
) -> Result<CanonicalReplayPlcValue, ReplayPackageError> {
    let type_id = match data_type {
        CanonicalType::Primitive(PrimitiveType::String(cap)) => format!("STRING[{cap}]"),
        CanonicalType::Primitive(kind) => kind.stable_id().to_owned(),
        _ => format!(
            "TYPE-SHA256:{}",
            Sha256::digest(&data_type.canonical_bytes(limits)?).to_hex()
        ),
    };
    let (encoding, node) = match (data_type, value) {
        (CanonicalType::Primitive(kind), PlcValue::Scalar(scalar)) => {
            let node = match scalar.value() {
                ScalarValue::Bool(v) => ReplayPlcNode::Bool(*v),
                ScalarValue::Signed(v) | ScalarValue::Time(v) => ReplayPlcNode::Text(v.to_string()),
                ScalarValue::Unsigned(v) => ReplayPlcNode::Text(v.to_string()),
                ScalarValue::BitString(v) => ReplayPlcNode::Text(format!(
                    "{v:0width$x}",
                    width = usize::from(kind.width_bits().unwrap_or(0) / 4)
                )),
                ScalarValue::Real(v) => ReplayPlcNode::Text(format!("{:08x}", v.bits())),
                ScalarValue::Lreal(v) => ReplayPlcNode::Text(format!("{:016x}", v.bits())),
                ScalarValue::Char(v) => ReplayPlcNode::Text(format!("{v:02x}")),
                ScalarValue::String(v) => ReplayPlcNode::CharBytes(v.clone()),
            };
            let enc = match kind {
                PrimitiveType::Bool => "boolean",
                PrimitiveType::Byte
                | PrimitiveType::Word
                | PrimitiveType::Dword
                | PrimitiveType::Lword => "fixed-width-hex",
                PrimitiveType::Real | PrimitiveType::Lreal => "ieee-bits-hex",
                PrimitiveType::Char => "char-byte-hex",
                PrimitiveType::String(_) => "char-bytes-unicode",
                _ => "decimal-text",
            };
            (enc.to_owned(), node)
        }
        (CanonicalType::Array { element_type, .. }, PlcValue::Array(values)) => (
            "declared-index-order".into(),
            ReplayPlcNode::Array(
                values
                    .iter()
                    .map(|v| plc_value_from_existing(element_type, v, limits))
                    .collect::<Result<_, _>>()?,
            ),
        ),
        (
            CanonicalType::AnonymousStruct { members } | CanonicalType::NamedStruct { members, .. },
            PlcValue::Struct(fields),
        ) => {
            let mut ordered = members.iter().collect::<Vec<_>>();
            ordered.sort_by_key(|m| m.declared_order);
            let mut out = Vec::new();
            for member in ordered {
                let field = fields
                    .iter()
                    .find(|f| f.member_id == member.id)
                    .ok_or_else(|| ReplayPackageError::PlcType("missing field".into()))?;
                out.push(ReplayStructField {
                    member_id: member.id,
                    value: plc_value_from_existing(&member.data_type, &field.value, limits)?,
                });
            }
            ("declared-member-order".into(), ReplayPlcNode::Struct(out))
        }
        _ => return Err(ReplayPackageError::PlcType("shape".into())),
    };
    let decoded = CanonicalReplayPlcValue {
        type_id,
        encoding,
        value: node,
    };
    validate_plc_wire(&decoded)?;
    Ok(decoded)
}

fn validate_plc_wire(value: &CanonicalReplayPlcValue) -> Result<(), ReplayPackageError> {
    let text = match &value.value {
        ReplayPlcNode::Text(text) => Some(text.as_str()),
        _ => None,
    };
    let exact_hex = |width: usize| {
        text.is_some_and(|candidate| {
            candidate.len() == width
                && candidate
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
    };
    let signed = || text.and_then(|candidate| candidate.parse::<i64>().ok());
    let unsigned = || text.and_then(|candidate| candidate.parse::<u64>().ok());
    let valid = match value.type_id.as_str() {
        "BOOL" => value.encoding == "boolean" && matches!(&value.value, ReplayPlcNode::Bool(_)),
        "SINT" => value.encoding == "decimal-text" && text.is_some_and(canonical_signed) && signed().is_some_and(|v| i8::try_from(v).is_ok()),
        "INT" => value.encoding == "decimal-text" && text.is_some_and(canonical_signed) && signed().is_some_and(|v| i16::try_from(v).is_ok()),
        "DINT" => value.encoding == "decimal-text" && text.is_some_and(canonical_signed) && signed().is_some_and(|v| i32::try_from(v).is_ok()),
        "LINT" | "TIME" => value.encoding == "decimal-text" && text.is_some_and(canonical_signed) && signed().is_some(),
        "USINT" => value.encoding == "decimal-text" && text.is_some_and(canonical_unsigned) && unsigned().is_some_and(|v| u8::try_from(v).is_ok()),
        "UINT" => value.encoding == "decimal-text" && text.is_some_and(canonical_unsigned) && unsigned().is_some_and(|v| u16::try_from(v).is_ok()),
        "UDINT" => value.encoding == "decimal-text" && text.is_some_and(canonical_unsigned) && unsigned().is_some_and(|v| u32::try_from(v).is_ok()),
        "ULINT" => value.encoding == "decimal-text" && text.is_some_and(canonical_unsigned) && unsigned().is_some(),
        "BYTE" => value.encoding == "fixed-width-hex" && exact_hex(2),
        "WORD" => value.encoding == "fixed-width-hex" && exact_hex(4),
        "DWORD" => value.encoding == "fixed-width-hex" && exact_hex(8),
        "LWORD" => value.encoding == "fixed-width-hex" && exact_hex(16),
        "REAL" => {
            value.encoding == "ieee-bits-hex"
                && exact_hex(8)
                && text
                    .and_then(|candidate| u32::from_str_radix(candidate, 16).ok())
                    .is_some_and(|bits| plc_types::CanonicalF32::from_bits(bits).bits() == bits)
        }
        "LREAL" => {
            value.encoding == "ieee-bits-hex"
                && exact_hex(16)
                && text
                    .and_then(|candidate| u64::from_str_radix(candidate, 16).ok())
                    .is_some_and(|bits| plc_types::CanonicalF64::from_bits(bits).bits() == bits)
        }
        "CHAR" => value.encoding == "char-byte-hex" && exact_hex(2),
        id if id.starts_with("STRING[") && id.ends_with(']') => id[7..id.len()-1]
            .parse::<u8>()
            .ok()
            .filter(|capacity| *capacity < 255)
            .is_some_and(|capacity| value.encoding == "char-bytes-unicode" && matches!(&value.value, ReplayPlcNode::CharBytes(bytes) if bytes.len() <= usize::from(capacity))),
        id if id.starts_with("TYPE-SHA256:") => {
            id.len() == 76
                && id[12..].bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                && ((value.encoding == "declared-index-order"
                && matches!(&value.value, ReplayPlcNode::Array(_)))
                || (value.encoding == "declared-member-order"
                    && matches!(&value.value, ReplayPlcNode::Struct(fields) if fields.iter().map(|field| field.member_id).collect::<BTreeSet<_>>().len() == fields.len())))
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(ReplayPackageError::PlcType(
            "noncanonical wire value".to_owned(),
        ))
    }
}
fn plc_value_json(value: &CanonicalReplayPlcValue) -> JsonValue {
    let node = match &value.value {
        ReplayPlcNode::Bool(v) => JsonValue::Bool(*v),
        ReplayPlcNode::Text(v) => JsonValue::String(v.clone()),
        ReplayPlcNode::CharBytes(v) => JsonValue::CharBytes(v.clone()),
        ReplayPlcNode::Array(v) => JsonValue::Array(v.iter().map(plc_value_json).collect()),
        ReplayPlcNode::Struct(v) => JsonValue::Array(
            v.iter()
                .map(|f| {
                    JsonValue::object([
                        (
                            "memberId".into(),
                            JsonValue::String(format!(
                                "{:032x}",
                                u128::from_be_bytes(f.member_id.as_bytes())
                            )),
                        ),
                        ("value".into(), plc_value_json(&f.value)),
                    ])
                })
                .collect(),
        ),
    };
    JsonValue::object([
        ("encoding".into(), JsonValue::String(value.encoding.clone())),
        ("typeId".into(), JsonValue::String(value.type_id.clone())),
        ("value".into(), node),
    ])
}

fn payload_value_json(value: &ReplayPayloadValue) -> JsonValue {
    let (kind, node) = match value {
        ReplayPayloadValue::Null => ("NULL", None),
        ReplayPayloadValue::Bool(v) => ("BOOL", Some(JsonValue::Bool(*v))),
        ReplayPayloadValue::Decimal(v) => ("DECIMAL", Some(JsonValue::String(v.clone()))),
        ReplayPayloadValue::Hex(v) => ("HEX", Some(JsonValue::String(v.clone()))),
        ReplayPayloadValue::Text(v) => ("TEXT", Some(JsonValue::String(v.clone()))),
        ReplayPayloadValue::Identity(v) => ("IDENTITY", Some(identity_json(*v))),
        ReplayPayloadValue::Hash(v) => ("HASH", Some(hash_json(*v))),
        ReplayPayloadValue::Plc(v) => ("PLC", Some(plc_value_json(v))),
        ReplayPayloadValue::Array(v) => (
            "ARRAY",
            Some(JsonValue::Array(v.iter().map(payload_value_json).collect())),
        ),
        ReplayPayloadValue::Object(v) => (
            "OBJECT",
            Some(JsonValue::Object(
                v.iter()
                    .map(|(k, v)| (k.clone(), payload_value_json(v)))
                    .collect(),
            )),
        ),
    };
    let mut object = BTreeMap::from([("kind".into(), JsonValue::from(kind))]);
    if let Some(node) = node {
        object.insert("value".into(), node);
    }
    JsonValue::Object(object)
}
fn payload_json(payload: &ReplayTypedPayload) -> JsonValue {
    JsonValue::object([
        (
            "fields".into(),
            JsonValue::Object(
                payload
                    .fields
                    .iter()
                    .map(|(k, v)| (k.clone(), payload_value_json(v)))
                    .collect(),
            ),
        ),
        (
            "schemaId".into(),
            JsonValue::from(event_kind_token(payload.event_kind)),
        ),
    ])
}
fn validate_payload(
    payload: &ReplayTypedPayload,
    limits: ReplayDecodeLimits,
) -> Result<(), ReplayPackageError> {
    fn walk(
        value: &ReplayPayloadValue,
        depth: usize,
        count: &mut usize,
        limits: ReplayDecodeLimits,
    ) -> Result<(), ReplayPackageError> {
        if depth > limits.max_payload_depth {
            return Err(ReplayPackageError::LimitExceeded("payload depth"));
        }
        *count = count
            .checked_add(1)
            .ok_or(ReplayPackageError::IntegerOverflow)?;
        if *count > limits.max_payload_values {
            return Err(ReplayPackageError::LimitExceeded("payload values"));
        }
        match value {
            ReplayPayloadValue::Decimal(v)
                if v.len() > limits.max_json_string_bytes || !canonical_signed(v) =>
            {
                Err(ReplayPackageError::InvalidToken("payload.decimal"))
            }
            ReplayPayloadValue::Hex(v)
                if v.is_empty()
                    || v.len() > limits.max_json_string_bytes
                    || v.len() % 2 != 0
                    || !v
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) =>
            {
                Err(ReplayPackageError::InvalidToken("payload.hex"))
            }
            ReplayPayloadValue::Text(v) if v.len() > limits.max_json_string_bytes => {
                Err(ReplayPackageError::LimitExceeded("payload text"))
            }
            ReplayPayloadValue::Identity(0) => {
                Err(ReplayPackageError::ZeroIdentity("payload.identity"))
            }
            ReplayPayloadValue::Hash(hash) if *hash == Hash32::ZERO => {
                Err(ReplayPackageError::ZeroIdentity("payload.hash"))
            }
            ReplayPayloadValue::Plc(value) => validate_plc_wire(value),
            ReplayPayloadValue::Array(v) => {
                v.iter().try_for_each(|x| walk(x, depth + 1, count, limits))
            }
            ReplayPayloadValue::Object(v) => {
                for (k, x) in v {
                    validate_token(k, "payload.field")?;
                    walk(x, depth + 1, count, limits)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
    let mut count = 0;
    for (k, v) in &payload.fields {
        validate_token(k, "payload.field")?;
        walk(v, 0, &mut count, limits)?;
    }
    Ok(())
}

fn result_json(result: &ReplayCommandResult) -> JsonValue {
    JsonValue::object([
        ("code".into(), JsonValue::from(result.code.clone())),
        ("detail".into(), payload_json(&result.detail)),
        ("status".into(), JsonValue::from(result.status.token())),
    ])
}
fn actor_json(actor: ReplayActorProvenance) -> JsonValue {
    JsonValue::object([
        ("actorId".into(), identity_json(actor.actor_id)),
        ("commandId".into(), identity_json(actor.command_id)),
        (
            "idempotencyKey".into(),
            identity_json(actor.idempotency_key),
        ),
        ("kind".into(), JsonValue::from(actor.kind.token())),
    ])
}
fn segment_predecessor_json(event: &ReplayPackageEvent) -> JsonValue {
    match (event.segment_predecessor, event.predecessor_event_sequence) {
        (Some(segment), Some(event_sequence)) => JsonValue::object([
            (
                "controllerEpoch".into(),
                decimal_json(segment.controller_epoch),
            ),
            (
                "controllerId".into(),
                identity_json(segment.controller_id.0),
            ),
            ("eventSequence".into(), decimal_json(event_sequence)),
            ("universeEpoch".into(), decimal_json(segment.universe_epoch)),
            ("universeId".into(), identity_json(segment.universe_id.0)),
        ]),
        _ => JsonValue::Null,
    }
}
fn event_json(event: &ReplayPackageEvent) -> JsonValue {
    let payload = payload_json(&event.payload);
    let result = result_json(&event.result);
    JsonValue::object([
        ("actor".into(), actor_json(event.actor)),
        ("artifactHash".into(), hash_json(event.artifact_hash)),
        (
            "controllerEpoch".into(),
            decimal_json(event.segment.controller_epoch),
        ),
        (
            "controllerId".into(),
            identity_json(event.segment.controller_id.0),
        ),
        (
            "eventKind".into(),
            JsonValue::from(event_kind_token(event.kind)),
        ),
        ("eventSequence".into(), decimal_json(event.event_sequence)),
        ("payload".into(), payload.clone()),
        (
            "payloadHash".into(),
            hash_json(event.payload.canonical_hash()),
        ),
        ("priority".into(), JsonValue::from(event.priority.token())),
        ("profileHash".into(), hash_json(event.profile_hash)),
        ("result".into(), result.clone()),
        (
            "resultHash".into(),
            hash_json(event.result.canonical_hash()),
        ),
        (
            "runtimePayloadHash".into(),
            hash_json(event.runtime_payload_hash),
        ),
        (
            "runtimeResultHash".into(),
            hash_json(event.runtime_result_hash),
        ),
        ("segmentPredecessor".into(), segment_predecessor_json(event)),
        (
            "timelineBranch".into(),
            JsonValue::Bool(event.universe_timeline_branch),
        ),
        (
            "universeEpoch".into(),
            decimal_json(event.segment.universe_epoch),
        ),
        (
            "universeId".into(),
            identity_json(event.segment.universe_id.0),
        ),
        (
            "virtualTimestampMs".into(),
            decimal_json(event.virtual_timestamp_ms),
        ),
    ])
}
fn encode_events(events: &[ReplayPackageEvent]) -> Vec<u8> {
    encode_jsonl(events.iter().map(event_json))
}
fn encode_boundaries(boundaries: &[ReplayBoundaryHash]) -> Vec<u8> {
    encode_jsonl(boundaries.iter().map(boundary_json))
}
fn encode_jsonl(values: impl Iterator<Item = JsonValue>) -> Vec<u8> {
    let mut out = Vec::new();
    for value in values {
        out.extend(canonical_json(&value));
        out.push(b'\n');
    }
    out
}

fn boundary_json(boundary: &ReplayBoundaryHash) -> JsonValue {
    JsonValue::object([
        (
            "boundaryKind".into(),
            JsonValue::from(boundary.kind.token()),
        ),
        (
            "causalInputEventSequence".into(),
            decimal_json(boundary.causal_input_event_sequence),
        ),
        (
            "controllerEpoch".into(),
            decimal_json(boundary.segment.controller_epoch),
        ),
        (
            "controllerId".into(),
            identity_json(boundary.segment.controller_id.0),
        ),
        (
            "eventSequence".into(),
            decimal_json(boundary.event_sequence),
        ),
        (
            "regionHashes".into(),
            JsonValue::Object(
                boundary
                    .region_hashes
                    .iter()
                    .map(|(k, v)| (k.token().into(), hash_json(*v)))
                    .collect(),
            ),
        ),
        (
            "runtimeStateHash".into(),
            hash_json(boundary.runtime_state_hash),
        ),
        ("scanSequence".into(), decimal_json(boundary.scan_sequence)),
        (
            "semanticStateHash".into(),
            hash_json(boundary.semantic_state_hash),
        ),
        (
            "universeEpoch".into(),
            decimal_json(boundary.segment.universe_epoch),
        ),
        (
            "universeId".into(),
            identity_json(boundary.segment.universe_id.0),
        ),
        (
            "virtualTimestampMs".into(),
            decimal_json(boundary.virtual_timestamp_ms),
        ),
    ])
}
fn semantic_region_hash(regions: &BTreeMap<ReplayStateRegion, Hash32>) -> Hash32 {
    let mut bytes = b"PES-REPLAY-REGIONS-1\0".to_vec();
    for region in ReplayStateRegion::ALL {
        bytes.extend_from_slice(region.token().as_bytes());
        bytes.push(0);
        if let Some(hash) = regions.get(&region) {
            bytes.extend_from_slice(hash.as_bytes());
        }
    }
    Sha256::digest(&bytes)
}

fn decode_events(
    bytes: &[u8],
    limits: ReplayDecodeLimits,
) -> Result<Vec<ReplayPackageEvent>, ReplayPackageError> {
    decode_jsonl(
        bytes,
        limits,
        |value, index| decode_event(value).map_err(|_| ReplayPackageError::InvalidEvent(index)),
        event_json,
        limits.max_events,
    )
}
fn decode_boundaries(
    bytes: &[u8],
    limits: ReplayDecodeLimits,
) -> Result<Vec<ReplayBoundaryHash>, ReplayPackageError> {
    decode_jsonl(
        bytes,
        limits,
        |value, index| {
            decode_boundary(value).map_err(|_| ReplayPackageError::InvalidBoundary(index))
        },
        boundary_json,
        limits.max_boundaries,
    )
}
fn decode_jsonl<T>(
    bytes: &[u8],
    limits: ReplayDecodeLimits,
    mut decode: impl FnMut(&JsonValue, usize) -> Result<T, ReplayPackageError>,
    encode: impl Fn(&T) -> JsonValue,
    max_records: usize,
) -> Result<Vec<T>, ReplayPackageError> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    if !bytes.ends_with(b"\n") || bytes.contains(&b'\r') || bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        return Err(ReplayPackageError::NonCanonicalJson("jsonl"));
    }
    let mut out = Vec::new();
    for (index, line) in bytes[..bytes.len() - 1].split(|b| *b == b'\n').enumerate() {
        if index >= max_records {
            return Err(ReplayPackageError::LimitExceeded("jsonl records"));
        }
        if line.is_empty() || line.len() > limits.max_line_bytes {
            return Err(ReplayPackageError::LimitExceeded("jsonl line"));
        }
        let value = parse_json(line, limits.json())
            .map_err(|()| ReplayPackageError::InvalidJson("jsonl"))?;
        let decoded = decode(&value, index)?;
        if canonical_json(&encode(&decoded)) != line {
            return Err(ReplayPackageError::NonCanonicalJson("jsonl"));
        }
        out.push(decoded);
    }
    Ok(out)
}

fn decode_payload(value: &JsonValue) -> Result<ReplayTypedPayload, ReplayPackageError> {
    let object = value.as_object()?;
    only_fields(object, &["fields", "schemaId"])?;
    let event_kind = parse_event_kind(required(object, "schemaId")?.as_str()?)?;
    let fields = required(object, "fields")?
        .as_object()?
        .iter()
        .map(|(k, v)| Ok((k.clone(), decode_payload_value(v)?)))
        .collect::<Result<_, ReplayPackageError>>()?;
    Ok(ReplayTypedPayload { event_kind, fields })
}
fn decode_payload_value(value: &JsonValue) -> Result<ReplayPayloadValue, ReplayPackageError> {
    let object = value.as_object()?;
    let kind = required(object, "kind")?.as_str()?;
    let node = object.get("value");
    match kind {
        "NULL" if node.is_none() => Ok(ReplayPayloadValue::Null),
        "BOOL" => Ok(ReplayPayloadValue::Bool(
            node.ok_or(ReplayPackageError::InvalidJson("value"))?
                .as_bool()?,
        )),
        "DECIMAL" => Ok(ReplayPayloadValue::Decimal(
            node.ok_or(ReplayPackageError::InvalidJson("value"))?
                .as_str()?
                .to_owned(),
        )),
        "HEX" => Ok(ReplayPayloadValue::Hex(
            node.ok_or(ReplayPackageError::InvalidJson("value"))?
                .as_str()?
                .to_owned(),
        )),
        "TEXT" => Ok(ReplayPayloadValue::Text(
            node.ok_or(ReplayPackageError::InvalidJson("value"))?
                .as_str()?
                .to_owned(),
        )),
        "IDENTITY" => Ok(ReplayPayloadValue::Identity(parse_identity(
            node.ok_or(ReplayPackageError::InvalidJson("value"))?,
        )?)),
        "HASH" => Ok(ReplayPayloadValue::Hash(parse_hash(
            node.ok_or(ReplayPackageError::InvalidJson("value"))?,
        )?)),
        "PLC" => Ok(ReplayPayloadValue::Plc(decode_plc(
            node.ok_or(ReplayPackageError::InvalidJson("value"))?,
        )?)),
        "ARRAY" => Ok(ReplayPayloadValue::Array(
            node.ok_or(ReplayPackageError::InvalidJson("value"))?
                .as_array()?
                .iter()
                .map(decode_payload_value)
                .collect::<Result<_, _>>()?,
        )),
        "OBJECT" => Ok(ReplayPayloadValue::Object(
            node.ok_or(ReplayPackageError::InvalidJson("value"))?
                .as_object()?
                .iter()
                .map(|(k, v)| Ok((k.clone(), decode_payload_value(v)?)))
                .collect::<Result<_, ReplayPackageError>>()?,
        )),
        _ => Err(ReplayPackageError::InvalidToken("payload.kind")),
    }
}
fn decode_plc(value: &JsonValue) -> Result<CanonicalReplayPlcValue, ReplayPackageError> {
    let object = value.as_object()?;
    only_fields(object, &["encoding", "typeId", "value"])?;
    let type_id = required(object, "typeId")?.as_str()?.to_owned();
    let encoding = required(object, "encoding")?.as_str()?.to_owned();
    validate_token(&type_id, "plc.typeId")?;
    validate_token(&encoding, "plc.encoding")?;
    let raw = required(object, "value")?;
    let node = if encoding == "boolean" {
        ReplayPlcNode::Bool(raw.as_bool()?)
    } else if encoding == "declared-index-order" {
        ReplayPlcNode::Array(
            raw.as_array()?
                .iter()
                .map(decode_plc)
                .collect::<Result<_, _>>()?,
        )
    } else if encoding == "declared-member-order" {
        let mut fields = Vec::new();
        for item in raw.as_array()? {
            let field = item.as_object()?;
            only_fields(field, &["memberId", "value"])?;
            let id = parse_identity(required(field, "memberId")?)?;
            fields.push(ReplayStructField {
                member_id: StableUuid::from_bytes(id.to_be_bytes())?,
                value: decode_plc(required(field, "value")?)?,
            });
        }
        ReplayPlcNode::Struct(fields)
    } else if encoding == "char-bytes-unicode" {
        ReplayPlcNode::CharBytes(
            raw.as_str()?
                .chars()
                .map(|c| {
                    u8::try_from(u32::from(c))
                        .map_err(|_| ReplayPackageError::InvalidJson("char bytes"))
                })
                .collect::<Result<_, _>>()?,
        )
    } else {
        ReplayPlcNode::Text(raw.as_str()?.to_owned())
    };
    let decoded = CanonicalReplayPlcValue {
        type_id,
        encoding,
        value: node,
    };
    validate_plc_wire(&decoded)?;
    Ok(decoded)
}

fn decode_actor(value: &JsonValue) -> Result<ReplayActorProvenance, ReplayPackageError> {
    let o = value.as_object()?;
    only_fields(o, &["actorId", "commandId", "idempotencyKey", "kind"])?;
    Ok(ReplayActorProvenance {
        kind: ActorKind::from_token(required(o, "kind")?.as_str()?)?,
        actor_id: parse_identity(required(o, "actorId")?)?,
        command_id: parse_identity(required(o, "commandId")?)?,
        idempotency_key: parse_identity(required(o, "idempotencyKey")?)?,
    })
}
fn decode_result(value: &JsonValue) -> Result<ReplayCommandResult, ReplayPackageError> {
    let o = value.as_object()?;
    only_fields(o, &["code", "detail", "status"])?;
    ReplayCommandResult::new(
        ReplayResultStatus::from_token(required(o, "status")?.as_str()?)?,
        required(o, "code")?.as_str()?,
        decode_payload(required(o, "detail")?)?,
    )
}
fn decode_segment_predecessor(
    value: &JsonValue,
) -> Result<(Option<ReplaySegment>, Option<u64>), ReplayPackageError> {
    if matches!(value, JsonValue::Null) {
        return Ok((None, None));
    }
    let object = value.as_object()?;
    only_fields(
        object,
        &[
            "controllerEpoch",
            "controllerId",
            "eventSequence",
            "universeEpoch",
            "universeId",
        ],
    )?;
    Ok((
        Some(ReplaySegment {
            universe_id: plc_runtime::UniverseId(parse_identity(required(object, "universeId")?)?),
            universe_epoch: required(object, "universeEpoch")?.as_u64()?,
            controller_id: plc_runtime::VirtualControllerId(parse_identity(required(
                object,
                "controllerId",
            )?)?),
            controller_epoch: required(object, "controllerEpoch")?.as_u64()?,
        }),
        Some(required(object, "eventSequence")?.as_u64()?),
    ))
}
fn decode_event(value: &JsonValue) -> Result<ReplayPackageEvent, ReplayPackageError> {
    let o = value.as_object()?;
    only_fields(
        o,
        &[
            "actor",
            "artifactHash",
            "controllerEpoch",
            "controllerId",
            "eventKind",
            "eventSequence",
            "payload",
            "payloadHash",
            "priority",
            "profileHash",
            "result",
            "resultHash",
            "runtimePayloadHash",
            "runtimeResultHash",
            "segmentPredecessor",
            "timelineBranch",
            "universeEpoch",
            "universeId",
            "virtualTimestampMs",
        ],
    )?;
    let payload = decode_payload(required(o, "payload")?)?;
    let result = decode_result(required(o, "result")?)?;
    let actor = decode_actor(required(o, "actor")?)?;
    let (segment_predecessor, predecessor_event_sequence) =
        decode_segment_predecessor(required(o, "segmentPredecessor")?)?;
    if parse_hash(required(o, "payloadHash")?)? != payload.canonical_hash() {
        return Err(ReplayPackageError::PayloadHashMismatch(0));
    }
    if parse_hash(required(o, "resultHash")?)? != result.canonical_hash() {
        return Err(ReplayPackageError::ResultHashMismatch(0));
    }
    Ok(ReplayPackageEvent {
        segment: ReplaySegment {
            universe_id: plc_runtime::UniverseId(parse_identity(required(o, "universeId")?)?),
            universe_epoch: required(o, "universeEpoch")?.as_u64()?,
            controller_id: plc_runtime::VirtualControllerId(parse_identity(required(
                o,
                "controllerId",
            )?)?),
            controller_epoch: required(o, "controllerEpoch")?.as_u64()?,
        },
        segment_predecessor,
        predecessor_event_sequence,
        universe_timeline_branch: required(o, "timelineBranch")?.as_bool()?,
        artifact_hash: parse_hash(required(o, "artifactHash")?)?,
        profile_hash: parse_hash(required(o, "profileHash")?)?,
        kind: parse_event_kind(required(o, "eventKind")?.as_str()?)?,
        event_sequence: required(o, "eventSequence")?.as_u64()?,
        virtual_timestamp_ms: required(o, "virtualTimestampMs")?.as_u64()?,
        priority: ReplayPriorityClass::from_token(required(o, "priority")?.as_str()?)?,
        actor,
        payload,
        result,
        runtime_payload_hash: parse_hash(required(o, "runtimePayloadHash")?)?,
        runtime_result_hash: parse_hash(required(o, "runtimeResultHash")?)?,
    })
}
fn decode_boundary(value: &JsonValue) -> Result<ReplayBoundaryHash, ReplayPackageError> {
    let o = value.as_object()?;
    only_fields(
        o,
        &[
            "boundaryKind",
            "causalInputEventSequence",
            "controllerEpoch",
            "controllerId",
            "eventSequence",
            "regionHashes",
            "runtimeStateHash",
            "scanSequence",
            "semanticStateHash",
            "universeEpoch",
            "universeId",
            "virtualTimestampMs",
        ],
    )?;
    let regions = required(o, "regionHashes")?
        .as_object()?
        .iter()
        .map(|(k, v)| Ok((ReplayStateRegion::from_token(k)?, parse_hash(v)?)))
        .collect::<Result<BTreeMap<_, _>, ReplayPackageError>>()?;
    validate_region_hashes(&regions)?;
    let semantic = parse_hash(required(o, "semanticStateHash")?)?;
    if semantic_region_hash(&regions) != semantic {
        return Err(ReplayPackageError::InvalidJson("semantic state hash"));
    }
    Ok(ReplayBoundaryHash {
        segment: ReplaySegment {
            universe_id: plc_runtime::UniverseId(parse_identity(required(o, "universeId")?)?),
            universe_epoch: required(o, "universeEpoch")?.as_u64()?,
            controller_id: plc_runtime::VirtualControllerId(parse_identity(required(
                o,
                "controllerId",
            )?)?),
            controller_epoch: required(o, "controllerEpoch")?.as_u64()?,
        },
        kind: ReplayBoundaryKind::from_token(required(o, "boundaryKind")?.as_str()?)?,
        causal_input_event_sequence: required(o, "causalInputEventSequence")?.as_u64()?,
        event_sequence: required(o, "eventSequence")?.as_u64()?,
        scan_sequence: required(o, "scanSequence")?.as_u64()?,
        virtual_timestamp_ms: required(o, "virtualTimestampMs")?.as_u64()?,
        runtime_state_hash: parse_hash(required(o, "runtimeStateHash")?)?,
        semantic_state_hash: semantic,
        region_hashes: regions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use plc_runtime::{UniverseId, VirtualControllerId};

    fn hash(byte: u8) -> Hash32 {
        Hash32::from_bytes([byte; 32])
    }

    #[allow(clippy::too_many_lines)]
    fn package() -> ReplayPackage {
        let segment = ReplaySegment {
            universe_id: UniverseId(1),
            universe_epoch: 1,
            controller_id: VirtualControllerId(2),
            controller_epoch: 1,
        };
        let input_payload = ReplayTypedPayload::new(
            ReplayEventKind::RawInputAccepted,
            BTreeMap::from([("value".into(), ReplayPayloadValue::Bool(true))]),
        )
        .unwrap();
        let input_result = ReplayTypedPayload::new(
            ReplayEventKind::RawInputAccepted,
            BTreeMap::from([("accepted".into(), ReplayPayloadValue::Bool(true))]),
        )
        .unwrap();
        let input_event = ReplayPackageEvent {
            segment,
            segment_predecessor: None,
            predecessor_event_sequence: None,
            universe_timeline_branch: false,
            artifact_hash: hash(2),
            profile_hash: hash(3),
            kind: ReplayEventKind::RawInputAccepted,
            event_sequence: 6,
            virtual_timestamp_ms: 0,
            priority: ReplayPriorityClass::RawInput,
            actor: ReplayActorProvenance {
                kind: ActorKind::Operator,
                actor_id: 3,
                command_id: 4,
                idempotency_key: 5,
            },
            payload: input_payload,
            result: ReplayCommandResult::new(
                ReplayResultStatus::Accepted,
                "INPUT_ACCEPTED",
                input_result,
            )
            .unwrap(),
            runtime_payload_hash: hash(4),
            runtime_result_hash: hash(5),
        };
        let payload = ReplayTypedPayload::new(
            ReplayEventKind::ScanCompleted,
            BTreeMap::from([("scan".into(), ReplayPayloadValue::Decimal("1".into()))]),
        )
        .unwrap();
        let result_detail = ReplayTypedPayload::new(
            ReplayEventKind::ScanCompleted,
            BTreeMap::from([("completed".into(), ReplayPayloadValue::Bool(true))]),
        )
        .unwrap();
        let event = ReplayPackageEvent {
            segment,
            segment_predecessor: None,
            predecessor_event_sequence: None,
            universe_timeline_branch: false,
            artifact_hash: hash(2),
            profile_hash: hash(3),
            kind: ReplayEventKind::ScanCompleted,
            event_sequence: 7,
            virtual_timestamp_ms: 10,
            priority: ReplayPriorityClass::ScheduledProgram,
            actor: ReplayActorProvenance {
                kind: ActorKind::System,
                actor_id: 3,
                command_id: 4,
                idempotency_key: 5,
            },
            payload,
            result: ReplayCommandResult::new(
                ReplayResultStatus::Accepted,
                "SCAN_OK",
                result_detail,
            )
            .unwrap(),
            runtime_payload_hash: hash(6),
            runtime_result_hash: hash(7),
        };
        let regions = ReplayStateRegion::DETAILED
            .into_iter()
            .enumerate()
            .map(|(index, region)| (region, hash(u8::try_from(index + 10).unwrap())))
            .collect::<BTreeMap<_, _>>();
        let events = vec![input_event, event];
        let mut boundary = ReplayBoundaryHash {
            segment,
            kind: ReplayBoundaryKind::ScanEnd,
            event_sequence: 7,
            causal_input_event_sequence: 6,
            scan_sequence: 1,
            virtual_timestamp_ms: 10,
            runtime_state_hash: hash(8),
            semantic_state_hash: Hash32::ZERO,
            region_hashes: regions,
        };
        boundary.bind_event_order(&events).unwrap();
        ReplayPackage::encode(ReplayPackageSpec {
            initial_snapshot_hash: hash(1),
            artifact_hash: hash(2),
            profile_hash: hash(3),
            deterministic_seed: 99,
            deterministic_algorithm: "XOSHIRO256SS-1".into(),
            runtime_version: RUNTIME_SEMANTICS_VERSION.into(),
            scheduler_version: SCHEDULER_VERSION.into(),
            events,
            boundaries: vec![boundary],
        })
        .unwrap()
    }

    fn rebuild(members: &[Vec<u8>; 4]) -> Vec<u8> {
        let payload = container_payload(members).unwrap();
        let mut encoded = payload.clone();
        encoded.extend_from_slice(Sha256::digest(&payload).as_bytes());
        encoded
    }

    fn spec_from(package: &ReplayPackage) -> ReplayPackageSpec {
        ReplayPackageSpec {
            initial_snapshot_hash: package.manifest.initial_snapshot_hash,
            artifact_hash: package.manifest.artifact_hash,
            profile_hash: package.manifest.profile_hash,
            deterministic_seed: package.manifest.deterministic_seed,
            deterministic_algorithm: package.manifest.deterministic_algorithm.clone(),
            runtime_version: package.manifest.runtime_version.clone(),
            scheduler_version: package.manifest.scheduler_version.clone(),
            events: package.events.clone(),
            boundaries: package.boundaries.clone(),
        }
    }

    #[test]
    fn canonical_package_is_byte_identical_and_round_trips() {
        let first = package();
        let second = package();
        assert_eq!(first.bytes(), second.bytes());
        assert_eq!(first.members().map(|member| member.name), MEMBER_NAMES);
        let decoded = ReplayPackage::decode(first.bytes(), ReplayDecodeLimits::edu21()).unwrap();
        assert_eq!(decoded.bytes(), first.bytes());
        assert_eq!(decoded.content_fingerprint(), first.content_fingerprint());
    }

    #[test]
    fn tampering_and_resource_weakening_fail_closed() {
        let package = package();
        let mut tampered = package.bytes().to_vec();
        tampered[20] ^= 1;
        assert_eq!(
            ReplayPackage::decode(&tampered, ReplayDecodeLimits::edu21()),
            Err(ReplayPackageError::ContainerIntegrityMismatch)
        );
        let mut limits = ReplayDecodeLimits::edu21();
        limits.max_package_bytes += 1;
        assert_eq!(
            ReplayPackage::decode(package.bytes(), limits),
            Err(ReplayPackageError::InvalidLimits)
        );
    }

    #[test]
    fn reordered_missing_and_noncanonical_members_are_rejected() {
        let package = package();
        let members = package.members().map(|member| member.bytes.to_vec());
        let mut payload = Vec::new();
        payload.extend_from_slice(MAGIC);
        payload.extend_from_slice(&CONTAINER_VERSION.to_be_bytes());
        payload.extend_from_slice(&4_u32.to_be_bytes());
        for index in [1, 0, 2, 3] {
            let name = MEMBER_NAMES[index];
            payload.extend_from_slice(&u16::try_from(name.len()).unwrap().to_be_bytes());
            payload.extend_from_slice(&u64::try_from(members[index].len()).unwrap().to_be_bytes());
            payload.extend_from_slice(name.as_bytes());
            payload.extend_from_slice(&members[index]);
        }
        let mut reordered = payload.clone();
        reordered.extend_from_slice(Sha256::digest(&payload).as_bytes());
        assert!(matches!(
            ReplayPackage::decode(&reordered, ReplayDecodeLimits::edu21()),
            Err(ReplayPackageError::UnexpectedMember { index: 0, .. })
        ));

        let mut missing = package.bytes().to_vec();
        missing[MAGIC.len() + 4..MAGIC.len() + 8].copy_from_slice(&3_u32.to_be_bytes());
        let length = missing.len();
        let digest = Sha256::digest(&missing[..length - 32]);
        missing[length - 32..].copy_from_slice(digest.as_bytes());
        assert_eq!(
            ReplayPackage::decode(&missing, ReplayDecodeLimits::edu21()),
            Err(ReplayPackageError::MemberCount(3))
        );

        let mut spaced = members;
        spaced[0].insert(1, b' ');
        assert_eq!(
            ReplayPackage::decode(&rebuild(&spaced), ReplayDecodeLimits::edu21()),
            Err(ReplayPackageError::NonCanonicalJson(MEMBER_NAMES[0]))
        );
    }

    #[test]
    fn fingerprint_hash_domain_omits_its_own_field_and_detects_forgery() {
        let package = package();
        let mut members = package.members().map(|member| member.bytes.to_vec());
        let mut manifest = parse_json(&members[0], ReplayDecodeLimits::edu21().json()).unwrap();
        let JsonValue::Object(fields) = &mut manifest else {
            panic!("manifest object")
        };
        fields.remove("contentFingerprint");
        let identity_members = [
            canonical_json(&manifest),
            members[1].clone(),
            members[2].clone(),
            members[3].clone(),
        ];
        assert_eq!(
            Sha256::digest(&container_payload(&identity_members).unwrap()),
            package.content_fingerprint()
        );

        let JsonValue::Object(fields) =
            parse_json(&members[0], ReplayDecodeLimits::edu21().json()).unwrap()
        else {
            panic!("manifest object")
        };
        let mut forged = fields;
        forged.insert("contentFingerprint".into(), hash_json(hash(99)));
        members[0] = canonical_json(&JsonValue::Object(forged));
        assert_eq!(
            ReplayPackage::decode(&rebuild(&members), ReplayDecodeLimits::edu21()),
            Err(ReplayPackageError::ContentFingerprintMismatch)
        );
    }

    #[test]
    fn jsonl_bom_cr_missing_lf_and_unknown_fields_fail_closed() {
        let package = package();
        for mutation in 0..3 {
            let mut members = package.members().map(|member| member.bytes.to_vec());
            match mutation {
                0 => {
                    members[2].splice(0..0, [0xef, 0xbb, 0xbf]);
                }
                1 => {
                    members[2].push(b'\r');
                }
                _ => {
                    members[2].pop();
                }
            }
            assert!(
                ReplayPackage::decode(&rebuild(&members), ReplayDecodeLimits::edu21()).is_err()
            );
        }
        let mut members = package.members().map(|member| member.bytes.to_vec());
        let first_line = members[2].split(|byte| *byte == b'\n').next().unwrap();
        let mut value = parse_json(first_line, ReplayDecodeLimits::edu21().json()).unwrap();
        let JsonValue::Object(fields) = &mut value else {
            panic!("event object")
        };
        fields.insert("uiViewport".into(), JsonValue::String("forbidden".into()));
        let mut events = canonical_json(&value);
        events.push(b'\n');
        let second_line_start = members[2].iter().position(|byte| *byte == b'\n').unwrap() + 1;
        events.extend_from_slice(&members[2][second_line_start..]);
        members[2] = events;
        assert!(matches!(
            ReplayPackage::decode(&rebuild(&members), ReplayDecodeLimits::edu21()),
            Err(ReplayPackageError::InvalidEvent(0) | ReplayPackageError::NonCanonicalJson("jsonl"))
        ));
    }

    #[test]
    fn event_order_segment_boundary_and_causal_rules_are_closed() {
        let package = package();
        let mut unordered = spec_from(&package);
        unordered.events.reverse();
        assert_eq!(
            ReplayPackage::encode(unordered),
            Err(ReplayPackageError::NonCanonicalEventOrder)
        );

        let mut missing = spec_from(&package);
        missing.boundaries.clear();
        assert_eq!(
            ReplayPackage::encode(missing),
            Err(ReplayPackageError::MissingBoundaryForEvent(7))
        );

        let mut bad_cause = spec_from(&package);
        bad_cause.boundaries[0].causal_input_event_sequence = 7;
        assert_eq!(
            ReplayPackage::encode(bad_cause),
            Err(ReplayPackageError::OrphanBoundary(7))
        );
    }

    #[test]
    fn runtime_aggregate_region_is_bound_to_the_runtime_boundary_hash() {
        let package = package();
        let mut spec = spec_from(&package);
        spec.boundaries[0]
            .region_hashes
            .insert(ReplayStateRegion::Runtime, hash(99));
        spec.boundaries[0].semantic_state_hash =
            semantic_region_hash(&spec.boundaries[0].region_hashes);
        assert_eq!(
            ReplayPackage::encode(spec),
            Err(ReplayPackageError::InvalidBoundary(0))
        );
    }

    #[test]
    fn char_string_bytes_use_forced_unicode_escapes() {
        let data_type = CanonicalType::Primitive(PrimitiveType::String(3));
        let scalar = plc_types::TypedScalar::new(
            PrimitiveType::String(3),
            ScalarValue::String(vec![0, 65, 255]),
        )
        .unwrap();
        let value = CanonicalReplayPlcValue::from_plc(
            &data_type,
            &PlcValue::scalar(scalar),
            AggregateLimits::edu21(),
        )
        .unwrap();
        assert_eq!(value.canonical_json_bytes(), br#"{"encoding":"char-bytes-unicode","typeId":"STRING[3]","value":"\u0000\u0041\u00ff"}"#);
    }

    #[test]
    fn canonical_plc_values_use_text_and_fixed_bit_encodings() {
        let signed = CanonicalReplayPlcValue::from_runtime(CanonicalValue::I32(-9)).unwrap();
        assert_eq!(
            signed.canonical_json_bytes(),
            br#"{"encoding":"decimal-text","typeId":"DINT","value":"-9"}"#
        );
        let bits = CanonicalReplayPlcValue::from_runtime(CanonicalValue::Bits32(0x12)).unwrap();
        assert_eq!(
            bits.canonical_json_bytes(),
            br#"{"encoding":"fixed-width-hex","typeId":"DWORD","value":"00000012"}"#
        );
        let negative_zero = CanonicalReplayPlcValue::from_runtime(CanonicalValue::F32(
            plc_types::CanonicalF32::new(-0.0),
        ))
        .unwrap();
        assert_eq!(
            negative_zero.canonical_json_bytes(),
            br#"{"encoding":"ieee-bits-hex","typeId":"REAL","value":"80000000"}"#
        );
    }

    #[test]
    fn first_divergence_stops_at_first_boundary_and_reports_regions() {
        let package = package();
        let mut observed = package.boundaries().to_vec();
        observed[0]
            .region_hashes
            .insert(ReplayStateRegion::Io, hash(99));
        observed[0].semantic_state_hash = semantic_region_hash(&observed[0].region_hashes);
        let divergence = package.first_divergence(&observed).unwrap().unwrap();
        assert_eq!(divergence.boundary_index, 0);
        assert_eq!(divergence.differing_regions, vec![ReplayStateRegion::Io]);
        assert_eq!(divergence.causal_event.unwrap().event_sequence, 6);
    }
}
