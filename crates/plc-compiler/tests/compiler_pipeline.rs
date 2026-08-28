use std::collections::{BTreeMap, BTreeSet};

use plc_compiler::{
    BuildAttempt, BuildAttemptId, BuildOutcome, BuildScope, BuildSnapshot, CancellationToken,
    Compiler, CompilerProfile, CompilerStage, DiagnosticCode, DiagnosticSeverity, Hash32,
    IrBasicBlockId, IrTerminatorKind, ResourceLimits, SclSource, TYPED_IR_VERSION, TypedIrProgram,
    VerificationError, phase2_diagnostic_registry, verify_typed_ir,
};
use plc_program::{
    BindingActual, BlockId, BlockInterface, CALL_FC, CallSite, CallSiteId, ControllerId,
    ControllerProgram, DataType, EngineeringNumber, InterfaceMember, InterfaceMemberId,
    InterfaceRole, ObDeclaration, ParameterBinding, ProgramBlock, ProgramUnitKind, VariableRef,
    validate_program,
};
use plc_runtime::{
    PRIORITY_TABLE_VERSION, RUNTIME_SEMANTICS_VERSION, SCHEDULER_VERSION, WORK_COST_VERSION,
};

const MAIN: BlockId = BlockId::new(1);
const HELPER: BlockId = BlockId::new(2);
const MAIN_ENABLED: InterfaceMemberId = InterfaceMemberId::new(1_001);
const MAIN_RESULT: InterfaceMemberId = InterfaceMemberId::new(1_002);
const MAIN_ARGUMENT: InterfaceMemberId = InterfaceMemberId::new(1_003);
const HELPER_INPUT: InterfaceMemberId = InterfaceMemberId::new(2_001);
const HELPER_TEMP: InterfaceMemberId = InterfaceMemberId::new(2_002);
const HELPER_RETURN: InterfaceMemberId = InterfaceMemberId::new(2_003);

fn number(value: u16) -> EngineeringNumber {
    EngineeringNumber::new(value).expect("test engineering number is nonzero")
}

fn member(
    id: InterfaceMemberId,
    name: &str,
    role: InterfaceRole,
    data_type: DataType,
    order: u32,
) -> InterfaceMember {
    InterfaceMember::plain(id, name, role, data_type, order)
}

fn base_program() -> ControllerProgram {
    let main = ProgramBlock::new(
        MAIN,
        "Main",
        number(1),
        ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain),
        BlockInterface::from_members([
            member(
                MAIN_ENABLED,
                "Enabled",
                InterfaceRole::Temp,
                DataType::Bool,
                0,
            ),
            member(
                MAIN_RESULT,
                "Result",
                InterfaceRole::Temp,
                DataType::DInt,
                1,
            ),
        ]),
    );
    let helper = ProgramBlock::new(
        HELPER,
        "Scale",
        number(1),
        ProgramUnitKind::Function,
        BlockInterface::from_members([
            member(
                HELPER_INPUT,
                "InputValue",
                InterfaceRole::Input,
                DataType::DInt,
                0,
            ),
            member(
                HELPER_TEMP,
                "Scratch",
                InterfaceRole::Temp,
                DataType::DInt,
                0,
            ),
            member(
                HELPER_RETURN,
                "Result",
                InterfaceRole::Return,
                DataType::DInt,
                0,
            ),
        ]),
    );
    let mut program = ControllerProgram::new(ControllerId::new(7));
    program.insert_block(main).expect("unique main block");
    program.insert_block(helper).expect("unique helper block");
    assert!(validate_program(&program).is_valid());
    program
}

fn base_sources(prefix_comment: &str) -> BTreeMap<BlockId, SclSource> {
    BTreeMap::from([
        (
            MAIN,
            SclSource::new(
                MAIN,
                format!(
                    "{prefix_comment}Enabled := TRUE; IF Enabled THEN Result := 2 + 3 * 4; ELSE Result := 0; END_IF;"
                ),
            ),
        ),
        (
            HELPER,
            SclSource::new(HELPER, "Result := InputValue + 1; RETURN;"),
        ),
    ])
}

fn snapshot(program: &ControllerProgram, sources: &BTreeMap<BlockId, SclSource>) -> BuildSnapshot {
    BuildSnapshot::capture(program, sources, CompilerProfile::edu21_core())
        .expect("fixture snapshot is valid")
}

fn compile(
    compiler: &Compiler,
    snapshot: BuildSnapshot,
    scope: BuildScope,
    attempt_id: u128,
    current: Option<Hash32>,
    cancellation: Option<&CancellationToken>,
) -> plc_compiler::BuildCompletion {
    let current = current.unwrap_or_else(|| snapshot.snapshot_hash());
    let attempt = BuildAttempt::new(BuildAttemptId::new(attempt_id), snapshot, scope);
    compiler.compile(&attempt, current, cancellation)
}

#[test]
fn genuine_scl_slice_builds_verified_ir_maps_probes_and_runtime_manifest() {
    let program = base_program();
    let sources = base_sources("");
    let snapshot = snapshot(&program, &sources);
    let compiler = Compiler::new(ResourceLimits::default()).expect("registry initializes");
    let completion = compile(
        &compiler,
        snapshot.clone(),
        BuildScope::RebuildAllSoftware,
        10,
        None,
        None,
    );

    assert_eq!(completion.report().outcome(), BuildOutcome::ArtifactCreated);
    assert!(completion.report().diagnostics().is_empty());
    assert_eq!(completion.report().attempt_id(), BuildAttemptId::new(10));
    assert_eq!(
        completion.report().snapshot_hash(),
        snapshot.snapshot_hash()
    );
    let artifact = completion
        .artifact()
        .expect("successful rebuild has artifact");
    assert_eq!(artifact.verified_ir().program().functions().len(), 2);
    assert!(!artifact.source_maps().entries().is_empty());
    assert_eq!(
        artifact.source_maps().entries().len(),
        artifact.probe_table().entries().len()
    );
    assert_eq!(
        artifact.manifest().runtime_version,
        RUNTIME_SEMANTICS_VERSION
    );
    assert_eq!(artifact.manifest().scheduler_version, SCHEDULER_VERSION);
    assert_eq!(
        artifact.manifest().priority_table_version,
        PRIORITY_TABLE_VERSION
    );
    assert_eq!(artifact.manifest().work_cost_version, WORK_COST_VERSION);
    assert_eq!(
        artifact.package_fingerprint(),
        completion.report().artifact_fingerprint().unwrap()
    );
    assert_eq!(
        artifact.semantic_fingerprint(),
        completion.report().semantic_fingerprint().unwrap()
    );
    assert_eq!(
        completion
            .report()
            .stage_metrics()
            .iter()
            .map(|metric| metric.stage)
            .collect::<Vec<_>>(),
        vec![
            CompilerStage::ProjectSchemaValidation,
            CompilerStage::ProfileAndCatalogResolution,
            CompilerStage::HardwareAndAddressValidation,
            CompilerStage::VirtualNetworkValidation,
            CompilerStage::SymbolAndReferenceResolution,
            CompilerStage::DependencyAndSignatureAnalysis,
            CompilerStage::LanguageAndControlFlowAnalysis,
            CompilerStage::TypeAndConversionChecking,
            CompilerStage::CallInstanceAndScheduleValidation,
            CompilerStage::CapabilityAndResourceValidation,
            CompilerStage::TypedIrLowering,
            CompilerStage::IndependentIrVerification,
            CompilerStage::SourceMapAndProbeConstruction,
            CompilerStage::ReportAndArtifactPublication,
        ]
    );

    let main = artifact
        .verified_ir()
        .program()
        .functions()
        .get(&MAIN)
        .unwrap();
    let runtime_operations: Vec<_> = main
        .blocks
        .values()
        .flat_map(|block| block.operations.iter())
        .map(|operation| operation.kind.runtime_operation().0)
        .collect();
    let multiply = runtime_operations
        .iter()
        .position(|operation| *operation == "EDU.RT.MULTIPLY.v1")
        .unwrap();
    let add = runtime_operations
        .iter()
        .position(|operation| *operation == "EDU.RT.ADD_CHECKED.v1")
        .unwrap();
    assert!(
        multiply < add,
        "precedence must lower multiplication before addition"
    );
}

#[test]
fn typed_literals_case_insensitive_names_and_time_components_lower() {
    let mut program = ControllerProgram::new(ControllerId::new(9));
    let main = ProgramBlock::new(
        MAIN,
        "Main",
        number(1),
        ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain),
        BlockInterface::from_members([
            member(
                InterfaceMemberId::new(11),
                "I",
                InterfaceRole::Temp,
                DataType::Int,
                0,
            ),
            member(
                InterfaceMemberId::new(12),
                "D",
                InterfaceRole::Temp,
                DataType::DInt,
                1,
            ),
            member(
                InterfaceMemberId::new(13),
                "R",
                InterfaceRole::Temp,
                DataType::Real,
                2,
            ),
            member(
                InterfaceMemberId::new(14),
                "T",
                InterfaceRole::Temp,
                DataType::Time,
                3,
            ),
            member(
                InterfaceMemberId::new(15),
                "S",
                InterfaceRole::Temp,
                DataType::String { capacity: 16 },
                4,
            ),
        ]),
    );
    program.insert_block(main).unwrap();
    let sources = BTreeMap::from([(
        MAIN,
        SclSource::new(
            MAIN,
            "i := INT#12; d := 16#FF; r := REAL#1.25; t := TIME#1s250ms; s := STRING#'it''s';",
        ),
    )]);
    let compiler = Compiler::new(ResourceLimits::default()).unwrap();
    let completion = compile(
        &compiler,
        snapshot(&program, &sources),
        BuildScope::RebuildAllSoftware,
        11,
        None,
        None,
    );
    assert_eq!(completion.report().outcome(), BuildOutcome::ArtifactCreated);
    assert!(completion.report().diagnostics().is_empty());
}

#[test]
fn malformed_and_unsupported_source_never_produce_artifacts() {
    let program = base_program();
    let compiler = Compiler::new(ResourceLimits::default()).unwrap();
    let cases = [
        (
            "Enabled := TRUE Result := 1;",
            DiagnosticCode::MALFORMED_STRUCTURE,
        ),
        (
            "WHILE TRUE DO Result := 1; END_WHILE;",
            DiagnosticCode::RECOGNIZED_UNSUPPORTED_SYNTAX,
        ),
        ("Enabled := 1 < 2 < 3;", DiagnosticCode::MALFORMED_STRUCTURE),
    ];
    for (index, (main_source, expected)) in cases.into_iter().enumerate() {
        let mut sources = base_sources("");
        sources.insert(MAIN, SclSource::new(MAIN, main_source));
        let completion = compile(
            &compiler,
            snapshot(&program, &sources),
            BuildScope::RebuildAllSoftware,
            20 + index as u128,
            None,
            None,
        );
        assert_eq!(completion.report().outcome(), BuildOutcome::BlockingFailure);
        assert!(completion.artifact().is_none());
        assert!(
            completion
                .report()
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code() == expected),
            "missing {expected:?} for {main_source}"
        );
    }
}

#[test]
fn binding_type_and_definite_assignment_fail_closed_with_precise_codes() {
    let program = base_program();
    let compiler = Compiler::new(ResourceLimits::default()).unwrap();
    let cases = [
        ("Missing := TRUE;", DiagnosticCode::UNRESOLVED_REFERENCE),
        (
            "Enabled := TRUE; Result := Enabled;",
            DiagnosticCode::TYPE_MISMATCH,
        ),
        (
            "Enabled := TRUE; IF Enabled THEN Result := 1; END_IF; Enabled := Result > 0;",
            DiagnosticCode::INVALID_CONTROL_FLOW,
        ),
    ];
    for (index, (main_source, expected)) in cases.into_iter().enumerate() {
        let mut sources = base_sources("");
        sources.insert(MAIN, SclSource::new(MAIN, main_source));
        let completion = compile(
            &compiler,
            snapshot(&program, &sources),
            BuildScope::RebuildAllSoftware,
            30 + index as u128,
            None,
            None,
        );
        assert!(completion.artifact().is_none());
        assert!(
            completion
                .report()
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code() == expected && diagnostic.is_blocking())
        );
    }
}

#[test]
fn fc_return_must_be_definitely_assigned_before_return() {
    let program = base_program();
    let mut sources = base_sources("");
    sources.insert(HELPER, SclSource::new(HELPER, "RETURN;"));
    let compiler = Compiler::new(ResourceLimits::default()).unwrap();
    let completion = compile(
        &compiler,
        snapshot(&program, &sources),
        BuildScope::RebuildAllSoftware,
        40,
        None,
        None,
    );
    assert!(completion.artifact().is_none());
    assert!(
        completion
            .report()
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == DiagnosticCode::INVALID_CONTROL_FLOW)
    );
}

#[test]
fn attempt_identity_and_cache_state_do_not_change_artifact_bytes() {
    let program = base_program();
    let sources = base_sources("");
    let snapshot = snapshot(&program, &sources);
    let compiler = Compiler::new(ResourceLimits::default()).unwrap();
    let first = compile(
        &compiler,
        snapshot.clone(),
        BuildScope::RebuildAllSoftware,
        100,
        None,
        None,
    );
    let second = compile(
        &compiler,
        snapshot,
        BuildScope::RebuildAllSoftware,
        999,
        None,
        None,
    );
    assert_ne!(first.report().attempt_id(), second.report().attempt_id());
    assert_eq!(first.artifact(), second.artifact());
    assert_eq!(
        first.artifact().unwrap().package_fingerprint(),
        second.artifact().unwrap().package_fingerprint()
    );
}

#[test]
fn comments_change_snapshot_and_package_but_not_semantic_ir() {
    let program = base_program();
    let plain = snapshot(&program, &base_sources(""));
    let commented = snapshot(&program, &base_sources("// nonsemantic note\n"));
    assert_ne!(plain.snapshot_hash(), commented.snapshot_hash());
    let compiler = Compiler::new(ResourceLimits::default()).unwrap();
    let first = compile(
        &compiler,
        plain,
        BuildScope::RebuildAllSoftware,
        1,
        None,
        None,
    );
    let second = compile(
        &compiler,
        commented,
        BuildScope::RebuildAllSoftware,
        2,
        None,
        None,
    );
    let first = first.artifact().unwrap();
    let second = second.artifact().unwrap();
    assert_eq!(
        first.verified_ir().program(),
        second.verified_ir().program()
    );
    assert_eq!(first.semantic_fingerprint(), second.semantic_fingerprint());
    assert_ne!(
        first.integrity().source_map_hash,
        second.integrity().source_map_hash
    );
    assert_ne!(first.package_fingerprint(), second.package_fingerprint());
}

#[test]
fn stale_completion_is_marked_without_mutating_artifact_identity() {
    let program = base_program();
    let snapshot = snapshot(&program, &base_sources(""));
    let compiler = Compiler::new(ResourceLimits::default()).unwrap();
    let fresh = compile(
        &compiler,
        snapshot.clone(),
        BuildScope::RebuildAllSoftware,
        5,
        None,
        None,
    );
    let stale = compile(
        &compiler,
        snapshot,
        BuildScope::RebuildAllSoftware,
        6,
        Some(Hash32::from_bytes([0x5a; 32])),
        None,
    );
    assert!(stale.report().is_stale());
    assert!(stale.report().diagnostics().iter().any(|diagnostic| {
        diagnostic.code() == DiagnosticCode::STALE_BUILD_RESULT && !diagnostic.is_blocking()
    }));
    assert_eq!(
        fresh.artifact().unwrap().package_fingerprint(),
        stale.artifact().unwrap().package_fingerprint()
    );
}

#[test]
fn cancellation_and_resource_limits_are_side_effect_free() {
    let program = base_program();
    let snapshot = snapshot(&program, &base_sources(""));
    let compiler = Compiler::new(ResourceLimits::default()).unwrap();
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let cancelled = compile(
        &compiler,
        snapshot.clone(),
        BuildScope::RebuildAllSoftware,
        50,
        None,
        Some(&cancellation),
    );
    assert_eq!(cancelled.report().outcome(), BuildOutcome::Cancelled);
    assert!(cancelled.artifact().is_none());

    let limits = ResourceLimits {
        max_tokens_per_block: 4,
        ..ResourceLimits::default()
    };
    let limited_compiler = Compiler::new(limits).unwrap();
    let limited = compile(
        &limited_compiler,
        snapshot,
        BuildScope::RebuildAllSoftware,
        51,
        None,
        None,
    );
    assert_eq!(limited.report().outcome(), BuildOutcome::ResourceLimit);
    assert!(limited.artifact().is_none());
    let codes: BTreeSet<_> = limited
        .report()
        .diagnostics()
        .iter()
        .map(plc_compiler::BuildDiagnostic::code)
        .collect();
    assert!(codes.contains(&DiagnosticCode::RESOURCE_LIMIT));
    assert!(codes.contains(&DiagnosticCode::BUILD_RESOURCE_OR_CANCEL));
}

fn program_with_call_dependency() -> ControllerProgram {
    let mut program = base_program();
    let mut main = program.block(MAIN).unwrap().clone();
    let mut members: Vec<_> = main.interface.members.values().cloned().collect();
    members.push(member(
        MAIN_ARGUMENT,
        "Argument",
        InterfaceRole::Temp,
        DataType::DInt,
        2,
    ));
    main.interface = BlockInterface::from_members(members);
    main.calls.push(CallSite {
        id: CallSiteId::new(1),
        instruction: CALL_FC,
        callee: HELPER,
        bindings: vec![
            ParameterBinding {
                formal: HELPER_INPUT,
                actual: BindingActual::Variable(VariableRef::CallerMember(MAIN_ARGUMENT)),
            },
            ParameterBinding {
                formal: HELPER_RETURN,
                actual: BindingActual::Variable(VariableRef::CallerMember(MAIN_RESULT)),
            },
        ],
        instance_owner: None,
    });
    program.replace_block(main).unwrap();
    assert!(validate_program(&program).is_valid());
    program
}

#[test]
fn dependency_aware_scopes_expand_prerequisites_and_transitive_dependents() {
    let program = program_with_call_dependency();
    let sources = base_sources("");
    let snapshot = snapshot(&program, &sources);
    let compiler = Compiler::new(ResourceLimits::default()).unwrap();
    let current = compiler
        .expand_scope(&snapshot, &BuildScope::CurrentObject(MAIN))
        .unwrap();
    assert_eq!(current.prerequisites(), &[HELPER]);
    assert_eq!(current.ordered_units(), &[MAIN, HELPER]);

    let changes = compiler
        .expand_scope(&snapshot, &BuildScope::SoftwareChanges(vec![HELPER]))
        .unwrap();
    assert_eq!(changes.affected_dependents(), &[MAIN]);
    assert_eq!(changes.ordered_units(), &[MAIN, HELPER]);
}

#[test]
fn narrow_scope_reports_object_valid_and_never_claims_an_artifact() {
    let program = base_program();
    let snapshot = snapshot(&program, &base_sources(""));
    let compiler = Compiler::new(ResourceLimits::default()).unwrap();
    let completion = compile(
        &compiler,
        snapshot,
        BuildScope::CurrentObject(MAIN),
        60,
        None,
        None,
    );
    assert_eq!(completion.report().outcome(), BuildOutcome::ObjectValid);
    assert!(completion.artifact().is_none());
    assert_eq!(
        completion
            .report()
            .expanded_scope()
            .unwrap()
            .ordered_units(),
        &[MAIN]
    );
}

#[test]
fn hardware_and_controller_scopes_are_unavailable_not_faked() {
    let program = base_program();
    let snapshot = snapshot(&program, &base_sources(""));
    let compiler = Compiler::new(ResourceLimits::default()).unwrap();
    for (index, scope) in [BuildScope::VirtualHardware, BuildScope::ControllerBuild]
        .into_iter()
        .enumerate()
    {
        let completion = compile(
            &compiler,
            snapshot.clone(),
            scope,
            70 + index as u128,
            None,
            None,
        );
        assert_eq!(completion.report().outcome(), BuildOutcome::BlockingFailure);
        assert!(completion.artifact().is_none());
        assert!(
            completion
                .report()
                .diagnostics()
                .iter()
                .any(|diagnostic| { diagnostic.code() == DiagnosticCode::CAPABILITY_UNAVAILABLE })
        );
    }
}

#[test]
fn missing_profile_capabilities_block_after_real_analysis() {
    let program = base_program();
    let sources = base_sources("");
    let profile = CompilerProfile::from_parts("Restricted", "1", Vec::new()).unwrap();
    let snapshot = BuildSnapshot::capture(&program, &sources, profile).unwrap();
    let compiler = Compiler::new(ResourceLimits::default()).unwrap();
    let completion = compile(
        &compiler,
        snapshot,
        BuildScope::RebuildAllSoftware,
        75,
        None,
        None,
    );
    assert_eq!(completion.report().outcome(), BuildOutcome::BlockingFailure);
    assert!(completion.artifact().is_none());
    assert_eq!(
        completion
            .report()
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == DiagnosticCode::CAPABILITY_UNAVAILABLE)
            .count(),
        4
    );
}

#[test]
fn independent_verifier_rejects_tampered_control_flow() {
    let program = base_program();
    let compiler = Compiler::new(ResourceLimits::default()).unwrap();
    let completion = compile(
        &compiler,
        snapshot(&program, &base_sources("")),
        BuildScope::RebuildAllSoftware,
        80,
        None,
        None,
    );
    let artifact = completion.artifact().unwrap();
    let mut functions = artifact.verified_ir().program().functions().clone();
    let function = functions.get_mut(&MAIN).unwrap();
    let entry = function.blocks.get_mut(&function.entry).unwrap();
    match &mut entry.terminator.kind {
        IrTerminatorKind::Branch { when_true, .. } => {
            *when_true = IrBasicBlockId::new(u32::MAX);
        }
        other => panic!("expected branch terminator, got {other:?}"),
    }
    let tampered = TypedIrProgram::from_untrusted_parts(TYPED_IR_VERSION, functions);
    assert_eq!(
        verify_typed_ir(
            tampered,
            artifact.source_maps(),
            artifact.probe_table(),
            &program,
        ),
        Err(VerificationError::MissingTarget(
            MAIN,
            IrBasicBlockId::new(u32::MAX)
        ))
    );
}

#[test]
fn diagnostic_registry_is_complete_original_and_separates_blocking_from_severity() {
    let registry = phase2_diagnostic_registry();
    registry.validate().expect("baseline registry validates");
    assert_eq!(registry.definitions().len(), 32);
    let codes: BTreeSet<_> = registry
        .definitions()
        .iter()
        .map(|definition| definition.code)
        .collect();
    assert_eq!(codes.len(), registry.definitions().len());
    let multiple_writer = registry.lookup(DiagnosticCode::MULTIPLE_WRITER).unwrap();
    assert_eq!(
        multiple_writer.default_severity,
        DiagnosticSeverity::Warning
    );
    assert!(!multiple_writer.blocking);
    assert!(
        registry
            .definitions()
            .iter()
            .all(|definition| definition.code.0.starts_with("EDU-"))
    );
}

#[test]
fn source_locations_are_utf8_byte_ranges_with_tested_display_mapping() {
    let source = SclSource::new(MAIN, "// α\nEnabled := TRUE;");
    let enabled_offset = u32::try_from("// α\n".len()).unwrap();
    let location = source.line_column(enabled_offset).unwrap();
    assert_eq!(location.line, 2);
    assert_eq!(location.column, 1);
    let inside_alpha = u32::try_from("// ".len() + 1).unwrap();
    assert!(source.line_column(inside_alpha).is_none());
}
