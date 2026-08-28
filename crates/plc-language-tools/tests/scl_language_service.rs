use plc_compiler::{ResourceLimits, SclSource, TextRange, scl::analyze_scl};
use plc_language_tools::{
    HoverInfo, RenameError, SclLanguageService, SemanticTokenKind, SignatureHelp,
};
use plc_program::{
    BlockId, BlockInterface, DataType, EngineeringNumber, InterfaceMember, InterfaceMemberId,
    InterfaceRole, ProgramBlock, ProgramUnitKind,
};

fn block() -> ProgramBlock {
    ProgramBlock::new(
        BlockId::new(1),
        "FC1",
        EngineeringNumber::new(1).expect("nonzero"),
        ProgramUnitKind::Function,
        BlockInterface::from_members([
            InterfaceMember::plain(
                InterfaceMemberId::new(10),
                "InputA",
                InterfaceRole::Input,
                DataType::Bool,
                0,
            ),
            InterfaceMember::plain(
                InterfaceMemberId::new(11),
                "InputB",
                InterfaceRole::Input,
                DataType::Bool,
                1,
            ),
            InterfaceMember::plain(
                InterfaceMemberId::new(12),
                "OutputQ",
                InterfaceRole::Output,
                DataType::Bool,
                0,
            ),
            InterfaceMember::plain(
                InterfaceMemberId::new(13),
                "Scratch",
                InterfaceRole::Temp,
                DataType::Bool,
                0,
            ),
        ]),
    )
}

fn byte_offset(text: &str, needle: &str, occurrence: usize) -> u32 {
    let offset = text
        .match_indices(needle)
        .nth(occurrence)
        .expect("needle occurrence")
        .0;
    u32::try_from(offset).expect("test source fits u32")
}

#[test]
fn diagnostics_and_semantics_are_the_compiler_snapshot_not_a_second_binder() {
    let source = SclSource::new(
        BlockId::new(1),
        "Scratch := InputA; OutputQ := Scratch AND InputB;",
    );
    let expected = analyze_scl(&source, &block(), ResourceLimits::default());
    let service = SclLanguageService::analyze(&source, &block(), ResourceLimits::default());
    assert_eq!(service.snapshot(), &expected);
    assert_eq!(service.diagnostics(), expected.diagnostics());
    assert!(service.diagnostics().is_empty());
}

#[test]
fn completion_uses_canonical_interface_symbols_and_utf8_byte_range() {
    let text = "// µ\nOut";
    let source = SclSource::new(BlockId::new(1), text);
    let service = SclLanguageService::analyze(&source, &block(), ResourceLimits::default());
    let items = service.completions(u32::try_from(text.len()).expect("small source"));
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].label, "OutputQ");
    assert_eq!(items[0].member, InterfaceMemberId::new(12));
    assert_eq!(items[0].replace_range, TextRange { start: 6, end: 9 });
    assert_eq!(service.display_position(6).expect("boundary").line, 2);
    assert!(
        service.display_position(4).is_none(),
        "inside UTF-8 code point"
    );
}

#[test]
fn hover_definition_and_references_share_stable_member_identity() {
    let text = "Scratch := InputA; OutputQ := InputA AND InputB;";
    let source = SclSource::new(BlockId::new(1), text);
    let service = SclLanguageService::analyze(&source, &block(), ResourceLimits::default());
    let offset = byte_offset(text, "InputA", 0) + 1;
    let hover = service.hover(offset).expect("hover");
    let HoverInfo::Symbol {
        definition,
        source: anchor,
        ..
    } = hover
    else {
        panic!("resolved symbol hover expected");
    };
    assert_eq!(definition.member, InterfaceMemberId::new(10));
    assert_eq!(definition.data_type, DataType::Bool);
    assert_eq!(anchor.source_revision_hash, source.revision_hash());
    assert_ne!(anchor.semantic_node_id.get(), 0);

    assert_eq!(
        service.definition(offset).expect("definition").member,
        InterfaceMemberId::new(10)
    );
    let references = service.references(offset);
    assert_eq!(references.len(), 2);
    assert!(
        references
            .iter()
            .all(|reference| reference.source_revision_hash == source.revision_hash())
    );
}

#[test]
fn semantic_rename_returns_interface_and_content_hash_guarded_source_edits() {
    let text = "Scratch := InputA; OutputQ := InputA AND InputB;";
    let source = SclSource::new(BlockId::new(1), text);
    let service = SclLanguageService::analyze(&source, &block(), ResourceLimits::default());
    let plan = service
        .rename(byte_offset(text, "InputA", 0) + 1, "StartSignal")
        .expect("semantic rename");
    assert_eq!(plan.declaration.member, InterfaceMemberId::new(10));
    assert_eq!(plan.declaration.expected_name, "InputA");
    assert_eq!(plan.declaration.replacement, "StartSignal");
    assert_eq!(plan.source_edits.len(), 2);
    assert!(plan.source_edits.iter().all(|edit| {
        edit.replacement == "StartSignal"
            && edit.source.source_revision_hash == source.revision_hash()
            && edit.source.owner_object_id == BlockId::new(1)
    }));

    assert_eq!(
        service.rename(byte_offset(text, "InputA", 0), "IF"),
        Err(RenameError::InvalidIdentifier)
    );
    assert_eq!(
        service.rename(byte_offset(text, "InputA", 0), "OutputQ"),
        Err(RenameError::NameCollision(InterfaceMemberId::new(12)))
    );
}

#[test]
fn unresolved_and_ambiguous_names_never_guess_during_rename() {
    let unresolved_text = "OutputQ := Missing;";
    let unresolved_source = SclSource::new(BlockId::new(1), unresolved_text);
    let unresolved =
        SclLanguageService::analyze(&unresolved_source, &block(), ResourceLimits::default());
    assert!(matches!(
        unresolved.hover(byte_offset(unresolved_text, "Missing", 0)),
        Some(HoverInfo::Unresolved { .. })
    ));
    assert_eq!(
        unresolved.rename(byte_offset(unresolved_text, "Missing", 0), "Fixed"),
        Err(RenameError::UnresolvedSymbol)
    );

    let ambiguous_block = ProgramBlock::new(
        BlockId::new(2),
        "FC2",
        EngineeringNumber::new(2).expect("nonzero"),
        ProgramUnitKind::Function,
        BlockInterface::from_members([
            InterfaceMember::plain(
                InterfaceMemberId::new(20),
                "Signal",
                InterfaceRole::Input,
                DataType::Bool,
                0,
            ),
            InterfaceMember::plain(
                InterfaceMemberId::new(21),
                "SIGNAL",
                InterfaceRole::Input,
                DataType::Bool,
                1,
            ),
            InterfaceMember::plain(
                InterfaceMemberId::new(22),
                "Q",
                InterfaceRole::Output,
                DataType::Bool,
                0,
            ),
        ]),
    );
    let ambiguous_text = "Q := signal;";
    let ambiguous_source = SclSource::new(BlockId::new(2), ambiguous_text);
    let ambiguous = SclLanguageService::analyze(
        &ambiguous_source,
        &ambiguous_block,
        ResourceLimits::default(),
    );
    assert_eq!(
        ambiguous.rename(byte_offset(ambiguous_text, "signal", 0), "Other"),
        Err(RenameError::AmbiguousSymbol)
    );
}

#[test]
fn semantic_tokens_combine_real_lexer_classes_with_bound_symbols() {
    let text = "IF InputA THEN OutputQ := TRUE; END_IF;";
    let source = SclSource::new(BlockId::new(1), text);
    let service = SclLanguageService::analyze(&source, &block(), ResourceLimits::default());
    let tokens = service.semantic_tokens();
    assert_eq!(tokens[0].kind, SemanticTokenKind::Keyword);
    assert!(tokens.iter().any(|token| {
        matches!(
            token.kind,
            SemanticTokenKind::Symbol {
                member,
                role: InterfaceRole::Input
            } if member == InterfaceMemberId::new(10)
        )
    }));
    assert!(
        tokens
            .iter()
            .any(|token| token.kind == SemanticTokenKind::Literal)
    );
    assert_eq!(service.folding_ranges().len(), 1);
}

#[test]
fn source_identity_changes_with_content_while_semantic_occurrences_remain_navigable() {
    let before_source = SclSource::new(BlockId::new(1), "OutputQ := InputA;");
    let after_source = SclSource::new(BlockId::new(1), "OutputQ := NOT InputA;");
    let before = SclLanguageService::analyze(&before_source, &block(), ResourceLimits::default());
    let after = SclLanguageService::analyze(&after_source, &block(), ResourceLimits::default());
    let before_ref = before
        .references(byte_offset(before_source.text(), "InputA", 0))
        .pop()
        .expect("before ref");
    let after_ref = after
        .references(byte_offset(after_source.text(), "InputA", 0))
        .pop()
        .expect("after ref");
    assert_ne!(
        before_ref.source_revision_hash,
        after_ref.source_revision_hash
    );
    assert_eq!(before_ref.owner_object_id, after_ref.owner_object_id);
}

#[test]
fn signature_help_is_truthful_for_the_initial_no_call_grammar() {
    let source = SclSource::new(BlockId::new(1), "OutputQ := InputA;");
    let service = SclLanguageService::analyze(&source, &block(), ResourceLimits::default());
    assert_eq!(
        service.signature_help(0),
        SignatureHelp::NoCallableSyntaxInInitialProfile
    );
}

#[test]
fn language_service_obeys_compiler_resource_limits_without_partial_semantics() {
    let source = SclSource::new(BlockId::new(1), "OutputQ := InputA;");
    let limits = ResourceLimits {
        max_source_bytes_per_block: 4,
        ..ResourceLimits::default()
    };
    let service = SclLanguageService::analyze(&source, &block(), limits);
    assert!(service.snapshot().resource_limit().is_some());
    assert!(service.snapshot().occurrences().is_empty());
    assert!(service.completions(2).is_empty());
    assert!(service.references(2).is_empty());
}
