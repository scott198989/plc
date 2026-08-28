use alloc::{collections::BTreeMap, vec::Vec};

use plc_compiler::{
    BinaryOperator, IrBasicBlock, IrBasicBlockId, IrFunction, IrOperation, IrOperationId,
    IrOperationKind, IrTerminator, IrTerminatorKind, IrType, IrValue, IrValueId, ProbeId,
    SourceMapId, SourceMapSite, TYPED_IR_VERSION, TypedIrProgram, UnaryOperator,
};
use plc_program::{
    ADD, BOOL_AND, BOOL_NOT, BOOL_OR, BOOL_XOR, BlockId, COMPARE_EQ, COMPARE_GE, COMPARE_GT,
    COMPARE_LE, COMPARE_LT, COMPARE_NE, DIVIDE, DataType, InstructionCode, InterfaceMemberId,
    InterfaceRole, MODULO, MOVE, MULTIPLY, ProgramBlock, SUBTRACT,
};

use crate::{
    ActivationRole, ConnectionId, ConnectionKind, FbdDiagnostic, FbdDocument, FbdDocumentId,
    FbdNetwork, InstanceIdentity, NetworkId, NodeId, NodeKind, PortDirection, PortId,
    TypeAdapterError, data_type_to_ir_type, validate_fbd,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FbdLowerError {
    InvalidGraph(Vec<FbdDiagnostic>),
    OwnerMismatch {
        document: BlockId,
        block: BlockId,
    },
    NonExecutableOwner,
    IdSpaceExhausted,
    MissingScheduledNetwork(NetworkId),
    MissingNode(NodeId),
    MissingPortValue {
        node: NodeId,
        port: PortId,
    },
    InvalidNodeArity {
        node: NodeId,
    },
    IncompatibleNodeType {
        node: NodeId,
    },
    UnsupportedDataType {
        node: NodeId,
        error: TypeAdapterError,
    },
    UnknownMember {
        node: NodeId,
        member: InterfaceMemberId,
    },
    ReadOnlyStore {
        node: NodeId,
        member: InterfaceMemberId,
    },
    SharedIrOperationUnavailable {
        node: NodeId,
        instruction: InstructionCode,
    },
    ActivationControlUnavailable {
        node: NodeId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FbdSourceLocation {
    pub document: FbdDocumentId,
    pub owner: BlockId,
    pub network: NetworkId,
    pub node: Option<NodeId>,
    pub port: Option<PortId>,
    pub connections: Vec<ConnectionId>,
    pub call_target: Option<BlockId>,
    pub state_instance: Option<InstanceIdentity>,
    pub symbol: Option<InterfaceMemberId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FbdSourceMapEntry {
    pub id: SourceMapId,
    pub site: SourceMapSite,
    pub source: FbdSourceLocation,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FbdSourceMapTable {
    entries: BTreeMap<SourceMapId, FbdSourceMapEntry>,
}

impl FbdSourceMapTable {
    #[must_use]
    pub fn entries(&self) -> &BTreeMap<SourceMapId, FbdSourceMapEntry> {
        &self.entries
    }

    #[must_use]
    pub fn get(&self, id: SourceMapId) -> Option<&FbdSourceMapEntry> {
        self.entries.get(&id)
    }

    #[must_use]
    pub fn source_to_ir(&self, source: &FbdSourceLocation) -> Vec<SourceMapSite> {
        self.entries
            .values()
            .filter(|entry| &entry.source == source)
            .map(|entry| entry.site)
            .collect()
    }

    #[must_use]
    pub fn connection_to_ir(&self, connection: ConnectionId) -> Vec<SourceMapSite> {
        self.entries
            .values()
            .filter(|entry| entry.source.connections.contains(&connection))
            .map(|entry| entry.site)
            .collect()
    }

    #[must_use]
    pub fn node_to_ir(&self, node: NodeId) -> Vec<SourceMapSite> {
        self.entries
            .values()
            .filter(|entry| entry.source.node == Some(node))
            .map(|entry| entry.site)
            .collect()
    }

    #[must_use]
    pub fn port_to_ir(&self, port: PortId) -> Vec<SourceMapSite> {
        self.entries
            .values()
            .filter(|entry| entry.source.port == Some(port))
            .map(|entry| entry.site)
            .collect()
    }

    #[must_use]
    pub fn symbol_to_ir(&self, symbol: InterfaceMemberId) -> Vec<SourceMapSite> {
        self.entries
            .values()
            .filter(|entry| entry.source.symbol == Some(symbol))
            .map(|entry| entry.site)
            .collect()
    }

    #[must_use]
    pub fn call_to_ir(&self, target: BlockId) -> Vec<SourceMapSite> {
        self.entries
            .values()
            .filter(|entry| entry.source.call_target == Some(target))
            .map(|entry| entry.site)
            .collect()
    }

    #[must_use]
    pub fn state_to_ir(&self, instance: &InstanceIdentity) -> Vec<SourceMapSite> {
        self.entries
            .values()
            .filter(|entry| entry.source.state_instance.as_ref() == Some(instance))
            .map(|entry| entry.site)
            .collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FbdProbeKind {
    Constant,
    SymbolRead,
    SymbolWrite,
    ExpressionOutput,
    NetworkExit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FbdProbe {
    pub id: ProbeId,
    pub source_map: SourceMapId,
    pub site: SourceMapSite,
    pub kind: FbdProbeKind,
    pub value_type: Option<IrType>,
    pub source: FbdSourceLocation,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FbdProbeTable {
    entries: BTreeMap<ProbeId, FbdProbe>,
}

impl FbdProbeTable {
    #[must_use]
    pub fn entries(&self) -> &BTreeMap<ProbeId, FbdProbe> {
        &self.entries
    }

    #[must_use]
    pub fn get(&self, id: ProbeId) -> Option<&FbdProbe> {
        self.entries.get(&id)
    }

    #[must_use]
    pub fn source_to_probes(&self, source: &FbdSourceLocation) -> Vec<ProbeId> {
        self.entries
            .values()
            .filter(|probe| &probe.source == source)
            .map(|probe| probe.id)
            .collect()
    }

    #[must_use]
    pub fn node_to_probes(&self, node: NodeId) -> Vec<ProbeId> {
        self.entries
            .values()
            .filter(|probe| probe.source.node == Some(node))
            .map(|probe| probe.id)
            .collect()
    }

    #[must_use]
    pub fn port_to_probes(&self, port: PortId) -> Vec<ProbeId> {
        self.entries
            .values()
            .filter(|probe| probe.source.port == Some(port))
            .map(|probe| probe.id)
            .collect()
    }

    #[must_use]
    pub fn connection_to_probes(&self, connection: ConnectionId) -> Vec<ProbeId> {
        self.entries
            .values()
            .filter(|probe| probe.source.connections.contains(&connection))
            .map(|probe| probe.id)
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FbdLoweredProgram {
    pub ir: TypedIrProgram,
    pub source_maps: FbdSourceMapTable,
    pub probes: FbdProbeTable,
}

/// Lowers a valid FBD graph into the one shared compiler IR. Calls, stateful
/// instructions, and activation control return typed seam errors until those
/// operations exist in `plc-compiler`; no language-specific executor is used.
pub fn lower_fbd_to_ir(
    document: &FbdDocument,
    owner: &ProgramBlock,
) -> Result<FbdLoweredProgram, FbdLowerError> {
    if document.owner != owner.id {
        return Err(FbdLowerError::OwnerMismatch {
            document: document.owner,
            block: owner.id,
        });
    }
    if !owner.kind.is_executable() {
        return Err(FbdLowerError::NonExecutableOwner);
    }
    let validation = validate_fbd(document);
    if !validation.can_lower() {
        return Err(FbdLowerError::InvalidGraph(validation.diagnostics));
    }
    let mut context = LoweringContext::new(document, owner);
    let mut blocks = BTreeMap::new();
    for (network_index, network_id) in document.ordered_network_ids.iter().enumerate() {
        let network = &document.networks[network_id];
        let schedule = validation
            .schedules
            .get(network_id)
            .ok_or(FbdLowerError::MissingScheduledNetwork(*network_id))?;
        let basic_block = context.lower_network(network_index, network, schedule)?;
        blocks.insert(basic_block.id, basic_block);
    }
    let entry = IrBasicBlockId::new(1);
    let function = IrFunction {
        owner: document.owner,
        source_kind: owner.kind,
        entry,
        blocks,
    };
    let functions = [(document.owner, function)].into_iter().collect();
    Ok(FbdLoweredProgram {
        ir: TypedIrProgram::from_untrusted_parts(TYPED_IR_VERSION, functions),
        source_maps: FbdSourceMapTable {
            entries: context.source_maps,
        },
        probes: FbdProbeTable {
            entries: context.probes,
        },
    })
}

struct LoweringContext<'a> {
    document: &'a FbdDocument,
    owner: &'a ProgramBlock,
    next_operation: u32,
    next_value: u32,
    next_mapping: u32,
    source_maps: BTreeMap<SourceMapId, FbdSourceMapEntry>,
    probes: BTreeMap<ProbeId, FbdProbe>,
}

impl<'a> LoweringContext<'a> {
    const fn new(document: &'a FbdDocument, owner: &'a ProgramBlock) -> Self {
        Self {
            document,
            owner,
            next_operation: 1,
            next_value: 1,
            next_mapping: 1,
            source_maps: BTreeMap::new(),
            probes: BTreeMap::new(),
        }
    }

    fn lower_network(
        &mut self,
        network_index: usize,
        network: &FbdNetwork,
        schedule: &[NodeId],
    ) -> Result<IrBasicBlock, FbdLowerError> {
        let block_number = u32::try_from(network_index)
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or(FbdLowerError::IdSpaceExhausted)?;
        let basic_block = IrBasicBlockId::new(block_number);
        let mut operations = Vec::new();
        let mut port_values = BTreeMap::<PortId, IrValueId>::new();
        let incoming = incoming_data_sources(network);
        for node_id in schedule {
            let node = network
                .nodes
                .get(node_id)
                .ok_or(FbdLowerError::MissingNode(*node_id))?;
            if node
                .ports
                .values()
                .any(|port| port.activation != ActivationRole::None)
            {
                return Err(FbdLowerError::ActivationControlUnavailable { node: node.id });
            }
            self.lower_node(
                network,
                basic_block,
                node,
                &incoming,
                &mut port_values,
                &mut operations,
            )?;
        }
        let terminator_kind = if network_index + 1 < self.document.ordered_network_ids.len() {
            let target = u32::try_from(network_index)
                .ok()
                .and_then(|value| value.checked_add(2))
                .ok_or(FbdLowerError::IdSpaceExhausted)?;
            IrTerminatorKind::Jump(IrBasicBlockId::new(target))
        } else {
            IrTerminatorKind::Return
        };
        let source = self.location(network.id, None, None, None, None, None);
        let (source_map, probe) = self.map_site(
            SourceMapSite {
                function: self.document.owner,
                basic_block,
                operation: None,
            },
            source,
            FbdProbeKind::NetworkExit,
            None,
        )?;
        Ok(IrBasicBlock {
            id: basic_block,
            operations,
            terminator: IrTerminator {
                kind: terminator_kind,
                source_map,
                probe,
            },
        })
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn lower_node(
        &mut self,
        network: &FbdNetwork,
        basic_block: IrBasicBlockId,
        node: &crate::FbdNode,
        incoming: &BTreeMap<PortId, PortId>,
        port_values: &mut BTreeMap<PortId, IrValueId>,
        operations: &mut Vec<IrOperation>,
    ) -> Result<(), FbdLowerError> {
        let inputs = node
            .ordered_port_ids
            .iter()
            .filter_map(|id| node.ports.get(id))
            .filter(|port| port.direction == PortDirection::Input)
            .map(|port| {
                let source = incoming
                    .get(&port.id)
                    .ok_or(FbdLowerError::MissingPortValue {
                        node: node.id,
                        port: port.id,
                    })?;
                port_values
                    .get(source)
                    .copied()
                    .ok_or(FbdLowerError::MissingPortValue {
                        node: node.id,
                        port: *source,
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let outputs: Vec<_> = node
            .ordered_port_ids
            .iter()
            .filter_map(|id| node.ports.get(id))
            .filter(|port| port.direction == PortDirection::Output)
            .collect();

        match &node.kind {
            NodeKind::Constant { value } => {
                let [output] = outputs.as_slice() else {
                    return Err(FbdLowerError::InvalidNodeArity { node: node.id });
                };
                if !inputs.is_empty()
                    || !value.is_compatible_with(
                        output
                            .data_type
                            .as_ref()
                            .ok_or(FbdLowerError::IncompatibleNodeType { node: node.id })?,
                    )
                {
                    return Err(FbdLowerError::IncompatibleNodeType { node: node.id });
                }
                let result_type = Self::ir_type(node.id, output.data_type.as_ref())?;
                let result = self.push_value_operation(
                    network.id,
                    basic_block,
                    node,
                    Some(output.id),
                    IrOperationKind::Constant(value.clone()),
                    result_type,
                    FbdProbeKind::Constant,
                    None,
                    None,
                    operations,
                )?;
                port_values.insert(output.id, result);
            }
            NodeKind::LoadMember { member } => {
                let [output] = outputs.as_slice() else {
                    return Err(FbdLowerError::InvalidNodeArity { node: node.id });
                };
                if !inputs.is_empty() {
                    return Err(FbdLowerError::InvalidNodeArity { node: node.id });
                }
                let declared =
                    self.owner
                        .interface
                        .member(*member)
                        .ok_or(FbdLowerError::UnknownMember {
                            node: node.id,
                            member: *member,
                        })?;
                if output.data_type.as_ref() != Some(&declared.data_type) {
                    return Err(FbdLowerError::IncompatibleNodeType { node: node.id });
                }
                let result_type = Self::ir_type(node.id, output.data_type.as_ref())?;
                let result = self.push_value_operation(
                    network.id,
                    basic_block,
                    node,
                    Some(output.id),
                    IrOperationKind::LoadMember { member: *member },
                    result_type,
                    FbdProbeKind::SymbolRead,
                    Some(*member),
                    None,
                    operations,
                )?;
                port_values.insert(output.id, result);
            }
            NodeKind::StoreMember { member } => {
                let [input] = inputs.as_slice() else {
                    return Err(FbdLowerError::InvalidNodeArity { node: node.id });
                };
                if !outputs.is_empty() {
                    return Err(FbdLowerError::InvalidNodeArity { node: node.id });
                }
                let declared =
                    self.owner
                        .interface
                        .member(*member)
                        .ok_or(FbdLowerError::UnknownMember {
                            node: node.id,
                            member: *member,
                        })?;
                if matches!(
                    declared.role,
                    InterfaceRole::Input | InterfaceRole::Constant
                ) {
                    return Err(FbdLowerError::ReadOnlyStore {
                        node: node.id,
                        member: *member,
                    });
                }
                let input_port = node
                    .ordered_port_ids
                    .iter()
                    .filter_map(|id| node.ports.get(id))
                    .find(|port| port.direction == PortDirection::Input)
                    .ok_or(FbdLowerError::InvalidNodeArity { node: node.id })?;
                if input_port.data_type.as_ref() != Some(&declared.data_type) {
                    return Err(FbdLowerError::IncompatibleNodeType { node: node.id });
                }
                self.push_effect_operation(
                    network.id,
                    basic_block,
                    node,
                    IrOperationKind::StoreMember {
                        target: *member,
                        value: *input,
                    },
                    FbdProbeKind::SymbolWrite,
                    Some(*member),
                    operations,
                )?;
            }
            NodeKind::Instruction { code, instance } => {
                if instance.is_some() {
                    return Err(FbdLowerError::SharedIrOperationUnavailable {
                        node: node.id,
                        instruction: *code,
                    });
                }
                let [output] = outputs.as_slice() else {
                    return Err(FbdLowerError::InvalidNodeArity { node: node.id });
                };
                let result_type = Self::ir_type(node.id, output.data_type.as_ref())?;
                let input_types = node
                    .ordered_port_ids
                    .iter()
                    .filter_map(|id| node.ports.get(id))
                    .filter(|port| port.direction == PortDirection::Input)
                    .map(|port| {
                        port.data_type
                            .as_ref()
                            .ok_or(FbdLowerError::IncompatibleNodeType { node: node.id })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let kind = lower_instruction_kind(
                    node.id,
                    *code,
                    &inputs,
                    &input_types,
                    output
                        .data_type
                        .as_ref()
                        .ok_or(FbdLowerError::IncompatibleNodeType { node: node.id })?,
                    &result_type,
                )?;
                let result = self.push_value_operation(
                    network.id,
                    basic_block,
                    node,
                    Some(output.id),
                    kind,
                    result_type,
                    FbdProbeKind::ExpressionOutput,
                    None,
                    instance.clone(),
                    operations,
                )?;
                port_values.insert(output.id, result);
            }
            NodeKind::Call { code, .. } => {
                return Err(FbdLowerError::SharedIrOperationUnavailable {
                    node: node.id,
                    instruction: *code,
                });
            }
            NodeKind::Unresolved { .. } => unreachable!("validation rejects unresolved nodes"),
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn push_value_operation(
        &mut self,
        network: NetworkId,
        basic_block: IrBasicBlockId,
        node: &crate::FbdNode,
        port: Option<PortId>,
        kind: IrOperationKind,
        result_type: IrType,
        probe_kind: FbdProbeKind,
        symbol: Option<InterfaceMemberId>,
        state_instance: Option<InstanceIdentity>,
        operations: &mut Vec<IrOperation>,
    ) -> Result<IrValueId, FbdLowerError> {
        let operation_id = self.operation_id()?;
        let result_id = self.value_id()?;
        let site = SourceMapSite {
            function: self.document.owner,
            basic_block,
            operation: Some(operation_id),
        };
        let source = self.location(
            network,
            Some(node.id),
            port,
            call_target(&node.kind),
            state_instance,
            symbol,
        );
        let (source_map, probe) =
            self.map_site(site, source, probe_kind, Some(result_type.clone()))?;
        operations.push(IrOperation {
            id: operation_id,
            result: Some(IrValue {
                id: result_id,
                data_type: result_type,
            }),
            kind,
            source_map,
            probe,
        });
        Ok(result_id)
    }

    #[allow(clippy::too_many_arguments)]
    fn push_effect_operation(
        &mut self,
        network: NetworkId,
        basic_block: IrBasicBlockId,
        node: &crate::FbdNode,
        kind: IrOperationKind,
        probe_kind: FbdProbeKind,
        symbol: Option<InterfaceMemberId>,
        operations: &mut Vec<IrOperation>,
    ) -> Result<(), FbdLowerError> {
        let operation_id = self.operation_id()?;
        let site = SourceMapSite {
            function: self.document.owner,
            basic_block,
            operation: Some(operation_id),
        };
        let source = self.location(
            network,
            Some(node.id),
            None,
            call_target(&node.kind),
            state_identity(&node.kind),
            symbol,
        );
        let (source_map, probe) = self.map_site(site, source, probe_kind, None)?;
        operations.push(IrOperation {
            id: operation_id,
            result: None,
            kind,
            source_map,
            probe,
        });
        Ok(())
    }

    fn map_site(
        &mut self,
        site: SourceMapSite,
        source: FbdSourceLocation,
        kind: FbdProbeKind,
        value_type: Option<IrType>,
    ) -> Result<(SourceMapId, ProbeId), FbdLowerError> {
        let numeric = self.next_mapping;
        self.next_mapping = self
            .next_mapping
            .checked_add(1)
            .ok_or(FbdLowerError::IdSpaceExhausted)?;
        let source_map = SourceMapId::new(numeric);
        let probe = ProbeId::new(numeric);
        self.source_maps.insert(
            source_map,
            FbdSourceMapEntry {
                id: source_map,
                site,
                source: source.clone(),
            },
        );
        self.probes.insert(
            probe,
            FbdProbe {
                id: probe,
                source_map,
                site,
                kind,
                value_type,
                source,
            },
        );
        Ok((source_map, probe))
    }

    #[allow(clippy::too_many_arguments)]
    fn location(
        &self,
        network: NetworkId,
        node: Option<NodeId>,
        port: Option<PortId>,
        call_target: Option<BlockId>,
        state_instance: Option<InstanceIdentity>,
        symbol: Option<InterfaceMemberId>,
    ) -> FbdSourceLocation {
        let connections = node.map_or_else(Vec::new, |node_id| {
            self.document
                .networks
                .get(&network)
                .map_or_else(Vec::new, |value| {
                    let target_ports = value
                        .nodes
                        .get(&node_id)
                        .map(|node| {
                            node.ports
                                .keys()
                                .copied()
                                .collect::<alloc::collections::BTreeSet<_>>()
                        })
                        .unwrap_or_default();
                    value
                        .connections
                        .values()
                        .filter(|connection| target_ports.contains(&connection.target))
                        .map(|connection| connection.id)
                        .collect()
                })
        });
        FbdSourceLocation {
            document: self.document.id,
            owner: self.document.owner,
            network,
            node,
            port,
            connections,
            call_target,
            state_instance,
            symbol,
        }
    }

    fn operation_id(&mut self) -> Result<IrOperationId, FbdLowerError> {
        let value = self.next_operation;
        self.next_operation = self
            .next_operation
            .checked_add(1)
            .ok_or(FbdLowerError::IdSpaceExhausted)?;
        Ok(IrOperationId::new(value))
    }

    fn value_id(&mut self) -> Result<IrValueId, FbdLowerError> {
        let value = self.next_value;
        self.next_value = self
            .next_value
            .checked_add(1)
            .ok_or(FbdLowerError::IdSpaceExhausted)?;
        Ok(IrValueId::new(value))
    }

    fn ir_type(node: NodeId, data_type: Option<&DataType>) -> Result<IrType, FbdLowerError> {
        let data_type = data_type.ok_or(FbdLowerError::IncompatibleNodeType { node })?;
        data_type_to_ir_type(data_type)
            .map_err(|error| FbdLowerError::UnsupportedDataType { node, error })
    }
}

fn incoming_data_sources(network: &FbdNetwork) -> BTreeMap<PortId, PortId> {
    network
        .connections
        .values()
        .filter(|connection| connection.kind == ConnectionKind::Data)
        .map(|connection| (connection.target, connection.source))
        .collect()
}

fn lower_instruction_kind(
    node: NodeId,
    code: InstructionCode,
    inputs: &[IrValueId],
    input_types: &[&DataType],
    output_data_type: &DataType,
    output_type: &IrType,
) -> Result<IrOperationKind, FbdLowerError> {
    if code == BOOL_NOT {
        let [operand] = inputs else {
            return Err(FbdLowerError::InvalidNodeArity { node });
        };
        if input_types != [&DataType::Bool] || output_data_type != &DataType::Bool {
            return Err(FbdLowerError::IncompatibleNodeType { node });
        }
        return Ok(IrOperationKind::Unary {
            operator: UnaryOperator::Not,
            operand: *operand,
        });
    }
    if code == MOVE {
        let [source] = inputs else {
            return Err(FbdLowerError::InvalidNodeArity { node });
        };
        if input_types.len() != 1 || input_types[0] != output_data_type {
            return Err(FbdLowerError::IncompatibleNodeType { node });
        }
        return Ok(IrOperationKind::Convert {
            source: *source,
            destination: output_type.clone(),
        });
    }
    let [left, right] = inputs else {
        return Err(FbdLowerError::InvalidNodeArity { node });
    };
    if input_types.len() != 2 || input_types[0] != input_types[1] {
        return Err(FbdLowerError::IncompatibleNodeType { node });
    }
    let operator = if code == BOOL_AND {
        BinaryOperator::And
    } else if code == BOOL_OR {
        BinaryOperator::Or
    } else if code == BOOL_XOR {
        BinaryOperator::Xor
    } else if code == ADD {
        BinaryOperator::Add
    } else if code == SUBTRACT {
        BinaryOperator::Subtract
    } else if code == MULTIPLY {
        BinaryOperator::Multiply
    } else if code == DIVIDE {
        BinaryOperator::Divide
    } else if code == MODULO {
        BinaryOperator::Modulo
    } else if code == COMPARE_EQ {
        BinaryOperator::Equal
    } else if code == COMPARE_NE {
        BinaryOperator::NotEqual
    } else if code == COMPARE_LT {
        BinaryOperator::Less
    } else if code == COMPARE_LE {
        BinaryOperator::LessEqual
    } else if code == COMPARE_GT {
        BinaryOperator::Greater
    } else if code == COMPARE_GE {
        BinaryOperator::GreaterEqual
    } else {
        return Err(FbdLowerError::SharedIrOperationUnavailable {
            node,
            instruction: code,
        });
    };
    let expects_bool = matches!(
        operator,
        BinaryOperator::And | BinaryOperator::Xor | BinaryOperator::Or
    );
    let is_comparison = matches!(
        operator,
        BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::Less
            | BinaryOperator::LessEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterEqual
    );
    if expects_bool && (input_types[0] != &DataType::Bool || output_data_type != &DataType::Bool) {
        return Err(FbdLowerError::IncompatibleNodeType { node });
    }
    if is_comparison && output_data_type != &DataType::Bool {
        return Err(FbdLowerError::IncompatibleNodeType { node });
    }
    if !expects_bool && !is_comparison {
        let numeric = matches!(
            input_types[0],
            DataType::Int | DataType::DInt | DataType::Real
        );
        let modulo_valid = operator != BinaryOperator::Modulo
            || matches!(input_types[0], DataType::Int | DataType::DInt);
        if !numeric || !modulo_valid || input_types[0] != output_data_type {
            return Err(FbdLowerError::IncompatibleNodeType { node });
        }
    }
    Ok(IrOperationKind::Binary {
        operator,
        left: *left,
        right: *right,
    })
}

fn call_target(kind: &NodeKind) -> Option<BlockId> {
    match kind {
        NodeKind::Call { target, .. } => Some(*target),
        NodeKind::Constant { .. }
        | NodeKind::LoadMember { .. }
        | NodeKind::StoreMember { .. }
        | NodeKind::Instruction { .. }
        | NodeKind::Unresolved { .. } => None,
    }
}

fn state_identity(kind: &NodeKind) -> Option<InstanceIdentity> {
    match kind {
        NodeKind::Instruction { instance, .. } | NodeKind::Call { instance, .. } => {
            instance.clone()
        }
        NodeKind::Constant { .. }
        | NodeKind::LoadMember { .. }
        | NodeKind::StoreMember { .. }
        | NodeKind::Unresolved { .. } => None,
    }
}
