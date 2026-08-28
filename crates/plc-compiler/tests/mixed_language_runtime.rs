use std::collections::BTreeMap;

use plc_compiler::{
    BuildAttempt, BuildAttemptId, BuildOutcome, BuildScope, BuildSnapshot, Compiler,
    CompilerProfile, FrontendArtifact, Hash32, IrFormalRef, ProbeTable, ResourceLimits,
    RuntimeMappedSite, SclSource, SourceLanguage, SourceMapTable, VerifiedIr,
    compose_frontend_artifacts, lower_scl_frontend_artifact, project_verified_ir_to_runtime,
};
use plc_lad::{
    CoilMode, LadDocument, LadDocumentId, LadEdgeId, LadLimits, LadNetwork, LadNetworkId, LadNode,
    LadNodeId, LadNodeKind, LadOperand, LadOperandId, LadOperandRef, LadPortId, LadPowerEdge,
    LadPowerPort, LadPowerPortDirection, lower_lad_to_ir,
};
use plc_language_tools::{
    ActivationRole, ConnectionId, ConnectionKind, EffectRole, FbdConnection, FbdDocument,
    FbdDocumentId, FbdNetwork, FbdNode, FbdPort, NetworkId, NodeId, NodeKind, PortDirection,
    PortId, PortMultiplicity, PortStatus, StateInstanceId, lower_fbd_to_verified_ir,
};
use plc_program::{
    BlockId, BlockInterface, CALL_FC, CanonicalValue as ProgramValue, ControllerId,
    ControllerProgram, DataType, EngineeringNumber, FORMAL_ELAPSED_TIME, FORMAL_INPUT,
    FORMAL_OUTPUT, FORMAL_PRESET_TIME, InterfaceMember, InterfaceMemberId, InterfaceRole,
    ObDeclaration, ProgramBlock, ProgramUnitKind, TIMER_ON_DELAY, VariableRef,
};
use plc_runtime::{
    CanonicalValue, CpuState, ReplayEventKind, RestartKind, RunOutcome, UniverseId,
    VirtualController, VirtualControllerId,
};

const MAIN: BlockId = BlockId::new(0x110);
const OUTPUT: InterfaceMemberId = InterfaceMemberId::new(0x1_102);

#[derive(Clone)]
struct SharedLowering {
    verified_ir: VerifiedIr,
    source_maps: SourceMapTable,
    probes: ProbeTable,
}

#[derive(Debug, PartialEq, Eq)]
struct ObservableExecution {
    output: CanonicalValue,
    cpu_state: CpuState,
    scan_sequence: u64,
    scan_start_time_ms: u64,
    completed_time_ms: u64,
    replay_kinds: Vec<ReplayEventKind>,
    diagnostic_count: usize,
}

fn program() -> ControllerProgram {
    let output = InterfaceMember::plain(OUTPUT, "Output", InterfaceRole::Temp, DataType::Bool, 0);
    let main = ProgramBlock::new(
        MAIN,
        "Main",
        EngineeringNumber::new(1).expect("nonzero OB number"),
        ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain),
        BlockInterface::from_members([output]),
    );
    let mut program = ControllerProgram::new(ControllerId::new(0x77));
    program.insert_block(main).expect("unique main block");
    program
}

fn scl(program: &ControllerProgram) -> SharedLowering {
    let sources = BTreeMap::from([(MAIN, SclSource::new(MAIN, "Output := TRUE;"))]);
    let snapshot = BuildSnapshot::capture(program, &sources, CompilerProfile::edu21_core())
        .expect("valid SCL snapshot");
    let snapshot_hash = snapshot.snapshot_hash();
    let completion = Compiler::new(ResourceLimits::default())
        .expect("compiler limits")
        .compile(
            &BuildAttempt::new(
                BuildAttemptId::new(0x51),
                snapshot,
                BuildScope::RebuildAllSoftware,
            ),
            snapshot_hash,
            None,
        );
    assert_eq!(
        completion.report().outcome(),
        BuildOutcome::ArtifactCreated,
        "{:#?}",
        completion.report()
    );
    let artifact = completion.artifact().expect("verified SCL artifact");
    SharedLowering {
        verified_ir: artifact.verified_ir().clone(),
        source_maps: artifact.source_maps().clone(),
        probes: artifact.probe_table().clone(),
    }
}

fn fbd(program: &ControllerProgram) -> SharedLowering {
    let load = FbdNode::from_ports(
        NodeId::new(1),
        0,
        NodeKind::Constant {
            value: ProgramValue::Bool(true),
        },
        [fbd_port(10, "OUT", PortDirection::Output)],
    );
    let store = FbdNode::from_ports(
        NodeId::new(2),
        1,
        NodeKind::StoreMember { member: OUTPUT },
        [fbd_port(20, "IN", PortDirection::Input)],
    );
    let document = FbdDocument::new(
        FbdDocumentId::new(0xFBD),
        MAIN,
        [FbdNetwork::from_parts(
            NetworkId::new(1),
            0,
            [load, store],
            [FbdConnection {
                id: ConnectionId::new(1),
                source: PortId::new(10),
                target: PortId::new(20),
                kind: ConnectionKind::Data,
            }],
        )],
    );
    let lowered = lower_fbd_to_verified_ir(&document, program).expect("verified FBD lowering");
    SharedLowering {
        verified_ir: lowered.verified_ir,
        source_maps: lowered.lowered.compiler_source_maps,
        probes: lowered.lowered.compiler_probes,
    }
}

fn fbd_port(id: u128, name: &str, direction: PortDirection) -> FbdPort {
    FbdPort {
        id: PortId::new(id),
        name: name.into(),
        direction,
        data_type: Some(DataType::Bool),
        required: direction == PortDirection::Input,
        multiplicity: if direction == PortDirection::Output {
            PortMultiplicity::Many
        } else {
            PortMultiplicity::One
        },
        activation: ActivationRole::None,
        status: PortStatus::Active,
        effect_role: EffectRole::Value,
        formal: None,
    }
}

fn timer_port(
    id: u128,
    name: &str,
    direction: PortDirection,
    data_type: DataType,
    formal: plc_program::InstructionFormalId,
) -> FbdPort {
    let mut port = fbd_port(id, name, direction);
    port.data_type = Some(data_type);
    port.formal = Some(IrFormalRef::Instruction(formal));
    port
}

fn typed_fbd_port(id: u128, name: &str, direction: PortDirection, data_type: DataType) -> FbdPort {
    let mut port = fbd_port(id, name, direction);
    port.data_type = Some(data_type);
    port
}

fn lad(program: &ControllerProgram) -> SharedLowering {
    let source = LadNode::from_power_ports(
        LadNodeId::new(1),
        0,
        LadNodeKind::PowerSource,
        [lad_power_port(10, LadPowerPortDirection::Output)],
    );
    let coil = LadNode::from_power_ports(
        LadNodeId::new(2),
        1,
        LadNodeKind::Coil {
            mode: CoilMode::Normal,
            operand: Some(lad_member_operand(101, OUTPUT)),
        },
        [lad_power_port(20, LadPowerPortDirection::Input)],
    );
    let document = LadDocument::new(
        LadDocumentId::new(0x1AD),
        MAIN,
        [LadNetwork::from_parts(
            LadNetworkId::new(1),
            0,
            [source, coil],
            [LadPowerEdge {
                id: LadEdgeId::new(1),
                source: LadPortId::new(10),
                target: LadPortId::new(20),
            }],
            [],
        )],
    );
    let lowered =
        lower_lad_to_ir(&document, program, LadLimits::default()).expect("verified LAD lowering");
    SharedLowering {
        verified_ir: lowered.verified_ir,
        source_maps: lowered.source_maps,
        probes: lowered.probes,
    }
}

fn lad_power_port(id: u128, direction: LadPowerPortDirection) -> LadPowerPort {
    LadPowerPort {
        id: LadPortId::new(id),
        direction,
    }
}

fn lad_member_operand(id: u128, member: InterfaceMemberId) -> LadOperandRef {
    LadOperandRef {
        id: LadOperandId::new(id),
        value: LadOperand::Variable(VariableRef::CallerMember(member)),
    }
}

fn execute(
    program: &ControllerProgram,
    lowering: &SharedLowering,
    controller_id: u128,
) -> ObservableExecution {
    let projection = project_verified_ir_to_runtime(
        &lowering.verified_ir,
        &lowering.source_maps,
        &lowering.probes,
        program,
        Hash32::ZERO,
    )
    .expect("shared verified IR must project to production runtime");
    let output_memory = projection
        .memory_for(MAIN, OUTPUT)
        .expect("output runtime binding");
    let mut controller = VirtualController::new(
        UniverseId(0xCA11),
        VirtualControllerId(controller_id),
        0x5eed,
    );
    controller.power_on().expect("power on");
    controller
        .install_verified_artifact(projection.package())
        .expect("verified load");
    controller
        .request_run(RestartKind::Resume)
        .expect("RUN transition");
    let report = match controller.run_scan().expect("scan command") {
        RunOutcome::Completed(report) => report,
        RunOutcome::Faulted(event) => panic!("shared runtime faulted: {event:?}"),
    };
    ObservableExecution {
        output: controller
            .actual_memory(output_memory)
            .expect("output memory value"),
        cpu_state: controller.cpu_state(),
        scan_sequence: report.scan_sequence,
        scan_start_time_ms: report.scan_start_time_ms,
        completed_time_ms: report.completed_time_ms,
        replay_kinds: controller
            .replay_events()
            .iter()
            .map(|event| event.kind)
            .collect(),
        diagnostic_count: controller.diagnostics().len(),
    }
}

#[test]
fn real_scl_fbd_and_lad_lowerers_share_one_production_runtime_semantics() {
    let program = program();
    let scl = execute(&program, &scl(&program), 0x501);
    let fbd = execute(&program, &fbd(&program), 0x502);
    let lad = execute(&program, &lad(&program), 0x503);

    assert_eq!(scl, fbd);
    assert_eq!(fbd, lad);
    assert_eq!(lad.output, CanonicalValue::Bool(true));
    assert_eq!(lad.cpu_state, CpuState::Run);
    assert_eq!(lad.scan_sequence, 1);
    assert_eq!(lad.scan_start_time_ms, 0);
    assert_eq!(lad.completed_time_ms, 10);
    assert_eq!(lad.diagnostic_count, 0);
}

#[test]
#[allow(clippy::too_many_lines)]
fn real_fbd_stateful_invocation_projects_and_runs_in_the_same_controller() {
    let program = program();
    let timer = FbdNode::from_ports(
        NodeId::new(3),
        2,
        NodeKind::Instruction {
            code: TIMER_ON_DELAY,
            instance: Some(plc_language_tools::InstanceIdentity::Instruction(
                StateInstanceId::new(7),
            )),
        },
        [
            timer_port(30, "IN", PortDirection::Input, DataType::Bool, FORMAL_INPUT),
            timer_port(
                31,
                "PT",
                PortDirection::Input,
                DataType::Time,
                FORMAL_PRESET_TIME,
            ),
            timer_port(
                32,
                "Q",
                PortDirection::Output,
                DataType::Bool,
                FORMAL_OUTPUT,
            ),
            timer_port(
                33,
                "ET",
                PortDirection::Output,
                DataType::Time,
                FORMAL_ELAPSED_TIME,
            ),
        ],
    );
    let document = FbdDocument::new(
        FbdDocumentId::new(0x710),
        MAIN,
        [FbdNetwork::from_parts(
            NetworkId::new(1),
            0,
            [
                FbdNode::from_ports(
                    NodeId::new(1),
                    0,
                    NodeKind::Constant {
                        value: ProgramValue::Bool(true),
                    },
                    [fbd_port(10, "OUT", PortDirection::Output)],
                ),
                FbdNode::from_ports(
                    NodeId::new(2),
                    1,
                    NodeKind::Constant {
                        value: ProgramValue::TimeMilliseconds(20),
                    },
                    [typed_fbd_port(
                        20,
                        "OUT",
                        PortDirection::Output,
                        DataType::Time,
                    )],
                ),
                timer,
                FbdNode::from_ports(
                    NodeId::new(4),
                    3,
                    NodeKind::StoreMember { member: OUTPUT },
                    [fbd_port(40, "IN", PortDirection::Input)],
                ),
            ],
            [
                FbdConnection {
                    id: ConnectionId::new(1),
                    source: PortId::new(10),
                    target: PortId::new(30),
                    kind: ConnectionKind::Data,
                },
                FbdConnection {
                    id: ConnectionId::new(2),
                    source: PortId::new(20),
                    target: PortId::new(31),
                    kind: ConnectionKind::Data,
                },
                FbdConnection {
                    id: ConnectionId::new(3),
                    source: PortId::new(32),
                    target: PortId::new(40),
                    kind: ConnectionKind::Data,
                },
            ],
        )],
    );
    let lowered = lower_fbd_to_verified_ir(&document, &program).expect("verified FBD timer");
    let projection = project_verified_ir_to_runtime(
        &lowered.verified_ir,
        &lowered.lowered.compiler_source_maps,
        &lowered.lowered.compiler_probes,
        &program,
        Hash32::ZERO,
    )
    .expect("stateful shared IR projection");
    let timer_binding = projection
        .source_bindings()
        .iter()
        .find(|binding| {
            binding
                .anchors
                .iter()
                .any(|anchor| anchor.state_instance_id == Some(7))
                && matches!(binding.runtime_site, RuntimeMappedSite::Instruction { .. })
        })
        .expect("timer keeps its source/probe/state identity");
    assert!(!timer_binding.anchors.is_empty());

    let output = projection.memory_for(MAIN, OUTPUT).expect("output memory");
    let mut controller =
        VirtualController::new(UniverseId(0xCA11), VirtualControllerId(0x710), 0x5eed);
    controller.power_on().expect("power on");
    controller
        .install_verified_artifact(projection.package())
        .expect("load timer artifact");
    controller
        .request_run(RestartKind::Resume)
        .expect("RUN timer");
    assert!(matches!(
        controller.run_scan(),
        Ok(RunOutcome::Completed(_))
    ));
    assert_eq!(
        controller.actual_memory(output),
        Some(CanonicalValue::Bool(false))
    );
    assert!(matches!(
        controller.run_scan(),
        Ok(RunOutcome::Completed(_))
    ));
    assert_eq!(
        controller.actual_memory(output),
        Some(CanonicalValue::Bool(true))
    );
    assert!(controller.diagnostics().is_empty());
}

#[test]
#[allow(clippy::too_many_lines)]
fn composed_fbd_caller_and_scl_fc_preserve_authored_call_identity_at_runtime() {
    let callee_id = BlockId::new(0x720);
    let formal_input = InterfaceMemberId::new(0x7_201);
    let formal_output = InterfaceMemberId::new(0x7_202);
    let caller_result = InterfaceMemberId::new(0x7_203);
    let main = ProgramBlock::new(
        MAIN,
        "Main",
        EngineeringNumber::new(1).expect("nonzero OB number"),
        ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain),
        BlockInterface::from_members([InterfaceMember::plain(
            caller_result,
            "Result",
            InterfaceRole::Temp,
            DataType::DInt,
            0,
        )]),
    );
    let mut required_output =
        InterfaceMember::plain(formal_output, "Y", InterfaceRole::Output, DataType::DInt, 0);
    required_output.required_output_binding = true;
    let function_block = ProgramBlock::new(
        callee_id,
        "AddOne",
        EngineeringNumber::new(2).expect("nonzero FC number"),
        ProgramUnitKind::Function,
        BlockInterface::from_members([
            InterfaceMember::plain(formal_input, "X", InterfaceRole::Input, DataType::DInt, 0),
            required_output,
        ]),
    );
    let mut program = ControllerProgram::new(ControllerId::new(0x720));
    program.insert_block(main).expect("main");
    program.insert_block(function_block).expect("callee");

    let mut call_input = typed_fbd_port(20, "X", PortDirection::Input, DataType::DInt);
    call_input.formal = Some(IrFormalRef::BlockMember(formal_input));
    call_input.effect_role = EffectRole::CallParameter;
    let mut call_output = typed_fbd_port(21, "Y", PortDirection::Output, DataType::DInt);
    call_output.formal = Some(IrFormalRef::BlockMember(formal_output));
    call_output.effect_role = EffectRole::CallParameter;
    let fbd = FbdDocument::new(
        FbdDocumentId::new(0x721),
        MAIN,
        [FbdNetwork::from_parts(
            NetworkId::new(1),
            0,
            [
                FbdNode::from_ports(
                    NodeId::new(1),
                    0,
                    NodeKind::Constant {
                        value: ProgramValue::DInt(4),
                    },
                    [typed_fbd_port(
                        10,
                        "OUT",
                        PortDirection::Output,
                        DataType::DInt,
                    )],
                ),
                FbdNode::from_ports(
                    NodeId::new(2),
                    1,
                    NodeKind::Call {
                        code: CALL_FC,
                        target: callee_id,
                        instance: None,
                    },
                    [call_input, call_output],
                ),
                FbdNode::from_ports(
                    NodeId::new(3),
                    2,
                    NodeKind::StoreMember {
                        member: caller_result,
                    },
                    [typed_fbd_port(
                        30,
                        "IN",
                        PortDirection::Input,
                        DataType::DInt,
                    )],
                ),
            ],
            [
                FbdConnection {
                    id: ConnectionId::new(1),
                    source: PortId::new(10),
                    target: PortId::new(20),
                    kind: ConnectionKind::Data,
                },
                FbdConnection {
                    id: ConnectionId::new(2),
                    source: PortId::new(21),
                    target: PortId::new(30),
                    kind: ConnectionKind::Data,
                },
            ],
        )],
    );
    let lowered_graph = lower_fbd_to_verified_ir(&fbd, &program).expect("verified FBD caller");
    let graph_frontend = FrontendArtifact::new(
        MAIN,
        SourceLanguage::Fbd,
        lowered_graph.verified_ir,
        lowered_graph.lowered.compiler_source_maps,
        lowered_graph.lowered.compiler_probes,
    );
    let scl_frontend = lower_scl_frontend_artifact(
        &program,
        &SclSource::new(callee_id, "Y := X + DINT#1;"),
        ResourceLimits::default(),
    )
    .expect("verified SCL callee");
    let composed = compose_frontend_artifacts(&program, &[graph_frontend, scl_frontend])
        .expect("mixed composition");
    let projection = project_verified_ir_to_runtime(
        composed.verified_ir(),
        composed.source_maps(),
        composed.probes(),
        &program,
        Hash32::ZERO,
    )
    .expect("mixed call projection");

    let result_memory = projection
        .memory_for(MAIN, caller_result)
        .expect("caller result memory");
    let mut controller =
        VirtualController::new(UniverseId(0xCA11), VirtualControllerId(0x721), 0x5eed);
    controller.power_on().expect("power on");
    controller
        .install_verified_artifact(projection.package())
        .expect("load mixed artifact");
    controller
        .request_run(RestartKind::Resume)
        .expect("RUN mixed artifact");
    let report = match controller.run_scan().expect("mixed scan") {
        RunOutcome::Completed(report) => report,
        RunOutcome::Faulted(event) => panic!("mixed call faulted: {event:?}"),
    };
    assert_eq!(
        controller.actual_memory(result_memory),
        Some(CanonicalValue::I32(5))
    );
    assert_eq!(report.call_boundaries.len(), 2);
    assert_eq!(report.call_boundaries[0].call_site_identity, 2);
    assert_eq!(report.call_boundaries[1].call_site_identity, 2);
    let mapped = projection
        .source_for(report.call_boundaries[0].source_identity)
        .expect("mapped FBD call");
    assert!(mapped.anchors.iter().all(|anchor| {
        anchor.language == SourceLanguage::Fbd && anchor.call_site_id == Some(2)
    }));
}
