use plc_commissioning::{UniverseId, VirtualControllerId, VirtualOnlineSessionId};
use plc_observability::{
    AccessCapabilities, BitRange, ObservationContext, ProbeCatalog, ProbeDefinition, ProbeLayer,
    PublicationBoundary, PublishedTargetValue, Quality, RuntimeTarget, StableTargetId,
    TargetReference, TraceCadence, TraceChannel, TraceChannelId, TraceConfig, TraceConfigId,
    TraceEngine, TraceExportFormat, TraceExportOptions, TraceLimits, TraceProbeKind, TraceTrigger,
    TraceTriggerId, ValueType,
};
use plc_runtime::{CanonicalValue, CpuState, Hash32, MemoryId, Sha256};

#[path = "../../../tests/support/isolation_fuzz.rs"]
mod isolation_fuzz;

fn hash(label: &str) -> Hash32 {
    Sha256::digest(label.as_bytes())
}

fn context() -> ObservationContext {
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
        cpu_state: CpuState::Run,
        virtual_timestamp_ms: 10,
        scan_sequence: 1,
        event_sequence: 1,
        publication_boundary: PublicationBoundary::ScanEnd,
    }
}

fn catalog() -> ProbeCatalog {
    let mut catalog = ProbeCatalog::new(hash("artifact"), hash("profile"));
    catalog
        .insert(ProbeDefinition {
            id: StableTargetId(20),
            runtime_target: RuntimeTarget::Memory(MemoryId(2)),
            bit_range: BitRange::whole_value(),
            value_type: ValueType::Bool,
            instance_path: vec![200],
            capabilities: AccessCapabilities {
                monitor: true,
                modify: true,
                force: true,
                trace: true,
                natural_layer: true,
                effective_layer: true,
            },
            primary_source: None,
            display_name: "Enable".into(),
        })
        .expect("probe");
    catalog
}

fn value() -> PublishedTargetValue {
    PublishedTargetValue {
        target_id: StableTargetId(20),
        value_type: ValueType::Bool,
        natural_value: CanonicalValue::Bool(true),
        effective_value: CanonicalValue::Bool(true),
        raw_input_value: Some(CanonicalValue::Bool(true)),
        committed_output_value: None,
        delivered_output_value: None,
        quality: Quality::Good,
        force: None,
    }
}

#[test]
fn trace_exports_route_the_complete_corpus_without_emitting_endpoint_or_deployable_text() {
    for (ordinal, case) in isolation_fuzz::cases().iter().enumerate() {
        let fuzz = case.value.as_str();
        let mut trace = TraceEngine::new(TraceLimits {
            configurations_per_project: 1,
            channels_per_configuration: 1,
            samples_per_capture: 4,
            concurrent_captures_per_controller: 1,
            minimum_virtual_cadence_ms: 1,
            maximum_virtual_duration_ms: 100,
            trigger_depth: 2,
            trigger_nodes: 4,
        })
        .expect("trace engine");
        let trace_id = TraceConfigId(ordinal as u128 + 1);
        let inserted = trace.upsert_config(TraceConfig {
            id: trace_id,
            trigger_id: TraceTriggerId(ordinal as u128 + 100),
            name: fuzz.into(),
            channels: vec![TraceChannel {
                id: TraceChannelId(1),
                alias: fuzz.into(),
                probe: TraceProbeKind::LoadedTarget {
                    target: TargetReference::Stable(StableTargetId(20)),
                    layer: ProbeLayer::Effective,
                },
                display_unit: Some(fuzz.into()),
            }],
            cadence: TraceCadence::EveryScans(1),
            trigger: TraceTrigger::Immediate,
            pre_trigger_samples: 0,
            post_trigger_samples: 0,
            post_trigger_duration_ms: None,
            maximum_duration_ms: 100,
        });
        if case.id == "lone-surrogate" {
            assert!(
                inserted.is_err(),
                "replacement-character input must fail closed"
            );
            continue;
        }
        inserted.unwrap_or_else(|error| {
            panic!(
                "trace config {} rejected before typed routing: {error}",
                case.id
            )
        });
        trace.arm(trace_id, context(), &catalog()).expect("arm");
        let captures = trace.publish(context(), &[value()], &[]).expect("publish");
        assert_eq!(captures.len(), 1, "corpus case {}", case.id);
        let capture = trace.capture(captures[0]).expect("capture");

        for (format, media_type) in [
            (TraceExportFormat::CanonicalJson, "application/json"),
            (TraceExportFormat::Csv, "text/csv"),
        ] {
            let exported = capture
                .export(TraceExportOptions { format })
                .expect("export");
            assert_eq!(exported.media_type, media_type);
            assert!(exported.content_hash != Hash32::ZERO);
            assert!(
                !exported
                    .bytes
                    .windows(fuzz.len())
                    .any(|window| window == fuzz.as_bytes()),
                "corpus case {} escaped the closed trace export",
                case.id
            );
        }
    }
}
