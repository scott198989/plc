use alloc::{collections::BTreeSet, vec::Vec};
use core::{error::Error, fmt};

use plc_runtime::CanonicalValue as RuntimeValue;
use plc_types::{
    AggregateLimits, CanonicalType, PlcValue, PrimitiveType, ScalarValue, TypeError, TypedScalar,
};

use crate::{ForceProvenance, PublishedTargetValue, Quality, SampleFreshness, StableTargetId};

const RECORD_MAGIC: &[u8; 16] = b"PES-OBS-LAYERS-1";
const BUNDLE_MAGIC: &[u8; 16] = b"PES-OBS-BUNDLE-1";
const LAYER_SCHEMA_VERSION: u16 = 1;
const MAX_STANDARD_RECORDS: u32 = 4_096;
const MAX_STANDARD_RECORD_BYTES: usize = 64 * 1_024 * 1_024;
const MAX_STANDARD_BUNDLE_BYTES: usize = 128 * 1_024 * 1_024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayerCodecLimits {
    pub aggregate: AggregateLimits,
    pub max_records: u32,
    pub max_record_bytes: usize,
    pub max_bundle_bytes: usize,
}

impl LayerCodecLimits {
    #[must_use]
    pub const fn edu21() -> Self {
        Self {
            aggregate: AggregateLimits::edu21(),
            max_records: MAX_STANDARD_RECORDS,
            max_record_bytes: MAX_STANDARD_RECORD_BYTES,
            max_bundle_bytes: MAX_STANDARD_BUNDLE_BYTES,
        }
    }

    fn validate(self) -> Result<(), LayerError> {
        if self.max_records == 0
            || self.max_records > MAX_STANDARD_RECORDS
            || self.max_record_bytes == 0
            || self.max_record_bytes > MAX_STANDARD_RECORD_BYTES
            || self.max_bundle_bytes == 0
            || self.max_bundle_bytes > MAX_STANDARD_BUNDLE_BYTES
            || self.max_record_bytes > self.max_bundle_bytes
        {
            return Err(LayerError::InvalidLimits);
        }
        Ok(())
    }
}

impl Default for LayerCodecLimits {
    fn default() -> Self {
        Self::edu21()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum LayerTargetKind {
    Memory = 1,
    Input = 2,
    Output = 3,
}

/// Engineering-owned values. Every field remains independent; none is an alias
/// for an online or loaded value.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EngineeringValueLayers {
    pub declared_default: Option<PlcValue>,
    pub declared_start: Option<PlcValue>,
    pub current_offline: Option<PlcValue>,
    pub constant: Option<PlcValue>,
    pub loaded_start: Option<PlcValue>,
    pub working: Option<PlcValue>,
}

/// Runtime and virtual-I/O values. Quality, freshness, and force provenance are
/// deliberately outside this value group.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuntimeValueLayers {
    pub actual: Option<PlcValue>,
    pub retained: Option<PlcValue>,
    pub snapshot: Option<PlcValue>,
    pub raw_input: Option<PlcValue>,
    pub natural: Option<PlcValue>,
    pub effective: Option<PlcValue>,
    pub committed_output: Option<PlcValue>,
    pub delivered_output: Option<PlcValue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayerForce {
    pub value: PlcValue,
    pub provenance: ForceProvenance,
}

/// One exact, typed layer record. Construction validates the whole record before
/// it can be serialized; callers never receive a partially admitted layer set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalLayerSnapshot {
    pub target_id: StableTargetId,
    pub target_kind: LayerTargetKind,
    pub data_type: CanonicalType,
    pub engineering: EngineeringValueLayers,
    pub runtime: RuntimeValueLayers,
    pub quality: Quality,
    pub freshness: SampleFreshness,
    pub force: Option<LayerForce>,
}

impl CanonicalLayerSnapshot {
    /// Validates type, shape, target-specific I/O layers, and force precedence.
    ///
    /// # Errors
    ///
    /// Returns a deterministic error for an empty snapshot, wrong target layer,
    /// type/shape mismatch, or force/effective disagreement.
    pub fn validate(&self, limits: LayerCodecLimits) -> Result<(), LayerError> {
        limits.validate()?;
        self.data_type.validate(limits.aggregate)?;
        let values = self.values();
        if values.iter().all(|value| value.is_none()) && self.force.is_none() {
            return Err(LayerError::NoValues);
        }
        for value in values.into_iter().flatten() {
            self.data_type.validate_value(value, limits.aggregate)?;
        }
        match self.target_kind {
            LayerTargetKind::Memory => {
                if self.runtime.raw_input.is_some()
                    || self.runtime.committed_output.is_some()
                    || self.runtime.delivered_output.is_some()
                {
                    return Err(LayerError::LayerUnavailableForTarget);
                }
            }
            LayerTargetKind::Input => {
                if self.runtime.committed_output.is_some()
                    || self.runtime.delivered_output.is_some()
                {
                    return Err(LayerError::LayerUnavailableForTarget);
                }
            }
            LayerTargetKind::Output => {
                if self.runtime.raw_input.is_some() {
                    return Err(LayerError::LayerUnavailableForTarget);
                }
            }
        }
        if let Some(force) = &self.force {
            self.data_type
                .validate_value(&force.value, limits.aggregate)?;
            if self.runtime.effective.as_ref() != Some(&force.value) {
                return Err(LayerError::ForceEffectiveMismatch);
            }
        }
        Ok(())
    }

    fn values(&self) -> [Option<&PlcValue>; 14] {
        [
            self.engineering.declared_default.as_ref(),
            self.engineering.declared_start.as_ref(),
            self.engineering.current_offline.as_ref(),
            self.engineering.constant.as_ref(),
            self.engineering.loaded_start.as_ref(),
            self.engineering.working.as_ref(),
            self.runtime.actual.as_ref(),
            self.runtime.retained.as_ref(),
            self.runtime.snapshot.as_ref(),
            self.runtime.raw_input.as_ref(),
            self.runtime.natural.as_ref(),
            self.runtime.effective.as_ref(),
            self.runtime.committed_output.as_ref(),
            self.runtime.delivered_output.as_ref(),
        ]
    }

    /// Encodes all layers in a fixed tagged order with big-endian lengths and
    /// the shared canonical PLC type/value codec.
    ///
    /// # Errors
    ///
    /// Returns a validation or capacity error without exposing partial bytes.
    pub fn canonical_bytes(&self, limits: LayerCodecLimits) -> Result<Vec<u8>, LayerError> {
        self.validate(limits)?;
        let type_bytes = self.data_type.canonical_bytes(limits.aggregate)?;
        let mut encoder = Encoder::new(limits.max_record_bytes);
        encoder.raw(RECORD_MAGIC)?;
        encoder.u16(LAYER_SCHEMA_VERSION)?;
        encoder.u128(self.target_id.0)?;
        encoder.u8(self.target_kind as u8)?;
        encoder.length_prefixed(&type_bytes)?;
        for (tag, value) in self.values().into_iter().enumerate() {
            encoder.u8(u8::try_from(tag + 1).map_err(|_| LayerError::LengthOverflow)?)?;
            encode_optional_value(&self.data_type, value, limits, &mut encoder)?;
        }
        encoder.u8(15)?;
        encoder.u8(self.quality as u8)?;
        encoder.u8(16)?;
        encoder.u8(self.freshness as u8)?;
        encoder.u8(17)?;
        match &self.force {
            Some(force) => {
                encoder.u8(1)?;
                encoder.u128(force.provenance.force_id)?;
                encoder.u64(force.provenance.registry_version)?;
                let bytes = self
                    .data_type
                    .serialize_value(&force.value, limits.aggregate)?;
                encoder.length_prefixed(&bytes)?;
            }
            None => encoder.u8(0)?,
        }
        Ok(encoder.finish())
    }

    /// Compares bytes to this record's exact canonical form.
    ///
    /// # Errors
    ///
    /// Returns a validation/capacity error before comparison.
    pub fn verify_canonical_bytes(
        &self,
        bytes: &[u8],
        limits: LayerCodecLimits,
    ) -> Result<bool, LayerError> {
        Ok(self.canonical_bytes(limits)? == bytes)
    }
}

fn encode_optional_value(
    data_type: &CanonicalType,
    value: Option<&PlcValue>,
    limits: LayerCodecLimits,
    encoder: &mut Encoder,
) -> Result<(), LayerError> {
    match value {
        Some(value) => {
            encoder.u8(1)?;
            let bytes = data_type.serialize_value(value, limits.aggregate)?;
            encoder.length_prefixed(&bytes)
        }
        None => encoder.u8(0),
    }
}

/// Canonical ordered collection used by snapshot/replay evidence. Target IDs
/// are unique and records are serialized in `(target_id, target_kind)` order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalLayerBundle {
    records: Vec<CanonicalLayerSnapshot>,
}

impl CanonicalLayerBundle {
    /// Admits and orders a complete bounded bundle atomically.
    ///
    /// # Errors
    ///
    /// Rejects empty, duplicate, invalid, or over-count input.
    pub fn new(
        mut records: Vec<CanonicalLayerSnapshot>,
        limits: LayerCodecLimits,
    ) -> Result<Self, LayerError> {
        limits.validate()?;
        if records.is_empty() {
            return Err(LayerError::NoRecords);
        }
        if u32::try_from(records.len()).map_or(true, |count| count > limits.max_records) {
            return Err(LayerError::RecordLimit);
        }
        records.sort_by_key(|record| (record.target_id, record.target_kind));
        let mut ids = BTreeSet::new();
        for record in &records {
            record.validate(limits)?;
            if !ids.insert(record.target_id) {
                return Err(LayerError::DuplicateTarget(record.target_id));
            }
        }
        Ok(Self { records })
    }

    pub fn records(&self) -> impl ExactSizeIterator<Item = &CanonicalLayerSnapshot> {
        self.records.iter()
    }

    /// Encodes length-delimited canonical records under a separate bundle cap.
    ///
    /// # Errors
    ///
    /// Returns a record or bundle capacity error with no partial output.
    pub fn canonical_bytes(&self, limits: LayerCodecLimits) -> Result<Vec<u8>, LayerError> {
        limits.validate()?;
        if u32::try_from(self.records.len()).map_or(true, |count| count > limits.max_records) {
            return Err(LayerError::RecordLimit);
        }
        let mut encoder = Encoder::new(limits.max_bundle_bytes);
        encoder.raw(BUNDLE_MAGIC)?;
        encoder.u16(LAYER_SCHEMA_VERSION)?;
        encoder.u32(u32::try_from(self.records.len()).map_err(|_| LayerError::LengthOverflow)?)?;
        for record in &self.records {
            encoder.length_prefixed(&record.canonical_bytes(limits)?)?;
        }
        Ok(encoder.finish())
    }
}

/// Scalar declaration/load inputs for adapting the existing production runtime
/// publication boundary into the shared layered record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScalarEngineeringValueLayers {
    pub declared_default: Option<RuntimeValue>,
    pub declared_start: Option<RuntimeValue>,
    pub current_offline: Option<RuntimeValue>,
    pub constant: Option<RuntimeValue>,
    pub loaded_start: Option<RuntimeValue>,
    pub working: Option<RuntimeValue>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScalarRuntimeValueLayers {
    pub actual: Option<RuntimeValue>,
    pub retained: Option<RuntimeValue>,
    pub snapshot: Option<RuntimeValue>,
}

/// Adapts the real scalar runtime publication model without changing a value,
/// type identity, float bit pattern, quality, freshness, or force provenance.
///
/// # Errors
///
/// Rejects any mismatched publication type or target-inapplicable I/O layer.
#[allow(clippy::too_many_arguments)]
pub fn scalar_layer_snapshot_from_publication(
    target_id: StableTargetId,
    target_kind: LayerTargetKind,
    engineering: ScalarEngineeringValueLayers,
    runtime: ScalarRuntimeValueLayers,
    publication: PublishedTargetValue,
    freshness: SampleFreshness,
    limits: LayerCodecLimits,
) -> Result<CanonicalLayerSnapshot, LayerError> {
    if publication.target_id != target_id {
        return Err(LayerError::RuntimeTargetMismatch);
    }
    let primitive = publication.value_type.primitive_type();
    let data_type = CanonicalType::Primitive(primitive);
    let engineering = EngineeringValueLayers {
        declared_default: transpose_runtime_value(engineering.declared_default, primitive)?,
        declared_start: transpose_runtime_value(engineering.declared_start, primitive)?,
        current_offline: transpose_runtime_value(engineering.current_offline, primitive)?,
        constant: transpose_runtime_value(engineering.constant, primitive)?,
        loaded_start: transpose_runtime_value(engineering.loaded_start, primitive)?,
        working: transpose_runtime_value(engineering.working, primitive)?,
    };
    let runtime = RuntimeValueLayers {
        actual: transpose_runtime_value(runtime.actual, primitive)?,
        retained: transpose_runtime_value(runtime.retained, primitive)?,
        snapshot: transpose_runtime_value(runtime.snapshot, primitive)?,
        raw_input: transpose_runtime_value(publication.raw_input_value, primitive)?,
        natural: Some(runtime_value(publication.natural_value, primitive)?),
        effective: Some(runtime_value(publication.effective_value, primitive)?),
        committed_output: transpose_runtime_value(publication.committed_output_value, primitive)?,
        delivered_output: transpose_runtime_value(publication.delivered_output_value, primitive)?,
    };
    let force = publication
        .force
        .map(|provenance| -> Result<LayerForce, LayerError> {
            Ok(LayerForce {
                value: runtime_value(publication.effective_value, primitive)?,
                provenance,
            })
        })
        .transpose()?;
    let snapshot = CanonicalLayerSnapshot {
        target_id,
        target_kind,
        data_type,
        engineering,
        runtime,
        quality: publication.quality,
        freshness,
        force,
    };
    snapshot.validate(limits)?;
    Ok(snapshot)
}

fn transpose_runtime_value(
    value: Option<RuntimeValue>,
    expected: PrimitiveType,
) -> Result<Option<PlcValue>, LayerError> {
    value
        .map(|value| runtime_value(value, expected))
        .transpose()
}

fn runtime_value(value: RuntimeValue, expected: PrimitiveType) -> Result<PlcValue, LayerError> {
    if value.value_type().primitive_type() != expected {
        return Err(LayerError::RuntimeTypeMismatch);
    }
    let scalar = match value {
        RuntimeValue::Bool(value) => ScalarValue::Bool(value),
        RuntimeValue::I8(value) => ScalarValue::Signed(i64::from(value)),
        RuntimeValue::I16(value) => ScalarValue::Signed(i64::from(value)),
        RuntimeValue::I32(value) => ScalarValue::Signed(i64::from(value)),
        RuntimeValue::I64(value) => ScalarValue::Signed(value),
        RuntimeValue::U8(value) => ScalarValue::Unsigned(u64::from(value)),
        RuntimeValue::U16(value) => ScalarValue::Unsigned(u64::from(value)),
        RuntimeValue::U32(value) => ScalarValue::Unsigned(u64::from(value)),
        RuntimeValue::U64(value) => ScalarValue::Unsigned(value),
        RuntimeValue::Bits8(value) => ScalarValue::BitString(u64::from(value)),
        RuntimeValue::Bits16(value) => ScalarValue::BitString(u64::from(value)),
        RuntimeValue::Bits32(value) => ScalarValue::BitString(u64::from(value)),
        RuntimeValue::Bits64(value) => ScalarValue::BitString(value),
        RuntimeValue::F32(value) => ScalarValue::Real(value),
        RuntimeValue::F64(value) => ScalarValue::Lreal(value),
        RuntimeValue::Char(value) => ScalarValue::Char(value),
        RuntimeValue::TimeMs(value) => ScalarValue::Time(value),
    };
    TypedScalar::new(expected, scalar)
        .map(PlcValue::scalar)
        .map_err(|_| LayerError::RuntimeTypeMismatch)
}

struct Encoder {
    bytes: Vec<u8>,
    limit: usize,
}

impl Encoder {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
        }
    }

    fn raw(&mut self, value: &[u8]) -> Result<(), LayerError> {
        let next = self
            .bytes
            .len()
            .checked_add(value.len())
            .ok_or(LayerError::CapacityExceeded)?;
        if next > self.limit {
            return Err(LayerError::CapacityExceeded);
        }
        self.bytes.extend_from_slice(value);
        Ok(())
    }

    fn u8(&mut self, value: u8) -> Result<(), LayerError> {
        self.raw(&[value])
    }

    fn u16(&mut self, value: u16) -> Result<(), LayerError> {
        self.raw(&value.to_be_bytes())
    }

    fn u32(&mut self, value: u32) -> Result<(), LayerError> {
        self.raw(&value.to_be_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), LayerError> {
        self.raw(&value.to_be_bytes())
    }

    fn u128(&mut self, value: u128) -> Result<(), LayerError> {
        self.raw(&value.to_be_bytes())
    }

    fn length_prefixed(&mut self, value: &[u8]) -> Result<(), LayerError> {
        self.u32(u32::try_from(value.len()).map_err(|_| LayerError::LengthOverflow)?)?;
        self.raw(value)
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayerError {
    InvalidLimits,
    NoValues,
    NoRecords,
    RecordLimit,
    DuplicateTarget(StableTargetId),
    LayerUnavailableForTarget,
    ForceEffectiveMismatch,
    RuntimeTargetMismatch,
    RuntimeTypeMismatch,
    CapacityExceeded,
    LengthOverflow,
    Type(TypeError),
}

impl From<TypeError> for LayerError {
    fn from(value: TypeError) -> Self {
        Self::Type(value)
    }
}

impl fmt::Display for LayerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "layer snapshot rejected: {self:?}")
    }
}

impl Error for LayerError {}
