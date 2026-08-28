use std::collections::BTreeMap;

use plc_compiler::{
    BuildAttempt, BuildAttemptId, BuildOutcome, BuildScope, BuildSnapshot, Compiler,
    CompilerProfile, ResourceLimits, RuntimeMappedSite, SclSource,
};
use plc_program::{
    BlockId, BlockInterface, ControllerId, ControllerProgram, DataType, EngineeringNumber,
    InterfaceMember, InterfaceMemberId, InterfaceRole, ObDeclaration, ProgramBlock,
    ProgramUnitKind, validate_program,
};
use plc_runtime::{
    CanonicalValue, CpuState, RestartKind, RunOutcome, UniverseId, ValueType, VerifiedArtifact,
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
            }
            | RuntimeMappedSite::Terminator {
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
        RuntimeMappedSite::Terminator { block, .. } if block == runtime_block
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
fn runtime_projection_executes_control_flow_and_real_without_approximation() {
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
    let branch_projection = completion
        .artifact()
        .unwrap()
        .runtime_projection()
        .expect("verified branch CFG is executable");
    let mut branch_controller =
        VirtualController::new(UniverseId(0xCAFE), VirtualControllerId(0xBEF1), 0x43);
    branch_controller.power_on().unwrap();
    branch_controller
        .install_verified_artifact(branch_projection.package())
        .unwrap();
    branch_controller.request_run(RestartKind::Resume).unwrap();
    assert!(matches!(
        branch_controller.run_scan().unwrap(),
        RunOutcome::Completed(_)
    ));
    assert_eq!(
        branch_controller.actual_memory(branch_projection.memory_for(MAIN, RESULT).unwrap()),
        Some(CanonicalValue::I32(1))
    );

    let real_program = program_with_members([member(RESULT, "Result", DataType::Real, 0)]);
    let completion = compile(&real_program, "Result := REAL#1.25;", 3);
    assert_eq!(completion.report().outcome(), BuildOutcome::ArtifactCreated);
    let artifact = completion.artifact().unwrap();
    let projection_result = artifact.runtime_projection();
    let projection = projection_result
        .as_ref()
        .expect("REAL is an exact scalar runtime type");
    let memory = projection.memory_for(MAIN, RESULT).unwrap();
    let definition = projection
        .package()
        .spec()
        .memory
        .iter()
        .find(|definition| definition.id == memory)
        .unwrap();
    assert_eq!(definition.value_type, ValueType::F32);
}

#[test]
#[allow(clippy::too_many_lines)]
fn verified_scl_fc_executes_with_copy_in_copy_out_and_two_call_outputs() {
    let function = BlockId::new(0x202);
    let formal_x = InterfaceMemberId::new(0x2_001);
    let formal_sum = InterfaceMemberId::new(0x2_002);
    let formal_product = InterfaceMemberId::new(0x2_003);
    let mut sum =
        InterfaceMember::plain(formal_sum, "Sum", InterfaceRole::Output, DataType::DInt, 0);
    sum.required_output_binding = true;
    let mut product = InterfaceMember::plain(
        formal_product,
        "Product",
        InterfaceRole::Output,
        DataType::DInt,
        1,
    );
    product.required_output_binding = true;
    let main = ProgramBlock::new(
        MAIN,
        "Main",
        number(1),
        ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain),
        BlockInterface::from_members([
            member(FIRST, "Arg", DataType::DInt, 0),
            member(SECOND, "Sum", DataType::DInt, 1),
            member(RESULT, "Product", DataType::DInt, 2),
        ]),
    );
    let function_block = ProgramBlock::new(
        function,
        "Calculate",
        number(2),
        ProgramUnitKind::Function,
        BlockInterface::from_members([
            InterfaceMember::plain(formal_x, "X", InterfaceRole::Input, DataType::DInt, 0),
            sum,
            product,
        ]),
    );
    let mut program = ControllerProgram::new(ControllerId::new(0x56));
    program.insert_block(main).unwrap();
    program.insert_block(function_block).unwrap();
    assert!(validate_program(&program).is_valid());

    let sources = BTreeMap::from([
        (
            MAIN,
            SclSource::new(
                MAIN,
                "Arg := DINT#4; Calculate(X := Arg, Sum => Sum, Product => Product);",
            ),
        ),
        (
            function,
            SclSource::new(function, "Sum := X + DINT#3; Product := X * DINT#2;"),
        ),
    ]);
    let snapshot = BuildSnapshot::capture(&program, &sources, CompilerProfile::edu21_core())
        .expect("two-block SCL snapshot");
    let current = snapshot.snapshot_hash();
    let completion = Compiler::new(ResourceLimits::default()).unwrap().compile(
        &BuildAttempt::new(
            BuildAttemptId::new(0x44),
            snapshot,
            BuildScope::RebuildAllSoftware,
        ),
        current,
        None,
    );
    assert_eq!(completion.report().outcome(), BuildOutcome::ArtifactCreated);
    let projection = completion
        .artifact()
        .unwrap()
        .runtime_projection()
        .expect("verified FC is runnable");
    let function_runtime_block = projection.block_for(function).expect("callable binding");
    let function_output_memory = projection
        .memory_for(function, formal_sum)
        .expect("callee frame binding");

    let mut controller =
        VirtualController::new(UniverseId(0xCAFE), VirtualControllerId(0xBEF0), 0x44);
    controller.power_on().unwrap();
    controller
        .install_verified_artifact(projection.package())
        .unwrap();
    controller.request_run(RestartKind::Resume).unwrap();
    let report = match controller.run_scan().unwrap() {
        RunOutcome::Completed(report) => report,
        RunOutcome::Faulted(event) => panic!("FC execution faulted: {event:?}"),
    };
    assert_eq!(
        controller.actual_memory(projection.memory_for(MAIN, SECOND).unwrap()),
        Some(CanonicalValue::I32(7))
    );
    assert_eq!(
        controller.actual_memory(projection.memory_for(MAIN, RESULT).unwrap()),
        Some(CanonicalValue::I32(8))
    );
    assert_eq!(
        controller.actual_memory(function_output_memory),
        Some(CanonicalValue::I32(0)),
        "FC frame storage must not leak into global runtime memory"
    );
    assert_eq!(
        controller.invocation_ordinal(function_runtime_block),
        Some(1)
    );
    assert_eq!(report.call_boundaries.len(), 2);
    assert_eq!(
        report.call_boundaries[0].callee_block,
        function_runtime_block
    );
    assert_eq!(
        report.call_boundaries[0].source_identity,
        report.call_boundaries[1].source_identity
    );
    let call_source = projection
        .source_for(report.call_boundaries[0].source_identity)
        .expect("call boundary retains its verified source/probe binding");
    assert!(matches!(
        call_source.runtime_site,
        RuntimeMappedSite::Instruction {
            block,
            operation_id: _,
            source_identity: _
        } if block == projection.block_for(MAIN).unwrap()
    ));
    assert!(!call_source.anchors.is_empty());
}
