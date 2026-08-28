use alloc::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    vec,
    vec::Vec,
};

use crate::{
    BlockId, BlockInterface, ControllerProgram, DependencyReason, InterfaceMemberId, ProgramIssue,
    ProgramUnitKind, validate_program,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InterfaceDelta {
    pub added: Vec<InterfaceMemberId>,
    pub removed: Vec<InterfaceMemberId>,
    pub changed: Vec<InterfaceMemberId>,
    pub public_signature_changed: bool,
    pub instance_layout_changed: bool,
    pub data_layout_changed: bool,
}

impl InterfaceDelta {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }

    #[must_use]
    pub fn changed_member_ids(&self) -> Vec<InterfaceMemberId> {
        let mut ids = BTreeSet::new();
        ids.extend(self.added.iter().copied());
        ids.extend(self.removed.iter().copied());
        ids.extend(self.changed.iter().copied());
        ids.into_iter().collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum InvalidationCode {
    OwnInterfaceChanged,
    CalledInterfaceChanged,
    DataLayoutChanged,
    InstanceLayoutChanged,
    TransitiveDependencyChanged,
}

/// Dependency path is ordered from the invalidated block to the changed block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidationExplanation {
    pub invalidated_block: BlockId,
    pub changed_block: BlockId,
    pub code: InvalidationCode,
    pub dependency_path: Vec<BlockId>,
    pub changed_member_ids: Vec<InterfaceMemberId>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InvalidationPlan {
    pub delta: InterfaceDelta,
    pub explanations: Vec<InvalidationExplanation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InvalidationError {
    MissingBlock(BlockId),
    ExistingProgramInvalid(Vec<ProgramIssue>),
    IllegalReplacement(Vec<ProgramIssue>),
}

impl ControllerProgram {
    /// Computes a deterministic dependency closure without applying the edit.
    /// The current aggregate must be valid. Caller binding failures caused by
    /// the proposed public signature are expected and are represented by the
    /// invalidation plan rather than treated as replacement-shape errors.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidationError::MissingBlock`] for an unknown identity,
    /// [`InvalidationError::ExistingProgramInvalid`] when the trusted baseline
    /// is already invalid, or [`InvalidationError::IllegalReplacement`] when
    /// the replacement interface itself violates program-model legality.
    pub fn explain_interface_change(
        &self,
        changed_block: BlockId,
        replacement: &BlockInterface,
    ) -> Result<InvalidationPlan, InvalidationError> {
        let current_report = validate_program(self);
        if !current_report.is_valid() {
            return Err(InvalidationError::ExistingProgramInvalid(
                current_report.issues,
            ));
        }
        let block = self
            .block(changed_block)
            .ok_or(InvalidationError::MissingBlock(changed_block))?;
        let delta = interface_delta(block.kind, &block.interface, replacement);
        if delta.is_empty() {
            return Ok(InvalidationPlan {
                delta,
                explanations: Vec::new(),
            });
        }

        let mut proposed = self.clone();
        proposed
            .block_mut(changed_block)
            .ok_or(InvalidationError::MissingBlock(changed_block))?
            .interface = replacement.clone();
        let proposed_report = validate_program(&proposed);
        let replacement_issues: Vec<_> = proposed_report
            .issues
            .into_iter()
            .filter(|issue| replacement_shape_issue(issue, changed_block))
            .collect();
        if !replacement_issues.is_empty() {
            return Err(InvalidationError::IllegalReplacement(replacement_issues));
        }

        let changed_member_ids = delta.changed_member_ids();
        let mut paths = BTreeMap::<BlockId, Vec<BlockId>>::new();
        let mut first_reasons = BTreeMap::<BlockId, DependencyReason>::new();
        let mut queue = VecDeque::new();
        paths.insert(changed_block, vec![changed_block]);
        queue.push_back(changed_block);

        while let Some(dependency) = queue.pop_front() {
            let dependency_path = paths[&dependency].clone();
            for edge in current_report.dependency_graph.dependents_of(dependency) {
                if dependency == changed_block && !initial_edge_applies(edge.reason, &delta) {
                    continue;
                }
                let mut candidate = Vec::with_capacity(dependency_path.len() + 1);
                candidate.push(edge.dependent);
                candidate.extend_from_slice(&dependency_path);
                let should_replace = paths.get(&edge.dependent).is_none_or(|existing| {
                    candidate.len() < existing.len()
                        || (candidate.len() == existing.len() && candidate < *existing)
                });
                if should_replace {
                    paths.insert(edge.dependent, candidate);
                    first_reasons.insert(edge.dependent, edge.reason);
                    queue.push_back(edge.dependent);
                }
            }
        }

        let explanations = paths
            .into_iter()
            .map(|(invalidated_block, dependency_path)| {
                let code = if invalidated_block == changed_block {
                    InvalidationCode::OwnInterfaceChanged
                } else if dependency_path.len() > 2 {
                    InvalidationCode::TransitiveDependencyChanged
                } else {
                    match first_reasons[&invalidated_block] {
                        DependencyReason::Call => InvalidationCode::CalledInterfaceChanged,
                        DependencyReason::DataUse => InvalidationCode::DataLayoutChanged,
                        DependencyReason::InstanceOf | DependencyReason::MultiInstanceState => {
                            InvalidationCode::InstanceLayoutChanged
                        }
                    }
                };
                InvalidationExplanation {
                    invalidated_block,
                    changed_block,
                    code,
                    dependency_path,
                    changed_member_ids: changed_member_ids.clone(),
                }
            })
            .collect();

        Ok(InvalidationPlan {
            delta,
            explanations,
        })
    }
}

fn interface_delta(
    kind: ProgramUnitKind,
    before: &BlockInterface,
    after: &BlockInterface,
) -> InterfaceDelta {
    let mut delta = InterfaceDelta::default();
    for (&id, prior) in &before.members {
        match after.members.get(&id) {
            None => {
                delta.removed.push(id);
                include_semantic_roles(kind, Some(prior), None, &mut delta);
            }
            Some(next) if next != prior => {
                delta.changed.push(id);
                include_semantic_roles(kind, Some(prior), Some(next), &mut delta);
            }
            Some(_) => {}
        }
    }
    for (&id, next) in &after.members {
        if !before.members.contains_key(&id) {
            delta.added.push(id);
            include_semantic_roles(kind, None, Some(next), &mut delta);
        }
    }
    if delta.added.is_empty()
        && delta.removed.is_empty()
        && before.ordered_member_ids != after.ordered_member_ids
    {
        for (&id, member) in before.members.iter().chain(&after.members) {
            if !delta.added.contains(&id)
                && !delta.removed.contains(&id)
                && !delta.changed.contains(&id)
            {
                delta.changed.push(id);
                include_semantic_roles(kind, Some(member), Some(member), &mut delta);
            }
        }
        delta.changed.sort_unstable();
        delta.changed.dedup();
    }
    delta
}

fn include_semantic_roles(
    kind: ProgramUnitKind,
    before: Option<&crate::InterfaceMember>,
    after: Option<&crate::InterfaceMember>,
    delta: &mut InterfaceDelta,
) {
    delta.public_signature_changed |= before
        .into_iter()
        .chain(after)
        .any(|member| member.role.is_public_signature());
    if kind == ProgramUnitKind::FunctionBlock {
        delta.instance_layout_changed |= before
            .into_iter()
            .chain(after)
            .any(|member| member.role.is_fb_instance_layout());
    }
    if kind == ProgramUnitKind::DataBlock(crate::DataBlockKind::Global) {
        delta.data_layout_changed |= before
            .into_iter()
            .chain(after)
            .any(|member| member.role == crate::InterfaceRole::Static);
    }
}

const fn initial_edge_applies(reason: DependencyReason, delta: &InterfaceDelta) -> bool {
    match reason {
        DependencyReason::Call => delta.public_signature_changed || delta.instance_layout_changed,
        DependencyReason::DataUse => delta.data_layout_changed,
        DependencyReason::InstanceOf | DependencyReason::MultiInstanceState => {
            delta.instance_layout_changed
        }
    }
}

fn replacement_shape_issue(issue: &ProgramIssue, changed_block: BlockId) -> bool {
    if issue.primary_block != Some(changed_block) && !issue.cycle.contains(&changed_block) {
        return false;
    }
    matches!(
        issue.code,
        crate::IssueCode::InterfaceLimitExceeded
            | crate::IssueCode::InterfaceKeyMismatch
            | crate::IssueCode::InterfaceOrderMismatch
            | crate::IssueCode::DuplicateMemberName
            | crate::IssueCode::DuplicateDeclaredOrder
            | crate::IssueCode::MultipleReturn
            | crate::IssueCode::RoleNotAllowed
            | crate::IssueCode::MemberMetadataIllegal
            | crate::IssueCode::MemberValueTypeMismatch
            | crate::IssueCode::InstanceTypeIllegal
            | crate::IssueCode::StateOwnershipCycle
    )
}
