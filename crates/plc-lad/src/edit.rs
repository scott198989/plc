use crate::{
    LadBranch, LadBranchId, LadDocument, LadEdgeId, LadNetwork, LadNetworkId, LadNode, LadNodeId,
    LadNodeKind, LadPowerEdge,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LadEdit {
    AddNetwork(LadNetwork),
    RemoveNetwork(LadNetworkId),
    MoveNetwork {
        network: LadNetworkId,
        new_index: usize,
    },
    AddNode {
        network: LadNetworkId,
        node: LadNode,
    },
    /// Keeps incident edges and branch metadata so malformed source remains
    /// visible and repairable instead of being silently rewritten.
    RemoveNodeKeepReferences {
        network: LadNetworkId,
        node: LadNodeId,
    },
    MoveNode {
        network: LadNetworkId,
        node: LadNodeId,
        new_index: usize,
    },
    ReplaceNodeKind {
        network: LadNetworkId,
        node: LadNodeId,
        kind: LadNodeKind,
    },
    AddPowerEdge {
        network: LadNetworkId,
        edge: LadPowerEdge,
    },
    RemovePowerEdgeKeepBranches {
        network: LadNetworkId,
        edge: LadEdgeId,
    },
    AddBranch {
        network: LadNetworkId,
        branch: LadBranch,
    },
    ReplaceBranch {
        network: LadNetworkId,
        branch: LadBranch,
    },
    RemoveBranchKeepNodes {
        network: LadNetworkId,
        branch: LadBranchId,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LadEditError {
    DuplicateNetwork(LadNetworkId),
    MissingNetwork(LadNetworkId),
    DuplicateNode(LadNodeId),
    MissingNode(LadNodeId),
    DuplicatePowerEdge(LadEdgeId),
    MissingPowerEdge(LadEdgeId),
    DuplicateBranch(LadBranchId),
    MissingBranch(LadBranchId),
    IndexOutOfBounds { requested: usize, length: usize },
    SemanticOrderOverflow,
    RevisionOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LadUndo {
    before: LadDocument,
}

impl LadUndo {
    /// Restores the exact pre-edit semantic document, including identities
    /// deleted by the batch. Layout is intentionally owned separately.
    pub fn restore(self, document: &mut LadDocument) {
        *document = self.before;
    }
}

/// Applies a semantic edit batch atomically and returns exact undo state.
///
/// # Errors
///
/// Returns the first deterministic identity/index/revision error. On failure,
/// `document` remains byte-for-byte equal to its pre-call value.
pub fn apply_lad_edits_atomically(
    document: &mut LadDocument,
    edits: &[LadEdit],
) -> Result<LadUndo, LadEditError> {
    let before = document.clone();
    if edits.is_empty() {
        return Ok(LadUndo { before });
    }
    let mut candidate = before.clone();
    for edit in edits {
        apply_one(&mut candidate, edit)?;
    }
    candidate.semantic_revision = before
        .semantic_revision
        .checked_add(1)
        .ok_or(LadEditError::RevisionOverflow)?;
    *document = candidate;
    Ok(LadUndo { before })
}

#[allow(clippy::too_many_lines)]
fn apply_one(document: &mut LadDocument, edit: &LadEdit) -> Result<(), LadEditError> {
    match edit {
        LadEdit::AddNetwork(network) => {
            if document.networks.contains_key(&network.id) {
                return Err(LadEditError::DuplicateNetwork(network.id));
            }
            let mut network = network.clone();
            network.semantic_order = order_value(document.ordered_network_ids.len())?;
            document.ordered_network_ids.push(network.id);
            document.networks.insert(network.id, network);
        }
        LadEdit::RemoveNetwork(network) => {
            if document.networks.remove(network).is_none() {
                return Err(LadEditError::MissingNetwork(*network));
            }
            document.ordered_network_ids.retain(|id| id != network);
            normalize_network_orders(document)?;
        }
        LadEdit::MoveNetwork { network, new_index } => {
            let old_index = document
                .ordered_network_ids
                .iter()
                .position(|id| id == network)
                .ok_or(LadEditError::MissingNetwork(*network))?;
            if *new_index >= document.ordered_network_ids.len() {
                return Err(LadEditError::IndexOutOfBounds {
                    requested: *new_index,
                    length: document.ordered_network_ids.len(),
                });
            }
            let id = document.ordered_network_ids.remove(old_index);
            document.ordered_network_ids.insert(*new_index, id);
            normalize_network_orders(document)?;
        }
        LadEdit::AddNode { network, node } => {
            let network = network_mut(document, *network)?;
            if network.nodes.contains_key(&node.id) {
                return Err(LadEditError::DuplicateNode(node.id));
            }
            let mut node = node.clone();
            node.semantic_order = order_value(network.ordered_node_ids.len())?;
            network.ordered_node_ids.push(node.id);
            network.nodes.insert(node.id, node);
        }
        LadEdit::RemoveNodeKeepReferences { network, node } => {
            let network = network_mut(document, *network)?;
            if network.nodes.remove(node).is_none() {
                return Err(LadEditError::MissingNode(*node));
            }
            network.ordered_node_ids.retain(|id| id != node);
            normalize_node_orders(network)?;
        }
        LadEdit::MoveNode {
            network,
            node,
            new_index,
        } => {
            let network = network_mut(document, *network)?;
            let old_index = network
                .ordered_node_ids
                .iter()
                .position(|id| id == node)
                .ok_or(LadEditError::MissingNode(*node))?;
            if *new_index >= network.ordered_node_ids.len() {
                return Err(LadEditError::IndexOutOfBounds {
                    requested: *new_index,
                    length: network.ordered_node_ids.len(),
                });
            }
            let id = network.ordered_node_ids.remove(old_index);
            network.ordered_node_ids.insert(*new_index, id);
            normalize_node_orders(network)?;
        }
        LadEdit::ReplaceNodeKind {
            network,
            node,
            kind,
        } => {
            let network = network_mut(document, *network)?;
            let node = network
                .nodes
                .get_mut(node)
                .ok_or(LadEditError::MissingNode(*node))?;
            node.kind = kind.clone();
        }
        LadEdit::AddPowerEdge { network, edge } => {
            let network = network_mut(document, *network)?;
            if network.power_edges.contains_key(&edge.id) {
                return Err(LadEditError::DuplicatePowerEdge(edge.id));
            }
            network.power_edges.insert(edge.id, edge.clone());
        }
        LadEdit::RemovePowerEdgeKeepBranches { network, edge } => {
            let network = network_mut(document, *network)?;
            if network.power_edges.remove(edge).is_none() {
                return Err(LadEditError::MissingPowerEdge(*edge));
            }
        }
        LadEdit::AddBranch { network, branch } => {
            let network = network_mut(document, *network)?;
            if network.branches.contains_key(&branch.id) {
                return Err(LadEditError::DuplicateBranch(branch.id));
            }
            network.ordered_branch_ids.push(branch.id);
            network.branches.insert(branch.id, branch.clone());
        }
        LadEdit::ReplaceBranch { network, branch } => {
            let network = network_mut(document, *network)?;
            if !network.branches.contains_key(&branch.id) {
                return Err(LadEditError::MissingBranch(branch.id));
            }
            network.branches.insert(branch.id, branch.clone());
        }
        LadEdit::RemoveBranchKeepNodes { network, branch } => {
            let network = network_mut(document, *network)?;
            if network.branches.remove(branch).is_none() {
                return Err(LadEditError::MissingBranch(*branch));
            }
            network.ordered_branch_ids.retain(|id| id != branch);
        }
    }
    Ok(())
}

fn network_mut(
    document: &mut LadDocument,
    id: LadNetworkId,
) -> Result<&mut LadNetwork, LadEditError> {
    document
        .networks
        .get_mut(&id)
        .ok_or(LadEditError::MissingNetwork(id))
}

fn normalize_network_orders(document: &mut LadDocument) -> Result<(), LadEditError> {
    for (index, id) in document.ordered_network_ids.iter().enumerate() {
        let network = document
            .networks
            .get_mut(id)
            .ok_or(LadEditError::MissingNetwork(*id))?;
        network.semantic_order = order_value(index)?;
    }
    Ok(())
}

fn normalize_node_orders(network: &mut LadNetwork) -> Result<(), LadEditError> {
    for (index, id) in network.ordered_node_ids.iter().enumerate() {
        let node = network
            .nodes
            .get_mut(id)
            .ok_or(LadEditError::MissingNode(*id))?;
        node.semantic_order = order_value(index)?;
    }
    Ok(())
}

fn order_value(index: usize) -> Result<u32, LadEditError> {
    u32::try_from(index).map_err(|_| LadEditError::SemanticOrderOverflow)
}
