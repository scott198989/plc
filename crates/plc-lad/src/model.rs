use alloc::{collections::BTreeMap, string::String, vec::Vec};

use plc_compiler::Hash32;
use plc_program::{
    BlockId, CallSiteId, CanonicalValue, DataType, InstanceOwner, InstancePath, InstructionCode,
    InstructionFormalId, InterfaceMemberId, StateKind, VariableRef,
};
use plc_types::{AggregateLimits, CanonicalType, PlcValue, PrimitiveType, ScalarValue};

use crate::{
    LAD_SCHEMA_VERSION, LadBranchId, LadBranchPathId, LadDocumentId, LadEdgeId, LadNetworkId,
    LadNodeId, LadOperandId, LadPortId, LadStateInstanceId, hash::CanonicalHasher,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContactMode {
    NormallyOpen,
    NormallyClosed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CoilMode {
    Normal,
    Negated,
    Set,
    Reset,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LadPowerPortDirection {
    Input,
    Output,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LadPowerPort {
    pub id: LadPortId,
    pub direction: LadPowerPortDirection,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LadOperand {
    Constant(CanonicalValue),
    Variable(VariableRef),
    Unresolved {
        spelling: String,
    },
    /// Preserved for repair. Free-form graphical expression evaluation is not
    /// an EDU-21 Core 1.0 capability and validation blocks it.
    Expression {
        source: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LadOperandRef {
    pub id: LadOperandId,
    pub value: LadOperand,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LadPinDirection {
    Input,
    Output,
    InOut,
    Activation,
    Status,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LadPortStatus {
    Active,
    Stale,
    Orphan,
}

/// Stable formal identity. Built-in instruction formals and user block
/// interface members live in separate identity domains and are never compared
/// by display name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LadFormalRef {
    Instruction(InstructionFormalId),
    BlockMember(InterfaceMemberId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LadPin {
    pub id: LadPortId,
    pub formal: Option<LadFormalRef>,
    pub name: String,
    pub direction: LadPinDirection,
    pub data_type: DataType,
    pub required: bool,
    pub status: LadPortStatus,
    pub binding: Option<LadOperandRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LadStateBinding {
    pub invocation: LadStateInstanceId,
    pub storage: VariableRef,
    pub kind: StateKind,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LadInstance {
    pub owner: InstanceOwner,
    pub path: InstancePath,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LadBox {
    pub instruction: InstructionCode,
    pub pins: BTreeMap<LadPortId, LadPin>,
    pub ordered_pin_ids: Vec<LadPortId>,
    pub state: Option<LadStateBinding>,
}

impl LadBox {
    #[must_use]
    pub fn from_pins(
        instruction: InstructionCode,
        pins: impl IntoIterator<Item = LadPin>,
        state: Option<LadStateBinding>,
    ) -> Self {
        let mut by_id = BTreeMap::new();
        let mut ordered_pin_ids = Vec::new();
        for pin in pins {
            ordered_pin_ids.push(pin.id);
            by_id.insert(pin.id, pin);
        }
        Self {
            instruction,
            pins: by_id,
            ordered_pin_ids,
            state,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LadCall {
    pub call_site: CallSiteId,
    pub instruction: InstructionCode,
    pub callee: BlockId,
    pub instance: Option<LadInstance>,
    pub pins: BTreeMap<LadPortId, LadPin>,
    pub ordered_pin_ids: Vec<LadPortId>,
}

impl LadCall {
    #[must_use]
    pub fn from_pins(
        call_site: CallSiteId,
        instruction: InstructionCode,
        callee: BlockId,
        instance: Option<LadInstance>,
        pins: impl IntoIterator<Item = LadPin>,
    ) -> Self {
        let mut by_id = BTreeMap::new();
        let mut ordered_pin_ids = Vec::new();
        for pin in pins {
            ordered_pin_ids.push(pin.id);
            by_id.insert(pin.id, pin);
        }
        Self {
            call_site,
            instruction,
            callee,
            instance,
            pins: by_id,
            ordered_pin_ids,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LadNodeKind {
    PowerSource,
    Contact {
        mode: ContactMode,
        operand: Option<LadOperandRef>,
    },
    Coil {
        mode: CoilMode,
        operand: Option<LadOperandRef>,
    },
    Box(LadBox),
    Call(LadCall),
    BranchSplit {
        branch: LadBranchId,
    },
    BranchJoin {
        branch: LadBranchId,
    },
    Return,
    /// Preserved source for a recognized but unavailable jump/label or future
    /// control node. It never lowers as another operation.
    UnsupportedControl {
        capability: String,
    },
    Unresolved {
        requested_name: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LadNode {
    pub id: LadNodeId,
    pub semantic_order: u32,
    pub kind: LadNodeKind,
    pub power_ports: BTreeMap<LadPortId, LadPowerPort>,
    pub ordered_power_port_ids: Vec<LadPortId>,
}

impl LadNode {
    #[must_use]
    pub fn from_power_ports(
        id: LadNodeId,
        semantic_order: u32,
        kind: LadNodeKind,
        power_ports: impl IntoIterator<Item = LadPowerPort>,
    ) -> Self {
        let mut by_id = BTreeMap::new();
        let mut ordered_power_port_ids = Vec::new();
        for port in power_ports {
            ordered_power_port_ids.push(port.id);
            by_id.insert(port.id, port);
        }
        Self {
            id,
            semantic_order,
            kind,
            power_ports: by_id,
            ordered_power_port_ids,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LadPowerEdge {
    pub id: LadEdgeId,
    pub source: LadPortId,
    pub target: LadPortId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LadBranchPath {
    pub id: LadBranchPathId,
    pub entry_edge: LadEdgeId,
    pub exit_edge: LadEdgeId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LadBranch {
    pub id: LadBranchId,
    pub split_node: LadNodeId,
    pub join_node: LadNodeId,
    pub paths: BTreeMap<LadBranchPathId, LadBranchPath>,
    pub ordered_path_ids: Vec<LadBranchPathId>,
}

impl LadBranch {
    #[must_use]
    pub fn from_paths(
        id: LadBranchId,
        split_node: LadNodeId,
        join_node: LadNodeId,
        paths: impl IntoIterator<Item = LadBranchPath>,
    ) -> Self {
        let mut by_id = BTreeMap::new();
        let mut ordered_path_ids = Vec::new();
        for path in paths {
            ordered_path_ids.push(path.id);
            by_id.insert(path.id, path);
        }
        Self {
            id,
            split_node,
            join_node,
            paths: by_id,
            ordered_path_ids,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LadNetwork {
    pub id: LadNetworkId,
    pub semantic_order: u32,
    pub nodes: BTreeMap<LadNodeId, LadNode>,
    pub ordered_node_ids: Vec<LadNodeId>,
    pub power_edges: BTreeMap<LadEdgeId, LadPowerEdge>,
    pub branches: BTreeMap<LadBranchId, LadBranch>,
    pub ordered_branch_ids: Vec<LadBranchId>,
}

impl LadNetwork {
    #[must_use]
    pub fn from_parts(
        id: LadNetworkId,
        semantic_order: u32,
        nodes: impl IntoIterator<Item = LadNode>,
        power_edges: impl IntoIterator<Item = LadPowerEdge>,
        branches: impl IntoIterator<Item = LadBranch>,
    ) -> Self {
        let mut node_map = BTreeMap::new();
        let mut ordered_node_ids = Vec::new();
        for node in nodes {
            ordered_node_ids.push(node.id);
            node_map.insert(node.id, node);
        }
        let power_edges = power_edges
            .into_iter()
            .map(|edge| (edge.id, edge))
            .collect();
        let mut branch_map = BTreeMap::new();
        let mut ordered_branch_ids = Vec::new();
        for branch in branches {
            ordered_branch_ids.push(branch.id);
            branch_map.insert(branch.id, branch);
        }
        Self {
            id,
            semantic_order,
            nodes: node_map,
            ordered_node_ids,
            power_edges,
            branches: branch_map,
            ordered_branch_ids,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LadDocument {
    pub schema_version: String,
    pub id: LadDocumentId,
    pub owner: BlockId,
    pub semantic_revision: u64,
    pub networks: BTreeMap<LadNetworkId, LadNetwork>,
    pub ordered_network_ids: Vec<LadNetworkId>,
}

impl LadDocument {
    #[must_use]
    pub fn new(
        id: LadDocumentId,
        owner: BlockId,
        networks: impl IntoIterator<Item = LadNetwork>,
    ) -> Self {
        let mut by_id = BTreeMap::new();
        let mut ordered_network_ids = Vec::new();
        for network in networks {
            ordered_network_ids.push(network.id);
            by_id.insert(network.id, network);
        }
        Self {
            schema_version: LAD_SCHEMA_VERSION.into(),
            id,
            owner,
            semantic_revision: 0,
            networks: by_id,
            ordered_network_ids,
        }
    }

    /// Fingerprints semantic graph data only. Coordinates, routes, zoom, and
    /// comments live in [`LadLayout`] and cannot influence this value.
    #[must_use]
    pub fn semantic_fingerprint(&self) -> Hash32 {
        let mut hasher = CanonicalHasher::new("PES-LAD-SEMANTIC-1");
        hasher.string(&self.schema_version);
        hasher.u128(self.id.get());
        hasher.u128(self.owner.get());
        hasher.u64(self.networks.len() as u64);
        hasher.u64(self.ordered_network_ids.len() as u64);
        for id in &self.ordered_network_ids {
            hasher.u128(id.get());
        }
        for (id, network) in &self.networks {
            hasher.u128(id.get());
            encode_network(&mut hasher, network);
        }
        hasher.finish()
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
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LadLayout {
    pub nodes: BTreeMap<LadNodeId, NodeLayout>,
    pub routes: BTreeMap<LadEdgeId, Vec<RoutePoint>>,
    pub node_comments: BTreeMap<LadNodeId, String>,
    pub network_comments: BTreeMap<LadNetworkId, String>,
    pub zoom_per_mille: u32,
}

fn encode_network(hasher: &mut CanonicalHasher, network: &LadNetwork) {
    hasher.u32(network.semantic_order);
    hasher.u64(network.ordered_node_ids.len() as u64);
    for id in &network.ordered_node_ids {
        hasher.u128(id.get());
    }
    hasher.u64(network.nodes.len() as u64);
    for (id, node) in &network.nodes {
        hasher.u128(id.get());
        encode_node(hasher, node);
    }
    hasher.u64(network.power_edges.len() as u64);
    for (id, edge) in &network.power_edges {
        hasher.u128(id.get());
        hasher.u128(edge.source.get());
        hasher.u128(edge.target.get());
    }
    hasher.u64(network.ordered_branch_ids.len() as u64);
    for id in &network.ordered_branch_ids {
        hasher.u128(id.get());
    }
    hasher.u64(network.branches.len() as u64);
    for (id, branch) in &network.branches {
        hasher.u128(id.get());
        hasher.u128(branch.split_node.get());
        hasher.u128(branch.join_node.get());
        hasher.u64(branch.ordered_path_ids.len() as u64);
        for path_id in &branch.ordered_path_ids {
            hasher.u128(path_id.get());
        }
        hasher.u64(branch.paths.len() as u64);
        for (path_id, path) in &branch.paths {
            hasher.u128(path_id.get());
            hasher.u128(path.entry_edge.get());
            hasher.u128(path.exit_edge.get());
        }
    }
}

fn encode_node(hasher: &mut CanonicalHasher, node: &LadNode) {
    hasher.u32(node.semantic_order);
    encode_node_kind(hasher, &node.kind);
    hasher.u64(node.ordered_power_port_ids.len() as u64);
    for id in &node.ordered_power_port_ids {
        hasher.u128(id.get());
    }
    hasher.u64(node.power_ports.len() as u64);
    for (id, port) in &node.power_ports {
        hasher.u128(id.get());
        hasher.u8(match port.direction {
            LadPowerPortDirection::Input => 1,
            LadPowerPortDirection::Output => 2,
        });
    }
}

fn encode_node_kind(hasher: &mut CanonicalHasher, kind: &LadNodeKind) {
    match kind {
        LadNodeKind::PowerSource => hasher.u8(1),
        LadNodeKind::Contact { mode, operand } => {
            hasher.u8(2);
            hasher.u8(match mode {
                ContactMode::NormallyOpen => 1,
                ContactMode::NormallyClosed => 2,
            });
            encode_optional_operand(hasher, operand.as_ref());
        }
        LadNodeKind::Coil { mode, operand } => {
            hasher.u8(3);
            hasher.u8(match mode {
                CoilMode::Normal => 1,
                CoilMode::Negated => 2,
                CoilMode::Set => 3,
                CoilMode::Reset => 4,
            });
            encode_optional_operand(hasher, operand.as_ref());
        }
        LadNodeKind::Box(value) => {
            hasher.u8(4);
            hasher.u16(value.instruction.0);
            encode_pins(hasher, &value.pins, &value.ordered_pin_ids);
            encode_state(hasher, value.state.as_ref());
        }
        LadNodeKind::Call(value) => {
            hasher.u8(5);
            hasher.u128(value.call_site.get());
            hasher.u16(value.instruction.0);
            hasher.u128(value.callee.get());
            encode_instance(hasher, value.instance.as_ref());
            encode_pins(hasher, &value.pins, &value.ordered_pin_ids);
        }
        LadNodeKind::BranchSplit { branch } => {
            hasher.u8(6);
            hasher.u128(branch.get());
        }
        LadNodeKind::BranchJoin { branch } => {
            hasher.u8(7);
            hasher.u128(branch.get());
        }
        LadNodeKind::Return => hasher.u8(8),
        LadNodeKind::UnsupportedControl { capability } => {
            hasher.u8(9);
            hasher.string(capability);
        }
        LadNodeKind::Unresolved { requested_name } => {
            hasher.u8(10);
            hasher.string(requested_name);
        }
    }
}

fn encode_pins(
    hasher: &mut CanonicalHasher,
    pins: &BTreeMap<LadPortId, LadPin>,
    ordered: &[LadPortId],
) {
    hasher.u64(ordered.len() as u64);
    for id in ordered {
        hasher.u128(id.get());
    }
    hasher.u64(pins.len() as u64);
    for (id, pin) in pins {
        hasher.u128(id.get());
        match pin.formal {
            Some(LadFormalRef::Instruction(formal)) => {
                hasher.bool(true);
                hasher.u8(1);
                hasher.u16(formal.0);
            }
            Some(LadFormalRef::BlockMember(formal)) => {
                hasher.bool(true);
                hasher.u8(2);
                hasher.u128(formal.get());
            }
            None => hasher.bool(false),
        }
        hasher.string(&pin.name);
        hasher.u8(match pin.direction {
            LadPinDirection::Input => 1,
            LadPinDirection::Output => 2,
            LadPinDirection::InOut => 3,
            LadPinDirection::Activation => 4,
            LadPinDirection::Status => 5,
        });
        encode_type(hasher, &pin.data_type);
        hasher.bool(pin.required);
        hasher.u8(match pin.status {
            LadPortStatus::Active => 1,
            LadPortStatus::Stale => 2,
            LadPortStatus::Orphan => 3,
        });
        encode_optional_operand(hasher, pin.binding.as_ref());
    }
}

fn encode_optional_operand(hasher: &mut CanonicalHasher, operand: Option<&LadOperandRef>) {
    match operand {
        Some(operand) => {
            hasher.bool(true);
            hasher.u128(operand.id.get());
            encode_operand(hasher, &operand.value);
        }
        None => hasher.bool(false),
    }
}

fn encode_operand(hasher: &mut CanonicalHasher, operand: &LadOperand) {
    match operand {
        LadOperand::Constant(value) => {
            hasher.u8(1);
            encode_value(hasher, value);
        }
        LadOperand::Variable(value) => {
            hasher.u8(2);
            encode_variable(hasher, value);
        }
        LadOperand::Unresolved { spelling } => {
            hasher.u8(3);
            hasher.string(spelling);
        }
        LadOperand::Expression { source } => {
            hasher.u8(4);
            hasher.string(source);
        }
    }
}

fn encode_state(hasher: &mut CanonicalHasher, state: Option<&LadStateBinding>) {
    match state {
        Some(state) => {
            hasher.bool(true);
            hasher.u128(state.invocation.get());
            encode_variable(hasher, &state.storage);
            hasher.u8(state_kind_tag(state.kind));
        }
        None => hasher.bool(false),
    }
}

fn encode_instance(hasher: &mut CanonicalHasher, instance: Option<&LadInstance>) {
    match instance {
        Some(instance) => {
            hasher.bool(true);
            match instance.owner {
                InstanceOwner::InstanceDb(block) => {
                    hasher.u8(1);
                    hasher.u128(block.get());
                }
                InstanceOwner::MultiInstance {
                    owner_fb,
                    static_member,
                } => {
                    hasher.u8(2);
                    hasher.u128(owner_fb.get());
                    hasher.u128(static_member.get());
                }
            }
            hasher.u128(instance.path.root_instance_db.get());
            hasher.u64(instance.path.multi_instance_slots.len() as u64);
            for slot in &instance.path.multi_instance_slots {
                hasher.u128(slot.get());
            }
        }
        None => hasher.bool(false),
    }
}

fn encode_variable(hasher: &mut CanonicalHasher, value: &VariableRef) {
    match value {
        VariableRef::CallerMember(member) => {
            hasher.u8(1);
            hasher.u128(member.get());
        }
        VariableRef::DataBlockMember { data_block, member } => {
            hasher.u8(2);
            hasher.u128(data_block.get());
            hasher.u128(member.get());
        }
    }
}

fn encode_type(hasher: &mut CanonicalHasher, value: &DataType) {
    match value {
        DataType::Bool => hasher.u8(1),
        DataType::Int => hasher.u8(2),
        DataType::DInt => hasher.u8(3),
        DataType::Real => hasher.u8(4),
        DataType::Time => hasher.u8(5),
        DataType::String { capacity } => {
            hasher.u8(6);
            hasher.u16(*capacity);
        }
        DataType::Named(name) => {
            hasher.u8(7);
            hasher.string(name);
        }
        DataType::BlockInstance(block) => {
            hasher.u8(8);
            hasher.u128(block.get());
        }
        DataType::InstructionState(kind) => {
            hasher.u8(9);
            hasher.u8(state_kind_tag(*kind));
        }
        DataType::SInt => hasher.u8(10),
        DataType::LInt => hasher.u8(11),
        DataType::USInt => hasher.u8(12),
        DataType::UInt => hasher.u8(13),
        DataType::UDInt => hasher.u8(14),
        DataType::ULInt => hasher.u8(15),
        DataType::Byte => hasher.u8(16),
        DataType::Word => hasher.u8(17),
        DataType::DWord => hasher.u8(18),
        DataType::LWord => hasher.u8(19),
        DataType::LReal => hasher.u8(20),
        DataType::Char => hasher.u8(21),
        DataType::Aggregate(data_type) => {
            hasher.u8(22);
            if let Ok(bytes) = data_type.canonical_bytes(AggregateLimits::edu21()) {
                hasher.bool(true);
                hasher.bytes(&bytes);
            } else {
                hasher.bool(false);
                encode_invalid_aggregate_type(hasher, data_type);
            }
        }
    }
}

fn encode_value(hasher: &mut CanonicalHasher, value: &CanonicalValue) {
    match value {
        CanonicalValue::Bool(value) => {
            hasher.u8(1);
            hasher.bool(*value);
        }
        CanonicalValue::Int(value) => {
            hasher.u8(2);
            hasher.i32(i32::from(*value));
        }
        CanonicalValue::DInt(value) => {
            hasher.u8(3);
            hasher.i32(*value);
        }
        CanonicalValue::RealBits(value) => {
            hasher.u8(4);
            hasher.u32(*value);
        }
        CanonicalValue::TimeMilliseconds(value) => {
            hasher.u8(5);
            hasher.i64(*value);
        }
        CanonicalValue::StringBytes(value) => {
            hasher.u8(6);
            hasher.bytes(value);
        }
        CanonicalValue::SInt(value) => {
            hasher.u8(7);
            hasher.i32(i32::from(*value));
        }
        CanonicalValue::LInt(value) => {
            hasher.u8(8);
            hasher.i64(*value);
        }
        CanonicalValue::USInt(value) => {
            hasher.u8(9);
            hasher.u8(*value);
        }
        CanonicalValue::UInt(value) => {
            hasher.u8(10);
            hasher.u16(*value);
        }
        CanonicalValue::UDInt(value) => {
            hasher.u8(11);
            hasher.u32(*value);
        }
        CanonicalValue::ULInt(value) => {
            hasher.u8(12);
            hasher.u64(*value);
        }
        CanonicalValue::Byte(value) => {
            hasher.u8(13);
            hasher.u8(*value);
        }
        CanonicalValue::Word(value) => {
            hasher.u8(14);
            hasher.u16(*value);
        }
        CanonicalValue::DWord(value) => {
            hasher.u8(15);
            hasher.u32(*value);
        }
        CanonicalValue::LWord(value) => {
            hasher.u8(16);
            hasher.u64(*value);
        }
        CanonicalValue::LRealBits(value) => {
            hasher.u8(17);
            hasher.u64(*value);
        }
        CanonicalValue::Char(value) => {
            hasher.u8(18);
            hasher.u8(*value);
        }
        CanonicalValue::Aggregate(value) => {
            hasher.u8(19);
            encode_plc_value(hasher, value);
        }
    }
}

fn encode_invalid_aggregate_type(hasher: &mut CanonicalHasher, data_type: &CanonicalType) {
    match data_type {
        CanonicalType::Primitive(primitive) => {
            hasher.u8(1);
            hasher.string(primitive.stable_id());
            if let PrimitiveType::String(capacity) = primitive {
                hasher.u8(*capacity);
            }
        }
        CanonicalType::Array {
            dimensions,
            element_type,
        } => {
            hasher.u8(2);
            hasher.u64(dimensions.len() as u64);
            for bound in dimensions {
                hasher.i32(bound.lower);
                hasher.i32(bound.upper);
            }
            encode_invalid_aggregate_type(hasher, element_type);
        }
        CanonicalType::AnonymousStruct { members } => {
            hasher.u8(3);
            encode_invalid_members(hasher, members);
        }
        CanonicalType::NamedStruct { id, members } => {
            hasher.u8(4);
            hasher.bytes(&id.as_bytes());
            encode_invalid_members(hasher, members);
        }
    }
}

fn encode_invalid_members(hasher: &mut CanonicalHasher, members: &[plc_types::StructMember]) {
    hasher.u64(members.len() as u64);
    for member in members {
        hasher.bytes(&member.id.as_bytes());
        hasher.string(&member.name);
        hasher.u32(member.declared_order);
        encode_invalid_aggregate_type(hasher, &member.data_type);
        match &member.reusable_default {
            Some(value) => {
                hasher.bool(true);
                encode_plc_value(hasher, value);
            }
            None => hasher.bool(false),
        }
    }
}

fn encode_plc_value(hasher: &mut CanonicalHasher, value: &PlcValue) {
    match value {
        PlcValue::Scalar(value) => {
            hasher.u8(1);
            hasher.string(value.data_type().stable_id());
            if let PrimitiveType::String(capacity) = value.data_type() {
                hasher.u8(capacity);
            }
            match value.value() {
                ScalarValue::Bool(value) => {
                    hasher.u8(1);
                    hasher.bool(*value);
                }
                ScalarValue::Signed(value) => {
                    hasher.u8(2);
                    hasher.i64(*value);
                }
                ScalarValue::Unsigned(value) | ScalarValue::BitString(value) => {
                    hasher.u8(3);
                    hasher.u64(*value);
                }
                ScalarValue::Real(value) => {
                    hasher.u8(4);
                    hasher.u32(value.bits());
                }
                ScalarValue::Lreal(value) => {
                    hasher.u8(5);
                    hasher.u64(value.bits());
                }
                ScalarValue::Char(value) => {
                    hasher.u8(6);
                    hasher.u8(*value);
                }
                ScalarValue::String(value) => {
                    hasher.u8(7);
                    hasher.bytes(value);
                }
                ScalarValue::Time(value) => {
                    hasher.u8(8);
                    hasher.i64(*value);
                }
            }
        }
        PlcValue::Array(values) => {
            hasher.u8(2);
            hasher.u64(values.len() as u64);
            for value in values {
                encode_plc_value(hasher, value);
            }
        }
        PlcValue::Struct(fields) => {
            hasher.u8(3);
            hasher.u64(fields.len() as u64);
            for field in fields {
                hasher.bytes(&field.member_id.as_bytes());
                encode_plc_value(hasher, &field.value);
            }
        }
    }
}

const fn state_kind_tag(value: StateKind) -> u8 {
    match value {
        StateKind::Edge => 1,
        StateKind::Timer => 2,
        StateKind::Counter => 3,
    }
}
