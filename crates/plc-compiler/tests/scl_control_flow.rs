use std::collections::BTreeMap;

use plc_compiler::{
    BuildAttempt, BuildAttemptId, BuildOutcome, BuildScope, BuildSnapshot, Compiler,
    CompilerProfile, DiagnosticCode as CompilerDiagnosticCode, ResourceLimits, RuntimeMappedSite,
    SclFrontendError, SclSource, lower_scl_frontend_artifact,
};
use plc_program::{
    BlockId, BlockInterface, CanonicalValue as ProgramCanonicalValue, ControllerId,
    ControllerProgram, DataType, EngineeringNumber, InterfaceMember, InterfaceMemberId,
    InterfaceRole, ObDeclaration, ProgramBlock, ProgramUnitKind,
};
use plc_runtime::{
    CanonicalValue, CpuState, DiagnosticCode, RestartKind, RunOutcome, UniverseId,
    VirtualController, VirtualControllerId,
};

const MAIN: BlockId = BlockId::new(0x5c1);
const WRITER: BlockId = BlockId::new(0x5c2);
const SELECTOR: InterfaceMemberId = InterfaceMemberId::new(0x5c10);
const INDEX: InterfaceMemberId = InterfaceMemberId::new(0x5c11);
const RESULT: InterfaceMemberId = InterfaceMemberId::new(0x5c12);
const FLAG: InterfaceMemberId = InterfaceMemberId::new(0x5c13);
const SMALL: InterfaceMemberId = InterfaceMemberId::new(0x5c14);
const UNSIGNED_INDEX: InterfaceMemberId = InterfaceMemberId::new(0x5c15);
const CASE_LIMIT: InterfaceMemberId = InterfaceMemberId::new(0x5c16);
const WRITER_INPUT: InterfaceMemberId = InterfaceMemberId::new(0x5c20);
const WRITER_OUTPUT: InterfaceMemberId = InterfaceMemberId::new(0x5c21);

fn member(id: InterfaceMemberId, name: &str, data_type: DataType, order: u32) -> InterfaceMember {
    InterfaceMember::plain(id, name, InterfaceRole::Temp, data_type, order)
}

fn program() -> ControllerProgram {
    let mut case_limit = InterfaceMember::plain(
        CASE_LIMIT,
        "CaseLimit",
        InterfaceRole::Constant,
        DataType::DInt,
        6,
    );
    case_limit.constant_value = Some(ProgramCanonicalValue::DInt(3));
    let main = ProgramBlock::new(
        MAIN,
        "Main",
        EngineeringNumber::new(1).expect("nonzero engineering number"),
        ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain),
        BlockInterface::from_members([
            member(SELECTOR, "Selector", DataType::DInt, 0),
            member(INDEX, "Index", DataType::DInt, 1),
            member(RESULT, "Result", DataType::DInt, 2),
            member(FLAG, "Flag", DataType::Bool, 3),
            member(SMALL, "Small", DataType::SInt, 4),
            member(UNSIGNED_INDEX, "UnsignedIndex", DataType::UInt, 5),
            case_limit,
        ]),
    );
    let mut program = ControllerProgram::new(ControllerId::new(0x5c));
    program.insert_block(main).expect("unique main block");
    program
}

fn program_with_writer() -> ControllerProgram {
    let mut program = program();
    let mut output =
        InterfaceMember::plain(WRITER_OUTPUT, "Y", InterfaceRole::Output, DataType::DInt, 0);
    output.required_output_binding = true;
    let writer = ProgramBlock::new(
        WRITER,
        "Writer",
        EngineeringNumber::new(2).expect("nonzero engineering number"),
        ProgramUnitKind::Function,
        BlockInterface::from_members([
            InterfaceMember::plain(WRITER_INPUT, "X", InterfaceRole::Input, DataType::DInt, 0),
            output,
        ]),
    );
    program.insert_block(writer).expect("unique writer block");
    program
}

fn compile(source: &str) -> plc_compiler::BuildCompletion {
    let program = program();
    let sources = BTreeMap::from([(MAIN, SclSource::new(MAIN, source))]);
    let snapshot = BuildSnapshot::capture(&program, &sources, CompilerProfile::edu21_core())
        .expect("control-flow fixture snapshot");
    let current = snapshot.snapshot_hash();
    Compiler::new(ResourceLimits::default())
        .expect("compiler")
        .compile(
            &BuildAttempt::new(
                BuildAttemptId::new(0x5c),
                snapshot,
                BuildScope::RebuildAllSoftware,
            ),
            current,
            None,
        )
}

fn run(source: &str) -> (plc_compiler::RuntimeArtifactProjection, VirtualController) {
    let completion = compile(source);
    assert_eq!(
        completion.report().outcome(),
        BuildOutcome::ArtifactCreated,
        "{:?}",
        completion.report().diagnostics()
    );
    let projection = completion
        .artifact()
        .expect("artifact")
        .runtime_projection()
        .expect("verified CFG projects to runtime");
    let mut controller = VirtualController::new(UniverseId(0x5c), VirtualControllerId(0x5c), 0x5c);
    assert_eq!(controller.cpu_state(), CpuState::PoweredOff);
    controller.power_on().expect("power on");
    controller
        .install_verified_artifact(projection.package())
        .expect("install verified CFG");
    controller.request_run(RestartKind::Resume).expect("run");
    match controller.run_scan().expect("scan command") {
        RunOutcome::Completed(_) => {}
        RunOutcome::Faulted(event) => panic!("control-flow fixture faulted: {event:?}"),
    }
    (projection, controller)
}

#[test]
fn case_for_while_repeat_exit_and_continue_execute_through_verified_cfg() {
    let source = r"
Selector := DINT#3;
Result := DINT#0;
CASE Selector OF
    DINT#0 + DINT#1: Result := DINT#10;
    DINT#2..DINT#4: Result := DINT#20;
ELSE
    Result := DINT#30;
END_CASE;
FOR Index := DINT#1 TO DINT#5 BY DINT#1 DO
    IF Index = DINT#2 THEN CONTINUE; END_IF;
    Result := Result + Index;
    IF Index = DINT#4 THEN EXIT; END_IF;
END_FOR;
WHILE Result < DINT#30 DO
    Result := Result + DINT#1;
END_WHILE;
REPEAT
    Result := Result - DINT#1;
UNTIL Result = DINT#29
END_REPEAT;
";
    let (projection, controller) = run(source);
    assert_eq!(
        controller.actual_memory(projection.memory_for(MAIN, RESULT).expect("result binding")),
        Some(CanonicalValue::I32(29))
    );
    assert_eq!(
        controller.actual_memory(projection.memory_for(MAIN, INDEX).expect("index binding")),
        Some(CanonicalValue::I32(4))
    );
    assert!(
        projection
            .source_bindings()
            .iter()
            .any(|binding| matches!(binding.runtime_site, RuntimeMappedSite::Terminator { .. }))
    );
    assert!(projection.source_bindings().iter().all(|binding| {
        !binding.anchors.is_empty()
            && binding.anchors.iter().all(|anchor| {
                anchor.text_range.is_some_and(|range| {
                    range.end <= u32::try_from(source.len()).unwrap_or(u32::MAX)
                })
            })
    }));

    let second = compile(source);
    let second = second
        .artifact()
        .expect("repeat artifact")
        .runtime_projection()
        .expect("repeat CFG projection");
    assert_eq!(
        projection.package().fingerprint(),
        second.package().fingerprint(),
        "control-flow artifact must be deterministic"
    );
    assert_eq!(projection.source_bindings(), second.source_bindings());
}

#[test]
fn descending_for_uses_signed_by_and_default_step_is_one() {
    let source = r"
Result := DINT#0;
FOR Index := DINT#3 TO DINT#1 BY -1 DO
    Result := Result + Index;
END_FOR;
FOR Index := DINT#1 TO DINT#3 DO
    Result := Result + Index;
END_FOR;
";
    let (projection, controller) = run(source);
    assert_eq!(
        controller.actual_memory(projection.memory_for(MAIN, RESULT).expect("result binding")),
        Some(CanonicalValue::I32(12))
    );
}

#[test]
fn for_by_is_evaluated_once_at_entry_and_uses_its_runtime_sign() {
    let source = r"
Selector := DINT#1;
Result := DINT#0;
FOR Index := DINT#1 TO DINT#5 BY Selector DO
    Result := Result + DINT#1;
    Selector := DINT#2;
END_FOR;
Selector := -DINT#2;
FOR Index := DINT#5 TO DINT#1 BY Selector DO
    Result := Result + DINT#10;
END_FOR;
";
    let (projection, controller) = run(source);
    assert_eq!(
        controller.actual_memory(projection.memory_for(MAIN, RESULT).expect("result binding")),
        Some(CanonicalValue::I32(35)),
        "the first loop must retain its entry step of +1 and the second must select descending execution from -2"
    );
    assert_eq!(
        controller.actual_memory(projection.memory_for(MAIN, INDEX).expect("index binding")),
        Some(CanonicalValue::I32(1))
    );
}

#[test]
fn runtime_zero_for_step_faults_before_the_loop_body_store() {
    let source = r"
Selector := DINT#0;
Result := DINT#7;
FOR Index := DINT#1 TO DINT#3 BY Selector DO
    Result := DINT#99;
END_FOR;
";
    let completion = compile(source);
    assert_eq!(completion.report().outcome(), BuildOutcome::ArtifactCreated);
    let projection = completion
        .artifact()
        .expect("dynamic zero-step artifact")
        .runtime_projection()
        .expect("verified dynamic zero-step CFG");
    let mut controller = VirtualController::new(UniverseId(0x5e), VirtualControllerId(0x5e), 0x5e);
    controller.power_on().expect("power on");
    controller
        .install_verified_artifact(projection.package())
        .expect("install");
    controller.request_run(RestartKind::Resume).expect("run");
    let event = match controller.run_scan().expect("scan") {
        RunOutcome::Faulted(event) => event,
        RunOutcome::Completed(_) => panic!("dynamic zero FOR step must fault"),
    };
    assert_eq!(event.code, DiagnosticCode::InvalidArgument);
    assert_eq!(
        controller.actual_memory(projection.memory_for(MAIN, RESULT).expect("result binding")),
        Some(CanonicalValue::I32(7)),
        "the loop body must not execute before zero-step rejection"
    );
    let context = event.fault_context.expect("zero-step source occurrence");
    let binding = projection
        .source_for(context.source_identity)
        .expect("runtime fault maps back to the FOR occurrence");
    assert!(binding.anchors.iter().any(|anchor| {
        anchor.text_range.is_some_and(|range| {
            let range = usize::try_from(range.start).expect("range start")
                ..usize::try_from(range.end).expect("range end");
            source[range].starts_with("FOR Index")
        })
    }));
}

#[test]
fn for_stops_at_the_integer_boundary_without_wrapping() {
    let source = r"
Result := DINT#0;
FOR Index := DINT#2147483646 TO DINT#2147483647 BY DINT#1 DO
    Result := Result + DINT#1;
END_FOR;
";
    let (projection, controller) = run(source);
    assert_eq!(
        controller.actual_memory(projection.memory_for(MAIN, RESULT).expect("result binding")),
        Some(CanonicalValue::I32(2))
    );
    assert_eq!(
        controller.actual_memory(projection.memory_for(MAIN, INDEX).expect("index binding")),
        Some(CanonicalValue::I32(i32::MAX))
    );
}

#[test]
fn for_uses_widened_crossing_test_without_storing_an_unused_next_value() {
    let source = r"
Result := DINT#0;
FOR Index := DINT#2147483646 TO DINT#2147483647 BY DINT#2 DO
    Result := Result + DINT#1;
END_FOR;
";
    let (projection, controller) = run(source);
    assert_eq!(
        controller.actual_memory(projection.memory_for(MAIN, RESULT).expect("result binding")),
        Some(CanonicalValue::I32(1))
    );
    assert_eq!(
        controller.actual_memory(projection.memory_for(MAIN, INDEX).expect("index binding")),
        Some(CanonicalValue::I32(i32::MAX - 1))
    );
}

#[test]
fn for_values_use_the_unique_implicit_conversion_to_the_iterator_type() {
    let source = r"
Small := SINT#1;
Result := DINT#0;
FOR Index := Small TO SINT#3 BY SINT#1 DO
    Result := Result + DINT#1;
END_FOR;
";
    let (projection, controller) = run(source);
    assert_eq!(
        controller.actual_memory(projection.memory_for(MAIN, RESULT).expect("result binding")),
        Some(CanonicalValue::I32(3))
    );
    assert_eq!(
        controller.actual_memory(projection.memory_for(MAIN, INDEX).expect("index binding")),
        Some(CanonicalValue::I32(3))
    );
}

#[test]
fn case_labels_accept_canonical_constant_symbols() {
    let source = r"
Selector := DINT#3;
Result := DINT#0;
CASE Selector OF
    CaseLimit: Result := DINT#7;
ELSE
    Result := DINT#9;
END_CASE;
";
    let (projection, controller) = run(source);
    assert_eq!(
        controller.actual_memory(projection.memory_for(MAIN, RESULT).expect("result binding")),
        Some(CanonicalValue::I32(7))
    );
}

#[test]
fn nested_exit_and_continue_target_the_innermost_active_loop() {
    let source = r"
Result := DINT#0;
FOR Index := DINT#1 TO DINT#3 DO
    Selector := DINT#0;
    WHILE Selector < DINT#3 DO
        Selector := Selector + DINT#1;
        IF Selector = DINT#1 THEN CONTINUE; END_IF;
        Result := Result + DINT#1;
        EXIT;
    END_WHILE;
    IF Index = DINT#2 THEN CONTINUE; END_IF;
    Result := Result + DINT#10;
END_FOR;
";
    let (projection, controller) = run(source);
    assert_eq!(
        controller.actual_memory(projection.memory_for(MAIN, RESULT).expect("result binding")),
        Some(CanonicalValue::I32(23))
    );
}

#[test]
fn case_and_loop_legality_fail_closed_with_stable_diagnostics() {
    let program = program();
    for (source, code) in [
        (
            "Selector := 1; CASE Selector OF 1..3: Result := 1; 3..5: Result := 2; END_CASE;",
            CompilerDiagnosticCode::INVALID_CONTROL_FLOW,
        ),
        (
            "Selector := 1; CASE Selector OF 5..2: Result := 1; END_CASE;",
            CompilerDiagnosticCode::CONSTANT_RANGE_OR_ARITHMETIC,
        ),
        (
            "Selector := 1; CASE Selector OF Selector: Result := 1; END_CASE;",
            CompilerDiagnosticCode::CONSTANT_RANGE_OR_ARITHMETIC,
        ),
        (
            "Selector := 1; CASE Selector OF 1 / 0: Result := 1; END_CASE;",
            CompilerDiagnosticCode::CONSTANT_RANGE_OR_ARITHMETIC,
        ),
        ("EXIT;", CompilerDiagnosticCode::INVALID_CONTROL_FLOW),
        (
            "FOR Index := 1 TO 3 BY 0 DO Result := 1; END_FOR;",
            CompilerDiagnosticCode::CONSTANT_RANGE_OR_ARITHMETIC,
        ),
        (
            "FOR Index := 1 TO 3 DO Index := 2; END_FOR;",
            CompilerDiagnosticCode::INVALID_CONTROL_FLOW,
        ),
        (
            "FOR UnsignedIndex := UINT#1 TO UINT#3 DO Result := 1; END_FOR;",
            CompilerDiagnosticCode::TYPE_MISMATCH,
        ),
    ] {
        let result = lower_scl_frontend_artifact(
            &program,
            &SclSource::new(MAIN, source),
            ResourceLimits::default(),
        );
        let SclFrontendError::Diagnostics(issues) = result.expect_err("illegal control flow")
        else {
            panic!("illegal source must fail with authored diagnostics");
        };
        assert!(
            issues.iter().any(|issue| issue.code == code),
            "missing {code:?} for {source:?}: {issues:?}"
        );
    }

    let unsupported = lower_scl_frontend_artifact(
        &program,
        &SclSource::new(MAIN, "Result.member := 1;"),
        ResourceLimits::default(),
    );
    let SclFrontendError::Diagnostics(issues) = unsupported.expect_err("unsupported neighbor")
    else {
        panic!("member assignment must remain fail-closed");
    };
    assert!(
        issues
            .iter()
            .any(|issue| { issue.code == CompilerDiagnosticCode::RECOGNIZED_UNSUPPORTED_SYNTAX })
    );
}

#[test]
fn active_for_iterator_cannot_be_written_through_a_call_output() {
    let program = program_with_writer();
    let source = "FOR Index := 1 TO 3 DO Writer(X := Index, Y => Index); END_FOR;";
    let error = lower_scl_frontend_artifact(
        &program,
        &SclSource::new(MAIN, source),
        ResourceLimits::default(),
    )
    .expect_err("call output must not write an active iterator");
    let SclFrontendError::Diagnostics(issues) = error else {
        panic!("iterator call write must fail with authored diagnostics");
    };
    assert!(issues.iter().any(|issue| {
        issue.code == CompilerDiagnosticCode::INVALID_CONTROL_FLOW
            && issue.cause.contains("active FOR iterator")
    }));
}

#[test]
fn infinite_structured_loop_is_stopped_by_the_runtime_work_budget() {
    let completion = compile("Flag := TRUE; WHILE Flag DO CONTINUE; END_WHILE;");
    assert_eq!(completion.report().outcome(), BuildOutcome::ArtifactCreated);
    let projection = completion
        .artifact()
        .expect("artifact")
        .runtime_projection()
        .expect("loop projection");
    let mut controller = VirtualController::new(UniverseId(0x5d), VirtualControllerId(0x5d), 0x5d);
    controller.power_on().expect("power on");
    controller
        .install_verified_artifact(projection.package())
        .expect("install");
    controller.request_run(RestartKind::Resume).expect("run");
    let event = match controller.run_scan().expect("scan") {
        RunOutcome::Faulted(event) => event,
        RunOutcome::Completed(_) => panic!("infinite loop must hit the deterministic watchdog"),
    };
    assert_eq!(event.code, DiagnosticCode::WorkUnitBudgetExceeded);
    assert_eq!(
        event
            .fault_context
            .expect("work-budget fault context")
            .work_units_before_operation,
        plc_runtime::MAX_WORK_UNITS_PER_SCAN
    );
}

#[test]
fn compiler_ir_budget_charges_control_flow_terminators() {
    let limits = ResourceLimits {
        max_ir_operations: 0,
        ..ResourceLimits::default()
    };
    let error = lower_scl_frontend_artifact(&program(), &SclSource::new(MAIN, ""), limits)
        .expect_err("even an empty block has a return terminator");
    let SclFrontendError::ResourceLimit(limit) = error else {
        panic!("terminator budget must fail with the IR operation ceiling");
    };
    assert_eq!(limit.key, "compiler.ir_operations");
    assert_eq!(limit.current, 1);
    assert_eq!(limit.maximum, 0);
}
