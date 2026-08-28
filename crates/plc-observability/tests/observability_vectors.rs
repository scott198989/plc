use plc_commissioning::*;
use plc_observability::*;
use plc_runtime::{
    ArtifactSpec, BlockId, Instruction, MemoryDefinition, Operation, ProgramBlock, ProgramImage,
    Sha256,
};

fn hash(label: &str) -> Hash32 {
    Sha256::digest(label.as_bytes())
}

fn context(
    cpu_state: CpuState,
    boundary: PublicationBoundary,
    scan_sequence: u64,
    virtual_timestamp_ms: u64,
) -> ObservationContext {
    ObservationContext {
        universe_id: UniverseId(1),
        universe_epoch: 1,
        controller_id: VirtualControllerId(2),
        controller_epoch: 3,
        session_id: VirtualOnlineSessionId(4),
        session_epoch: 5,
        package_fingerprint: hash("package"),
        artifact_fingerprint: hash("artifact"),
        profile_fingerprint: hash("profile"),
        target_state_hash: hash("state"),
        cpu_state,
        virtual_timestamp_ms,
        scan_sequence,
        event_sequence: scan_sequence,
        publication_boundary: boundary,
    }
}

fn capabilities() -> AccessCapabilities {
    AccessCapabilities {
        monitor: true,
        modify: true,
        force: true,
        trace: true,
        natural_layer: true,
        effective_layer: true,
    }
}

fn catalog() -> ProbeCatalog {
    let mut catalog = ProbeCatalog::new(hash("artifact"), hash("profile"));
    catalog
        .insert(ProbeDefinition {
            id: StableTargetId(10),
            runtime_target: RuntimeTarget::Memory(MemoryId(1)),
            bit_range: BitRange::whole_value(),
            value_type: ValueType::I32,
            instance_path: vec![100],
            capabilities: capabilities(),
            primary_source: Some(SourceAnchor {
                artifact_fingerprint: hash("artifact"),
                file_identity: 1,
                semantic_identity: 10,
                start_utf16: 0,
                end_utf16: 4,
            }),
            display_name: "Counter".into(),
        })
        .unwrap();
    catalog
        .insert(ProbeDefinition {
            id: StableTargetId(20),
            runtime_target: RuntimeTarget::Memory(MemoryId(2)),
            bit_range: BitRange::whole_value(),
            value_type: ValueType::Bool,
            instance_path: vec![200],
            capabilities: capabilities(),
            primary_source: None,
            display_name: "Enable".into(),
        })
        .unwrap();
    catalog
        .insert(ProbeDefinition {
            id: StableTargetId(30),
            runtime_target: RuntimeTarget::Output(ChannelId(1)),
            bit_range: BitRange::whole_value(),
            value_type: ValueType::I32,
            instance_path: vec![300],
            capabilities: capabilities(),
            primary_source: None,
            display_name: "Output".into(),
        })
        .unwrap();
    catalog
        .insert(ProbeDefinition {
            id: StableTargetId(40),
            runtime_target: RuntimeTarget::Input(ChannelId(2)),
            bit_range: BitRange::whole_value(),
            value_type: ValueType::Bool,
            instance_path: vec![400],
            capabilities: capabilities(),
            primary_source: None,
            display_name: "Raw input".into(),
        })
        .unwrap();
    catalog
}

fn value_i32(value: i32) -> PublishedTargetValue {
    PublishedTargetValue {
        target_id: StableTargetId(10),
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

fn value_bool(value: bool) -> PublishedTargetValue {
    PublishedTargetValue {
        target_id: StableTargetId(20),
        value_type: ValueType::Bool,
        natural_value: CanonicalValue::Bool(value),
        effective_value: CanonicalValue::Bool(value),
        raw_input_value: Some(CanonicalValue::Bool(value)),
        committed_output_value: None,
        delivered_output_value: None,
        quality: Quality::Good,
        force: None,
    }
}

fn value_input_bool(value: bool) -> PublishedTargetValue {
    let mut published = value_bool(value);
    published.target_id = StableTargetId(40);
    published
}

#[test]
fn watch_persistence_is_config_only_and_sampling_is_bounded() {
    let catalog = catalog();
    let mut engine = MonitoringEngine::new(MonitoringLimits {
        tables_per_project: 2,
        rows_per_table: 4,
        active_subscriptions_per_controller: 4,
        retained_samples_per_row: 2,
    })
    .unwrap();
    engine
        .upsert_table(WatchTable {
            id: WatchTableId(1),
            name: "Main".into(),
            rows: vec![
                WatchRow {
                    id: WatchRowId(2),
                    target: TargetReference::SourceOnly(SourceAnchor {
                        artifact_fingerprint: hash("artifact"),
                        file_identity: 1,
                        semantic_identity: 99,
                        start_utf16: 0,
                        end_utf16: 1,
                    }),
                    layer: ProbeLayer::Natural,
                    display_base: DisplayBase::Automatic,
                    unit: None,
                    format: None,
                    note: None,
                    order: 2,
                },
                WatchRow {
                    id: WatchRowId(1),
                    target: TargetReference::Stable(StableTargetId(10)),
                    layer: ProbeLayer::Effective,
                    display_base: DisplayBase::Decimal,
                    unit: Some("count".into()),
                    format: None,
                    note: Some("bounded history".into()),
                    order: 1,
                },
                WatchRow {
                    id: WatchRowId(3),
                    target: TargetReference::Stable(StableTargetId(40)),
                    layer: ProbeLayer::RawInput,
                    display_base: DisplayBase::Automatic,
                    unit: None,
                    format: None,
                    note: None,
                    order: 3,
                },
            ],
        })
        .unwrap();
    let persisted_before = engine.persistence().unwrap();
    assert!(persisted_before.verify());
    assert_eq!(persisted_before.tables[0].rows[0].id, WatchRowId(1));

    engine
        .start(
            context(CpuState::Run, PublicationBoundary::ScanEnd, 1, 10),
            &catalog,
        )
        .unwrap();
    assert_eq!(engine.state(), MonitorState::Degraded);
    assert_eq!(
        engine.failure(WatchRowId(2)),
        Some(&MonitorFailure::SourceOnlyTarget)
    );
    for scan in 1..=3 {
        engine
            .publish(
                context(CpuState::Run, PublicationBoundary::ScanEnd, scan, scan * 10),
                &[
                    value_i32(scan as i32),
                    value_bool(scan.is_multiple_of(2)),
                    value_input_bool(scan.is_multiple_of(2)),
                ],
            )
            .unwrap();
    }
    assert_eq!(engine.history(WatchRowId(1)).unwrap().len(), 2);
    assert_eq!(
        engine.latest(WatchRowId(1)).unwrap().value,
        CanonicalValue::I32(3)
    );
    engine.mark_stale();
    assert_eq!(
        engine.latest(WatchRowId(1)).unwrap().freshness,
        SampleFreshness::Stale
    );
    assert_eq!(
        engine.latest(WatchRowId(3)).unwrap().layer,
        ProbeLayer::RawInput
    );
    engine.stop().unwrap();
    assert_eq!(
        engine.latest(WatchRowId(1)).unwrap().freshness,
        SampleFreshness::Stale
    );
    let persisted_after = engine.persistence().unwrap();
    assert_eq!(persisted_before, persisted_after);
}

fn force_create_command(
    registry: &ForceRegistry,
    ctx: ObservationContext,
    id: u128,
) -> ForceCommand {
    ForceCommand {
        command_id: id,
        idempotency_key: id + 1_000,
        expected_universe_epoch: ctx.universe_epoch,
        expected_controller_epoch: ctx.controller_epoch,
        expected_session_epoch: ctx.session_epoch,
        expected_artifact_fingerprint: ctx.artifact_fingerprint,
        expected_target_state_hash: ctx.target_state_hash,
        expected_registry_version: registry.version(),
        expected_registry_hash: registry.registry_hash(),
        audit_context_hash: hash("force-audit"),
        kind: ForceCommandKind::Create {
            force_id: ForceId(id),
            target: TargetReference::Stable(StableTargetId(10)),
            value: CanonicalValue::I32(99),
            natural_at_application: CanonicalValue::I32(7),
            actor_identity: 42,
            reason: "commissioning check".into(),
        },
    }
}

#[test]
fn force_registry_is_cas_overlap_aware_and_snapshot_rebindable() {
    let catalog = catalog();
    let ctx = context(CpuState::Stop, PublicationBoundary::SerializedCommand, 0, 0);
    let mut registry = ForceRegistry::new();
    let command = force_create_command(&registry, ctx, 1);
    let (receipt, plan) = registry.apply_at_boundary(&command, ctx, &catalog).unwrap();
    assert!(!receipt.duplicate);
    assert_eq!(plan.set_values.len(), 1);
    assert_eq!(
        registry.effective_value(RuntimeTarget::Memory(MemoryId(1)), CanonicalValue::I32(8)),
        (CanonicalValue::I32(99), Some(ForceId(1)))
    );
    let duplicate = registry
        .apply_at_boundary(&command, ctx, &catalog)
        .unwrap()
        .0;
    assert!(duplicate.duplicate);
    assert_eq!(registry.audit_records().len(), 1);
    let create_audit = registry.audit_records().next().unwrap();
    assert_eq!(create_audit.action, ForceAuditAction::Create);
    assert_eq!(create_audit.requester_identity, 42);
    assert_eq!(create_audit.reason, "commissioning check");
    assert_eq!(
        create_audit.after.as_ref().unwrap().typed_value,
        CanonicalValue::I32(99)
    );
    assert!(create_audit.verify());

    let before = registry.registry_hash();
    let overlap = force_create_command(&registry, ctx, 2);
    assert_eq!(
        registry.apply_at_boundary(&overlap, ctx, &catalog),
        Err(ForceError::TargetOverlap)
    );
    assert_eq!(registry.registry_hash(), before);
    assert_eq!(registry.audit_records().len(), 1);

    let replace = ForceCommand {
        command_id: 10,
        idempotency_key: 1_010,
        expected_universe_epoch: ctx.universe_epoch,
        expected_controller_epoch: ctx.controller_epoch,
        expected_session_epoch: ctx.session_epoch,
        expected_artifact_fingerprint: ctx.artifact_fingerprint,
        expected_target_state_hash: ctx.target_state_hash,
        expected_registry_version: registry.version(),
        expected_registry_hash: registry.registry_hash(),
        audit_context_hash: hash("replace-audit"),
        kind: ForceCommandKind::Replace {
            force_id: ForceId(1),
            expected_entry_hash: registry.entry(ForceId(1)).unwrap().entry_hash,
            value: CanonicalValue::I32(88),
            actor_identity: 43,
            reason: "adjust commissioning value".into(),
        },
    };
    let (replace_receipt, _) = registry.apply_at_boundary(&replace, ctx, &catalog).unwrap();
    assert_eq!(replace_receipt.audit_record_hashes.len(), 1);
    assert_eq!(
        replace_receipt.affected_targets[0].typed_value,
        CanonicalValue::I32(88)
    );
    assert_eq!(registry.audit_records().len(), 2);
    let replace_audit = registry.audit_records().last().unwrap();
    assert_eq!(replace_audit.action, ForceAuditAction::Replace);
    assert_eq!(
        replace_audit.before.as_ref().unwrap().typed_value,
        CanonicalValue::I32(99)
    );
    assert_eq!(
        replace_audit.after.as_ref().unwrap().typed_value,
        CanonicalValue::I32(88)
    );
    assert!(registry.verify_audit_stream());

    let projection = registry.global_projection();
    assert!(projection.verify());
    assert_eq!(projection.controller_ids, vec![ctx.controller_id.0]);
    assert_eq!(projection.audit_record_count, 2);
    assert_eq!(projection.entries[0].force.value, CanonicalValue::I32(88));
    assert_eq!(
        projection.entries[0].navigation_identity,
        StableTargetId(10)
    );

    let snapshot = registry.snapshot(ctx);
    let mut rebound_context = ctx;
    rebound_context.controller_epoch += 1;
    rebound_context.session_epoch += 1;
    let (rebound, reapply) =
        ForceRegistry::rebind_snapshot(&snapshot, rebound_context, &catalog).unwrap();
    assert_eq!(rebound.active_ids(), vec![ForceId(1)]);
    assert_eq!(reapply.set_values.len(), 1);
    let rebound_entry = rebound.entry(ForceId(1)).unwrap();
    assert_eq!(rebound_entry.created_universe_epoch, ctx.universe_epoch);
    assert_eq!(rebound_entry.created_controller_epoch, ctx.controller_epoch);
    assert_eq!(
        rebound_entry.bound_universe_epoch,
        rebound_context.universe_epoch
    );
    assert_eq!(
        rebound_entry.bound_controller_epoch,
        rebound_context.controller_epoch
    );
    let summary = rebound.active_summary();
    assert_eq!(summary.count, 1);
    assert_eq!(summary.registry_hash, rebound.registry_hash());
    assert_eq!(summary.entries[0].navigation_identity, StableTargetId(10));
    assert_eq!(rebound.audit_records().len(), 2);
    assert_eq!(rebound.audit_head_hash(), registry.audit_head_hash());
    assert!(rebound.verify_audit_stream());

    let explicit_remove = ForceCommand {
        command_id: 40,
        idempotency_key: 41,
        expected_universe_epoch: ctx.universe_epoch,
        expected_controller_epoch: ctx.controller_epoch,
        expected_session_epoch: ctx.session_epoch,
        expected_artifact_fingerprint: ctx.artifact_fingerprint,
        expected_target_state_hash: ctx.target_state_hash,
        expected_registry_version: registry.version(),
        expected_registry_hash: registry.registry_hash(),
        audit_context_hash: hash("explicit-remove-audit"),
        kind: ForceCommandKind::Remove {
            force_id: ForceId(1),
            expected_entry_hash: registry.entry(ForceId(1)).unwrap().entry_hash,
            actor_identity: 44,
            reason: "remove individual force".into(),
        },
    };
    let (remove_receipt, _) = registry
        .apply_at_boundary(&explicit_remove, ctx, &catalog)
        .unwrap();
    assert_eq!(remove_receipt.affected_force_ids, vec![ForceId(1)]);
    assert_eq!(
        remove_receipt.affected_targets[0].target_id,
        StableTargetId(10)
    );
    assert_eq!(
        registry.audit_records().last().unwrap().action,
        ForceAuditAction::Remove
    );

    let second_create = force_create_command(&registry, ctx, 2);
    registry
        .apply_at_boundary(&second_create, ctx, &catalog)
        .unwrap();

    let preview = registry.preview_remove_all(ctx);
    let remove = ForceCommand {
        command_id: 50,
        idempotency_key: 51,
        expected_universe_epoch: ctx.universe_epoch,
        expected_controller_epoch: ctx.controller_epoch,
        expected_session_epoch: ctx.session_epoch,
        expected_artifact_fingerprint: ctx.artifact_fingerprint,
        expected_target_state_hash: ctx.target_state_hash,
        expected_registry_version: registry.version(),
        expected_registry_hash: registry.registry_hash(),
        audit_context_hash: hash("remove-audit"),
        kind: ForceCommandKind::RemoveAll {
            approval: RemoveAllApproval::approve(&preview),
            actor_identity: 42,
            reason: "end commissioning".into(),
        },
    };
    let (remove_all_receipt, remove_plan) =
        registry.apply_at_boundary(&remove, ctx, &catalog).unwrap();
    assert!(registry.active_ids().is_empty());
    assert_eq!(remove_plan.remove_targets.len(), 1);
    assert_eq!(remove_all_receipt.affected_force_ids, vec![ForceId(2)]);
    assert_eq!(remove_all_receipt.affected_targets.len(), 1);
    assert_eq!(
        registry.audit_records().last().unwrap().action,
        ForceAuditAction::RemoveAll
    );
    assert_eq!(registry.audit_records().len(), 5);
    assert!(registry.verify_audit_stream());
    let empty_projection = registry.global_projection();
    assert!(empty_projection.verify());
    assert_eq!(empty_projection.count, 0);
    assert_eq!(empty_projection.audit_record_count, 5);
}

fn modify_command(
    scheduler_id: u128,
    ctx: ObservationContext,
    catalog: &ProbeCatalog,
    forces: &ForceRegistry,
    value: CanonicalValue,
    allow_overwrite: bool,
) -> ModifyCommand {
    ModifyCommand {
        command_id: scheduler_id,
        idempotency_key: scheduler_id + 500,
        session_id: ctx.session_id,
        controller_id: ctx.controller_id,
        expected_universe_epoch: ctx.universe_epoch,
        expected_controller_epoch: ctx.controller_epoch,
        expected_session_epoch: ctx.session_epoch,
        expected_artifact_fingerprint: ctx.artifact_fingerprint,
        expected_target_state_hash: ctx.target_state_hash,
        expected_probe_catalog_hash: catalog.catalog_hash(),
        expected_force_registry_version: forces.version(),
        expected_force_registry_hash: forces.registry_hash(),
        allow_overwrite_queued: allow_overwrite,
        requested_boundary: ctx.publication_boundary,
        author_identity: 77,
        audit_context_hash: hash("modify-audit"),
        items: vec![ModifyItem {
            target: TargetReference::Stable(StableTargetId(10)),
            expected_instance_path: vec![100],
            expected_value_type: ValueType::I32,
            value,
        }],
    }
}

#[test]
fn modify_is_exactly_once_aggregate_atomic_and_overwrite_explicit() {
    let catalog = catalog();
    let forces = ForceRegistry::new();
    let ctx = context(CpuState::Run, PublicationBoundary::ScanEnd, 2, 20);
    let mut scheduler = ModifyScheduler::default();

    let mut invalid = modify_command(1, ctx, &catalog, &forces, CanonicalValue::I32(7), false);
    invalid.items.push(ModifyItem {
        target: TargetReference::Stable(StableTargetId(20)),
        expected_instance_path: vec![200],
        expected_value_type: ValueType::Bool,
        value: CanonicalValue::I32(1),
    });
    assert!(matches!(
        scheduler.submit(invalid, ctx, &catalog, &forces),
        Err(ModifyError::TypeMismatch(StableTargetId(20)))
    ));
    assert_eq!(scheduler.pending_count(), 0);

    let mut mixed = modify_command(9, ctx, &catalog, &forces, CanonicalValue::I32(7), false);
    mixed.items.push(ModifyItem {
        target: TargetReference::Stable(StableTargetId(30)),
        expected_instance_path: vec![300],
        expected_value_type: ValueType::I32,
        value: CanonicalValue::I32(2),
    });
    assert_eq!(
        scheduler.submit_with_io_state(
            mixed,
            ctx,
            &catalog,
            &forces,
            &[RuntimeIoState {
                target_id: StableTargetId(30),
                runtime_present: true,
                quality: Quality::Good,
            }],
        ),
        Err(ModifyError::MixedApplicationStage)
    );
    assert_eq!(scheduler.pending_count(), 0);

    let first = modify_command(2, ctx, &catalog, &forces, CanonicalValue::I32(8), false);
    let receipt = scheduler
        .submit(first.clone(), ctx, &catalog, &forces)
        .unwrap();
    assert_eq!(receipt.state, ModifyReceiptState::Queued);
    assert!(
        scheduler
            .submit(first, ctx, &catalog, &forces)
            .unwrap()
            .duplicate
    );
    let replacement = modify_command(3, ctx, &catalog, &forces, CanonicalValue::I32(9), true);
    scheduler
        .submit(replacement, ctx, &catalog, &forces)
        .unwrap();
    assert_eq!(
        scheduler.receipt_by_idempotency_key(502).unwrap().state,
        ModifyReceiptState::Canceled
    );
    let plan = scheduler.next_due(ctx, &catalog, &forces).unwrap().unwrap();
    assert_eq!(plan.writes[0].value, CanonicalValue::I32(9));
    let applied = scheduler.commit(&plan, hash("applied"), 77).unwrap();
    assert_eq!(applied.state, ModifyReceiptState::Applied);
    assert_eq!(scheduler.pending_count(), 0);
}

#[test]
fn force_conflict_rejects_modify_before_queueing() {
    let catalog = catalog();
    let ctx = context(CpuState::Stop, PublicationBoundary::SerializedCommand, 0, 0);
    let mut forces = ForceRegistry::new();
    let force = force_create_command(&forces, ctx, 10);
    forces.apply_at_boundary(&force, ctx, &catalog).unwrap();
    let mut scheduler = ModifyScheduler::default();
    let modify = modify_command(11, ctx, &catalog, &forces, CanonicalValue::I32(4), false);
    assert_eq!(
        scheduler.submit(modify, ctx, &catalog, &forces),
        Err(ModifyError::ForceConflict(StableTargetId(10)))
    );
    assert_eq!(scheduler.pending_count(), 0);
}

#[test]
fn io_modify_and_force_require_typed_runtime_quality_evidence() {
    let catalog = catalog();
    let forces = ForceRegistry::new();
    let ctx = context(CpuState::Run, PublicationBoundary::ScanEnd, 2, 20);
    let mut modify = modify_command(
        41,
        ctx,
        &catalog,
        &forces,
        CanonicalValue::Bool(true),
        false,
    );
    modify.items = vec![ModifyItem {
        target: TargetReference::Stable(StableTargetId(40)),
        expected_instance_path: vec![400],
        expected_value_type: ValueType::Bool,
        value: CanonicalValue::Bool(true),
    }];
    let mut scheduler = ModifyScheduler::default();
    assert_eq!(
        scheduler.submit(modify.clone(), ctx, &catalog, &forces),
        Err(ModifyError::QualityUnavailable(StableTargetId(40)))
    );
    assert!(matches!(
        scheduler.submit_with_io_state(
            modify.clone(),
            ctx,
            &catalog,
            &forces,
            &[RuntimeIoState {
                target_id: StableTargetId(40),
                runtime_present: true,
                quality: Quality::Bad,
            }],
        ),
        Err(ModifyError::IoQualityRejected {
            target_id: StableTargetId(40),
            quality: Quality::Bad,
        })
    ));
    scheduler
        .submit_with_io_state(
            modify,
            ctx,
            &catalog,
            &forces,
            &[RuntimeIoState {
                target_id: StableTargetId(40),
                runtime_present: true,
                quality: Quality::Uncertain,
            }],
        )
        .unwrap();

    let mut command = force_create_command(&forces, ctx, 42);
    command.kind = ForceCommandKind::Create {
        force_id: ForceId(42),
        target: TargetReference::Stable(StableTargetId(40)),
        value: CanonicalValue::Bool(true),
        natural_at_application: CanonicalValue::Bool(false),
        actor_identity: 42,
        reason: "bad channel commissioning".into(),
    };
    let mut missing = forces.clone();
    assert_eq!(
        missing.apply_at_boundary(&command, ctx, &catalog),
        Err(ForceError::QualityUnavailable(StableTargetId(40)))
    );
    let mut not_present = forces.clone();
    assert_eq!(
        not_present.apply_at_boundary_with_io_state(
            &command,
            ctx,
            &catalog,
            &[RuntimeIoState {
                target_id: StableTargetId(40),
                runtime_present: false,
                quality: Quality::NotPresent,
            }],
        ),
        Err(ForceError::IoNotPresent(StableTargetId(40)))
    );
    let mut bad = forces.clone();
    bad.apply_at_boundary_with_io_state(
        &command,
        ctx,
        &catalog,
        &[RuntimeIoState {
            target_id: StableTargetId(40),
            runtime_present: true,
            quality: Quality::Bad,
        }],
    )
    .unwrap();
    let entry = bad.entry(ForceId(42)).unwrap();
    assert_eq!(entry.underlying_quality, Quality::Bad);
    assert!(entry.quality_warning);
}

#[test]
fn trace_trigger_pre_post_buffers_and_replay_are_deterministic() {
    let catalog = catalog();
    let mut trace = TraceEngine::new(TraceLimits {
        configurations_per_project: 4,
        channels_per_configuration: 4,
        samples_per_capture: 20,
        concurrent_captures_per_controller: 2,
        minimum_virtual_cadence_ms: 10,
        maximum_virtual_duration_ms: 1_000,
        trigger_depth: 8,
        trigger_nodes: 32,
    })
    .unwrap();
    trace
        .upsert_config(TraceConfig {
            id: TraceConfigId(1),
            trigger_id: TraceTriggerId(101),
            name: "rise".into(),
            channels: vec![TraceChannel {
                id: TraceChannelId(1),
                alias: "input".into(),
                probe: TraceProbeKind::LoadedTarget {
                    target: TargetReference::Stable(StableTargetId(20)),
                    layer: ProbeLayer::Effective,
                },
                display_unit: None,
            }],
            cadence: TraceCadence::EveryScans(1),
            trigger: TraceTrigger::BooleanRising(TraceChannelId(1)),
            pre_trigger_samples: 1,
            post_trigger_samples: 1,
            post_trigger_duration_ms: None,
            maximum_duration_ms: 500,
        })
        .unwrap();
    trace
        .arm(
            TraceConfigId(1),
            context(CpuState::Run, PublicationBoundary::ScanEnd, 1, 10),
            &catalog,
        )
        .unwrap();
    assert!(
        trace
            .publish(
                context(CpuState::Run, PublicationBoundary::ScanEnd, 1, 10),
                &[value_bool(false)],
                &[],
            )
            .unwrap()
            .is_empty()
    );
    assert!(
        trace
            .publish(
                context(CpuState::Run, PublicationBoundary::ScanEnd, 2, 20),
                &[value_bool(true)],
                &[],
            )
            .unwrap()
            .is_empty()
    );
    let completed = trace
        .publish(
            context(CpuState::Run, PublicationBoundary::ScanEnd, 3, 30),
            &[value_bool(true)],
            &[],
        )
        .unwrap();
    assert_eq!(completed.len(), 1);
    assert_eq!(trace.state(TraceConfigId(1)), TraceState::Completed);
    let capture = trace.capture(completed[0]).unwrap();
    assert_eq!(capture.samples.len(), 3);
    assert!(capture.verify());
    assert_eq!(capture.replay_hash(), capture.replay_hash());
    let json = capture
        .export(TraceExportOptions {
            format: TraceExportFormat::CanonicalJson,
        })
        .unwrap();
    let json_again = capture
        .export(TraceExportOptions {
            format: TraceExportFormat::CanonicalJson,
        })
        .unwrap();
    let csv = capture
        .export(TraceExportOptions {
            format: TraceExportFormat::Csv,
        })
        .unwrap();
    assert_eq!(json, json_again);
    assert!(json.bytes.starts_with(b"{\"schemaVersion\":1"));
    assert!(csv.bytes.starts_with(b"schemaVersion,captureId"));
    let captured_content_hash = capture.content_hash;
    let save_context = context(CpuState::Run, PublicationBoundary::ScanEnd, 3, 30);
    let save = SaveTraceResultCommand {
        command_id: 0x700,
        idempotency_key: 0x701,
        result_id: TraceSavedResultId(0x702),
        capture_id: completed[0],
        actor_identity: 0x703,
        audit_context_hash: hash("trace-save"),
    };
    let saved = trace.save_result(save, save_context).unwrap();
    assert!(saved.verify());
    assert_eq!(saved.capture.content_hash, captured_content_hash);
    assert_eq!(trace.save_result(save, save_context).unwrap(), saved);
}

#[test]
fn trace_aborts_on_epoch_change_and_keeps_terminal_capture() {
    let catalog = catalog();
    let mut trace = TraceEngine::new(TraceLimits::edu21()).unwrap();
    trace
        .upsert_config(TraceConfig {
            id: TraceConfigId(9),
            trigger_id: TraceTriggerId(109),
            name: "wait".into(),
            channels: vec![TraceChannel {
                id: TraceChannelId(9),
                alias: "input".into(),
                probe: TraceProbeKind::LoadedTarget {
                    target: TargetReference::Stable(StableTargetId(20)),
                    layer: ProbeLayer::Natural,
                },
                display_unit: None,
            }],
            cadence: TraceCadence::EveryScans(1),
            trigger: TraceTrigger::BooleanFalling(TraceChannelId(9)),
            pre_trigger_samples: 0,
            post_trigger_samples: 1,
            post_trigger_duration_ms: None,
            maximum_duration_ms: 100,
        })
        .unwrap();
    let initial = context(CpuState::Run, PublicationBoundary::ScanEnd, 1, 10);
    trace.arm(TraceConfigId(9), initial, &catalog).unwrap();
    let mut changed = initial;
    changed.controller_epoch += 1;
    let captures = trace.publish(changed, &[value_bool(true)], &[]).unwrap();
    assert_eq!(trace.state(TraceConfigId(9)), TraceState::Aborted);
    assert_eq!(
        trace.capture(captures[0]).unwrap().aborted,
        Some(TraceAbortReason::ControllerEpochChanged)
    );
}

#[test]
fn trace_snapshot_restore_rebinds_epoch_atomically_and_waits_for_run() {
    let catalog = catalog();
    let mut trace = TraceEngine::new(TraceLimits::edu21()).unwrap();
    trace
        .upsert_config(TraceConfig {
            id: TraceConfigId(10),
            trigger_id: TraceTriggerId(110),
            name: "restored".into(),
            channels: vec![TraceChannel {
                id: TraceChannelId(10),
                alias: "input".into(),
                probe: TraceProbeKind::LoadedTarget {
                    target: TargetReference::Stable(StableTargetId(20)),
                    layer: ProbeLayer::Effective,
                },
                display_unit: None,
            }],
            cadence: TraceCadence::EveryScans(1),
            trigger: TraceTrigger::BooleanRising(TraceChannelId(10)),
            pre_trigger_samples: 0,
            post_trigger_samples: 0,
            post_trigger_duration_ms: None,
            maximum_duration_ms: 100,
        })
        .unwrap();
    let initial = context(CpuState::Stop, PublicationBoundary::SerializedCommand, 0, 0);
    trace.arm(TraceConfigId(10), initial, &catalog).unwrap();
    let snapshot = trace.capture_snapshot(initial);
    assert!(snapshot.verify());
    let mut rebound = initial;
    rebound.universe_epoch += 1;
    rebound.controller_epoch += 1;
    rebound.session_epoch += 1;
    rebound.event_sequence = 1;
    let mut restored = TraceEngine::restore_snapshot(
        &snapshot,
        rebound,
        &catalog,
        TraceEventKey {
            universe_epoch: rebound.universe_epoch,
            event_sequence: rebound.event_sequence,
        },
    )
    .unwrap();
    assert_eq!(restored.state(TraceConfigId(10)), TraceState::Armed);
    assert!(
        restored
            .publish(rebound, &[value_bool(true)], &[])
            .unwrap()
            .is_empty()
    );
    rebound.cpu_state = CpuState::Run;
    rebound.publication_boundary = PublicationBoundary::ScanEnd;
    rebound.scan_sequence = 1;
    rebound.event_sequence = 2;
    rebound.virtual_timestamp_ms = 10;
    let completed = restored.publish(rebound, &[value_bool(true)], &[]).unwrap();
    assert_eq!(completed.len(), 1);
    assert!(restored.capture(completed[0]).unwrap().verify());
}

fn condition_key(registry: &DiagnosticRegistry, subject: u128) -> ConditionKey {
    ConditionKey {
        definition_id: registry.by_code("EDU-RTM-0007").unwrap().id,
        subject_identity: subject,
        provider_instance_identity: 900,
        discriminator_hash: hash("counter-limit"),
    }
}

fn activate(key: ConditionKey, transition_id: u128) -> DiagnosticTransition {
    DiagnosticTransition::ActivateCondition {
        transition_id,
        key,
        severity_override: None,
        payload_hash: hash("condition"),
        related_identities: vec![key.subject_identity],
        causal: CausalReference::root(),
        rejectable: true,
    }
}

#[test]
fn diagnostic_trace_binds_version_and_fires_on_first_strictly_later_matching_occurrence() {
    let catalog = catalog();
    let registry = DiagnosticRegistry::edu21_runtime();
    let mut ledger = DiagnosticLedger::new(registry.clone(), DiagnosticLimits::edu21()).unwrap();
    let definition = registry.by_code("EDU-CPU-0002").unwrap();
    let diagnostic_context = context(CpuState::Run, PublicationBoundary::ScanEnd, 0, 0);
    let first = ledger
        .apply_provider_transition(
            DiagnosticTransition::EmitOneShot {
                transition_id: 1,
                definition_id: definition.id,
                severity_override: None,
                payload_hash: hash("first-mode-rejection"),
                related_identities: vec![],
                causal: CausalReference::root(),
            },
            diagnostic_context,
        )
        .unwrap();

    let mut arm_context = diagnostic_context;
    arm_context.event_sequence = first.event_sequence;
    let mut trace = TraceEngine::new(TraceLimits::edu21()).unwrap();
    trace
        .upsert_config(TraceConfig {
            id: TraceConfigId(20),
            trigger_id: TraceTriggerId(120),
            name: "diagnostic".into(),
            channels: vec![TraceChannel {
                id: TraceChannelId(20),
                alias: "input".into(),
                probe: TraceProbeKind::LoadedTarget {
                    target: TargetReference::Stable(StableTargetId(20)),
                    layer: ProbeLayer::Effective,
                },
                display_unit: None,
            }],
            cadence: TraceCadence::EveryScans(100),
            trigger: TraceTrigger::DiagnosticEvent(DiagnosticEventTrigger {
                definition_id: definition.id,
                code_version: definition.code_version,
                lifecycle: DiagnosticEventKind::OneShot,
                primary_target_id: None,
                root_occurrence_id: None,
            }),
            pre_trigger_samples: 0,
            post_trigger_samples: 0,
            post_trigger_duration_ms: None,
            maximum_duration_ms: 100,
        })
        .unwrap();
    trace
        .arm_with_diagnostics(TraceConfigId(20), arm_context, &catalog, &registry)
        .unwrap();

    let second = ledger
        .apply_provider_transition(
            DiagnosticTransition::EmitOneShot {
                transition_id: 2,
                definition_id: definition.id,
                severity_override: None,
                payload_hash: hash("second-mode-rejection"),
                related_identities: vec![],
                causal: CausalReference::root(),
            },
            diagnostic_context,
        )
        .unwrap();
    let event = TraceDiagnosticEvent::from_authoritative(&second, &registry).unwrap();
    let mut publish_context = diagnostic_context;
    publish_context.scan_sequence = 1;
    publish_context.event_sequence = second.event_sequence;
    publish_context.virtual_timestamp_ms = 10;
    let captures = trace
        .publish(publish_context, &[value_bool(true)], &[event])
        .unwrap();
    let capture = trace.capture(captures[0]).unwrap();
    assert_eq!(capture.matched_occurrence_id, Some(second.occurrence_id));
    assert_eq!(capture.trigger_boundary, Some(PublicationBoundary::ScanEnd));
    assert_eq!(
        capture.samples[0].diagnostic_occurrence_ids,
        vec![second.occurrence_id]
    );
}

#[test]
fn diagnostics_have_fixed_registry_uuid_lifecycle_caps_and_one_gap() {
    let registry = DiagnosticRegistry::edu21_runtime();
    assert_eq!(registry.definitions().len(), 26);
    let mut ledger = DiagnosticLedger::new(
        registry.clone(),
        DiagnosticLimits {
            ordinary_nonfatal_active: 2,
            total_active: 3,
            retained_events: 4,
        },
    )
    .unwrap();
    let ctx = context(CpuState::Run, PublicationBoundary::ScanEnd, 1, 10);
    let first = ledger
        .apply_provider_transition(activate(condition_key(&registry, 1), 1), ctx)
        .unwrap();
    let uuid = first.occurrence_id.0.to_be_bytes();
    assert_eq!(uuid[6] >> 4, 5);
    assert_eq!(uuid[8] >> 6, 2);
    ledger
        .apply_provider_transition(activate(condition_key(&registry, 2), 2), ctx)
        .unwrap();
    assert!(matches!(
        ledger.apply_provider_transition(activate(condition_key(&registry, 3), 3), ctx),
        Err(DiagnosticError::CapacityRejected {
            proposed_nonfatal: 3,
            proposed_total: 3,
            ..
        })
    ));
    ledger
        .apply_provider_transition(
            DiagnosticTransition::AcknowledgeCondition {
                transition_id: 4,
                key: condition_key(&registry, 1),
                actor_identity: 88,
                causal: CausalReference {
                    parent_occurrence_id: Some(first.occurrence_id),
                    root_occurrence_id: Some(first.root_occurrence_id),
                },
            },
            context(CpuState::Run, PublicationBoundary::SerializedCommand, 1, 10),
        )
        .unwrap();
    ledger
        .apply_provider_transition(
            DiagnosticTransition::ClearCondition {
                transition_id: 5,
                key: condition_key(&registry, 1),
                payload_hash: hash("clear"),
                causal: CausalReference {
                    parent_occurrence_id: Some(first.occurrence_id),
                    root_occurrence_id: Some(first.root_occurrence_id),
                },
            },
            ctx,
        )
        .unwrap();
    let retained = ledger.retained_events();
    assert!(retained.len() <= 4);
    assert_eq!(
        retained
            .iter()
            .filter(|event| event.kind == DiagnosticEventKind::Compaction)
            .count(),
        1
    );
    let one_shot = registry.by_code("EDU-CPU-0002").unwrap().id;
    let causal = ledger
        .apply_provider_transition(
            DiagnosticTransition::EmitOneShot {
                transition_id: 6,
                definition_id: one_shot,
                severity_override: None,
                payload_hash: hash("mode"),
                related_identities: vec![1],
                causal: CausalReference {
                    parent_occurrence_id: Some(first.occurrence_id),
                    root_occurrence_id: None,
                },
            },
            ctx,
        )
        .unwrap();
    assert_eq!(causal.parent_occurrence_id, Some(first.occurrence_id));
    assert_eq!(causal.parent_resolution, Some(CausalResolution::Compacted));
    assert!(ledger.replay_hash().is_ok());
    let snapshot = ledger.capture_snapshot(ctx);
    assert!(snapshot.verify());
    let retained_hashes = ledger
        .retained_events()
        .into_iter()
        .map(|event| event.event_hash)
        .collect::<Vec<_>>();
    let mut restored_context = ctx;
    restored_context.universe_epoch += 1;
    restored_context.controller_epoch += 1;
    restored_context.event_sequence = 1;
    let restored = DiagnosticLedger::restore_snapshot(&snapshot, restored_context).unwrap();
    assert_eq!(
        restored
            .retained_events()
            .into_iter()
            .map(|event| event.event_hash)
            .collect::<Vec<_>>(),
        retained_hashes
    );
    assert_eq!(
        restored
            .active_conditions()
            .map(|condition| (condition.condition_id, condition.acknowledged))
            .collect::<Vec<_>>(),
        ledger
            .active_conditions()
            .map(|condition| (condition.condition_id, condition.acknowledged))
            .collect::<Vec<_>>()
    );
    assert!(restored.replay_hash().is_ok());

    let mut second = DiagnosticLedger::new(
        registry.clone(),
        DiagnosticLimits {
            ordinary_nonfatal_active: 2,
            total_active: 3,
            retained_events: 4,
        },
    )
    .unwrap();
    assert_eq!(
        second
            .apply_provider_transition(activate(condition_key(&registry, 1), 1), ctx)
            .unwrap()
            .occurrence_id,
        first.occurrence_id
    );
}

#[test]
fn navigation_is_identity_based_side_aware_and_transactional() {
    let offline = hash("offline");
    let loaded = hash("loaded");
    let mut builder = NavigationIndexBuilder::new(1, offline, Some(loaded)).unwrap();
    builder
        .insert_anchor(NavigationAnchor {
            identity: SemanticIdentity(1),
            kind: NavigationKind::SourceSpan,
            side: ArtifactSide::CurrentOffline,
            artifact_fingerprint: offline,
            source: None,
            probe_target: None,
            tombstone_reason_hash: None,
        })
        .unwrap();
    builder
        .insert_anchor(NavigationAnchor {
            identity: SemanticIdentity(1),
            kind: NavigationKind::SourceSpan,
            side: ArtifactSide::Loaded,
            artifact_fingerprint: loaded,
            source: None,
            probe_target: Some(StableTargetId(10)),
            tombstone_reason_hash: None,
        })
        .unwrap();
    builder
        .insert_anchor(NavigationAnchor {
            identity: SemanticIdentity(2),
            kind: NavigationKind::Tombstone,
            side: ArtifactSide::CurrentOffline,
            artifact_fingerprint: offline,
            source: None,
            probe_target: None,
            tombstone_reason_hash: Some(hash("deleted")),
        })
        .unwrap();
    builder
        .relate(SemanticIdentity(1), SemanticIdentity(2))
        .unwrap();
    builder
        .route_diagnostic(77, SemanticIdentity(1), vec![SemanticIdentity(2)])
        .unwrap();
    let index = builder.commit().unwrap();
    assert_eq!(
        index
            .resolve(SemanticIdentity(1), ArtifactSide::Loaded)
            .unwrap()
            .primary
            .artifact_fingerprint,
        loaded
    );
    let diagnostic = index
        .resolve_diagnostic(77, ArtifactSide::CurrentOffline)
        .unwrap();
    assert_eq!(diagnostic.related[0].kind, NavigationKind::Tombstone);
    assert!(matches!(
        index.begin_update(1),
        Err(NavigationError::RevisionNotMonotonic)
    ));
}

fn minimal_load_package() -> VirtualLoadPackage {
    let runtime_artifact = ArtifactPackage::seal_verified(ArtifactSpec::edu21(
        hash("virtual-profile"),
        vec![MemoryDefinition {
            id: MemoryId(1),
            value_type: ValueType::I32,
            loaded_start: CanonicalValue::I32(0),
            retentive: false,
        }],
        vec![],
        vec![],
        ProgramImage {
            startup: None,
            timed: vec![],
            cyclic: ProgramBlock {
                id: BlockId(1),
                instructions: vec![Instruction::new(1, 100, Operation::Noop)],
            },
        },
    ))
    .unwrap();
    VirtualLoadPackage::seal_verified(LoadPackageParts {
        runtime_artifact,
        semantic_build_fingerprint: hash("semantic"),
        verified_ir_fingerprint: hash("ir"),
        schedule_fingerprint: hash("schedule"),
        hardware_fingerprint: hash("hardware"),
        source_map_fingerprint: hash("source"),
        probe_identity_fingerprint: hash("probes"),
        capability_fingerprint: hash("caps"),
        build_snapshot_hash: hash("build"),
        build_is_current: true,
        blocking_diagnostic_count: 0,
        memory_schema: vec![MemoryMemberSchema {
            member_id: 10,
            runtime_memory_id: MemoryId(1),
            value_type: ValueType::I32,
            role: MemoryRole::Marker,
            instance_path: vec![10],
            retentive: false,
            loaded_start: CanonicalValue::I32(0),
        }],
        state_schema: vec![],
    })
    .unwrap()
}

#[test]
fn observation_context_can_only_bind_an_online_virtual_universe_session() {
    let package = minimal_load_package();
    let offline_id = OfflineControllerId(2);
    let controller_id = VirtualControllerId(3);
    let session_id = VirtualOnlineSessionId(4);
    let mut offline = OfflineEngineeringState {
        configured: ConfiguredController {
            id: offline_id,
            configured_hardware_fingerprint: package.hardware_fingerprint(),
            profile_fingerprint: package.profile_fingerprint(),
        },
        source_revision_hash: hash("revision"),
        build_snapshot_hash: None,
        project_saved: true,
        source_to_build: OfflineSourceBuild::Absent,
        software_build_current: false,
        hardware_build_current: false,
        current_package_fingerprint: None,
        built_hardware: None,
    };
    offline.record_build(&package);
    let mut universe = VirtualUniverse::new(UniverseId(1));
    universe.register_offline_controller(offline).unwrap();
    universe
        .create_instance(CreateInstanceCommand {
            command_id: 1,
            instance_id: ControllerInstanceId(5),
            offline_controller_id: offline_id,
            controller_id,
            deterministic_seed: 6,
        })
        .unwrap();
    universe.power_on(controller_id).unwrap();
    let preview = universe
        .prepare_load(
            controller_id,
            &package,
            LoadRequest {
                expected_build_snapshot_hash: package.build_snapshot_hash(),
                requested_post_load_mode: PostLoadMode::Preserve,
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
        .begin_go_online(session_id, offline_id, controller_id)
        .unwrap();
    universe.complete_go_online(session_id).unwrap();
    let binding = universe.session_command_binding(session_id).unwrap();
    let bound = ObservationContext::from_virtual_universe(
        &universe,
        binding,
        PublicationBoundary::SerializedCommand,
    )
    .unwrap();
    assert_eq!(bound.controller_id, controller_id);
    let mut stale = binding;
    stale.expected_session_epoch += 1;
    assert_eq!(
        ObservationContext::from_virtual_universe(
            &universe,
            stale,
            PublicationBoundary::SerializedCommand
        ),
        Err(ContextError::StaleSessionEpoch)
    );
}
