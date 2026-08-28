#![allow(clippy::too_many_lines)]

use plc_compiler::{
    BinaryOperator, DiagnosticCode, IrBasicBlockId, IrFormalRef, IrInstanceIdentity,
    IrOperationKind, SourceLanguage, UnaryOperator,
};
use plc_lad::{
    CoilMode, ContactMode, LadBox, LadBranch, LadBranchId, LadBranchPath, LadBranchPathId, LadCall,
    LadDiagnosticReason, LadDocument, LadDocumentId, LadEdgeId, LadEdit, LadEditError,
    LadFormalRef, LadGraphReason, LadInstance, LadLayout, LadLimits, LadNetwork, LadNetworkId,
    LadNode, LadNodeId, LadNodeKind, LadOperand, LadOperandId, LadOperandRef, LadPin,
    LadPinDirection, LadPortId, LadPortStatus, LadPowerEdge, LadPowerPort, LadPowerPortDirection,
    LadStateBinding, LadStateInstanceId, NodeLayout, RoutePoint, SharedIrRequirement,
    apply_lad_edits_atomically, lower_lad_to_ir, validate_lad,
};
use plc_program::{
    BlockId, BlockInterface, CALL_FB, CALL_FC, CanonicalValue, ControllerId, DataBlockKind,
    DataType, EngineeringNumber, FORMAL_ELAPSED_TIME, FORMAL_INPUT, FORMAL_OUTPUT,
    FORMAL_PRESET_TIME, InstanceOwner, InstancePath, InterfaceMember, InterfaceMemberId,
    InterfaceRole, ObDeclaration, ProgramBlock, ProgramUnitKind, StateKind, TIMER_ON_DELAY,
    VariableRef,
};

const OWNER: BlockId = BlockId::new(10);
const INPUT_A: InterfaceMemberId = InterfaceMemberId::new(1_001);
const INPUT_B: InterfaceMemberId = InterfaceMemberId::new(1_002);
const INPUT_C: InterfaceMemberId = InterfaceMemberId::new(1_003);
const OUTPUT_Q: InterfaceMemberId = InterfaceMemberId::new(1_004);
const OUTPUT_R: InterfaceMemberId = InterfaceMemberId::new(1_005);
const OUTPUT_ET_A: InterfaceMemberId = InterfaceMemberId::new(1_006);
const OUTPUT_ET_B: InterfaceMemberId = InterfaceMemberId::new(1_007);

fn member_operand(id: u128, member: InterfaceMemberId) -> LadOperandRef {
    LadOperandRef {
        id: LadOperandId::new(id),
        value: LadOperand::Variable(VariableRef::CallerMember(member)),
    }
}

fn input_port(id: u128) -> LadPowerPort {
    LadPowerPort {
        id: LadPortId::new(id),
        direction: LadPowerPortDirection::Input,
    }
}

fn output_port(id: u128) -> LadPowerPort {
    LadPowerPort {
        id: LadPortId::new(id),
        direction: LadPowerPortDirection::Output,
    }
}

fn edge(id: u128, source: u128, target: u128) -> LadPowerEdge {
    LadPowerEdge {
        id: LadEdgeId::new(id),
        source: LadPortId::new(source),
        target: LadPortId::new(target),
    }
}

fn source_node(id: u128, order: u32, output: u128) -> LadNode {
    LadNode::from_power_ports(
        LadNodeId::new(id),
        order,
        LadNodeKind::PowerSource,
        [output_port(output)],
    )
}

fn contact_node(
    id: u128,
    order: u32,
    input: u128,
    output: u128,
    operand_id: u128,
    member: InterfaceMemberId,
    mode: ContactMode,
) -> LadNode {
    LadNode::from_power_ports(
        LadNodeId::new(id),
        order,
        LadNodeKind::Contact {
            mode,
            operand: Some(member_operand(operand_id, member)),
        },
        [input_port(input), output_port(output)],
    )
}

fn coil_node(
    id: u128,
    order: u32,
    input: u128,
    operand_id: u128,
    member: InterfaceMemberId,
    mode: CoilMode,
) -> LadNode {
    LadNode::from_power_ports(
        LadNodeId::new(id),
        order,
        LadNodeKind::Coil {
            mode,
            operand: Some(member_operand(operand_id, member)),
        },
        [input_port(input)],
    )
}

fn owner_program() -> plc_program::ControllerProgram {
    let owner = ProgramBlock::new(
        OWNER,
        "Main",
        EngineeringNumber::new(1).expect("nonzero"),
        ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain),
        BlockInterface::from_members([
            InterfaceMember::plain(INPUT_A, "A", InterfaceRole::Input, DataType::Bool, 0),
            InterfaceMember::plain(INPUT_B, "B", InterfaceRole::Input, DataType::Bool, 1),
            InterfaceMember::plain(INPUT_C, "C", InterfaceRole::Input, DataType::Bool, 2),
            InterfaceMember::plain(OUTPUT_Q, "Q", InterfaceRole::Output, DataType::Bool, 0),
            InterfaceMember::plain(OUTPUT_R, "R", InterfaceRole::Output, DataType::Bool, 1),
            InterfaceMember::plain(
                OUTPUT_ET_A,
                "ET_A",
                InterfaceRole::Output,
                DataType::Time,
                2,
            ),
            InterfaceMember::plain(
                OUTPUT_ET_B,
                "ET_B",
                InterfaceRole::Output,
                DataType::Time,
                3,
            ),
        ]),
    );
    let mut program = plc_program::ControllerProgram::new(ControllerId::new(77));
    program.insert_block(owner).expect("unique owner");
    program
}

#[allow(clippy::too_many_arguments)]
fn timer_node(
    id: u128,
    order: u32,
    power_input: u128,
    power_output: u128,
    pin_base: u128,
    operand_base: u128,
    state_invocation: u128,
    state_db: BlockId,
    state_member: InterfaceMemberId,
    q_output: InterfaceMemberId,
    et_output: InterfaceMemberId,
) -> LadNode {
    let pins = [
        LadPin {
            id: LadPortId::new(pin_base + 1),
            formal: Some(LadFormalRef::Instruction(FORMAL_INPUT)),
            name: "IN".into(),
            direction: LadPinDirection::Input,
            data_type: DataType::Bool,
            required: true,
            status: LadPortStatus::Active,
            binding: Some(member_operand(operand_base + 1, INPUT_A)),
        },
        LadPin {
            id: LadPortId::new(pin_base + 2),
            formal: Some(LadFormalRef::Instruction(FORMAL_PRESET_TIME)),
            name: "PT".into(),
            direction: LadPinDirection::Input,
            data_type: DataType::Time,
            required: true,
            status: LadPortStatus::Active,
            binding: Some(LadOperandRef {
                id: LadOperandId::new(operand_base + 2),
                value: LadOperand::Constant(CanonicalValue::TimeMilliseconds(100)),
            }),
        },
        LadPin {
            id: LadPortId::new(pin_base + 3),
            formal: Some(LadFormalRef::Instruction(FORMAL_OUTPUT)),
            name: "Q".into(),
            direction: LadPinDirection::Output,
            data_type: DataType::Bool,
            required: true,
            status: LadPortStatus::Active,
            binding: Some(member_operand(operand_base + 3, q_output)),
        },
        LadPin {
            id: LadPortId::new(pin_base + 4),
            formal: Some(LadFormalRef::Instruction(FORMAL_ELAPSED_TIME)),
            name: "ET".into(),
            direction: LadPinDirection::Output,
            data_type: DataType::Time,
            required: true,
            status: LadPortStatus::Active,
            binding: Some(member_operand(operand_base + 4, et_output)),
        },
    ];
    LadNode::from_power_ports(
        LadNodeId::new(id),
        order,
        LadNodeKind::Box(LadBox::from_pins(
            TIMER_ON_DELAY,
            pins,
            Some(LadStateBinding {
                invocation: LadStateInstanceId::new(state_invocation),
                storage: VariableRef::DataBlockMember {
                    data_block: state_db,
                    member: state_member,
                },
                kind: StateKind::Timer,
            }),
        )),
        [input_port(power_input), output_port(power_output)],
    )
}

fn parallel_document() -> LadDocument {
    let branch = LadBranch::from_paths(
        LadBranchId::new(900),
        LadNodeId::new(2),
        LadNodeId::new(6),
        [
            LadBranchPath {
                id: LadBranchPathId::new(901),
                entry_edge: LadEdgeId::new(2),
                exit_edge: LadEdgeId::new(3),
            },
            LadBranchPath {
                id: LadBranchPathId::new(902),
                entry_edge: LadEdgeId::new(4),
                exit_edge: LadEdgeId::new(6),
            },
        ],
    );
    let nodes = [
        source_node(1, 0, 101),
        LadNode::from_power_ports(
            LadNodeId::new(2),
            1,
            LadNodeKind::BranchSplit {
                branch: LadBranchId::new(900),
            },
            [input_port(201), output_port(202), output_port(203)],
        ),
        contact_node(3, 2, 301, 302, 801, INPUT_A, ContactMode::NormallyOpen),
        contact_node(4, 3, 401, 402, 802, INPUT_B, ContactMode::NormallyOpen),
        contact_node(5, 4, 501, 502, 803, INPUT_C, ContactMode::NormallyOpen),
        LadNode::from_power_ports(
            LadNodeId::new(6),
            5,
            LadNodeKind::BranchJoin {
                branch: LadBranchId::new(900),
            },
            [input_port(601), input_port(602), output_port(603)],
        ),
        coil_node(7, 6, 701, 804, OUTPUT_Q, CoilMode::Normal),
    ];
    let network = LadNetwork::from_parts(
        LadNetworkId::new(100),
        0,
        nodes,
        [
            edge(1, 101, 201),
            edge(2, 202, 301),
            edge(3, 302, 601),
            edge(4, 203, 401),
            edge(5, 402, 501),
            edge(6, 502, 602),
            edge(7, 603, 701),
        ],
        [branch],
    );
    LadDocument::new(LadDocumentId::new(500), OWNER, [network])
}

fn nested_parallel_document() -> LadDocument {
    let outer = LadBranch::from_paths(
        LadBranchId::new(1_900),
        LadNodeId::new(1_002),
        LadNodeId::new(1_008),
        [
            LadBranchPath {
                id: LadBranchPathId::new(1_901),
                entry_edge: LadEdgeId::new(1_002),
                exit_edge: LadEdgeId::new(1_003),
            },
            LadBranchPath {
                id: LadBranchPathId::new(1_902),
                entry_edge: LadEdgeId::new(1_004),
                exit_edge: LadEdgeId::new(1_009),
            },
        ],
    );
    let inner = LadBranch::from_paths(
        LadBranchId::new(1_910),
        LadNodeId::new(1_004),
        LadNodeId::new(1_007),
        [
            LadBranchPath {
                id: LadBranchPathId::new(1_911),
                entry_edge: LadEdgeId::new(1_005),
                exit_edge: LadEdgeId::new(1_006),
            },
            LadBranchPath {
                id: LadBranchPathId::new(1_912),
                entry_edge: LadEdgeId::new(1_007),
                exit_edge: LadEdgeId::new(1_008),
            },
        ],
    );
    let network = LadNetwork::from_parts(
        LadNetworkId::new(1_100),
        0,
        [
            source_node(1_001, 0, 11_101),
            LadNode::from_power_ports(
                LadNodeId::new(1_002),
                1,
                LadNodeKind::BranchSplit {
                    branch: LadBranchId::new(1_900),
                },
                [input_port(11_201), output_port(11_202), output_port(11_203)],
            ),
            contact_node(
                1_003,
                2,
                11_301,
                11_302,
                11_801,
                INPUT_A,
                ContactMode::NormallyOpen,
            ),
            LadNode::from_power_ports(
                LadNodeId::new(1_004),
                3,
                LadNodeKind::BranchSplit {
                    branch: LadBranchId::new(1_910),
                },
                [input_port(11_401), output_port(11_402), output_port(11_403)],
            ),
            contact_node(
                1_005,
                4,
                11_501,
                11_502,
                11_802,
                INPUT_B,
                ContactMode::NormallyOpen,
            ),
            contact_node(
                1_006,
                5,
                11_601,
                11_602,
                11_803,
                INPUT_C,
                ContactMode::NormallyOpen,
            ),
            LadNode::from_power_ports(
                LadNodeId::new(1_007),
                6,
                LadNodeKind::BranchJoin {
                    branch: LadBranchId::new(1_910),
                },
                [input_port(11_701), input_port(11_702), output_port(11_703)],
            ),
            LadNode::from_power_ports(
                LadNodeId::new(1_008),
                7,
                LadNodeKind::BranchJoin {
                    branch: LadBranchId::new(1_900),
                },
                [input_port(11_801), input_port(11_802), output_port(11_803)],
            ),
            coil_node(1_009, 8, 11_901, 11_804, OUTPUT_Q, CoilMode::Normal),
        ],
        [
            edge(1_001, 11_101, 11_201),
            edge(1_002, 11_202, 11_301),
            edge(1_003, 11_302, 11_801),
            edge(1_004, 11_203, 11_401),
            edge(1_005, 11_402, 11_501),
            edge(1_006, 11_502, 11_701),
            edge(1_007, 11_403, 11_601),
            edge(1_008, 11_602, 11_702),
            edge(1_009, 11_703, 11_802),
            edge(1_010, 11_803, 11_901),
        ],
        [outer, inner],
    );
    LadDocument::new(LadDocumentId::new(1_500), OWNER, [network])
}

fn simple_network(
    network: u128,
    order: u32,
    base: u128,
    input_member: InterfaceMemberId,
    output_member: InterfaceMemberId,
    contact_mode: ContactMode,
    coil_mode: CoilMode,
) -> LadNetwork {
    LadNetwork::from_parts(
        LadNetworkId::new(network),
        order,
        [
            source_node(base + 1, 0, base + 101),
            contact_node(
                base + 2,
                1,
                base + 201,
                base + 202,
                base + 801,
                input_member,
                contact_mode,
            ),
            coil_node(
                base + 3,
                2,
                base + 301,
                base + 802,
                output_member,
                coil_mode,
            ),
        ],
        [
            edge(base + 1, base + 101, base + 201),
            edge(base + 2, base + 202, base + 301),
        ],
        [],
    )
}

#[test]
fn parallel_power_graph_lowers_in_stored_branch_order_with_graphical_probes() {
    let document = parallel_document();
    let program = owner_program();
    let report = validate_lad(&document, &program, LadLimits::default());
    assert!(report.can_lower(), "{:#?}", report.diagnostics);
    assert_eq!(
        report.networks[&LadNetworkId::new(100)].execution_order,
        [1_u128, 2, 3, 4, 5, 6, 7].map(LadNodeId::new)
    );

    let first = lower_lad_to_ir(&document, &program, LadLimits::default()).expect("valid LAD");
    let second = lower_lad_to_ir(&document, &program, LadLimits::default()).expect("repeat");
    assert_eq!(first, second);
    let function = &first.ir().functions()[&OWNER];
    let operations = &function.blocks[&IrBasicBlockId::new(1)].operations;
    assert_eq!(operations.len(), 9);
    assert!(matches!(operations[0].kind, IrOperationKind::Constant(_)));
    assert!(matches!(
        operations[7].kind,
        IrOperationKind::Binary {
            operator: BinaryOperator::Or,
            ..
        }
    ));
    assert!(matches!(
        operations[8].kind,
        IrOperationKind::StoreMember {
            target: OUTPUT_Q,
            ..
        }
    ));
    assert_eq!(first.source_maps.entries().len(), 10);
    assert_eq!(first.probes.entries().len(), 10);
    assert!(first.source_maps.entries().values().all(|entry| {
        entry.anchors.iter().all(|anchor| {
            anchor.language == SourceLanguage::Lad
                && anchor.text_range.is_none()
                && anchor.network_id == Some(LadNetworkId::new(100).get())
        })
    }));
    assert!(!first.node_to_ir(LadNodeId::new(3)).is_empty());
    assert!(!first.edge_to_ir(LadEdgeId::new(6)).is_empty());
    assert!(!first.operand_to_ir(LadOperandId::new(801)).is_empty());
    assert!(!first.edge_to_probes(LadEdgeId::new(7)).is_empty());
}

#[test]
fn authored_branch_path_order_overrides_incidental_edge_and_node_identity_order() {
    let mut document = parallel_document();
    document
        .networks
        .get_mut(&LadNetworkId::new(100))
        .expect("network")
        .branches
        .get_mut(&LadBranchId::new(900))
        .expect("branch")
        .ordered_path_ids = vec![LadBranchPathId::new(902), LadBranchPathId::new(901)];
    let program = owner_program();
    let report = validate_lad(&document, &program, LadLimits::default());
    assert!(report.can_lower(), "{:#?}", report.diagnostics);
    assert_eq!(
        report.networks[&LadNetworkId::new(100)].execution_order,
        [1_u128, 2, 4, 5, 3, 6, 7].map(LadNodeId::new)
    );
    let lowered = lower_lad_to_ir(&document, &program, LadLimits::default())
        .expect("reordered paths remain valid");
    let loaded_members: Vec<_> = lowered.ir().functions()[&OWNER].blocks[&IrBasicBlockId::new(1)]
        .operations
        .iter()
        .filter_map(|operation| match operation.kind {
            IrOperationKind::LoadMember { member } => Some(member),
            _ => None,
        })
        .collect();
    assert_eq!(loaded_members, [INPUT_B, INPUT_C, INPUT_A]);
}

#[test]
fn nested_parallel_branches_follow_authored_path_order_not_coordinates_or_edge_ids() {
    let document = nested_parallel_document();
    let program = owner_program();
    let report = validate_lad(&document, &program, LadLimits::default());
    assert!(report.can_lower(), "{:#?}", report.diagnostics);
    assert_eq!(
        report.networks[&LadNetworkId::new(1_100)].execution_order,
        [
            1_001_u128, 1_002, 1_003, 1_004, 1_005, 1_006, 1_007, 1_008, 1_009
        ]
        .map(LadNodeId::new)
    );

    let lowered = lower_lad_to_ir(&document, &program, LadLimits::default())
        .expect("nested parallel graph lowers");
    let operations = &lowered.ir().functions()[&OWNER].blocks[&IrBasicBlockId::new(1)].operations;
    assert_eq!(operations.len(), 10);
    assert!(matches!(
        operations[7].kind,
        IrOperationKind::Binary {
            operator: BinaryOperator::Or,
            ..
        }
    ));
    assert!(matches!(
        operations[8].kind,
        IrOperationKind::Binary {
            operator: BinaryOperator::Or,
            ..
        }
    ));
    assert!(!lowered.edge_to_ir(LadEdgeId::new(1_008)).is_empty());
    assert!(!lowered.edge_to_ir(LadEdgeId::new(1_009)).is_empty());
}

#[test]
fn layout_reroute_zoom_and_comments_cannot_change_semantic_ir_or_fingerprint() {
    let document = parallel_document();
    let program = owner_program();
    let fingerprint = document.semantic_fingerprint();
    let ir = lower_lad_to_ir(&document, &program, LadLimits::default())
        .expect("lower")
        .ir()
        .semantic_fingerprint();

    let mut first_layout = LadLayout {
        zoom_per_mille: 1_000,
        ..LadLayout::default()
    };
    first_layout.nodes.insert(
        LadNodeId::new(3),
        NodeLayout {
            x: 10,
            y: 20,
            width: 90,
            height: 40,
        },
    );
    first_layout
        .routes
        .insert(LadEdgeId::new(2), vec![RoutePoint { x: 12, y: 20 }]);
    first_layout
        .node_comments
        .insert(LadNodeId::new(3), "first layout".into());
    let mut second_layout = first_layout.clone();
    second_layout.zoom_per_mille = 2_500;
    second_layout
        .nodes
        .get_mut(&LadNodeId::new(3))
        .expect("node")
        .x = -900;
    second_layout.routes.insert(
        LadEdgeId::new(2),
        vec![RoutePoint { x: -100, y: 700 }, RoutePoint { x: 900, y: -2 }],
    );
    second_layout
        .node_comments
        .insert(LadNodeId::new(3), "completely changed".into());
    assert_ne!(first_layout, second_layout);
    assert_eq!(document.semantic_fingerprint(), fingerprint);
    assert_eq!(
        lower_lad_to_ir(&document, &program, LadLimits::default())
            .expect("lower after presentation mutation")
            .ir()
            .semantic_fingerprint(),
        ir
    );
}

#[test]
fn destructive_edit_preserves_invalid_source_and_exact_undo_restores_identities() {
    let mut document = parallel_document();
    let original = document.clone();
    let failed = apply_lad_edits_atomically(
        &mut document,
        &[
            LadEdit::RemoveNodeKeepReferences {
                network: LadNetworkId::new(100),
                node: LadNodeId::new(3),
            },
            LadEdit::RemovePowerEdgeKeepBranches {
                network: LadNetworkId::new(100),
                edge: LadEdgeId::new(99_999),
            },
        ],
    );
    assert_eq!(
        failed,
        Err(LadEditError::MissingPowerEdge(LadEdgeId::new(99_999)))
    );
    assert_eq!(document, original, "failed batch rolls back exactly");

    let undo = apply_lad_edits_atomically(
        &mut document,
        &[LadEdit::RemoveNodeKeepReferences {
            network: LadNetworkId::new(100),
            node: LadNodeId::new(3),
        }],
    )
    .expect("editable deletion");
    let report = validate_lad(&document, &owner_program(), LadLimits::default());
    assert!(!report.can_lower());
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.reason == LadDiagnosticReason::Graph(LadGraphReason::OrphanEdge)
    }));
    assert!(matches!(
        lower_lad_to_ir(&document, &owner_program(), LadLimits::default()),
        Err(plc_lad::LadLowerError::InvalidGraph(_))
    ));
    undo.restore(&mut document);
    assert_eq!(document, original);
    assert!(
        document.networks[&LadNetworkId::new(100)]
            .nodes
            .contains_key(&LadNodeId::new(3))
    );
}

#[test]
fn atomic_reorder_and_semantic_replacement_normalize_orders_and_advance_once() {
    let mut document = LadDocument::new(
        LadDocumentId::new(650),
        OWNER,
        [
            simple_network(
                651,
                0,
                60_000,
                INPUT_A,
                OUTPUT_Q,
                ContactMode::NormallyOpen,
                CoilMode::Normal,
            ),
            simple_network(
                652,
                1,
                70_000,
                INPUT_B,
                OUTPUT_R,
                ContactMode::NormallyOpen,
                CoilMode::Normal,
            ),
        ],
    );
    let no_op_before = document.clone();
    let no_op_undo = apply_lad_edits_atomically(&mut document, &[]).expect("empty batch");
    assert_eq!(document, no_op_before, "empty batch cannot dirty semantics");
    no_op_undo.restore(&mut document);
    assert_eq!(document, no_op_before);

    let before = document.clone();
    let before_fingerprint = document.semantic_fingerprint();
    let undo = apply_lad_edits_atomically(
        &mut document,
        &[
            LadEdit::MoveNetwork {
                network: LadNetworkId::new(652),
                new_index: 0,
            },
            LadEdit::ReplaceNodeKind {
                network: LadNetworkId::new(651),
                node: LadNodeId::new(60_002),
                kind: LadNodeKind::Contact {
                    mode: ContactMode::NormallyClosed,
                    operand: Some(member_operand(60_801, INPUT_A)),
                },
            },
        ],
    )
    .expect("atomic semantic batch");

    assert_eq!(document.semantic_revision, before.semantic_revision + 1);
    assert_eq!(
        document.ordered_network_ids,
        [LadNetworkId::new(652), LadNetworkId::new(651)]
    );
    assert_eq!(document.networks[&LadNetworkId::new(652)].semantic_order, 0);
    assert_eq!(document.networks[&LadNetworkId::new(651)].semantic_order, 1);
    assert_ne!(document.semantic_fingerprint(), before_fingerprint);
    assert!(
        validate_lad(&document, &owner_program(), LadLimits::default()).can_lower(),
        "reordering does not corrupt the graph"
    );

    undo.restore(&mut document);
    assert_eq!(document, before);
}

#[test]
fn normally_closed_and_negated_coil_have_explicit_shared_ir_not_renderer_behavior() {
    let document = LadDocument::new(
        LadDocumentId::new(700),
        OWNER,
        [simple_network(
            701,
            0,
            10_000,
            INPUT_A,
            OUTPUT_Q,
            ContactMode::NormallyClosed,
            CoilMode::Negated,
        )],
    );
    let lowered = lower_lad_to_ir(&document, &owner_program(), LadLimits::default())
        .expect("supported contact and coil");
    let operations = &lowered.ir().functions()[&OWNER].blocks[&IrBasicBlockId::new(1)].operations;
    let not_count = operations
        .iter()
        .filter(|operation| {
            matches!(
                operation.kind,
                IrOperationKind::Unary {
                    operator: UnaryOperator::Not,
                    ..
                }
            )
        })
        .count();
    assert_eq!(
        not_count, 2,
        "contact inversion and coil inversion are explicit"
    );
}

#[test]
fn set_and_reset_coils_lower_as_explicit_read_modify_write_semantics() {
    let document = LadDocument::new(
        LadDocumentId::new(750),
        OWNER,
        [
            simple_network(
                751,
                0,
                75_000,
                INPUT_A,
                OUTPUT_Q,
                ContactMode::NormallyOpen,
                CoilMode::Set,
            ),
            simple_network(
                752,
                1,
                76_000,
                INPUT_B,
                OUTPUT_R,
                ContactMode::NormallyOpen,
                CoilMode::Reset,
            ),
        ],
    );
    let lowered = lower_lad_to_ir(&document, &owner_program(), LadLimits::default())
        .expect("SET/RESET are representable without conditional-store shortcuts");
    let function = &lowered.ir().functions()[&OWNER];
    let set_operations = &function.blocks[&IrBasicBlockId::new(1)].operations;
    assert!(set_operations.iter().any(|operation| {
        matches!(
            operation.kind,
            IrOperationKind::Binary {
                operator: BinaryOperator::Or,
                ..
            }
        )
    }));
    assert!(set_operations.iter().any(|operation| {
        matches!(
            operation.kind,
            IrOperationKind::LoadMember { member: OUTPUT_Q }
        )
    }));
    let reset_operations = &function.blocks[&IrBasicBlockId::new(2)].operations;
    assert!(reset_operations.iter().any(|operation| {
        matches!(
            operation.kind,
            IrOperationKind::LoadMember { member: OUTPUT_R }
        )
    }));
    assert!(reset_operations.iter().any(|operation| {
        matches!(
            operation.kind,
            IrOperationKind::Unary {
                operator: UnaryOperator::Not,
                ..
            }
        )
    }));
    assert!(reset_operations.iter().any(|operation| {
        matches!(
            operation.kind,
            IrOperationKind::Binary {
                operator: BinaryOperator::And,
                ..
            }
        )
    }));
}

#[test]
fn powered_return_uses_real_control_flow_and_false_power_falls_through() {
    let return_network = LadNetwork::from_parts(
        LadNetworkId::new(781),
        0,
        [
            source_node(78_001, 0, 78_101),
            contact_node(
                78_002,
                1,
                78_201,
                78_202,
                78_801,
                INPUT_A,
                ContactMode::NormallyOpen,
            ),
            LadNode::from_power_ports(
                LadNodeId::new(78_003),
                2,
                LadNodeKind::Return,
                [input_port(78_301)],
            ),
        ],
        [edge(78_001, 78_101, 78_201), edge(78_002, 78_202, 78_301)],
        [],
    );
    let document = LadDocument::new(
        LadDocumentId::new(780),
        OWNER,
        [
            return_network,
            simple_network(
                782,
                1,
                79_000,
                INPUT_B,
                OUTPUT_Q,
                ContactMode::NormallyOpen,
                CoilMode::Normal,
            ),
        ],
    );
    let lowered = lower_lad_to_ir(&document, &owner_program(), LadLimits::default())
        .expect("conditional return lowers to shared CFG");
    let function = &lowered.ir().functions()[&OWNER];
    assert_eq!(function.blocks.len(), 3);
    assert!(matches!(
        function.blocks[&IrBasicBlockId::new(1)].terminator.kind,
        plc_compiler::IrTerminatorKind::Branch {
            when_true,
            when_false,
            ..
        } if when_true == IrBasicBlockId::new(3) && when_false == IrBasicBlockId::new(2)
    ));
    assert!(matches!(
        function.blocks[&IrBasicBlockId::new(3)].terminator.kind,
        plc_compiler::IrTerminatorKind::Return
    ));
    assert!(!lowered.node_to_ir(LadNodeId::new(78_003)).is_empty());
}

#[test]
fn repeated_writes_use_common_warning_and_network_order_stays_semantic() {
    let document = LadDocument::new(
        LadDocumentId::new(800),
        OWNER,
        [
            simple_network(
                801,
                0,
                20_000,
                INPUT_A,
                OUTPUT_Q,
                ContactMode::NormallyOpen,
                CoilMode::Normal,
            ),
            simple_network(
                802,
                1,
                30_000,
                INPUT_B,
                OUTPUT_Q,
                ContactMode::NormallyOpen,
                CoilMode::Normal,
            ),
        ],
    );
    let report = validate_lad(&document, &owner_program(), LadLimits::default());
    assert!(report.can_lower());
    let warning = report
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagnosticCode::MULTIPLE_WRITER)
        .expect("common writer warning");
    assert!(!warning.blocking);
    assert_eq!(warning.related.len(), 1);
    let lowered = lower_lad_to_ir(&document, &owner_program(), LadLimits::default())
        .expect("warnings do not block");
    let function = &lowered.ir().functions()[&OWNER];
    assert_eq!(
        function.blocks[&IrBasicBlockId::new(1)].terminator.kind,
        plc_compiler::IrTerminatorKind::Jump(IrBasicBlockId::new(2))
    );
    assert!(matches!(
        function.blocks[&IrBasicBlockId::new(2)].terminator.kind,
        plc_compiler::IrTerminatorKind::Return
    ));
}

#[test]
fn typed_timer_box_lowers_to_verified_stateful_invocation_with_eno() {
    let state_db = BlockId::new(20);
    let state_member = InterfaceMemberId::new(2_001);
    let mut program = owner_program();
    program
        .insert_block(ProgramBlock::new(
            state_db,
            "State",
            EngineeringNumber::new(20).expect("nonzero"),
            ProgramUnitKind::DataBlock(DataBlockKind::Global),
            BlockInterface::from_members([InterfaceMember::plain(
                state_member,
                "TimerState",
                InterfaceRole::Static,
                DataType::InstructionState(StateKind::Timer),
                0,
            )]),
        ))
        .expect("state DB");
    let pins = [
        LadPin {
            id: LadPortId::new(51),
            formal: Some(LadFormalRef::Instruction(FORMAL_INPUT)),
            name: "IN".into(),
            direction: LadPinDirection::Input,
            data_type: DataType::Bool,
            required: true,
            status: LadPortStatus::Active,
            binding: Some(member_operand(951, INPUT_A)),
        },
        LadPin {
            id: LadPortId::new(52),
            formal: Some(LadFormalRef::Instruction(FORMAL_PRESET_TIME)),
            name: "PT".into(),
            direction: LadPinDirection::Input,
            data_type: DataType::Time,
            required: true,
            status: LadPortStatus::Active,
            binding: Some(LadOperandRef {
                id: LadOperandId::new(952),
                value: LadOperand::Constant(CanonicalValue::TimeMilliseconds(100)),
            }),
        },
        LadPin {
            id: LadPortId::new(53),
            formal: Some(LadFormalRef::Instruction(FORMAL_OUTPUT)),
            name: "Q".into(),
            direction: LadPinDirection::Output,
            data_type: DataType::Bool,
            required: true,
            status: LadPortStatus::Active,
            binding: Some(member_operand(953, OUTPUT_Q)),
        },
        LadPin {
            id: LadPortId::new(54),
            formal: Some(LadFormalRef::Instruction(FORMAL_ELAPSED_TIME)),
            name: "ET".into(),
            direction: LadPinDirection::Output,
            data_type: DataType::Time,
            required: true,
            status: LadPortStatus::Active,
            binding: Some(member_operand(954, OUTPUT_ET_A)),
        },
    ];
    let timer = LadNode::from_power_ports(
        LadNodeId::new(52),
        1,
        LadNodeKind::Box(LadBox::from_pins(
            TIMER_ON_DELAY,
            pins,
            Some(LadStateBinding {
                invocation: LadStateInstanceId::new(99),
                storage: VariableRef::DataBlockMember {
                    data_block: state_db,
                    member: state_member,
                },
                kind: StateKind::Timer,
            }),
        )),
        [input_port(5_201), output_port(5_202)],
    );
    let network = LadNetwork::from_parts(
        LadNetworkId::new(50),
        0,
        [
            source_node(51, 0, 5_101),
            timer,
            coil_node(53, 2, 5_301, 955, OUTPUT_R, CoilMode::Normal),
        ],
        [edge(5_001, 5_101, 5_201), edge(5_002, 5_202, 5_301)],
        [],
    );
    let mut document = LadDocument::new(LadDocumentId::new(50), OWNER, [network]);
    let report = validate_lad(&document, &program, LadLimits::default());
    assert!(report.can_lower(), "{:#?}", report.diagnostics);
    let lowered = lower_lad_to_ir(&document, &program, LadLimits::default())
        .expect("registry-defined timer lowers to shared verified IR");
    let operations = &lowered.ir().functions()[&OWNER].blocks[&IrBasicBlockId::new(1)].operations;
    let invocation = operations
        .iter()
        .find(|operation| {
            matches!(
                operation.kind,
                IrOperationKind::InvokeInstruction {
                    instruction: TIMER_ON_DELAY,
                    ..
                }
            )
        })
        .expect("timer invocation");
    let IrOperationKind::InvokeInstruction {
        outputs,
        instance,
        activation,
        ..
    } = &invocation.kind
    else {
        unreachable!();
    };
    assert!(matches!(
        instance,
        Some(IrInstanceIdentity::Instruction {
            stable_id: 99,
            kind: StateKind::Timer,
        })
    ));
    assert!(activation.is_some(), "rung power is explicit EN semantics");
    assert!(
        outputs
            .iter()
            .any(|output| output.formal
                == IrFormalRef::Instruction(plc_program::FORMAL_ENABLE_OUTPUT))
    );
    assert!(operations.iter().any(|operation| {
        matches!(
            operation.kind,
            IrOperationKind::StoreMember {
                target: OUTPUT_Q,
                ..
            }
        )
    }));
    assert!(operations.iter().any(|operation| {
        matches!(
            operation.kind,
            IrOperationKind::StoreMember {
                target: OUTPUT_ET_A,
                ..
            }
        )
    }));
    assert!(!lowered.state_to_ir(LadStateInstanceId::new(99)).is_empty());

    let timer = document
        .networks
        .get_mut(&LadNetworkId::new(50))
        .expect("network")
        .nodes
        .get_mut(&LadNodeId::new(52))
        .expect("timer");
    let LadNodeKind::Box(timer) = &mut timer.kind else {
        unreachable!();
    };
    timer.pins.remove(&LadPortId::new(54));
    timer
        .ordered_pin_ids
        .retain(|pin| *pin != LadPortId::new(54));
    let missing = validate_lad(&document, &program, LadLimits::default());
    assert!(!missing.can_lower());
    assert!(missing.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::REQUIRED_BINDING_MISSING
            && diagnostic.primary.instruction_formal == Some(FORMAL_ELAPSED_TIME)
    }));
    assert!(matches!(
        lower_lad_to_ir(&document, &program, LadLimits::default()),
        Err(plc_lad::LadLowerError::InvalidGraph(_))
    ));
}

#[test]
fn distinct_stateful_invocations_cannot_alias_the_same_timer_storage() {
    let state_db = BlockId::new(25);
    let state_member = InterfaceMemberId::new(2_501);
    let mut program = owner_program();
    program
        .insert_block(ProgramBlock::new(
            state_db,
            "TimerState",
            EngineeringNumber::new(25).expect("nonzero"),
            ProgramUnitKind::DataBlock(DataBlockKind::Global),
            BlockInterface::from_members([InterfaceMember::plain(
                state_member,
                "SharedTimerState",
                InterfaceRole::Static,
                DataType::InstructionState(StateKind::Timer),
                0,
            )]),
        ))
        .expect("state DB");
    let network = LadNetwork::from_parts(
        LadNetworkId::new(250),
        0,
        [
            source_node(251, 0, 25_101),
            timer_node(
                252,
                1,
                25_201,
                25_202,
                251_000,
                252_000,
                25_901,
                state_db,
                state_member,
                OUTPUT_Q,
                OUTPUT_ET_A,
            ),
            timer_node(
                253,
                2,
                25_301,
                25_302,
                253_000,
                254_000,
                25_902,
                state_db,
                state_member,
                OUTPUT_R,
                OUTPUT_ET_B,
            ),
            LadNode::from_power_ports(
                LadNodeId::new(254),
                3,
                LadNodeKind::Return,
                [input_port(25_401)],
            ),
        ],
        [
            edge(25_001, 25_101, 25_201),
            edge(25_002, 25_202, 25_301),
            edge(25_003, 25_302, 25_401),
        ],
        [],
    );
    let document = LadDocument::new(LadDocumentId::new(250), OWNER, [network]);
    let report = validate_lad(&document, &program, LadLimits::default());
    assert!(!report.can_lower());
    let alias = report
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.reason == LadDiagnosticReason::AliasedStateBinding)
        .expect("aliased state diagnostic");
    assert_eq!(alias.code, DiagnosticCode::INSTANCE_INVALID);
    assert_eq!(
        alias.primary.state_instance,
        Some(LadStateInstanceId::new(25_902))
    );
    assert_eq!(alias.related.len(), 1);
    assert_eq!(
        alias.related[0].state_instance,
        Some(LadStateInstanceId::new(25_901))
    );
    assert!(matches!(
        lower_lad_to_ir(&document, &program, LadLimits::default()),
        Err(plc_lad::LadLowerError::InvalidGraph(_))
    ));
}

#[test]
fn fb_call_pins_instance_and_eno_lower_to_verified_shared_call_ir() {
    let callee = BlockId::new(30);
    let instance_db = BlockId::new(31);
    let formal_input = InterfaceMemberId::new(3_001);
    let formal_output = InterfaceMemberId::new(3_002);
    let mut output =
        InterfaceMember::plain(formal_output, "Y", InterfaceRole::Output, DataType::Bool, 0);
    output.required_output_binding = true;
    let mut program = owner_program();
    program
        .insert_block(ProgramBlock::new(
            callee,
            "Valve",
            EngineeringNumber::new(30).expect("nonzero"),
            ProgramUnitKind::FunctionBlock,
            BlockInterface::from_members([
                InterfaceMember::plain(formal_input, "X", InterfaceRole::Input, DataType::Bool, 0),
                output,
            ]),
        ))
        .expect("callee");
    program
        .insert_block(ProgramBlock::new(
            instance_db,
            "ValveInstance",
            EngineeringNumber::new(31).expect("nonzero"),
            ProgramUnitKind::DataBlock(DataBlockKind::Instance { fb_type: callee }),
            BlockInterface::default(),
        ))
        .expect("instance");
    let call_site = plc_program::CallSiteId::new(330);
    let call = LadCall::from_pins(
        call_site,
        CALL_FB,
        callee,
        Some(LadInstance {
            owner: InstanceOwner::InstanceDb(instance_db),
            path: InstancePath {
                root_instance_db: instance_db,
                multi_instance_slots: Vec::new(),
            },
        }),
        [
            LadPin {
                id: LadPortId::new(3301),
                formal: Some(LadFormalRef::BlockMember(formal_input)),
                name: "X".into(),
                direction: LadPinDirection::Input,
                data_type: DataType::Bool,
                required: true,
                status: LadPortStatus::Active,
                binding: Some(member_operand(3_901, INPUT_A)),
            },
            LadPin {
                id: LadPortId::new(3302),
                formal: Some(LadFormalRef::BlockMember(formal_output)),
                name: "Y".into(),
                direction: LadPinDirection::Output,
                data_type: DataType::Bool,
                required: true,
                status: LadPortStatus::Active,
                binding: Some(member_operand(3_902, OUTPUT_Q)),
            },
        ],
    );
    let network = LadNetwork::from_parts(
        LadNetworkId::new(330),
        0,
        [
            source_node(331, 0, 33_101),
            LadNode::from_power_ports(
                LadNodeId::new(332),
                1,
                LadNodeKind::Call(call),
                [input_port(33_201), output_port(33_202)],
            ),
            coil_node(333, 2, 33_301, 3_903, OUTPUT_R, CoilMode::Normal),
        ],
        [edge(33_001, 33_101, 33_201), edge(33_002, 33_202, 33_301)],
        [],
    );
    let mut document = LadDocument::new(LadDocumentId::new(330), OWNER, [network]);
    let report = validate_lad(&document, &program, LadLimits::default());
    assert!(report.can_lower(), "{:#?}", report.diagnostics);
    let lowered = lower_lad_to_ir(&document, &program, LadLimits::default())
        .expect("FB call lowers to the shared call operation");
    let operation = lowered.ir().functions()[&OWNER].blocks[&IrBasicBlockId::new(1)]
        .operations
        .iter()
        .find(|operation| matches!(operation.kind, IrOperationKind::CallBlock { .. }))
        .expect("call operation");
    assert!(matches!(
        &operation.kind,
        IrOperationKind::CallBlock {
            call_instruction: CALL_FB,
            target,
            instance: Some(IrInstanceIdentity::FunctionBlock(path)),
            activation: Some(_),
            ..
        } if *target == callee && path.root_instance_db == instance_db
    ));
    let operations = &lowered.ir().functions()[&OWNER].blocks[&IrBasicBlockId::new(1)].operations;
    assert!(operations.iter().any(|operation| {
        matches!(
            operation.kind,
            IrOperationKind::StoreMember {
                target: OUTPUT_Q,
                ..
            }
        )
    }));
    assert!(!lowered.call_to_ir(call_site).is_empty());

    let call_node = document
        .networks
        .get_mut(&LadNetworkId::new(330))
        .expect("network")
        .nodes
        .get_mut(&LadNodeId::new(332))
        .expect("call node");
    let LadNodeKind::Call(call) = &mut call_node.kind else {
        panic!("call node kind");
    };
    call.pins
        .get_mut(&LadPortId::new(3301))
        .expect("input pin")
        .status = LadPortStatus::Stale;
    let stale = validate_lad(&document, &program, LadLimits::default());
    assert!(!stale.can_lower());
    assert!(stale.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::STALE_FORMAL
            && diagnostic.primary.port == Some(LadPortId::new(3301))
            && diagnostic.primary.call_site == Some(call_site)
    }));
}

#[test]
fn fc_call_uses_copy_in_copy_out_without_fabricating_instance_state() {
    let callee = BlockId::new(35);
    let formal_input = InterfaceMemberId::new(3_501);
    let formal_output = InterfaceMemberId::new(3_502);
    let mut output =
        InterfaceMember::plain(formal_output, "Y", InterfaceRole::Output, DataType::Bool, 0);
    output.required_output_binding = true;
    let mut program = owner_program();
    program
        .insert_block(ProgramBlock::new(
            callee,
            "Invert",
            EngineeringNumber::new(35).expect("nonzero"),
            ProgramUnitKind::Function,
            BlockInterface::from_members([
                InterfaceMember::plain(formal_input, "X", InterfaceRole::Input, DataType::Bool, 0),
                output,
            ]),
        ))
        .expect("FC");
    let call_site = plc_program::CallSiteId::new(350);
    let call = LadCall::from_pins(
        call_site,
        CALL_FC,
        callee,
        None,
        [
            LadPin {
                id: LadPortId::new(3501),
                formal: Some(LadFormalRef::BlockMember(formal_input)),
                name: "X".into(),
                direction: LadPinDirection::Input,
                data_type: DataType::Bool,
                required: true,
                status: LadPortStatus::Active,
                binding: Some(member_operand(35_901, INPUT_C)),
            },
            LadPin {
                id: LadPortId::new(3502),
                formal: Some(LadFormalRef::BlockMember(formal_output)),
                name: "Y".into(),
                direction: LadPinDirection::Output,
                data_type: DataType::Bool,
                required: true,
                status: LadPortStatus::Active,
                binding: Some(member_operand(35_902, OUTPUT_Q)),
            },
        ],
    );
    let network = LadNetwork::from_parts(
        LadNetworkId::new(350),
        0,
        [
            source_node(351, 0, 35_101),
            LadNode::from_power_ports(
                LadNodeId::new(352),
                1,
                LadNodeKind::Call(call),
                [input_port(35_201), output_port(35_202)],
            ),
            coil_node(353, 2, 35_301, 35_903, OUTPUT_R, CoilMode::Normal),
        ],
        [edge(35_001, 35_101, 35_201), edge(35_002, 35_202, 35_301)],
        [],
    );
    let document = LadDocument::new(LadDocumentId::new(350), OWNER, [network]);
    let lowered = lower_lad_to_ir(&document, &program, LadLimits::default())
        .expect("FC call lowers without instance state");
    assert!(
        lowered.ir().functions()[&OWNER].blocks[&IrBasicBlockId::new(1)]
            .operations
            .iter()
            .any(|operation| {
                matches!(
                    operation.kind,
                    IrOperationKind::CallBlock {
                        call_instruction: CALL_FC,
                        target,
                        instance: None,
                        activation: Some(_),
                        ..
                    } if target == callee
                )
            })
    );
    assert!(!lowered.call_to_ir(call_site).is_empty());
}

#[test]
fn illegal_contact_and_coil_operands_are_navigable_errors() {
    let mut bad_contact = parallel_document();
    let contact = bad_contact
        .networks
        .get_mut(&LadNetworkId::new(100))
        .expect("network")
        .nodes
        .get_mut(&LadNodeId::new(3))
        .expect("contact");
    if let LadNodeKind::Contact { operand, .. } = &mut contact.kind {
        *operand = Some(LadOperandRef {
            id: LadOperandId::new(801),
            value: LadOperand::Constant(CanonicalValue::DInt(7)),
        });
    }
    let report = validate_lad(&bad_contact, &owner_program(), LadLimits::default());
    assert!(!report.can_lower());
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::TYPE_MISMATCH
            && diagnostic.primary.operand == Some(LadOperandId::new(801))
    }));

    let mut bad_coil = parallel_document();
    let coil = bad_coil
        .networks
        .get_mut(&LadNetworkId::new(100))
        .expect("network")
        .nodes
        .get_mut(&LadNodeId::new(7))
        .expect("coil");
    if let LadNodeKind::Coil { operand, .. } = &mut coil.kind {
        *operand = Some(LadOperandRef {
            id: LadOperandId::new(804),
            value: LadOperand::Constant(CanonicalValue::Bool(true)),
        });
    }
    let report = validate_lad(&bad_coil, &owner_program(), LadLimits::default());
    assert!(
        report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::ILLEGAL_OR_OVERLAPPING_BINDING
        })
    );
}

#[test]
fn genuine_shared_ir_db_storage_gap_is_exact_and_never_aliases_a_caller_member() {
    let data_block = BlockId::new(45);
    let db_member = InterfaceMemberId::new(4_501);
    let mut program = owner_program();
    program
        .insert_block(ProgramBlock::new(
            data_block,
            "Signals",
            EngineeringNumber::new(45).expect("nonzero"),
            ProgramUnitKind::DataBlock(DataBlockKind::Global),
            BlockInterface::from_members([InterfaceMember::plain(
                db_member,
                "RemoteEnable",
                InterfaceRole::Static,
                DataType::Bool,
                0,
            )]),
        ))
        .expect("global DB");
    let mut network = simple_network(
        451,
        0,
        45_000,
        INPUT_A,
        OUTPUT_Q,
        ContactMode::NormallyOpen,
        CoilMode::Normal,
    );
    let contact = network
        .nodes
        .get_mut(&LadNodeId::new(45_002))
        .expect("contact");
    let LadNodeKind::Contact { operand, .. } = &mut contact.kind else {
        unreachable!();
    };
    *operand = Some(LadOperandRef {
        id: LadOperandId::new(45_801),
        value: LadOperand::Variable(VariableRef::DataBlockMember {
            data_block,
            member: db_member,
        }),
    });
    let document = LadDocument::new(LadDocumentId::new(450), OWNER, [network]);
    assert!(validate_lad(&document, &program, LadLimits::default()).can_lower());
    assert_eq!(
        lower_lad_to_ir(&document, &program, LadLimits::default()),
        Err(plc_lad::LadLowerError::SharedIrGap(plc_lad::SharedIrGap {
            requirements: vec![SharedIrRequirement::DataBlockStorage {
                node: LadNodeId::new(45_002),
                operand: LadOperandId::new(45_801),
                data_block,
                member: db_member,
            }],
        }))
    );
}

#[test]
fn malformed_branch_cycle_zero_path_and_resource_limit_never_emit_ir() {
    let program = owner_program();
    let mut open = parallel_document();
    open.networks
        .get_mut(&LadNetworkId::new(100))
        .expect("network")
        .power_edges
        .remove(&LadEdgeId::new(3));
    let report = validate_lad(&open, &program, LadLimits::default());
    assert!(!report.can_lower());
    assert!(report.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.reason,
            LadDiagnosticReason::Graph(
                LadGraphReason::OpenBranch | LadGraphReason::DanglingPowerPort
            )
        )
    }));
    assert!(matches!(
        lower_lad_to_ir(&open, &program, LadLimits::default()),
        Err(plc_lad::LadLowerError::InvalidGraph(_))
    ));

    let mut zero = parallel_document();
    let network = zero
        .networks
        .get_mut(&LadNetworkId::new(100))
        .expect("network");
    network
        .power_edges
        .insert(LadEdgeId::new(2), edge(2, 202, 601));
    *network
        .branches
        .get_mut(&LadBranchId::new(900))
        .expect("branch")
        .paths
        .get_mut(&LadBranchPathId::new(901))
        .expect("path") = LadBranchPath {
        id: LadBranchPathId::new(901),
        entry_edge: LadEdgeId::new(2),
        exit_edge: LadEdgeId::new(2),
    };
    let report = validate_lad(&zero, &program, LadLimits::default());
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.reason == LadDiagnosticReason::Graph(LadGraphReason::ZeroPathBranch)
    }));

    let limited = validate_lad(
        &parallel_document(),
        &program,
        LadLimits {
            max_nodes_per_network: 2,
            ..LadLimits::default()
        },
    );
    assert!(limited.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::RESOURCE_LIMIT && diagnostic.blocking
    }));
    assert!(
        limited.networks.is_empty(),
        "an over-limit graph is rejected before topology traversal"
    );

    let diagnostics_bounded = validate_lad(
        &open,
        &program,
        LadLimits {
            max_diagnostics: 1,
            ..LadLimits::default()
        },
    );
    assert_eq!(diagnostics_bounded.diagnostics.len(), 1);
    assert_eq!(
        diagnostics_bounded.diagnostics[0].code,
        DiagnosticCode::RESOURCE_LIMIT
    );
    assert!(!diagnostics_bounded.can_lower());
}

#[test]
fn bounded_orphan_mutation_corpus_is_deterministic_and_fail_closed() {
    let program = owner_program();
    for mutation in 0_u128..64 {
        let mut candidate = parallel_document();
        candidate
            .networks
            .get_mut(&LadNetworkId::new(100))
            .expect("network")
            .power_edges
            .insert(
                LadEdgeId::new(100_000 + mutation),
                edge(100_000 + mutation, 900_000 + mutation, 301),
            );
        let first = validate_lad(&candidate, &program, LadLimits::default());
        let second = validate_lad(&candidate, &program, LadLimits::default());
        assert_eq!(first, second);
        assert!(!first.can_lower());
        assert!(first.diagnostics.iter().any(|diagnostic| {
            diagnostic.reason == LadDiagnosticReason::Graph(LadGraphReason::OrphanEdge)
        }));
        assert!(matches!(
            lower_lad_to_ir(&candidate, &program, LadLimits::default()),
            Err(plc_lad::LadLowerError::InvalidGraph(_))
        ));
    }
}

#[test]
fn semantic_ids_must_be_new_for_independent_network_copies() {
    let original_network = simple_network(
        901,
        0,
        40_000,
        INPUT_A,
        OUTPUT_Q,
        ContactMode::NormallyOpen,
        CoilMode::Normal,
    );
    let mut copied_with_reused_ids = original_network.clone();
    copied_with_reused_ids.id = LadNetworkId::new(902);
    copied_with_reused_ids.semantic_order = 1;
    let invalid = LadDocument::new(
        LadDocumentId::new(900),
        OWNER,
        [original_network.clone(), copied_with_reused_ids],
    );
    let report = validate_lad(&invalid, &owner_program(), LadLimits::default());
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.reason == LadDiagnosticReason::Graph(LadGraphReason::DuplicateSemanticIdentity)
    }));

    let valid = LadDocument::new(
        LadDocumentId::new(901),
        OWNER,
        [
            original_network,
            simple_network(
                902,
                1,
                50_000,
                INPUT_B,
                OUTPUT_R,
                ContactMode::NormallyOpen,
                CoilMode::Normal,
            ),
        ],
    );
    assert!(
        validate_lad(&valid, &owner_program(), LadLimits::default()).can_lower(),
        "an independent copy with new semantic IDs is legal"
    );
}
