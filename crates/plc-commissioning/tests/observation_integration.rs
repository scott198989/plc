use plc_commissioning::*;
use plc_runtime::{
    ArtifactSpec, BlockId, ChannelDefinition, ChannelDirection, Instruction, Operand, Operation,
    ProgramBlock, ProgramImage, RestartKind, Sha256,
};

const UNIVERSE: UniverseId = UniverseId(0x7100);
const OFFLINE: OfflineControllerId = OfflineControllerId(0x7200);
const INSTANCE: ControllerInstanceId = ControllerInstanceId(0x7300);
const CONTROLLER: VirtualControllerId = VirtualControllerId(0x7400);
const SESSION: VirtualOnlineSessionId = VirtualOnlineSessionId(0x7500);
const MEMORY: MemoryId = MemoryId(1);
const INPUT: ChannelId = ChannelId(1);

fn hash(value: &str) -> Hash32 {
    Sha256::digest(value.as_bytes())
}

fn load_package() -> VirtualLoadPackage {
    let runtime_artifact = ArtifactPackage::seal_verified(ArtifactSpec::edu21(
        hash("observation-profile"),
        vec![plc_runtime::MemoryDefinition {
            id: MEMORY,
            value_type: ValueType::I32,
            loaded_start: CanonicalValue::I32(0),
            retentive: true,
        }],
        vec![ChannelDefinition {
            id: INPUT,
            direction: ChannelDirection::Input,
            value_type: ValueType::Bool,
            canonical_default: CanonicalValue::Bool(false),
        }],
        vec![],
        ProgramImage {
            startup: None,
            timed: vec![],
            cyclic: ProgramBlock {
                id: BlockId(1),
                instructions: vec![Instruction::new(
                    1,
                    0x7101,
                    Operation::AddI32 {
                        left: Operand::Memory(MEMORY),
                        right: Operand::Constant(CanonicalValue::I32(1)),
                        target: MEMORY,
                    },
                )],
            },
        },
    ))
    .unwrap();
    VirtualLoadPackage::seal_verified(LoadPackageParts {
        runtime_artifact,
        semantic_build_fingerprint: hash("observation-semantic"),
        verified_ir_fingerprint: hash("observation-ir"),
        schedule_fingerprint: hash("observation-schedule"),
        hardware_fingerprint: hash("observation-hardware"),
        source_map_fingerprint: hash("observation-source-map"),
        probe_identity_fingerprint: hash("observation-probes"),
        capability_fingerprint: hash("observation-capabilities"),
        build_snapshot_hash: hash("observation-build"),
        build_is_current: true,
        blocking_diagnostic_count: 0,
        memory_schema: vec![MemoryMemberSchema {
            member_id: 0x7110,
            runtime_memory_id: MEMORY,
            value_type: ValueType::I32,
            role: MemoryRole::Marker,
            instance_path: vec![],
            retentive: true,
            loaded_start: CanonicalValue::I32(0),
        }],
        state_schema: vec![],
    })
    .unwrap()
}

fn online_universe() -> VirtualUniverse {
    let package = load_package();
    let mut offline = OfflineEngineeringState {
        configured: ConfiguredController {
            id: OFFLINE,
            configured_hardware_fingerprint: package.hardware_fingerprint(),
            profile_fingerprint: package.profile_fingerprint(),
        },
        source_revision_hash: hash("observation-source"),
        build_snapshot_hash: None,
        project_saved: true,
        source_to_build: OfflineSourceBuild::Absent,
        software_build_current: false,
        hardware_build_current: false,
        current_package_fingerprint: None,
        built_hardware: None,
    };
    offline.record_build(&package);
    let mut universe = VirtualUniverse::new(UNIVERSE);
    universe.register_offline_controller(offline).unwrap();
    universe
        .create_instance(CreateInstanceCommand {
            command_id: 1,
            instance_id: INSTANCE,
            offline_controller_id: OFFLINE,
            controller_id: CONTROLLER,
            deterministic_seed: 0x55aa,
        })
        .unwrap();
    universe.power_on(CONTROLLER).unwrap();
    let preview = universe
        .prepare_load(
            CONTROLLER,
            &package,
            LoadRequest {
                expected_build_snapshot_hash: package.build_snapshot_hash(),
                requested_post_load_mode: PostLoadMode::Stop,
                initialize_compatible_members: false,
                valid_through_event_sequence: u64::MAX,
            },
        )
        .unwrap();
    universe
        .commit_load(
            &preview,
            PreviewApproval::approve(&preview),
            &package,
            LoadExecution::default(),
        )
        .unwrap();
    universe
        .begin_go_online(SESSION, OFFLINE, CONTROLLER)
        .unwrap();
    universe.complete_go_online(SESSION).unwrap();
    universe
}

fn boundary_command(universe: &VirtualUniverse) -> RuntimeBoundaryCommand {
    let runtime = universe.controller(CONTROLLER).unwrap().runtime();
    RuntimeBoundaryCommand {
        command_id: 0x7601,
        controller_id: CONTROLLER,
        expected_controller_epoch: runtime.controller_epoch(),
        expected_artifact_fingerprint: runtime.loaded_fingerprint().unwrap(),
        expected_state_hash: runtime.semantic_state_hash(),
        natural_writes: vec![],
        force_deltas: vec![],
        audit_context_hash: hash("observation-audit"),
    }
}

fn projection(
    universe: &VirtualUniverse,
    command_deltas: &[RuntimeForceDelta],
    next_registry_hash: Hash32,
    ids: Vec<ForceId>,
) -> ForceRegistryProjection {
    let instance = universe.controller(CONTROLLER).unwrap();
    let overlay_hash = instance
        .runtime()
        .projected_force_overlay_hash(command_deltas)
        .unwrap();
    ForceRegistryProjection::new(
        instance.force_registry_hash(),
        next_registry_hash,
        ids,
        overlay_hash,
    )
    .unwrap()
}

#[test]
fn serialized_observation_command_commits_runtime_and_force_projection_atomically() {
    let mut universe = online_universe();
    let binding = universe.session_command_binding(SESSION).unwrap();
    let mut command = boundary_command(&universe);
    command.natural_writes.push(RuntimeNaturalWrite {
        target: RuntimeValueTarget::Memory(MEMORY),
        value: CanonicalValue::I32(7),
    });
    command.force_deltas.push(RuntimeForceDelta {
        target: RuntimeValueTarget::Memory(MEMORY),
        value: Some(CanonicalValue::I32(41)),
    });
    let registry_hash = hash("force-registry-1");
    let projection = projection(
        &universe,
        &command.force_deltas,
        registry_hash,
        vec![ForceId(1)],
    );

    let receipt = universe
        .apply_observation_boundary(binding, &command, &projection)
        .unwrap();
    let instance = universe.controller(CONTROLLER).unwrap();
    assert_eq!(
        instance.runtime().actual_memory(MEMORY),
        Some(CanonicalValue::I32(7))
    );
    assert_eq!(
        instance
            .runtime()
            .effective_value(RuntimeValueTarget::Memory(MEMORY)),
        Some(CanonicalValue::I32(41))
    );
    assert_eq!(instance.active_force_ids(), &[ForceId(1)]);
    assert_eq!(instance.force_registry_hash(), registry_hash);
    assert_eq!(
        receipt.controller_state_hash,
        instance.semantic_state_hash()
    );
    assert!(universe.session(SESSION).unwrap().comparison().force_active);
    assert_eq!(
        universe.audit().last().unwrap().kind,
        CommissioningAuditKind::ObservationCommand
    );
}

#[test]
fn virtual_input_command_uses_online_binding_refresh_audit_and_runtime_idempotency() {
    let mut universe = online_universe();
    let binding = universe.session_command_binding(SESSION).unwrap();
    let command = InputCommand {
        command_id: CommandId(0x7550),
        idempotency_key: 0x7551,
        controller_id: CONTROLLER,
        expected_controller_epoch: binding.expected_controller_epoch,
        channel_id: INPUT,
        value: CanonicalValue::Bool(true),
        audit_provenance_hash: hash("virtual-input-audit"),
    };
    let audit_count = universe.audit().len();
    let receipt = universe
        .set_virtual_input_raw(binding, command.clone())
        .unwrap();
    assert!(!receipt.duplicate);
    assert_eq!(universe.audit().len(), audit_count + 1);
    let audit = universe.audit().last().unwrap();
    assert_eq!(audit.kind, CommissioningAuditKind::VirtualInputChanged);
    assert!(audit.success);
    assert_ne!(audit.pre_state_hash, audit.post_state_hash);
    assert_eq!(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .runtime()
            .boundary()
            .raw_input(INPUT)
            .unwrap()
            .canonical_value,
        CanonicalValue::Bool(true)
    );

    assert!(matches!(
        universe.set_virtual_input_raw(binding, command.clone()),
        Err(CommissioningError::Session(SessionError::StaleTargetState))
    ));
    let state_before_duplicate = universe.semantic_state_hash();
    let duplicate = universe
        .set_virtual_input_raw(universe.session_command_binding(SESSION).unwrap(), command)
        .unwrap();
    assert!(duplicate.duplicate);
    assert_eq!(universe.semantic_state_hash(), state_before_duplicate);
    assert_eq!(universe.audit().len(), audit_count + 1);
}

#[test]
fn stale_or_inconsistent_projection_rolls_back_exactly() {
    let mut universe = online_universe();
    let binding = universe.session_command_binding(SESSION).unwrap();
    let mut command = boundary_command(&universe);
    command.force_deltas.push(RuntimeForceDelta {
        target: RuntimeValueTarget::Memory(MEMORY),
        value: Some(CanonicalValue::I32(9)),
    });
    let before = universe.semantic_state_hash();
    let bad_projection = ForceRegistryProjection::new(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .force_registry_hash(),
        hash("force-registry-bad"),
        vec![ForceId(1)],
        hash("wrong-runtime-overlay"),
    )
    .unwrap();

    assert!(matches!(
        universe.apply_observation_boundary(binding, &command, &bad_projection),
        Err(CommissioningError::ForceRuntimeProjectionMismatch { .. })
    ));
    assert_eq!(universe.semantic_state_hash(), before);
    assert!(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .runtime()
            .force_overlays()
            .is_empty()
    );
}

#[test]
fn run_scan_command_is_atomic_and_reset_clears_both_force_states() {
    let mut universe = online_universe();
    let binding = universe.session_command_binding(SESSION).unwrap();
    universe.request_run(binding, RestartKind::Resume).unwrap();
    let binding = universe.session_command_binding(SESSION).unwrap();
    let instance = universe.controller(CONTROLLER).unwrap();
    let runtime = instance.runtime();
    let deltas = vec![RuntimeForceDelta {
        target: RuntimeValueTarget::Memory(MEMORY),
        value: Some(CanonicalValue::I32(50)),
    }];
    let command = RuntimeScanCommand {
        command_id: 0x7602,
        controller_id: CONTROLLER,
        expected_controller_epoch: runtime.controller_epoch(),
        expected_artifact_fingerprint: runtime.loaded_fingerprint().unwrap(),
        expected_state_hash: runtime.semantic_state_hash(),
        pre_program_writes: vec![RuntimeNaturalWrite {
            target: RuntimeValueTarget::Memory(MEMORY),
            value: CanonicalValue::I32(10),
        }],
        post_program_writes: vec![],
        force_deltas: deltas.clone(),
        audit_context_hash: hash("observation-run-audit"),
    };
    let projection = projection(
        &universe,
        &deltas,
        hash("force-registry-run"),
        vec![ForceId(10)],
    );
    universe
        .run_scan_with_observation(binding, &command, &projection)
        .unwrap();
    let instance = universe.controller(CONTROLLER).unwrap();
    assert_eq!(
        instance.runtime().actual_memory(MEMORY),
        Some(CanonicalValue::I32(51))
    );
    assert_eq!(
        instance
            .runtime()
            .force_overlay(RuntimeValueTarget::Memory(MEMORY)),
        Some(CanonicalValue::I32(50))
    );

    let binding = universe.session_command_binding(SESSION).unwrap();
    universe.request_stop(binding).unwrap();
    let preview = universe
        .prepare_reset_instance(CONTROLLER, ResetInstanceKind::MemoryReset)
        .unwrap();
    universe
        .reset_instance(
            &preview,
            ResetInstanceApproval::approve(&preview),
            LifecycleExecution::default(),
        )
        .unwrap();
    let instance = universe.controller(CONTROLLER).unwrap();
    assert!(instance.active_force_ids().is_empty());
    assert!(instance.runtime().force_overlays().is_empty());
    assert_eq!(
        instance.runtime().actual_memory(MEMORY),
        Some(CanonicalValue::I32(0))
    );
}
