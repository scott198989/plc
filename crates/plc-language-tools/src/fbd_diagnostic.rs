use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::String,
    vec::Vec,
};

use plc_program::{
    CALL_FB, CALL_FC, InterfaceMemberId, SideEffectClass, StateRequirement,
    phase2_instruction_registry,
};

use crate::{
    ActivationRole, ConnectionId, ConnectionKind, FBD_SCHEMA_VERSION, FbdDocument, FbdNetwork,
    InstanceIdentity, NetworkId, NodeId, NodeKind, PortDirection, PortId, PortMultiplicity,
    PortStatus,
};

const MAX_NETWORKS: usize = 4_096;
const MAX_NODES: usize = 65_536;
const MAX_PORTS: usize = 524_288;
const MAX_CONNECTIONS: usize = 524_288;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FbdDiagnosticCode {
    SchemaVersion,
    ResourceLimit,
    MissingNetwork,
    InvalidNetworkOrder,
    InvalidNodeOrder,
    InvalidPortOrder,
    IdentityKeyMismatch,
    DuplicateNodeIdentity,
    DuplicatePortIdentity,
    DuplicateConnectionIdentity,
    InvalidPortShape,
    StalePort,
    OrphanConnection,
    InvalidConnectionDirection,
    IncompatibleDataType,
    MultipleInputSources,
    MissingRequiredInput,
    UnresolvedNode,
    UnavailableInstruction,
    InvalidStateInstance,
    InvalidCall,
    ActivationPortNotDeclared,
    CyclicDependency,
    AmbiguousEffectOrder,
    MultipleWriter,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FbdDiagnostic {
    pub code: FbdDiagnosticCode,
    pub severity: DiagnosticSeverity,
    pub network: Option<NetworkId>,
    pub node: Option<NodeId>,
    pub port: Option<PortId>,
    pub connection: Option<ConnectionId>,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FbdValidationReport {
    pub diagnostics: Vec<FbdDiagnostic>,
    pub schedules: BTreeMap<NetworkId, Vec<NodeId>>,
}

impl FbdValidationReport {
    #[must_use]
    pub fn can_lower(&self) -> bool {
        !self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
    }
}

#[must_use]
pub fn validate_fbd(document: &FbdDocument) -> FbdValidationReport {
    let mut validator = Validator::default();
    validator.validate(document);
    validator.report
}

#[derive(Default)]
struct Validator {
    report: FbdValidationReport,
    node_ids: BTreeSet<NodeId>,
    port_ids: BTreeSet<PortId>,
    connection_ids: BTreeSet<ConnectionId>,
    node_count: usize,
    port_count: usize,
    connection_count: usize,
}

impl Validator {
    fn validate(&mut self, document: &FbdDocument) {
        if document.schema_version != FBD_SCHEMA_VERSION {
            self.issue(
                FbdDiagnosticCode::SchemaVersion,
                DiagnosticSeverity::Error,
                Location::default(),
                "FBD schema version is not supported",
            );
        }
        if document.networks.is_empty() {
            self.issue(
                FbdDiagnosticCode::MissingNetwork,
                DiagnosticSeverity::Error,
                Location::default(),
                "an executable FBD document requires at least one semantic network",
            );
        }
        if document.networks.len() > MAX_NETWORKS {
            self.issue(
                FbdDiagnosticCode::ResourceLimit,
                DiagnosticSeverity::Error,
                Location::default(),
                "FBD network count exceeds the defensive Phase 2 limit",
            );
        }
        if !ordered_projection_is_exact(&document.ordered_network_ids, document.networks.keys()) {
            self.issue(
                FbdDiagnosticCode::InvalidNetworkOrder,
                DiagnosticSeverity::Error,
                Location::default(),
                "stored network order must contain each network exactly once",
            );
        }
        for (index, network_id) in document.ordered_network_ids.iter().enumerate() {
            if let Some(network) = document.networks.get(network_id)
                && usize::try_from(network.semantic_order).ok() != Some(index)
            {
                self.issue(
                    FbdDiagnosticCode::InvalidNetworkOrder,
                    DiagnosticSeverity::Error,
                    Location::network(network.id),
                    "network semantic order must be unique, contiguous, and match stored order",
                );
            }
        }
        for (network_key, network) in &document.networks {
            if *network_key != network.id {
                self.issue(
                    FbdDiagnosticCode::IdentityKeyMismatch,
                    DiagnosticSeverity::Error,
                    Location::network(network.id),
                    "network map key and stable network identity disagree",
                );
            }
            self.validate_network(network);
        }
        if self.node_count > MAX_NODES
            || self.port_count > MAX_PORTS
            || self.connection_count > MAX_CONNECTIONS
        {
            self.issue(
                FbdDiagnosticCode::ResourceLimit,
                DiagnosticSeverity::Error,
                Location::default(),
                "FBD graph exceeds a defensive Phase 2 node, port, or connection limit",
            );
        }
    }

    #[allow(clippy::too_many_lines)]
    fn validate_network(&mut self, network: &FbdNetwork) {
        if !ordered_projection_is_exact(&network.ordered_node_ids, network.nodes.keys()) {
            self.issue(
                FbdDiagnosticCode::InvalidNodeOrder,
                DiagnosticSeverity::Error,
                Location::network(network.id),
                "stored node order must contain each node exactly once",
            );
        }
        let mut semantic_orders = BTreeSet::new();
        for (index, node_id) in network.ordered_node_ids.iter().enumerate() {
            if let Some(node) = network.nodes.get(node_id)
                && (usize::try_from(node.semantic_order).ok() != Some(index)
                    || !semantic_orders.insert(node.semantic_order))
            {
                self.issue(
                    FbdDiagnosticCode::AmbiguousEffectOrder,
                    DiagnosticSeverity::Error,
                    Location::node(network.id, node.id),
                    "node semantic order must be unique, contiguous, and match stored order",
                );
            }
        }

        let mut ports = BTreeMap::new();
        for (node_key, node) in &network.nodes {
            self.node_count = self.node_count.saturating_add(1);
            if *node_key != node.id {
                self.issue(
                    FbdDiagnosticCode::IdentityKeyMismatch,
                    DiagnosticSeverity::Error,
                    Location::node(network.id, node.id),
                    "node map key and stable node identity disagree",
                );
            }
            if !self.node_ids.insert(node.id) {
                self.issue(
                    FbdDiagnosticCode::DuplicateNodeIdentity,
                    DiagnosticSeverity::Error,
                    Location::node(network.id, node.id),
                    "node identity is reused in another network",
                );
            }
            self.validate_node(network.id, node);
            for (port_key, port) in &node.ports {
                self.port_count = self.port_count.saturating_add(1);
                if *port_key != port.id {
                    self.issue(
                        FbdDiagnosticCode::IdentityKeyMismatch,
                        DiagnosticSeverity::Error,
                        Location::port(network.id, node.id, port.id),
                        "port map key and stable port identity disagree",
                    );
                }
                if !self.port_ids.insert(port.id)
                    || ports.insert(port.id, (node.id, port)).is_some()
                {
                    self.issue(
                        FbdDiagnosticCode::DuplicatePortIdentity,
                        DiagnosticSeverity::Error,
                        Location::port(network.id, node.id, port.id),
                        "port identity is reused",
                    );
                }
            }
        }

        let mut incoming = BTreeMap::<PortId, usize>::new();
        let mut dependencies = BTreeSet::<(NodeId, NodeId)>::new();
        for (connection_key, connection) in &network.connections {
            self.connection_count = self.connection_count.saturating_add(1);
            if *connection_key != connection.id {
                self.issue(
                    FbdDiagnosticCode::IdentityKeyMismatch,
                    DiagnosticSeverity::Error,
                    Location::connection(network.id, connection.id),
                    "connection map key and stable connection identity disagree",
                );
            }
            if !self.connection_ids.insert(connection.id) {
                self.issue(
                    FbdDiagnosticCode::DuplicateConnectionIdentity,
                    DiagnosticSeverity::Error,
                    Location::connection(network.id, connection.id),
                    "connection identity is reused in another network",
                );
            }
            let Some(&(source_node, source_port)) = ports.get(&connection.source) else {
                self.issue(
                    FbdDiagnosticCode::OrphanConnection,
                    DiagnosticSeverity::Error,
                    Location::connection(network.id, connection.id),
                    "connection source port does not exist in this network",
                );
                continue;
            };
            let Some(&(target_node, target_port)) = ports.get(&connection.target) else {
                self.issue(
                    FbdDiagnosticCode::OrphanConnection,
                    DiagnosticSeverity::Error,
                    Location::connection(network.id, connection.id),
                    "connection target port does not exist in this network",
                );
                continue;
            };
            *incoming.entry(target_port.id).or_default() += 1;
            if !connection_shape_is_valid(
                connection.kind,
                source_port.direction,
                target_port.direction,
            ) {
                self.issue(
                    FbdDiagnosticCode::InvalidConnectionDirection,
                    DiagnosticSeverity::Error,
                    Location::connection(network.id, connection.id),
                    "connection must run output-to-input in its declared data or execution domain",
                );
                continue;
            }
            if connection.kind == ConnectionKind::Data
                && source_port.data_type != target_port.data_type
            {
                self.issue(
                    FbdDiagnosticCode::IncompatibleDataType,
                    DiagnosticSeverity::Error,
                    Location::connection(network.id, connection.id),
                    "connected ports require one exact canonical type; no implicit conversion is available",
                );
            }
            dependencies.insert((source_node, target_node));
        }

        for (port_id, count) in &incoming {
            let Some(&(node_id, port)) = ports.get(port_id) else {
                continue;
            };
            if port.direction == PortDirection::Input && *count > 1 {
                self.issue(
                    FbdDiagnosticCode::MultipleInputSources,
                    DiagnosticSeverity::Error,
                    Location::port(network.id, node_id, *port_id),
                    "ordinary FBD data input has more than one source",
                );
            }
            if port.multiplicity == PortMultiplicity::One && *count > 1 {
                self.issue(
                    FbdDiagnosticCode::MultipleInputSources,
                    DiagnosticSeverity::Error,
                    Location::port(network.id, node_id, *port_id),
                    "single-source port has more than one incoming connection",
                );
            }
        }
        for (&port_id, &(node_id, port)) in &ports {
            if port.required
                && matches!(
                    port.direction,
                    PortDirection::Input | PortDirection::ExecutionInput
                )
                && incoming.get(&port_id).copied().unwrap_or(0) == 0
            {
                self.issue(
                    FbdDiagnosticCode::MissingRequiredInput,
                    DiagnosticSeverity::Error,
                    Location::port(network.id, node_id, port_id),
                    "required input is not connected",
                );
            }
        }

        match schedule_network(network, &dependencies) {
            Some(schedule) => {
                self.check_multiple_writers(network, &schedule);
                self.report.schedules.insert(network.id, schedule);
            }
            None => self.issue(
                FbdDiagnosticCode::CyclicDependency,
                DiagnosticSeverity::Error,
                Location::network(network.id),
                "the union of data and execution dependencies contains a cycle",
            ),
        }
    }

    fn validate_node(&mut self, network: NetworkId, node: &crate::FbdNode) {
        if !ordered_projection_is_exact(&node.ordered_port_ids, node.ports.keys()) {
            self.issue(
                FbdDiagnosticCode::InvalidPortOrder,
                DiagnosticSeverity::Error,
                Location::node(network, node.id),
                "stored port order must contain each port exactly once",
            );
        }
        for port in node.ports.values() {
            let shape_valid = match port.direction {
                PortDirection::Input | PortDirection::Output => port.data_type.is_some(),
                PortDirection::ExecutionInput | PortDirection::ExecutionOutput => {
                    port.data_type.is_none()
                }
            };
            if !shape_valid {
                self.issue(
                    FbdDiagnosticCode::InvalidPortShape,
                    DiagnosticSeverity::Error,
                    Location::port(network, node.id, port.id),
                    "data ports require a canonical type and execution ports must be untyped",
                );
            }
            if port.status == PortStatus::Stale {
                self.issue(
                    FbdDiagnosticCode::StalePort,
                    DiagnosticSeverity::Error,
                    Location::port(network, node.id, port.id),
                    "stale port is preserved for repair but cannot be lowered",
                );
            }
            if port.activation != ActivationRole::None {
                self.issue(
                    FbdDiagnosticCode::ActivationPortNotDeclared,
                    DiagnosticSeverity::Error,
                    Location::port(network, node.id, port.id),
                    "EN or ENO is not declared by the shared instruction registry",
                );
            }
        }
        self.validate_node_kind(network, node);
    }

    fn validate_node_kind(&mut self, network: NetworkId, node: &crate::FbdNode) {
        match &node.kind {
            NodeKind::Unresolved { .. } => self.issue(
                FbdDiagnosticCode::UnresolvedNode,
                DiagnosticSeverity::Error,
                Location::node(network, node.id),
                "unresolved block is retained as editable invalid graph state",
            ),
            NodeKind::Instruction { code, instance } => {
                let Some(definition) = phase2_instruction_registry().lookup(*code) else {
                    self.issue(
                        FbdDiagnosticCode::UnavailableInstruction,
                        DiagnosticSeverity::Error,
                        Location::node(network, node.id),
                        "instruction code is not in the shared Phase 2 registry",
                    );
                    return;
                };
                let valid_instance = match definition.state_requirement {
                    StateRequirement::None => instance.is_none(),
                    StateRequirement::Explicit(_) => {
                        matches!(instance, Some(InstanceIdentity::Instruction(_)))
                    }
                    StateRequirement::FunctionBlockInstance => {
                        matches!(instance, Some(InstanceIdentity::FunctionBlock { .. }))
                    }
                };
                if !valid_instance {
                    self.issue(
                        FbdDiagnosticCode::InvalidStateInstance,
                        DiagnosticSeverity::Error,
                        Location::node(network, node.id),
                        "instruction state requirement and explicit instance identity disagree",
                    );
                }
            }
            NodeKind::Call {
                code,
                target,
                instance,
            } => {
                let valid = if *code == CALL_FC {
                    instance.is_none()
                } else if *code == CALL_FB {
                    matches!(instance, Some(InstanceIdentity::FunctionBlock { .. }))
                } else {
                    false
                };
                if !valid || target.get() == 0 {
                    self.issue(
                        FbdDiagnosticCode::InvalidCall,
                        DiagnosticSeverity::Error,
                        Location::node(network, node.id),
                        "call requires CALL_FC without state or CALL_FB with explicit instance identity",
                    );
                }
            }
            NodeKind::Constant { .. }
            | NodeKind::LoadMember { .. }
            | NodeKind::StoreMember { .. } => {}
        }
    }

    fn check_multiple_writers(&mut self, network: &FbdNetwork, schedule: &[NodeId]) {
        let mut writers = BTreeMap::<InterfaceMemberId, Vec<NodeId>>::new();
        for node_id in schedule {
            let Some(node) = network.nodes.get(node_id) else {
                continue;
            };
            if let NodeKind::StoreMember { member } = node.kind {
                writers.entry(member).or_default().push(node.id);
            }
        }
        for nodes in writers.values().filter(|nodes| nodes.len() > 1) {
            let last = nodes.last().copied();
            self.issue(
                FbdDiagnosticCode::MultipleWriter,
                DiagnosticSeverity::Warning,
                Location {
                    network: Some(network.id),
                    node: last,
                    ..Location::default()
                },
                "multiple nodes write one symbol; the later scheduled write wins",
            );
        }

        for node in network.nodes.values() {
            let effectful = match &node.kind {
                NodeKind::StoreMember { .. } | NodeKind::Call { .. } => true,
                NodeKind::Instruction { code, .. } => phase2_instruction_registry()
                    .lookup(*code)
                    .is_some_and(|definition| definition.side_effect != SideEffectClass::Pure),
                NodeKind::Constant { .. }
                | NodeKind::LoadMember { .. }
                | NodeKind::Unresolved { .. } => false,
            };
            if effectful && !network.ordered_node_ids.contains(&node.id) {
                self.issue(
                    FbdDiagnosticCode::AmbiguousEffectOrder,
                    DiagnosticSeverity::Error,
                    Location::node(network.id, node.id),
                    "effectful node needs an execution dependency or stored semantic order",
                );
            }
        }
    }

    fn issue(
        &mut self,
        code: FbdDiagnosticCode,
        severity: DiagnosticSeverity,
        location: Location,
        message: impl Into<String>,
    ) {
        self.report.diagnostics.push(FbdDiagnostic {
            code,
            severity,
            network: location.network,
            node: location.node,
            port: location.port,
            connection: location.connection,
            message: message.into(),
        });
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Location {
    network: Option<NetworkId>,
    node: Option<NodeId>,
    port: Option<PortId>,
    connection: Option<ConnectionId>,
}

impl Location {
    const fn network(network: NetworkId) -> Self {
        Self {
            network: Some(network),
            node: None,
            port: None,
            connection: None,
        }
    }

    const fn node(network: NetworkId, node: NodeId) -> Self {
        Self {
            network: Some(network),
            node: Some(node),
            port: None,
            connection: None,
        }
    }

    const fn port(network: NetworkId, node: NodeId, port: PortId) -> Self {
        Self {
            network: Some(network),
            node: Some(node),
            port: Some(port),
            connection: None,
        }
    }

    const fn connection(network: NetworkId, connection: ConnectionId) -> Self {
        Self {
            network: Some(network),
            node: None,
            port: None,
            connection: Some(connection),
        }
    }
}

fn ordered_projection_is_exact<'a, T: 'a + Copy + Ord>(
    projection: &[T],
    keys: impl Iterator<Item = &'a T>,
) -> bool {
    let expected: BTreeSet<_> = keys.copied().collect();
    let actual: BTreeSet<_> = projection.iter().copied().collect();
    projection.len() == expected.len() && actual == expected
}

const fn connection_shape_is_valid(
    kind: ConnectionKind,
    source: PortDirection,
    target: PortDirection,
) -> bool {
    matches!(
        (kind, source, target),
        (
            ConnectionKind::Data,
            PortDirection::Output,
            PortDirection::Input
        ) | (
            ConnectionKind::Execution,
            PortDirection::ExecutionOutput,
            PortDirection::ExecutionInput
        )
    )
}

fn schedule_network(
    network: &FbdNetwork,
    dependencies: &BTreeSet<(NodeId, NodeId)>,
) -> Option<Vec<NodeId>> {
    let mut indegree: BTreeMap<NodeId, usize> = network
        .nodes
        .keys()
        .copied()
        .map(|node| (node, 0))
        .collect();
    let mut outgoing = BTreeMap::<NodeId, BTreeSet<NodeId>>::new();
    for &(source, target) in dependencies {
        if !network.nodes.contains_key(&source) || !network.nodes.contains_key(&target) {
            continue;
        }
        if outgoing.entry(source).or_default().insert(target) {
            *indegree.entry(target).or_default() += 1;
        }
    }
    let mut ready = BTreeSet::<(u32, NodeId)>::new();
    for (&node_id, &degree) in &indegree {
        if degree == 0 {
            let order = network.nodes[&node_id].semantic_order;
            ready.insert((order, node_id));
        }
    }
    let mut schedule = Vec::with_capacity(network.nodes.len());
    while let Some(&(order, node_id)) = ready.first() {
        ready.remove(&(order, node_id));
        schedule.push(node_id);
        if let Some(targets) = outgoing.get(&node_id) {
            for &target in targets {
                let degree = indegree.get_mut(&target)?;
                *degree = degree.checked_sub(1)?;
                if *degree == 0 {
                    ready.insert((network.nodes[&target].semantic_order, target));
                }
            }
        }
    }
    (schedule.len() == network.nodes.len()).then_some(schedule)
}
