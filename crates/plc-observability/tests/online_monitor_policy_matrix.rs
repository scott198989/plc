//! VER-ONL-0002 and VER-MON-0001 CPU-state, session-loss, and display-neutrality vectors.

use plc_commissioning::{
    ConfiguredController, ControllerInstanceId, CreateInstanceCommand, LifecycleExecution,
    LoadExecution, LoadPackageParts, LoadRequest, MemoryMemberSchema, MemoryRole,
    OfflineControllerId, OfflineEngineeringState, OfflineSourceBuild, PostLoadMode,
    PreviewApproval, ResetInstanceApproval, ResetInstanceKind, SessionState, VirtualLoadPackage,
    VirtualOnlineSessionId, VirtualUniverse,
};
use plc_observability::*;
use plc_runtime::{
    ArtifactPackage, ArtifactSpec, BlockId, Instruction, MemoryDefinition, Operand, Operation,
    ProgramBlock, ProgramImage, RestartKind, RuntimeValueTarget, Sha256,
};

const UNIVERSE: UniverseId = UniverseId(0x9100);
const OFFLINE: OfflineControllerId = OfflineControllerId(0x9200);
const INSTANCE: ControllerInstanceId = ControllerInstanceId(0x9300);
const CONTROLLER: VirtualControllerId = VirtualControllerId(0x9400);
const SESSION: VirtualOnlineSessionId = VirtualOnlineSessionId(0x9500);
const MEMORY: MemoryId = MemoryId(1);
const TARGET: StableTargetId = StableTargetId(0x9600);
const TABLE: WatchTableId = WatchTableId(0x9700);
const ROW: WatchRowId = WatchRowId(0x9701);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActiveTraceOutcome {
    Hold,
    Sample,
    Abort(TraceAbortReason),
}

#[derive(Clone, Copy, Debug)]
struct CpuPolicyCase {
    state: CpuState,
    monitor_boundary: PublicationBoundary,
    tool_boundary: PublicationBoundary,
    monitor_publishes: bool,
    modify_allowed: bool,
    force_create_allowed: bool,
    force_remove_allowed: bool,
    trace_arm_allowed: bool,
    active_trace_outcome: ActiveTraceOutcome,
}

const CPU_POLICY_CASES: [CpuPolicyCase; 7] = [
    CpuPolicyCase {
        state: CpuState::PoweredOff,
        monitor_boundary: PublicationBoundary::SerializedCommand,
        tool_boundary: PublicationBoundary::SerializedCommand,
        monitor_publishes: false,
        modify_allowed: false,
        force_create_allowed: false,
        force_remove_allowed: false,
        trace_arm_allowed: false,
        active_trace_outcome: ActiveTraceOutcome::Abort(TraceAbortReason::SessionLost),
    },
    CpuPolicyCase {
        state: CpuState::Stop,
        monitor_boundary: PublicationBoundary::SerializedCommand,
        tool_boundary: PublicationBoundary::SerializedCommand,
        monitor_publishes: true,
        modify_allowed: true,
        force_create_allowed: true,
        force_remove_allowed: true,
        trace_arm_allowed: true,
        active_trace_outcome: ActiveTraceOutcome::Hold,
    },
    CpuPolicyCase {
        state: CpuState::Startup,
        monitor_boundary: PublicationBoundary::SerializedCommand,
        tool_boundary: PublicationBoundary::SerializedCommand,
        monitor_publishes: false,
        modify_allowed: false,
        force_create_allowed: false,
        force_remove_allowed: false,
        trace_arm_allowed: false,
        active_trace_outcome: ActiveTraceOutcome::Hold,
    },
    CpuPolicyCase {
        state: CpuState::Resetting,
        monitor_boundary: PublicationBoundary::SerializedCommand,
        tool_boundary: PublicationBoundary::SerializedCommand,
        monitor_publishes: false,
        modify_allowed: false,
        force_create_allowed: false,
        force_remove_allowed: false,
        trace_arm_allowed: false,
        active_trace_outcome: ActiveTraceOutcome::Abort(TraceAbortReason::CpuReset),
    },
    CpuPolicyCase {
        state: CpuState::Run,
        monitor_boundary: PublicationBoundary::ScanEnd,
        tool_boundary: PublicationBoundary::ScanEnd,
        monitor_publishes: true,
        modify_allowed: true,
        force_create_allowed: true,
        force_remove_allowed: true,
        trace_arm_allowed: true,
        active_trace_outcome: ActiveTraceOutcome::Sample,
    },
    CpuPolicyCase {
        state: CpuState::PausedEducational,
        monitor_boundary: PublicationBoundary::SerializedCommand,
        tool_boundary: PublicationBoundary::SerializedCommand,
        monitor_publishes: true,
        modify_allowed: true,
        force_create_allowed: true,
        force_remove_allowed: true,
        trace_arm_allowed: true,
        active_trace_outcome: ActiveTraceOutcome::Hold,
    },
    CpuPolicyCase {
        state: CpuState::Faulted,
        monitor_boundary: PublicationBoundary::FatalFault,
        tool_boundary: PublicationBoundary::SerializedCommand,
        monitor_publishes: true,
        modify_allowed: false,
        force_create_allowed: false,
        force_remove_allowed: true,
        trace_arm_allowed: false,
        active_trace_outcome: ActiveTraceOutcome::Abort(TraceAbortReason::CpuFault),
    },
];

fn hash(label: &str) -> Hash32 {
    Sha256::digest(label.as_bytes())
}

fn context(
    cpu_state: CpuState,
    boundary: PublicationBoundary,
    scan_sequence: u64,
) -> ObservationContext {
    ObservationContext {
        universe_id: UNIVERSE,
        universe_epoch: 1,
        controller_id: CONTROLLER,
        controller_epoch: 3,
        session_id: SESSION,
        session_epoch: 5,
        package_fingerprint: hash("policy-package"),
        artifact_fingerprint: hash("policy-artifact"),
        profile_fingerprint: hash("policy-profile"),
        target_state_hash: hash("policy-state"),
        cpu_state,
        virtual_timestamp_ms: scan_sequence.saturating_mul(10),
        scan_sequence,
        event_sequence: scan_sequence.saturating_add(100),
        publication_boundary: boundary,
    }
}

fn access_capabilities() -> AccessCapabilities {
    AccessCapabilities {
        monitor: true,
        modify: true,
        force: true,
        trace: true,
        natural_layer: true,
        effective_layer: true,
    }
}

fn policy_catalog() -> ProbeCatalog {
    let mut catalog = ProbeCatalog::new(hash("policy-artifact"), hash("policy-profile"));
    catalog
        .insert(ProbeDefinition {
            id: TARGET,
            runtime_target: RuntimeTarget::Memory(MEMORY),
            bit_range: BitRange::whole_value(),
            value_type: ValueType::I32,
            instance_path: vec![0x9601],
            capabilities: access_capabilities(),
            primary_source: None,
            display_name: "PolicyValue".into(),
        })
        .unwrap();
    catalog
}

fn publication(value: i32) -> PublishedTargetValue {
    PublishedTargetValue {
        target_id: TARGET,
        value_type: ValueType::I32,
        natural_value: CanonicalValue::I32(value),
        effective_value: CanonicalValue::I32(value),
        raw_input_value: None,
        committed_output_value: None,
        delivered_output_value: None,
        quality: Quality::Good,
        force: None,
    }
}

fn monitor(retained_samples_per_row: usize) -> MonitoringEngine {
    let mut engine = MonitoringEngine::new(MonitoringLimits {
        tables_per_project: 2,
        rows_per_table: 4,
        active_subscriptions_per_controller: 4,
        retained_samples_per_row,
    })
    .unwrap();
    engine
        .upsert_table(WatchTable {
            id: TABLE,
            name: "CPU policy".into(),
            rows: vec![WatchRow {
                id: ROW,
                target: TargetReference::Stable(TARGET),
                layer: ProbeLayer::Effective,
                display_base: DisplayBase::Decimal,
                unit: Some("count".into()),
                format: None,
                note: Some("verification-only presentation metadata".into()),
                order: 1,
            }],
        })
        .unwrap();
    engine
}

fn modify_command(
    command_id: u128,
    ctx: ObservationContext,
    catalog: &ProbeCatalog,
    registry: &ForceRegistry,
) -> ModifyCommand {
    ModifyCommand {
        command_id,
        idempotency_key: command_id.saturating_add(0x10_0000),
        session_id: ctx.session_id,
        controller_id: ctx.controller_id,
        expected_universe_epoch: ctx.universe_epoch,
        expected_controller_epoch: ctx.controller_epoch,
        expected_session_epoch: ctx.session_epoch,
        expected_artifact_fingerprint: ctx.artifact_fingerprint,
        expected_target_state_hash: ctx.target_state_hash,
        expected_probe_catalog_hash: catalog.catalog_hash(),
        expected_force_registry_version: registry.version(),
        expected_force_registry_hash: registry.registry_hash(),
        allow_overwrite_queued: false,
        requested_boundary: ctx.publication_boundary,
        author_identity: 0x9800,
        audit_context_hash: hash("modify-policy-audit"),
        items: vec![ModifyItem {
            target: TargetReference::Stable(TARGET),
            expected_instance_path: vec![0x9601],
            expected_value_type: ValueType::I32,
            value: CanonicalValue::I32(41),
        }],
    }
}

fn create_force_command(
    command_id: u128,
    force_id: ForceId,
    ctx: ObservationContext,
    registry: &ForceRegistry,
) -> ForceCommand {
    ForceCommand {
        command_id,
        idempotency_key: command_id.saturating_add(0x20_0000),
        expected_universe_epoch: ctx.universe_epoch,
        expected_controller_epoch: ctx.controller_epoch,
        expected_session_epoch: ctx.session_epoch,
        expected_artifact_fingerprint: ctx.artifact_fingerprint,
        expected_target_state_hash: ctx.target_state_hash,
        expected_registry_version: registry.version(),
        expected_registry_hash: registry.registry_hash(),
        audit_context_hash: hash("force-policy-create"),
        kind: ForceCommandKind::Create {
            force_id,
            target: TargetReference::Stable(TARGET),
            value: CanonicalValue::I32(99),
            natural_at_application: CanonicalValue::I32(7),
            actor_identity: 0x9801,
            reason: "CPU policy verification".into(),
        },
    }
}

fn remove_force_command(
    command_id: u128,
    force_id: ForceId,
    ctx: ObservationContext,
    registry: &ForceRegistry,
) -> ForceCommand {
    ForceCommand {
        command_id,
        idempotency_key: command_id.saturating_add(0x30_0000),
        expected_universe_epoch: ctx.universe_epoch,
        expected_controller_epoch: ctx.controller_epoch,
        expected_session_epoch: ctx.session_epoch,
        expected_artifact_fingerprint: ctx.artifact_fingerprint,
        expected_target_state_hash: ctx.target_state_hash,
        expected_registry_version: registry.version(),
        expected_registry_hash: registry.registry_hash(),
        audit_context_hash: hash("force-policy-remove"),
        kind: ForceCommandKind::Remove {
            force_id,
            expected_entry_hash: registry.entry(force_id).unwrap().entry_hash,
            actor_identity: 0x9802,
            reason: "CPU policy verification cleanup".into(),
        },
    }
}

fn trace_engine(config_id: TraceConfigId) -> TraceEngine {
    let mut trace = TraceEngine::new(TraceLimits {
        configurations_per_project: 2,
        channels_per_configuration: 2,
        samples_per_capture: 16,
        concurrent_captures_per_controller: 1,
        minimum_virtual_cadence_ms: 10,
        maximum_virtual_duration_ms: 1_000,
        trigger_depth: 8,
        trigger_nodes: 16,
    })
    .unwrap();
    trace
        .upsert_config(TraceConfig {
            id: config_id,
            trigger_id: TraceTriggerId(config_id.0.saturating_add(1)),
            name: "CPU policy trace".into(),
            channels: vec![TraceChannel {
                id: TraceChannelId(config_id.0.saturating_add(2)),
                alias: "policy_value".into(),
                probe: TraceProbeKind::LoadedTarget {
                    target: TargetReference::Stable(TARGET),
                    layer: ProbeLayer::Effective,
                },
                display_unit: None,
            }],
            cadence: TraceCadence::EveryScans(1),
            trigger: TraceTrigger::Immediate,
            pre_trigger_samples: 0,
            post_trigger_samples: 0,
            post_trigger_duration_ms: None,
            maximum_duration_ms: 500,
        })
        .unwrap();
    trace
}

#[test]
fn every_cpu_state_drives_one_monitor_modify_force_and_trace_policy_matrix() {
    let catalog = policy_catalog();

    for (ordinal, case) in CPU_POLICY_CASES.iter().copied().enumerate() {
        let ordinal = ordinal as u128;
        let initial = context(CpuState::Run, PublicationBoundary::ScanEnd, 1);
        let monitor_context = context(case.state, case.monitor_boundary, 2);
        let tool_context = context(case.state, case.tool_boundary, 2);

        let mut monitoring = monitor(8);
        monitoring.start(initial, &catalog).unwrap();
        assert_eq!(monitoring.publish(initial, &[publication(1)]), Ok(1));
        assert_eq!(monitoring.history(ROW).unwrap().len(), 1);
        let appended = monitoring
            .publish(monitor_context, &[publication(2)])
            .unwrap();
        if case.monitor_publishes {
            assert_eq!(appended, 1, "monitor publication in {:?}", case.state);
            assert_eq!(monitoring.history(ROW).unwrap().len(), 2);
            let latest = monitoring.latest(ROW).unwrap();
            assert_eq!(latest.value, CanonicalValue::I32(2));
            assert_eq!(latest.freshness, SampleFreshness::Current);
            assert_eq!(latest.boundary, case.monitor_boundary);
            if case.state == CpuState::Faulted {
                let later_fault_publication =
                    context(CpuState::Faulted, PublicationBoundary::SerializedCommand, 3);
                assert_eq!(
                    monitoring.publish(later_fault_publication, &[publication(3)]),
                    Ok(1)
                );
                assert_eq!(
                    monitoring.latest(ROW).unwrap().boundary,
                    PublicationBoundary::SerializedCommand
                );
            }
        } else {
            assert_eq!(appended, 0, "monitor suppression in {:?}", case.state);
            assert_eq!(monitoring.history(ROW).unwrap().len(), 1);
            assert_eq!(
                monitoring.latest(ROW).unwrap().freshness,
                SampleFreshness::Stale
            );
        }

        let registry = ForceRegistry::new();
        let mut scheduler = ModifyScheduler::default();
        let modify_id = 0xa000_u128.saturating_add(ordinal.saturating_mul(0x100));
        let command = modify_command(modify_id, tool_context, &catalog, &registry);
        if case.modify_allowed {
            let queued = scheduler
                .submit(command, tool_context, &catalog, &registry)
                .unwrap();
            assert_eq!(queued.state, ModifyReceiptState::Queued);
            assert_eq!(queued.scheduled_boundary, case.tool_boundary);
            let plan = scheduler
                .next_due(tool_context, &catalog, &registry)
                .unwrap()
                .unwrap();
            let applied = scheduler
                .commit(
                    &plan,
                    hash("matrix-modify-applied"),
                    tool_context.event_sequence.saturating_add(1),
                )
                .unwrap();
            assert_eq!(applied.state, ModifyReceiptState::Applied);

            let cancel_command = modify_command(
                modify_id.saturating_add(1),
                tool_context,
                &catalog,
                &registry,
            );
            let cancel_id = cancel_command.command_id;
            scheduler
                .submit(cancel_command, tool_context, &catalog, &registry)
                .unwrap();
            let canceled = scheduler.cancel(cancel_id, tool_context).unwrap();
            assert_eq!(canceled.state, ModifyReceiptState::Canceled);
            assert_eq!(canceled.cancellation_code, Some("USER_CANCELED"));
            assert_eq!(scheduler.pending_count(), 0);
        } else {
            assert_eq!(
                scheduler.submit(command, tool_context, &catalog, &registry),
                Err(ModifyError::CpuStateDisallowed(case.state))
            );
            assert_eq!(scheduler.pending_count(), 0);

            let stop_context = context(CpuState::Stop, PublicationBoundary::SerializedCommand, 3);
            let queued_then_invalid = modify_command(
                modify_id.saturating_add(2),
                stop_context,
                &catalog,
                &registry,
            );
            let idempotency_key = queued_then_invalid.idempotency_key;
            scheduler
                .submit(queued_then_invalid, stop_context, &catalog, &registry)
                .unwrap();
            assert_eq!(
                scheduler
                    .next_due(tool_context, &catalog, &registry)
                    .unwrap(),
                None
            );
            let canceled = scheduler
                .receipt_by_idempotency_key(idempotency_key)
                .unwrap();
            assert_eq!(canceled.state, ModifyReceiptState::Canceled);
            assert_eq!(canceled.cancellation_code, Some("TARGET_CONTEXT_CHANGED"));
        }

        let create_id = 0xb000_u128.saturating_add(ordinal.saturating_mul(0x100));
        let create_force_id = ForceId(create_id.saturating_add(1));
        let mut create_registry = ForceRegistry::new();
        let create =
            create_force_command(create_id, create_force_id, tool_context, &create_registry);
        let create_version = create_registry.version();
        let create_hash = create_registry.registry_hash();
        if case.force_create_allowed {
            let (receipt, plan) = create_registry
                .apply_at_boundary(&create, tool_context, &catalog)
                .unwrap();
            assert_eq!(receipt.applied_boundary, case.tool_boundary);
            assert_eq!(plan.boundary, case.tool_boundary);
            assert_eq!(create_registry.active_ids(), vec![create_force_id]);
        } else {
            assert_eq!(
                create_registry.apply_at_boundary(&create, tool_context, &catalog),
                Err(ForceError::CpuStateDisallowed(case.state))
            );
            assert_eq!(create_registry.version(), create_version);
            assert_eq!(create_registry.registry_hash(), create_hash);
        }

        let seed_context = context(CpuState::Stop, PublicationBoundary::SerializedCommand, 4);
        let seeded_force_id = ForceId(create_id.saturating_add(2));
        let mut removal_registry = ForceRegistry::new();
        let seed = create_force_command(
            create_id.saturating_add(10),
            seeded_force_id,
            seed_context,
            &removal_registry,
        );
        removal_registry
            .apply_at_boundary(&seed, seed_context, &catalog)
            .unwrap();
        let remove = remove_force_command(
            create_id.saturating_add(11),
            seeded_force_id,
            tool_context,
            &removal_registry,
        );
        let removal_version = removal_registry.version();
        let removal_hash = removal_registry.registry_hash();
        if case.force_remove_allowed {
            let (receipt, plan) = removal_registry
                .apply_at_boundary(&remove, tool_context, &catalog)
                .unwrap();
            assert_eq!(receipt.applied_boundary, case.tool_boundary);
            assert_eq!(plan.boundary, case.tool_boundary);
            assert!(removal_registry.active_ids().is_empty());
        } else {
            assert_eq!(
                removal_registry.apply_at_boundary(&remove, tool_context, &catalog),
                Err(ForceError::CpuStateDisallowed(case.state))
            );
            assert_eq!(removal_registry.version(), removal_version);
            assert_eq!(removal_registry.registry_hash(), removal_hash);
            assert_eq!(removal_registry.active_ids(), vec![seeded_force_id]);
        }

        let arm_config = TraceConfigId(0xc000_u128.saturating_add(ordinal.saturating_mul(0x10)));
        let mut arm_trace = trace_engine(arm_config);
        if case.trace_arm_allowed {
            arm_trace.arm(arm_config, tool_context, &catalog).unwrap();
            if case.state == CpuState::Run {
                let captures = arm_trace
                    .publish(tool_context, &[publication(3)], &[])
                    .unwrap();
                assert_eq!(captures.len(), 1);
                let capture = arm_trace.capture(captures[0]).unwrap();
                assert_eq!(capture.samples.len(), 1);
                assert_eq!(capture.aborted, None);
            } else {
                assert!(
                    arm_trace
                        .publish(tool_context, &[publication(3)], &[])
                        .unwrap()
                        .is_empty()
                );
                assert_eq!(arm_trace.state(arm_config), TraceState::Armed);
                let first_run = context(CpuState::Run, PublicationBoundary::ScanEnd, 5);
                let captures = arm_trace
                    .publish(first_run, &[publication(4)], &[])
                    .unwrap();
                assert_eq!(captures.len(), 1);
                let capture = arm_trace.capture(captures[0]).unwrap();
                assert_eq!(capture.samples.len(), 1);
                assert_eq!(capture.samples[0].boundary, PublicationBoundary::ScanEnd);
                assert_eq!(capture.aborted, None);
            }
        } else {
            assert_eq!(
                arm_trace.arm(arm_config, tool_context, &catalog),
                Err(TraceError::CpuStateDisallowed(case.state))
            );
            assert_eq!(arm_trace.state(arm_config), TraceState::Idle);
        }

        let active_config = TraceConfigId(arm_config.0.saturating_add(8));
        let mut active_trace = trace_engine(active_config);
        active_trace
            .arm(
                active_config,
                context(CpuState::Stop, PublicationBoundary::SerializedCommand, 1),
                &catalog,
            )
            .unwrap();
        let captures = active_trace
            .publish(monitor_context, &[publication(5)], &[])
            .unwrap();
        match case.active_trace_outcome {
            ActiveTraceOutcome::Hold => {
                assert!(captures.is_empty());
                assert_eq!(active_trace.state(active_config), TraceState::Armed);
            }
            ActiveTraceOutcome::Sample => {
                assert_eq!(captures.len(), 1);
                assert_eq!(active_trace.state(active_config), TraceState::Completed);
                let capture = active_trace.capture(captures[0]).unwrap();
                assert_eq!(capture.samples.len(), 1);
                assert_eq!(capture.aborted, None);
            }
            ActiveTraceOutcome::Abort(reason) => {
                assert_eq!(captures.len(), 1);
                assert_eq!(active_trace.state(active_config), TraceState::Aborted);
                assert_eq!(
                    active_trace.capture(captures[0]).unwrap().aborted,
                    Some(reason)
                );
            }
        }
    }
}

fn runtime_package() -> VirtualLoadPackage {
    let runtime_artifact = ArtifactPackage::seal_verified(ArtifactSpec::edu21(
        hash("runtime-profile"),
        vec![MemoryDefinition {
            id: MEMORY,
            value_type: ValueType::I32,
            loaded_start: CanonicalValue::I32(0),
            retentive: true,
        }],
        vec![],
        vec![],
        ProgramImage {
            startup: None,
            timed: vec![],
            cyclic: ProgramBlock {
                id: BlockId(1),
                instructions: vec![Instruction::new(
                    1,
                    0x9901,
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
        semantic_build_fingerprint: hash("runtime-semantic"),
        verified_ir_fingerprint: hash("runtime-ir"),
        schedule_fingerprint: hash("runtime-schedule"),
        hardware_fingerprint: hash("runtime-hardware"),
        source_map_fingerprint: hash("runtime-source-map"),
        probe_identity_fingerprint: hash("runtime-probes"),
        capability_fingerprint: hash("runtime-capabilities"),
        build_snapshot_hash: hash("runtime-build"),
        build_is_current: true,
        blocking_diagnostic_count: 0,
        memory_schema: vec![MemoryMemberSchema {
            member_id: TARGET.0,
            runtime_memory_id: MEMORY,
            value_type: ValueType::I32,
            role: MemoryRole::Marker,
            instance_path: vec![0x9601],
            retentive: true,
            loaded_start: CanonicalValue::I32(0),
        }],
        state_schema: vec![],
    })
    .unwrap()
}

fn online_universe() -> (VirtualUniverse, ProbeCatalog) {
    let package = runtime_package();
    let mut offline = OfflineEngineeringState {
        configured: ConfiguredController {
            id: OFFLINE,
            configured_hardware_fingerprint: package.hardware_fingerprint(),
            profile_fingerprint: package.profile_fingerprint(),
        },
        source_revision_hash: hash("runtime-source"),
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

    let artifact = universe
        .controller(CONTROLLER)
        .unwrap()
        .runtime()
        .loaded_fingerprint()
        .unwrap();
    let mut catalog = ProbeCatalog::new(artifact, package.profile_fingerprint());
    catalog
        .insert(ProbeDefinition {
            id: TARGET,
            runtime_target: RuntimeTarget::Memory(MEMORY),
            bit_range: BitRange::whole_value(),
            value_type: ValueType::I32,
            instance_path: vec![0x9601],
            capabilities: access_capabilities(),
            primary_source: None,
            display_name: "RuntimePolicyValue".into(),
        })
        .unwrap();
    (universe, catalog)
}

fn online_context(universe: &VirtualUniverse, boundary: PublicationBoundary) -> ObservationContext {
    let binding = universe.session_command_binding(SESSION).unwrap();
    ObservationContext::from_virtual_universe(universe, binding, boundary).unwrap()
}

#[test]
fn warm_restart_preserves_force_and_destructive_previews_enumerate_then_clear_it() {
    let (mut universe, catalog) = online_universe();
    let mut registry = ForceRegistry::new();
    let pending_modifies = ModifyScheduler::default();
    let stop_context = online_context(&universe, PublicationBoundary::SerializedCommand);
    let force_id = ForceId(0xa100);
    let create = create_force_command(0xa101, force_id, stop_context, &registry);
    let binding = universe.session_command_binding(SESSION).unwrap();
    let executed = execute_force_command(
        &mut universe,
        binding,
        &mut registry,
        &pending_modifies,
        &catalog,
        &create,
    )
    .unwrap();
    assert!(executed.publication.is_some());
    assert_eq!(registry.active_ids(), vec![force_id]);
    assert_eq!(
        universe.controller(CONTROLLER).unwrap().active_force_ids(),
        &[force_id]
    );

    let run_binding = universe.session_command_binding(SESSION).unwrap();
    universe
        .request_run(run_binding, RestartKind::WarmRestart)
        .unwrap();
    let controller = universe.controller(CONTROLLER).unwrap();
    assert_eq!(controller.runtime().cpu_state(), CpuState::Run);
    assert_eq!(controller.active_force_ids(), &[force_id]);
    assert_eq!(registry.active_ids(), vec![force_id]);
    assert_eq!(
        controller
            .runtime()
            .effective_value(RuntimeValueTarget::Memory(MEMORY)),
        Some(CanonicalValue::I32(99))
    );

    let stop_binding = universe.session_command_binding(SESSION).unwrap();
    universe.request_stop(stop_binding).unwrap();
    let before_preview = universe
        .controller(CONTROLLER)
        .unwrap()
        .semantic_state_hash();
    let power_cycle = universe
        .prepare_reset_instance(CONTROLLER, ResetInstanceKind::SimulatedPowerCycle)
        .unwrap();
    let memory_reset = universe
        .prepare_reset_instance(CONTROLLER, ResetInstanceKind::MemoryReset)
        .unwrap();
    assert_eq!(power_cycle.cleared_force_ids, vec![force_id]);
    assert_eq!(memory_reset.cleared_force_ids, vec![force_id]);
    assert_eq!(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .semantic_state_hash(),
        before_preview
    );
    assert_eq!(
        universe.controller(CONTROLLER).unwrap().active_force_ids(),
        &[force_id]
    );

    let result = universe
        .reset_instance(
            &memory_reset,
            ResetInstanceApproval::approve(&memory_reset),
            LifecycleExecution::default(),
        )
        .unwrap();
    assert_eq!(result.cleared_force_ids, vec![force_id]);
    assert!(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .active_force_ids()
            .is_empty()
    );
}

#[test]
fn session_loss_stales_cached_samples_and_reconnect_requires_full_resolution() {
    let (mut universe, catalog) = online_universe();
    let old_binding = universe.session_command_binding(SESSION).unwrap();
    let old_context = ObservationContext::from_virtual_universe(
        &universe,
        old_binding,
        PublicationBoundary::SerializedCommand,
    )
    .unwrap();
    let mut monitoring = monitor(8);
    monitoring.start(old_context, &catalog).unwrap();
    assert_eq!(monitoring.publish(old_context, &[publication(10)]), Ok(1));
    let persistence = monitoring.persistence().unwrap();

    let reset = universe
        .prepare_reset_instance(CONTROLLER, ResetInstanceKind::SimulatedPowerCycle)
        .unwrap();
    universe
        .reset_instance(
            &reset,
            ResetInstanceApproval::approve(&reset),
            LifecycleExecution::default(),
        )
        .unwrap();
    assert_eq!(
        universe.session(SESSION).unwrap().state(),
        SessionState::VirtualLinkLost
    );
    monitoring.mark_stale();
    assert_eq!(monitoring.state(), MonitorState::Degraded);
    assert_eq!(
        monitoring.latest(ROW).unwrap().freshness,
        SampleFreshness::Stale
    );
    assert_eq!(
        ObservationContext::from_virtual_universe(
            &universe,
            old_binding,
            PublicationBoundary::SerializedCommand,
        ),
        Err(ContextError::SessionNotOnline(
            SessionState::VirtualLinkLost
        ))
    );

    universe.begin_reconnect(SESSION).unwrap();
    universe.complete_reconnect(SESSION).unwrap();
    let rebound_context = online_context(&universe, PublicationBoundary::SerializedCommand);
    assert_ne!(
        old_context.controller_epoch,
        rebound_context.controller_epoch
    );
    assert_ne!(old_context.session_epoch, rebound_context.session_epoch);
    assert_eq!(
        monitoring.publish(rebound_context, &[publication(11)]),
        Err(MonitorError::EpochChanged)
    );
    assert_eq!(
        monitoring.latest(ROW).unwrap().freshness,
        SampleFreshness::Stale
    );
    assert_eq!(monitoring.persistence().unwrap(), persistence);

    monitoring.stop().unwrap();
    monitoring.start(rebound_context, &catalog).unwrap();
    assert_eq!(
        monitoring.publish(rebound_context, &[publication(12)]),
        Ok(1)
    );
    let rebound = monitoring.latest(ROW).unwrap();
    assert_eq!(rebound.value, CanonicalValue::I32(12));
    assert_eq!(rebound.freshness, SampleFreshness::Current);
    assert_eq!(monitoring.persistence().unwrap(), persistence);
}

#[test]
fn throttled_display_reads_do_not_change_the_authoritative_publication_stream() {
    let catalog = policy_catalog();
    let initial = context(CpuState::Run, PublicationBoundary::ScanEnd, 0);
    let mut unthrottled = monitor(32);
    let mut throttled = monitor(32);
    unthrottled.start(initial, &catalog).unwrap();
    throttled.start(initial, &catalog).unwrap();

    let mut rendered = Vec::new();
    for scan in 1..=12 {
        let publication_context = context(CpuState::Run, PublicationBoundary::ScanEnd, scan);
        let value = publication(scan as i32);
        assert_eq!(unthrottled.publish(publication_context, &[value]), Ok(1));
        assert_eq!(throttled.publish(publication_context, &[value]), Ok(1));
        assert_eq!(
            unthrottled.latest(ROW).unwrap().value,
            CanonicalValue::I32(scan as i32)
        );
        if scan.is_multiple_of(4) {
            rendered.push(throttled.latest(ROW).unwrap().value);
        }
    }

    assert_eq!(
        rendered,
        vec![
            CanonicalValue::I32(4),
            CanonicalValue::I32(8),
            CanonicalValue::I32(12),
        ]
    );
    assert_eq!(unthrottled.history(ROW), throttled.history(ROW));
    assert_eq!(unthrottled.history(ROW).unwrap().len(), 12);
    assert_eq!(
        unthrottled.persistence().unwrap(),
        throttled.persistence().unwrap()
    );
}
