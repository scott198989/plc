use plc_compiler::{Hash32, project_verified_ir_to_runtime};
use plc_lad::{
    CoilMode, LadBox, LadDiagnosticReason, LadDocument, LadDocumentId, LadEdgeId, LadFormalRef,
    LadLimits, LadNetwork, LadNetworkId, LadNode, LadNodeId, LadNodeKind, LadOperand, LadOperandId,
    LadOperandRef, LadPin, LadPinDirection, LadPortId, LadPortStatus, LadPowerEdge, LadPowerPort,
    LadPowerPortDirection, lower_lad_to_ir, validate_lad,
};
use plc_program::{
    BlockId, BlockInterface, CanonicalValue as ProgramValue, ControllerId, ControllerProgram,
    DataType, EngineeringNumber, FORMAL_LIMIT_INPUT, FORMAL_LIMIT_OUTPUT, FORMAL_MAXIMUM,
    FORMAL_MINIMUM, InterfaceMember, InterfaceMemberId, InterfaceRole, LIMIT, ObDeclaration,
    ProgramBlock, ProgramUnitKind, VariableRef,
};
use plc_runtime::{
    CanonicalValue, RestartKind, RunOutcome, UniverseId, VirtualController, VirtualControllerId,
};

const MAIN: BlockId = BlockId::new(0x1150);
const OUTPUT: InterfaceMemberId = InterfaceMemberId::new(0x1151);
const POWER: InterfaceMemberId = InterfaceMemberId::new(0x1152);

fn program() -> ControllerProgram {
    let main = ProgramBlock::new(
        MAIN,
        "Main",
        EngineeringNumber::new(1).expect("nonzero OB number"),
        ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain),
        BlockInterface::from_members([
            InterfaceMember::plain(OUTPUT, "Output", InterfaceRole::Temp, DataType::DInt, 0),
            InterfaceMember::plain(POWER, "Power", InterfaceRole::Temp, DataType::Bool, 1),
        ]),
    );
    let mut program = ControllerProgram::new(ControllerId::new(0x1153));
    program.insert_block(main).expect("unique main block");
    program
}

fn constant(id: u128, value: i32) -> LadOperandRef {
    LadOperandRef {
        id: LadOperandId::new(id),
        value: LadOperand::Constant(ProgramValue::DInt(value)),
    }
}

fn member(id: u128, member: InterfaceMemberId) -> LadOperandRef {
    LadOperandRef {
        id: LadOperandId::new(id),
        value: LadOperand::Variable(VariableRef::CallerMember(member)),
    }
}

fn power_port(id: u128, direction: LadPowerPortDirection) -> LadPowerPort {
    LadPowerPort {
        id: LadPortId::new(id),
        direction,
    }
}

fn input_pin(id: u128, formal: plc_program::InstructionFormalId, name: &str, value: i32) -> LadPin {
    LadPin {
        id: LadPortId::new(id),
        formal: Some(LadFormalRef::Instruction(formal)),
        name: name.into(),
        direction: LadPinDirection::Input,
        data_type: DataType::DInt,
        required: true,
        status: LadPortStatus::Active,
        binding: Some(constant(id + 100, value)),
    }
}

fn document(minimum: i32, input: i32, maximum: i32) -> LadDocument {
    let source = LadNode::from_power_ports(
        LadNodeId::new(1),
        0,
        LadNodeKind::PowerSource,
        [power_port(10, LadPowerPortDirection::Output)],
    );
    let limit = LadNode::from_power_ports(
        LadNodeId::new(2),
        1,
        LadNodeKind::Box(LadBox::from_pins(
            LIMIT,
            [
                input_pin(201, FORMAL_MINIMUM, "MN", minimum),
                input_pin(202, FORMAL_LIMIT_INPUT, "IN", input),
                input_pin(203, FORMAL_MAXIMUM, "MX", maximum),
                LadPin {
                    id: LadPortId::new(204),
                    formal: Some(LadFormalRef::Instruction(FORMAL_LIMIT_OUTPUT)),
                    name: "OUT".into(),
                    direction: LadPinDirection::Output,
                    data_type: DataType::DInt,
                    required: true,
                    status: LadPortStatus::Active,
                    binding: Some(member(304, OUTPUT)),
                },
            ],
            None,
        )),
        [
            power_port(20, LadPowerPortDirection::Input),
            power_port(21, LadPowerPortDirection::Output),
        ],
    );
    let coil = LadNode::from_power_ports(
        LadNodeId::new(3),
        2,
        LadNodeKind::Coil {
            mode: CoilMode::Normal,
            operand: Some(member(401, POWER)),
        },
        [power_port(30, LadPowerPortDirection::Input)],
    );
    LadDocument::new(
        LadDocumentId::new(0x1154),
        MAIN,
        [LadNetwork::from_parts(
            LadNetworkId::new(1),
            0,
            [source, limit, coil],
            [
                LadPowerEdge {
                    id: LadEdgeId::new(1),
                    source: LadPortId::new(10),
                    target: LadPortId::new(20),
                },
                LadPowerEdge {
                    id: LadEdgeId::new(2),
                    source: LadPortId::new(21),
                    target: LadPortId::new(30),
                },
            ],
            [],
        )],
    )
}

#[test]
fn limit_validates_constants_then_lowers_projects_and_clamps_at_runtime() {
    let program = program();
    let valid = document(10, 99, 20);
    let lowered = lower_lad_to_ir(&valid, &program, LadLimits::default())
        .expect("valid LIMIT lowers to verified IR");
    let projection = project_verified_ir_to_runtime(
        &lowered.verified_ir,
        &lowered.source_maps,
        &lowered.probes,
        &program,
        Hash32::ZERO,
    )
    .expect("LIMIT projects to runtime");
    let output = projection
        .memory_for(MAIN, OUTPUT)
        .expect("LIMIT OUT binding");
    let mut controller =
        VirtualController::new(UniverseId(0x1155), VirtualControllerId(0x1156), 0x5eed);
    controller.power_on().expect("power on");
    controller
        .install_verified_artifact(projection.package())
        .expect("verified load");
    controller
        .request_run(RestartKind::Resume)
        .expect("RUN transition");
    assert!(matches!(
        controller.run_scan(),
        Ok(RunOutcome::Completed(_))
    ));
    assert_eq!(
        controller.actual_memory(output),
        Some(CanonicalValue::I32(20))
    );

    let invalid = validate_lad(&document(20, 15, 10), &program, LadLimits::default());
    assert!(invalid.diagnostics.iter().any(|diagnostic| {
        diagnostic.blocking && diagnostic.reason == LadDiagnosticReason::InvalidConstantRange
    }));
}
