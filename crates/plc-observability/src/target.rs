use alloc::{collections::BTreeMap, string::String, vec::Vec};
use core::{error::Error, fmt};

use plc_runtime::{ChannelId, Hash32, MemoryId, ValueType};

use crate::canonical::CanonicalHasher;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StableTargetId(pub u128);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum RuntimeTarget {
    Memory(MemoryId) = 1,
    Input(ChannelId) = 2,
    Output(ChannelId) = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ProbeLayer {
    Natural = 1,
    Effective = 2,
    RawInput = 3,
    CommittedOutput = 4,
    DeliveredOutput = 5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BitRange {
    pub offset: u16,
    pub width: u16,
}

impl BitRange {
    pub fn new(offset: u16, width: u16) -> Result<Self, TargetError> {
        if width == 0 || u32::from(offset) + u32::from(width) > 128 {
            return Err(TargetError::InvalidBitRange { offset, width });
        }
        Ok(Self { offset, width })
    }

    pub const fn whole_value() -> Self {
        Self {
            offset: 0,
            width: 128,
        }
    }

    pub const fn overlaps(self, other: Self) -> bool {
        let self_end = self.offset as u32 + self.width as u32;
        let other_end = other.offset as u32 + other.width as u32;
        self.offset < other_end as u16 && other.offset < self_end as u16
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccessCapabilities {
    pub monitor: bool,
    pub modify: bool,
    pub force: bool,
    pub trace: bool,
    pub natural_layer: bool,
    pub effective_layer: bool,
}

impl AccessCapabilities {
    pub const fn permits_layer(self, layer: ProbeLayer) -> bool {
        match layer {
            ProbeLayer::Natural => self.natural_layer,
            ProbeLayer::Effective => self.effective_layer,
            ProbeLayer::RawInput | ProbeLayer::CommittedOutput | ProbeLayer::DeliveredOutput => {
                self.monitor
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceAnchor {
    pub artifact_fingerprint: Hash32,
    pub file_identity: u128,
    pub semantic_identity: u128,
    pub start_utf16: u32,
    pub end_utf16: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbeDefinition {
    pub id: StableTargetId,
    pub runtime_target: RuntimeTarget,
    pub bit_range: BitRange,
    pub value_type: ValueType,
    pub instance_path: Vec<u128>,
    pub capabilities: AccessCapabilities,
    pub primary_source: Option<SourceAnchor>,
    pub display_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TargetReference {
    Stable(StableTargetId),
    SourceOnly(SourceAnchor),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedTarget {
    pub id: StableTargetId,
    pub runtime_target: RuntimeTarget,
    pub bit_range: BitRange,
    pub value_type: ValueType,
    pub instance_path: Vec<u128>,
    pub layer: ProbeLayer,
    pub source: Option<SourceAnchor>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbeCatalog {
    artifact_fingerprint: Hash32,
    profile_fingerprint: Hash32,
    definitions: BTreeMap<StableTargetId, ProbeDefinition>,
    catalog_hash: Hash32,
}

impl ProbeCatalog {
    pub fn new(artifact_fingerprint: Hash32, profile_fingerprint: Hash32) -> Self {
        let mut value = Self {
            artifact_fingerprint,
            profile_fingerprint,
            definitions: BTreeMap::new(),
            catalog_hash: Hash32::ZERO,
        };
        value.catalog_hash = value.calculate_hash();
        value
    }

    pub fn insert(&mut self, definition: ProbeDefinition) -> Result<(), TargetError> {
        if definition.display_name.is_empty() {
            return Err(TargetError::EmptyDisplayName);
        }
        if let Some(source) = &definition.primary_source
            && source.artifact_fingerprint != self.artifact_fingerprint
        {
            return Err(TargetError::SourceArtifactMismatch);
        }
        if self.definitions.contains_key(&definition.id) {
            return Err(TargetError::DuplicateStableIdentity(definition.id));
        }
        self.definitions.insert(definition.id, definition);
        self.catalog_hash = self.calculate_hash();
        Ok(())
    }

    pub fn resolve(
        &self,
        reference: &TargetReference,
        layer: ProbeLayer,
        expected_artifact_fingerprint: Hash32,
        expected_profile_fingerprint: Hash32,
    ) -> Result<ResolvedTarget, TargetError> {
        if self.artifact_fingerprint != expected_artifact_fingerprint {
            return Err(TargetError::ArtifactMismatch);
        }
        if self.profile_fingerprint != expected_profile_fingerprint {
            return Err(TargetError::ProfileMismatch);
        }
        let id = match reference {
            TargetReference::Stable(id) => *id,
            TargetReference::SourceOnly(_) => return Err(TargetError::SourceOnlyReference),
        };
        let definition = self
            .definitions
            .get(&id)
            .ok_or(TargetError::UnknownStableIdentity(id))?;
        if !definition.capabilities.permits_layer(layer) {
            return Err(TargetError::LayerUnavailable { id, layer });
        }
        let layer_matches_target = match layer {
            ProbeLayer::RawInput => matches!(definition.runtime_target, RuntimeTarget::Input(_)),
            ProbeLayer::CommittedOutput | ProbeLayer::DeliveredOutput => {
                matches!(definition.runtime_target, RuntimeTarget::Output(_))
            }
            ProbeLayer::Natural | ProbeLayer::Effective => true,
        };
        if !layer_matches_target {
            return Err(TargetError::LayerUnavailable { id, layer });
        }
        Ok(ResolvedTarget {
            id,
            runtime_target: definition.runtime_target,
            bit_range: definition.bit_range,
            value_type: definition.value_type,
            instance_path: definition.instance_path.clone(),
            layer,
            source: definition.primary_source.clone(),
        })
    }

    pub const fn artifact_fingerprint(&self) -> Hash32 {
        self.artifact_fingerprint
    }

    pub const fn profile_fingerprint(&self) -> Hash32 {
        self.profile_fingerprint
    }

    pub const fn catalog_hash(&self) -> Hash32 {
        self.catalog_hash
    }

    pub fn definition(&self, id: StableTargetId) -> Option<&ProbeDefinition> {
        self.definitions.get(&id)
    }

    pub fn definitions(&self) -> impl ExactSizeIterator<Item = &ProbeDefinition> {
        self.definitions.values()
    }

    fn calculate_hash(&self) -> Hash32 {
        let mut hasher = CanonicalHasher::new("PES-PROBE-CATALOG-1");
        hasher.hash(self.artifact_fingerprint);
        hasher.hash(self.profile_fingerprint);
        hasher.u64(self.definitions.len() as u64);
        for definition in self.definitions.values() {
            hasher.u128(definition.id.0);
            encode_runtime_target(definition.runtime_target, &mut hasher);
            hasher.u16(definition.bit_range.offset);
            hasher.u16(definition.bit_range.width);
            hasher.u8(definition.value_type as u8);
            hasher.u64(definition.instance_path.len() as u64);
            for identity in &definition.instance_path {
                hasher.u128(*identity);
            }
            encode_capabilities(definition.capabilities, &mut hasher);
            match &definition.primary_source {
                Some(source) => {
                    hasher.bool(true);
                    encode_source_anchor(source, &mut hasher);
                }
                None => hasher.bool(false),
            }
            hasher.string(&definition.display_name);
        }
        hasher.finish()
    }
}

pub(crate) fn encode_runtime_target(target: RuntimeTarget, hasher: &mut CanonicalHasher) {
    match target {
        RuntimeTarget::Memory(id) => {
            hasher.u8(1);
            hasher.u32(id.0);
        }
        RuntimeTarget::Input(id) => {
            hasher.u8(2);
            hasher.u32(id.0);
        }
        RuntimeTarget::Output(id) => {
            hasher.u8(3);
            hasher.u32(id.0);
        }
    }
}

pub(crate) fn encode_source_anchor(source: &SourceAnchor, hasher: &mut CanonicalHasher) {
    hasher.hash(source.artifact_fingerprint);
    hasher.u128(source.file_identity);
    hasher.u128(source.semantic_identity);
    hasher.u32(source.start_utf16);
    hasher.u32(source.end_utf16);
}

fn encode_capabilities(capabilities: AccessCapabilities, hasher: &mut CanonicalHasher) {
    hasher.bool(capabilities.monitor);
    hasher.bool(capabilities.modify);
    hasher.bool(capabilities.force);
    hasher.bool(capabilities.trace);
    hasher.bool(capabilities.natural_layer);
    hasher.bool(capabilities.effective_layer);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TargetError {
    InvalidBitRange {
        offset: u16,
        width: u16,
    },
    EmptyDisplayName,
    DuplicateStableIdentity(StableTargetId),
    UnknownStableIdentity(StableTargetId),
    SourceOnlyReference,
    SourceArtifactMismatch,
    ArtifactMismatch,
    ProfileMismatch,
    LayerUnavailable {
        id: StableTargetId,
        layer: ProbeLayer,
    },
}

impl fmt::Display for TargetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "probe target rejected: {self:?}")
    }
}

impl Error for TargetError {}
