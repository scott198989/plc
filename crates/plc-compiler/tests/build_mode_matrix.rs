use std::collections::BTreeMap;

use plc_compiler::{
    ArtifactFreshness, BuildAttempt, BuildAttemptId, BuildCache, BuildMode, BuildOutcome,
    BuildPublicationState, BuildScope, BuildSnapshot, CacheLookup, CancellationToken, Compiler,
    CompilerProfile, DiagnosticCode, PublicationDecision, ResourceLimits, SclSource,
};
use plc_program::{
    BlockId, BlockInterface, ControllerId, ControllerProgram, DataType, EngineeringNumber,
    InterfaceMember, InterfaceMemberId, InterfaceRole, ObDeclaration, ProgramBlock,
    ProgramUnitKind,
};

const MAIN: BlockId = BlockId::new(1);
const RESULT: InterfaceMemberId = InterfaceMemberId::new(2);

fn fixture(source: &str) -> BuildSnapshot {
    let main = ProgramBlock::new(
        MAIN,
        "Main",
        EngineeringNumber::new(1).unwrap(),
        ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain),
        BlockInterface::from_members([InterfaceMember::plain(
            RESULT,
            "Result",
            InterfaceRole::Temp,
            DataType::DInt,
            0,
        )]),
    );
    let mut program = ControllerProgram::new(ControllerId::new(3));
    program.insert_block(main).unwrap();
    BuildSnapshot::capture(
        &program,
        &BTreeMap::from([(MAIN, SclSource::new(MAIN, source))]),
        CompilerProfile::edu21_core(),
    )
    .unwrap()
}

fn attempt(id: u128, snapshot: BuildSnapshot) -> BuildAttempt {
    BuildAttempt::new(
        BuildAttemptId::new(id),
        snapshot,
        BuildScope::RebuildAllSoftware,
    )
}

#[test]
fn four_modes_and_disposable_cache_preserve_complete_artifact_identity() {
    let snapshot = fixture("Result := 40 + 2;");
    let compiler = Compiler::new(ResourceLimits::default()).unwrap();
    let mut cache = BuildCache::default();
    let mut completions = Vec::new();
    for (index, mode) in [
        BuildMode::ColdCache,
        BuildMode::WarmCache,
        BuildMode::Incremental,
        BuildMode::RebuildAll,
    ]
    .into_iter()
    .enumerate()
    {
        let build = attempt(index as u128 + 10, snapshot.clone());
        let completion =
            compiler.compile_in_mode(&build, snapshot.snapshot_hash(), None, mode, &mut cache);
        assert_eq!(completion.report().build_mode(), mode);
        assert_eq!(completion.report().outcome(), BuildOutcome::ArtifactCreated);
        assert!(completion.artifact().is_some());
        completions.push(completion);
    }
    assert_eq!(
        completions[0].report().cache_lookup(),
        CacheLookup::BypassedCold
    );
    assert!(completions[0].report().cache_published());
    assert_eq!(completions[1].report().cache_lookup(), CacheLookup::Hit);
    assert_eq!(completions[2].report().cache_lookup(), CacheLookup::Hit);
    assert_eq!(
        completions[3].report().cache_lookup(),
        CacheLookup::BypassedRebuild
    );
    assert!(completions[3].report().cache_published());
    let reference = completions[0].artifact().unwrap();
    for completion in &completions[1..] {
        let artifact = completion.artifact().unwrap();
        assert_eq!(artifact, reference);
        assert_eq!(
            artifact.package_fingerprint(),
            reference.package_fingerprint()
        );
        assert_eq!(
            artifact.semantic_fingerprint(),
            reference.semantic_fingerprint()
        );
        assert_eq!(artifact.verified_ir(), reference.verified_ir());
    }
    assert_eq!(cache.len(), 1);

    cache.clear();
    assert!(cache.is_empty());
    let rebuilt = compiler.compile_in_mode(
        &attempt(99, snapshot.clone()),
        snapshot.snapshot_hash(),
        None,
        BuildMode::WarmCache,
        &mut cache,
    );
    assert_eq!(rebuilt.report().cache_lookup(), CacheLookup::Miss);
    assert!(rebuilt.report().cache_published());
    assert_eq!(rebuilt.artifact(), Some(reference));
}

#[test]
fn nonsemantic_source_change_misses_cache_but_preserves_semantic_ir() {
    let plain = fixture("Result := 40 + 2;");
    let commented = fixture("// presentation only\nResult := 40 + 2;");
    assert_ne!(plain.snapshot_hash(), commented.snapshot_hash());
    let compiler = Compiler::new(ResourceLimits::default()).unwrap();
    let mut cache = BuildCache::default();
    let first = compiler.compile_in_mode(
        &attempt(1, plain.clone()),
        plain.snapshot_hash(),
        None,
        BuildMode::ColdCache,
        &mut cache,
    );
    let second = compiler.compile_in_mode(
        &attempt(2, commented.clone()),
        commented.snapshot_hash(),
        None,
        BuildMode::Incremental,
        &mut cache,
    );
    assert_eq!(second.report().cache_lookup(), CacheLookup::Miss);
    let first = first.artifact().unwrap();
    let second = second.artifact().unwrap();
    assert_eq!(first.semantic_fingerprint(), second.semantic_fingerprint());
    assert_eq!(
        first.verified_ir().program(),
        second.verified_ir().program()
    );
    assert_ne!(
        first.integrity().source_map_hash,
        second.integrity().source_map_hash
    );
    assert_ne!(first.package_fingerprint(), second.package_fingerprint());
}

#[test]
fn failed_cancelled_and_stale_attempts_replace_reports_but_retain_artifact() {
    let valid = fixture("Result := 42;");
    let invalid = fixture("Result := ;");
    let compiler = Compiler::new(ResourceLimits::default()).unwrap();
    let mut cache = BuildCache::default();
    let mut publication = BuildPublicationState::new(valid.snapshot_hash());

    let successful = compiler.compile_in_mode(
        &attempt(1, valid.clone()),
        valid.snapshot_hash(),
        None,
        BuildMode::ColdCache,
        &mut cache,
    );
    let successful_fingerprint = successful.artifact().unwrap().package_fingerprint();
    assert_eq!(
        publication.apply(successful),
        PublicationDecision::Published
    );
    assert_eq!(publication.artifact_freshness(), ArtifactFreshness::Current);
    assert!(publication.current_diagnostics().is_empty());

    publication.set_current_snapshot_hash(invalid.snapshot_hash());
    assert_eq!(publication.artifact_freshness(), ArtifactFreshness::Stale);
    let failed = compiler.compile_in_mode(
        &attempt(2, invalid.clone()),
        invalid.snapshot_hash(),
        None,
        BuildMode::Incremental,
        &mut cache,
    );
    assert_eq!(failed.report().outcome(), BuildOutcome::BlockingFailure);
    assert!(failed.artifact().is_none());
    assert_eq!(
        publication.apply(failed),
        PublicationDecision::RetainedPrevious
    );
    assert!(!publication.current_diagnostics().is_empty());
    assert_eq!(
        publication
            .last_successful_artifact()
            .unwrap()
            .package_fingerprint(),
        successful_fingerprint
    );

    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let cancelled = compiler.compile_in_mode(
        &attempt(3, invalid.clone()),
        invalid.snapshot_hash(),
        Some(&cancellation),
        BuildMode::WarmCache,
        &mut cache,
    );
    assert_eq!(cancelled.report().outcome(), BuildOutcome::Cancelled);
    assert!(cancelled.artifact().is_none());
    assert_eq!(
        publication.apply(cancelled),
        PublicationDecision::RetainedPrevious
    );
    assert!(
        publication
            .current_diagnostics()
            .iter()
            .any(|diagnostic| { diagnostic.code() == DiagnosticCode::BUILD_RESOURCE_OR_CANCEL })
    );
    assert!(
        !publication
            .current_diagnostics()
            .iter()
            .any(|diagnostic| { diagnostic.code() == DiagnosticCode::INVALID_CONTROL_FLOW })
    );

    let stale = compiler.compile_in_mode(
        &attempt(4, valid),
        invalid.snapshot_hash(),
        None,
        BuildMode::WarmCache,
        &mut cache,
    );
    assert!(stale.report().is_stale());
    assert!(stale.artifact().is_some());
    assert_eq!(publication.apply(stale), PublicationDecision::RejectedStale);
    assert_eq!(
        publication
            .last_successful_artifact()
            .unwrap()
            .package_fingerprint(),
        successful_fingerprint
    );
}

#[test]
fn cache_publication_is_atomic_across_failure_and_cancel_on_a_hit() {
    let valid = fixture("Result := 42;");
    let invalid = fixture("Result := ;");
    let compiler = Compiler::new(ResourceLimits::default()).unwrap();
    let mut cache = BuildCache::default();
    let seeded = compiler.compile_in_mode(
        &attempt(1, valid.clone()),
        valid.snapshot_hash(),
        None,
        BuildMode::ColdCache,
        &mut cache,
    );
    assert!(seeded.artifact().is_some());
    assert_eq!(cache.len(), 1);

    let failed = compiler.compile_in_mode(
        &attempt(2, invalid.clone()),
        invalid.snapshot_hash(),
        None,
        BuildMode::WarmCache,
        &mut cache,
    );
    assert!(failed.artifact().is_none());
    assert!(!failed.report().cache_published());
    assert_eq!(cache.len(), 1);

    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let cancelled = compiler.compile_in_mode(
        &attempt(3, valid.clone()),
        valid.snapshot_hash(),
        Some(&cancellation),
        BuildMode::WarmCache,
        &mut cache,
    );
    assert_eq!(cancelled.report().cache_lookup(), CacheLookup::Hit);
    assert_eq!(cancelled.report().outcome(), BuildOutcome::Cancelled);
    assert!(cancelled.artifact().is_none());
    assert!(!cancelled.report().cache_published());
    assert_eq!(cache.len(), 1);
}
