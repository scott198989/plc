#![allow(clippy::too_many_arguments, clippy::too_many_lines)]

use plc_commissioning::VirtualOnlineSessionId;
use plc_hardware::{
    AddressRequest, ChannelConditionProjection, ChannelDirection, ChannelId, ChannelQuality,
    ChannelRawValue, ConditionLifecycle, ConfiguredModule, ConfiguredState, DeviceRole,
    HardwareConditionEngine, HardwareConditionKey, HardwareFaultAction, HardwareFaultCommand,
    ModuleCatalogId, ModuleId, NaturalChannelSample, ParameterId, PortClass, PoweredState,
    RuntimeDeviceRole, RuntimeHardwareConfiguration, RuntimeModuleConfiguration, RuntimeState,
    TrainingProfile, Uuid, VirtualDevice, VirtualDeviceId, VirtualDeviceName, VirtualInterface,
    VirtualInterfaceId, VirtualIpAddress, VirtualLink, VirtualLinkId, VirtualNetwork, VirtualPort,
    VirtualPortId, VirtualSubnet, VirtualSubnetId,
};
use plc_observability::{
    AccessCapabilities, BitRange, DiagnosticLedger, DiagnosticLimits, DiagnosticRegistry,
    DisplayBase, HardwareDiagnosticBridge, MonitoringEngine, MonitoringLimits, ObservationContext,
    ProbeCatalog, ProbeDefinition, ProbeLayer, PublicationBoundary, PublishedTargetValue, Quality,
    RuntimeTarget, StableTargetId, TargetReference, TraceCadence, TraceChannel, TraceChannelId,
    TraceConfig, TraceConfigId, TraceDiagnosticEvent, TraceEngine, TraceLimits, TraceProbeKind,
    TraceTrigger, TraceTriggerId, WatchRow, WatchRowId, WatchTable, WatchTableId,
};
use plc_runtime::{
    CanonicalValue, CpuState, Hash32, MemoryId, Sha256, UniverseId, ValueType, VirtualControllerId,
};
use plc_system::{
    HardwareObservationCapture, HardwareObservationError, HardwareObservationLinkRequest,
    HardwareObservationSnapshot,
};

struct Ids {
    ordinal: u64,
}

impl Ids {
    fn new() -> Self {
        Self { ordinal: 1 }
    }

    fn next<T: From<Uuid>>(&mut self) -> T {
        let id = Uuid::deterministic_v4(b"system-hardware-observation-matrix", self.ordinal);
        self.ordinal += 1;
        T::from(id)
    }
}

struct Fixture {
    profile: TrainingProfile,
    engine: HardwareConditionEngine,
    configuration: RuntimeHardwareConfiguration,
    pristine_network: VirtualNetwork,
    station: VirtualDeviceId,
    digital_input_module: ModuleId,
    digital_output_module: ModuleId,
    digital_input: ChannelId,
    digital_output: ChannelId,
    analog_input: ChannelId,
    analog_output: ChannelId,
    link: VirtualLinkId,
}

fn configured_module(
    profile: &TrainingProfile,
    ids: &mut Ids,
    catalog: ModuleCatalogId,
    input_start: u32,
    output_start: u32,
) -> ConfiguredModule {
    let definition = profile.module(catalog).expect("catalog");
    let channel_ids = (0..definition.channels.channel_count())
        .map(|_| ids.next())
        .collect::<Vec<ChannelId>>();
    let parameter_count = match catalog {
        ModuleCatalogId::Vai4 | ModuleCatalogId::Vao4 => 4,
        _ => 0,
    };
    let parameter_ids = (0..parameter_count)
        .map(|_| ids.next())
        .collect::<Vec<ParameterId>>();
    ConfiguredModule::from_catalog(
        profile,
        ids.next(),
        ids.ordinal,
        catalog,
        &channel_ids,
        &parameter_ids,
        if definition.input_bytes == 0 {
            AddressRequest::NotUsed
        } else {
            AddressRequest::Explicit(input_start)
        },
        if definition.output_bytes == 0 {
            AddressRequest::NotUsed
        } else {
            AddressRequest::Explicit(output_start)
        },
    )
    .expect("configured module")
}

fn add_interface(
    network: &mut VirtualNetwork,
    ids: &mut Ids,
    owner: VirtualDeviceId,
    provider: Option<ModuleId>,
    address: &str,
    subnet: VirtualSubnetId,
    role: DeviceRole,
) -> (VirtualInterfaceId, VirtualPortId) {
    let interface = ids.next();
    network
        .add_interface(VirtualInterface {
            id: interface,
            creation_ordinal: ids.ordinal,
            owner_device_id: owner,
            provider_module_id: provider,
            name: format!("if-{}", ids.ordinal),
            address: Some(VirtualIpAddress::parse(address).expect("address")),
            subnet_id: Some(subnet),
            port_class: PortClass::EduLink,
            role,
            configured_state: ConfiguredState::Enabled,
            runtime_state: RuntimeState::Available,
        })
        .expect("interface");
    let port = ids.next();
    network
        .add_port(VirtualPort {
            id: port,
            creation_ordinal: ids.ordinal,
            owner_interface_id: interface,
            name: "p1".to_owned(),
            configured_state: ConfiguredState::Enabled,
            runtime_state: RuntimeState::Available,
        })
        .expect("port");
    (interface, port)
}

fn fixture() -> Fixture {
    let profile = TrainingProfile::edu21();
    let mut ids = Ids::new();
    let controller = ids.next();
    let station = ids.next();
    let digital_input = configured_module(&profile, &mut ids, ModuleCatalogId::Vdi16, 0, 0);
    let digital_output = configured_module(&profile, &mut ids, ModuleCatalogId::Vdo16, 0, 0);
    let analog_input = configured_module(&profile, &mut ids, ModuleCatalogId::Vai4, 2, 0);
    let analog_output = configured_module(&profile, &mut ids, ModuleCatalogId::Vao4, 0, 2);

    let mut network = VirtualNetwork::new();
    for (id, ordinal, name) in [(controller, 1, "controller-a"), (station, 2, "station-a")] {
        network
            .add_device(VirtualDevice {
                id,
                creation_ordinal: ordinal,
                device_name: VirtualDeviceName::parse(name).expect("device name"),
                powered_state: PoweredState::Powered,
            })
            .expect("device");
    }
    let subnet = ids.next();
    network
        .add_subnet(VirtualSubnet {
            id: subnet,
            creation_ordinal: 1,
            name: "cell".to_owned(),
            network_address: VirtualIpAddress::parse("192.0.2.0").expect("network"),
            prefix_length: 24,
        })
        .expect("subnet");
    let (_, controller_port) = add_interface(
        &mut network,
        &mut ids,
        controller,
        None,
        "192.0.2.1",
        subnet,
        DeviceRole::Controller,
    );
    let (_, station_port) = add_interface(
        &mut network,
        &mut ids,
        station,
        None,
        "192.0.2.2",
        subnet,
        DeviceRole::Station,
    );
    let link = ids.next();
    network
        .add_link(VirtualLink {
            id: link,
            creation_ordinal: 1,
            endpoint_port_ids: [controller_port, station_port],
            configured_state: ConfiguredState::Enabled,
            runtime_state: RuntimeState::Available,
        })
        .expect("link");
    assert!(network.validate_configuration().is_empty());

    let mut configuration = RuntimeHardwareConfiguration::new();
    configuration
        .add_device(controller, RuntimeDeviceRole::Controller)
        .expect("controller");
    configuration
        .add_device(station, RuntimeDeviceRole::Station)
        .expect("station");
    for (module, owner, required_link) in [
        (digital_input.clone(), controller, None),
        (digital_output.clone(), controller, None),
        (analog_input.clone(), station, Some(link)),
        (analog_output.clone(), station, Some(link)),
    ] {
        configuration
            .add_module(RuntimeModuleConfiguration {
                configured_module: module,
                owner_device_id: owner,
                required_link_id: required_link,
            })
            .expect("runtime module");
    }
    let pristine_network = network.clone();
    let engine = HardwareConditionEngine::new(profile.pin(), configuration.clone(), network, 7)
        .expect("condition engine");
    Fixture {
        profile,
        engine,
        configuration,
        pristine_network,
        station,
        digital_input_module: digital_input.id,
        digital_output_module: digital_output.id,
        digital_input: digital_input.channels[0].id,
        digital_output: digital_output.channels[0].id,
        analog_input: analog_input.channels[0].id,
        analog_output: analog_output.channels[0].id,
        link,
    }
}

fn hash(label: &str) -> Hash32 {
    Sha256::digest(label.as_bytes())
}

fn context(event_sequence: u64) -> ObservationContext {
    ObservationContext {
        universe_id: UniverseId(0x5101),
        universe_epoch: 3,
        controller_id: VirtualControllerId(0x5102),
        controller_epoch: 7,
        session_id: VirtualOnlineSessionId(0x5103),
        session_epoch: 2,
        package_fingerprint: hash("hardware-matrix-package"),
        artifact_fingerprint: hash("hardware-matrix-artifact"),
        profile_fingerprint: hash("hardware-matrix-profile"),
        target_state_hash: Sha256::digest(&event_sequence.to_be_bytes()),
        cpu_state: CpuState::Run,
        virtual_timestamp_ms: event_sequence * 10,
        scan_sequence: event_sequence,
        event_sequence,
        publication_boundary: PublicationBoundary::ScanEnd,
    }
}

fn capabilities() -> AccessCapabilities {
    AccessCapabilities {
        monitor: true,
        modify: false,
        force: false,
        trace: true,
        natural_layer: true,
        effective_layer: true,
    }
}

fn catalog() -> ProbeCatalog {
    let mut catalog = ProbeCatalog::new(
        hash("hardware-matrix-artifact"),
        hash("hardware-matrix-profile"),
    );
    for (ordinal, value_type) in [
        (1_u128, ValueType::Bool),
        (2, ValueType::Bool),
        (3, ValueType::I16),
        (4, ValueType::I16),
    ] {
        catalog
            .insert(ProbeDefinition {
                id: StableTargetId(ordinal),
                runtime_target: RuntimeTarget::Memory(MemoryId(
                    u32::try_from(ordinal).expect("memory identity"),
                )),
                bit_range: BitRange::whole_value(),
                value_type,
                instance_path: vec![ordinal],
                capabilities: capabilities(),
                primary_source: None,
                display_name: format!("hardware-{ordinal}"),
            })
            .expect("probe");
    }
    catalog
}

fn observation_quality(value: ChannelQuality) -> Quality {
    match value {
        ChannelQuality::Good => Quality::Good,
        ChannelQuality::Uncertain => Quality::Uncertain,
        ChannelQuality::Bad => Quality::Bad,
        ChannelQuality::NotPresent => Quality::NotPresent,
    }
}

fn canonical(value: ChannelRawValue) -> CanonicalValue {
    match value {
        ChannelRawValue::Bool(value) => CanonicalValue::Bool(value),
        ChannelRawValue::Int(value) => CanonicalValue::I16(value),
    }
}

fn published(
    target: StableTargetId,
    natural: ChannelRawValue,
    projection: &ChannelConditionProjection,
) -> PublishedTargetValue {
    let input = projection.direction == ChannelDirection::Input;
    PublishedTargetValue {
        target_id: target,
        value_type: canonical(projection.cpu_value).value_type(),
        natural_value: canonical(natural),
        effective_value: canonical(projection.cpu_value),
        raw_input_value: input.then(|| canonical(natural)),
        committed_output_value: (!input).then(|| canonical(natural)),
        delivered_output_value: (!input).then(|| canonical(projection.delivered_value)),
        quality: observation_quality(projection.quality),
        force: None,
    }
}

fn projections(fixture: &Fixture) -> Vec<PublishedTargetValue> {
    [
        (
            StableTargetId(1),
            fixture.digital_input,
            ChannelRawValue::Bool(true),
        ),
        (
            StableTargetId(2),
            fixture.digital_output,
            ChannelRawValue::Bool(true),
        ),
        (
            StableTargetId(3),
            fixture.analog_input,
            ChannelRawValue::Int(1_234),
        ),
        (
            StableTargetId(4),
            fixture.analog_output,
            ChannelRawValue::Int(777),
        ),
    ]
    .into_iter()
    .map(|(target, channel, raw)| {
        let projection = fixture
            .engine
            .project_channel(
                channel,
                NaturalChannelSample {
                    raw_value: raw,
                    provider_quality: ChannelQuality::Good,
                    force_overlay_active: false,
                },
            )
            .expect("channel projection");
        published(target, raw, &projection)
    })
    .collect()
}

fn observe_transition(
    fixture: &mut Fixture,
    action: HardwareFaultAction,
    expected_condition: HardwareConditionKey,
    lifecycle: ConditionLifecycle,
    affected_target: StableTargetId,
    ordinal: u64,
    commands: &mut Vec<HardwareFaultCommand>,
    ledger: &mut DiagnosticLedger,
    bridge: &mut HardwareDiagnosticBridge,
    monitor: &mut MonitoringEngine,
    trace: &mut TraceEngine,
    catalog: &ProbeCatalog,
    links: &mut Vec<HardwareObservationLinkRequest>,
) {
    let value_type = catalog
        .definition(affected_target)
        .expect("affected probe")
        .value_type;
    let trace_id = TraceConfigId(u128::from(ordinal));
    trace
        .upsert_config(TraceConfig {
            id: trace_id,
            trigger_id: TraceTriggerId(u128::from(ordinal)),
            name: format!("condition-{ordinal}"),
            channels: vec![TraceChannel {
                id: TraceChannelId(u128::from(ordinal)),
                alias: "affected".to_owned(),
                probe: TraceProbeKind::LoadedTarget {
                    target: TargetReference::Stable(affected_target),
                    layer: ProbeLayer::Effective,
                },
                display_unit: None,
            }],
            cadence: TraceCadence::EveryScans(1),
            trigger: TraceTrigger::Immediate,
            pre_trigger_samples: 0,
            post_trigger_samples: 0,
            post_trigger_duration_ms: None,
            maximum_duration_ms: 100,
        })
        .expect("trace config");
    trace
        .arm(trace_id, context(ordinal * 10), catalog)
        .expect("trace arm");

    let command = HardwareFaultCommand {
        idempotency_key: Uuid::deterministic_v4(b"system-hardware-observation-command", ordinal),
        expected_controller_epoch: 7,
        action,
    };
    let receipt = fixture
        .engine
        .apply(command.clone())
        .expect("hardware transition");
    commands.push(command);
    assert!(
        receipt
            .events
            .iter()
            .any(|event| { event.condition == expected_condition && event.lifecycle == lifecycle })
    );

    let publish_context = context(ordinal * 10 + 1);
    let diagnostic_receipts = bridge
        .ingest_events(ledger, publish_context, &receipt.events)
        .expect("diagnostic bridge");
    assert!(
        diagnostic_receipts
            .iter()
            .all(plc_observability::HardwareDiagnosticReceipt::verify)
    );
    let authoritative = diagnostic_receipts
        .iter()
        .map(|receipt| {
            ledger
                .retained_events()
                .into_iter()
                .find(|event| event.occurrence_id == receipt.ledger_occurrence_id)
                .expect("authoritative diagnostic")
        })
        .collect::<Vec<_>>();
    let diagnostic_events = authoritative
        .iter()
        .map(|event| {
            TraceDiagnosticEvent::from_authoritative(event, ledger.registry())
                .expect("trace diagnostic")
        })
        .collect::<Vec<_>>();
    let values = projections(fixture);
    monitor
        .publish(publish_context, &values)
        .expect("monitor publication");
    let row = WatchRowId(affected_target.0);
    let sample = monitor.latest(row).expect("causal monitor sample");
    assert_eq!(sample.event_sequence, publish_context.event_sequence);
    assert_eq!(sample.value.value_type(), value_type);
    let completed = trace
        .publish(publish_context, &values, &diagnostic_events)
        .expect("trace publication");
    let capture = trace
        .capture(*completed.first().expect("completed trace"))
        .expect("trace capture");
    let occurrence_ids = authoritative
        .iter()
        .map(|event| event.occurrence_id)
        .collect::<Vec<_>>();
    assert!(capture.samples.iter().any(|sample| {
        occurrence_ids
            .iter()
            .all(|occurrence| sample.diagnostic_occurrence_ids.contains(occurrence))
    }));
    links.extend(
        diagnostic_receipts
            .iter()
            .filter(|receipt| !receipt.duplicate)
            .map(|receipt| HardwareObservationLinkRequest {
                provider_event_sequence: receipt.provider_key.provider_event_sequence,
                diagnostic_occurrence_id: receipt.ledger_occurrence_id,
                monitor_row_id: row,
                publication_event_sequence: publish_context.event_sequence,
                trace_capture_id: capture.id,
            }),
    );

    let replayed = HardwareConditionEngine::replay(
        fixture.profile.pin(),
        fixture.configuration.clone(),
        fixture.pristine_network.clone(),
        7,
        commands.clone(),
    )
    .expect("condition replay");
    assert_eq!(replayed.snapshot(), fixture.engine.snapshot());
    let diagnostic_snapshot = ledger.capture_snapshot(publish_context);
    let trace_snapshot = trace.capture_snapshot(publish_context);
    assert!(diagnostic_snapshot.verify());
    assert!(trace_snapshot.verify());
    assert_eq!(
        bridge.replay_hash().expect("bridge replay"),
        bridge.bridge_hash()
    );
}

#[test]
fn complete_physical_condition_matrix_keeps_one_causal_monitor_trace_snapshot_and_replay_path() {
    let mut fixture = fixture();
    let catalog = catalog();
    let mut monitor = MonitoringEngine::new(MonitoringLimits::edu21()).expect("monitor");
    monitor
        .upsert_table(WatchTable {
            id: WatchTableId(1),
            name: "physical matrix".to_owned(),
            rows: (1_u128..=4)
                .map(|ordinal| WatchRow {
                    id: WatchRowId(ordinal),
                    target: TargetReference::Stable(StableTargetId(ordinal)),
                    layer: ProbeLayer::Effective,
                    display_base: DisplayBase::Automatic,
                    unit: None,
                    format: None,
                    note: None,
                    order: u32::try_from(ordinal).expect("watch order"),
                })
                .collect(),
        })
        .expect("watch table");
    monitor.start(context(1), &catalog).expect("monitor start");
    let mut trace = TraceEngine::new(TraceLimits::edu21()).expect("trace");
    let mut ledger = DiagnosticLedger::new(
        DiagnosticRegistry::edu21_runtime(),
        DiagnosticLimits::edu21(),
    )
    .expect("ledger");
    let mut bridge = HardwareDiagnosticBridge::default();
    let mut commands = Vec::new();
    let mut links = Vec::new();
    let steps = [
        (
            HardwareFaultAction::PullModule(fixture.digital_input_module),
            HardwareFaultAction::RestoreModule(fixture.digital_input_module),
            HardwareConditionKey::ModuleNotPresent(fixture.digital_input_module),
            StableTargetId(1),
        ),
        (
            HardwareFaultAction::InstallWrongModule {
                module_id: fixture.digital_output_module,
                installed_catalog: ModuleCatalogId::Vdi16,
            },
            HardwareFaultAction::RestoreConfiguredModule(fixture.digital_output_module),
            HardwareConditionKey::WrongModule(fixture.digital_output_module),
            StableTargetId(2),
        ),
        (
            HardwareFaultAction::SetChannelFault(fixture.digital_output),
            HardwareFaultAction::ClearChannelFault(fixture.digital_output),
            HardwareConditionKey::ChannelFault(fixture.digital_output),
            StableTargetId(2),
        ),
        (
            HardwareFaultAction::SetWireBreak(fixture.analog_input),
            HardwareFaultAction::ClearWireBreak(fixture.analog_input),
            HardwareConditionKey::WireBreak(fixture.analog_input),
            StableTargetId(3),
        ),
        (
            HardwareFaultAction::SetStationAvailable {
                device_id: fixture.station,
                available: false,
            },
            HardwareFaultAction::SetStationAvailable {
                device_id: fixture.station,
                available: true,
            },
            HardwareConditionKey::StationUnavailable(fixture.station),
            StableTargetId(3),
        ),
        (
            HardwareFaultAction::SetVirtualLinkAvailable {
                link_id: fixture.link,
                available: false,
            },
            HardwareFaultAction::SetVirtualLinkAvailable {
                link_id: fixture.link,
                available: true,
            },
            HardwareConditionKey::LinkUnavailable(fixture.link),
            StableTargetId(4),
        ),
    ];

    let mut ordinal = 1_u64;
    for (activate, clear, condition, target) in steps {
        observe_transition(
            &mut fixture,
            activate,
            condition,
            ConditionLifecycle::Activated,
            target,
            ordinal,
            &mut commands,
            &mut ledger,
            &mut bridge,
            &mut monitor,
            &mut trace,
            &catalog,
            &mut links,
        );
        let active = projections(&fixture);
        let affected = &active[usize::try_from(target.0 - 1).expect("target index")];
        assert_ne!(affected.quality, Quality::Good);
        if matches!(target, StableTargetId(2 | 4)) {
            assert_ne!(
                affected.committed_output_value,
                affected.delivered_output_value
            );
        }
        ordinal += 1;
        observe_transition(
            &mut fixture,
            clear,
            condition,
            ConditionLifecycle::Cleared,
            target,
            ordinal,
            &mut commands,
            &mut ledger,
            &mut bridge,
            &mut monitor,
            &mut trace,
            &catalog,
            &mut links,
        );
        assert_eq!(
            projections(&fixture)[usize::try_from(target.0 - 1).expect("target index")].quality,
            Quality::Good
        );
        ordinal += 1;
    }

    assert_eq!(commands.len(), 12);
    assert!(fixture.engine.condition_events().len() >= 12);
    assert_eq!(
        bridge.receipts().len(),
        fixture.engine.condition_events().len()
    );
    assert!(monitor.persistence().expect("monitor snapshot").verify());

    let aggregate_context = context((ordinal - 1) * 10 + 1);
    let snapshot = HardwareObservationSnapshot::capture(HardwareObservationCapture {
        conditions: &fixture.engine,
        commands: &commands,
        monitoring: &monitor,
        traces: &trace,
        diagnostics: &ledger,
        diagnostic_bridge: &bridge,
        context: aggregate_context,
        links: &links,
    })
    .expect("aggregate hardware-observation snapshot");
    assert!(snapshot.verify());
    assert_eq!(snapshot.command_count(), commands.len());
    assert_eq!(
        snapshot.causal_link_count(),
        fixture.engine.condition_events().len()
    );
    let replay = snapshot
        .replay(
            fixture.profile.pin(),
            fixture.configuration.clone(),
            fixture.pristine_network.clone(),
        )
        .expect("aggregate hardware-observation replay");
    assert_eq!(replay.snapshot_content_hash, snapshot.content_hash);
    assert_eq!(
        replay.replayed_condition_fingerprint,
        fixture.engine.snapshot().state_fingerprint
    );
    assert_eq!(replay.replayed_command_count, commands.len());
    assert_eq!(replay.replayed_causal_link_count, links.len());

    let mut incomplete_links = links.clone();
    incomplete_links.pop();
    assert!(matches!(
        HardwareObservationSnapshot::capture(HardwareObservationCapture {
            conditions: &fixture.engine,
            commands: &commands,
            monitoring: &monitor,
            traces: &trace,
            diagnostics: &ledger,
            diagnostic_bridge: &bridge,
            context: aggregate_context,
            links: &incomplete_links,
        }),
        Err(HardwareObservationError::DuplicateOrIncompleteCausalLinks)
    ));
    let mut tampered = snapshot.clone();
    tampered.conditions.command_boundary += 1;
    assert!(!tampered.verify());
}
