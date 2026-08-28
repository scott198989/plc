use alloc::{collections::BTreeMap, string::String, vec::Vec};

use plc_compiler::IrFormalRef;
use plc_core::{Sha256Digest, sha256};
use plc_program::{
    BlockId, CanonicalValue, DataType, DisabledExecutionBehavior as RegistryDisabledBehavior,
    InstructionActivationPolicy, InstructionCode, InterfaceMemberId, phase2_instruction_registry,
};
use plc_types::{AggregateLimits, CanonicalType, PlcValue, PrimitiveType, ScalarValue};

use crate::{
    ConnectionId, FBD_SCHEMA_VERSION, FbdDocumentId, NetworkId, NodeId, PortId, StateInstanceId,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PortDirection {
    Input,
    Output,
    ExecutionInput,
    ExecutionOutput,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PortMultiplicity {
    One,
    Many,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActivationRole {
    None,
    Enable,
    EnableOutput,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PortStatus {
    Active,
    Stale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EffectRole {
    Value,
    SymbolRead,
    SymbolWrite,
    CallParameter,
    State,
    Execution,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FbdPort {
    pub id: PortId,
    pub name: String,
    pub direction: PortDirection,
    pub data_type: Option<DataType>,
    pub required: bool,
    pub multiplicity: PortMultiplicity,
    pub activation: ActivationRole,
    pub status: PortStatus,
    pub effect_role: EffectRole,
    /// Stable registry formal or callee-interface member identity. Primitive
    /// constant/load/store ports use `None`.
    pub formal: Option<IrFormalRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InstanceIdentity {
    Instruction(StateInstanceId),
    FunctionBlock {
        root_instance_db: BlockId,
        multi_instance_members: Vec<InterfaceMemberId>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Constant {
        value: CanonicalValue,
    },
    LoadMember {
        member: InterfaceMemberId,
    },
    StoreMember {
        member: InterfaceMemberId,
    },
    Instruction {
        code: InstructionCode,
        instance: Option<InstanceIdentity>,
    },
    Call {
        code: InstructionCode,
        target: BlockId,
        instance: Option<InstanceIdentity>,
    },
    Unresolved {
        requested_name: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FbdNode {
    pub id: NodeId,
    /// Stored semantic/effect order. Valid networks use contiguous zero-based
    /// values matching `ordered_node_ids`.
    pub semantic_order: u32,
    pub kind: NodeKind,
    pub ports: BTreeMap<PortId, FbdPort>,
    pub ordered_port_ids: Vec<PortId>,
}

impl FbdNode {
    #[must_use]
    pub fn from_ports(
        id: NodeId,
        semantic_order: u32,
        kind: NodeKind,
        ports: impl IntoIterator<Item = FbdPort>,
    ) -> Self {
        let mut by_id = BTreeMap::new();
        let mut ordered_port_ids = Vec::new();
        for port in ports {
            ordered_port_ids.push(port.id);
            by_id.insert(port.id, port);
        }
        Self {
            id,
            semantic_order,
            kind,
            ports: by_id,
            ordered_port_ids,
        }
    }

    #[must_use]
    pub fn port(&self, id: PortId) -> Option<&FbdPort> {
        self.ports.get(&id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConnectionKind {
    Data,
    Execution,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FbdConnection {
    pub id: ConnectionId,
    pub source: PortId,
    pub target: PortId,
    pub kind: ConnectionKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FbdNetwork {
    pub id: NetworkId,
    pub semantic_order: u32,
    pub nodes: BTreeMap<NodeId, FbdNode>,
    pub ordered_node_ids: Vec<NodeId>,
    pub connections: BTreeMap<ConnectionId, FbdConnection>,
}

impl FbdNetwork {
    #[must_use]
    pub fn from_parts(
        id: NetworkId,
        semantic_order: u32,
        nodes: impl IntoIterator<Item = FbdNode>,
        connections: impl IntoIterator<Item = FbdConnection>,
    ) -> Self {
        let mut node_map = BTreeMap::new();
        let mut ordered_node_ids = Vec::new();
        for node in nodes {
            ordered_node_ids.push(node.id);
            node_map.insert(node.id, node);
        }
        let connections = connections
            .into_iter()
            .map(|connection| (connection.id, connection))
            .collect();
        Self {
            id,
            semantic_order,
            nodes: node_map,
            ordered_node_ids,
            connections,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FbdDocument {
    pub schema_version: String,
    pub id: FbdDocumentId,
    pub owner: BlockId,
    pub networks: BTreeMap<NetworkId, FbdNetwork>,
    pub ordered_network_ids: Vec<NetworkId>,
}

impl FbdDocument {
    #[must_use]
    pub fn new(
        id: FbdDocumentId,
        owner: BlockId,
        networks: impl IntoIterator<Item = FbdNetwork>,
    ) -> Self {
        let mut by_id = BTreeMap::new();
        let mut ordered_network_ids = Vec::new();
        for network in networks {
            ordered_network_ids.push(network.id);
            by_id.insert(network.id, network);
        }
        Self {
            schema_version: FBD_SCHEMA_VERSION.into(),
            id,
            owner,
            networks: by_id,
            ordered_network_ids,
        }
    }

    /// Fingerprints only semantic FBD state. Node coordinates, edge routing,
    /// groups, alignment, and viewport state are absent by construction.
    #[must_use]
    pub fn semantic_fingerprint(&self) -> Sha256Digest {
        let mut bytes = Vec::new();
        push_string(&mut bytes, "PES-FBD-SEMANTIC-1");
        push_string(&mut bytes, &self.schema_version);
        push_u128(&mut bytes, self.id.get());
        push_u128(&mut bytes, self.owner.get());
        push_len(&mut bytes, self.ordered_network_ids.len());
        for network_id in &self.ordered_network_ids {
            push_u128(&mut bytes, network_id.get());
        }
        push_len(&mut bytes, self.networks.len());
        for (network_id, network) in &self.networks {
            push_u128(&mut bytes, network_id.get());
            encode_network(&mut bytes, network);
        }
        sha256(&bytes)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RoutePoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeLayout {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub group: Option<u128>,
    pub alignment_rank: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FbdLayout {
    pub nodes: BTreeMap<NodeId, NodeLayout>,
    pub routes: BTreeMap<ConnectionId, Vec<RoutePoint>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisabledOutputBehavior {
    DefaultValue,
    StoredValueWithoutUpdate,
    NoEffect,
}

/// Defines the canonical disabled behavior without executing it. Runtime
/// enforcement remains in the one shared execution engine.
#[must_use]
pub fn disabled_output_behavior(kind: &NodeKind) -> DisabledOutputBehavior {
    match kind {
        NodeKind::Constant { .. } | NodeKind::LoadMember { .. } => {
            DisabledOutputBehavior::DefaultValue
        }
        NodeKind::StoreMember { .. } | NodeKind::Call { .. } | NodeKind::Unresolved { .. } => {
            DisabledOutputBehavior::NoEffect
        }
        NodeKind::Instruction { code, .. } => phase2_instruction_registry().lookup(*code).map_or(
            DisabledOutputBehavior::NoEffect,
            |definition| match definition.activation {
                InstructionActivationPolicy::None => DisabledOutputBehavior::NoEffect,
                InstructionActivationPolicy::EnableStatus { when_disabled, .. } => {
                    match when_disabled {
                        RegistryDisabledBehavior::DefaultOutputsNoStateChange => {
                            DisabledOutputBehavior::DefaultValue
                        }
                        RegistryDisabledBehavior::PreserveOutputsNoStateChange => {
                            DisabledOutputBehavior::StoredValueWithoutUpdate
                        }
                        RegistryDisabledBehavior::SuppressEffects => {
                            DisabledOutputBehavior::NoEffect
                        }
                    }
                }
            },
        ),
    }
}

fn encode_network(bytes: &mut Vec<u8>, network: &FbdNetwork) {
    bytes.push(1);
    push_u128(bytes, network.id.get());
    push_u32(bytes, network.semantic_order);
    push_len(bytes, network.ordered_node_ids.len());
    for node_id in &network.ordered_node_ids {
        push_u128(bytes, node_id.get());
    }
    push_len(bytes, network.nodes.len());
    for (node_id, node) in &network.nodes {
        push_u128(bytes, node_id.get());
        encode_node(bytes, node);
    }
    push_len(bytes, network.connections.len());
    for (id, connection) in &network.connections {
        push_u128(bytes, id.get());
        push_u128(bytes, connection.source.get());
        push_u128(bytes, connection.target.get());
        bytes.push(match connection.kind {
            ConnectionKind::Data => 1,
            ConnectionKind::Execution => 2,
        });
    }
}

fn encode_node(bytes: &mut Vec<u8>, node: &FbdNode) {
    bytes.push(1);
    push_u128(bytes, node.id.get());
    push_u32(bytes, node.semantic_order);
    encode_node_kind(bytes, &node.kind);
    push_len(bytes, node.ordered_port_ids.len());
    for port_id in &node.ordered_port_ids {
        push_u128(bytes, port_id.get());
    }
    push_len(bytes, node.ports.len());
    for (port_id, port) in &node.ports {
        push_u128(bytes, port_id.get());
        encode_port(bytes, port);
    }
}

fn encode_node_kind(bytes: &mut Vec<u8>, kind: &NodeKind) {
    match kind {
        NodeKind::Constant { value } => {
            bytes.push(1);
            encode_value(bytes, value);
        }
        NodeKind::LoadMember { member } => {
            bytes.push(2);
            push_u128(bytes, member.get());
        }
        NodeKind::StoreMember { member } => {
            bytes.push(3);
            push_u128(bytes, member.get());
        }
        NodeKind::Instruction { code, instance } => {
            bytes.push(4);
            push_u16(bytes, code.0);
            encode_instance(bytes, instance.as_ref());
        }
        NodeKind::Call {
            code,
            target,
            instance,
        } => {
            bytes.push(5);
            push_u16(bytes, code.0);
            push_u128(bytes, target.get());
            encode_instance(bytes, instance.as_ref());
        }
        NodeKind::Unresolved { requested_name } => {
            bytes.push(6);
            push_string(bytes, requested_name);
        }
    }
}

fn encode_instance(bytes: &mut Vec<u8>, instance: Option<&InstanceIdentity>) {
    match instance {
        None => bytes.push(0),
        Some(InstanceIdentity::Instruction(id)) => {
            bytes.push(1);
            push_u128(bytes, id.get());
        }
        Some(InstanceIdentity::FunctionBlock {
            root_instance_db,
            multi_instance_members,
        }) => {
            bytes.push(2);
            push_u128(bytes, root_instance_db.get());
            push_len(bytes, multi_instance_members.len());
            for member in multi_instance_members {
                push_u128(bytes, member.get());
            }
        }
    }
}

fn encode_port(bytes: &mut Vec<u8>, port: &FbdPort) {
    bytes.push(1);
    push_u128(bytes, port.id.get());
    push_string(bytes, &port.name);
    bytes.push(port_direction_tag(port.direction));
    match &port.data_type {
        Some(data_type) => {
            bytes.push(1);
            encode_type(bytes, data_type);
        }
        None => bytes.push(0),
    }
    bytes.push(u8::from(port.required));
    bytes.push(match port.multiplicity {
        PortMultiplicity::One => 1,
        PortMultiplicity::Many => 2,
    });
    bytes.push(match port.activation {
        ActivationRole::None => 0,
        ActivationRole::Enable => 1,
        ActivationRole::EnableOutput => 2,
    });
    bytes.push(match port.status {
        PortStatus::Active => 1,
        PortStatus::Stale => 2,
    });
    bytes.push(match port.effect_role {
        EffectRole::Value => 1,
        EffectRole::SymbolRead => 2,
        EffectRole::SymbolWrite => 3,
        EffectRole::CallParameter => 4,
        EffectRole::State => 5,
        EffectRole::Execution => 6,
    });
    match port.formal {
        None => bytes.push(0),
        Some(IrFormalRef::Instruction(formal)) => {
            bytes.push(1);
            push_u16(bytes, formal.0);
        }
        Some(IrFormalRef::BlockMember(member)) => {
            bytes.push(2);
            push_u128(bytes, member.get());
        }
    }
}

fn encode_type(bytes: &mut Vec<u8>, data_type: &DataType) {
    match data_type {
        DataType::Bool => bytes.push(1),
        DataType::Int => bytes.push(2),
        DataType::DInt => bytes.push(3),
        DataType::Real => bytes.push(4),
        DataType::Time => bytes.push(5),
        DataType::String { capacity } => {
            bytes.push(6);
            push_u16(bytes, *capacity);
        }
        DataType::Named(name) => {
            bytes.push(7);
            push_string(bytes, name);
        }
        DataType::BlockInstance(block) => {
            bytes.push(8);
            push_u128(bytes, block.get());
        }
        DataType::InstructionState(state) => {
            bytes.push(9);
            bytes.push(match state {
                plc_program::StateKind::Edge => 1,
                plc_program::StateKind::Timer => 2,
                plc_program::StateKind::Counter => 3,
            });
        }
        DataType::SInt => bytes.push(10),
        DataType::LInt => bytes.push(11),
        DataType::USInt => bytes.push(12),
        DataType::UInt => bytes.push(13),
        DataType::UDInt => bytes.push(14),
        DataType::ULInt => bytes.push(15),
        DataType::Byte => bytes.push(16),
        DataType::Word => bytes.push(17),
        DataType::DWord => bytes.push(18),
        DataType::LWord => bytes.push(19),
        DataType::LReal => bytes.push(20),
        DataType::Char => bytes.push(21),
        DataType::Aggregate(data_type) => {
            bytes.push(22);
            if let Ok(encoded) = data_type.canonical_bytes(AggregateLimits::edu21()) {
                bytes.push(1);
                push_len(bytes, encoded.len());
                bytes.extend_from_slice(&encoded);
            } else {
                bytes.push(0);
                encode_invalid_aggregate_type(bytes, data_type);
            }
        }
    }
}

fn encode_value(bytes: &mut Vec<u8>, value: &CanonicalValue) {
    match value {
        CanonicalValue::Bool(value) => {
            bytes.push(1);
            bytes.push(u8::from(*value));
        }
        CanonicalValue::Int(value) => {
            bytes.push(2);
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        CanonicalValue::DInt(value) => {
            bytes.push(3);
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        CanonicalValue::RealBits(value) => {
            bytes.push(4);
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        CanonicalValue::TimeMilliseconds(value) => {
            bytes.push(5);
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        CanonicalValue::StringBytes(value) => {
            bytes.push(6);
            push_len(bytes, value.len());
            bytes.extend_from_slice(value);
        }
        CanonicalValue::SInt(value) => {
            bytes.push(7);
            bytes.push(value.cast_unsigned());
        }
        CanonicalValue::LInt(value) => {
            bytes.push(8);
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        CanonicalValue::USInt(value) => {
            bytes.push(9);
            bytes.push(*value);
        }
        CanonicalValue::UInt(value) => {
            bytes.push(10);
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        CanonicalValue::UDInt(value) => {
            bytes.push(11);
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        CanonicalValue::ULInt(value) => {
            bytes.push(12);
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        CanonicalValue::Byte(value) => {
            bytes.push(13);
            bytes.push(*value);
        }
        CanonicalValue::Word(value) => {
            bytes.push(14);
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        CanonicalValue::DWord(value) => {
            bytes.push(15);
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        CanonicalValue::LWord(value) => {
            bytes.push(16);
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        CanonicalValue::LRealBits(value) => {
            bytes.push(17);
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        CanonicalValue::Char(value) => {
            bytes.push(18);
            bytes.push(*value);
        }
        CanonicalValue::Aggregate(value) => {
            bytes.push(19);
            encode_plc_value(bytes, value);
        }
    }
}

fn encode_invalid_aggregate_type(bytes: &mut Vec<u8>, data_type: &CanonicalType) {
    match data_type {
        CanonicalType::Primitive(primitive) => {
            bytes.push(1);
            push_string(bytes, primitive.stable_id());
            if let PrimitiveType::String(capacity) = primitive {
                bytes.push(*capacity);
            }
        }
        CanonicalType::Array {
            dimensions,
            element_type,
        } => {
            bytes.push(2);
            push_len(bytes, dimensions.len());
            for bound in dimensions {
                bytes.extend_from_slice(&bound.lower.to_be_bytes());
                bytes.extend_from_slice(&bound.upper.to_be_bytes());
            }
            encode_invalid_aggregate_type(bytes, element_type);
        }
        CanonicalType::AnonymousStruct { members } => {
            bytes.push(3);
            encode_invalid_members(bytes, members);
        }
        CanonicalType::NamedStruct { id, members } => {
            bytes.push(4);
            bytes.extend_from_slice(&id.as_bytes());
            encode_invalid_members(bytes, members);
        }
    }
}

fn encode_invalid_members(bytes: &mut Vec<u8>, members: &[plc_types::StructMember]) {
    push_len(bytes, members.len());
    for member in members {
        bytes.extend_from_slice(&member.id.as_bytes());
        push_string(bytes, &member.name);
        bytes.extend_from_slice(&member.declared_order.to_be_bytes());
        encode_invalid_aggregate_type(bytes, &member.data_type);
        match &member.reusable_default {
            Some(value) => {
                bytes.push(1);
                encode_plc_value(bytes, value);
            }
            None => bytes.push(0),
        }
    }
}

fn encode_plc_value(bytes: &mut Vec<u8>, value: &PlcValue) {
    match value {
        PlcValue::Scalar(value) => {
            bytes.push(1);
            push_string(bytes, value.data_type().stable_id());
            if let PrimitiveType::String(capacity) = value.data_type() {
                bytes.push(capacity);
            }
            match value.value() {
                ScalarValue::Bool(value) => {
                    bytes.push(1);
                    bytes.push(u8::from(*value));
                }
                ScalarValue::Signed(value) => {
                    bytes.push(2);
                    bytes.extend_from_slice(&value.to_be_bytes());
                }
                ScalarValue::Unsigned(value) | ScalarValue::BitString(value) => {
                    bytes.push(3);
                    bytes.extend_from_slice(&value.to_be_bytes());
                }
                ScalarValue::Real(value) => {
                    bytes.push(4);
                    bytes.extend_from_slice(&value.bits().to_be_bytes());
                }
                ScalarValue::Lreal(value) => {
                    bytes.push(5);
                    bytes.extend_from_slice(&value.bits().to_be_bytes());
                }
                ScalarValue::Char(value) => {
                    bytes.push(6);
                    bytes.push(*value);
                }
                ScalarValue::String(value) => {
                    bytes.push(7);
                    push_len(bytes, value.len());
                    bytes.extend_from_slice(value);
                }
                ScalarValue::Time(value) => {
                    bytes.push(8);
                    bytes.extend_from_slice(&value.to_be_bytes());
                }
            }
        }
        PlcValue::Array(values) => {
            bytes.push(2);
            push_len(bytes, values.len());
            for value in values {
                encode_plc_value(bytes, value);
            }
        }
        PlcValue::Struct(fields) => {
            bytes.push(3);
            push_len(bytes, fields.len());
            for field in fields {
                bytes.extend_from_slice(&field.member_id.as_bytes());
                encode_plc_value(bytes, &field.value);
            }
        }
    }
}

const fn port_direction_tag(value: PortDirection) -> u8 {
    match value {
        PortDirection::Input => 1,
        PortDirection::Output => 2,
        PortDirection::ExecutionInput => 3,
        PortDirection::ExecutionOutput => 4,
    }
}

fn push_len(bytes: &mut Vec<u8>, value: usize) {
    push_u64(bytes, u64::try_from(value).unwrap_or(u64::MAX));
}

fn push_string(bytes: &mut Vec<u8>, value: &str) {
    push_len(bytes, value.len());
    bytes.extend_from_slice(value.as_bytes());
}

fn push_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn push_u128(bytes: &mut Vec<u8>, value: u128) {
    bytes.extend_from_slice(&value.to_be_bytes());
}
