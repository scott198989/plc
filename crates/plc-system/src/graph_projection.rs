//! Canonical, coordinate-free LAD/FBD payload decoding.
//!
//! A program-block payload with `language = "LAD" | "FBD"` stores a `graph`
//! record. Both schemas require stable UUID text identities and ordered lists;
//! geometry is forbidden from these records and remains presentation data.

use std::collections::BTreeMap;

use plc_compiler::IrFormalRef;
use plc_core::{ObjectId, PayloadValue};
use plc_lad::{
    CoilMode, ContactMode, LadBox, LadBranch, LadBranchId, LadBranchPath, LadBranchPathId, LadCall,
    LadDocument, LadDocumentId, LadEdgeId, LadFormalRef, LadInstance, LadNetwork, LadNetworkId,
    LadNode, LadNodeId, LadNodeKind, LadOperand, LadOperandId, LadOperandRef, LadPin,
    LadPinDirection, LadPortId, LadPortStatus, LadPowerEdge, LadPowerPort, LadPowerPortDirection,
    LadStateBinding, LadStateInstanceId,
};
use plc_language_tools::{
    ActivationRole, ConnectionId, ConnectionKind, EffectRole, FbdConnection, FbdDocument,
    FbdDocumentId, FbdNetwork, FbdNode, FbdPort, InstanceIdentity, NetworkId, NodeId, NodeKind,
    PortDirection, PortId, PortMultiplicity, PortStatus, StateInstanceId,
};
use plc_program::{
    BlockId, CallSiteId, InstanceOwner, InstancePath, InstructionCode, InstructionFormalId,
    InterfaceMemberId, StateKind, VariableRef,
};

use crate::software_projection::{
    as_record, parse_data_type, parse_identity, parse_value, record_text, record_unsigned,
};
use crate::{AuthoredLanguage, GraphicalBodyHook};

pub const CANONICAL_LAD_GRAPH_SCHEMA: &str = "edu.lad-semantic-graph/1";
pub const CANONICAL_FBD_GRAPH_SCHEMA: &str = "edu.fbd-semantic-graph/1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodedGraphicalBody {
    Lad(LadDocument),
    Fbd(FbdDocument),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphDecodeError {
    pub owner_object_id: ObjectId,
    pub owner_block_id: BlockId,
    pub semantic_path: String,
    pub message: String,
}

/// Decodes a canonical semantic graph into its native frontend model.
/// Recognized-but-invalid graphs remain in the project; this function returns
/// a navigable typed error and never emits partial executable IR.
pub fn decode_graphical_body(
    body: &GraphicalBodyHook,
) -> Result<DecodedGraphicalBody, GraphDecodeError> {
    let graph = body
        .semantic_payload
        .get("graph")
        .ok_or_else(|| error(body, "graph", "required graph record is absent"))?;
    let graph = as_record(graph, "graph").map_err(|message| error(body, "graph", message))?;
    match body.language {
        AuthoredLanguage::Lad => decode_lad(body, graph).map(DecodedGraphicalBody::Lad),
        AuthoredLanguage::Fbd => decode_fbd(body, graph).map(DecodedGraphicalBody::Fbd),
        AuthoredLanguage::Scl => Err(error(
            body,
            "language",
            "SCL source cannot be decoded as a graphical body",
        )),
    }
}

fn decode_fbd(
    body: &GraphicalBodyHook,
    graph: &BTreeMap<String, PayloadValue>,
) -> Result<FbdDocument, GraphDecodeError> {
    require_schema(body, graph, CANONICAL_FBD_GRAPH_SCHEMA)?;
    let document_id = FbdDocumentId::new(identity_field(body, graph, "documentId", "graph")?);
    let networks = record_list(body, graph, "networks", "graph")?;
    let mut decoded = Vec::with_capacity(networks.len());
    for (network_index, value) in networks.iter().enumerate() {
        let path = format!("graph.networks[{network_index}]");
        let network = record(body, value, &path)?;
        let network_id = NetworkId::new(identity_field(body, network, "id", &path)?);
        let semantic_order = u32_field(body, network, "semanticOrder", &path)?;
        let nodes = record_list(body, network, "nodes", &path)?;
        let mut decoded_nodes = Vec::with_capacity(nodes.len());
        for (node_index, node) in nodes.iter().enumerate() {
            decoded_nodes.push(decode_fbd_node(
                body,
                record(body, node, &format!("{path}.nodes[{node_index}]"))?,
                &format!("{path}.nodes[{node_index}]"),
            )?);
        }
        let connections = record_list(body, network, "connections", &path)?;
        let mut decoded_connections = Vec::with_capacity(connections.len());
        for (connection_index, connection) in connections.iter().enumerate() {
            let connection_path = format!("{path}.connections[{connection_index}]");
            let connection = record(body, connection, &connection_path)?;
            decoded_connections.push(FbdConnection {
                id: ConnectionId::new(identity_field(body, connection, "id", &connection_path)?),
                source: PortId::new(identity_field(
                    body,
                    connection,
                    "sourcePortId",
                    &connection_path,
                )?),
                target: PortId::new(identity_field(
                    body,
                    connection,
                    "targetPortId",
                    &connection_path,
                )?),
                kind: match text_field(body, connection, "kind", &connection_path)? {
                    "data" => ConnectionKind::Data,
                    "execution" => ConnectionKind::Execution,
                    value => {
                        return Err(error(
                            body,
                            format!("{connection_path}.kind"),
                            format!("unsupported FBD connection kind '{value}'"),
                        ));
                    }
                },
            });
        }
        decoded.push(FbdNetwork::from_parts(
            network_id,
            semantic_order,
            decoded_nodes,
            decoded_connections,
        ));
    }
    Ok(FbdDocument::new(document_id, body.owner_block_id, decoded))
}

fn decode_fbd_node(
    body: &GraphicalBodyHook,
    node: &BTreeMap<String, PayloadValue>,
    path: &str,
) -> Result<FbdNode, GraphDecodeError> {
    let id = NodeId::new(identity_field(body, node, "id", path)?);
    let semantic_order = u32_field(body, node, "semanticOrder", path)?;
    let kind = match text_field(body, node, "nodeKind", path)? {
        "constant" => {
            let data_type = parse_data_type(text_field(body, node, "dataType", path)?)
                .map_err(|message| error(body, format!("{path}.dataType"), message))?;
            let value = node
                .get("value")
                .ok_or_else(|| error(body, format!("{path}.value"), "constant value is absent"))?;
            NodeKind::Constant {
                value: parse_value(value, &data_type)
                    .map_err(|message| error(body, format!("{path}.value"), message))?,
            }
        }
        "load-member" => NodeKind::LoadMember {
            member: InterfaceMemberId::new(identity_field(body, node, "memberId", path)?),
        },
        "store-member" => NodeKind::StoreMember {
            member: InterfaceMemberId::new(identity_field(body, node, "memberId", path)?),
        },
        "instruction" => NodeKind::Instruction {
            code: instruction_code(body, node, path)?,
            instance: optional_fbd_instance(body, node, path, true)?,
        },
        "call" => NodeKind::Call {
            code: instruction_code(body, node, path)?,
            target: BlockId::new(identity_field(body, node, "targetBlockId", path)?),
            instance: optional_fbd_instance(body, node, path, false)?,
        },
        "unresolved" => NodeKind::Unresolved {
            requested_name: text_field(body, node, "requestedName", path)?.to_owned(),
        },
        value => {
            return Err(error(
                body,
                format!("{path}.nodeKind"),
                format!("unsupported FBD node kind '{value}'"),
            ));
        }
    };
    let ports = record_list(body, node, "ports", path)?;
    let mut decoded_ports = Vec::with_capacity(ports.len());
    for (index, port) in ports.iter().enumerate() {
        let port_path = format!("{path}.ports[{index}]");
        decoded_ports.push(decode_fbd_port(
            body,
            record(body, port, &port_path)?,
            &port_path,
        )?);
    }
    Ok(FbdNode::from_ports(id, semantic_order, kind, decoded_ports))
}

fn optional_fbd_instance(
    body: &GraphicalBodyHook,
    node: &BTreeMap<String, PayloadValue>,
    path: &str,
    allow_legacy_instruction_state: bool,
) -> Result<Option<InstanceIdentity>, GraphDecodeError> {
    let legacy = optional_identity_field(body, node, "stateInstanceId", path)?;
    let Some(value) = node.get("instance") else {
        if !allow_legacy_instruction_state && legacy.is_some() {
            return Err(error(
                body,
                format!("{path}.stateInstanceId"),
                "call nodes require a function-block instance record, not an instruction state identity",
            ));
        }
        return Ok(legacy
            .map(StateInstanceId::new)
            .map(InstanceIdentity::Instruction));
    };
    if matches!(value, PayloadValue::Null) {
        if !allow_legacy_instruction_state && legacy.is_some() {
            return Err(error(
                body,
                format!("{path}.stateInstanceId"),
                "call nodes require a function-block instance record, not an instruction state identity",
            ));
        }
        return Ok(legacy
            .map(StateInstanceId::new)
            .map(InstanceIdentity::Instruction));
    }
    if legacy.is_some() {
        return Err(error(
            body,
            format!("{path}.instance"),
            "instance and legacy stateInstanceId cannot both be present",
        ));
    }

    let instance_path = format!("{path}.instance");
    let instance = record(body, value, &instance_path)?;
    match text_field(body, instance, "kind", &instance_path)? {
        "instruction-state" => {
            if !allow_legacy_instruction_state {
                return Err(error(
                    body,
                    format!("{instance_path}.kind"),
                    "call nodes do not accept instruction-state instances",
                ));
            }
            Ok(Some(InstanceIdentity::Instruction(StateInstanceId::new(
                identity_field(body, instance, "stateInstanceId", &instance_path)?,
            ))))
        }
        "function-block" => Ok(Some(InstanceIdentity::FunctionBlock {
            root_instance_db: BlockId::new(identity_field(
                body,
                instance,
                "rootInstanceDbId",
                &instance_path,
            )?),
            multi_instance_members: identity_list_field(
                body,
                instance,
                "multiInstanceMemberIds",
                &instance_path,
            )?
            .into_iter()
            .map(InterfaceMemberId::new)
            .collect(),
        })),
        value => Err(error(
            body,
            format!("{instance_path}.kind"),
            format!("unsupported FBD instance kind '{value}'"),
        )),
    }
}

fn decode_fbd_port(
    body: &GraphicalBodyHook,
    port: &BTreeMap<String, PayloadValue>,
    path: &str,
) -> Result<FbdPort, GraphDecodeError> {
    let formal = match optional_text_field(body, port, "formalKind", path)? {
        None => {
            if port
                .get("formalId")
                .is_some_and(|value| !matches!(value, PayloadValue::Null))
            {
                return Err(error(
                    body,
                    format!("{path}.formalKind"),
                    "formalKind is required whenever formalId is present",
                ));
            }
            None
        }
        Some("instruction") => Some(IrFormalRef::Instruction(InstructionFormalId(u16_field(
            body, port, "formalId", path,
        )?))),
        Some("block-member") => Some(IrFormalRef::BlockMember(InterfaceMemberId::new(
            identity_field(body, port, "formalId", path)?,
        ))),
        Some(value) => {
            return Err(error(
                body,
                format!("{path}.formalKind"),
                format!("unsupported formal kind '{value}'"),
            ));
        }
    };
    Ok(FbdPort {
        id: PortId::new(identity_field(body, port, "id", path)?),
        name: text_field(body, port, "name", path)?.to_owned(),
        direction: match text_field(body, port, "direction", path)? {
            "input" => PortDirection::Input,
            "output" => PortDirection::Output,
            "execution-input" => PortDirection::ExecutionInput,
            "execution-output" => PortDirection::ExecutionOutput,
            value => {
                return Err(error(
                    body,
                    format!("{path}.direction"),
                    format!("unsupported FBD port direction '{value}'"),
                ));
            }
        },
        data_type: optional_text_field(body, port, "dataType", path)?
            .map(parse_data_type)
            .transpose()
            .map_err(|message| error(body, format!("{path}.dataType"), message))?,
        required: bool_field(body, port, "required", path)?,
        multiplicity: match text_field(body, port, "multiplicity", path)? {
            "one" => PortMultiplicity::One,
            "many" => PortMultiplicity::Many,
            value => {
                return Err(error(
                    body,
                    format!("{path}.multiplicity"),
                    format!("unsupported FBD port multiplicity '{value}'"),
                ));
            }
        },
        activation: match text_field(body, port, "activation", path)? {
            "none" => ActivationRole::None,
            "enable" => ActivationRole::Enable,
            "enable-output" => ActivationRole::EnableOutput,
            value => {
                return Err(error(
                    body,
                    format!("{path}.activation"),
                    format!("unsupported FBD activation role '{value}'"),
                ));
            }
        },
        status: match text_field(body, port, "status", path)? {
            "active" => PortStatus::Active,
            "stale" => PortStatus::Stale,
            value => {
                return Err(error(
                    body,
                    format!("{path}.status"),
                    format!("unsupported FBD port status '{value}'"),
                ));
            }
        },
        effect_role: match text_field(body, port, "effectRole", path)? {
            "value" => EffectRole::Value,
            "symbol-read" => EffectRole::SymbolRead,
            "symbol-write" => EffectRole::SymbolWrite,
            "call-parameter" => EffectRole::CallParameter,
            "state" => EffectRole::State,
            "execution" => EffectRole::Execution,
            value => {
                return Err(error(
                    body,
                    format!("{path}.effectRole"),
                    format!("unsupported FBD effect role '{value}'"),
                ));
            }
        },
        formal,
    })
}

fn decode_lad(
    body: &GraphicalBodyHook,
    graph: &BTreeMap<String, PayloadValue>,
) -> Result<LadDocument, GraphDecodeError> {
    require_schema(body, graph, CANONICAL_LAD_GRAPH_SCHEMA)?;
    let document_id = LadDocumentId::new(identity_field(body, graph, "documentId", "graph")?);
    let networks = record_list(body, graph, "networks", "graph")?;
    let mut decoded = Vec::with_capacity(networks.len());
    for (network_index, network) in networks.iter().enumerate() {
        let path = format!("graph.networks[{network_index}]");
        let network = record(body, network, &path)?;
        let nodes = record_list(body, network, "nodes", &path)?;
        let mut decoded_nodes = Vec::with_capacity(nodes.len());
        for (node_index, node) in nodes.iter().enumerate() {
            let node_path = format!("{path}.nodes[{node_index}]");
            decoded_nodes.push(decode_lad_node(
                body,
                record(body, node, &node_path)?,
                &node_path,
            )?);
        }
        let edges = record_list(body, network, "edges", &path)?;
        let mut decoded_edges = Vec::with_capacity(edges.len());
        for (edge_index, edge) in edges.iter().enumerate() {
            let edge_path = format!("{path}.edges[{edge_index}]");
            let edge = record(body, edge, &edge_path)?;
            decoded_edges.push(LadPowerEdge {
                id: LadEdgeId::new(identity_field(body, edge, "id", &edge_path)?),
                source: LadPortId::new(identity_field(body, edge, "sourcePortId", &edge_path)?),
                target: LadPortId::new(identity_field(body, edge, "targetPortId", &edge_path)?),
            });
        }
        let mut decoded_branches = Vec::new();
        for (branch_index, branch) in optional_record_list(body, network, "branches", &path)?
            .iter()
            .enumerate()
        {
            let branch_path = format!("{path}.branches[{branch_index}]");
            let branch = record(body, branch, &branch_path)?;
            let paths = record_list(body, branch, "paths", &branch_path)?;
            let mut decoded_paths = Vec::with_capacity(paths.len());
            for (index, branch_value) in paths.iter().enumerate() {
                let item_path = format!("{branch_path}.paths[{index}]");
                let branch_value = record(body, branch_value, &item_path)?;
                decoded_paths.push(LadBranchPath {
                    id: LadBranchPathId::new(identity_field(body, branch_value, "id", &item_path)?),
                    entry_edge: LadEdgeId::new(identity_field(
                        body,
                        branch_value,
                        "entryEdgeId",
                        &item_path,
                    )?),
                    exit_edge: LadEdgeId::new(identity_field(
                        body,
                        branch_value,
                        "exitEdgeId",
                        &item_path,
                    )?),
                });
            }
            decoded_branches.push(LadBranch::from_paths(
                LadBranchId::new(identity_field(body, branch, "id", &branch_path)?),
                LadNodeId::new(identity_field(body, branch, "splitNodeId", &branch_path)?),
                LadNodeId::new(identity_field(body, branch, "joinNodeId", &branch_path)?),
                decoded_paths,
            ));
        }
        decoded.push(LadNetwork::from_parts(
            LadNetworkId::new(identity_field(body, network, "id", &path)?),
            u32_field(body, network, "semanticOrder", &path)?,
            decoded_nodes,
            decoded_edges,
            decoded_branches,
        ));
    }
    let mut document = LadDocument::new(document_id, body.owner_block_id, decoded);
    document.semantic_revision =
        optional_unsigned_field(body, graph, "semanticRevision", "graph")?.unwrap_or(0);
    Ok(document)
}

fn decode_lad_node(
    body: &GraphicalBodyHook,
    node: &BTreeMap<String, PayloadValue>,
    path: &str,
) -> Result<LadNode, GraphDecodeError> {
    let kind = match text_field(body, node, "nodeKind", path)? {
        "power-source" => LadNodeKind::PowerSource,
        "contact" => LadNodeKind::Contact {
            mode: match text_field(body, node, "mode", path)? {
                "normally-open" => ContactMode::NormallyOpen,
                "normally-closed" => ContactMode::NormallyClosed,
                value => {
                    return Err(error(
                        body,
                        format!("{path}.mode"),
                        format!("unsupported contact mode '{value}'"),
                    ));
                }
            },
            operand: optional_operand(body, node, path)?,
        },
        "coil" => LadNodeKind::Coil {
            mode: match text_field(body, node, "mode", path)? {
                "normal" => CoilMode::Normal,
                "negated" => CoilMode::Negated,
                "set" => CoilMode::Set,
                "reset" => CoilMode::Reset,
                value => {
                    return Err(error(
                        body,
                        format!("{path}.mode"),
                        format!("unsupported coil mode '{value}'"),
                    ));
                }
            },
            operand: optional_operand(body, node, path)?,
        },
        "box" => LadNodeKind::Box(decode_lad_box(body, node, path)?),
        "call" => LadNodeKind::Call(decode_lad_call(body, node, path)?),
        "branch-split" => LadNodeKind::BranchSplit {
            branch: LadBranchId::new(identity_field(body, node, "branchId", path)?),
        },
        "branch-join" => LadNodeKind::BranchJoin {
            branch: LadBranchId::new(identity_field(body, node, "branchId", path)?),
        },
        "return" => LadNodeKind::Return,
        "unsupported-control" => LadNodeKind::UnsupportedControl {
            capability: text_field(body, node, "capability", path)?.to_owned(),
        },
        "unresolved" => LadNodeKind::Unresolved {
            requested_name: text_field(body, node, "requestedName", path)?.to_owned(),
        },
        value => {
            return Err(error(
                body,
                format!("{path}.nodeKind"),
                format!("unsupported LAD node kind '{value}'"),
            ));
        }
    };
    let ports = record_list(body, node, "powerPorts", path)?;
    let mut decoded_ports = Vec::with_capacity(ports.len());
    for (index, port) in ports.iter().enumerate() {
        let port_path = format!("{path}.powerPorts[{index}]");
        let port = record(body, port, &port_path)?;
        decoded_ports.push(LadPowerPort {
            id: LadPortId::new(identity_field(body, port, "id", &port_path)?),
            direction: match text_field(body, port, "direction", &port_path)? {
                "input" => LadPowerPortDirection::Input,
                "output" => LadPowerPortDirection::Output,
                value => {
                    return Err(error(
                        body,
                        format!("{port_path}.direction"),
                        format!("unsupported LAD power-port direction '{value}'"),
                    ));
                }
            },
        });
    }
    Ok(LadNode::from_power_ports(
        LadNodeId::new(identity_field(body, node, "id", path)?),
        u32_field(body, node, "semanticOrder", path)?,
        kind,
        decoded_ports,
    ))
}

fn decode_lad_box(
    body: &GraphicalBodyHook,
    node: &BTreeMap<String, PayloadValue>,
    path: &str,
) -> Result<LadBox, GraphDecodeError> {
    Ok(LadBox::from_pins(
        instruction_code(body, node, path)?,
        decode_lad_pins(body, node, path)?,
        optional_lad_state(body, node, path)?,
    ))
}

fn decode_lad_call(
    body: &GraphicalBodyHook,
    node: &BTreeMap<String, PayloadValue>,
    path: &str,
) -> Result<LadCall, GraphDecodeError> {
    Ok(LadCall::from_pins(
        CallSiteId::new(identity_field(body, node, "callSiteId", path)?),
        instruction_code(body, node, path)?,
        BlockId::new(identity_field(body, node, "targetBlockId", path)?),
        optional_lad_instance(body, node, path)?,
        decode_lad_pins(body, node, path)?,
    ))
}

fn decode_lad_pins(
    body: &GraphicalBodyHook,
    node: &BTreeMap<String, PayloadValue>,
    path: &str,
) -> Result<Vec<LadPin>, GraphDecodeError> {
    let pins = record_list(body, node, "pins", path)?;
    let mut decoded = Vec::with_capacity(pins.len());
    for (index, value) in pins.iter().enumerate() {
        let pin_path = format!("{path}.pins[{index}]");
        let pin = record(body, value, &pin_path)?;
        let formal_kind = optional_text_field(body, pin, "formalKind", &pin_path)?;
        let formal = match formal_kind {
            Some("instruction") => Some(LadFormalRef::Instruction(InstructionFormalId(u16_field(
                body, pin, "formalId", &pin_path,
            )?))),
            Some("block-member") => Some(LadFormalRef::BlockMember(InterfaceMemberId::new(
                identity_field(body, pin, "formalId", &pin_path)?,
            ))),
            Some(value) => {
                return Err(error(
                    body,
                    format!("{pin_path}.formalKind"),
                    format!("unsupported LAD formal kind '{value}'"),
                ));
            }
            None => {
                if pin
                    .get("formalId")
                    .is_some_and(|value| !matches!(value, PayloadValue::Null))
                {
                    return Err(error(
                        body,
                        format!("{pin_path}.formalKind"),
                        "formalKind is required whenever formalId is present",
                    ));
                }
                None
            }
        };
        let data_type = parse_data_type(text_field(body, pin, "dataType", &pin_path)?)
            .map_err(|message| error(body, format!("{pin_path}.dataType"), message))?;
        decoded.push(LadPin {
            id: LadPortId::new(identity_field(body, pin, "id", &pin_path)?),
            formal,
            name: text_field(body, pin, "name", &pin_path)?.to_owned(),
            direction: match text_field(body, pin, "direction", &pin_path)? {
                "input" => LadPinDirection::Input,
                "output" => LadPinDirection::Output,
                "inout" => LadPinDirection::InOut,
                "activation" => LadPinDirection::Activation,
                "status" => LadPinDirection::Status,
                value => {
                    return Err(error(
                        body,
                        format!("{pin_path}.direction"),
                        format!("unsupported LAD pin direction '{value}'"),
                    ));
                }
            },
            data_type,
            required: bool_field(body, pin, "required", &pin_path)?,
            status: match text_field(body, pin, "status", &pin_path)? {
                "active" => LadPortStatus::Active,
                "stale" => LadPortStatus::Stale,
                "orphan" => LadPortStatus::Orphan,
                value => {
                    return Err(error(
                        body,
                        format!("{pin_path}.status"),
                        format!("unsupported LAD pin status '{value}'"),
                    ));
                }
            },
            binding: optional_lad_operand_field(body, pin, "binding", &pin_path)?,
        });
    }
    Ok(decoded)
}

fn optional_lad_state(
    body: &GraphicalBodyHook,
    node: &BTreeMap<String, PayloadValue>,
    path: &str,
) -> Result<Option<LadStateBinding>, GraphDecodeError> {
    let Some(value) = node.get("state") else {
        return Ok(None);
    };
    if matches!(value, PayloadValue::Null) {
        return Ok(None);
    }
    let state_path = format!("{path}.state");
    let state = record(body, value, &state_path)?;
    let storage_value = state.get("storage").ok_or_else(|| {
        error(
            body,
            format!("{state_path}.storage"),
            "required storage record is absent",
        )
    })?;
    Ok(Some(LadStateBinding {
        invocation: LadStateInstanceId::new(identity_field(
            body,
            state,
            "invocationId",
            &state_path,
        )?),
        storage: decode_variable_ref(body, storage_value, &format!("{state_path}.storage"))?,
        kind: match text_field(body, state, "stateKind", &state_path)? {
            "edge" => StateKind::Edge,
            "timer" => StateKind::Timer,
            "counter" => StateKind::Counter,
            value => {
                return Err(error(
                    body,
                    format!("{state_path}.stateKind"),
                    format!("unsupported LAD state kind '{value}'"),
                ));
            }
        },
    }))
}

fn optional_lad_instance(
    body: &GraphicalBodyHook,
    node: &BTreeMap<String, PayloadValue>,
    path: &str,
) -> Result<Option<LadInstance>, GraphDecodeError> {
    let Some(value) = node.get("instance") else {
        return Ok(None);
    };
    if matches!(value, PayloadValue::Null) {
        return Ok(None);
    }
    let instance_path = format!("{path}.instance");
    let instance = record(body, value, &instance_path)?;
    let owner_value = instance.get("owner").ok_or_else(|| {
        error(
            body,
            format!("{instance_path}.owner"),
            "required instance owner record is absent",
        )
    })?;
    let owner_path = format!("{instance_path}.owner");
    let owner = record(body, owner_value, &owner_path)?;
    let owner = match text_field(body, owner, "kind", &owner_path)? {
        "instance-db" => InstanceOwner::InstanceDb(BlockId::new(identity_field(
            body,
            owner,
            "instanceDbId",
            &owner_path,
        )?)),
        "multi-instance" => InstanceOwner::MultiInstance {
            owner_fb: BlockId::new(identity_field(body, owner, "ownerFbId", &owner_path)?),
            static_member: InterfaceMemberId::new(identity_field(
                body,
                owner,
                "staticMemberId",
                &owner_path,
            )?),
        },
        value => {
            return Err(error(
                body,
                format!("{owner_path}.kind"),
                format!("unsupported LAD instance-owner kind '{value}'"),
            ));
        }
    };

    let path_value = instance.get("path").ok_or_else(|| {
        error(
            body,
            format!("{instance_path}.path"),
            "required instance path record is absent",
        )
    })?;
    let structural_path = format!("{instance_path}.path");
    let path_record = record(body, path_value, &structural_path)?;
    Ok(Some(LadInstance {
        owner,
        path: InstancePath {
            root_instance_db: BlockId::new(identity_field(
                body,
                path_record,
                "rootInstanceDbId",
                &structural_path,
            )?),
            multi_instance_slots: identity_list_field(
                body,
                path_record,
                "multiInstanceMemberIds",
                &structural_path,
            )?
            .into_iter()
            .map(InterfaceMemberId::new)
            .collect(),
        },
    }))
}

fn optional_operand(
    body: &GraphicalBodyHook,
    node: &BTreeMap<String, PayloadValue>,
    path: &str,
) -> Result<Option<LadOperandRef>, GraphDecodeError> {
    optional_lad_operand_field(body, node, "operand", path)
}

fn optional_lad_operand_field(
    body: &GraphicalBodyHook,
    parent: &BTreeMap<String, PayloadValue>,
    field: &str,
    path: &str,
) -> Result<Option<LadOperandRef>, GraphDecodeError> {
    let Some(value) = parent.get(field) else {
        return Ok(None);
    };
    if matches!(value, PayloadValue::Null) {
        return Ok(None);
    }
    let operand_path = format!("{path}.{field}");
    let operand = record(body, value, &operand_path)?;
    let value = match text_field(body, operand, "kind", &operand_path)? {
        "caller-member" | "data-block-member" => {
            LadOperand::Variable(decode_variable_ref_record(body, operand, &operand_path)?)
        }
        "constant" => {
            let data_type = parse_data_type(text_field(body, operand, "dataType", &operand_path)?)
                .map_err(|message| error(body, format!("{operand_path}.dataType"), message))?;
            let canonical = operand.get("value").ok_or_else(|| {
                error(
                    body,
                    format!("{operand_path}.value"),
                    "constant operand value is absent",
                )
            })?;
            LadOperand::Constant(
                parse_value(canonical, &data_type)
                    .map_err(|message| error(body, format!("{operand_path}.value"), message))?,
            )
        }
        "unresolved" => LadOperand::Unresolved {
            spelling: text_field(body, operand, "spelling", &operand_path)?.to_owned(),
        },
        "expression" => LadOperand::Expression {
            source: text_field(body, operand, "source", &operand_path)?.to_owned(),
        },
        value => {
            return Err(error(
                body,
                format!("{operand_path}.kind"),
                format!("unsupported LAD operand kind '{value}'"),
            ));
        }
    };
    Ok(Some(LadOperandRef {
        id: LadOperandId::new(identity_field(body, operand, "id", &operand_path)?),
        value,
    }))
}

fn decode_variable_ref(
    body: &GraphicalBodyHook,
    value: &PayloadValue,
    path: &str,
) -> Result<VariableRef, GraphDecodeError> {
    decode_variable_ref_record(body, record(body, value, path)?, path)
}

fn decode_variable_ref_record(
    body: &GraphicalBodyHook,
    variable: &BTreeMap<String, PayloadValue>,
    path: &str,
) -> Result<VariableRef, GraphDecodeError> {
    match text_field(body, variable, "kind", path)? {
        "caller-member" => Ok(VariableRef::CallerMember(InterfaceMemberId::new(
            identity_field(body, variable, "memberId", path)?,
        ))),
        "data-block-member" => Ok(VariableRef::DataBlockMember {
            data_block: BlockId::new(identity_field(body, variable, "dataBlockId", path)?),
            member: InterfaceMemberId::new(identity_field(body, variable, "memberId", path)?),
        }),
        value => Err(error(
            body,
            format!("{path}.kind"),
            format!("unsupported variable-reference kind '{value}'"),
        )),
    }
}

fn require_schema(
    body: &GraphicalBodyHook,
    graph: &BTreeMap<String, PayloadValue>,
    expected: &str,
) -> Result<(), GraphDecodeError> {
    let actual = text_field(body, graph, "schema", "graph")?;
    if actual == expected {
        Ok(())
    } else {
        Err(error(
            body,
            "graph.schema",
            format!("unsupported graph schema '{actual}'; expected '{expected}'"),
        ))
    }
}

fn record<'a>(
    body: &GraphicalBodyHook,
    value: &'a PayloadValue,
    path: &str,
) -> Result<&'a BTreeMap<String, PayloadValue>, GraphDecodeError> {
    as_record(value, path).map_err(|message| error(body, path, message))
}

fn record_list<'a>(
    body: &GraphicalBodyHook,
    record: &'a BTreeMap<String, PayloadValue>,
    key: &str,
    path: &str,
) -> Result<&'a [PayloadValue], GraphDecodeError> {
    match record.get(key) {
        Some(PayloadValue::List(values)) => Ok(values),
        Some(_) => Err(error(body, format!("{path}.{key}"), "value must be a list")),
        None => Err(error(
            body,
            format!("{path}.{key}"),
            "required list is absent",
        )),
    }
}

fn optional_record_list<'a>(
    body: &GraphicalBodyHook,
    record: &'a BTreeMap<String, PayloadValue>,
    key: &str,
    path: &str,
) -> Result<&'a [PayloadValue], GraphDecodeError> {
    match record.get(key) {
        Some(PayloadValue::List(values)) => Ok(values),
        Some(_) => Err(error(body, format!("{path}.{key}"), "value must be a list")),
        None => Ok(&[]),
    }
}

fn identity_list_field(
    body: &GraphicalBodyHook,
    record: &BTreeMap<String, PayloadValue>,
    key: &str,
    path: &str,
) -> Result<Vec<u128>, GraphDecodeError> {
    let values = record_list(body, record, key, path)?;
    let mut identities = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let item_path = format!("{path}.{key}[{index}]");
        let PayloadValue::String(value) = value else {
            return Err(error(
                body,
                item_path,
                "identity must be canonical UUID text",
            ));
        };
        identities.push(parse_identity(value).map_err(|message| error(body, item_path, message))?);
    }
    Ok(identities)
}

fn text_field<'a>(
    body: &GraphicalBodyHook,
    record: &'a BTreeMap<String, PayloadValue>,
    key: &str,
    path: &str,
) -> Result<&'a str, GraphDecodeError> {
    record_text(record, key).map_err(|message| error(body, format!("{path}.{key}"), message))
}

fn optional_text_field<'a>(
    body: &GraphicalBodyHook,
    record: &'a BTreeMap<String, PayloadValue>,
    key: &str,
    path: &str,
) -> Result<Option<&'a str>, GraphDecodeError> {
    match record.get(key) {
        None | Some(PayloadValue::Null) => Ok(None),
        Some(PayloadValue::String(value)) => Ok(Some(value)),
        Some(_) => Err(error(
            body,
            format!("{path}.{key}"),
            "value must be text or null",
        )),
    }
}

fn identity_field(
    body: &GraphicalBodyHook,
    record: &BTreeMap<String, PayloadValue>,
    key: &str,
    path: &str,
) -> Result<u128, GraphDecodeError> {
    parse_identity(text_field(body, record, key, path)?)
        .map_err(|message| error(body, format!("{path}.{key}"), message))
}

fn optional_identity_field(
    body: &GraphicalBodyHook,
    record: &BTreeMap<String, PayloadValue>,
    key: &str,
    path: &str,
) -> Result<Option<u128>, GraphDecodeError> {
    optional_text_field(body, record, key, path)?
        .map(parse_identity)
        .transpose()
        .map_err(|message| error(body, format!("{path}.{key}"), message))
}

fn u32_field(
    body: &GraphicalBodyHook,
    record: &BTreeMap<String, PayloadValue>,
    key: &str,
    path: &str,
) -> Result<u32, GraphDecodeError> {
    u32::try_from(
        record_unsigned(record, key)
            .map_err(|message| error(body, format!("{path}.{key}"), message))?,
    )
    .map_err(|_| error(body, format!("{path}.{key}"), "value exceeds UInt32"))
}

fn u16_field(
    body: &GraphicalBodyHook,
    record: &BTreeMap<String, PayloadValue>,
    key: &str,
    path: &str,
) -> Result<u16, GraphDecodeError> {
    u16::try_from(
        record_unsigned(record, key)
            .map_err(|message| error(body, format!("{path}.{key}"), message))?,
    )
    .map_err(|_| error(body, format!("{path}.{key}"), "value exceeds UInt16"))
}

fn optional_unsigned_field(
    body: &GraphicalBodyHook,
    record: &BTreeMap<String, PayloadValue>,
    key: &str,
    path: &str,
) -> Result<Option<u64>, GraphDecodeError> {
    match record.get(key) {
        None => Ok(None),
        Some(PayloadValue::Unsigned(value)) => Ok(Some(*value)),
        Some(_) => Err(error(
            body,
            format!("{path}.{key}"),
            "value must be unsigned",
        )),
    }
}

fn bool_field(
    body: &GraphicalBodyHook,
    record: &BTreeMap<String, PayloadValue>,
    key: &str,
    path: &str,
) -> Result<bool, GraphDecodeError> {
    match record.get(key) {
        Some(PayloadValue::Bool(value)) => Ok(*value),
        Some(_) => Err(error(
            body,
            format!("{path}.{key}"),
            "value must be Boolean",
        )),
        None => Err(error(
            body,
            format!("{path}.{key}"),
            "required Boolean is absent",
        )),
    }
}

fn instruction_code(
    body: &GraphicalBodyHook,
    record: &BTreeMap<String, PayloadValue>,
    path: &str,
) -> Result<InstructionCode, GraphDecodeError> {
    u16_field(body, record, "instructionCode", path).map(InstructionCode)
}

fn error(
    body: &GraphicalBodyHook,
    path: impl Into<String>,
    message: impl Into<String>,
) -> GraphDecodeError {
    GraphDecodeError {
        owner_object_id: body.owner_object_id,
        owner_block_id: body.owner_block_id,
        semantic_path: path.into(),
        message: message.into(),
    }
}
