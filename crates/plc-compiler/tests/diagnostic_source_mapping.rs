use std::collections::BTreeMap;

use plc_compiler::{
    BuildAttempt, BuildAttemptId, BuildScope, BuildSnapshot, Compiler, CompilerProfile,
    DiagnosticCode, DiagnosticTarget, ResourceLimits, SclSource, SourceAnchor, TextRange,
};
use plc_program::{
    BlockId, BlockInterface, ControllerId, ControllerProgram, DataType, EngineeringNumber,
    InterfaceMember, InterfaceMemberId, InterfaceRole, ObDeclaration, ProgramBlock,
    ProgramUnitKind,
};

const MAIN: BlockId = BlockId::new(1);

fn program() -> ControllerProgram {
    let mut program = ControllerProgram::new(ControllerId::new(1));
    program
        .insert_block(ProgramBlock::new(
            MAIN,
            "Main",
            EngineeringNumber::new(1).expect("nonzero engineering number"),
            ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain),
            BlockInterface::from_members([InterfaceMember::plain(
                InterfaceMemberId::new(10),
                "Known",
                InterfaceRole::Temp,
                DataType::Bool,
                0,
            )]),
        ))
        .expect("unique cyclic main");
    program
}

fn compile_invalid(source: SclSource, attempt: u128) -> plc_compiler::BuildCompletion {
    let snapshot = BuildSnapshot::capture(
        &program(),
        &BTreeMap::from([(MAIN, source)]),
        CompilerProfile::edu21_core(),
    )
    .expect("valid immutable snapshot");
    Compiler::new(ResourceLimits::default())
        .expect("compiler registry")
        .compile(
            &BuildAttempt::new(
                BuildAttemptId::new(attempt),
                snapshot.clone(),
                BuildScope::RebuildAllSoftware,
            ),
            snapshot.snapshot_hash(),
            None,
        )
}

fn source_anchor(completion: &plc_compiler::BuildCompletion, code: DiagnosticCode) -> SourceAnchor {
    let diagnostic = completion
        .report()
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code() == code)
        .expect("expected diagnostic");
    let DiagnosticTarget::Source(anchor) = diagnostic.primary() else {
        panic!("diagnostic must retain a source anchor");
    };
    anchor.clone()
}

#[test]
fn diagnostic_anchors_use_utf8_bytes_and_stable_semantic_identity_across_formatting() {
    let before_source = SclSource::new(MAIN, "Known := Missing;");
    let formatted_source = SclSource::new(MAIN, "// µ\r\nKnown\t:= (* formatting *) Missing ;\n");
    let before = compile_invalid(before_source.clone(), 1);
    let formatted = compile_invalid(formatted_source.clone(), 2);
    let before_anchor = source_anchor(&before, DiagnosticCode::UNRESOLVED_REFERENCE);
    let formatted_anchor = source_anchor(&formatted, DiagnosticCode::UNRESOLVED_REFERENCE);

    assert_eq!(
        before_anchor
            .text_range
            .and_then(|range| before_source.range_text(range)),
        Some("Missing")
    );
    assert_eq!(
        formatted_anchor
            .text_range
            .and_then(|range| formatted_source.range_text(range)),
        Some("Missing")
    );
    assert_eq!(
        before_anchor.stable_identity(),
        formatted_anchor.stable_identity()
    );
    assert_ne!(
        before_anchor.source_revision_hash,
        formatted_anchor.source_revision_hash
    );
}

#[test]
fn missing_token_diagnostic_retains_zero_width_end_of_source_range() {
    let source = SclSource::new(MAIN, "Known := TRUE");
    let completion = compile_invalid(source.clone(), 3);
    let anchor = source_anchor(&completion, DiagnosticCode::MALFORMED_STRUCTURE);
    let expected = TextRange::empty(u32::try_from(source.text().len()).expect("small source"));
    assert_eq!(anchor.text_range, Some(expected));
    assert_eq!(source.range_text(expected), Some(""));
}
