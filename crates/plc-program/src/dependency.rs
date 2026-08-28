use alloc::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    vec,
    vec::Vec,
};

use crate::{BlockId, CallSiteId, InstructionUseId, InterfaceMemberId};

/// A stable identity in the semantic dependency graph. The variants keep
/// object identities and finer-grained semantic-unit identities distinct even
/// when their caller-supplied UUID payloads happen to be numerically equal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DependencyNodeId {
    Object(BlockId),
    InterfaceMember {
        owner: BlockId,
        member: InterfaceMemberId,
    },
    CallSite {
        owner: BlockId,
        call_site: CallSiteId,
    },
    InstructionUse {
        owner: BlockId,
        instruction_use: InstructionUseId,
    },
    /// A caller-supplied UUID for semantic units owned outside the canonical
    /// PLC program aggregate (profile, hardware, network, watch, or trace).
    ExternalSemanticUnit(u128),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DependencyEdgeKind {
    Declaration,
    TypeUse,
    MemberUse,
    ValueRead,
    ValueWrite,
    Call,
    Instance,
    Interface,
    StorageLayout,
    InstructionCapability,
    ProfileCapability,
    Address,
    HardwareChannel,
    NetworkAssignment,
    FutureHmiBinding,
    FutureLibraryVersion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DependencyOccurrenceId {
    InterfaceMember(InterfaceMemberId),
    CallSite(CallSiteId),
    InstructionUse(InstructionUseId),
    External(u128),
}

/// A navigable project/source location attached to a usage edge. UTF-8 ranges
/// are half-open and remain optional because structural project records do not
/// always originate in textual source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DependencyLocation {
    ProjectObject(BlockId),
    SourceOccurrence {
        owner: BlockId,
        occurrence: DependencyOccurrenceId,
        utf8_start: Option<u32>,
        utf8_end: Option<u32>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DependencyResolution {
    Resolved,
    Unresolved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticDependencyEdge {
    pub dependent: DependencyNodeId,
    pub dependency: DependencyNodeId,
    pub kind: DependencyEdgeKind,
    pub location: Option<DependencyLocation>,
    pub resolution: DependencyResolution,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DependencyGraphError {
    MissingDependent(DependencyNodeId),
    MissingResolvedDependency(DependencyNodeId),
    InvalidSourceRange { start: u32, end: u32 },
    UnknownNode(DependencyNodeId),
    AlreadyDeleted(DependencyNodeId),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SemanticDependencyGraph {
    nodes: BTreeSet<DependencyNodeId>,
    deleted_nodes: BTreeSet<DependencyNodeId>,
    edges: BTreeSet<SemanticDependencyEdge>,
}

impl SemanticDependencyGraph {
    pub fn insert_node(&mut self, node: DependencyNodeId) {
        self.nodes.insert(node);
        self.deleted_nodes.remove(&node);
    }

    #[must_use]
    pub fn nodes(&self) -> &BTreeSet<DependencyNodeId> {
        &self.nodes
    }

    #[must_use]
    pub fn deleted_nodes(&self) -> &BTreeSet<DependencyNodeId> {
        &self.deleted_nodes
    }

    #[must_use]
    pub fn edges(&self) -> &BTreeSet<SemanticDependencyEdge> {
        &self.edges
    }

    /// Inserts a typed usage edge after validating its identity and location.
    /// An unresolved target is deliberately permitted without a target node so
    /// deletion and missing-reference usages remain present and diagnosable.
    ///
    /// # Errors
    ///
    /// Returns a missing-node or malformed UTF-8 range defect.
    pub fn insert_edge(
        &mut self,
        edge: SemanticDependencyEdge,
    ) -> Result<(), DependencyGraphError> {
        if !self.nodes.contains(&edge.dependent) || self.deleted_nodes.contains(&edge.dependent) {
            return Err(DependencyGraphError::MissingDependent(edge.dependent));
        }
        if edge.resolution == DependencyResolution::Resolved
            && (!self.nodes.contains(&edge.dependency)
                || self.deleted_nodes.contains(&edge.dependency))
        {
            return Err(DependencyGraphError::MissingResolvedDependency(
                edge.dependency,
            ));
        }
        if let Some(DependencyLocation::SourceOccurrence {
            utf8_start: Some(start),
            utf8_end: Some(end),
            ..
        }) = edge.location
            && start > end
        {
            return Err(DependencyGraphError::InvalidSourceRange { start, end });
        }
        self.edges.insert(edge);
        Ok(())
    }

    /// Converts every usage of a deleted identity into an unresolved edge.
    /// Usages are retained verbatim, including their source/project locations.
    ///
    /// # Errors
    ///
    /// Returns an unknown-node or duplicate-deletion defect.
    pub fn mark_deleted(
        &mut self,
        node: DependencyNodeId,
    ) -> Result<Vec<SemanticDependencyEdge>, DependencyGraphError> {
        if !self.nodes.contains(&node) {
            return Err(DependencyGraphError::UnknownNode(node));
        }
        if !self.deleted_nodes.insert(node) {
            return Err(DependencyGraphError::AlreadyDeleted(node));
        }
        let mut unresolved = Vec::new();
        self.edges = self
            .edges
            .iter()
            .copied()
            .map(|mut edge| {
                if edge.dependency == node {
                    edge.resolution = DependencyResolution::Unresolved;
                    unresolved.push(edge);
                }
                edge
            })
            .collect();
        Ok(unresolved)
    }

    pub fn dependencies_of(
        &self,
        node: DependencyNodeId,
    ) -> impl Iterator<Item = &SemanticDependencyEdge> {
        self.edges.iter().filter(move |edge| edge.dependent == node)
    }

    pub fn dependents_of(
        &self,
        node: DependencyNodeId,
    ) -> impl Iterator<Item = &SemanticDependencyEdge> {
        self.edges
            .iter()
            .filter(move |edge| edge.dependency == node)
    }

    #[must_use]
    pub fn unresolved_edges(&self) -> Vec<SemanticDependencyEdge> {
        self.edges
            .iter()
            .copied()
            .filter(|edge| edge.resolution == DependencyResolution::Unresolved)
            .collect()
    }

    /// Computes the deterministic, shortest-path invalidation closure for one
    /// semantic change. Rename and presentation edits are represented as
    /// index-only effects and never retarget UUID-based semantic edges.
    #[must_use]
    pub fn explain_change(&self, change: SemanticChange) -> SemanticInvalidationPlan {
        let (root_effect, root_reason) = root_effect(change.kind);
        let mut paths = BTreeMap::from([(change.node, vec![change.node])]);
        let mut first_kind = BTreeMap::<DependencyNodeId, DependencyEdgeKind>::new();
        let mut queue = VecDeque::new();
        if root_effect == InvalidationEffect::EditedUnitSemantic
            || root_effect == InvalidationEffect::UnresolvedReference
        {
            for edge in self.dependents_of(change.node) {
                if edge_affected(edge.kind, change.kind) {
                    let dependent = edge.dependent;
                    paths.insert(dependent, vec![dependent, change.node]);
                    first_kind.insert(dependent, edge.kind);
                    queue.push_back(dependent);
                }
            }
        }
        while let Some(dependency) = queue.pop_front() {
            let dependency_path = paths[&dependency].clone();
            for edge in self.dependents_of(dependency) {
                let mut candidate = Vec::with_capacity(dependency_path.len() + 1);
                candidate.push(edge.dependent);
                candidate.extend_from_slice(&dependency_path);
                let should_replace = paths.get(&edge.dependent).is_none_or(|existing| {
                    candidate.len() < existing.len()
                        || (candidate.len() == existing.len() && candidate < *existing)
                });
                if should_replace {
                    paths.insert(edge.dependent, candidate);
                    first_kind.insert(edge.dependent, edge.kind);
                    queue.push_back(edge.dependent);
                }
            }
        }
        let invalidations = paths
            .into_iter()
            .map(|(node, dependency_path)| {
                let (effect, reason) = if node == change.node {
                    (root_effect, root_reason)
                } else {
                    (
                        if change.kind == SemanticChangeKind::Deletion {
                            InvalidationEffect::UnresolvedReference
                        } else {
                            InvalidationEffect::DependentSemantic
                        },
                        reason_for_edge(first_kind[&node]),
                    )
                };
                SemanticInvalidation {
                    node,
                    effect,
                    reason,
                    dependency_path,
                }
            })
            .collect();
        SemanticInvalidationPlan {
            change,
            invalidations,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SemanticChangeKind {
    Body,
    ConstantValue,
    PublicName,
    TypeShape,
    PublicInterface,
    CallTarget,
    StorageLayout,
    InstructionRegistry,
    TrainingProfile,
    GlobalIrContract,
    AddressContract,
    HardwareChannel,
    NetworkAssignment,
    SchedulingDeclaration,
    PresentationOnly,
    Deletion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SemanticChange {
    pub node: DependencyNodeId,
    pub kind: SemanticChangeKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InvalidationEffect {
    EditedUnitSemantic,
    DependentSemantic,
    SourceIndexOnly,
    UnresolvedReference,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InvalidationReason {
    EditedBody,
    ConstantValueChanged,
    StableIdentityRenamed,
    TypeShapeChanged,
    PublicInterfaceChanged,
    CallTargetChanged,
    StorageLayoutChanged,
    InstructionRegistryChanged,
    TrainingProfileChanged,
    GlobalIrContractChanged,
    AddressContractChanged,
    HardwareChannelChanged,
    NetworkAssignmentChanged,
    SchedulingDeclarationChanged,
    PresentationChanged,
    DependencyDeleted,
    DeclarationDependency,
    TypeDependency,
    MemberDependency,
    ValueReadDependency,
    ValueWriteDependency,
    CallDependency,
    InstanceDependency,
    InterfaceDependency,
    StorageDependency,
    InstructionCapabilityDependency,
    ProfileCapabilityDependency,
    AddressDependency,
    HardwareChannelDependency,
    NetworkAssignmentDependency,
    FutureHmiDependency,
    FutureLibraryDependency,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticInvalidation {
    pub node: DependencyNodeId,
    pub effect: InvalidationEffect,
    pub reason: InvalidationReason,
    /// Ordered from the invalidated node to the changed dependency.
    pub dependency_path: Vec<DependencyNodeId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticInvalidationPlan {
    pub change: SemanticChange,
    pub invalidations: Vec<SemanticInvalidation>,
}

impl SemanticInvalidationPlan {
    #[must_use]
    pub fn semantic_nodes(&self) -> BTreeSet<DependencyNodeId> {
        self.invalidations
            .iter()
            .filter(|item| item.effect != InvalidationEffect::SourceIndexOnly)
            .map(|item| item.node)
            .collect()
    }
}

const fn root_effect(change: SemanticChangeKind) -> (InvalidationEffect, InvalidationReason) {
    match change {
        SemanticChangeKind::Body => (
            InvalidationEffect::EditedUnitSemantic,
            InvalidationReason::EditedBody,
        ),
        SemanticChangeKind::ConstantValue => (
            InvalidationEffect::EditedUnitSemantic,
            InvalidationReason::ConstantValueChanged,
        ),
        SemanticChangeKind::PublicName => (
            InvalidationEffect::SourceIndexOnly,
            InvalidationReason::StableIdentityRenamed,
        ),
        SemanticChangeKind::TypeShape => (
            InvalidationEffect::EditedUnitSemantic,
            InvalidationReason::TypeShapeChanged,
        ),
        SemanticChangeKind::PublicInterface => (
            InvalidationEffect::EditedUnitSemantic,
            InvalidationReason::PublicInterfaceChanged,
        ),
        SemanticChangeKind::CallTarget => (
            InvalidationEffect::EditedUnitSemantic,
            InvalidationReason::CallTargetChanged,
        ),
        SemanticChangeKind::StorageLayout => (
            InvalidationEffect::EditedUnitSemantic,
            InvalidationReason::StorageLayoutChanged,
        ),
        SemanticChangeKind::InstructionRegistry => (
            InvalidationEffect::EditedUnitSemantic,
            InvalidationReason::InstructionRegistryChanged,
        ),
        SemanticChangeKind::TrainingProfile => (
            InvalidationEffect::EditedUnitSemantic,
            InvalidationReason::TrainingProfileChanged,
        ),
        SemanticChangeKind::GlobalIrContract => (
            InvalidationEffect::EditedUnitSemantic,
            InvalidationReason::GlobalIrContractChanged,
        ),
        SemanticChangeKind::AddressContract => (
            InvalidationEffect::EditedUnitSemantic,
            InvalidationReason::AddressContractChanged,
        ),
        SemanticChangeKind::HardwareChannel => (
            InvalidationEffect::EditedUnitSemantic,
            InvalidationReason::HardwareChannelChanged,
        ),
        SemanticChangeKind::NetworkAssignment => (
            InvalidationEffect::EditedUnitSemantic,
            InvalidationReason::NetworkAssignmentChanged,
        ),
        SemanticChangeKind::SchedulingDeclaration => (
            InvalidationEffect::EditedUnitSemantic,
            InvalidationReason::SchedulingDeclarationChanged,
        ),
        SemanticChangeKind::PresentationOnly => (
            InvalidationEffect::SourceIndexOnly,
            InvalidationReason::PresentationChanged,
        ),
        SemanticChangeKind::Deletion => (
            InvalidationEffect::UnresolvedReference,
            InvalidationReason::DependencyDeleted,
        ),
    }
}

const fn edge_affected(edge: DependencyEdgeKind, change: SemanticChangeKind) -> bool {
    match change {
        SemanticChangeKind::Body
        | SemanticChangeKind::PublicName
        | SemanticChangeKind::CallTarget
        | SemanticChangeKind::PresentationOnly => false,
        SemanticChangeKind::ConstantValue => matches!(
            edge,
            DependencyEdgeKind::ValueRead | DependencyEdgeKind::MemberUse
        ),
        SemanticChangeKind::TypeShape => matches!(
            edge,
            DependencyEdgeKind::TypeUse
                | DependencyEdgeKind::MemberUse
                | DependencyEdgeKind::Interface
                | DependencyEdgeKind::StorageLayout
                | DependencyEdgeKind::Instance
                | DependencyEdgeKind::FutureHmiBinding
                | DependencyEdgeKind::FutureLibraryVersion
        ),
        SemanticChangeKind::PublicInterface => matches!(
            edge,
            DependencyEdgeKind::Call
                | DependencyEdgeKind::Instance
                | DependencyEdgeKind::Interface
                | DependencyEdgeKind::MemberUse
        ),
        SemanticChangeKind::StorageLayout => matches!(
            edge,
            DependencyEdgeKind::MemberUse
                | DependencyEdgeKind::ValueRead
                | DependencyEdgeKind::ValueWrite
                | DependencyEdgeKind::Instance
                | DependencyEdgeKind::StorageLayout
                | DependencyEdgeKind::Address
                | DependencyEdgeKind::FutureHmiBinding
        ),
        SemanticChangeKind::InstructionRegistry => {
            matches!(edge, DependencyEdgeKind::InstructionCapability)
        }
        SemanticChangeKind::TrainingProfile => matches!(
            edge,
            DependencyEdgeKind::ProfileCapability | DependencyEdgeKind::InstructionCapability
        ),
        SemanticChangeKind::GlobalIrContract | SemanticChangeKind::Deletion => true,
        SemanticChangeKind::AddressContract => matches!(
            edge,
            DependencyEdgeKind::Address
                | DependencyEdgeKind::HardwareChannel
                | DependencyEdgeKind::FutureHmiBinding
        ),
        SemanticChangeKind::HardwareChannel => {
            matches!(edge, DependencyEdgeKind::HardwareChannel)
        }
        SemanticChangeKind::NetworkAssignment => {
            matches!(edge, DependencyEdgeKind::NetworkAssignment)
        }
        SemanticChangeKind::SchedulingDeclaration => {
            matches!(edge, DependencyEdgeKind::Declaration)
        }
    }
}

const fn reason_for_edge(edge: DependencyEdgeKind) -> InvalidationReason {
    match edge {
        DependencyEdgeKind::Declaration => InvalidationReason::DeclarationDependency,
        DependencyEdgeKind::TypeUse => InvalidationReason::TypeDependency,
        DependencyEdgeKind::MemberUse => InvalidationReason::MemberDependency,
        DependencyEdgeKind::ValueRead => InvalidationReason::ValueReadDependency,
        DependencyEdgeKind::ValueWrite => InvalidationReason::ValueWriteDependency,
        DependencyEdgeKind::Call => InvalidationReason::CallDependency,
        DependencyEdgeKind::Instance => InvalidationReason::InstanceDependency,
        DependencyEdgeKind::Interface => InvalidationReason::InterfaceDependency,
        DependencyEdgeKind::StorageLayout => InvalidationReason::StorageDependency,
        DependencyEdgeKind::InstructionCapability => {
            InvalidationReason::InstructionCapabilityDependency
        }
        DependencyEdgeKind::ProfileCapability => InvalidationReason::ProfileCapabilityDependency,
        DependencyEdgeKind::Address => InvalidationReason::AddressDependency,
        DependencyEdgeKind::HardwareChannel => InvalidationReason::HardwareChannelDependency,
        DependencyEdgeKind::NetworkAssignment => InvalidationReason::NetworkAssignmentDependency,
        DependencyEdgeKind::FutureHmiBinding => InvalidationReason::FutureHmiDependency,
        DependencyEdgeKind::FutureLibraryVersion => InvalidationReason::FutureLibraryDependency,
    }
}
