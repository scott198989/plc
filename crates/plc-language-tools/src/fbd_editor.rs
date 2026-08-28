use crate::{
    ConnectionId, FbdConnection, FbdDocument, FbdNetwork, FbdNode, NetworkId, NodeId, NodeKind,
    PortId,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FbdEdit {
    AddNetwork(FbdNetwork),
    RemoveNetwork(NetworkId),
    MoveNetwork {
        network: NetworkId,
        new_index: usize,
    },
    AddNode {
        network: NetworkId,
        node: FbdNode,
    },
    RemoveNodeKeepConnections {
        network: NetworkId,
        node: NodeId,
    },
    MoveNode {
        network: NetworkId,
        node: NodeId,
        new_index: usize,
    },
    ReplaceNodeKind {
        network: NetworkId,
        node: NodeId,
        kind: NodeKind,
    },
    RemovePortKeepConnections {
        network: NetworkId,
        node: NodeId,
        port: PortId,
    },
    AddConnection {
        network: NetworkId,
        connection: FbdConnection,
    },
    RemoveConnection {
        network: NetworkId,
        connection: ConnectionId,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FbdEditError {
    DuplicateNetwork(NetworkId),
    MissingNetwork(NetworkId),
    DuplicateNode(NodeId),
    MissingNode(NodeId),
    MissingPort(PortId),
    DuplicateConnection(ConnectionId),
    MissingConnection(ConnectionId),
    IndexOutOfBounds { requested: usize, length: usize },
    SemanticOrderOverflow,
}

/// Applies an edit batch atomically. Structural deletion deliberately keeps
/// dangling connections so the editor can display and repair invalid graphs;
/// validation, never mutation, decides whether lowering is allowed.
pub fn apply_fbd_edits_atomically(
    document: &mut FbdDocument,
    edits: &[FbdEdit],
) -> Result<(), FbdEditError> {
    let mut candidate = document.clone();
    for edit in edits {
        apply_one(&mut candidate, edit)?;
    }
    *document = candidate;
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn apply_one(document: &mut FbdDocument, edit: &FbdEdit) -> Result<(), FbdEditError> {
    match edit {
        FbdEdit::AddNetwork(network) => {
            if document.networks.contains_key(&network.id) {
                return Err(FbdEditError::DuplicateNetwork(network.id));
            }
            let mut network = network.clone();
            network.semantic_order = order_value(document.ordered_network_ids.len())?;
            document.ordered_network_ids.push(network.id);
            document.networks.insert(network.id, network);
        }
        FbdEdit::RemoveNetwork(network) => {
            if document.networks.remove(network).is_none() {
                return Err(FbdEditError::MissingNetwork(*network));
            }
            document.ordered_network_ids.retain(|id| id != network);
            normalize_network_orders(document)?;
        }
        FbdEdit::MoveNetwork { network, new_index } => {
            let Some(old_index) = document
                .ordered_network_ids
                .iter()
                .position(|id| id == network)
            else {
                return Err(FbdEditError::MissingNetwork(*network));
            };
            if *new_index >= document.ordered_network_ids.len() {
                return Err(FbdEditError::IndexOutOfBounds {
                    requested: *new_index,
                    length: document.ordered_network_ids.len(),
                });
            }
            let id = document.ordered_network_ids.remove(old_index);
            document.ordered_network_ids.insert(*new_index, id);
            normalize_network_orders(document)?;
        }
        FbdEdit::AddNode { network, node } => {
            let network = network_mut(document, *network)?;
            if network.nodes.contains_key(&node.id) {
                return Err(FbdEditError::DuplicateNode(node.id));
            }
            let mut node = node.clone();
            node.semantic_order = order_value(network.ordered_node_ids.len())?;
            network.ordered_node_ids.push(node.id);
            network.nodes.insert(node.id, node);
        }
        FbdEdit::RemoveNodeKeepConnections { network, node } => {
            let network = network_mut(document, *network)?;
            if network.nodes.remove(node).is_none() {
                return Err(FbdEditError::MissingNode(*node));
            }
            network.ordered_node_ids.retain(|id| id != node);
            normalize_node_orders(network)?;
        }
        FbdEdit::MoveNode {
            network,
            node,
            new_index,
        } => {
            let network = network_mut(document, *network)?;
            let Some(old_index) = network.ordered_node_ids.iter().position(|id| id == node) else {
                return Err(FbdEditError::MissingNode(*node));
            };
            if *new_index >= network.ordered_node_ids.len() {
                return Err(FbdEditError::IndexOutOfBounds {
                    requested: *new_index,
                    length: network.ordered_node_ids.len(),
                });
            }
            let id = network.ordered_node_ids.remove(old_index);
            network.ordered_node_ids.insert(*new_index, id);
            normalize_node_orders(network)?;
        }
        FbdEdit::ReplaceNodeKind {
            network,
            node,
            kind,
        } => {
            network_mut(document, *network)?
                .nodes
                .get_mut(node)
                .ok_or(FbdEditError::MissingNode(*node))?
                .kind = kind.clone();
        }
        FbdEdit::RemovePortKeepConnections {
            network,
            node,
            port,
        } => {
            let node = network_mut(document, *network)?
                .nodes
                .get_mut(node)
                .ok_or(FbdEditError::MissingNode(*node))?;
            if node.ports.remove(port).is_none() {
                return Err(FbdEditError::MissingPort(*port));
            }
            node.ordered_port_ids.retain(|id| id != port);
        }
        FbdEdit::AddConnection {
            network,
            connection,
        } => {
            let network = network_mut(document, *network)?;
            if network.connections.contains_key(&connection.id) {
                return Err(FbdEditError::DuplicateConnection(connection.id));
            }
            network
                .connections
                .insert(connection.id, connection.clone());
        }
        FbdEdit::RemoveConnection {
            network,
            connection,
        } => {
            if network_mut(document, *network)?
                .connections
                .remove(connection)
                .is_none()
            {
                return Err(FbdEditError::MissingConnection(*connection));
            }
        }
    }
    Ok(())
}

fn network_mut(document: &mut FbdDocument, id: NetworkId) -> Result<&mut FbdNetwork, FbdEditError> {
    document
        .networks
        .get_mut(&id)
        .ok_or(FbdEditError::MissingNetwork(id))
}

fn normalize_network_orders(document: &mut FbdDocument) -> Result<(), FbdEditError> {
    for (index, id) in document.ordered_network_ids.iter().enumerate() {
        document
            .networks
            .get_mut(id)
            .ok_or(FbdEditError::MissingNetwork(*id))?
            .semantic_order = order_value(index)?;
    }
    Ok(())
}

fn normalize_node_orders(network: &mut FbdNetwork) -> Result<(), FbdEditError> {
    for (index, id) in network.ordered_node_ids.iter().enumerate() {
        network
            .nodes
            .get_mut(id)
            .ok_or(FbdEditError::MissingNode(*id))?
            .semantic_order = order_value(index)?;
    }
    Ok(())
}

fn order_value(index: usize) -> Result<u32, FbdEditError> {
    u32::try_from(index).map_err(|_| FbdEditError::SemanticOrderOverflow)
}
