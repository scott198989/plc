use plc_commissioning::*;
use plc_observability::*;
use plc_runtime::{
    ArtifactSpec, BlockId, Instruction, Operand, Operation, ProgramBlock, ProgramImage,
    RestartKind, SCAN_QUANTUM_MS, Sha256,
};

const UNIVERSE: UniverseId = UniverseId(0x8100);
const OFFLINE: OfflineControllerId = OfflineControllerId(0x8200);
const INSTANCE: ControllerInstanceId = ControllerInstanceId(0x8300);
const CONTROLLER: VirtualControllerId = VirtualControllerId(0x8400);
const SESSION: VirtualOnlineSessionId = VirtualOnlineSessionId(0x8500);
const MEMORY: MemoryId = MemoryId(1);
const TARGET: StableTargetId = StableTargetId(0x8600);

fn hash(value: &str) -> Hash32 {
    Sha256::digest(value.as_bytes())
}

fn package() -> VirtualLoadPackage {
    let runtime_artifact = ArtifactPackage::seal_verified(ArtifactSpec::edu21(
        hash("execution-profile"),
        vec![plc_runtime::MemoryDefinition {
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
                    0x8101,
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
        semantic_build_fingerprint: hash("execution-semantic"),
        verified_ir_fingerprint: hash("execution-ir"),
        schedule_fingerprint: hash("execution-schedule"),
        hardware_fingerprint: hash("execution-hardware"),
        source_map_fingerprint: hash("execution-source-map"),
        probe_identity_fingerprint: hash("execution-probes"),
        capability_fingerprint: hash("execution-capabilities"),
        build_snapshot_hash: hash("execution-build"),
        build_is_current: true,
        blocking_diagnostic_count: 0,
        memory_schema: vec![MemoryMemberSchema {
            member_id: TARGET.0,
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

fn setup() -> (VirtualUniverse, ProbeCatalog) {
    let package = package();
    let mut offline = OfflineEngineeringState {
        configured: ConfiguredController {
            id: OFFLINE,
            configured_hardware_fingerprint: package.hardware_fingerprint(),
            profile_fingerprint: package.profile_fingerprint(),
        },
        source_revision_hash: hash("execution-source"),
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
            instance_path: vec![],
            capabilities: AccessCapabilities {
                monitor: true,
                modify: true,
                force: true,
                trace: true,
                natural_layer: true,
                effective_layer: true,
            },
            primary_source: None,
            display_name: "execution-memory".into(),
        })
        .unwrap();
    (universe, catalog)
}

fn modify_command(
    context: ObservationContext,
    catalog: &ProbeCatalog,
    registry: &ForceRegistry,
) -> ModifyCommand {
    ModifyCommand {
        command_id: 0x8701,
        idempotency_key: 0x8801,
        session_id: context.session_id,
        controller_id: context.controller_id,
        expected_universe_epoch: context.universe_epoch,
        expected_controller_epoch: context.controller_epoch,
        expected_session_epoch: context.session_epoch,
        expected_artifact_fingerprint: context.artifact_fingerprint,
        expected_target_state_hash: context.target_state_hash,
        expected_probe_catalog_hash: catalog.catalog_hash(),
        expected_force_registry_version: registry.version(),
        expected_force_registry_hash: registry.registry_hash(),
        allow_overwrite_queued: false,
        requested_boundary: context.publication_boundary,
        author_identity: 0x89,
        audit_context_hash: hash("execution-modify-audit"),
        items: vec![ModifyItem {
            target: TargetReference::Stable(TARGET),
            expected_instance_path: vec![],
            expected_value_type: ValueType::I32,
            value: CanonicalValue::I32(12),
        }],
    }
}

fn force_command(context: ObservationContext, registry: &ForceRegistry) -> ForceCommand {
    ForceCommand {
        command_id: 0x8702,
        idempotency_key: 0x8802,
        expected_universe_epoch: context.universe_epoch,
        expected_controller_epoch: context.controller_epoch,
        expected_session_epoch: context.session_epoch,
        expected_artifact_fingerprint: context.artifact_fingerprint,
        expected_target_state_hash: context.target_state_hash,
        expected_registry_version: registry.version(),
        expected_registry_hash: registry.registry_hash(),
        audit_context_hash: hash("execution-force-audit"),
        kind: ForceCommandKind::Create {
            force_id: ForceId(1),
            target: TargetReference::Stable(TARGET),
            value: CanonicalValue::I32(33),
            natural_at_application: CanonicalValue::I32(0),
            actor_identity: 0x8901,
            reason: "commissioning exercise".into(),
        },
    }
}

#[test]
fn queued_modify_publishes_once_and_commits_scheduler_with_the_universe() {
    let (mut universe, catalog) = setup();
    let registry = ForceRegistry::new();
    assert_eq!(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .force_registry_hash(),
        registry.registry_hash()
    );
    let binding = universe.session_command_binding(SESSION).unwrap();
    let context = ObservationContext::from_virtual_universe(
        &universe,
        binding,
        PublicationBoundary::SerializedCommand,
    )
    .unwrap();
    let mut scheduler = ModifyScheduler::default();
    scheduler
        .submit(
            modify_command(context, &catalog, &registry),
            context,
            &catalog,
            &registry,
        )
        .unwrap();
    let plan = scheduler
        .next_due(context, &catalog, &registry)
        .unwrap()
        .unwrap();

    let result =
        publish_modify_plan(&mut universe, binding, &mut scheduler, &registry, &plan).unwrap();
    assert_eq!(result.modify.state, ModifyReceiptState::Applied);
    assert_eq!(scheduler.pending_count(), 0);
    assert_eq!(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .runtime()
            .actual_memory(MEMORY),
        Some(CanonicalValue::I32(12))
    );
}

#[test]
fn trace_scan_metrics_are_sampled_from_the_authoritative_commissioned_scan_receipt() {
    let (mut universe, catalog) = setup();
    let registry = ForceRegistry::new();
    let initial_binding = universe.session_command_binding(SESSION).unwrap();
    universe
        .request_run(initial_binding, RestartKind::WarmRestart)
        .unwrap();
    let binding = universe.session_command_binding(SESSION).unwrap();
    let arm_context =
        ObservationContext::from_virtual_universe(&universe, binding, PublicationBoundary::ScanEnd)
            .unwrap();

    let mut trace = TraceEngine::new(TraceLimits::edu21()).unwrap();
    trace
        .upsert_config(TraceConfig {
            id: TraceConfigId(0x8a00),
            trigger_id: TraceTriggerId(0x8a01),
            name: "authoritative scan metrics".into(),
            channels: vec![
                TraceChannel {
                    id: TraceChannelId(0x8a10),
                    alias: "scan quantum".into(),
                    probe: TraceProbeKind::ScanQuantumMs,
                    display_unit: Some("ms".into()),
                },
                TraceChannel {
                    id: TraceChannelId(0x8a11),
                    alias: "scan work".into(),
                    probe: TraceProbeKind::ScanWorkUnits,
                    display_unit: Some("work units".into()),
                },
            ],
            cadence: TraceCadence::EveryScans(1),
            trigger: TraceTrigger::Immediate,
            pre_trigger_samples: 0,
            post_trigger_samples: 0,
            post_trigger_duration_ms: None,
            maximum_duration_ms: 100,
        })
        .unwrap();
    trace
        .arm(TraceConfigId(0x8a00), arm_context, &catalog)
        .unwrap();
    assert_eq!(
        trace.publish(arm_context, &[], &[]).unwrap_err(),
        TraceError::RuntimeMetricPublicationRequired
    );

    let mut scheduler = ModifyScheduler::default();
    scheduler
        .submit(
            modify_command(arm_context, &catalog, &registry),
            arm_context,
            &catalog,
            &registry,
        )
        .unwrap();
    let plan = scheduler
        .next_due(arm_context, &catalog, &registry)
        .unwrap()
        .unwrap();
    let execution =
        publish_modify_plan(&mut universe, binding, &mut scheduler, &registry, &plan).unwrap();
    let RuntimePublicationReceipt::Scan(scan) = &execution.publication else {
        panic!("scan-end modify must return a commissioned scan receipt");
    };
    let published_context = ObservationContext::from_virtual_universe(
        &universe,
        universe.session_command_binding(SESSION).unwrap(),
        PublicationBoundary::ScanEnd,
    )
    .unwrap();
    let runtime_publication =
        TraceRuntimePublication::from_commissioned_scan(published_context, scan).unwrap();
    let RunOutcome::Completed(report) = &scan.runtime.outcome else {
        panic!("test artifact must complete its scan");
    };
    let captures = trace
        .publish_with_runtime(published_context, &[], &[], &runtime_publication)
        .unwrap();
    let capture = trace.capture(captures[0]).unwrap();
    assert_eq!(capture.samples.len(), 1);
    assert_eq!(
        capture.samples[0].channel_values[0].probe_identity,
        TraceProbeIdentity::ScanQuantumMs
    );
    assert_eq!(
        capture.samples[0].channel_values[0].value,
        Some(CanonicalValue::TimeMs(
            i64::try_from(SCAN_QUANTUM_MS).expect("scan quantum fits signed TIME"),
        ))
    );
    assert_eq!(
        capture.samples[0].channel_values[1].probe_identity,
        TraceProbeIdentity::ScanWorkUnits
    );
    assert_eq!(
        capture.samples[0].channel_values[1].value,
        Some(CanonicalValue::U32(report.work_units))
    );
    assert!(capture.verify());
}

#[test]
fn runtime_diagnostic_bridge_ingests_real_provider_events_exactly_once() {
    let (mut universe, catalog) = setup();
    let registry = ForceRegistry::new();
    let rejected_binding = universe.session_command_binding(SESSION).unwrap();
    assert!(universe.request_stop(rejected_binding).is_err());
    let provider_cpu_event = universe
        .controller(CONTROLLER)
        .unwrap()
        .runtime()
        .diagnostics()[0]
        .clone();

    let mut ledger = DiagnosticLedger::new(
        DiagnosticRegistry::edu21_runtime(),
        DiagnosticLimits::edu21(),
    )
    .unwrap();
    let mut bridge = RuntimeDiagnosticBridge::default();
    let first = bridge
        .ingest_from_virtual_universe(
            &mut ledger,
            &universe,
            universe.session_command_binding(SESSION).unwrap(),
        )
        .unwrap();
    assert_eq!(first.len(), 1);
    assert!(!first[0].duplicate);
    assert_eq!(
        first[0].provider_key.occurrence_id,
        provider_cpu_event.occurrence_id
    );
    assert_eq!(
        ledger
            .registry()
            .definition(ledger.retained_events().last().unwrap().definition_id)
            .unwrap()
            .code
            .0,
        "EDU-CPU-0002"
    );
    let ledger_hash_after_first = ledger.ledger_hash();
    let duplicate = bridge
        .ingest_from_virtual_universe(
            &mut ledger,
            &universe,
            universe.session_command_binding(SESSION).unwrap(),
        )
        .unwrap();
    assert_eq!(duplicate.len(), 1);
    assert!(duplicate[0].duplicate);
    assert_eq!(ledger.ledger_hash(), ledger_hash_after_first);

    let mut scheduler = ModifyScheduler::default();
    let binding = universe.session_command_binding(SESSION).unwrap();
    let stop_context = ObservationContext::from_virtual_universe(
        &universe,
        binding,
        PublicationBoundary::SerializedCommand,
    )
    .unwrap();
    let mut overflow = modify_command(stop_context, &catalog, &registry);
    overflow.command_id = 0x8b00;
    overflow.idempotency_key = 0x8b01;
    overflow.items[0].value = CanonicalValue::I32(i32::MAX);
    scheduler
        .submit(overflow, stop_context, &catalog, &registry)
        .unwrap();
    let plan = scheduler
        .next_due(stop_context, &catalog, &registry)
        .unwrap()
        .unwrap();
    publish_modify_plan(&mut universe, binding, &mut scheduler, &registry, &plan).unwrap();
    let run_binding = universe.session_command_binding(SESSION).unwrap();
    universe
        .request_run(run_binding, RestartKind::WarmRestart)
        .unwrap();
    let RunOutcome::Faulted(provider_fault) = universe
        .run_scan(universe.session_command_binding(SESSION).unwrap())
        .unwrap()
    else {
        panic!("overflowing AddI32 must fault the authoritative runtime");
    };

    let receipts = bridge
        .ingest_from_virtual_universe(
            &mut ledger,
            &universe,
            universe.session_command_binding(SESSION).unwrap(),
        )
        .unwrap();
    let fault_receipt = receipts.iter().find(|receipt| !receipt.duplicate).unwrap();
    assert_eq!(
        fault_receipt.provider_key.occurrence_id,
        provider_fault.occurrence_id
    );
    assert!(fault_receipt.verify());
    let fault_event = ledger
        .retained_events()
        .into_iter()
        .find(|event| event.occurrence_id == fault_receipt.ledger_occurrence_id)
        .unwrap();
    assert_eq!(
        fault_event.payload_hash,
        fault_receipt.provider_payload_hash
    );
    assert_eq!(fault_event.severity, DiagnosticSeverity::Fatal);
    assert_eq!(
        ledger
            .registry()
            .definition(fault_event.definition_id)
            .unwrap()
            .code
            .0,
        "EDU-RTM-0001"
    );
    assert_eq!(ledger.active_conditions().len(), 1);
    assert_eq!(bridge.receipts().len(), 2);
    assert_eq!(bridge.replay_hash().unwrap(), bridge.bridge_hash());
}

#[test]
fn force_execution_is_atomic_and_duplicate_receipt_does_not_publish_again() {
    let (mut universe, catalog) = setup();
    let mut registry = ForceRegistry::new();
    let scheduler = ModifyScheduler::default();
    let binding = universe.session_command_binding(SESSION).unwrap();
    let context = ObservationContext::from_virtual_universe(
        &universe,
        binding,
        PublicationBoundary::SerializedCommand,
    )
    .unwrap();
    let command = force_command(context, &registry);
    let first = execute_force_command(
        &mut universe,
        binding,
        &mut registry,
        &scheduler,
        &catalog,
        &command,
    )
    .unwrap();
    assert!(first.publication.is_some());
    let RuntimePublicationReceipt::Serialized(publication) = first.publication.as_ref().unwrap()
    else {
        panic!("STOP force must publish at a serialized command boundary");
    };
    assert_eq!(
        first.force.first_affected_event_sequence,
        publication.runtime.event_sequence
    );
    assert_eq!(
        first.force.first_affected_scan_sequence,
        publication.runtime.scan_sequence
    );
    assert_eq!(
        first.force.first_affected_virtual_timestamp_ms,
        publication.runtime.virtual_timestamp_ms
    );
    assert_eq!(registry.audit_records().len(), 1);
    assert!(registry.verify_audit_stream());
    assert!(registry.global_projection().verify());
    assert_eq!(registry.active_ids(), vec![ForceId(1)]);
    assert_eq!(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .force_registry_hash(),
        registry.registry_hash()
    );
    assert_eq!(
        universe
            .controller(CONTROLLER)
            .unwrap()
            .runtime()
            .effective_value(plc_runtime::RuntimeValueTarget::Memory(MEMORY)),
        Some(CanonicalValue::I32(33))
    );

    let before = universe.semantic_state_hash();
    let fresh_binding = universe.session_command_binding(SESSION).unwrap();
    let duplicate = execute_force_command(
        &mut universe,
        fresh_binding,
        &mut registry,
        &scheduler,
        &catalog,
        &command,
    )
    .unwrap();
    assert!(duplicate.force.duplicate);
    assert!(duplicate.publication.is_none());
    assert_eq!(universe.semantic_state_hash(), before);
}

#[test]
fn a_new_force_conflicting_with_a_queued_modify_rolls_back_both_models() {
    let (mut universe, catalog) = setup();
    let mut registry = ForceRegistry::new();
    let binding = universe.session_command_binding(SESSION).unwrap();
    let context = ObservationContext::from_virtual_universe(
        &universe,
        binding,
        PublicationBoundary::SerializedCommand,
    )
    .unwrap();
    let mut scheduler = ModifyScheduler::default();
    scheduler
        .submit(
            modify_command(context, &catalog, &registry),
            context,
            &catalog,
            &registry,
        )
        .unwrap();
    let command = force_command(context, &registry);
    let universe_before = universe.semantic_state_hash();
    let registry_before = registry.registry_hash();

    assert_eq!(
        execute_force_command(
            &mut universe,
            binding,
            &mut registry,
            &scheduler,
            &catalog,
            &command,
        )
        .unwrap_err(),
        ExecutionError::PendingModifyConflict(RuntimeTarget::Memory(MEMORY))
    );
    assert_eq!(universe.semantic_state_hash(), universe_before);
    assert_eq!(registry.registry_hash(), registry_before);
    assert_eq!(scheduler.pending_count(), 1);
}
