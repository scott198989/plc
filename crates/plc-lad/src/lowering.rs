use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec,
    vec::Vec,
};

use plc_compiler::{
    BinaryOperator, GraphSourceIds, Hash32, IrActivation, IrBasicBlock, IrBasicBlockId,
    IrBoundInput, IrDeclaredOutput, IrFormalRef, IrFunction, IrInstanceIdentity, IrOperation,
    IrOperationId, IrOperationKind, IrTerminator, IrTerminatorKind, IrType, IrValue, IrValueId,
    ProbeDefinition, ProbeId, ProbeKind, ProbeTable, SemanticNodeId, SourceAnchor, SourceLanguage,
    SourceMapEntry, SourceMapId, SourceMapSite, SourceMapTable, TYPED_IR_VERSION, TypedIrProgram,
    UnaryOperator, VerificationError, VerifiedIr, verify_typed_ir,
};
use plc_program::{
    BlockId, CALL_FB, CanonicalValue, ControllerProgram, DataType, InstructionActivationPolicy,
    InstructionCode, InterfaceMemberId, VariableRef, phase2_instruction_registry,
};

use crate::{
    CoilMode, ContactMode, LadDiagnostic, LadDocument, LadEdgeId, LadFormalRef, LadLimits,
    LadNetwork, LadNetworkId, LadNode, LadNodeId, LadNodeKind, LadOperand, LadOperandId,
    LadOperandRef, LadPin, LadPinDirection, LadPortId, LadStateInstanceId, LadValidationReport,
    validate_lad,
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SharedIrRequirement {
    /// Shared IR load/store operations currently address only the executing
    /// block's interface. A DB operand is retained and rejected, never aliased
    /// to a caller member with the same member identity.
    DataBlockStorage {
        node: LadNodeId,
        operand: LadOperandId,
        data_block: BlockId,
        member: InterfaceMemberId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedIrGap {
    pub requirements: Vec<SharedIrRequirement>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LadLowerError {
    InvalidGraph(Vec<LadDiagnostic>),
    OwnerMismatch {
        document: BlockId,
        block: BlockId,
    },
    NonExecutableOwner,
    MissingNetworkAnalysis(LadNetworkId),
    MissingNode {
        network: LadNetworkId,
        node: LadNodeId,
    },
    MissingPowerValue {
        network: LadNetworkId,
        node: LadNodeId,
    },
    InvalidOperand {
        network: LadNetworkId,
        node: LadNodeId,
    },
    InvalidPowerArity {
        network: LadNetworkId,
        node: LadNodeId,
    },
    InvalidFormal {
        network: LadNetworkId,
        node: LadNodeId,
        port: LadPortId,
    },
    MissingInstructionDefinition(InstructionCode),
    MissingInvocationOutput {
        network: LadNetworkId,
        node: LadNodeId,
        formal: IrFormalRef,
    },
    UnsupportedDataType {
        network: LadNetworkId,
        node: LadNodeId,
    },
    InvalidSourceAnchor,
    IdSpaceExhausted,
    SharedIrGap(SharedIrGap),
    SharedIrVerification(VerificationError),
}

/// Verifier-admitted LAD IR plus the exact shared source-map and probe tables
/// used for admission. No alternate LAD execution engine exists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LadIrArtifact {
    pub verified_ir: VerifiedIr,
    pub source_maps: SourceMapTable,
    pub probes: ProbeTable,
}

impl LadIrArtifact {
    #[must_use]
    pub const fn ir(&self) -> &TypedIrProgram {
        self.verified_ir.program()
    }

    #[must_use]
    pub const fn verification_hash(&self) -> Hash32 {
        self.verified_ir.verification_hash()
    }

    #[must_use]
    pub fn node_to_ir(&self, node: LadNodeId) -> Vec<SourceMapSite> {
        self.source_sites(|anchor| anchor.node_id == Some(node.get()))
    }

    #[must_use]
    pub fn edge_to_ir(&self, edge: LadEdgeId) -> Vec<SourceMapSite> {
        self.source_sites(|anchor| anchor.edge_id == Some(edge.get()))
    }

    #[must_use]
    pub fn operand_to_ir(&self, operand: LadOperandId) -> Vec<SourceMapSite> {
        self.source_sites(|anchor| anchor.operand_id == Some(operand.get()))
    }

    #[must_use]
    pub fn call_to_ir(&self, call_site: plc_program::CallSiteId) -> Vec<SourceMapSite> {
        self.source_sites(|anchor| anchor.call_site_id == Some(call_site.get()))
    }

    #[must_use]
    pub fn state_to_ir(&self, state: LadStateInstanceId) -> Vec<SourceMapSite> {
        self.source_sites(|anchor| anchor.state_instance_id == Some(state.get()))
    }

    #[must_use]
    pub fn edge_to_probes(&self, edge: LadEdgeId) -> Vec<ProbeId> {
        let sites: BTreeSet<_> = self.edge_to_ir(edge).into_iter().collect();
        self.probes
            .entries()
            .values()
            .filter(|probe| sites.contains(&probe.site))
            .map(|probe| probe.id)
            .collect()
    }

    fn source_sites(&self, predicate: impl Fn(&SourceAnchor) -> bool) -> Vec<SourceMapSite> {
        self.source_maps
            .entries()
            .values()
            .filter(|entry| entry.anchors.iter().any(&predicate))
            .map(|entry| entry.site)
            .collect()
    }
}

/// Lowers a valid LAD document into the one shared typed IR and independently
/// verifies every operation, formal, source anchor, and probe before returning.
///
/// Invalid authored source remains editable in the input document and returns
/// diagnostics without producing a [`LadIrArtifact`].
///
/// # Errors
///
/// Returns deterministic graph, binding, shared-capability, identity, mapping,
/// or independent IR-verification errors. No partially verified artifact is
/// returned.
pub fn lower_lad_to_ir(
    document: &LadDocument,
    program: &ControllerProgram,
    limits: LadLimits,
) -> Result<LadIrArtifact, LadLowerError> {
    let validation = validate_lad(document, program, limits);
    if !validation.can_lower() {
        return Err(LadLowerError::InvalidGraph(validation.diagnostics));
    }
    let owner = program
        .block(document.owner)
        .ok_or(LadLowerError::OwnerMismatch {
            document: document.owner,
            block: document.owner,
        })?;
    if owner.id != document.owner {
        return Err(LadLowerError::OwnerMismatch {
            document: document.owner,
            block: owner.id,
        });
    }
    if !owner.kind.is_executable() {
        return Err(LadLowerError::NonExecutableOwner);
    }
    let requirements = collect_shared_ir_gaps(document);
    if !requirements.is_empty() {
        return Err(LadLowerError::SharedIrGap(SharedIrGap { requirements }));
    }

    let mut context = LoweringContext::new(document, &validation);
    let mut blocks = BTreeMap::new();
    for (network_index, network_id) in document.ordered_network_ids.iter().enumerate() {
        let network = document
            .networks
            .get(network_id)
            .ok_or(LadLowerError::MissingNetworkAnalysis(*network_id))?;
        for block in context.lower_network(network_index, network)? {
            blocks.insert(block.id, block);
        }
    }
    let entry = IrBasicBlockId::new(1);
    let function = IrFunction {
        owner: document.owner,
        source_kind: owner.kind,
        entry,
        blocks,
    };
    let ir = TypedIrProgram::from_untrusted_parts(
        TYPED_IR_VERSION,
        [(document.owner, function)].into_iter().collect(),
    );
    let source_maps = SourceMapTable::from_untrusted_entries(context.source_maps);
    let probes = ProbeTable::from_untrusted_entries(context.probes);
    let verified_ir = verify_typed_ir(ir, &source_maps, &probes, program)
        .map_err(LadLowerError::SharedIrVerification)?;
    Ok(LadIrArtifact {
        verified_ir,
        source_maps,
        probes,
    })
}

fn collect_shared_ir_gaps(document: &LadDocument) -> Vec<SharedIrRequirement> {
    let mut requirements = Vec::new();
    for network in document.networks.values() {
        for node in network.nodes.values() {
            for operand in node_operands(node) {
                if let LadOperand::Variable(VariableRef::DataBlockMember { data_block, member }) =
                    operand.value
                {
                    requirements.push(SharedIrRequirement::DataBlockStorage {
                        node: node.id,
                        operand: operand.id,
                        data_block,
                        member,
                    });
                }
            }
        }
    }
    requirements.sort();
    requirements.dedup();
    requirements
}

#[derive(Clone)]
struct AnchorSpec {
    network: LadNetworkId,
    node: Option<LadNodeId>,
    port: Option<LadPortId>,
    edges: Vec<LadEdgeId>,
    operand: Option<LadOperandId>,
    call_site: Option<plc_program::CallSiteId>,
    state_instance: Option<LadStateInstanceId>,
}

struct LoweringContext<'a> {
    document: &'a LadDocument,
    validation: &'a LadValidationReport,
    source_revision_hash: Hash32,
    next_operation: u32,
    next_value: u32,
    next_mapping: u32,
    source_maps: BTreeMap<SourceMapId, SourceMapEntry>,
    probes: BTreeMap<ProbeId, ProbeDefinition>,
}

impl<'a> LoweringContext<'a> {
    fn new(document: &'a LadDocument, validation: &'a LadValidationReport) -> Self {
        Self {
            document,
            validation,
            source_revision_hash: document.semantic_fingerprint(),
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
        network: &LadNetwork,
    ) -> Result<Vec<IrBasicBlock>, LadLowerError> {
        let basic_block = Self::network_block_id(network_index)?;
        let analysis = self
            .validation
            .networks
            .get(&network.id)
            .ok_or(LadLowerError::MissingNetworkAnalysis(network.id))?;
        let mut operations = Vec::new();
        let mut port_values = BTreeMap::<LadPortId, IrValueId>::new();
        for node_id in &analysis.execution_order {
            let node = network
                .nodes
                .get(node_id)
                .ok_or(LadLowerError::MissingNode {
                    network: network.id,
                    node: *node_id,
                })?;
            self.lower_node(
                network,
                basic_block,
                node,
                &mut port_values,
                &mut operations,
            )?;
        }

        let return_node = analysis
            .terminals
            .iter()
            .find_map(|id| matches!(network.nodes[id].kind, LadNodeKind::Return).then_some(*id));
        if let Some(return_node) = return_node {
            self.lower_conditional_return(
                network_index,
                network,
                return_node,
                basic_block,
                operations,
                &port_values,
            )
        } else {
            let kind = if network_index + 1 == self.document.ordered_network_ids.len() {
                IrTerminatorKind::Return
            } else {
                IrTerminatorKind::Jump(Self::network_block_id(network_index + 1)?)
            };
            let site = SourceMapSite {
                function: self.document.owner,
                basic_block,
                operation: None,
            };
            let anchor = AnchorSpec {
                network: network.id,
                node: analysis.terminals.first().copied(),
                port: None,
                edges: Vec::new(),
                operand: None,
                call_site: None,
                state_instance: None,
            };
            let (source_map, probe) = self.map_site(site, vec![anchor], ProbeKind::Return, None)?;
            Ok(vec![IrBasicBlock {
                id: basic_block,
                operations,
                terminator: IrTerminator {
                    kind,
                    source_map,
                    probe,
                },
            }])
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_conditional_return(
        &mut self,
        network_index: usize,
        network: &LadNetwork,
        return_node: LadNodeId,
        basic_block: IrBasicBlockId,
        operations: Vec<IrOperation>,
        port_values: &BTreeMap<LadPortId, IrValueId>,
    ) -> Result<Vec<IrBasicBlock>, LadLowerError> {
        let node = &network.nodes[&return_node];
        let condition = incoming_value(network, node, port_values)?;
        let sink = self.return_block_id(network_index)?;
        let when_false = if network_index + 1 == self.document.ordered_network_ids.len() {
            sink
        } else {
            Self::network_block_id(network_index + 1)?
        };
        let branch_site = SourceMapSite {
            function: self.document.owner,
            basic_block,
            operation: None,
        };
        let anchor = Self::node_anchor(network, node, None, None, None, None);
        let (source_map, probe) = self.map_site(
            branch_site,
            vec![anchor.clone()],
            ProbeKind::Branch,
            Some(IrType::Bool),
        )?;
        let sink_site = SourceMapSite {
            function: self.document.owner,
            basic_block: sink,
            operation: None,
        };
        let (sink_map, sink_probe) =
            self.map_site(sink_site, vec![anchor], ProbeKind::Return, None)?;
        Ok(vec![
            IrBasicBlock {
                id: basic_block,
                operations,
                terminator: IrTerminator {
                    kind: IrTerminatorKind::Branch {
                        condition,
                        when_true: sink,
                        when_false,
                    },
                    source_map,
                    probe,
                },
            },
            IrBasicBlock {
                id: sink,
                operations: Vec::new(),
                terminator: IrTerminator {
                    kind: IrTerminatorKind::Return,
                    source_map: sink_map,
                    probe: sink_probe,
                },
            },
        ])
    }

    fn lower_node(
        &mut self,
        network: &LadNetwork,
        basic_block: IrBasicBlockId,
        node: &LadNode,
        port_values: &mut BTreeMap<LadPortId, IrValueId>,
        operations: &mut Vec<IrOperation>,
    ) -> Result<(), LadLowerError> {
        match &node.kind {
            LadNodeKind::PowerSource => {
                self.lower_power_source(network, basic_block, node, port_values, operations)
            }
            LadNodeKind::Contact { mode, operand } => self.lower_contact(
                network,
                basic_block,
                node,
                *mode,
                operand.as_ref(),
                port_values,
                operations,
            ),
            LadNodeKind::BranchSplit { .. } => {
                let input = incoming_value(network, node, port_values)?;
                for port in output_ports(node) {
                    port_values.insert(port, input);
                }
                Ok(())
            }
            LadNodeKind::BranchJoin { .. } => {
                self.lower_branch_join(network, basic_block, node, port_values, operations)
            }
            LadNodeKind::Coil { mode, operand } => self.lower_coil(
                network,
                basic_block,
                node,
                *mode,
                operand.as_ref(),
                port_values,
                operations,
            ),
            LadNodeKind::Box(value) => {
                self.lower_box(network, basic_block, node, value, port_values, operations)
            }
            LadNodeKind::Call(value) => {
                self.lower_call(network, basic_block, node, value, port_values, operations)
            }
            LadNodeKind::Return => Ok(()),
            LadNodeKind::UnsupportedControl { .. } | LadNodeKind::Unresolved { .. } => {
                unreachable!("validation rejects unavailable nodes")
            }
        }
    }

    fn lower_power_source(
        &mut self,
        network: &LadNetwork,
        basic_block: IrBasicBlockId,
        node: &LadNode,
        port_values: &mut BTreeMap<LadPortId, IrValueId>,
        operations: &mut Vec<IrOperation>,
    ) -> Result<(), LadLowerError> {
        let output = single_output(network.id, node)?;
        let anchor = Self::node_anchor(network, node, Some(output), None, None, None);
        let value = self.push_value(
            basic_block,
            IrOperationKind::Constant(CanonicalValue::Bool(true)),
            IrType::Bool,
            vec![anchor],
            ProbeKind::NetworkPower,
            operations,
        )?;
        port_values.insert(output, value);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_contact(
        &mut self,
        network: &LadNetwork,
        basic_block: IrBasicBlockId,
        node: &LadNode,
        mode: ContactMode,
        operand: Option<&LadOperandRef>,
        port_values: &mut BTreeMap<LadPortId, IrValueId>,
        operations: &mut Vec<IrOperation>,
    ) -> Result<(), LadLowerError> {
        let input = incoming_value(network, node, port_values)?;
        let output = single_output(network.id, node)?;
        let operand = operand.ok_or(LadLowerError::InvalidOperand {
            network: network.id,
            node: node.id,
        })?;
        let mut condition = self.lower_operand(
            network,
            basic_block,
            node,
            operand,
            &DataType::Bool,
            None,
            operations,
        )?;
        if mode == ContactMode::NormallyClosed {
            condition = self.push_value(
                basic_block,
                IrOperationKind::Unary {
                    operator: UnaryOperator::Not,
                    operand: condition,
                },
                IrType::Bool,
                vec![Self::node_anchor(
                    network,
                    node,
                    None,
                    Some(operand.id),
                    None,
                    None,
                )],
                ProbeKind::Expression,
                operations,
            )?;
        }
        let power = self.push_value(
            basic_block,
            IrOperationKind::Binary {
                operator: BinaryOperator::And,
                left: input,
                right: condition,
            },
            IrType::Bool,
            vec![Self::node_anchor(
                network,
                node,
                Some(output),
                Some(operand.id),
                None,
                None,
            )],
            ProbeKind::EdgeValue,
            operations,
        )?;
        port_values.insert(output, power);
        Ok(())
    }

    fn lower_branch_join(
        &mut self,
        network: &LadNetwork,
        basic_block: IrBasicBlockId,
        node: &LadNode,
        port_values: &mut BTreeMap<LadPortId, IrValueId>,
        operations: &mut Vec<IrOperation>,
    ) -> Result<(), LadLowerError> {
        let LadNodeKind::BranchJoin { branch } = node.kind else {
            unreachable!("dispatch ensures branch join")
        };
        let branch = network
            .branches
            .get(&branch)
            .ok_or(LadLowerError::InvalidPowerArity {
                network: network.id,
                node: node.id,
            })?;
        let mut path_values = Vec::new();
        for path_id in &branch.ordered_path_ids {
            let path = &branch.paths[path_id];
            let edge = &network.power_edges[&path.exit_edge];
            let value =
                port_values
                    .get(&edge.source)
                    .copied()
                    .ok_or(LadLowerError::MissingPowerValue {
                        network: network.id,
                        node: node.id,
                    })?;
            path_values.push((path.exit_edge, value));
        }
        let (first_edge, mut joined) =
            path_values
                .first()
                .copied()
                .ok_or(LadLowerError::InvalidPowerArity {
                    network: network.id,
                    node: node.id,
                })?;
        let mut previous_edge = first_edge;
        for (edge, value) in path_values.into_iter().skip(1) {
            let mut anchor = Self::node_anchor(network, node, None, None, None, None);
            anchor.edges = vec![previous_edge, edge];
            joined = self.push_value(
                basic_block,
                IrOperationKind::Binary {
                    operator: BinaryOperator::Or,
                    left: joined,
                    right: value,
                },
                IrType::Bool,
                vec![anchor],
                ProbeKind::EdgeValue,
                operations,
            )?;
            previous_edge = edge;
        }
        port_values.insert(single_output(network.id, node)?, joined);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_coil(
        &mut self,
        network: &LadNetwork,
        basic_block: IrBasicBlockId,
        node: &LadNode,
        mode: CoilMode,
        operand: Option<&LadOperandRef>,
        port_values: &BTreeMap<LadPortId, IrValueId>,
        operations: &mut Vec<IrOperation>,
    ) -> Result<(), LadLowerError> {
        let mut power = incoming_value(network, node, port_values)?;
        let operand = operand.ok_or(LadLowerError::InvalidOperand {
            network: network.id,
            node: node.id,
        })?;
        match mode {
            CoilMode::Normal => {}
            CoilMode::Negated => {
                power = self.bool_not(network, basic_block, node, power, operations)?;
            }
            CoilMode::Set => {
                let current = self.lower_operand(
                    network,
                    basic_block,
                    node,
                    operand,
                    &DataType::Bool,
                    None,
                    operations,
                )?;
                power = self.bool_binary(
                    network,
                    basic_block,
                    node,
                    BinaryOperator::Or,
                    current,
                    power,
                    operations,
                )?;
            }
            CoilMode::Reset => {
                let current = self.lower_operand(
                    network,
                    basic_block,
                    node,
                    operand,
                    &DataType::Bool,
                    None,
                    operations,
                )?;
                let inverse = self.bool_not(network, basic_block, node, power, operations)?;
                power = self.bool_binary(
                    network,
                    basic_block,
                    node,
                    BinaryOperator::And,
                    current,
                    inverse,
                    operations,
                )?;
            }
        }
        self.store_operand(network, basic_block, node, operand, power, None, operations)?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn lower_box(
        &mut self,
        network: &LadNetwork,
        basic_block: IrBasicBlockId,
        node: &LadNode,
        value: &crate::LadBox,
        port_values: &mut BTreeMap<LadPortId, IrValueId>,
        operations: &mut Vec<IrOperation>,
    ) -> Result<(), LadLowerError> {
        let definition = phase2_instruction_registry()
            .lookup(value.instruction)
            .ok_or(LadLowerError::MissingInstructionDefinition(
                value.instruction,
            ))?;
        let input_power = incoming_value(network, node, port_values)?;
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        let mut output_pins = BTreeMap::<IrFormalRef, &LadPin>::new();
        for pin in value.pins.values() {
            let Some(LadFormalRef::Instruction(formal)) = pin.formal else {
                return Err(LadLowerError::InvalidFormal {
                    network: network.id,
                    node: node.id,
                    port: pin.id,
                });
            };
            let formal_ref = IrFormalRef::Instruction(formal);
            match pin.direction {
                LadPinDirection::Input => inputs.push(IrBoundInput {
                    formal: formal_ref,
                    value: self.lower_pin_operand(
                        network,
                        basic_block,
                        node,
                        pin,
                        None,
                        operations,
                    )?,
                }),
                LadPinDirection::InOut => {
                    inputs.push(IrBoundInput {
                        formal: formal_ref,
                        value: self.lower_pin_operand(
                            network,
                            basic_block,
                            node,
                            pin,
                            None,
                            operations,
                        )?,
                    });
                    outputs.push(IrDeclaredOutput {
                        formal: formal_ref,
                        data_type: Self::pin_ir_type(network, node, pin)?,
                    });
                    output_pins.insert(formal_ref, pin);
                }
                LadPinDirection::Output => {
                    outputs.push(IrDeclaredOutput {
                        formal: formal_ref,
                        data_type: Self::pin_ir_type(network, node, pin)?,
                    });
                    output_pins.insert(formal_ref, pin);
                }
                LadPinDirection::Status => {
                    output_pins.insert(formal_ref, pin);
                }
                LadPinDirection::Activation => {}
            }
        }
        inputs.sort_by_key(|binding| binding.formal);
        let (activation, status_formal) =
            activation_contract(definition.activation, input_power, &mut outputs);
        outputs.sort_by_key(|declaration| declaration.formal);
        let instance = value
            .state
            .as_ref()
            .map(|state| IrInstanceIdentity::Instruction {
                stable_id: state.invocation.get(),
                kind: state.kind,
            });
        let state_id = value.state.as_ref().map(|state| state.invocation);
        let invocation = self.push_effect(
            basic_block,
            IrOperationKind::InvokeInstruction {
                instruction: value.instruction,
                inputs,
                outputs: outputs.clone(),
                instance,
                activation,
            },
            vec![Self::node_anchor(network, node, None, None, None, state_id)],
            if state_id.is_some() {
                ProbeKind::State
            } else {
                ProbeKind::Expression
            },
            operations,
        )?;
        let projected = self.project_invocation_outputs(
            network,
            basic_block,
            node,
            invocation,
            &outputs,
            &output_pins,
            None,
            state_id,
            operations,
        )?;
        self.store_projected_pins(
            network,
            basic_block,
            node,
            &output_pins,
            &projected,
            None,
            operations,
        )?;
        let power_output = if let Some(status) = status_formal {
            *projected.get(&IrFormalRef::Instruction(status)).ok_or(
                LadLowerError::MissingInvocationOutput {
                    network: network.id,
                    node: node.id,
                    formal: IrFormalRef::Instruction(status),
                },
            )?
        } else {
            input_power
        };
        port_values.insert(single_output(network.id, node)?, power_output);
        Ok(())
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn lower_call(
        &mut self,
        network: &LadNetwork,
        basic_block: IrBasicBlockId,
        node: &LadNode,
        value: &crate::LadCall,
        port_values: &mut BTreeMap<LadPortId, IrValueId>,
        operations: &mut Vec<IrOperation>,
    ) -> Result<(), LadLowerError> {
        let definition = phase2_instruction_registry()
            .lookup(value.instruction)
            .ok_or(LadLowerError::MissingInstructionDefinition(
                value.instruction,
            ))?;
        let input_power = incoming_value(network, node, port_values)?;
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        let mut output_pins = BTreeMap::<IrFormalRef, &LadPin>::new();
        for pin in value.pins.values() {
            match (pin.formal, pin.direction) {
                (Some(LadFormalRef::BlockMember(formal)), LadPinDirection::Input) => {
                    inputs.push(IrBoundInput {
                        formal: IrFormalRef::BlockMember(formal),
                        value: self.lower_pin_operand(
                            network,
                            basic_block,
                            node,
                            pin,
                            Some(value.call_site),
                            operations,
                        )?,
                    });
                }
                (Some(LadFormalRef::BlockMember(formal)), LadPinDirection::InOut) => {
                    let formal_ref = IrFormalRef::BlockMember(formal);
                    inputs.push(IrBoundInput {
                        formal: formal_ref,
                        value: self.lower_pin_operand(
                            network,
                            basic_block,
                            node,
                            pin,
                            Some(value.call_site),
                            operations,
                        )?,
                    });
                    outputs.push(IrDeclaredOutput {
                        formal: formal_ref,
                        data_type: Self::pin_ir_type(network, node, pin)?,
                    });
                    output_pins.insert(formal_ref, pin);
                }
                (Some(LadFormalRef::BlockMember(formal)), LadPinDirection::Output) => {
                    let formal_ref = IrFormalRef::BlockMember(formal);
                    outputs.push(IrDeclaredOutput {
                        formal: formal_ref,
                        data_type: Self::pin_ir_type(network, node, pin)?,
                    });
                    output_pins.insert(formal_ref, pin);
                }
                (Some(LadFormalRef::Instruction(formal)), LadPinDirection::Status) => {
                    output_pins.insert(IrFormalRef::Instruction(formal), pin);
                }
                (Some(LadFormalRef::Instruction(_)), LadPinDirection::Activation) => {}
                _ => {
                    return Err(LadLowerError::InvalidFormal {
                        network: network.id,
                        node: node.id,
                        port: pin.id,
                    });
                }
            }
        }
        inputs.sort_by_key(|binding| binding.formal);
        let (activation, status_formal) =
            activation_contract(definition.activation, input_power, &mut outputs);
        outputs.sort_by_key(|declaration| declaration.formal);
        let instance = if value.instruction == CALL_FB {
            value
                .instance
                .as_ref()
                .map(|instance| IrInstanceIdentity::FunctionBlock(instance.path.clone()))
        } else {
            None
        };
        let invocation = self.push_effect(
            basic_block,
            IrOperationKind::CallBlock {
                call_instruction: value.instruction,
                target: value.callee,
                inputs,
                outputs: outputs.clone(),
                instance,
                activation,
            },
            vec![Self::node_anchor(
                network,
                node,
                None,
                None,
                Some(value.call_site),
                None,
            )],
            ProbeKind::Call,
            operations,
        )?;
        let projected = self.project_invocation_outputs(
            network,
            basic_block,
            node,
            invocation,
            &outputs,
            &output_pins,
            Some(value.call_site),
            None,
            operations,
        )?;
        self.store_projected_pins(
            network,
            basic_block,
            node,
            &output_pins,
            &projected,
            Some(value.call_site),
            operations,
        )?;
        let power_output = if let Some(status) = status_formal {
            *projected.get(&IrFormalRef::Instruction(status)).ok_or(
                LadLowerError::MissingInvocationOutput {
                    network: network.id,
                    node: node.id,
                    formal: IrFormalRef::Instruction(status),
                },
            )?
        } else {
            input_power
        };
        port_values.insert(single_output(network.id, node)?, power_output);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn project_invocation_outputs(
        &mut self,
        network: &LadNetwork,
        basic_block: IrBasicBlockId,
        node: &LadNode,
        invocation: IrOperationId,
        outputs: &[IrDeclaredOutput],
        output_pins: &BTreeMap<IrFormalRef, &LadPin>,
        call_site: Option<plc_program::CallSiteId>,
        state_instance: Option<LadStateInstanceId>,
        operations: &mut Vec<IrOperation>,
    ) -> Result<BTreeMap<IrFormalRef, IrValueId>, LadLowerError> {
        let mut projected = BTreeMap::new();
        for output in outputs {
            let pin = output_pins.get(&output.formal).copied();
            let port = pin.map(|pin| pin.id).or_else(|| {
                matches!(output.formal, IrFormalRef::Instruction(_))
                    .then(|| single_output(network.id, node).ok())
                    .flatten()
            });
            let operand = pin
                .and_then(|pin| pin.binding.as_ref())
                .map(|value| value.id);
            let value = self.push_value(
                basic_block,
                IrOperationKind::InvocationOutput {
                    invocation,
                    formal: output.formal,
                },
                output.data_type.clone(),
                vec![Self::node_anchor(
                    network,
                    node,
                    port,
                    operand,
                    call_site,
                    state_instance,
                )],
                ProbeKind::PortValue,
                operations,
            )?;
            projected.insert(output.formal, value);
        }
        Ok(projected)
    }

    #[allow(clippy::too_many_arguments)]
    fn store_projected_pins(
        &mut self,
        network: &LadNetwork,
        basic_block: IrBasicBlockId,
        node: &LadNode,
        output_pins: &BTreeMap<IrFormalRef, &LadPin>,
        projected: &BTreeMap<IrFormalRef, IrValueId>,
        call_site: Option<plc_program::CallSiteId>,
        operations: &mut Vec<IrOperation>,
    ) -> Result<(), LadLowerError> {
        for (formal, pin) in output_pins {
            if pin.direction == LadPinDirection::Status {
                continue;
            }
            let Some(operand) = pin.binding.as_ref() else {
                continue;
            };
            let value =
                projected
                    .get(formal)
                    .copied()
                    .ok_or(LadLowerError::MissingInvocationOutput {
                        network: network.id,
                        node: node.id,
                        formal: *formal,
                    })?;
            self.store_operand(
                network,
                basic_block,
                node,
                operand,
                value,
                call_site,
                operations,
            )?;
        }
        Ok(())
    }

    fn lower_pin_operand(
        &mut self,
        network: &LadNetwork,
        basic_block: IrBasicBlockId,
        node: &LadNode,
        pin: &LadPin,
        call_site: Option<plc_program::CallSiteId>,
        operations: &mut Vec<IrOperation>,
    ) -> Result<IrValueId, LadLowerError> {
        let operand = pin.binding.as_ref().ok_or(LadLowerError::InvalidOperand {
            network: network.id,
            node: node.id,
        })?;
        self.lower_operand(
            network,
            basic_block,
            node,
            operand,
            &pin.data_type,
            call_site,
            operations,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_operand(
        &mut self,
        network: &LadNetwork,
        basic_block: IrBasicBlockId,
        node: &LadNode,
        operand: &LadOperandRef,
        expected: &DataType,
        call_site: Option<plc_program::CallSiteId>,
        operations: &mut Vec<IrOperation>,
    ) -> Result<IrValueId, LadLowerError> {
        let (kind, probe_kind) = match &operand.value {
            LadOperand::Constant(value) => (
                IrOperationKind::Constant(value.clone()),
                ProbeKind::Constant,
            ),
            LadOperand::Variable(VariableRef::CallerMember(member)) => (
                IrOperationKind::LoadMember { member: *member },
                ProbeKind::StorageRead,
            ),
            LadOperand::Variable(VariableRef::DataBlockMember { .. })
            | LadOperand::Unresolved { .. }
            | LadOperand::Expression { .. } => {
                return Err(LadLowerError::InvalidOperand {
                    network: network.id,
                    node: node.id,
                });
            }
        };
        let data_type = ir_type(expected).ok_or(LadLowerError::UnsupportedDataType {
            network: network.id,
            node: node.id,
        })?;
        self.push_value(
            basic_block,
            kind,
            data_type,
            vec![Self::node_anchor(
                network,
                node,
                None,
                Some(operand.id),
                call_site,
                None,
            )],
            probe_kind,
            operations,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn store_operand(
        &mut self,
        network: &LadNetwork,
        basic_block: IrBasicBlockId,
        node: &LadNode,
        operand: &LadOperandRef,
        value: IrValueId,
        call_site: Option<plc_program::CallSiteId>,
        operations: &mut Vec<IrOperation>,
    ) -> Result<IrOperationId, LadLowerError> {
        let LadOperand::Variable(VariableRef::CallerMember(member)) = operand.value else {
            return Err(LadLowerError::InvalidOperand {
                network: network.id,
                node: node.id,
            });
        };
        self.push_effect(
            basic_block,
            IrOperationKind::StoreMember {
                target: member,
                value,
            },
            vec![Self::node_anchor(
                network,
                node,
                None,
                Some(operand.id),
                call_site,
                None,
            )],
            ProbeKind::StorageWrite,
            operations,
        )
    }

    fn bool_not(
        &mut self,
        network: &LadNetwork,
        basic_block: IrBasicBlockId,
        node: &LadNode,
        operand: IrValueId,
        operations: &mut Vec<IrOperation>,
    ) -> Result<IrValueId, LadLowerError> {
        self.push_value(
            basic_block,
            IrOperationKind::Unary {
                operator: UnaryOperator::Not,
                operand,
            },
            IrType::Bool,
            vec![Self::node_anchor(network, node, None, None, None, None)],
            ProbeKind::Expression,
            operations,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn bool_binary(
        &mut self,
        network: &LadNetwork,
        basic_block: IrBasicBlockId,
        node: &LadNode,
        operator: BinaryOperator,
        left: IrValueId,
        right: IrValueId,
        operations: &mut Vec<IrOperation>,
    ) -> Result<IrValueId, LadLowerError> {
        self.push_value(
            basic_block,
            IrOperationKind::Binary {
                operator,
                left,
                right,
            },
            IrType::Bool,
            vec![Self::node_anchor(network, node, None, None, None, None)],
            ProbeKind::Expression,
            operations,
        )
    }

    fn pin_ir_type(
        network: &LadNetwork,
        node: &LadNode,
        pin: &LadPin,
    ) -> Result<IrType, LadLowerError> {
        ir_type(&pin.data_type).ok_or(LadLowerError::UnsupportedDataType {
            network: network.id,
            node: node.id,
        })
    }

    fn push_value(
        &mut self,
        basic_block: IrBasicBlockId,
        kind: IrOperationKind,
        data_type: IrType,
        anchors: Vec<AnchorSpec>,
        probe_kind: ProbeKind,
        operations: &mut Vec<IrOperation>,
    ) -> Result<IrValueId, LadLowerError> {
        let operation = self.operation_id()?;
        let value = self.value_id()?;
        let site = SourceMapSite {
            function: self.document.owner,
            basic_block,
            operation: Some(operation),
        };
        let (source_map, probe) =
            self.map_site(site, anchors, probe_kind, Some(data_type.clone()))?;
        operations.push(IrOperation {
            id: operation,
            result: Some(IrValue {
                id: value,
                data_type,
            }),
            kind,
            source_map,
            probe,
        });
        Ok(value)
    }

    fn push_effect(
        &mut self,
        basic_block: IrBasicBlockId,
        kind: IrOperationKind,
        anchors: Vec<AnchorSpec>,
        probe_kind: ProbeKind,
        operations: &mut Vec<IrOperation>,
    ) -> Result<IrOperationId, LadLowerError> {
        let operation = self.operation_id()?;
        let site = SourceMapSite {
            function: self.document.owner,
            basic_block,
            operation: Some(operation),
        };
        let (source_map, probe) = self.map_site(site, anchors, probe_kind, None)?;
        operations.push(IrOperation {
            id: operation,
            result: None,
            kind,
            source_map,
            probe,
        });
        Ok(operation)
    }

    fn map_site(
        &mut self,
        site: SourceMapSite,
        specs: Vec<AnchorSpec>,
        kind: ProbeKind,
        value_type: Option<IrType>,
    ) -> Result<(SourceMapId, ProbeId), LadLowerError> {
        let numeric = self.next_mapping;
        self.next_mapping = self
            .next_mapping
            .checked_add(1)
            .ok_or(LadLowerError::IdSpaceExhausted)?;
        let source_map = SourceMapId::new(numeric);
        let probe = ProbeId::new(numeric);
        let mut anchors = Vec::new();
        for spec in specs {
            if spec.edges.is_empty() {
                anchors.push(self.shared_anchor(numeric, &spec, None)?);
            } else {
                for edge in &spec.edges {
                    anchors.push(self.shared_anchor(numeric, &spec, Some(*edge))?);
                }
            }
        }
        anchors.sort();
        anchors.dedup();
        if anchors.is_empty() {
            return Err(LadLowerError::InvalidSourceAnchor);
        }
        self.source_maps.insert(
            source_map,
            SourceMapEntry {
                id: source_map,
                site,
                anchors,
                compiler_generated: false,
            },
        );
        self.probes.insert(
            probe,
            ProbeDefinition {
                id: probe,
                site,
                kind,
                value_type,
                source_map,
            },
        );
        Ok((source_map, probe))
    }

    fn shared_anchor(
        &self,
        semantic_node: u32,
        spec: &AnchorSpec,
        edge: Option<LadEdgeId>,
    ) -> Result<SourceAnchor, LadLowerError> {
        SourceAnchor::graph(
            self.document.owner,
            self.source_revision_hash,
            SourceLanguage::Lad,
            SemanticNodeId::new(semantic_node),
            GraphSourceIds {
                network_id: Some(spec.network.get()),
                node_id: spec.node.map(LadNodeId::get),
                port_id: spec.port.map(LadPortId::get),
                edge_id: edge.map(LadEdgeId::get),
                operand_id: spec.operand.map(LadOperandId::get),
                call_site_id: spec.call_site.map(plc_program::CallSiteId::get),
                state_instance_id: spec.state_instance.map(LadStateInstanceId::get),
            },
        )
        .ok_or(LadLowerError::InvalidSourceAnchor)
    }

    fn node_anchor(
        network: &LadNetwork,
        node: &LadNode,
        port: Option<LadPortId>,
        operand: Option<LadOperandId>,
        call_site: Option<plc_program::CallSiteId>,
        state_instance: Option<LadStateInstanceId>,
    ) -> AnchorSpec {
        let edges = network
            .power_edges
            .values()
            .filter(|edge| {
                node.power_ports.contains_key(&edge.source)
                    || node.power_ports.contains_key(&edge.target)
            })
            .map(|edge| edge.id)
            .collect();
        AnchorSpec {
            network: network.id,
            node: Some(node.id),
            port,
            edges,
            operand,
            call_site,
            state_instance,
        }
    }

    fn network_block_id(index: usize) -> Result<IrBasicBlockId, LadLowerError> {
        u32::try_from(index)
            .ok()
            .and_then(|value| value.checked_add(1))
            .map(IrBasicBlockId::new)
            .ok_or(LadLowerError::IdSpaceExhausted)
    }

    fn return_block_id(&self, index: usize) -> Result<IrBasicBlockId, LadLowerError> {
        let networks = u32::try_from(self.document.ordered_network_ids.len())
            .map_err(|_| LadLowerError::IdSpaceExhausted)?;
        u32::try_from(index)
            .ok()
            .and_then(|value| value.checked_add(1))
            .and_then(|value| networks.checked_add(value))
            .map(IrBasicBlockId::new)
            .ok_or(LadLowerError::IdSpaceExhausted)
    }

    fn operation_id(&mut self) -> Result<IrOperationId, LadLowerError> {
        let value = self.next_operation;
        self.next_operation = self
            .next_operation
            .checked_add(1)
            .ok_or(LadLowerError::IdSpaceExhausted)?;
        Ok(IrOperationId::new(value))
    }

    fn value_id(&mut self) -> Result<IrValueId, LadLowerError> {
        let value = self.next_value;
        self.next_value = self
            .next_value
            .checked_add(1)
            .ok_or(LadLowerError::IdSpaceExhausted)?;
        Ok(IrValueId::new(value))
    }
}

fn activation_contract(
    policy: InstructionActivationPolicy,
    enable_value: IrValueId,
    outputs: &mut Vec<IrDeclaredOutput>,
) -> (
    Option<IrActivation>,
    Option<plc_program::InstructionFormalId>,
) {
    match policy {
        InstructionActivationPolicy::None => (None, None),
        InstructionActivationPolicy::EnableStatus {
            enable,
            status,
            status_when_disabled,
            when_disabled,
        } => {
            outputs.push(IrDeclaredOutput {
                formal: IrFormalRef::Instruction(status),
                data_type: IrType::Bool,
            });
            (
                Some(IrActivation {
                    enable: enable_value,
                    enable_formal: enable,
                    status_formal: status,
                    status_when_disabled,
                    when_disabled,
                }),
                Some(status),
            )
        }
    }
}

fn incoming_value(
    network: &LadNetwork,
    node: &LadNode,
    values: &BTreeMap<LadPortId, IrValueId>,
) -> Result<IrValueId, LadLowerError> {
    let input = node
        .power_ports
        .values()
        .find(|port| port.direction == crate::LadPowerPortDirection::Input)
        .ok_or(LadLowerError::InvalidPowerArity {
            network: network.id,
            node: node.id,
        })?;
    let edge = network
        .power_edges
        .values()
        .find(|edge| edge.target == input.id)
        .ok_or(LadLowerError::MissingPowerValue {
            network: network.id,
            node: node.id,
        })?;
    values
        .get(&edge.source)
        .copied()
        .ok_or(LadLowerError::MissingPowerValue {
            network: network.id,
            node: node.id,
        })
}

fn single_output(network: LadNetworkId, node: &LadNode) -> Result<LadPortId, LadLowerError> {
    let mut ports = output_ports(node);
    let output = ports.next().ok_or(LadLowerError::InvalidPowerArity {
        network,
        node: node.id,
    })?;
    if ports.next().is_some() {
        return Err(LadLowerError::InvalidPowerArity {
            network,
            node: node.id,
        });
    }
    Ok(output)
}

fn output_ports(node: &LadNode) -> impl Iterator<Item = LadPortId> + '_ {
    node.ordered_power_port_ids.iter().filter_map(|id| {
        node.power_ports.get(id).and_then(|port| {
            (port.direction == crate::LadPowerPortDirection::Output).then_some(port.id)
        })
    })
}

fn node_operands(node: &LadNode) -> Vec<&LadOperandRef> {
    match &node.kind {
        LadNodeKind::Contact { operand, .. } | LadNodeKind::Coil { operand, .. } => {
            operand.iter().collect()
        }
        LadNodeKind::Box(value) => value
            .pins
            .values()
            .filter_map(|pin| pin.binding.as_ref())
            .collect(),
        LadNodeKind::Call(value) => value
            .pins
            .values()
            .filter_map(|pin| pin.binding.as_ref())
            .collect(),
        LadNodeKind::PowerSource
        | LadNodeKind::BranchSplit { .. }
        | LadNodeKind::BranchJoin { .. }
        | LadNodeKind::Return
        | LadNodeKind::UnsupportedControl { .. }
        | LadNodeKind::Unresolved { .. } => Vec::new(),
    }
}

fn ir_type(value: &DataType) -> Option<IrType> {
    match value {
        DataType::Bool => Some(IrType::Bool),
        DataType::Int => Some(IrType::Int),
        DataType::DInt => Some(IrType::DInt),
        DataType::Real => Some(IrType::Real),
        DataType::Time => Some(IrType::Time),
        DataType::String { capacity } => Some(IrType::String {
            capacity: *capacity,
        }),
        DataType::Named(_) | DataType::BlockInstance(_) | DataType::InstructionState(_) => None,
    }
}
