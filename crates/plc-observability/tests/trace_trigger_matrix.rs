use plc_commissioning::*;
use plc_observability::*;
use plc_runtime::{CanonicalValue, Sha256};
use plc_types::CanonicalF32;

const BOOL_TARGET: StableTargetId = StableTargetId(0x10);
const I32_TARGET: StableTargetId = StableTargetId(0x20);
const F32_TARGET: StableTargetId = StableTargetId(0x30);
const BOOL_CHANNEL: TraceChannelId = TraceChannelId(0x100);
const I32_CHANNEL: TraceChannelId = TraceChannelId(0x200);
const F32_CHANNEL: TraceChannelId = TraceChannelId(0x300);

fn hash(label: &str) -> Hash32 {
    Sha256::digest(label.as_bytes())
}

fn context(scan_sequence: u64) -> ObservationContext {
    ObservationContext {
        universe_id: UniverseId(1),
        universe_epoch: 2,
        controller_id: VirtualControllerId(3),
        controller_epoch: 4,
        session_id: VirtualOnlineSessionId(5),
        session_epoch: 6,
        package_fingerprint: hash("package"),
        artifact_fingerprint: hash("artifact"),
        profile_fingerprint: hash("profile"),
        target_state_hash: hash("state"),
        cpu_state: CpuState::Run,
        virtual_timestamp_ms: scan_sequence * 10,
        scan_sequence,
        event_sequence: scan_sequence + 10,
        publication_boundary: PublicationBoundary::ScanEnd,
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
    for (id, memory, value_type, display_name) in [
        (BOOL_TARGET, 1, ValueType::Bool, "Enable"),
        (I32_TARGET, 2, ValueType::I32, "Count"),
        (F32_TARGET, 3, ValueType::F32, "Level"),
    ] {
        catalog
            .insert(ProbeDefinition {
                id,
                runtime_target: RuntimeTarget::Memory(MemoryId(memory)),
                bit_range: BitRange::whole_value(),
                value_type,
                instance_path: vec![u128::from(memory)],
                capabilities: capabilities(),
                primary_source: None,
                display_name: display_name.into(),
            })
            .unwrap();
    }
    catalog
}

fn channel(id: TraceChannelId, target: StableTargetId, alias: &str) -> TraceChannel {
    TraceChannel {
        id,
        alias: alias.into(),
        probe: TraceProbeKind::LoadedTarget {
            target: TargetReference::Stable(target),
            layer: ProbeLayer::Effective,
        },
        display_unit: None,
    }
}

fn value(
    target_id: StableTargetId,
    value_type: ValueType,
    value: CanonicalValue,
) -> PublishedTargetValue {
    PublishedTargetValue {
        target_id,
        value_type,
        natural_value: value,
        effective_value: value,
        raw_input_value: None,
        committed_output_value: None,
        delivered_output_value: None,
        quality: Quality::Good,
        force: None,
    }
}

fn bool_value(value_: bool) -> PublishedTargetValue {
    value(BOOL_TARGET, ValueType::Bool, CanonicalValue::Bool(value_))
}

fn i32_value(value_: i32) -> PublishedTargetValue {
    value(I32_TARGET, ValueType::I32, CanonicalValue::I32(value_))
}

fn f32_value(value_: f32) -> PublishedTargetValue {
    value(
        F32_TARGET,
        ValueType::F32,
        CanonicalValue::F32(CanonicalF32::new(value_)),
    )
}

fn config(id: u128, channels: Vec<TraceChannel>, trigger: TraceTrigger) -> TraceConfig {
    TraceConfig {
        id: TraceConfigId(id),
        trigger_id: TraceTriggerId(id + 0x1_000),
        name: format!("trace-{id}"),
        channels,
        cadence: TraceCadence::EveryScans(1),
        trigger,
        pre_trigger_samples: 0,
        post_trigger_samples: 0,
        post_trigger_duration_ms: None,
        maximum_duration_ms: 1_000,
    }
}

fn complete_after_two_samples(
    config: TraceConfig,
    before: &[PublishedTargetValue],
    after: &[PublishedTargetValue],
) -> TraceCapture {
    let config_id = config.id;
    let mut trace = TraceEngine::new(TraceLimits::edu21()).unwrap();
    trace.upsert_config(config).unwrap();
    trace.arm(config_id, context(0), &catalog()).unwrap();
    assert!(trace.publish(context(1), before, &[]).unwrap().is_empty());
    let completed = trace.publish(context(2), after, &[]).unwrap();
    assert_eq!(completed.len(), 1);
    trace.capture(completed[0]).unwrap().clone()
}

#[test]
fn every_non_diagnostic_trigger_form_has_exact_typed_runtime_semantics() {
    let mut immediate = TraceEngine::new(TraceLimits::edu21()).unwrap();
    let immediate_config = config(
        1,
        vec![channel(BOOL_CHANNEL, BOOL_TARGET, "enable")],
        TraceTrigger::Immediate,
    );
    immediate.upsert_config(immediate_config).unwrap();
    immediate
        .arm(TraceConfigId(1), context(0), &catalog())
        .unwrap();
    assert_eq!(
        immediate
            .publish(context(1), &[bool_value(false)], &[])
            .unwrap()
            .len(),
        1
    );

    let falling = complete_after_two_samples(
        config(
            2,
            vec![channel(BOOL_CHANNEL, BOOL_TARGET, "enable")],
            TraceTrigger::BooleanFalling(BOOL_CHANNEL),
        ),
        &[bool_value(true)],
        &[bool_value(false)],
    );
    assert_eq!(falling.trigger_sample_ordinal, Some(1));

    for (ordinal, operator, before, after) in [
        (0, ComparisonOperator::Equal, 9, 10),
        (1, ComparisonOperator::NotEqual, 10, 9),
        (2, ComparisonOperator::Less, 10, 9),
        (3, ComparisonOperator::LessOrEqual, 11, 10),
        (4, ComparisonOperator::Greater, 10, 11),
        (5, ComparisonOperator::GreaterOrEqual, 9, 10),
    ] {
        let capture = complete_after_two_samples(
            config(
                10 + ordinal,
                vec![channel(I32_CHANNEL, I32_TARGET, "count")],
                TraceTrigger::NumericCrossing {
                    channel: I32_CHANNEL,
                    operator,
                    threshold: NumericValue::I32(10),
                },
            ),
            &[i32_value(before)],
            &[i32_value(after)],
        );
        assert_eq!(capture.trigger_sample_ordinal, Some(1), "{operator:?}");
    }

    let expression = TraceTrigger::Expression(ExpressionNode::All(vec![
        ExpressionNode::BooleanChannel(BOOL_CHANNEL),
        ExpressionNode::Any(vec![
            ExpressionNode::NumericComparison {
                channel: I32_CHANNEL,
                operator: ComparisonOperator::Greater,
                threshold: NumericValue::I32(10),
            },
            ExpressionNode::Not(Box::new(ExpressionNode::BooleanChannel(BOOL_CHANNEL))),
        ]),
    ]));
    let compound = complete_after_two_samples(
        config(
            20,
            vec![
                channel(BOOL_CHANNEL, BOOL_TARGET, "enable"),
                channel(I32_CHANNEL, I32_TARGET, "count"),
            ],
            expression,
        ),
        &[bool_value(false), i32_value(9)],
        &[bool_value(true), i32_value(11)],
    );
    assert_eq!(compound.trigger_sample_ordinal, Some(1));
    assert!(compound.verify());
}

#[test]
fn real_nan_never_satisfies_an_ordered_crossing_and_exact_type_is_enforced() {
    let config_id = TraceConfigId(30);
    let mut trace = TraceEngine::new(TraceLimits::edu21()).unwrap();
    trace
        .upsert_config(config(
            config_id.0,
            vec![channel(F32_CHANNEL, F32_TARGET, "level")],
            TraceTrigger::NumericCrossing {
                channel: F32_CHANNEL,
                operator: ComparisonOperator::Greater,
                threshold: NumericValue::F32(CanonicalF32::new(0.0)),
            },
        ))
        .unwrap();
    trace.arm(config_id, context(0), &catalog()).unwrap();
    assert!(
        trace
            .publish(context(1), &[f32_value(-1.0)], &[])
            .unwrap()
            .is_empty()
    );
    assert!(
        trace
            .publish(context(2), &[f32_value(f32::NAN)], &[])
            .unwrap()
            .is_empty(),
        "NaN cannot satisfy an ordered comparison"
    );
    assert_eq!(
        trace
            .publish(context(3), &[f32_value(1.0)], &[])
            .unwrap()
            .len(),
        1
    );

    let mismatched_id = TraceConfigId(31);
    let mut mismatched = TraceEngine::new(TraceLimits::edu21()).unwrap();
    mismatched
        .upsert_config(config(
            mismatched_id.0,
            vec![channel(I32_CHANNEL, I32_TARGET, "count")],
            TraceTrigger::NumericCrossing {
                channel: I32_CHANNEL,
                operator: ComparisonOperator::Greater,
                threshold: NumericValue::U32(0),
            },
        ))
        .unwrap();
    assert_eq!(
        mismatched.arm(mismatched_id, context(0), &catalog()),
        Err(TraceError::TriggerTypeMismatch(I32_CHANNEL))
    );
    assert_eq!(
        mismatched.state(mismatched_id),
        TraceState::Idle,
        "failed validation cannot partially mutate trace state"
    );
}

#[test]
fn profile_limits_and_missing_publications_fail_closed_or_are_explicit() {
    let limits = TraceLimits {
        configurations_per_project: 2,
        channels_per_configuration: 1,
        samples_per_capture: 2,
        concurrent_captures_per_controller: 1,
        minimum_virtual_cadence_ms: 10,
        maximum_virtual_duration_ms: 100,
        trigger_depth: 2,
        trigger_nodes: 3,
    };
    let mut trace = TraceEngine::new(limits).unwrap();

    let mut too_many_channels = config(
        40,
        vec![
            channel(BOOL_CHANNEL, BOOL_TARGET, "enable"),
            channel(I32_CHANNEL, I32_TARGET, "count"),
        ],
        TraceTrigger::Immediate,
    );
    too_many_channels.maximum_duration_ms = 100;
    assert_eq!(
        trace.upsert_config(too_many_channels),
        Err(TraceError::ChannelLimitExceeded)
    );

    let mut too_many_samples = config(
        41,
        vec![channel(BOOL_CHANNEL, BOOL_TARGET, "enable")],
        TraceTrigger::Immediate,
    );
    too_many_samples.pre_trigger_samples = 1;
    too_many_samples.post_trigger_samples = 1;
    too_many_samples.maximum_duration_ms = 100;
    assert_eq!(
        trace.upsert_config(too_many_samples),
        Err(TraceError::SampleLimitExceeded)
    );

    let mut invalid_cadence = config(
        42,
        vec![channel(BOOL_CHANNEL, BOOL_TARGET, "enable")],
        TraceTrigger::Immediate,
    );
    invalid_cadence.cadence = TraceCadence::VirtualIntervalMs(9);
    invalid_cadence.maximum_duration_ms = 100;
    assert_eq!(
        trace.upsert_config(invalid_cadence),
        Err(TraceError::CadenceInvalid)
    );

    let mut excessive_duration = config(
        44,
        vec![channel(BOOL_CHANNEL, BOOL_TARGET, "enable")],
        TraceTrigger::Immediate,
    );
    excessive_duration.maximum_duration_ms = 101;
    assert_eq!(
        trace.upsert_config(excessive_duration),
        Err(TraceError::DurationLimitInvalid)
    );

    let mut excessive_expression = config(
        45,
        vec![channel(BOOL_CHANNEL, BOOL_TARGET, "enable")],
        TraceTrigger::Expression(ExpressionNode::Not(Box::new(ExpressionNode::Not(
            Box::new(ExpressionNode::BooleanChannel(BOOL_CHANNEL)),
        )))),
    );
    excessive_expression.maximum_duration_ms = 100;
    assert!(matches!(
        trace.upsert_config(excessive_expression),
        Err(TraceError::TriggerComplexityExceeded { .. })
    ));

    let mut missing = config(
        43,
        vec![channel(BOOL_CHANNEL, BOOL_TARGET, "enable")],
        TraceTrigger::Immediate,
    );
    missing.maximum_duration_ms = 100;
    trace.upsert_config(missing).unwrap();
    trace
        .arm(TraceConfigId(43), context(0), &catalog())
        .unwrap();
    let completed = trace.publish(context(1), &[], &[]).unwrap();
    let capture = trace.capture(completed[0]).unwrap().clone();
    assert_eq!(capture.samples[0].gap, Some(GapReason::PublicationMissing));
    assert_eq!(capture.samples[0].values, vec![None]);
    assert!(capture.verify());

    let mut replay = TraceEngine::new(limits).unwrap();
    let mut replay_config = config(
        43,
        vec![channel(BOOL_CHANNEL, BOOL_TARGET, "enable")],
        TraceTrigger::Immediate,
    );
    replay_config.maximum_duration_ms = 100;
    replay.upsert_config(replay_config).unwrap();
    replay
        .arm(TraceConfigId(43), context(0), &catalog())
        .unwrap();
    let replayed = replay.publish(context(1), &[], &[]).unwrap();
    let replayed_capture = replay.capture(replayed[0]).unwrap();
    assert_eq!(capture, *replayed_capture);
    assert_eq!(capture.replay_hash(), replayed_capture.replay_hash());

    let one_config_limits = TraceLimits {
        configurations_per_project: 1,
        ..limits
    };
    let mut bounded_configs = TraceEngine::new(one_config_limits).unwrap();
    let mut first = config(
        50,
        vec![channel(BOOL_CHANNEL, BOOL_TARGET, "enable")],
        TraceTrigger::Immediate,
    );
    first.maximum_duration_ms = 100;
    bounded_configs.upsert_config(first).unwrap();
    let mut second = config(
        51,
        vec![channel(BOOL_CHANNEL, BOOL_TARGET, "enable")],
        TraceTrigger::Immediate,
    );
    second.maximum_duration_ms = 100;
    assert_eq!(
        bounded_configs.upsert_config(second),
        Err(TraceError::ConfigurationLimitExceeded)
    );

    let mut concurrent = TraceEngine::new(limits).unwrap();
    for id in [60, 61] {
        let mut candidate = config(
            id,
            vec![channel(BOOL_CHANNEL, BOOL_TARGET, &format!("enable-{id}"))],
            TraceTrigger::Immediate,
        );
        candidate.maximum_duration_ms = 100;
        concurrent.upsert_config(candidate).unwrap();
    }
    concurrent
        .arm(TraceConfigId(60), context(0), &catalog())
        .unwrap();
    assert_eq!(
        concurrent.arm(TraceConfigId(61), context(0), &catalog()),
        Err(TraceError::ConcurrentCaptureLimitExceeded)
    );
}
