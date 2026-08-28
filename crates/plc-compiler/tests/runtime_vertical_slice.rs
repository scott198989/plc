use std::collections::BTreeMap;

use plc_compiler::{
    BuildAttempt, BuildAttemptId, BuildOutcome, BuildScope, BuildSnapshot, Compiler,
    CompilerProfile, ResourceLimits, RuntimeAdapterError, RuntimeMappedSite, SclSource,
};
use plc_program::{
    BlockId, BlockInterface, ControllerId, ControllerProgram, DataType, EngineeringNumber,
    InterfaceMember, InterfaceMemberId, InterfaceRole, ObDeclaration, ProgramBlock,
    ProgramUnitKind, validate_program,
};
use plc_runtime::{
    CanonicalValue, CpuState, RestartKind, RunOutcome, UniverseId, VerifiedArtifact,
    VirtualController, VirtualControllerId,
};

const MAIN: BlockId = BlockId::new(0x101);
const FIRST: InterfaceMemberId = InterfaceMemberId::new(0x1_001);
const SECOND: InterfaceMemberId = InterfaceMemberId::new(0x1_002);
const RESULT: InterfaceMemberId = InterfaceMemberId::new(0x1_003);

fn number(value: u16) -> EngineeringNumber {
    EngineeringNumber::new(value).expect("test engineering number is nonzero")
}

fn member(id: InterfaceMemberId, name: &str, data_type: DataType, order: u32) -> InterfaceMember {
    InterfaceMember::plain(id, name, InterfaceRole::Temp, data_type, order)
}

fn program_with_members(members: impl IntoIterator<Item = InterfaceMember>) -> ControllerProgram {
    let main = ProgramBlock::new(
        MAIN,
        "Main",
        number(1),
        ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain),
        BlockInterface::from_members(members),
    );
    let mut program = ControllerProgram::new(ControllerId::new(0x55));
    program.insert_block(main).expect("unique main block");
    assert!(validate_program(&program).is_valid());
    program
}

fn compile(
    program: &ControllerProgram,
    source: &str,
    attempt_id: u128,
) -> plc_compiler::BuildCompletion {
    let sources = BTreeMap::from([(MAIN, SclSource::new(MAIN, source))]);
    let snapshot = BuildSnapshot::capture(program, &sources, CompilerProfile::edu21_core())
        .expect("runtime fixture snapshot is valid");
    let current = snapshot.snapshot_hash();
    let attempt = BuildAttempt::new(
        BuildAttemptId::new(attempt_id),
        snapshot,
        BuildScope::RebuildAllSoftware,
    );
    Compiler::new(ResourceLimits::default())
        .expect("compiler initializes")
        .compile(&attempt, current, None)
}

#[test]
fn nontrivial_scl_compiles_loads_runs_and_is_observed_through_virtual_controller() {
    let program = program_with_members([
        member(FIRST, "First", DataType::DInt, 0),
        member(SECOND, "Second", DataType::DInt, 1),
        member(RESULT, "Result", DataType::DInt, 2),
    ]);
    let source = "First := DINT#7; Second := First + DINT#5; Result := Second / DINT#3;";
    let first = compile(&program, source, 1);
    assert_eq!(first.report().outcome(), BuildOutcome::ArtifactCreated);
    let artifact = first
        .artifact()
        .expect("valid full build publishes artifact");
    let projection = artifact
        .runtime_projection()
        .expect("admitted linear SCL lowers to production runtime artifact");

    let accepted = VerifiedArtifact::accept(projection.package())
        .expect("production runtime independently accepts compiler package");
    assert_eq!(accepted.fingerprint(), projection.package().fingerprint());
    let first_memory = projection.memory_for(MAIN, FIRST).unwrap();
    let second_memory = projection.memory_for(MAIN, SECOND).unwrap();
    let result_memory = projection.memory_for(MAIN, RESULT).unwrap();
    let runtime_block = projection.block_for(MAIN).unwrap();

    let instructions = &projection.package().spec().program.cyclic.instructions;
    assert!(
        instructions.len() >= 6,
        "fixture must exercise genuine dataflow"
    );
    for instruction in instructions {
        let binding = projection
            .source_for(instruction.source_identity)
            .expect("every emitted runtime instruction retains source/probe identity");
        assert!(matches!(
            binding.runtime_site,
            RuntimeMappedSite::Instruction {
                block,
                operation_id,
                source_identity,
            } if block == runtime_block
                && operation_id == instruction.operation_id
                && source_identity == instruction.source_identity
        ));
        assert!(!binding.anchors.is_empty());
    }
    assert!(projection.source_bindings().iter().any(|binding| matches!(
        binding.runtime_site,
        RuntimeMappedSite::BlockReturn { block } if block == runtime_block
    )));

    let mut controller =
        VirtualController::new(UniverseId(0xCAFE), VirtualControllerId(0xBEEF), 0x1234_5678);
    assert_eq!(controller.cpu_state(), CpuState::PoweredOff);
    controller.power_on().unwrap();
    controller
        .install_verified_artifact(projection.package())
        .expect("verified compiler package installs atomically");
    controller.request_run(RestartKind::Resume).unwrap();
    let report = match controller.run_scan().expect("scan command is legal") {
        RunOutcome::Completed(report) => report,
        RunOutcome::Faulted(event) => panic!("compiled program unexpectedly faulted: {event:?}"),
    };
    assert_eq!(report.scan_sequence, 1);
    assert_eq!(
        controller.actual_memory(first_memory),
        Some(CanonicalValue::I32(7))
    );
    assert_eq!(
        controller.actual_memory(second_memory),
        Some(CanonicalValue::I32(12))
    );
    assert_eq!(
        controller.actual_memory(result_memory),
        Some(CanonicalValue::I32(4))
    );
    assert_eq!(controller.invocation_ordinal(runtime_block), Some(1));

    let second = compile(&program, source, u128::MAX);
    let second_projection = second.artifact().unwrap().runtime_projection().unwrap();
    assert_eq!(
        projection.package().fingerprint(),
        second_projection.package().fingerprint(),
        "attempt identity must not perturb the runnable package"
    );
    assert_eq!(
        projection.memory_bindings(),
        second_projection.memory_bindings()
    );
    assert_eq!(
        projection.block_bindings(),
        second_projection.block_bindings()
    );
    assert_eq!(
        projection.source_bindings(),
        second_projection.source_bindings()
    );
}

#[test]
fn runtime_projection_rejects_compiler_ir_it_cannot_execute_without_approximation() {
    let branch_program = program_with_members([
        member(FIRST, "Gate", DataType::Bool, 0),
        member(RESULT, "Result", DataType::DInt, 1),
    ]);
    let completion = compile(
        &branch_program,
        "Gate := TRUE; IF Gate THEN Result := 1; ELSE Result := 2; END_IF;",
        2,
    );
    assert_eq!(completion.report().outcome(), BuildOutcome::ArtifactCreated);
    assert!(matches!(
        completion.artifact().unwrap().runtime_projection(),
        Err(RuntimeAdapterError::UnsupportedControlFlow { owner: MAIN, .. })
    ));

    let real_program = program_with_members([member(RESULT, "Result", DataType::Real, 0)]);
    let completion = compile(&real_program, "Result := REAL#1.25;", 3);
    assert_eq!(completion.report().outcome(), BuildOutcome::ArtifactCreated);
    assert_eq!(
        completion.artifact().unwrap().runtime_projection(),
        Err(RuntimeAdapterError::UnsupportedMemberType {
            owner: MAIN,
            member: RESULT,
            data_type: DataType::Real,
        })
    );
}
