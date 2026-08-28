use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};
use core::{error::Error, fmt};

use plc_runtime::Hash32;

use crate::{
    SourceAnchor, StableTargetId, canonical::CanonicalHasher, target::encode_source_anchor,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticIdentity(pub u128);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ArtifactSide {
    CurrentOffline = 1,
    Loaded = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum NavigationKind {
    ProjectObject = 1,
    HardwareObject = 2,
    ProgramObject = 3,
    SourceSpan = 4,
    ProbeTarget = 5,
    DiagnosticSubject = 6,
    Tombstone = 7,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum NavigationRelationshipKind {
    Selected = 1,
    Definition = 2,
    Use = 3,
    Call = 4,
    Assignment = 5,
    AddressOverlap = 6,
    TypeDependency = 7,
    HardwareBinding = 8,
    DiagnosticPrimary = 9,
    DiagnosticReference = 10,
    ProbeReference = 11,
    ForceReference = 12,
    TraceReference = 13,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum NavigationValidity {
    Valid = 1,
    StaleArtifact = 2,
    TargetRemoved = 3,
    Unresolved = 4,
    Ambiguous = 5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LoadedArtifactBinding {
    pub fingerprint: Hash32,
    pub controller_epoch: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NavigationDomainProjection {
    ProjectObject {
        object_identity: u128,
    },
    ProgramMember {
        owner_identity: u128,
        member_identity: u128,
    },
    HardwareObject {
        object_identity: u128,
    },
    ProbeTarget {
        target: StableTargetId,
    },
    DiagnosticSubject {
        subject_identity: u128,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationAnchor {
    pub identity: SemanticIdentity,
    pub kind: NavigationKind,
    pub side: ArtifactSide,
    pub artifact_fingerprint: Hash32,
    pub controller_epoch: Option<u64>,
    pub source: Option<SourceAnchor>,
    pub domain_projection: Option<NavigationDomainProjection>,
    pub probe_target: Option<StableTargetId>,
    pub relationship_kind: NavigationRelationshipKind,
    pub validity: NavigationValidity,
    pub tombstone_reason_hash: Option<Hash32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationResult {
    pub primary: NavigationAnchor,
    pub related: Vec<NavigationAnchor>,
    pub index_revision: u64,
    pub index_hash: Hash32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationIndex {
    revision: u64,
    offline_artifact_fingerprint: Hash32,
    loaded_artifact: Option<LoadedArtifactBinding>,
    anchors: BTreeMap<(SemanticIdentity, ArtifactSide), NavigationAnchor>,
    relationships:
        BTreeMap<SemanticIdentity, BTreeSet<(NavigationRelationshipKind, SemanticIdentity)>>,
    diagnostic_primary: BTreeMap<u128, SemanticIdentity>,
    diagnostic_related: BTreeMap<u128, BTreeSet<(NavigationRelationshipKind, SemanticIdentity)>>,
    index_hash: Hash32,
}

impl NavigationIndex {
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    pub const fn index_hash(&self) -> Hash32 {
        self.index_hash
    }

    pub fn resolve(
        &self,
        identity: SemanticIdentity,
        preferred_side: ArtifactSide,
    ) -> Result<NavigationResult, NavigationError> {
        let mut primary = self
            .anchors
            .get(&(identity, preferred_side))
            .or_else(|| {
                let alternate = match preferred_side {
                    ArtifactSide::CurrentOffline => ArtifactSide::Loaded,
                    ArtifactSide::Loaded => ArtifactSide::CurrentOffline,
                };
                self.anchors.get(&(identity, alternate))
            })
            .cloned()
            .ok_or(NavigationError::UnknownIdentity(identity))?;
        primary.relationship_kind = NavigationRelationshipKind::Selected;
        self.apply_artifact_validity(&mut primary);
        let related = self
            .relationships
            .get(&identity)
            .into_iter()
            .flat_map(|relationships| relationships.iter())
            .filter_map(|(kind, related)| {
                self.anchors
                    .get(&(*related, preferred_side))
                    .or_else(|| {
                        let alternate = match preferred_side {
                            ArtifactSide::CurrentOffline => ArtifactSide::Loaded,
                            ArtifactSide::Loaded => ArtifactSide::CurrentOffline,
                        };
                        self.anchors.get(&(*related, alternate))
                    })
                    .cloned()
                    .map(|mut anchor| {
                        anchor.relationship_kind = *kind;
                        self.apply_artifact_validity(&mut anchor);
                        anchor
                    })
            })
            .collect();
        Ok(NavigationResult {
            primary,
            related,
            index_revision: self.revision,
            index_hash: self.index_hash,
        })
    }

    pub fn resolve_diagnostic(
        &self,
        occurrence_id: u128,
        preferred_side: ArtifactSide,
    ) -> Result<NavigationResult, NavigationError> {
        let primary_identity = self
            .diagnostic_primary
            .get(&occurrence_id)
            .copied()
            .ok_or(NavigationError::UnknownDiagnostic(occurrence_id))?;
        let mut result = self.resolve(primary_identity, preferred_side)?;
        result.primary.relationship_kind = NavigationRelationshipKind::DiagnosticPrimary;
        if let Some(related) = self.diagnostic_related.get(&occurrence_id) {
            let existing = result
                .related
                .iter()
                .map(|anchor| (anchor.relationship_kind, anchor.identity))
                .collect::<BTreeSet<_>>();
            for (relationship_kind, identity) in related {
                if existing.contains(&(*relationship_kind, *identity)) {
                    continue;
                }
                if let Ok(mut anchor) = self.resolve(*identity, preferred_side) {
                    anchor.primary.relationship_kind = *relationship_kind;
                    result.related.push(anchor.primary);
                }
            }
            result
                .related
                .sort_by_key(|anchor| (anchor.relationship_kind, anchor.identity, anchor.side));
        }
        Ok(result)
    }

    fn apply_artifact_validity(&self, anchor: &mut NavigationAnchor) {
        if anchor.validity == NavigationValidity::Valid
            && anchor.side == ArtifactSide::Loaded
            && self
                .loaded_artifact
                .is_some_and(|loaded| loaded.fingerprint != self.offline_artifact_fingerprint)
        {
            anchor.validity = NavigationValidity::StaleArtifact;
        }
    }

    pub fn begin_update(&self, revision: u64) -> Result<NavigationIndexBuilder, NavigationError> {
        if revision <= self.revision {
            return Err(NavigationError::RevisionNotMonotonic);
        }
        Ok(NavigationIndexBuilder {
            revision,
            offline_artifact_fingerprint: self.offline_artifact_fingerprint,
            loaded_artifact: self.loaded_artifact,
            anchors: self.anchors.clone(),
            relationships: self.relationships.clone(),
            diagnostic_primary: self.diagnostic_primary.clone(),
            diagnostic_related: self.diagnostic_related.clone(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct NavigationIndexBuilder {
    revision: u64,
    offline_artifact_fingerprint: Hash32,
    loaded_artifact: Option<LoadedArtifactBinding>,
    anchors: BTreeMap<(SemanticIdentity, ArtifactSide), NavigationAnchor>,
    relationships:
        BTreeMap<SemanticIdentity, BTreeSet<(NavigationRelationshipKind, SemanticIdentity)>>,
    diagnostic_primary: BTreeMap<u128, SemanticIdentity>,
    diagnostic_related: BTreeMap<u128, BTreeSet<(NavigationRelationshipKind, SemanticIdentity)>>,
}

impl NavigationIndexBuilder {
    pub fn new(
        revision: u64,
        offline_artifact_fingerprint: Hash32,
        loaded_artifact: Option<LoadedArtifactBinding>,
    ) -> Result<Self, NavigationError> {
        if revision == 0 {
            return Err(NavigationError::RevisionNotMonotonic);
        }
        Ok(Self {
            revision,
            offline_artifact_fingerprint,
            loaded_artifact,
            anchors: BTreeMap::new(),
            relationships: BTreeMap::new(),
            diagnostic_primary: BTreeMap::new(),
            diagnostic_related: BTreeMap::new(),
        })
    }

    pub fn insert_anchor(&mut self, anchor: NavigationAnchor) -> Result<(), NavigationError> {
        let (expected_fingerprint, expected_epoch) = match anchor.side {
            ArtifactSide::CurrentOffline => (Some(self.offline_artifact_fingerprint), None),
            ArtifactSide::Loaded => self.loaded_artifact.map_or((None, None), |loaded| {
                (Some(loaded.fingerprint), Some(loaded.controller_epoch))
            }),
        };
        if expected_fingerprint != Some(anchor.artifact_fingerprint)
            || expected_epoch != anchor.controller_epoch
        {
            return Err(NavigationError::AnchorArtifactMismatch);
        }
        if anchor
            .source
            .as_ref()
            .is_some_and(|source| source.artifact_fingerprint != anchor.artifact_fingerprint)
        {
            return Err(NavigationError::SourceArtifactMismatch);
        }
        if anchor.source.is_none()
            && anchor.domain_projection.is_none()
            && anchor.kind != NavigationKind::Tombstone
        {
            return Err(NavigationError::LocationRequired);
        }
        if anchor.kind == NavigationKind::Tombstone {
            if anchor.tombstone_reason_hash.is_none() {
                return Err(NavigationError::TombstoneReasonRequired);
            }
            if anchor.validity != NavigationValidity::TargetRemoved {
                return Err(NavigationError::TombstoneValidityRequired);
            }
        } else if anchor.tombstone_reason_hash.is_some() {
            return Err(NavigationError::UnexpectedTombstoneReason);
        }
        let key = (anchor.identity, anchor.side);
        if self.anchors.insert(key, anchor).is_some() {
            return Err(NavigationError::DuplicateAnchor(key.0, key.1));
        }
        Ok(())
    }

    pub fn relate(
        &mut self,
        from: SemanticIdentity,
        to: SemanticIdentity,
    ) -> Result<(), NavigationError> {
        self.relate_kind(from, to, NavigationRelationshipKind::Use)
    }

    pub fn relate_kind(
        &mut self,
        from: SemanticIdentity,
        to: SemanticIdentity,
        kind: NavigationRelationshipKind,
    ) -> Result<(), NavigationError> {
        if from == to {
            return Err(NavigationError::SelfRelationship(from));
        }
        if matches!(
            kind,
            NavigationRelationshipKind::Selected
                | NavigationRelationshipKind::DiagnosticPrimary
                | NavigationRelationshipKind::DiagnosticReference
        ) {
            return Err(NavigationError::InvalidRelationshipKind(kind));
        }
        self.relationships
            .entry(from)
            .or_default()
            .insert((kind, to));
        self.relationships
            .entry(to)
            .or_default()
            .insert((kind, from));
        Ok(())
    }

    pub fn route_diagnostic(
        &mut self,
        occurrence_id: u128,
        primary: SemanticIdentity,
        related: Vec<SemanticIdentity>,
    ) -> Result<(), NavigationError> {
        self.route_diagnostic_with_roles(
            occurrence_id,
            primary,
            related
                .into_iter()
                .map(|identity| (NavigationRelationshipKind::DiagnosticReference, identity))
                .collect(),
        )
    }

    pub fn route_diagnostic_with_roles(
        &mut self,
        occurrence_id: u128,
        primary: SemanticIdentity,
        related: Vec<(NavigationRelationshipKind, SemanticIdentity)>,
    ) -> Result<(), NavigationError> {
        if related.iter().any(|(kind, _)| {
            matches!(
                kind,
                NavigationRelationshipKind::Selected
                    | NavigationRelationshipKind::DiagnosticPrimary
            )
        }) {
            return Err(NavigationError::InvalidDiagnosticRelationship);
        }
        if self.diagnostic_primary.contains_key(&occurrence_id) {
            return Err(NavigationError::DuplicateDiagnosticRoute(occurrence_id));
        }
        self.diagnostic_primary.insert(occurrence_id, primary);
        self.diagnostic_related
            .insert(occurrence_id, related.into_iter().collect());
        Ok(())
    }

    pub fn commit(self) -> Result<NavigationIndex, NavigationError> {
        let known = self
            .anchors
            .keys()
            .map(|(identity, _)| *identity)
            .collect::<BTreeSet<_>>();
        for (from, related) in &self.relationships {
            if !known.contains(from)
                || related
                    .iter()
                    .any(|(_, identity)| !known.contains(identity))
            {
                return Err(NavigationError::DanglingRelationship(*from));
            }
        }
        for (occurrence, primary) in &self.diagnostic_primary {
            if !known.contains(primary)
                || self
                    .diagnostic_related
                    .get(occurrence)
                    .is_some_and(|related| {
                        related
                            .iter()
                            .any(|(_, identity)| !known.contains(identity))
                    })
            {
                return Err(NavigationError::DanglingDiagnosticRoute(*occurrence));
            }
        }
        let mut index = NavigationIndex {
            revision: self.revision,
            offline_artifact_fingerprint: self.offline_artifact_fingerprint,
            loaded_artifact: self.loaded_artifact,
            anchors: self.anchors,
            relationships: self.relationships,
            diagnostic_primary: self.diagnostic_primary,
            diagnostic_related: self.diagnostic_related,
            index_hash: Hash32::ZERO,
        };
        index.index_hash = hash_index(&index);
        Ok(index)
    }
}

fn hash_index(index: &NavigationIndex) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-NAVIGATION-INDEX-1");
    hasher.u64(index.revision);
    hasher.hash(index.offline_artifact_fingerprint);
    match index.loaded_artifact {
        Some(loaded) => {
            hasher.bool(true);
            hasher.hash(loaded.fingerprint);
            hasher.u64(loaded.controller_epoch);
        }
        None => hasher.bool(false),
    }
    hasher.u64(index.anchors.len() as u64);
    for anchor in index.anchors.values() {
        hasher.u128(anchor.identity.0);
        hasher.u8(anchor.kind as u8);
        hasher.u8(anchor.side as u8);
        hasher.hash(anchor.artifact_fingerprint);
        match anchor.controller_epoch {
            Some(epoch) => {
                hasher.bool(true);
                hasher.u64(epoch);
            }
            None => hasher.bool(false),
        }
        match &anchor.source {
            Some(source) => {
                hasher.bool(true);
                encode_source_anchor(source, &mut hasher);
            }
            None => hasher.bool(false),
        }
        match anchor.domain_projection {
            Some(NavigationDomainProjection::ProjectObject { object_identity }) => {
                hasher.u8(1);
                hasher.u128(object_identity);
            }
            Some(NavigationDomainProjection::ProgramMember {
                owner_identity,
                member_identity,
            }) => {
                hasher.u8(2);
                hasher.u128(owner_identity);
                hasher.u128(member_identity);
            }
            Some(NavigationDomainProjection::HardwareObject { object_identity }) => {
                hasher.u8(3);
                hasher.u128(object_identity);
            }
            Some(NavigationDomainProjection::ProbeTarget { target }) => {
                hasher.u8(4);
                hasher.u128(target.0);
            }
            Some(NavigationDomainProjection::DiagnosticSubject { subject_identity }) => {
                hasher.u8(5);
                hasher.u128(subject_identity);
            }
            None => hasher.u8(0),
        }
        match anchor.probe_target {
            Some(target) => {
                hasher.bool(true);
                hasher.u128(target.0);
            }
            None => hasher.bool(false),
        }
        hasher.u8(anchor.relationship_kind as u8);
        hasher.u8(anchor.validity as u8);
        match anchor.tombstone_reason_hash {
            Some(hash) => {
                hasher.bool(true);
                hasher.hash(hash);
            }
            None => hasher.bool(false),
        }
    }
    hasher.u64(index.relationships.len() as u64);
    for (identity, related) in &index.relationships {
        hasher.u128(identity.0);
        hasher.u64(related.len() as u64);
        for (kind, related) in related {
            hasher.u8(*kind as u8);
            hasher.u128(related.0);
        }
    }
    hasher.u64(index.diagnostic_primary.len() as u64);
    for (occurrence, primary) in &index.diagnostic_primary {
        hasher.u128(*occurrence);
        hasher.u128(primary.0);
        let related = index.diagnostic_related.get(occurrence);
        hasher.u64(related.map_or(0, BTreeSet::len) as u64);
        if let Some(related) = related {
            for (kind, identity) in related {
                hasher.u8(*kind as u8);
                hasher.u128(identity.0);
            }
        }
    }
    hasher.finish()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NavigationError {
    RevisionNotMonotonic,
    AnchorArtifactMismatch,
    SourceArtifactMismatch,
    LocationRequired,
    TombstoneReasonRequired,
    TombstoneValidityRequired,
    UnexpectedTombstoneReason,
    DuplicateAnchor(SemanticIdentity, ArtifactSide),
    SelfRelationship(SemanticIdentity),
    InvalidRelationshipKind(NavigationRelationshipKind),
    DanglingRelationship(SemanticIdentity),
    DuplicateDiagnosticRoute(u128),
    InvalidDiagnosticRelationship,
    DanglingDiagnosticRoute(u128),
    UnknownIdentity(SemanticIdentity),
    UnknownDiagnostic(u128),
}

impl fmt::Display for NavigationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "semantic navigation rejected: {self:?}")
    }
}

impl Error for NavigationError {}
