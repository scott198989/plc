use plc_compiler::{Hash32, IrOperationKind, project_verified_ir_to_runtime};
use plc_lad::{
    CoilMode, LadBox, LadDocument, LadDocumentId, LadEdgeId, LadFormalRef, LadLimits, LadNetwork,
    LadNetworkId, LadNode, LadNodeId, LadNodeKind, LadOperand, LadOperandId, LadOperandRef, LadPin,
    LadPinDirection, LadPortId, LadPortStatus, LadPowerEdge, LadPowerPort, LadPowerPortDirection,
    lower_lad_to_ir,
};
use plc_program::{
    BLKMOVE, BlockId, BlockInterface, CanonicalValue as ProgramValue, ControllerId,
    ControllerProgram, DataType, EngineeringNumber, FILL, FORMAL_INPUT, FORMAL_OUTPUT,
    InterfaceMember, InterfaceMemberId, InterfaceRole, ObDeclaration, ProgramBlock,
    ProgramUnitKind, VariableRef,
};
use plc_runtime::{RestartKind, RunOutcome, UniverseId, VirtualController, VirtualControllerId};
use plc_types::{ArrayBound, CanonicalType, PlcValue, PrimitiveType, ScalarValue, TypedScalar};

const MAIN: BlockId = BlockId::new(0xA660);
const FILL_VALUE: InterfaceMemberId = InterfaceMemberId::new(0xA661);
const FILLED: InterfaceMemberId = InterfaceMemberId::new(0xA662);
const COPIED: InterfaceMemberId = InterfaceMemberId::new(0xA663);
const POWER: InterfaceMemberId = InterfaceMemberId::new(0xA664);

fn dint(value: i32) -> PlcValue {
    PlcValue::Scalar(
        TypedScalar::new(PrimitiveType::Dint, ScalarValue::Signed(i64::from(value)))
            .expect("valid DINT"),
    )
}

fn values(items: &[i32]) -> PlcValue {
    PlcValue::Array(items.iter().copied().map(dint).collect())
}

fn array_type() -> CanonicalType {
    CanonicalType::Array {
        dimensions: vec![ArrayBound { lower: 0, upper: 2 }],
        element_type: Box::new(CanonicalType::Primitive(PrimitiveType::Dint)),
    }
}

fn program() -> ControllerProgram {
    let mut fill_value = InterfaceMember::plain(
        FILL_VALUE,
        "FillValue",
        InterfaceRole::Temp,
        DataType::DInt,
        0,
    );
    fill_value.start_value = Some(ProgramValue::DInt(7));
    let mut filled = InterfaceMember::plain(
        FILLED,
        "Filled",
        InterfaceRole::Temp,
        DataType::Aggregate(array_type()),
        1,
    );
    filled.start_value = Some(ProgramValue::Aggregate(values(&[0, 0, 0])));
    let mut copied = InterfaceMember::plain(
        COPIED,
        "Copied",
        InterfaceRole::Temp,
        DataType::Aggregate(array_type()),
        2,
    );
    copied.start_value = Some(ProgramValue::Aggregate(values(&[9, 9, 9])));
    let block = ProgramBlock::new(
        MAIN,
        "Main",
        EngineeringNumber::new(1).expect("nonzero OB number"),
        ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain),
        BlockInterface::from_members([
            fill_value,
            filled,
            copied,
            InterfaceMember::plain(POWER, "Power", InterfaceRole::Temp, DataType::Bool, 3),
        ]),
    );
    let mut program = ControllerProgram::new(ControllerId::new(0xA665));
    program.insert_block(block).expect("unique main block");
    program
}

fn member_operand(id: u128, member: InterfaceMemberId) -> LadOperandRef {
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

#[allow(clippy::too_many_arguments)]
fn aggregate_box(
    id: u128,
    order: u32,
    input_power: u128,
    output_power: u128,
    instruction: plc_program::InstructionCode,
    input_member: InterfaceMemberId,
    input_type: DataType,
    target_member: InterfaceMemberId,
) -> LadNode {
    LadNode::from_power_ports(
        LadNodeId::new(id),
        order,
        LadNodeKind::Box(LadBox::from_pins(
            instruction,
            [
                LadPin {
                    id: LadPortId::new(id * 100 + 1),
                    formal: Some(LadFormalRef::Instruction(FORMAL_INPUT)),
                    name: "IN".into(),
                    direction: LadPinDirection::Input,
                    data_type: input_type,
                    required: true,
                    status: LadPortStatus::Active,
                    binding: Some(member_operand(id * 1_000 + 1, input_member)),
                },
                LadPin {
                    id: LadPortId::new(id * 100 + 2),
                    formal: Some(LadFormalRef::Instruction(FORMAL_OUTPUT)),
                    name: "OUT".into(),
                    direction: LadPinDirection::Output,
                    data_type: DataType::Aggregate(array_type()),
                    required: true,
                    status: LadPortStatus::Active,
                    binding: Some(member_operand(id * 1_000 + 2, target_member)),
                },
            ],
            None,
        )),
        [
            power_port(input_power, LadPowerPortDirection::Input),
            power_port(output_power, LadPowerPortDirection::Output),
        ],
    )
}

fn document() -> LadDocument {
    let source = LadNode::from_power_ports(
        LadNodeId::new(1),
        0,
        LadNodeKind::PowerSource,
        [power_port(10, LadPowerPortDirection::Output)],
    );
    let fill = aggregate_box(2, 1, 20, 21, FILL, FILL_VALUE, DataType::DInt, FILLED);
    let block_move = aggregate_box(
        3,
        2,
        30,
        31,
        BLKMOVE,
        FILLED,
        DataType::Aggregate(array_type()),
        COPIED,
    );
    let coil = LadNode::from_power_ports(
        LadNodeId::new(4),
        3,
        LadNodeKind::Coil {
            mode: CoilMode::Normal,
            operand: Some(member_operand(4_001, POWER)),
        },
        [power_port(40, LadPowerPortDirection::Input)],
    );
    LadDocument::new(
        LadDocumentId::new(0xA666),
        MAIN,
        [LadNetwork::from_parts(
            LadNetworkId::new(1),
            0,
            [source, fill, block_move, coil],
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
                LadPowerEdge {
                    id: LadEdgeId::new(3),
                    source: LadPortId::new(31),
                    target: LadPortId::new(40),
                },
            ],
            [],
        )],
    )
}

#[test]
fn lad_fill_and_block_move_lower_verify_project_and_execute_through_one_runtime() {
    let program = program();
    let lowered = lower_lad_to_ir(&document(), &program, LadLimits::default())
        .expect("registry-valid aggregate LAD lowers to verified IR");
    let operations = &lowered.ir().functions()[&MAIN]
        .blocks
        .values()
        .next()
        .unwrap()
        .operations;
    assert!(operations.iter().any(|operation| matches!(
        operation.kind,
        IrOperationKind::AggregateInstruction {
            instruction: FILL,
            ..
        }
    )));
    assert!(operations.iter().any(|operation| matches!(
        operation.kind,
        IrOperationKind::AggregateInstruction {
            instruction: BLKMOVE,
            ..
        }
    )));

    let projection = project_verified_ir_to_runtime(
        &lowered.verified_ir,
        &lowered.source_maps,
        &lowered.probes,
        &program,
        Hash32::ZERO,
    )
    .expect("aggregate IR projects to the production runtime");
    let filled = projection
        .aggregate_memory_for(MAIN, FILLED)
        .expect("FILL aggregate runtime binding");
    let copied = projection
        .aggregate_memory_for(MAIN, COPIED)
        .expect("BLKMOVE aggregate runtime binding");
    let mut controller =
        VirtualController::new(UniverseId(0xA667), VirtualControllerId(0xA668), 0x5eed);
    controller.power_on().expect("power on");
    controller
        .install_verified_artifact(projection.package())
        .expect("install verified aggregate artifact");
    controller
        .request_run(RestartKind::Resume)
        .expect("RUN transition");
    match controller.run_scan().expect("aggregate scan") {
        RunOutcome::Completed(_) => {}
        RunOutcome::Faulted(event) => panic!("aggregate runtime faulted: {event:?}"),
    }
    assert_eq!(
        controller.actual_aggregate_memory(filled),
        Some(&values(&[7, 7, 7]))
    );
    assert_eq!(
        controller.actual_aggregate_memory(copied),
        Some(&values(&[7, 7, 7]))
    );
    assert_eq!(controller.diagnostics().len(), 0);
}
