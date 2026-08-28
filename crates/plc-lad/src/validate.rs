use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::String,
    vec,
    vec::Vec,
};

use plc_compiler::{DiagnosticCode, DiagnosticSeverity};
use plc_program::{
    BlockId, BoundInstructionFormal, CALL_FB, CALL_FC, ControllerProgram, DataBlockKind, DataType,
    InstanceOwner, InstructionBindingError, InstructionCategory, InstructionDefinition,
    InstructionFormalDirection, InstructionFormalId, InterfaceMember, InterfaceMemberId,
    InterfaceRole, ProgramBlock, ProgramUnitKind, StateRequirement, VariableRef,
    phase2_instruction_registry,
};

use crate::{
    LAD_SCHEMA_VERSION, LadBox, LadBranchId, LadBranchPathId, LadCall, LadDocument, LadEdgeId,
    LadFormalRef, LadNetwork, LadNetworkId, LadNodeId, LadNodeKind, LadOperand, LadOperandId,
    LadOperandRef, LadPin, LadPinDirection, LadPortId, LadPortStatus, LadPowerPortDirection,
    LadStateInstanceId, MAX_EDGES_PER_NETWORK, MAX_NETWORKS_PER_BLOCK, MAX_NODES_PER_NETWORK,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LadLimits {
    pub max_networks: usize,
    pub max_nodes_per_network: usize,
    pub max_edges_per_network: usize,
    pub max_diagnostics: usize,
}

impl Default for LadLimits {
    fn default() -> Self {
        Self {
            max_networks: MAX_NETWORKS_PER_BLOCK,
            max_nodes_per_network: MAX_NODES_PER_NETWORK,
            max_edges_per_network: MAX_EDGES_PER_NETWORK,
            max_diagnostics: 10_000,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LadGraphReason {
    SchemaVersion,
    OwnerMissingOrNonExecutable,
    InvalidNetworkOrder,
    InvalidNodeOrder,
    InvalidPowerPortOrder,
    InvalidBranchOrder,
    InvalidBranchPathOrder,
    DuplicateSemanticIdentity,
    DuplicatePowerPort,
    ExactlyOnePowerSource,
    OrphanEdge,
    WrongPortDirection,
    DanglingPowerPort,
    IllegalNodeArity,
    UnreachableFragment,
    ControlFlowCycle,
    MissingBranch,
    MismatchedBranch,
    ZeroPathBranch,
    OpenBranch,
    IllegalBranchJoin,
    IllegalTerminal,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LadDiagnosticReason {
    Graph(LadGraphReason),
    ResourceLimit {
        key: &'static str,
        current: usize,
        maximum: usize,
    },
    MissingOperand,
    UnresolvedOperand(String),
    ExpressionUnavailable,
    TypeMismatch {
        expected: DataType,
        actual: Option<DataType>,
    },
    InstructionTypeConstraintMismatch {
        formal: InstructionFormalId,
        actual: Option<DataType>,
    },
    ReadOnlyTarget,
    StaleOrOrphanPin,
    RequiredPinUnbound,
    InvalidPinDirection,
    DuplicateFormal,
    UnavailableInstruction,
    InvalidStateBinding,
    AliasedStateBinding,
    InvalidCall,
    OverlappingCallBinding,
    MultipleWriter,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct LadLocation {
    pub owner: Option<BlockId>,
    pub network: Option<LadNetworkId>,
    pub node: Option<LadNodeId>,
    pub port: Option<LadPortId>,
    pub edge: Option<LadEdgeId>,
    pub branch: Option<LadBranchId>,
    pub branch_path: Option<LadBranchPathId>,
    pub operand: Option<LadOperandId>,
    pub call_site: Option<plc_program::CallSiteId>,
    pub state_instance: Option<LadStateInstanceId>,
    pub member: Option<InterfaceMemberId>,
    pub instruction_formal: Option<InstructionFormalId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LadDiagnostic {
    pub code: DiagnosticCode,
    pub severity: DiagnosticSeverity,
    pub blocking: bool,
    pub primary: LadLocation,
    pub related: Vec<LadLocation>,
    pub reason: LadDiagnosticReason,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LadNetworkAnalysis {
    pub network: LadNetworkId,
    pub execution_order: Vec<LadNodeId>,
    pub power_source: Option<LadNodeId>,
    pub terminals: Vec<LadNodeId>,
    pub structurally_valid: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LadValidationReport {
    pub diagnostics: Vec<LadDiagnostic>,
    pub networks: BTreeMap<LadNetworkId, LadNetworkAnalysis>,
}

impl LadValidationReport {
    #[must_use]
    pub fn can_lower(&self) -> bool {
        !self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.blocking)
    }

    #[must_use]
    pub fn can_lower_network(&self, network: LadNetworkId) -> bool {
        self.networks
            .get(&network)
            .is_some_and(|analysis| analysis.structurally_valid)
            && !self.diagnostics.iter().any(|diagnostic| {
                diagnostic.blocking && diagnostic.primary.network == Some(network)
            })
    }
}

/// Validates topology and instruction/operand typing independently. Invalid
/// graphs are not mutated and remain available to the editor.
#[must_use]
pub fn validate_lad(
    document: &LadDocument,
    program: &ControllerProgram,
    limits: LadLimits,
) -> LadValidationReport {
    let owner = program.block(document.owner);
    let mut validator = Validator {
        document,
        program,
        owner,
        limits,
        diagnostics: Vec::new(),
        diagnostics_observed: 0,
        diagnostics_truncated: false,
        networks: BTreeMap::new(),
        state_owners: BTreeMap::new(),
        state_invocations: BTreeSet::new(),
        call_sites: BTreeSet::new(),
        writers: BTreeMap::new(),
    };
    validator.validate_document();
    validator.finish()
}

struct Validator<'a> {
    document: &'a LadDocument,
    program: &'a ControllerProgram,
    owner: Option<&'a ProgramBlock>,
    limits: LadLimits,
    diagnostics: Vec<LadDiagnostic>,
    diagnostics_observed: usize,
    diagnostics_truncated: bool,
    networks: BTreeMap<LadNetworkId, LadNetworkAnalysis>,
    state_owners: BTreeMap<VariableRef, LadLocation>,
    state_invocations: BTreeSet<LadStateInstanceId>,
    call_sites: BTreeSet<plc_program::CallSiteId>,
    writers: BTreeMap<VariableRef, Vec<LadLocation>>,
}

impl Validator<'_> {
    fn validate_document(&mut self) {
        if self.document.schema_version != LAD_SCHEMA_VERSION {
            self.graph(None, LadGraphReason::SchemaVersion);
        }
        if self.owner.is_none_or(|owner| !owner.kind.is_executable()) {
            self.graph(None, LadGraphReason::OwnerMissingOrNonExecutable);
        }
        if self.document.networks.len() > self.limits.max_networks {
            self.resource(
                None,
                "lad.networks",
                self.document.networks.len(),
                self.limits.max_networks,
            );
            return;
        }
        let mut resource_limit_exceeded = false;
        for network in self.document.networks.values() {
            if network.nodes.len() > self.limits.max_nodes_per_network {
                self.resource(
                    Some(network.id),
                    "lad.nodes",
                    network.nodes.len(),
                    self.limits.max_nodes_per_network,
                );
                resource_limit_exceeded = true;
            }
            if network.power_edges.len() > self.limits.max_edges_per_network {
                self.resource(
                    Some(network.id),
                    "lad.edges",
                    network.power_edges.len(),
                    self.limits.max_edges_per_network,
                );
                resource_limit_exceeded = true;
            }
        }
        if resource_limit_exceeded {
            return;
        }
        if !ordered_projection_matches(
            &self.document.ordered_network_ids,
            self.document.networks.keys().copied(),
        ) || self
            .document
            .ordered_network_ids
            .iter()
            .enumerate()
            .any(|(index, id)| {
                self.document
                    .networks
                    .get(id)
                    .is_none_or(|network| usize::try_from(network.semantic_order) != Ok(index))
            })
        {
            self.graph(None, LadGraphReason::InvalidNetworkOrder);
        }
        self.validate_global_identities();
        for network_id in &self.document.ordered_network_ids {
            if let Some(network) = self.document.networks.get(network_id) {
                self.validate_network(network);
            }
        }
        for network in self.document.networks.values() {
            if !self.networks.contains_key(&network.id) {
                self.validate_network(network);
            }
        }
        self.emit_multiple_writers();
    }

    fn validate_global_identities(&mut self) {
        let mut nodes = BTreeSet::new();
        let mut ports = BTreeSet::new();
        let mut edges = BTreeSet::new();
        let mut branches = BTreeSet::new();
        let mut paths = BTreeSet::new();
        let mut operands = BTreeSet::new();
        for network in self.document.networks.values() {
            for node in network.nodes.values() {
                let node_location = self.node_location(network.id, node.id);
                if !nodes.insert(node.id) {
                    self.push(
                        DiagnosticCode::MALFORMED_STRUCTURE,
                        DiagnosticSeverity::Error,
                        true,
                        node_location.clone(),
                        Vec::new(),
                        LadDiagnosticReason::Graph(LadGraphReason::DuplicateSemanticIdentity),
                    );
                }
                for port in node.power_ports.values() {
                    if !ports.insert(port.id) {
                        self.push(
                            DiagnosticCode::MALFORMED_STRUCTURE,
                            DiagnosticSeverity::Error,
                            true,
                            self.port_location(network.id, node.id, port.id),
                            Vec::new(),
                            LadDiagnosticReason::Graph(LadGraphReason::DuplicateSemanticIdentity),
                        );
                    }
                }
                for operand in node_operands(node) {
                    if !operands.insert(operand.id) {
                        self.push(
                            DiagnosticCode::MALFORMED_STRUCTURE,
                            DiagnosticSeverity::Error,
                            true,
                            LadLocation {
                                operand: Some(operand.id),
                                ..node_location.clone()
                            },
                            Vec::new(),
                            LadDiagnosticReason::Graph(LadGraphReason::DuplicateSemanticIdentity),
                        );
                    }
                }
            }
            for edge in network.power_edges.values() {
                if !edges.insert(edge.id) {
                    self.edge_graph(
                        network.id,
                        edge.id,
                        LadGraphReason::DuplicateSemanticIdentity,
                    );
                }
            }
            for branch in network.branches.values() {
                if !branches.insert(branch.id) {
                    self.branch_graph(
                        network.id,
                        branch.id,
                        None,
                        LadGraphReason::DuplicateSemanticIdentity,
                    );
                }
                for path in branch.paths.values() {
                    if !paths.insert(path.id) {
                        self.branch_graph(
                            network.id,
                            branch.id,
                            Some(path.id),
                            LadGraphReason::DuplicateSemanticIdentity,
                        );
                    }
                }
            }
        }
    }

    fn finish(mut self) -> LadValidationReport {
        self.diagnostics.sort_by(|left, right| {
            (
                left.primary.network,
                left.primary.node,
                left.primary.port,
                left.primary.edge,
                left.code,
                &left.reason,
                &left.related,
            )
                .cmp(&(
                    right.primary.network,
                    right.primary.node,
                    right.primary.port,
                    right.primary.edge,
                    right.code,
                    &right.reason,
                    &right.related,
                ))
        });
        self.diagnostics.dedup();
        if self.diagnostics_truncated {
            let effective_limit = self.limits.max_diagnostics.max(1);
            self.diagnostics.truncate(effective_limit.saturating_sub(1));
            self.diagnostics.push(LadDiagnostic {
                code: DiagnosticCode::RESOURCE_LIMIT,
                severity: DiagnosticSeverity::Error,
                blocking: true,
                primary: self.location(None),
                related: Vec::new(),
                reason: LadDiagnosticReason::ResourceLimit {
                    key: "lad.diagnostics",
                    current: self.diagnostics_observed,
                    maximum: self.limits.max_diagnostics,
                },
            });
        }
        LadValidationReport {
            diagnostics: self.diagnostics,
            networks: self.networks,
        }
    }

    #[allow(clippy::too_many_lines)]
    fn validate_network(&mut self, network: &LadNetwork) {
        let diagnostic_start = self.diagnostics.len();
        if network.nodes.len() > self.limits.max_nodes_per_network {
            self.resource(
                Some(network.id),
                "lad.nodes",
                network.nodes.len(),
                self.limits.max_nodes_per_network,
            );
        }
        if network.power_edges.len() > self.limits.max_edges_per_network {
            self.resource(
                Some(network.id),
                "lad.edges",
                network.power_edges.len(),
                self.limits.max_edges_per_network,
            );
        }
        if !ordered_projection_matches(&network.ordered_node_ids, network.nodes.keys().copied())
            || network
                .ordered_node_ids
                .iter()
                .enumerate()
                .any(|(index, id)| {
                    network
                        .nodes
                        .get(id)
                        .is_none_or(|node| usize::try_from(node.semantic_order) != Ok(index))
                })
        {
            self.graph(Some(network.id), LadGraphReason::InvalidNodeOrder);
        }
        if !ordered_projection_matches(
            &network.ordered_branch_ids,
            network.branches.keys().copied(),
        ) {
            self.graph(Some(network.id), LadGraphReason::InvalidBranchOrder);
        }

        let mut index = GraphIndex::new();
        self.index_ports(network, &mut index);
        self.index_edges(network, &mut index);
        self.validate_branches(network, &index);
        self.validate_node_arities(network, &index);
        self.validate_node_semantics(network);

        let sources: Vec<_> = network
            .nodes
            .values()
            .filter(|node| matches!(node.kind, LadNodeKind::PowerSource))
            .map(|node| node.id)
            .collect();
        if sources.len() != 1 {
            self.graph(Some(network.id), LadGraphReason::ExactlyOnePowerSource);
        }
        let power_source = sources.first().copied();
        let terminals: Vec<_> = network
            .nodes
            .values()
            .filter(|node| matches!(node.kind, LadNodeKind::Coil { .. } | LadNodeKind::Return))
            .map(|node| node.id)
            .collect();
        if terminals.is_empty() {
            self.graph(Some(network.id), LadGraphReason::IllegalTerminal);
        }
        if has_cycle(network, &index) {
            self.graph(Some(network.id), LadGraphReason::ControlFlowCycle);
        }
        if let Some(source) = power_source {
            let reachable = reachable_nodes(source, &index);
            for node in network.nodes.keys() {
                if !reachable.contains(node) {
                    self.push(
                        DiagnosticCode::INVALID_POWER_GRAPH,
                        DiagnosticSeverity::Error,
                        true,
                        self.node_location(network.id, *node),
                        Vec::new(),
                        LadDiagnosticReason::Graph(LadGraphReason::UnreachableFragment),
                    );
                }
            }
        }

        let blocking_here = self.diagnostics[diagnostic_start..]
            .iter()
            .any(|diagnostic| diagnostic.blocking);
        let execution_order = if blocking_here {
            Vec::new()
        } else {
            power_source.map_or_else(Vec::new, |source| {
                build_execution_order(network, &index, source)
            })
        };
        let structurally_valid = !blocking_here && execution_order.len() == network.nodes.len();
        if !blocking_here && !structurally_valid {
            self.graph(Some(network.id), LadGraphReason::OpenBranch);
        }
        self.networks.insert(
            network.id,
            LadNetworkAnalysis {
                network: network.id,
                execution_order,
                power_source,
                terminals,
                structurally_valid,
            },
        );
    }

    fn index_ports(&mut self, network: &LadNetwork, index: &mut GraphIndex) {
        for node in network.nodes.values() {
            if !ordered_projection_matches(
                &node.ordered_power_port_ids,
                node.power_ports.keys().copied(),
            ) {
                self.push(
                    DiagnosticCode::INVALID_POWER_GRAPH,
                    DiagnosticSeverity::Error,
                    true,
                    self.node_location(network.id, node.id),
                    Vec::new(),
                    LadDiagnosticReason::Graph(LadGraphReason::InvalidPowerPortOrder),
                );
            }
            for port in node.power_ports.values() {
                if index
                    .port_owner
                    .insert(port.id, (node.id, port.direction))
                    .is_some()
                {
                    self.push(
                        DiagnosticCode::INVALID_POWER_GRAPH,
                        DiagnosticSeverity::Error,
                        true,
                        self.port_location(network.id, node.id, port.id),
                        Vec::new(),
                        LadDiagnosticReason::Graph(LadGraphReason::DuplicatePowerPort),
                    );
                }
            }
        }
    }

    fn index_edges(&mut self, network: &LadNetwork, index: &mut GraphIndex) {
        for edge in network.power_edges.values() {
            let source = index.port_owner.get(&edge.source).copied();
            let target = index.port_owner.get(&edge.target).copied();
            let Some((source_node, source_direction)) = source else {
                self.edge_graph(network.id, edge.id, LadGraphReason::OrphanEdge);
                continue;
            };
            let Some((target_node, target_direction)) = target else {
                self.edge_graph(network.id, edge.id, LadGraphReason::OrphanEdge);
                continue;
            };
            if source_direction != LadPowerPortDirection::Output
                || target_direction != LadPowerPortDirection::Input
            {
                self.edge_graph(network.id, edge.id, LadGraphReason::WrongPortDirection);
                continue;
            }
            index.outgoing.entry(source_node).or_default().push((
                edge.id,
                target_node,
                edge.source,
                edge.target,
            ));
            index.incoming.entry(target_node).or_default().push((
                edge.id,
                source_node,
                edge.source,
                edge.target,
            ));
            *index.port_use.entry(edge.source).or_default() += 1;
            *index.port_use.entry(edge.target).or_default() += 1;
        }
        for edges in index.outgoing.values_mut() {
            edges.sort_by_key(|edge| edge.0);
        }
        for edges in index.incoming.values_mut() {
            edges.sort_by_key(|edge| edge.0);
        }
        for (port, (node, _)) in &index.port_owner {
            if index.port_use.get(port).copied().unwrap_or(0) != 1 {
                self.push(
                    DiagnosticCode::INVALID_POWER_GRAPH,
                    DiagnosticSeverity::Error,
                    true,
                    self.port_location(network.id, *node, *port),
                    Vec::new(),
                    LadDiagnosticReason::Graph(LadGraphReason::DanglingPowerPort),
                );
            }
        }
    }

    fn validate_branches(&mut self, network: &LadNetwork, index: &GraphIndex) {
        for branch in network.branches.values() {
            if !ordered_projection_matches(&branch.ordered_path_ids, branch.paths.keys().copied()) {
                self.branch_graph(
                    network.id,
                    branch.id,
                    None,
                    LadGraphReason::InvalidBranchPathOrder,
                );
            }
            let split_matches = network.nodes.get(&branch.split_node).is_some_and(|node| {
                matches!(node.kind, LadNodeKind::BranchSplit { branch: id } if id == branch.id)
            });
            let join_matches = network.nodes.get(&branch.join_node).is_some_and(|node| {
                matches!(node.kind, LadNodeKind::BranchJoin { branch: id } if id == branch.id)
            });
            if !split_matches || !join_matches {
                self.branch_graph(
                    network.id,
                    branch.id,
                    None,
                    LadGraphReason::MismatchedBranch,
                );
            }
            if branch.paths.is_empty() {
                self.branch_graph(network.id, branch.id, None, LadGraphReason::ZeroPathBranch);
            }
            let mut entry_edges = BTreeSet::new();
            let mut exit_edges = BTreeSet::new();
            for path_id in &branch.ordered_path_ids {
                let Some(path) = branch.paths.get(path_id) else {
                    continue;
                };
                if !entry_edges.insert(path.entry_edge) || !exit_edges.insert(path.exit_edge) {
                    self.branch_graph(
                        network.id,
                        branch.id,
                        Some(*path_id),
                        LadGraphReason::IllegalBranchJoin,
                    );
                }
                let entry = edge_nodes(network, index, path.entry_edge);
                let exit = edge_nodes(network, index, path.exit_edge);
                let valid_endpoints = entry.is_some_and(|(from, _)| from == branch.split_node)
                    && exit.is_some_and(|(_, to)| to == branch.join_node);
                if !valid_endpoints {
                    self.branch_graph(
                        network.id,
                        branch.id,
                        Some(*path_id),
                        LadGraphReason::OpenBranch,
                    );
                    continue;
                }
                let (_, start) = entry.expect("checked above");
                let (end, _) = exit.expect("checked above");
                if path.entry_edge == path.exit_edge || start == branch.join_node {
                    self.branch_graph(
                        network.id,
                        branch.id,
                        Some(*path_id),
                        LadGraphReason::ZeroPathBranch,
                    );
                } else if !reachable_before_join(start, end, branch.join_node, index) {
                    self.branch_graph(
                        network.id,
                        branch.id,
                        Some(*path_id),
                        LadGraphReason::OpenBranch,
                    );
                }
            }
        }
        for node in network.nodes.values() {
            let branch = match node.kind {
                LadNodeKind::BranchSplit { branch } | LadNodeKind::BranchJoin { branch } => {
                    Some(branch)
                }
                _ => None,
            };
            if branch.is_some_and(|id| !network.branches.contains_key(&id)) {
                self.push(
                    DiagnosticCode::INVALID_POWER_GRAPH,
                    DiagnosticSeverity::Error,
                    true,
                    self.node_location(network.id, node.id),
                    Vec::new(),
                    LadDiagnosticReason::Graph(LadGraphReason::MissingBranch),
                );
            }
        }
    }

    fn validate_node_arities(&mut self, network: &LadNetwork, index: &GraphIndex) {
        for node in network.nodes.values() {
            let incoming = index.incoming.get(&node.id).map_or(0, Vec::len);
            let outgoing = index.outgoing.get(&node.id).map_or(0, Vec::len);
            let branch_paths = match node.kind {
                LadNodeKind::BranchSplit { branch } | LadNodeKind::BranchJoin { branch } => network
                    .branches
                    .get(&branch)
                    .map_or(0, |value| value.paths.len()),
                _ => 0,
            };
            let valid = match node.kind {
                LadNodeKind::PowerSource => incoming == 0 && outgoing == 1,
                LadNodeKind::Contact { .. }
                | LadNodeKind::Box(_)
                | LadNodeKind::Call(_)
                | LadNodeKind::UnsupportedControl { .. }
                | LadNodeKind::Unresolved { .. } => incoming == 1 && outgoing == 1,
                LadNodeKind::BranchSplit { .. } => {
                    incoming == 1 && branch_paths >= 2 && outgoing == branch_paths
                }
                LadNodeKind::BranchJoin { .. } => {
                    outgoing == 1 && branch_paths >= 2 && incoming == branch_paths
                }
                LadNodeKind::Coil { .. } | LadNodeKind::Return => incoming == 1 && outgoing == 0,
            };
            let input_ports = node
                .power_ports
                .values()
                .filter(|port| port.direction == LadPowerPortDirection::Input)
                .count();
            let output_ports = node
                .power_ports
                .values()
                .filter(|port| port.direction == LadPowerPortDirection::Output)
                .count();
            if !valid || incoming != input_ports || outgoing != output_ports {
                self.push(
                    DiagnosticCode::INVALID_POWER_GRAPH,
                    DiagnosticSeverity::Error,
                    true,
                    self.node_location(network.id, node.id),
                    Vec::new(),
                    LadDiagnosticReason::Graph(LadGraphReason::IllegalNodeArity),
                );
            }
            if incoming > 1 && !matches!(node.kind, LadNodeKind::BranchJoin { .. }) {
                self.push(
                    DiagnosticCode::INVALID_POWER_GRAPH,
                    DiagnosticSeverity::Error,
                    true,
                    self.node_location(network.id, node.id),
                    Vec::new(),
                    LadDiagnosticReason::Graph(LadGraphReason::IllegalBranchJoin),
                );
            }
            if outgoing > 1 && !matches!(node.kind, LadNodeKind::BranchSplit { .. }) {
                self.push(
                    DiagnosticCode::INVALID_POWER_GRAPH,
                    DiagnosticSeverity::Error,
                    true,
                    self.node_location(network.id, node.id),
                    Vec::new(),
                    LadDiagnosticReason::Graph(LadGraphReason::IllegalBranchJoin),
                );
            }
        }
    }

    fn validate_node_semantics(&mut self, network: &LadNetwork) {
        for node in network.nodes.values() {
            match &node.kind {
                LadNodeKind::Contact { operand, .. } => {
                    self.validate_read_operand(
                        network.id,
                        node.id,
                        operand.as_ref(),
                        &DataType::Bool,
                    );
                }
                LadNodeKind::Coil { operand, .. } => {
                    self.validate_write_operand(
                        network.id,
                        node.id,
                        operand.as_ref(),
                        &DataType::Bool,
                    );
                }
                LadNodeKind::Box(value) => self.validate_box(network.id, node.id, value),
                LadNodeKind::Call(value) => self.validate_call(network.id, node.id, value),
                LadNodeKind::UnsupportedControl { .. } | LadNodeKind::Unresolved { .. } => {
                    self.push(
                        DiagnosticCode::CAPABILITY_UNAVAILABLE,
                        DiagnosticSeverity::Error,
                        true,
                        self.node_location(network.id, node.id),
                        Vec::new(),
                        LadDiagnosticReason::UnavailableInstruction,
                    );
                }
                LadNodeKind::PowerSource
                | LadNodeKind::BranchSplit { .. }
                | LadNodeKind::BranchJoin { .. }
                | LadNodeKind::Return => {}
            }
        }
    }

    fn validate_box(&mut self, network: LadNetworkId, node: LadNodeId, value: &LadBox) {
        let registry = *phase2_instruction_registry();
        let Some(definition) = registry.lookup(value.instruction) else {
            self.push(
                DiagnosticCode::CAPABILITY_UNAVAILABLE,
                DiagnosticSeverity::Error,
                true,
                self.node_location(network, node),
                Vec::new(),
                LadDiagnosticReason::UnavailableInstruction,
            );
            return;
        };
        if matches!(
            definition.category,
            InstructionCategory::Call | InstructionCategory::Control
        ) {
            self.push(
                DiagnosticCode::CAPABILITY_UNAVAILABLE,
                DiagnosticSeverity::Error,
                true,
                self.node_location(network, node),
                Vec::new(),
                LadDiagnosticReason::UnavailableInstruction,
            );
        }
        self.validate_pin_projection(network, node, &value.pins, &value.ordered_pin_ids);
        let mut seen_formals = BTreeSet::new();
        let mut bound_types = Vec::new();
        for pin in value.pins.values() {
            if let Some(LadFormalRef::Instruction(formal)) = pin.formal {
                if !seen_formals.insert(formal) {
                    self.push(
                        DiagnosticCode::ILLEGAL_OR_OVERLAPPING_BINDING,
                        DiagnosticSeverity::Error,
                        true,
                        self.pin_location(network, node, pin, None),
                        Vec::new(),
                        LadDiagnosticReason::DuplicateFormal,
                    );
                }
                bound_types.push((formal, pin.data_type.clone()));
            }
        }
        match (definition.state_requirement, value.state.as_ref()) {
            (StateRequirement::None, None) => {}
            (StateRequirement::Explicit(expected), Some(state)) if state.kind == expected => {
                self.validate_state(network, node, state);
                bound_types.push((
                    plc_program::FORMAL_STATE,
                    DataType::InstructionState(expected),
                ));
            }
            (StateRequirement::FunctionBlockInstance, _)
            | (StateRequirement::None, Some(_))
            | (StateRequirement::Explicit(_), None | Some(_)) => {
                self.push(
                    DiagnosticCode::INSTANCE_INVALID,
                    DiagnosticSeverity::Error,
                    true,
                    self.node_location(network, node),
                    Vec::new(),
                    LadDiagnosticReason::InvalidStateBinding,
                );
            }
        }
        for pin in value.pins.values() {
            self.validate_instruction_pin(network, node, pin, definition, &bound_types);
        }
        let canonical_bindings = bound_types
            .iter()
            .cloned()
            .map(|(formal, data_type)| BoundInstructionFormal { formal, data_type });
        if let Err(error) = registry.bind_types(value.instruction, canonical_bindings) {
            self.instruction_binding_diagnostic(network, node, error);
        }
    }

    fn validate_state(
        &mut self,
        network: LadNetworkId,
        node: LadNodeId,
        state: &crate::LadStateBinding,
    ) {
        let location = LadLocation {
            state_instance: Some(state.invocation),
            ..self.node_location(network, node)
        };
        let valid_storage = self.resolve_variable(&state.storage).is_some_and(|member| {
            member.role == InterfaceRole::Static
                && member.data_type == DataType::InstructionState(state.kind)
        });
        if !valid_storage || !self.state_invocations.insert(state.invocation) {
            self.push(
                DiagnosticCode::INSTANCE_INVALID,
                DiagnosticSeverity::Error,
                true,
                location.clone(),
                Vec::new(),
                LadDiagnosticReason::InvalidStateBinding,
            );
        }
        if let Some(previous) = self
            .state_owners
            .insert(state.storage.clone(), location.clone())
        {
            self.push(
                DiagnosticCode::INSTANCE_INVALID,
                DiagnosticSeverity::Error,
                true,
                location,
                vec![previous],
                LadDiagnosticReason::AliasedStateBinding,
            );
        }
    }

    #[allow(clippy::too_many_lines)]
    fn validate_call(&mut self, network: LadNetworkId, node: LadNodeId, value: &LadCall) {
        let base_location = LadLocation {
            call_site: Some(value.call_site),
            ..self.node_location(network, node)
        };
        if !self.call_sites.insert(value.call_site) {
            self.push(
                DiagnosticCode::ILLEGAL_OR_OVERLAPPING_BINDING,
                DiagnosticSeverity::Error,
                true,
                base_location.clone(),
                Vec::new(),
                LadDiagnosticReason::InvalidCall,
            );
        }
        let Some(callee) = self.program.block(value.callee) else {
            self.push(
                DiagnosticCode::UNRESOLVED_REFERENCE,
                DiagnosticSeverity::Error,
                true,
                base_location,
                Vec::new(),
                LadDiagnosticReason::InvalidCall,
            );
            return;
        };
        let kind_valid = (value.instruction == CALL_FC && callee.kind == ProgramUnitKind::Function)
            || (value.instruction == CALL_FB && callee.kind == ProgramUnitKind::FunctionBlock);
        if !kind_valid {
            self.push(
                DiagnosticCode::ILLEGAL_OR_OVERLAPPING_BINDING,
                DiagnosticSeverity::Error,
                true,
                base_location.clone(),
                Vec::new(),
                LadDiagnosticReason::InvalidCall,
            );
        }
        let Some(call_definition) = phase2_instruction_registry().lookup(value.instruction) else {
            self.push(
                DiagnosticCode::CAPABILITY_UNAVAILABLE,
                DiagnosticSeverity::Error,
                true,
                base_location.clone(),
                Vec::new(),
                LadDiagnosticReason::UnavailableInstruction,
            );
            return;
        };
        self.validate_pin_projection(network, node, &value.pins, &value.ordered_pin_ids);
        let mut bound_block_formals = BTreeSet::new();
        let mut all_formals = BTreeSet::new();
        let mut call_writes = BTreeMap::<VariableRef, LadLocation>::new();
        for pin in value.pins.values() {
            if let Some(formal) = pin.formal {
                if !all_formals.insert(formal) {
                    self.push(
                        DiagnosticCode::ILLEGAL_OR_OVERLAPPING_BINDING,
                        DiagnosticSeverity::Error,
                        true,
                        self.pin_location(network, node, pin, Some(value.call_site)),
                        Vec::new(),
                        LadDiagnosticReason::DuplicateFormal,
                    );
                }
                if let LadFormalRef::BlockMember(member) = formal {
                    bound_block_formals.insert(member);
                }
            }
            self.validate_call_pin(network, node, pin, callee, call_definition, value.call_site);
            if matches!(
                pin.direction,
                LadPinDirection::Output | LadPinDirection::InOut
            ) && matches!(pin.formal, Some(LadFormalRef::BlockMember(_)))
                && let Some(LadOperandRef {
                    value: LadOperand::Variable(variable),
                    ..
                }) = &pin.binding
            {
                let location = self.pin_location(network, node, pin, Some(value.call_site));
                if let Some(previous) = call_writes.insert(variable.clone(), location.clone()) {
                    self.push(
                        DiagnosticCode::ILLEGAL_OR_OVERLAPPING_BINDING,
                        DiagnosticSeverity::Error,
                        true,
                        location,
                        vec![previous],
                        LadDiagnosticReason::OverlappingCallBinding,
                    );
                }
            }
        }
        for member in callee.interface.members.values() {
            let required = call_member_required(member);
            if required && !bound_block_formals.contains(&member.id) {
                self.push(
                    DiagnosticCode::REQUIRED_BINDING_MISSING,
                    DiagnosticSeverity::Error,
                    true,
                    LadLocation {
                        member: Some(member.id),
                        ..base_location.clone()
                    },
                    Vec::new(),
                    LadDiagnosticReason::RequiredPinUnbound,
                );
            }
        }
        if value.instruction == CALL_FB {
            if value
                .instance
                .as_ref()
                .is_none_or(|instance| !self.valid_fb_instance(callee, instance))
            {
                self.push(
                    DiagnosticCode::INSTANCE_INVALID,
                    DiagnosticSeverity::Error,
                    true,
                    base_location,
                    Vec::new(),
                    LadDiagnosticReason::InvalidCall,
                );
            }
        } else if value.instance.is_some() {
            self.push(
                DiagnosticCode::INSTANCE_INVALID,
                DiagnosticSeverity::Error,
                true,
                base_location,
                Vec::new(),
                LadDiagnosticReason::InvalidCall,
            );
        }
    }

    fn valid_fb_instance(&self, callee: &ProgramBlock, instance: &crate::LadInstance) -> bool {
        match instance.owner {
            InstanceOwner::InstanceDb(instance_db) => {
                instance.path.root_instance_db == instance_db
                    && instance.path.multi_instance_slots.is_empty()
                    && self.program.block(instance_db).is_some_and(|block| {
                        block.kind
                            == ProgramUnitKind::DataBlock(DataBlockKind::Instance {
                                fb_type: callee.id,
                            })
                    })
            }
            InstanceOwner::MultiInstance {
                owner_fb,
                static_member,
            } => {
                let owner = self.program.block(owner_fb);
                owner.is_some_and(|block| {
                    block.kind == ProgramUnitKind::FunctionBlock
                        && block.interface.member(static_member).is_some_and(|member| {
                            member.role == InterfaceRole::Static
                                && member.data_type == DataType::BlockInstance(callee.id)
                        })
                }) && instance.path.multi_instance_slots.last() == Some(&static_member)
            }
        }
    }

    fn validate_pin_projection(
        &mut self,
        network: LadNetworkId,
        node: LadNodeId,
        pins: &BTreeMap<LadPortId, LadPin>,
        ordered: &[LadPortId],
    ) {
        if !ordered_projection_matches(ordered, pins.keys().copied()) {
            self.push(
                DiagnosticCode::MALFORMED_STRUCTURE,
                DiagnosticSeverity::Error,
                true,
                self.node_location(network, node),
                Vec::new(),
                LadDiagnosticReason::Graph(LadGraphReason::InvalidPowerPortOrder),
            );
        }
    }

    fn validate_instruction_pin(
        &mut self,
        network: LadNetworkId,
        node: LadNodeId,
        pin: &LadPin,
        definition: &InstructionDefinition,
        bound_types: &[(InstructionFormalId, DataType)],
    ) {
        let location = self.pin_location(network, node, pin, None);
        let Some(LadFormalRef::Instruction(formal_id)) = pin.formal else {
            self.push(
                DiagnosticCode::STALE_FORMAL,
                DiagnosticSeverity::Error,
                true,
                location.clone(),
                Vec::new(),
                LadDiagnosticReason::StaleOrOrphanPin,
            );
            self.validate_pin_binding(network, node, pin, None);
            return;
        };
        let Some(formal) = definition.formal(formal_id) else {
            self.push(
                DiagnosticCode::STALE_FORMAL,
                DiagnosticSeverity::Error,
                true,
                location.clone(),
                Vec::new(),
                LadDiagnosticReason::StaleOrOrphanPin,
            );
            self.validate_pin_binding(network, node, pin, None);
            return;
        };
        if !instruction_direction_matches(pin.direction, formal.direction) {
            self.push(
                DiagnosticCode::ILLEGAL_OR_OVERLAPPING_BINDING,
                DiagnosticSeverity::Error,
                true,
                location.clone(),
                Vec::new(),
                LadDiagnosticReason::InvalidPinDirection,
            );
        }
        if pin.required != formal.required || !pin.name.eq_ignore_ascii_case(formal.name) {
            self.push(
                DiagnosticCode::STALE_FORMAL,
                DiagnosticSeverity::Error,
                true,
                location.clone(),
                Vec::new(),
                LadDiagnosticReason::StaleOrOrphanPin,
            );
        }
        if !formal.type_constraint.accepts(&pin.data_type, bound_types) {
            self.push(
                DiagnosticCode::TYPE_MISMATCH,
                DiagnosticSeverity::Error,
                true,
                location,
                Vec::new(),
                LadDiagnosticReason::InstructionTypeConstraintMismatch {
                    formal: formal_id,
                    actual: Some(pin.data_type.clone()),
                },
            );
        }
        self.validate_pin_binding(network, node, pin, None);
    }

    #[allow(clippy::too_many_lines)]
    fn validate_call_pin(
        &mut self,
        network: LadNetworkId,
        node: LadNodeId,
        pin: &LadPin,
        callee: &ProgramBlock,
        call_definition: &InstructionDefinition,
        call_site: plc_program::CallSiteId,
    ) {
        let location = self.pin_location(network, node, pin, Some(call_site));
        match pin.formal {
            Some(LadFormalRef::BlockMember(member_id)) => {
                let Some(formal) = callee.interface.member(member_id) else {
                    self.push(
                        DiagnosticCode::STALE_FORMAL,
                        DiagnosticSeverity::Error,
                        true,
                        location.clone(),
                        Vec::new(),
                        LadDiagnosticReason::StaleOrOrphanPin,
                    );
                    self.validate_pin_binding(network, node, pin, Some(call_site));
                    return;
                };
                let direction_valid = matches!(
                    (pin.direction, formal.role),
                    (LadPinDirection::Input, InterfaceRole::Input)
                        | (
                            LadPinDirection::Output,
                            InterfaceRole::Output | InterfaceRole::Return
                        )
                        | (LadPinDirection::InOut, InterfaceRole::InOut)
                );
                if !direction_valid {
                    self.push(
                        DiagnosticCode::ILLEGAL_OR_OVERLAPPING_BINDING,
                        DiagnosticSeverity::Error,
                        true,
                        location.clone(),
                        Vec::new(),
                        LadDiagnosticReason::InvalidPinDirection,
                    );
                }
                if pin.data_type != formal.data_type {
                    self.push(
                        DiagnosticCode::TYPE_MISMATCH,
                        DiagnosticSeverity::Error,
                        true,
                        location.clone(),
                        Vec::new(),
                        LadDiagnosticReason::TypeMismatch {
                            expected: formal.data_type.clone(),
                            actual: Some(pin.data_type.clone()),
                        },
                    );
                }
                if pin.required != call_member_required(formal)
                    || !pin.name.eq_ignore_ascii_case(&formal.name)
                {
                    self.push(
                        DiagnosticCode::STALE_FORMAL,
                        DiagnosticSeverity::Error,
                        true,
                        location.clone(),
                        Vec::new(),
                        LadDiagnosticReason::StaleOrOrphanPin,
                    );
                }
            }
            Some(LadFormalRef::Instruction(formal_id)) => {
                let Some(formal) = call_definition.formal(formal_id) else {
                    self.push(
                        DiagnosticCode::STALE_FORMAL,
                        DiagnosticSeverity::Error,
                        true,
                        location.clone(),
                        Vec::new(),
                        LadDiagnosticReason::StaleOrOrphanPin,
                    );
                    self.validate_pin_binding(network, node, pin, Some(call_site));
                    return;
                };
                if !matches!(
                    formal.direction,
                    InstructionFormalDirection::Activation | InstructionFormalDirection::Status
                ) || !instruction_direction_matches(pin.direction, formal.direction)
                {
                    self.push(
                        DiagnosticCode::ILLEGAL_OR_OVERLAPPING_BINDING,
                        DiagnosticSeverity::Error,
                        true,
                        location.clone(),
                        Vec::new(),
                        LadDiagnosticReason::InvalidPinDirection,
                    );
                }
                if pin.data_type != DataType::Bool {
                    self.push(
                        DiagnosticCode::TYPE_MISMATCH,
                        DiagnosticSeverity::Error,
                        true,
                        location.clone(),
                        Vec::new(),
                        LadDiagnosticReason::TypeMismatch {
                            expected: DataType::Bool,
                            actual: Some(pin.data_type.clone()),
                        },
                    );
                }
                if pin.required != formal.required || !pin.name.eq_ignore_ascii_case(formal.name) {
                    self.push(
                        DiagnosticCode::STALE_FORMAL,
                        DiagnosticSeverity::Error,
                        true,
                        location.clone(),
                        Vec::new(),
                        LadDiagnosticReason::StaleOrOrphanPin,
                    );
                }
            }
            None => self.push(
                DiagnosticCode::STALE_FORMAL,
                DiagnosticSeverity::Error,
                true,
                location,
                Vec::new(),
                LadDiagnosticReason::StaleOrOrphanPin,
            ),
        }
        self.validate_pin_binding(network, node, pin, Some(call_site));
    }

    fn validate_pin_binding(
        &mut self,
        network: LadNetworkId,
        node: LadNodeId,
        pin: &LadPin,
        call_site: Option<plc_program::CallSiteId>,
    ) {
        let location = self.pin_location(network, node, pin, call_site);
        if pin.status != LadPortStatus::Active {
            self.push(
                DiagnosticCode::STALE_FORMAL,
                DiagnosticSeverity::Error,
                true,
                location.clone(),
                Vec::new(),
                LadDiagnosticReason::StaleOrOrphanPin,
            );
        }
        if matches!(
            pin.direction,
            LadPinDirection::Activation | LadPinDirection::Status
        ) {
            if pin.data_type != DataType::Bool {
                self.push(
                    DiagnosticCode::TYPE_MISMATCH,
                    DiagnosticSeverity::Error,
                    true,
                    location.clone(),
                    Vec::new(),
                    LadDiagnosticReason::TypeMismatch {
                        expected: DataType::Bool,
                        actual: Some(pin.data_type.clone()),
                    },
                );
            }
            if pin.binding.is_some() {
                self.push(
                    DiagnosticCode::ILLEGAL_OR_OVERLAPPING_BINDING,
                    DiagnosticSeverity::Error,
                    true,
                    location,
                    Vec::new(),
                    LadDiagnosticReason::InvalidPinDirection,
                );
            }
            return;
        }
        if pin.required && pin.binding.is_none() {
            self.push(
                DiagnosticCode::REQUIRED_BINDING_MISSING,
                DiagnosticSeverity::Error,
                true,
                location,
                Vec::new(),
                LadDiagnosticReason::RequiredPinUnbound,
            );
            return;
        }
        if let Some(binding) = pin.binding.as_ref() {
            if matches!(
                pin.direction,
                LadPinDirection::Output | LadPinDirection::InOut
            ) {
                self.validate_write_operand(network, node, Some(binding), &pin.data_type);
            } else {
                self.validate_read_operand(network, node, Some(binding), &pin.data_type);
            }
        }
    }

    fn instruction_binding_diagnostic(
        &mut self,
        network: LadNetworkId,
        node: LadNodeId,
        error: InstructionBindingError,
    ) {
        let (code, formal, reason) = match error {
            InstructionBindingError::UnknownInstruction(_) => (
                DiagnosticCode::CAPABILITY_UNAVAILABLE,
                None,
                LadDiagnosticReason::UnavailableInstruction,
            ),
            InstructionBindingError::MissingRequiredFormal(_, formal) => (
                DiagnosticCode::REQUIRED_BINDING_MISSING,
                Some(formal),
                LadDiagnosticReason::RequiredPinUnbound,
            ),
            InstructionBindingError::UnknownFormal(_, formal)
            | InstructionBindingError::DuplicateFormal(_, formal) => (
                DiagnosticCode::STALE_FORMAL,
                Some(formal),
                LadDiagnosticReason::StaleOrOrphanPin,
            ),
            InstructionBindingError::TypeConstraint(_, formal) => (
                DiagnosticCode::TYPE_MISMATCH,
                Some(formal),
                LadDiagnosticReason::InstructionTypeConstraintMismatch {
                    formal,
                    actual: None,
                },
            ),
        };
        self.push(
            code,
            DiagnosticSeverity::Error,
            true,
            LadLocation {
                instruction_formal: formal,
                ..self.node_location(network, node)
            },
            Vec::new(),
            reason,
        );
    }

    fn validate_read_operand(
        &mut self,
        network: LadNetworkId,
        node: LadNodeId,
        operand: Option<&LadOperandRef>,
        expected: &DataType,
    ) {
        let Some(operand) = operand else {
            self.push(
                DiagnosticCode::UNRESOLVED_REFERENCE,
                DiagnosticSeverity::Error,
                true,
                self.node_location(network, node),
                Vec::new(),
                LadDiagnosticReason::MissingOperand,
            );
            return;
        };
        let location = LadLocation {
            operand: Some(operand.id),
            ..self.node_location(network, node)
        };
        let (actual, compatible) = match &operand.value {
            LadOperand::Constant(value) => {
                (canonical_type(value), value.is_compatible_with(expected))
            }
            LadOperand::Variable(variable) => self
                .resolve_variable(variable)
                .map(|member| member.data_type.clone())
                .map_or((None, false), |actual| {
                    let compatible = &actual == expected;
                    (Some(actual), compatible)
                }),
            LadOperand::Unresolved { spelling } => {
                self.push(
                    DiagnosticCode::UNRESOLVED_REFERENCE,
                    DiagnosticSeverity::Error,
                    true,
                    location,
                    Vec::new(),
                    LadDiagnosticReason::UnresolvedOperand(spelling.clone()),
                );
                return;
            }
            LadOperand::Expression { .. } => {
                self.push(
                    DiagnosticCode::CAPABILITY_UNAVAILABLE,
                    DiagnosticSeverity::Error,
                    true,
                    location,
                    Vec::new(),
                    LadDiagnosticReason::ExpressionUnavailable,
                );
                return;
            }
        };
        if !compatible {
            self.push(
                DiagnosticCode::TYPE_MISMATCH,
                DiagnosticSeverity::Error,
                true,
                location,
                Vec::new(),
                LadDiagnosticReason::TypeMismatch {
                    expected: expected.clone(),
                    actual,
                },
            );
        }
    }

    fn validate_write_operand(
        &mut self,
        network: LadNetworkId,
        node: LadNodeId,
        operand: Option<&LadOperandRef>,
        expected: &DataType,
    ) {
        self.validate_read_operand(network, node, operand, expected);
        let Some(LadOperandRef {
            id,
            value: LadOperand::Variable(variable),
        }) = operand
        else {
            if let Some(operand) = operand {
                self.push(
                    DiagnosticCode::ILLEGAL_OR_OVERLAPPING_BINDING,
                    DiagnosticSeverity::Error,
                    true,
                    LadLocation {
                        operand: Some(operand.id),
                        ..self.node_location(network, node)
                    },
                    Vec::new(),
                    LadDiagnosticReason::ReadOnlyTarget,
                );
            }
            return;
        };
        let writable = self.resolve_variable(variable).is_some_and(|member| {
            !matches!(member.role, InterfaceRole::Input | InterfaceRole::Constant)
        });
        let location = LadLocation {
            operand: Some(*id),
            member: Some(variable_member(variable)),
            ..self.node_location(network, node)
        };
        if !writable {
            self.push(
                DiagnosticCode::ILLEGAL_OR_OVERLAPPING_BINDING,
                DiagnosticSeverity::Error,
                true,
                location.clone(),
                Vec::new(),
                LadDiagnosticReason::ReadOnlyTarget,
            );
        }
        self.writers
            .entry(variable.clone())
            .or_default()
            .push(location);
    }

    fn resolve_variable(&self, variable: &VariableRef) -> Option<&InterfaceMember> {
        match variable {
            VariableRef::CallerMember(member) => self.owner?.interface.member(*member),
            VariableRef::DataBlockMember { data_block, member } => {
                self.program.block(*data_block)?.interface.member(*member)
            }
        }
    }

    fn emit_multiple_writers(&mut self) {
        let repeated: Vec<_> = self
            .writers
            .iter()
            .filter(|(_, locations)| locations.len() > 1)
            .map(|(variable, locations)| (variable.clone(), locations.clone()))
            .collect();
        for (variable, mut locations) in repeated {
            locations.sort();
            let primary = locations
                .last()
                .cloned()
                .unwrap_or_else(|| self.location(None));
            let related = locations[..locations.len().saturating_sub(1)].to_vec();
            self.push(
                DiagnosticCode::MULTIPLE_WRITER,
                DiagnosticSeverity::Warning,
                false,
                LadLocation {
                    member: Some(variable_member(&variable)),
                    ..primary
                },
                related,
                LadDiagnosticReason::MultipleWriter,
            );
        }
    }

    fn graph(&mut self, network: Option<LadNetworkId>, reason: LadGraphReason) {
        self.push(
            DiagnosticCode::INVALID_POWER_GRAPH,
            DiagnosticSeverity::Error,
            true,
            self.location(network),
            Vec::new(),
            LadDiagnosticReason::Graph(reason),
        );
    }

    fn edge_graph(&mut self, network: LadNetworkId, edge: LadEdgeId, reason: LadGraphReason) {
        self.push(
            DiagnosticCode::INVALID_POWER_GRAPH,
            DiagnosticSeverity::Error,
            true,
            LadLocation {
                edge: Some(edge),
                ..self.location(Some(network))
            },
            Vec::new(),
            LadDiagnosticReason::Graph(reason),
        );
    }

    fn branch_graph(
        &mut self,
        network: LadNetworkId,
        branch: LadBranchId,
        path: Option<LadBranchPathId>,
        reason: LadGraphReason,
    ) {
        self.push(
            DiagnosticCode::INVALID_POWER_GRAPH,
            DiagnosticSeverity::Error,
            true,
            LadLocation {
                branch: Some(branch),
                branch_path: path,
                ..self.location(Some(network))
            },
            Vec::new(),
            LadDiagnosticReason::Graph(reason),
        );
    }

    fn resource(
        &mut self,
        network: Option<LadNetworkId>,
        key: &'static str,
        current: usize,
        maximum: usize,
    ) {
        self.push(
            DiagnosticCode::RESOURCE_LIMIT,
            DiagnosticSeverity::Error,
            true,
            self.location(network),
            Vec::new(),
            LadDiagnosticReason::ResourceLimit {
                key,
                current,
                maximum,
            },
        );
    }

    fn push(
        &mut self,
        code: DiagnosticCode,
        severity: DiagnosticSeverity,
        blocking: bool,
        primary: LadLocation,
        related: Vec<LadLocation>,
        reason: LadDiagnosticReason,
    ) {
        self.diagnostics_observed = self.diagnostics_observed.saturating_add(1);
        let effective_limit = self.limits.max_diagnostics.max(1);
        if self.diagnostics.len() < effective_limit {
            self.diagnostics.push(LadDiagnostic {
                code,
                severity,
                blocking,
                primary,
                related,
                reason,
            });
        } else {
            self.diagnostics_truncated = true;
        }
    }

    fn location(&self, network: Option<LadNetworkId>) -> LadLocation {
        LadLocation {
            owner: Some(self.document.owner),
            network,
            ..LadLocation::default()
        }
    }

    fn node_location(&self, network: LadNetworkId, node: LadNodeId) -> LadLocation {
        LadLocation {
            node: Some(node),
            ..self.location(Some(network))
        }
    }

    fn port_location(
        &self,
        network: LadNetworkId,
        node: LadNodeId,
        port: LadPortId,
    ) -> LadLocation {
        LadLocation {
            port: Some(port),
            ..self.node_location(network, node)
        }
    }

    fn pin_location(
        &self,
        network: LadNetworkId,
        node: LadNodeId,
        pin: &LadPin,
        call_site: Option<plc_program::CallSiteId>,
    ) -> LadLocation {
        LadLocation {
            port: Some(pin.id),
            operand: pin.binding.as_ref().map(|binding| binding.id),
            member: match pin.formal {
                Some(LadFormalRef::BlockMember(member)) => Some(member),
                Some(LadFormalRef::Instruction(_)) | None => None,
            },
            instruction_formal: match pin.formal {
                Some(LadFormalRef::Instruction(formal)) => Some(formal),
                Some(LadFormalRef::BlockMember(_)) | None => None,
            },
            call_site,
            ..self.node_location(network, node)
        }
    }
}

#[derive(Default)]
struct GraphIndex {
    port_owner: BTreeMap<LadPortId, (LadNodeId, LadPowerPortDirection)>,
    port_use: BTreeMap<LadPortId, usize>,
    outgoing: BTreeMap<LadNodeId, Vec<IndexedEdge>>,
    incoming: BTreeMap<LadNodeId, Vec<IndexedEdge>>,
}

type IndexedEdge = (LadEdgeId, LadNodeId, LadPortId, LadPortId);

const fn instruction_direction_matches(
    pin: LadPinDirection,
    formal: InstructionFormalDirection,
) -> bool {
    matches!(
        (pin, formal),
        (LadPinDirection::Input, InstructionFormalDirection::Input)
            | (LadPinDirection::Output, InstructionFormalDirection::Output)
            | (LadPinDirection::InOut, InstructionFormalDirection::InOut)
            | (
                LadPinDirection::Activation,
                InstructionFormalDirection::Activation
            )
            | (LadPinDirection::Status, InstructionFormalDirection::Status)
    )
}

fn call_member_required(member: &InterfaceMember) -> bool {
    match member.role {
        InterfaceRole::Input => member.default_value.is_none(),
        InterfaceRole::InOut => true,
        InterfaceRole::Output | InterfaceRole::Return => member.required_output_binding,
        InterfaceRole::Static | InterfaceRole::Temp | InterfaceRole::Constant => false,
    }
}

impl GraphIndex {
    const fn new() -> Self {
        Self {
            port_owner: BTreeMap::new(),
            port_use: BTreeMap::new(),
            outgoing: BTreeMap::new(),
            incoming: BTreeMap::new(),
        }
    }
}

fn ordered_projection_matches<T: Copy + Ord>(
    ordered: &[T],
    keys: impl IntoIterator<Item = T>,
) -> bool {
    let keys: BTreeSet<_> = keys.into_iter().collect();
    ordered.len() == keys.len() && ordered.iter().copied().collect::<BTreeSet<_>>() == keys
}

fn edge_nodes(
    network: &LadNetwork,
    index: &GraphIndex,
    edge: LadEdgeId,
) -> Option<(LadNodeId, LadNodeId)> {
    let edge = network.power_edges.get(&edge)?;
    let source = index.port_owner.get(&edge.source)?.0;
    let target = index.port_owner.get(&edge.target)?.0;
    Some((source, target))
}

fn reachable_before_join(
    start: LadNodeId,
    end: LadNodeId,
    join: LadNodeId,
    index: &GraphIndex,
) -> bool {
    let mut pending = vec![start];
    let mut visited = BTreeSet::new();
    while let Some(node) = pending.pop() {
        if node == end {
            return true;
        }
        if node == join || !visited.insert(node) {
            continue;
        }
        if let Some(edges) = index.outgoing.get(&node) {
            for edge in edges.iter().rev() {
                pending.push(edge.1);
            }
        }
    }
    false
}

fn has_cycle(network: &LadNetwork, index: &GraphIndex) -> bool {
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    network
        .nodes
        .keys()
        .copied()
        .any(|node| visit_cycle(node, index, &mut visiting, &mut visited))
}

fn visit_cycle(
    node: LadNodeId,
    index: &GraphIndex,
    visiting: &mut BTreeSet<LadNodeId>,
    visited: &mut BTreeSet<LadNodeId>,
) -> bool {
    if visited.contains(&node) {
        return false;
    }
    if !visiting.insert(node) {
        return true;
    }
    let cyclic = index.outgoing.get(&node).is_some_and(|edges| {
        edges
            .iter()
            .any(|edge| visit_cycle(edge.1, index, visiting, visited))
    });
    visiting.remove(&node);
    visited.insert(node);
    cyclic
}

fn reachable_nodes(source: LadNodeId, index: &GraphIndex) -> BTreeSet<LadNodeId> {
    let mut pending = vec![source];
    let mut visited = BTreeSet::new();
    while let Some(node) = pending.pop() {
        if !visited.insert(node) {
            continue;
        }
        if let Some(edges) = index.outgoing.get(&node) {
            for edge in edges.iter().rev() {
                pending.push(edge.1);
            }
        }
    }
    visited
}

fn build_execution_order(
    network: &LadNetwork,
    index: &GraphIndex,
    source: LadNodeId,
) -> Vec<LadNodeId> {
    let mut order = Vec::new();
    let mut visited = BTreeSet::new();
    walk_semantic(network, index, source, None, &mut visited, &mut order);
    order
}

fn walk_semantic(
    network: &LadNetwork,
    index: &GraphIndex,
    node: LadNodeId,
    stop: Option<LadNodeId>,
    visited: &mut BTreeSet<LadNodeId>,
    order: &mut Vec<LadNodeId>,
) {
    if Some(node) == stop || !visited.insert(node) {
        return;
    }
    order.push(node);
    let Some(value) = network.nodes.get(&node) else {
        return;
    };
    if let LadNodeKind::BranchSplit { branch } = value.kind
        && let Some(branch) = network.branches.get(&branch)
    {
        for path_id in &branch.ordered_path_ids {
            let Some(path) = branch.paths.get(path_id) else {
                continue;
            };
            let Some((_, start)) = edge_nodes(network, index, path.entry_edge) else {
                continue;
            };
            walk_semantic(
                network,
                index,
                start,
                Some(branch.join_node),
                visited,
                order,
            );
        }
        if visited.insert(branch.join_node) {
            order.push(branch.join_node);
        }
        if let Some(edge) = index
            .outgoing
            .get(&branch.join_node)
            .and_then(|edges| edges.first())
        {
            walk_semantic(network, index, edge.1, stop, visited, order);
        }
        return;
    }
    if let Some(edge) = index.outgoing.get(&node).and_then(|edges| edges.first()) {
        walk_semantic(network, index, edge.1, stop, visited, order);
    }
}

fn canonical_type(value: &plc_program::CanonicalValue) -> Option<DataType> {
    match value {
        plc_program::CanonicalValue::Bool(_) => Some(DataType::Bool),
        plc_program::CanonicalValue::Int(_) => Some(DataType::Int),
        plc_program::CanonicalValue::DInt(_) => Some(DataType::DInt),
        plc_program::CanonicalValue::RealBits(_) => Some(DataType::Real),
        plc_program::CanonicalValue::TimeMilliseconds(_) => Some(DataType::Time),
        plc_program::CanonicalValue::StringBytes(bytes) => u16::try_from(bytes.len())
            .ok()
            .map(|capacity| DataType::String { capacity }),
    }
}

const fn variable_member(variable: &VariableRef) -> InterfaceMemberId {
    match variable {
        VariableRef::CallerMember(member) | VariableRef::DataBlockMember { member, .. } => *member,
    }
}

fn node_operands(node: &crate::LadNode) -> Vec<&LadOperandRef> {
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
