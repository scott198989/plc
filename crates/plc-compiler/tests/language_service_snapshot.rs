use plc_compiler::{
    DiagnosticCode, ResourceLimits, SclSource, SourceLanguage,
    scl::{SclAccessKind, SclOccurrenceResolution, analyze_scl},
};
use plc_program::{
    BlockId, BlockInterface, DataType, EngineeringNumber, InterfaceMember, InterfaceMemberId,
    InterfaceRole, ObDeclaration, ProgramBlock, ProgramUnitKind,
};

const MAIN: BlockId = BlockId::new(0x201);
const ENABLE: InterfaceMemberId = InterfaceMemberId::new(0x2_001);
const COUNTER: InterfaceMemberId = InterfaceMemberId::new(0x2_002);

fn block() -> ProgramBlock {
    ProgramBlock::new(
        MAIN,
        "Main",
        EngineeringNumber::new(1).unwrap(),
        ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain),
        BlockInterface::from_members([
            InterfaceMember::plain(ENABLE, "Enable", InterfaceRole::Temp, DataType::Bool, 0),
            InterfaceMember::plain(COUNTER, "Counter", InterfaceRole::Temp, DataType::DInt, 1),
        ]),
    )
}

#[test]
fn public_analysis_reuses_real_binding_typing_and_source_identity() {
    let source = SclSource::new(
        MAIN,
        "enable := TRUE; Counter := DINT#1; IF Enable THEN counter := COUNTER + DINT#1; END_IF; Missing := Counter;",
    );
    let first = analyze_scl(&source, &block(), ResourceLimits::default());
    let second = analyze_scl(&source, &block(), ResourceLimits::default());
    assert_eq!(first, second, "semantic snapshot must be deterministic");
    assert_eq!(first.source(), &source);
    assert_eq!(first.symbols().len(), 2);
    assert_eq!(first.symbols()[0].member, ENABLE);
    assert_eq!(first.symbols()[0].data_type, DataType::Bool);
    assert_eq!(first.symbols()[1].member, COUNTER);
    assert_eq!(first.symbols()[1].data_type, DataType::DInt);

    assert_eq!(first.occurrences().len(), 7);
    for occurrence in first.occurrences() {
        assert_eq!(occurrence.source.owner_object_id, MAIN);
        assert_eq!(
            occurrence.source.source_revision_hash,
            source.revision_hash()
        );
        assert_eq!(occurrence.source.language, SourceLanguage::Scl);
        assert_eq!(
            occurrence
                .source
                .text_range
                .and_then(|range| source.range_text(range)),
            Some(occurrence.spelling.as_str())
        );
    }
    let unresolved = first
        .occurrences()
        .iter()
        .find(|occurrence| occurrence.spelling == "Missing")
        .expect("unresolved occurrence is retained");
    assert_eq!(unresolved.access, SclAccessKind::Write);
    assert_eq!(unresolved.resolution, SclOccurrenceResolution::Unresolved);
    assert_eq!(unresolved.member, None);
    let counter_reads = first
        .occurrences()
        .iter()
        .filter(|occurrence| {
            occurrence.member == Some(COUNTER) && occurrence.access == SclAccessKind::Read
        })
        .count();
    assert_eq!(counter_reads, 2);
    assert!(first.occurrences().iter().all(|occurrence| {
        occurrence.resolution != SclOccurrenceResolution::Resolved
            || occurrence.data_type.is_some() && occurrence.role.is_some()
    }));

    assert!(
        first
            .type_facts()
            .iter()
            .any(|fact| fact.data_type == DataType::Bool)
    );
    assert!(
        first
            .type_facts()
            .iter()
            .any(|fact| fact.data_type == DataType::DInt)
    );
    assert!(first.type_facts().iter().all(|fact| {
        fact.source.owner_object_id == MAIN
            && fact.source.source_revision_hash == source.revision_hash()
    }));
    assert!(first.diagnostics().iter().any(|issue| {
        issue.code == DiagnosticCode::UNRESOLVED_REFERENCE
            && source.range_text(issue.range) == Some("Missing")
    }));
    assert!(first.missing_tokens().is_empty());
    assert_eq!(first.folding_ranges().len(), 1);
    assert!(
        source
            .range_text(first.folding_ranges()[0])
            .is_some_and(|text| text.starts_with("IF Enable") && text.ends_with("END_IF;"))
    );
    assert!(first.resource_limit().is_none());
}

#[test]
fn public_analysis_preserves_recovery_diagnostics_without_inventing_bindings() {
    let source = SclSource::new(MAIN, "Enable := TRUE Counter := DINT#1;");
    let snapshot = analyze_scl(&source, &block(), ResourceLimits::default());
    assert!(!snapshot.missing_tokens().is_empty());
    assert!(
        snapshot
            .diagnostics()
            .iter()
            .any(|issue| issue.code == DiagnosticCode::MALFORMED_STRUCTURE)
    );
    assert!(snapshot.occurrences().iter().all(|occurrence| {
        occurrence.resolution == SclOccurrenceResolution::Resolved
            && matches!(occurrence.member, Some(ENABLE | COUNTER))
    }));
}
