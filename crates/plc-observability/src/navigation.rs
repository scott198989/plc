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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationAnchor {
    pub identity: SemanticIdentity,
    pub kind: NavigationKind,
    pub side: ArtifactSide,
    pub artifact_fingerprint: Hash32,
    pub source: Option<SourceAnchor>,
    pub probe_target: Option<StableTargetId>,
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
    loaded_artifact_fingerprint: Option<Hash32>,
    anchors: BTreeMap<(SemanticIdentity, ArtifactSide), NavigationAnchor>,
    relationships: BTreeMap<SemanticIdentity, BTreeSet<SemanticIdentity>>,
    diagnostic_primary: BTreeMap<u128, SemanticIdentity>,
    diagnostic_related: BTreeMap<u128, BTreeSet<SemanticIdentity>>,
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
        let primary = self
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
        let related = self
            .relationships
            .get(&identity)
            .into_iter()
            .flat_map(|identities| identities.iter())
            .filter_map(|related| {
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
        if let Some(related) = self.diagnostic_related.get(&occurrence_id) {
            let existing = result
                .related
                .iter()
                .map(|anchor| anchor.identity)
                .collect::<BTreeSet<_>>();
            for identity in related {
                if existing.contains(identity) {
                    continue;
                }
                if let Ok(anchor) = self.resolve(*identity, preferred_side) {
                    result.related.push(anchor.primary);
                }
            }
            result
                .related
                .sort_by_key(|anchor| (anchor.identity, anchor.side));
        }
        Ok(result)
    }

    pub fn begin_update(&self, revision: u64) -> Result<NavigationIndexBuilder, NavigationError> {
        if revision <= self.revision {
            return Err(NavigationError::RevisionNotMonotonic);
        }
        Ok(NavigationIndexBuilder {
            revision,
            offline_artifact_fingerprint: self.offline_artifact_fingerprint,
            loaded_artifact_fingerprint: self.loaded_artifact_fingerprint,
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
    loaded_artifact_fingerprint: Option<Hash32>,
    anchors: BTreeMap<(SemanticIdentity, ArtifactSide), NavigationAnchor>,
    relationships: BTreeMap<SemanticIdentity, BTreeSet<SemanticIdentity>>,
    diagnostic_primary: BTreeMap<u128, SemanticIdentity>,
    diagnostic_related: BTreeMap<u128, BTreeSet<SemanticIdentity>>,
}

impl NavigationIndexBuilder {
    pub fn new(
        revision: u64,
        offline_artifact_fingerprint: Hash32,
        loaded_artifact_fingerprint: Option<Hash32>,
    ) -> Result<Self, NavigationError> {
        if revision == 0 {
            return Err(NavigationError::RevisionNotMonotonic);
        }
        Ok(Self {
            revision,
            offline_artifact_fingerprint,
            loaded_artifact_fingerprint,
            anchors: BTreeMap::new(),
            relationships: BTreeMap::new(),
            diagnostic_primary: BTreeMap::new(),
            diagnostic_related: BTreeMap::new(),
        })
    }

    pub fn insert_anchor(&mut self, anchor: NavigationAnchor) -> Result<(), NavigationError> {
        let expected_fingerprint = match anchor.side {
            ArtifactSide::CurrentOffline => Some(self.offline_artifact_fingerprint),
            ArtifactSide::Loaded => self.loaded_artifact_fingerprint,
        };
        if expected_fingerprint != Some(anchor.artifact_fingerprint) {
            return Err(NavigationError::AnchorArtifactMismatch);
        }
        if anchor.kind == NavigationKind::Tombstone && anchor.tombstone_reason_hash.is_none() {
            return Err(NavigationError::TombstoneReasonRequired);
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
        if from == to {
            return Err(NavigationError::SelfRelationship(from));
        }
        self.relationships.entry(from).or_default().insert(to);
        self.relationships.entry(to).or_default().insert(from);
        Ok(())
    }

    pub fn route_diagnostic(
        &mut self,
        occurrence_id: u128,
        primary: SemanticIdentity,
        related: Vec<SemanticIdentity>,
    ) -> Result<(), NavigationError> {
        if self
            .diagnostic_primary
            .insert(occurrence_id, primary)
            .is_some()
        {
            return Err(NavigationError::DuplicateDiagnosticRoute(occurrence_id));
        }
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
            if !known.contains(from) || related.iter().any(|identity| !known.contains(identity)) {
                return Err(NavigationError::DanglingRelationship(*from));
            }
        }
        for (occurrence, primary) in &self.diagnostic_primary {
            if !known.contains(primary)
                || self
                    .diagnostic_related
                    .get(occurrence)
                    .is_some_and(|related| related.iter().any(|identity| !known.contains(identity)))
            {
                return Err(NavigationError::DanglingDiagnosticRoute(*occurrence));
            }
        }
        let mut index = NavigationIndex {
            revision: self.revision,
            offline_artifact_fingerprint: self.offline_artifact_fingerprint,
            loaded_artifact_fingerprint: self.loaded_artifact_fingerprint,
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
    match index.loaded_artifact_fingerprint {
        Some(hash) => {
            hasher.bool(true);
            hasher.hash(hash);
        }
        None => hasher.bool(false),
    }
    hasher.u64(index.anchors.len() as u64);
    for anchor in index.anchors.values() {
        hasher.u128(anchor.identity.0);
        hasher.u8(anchor.kind as u8);
        hasher.u8(anchor.side as u8);
        hasher.hash(anchor.artifact_fingerprint);
        match &anchor.source {
            Some(source) => {
                hasher.bool(true);
                encode_source_anchor(source, &mut hasher);
            }
            None => hasher.bool(false),
        }
        match anchor.probe_target {
            Some(target) => {
                hasher.bool(true);
                hasher.u128(target.0);
            }
            None => hasher.bool(false),
        }
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
        for related in related {
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
            for identity in related {
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
    TombstoneReasonRequired,
    DuplicateAnchor(SemanticIdentity, ArtifactSide),
    SelfRelationship(SemanticIdentity),
    DanglingRelationship(SemanticIdentity),
    DuplicateDiagnosticRoute(u128),
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
