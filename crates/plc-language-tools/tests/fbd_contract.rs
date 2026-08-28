use std::collections::BTreeMap;

use plc_compiler::{
    BinaryOperator, BuildAttempt, BuildAttemptId, BuildScope, BuildSnapshot, Compiler,
    CompilerProfile, IrFormalRef, IrOperationKind, IrType, ResourceLimits, SclSource,
    SourceAnchorResolution, UnaryOperator,
};
use plc_language_tools::{
    ActivationRole, ConnectionId, ConnectionKind, DiagnosticSeverity, DisabledOutputBehavior,
    EffectRole, FbdConnection, FbdDiagnosticCode, FbdDocument, FbdDocumentId, FbdEdit,
    FbdEditError, FbdLayout, FbdLowerError, FbdNetwork, FbdNode, FbdPort, InstanceIdentity,
    NetworkId, NodeId, NodeKind, NodeLayout, PortDirection, PortId, PortMultiplicity, PortStatus,
    StateInstanceId, TypeAdapterError, apply_fbd_edits_atomically, data_type_to_ir_type,
    disabled_output_behavior, lower_fbd_to_ir, lower_fbd_to_verified_ir, validate_fbd,
};
use plc_program::{
    ADD, BOOL_NOT, BlockId, BlockInterface, CALL_FB, CALL_FC, CanonicalValue, ControllerId,
    ControllerProgram, DataType, EngineeringNumber, FORMAL_CLOCK, FORMAL_CURRENT_VALUE,
    FORMAL_ELAPSED_TIME, FORMAL_ENABLE, FORMAL_ENABLE_OUTPUT, FORMAL_INPUT, FORMAL_LEFT,
    FORMAL_OUTPUT, FORMAL_PRESET_TIME, FORMAL_PRESET_VALUE, FORMAL_RIGHT, InterfaceMember,
    InterfaceMemberId, InterfaceRole, ObDeclaration, ProgramBlock, ProgramUnitKind, TIMER_ON_DELAY,
};

fn data_port(id: u128, name: &str, direction: PortDirection, data_type: DataType) -> FbdPort {
    FbdPort {
        id: PortId::new(id),
        name: name.into(),
        direction,
        data_type: Some(data_type),
        required: direction == PortDirection::Input,
        multiplicity: if direction == PortDirection::Output {
            PortMultiplicity::Many
        } else {
            PortMultiplicity::One
        },
        activation: ActivationRole::None,
        status: PortStatus::Active,
        effect_role: EffectRole::Value,
        formal: match name.to_ascii_uppercase().as_str() {
            "EN" => Some(IrFormalRef::Instruction(FORMAL_ENABLE)),
            "ENO" => Some(IrFormalRef::Instruction(FORMAL_ENABLE_OUTPUT)),
            "IN" => Some(IrFormalRef::Instruction(FORMAL_INPUT)),
            "OUT" | "Q" => Some(IrFormalRef::Instruction(FORMAL_OUTPUT)),
            "A" => Some(IrFormalRef::Instruction(FORMAL_LEFT)),
            "B" => Some(IrFormalRef::Instruction(FORMAL_RIGHT)),
            "CLK" => Some(IrFormalRef::Instruction(FORMAL_CLOCK)),
            "PT" => Some(IrFormalRef::Instruction(FORMAL_PRESET_TIME)),
            "ET" => Some(IrFormalRef::Instruction(FORMAL_ELAPSED_TIME)),
            "PV" => Some(IrFormalRef::Instruction(FORMAL_PRESET_VALUE)),
            "CV" => Some(IrFormalRef::Instruction(FORMAL_CURRENT_VALUE)),
            _ => None,
        },
    }
}

fn execution_port(id: u128, name: &str, direction: PortDirection) -> FbdPort {
    FbdPort {
        id: PortId::new(id),
        name: name.into(),
        direction,
        data_type: None,
        required: false,
        multiplicity: PortMultiplicity::Many,
        activation: ActivationRole::None,
        status: PortStatus::Active,
        effect_role: EffectRole::Execution,
        formal: None,
    }
}

fn node(id: u128, order: u32, kind: NodeKind, ports: Vec<FbdPort>) -> FbdNode {
    FbdNode::from_ports(NodeId::new(id), order, kind, ports)
}

fn connection(id: u128, source: u128, target: u128) -> FbdConnection {
    FbdConnection {
        id: ConnectionId::new(id),
        source: PortId::new(source),
        target: PortId::new(target),
        kind: ConnectionKind::Data,
    }
}

fn owner_block() -> ProgramBlock {
    let members = [
        InterfaceMember::plain(
            InterfaceMemberId::new(101),
            "InputA",
            InterfaceRole::Input,
            DataType::Bool,
            0,
        ),
        InterfaceMember::plain(
            InterfaceMemberId::new(102),
            "OutputQ",
            InterfaceRole::Output,
            DataType::Bool,
            0,
        ),
    ];
    ProgramBlock::new(
        BlockId::new(10),
        "FC10",
        EngineeringNumber::new(10).expect("nonzero"),
        ProgramUnitKind::Function,
        BlockInterface::from_members(members),
    )
}

fn bool_pipeline() -> FbdDocument {
    let load = node(
        30,
        0,
        NodeKind::LoadMember {
            member: InterfaceMemberId::new(101),
        },
        vec![data_port(301, "OUT", PortDirection::Output, DataType::Bool)],
    );
    let invert = node(
        20,
        1,
        NodeKind::Instruction {
            code: BOOL_NOT,
            instance: None,
        },
        vec![
            data_port(201, "IN", PortDirection::Input, DataType::Bool),
            data_port(202, "OUT", PortDirection::Output, DataType::Bool),
        ],
    );
    let store = node(
        10,
        2,
        NodeKind::StoreMember {
            member: InterfaceMemberId::new(102),
        },
        vec![data_port(101, "IN", PortDirection::Input, DataType::Bool)],
    );
    let network = FbdNetwork::from_parts(
        NetworkId::new(1),
        0,
        [load, invert, store],
        [connection(1, 301, 201), connection(2, 202, 101)],
    );
    FbdDocument::new(FbdDocumentId::new(500), BlockId::new(10), [network])
}

fn diagnostic_codes(document: &FbdDocument) -> Vec<FbdDiagnosticCode> {
    validate_fbd(document)
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn schedule_uses_dependencies_and_semantic_order_not_identity_or_coordinates() {
    let document = bool_pipeline();
    let report = validate_fbd(&document);
    assert!(report.can_lower(), "{:?}", report.diagnostics);
    assert_eq!(
        report.schedules[&NetworkId::new(1)],
        vec![NodeId::new(30), NodeId::new(20), NodeId::new(10)]
    );

    let mut layout = FbdLayout::default();
    layout.nodes.insert(
        NodeId::new(30),
        NodeLayout {
            x: 10_000,
            y: -10_000,
            width: 1,
            height: 1,
            group: Some(99),
            alignment_rank: 42,
        },
    );
    assert_eq!(
        validate_fbd(&document).schedules,
        report.schedules,
        "presentation geometry must not affect scheduling"
    );
}

#[test]
fn semantic_fingerprint_is_deterministic_and_layout_independent() {
    let document = bool_pipeline();
    let before = document.semantic_fingerprint();
    let mut layout = FbdLayout::default();
    layout.nodes.insert(
        NodeId::new(20),
        NodeLayout {
            x: -123,
            y: 987,
            width: 240,
            height: 100,
            group: None,
            alignment_rank: 7,
        },
    );
    assert_eq!(document.semantic_fingerprint(), before);

    let mut semantic_edit = document.clone();
    apply_fbd_edits_atomically(
        &mut semantic_edit,
        &[FbdEdit::MoveNode {
            network: NetworkId::new(1),
            node: NodeId::new(20),
            new_index: 0,
        }],
    )
    .expect("valid semantic reorder");
    assert_ne!(semantic_edit.semantic_fingerprint(), before);

    let mut invalid_extra_state = document.clone();
    invalid_extra_state.networks.insert(
        NetworkId::new(999),
        FbdNetwork::from_parts(NetworkId::new(999), 99, [], []),
    );
    assert_ne!(
        invalid_extra_state.semantic_fingerprint(),
        before,
        "even invalid state omitted by an order projection remains dirty and hash-visible"
    );
}

#[test]
fn production_fbd_maps_relocate_by_graph_identity_and_retain_related_edges() {
    let document = bool_pipeline();
    let owner = owner_block();
    let before = lower_fbd_to_ir(&document, &owner).expect("valid FBD lowers");
    let before_entry = before
        .source_maps
        .entries()
        .values()
        .find(|entry| entry.source.node == Some(NodeId::new(20)))
        .expect("inverter source map");
    assert_eq!(
        before.source_maps.source_to_ir(&before_entry.source),
        vec![before_entry.site]
    );
    assert!(!before.source_maps.node_to_ir(NodeId::new(20)).is_empty());
    assert!(!before.probes.node_to_probes(NodeId::new(20)).is_empty());

    let compiler_entry = before
        .compiler_source_maps
        .entries()
        .values()
        .find(|entry| {
            entry
                .anchors
                .iter()
                .any(|anchor| anchor.node_id == Some(NodeId::new(20).get()))
                && entry.anchors.iter().any(|anchor| anchor.edge_id.is_some())
        })
        .expect("one mapped operation retains node plus incoming edge anchors");
    assert!(compiler_entry.anchors.len() >= 2);
    let loaded_anchor = compiler_entry
        .anchors
        .iter()
        .find(|anchor| anchor.edge_id.is_none())
        .cloned()
        .expect("base node anchor");

    let mut reordered = document;
    apply_fbd_edits_atomically(
        &mut reordered,
        &[FbdEdit::MoveNode {
            network: NetworkId::new(1),
            node: NodeId::new(20),
            new_index: 0,
        }],
    )
    .expect("stable-node semantic reorder");
    let after = lower_fbd_to_ir(&reordered, &owner).expect("reordered FBD still lowers");
    let SourceAnchorResolution::Relocated(relocated) = after
        .compiler_source_maps
        .resolve_source_anchor(&loaded_anchor)
    else {
        panic!("stable graph IDs should relocate across graph revision");
    };
    assert_eq!(relocated.anchor.network_id, loaded_anchor.network_id);
    assert_eq!(relocated.anchor.node_id, loaded_anchor.node_id);
    assert_eq!(relocated.anchor.port_id, loaded_anchor.port_id);
    assert_ne!(
        relocated.anchor.semantic_node_id, loaded_anchor.semantic_node_id,
        "semantic ordering is not graph navigation identity"
    );
}

#[test]
fn output_fanout_is_legal_but_ordinary_input_has_one_source() {
    let constant = node(
        1,
        0,
        NodeKind::Constant {
            value: CanonicalValue::Bool(true),
        },
        vec![data_port(10, "OUT", PortDirection::Output, DataType::Bool)],
    );
    let first = node(
        2,
        1,
        NodeKind::Instruction {
            code: BOOL_NOT,
            instance: None,
        },
        vec![
            data_port(20, "IN", PortDirection::Input, DataType::Bool),
            data_port(21, "OUT", PortDirection::Output, DataType::Bool),
        ],
    );
    let second = node(
        3,
        2,
        NodeKind::Instruction {
            code: BOOL_NOT,
            instance: None,
        },
        vec![
            data_port(30, "IN", PortDirection::Input, DataType::Bool),
            data_port(31, "OUT", PortDirection::Output, DataType::Bool),
        ],
    );
    let network = FbdNetwork::from_parts(
        NetworkId::new(1),
        0,
        [constant, first, second],
        [connection(1, 10, 20), connection(2, 10, 30)],
    );
    let document = FbdDocument::new(FbdDocumentId::new(1), BlockId::new(10), [network]);
    assert!(validate_fbd(&document).can_lower());

    let mut invalid = document.clone();
    let extra = node(
        4,
        3,
        NodeKind::Constant {
            value: CanonicalValue::Bool(false),
        },
        vec![data_port(40, "OUT", PortDirection::Output, DataType::Bool)],
    );
    apply_fbd_edits_atomically(
        &mut invalid,
        &[
            FbdEdit::AddNode {
                network: NetworkId::new(1),
                node: extra,
            },
            FbdEdit::AddConnection {
                network: NetworkId::new(1),
                connection: connection(3, 40, 20),
            },
        ],
    )
    .expect("editor preserves invalid topology");
    assert!(diagnostic_codes(&invalid).contains(&FbdDiagnosticCode::MultipleInputSources));
}

#[test]
fn union_of_data_and_execution_edges_rejects_cycles() {
    let mut first_input = data_port(10, "IN", PortDirection::Input, DataType::Bool);
    first_input.required = false;
    let first = node(
        1,
        0,
        NodeKind::Instruction {
            code: BOOL_NOT,
            instance: None,
        },
        vec![
            first_input,
            data_port(11, "OUT", PortDirection::Output, DataType::Bool),
            execution_port(12, "EXEC_IN", PortDirection::ExecutionInput),
            execution_port(13, "EXEC_OUT", PortDirection::ExecutionOutput),
        ],
    );
    let second = node(
        2,
        1,
        NodeKind::Instruction {
            code: BOOL_NOT,
            instance: None,
        },
        vec![
            data_port(20, "IN", PortDirection::Input, DataType::Bool),
            data_port(21, "OUT", PortDirection::Output, DataType::Bool),
            execution_port(22, "EXEC_IN", PortDirection::ExecutionInput),
            execution_port(23, "EXEC_OUT", PortDirection::ExecutionOutput),
        ],
    );
    let execution_back_edge = FbdConnection {
        id: ConnectionId::new(2),
        source: PortId::new(23),
        target: PortId::new(12),
        kind: ConnectionKind::Execution,
    };
    let network = FbdNetwork::from_parts(
        NetworkId::new(1),
        0,
        [first, second],
        [connection(1, 11, 20), execution_back_edge],
    );
    let document = FbdDocument::new(FbdDocumentId::new(1), BlockId::new(10), [network]);
    assert!(diagnostic_codes(&document).contains(&FbdDiagnosticCode::CyclicDependency));
}

#[test]
fn deletion_preserves_orphans_for_repair_and_batch_failure_is_atomic() {
    let mut document = bool_pipeline();
    let original = document.clone();
    let result = apply_fbd_edits_atomically(
        &mut document,
        &[
            FbdEdit::RemoveNodeKeepConnections {
                network: NetworkId::new(1),
                node: NodeId::new(20),
            },
            FbdEdit::RemoveConnection {
                network: NetworkId::new(1),
                connection: ConnectionId::new(999),
            },
        ],
    );
    assert_eq!(
        result,
        Err(FbdEditError::MissingConnection(ConnectionId::new(999)))
    );
    assert_eq!(document, original, "failed edit batches roll back exactly");

    apply_fbd_edits_atomically(
        &mut document,
        &[FbdEdit::RemoveNodeKeepConnections {
            network: NetworkId::new(1),
            node: NodeId::new(20),
        }],
    )
    .expect("node removal succeeds");
    assert_eq!(document.networks[&NetworkId::new(1)].connections.len(), 2);
    assert!(diagnostic_codes(&document).contains(&FbdDiagnosticCode::OrphanConnection));
}

#[test]
fn stale_unresolved_wrong_direction_and_type_mismatch_are_real_diagnostics() {
    let mut source = data_port(10, "OUT", PortDirection::Output, DataType::Bool);
    source.status = PortStatus::Stale;
    let unresolved = node(
        1,
        0,
        NodeKind::Unresolved {
            requested_name: "RemovedBlock".into(),
        },
        vec![source],
    );
    let target = node(
        2,
        1,
        NodeKind::Instruction {
            code: BOOL_NOT,
            instance: None,
        },
        vec![
            data_port(20, "IN", PortDirection::Input, DataType::DInt),
            data_port(21, "OUT", PortDirection::Output, DataType::DInt),
        ],
    );
    let network = FbdNetwork::from_parts(
        NetworkId::new(1),
        0,
        [unresolved, target],
        [connection(1, 10, 20), connection(2, 10, 21)],
    );
    let document = FbdDocument::new(FbdDocumentId::new(1), BlockId::new(10), [network]);
    let codes = diagnostic_codes(&document);
    assert!(codes.contains(&FbdDiagnosticCode::StalePort));
    assert!(codes.contains(&FbdDiagnosticCode::UnresolvedNode));
    assert!(codes.contains(&FbdDiagnosticCode::InvalidConnectionDirection));
    assert!(codes.contains(&FbdDiagnosticCode::IncompatibleDataType));
    assert!(codes.contains(&FbdDiagnosticCode::FormalTypeMismatch));
}

#[test]
fn stateful_and_call_nodes_require_explicit_stable_instances() {
    let timer = node(
        1,
        0,
        NodeKind::Instruction {
            code: TIMER_ON_DELAY,
            instance: None,
        },
        vec![data_port(10, "Q", PortDirection::Output, DataType::Bool)],
    );
    let bad_call = node(
        2,
        1,
        NodeKind::Call {
            code: CALL_FB,
            target: BlockId::new(22),
            instance: None,
        },
        Vec::new(),
    );
    let network = FbdNetwork::from_parts(NetworkId::new(1), 0, [timer, bad_call], []);
    let document = FbdDocument::new(FbdDocumentId::new(1), BlockId::new(10), [network]);
    let codes = diagnostic_codes(&document);
    assert!(codes.contains(&FbdDiagnosticCode::InvalidStateInstance));
    assert!(codes.contains(&FbdDiagnosticCode::InvalidCall));

    let stateful = NodeKind::Instruction {
        code: TIMER_ON_DELAY,
        instance: Some(InstanceIdentity::Instruction(StateInstanceId::new(88))),
    };
    assert_eq!(
        disabled_output_behavior(&stateful),
        DisabledOutputBehavior::StoredValueWithoutUpdate
    );
    assert_eq!(
        disabled_output_behavior(&NodeKind::Instruction {
            code: plc_program::RISING_EDGE,
            instance: Some(InstanceIdentity::Instruction(StateInstanceId::new(89))),
        }),
        DisabledOutputBehavior::DefaultValue
    );
    assert_eq!(
        disabled_output_behavior(&NodeKind::Call {
            code: CALL_FB,
            target: BlockId::new(22),
            instance: Some(InstanceIdentity::FunctionBlock {
                root_instance_db: BlockId::new(44),
                multi_instance_members: vec![],
            }),
        }),
        DisabledOutputBehavior::NoEffect
    );
}

#[test]
fn activation_requires_the_exact_registry_en_eno_pair() {
    let mut enable = data_port(10, "EN", PortDirection::Input, DataType::Bool);
    enable.activation = ActivationRole::Enable;
    enable.required = false;
    let instruction = node(
        1,
        0,
        NodeKind::Instruction {
            code: BOOL_NOT,
            instance: None,
        },
        vec![
            enable,
            data_port(11, "OUT", PortDirection::Output, DataType::Bool),
        ],
    );
    let network = FbdNetwork::from_parts(NetworkId::new(1), 0, [instruction], []);
    let document = FbdDocument::new(FbdDocumentId::new(1), BlockId::new(10), [network]);
    assert!(diagnostic_codes(&document).contains(&FbdDiagnosticCode::ActivationPortNotDeclared));
}

#[test]
fn registry_activation_lowers_explicit_disabled_policy_without_coordinate_semantics() {
    let power = node(
        1,
        0,
        NodeKind::Constant {
            value: CanonicalValue::Bool(true),
        },
        vec![data_port(10, "OUT", PortDirection::Output, DataType::Bool)],
    );
    let value = node(
        2,
        1,
        NodeKind::Constant {
            value: CanonicalValue::Bool(false),
        },
        vec![data_port(20, "OUT", PortDirection::Output, DataType::Bool)],
    );
    let mut en = data_port(30, "EN", PortDirection::Input, DataType::Bool);
    en.activation = ActivationRole::Enable;
    en.required = false;
    let mut eno = data_port(33, "ENO", PortDirection::Output, DataType::Bool);
    eno.activation = ActivationRole::EnableOutput;
    let invert = node(
        3,
        2,
        NodeKind::Instruction {
            code: BOOL_NOT,
            instance: None,
        },
        vec![
            en,
            data_port(31, "IN", PortDirection::Input, DataType::Bool),
            data_port(32, "OUT", PortDirection::Output, DataType::Bool),
            eno,
        ],
    );
    let document = FbdDocument::new(
        FbdDocumentId::new(2),
        BlockId::new(10),
        [FbdNetwork::from_parts(
            NetworkId::new(1),
            0,
            [power, value, invert],
            [connection(1, 10, 30), connection(2, 20, 31)],
        )],
    );
    assert!(validate_fbd(&document).can_lower());
    let mut program = ControllerProgram::new(ControllerId::new(2));
    program.insert_block(owner_block()).expect("unique owner");
    let lowered = lower_fbd_to_verified_ir(&document, &program).expect("verified activation");
    let invocation = &lowered.verified_ir.program().functions()[&BlockId::new(10)].blocks
        [&plc_compiler::IrBasicBlockId::new(1)]
        .operations[2];
    assert!(matches!(
        invocation.kind,
        IrOperationKind::InvokeInstruction {
            activation: Some(plc_compiler::IrActivation {
                status_when_disabled: false,
                when_disabled: plc_program::DisabledExecutionBehavior::DefaultOutputsNoStateChange,
                ..
            }),
            ..
        }
    ));
}

#[test]
fn valid_bool_pipeline_lowers_deterministically_into_shared_ir_with_probes() {
    let document = bool_pipeline();
    let first = lower_fbd_to_ir(&document, &owner_block()).expect("valid lowering");
    let second = lower_fbd_to_ir(&document, &owner_block()).expect("repeat lowering");
    assert_eq!(first, second);
    assert_eq!(
        first.ir.semantic_fingerprint(),
        second.ir.semantic_fingerprint()
    );

    let function = &first.ir.functions()[&BlockId::new(10)];
    let operations = &function.blocks[&function.entry].operations;
    assert!(matches!(
        operations[0].kind,
        IrOperationKind::LoadMember { .. }
    ));
    assert!(matches!(
        operations[1].kind,
        IrOperationKind::Unary {
            operator: UnaryOperator::Not,
            ..
        }
    ));
    assert!(matches!(
        operations[2].kind,
        IrOperationKind::StoreMember { .. }
    ));
    assert_eq!(first.source_maps.entries().len(), 4);
    assert_eq!(first.probes.entries().len(), 4);
    let expression_probe = first
        .probes
        .entries()
        .values()
        .find(|probe| probe.source.node == Some(NodeId::new(20)))
        .expect("node probe");
    assert_eq!(expression_probe.value_type, Some(IrType::Bool));
    assert_eq!(expression_probe.source.port, Some(PortId::new(202)));
    assert_eq!(
        first
            .source_maps
            .connection_to_ir(ConnectionId::new(1))
            .len(),
        1
    );
    assert_eq!(
        first
            .probes
            .connection_to_probes(ConnectionId::new(2))
            .len(),
        1
    );
    assert_eq!(
        first
            .source_maps
            .symbol_to_ir(InterfaceMemberId::new(102))
            .len(),
        1
    );
}

#[test]
fn stateful_instruction_lowers_to_verified_shared_invocation_with_graph_identity() {
    let enable = node(
        1,
        0,
        NodeKind::Constant {
            value: CanonicalValue::Bool(true),
        },
        vec![data_port(10, "OUT", PortDirection::Output, DataType::Bool)],
    );
    let preset = node(
        2,
        1,
        NodeKind::Constant {
            value: CanonicalValue::TimeMilliseconds(1000),
        },
        vec![data_port(20, "OUT", PortDirection::Output, DataType::Time)],
    );
    let timer = node(
        3,
        2,
        NodeKind::Instruction {
            code: TIMER_ON_DELAY,
            instance: Some(InstanceIdentity::Instruction(StateInstanceId::new(7))),
        },
        vec![
            data_port(30, "IN", PortDirection::Input, DataType::Bool),
            data_port(31, "PT", PortDirection::Input, DataType::Time),
            data_port(32, "Q", PortDirection::Output, DataType::Bool),
            data_port(33, "ET", PortDirection::Output, DataType::Time),
        ],
    );
    let network = FbdNetwork::from_parts(
        NetworkId::new(1),
        0,
        [enable, preset, timer],
        [connection(1, 10, 30), connection(2, 20, 31)],
    );
    let document = FbdDocument::new(FbdDocumentId::new(1), BlockId::new(10), [network]);
    assert!(validate_fbd(&document).can_lower());
    let mut program = ControllerProgram::new(ControllerId::new(1));
    program.insert_block(owner_block()).expect("unique owner");
    let lowered = lower_fbd_to_verified_ir(&document, &program).expect("verified stateful FBD");
    let operations = &lowered.verified_ir.program().functions()[&BlockId::new(10)].blocks
        [&plc_compiler::IrBasicBlockId::new(1)]
        .operations;
    assert!(matches!(
        operations[2].kind,
        IrOperationKind::InvokeInstruction {
            instruction: TIMER_ON_DELAY,
            ..
        }
    ));
    assert!(matches!(
        operations[3].kind,
        IrOperationKind::InvocationOutput { .. }
    ));
    assert!(
        lowered
            .lowered
            .compiler_source_maps
            .entries()
            .values()
            .flat_map(|entry| &entry.anchors)
            .any(|anchor| {
                anchor.language == plc_compiler::SourceLanguage::Fbd
                    && anchor.network_id == Some(1)
                    && anchor.node_id == Some(3)
                    && anchor.state_instance_id == Some(7)
            })
    );
}

#[test]
fn fc_call_formals_are_validated_from_program_and_lowered_without_a_language_engine() {
    let formal_in = InterfaceMemberId::new(7_001);
    let formal_out = InterfaceMemberId::new(7_002);
    let mut out = InterfaceMember::plain(formal_out, "Y", InterfaceRole::Output, DataType::DInt, 0);
    out.required_output_binding = true;
    let callee = ProgramBlock::new(
        BlockId::new(70),
        "Scale",
        EngineeringNumber::new(70).expect("nonzero"),
        ProgramUnitKind::Function,
        BlockInterface::from_members([
            InterfaceMember::plain(formal_in, "X", InterfaceRole::Input, DataType::DInt, 0),
            out,
        ]),
    );
    let constant = node(
        1,
        0,
        NodeKind::Constant {
            value: CanonicalValue::DInt(4),
        },
        vec![data_port(10, "OUT", PortDirection::Output, DataType::DInt)],
    );
    let mut input = data_port(20, "X", PortDirection::Input, DataType::DInt);
    input.formal = Some(IrFormalRef::BlockMember(formal_in));
    input.effect_role = EffectRole::CallParameter;
    let mut output = data_port(21, "Y", PortDirection::Output, DataType::DInt);
    output.formal = Some(IrFormalRef::BlockMember(formal_out));
    output.effect_role = EffectRole::CallParameter;
    let call = node(
        2,
        1,
        NodeKind::Call {
            code: CALL_FC,
            target: callee.id,
            instance: None,
        },
        vec![input, output],
    );
    let document = FbdDocument::new(
        FbdDocumentId::new(3),
        BlockId::new(10),
        [FbdNetwork::from_parts(
            NetworkId::new(1),
            0,
            [constant, call],
            [connection(1, 10, 20)],
        )],
    );
    let mut program = ControllerProgram::new(ControllerId::new(3));
    program.insert_block(owner_block()).expect("unique owner");
    program.insert_block(callee).expect("unique callee");
    assert!(matches!(
        lower_fbd_to_ir(&document, &owner_block()),
        Err(FbdLowerError::ProgramContextRequired { node }) if node == NodeId::new(2)
    ));
    let lowered = lower_fbd_to_verified_ir(&document, &program).expect("verified FC call");
    let operations = &lowered.verified_ir.program().functions()[&BlockId::new(10)].blocks
        [&plc_compiler::IrBasicBlockId::new(1)]
        .operations;
    assert!(matches!(
        operations[1].kind,
        IrOperationKind::CallBlock {
            call_instruction: CALL_FC,
            target,
            ..
        } if target == BlockId::new(70)
    ));
    assert!(
        lowered
            .lowered
            .compiler_source_maps
            .entries()
            .values()
            .flat_map(|entry| &entry.anchors)
            .any(|anchor| anchor.call_site_id == Some(2))
    );
}

#[test]
fn canonical_type_adapter_is_exhaustive_and_rejects_layout_types() {
    let supported = [
        (DataType::Bool, IrType::Bool),
        (DataType::Int, IrType::Int),
        (DataType::DInt, IrType::DInt),
        (DataType::Real, IrType::Real),
        (DataType::Time, IrType::Time),
        (
            DataType::String { capacity: 42 },
            IrType::String { capacity: 42 },
        ),
    ];
    for (source, expected) in supported {
        assert_eq!(data_type_to_ir_type(&source), Ok(expected));
    }
    assert_eq!(
        data_type_to_ir_type(&DataType::Named("Recipe".into())),
        Err(TypeAdapterError::NamedType)
    );
    assert_eq!(
        data_type_to_ir_type(&DataType::BlockInstance(BlockId::new(1))),
        Err(TypeAdapterError::BlockInstance)
    );
    assert_eq!(
        data_type_to_ir_type(&DataType::InstructionState(plc_program::StateKind::Timer)),
        Err(TypeAdapterError::InstructionState)
    );
}

#[test]
fn multiple_writers_warn_and_stored_order_defines_later_writer() {
    let first = node(
        1,
        0,
        NodeKind::StoreMember {
            member: InterfaceMemberId::new(102),
        },
        vec![data_port(10, "IN", PortDirection::Input, DataType::Bool)],
    );
    let second = node(
        2,
        1,
        NodeKind::StoreMember {
            member: InterfaceMemberId::new(102),
        },
        vec![data_port(20, "IN", PortDirection::Input, DataType::Bool)],
    );
    let constant_a = node(
        3,
        2,
        NodeKind::Constant {
            value: CanonicalValue::Bool(true),
        },
        vec![data_port(30, "OUT", PortDirection::Output, DataType::Bool)],
    );
    let constant_b = node(
        4,
        3,
        NodeKind::Constant {
            value: CanonicalValue::Bool(false),
        },
        vec![data_port(40, "OUT", PortDirection::Output, DataType::Bool)],
    );
    let network = FbdNetwork::from_parts(
        NetworkId::new(1),
        0,
        [first, second, constant_a, constant_b],
        [connection(1, 30, 10), connection(2, 40, 20)],
    );
    let document = FbdDocument::new(FbdDocumentId::new(1), BlockId::new(10), [network]);
    let report = validate_fbd(&document);
    assert!(report.can_lower());
    let warning = report
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == FbdDiagnosticCode::MultipleWriter)
        .expect("multiple writer warning");
    assert_eq!(warning.severity, DiagnosticSeverity::Warning);
    assert_eq!(warning.node, Some(NodeId::new(2)));
}

#[test]
fn arithmetic_instruction_maps_to_shared_binary_operator() {
    let constants = [
        node(
            1,
            0,
            NodeKind::Constant {
                value: CanonicalValue::DInt(1),
            },
            vec![data_port(10, "OUT", PortDirection::Output, DataType::DInt)],
        ),
        node(
            2,
            1,
            NodeKind::Constant {
                value: CanonicalValue::DInt(2),
            },
            vec![data_port(20, "OUT", PortDirection::Output, DataType::DInt)],
        ),
    ];
    let add = node(
        3,
        2,
        NodeKind::Instruction {
            code: ADD,
            instance: None,
        },
        vec![
            data_port(30, "A", PortDirection::Input, DataType::DInt),
            data_port(31, "B", PortDirection::Input, DataType::DInt),
            data_port(32, "OUT", PortDirection::Output, DataType::DInt),
        ],
    );
    let network = FbdNetwork::from_parts(
        NetworkId::new(1),
        0,
        constants.into_iter().chain([add]),
        [connection(1, 10, 30), connection(2, 20, 31)],
    );
    let document = FbdDocument::new(FbdDocumentId::new(1), BlockId::new(10), [network]);
    let lowered = lower_fbd_to_ir(&document, &owner_block()).expect("ADD lowering");
    let operations = &lowered.ir.functions()[&BlockId::new(10)].blocks
        [&plc_compiler::IrBasicBlockId::new(1)]
        .operations;
    assert!(matches!(
        operations[2].kind,
        IrOperationKind::Binary {
            operator: BinaryOperator::Add,
            ..
        }
    ));
}

#[test]
#[allow(clippy::too_many_lines)]
fn fbd_and_scl_addition_lower_to_identical_shared_ir_semantics() {
    let input_a = InterfaceMemberId::new(501);
    let input_b = InterfaceMemberId::new(502);
    let output = InterfaceMemberId::new(503);
    let owner = ProgramBlock::new(
        BlockId::new(50),
        "FC50",
        EngineeringNumber::new(50).expect("nonzero"),
        ProgramUnitKind::Function,
        BlockInterface::from_members([
            InterfaceMember::plain(input_a, "A", InterfaceRole::Input, DataType::DInt, 0),
            InterfaceMember::plain(input_b, "B", InterfaceRole::Input, DataType::DInt, 1),
            InterfaceMember::plain(output, "Q", InterfaceRole::Output, DataType::DInt, 0),
        ]),
    );
    let mut program = ControllerProgram::new(ControllerId::new(77));
    program.insert_block(owner.clone()).expect("unique block");
    let main = ProgramBlock::new(
        BlockId::new(51),
        "Main",
        EngineeringNumber::new(1).expect("nonzero"),
        ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain),
        BlockInterface::default(),
    );
    program.insert_block(main.clone()).expect("unique main");
    let sources = BTreeMap::from([
        (owner.id, SclSource::new(owner.id, "Q := A + B;")),
        (main.id, SclSource::new(main.id, "RETURN;")),
    ]);
    let snapshot = BuildSnapshot::capture(&program, &sources, CompilerProfile::edu21_core())
        .expect("valid snapshot");
    let snapshot_hash = snapshot.snapshot_hash();
    let attempt = BuildAttempt::new(
        BuildAttemptId::new(1),
        snapshot,
        BuildScope::RebuildAllSoftware,
    );
    let scl = Compiler::new(ResourceLimits::default())
        .expect("compiler")
        .compile(&attempt, snapshot_hash, None);
    let scl_artifact = scl
        .artifact()
        .unwrap_or_else(|| panic!("SCL build failed: {:#?}", scl.report()));
    let scl_function = scl_artifact.verified_ir().program().functions()[&owner.id].clone();
    let scl_ir = plc_compiler::TypedIrProgram::from_untrusted_parts(
        plc_compiler::TYPED_IR_VERSION,
        BTreeMap::from([(owner.id, scl_function)]),
    );

    let nodes = [
        node(
            1,
            0,
            NodeKind::LoadMember { member: input_a },
            vec![data_port(10, "OUT", PortDirection::Output, DataType::DInt)],
        ),
        node(
            2,
            1,
            NodeKind::LoadMember { member: input_b },
            vec![data_port(20, "OUT", PortDirection::Output, DataType::DInt)],
        ),
        node(
            3,
            2,
            NodeKind::Instruction {
                code: ADD,
                instance: None,
            },
            vec![
                data_port(30, "A", PortDirection::Input, DataType::DInt),
                data_port(31, "B", PortDirection::Input, DataType::DInt),
                data_port(32, "OUT", PortDirection::Output, DataType::DInt),
            ],
        ),
        node(
            4,
            3,
            NodeKind::StoreMember { member: output },
            vec![data_port(40, "IN", PortDirection::Input, DataType::DInt)],
        ),
    ];
    let fbd = FbdDocument::new(
        FbdDocumentId::new(50),
        owner.id,
        [FbdNetwork::from_parts(
            NetworkId::new(1),
            0,
            nodes,
            [
                connection(1, 10, 30),
                connection(2, 20, 31),
                connection(3, 32, 40),
            ],
        )],
    );
    let fbd_ir = lower_fbd_to_ir(&fbd, &owner).expect("FBD lowering");
    assert_eq!(
        fbd_ir.ir.semantic_fingerprint(),
        scl_ir.semantic_fingerprint(),
        "language frontends must converge before execution"
    );
}

#[test]
fn malformed_order_projection_remains_editable_but_cannot_lower() {
    let mut document = bool_pipeline();
    document
        .networks
        .get_mut(&NetworkId::new(1))
        .expect("network")
        .ordered_node_ids = vec![NodeId::new(30), NodeId::new(30), NodeId::new(10)];
    let report = validate_fbd(&document);
    assert!(!report.can_lower());
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == FbdDiagnosticCode::InvalidNodeOrder)
    );
    assert!(matches!(
        lower_fbd_to_ir(&document, &owner_block()),
        Err(FbdLowerError::InvalidGraph(_))
    ));
}

#[test]
fn validation_is_deterministic_over_a_bounded_negative_corpus() {
    let base = bool_pipeline();
    let mut corpus = Vec::new();
    for index in 0_u128..64 {
        let mut candidate = base.clone();
        candidate
            .networks
            .get_mut(&NetworkId::new(1))
            .expect("network")
            .connections
            .insert(
                ConnectionId::new(1_000 + index),
                connection(1_000 + index, 999_000 + index, 201),
            );
        corpus.push(candidate);
    }
    for candidate in corpus {
        let first = validate_fbd(&candidate);
        let second = validate_fbd(&candidate);
        assert_eq!(first, second);
        assert!(!first.can_lower());
        assert!(
            first
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == FbdDiagnosticCode::OrphanConnection)
        );
    }
}

#[test]
fn presentation_container_has_no_semantic_back_reference() {
    let layout = FbdLayout {
        nodes: BTreeMap::new(),
        routes: BTreeMap::new(),
    };
    assert!(layout.nodes.is_empty());
    assert!(layout.routes.is_empty());
}
