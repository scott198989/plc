use plc_compiler::{
    ResourceLimits, SclSource,
    scl::{SclAccessKind, SclOccurrenceKind, SclOccurrenceResolution},
};
use plc_language_tools::{
    RenameError, SclLanguageService, SclNavigationRelationship, SclNavigationTarget,
    SclNavigationValidity, SignatureHelp,
};
use plc_program::{
    BlockId, BlockInterface, ControllerId, ControllerProgram, DataType, EngineeringNumber,
    InterfaceMember, InterfaceMemberId, InterfaceRole, ProgramBlock, ProgramUnitKind,
};

fn number(value: u16) -> EngineeringNumber {
    EngineeringNumber::new(value).expect("nonzero engineering number")
}

fn byte_offset(text: &str, needle: &str) -> u32 {
    u32::try_from(text.find(needle).expect("needle exists")).expect("small source")
}

fn colliding_identity_program() -> (ControllerProgram, ProgramBlock, ProgramBlock) {
    let caller = ProgramBlock::new(
        BlockId::new(1),
        "Caller",
        number(1),
        ProgramUnitKind::Function,
        BlockInterface::from_members([
            InterfaceMember::plain(
                InterfaceMemberId::new(40),
                "Local",
                InterfaceRole::Input,
                DataType::Bool,
                0,
            ),
            InterfaceMember::plain(
                InterfaceMemberId::new(41),
                "Result",
                InterfaceRole::Output,
                DataType::Bool,
                0,
            ),
        ]),
    );
    let mut output = InterfaceMember::plain(
        InterfaceMemberId::new(42),
        "Y",
        InterfaceRole::Output,
        DataType::Bool,
        0,
    );
    output.required_output_binding = true;
    let scale = ProgramBlock::new(
        BlockId::new(2),
        "Scale",
        number(2),
        ProgramUnitKind::Function,
        BlockInterface::from_members([
            InterfaceMember::plain(
                InterfaceMemberId::new(40),
                "X",
                InterfaceRole::Input,
                DataType::Bool,
                0,
            ),
            output,
        ]),
    );
    let mut program = ControllerProgram::new(ControllerId::new(1));
    program.insert_block(caller.clone()).expect("unique caller");
    program.insert_block(scale.clone()).expect("unique callee");
    (program, caller, scale)
}

#[test]
fn canonical_owner_member_identity_prevents_cross_block_id_collision() {
    let (program, caller, scale) = colliding_identity_program();
    let text = "Scale(X := Local, Y => Result);";
    let source = SclSource::new(caller.id, text);
    let service = SclLanguageService::analyze_with_program(
        &source,
        &caller,
        &program,
        ResourceLimits::default(),
    );
    assert!(
        service.diagnostics().is_empty(),
        "{:?}",
        service.diagnostics()
    );

    let local_references = service.references(byte_offset(text, "Local"));
    assert_eq!(local_references.len(), 1);
    assert_eq!(
        local_references[0]
            .text_range
            .and_then(|range| source.range_text(range)),
        Some("Local")
    );
    assert_eq!(
        service
            .definition(byte_offset(text, "X"))
            .expect("callee formal definition")
            .owner,
        scale.id
    );
    assert_eq!(
        service.signature_help(byte_offset(text, "Local")),
        SignatureHelp::NotAtCallable,
        "ordinary member references are not call sites"
    );
    assert!(matches!(
        service.signature_help(byte_offset(text, "Scale")),
        SignatureHelp::Call { target, .. } if target == scale.id
    ));

    let rename = service
        .rename(byte_offset(text, "X"), "Gain")
        .expect("external formal rename is identity based");
    assert_eq!(rename.declaration.owner, scale.id);
    assert_eq!(rename.declaration.member, InterfaceMemberId::new(40));
    assert_eq!(rename.source_edits.len(), 1);
    assert_eq!(
        rename.source_edits[0]
            .source
            .text_range
            .and_then(|range| source.range_text(range)),
        Some("X")
    );
    assert_eq!(
        service.rename(byte_offset(text, "X"), "Y"),
        Err(RenameError::NameCollision(InterfaceMemberId::new(42)))
    );
}

#[test]
fn compiler_navigation_projection_preserves_definition_use_call_and_assignment_roles() {
    let (program, caller, scale) = colliding_identity_program();
    let text = "Scale(X := Local, Y => Result);";
    let source = SclSource::new(caller.id, text);
    let service = SclLanguageService::analyze_with_program(
        &source,
        &caller,
        &program,
        ResourceLimits::default(),
    );
    let entries = service.navigation_entries();

    for expected in [
        (
            SclNavigationTarget::Block(scale.id),
            SclNavigationRelationship::Definition,
        ),
        (
            SclNavigationTarget::Member {
                owner: caller.id,
                member: InterfaceMemberId::new(40),
            },
            SclNavigationRelationship::Definition,
        ),
        (
            SclNavigationTarget::Member {
                owner: scale.id,
                member: InterfaceMemberId::new(40),
            },
            SclNavigationRelationship::Definition,
        ),
        (
            SclNavigationTarget::Block(scale.id),
            SclNavigationRelationship::Call,
        ),
        (
            SclNavigationTarget::Member {
                owner: scale.id,
                member: InterfaceMemberId::new(40),
            },
            SclNavigationRelationship::Use,
        ),
        (
            SclNavigationTarget::Member {
                owner: caller.id,
                member: InterfaceMemberId::new(41),
            },
            SclNavigationRelationship::Assignment,
        ),
    ] {
        assert!(entries.iter().any(|entry| {
            entry.target == expected.0
                && entry.relationship == expected.1
                && entry.validity == SclNavigationValidity::Valid
        }));
    }

    let occurrences = service.snapshot().occurrences();
    assert!(occurrences.iter().any(|occurrence| {
        occurrence.kind == SclOccurrenceKind::CallTarget
            && occurrence.definition_owner == Some(scale.id)
    }));
    assert!(occurrences.iter().any(|occurrence| {
        occurrence.kind == SclOccurrenceKind::CallFormal
            && occurrence.member == Some(InterfaceMemberId::new(40))
    }));
    assert!(occurrences.iter().any(|occurrence| {
        occurrence.kind == SclOccurrenceKind::MemberReference
            && occurrence.access == SclAccessKind::Write
            && occurrence.member == Some(InterfaceMemberId::new(41))
    }));
}

#[test]
fn invalid_editable_source_keeps_definitions_and_explicit_invalid_relationships() {
    let ambiguous = ProgramBlock::new(
        BlockId::new(5),
        "Ambiguous",
        number(5),
        ProgramUnitKind::Function,
        BlockInterface::from_members([
            InterfaceMember::plain(
                InterfaceMemberId::new(50),
                "Signal",
                InterfaceRole::Input,
                DataType::Bool,
                0,
            ),
            InterfaceMember::plain(
                InterfaceMemberId::new(51),
                "SIGNAL",
                InterfaceRole::Input,
                DataType::Bool,
                1,
            ),
            InterfaceMember::plain(
                InterfaceMemberId::new(52),
                "Output",
                InterfaceRole::Output,
                DataType::Bool,
                0,
            ),
        ]),
    );
    let mut program = ControllerProgram::new(ControllerId::new(5));
    program
        .insert_block(ambiguous.clone())
        .expect("unique block");
    let source = SclSource::new(ambiguous.id, "Output := signal; MissingCall();");
    let service = SclLanguageService::analyze_with_program(
        &source,
        &ambiguous,
        &program,
        ResourceLimits::default(),
    );
    assert!(!service.diagnostics().is_empty());
    let entries = service.navigation_entries();
    assert_eq!(
        entries
            .iter()
            .filter(|entry| entry.relationship == SclNavigationRelationship::Definition)
            .count(),
        3
    );
    assert!(entries.iter().any(|entry| {
        entry.relationship == SclNavigationRelationship::Use
            && entry.validity == SclNavigationValidity::Ambiguous
    }));
    assert!(entries.iter().any(|entry| {
        entry.relationship == SclNavigationRelationship::Call
            && entry.validity == SclNavigationValidity::Unresolved
    }));
    assert!(
        service
            .snapshot()
            .occurrences()
            .iter()
            .any(|occurrence| { occurrence.resolution == SclOccurrenceResolution::Ambiguous })
    );
    assert!(service.snapshot().occurrences().iter().any(|occurrence| {
        occurrence.resolution == SclOccurrenceResolution::Unresolved
            && occurrence.kind == SclOccurrenceKind::CallTarget
    }));
}

#[test]
fn loaded_and_offline_snapshots_do_not_retarget_same_named_replacements() {
    let loaded_block = ProgramBlock::new(
        BlockId::new(7),
        "Loaded",
        number(7),
        ProgramUnitKind::Function,
        BlockInterface::from_members([
            InterfaceMember::plain(
                InterfaceMemberId::new(70),
                "Signal",
                InterfaceRole::Input,
                DataType::Bool,
                0,
            ),
            InterfaceMember::plain(
                InterfaceMemberId::new(72),
                "Output",
                InterfaceRole::Output,
                DataType::Bool,
                0,
            ),
        ]),
    );
    let offline_block = ProgramBlock::new(
        loaded_block.id,
        "Loaded",
        number(7),
        ProgramUnitKind::Function,
        BlockInterface::from_members([
            InterfaceMember::plain(
                InterfaceMemberId::new(71),
                "Signal",
                InterfaceRole::Input,
                DataType::Bool,
                0,
            ),
            InterfaceMember::plain(
                InterfaceMemberId::new(72),
                "Output",
                InterfaceRole::Output,
                DataType::Bool,
                0,
            ),
        ]),
    );
    let loaded_source = SclSource::new(loaded_block.id, "Output := Signal;");
    let offline_source = SclSource::new(offline_block.id, "// offline edit\nOutput := Signal;");
    let loaded =
        SclLanguageService::analyze(&loaded_source, &loaded_block, ResourceLimits::default());
    let offline =
        SclLanguageService::analyze(&offline_source, &offline_block, ResourceLimits::default());
    let loaded_use = loaded
        .navigation_entries()
        .into_iter()
        .find(|entry| entry.relationship == SclNavigationRelationship::Use)
        .expect("loaded use");
    let offline_use = offline
        .navigation_entries()
        .into_iter()
        .find(|entry| entry.relationship == SclNavigationRelationship::Use)
        .expect("offline use");
    assert_eq!(
        loaded_use.target,
        SclNavigationTarget::Member {
            owner: loaded_block.id,
            member: InterfaceMemberId::new(70),
        }
    );
    assert_eq!(
        offline_use.target,
        SclNavigationTarget::Member {
            owner: offline_block.id,
            member: InterfaceMemberId::new(71),
        }
    );
    assert_ne!(
        loaded_use
            .source
            .expect("loaded source")
            .source_revision_hash,
        offline_use
            .source
            .expect("offline source")
            .source_revision_hash
    );
}
