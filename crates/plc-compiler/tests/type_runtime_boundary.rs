use std::collections::BTreeMap;

use plc_compiler::{
    BuildAttempt, BuildAttemptId, BuildOutcome, BuildScope, BuildSnapshot, Compiler,
    CompilerProfile, ResourceLimits, SclSource,
};
use plc_program::{
    BlockId, BlockInterface, ControllerId, ControllerProgram, DataType, EngineeringNumber,
    InterfaceMember, InterfaceMemberId, InterfaceRole, ObDeclaration, ProgramBlock,
    ProgramUnitKind, validate_program,
};
use plc_runtime::{
    CanonicalValue, CpuState, RestartKind, RunOutcome, UniverseId, VirtualController,
    VirtualControllerId,
};
use plc_types::CanonicalF32;

const MAIN: BlockId = BlockId::new(0x7a01);
const WRAP_ADD: InterfaceMemberId = InterfaceMemberId::new(0x7a11);
const WRAP_SUBTRACT: InterfaceMemberId = InterfaceMemberId::new(0x7a12);
const WRAP_MULTIPLY: InterfaceMemberId = InterfaceMemberId::new(0x7a13);
const WRAP_NEGATE: InterfaceMemberId = InterfaceMemberId::new(0x7a14);
const UNSIGNED_WRAP: InterfaceMemberId = InterfaceMemberId::new(0x7a15);
const INFINITY: InterfaceMemberId = InterfaceMemberId::new(0x7a16);
const NOT_A_NUMBER: InterfaceMemberId = InterfaceMemberId::new(0x7a17);
const NEGATIVE_ZERO: InterfaceMemberId = InterfaceMemberId::new(0x7a18);

fn member(id: InterfaceMemberId, name: &str, data_type: DataType, order: u32) -> InterfaceMember {
    InterfaceMember::plain(id, name, InterfaceRole::Temp, data_type, order)
}

fn program() -> ControllerProgram {
    let main = ProgramBlock::new(
        MAIN,
        "TypeBoundary",
        EngineeringNumber::new(1).unwrap(),
        ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain),
        BlockInterface::from_members([
            member(WRAP_ADD, "WrapAdd", DataType::SInt, 0),
            member(WRAP_SUBTRACT, "WrapSubtract", DataType::SInt, 1),
            member(WRAP_MULTIPLY, "WrapMultiply", DataType::SInt, 2),
            member(WRAP_NEGATE, "WrapNegate", DataType::SInt, 3),
            member(UNSIGNED_WRAP, "UnsignedWrap", DataType::USInt, 4),
            member(INFINITY, "Infinity", DataType::Real, 5),
            member(NOT_A_NUMBER, "NotANumber", DataType::Real, 6),
            member(NEGATIVE_ZERO, "NegativeZero", DataType::Real, 7),
        ]),
    );
    let mut program = ControllerProgram::new(ControllerId::new(0x7aff));
    program.insert_block(main).unwrap();
    assert!(validate_program(&program).is_valid());
    program
}

#[test]
fn compiler_verified_ir_and_runtime_share_wrapping_and_ieee_semantics() {
    let program = program();
    let source = SclSource::new(
        MAIN,
        r"
WrapAdd := SINT#127 + SINT#1;
WrapSubtract := (-SINT#127 - SINT#1) - SINT#1;
WrapMultiply := SINT#127 * SINT#2;
WrapNegate := -(SINT#127 + SINT#1);
UnsignedWrap := USINT#255 + USINT#1;
Infinity := REAL#3.4028235e38 * REAL#2.0;
NotANumber := Infinity - Infinity;
NegativeZero := -REAL#0.0;
",
    );
    let sources = BTreeMap::from([(MAIN, source)]);
    let snapshot = BuildSnapshot::capture(&program, &sources, CompilerProfile::edu21_core())
        .expect("canonical type boundary snapshot is valid");
    let current = snapshot.snapshot_hash();
    let completion = Compiler::new(ResourceLimits::default()).unwrap().compile(
        &BuildAttempt::new(
            BuildAttemptId::new(0x7a55),
            snapshot,
            BuildScope::RebuildAllSoftware,
        ),
        current,
        None,
    );
    assert_eq!(
        completion.report().outcome(),
        BuildOutcome::ArtifactCreated,
        "boundary diagnostics: {:?}",
        completion.report().diagnostics()
    );
    let projection = completion
        .artifact()
        .unwrap()
        .runtime_projection()
        .expect("independently verified boundary IR projects to the production runtime");

    let mut controller =
        VirtualController::new(UniverseId(0x7a00), VirtualControllerId(0x7a01), 0x7a02);
    controller.power_on().unwrap();
    controller
        .install_verified_artifact(projection.package())
        .unwrap();
    controller.request_run(RestartKind::Resume).unwrap();
    assert_eq!(controller.cpu_state(), CpuState::Run);
    match controller.run_scan().unwrap() {
        RunOutcome::Completed(report) => assert_eq!(report.scan_sequence, 1),
        RunOutcome::Faulted(event) => {
            panic!("type boundary program unexpectedly faulted: {event:?}")
        }
    }

    let observed = [
        (WRAP_ADD, CanonicalValue::I8(i8::MIN)),
        (WRAP_SUBTRACT, CanonicalValue::I8(i8::MAX)),
        (WRAP_MULTIPLY, CanonicalValue::I8(-2)),
        (WRAP_NEGATE, CanonicalValue::I8(i8::MIN)),
        (UNSIGNED_WRAP, CanonicalValue::U8(0)),
        (
            INFINITY,
            CanonicalValue::F32(CanonicalF32::new(f32::INFINITY)),
        ),
        (
            NOT_A_NUMBER,
            CanonicalValue::F32(CanonicalF32::from_bits(CanonicalF32::QUIET_NAN_BITS)),
        ),
        (NEGATIVE_ZERO, CanonicalValue::F32(CanonicalF32::new(-0.0))),
    ];
    for (member, expected) in observed {
        let memory = projection
            .memory_for(MAIN, member)
            .expect("every boundary member has one deterministic runtime binding");
        assert_eq!(
            controller.actual_memory(memory),
            Some(expected),
            "compiler/runtime semantic drift for member {member:?}"
        );
    }
}

#[test]
fn malformed_or_out_of_range_literals_never_publish_partial_runtime_artifacts() {
    let program = program();
    let invalid_sources = [
        "WrapAdd := SINT#128;",
        "UnsignedWrap := USINT#256;",
        "Infinity := REAL#3.5e38;",
        "WrapAdd := TRUE + SINT#1;",
    ];
    for (ordinal, source) in invalid_sources.into_iter().enumerate() {
        let sources = BTreeMap::from([(MAIN, SclSource::new(MAIN, source))]);
        let snapshot = BuildSnapshot::capture(&program, &sources, CompilerProfile::edu21_core())
            .expect("invalid source is still a valid immutable build snapshot");
        let current = snapshot.snapshot_hash();
        let completion = Compiler::new(ResourceLimits::default()).unwrap().compile(
            &BuildAttempt::new(
                BuildAttemptId::new(0x7b00 + ordinal as u128),
                snapshot,
                BuildScope::RebuildAllSoftware,
            ),
            current,
            None,
        );
        assert_eq!(
            completion.report().outcome(),
            BuildOutcome::BlockingFailure,
            "invalid literal case unexpectedly passed: {source}"
        );
        assert!(completion.artifact().is_none());
    }
}
