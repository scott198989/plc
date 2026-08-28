use plc_commissioning::*;
use plc_runtime::{
    ArtifactSpec, BlockId, ChannelDefinition, Instruction, MemoryDefinition, Operand, Operation,
    ProgramBlock, ProgramImage, RestartKind, Sha256, StateDefinition, VirtualController,
};

const UNIVERSE: UniverseId = UniverseId(0x1000);
const OFFLINE: OfflineControllerId = OfflineControllerId(0x2000);
const INSTANCE: ControllerInstanceId = ControllerInstanceId(0x3000);
const CONTROLLER: VirtualControllerId = VirtualControllerId(0x4000);
const SESSION: VirtualOnlineSessionId = VirtualOnlineSessionId(0x5000);
const CLONE_INSTANCE: ControllerInstanceId = ControllerInstanceId(0x3001);
const CLONE_CONTROLLER: VirtualControllerId = VirtualControllerId(0x4001);
const REPLACEMENT_INSTANCE: ControllerInstanceId = ControllerInstanceId(0x3002);

#[derive(Clone)]
struct Member {
    member_id: u128,
    runtime_id: u32,
    start: i32,
    role: MemoryRole,
    path: Vec<u128>,
    retentive: bool,
}

struct PackageRecipe<'a> {
    code_step: i32,
    members: &'a [Member],
    source_label: &'a str,
    semantic_label: &'a str,
    hardware_label: &'a str,
    profile_label: &'a str,
    build_label: &'a str,
    build_is_current: bool,
    blocking_diagnostic_count: u32,
}

fn hash(label: &str) -> Hash32 {
    Sha256::digest(label.as_bytes())
}

fn package_parts(recipe: PackageRecipe<'_>) -> LoadPackageParts {
    let memory = recipe
        .members
        .iter()
        .map(|member| MemoryDefinition {
            id: MemoryId(member.runtime_id),
            value_type: ValueType::I32,
            loaded_start: CanonicalValue::I32(member.start),
            retentive: member.retentive,
        })
        .collect::<Vec<_>>();
    let operation = if let Some(first) = recipe.members.first() {
        Operation::AddI32 {
            left: Operand::Memory(MemoryId(first.runtime_id)),
            right: Operand::Constant(CanonicalValue::I32(recipe.code_step)),
            target: MemoryId(first.runtime_id),
        }
    } else {
        Operation::Noop
    };
    let runtime_artifact = ArtifactPackage::seal_verified(ArtifactSpec::edu21(
        hash(recipe.profile_label),
        memory,
        Vec::<ChannelDefinition>::new(),
        Vec::<StateDefinition>::new(),
        ProgramImage {
            startup: None,
            timed: vec![],
            cyclic: ProgramBlock {
                id: BlockId(1),
                instructions: vec![Instruction::new(1, 0xabc, operation)],
            },
        },
    ))
    .unwrap();
    LoadPackageParts {
        runtime_artifact,
        semantic_build_fingerprint: hash(recipe.semantic_label),
        verified_ir_fingerprint: hash(&format!("ir-step-{}", recipe.code_step)),
        schedule_fingerprint: hash("schedule-1"),
        hardware_fingerprint: hash(recipe.hardware_label),
        source_map_fingerprint: hash(recipe.source_label),
        probe_identity_fingerprint: hash("probes-1"),
        capability_fingerprint: hash("capabilities-1"),
        build_snapshot_hash: hash(recipe.build_label),
        build_is_current: recipe.build_is_current,
        blocking_diagnostic_count: recipe.blocking_diagnostic_count,
        memory_schema: recipe
            .members
            .iter()
            .map(|member| MemoryMemberSchema {
                member_id: member.member_id,
                runtime_memory_id: MemoryId(member.runtime_id),
                value_type: ValueType::I32,
                role: member.role,
                instance_path: member.path.clone(),
                retentive: member.retentive,
                loaded_start: CanonicalValue::I32(member.start),
            })
            .collect(),
        state_schema: vec![],
    }
}

fn package(
    code_step: i32,
    members: &[Member],
    source_label: &str,
    semantic_label: &str,
    hardware_label: &str,
    build_label: &str,
) -> VirtualLoadPackage {
    VirtualLoadPackage::seal_verified(package_parts(PackageRecipe {
        code_step,
        members,
        source_label,
        semantic_label,
        hardware_label,
        profile_label: "profile-1",
        build_label,
        build_is_current: true,
        blocking_diagnostic_count: 0,
    }))
    .unwrap()
}

fn faulting_package(members: &[Member]) -> VirtualLoadPackage {
    let mut parts = package_parts(PackageRecipe {
        code_step: 1,
        members,
        source_label: "fault-source-map",
        semantic_label: "fault-semantic",
        hardware_label: "hardware-1",
        profile_label: "profile-1",
        build_label: "fault-build",
        build_is_current: true,
        blocking_diagnostic_count: 0,
    });
    let memory = members
        .iter()
        .map(|member| MemoryDefinition {
            id: MemoryId(member.runtime_id),
            value_type: ValueType::I32,
            loaded_start: CanonicalValue::I32(member.start),
            retentive: member.retentive,
        })
        .collect();
    parts.runtime_artifact = ArtifactPackage::seal_verified(ArtifactSpec::edu21(
        hash("profile-1"),
        memory,
        Vec::<ChannelDefinition>::new(),
        Vec::<StateDefinition>::new(),
        ProgramImage {
            startup: None,
            timed: vec![],
            cyclic: ProgramBlock {
                id: BlockId(1),
                instructions: vec![Instruction::new(
                    1,
                    0xf017,
                    Operation::DivideI32 {
                        numerator: Operand::Memory(MemoryId(1)),
                        denominator: Operand::Constant(CanonicalValue::I32(0)),
                        target: MemoryId(2),
                    },
                )],
            },
        },
    ))
    .unwrap();
    parts.verified_ir_fingerprint = hash("fault-ir");
    VirtualLoadPackage::seal_verified(parts).unwrap()
}

fn base_members() -> Vec<Member> {
    vec![
        Member {
            member_id: 100,
            runtime_id: 1,
            start: 5,
            role: MemoryRole::Marker,
            path: vec![10],
            retentive: true,
        },
        Member {
            member_id: 200,
            runtime_id: 2,
            start: 9,
            role: MemoryRole::GlobalDb,
            path: vec![20],
            retentive: false,
        },
    ]
}

fn configured_offline(package: &VirtualLoadPackage) -> OfflineEngineeringState {
    let mut state = OfflineEngineeringState {
        configured: ConfiguredController {
            id: OFFLINE,
            configured_hardware_fingerprint: package.hardware_fingerprint(),
            profile_fingerprint: package.profile_fingerprint(),
        },
        source_revision_hash: hash("source-revision-1"),
        build_snapshot_hash: None,
        project_saved: true,
        source_to_build: OfflineSourceBuild::Absent,
        software_build_current: false,
        hardware_build_current: false,
        current_package_fingerprint: None,
        built_hardware: None,
    };
    state.record_build(package);
    state
}

fn universe_for(package: &VirtualLoadPackage) -> VirtualUniverse {
    let mut universe = VirtualUniverse::new(UNIVERSE);
    universe
        .register_offline_controller(configured_offline(package))
        .unwrap();
    universe
        .create_instance(CreateInstanceCommand {
            command_id: 1,
            instance_id: INSTANCE,
            offline_controller_id: OFFLINE,
            controller_id: CONTROLLER,
            deterministic_seed: 0x55aa,
        })
        .unwrap();
    assert_eq!(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .runtime()
            .cpu_state(),
        CpuState::PoweredOff
    );
    universe.power_on(CONTROLLER).unwrap();
    universe
}

fn load_request(package: &VirtualLoadPackage, mode: PostLoadMode) -> LoadRequest {
    LoadRequest {
        expected_build_snapshot_hash: package.build_snapshot_hash(),
        requested_post_load_mode: mode,
        initialize_compatible_members: false,
        valid_through_event_sequence: u64::MAX,
    }
}

fn commit(
    universe: &mut VirtualUniverse,
    package: &VirtualLoadPackage,
    mode: PostLoadMode,
) -> LoadResult {
    let preview = universe
        .prepare_load(CONTROLLER, package, load_request(package, mode))
        .unwrap();
    assert!(preview.blockers().is_empty(), "{:?}", preview.blockers());
    universe
        .commit_load(
            &preview,
            PreviewApproval::approve(&preview),
            package,
            LoadExecution::default(),
        )
        .unwrap()
}

fn open_session(universe: &mut VirtualUniverse) {
    universe
        .begin_go_online(SESSION, OFFLINE, CONTROLLER)
        .unwrap();
    assert_eq!(
        universe.session(SESSION).unwrap().state(),
        SessionState::Opening
    );
    universe.complete_go_online(SESSION).unwrap();
}

#[test]
fn universe_power_off_uses_authoritative_runtime_and_audits_the_cpu_boundary() {
    let base = package(
        1,
        &base_members(),
        "source-map-power-off",
        "semantic-power-off",
        "hardware-power-off",
        "build-power-off",
    );
    let mut universe = universe_for(&base);
    assert_eq!(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .runtime()
            .cpu_state(),
        CpuState::Stop
    );
    universe.power_off(CONTROLLER).unwrap();
    assert_eq!(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .runtime()
            .cpu_state(),
        CpuState::PoweredOff
    );
    let audit = universe.audit().last().expect("power-off audit event");
    assert_eq!(audit.kind, CommissioningAuditKind::CpuCommand);
    assert_eq!(audit.controller_id, Some(CONTROLLER));
    assert!(audit.success);
    assert!(audit.post_state_hash.is_some());
}

#[test]
fn controller_snapshot_restore_is_atomic_epoch_advancing_and_invalidates_the_session() {
    let base = package(
        1,
        &base_members(),
        "source-map-snapshot",
        "semantic-snapshot",
        "hardware-snapshot",
        "build-snapshot",
    );
    let mut universe = universe_for(&base);
    commit(&mut universe, &base, PostLoadMode::Stop);
    open_session(&mut universe);
    let snapshot = universe
        .controller(CONTROLLER)
        .unwrap()
        .runtime()
        .capture_snapshot()
        .unwrap();
    let old_epoch = universe
        .controller(CONTROLLER)
        .unwrap()
        .runtime()
        .controller_epoch();
    let old_universe_epoch = universe.universe_epoch();
    let binding = universe.session_command_binding(SESSION).unwrap();
    let restored = universe
        .restore_controller_snapshot(binding, &snapshot)
        .unwrap();
    let controller = universe.controller(CONTROLLER).unwrap();
    assert_eq!(controller.runtime().controller_epoch(), old_epoch + 1);
    assert_eq!(controller.runtime().last_state_hash(), restored);
    assert_eq!(universe.universe_epoch(), old_universe_epoch + 1);
    assert_eq!(
        universe.session(SESSION).unwrap().state(),
        SessionState::VirtualLinkLost
    );
    assert!(matches!(
        universe.request_stop(binding),
        Err(CommissioningError::Session(SessionError::NotOnline(
            SessionState::VirtualLinkLost
        )))
    ));
    let audit = universe.audit().last().expect("snapshot restore audit");
    assert_eq!(
        audit.kind,
        CommissioningAuditKind::ControllerSnapshotRestored
    );
    assert!(audit.success);
    assert_ne!(audit.pre_state_hash, audit.post_state_hash);
}

#[test]
fn mismatched_controller_snapshot_is_rejected_without_mutating_the_target() {
    let base = package(
        1,
        &base_members(),
        "source-map-snapshot-mismatch",
        "semantic-snapshot-mismatch",
        "hardware-snapshot-mismatch",
        "build-snapshot-mismatch",
    );
    let mut universe = universe_for(&base);
    commit(&mut universe, &base, PostLoadMode::Stop);
    open_session(&mut universe);
    let binding = universe.session_command_binding(SESSION).unwrap();
    let universe_epoch_before = universe.universe_epoch();
    let before = universe
        .controller(CONTROLLER)
        .unwrap()
        .semantic_state_hash();

    let mut other = VirtualController::new(UniverseId(0xdead), VirtualControllerId(0xbeef), 0x55);
    other.power_on().unwrap();
    other
        .install_verified_artifact(base.runtime_artifact())
        .unwrap();
    let wrong_snapshot = other.capture_snapshot().unwrap();
    assert_eq!(
        universe.restore_controller_snapshot(binding, &wrong_snapshot),
        Err(CommissioningError::Snapshot(SnapshotError::WrongController))
    );
    assert_eq!(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .semantic_state_hash(),
        before
    );
    assert_eq!(universe.universe_epoch(), universe_epoch_before);
    assert_eq!(
        universe.session(SESSION).unwrap().state(),
        SessionState::Online
    );
    let audit = universe.audit().last().expect("failed restore audit");
    assert_eq!(
        audit.kind,
        CommissioningAuditKind::ControllerSnapshotRestored
    );
    assert!(!audit.success);
    assert_eq!(audit.pre_state_hash, audit.post_state_hash);
}

#[test]
fn preview_is_deterministic_and_initial_load_is_explicit() {
    let base = package(
        1,
        &base_members(),
        "source-map-1",
        "semantic-1",
        "hardware-1",
        "build-1",
    );
    let first = universe_for(&base);
    let second = universe_for(&base);
    let a = first
        .prepare_load(
            CONTROLLER,
            &base,
            load_request(&base, PostLoadMode::Preserve),
        )
        .unwrap();
    let b = second
        .prepare_load(
            CONTROLLER,
            &base,
            load_request(&base, PostLoadMode::Preserve),
        )
        .unwrap();
    assert_eq!(a.hash(), b.hash());
    assert_eq!(a.id(), b.id());
    assert_eq!(
        a.hash().to_hex(),
        "bc786242dfffaeda31f5804e63b7dde40c8d7e6d06351149ca105f5c5b6188b6"
    );
    assert_eq!(a.compatibility(), CompatibilityClass::Initial);
    assert!(a.requires_stop());
    assert!(a.blockers().is_empty());
    assert!(
        a.memory_actions()
            .iter()
            .all(|action| action.kind == MemoryActionKind::Initialize)
    );
}

#[test]
fn corrupt_stale_and_blocking_candidates_are_rejected_without_target_mutation() {
    let members = base_members();
    let valid = package(
        1,
        &members,
        "source-map-1",
        "semantic-1",
        "hardware-1",
        "build-1",
    );
    let mut universe = universe_for(&valid);
    let before = universe
        .controller(CONTROLLER)
        .unwrap()
        .semantic_state_hash();

    let stale = VirtualLoadPackage::seal_verified(package_parts(PackageRecipe {
        code_step: 1,
        members: &members,
        source_label: "source-map-2",
        semantic_label: "semantic-2",
        hardware_label: "hardware-1",
        profile_label: "profile-1",
        build_label: "build-2",
        build_is_current: false,
        blocking_diagnostic_count: 0,
    }))
    .unwrap();
    universe
        .offline_controller_mut(OFFLINE)
        .unwrap()
        .record_build(&stale);
    let stale_preview = universe
        .prepare_load(
            CONTROLLER,
            &stale,
            load_request(&stale, PostLoadMode::Preserve),
        )
        .unwrap();
    assert!(
        stale_preview
            .blockers()
            .contains(&LoadBlocker::CandidateBuildStale)
    );

    let parts = package_parts(PackageRecipe {
        code_step: 1,
        members: &members,
        source_label: "source-map-3",
        semantic_label: "semantic-3",
        hardware_label: "hardware-1",
        profile_label: "profile-1",
        build_label: "build-3",
        build_is_current: true,
        blocking_diagnostic_count: 4,
    });
    let corrupt = VirtualLoadPackage::from_untrusted_package(parts, Hash32::ZERO, true);
    universe
        .offline_controller_mut(OFFLINE)
        .unwrap()
        .record_build(&corrupt);
    let preview = universe
        .prepare_load(
            CONTROLLER,
            &corrupt,
            load_request(&corrupt, PostLoadMode::Preserve),
        )
        .unwrap();
    assert!(
        preview
            .blockers()
            .contains(&LoadBlocker::CandidateIntegrity)
    );
    assert!(
        preview
            .blockers()
            .contains(&LoadBlocker::CandidateHasBlockingDiagnostics(4))
    );
    assert!(matches!(
        universe.commit_load(
            &preview,
            PreviewApproval::approve(&preview),
            &corrupt,
            LoadExecution::default(),
        ),
        Err(CommissioningError::Package(_))
    ));
    assert_eq!(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .semantic_state_hash(),
        before
    );
}

#[test]
fn memory_action_matrix_preserves_initializes_and_removes_exact_identities() {
    let base = package(
        1,
        &base_members(),
        "source-map-1",
        "semantic-1",
        "hardware-1",
        "build-1",
    );
    let mut universe = universe_for(&base);
    commit(&mut universe, &base, PostLoadMode::Run);
    open_session(&mut universe);
    let binding = universe.session_command_binding(SESSION).unwrap();
    universe.run_scan(binding).unwrap();
    assert_eq!(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .runtime()
            .actual_memory(MemoryId(1)),
        Some(CanonicalValue::I32(6))
    );

    let candidate_members = vec![
        Member {
            member_id: 100,
            runtime_id: 1,
            start: 50,
            role: MemoryRole::Marker,
            path: vec![10],
            retentive: true,
        },
        Member {
            member_id: 300,
            runtime_id: 3,
            start: 30,
            role: MemoryRole::GlobalDb,
            path: vec![30],
            retentive: false,
        },
    ];
    let candidate = package(
        1,
        &candidate_members,
        "source-map-2",
        "semantic-2",
        "hardware-1",
        "build-2",
    );
    universe
        .offline_controller_mut(OFFLINE)
        .unwrap()
        .record_build(&candidate);
    let preview = universe
        .prepare_load(
            CONTROLLER,
            &candidate,
            load_request(&candidate, PostLoadMode::Preserve),
        )
        .unwrap();
    assert_eq!(
        preview.compatibility(),
        CompatibilityClass::MemorySchemaChanging
    );
    assert!(preview.requires_stop());
    assert!(
        preview
            .memory_actions()
            .iter()
            .any(|action| { action.member_id == 100 && action.kind == MemoryActionKind::Preserve })
    );
    assert!(
        preview
            .memory_actions()
            .iter()
            .any(|action| { action.member_id == 200 && action.kind == MemoryActionKind::Remove })
    );
    assert!(
        preview.memory_actions().iter().any(|action| {
            action.member_id == 300 && action.kind == MemoryActionKind::Initialize
        })
    );

    let result = universe
        .commit_load(
            &preview,
            PreviewApproval::approve(&preview),
            &candidate,
            LoadExecution::default(),
        )
        .unwrap();
    assert_eq!(result.preserved_member_ids, vec![100]);
    assert_eq!(result.removed_member_ids, vec![200]);
    assert_eq!(result.initialized_member_ids, vec![300]);
    assert_eq!(result.final_cpu_state, CpuState::Stop);
    let runtime = universe.controller(CONTROLLER).unwrap().runtime();
    assert_eq!(
        runtime.actual_memory(MemoryId(1)),
        Some(CanonicalValue::I32(6))
    );
    assert_eq!(runtime.actual_memory(MemoryId(2)), None);
    assert_eq!(
        runtime.actual_memory(MemoryId(3)),
        Some(CanonicalValue::I32(30))
    );
}

#[test]
fn injected_commit_failure_restores_exact_target_and_keeps_failure_audit() {
    let members = base_members();
    let base = package(
        1,
        &members,
        "source-map-1",
        "semantic-1",
        "hardware-1",
        "build-1",
    );
    let mut universe = universe_for(&base);
    commit(&mut universe, &base, PostLoadMode::Run);
    open_session(&mut universe);
    let binding = universe.session_command_binding(SESSION).unwrap();
    universe.run_scan(binding).unwrap();

    let candidate = package(
        2,
        &members,
        "source-map-2",
        "semantic-2",
        "hardware-1",
        "build-2",
    );
    universe
        .offline_controller_mut(OFFLINE)
        .unwrap()
        .record_build(&candidate);
    let preview = universe
        .prepare_load(
            CONTROLLER,
            &candidate,
            load_request(&candidate, PostLoadMode::Preserve),
        )
        .unwrap();
    assert_eq!(preview.compatibility(), CompatibilityClass::CodeOnly);
    assert!(!preview.requires_stop());
    let before = universe
        .controller(CONTROLLER)
        .unwrap()
        .semantic_state_hash();
    let old_package = universe
        .controller(CONTROLLER)
        .unwrap()
        .loaded_package()
        .unwrap()
        .fingerprint();
    let old_epoch = universe
        .controller(CONTROLLER)
        .unwrap()
        .runtime()
        .controller_epoch();
    let session_before = universe.session(SESSION).unwrap().clone();
    let error = universe
        .commit_load(
            &preview,
            PreviewApproval::approve(&preview),
            &candidate,
            LoadExecution {
                failure_point: InternalFailurePoint::AfterCommitSwap,
            },
        )
        .unwrap_err();
    assert_eq!(
        error,
        CommissioningError::LoadRolledBack {
            failure_point: InternalFailurePoint::AfterCommitSwap,
            pre_state_hash: before,
            restored_state_hash: before,
        }
    );
    let instance = universe.controller(CONTROLLER).unwrap();
    assert_eq!(instance.semantic_state_hash(), before);
    assert_eq!(
        instance.loaded_package().unwrap().fingerprint(),
        old_package
    );
    assert_eq!(instance.runtime().controller_epoch(), old_epoch);
    assert_eq!(universe.session(SESSION).unwrap(), &session_before);
    let audit = universe.audit().last().unwrap();
    assert_eq!(audit.kind, CommissioningAuditKind::LoadFailed);
    assert!(!audit.success);
    assert_eq!(audit.pre_state_hash, audit.post_state_hash);

    let result = universe
        .commit_load(
            &preview,
            PreviewApproval::approve(&preview),
            &candidate,
            LoadExecution::default(),
        )
        .unwrap();
    assert_eq!(result.final_cpu_state, CpuState::Run);
    assert_eq!(result.controller_epoch, old_epoch + 1);
    assert_eq!(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .runtime()
            .actual_memory(MemoryId(1)),
        Some(CanonicalValue::I32(6))
    );
    assert_eq!(
        universe.session(SESSION).unwrap().state(),
        SessionState::VirtualLinkLost
    );
}

#[test]
fn online_session_epochs_and_stale_commands_are_guarded() {
    let base = package(
        1,
        &base_members(),
        "source-map-1",
        "semantic-1",
        "hardware-1",
        "build-1",
    );
    let mut universe = universe_for(&base);
    commit(&mut universe, &base, PostLoadMode::Preserve);
    let cpu_before = universe
        .controller(CONTROLLER)
        .unwrap()
        .runtime()
        .cpu_state();
    open_session(&mut universe);
    assert_eq!(
        universe.session(SESSION).unwrap().state(),
        SessionState::Online
    );
    assert_eq!(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .runtime()
            .cpu_state(),
        cpu_before
    );
    assert_eq!(
        universe.session(SESSION).unwrap().comparison().monitoring,
        MonitoringComparison::Inactive
    );

    let stale = universe.session_command_binding(SESSION).unwrap();
    let mut wrong_universe = stale;
    wrong_universe.expected_universe_epoch += 1;
    assert!(matches!(
        universe.request_run(wrong_universe, RestartKind::Resume),
        Err(CommissioningError::Session(
            SessionError::StaleUniverseEpoch
        ))
    ));
    let mut wrong_controller = stale;
    wrong_controller.expected_controller_epoch += 1;
    assert!(matches!(
        universe.request_run(wrong_controller, RestartKind::Resume),
        Err(CommissioningError::Session(
            SessionError::StaleControllerEpoch
        ))
    ));
    let mut wrong_session = stale;
    wrong_session.expected_session_epoch += 1;
    assert!(matches!(
        universe.request_run(wrong_session, RestartKind::Resume),
        Err(CommissioningError::Session(SessionError::StaleSessionEpoch))
    ));
    universe.request_run(stale, RestartKind::Resume).unwrap();
    assert_eq!(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .runtime()
            .cpu_state(),
        CpuState::Run
    );
    assert!(matches!(
        universe.request_stop(stale),
        Err(CommissioningError::Session(SessionError::StaleTargetState))
    ));
    assert_eq!(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .runtime()
            .cpu_state(),
        CpuState::Run
    );
    let current = universe.session_command_binding(SESSION).unwrap();
    universe.request_stop(current).unwrap();

    let prior_session_epoch = universe.session(SESSION).unwrap().session_epoch();
    let candidate = package(
        2,
        &base_members(),
        "source-map-2",
        "semantic-2",
        "hardware-1",
        "build-2",
    );
    universe
        .offline_controller_mut(OFFLINE)
        .unwrap()
        .record_build(&candidate);
    commit(&mut universe, &candidate, PostLoadMode::Preserve);
    assert_eq!(
        universe.session(SESSION).unwrap().state(),
        SessionState::VirtualLinkLost
    );
    assert!(universe.session(SESSION).unwrap().session_epoch() > prior_session_epoch);
    universe.begin_reconnect(SESSION).unwrap();
    assert_eq!(
        universe.session(SESSION).unwrap().state(),
        SessionState::Reconnecting
    );
    universe.complete_reconnect(SESSION).unwrap();
    assert_eq!(
        universe.session(SESSION).unwrap().state(),
        SessionState::Online
    );
    assert_eq!(
        universe.session(SESSION).unwrap().controller_epoch(),
        Some(
            universe
                .controller(CONTROLLER)
                .unwrap()
                .runtime()
                .controller_epoch()
        )
    );
    universe.begin_disconnect(SESSION).unwrap();
    assert_eq!(
        universe.session(SESSION).unwrap().state(),
        SessionState::Closing
    );
    universe.complete_disconnect(SESSION).unwrap();
    assert_eq!(
        universe.session(SESSION).unwrap().state(),
        SessionState::Closed
    );
}

#[test]
fn comparison_keeps_offline_edits_visible_without_mutating_loaded_target() {
    let base = package(
        1,
        &base_members(),
        "source-map-1",
        "semantic-1",
        "hardware-1",
        "build-1",
    );
    let mut universe = universe_for(&base);
    open_session(&mut universe);
    assert_eq!(
        universe
            .session(SESSION)
            .unwrap()
            .comparison()
            .software_to_loaded,
        PackageComparison::NotLoaded
    );
    assert_eq!(
        universe
            .session(SESSION)
            .unwrap()
            .comparison()
            .hardware_to_loaded,
        HardwareComparison::NotLoaded
    );

    commit(&mut universe, &base, PostLoadMode::Preserve);
    universe.begin_reconnect(SESSION).unwrap();
    universe.complete_reconnect(SESSION).unwrap();
    let comparison = universe.session(SESSION).unwrap().comparison();
    assert_eq!(comparison.software_to_loaded, PackageComparison::Match);
    assert_eq!(comparison.hardware_to_loaded, HardwareComparison::Match);
    assert_eq!(comparison.profile, ProfileComparison::Match);
    let target_hash = universe
        .controller(CONTROLLER)
        .unwrap()
        .semantic_state_hash();
    let loaded = universe
        .controller(CONTROLLER)
        .unwrap()
        .loaded_package()
        .unwrap()
        .fingerprint();

    universe
        .offline_controller_mut(OFFLINE)
        .unwrap()
        .mark_source_edited(hash("source-revision-2"));
    universe.refresh_session_comparison(SESSION).unwrap();
    let comparison = universe.session(SESSION).unwrap().comparison();
    assert_eq!(comparison.source_to_build, OfflineSourceBuild::Stale);
    assert!(!comparison.software_build_current);
    assert_eq!(comparison.software_to_loaded, PackageComparison::Match);
    assert_eq!(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .loaded_package()
            .unwrap()
            .fingerprint(),
        loaded
    );
    assert_eq!(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .semantic_state_hash(),
        target_hash
    );
    universe
        .offline_controller_mut(OFFLINE)
        .unwrap()
        .mark_saved();
    universe.refresh_session_comparison(SESSION).unwrap();
    assert_eq!(
        universe
            .session(SESSION)
            .unwrap()
            .comparison()
            .source_to_build,
        OfflineSourceBuild::Stale
    );
}

#[test]
fn package_identity_only_load_preserves_runtime_and_still_invalidates_old_session_epoch() {
    let members = base_members();
    let base = package(
        1,
        &members,
        "source-map-1",
        "semantic-1",
        "hardware-1",
        "build-1",
    );
    let mapping_update = package(
        1,
        &members,
        "source-map-2",
        "semantic-1",
        "hardware-1",
        "build-2",
    );
    assert_eq!(
        base.runtime_artifact().fingerprint(),
        mapping_update.runtime_artifact().fingerprint()
    );
    let mut universe = universe_for(&base);
    commit(&mut universe, &base, PostLoadMode::Run);
    open_session(&mut universe);
    let binding = universe.session_command_binding(SESSION).unwrap();
    universe.run_scan(binding).unwrap();
    let value = universe
        .controller(CONTROLLER)
        .unwrap()
        .runtime()
        .actual_memory(MemoryId(1));
    let epoch = universe
        .controller(CONTROLLER)
        .unwrap()
        .runtime()
        .controller_epoch();
    universe
        .offline_controller_mut(OFFLINE)
        .unwrap()
        .record_build(&mapping_update);
    let preview = universe
        .prepare_load(
            CONTROLLER,
            &mapping_update,
            load_request(&mapping_update, PostLoadMode::Preserve),
        )
        .unwrap();
    assert_eq!(
        preview.compatibility(),
        CompatibilityClass::PackageIdentityOnly
    );
    let result = universe
        .commit_load(
            &preview,
            PreviewApproval::approve(&preview),
            &mapping_update,
            LoadExecution::default(),
        )
        .unwrap();
    assert_eq!(result.final_cpu_state, CpuState::Run);
    assert_eq!(result.controller_epoch, epoch + 1);
    assert_eq!(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .runtime()
            .actual_memory(MemoryId(1)),
        value
    );
    assert_eq!(
        universe.session(SESSION).unwrap().state(),
        SessionState::VirtualLinkLost
    );
}

#[test]
fn instance_removal_requires_bound_preview_and_marks_session_lost() {
    let base = package(
        1,
        &base_members(),
        "source-map-1",
        "semantic-1",
        "hardware-1",
        "build-1",
    );
    let mut universe = universe_for(&base);
    commit(&mut universe, &base, PostLoadMode::Preserve);
    open_session(&mut universe);
    let preview = universe.prepare_remove_instance(CONTROLLER).unwrap();
    assert_eq!(preview.invalidated_session_ids, vec![SESSION]);
    let mut tampered = preview.clone();
    tampered.invalidated_session_ids.clear();
    assert!(matches!(
        universe.remove_instance(&tampered, RemoveInstanceApproval::approve(&tampered)),
        Err(CommissioningError::ApprovalMismatch)
    ));
    let removed = universe
        .remove_instance(&preview, RemoveInstanceApproval::approve(&preview))
        .unwrap();
    assert_eq!(removed.instance_id(), INSTANCE);
    assert!(universe.controller(CONTROLLER).is_none());
    assert_eq!(
        universe.session(SESSION).unwrap().state(),
        SessionState::VirtualLinkLost
    );
}

#[test]
fn journey_f_load_failure_mismatch_and_go_online_are_state_separated() {
    let members = base_members();
    let initial = package(
        1,
        &members,
        "source-map-1",
        "semantic-1",
        "hardware-1",
        "build-1",
    );
    let mut universe = universe_for(&initial);
    let initial_result = commit(&mut universe, &initial, PostLoadMode::Preserve);
    assert_eq!(
        initial_result.comparison.software_to_loaded,
        PackageComparison::Match
    );
    open_session(&mut universe);
    let cpu_before_online_observation = universe
        .controller(CONTROLLER)
        .unwrap()
        .runtime()
        .cpu_state();
    universe.observe_session(SESSION).unwrap();
    assert_eq!(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .runtime()
            .cpu_state(),
        cpu_before_online_observation
    );

    let candidate = package(
        2,
        &members,
        "source-map-2",
        "semantic-2",
        "hardware-1",
        "build-2",
    );
    universe
        .offline_controller_mut(OFFLINE)
        .unwrap()
        .record_build(&candidate);
    let preview = universe
        .prepare_load(
            CONTROLLER,
            &candidate,
            load_request(&candidate, PostLoadMode::Preserve),
        )
        .unwrap();
    let pre = universe
        .controller(CONTROLLER)
        .unwrap()
        .semantic_state_hash();
    assert!(matches!(
        universe.commit_load(
            &preview,
            PreviewApproval::approve(&preview),
            &candidate,
            LoadExecution {
                failure_point: InternalFailurePoint::AfterRuntimeStage,
            },
        ),
        Err(CommissioningError::LoadRolledBack {
            pre_state_hash,
            restored_state_hash,
            ..
        }) if pre_state_hash == pre && restored_state_hash == pre
    ));
    assert_eq!(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .semantic_state_hash(),
        pre
    );

    universe
        .offline_controller_mut(OFFLINE)
        .unwrap()
        .mark_source_edited(hash("journey-f-edit"));
    universe.refresh_session_comparison(SESSION).unwrap();
    let comparison = universe.session(SESSION).unwrap().comparison();
    assert_eq!(comparison.source_to_build, OfflineSourceBuild::Stale);
    assert_eq!(comparison.software_to_loaded, PackageComparison::Mismatch);
    assert_eq!(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .loaded_package()
            .unwrap()
            .fingerprint(),
        initial.fingerprint()
    );
}

#[test]
fn typed_instance_lifecycle_clone_reset_replace_and_fault_state_are_atomic() {
    let base = package(
        1,
        &base_members(),
        "source-map-1",
        "semantic-1",
        "hardware-1",
        "build-1",
    );
    let mut universe = universe_for(&base);
    commit(&mut universe, &base, PostLoadMode::Run);
    open_session(&mut universe);
    let scan_binding = universe.session_command_binding(SESSION).unwrap();
    universe.run_scan(scan_binding).unwrap();

    let source = universe.controller(CONTROLLER).unwrap();
    let source_state_hash = source.semantic_state_hash();
    let source_controller_epoch = source.runtime().controller_epoch();
    let source_memory = source.runtime().actual_memory(MemoryId(1));
    let source_package = source.loaded_package().unwrap().fingerprint();
    let clone_result = universe
        .clone_instance(CloneInstanceCommand {
            command_id: 0x6100,
            source_controller_id: CONTROLLER,
            clone_instance_id: CLONE_INSTANCE,
            clone_controller_id: CLONE_CONTROLLER,
            expected_universe_epoch: universe.universe_epoch(),
            expected_source_controller_epoch: source_controller_epoch,
            expected_source_state_hash: source_state_hash,
        })
        .unwrap();
    let clone = universe.controller(CLONE_CONTROLLER).unwrap();
    assert_eq!(clone.instance_id(), CLONE_INSTANCE);
    assert_eq!(clone.runtime().controller_id(), CLONE_CONTROLLER);
    assert_eq!(clone.runtime().controller_epoch(), 1);
    assert_eq!(clone.runtime().cpu_state(), CpuState::Run);
    assert_eq!(clone.runtime().actual_memory(MemoryId(1)), source_memory);
    assert_eq!(
        clone.loaded_package().unwrap().fingerprint(),
        source_package
    );
    assert_eq!(clone_result.clone_state_hash, clone.semantic_state_hash());
    assert_eq!(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .semantic_state_hash(),
        source_state_hash
    );

    assert!(matches!(
        universe.clone_instance(CloneInstanceCommand {
            command_id: 0x6101,
            source_controller_id: CONTROLLER,
            clone_instance_id: ControllerInstanceId(0x3010),
            clone_controller_id: VirtualControllerId(0x4010),
            expected_universe_epoch: universe.universe_epoch(),
            expected_source_controller_epoch: source_controller_epoch,
            expected_source_state_hash: Hash32::ZERO,
        }),
        Err(CommissioningError::PreviewStateChanged)
    ));

    let offline_hardware = universe
        .offline_controller(OFFLINE)
        .unwrap()
        .configured
        .configured_hardware_fingerprint;
    let clone_before_fault = universe.controller(CLONE_CONTROLLER).unwrap();
    let clone_epoch = clone_before_fault.runtime().controller_epoch();
    let clone_hash = clone_before_fault.semantic_state_hash();
    universe
        .apply_actual_hardware_fault(ActualHardwareFaultCommand {
            command_id: 0x6200,
            target_controller_id: CLONE_CONTROLLER,
            expected_universe_epoch: universe.universe_epoch(),
            expected_controller_epoch: clone_epoch,
            expected_target_state_hash: clone_hash,
            present: false,
            fault_state_hash: hash("module-not-present"),
        })
        .unwrap();
    assert!(
        !universe
            .actual_hardware_matches_configured(CLONE_CONTROLLER)
            .unwrap()
    );
    assert_eq!(
        universe
            .offline_controller(OFFLINE)
            .unwrap()
            .configured
            .configured_hardware_fingerprint,
        offline_hardware
    );

    let stop_binding = universe.session_command_binding(SESSION).unwrap();
    universe.request_stop(stop_binding).unwrap();
    let reset_preview = universe
        .prepare_reset_instance(CONTROLLER, ResetInstanceKind::SimulatedPowerCycle)
        .unwrap();
    assert_eq!(reset_preview.reset_member_ids, vec![200]);
    assert_eq!(reset_preview.preserved_member_ids, vec![100]);
    assert_eq!(reset_preview.final_cpu_state, CpuState::Stop);
    assert!(reset_preview.invalidated_session_ids.contains(&SESSION));
    let pre_reset_hash = universe
        .controller(CONTROLLER)
        .unwrap()
        .semantic_state_hash();
    assert!(matches!(
        universe.reset_instance(
            &reset_preview,
            ResetInstanceApproval::approve(&reset_preview),
            LifecycleExecution {
                failure_point: InternalFailurePoint::AfterCommitSwap,
            },
        ),
        Err(CommissioningError::LifecycleRolledBack {
            failure_point: InternalFailurePoint::AfterCommitSwap,
            pre_state_hash,
            restored_state_hash,
        }) if pre_state_hash == pre_reset_hash && restored_state_hash == pre_reset_hash
    ));
    assert_eq!(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .semantic_state_hash(),
        pre_reset_hash
    );
    assert_eq!(
        universe.session(SESSION).unwrap().state(),
        SessionState::Online
    );
    let reset_result = universe
        .reset_instance(
            &reset_preview,
            ResetInstanceApproval::approve(&reset_preview),
            LifecycleExecution::default(),
        )
        .unwrap();
    assert_eq!(
        reset_result.new_controller_epoch,
        reset_result.old_controller_epoch + 1
    );
    assert_eq!(reset_result.final_cpu_state, CpuState::Stop);
    assert_eq!(
        universe.session(SESSION).unwrap().state(),
        SessionState::VirtualLinkLost
    );
    let memory_reset_preview = universe
        .prepare_reset_instance(CONTROLLER, ResetInstanceKind::MemoryReset)
        .unwrap();
    assert_eq!(memory_reset_preview.reset_member_ids, vec![100, 200]);
    assert!(memory_reset_preview.preserved_member_ids.is_empty());

    let replace_preview = universe
        .prepare_replace_instance(ReplaceInstanceCommand {
            command_id: 0x6300,
            target_controller_id: CLONE_CONTROLLER,
            replacement_instance_id: REPLACEMENT_INSTANCE,
        })
        .unwrap();
    assert_eq!(replace_preview.removed_member_ids, vec![100, 200]);
    assert_eq!(replace_preview.final_cpu_state, CpuState::PoweredOff);
    let pre_replace_hash = universe
        .controller(CLONE_CONTROLLER)
        .unwrap()
        .semantic_state_hash();
    assert!(matches!(
        universe.replace_instance(
            &replace_preview,
            ReplaceInstanceApproval::approve(&replace_preview),
            LifecycleExecution {
                failure_point: InternalFailurePoint::AfterCommitSwap,
            },
        ),
        Err(CommissioningError::LifecycleRolledBack {
            failure_point: InternalFailurePoint::AfterCommitSwap,
            pre_state_hash,
            restored_state_hash,
        }) if pre_state_hash == pre_replace_hash && restored_state_hash == pre_replace_hash
    ));
    assert_eq!(
        universe
            .controller(CLONE_CONTROLLER)
            .unwrap()
            .semantic_state_hash(),
        pre_replace_hash
    );
    let replace_result = universe
        .replace_instance(
            &replace_preview,
            ReplaceInstanceApproval::approve(&replace_preview),
            LifecycleExecution::default(),
        )
        .unwrap();
    let replacement = universe.controller(CLONE_CONTROLLER).unwrap();
    assert_eq!(replacement.instance_id(), REPLACEMENT_INSTANCE);
    assert_eq!(replacement.runtime().cpu_state(), CpuState::PoweredOff);
    assert!(replacement.loaded_package().is_none());
    assert_eq!(
        replace_result.runtime.new_controller_epoch,
        replace_result.runtime.old_controller_epoch + 1
    );
    assert!(
        universe
            .actual_hardware_matches_configured(CLONE_CONTROLLER)
            .unwrap()
    );
}

#[test]
fn online_session_observes_faulted_cpu_without_changing_target_state() {
    let package = faulting_package(&base_members());
    let mut universe = universe_for(&package);
    commit(&mut universe, &package, PostLoadMode::Preserve);
    open_session(&mut universe);
    let run = universe.session_command_binding(SESSION).unwrap();
    universe.request_run(run, RestartKind::Resume).unwrap();
    let scan = universe.session_command_binding(SESSION).unwrap();
    assert!(matches!(
        universe.run_scan(scan).unwrap(),
        RunOutcome::Faulted(_)
    ));
    assert_eq!(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .runtime()
            .cpu_state(),
        CpuState::Faulted
    );
    assert_eq!(
        universe.session(SESSION).unwrap().state(),
        SessionState::Online
    );

    let faulted_state_hash = universe
        .controller(CONTROLLER)
        .unwrap()
        .semantic_state_hash();
    let second_session = VirtualOnlineSessionId(0x5001);
    universe
        .begin_go_online(second_session, OFFLINE, CONTROLLER)
        .unwrap();
    universe.complete_go_online(second_session).unwrap();
    universe.observe_session(second_session).unwrap();
    assert_eq!(
        universe.session(second_session).unwrap().state(),
        SessionState::Online
    );
    assert_eq!(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .semantic_state_hash(),
        faulted_state_hash
    );
}

#[test]
fn request_run_rejects_invalid_required_virtual_hardware_without_state_mutation() {
    let base = package(
        1,
        &base_members(),
        "source-map-required-hardware",
        "semantic-required-hardware",
        "hardware-required-hardware",
        "build-required-hardware",
    );
    let mut universe = universe_for(&base);
    commit(&mut universe, &base, PostLoadMode::Stop);
    open_session(&mut universe);

    let controller_epoch = universe
        .controller(CONTROLLER)
        .unwrap()
        .runtime()
        .controller_epoch();
    let target_state_hash = universe
        .controller(CONTROLLER)
        .unwrap()
        .semantic_state_hash();
    let fault_state_hash = hash("required-module-pulled");
    universe
        .apply_actual_hardware_fault(ActualHardwareFaultCommand {
            command_id: 0x6400,
            target_controller_id: CONTROLLER,
            expected_universe_epoch: universe.universe_epoch(),
            expected_controller_epoch: controller_epoch,
            expected_target_state_hash: target_state_hash,
            present: false,
            fault_state_hash,
        })
        .unwrap();

    let before = universe
        .controller(CONTROLLER)
        .unwrap()
        .semantic_state_hash();
    let error = universe
        .request_run(
            universe.session_command_binding(SESSION).unwrap(),
            RestartKind::Resume,
        )
        .unwrap_err();
    assert!(matches!(
        error,
        CommissioningError::RequiredVirtualHardwareInvalid(CONTROLLER)
    ));
    let controller = universe.controller(CONTROLLER).unwrap();
    assert_eq!(controller.runtime().cpu_state(), CpuState::Stop);
    assert_eq!(controller.semantic_state_hash(), before);
}
