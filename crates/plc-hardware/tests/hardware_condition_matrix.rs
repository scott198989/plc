#![allow(clippy::too_many_arguments, clippy::too_many_lines)]

use plc_hardware::{
    AddressArea, AddressRequest, ChannelAddress, ChannelConditionProjection, ChannelDirection,
    ChannelId, ChannelQuality, ChannelRawValue, ConditionLifecycle, ConfiguredModule,
    ConfiguredState, DeviceRole, HardwareChannelBinding, HardwareConditionEngine,
    HardwareConditionError, HardwareConditionKey, HardwareFaultAction, HardwareFaultCommand,
    ModuleCatalogId, ModuleId, NaturalChannelSample, ParameterId, PortClass, PoweredState,
    PrimitiveType, ProcessImageError, RuntimeDeviceRole, RuntimeHardwareConfiguration,
    RuntimeModuleConfiguration, RuntimeState, TrainingProfile, Uuid, VirtualDevice,
    VirtualDeviceId, VirtualDeviceName, VirtualInterface, VirtualInterfaceId, VirtualIpAddress,
    VirtualLink, VirtualLinkId, VirtualNetwork, VirtualPort, VirtualPortId, VirtualSubnet,
    VirtualSubnetId,
};

struct Ids {
    next: u64,
}

impl Ids {
    fn new() -> Self {
        Self { next: 1 }
    }

    fn next<T: From<Uuid>>(&mut self) -> T {
        let id = Uuid::deterministic_v4(b"edu21-hardware-condition-matrix", self.next);
        self.next += 1;
        T::from(id)
    }
}

struct Fixture {
    profile: TrainingProfile,
    engine: HardwareConditionEngine,
    pristine_network: VirtualNetwork,
    configuration: RuntimeHardwareConfiguration,
    controller_device: VirtualDeviceId,
    station_device: VirtualDeviceId,
    digital_input_module: ModuleId,
    digital_output_module: ModuleId,
    vlink_module: ModuleId,
    digital_input: ChannelId,
    digital_output: ChannelId,
    analog_input: ChannelId,
    analog_output: ChannelId,
    provider_interface: VirtualInterfaceId,
    provider_ports: [VirtualPortId; 2],
    link: VirtualLinkId,
    next_command: u64,
}

impl Fixture {
    fn command(&mut self, action: HardwareFaultAction) -> HardwareFaultCommand {
        let command = HardwareFaultCommand {
            idempotency_key: Uuid::deterministic_v4(
                b"edu21-hardware-fault-command",
                self.next_command,
            ),
            expected_controller_epoch: 7,
            action,
        };
        self.next_command += 1;
        command
    }

    fn project(&self, channel_id: ChannelId, value: ChannelRawValue) -> ChannelConditionProjection {
        self.engine
            .project_channel(
                channel_id,
                NaturalChannelSample {
                    raw_value: value,
                    provider_quality: ChannelQuality::Good,
                    force_overlay_active: false,
                },
            )
            .expect("configured channel")
    }
}

fn configured_module(
    profile: &TrainingProfile,
    ids: &mut Ids,
    catalog: ModuleCatalogId,
    input_start: u32,
    output_start: u32,
) -> ConfiguredModule {
    let definition = profile.module(catalog).expect("catalog");
    let channel_ids: Vec<ChannelId> = (0..definition.channels.channel_count())
        .map(|_| ids.next())
        .collect();
    let parameter_count = match catalog {
        ModuleCatalogId::Vai4 | ModuleCatalogId::Vao4 | ModuleCatalogId::Vrtd4 => 4,
        _ => 0,
    };
    let parameter_ids: Vec<ParameterId> = (0..parameter_count).map(|_| ids.next()).collect();
    ConfiguredModule::from_catalog(
        profile,
        ids.next(),
        ids.next,
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
    owner_device_id: VirtualDeviceId,
    provider_module_id: Option<ModuleId>,
    address: &str,
    subnet_id: VirtualSubnetId,
    role: DeviceRole,
    port_count: usize,
) -> (VirtualInterfaceId, Vec<VirtualPortId>) {
    let interface_id = ids.next();
    network
        .add_interface(VirtualInterface {
            id: interface_id,
            creation_ordinal: ids.next,
            owner_device_id,
            provider_module_id,
            name: format!("edu-if-{}", ids.next),
            address: Some(VirtualIpAddress::parse(address).expect("address")),
            subnet_id: Some(subnet_id),
            port_class: PortClass::EduLink,
            role,
            configured_state: ConfiguredState::Enabled,
            runtime_state: RuntimeState::Available,
        })
        .expect("interface");
    let mut ports = Vec::new();
    for index in 0..port_count {
        let port_id = ids.next();
        network
            .add_port(VirtualPort {
                id: port_id,
                creation_ordinal: ids.next,
                owner_interface_id: interface_id,
                name: format!("p{index}"),
                configured_state: ConfiguredState::Enabled,
                runtime_state: RuntimeState::Available,
            })
            .expect("port");
        ports.push(port_id);
    }
    (interface_id, ports)
}

fn fixture() -> Fixture {
    let profile = TrainingProfile::edu21();
    let mut ids = Ids::new();
    let controller_device = ids.next();
    let station_device = ids.next();
    let digital_input = configured_module(&profile, &mut ids, ModuleCatalogId::Vdi16, 0, 0);
    let digital_output = configured_module(&profile, &mut ids, ModuleCatalogId::Vdo16, 0, 0);
    let analog_input = configured_module(&profile, &mut ids, ModuleCatalogId::Vai4, 2, 0);
    let analog_output = configured_module(&profile, &mut ids, ModuleCatalogId::Vao4, 0, 2);
    let vlink = configured_module(&profile, &mut ids, ModuleCatalogId::Vlink2, 0, 0);

    let mut network = VirtualNetwork::new();
    for (id, ordinal, name) in [
        (controller_device, 1, "controller-a"),
        (station_device, 2, "station-a"),
    ] {
        network
            .add_device(VirtualDevice {
                id,
                creation_ordinal: ordinal,
                device_name: VirtualDeviceName::parse(name).expect("name"),
                powered_state: PoweredState::Powered,
            })
            .expect("device");
    }
    let subnet_id = ids.next();
    network
        .add_subnet(VirtualSubnet {
            id: subnet_id,
            creation_ordinal: 1,
            name: "cell".to_owned(),
            network_address: VirtualIpAddress::parse("192.0.2.0").expect("network"),
            prefix_length: 24,
        })
        .expect("subnet");
    let _integrated = add_interface(
        &mut network,
        &mut ids,
        controller_device,
        None,
        "192.0.2.1",
        subnet_id,
        DeviceRole::Controller,
        1,
    );
    let (provider_interface, provider_ports) = add_interface(
        &mut network,
        &mut ids,
        controller_device,
        Some(vlink.id),
        "192.0.2.2",
        subnet_id,
        DeviceRole::Controller,
        2,
    );
    let (_station_interface, station_ports) = add_interface(
        &mut network,
        &mut ids,
        station_device,
        None,
        "192.0.2.3",
        subnet_id,
        DeviceRole::Station,
        2,
    );
    let link = ids.next();
    network
        .add_link(VirtualLink {
            id: link,
            creation_ordinal: 1,
            endpoint_port_ids: [provider_ports[0], station_ports[0]],
            configured_state: ConfiguredState::Enabled,
            runtime_state: RuntimeState::Available,
        })
        .expect("link");
    assert!(network.validate_configuration().is_empty());

    let mut configuration = RuntimeHardwareConfiguration::new();
    configuration
        .add_device(controller_device, RuntimeDeviceRole::Controller)
        .expect("controller");
    configuration
        .add_device(station_device, RuntimeDeviceRole::Station)
        .expect("station");
    for (module, owner, required_link) in [
        (digital_input.clone(), controller_device, None),
        (digital_output.clone(), controller_device, None),
        (analog_input.clone(), station_device, Some(link)),
        (analog_output.clone(), station_device, Some(link)),
        (vlink.clone(), controller_device, None),
    ] {
        configuration
            .add_module(RuntimeModuleConfiguration {
                configured_module: module,
                owner_device_id: owner,
                required_link_id: required_link,
            })
            .expect("module");
    }
    let pristine_network = network.clone();
    let engine = HardwareConditionEngine::new(profile.pin(), configuration.clone(), network, 7)
        .expect("runtime engine");
    Fixture {
        profile,
        engine,
        pristine_network,
        configuration,
        controller_device,
        station_device,
        digital_input_module: digital_input.id,
        digital_output_module: digital_output.id,
        vlink_module: vlink.id,
        digital_input: digital_input.channels[0].id,
        digital_output: digital_output.channels[0].id,
        analog_input: analog_input.channels[0].id,
        analog_output: analog_output.channels[0].id,
        provider_interface,
        provider_ports: [provider_ports[0], provider_ports[1]],
        link,
        next_command: 1,
    }
}

fn engine_with_mutated_module(
    target: ModuleId,
    mutate: impl FnOnce(&mut ConfiguredModule),
) -> Result<HardwareConditionEngine, HardwareConditionError> {
    let fixture = fixture();
    let mut configuration = RuntimeHardwareConfiguration::new();
    for (id, role) in fixture.configuration.devices() {
        configuration.add_device(*id, *role)?;
    }
    let mut mutate = Some(mutate);
    for module in fixture.configuration.modules().values() {
        let mut module = module.clone();
        if module.configured_module.id == target {
            mutate.take().expect("target appears once")(&mut module.configured_module);
        }
        configuration.add_module(module)?;
    }
    HardwareConditionEngine::new(
        fixture.profile.pin(),
        configuration,
        fixture.pristine_network,
        7,
    )
}

#[test]
fn handcrafted_channel_shape_deviations_fail_closed_before_runtime_creation() {
    let fixture = fixture();
    let digital = fixture.digital_input_module;
    let analog = fixture
        .configuration
        .modules()
        .values()
        .find(|module| {
            module
                .configured_module
                .channels
                .first()
                .is_some_and(|channel| channel.id == fixture.analog_input)
        })
        .expect("analog module")
        .configured_module
        .id;
    let corruptions: [fn(&mut ConfiguredModule); 6] = [
        |module| module.channels[0].direction = ChannelDirection::Output,
        |module| module.channels[0].direction_index = 9,
        |module| module.channels[0].raw_type = PrimitiveType::Int,
        |module| module.channels[0].declared_ordinal = 1,
        |module| module.channels[0].diagnostic_capabilities.channel_fault = false,
        |module| module.channels[0].diagnostic_capabilities.wire_break = true,
    ];
    for corrupt in corruptions {
        assert!(matches!(
            engine_with_mutated_module(digital, corrupt),
            Err(HardwareConditionError::InvalidConfiguration(_, id)) if id == digital.uuid()
        ));
    }
    assert!(matches!(
        engine_with_mutated_module(analog, |module| {
            module.channels[0].engineering_scaling = None;
        }),
        Err(HardwareConditionError::InvalidConfiguration(_, id)) if id == analog.uuid()
    ));
    assert!(matches!(
        engine_with_mutated_module(analog, |module| {
            let scaling = module.channels[0]
                .engineering_scaling
                .as_mut()
                .expect("scaling");
            core::mem::swap(&mut scaling.engineering_min, &mut scaling.engineering_max);
        }),
        Err(HardwareConditionError::InvalidConfiguration(_, id)) if id == analog.uuid()
    ));
    assert!(matches!(
        engine_with_mutated_module(digital, |module| module.channels.swap(0, 1)),
        Err(HardwareConditionError::InvalidConfiguration(_, id)) if id == digital.uuid()
    ));
}

#[test]
fn every_edu21_channel_representation_uses_little_endian_lsb0_and_twos_complement() {
    let mut ids = Ids::new();
    for catalog in [
        ModuleCatalogId::Vai4,
        ModuleCatalogId::Vao4,
        ModuleCatalogId::Vrtd4,
    ] {
        for channel_index in 0_u8..4 {
            let binding = HardwareChannelBinding {
                controller_id: ids.next(),
                controller_creation_ordinal: 1,
                module_id: ids.next(),
                location_rank: 0,
                station_creation_ordinal: 0,
                slot_number: 1,
                module_creation_ordinal: u64::from(channel_index),
                channel_id: ids.next(),
                channel_index,
                direction: if catalog == ModuleCatalogId::Vao4 {
                    ChannelDirection::Output
                } else {
                    ChannelDirection::Input
                },
                raw_type: PrimitiveType::Int,
                address: ChannelAddress::Word {
                    area: if catalog == ModuleCatalogId::Vao4 {
                        AddressArea::Output
                    } else {
                        AddressArea::Input
                    },
                    byte: u32::from(channel_index) * 2,
                },
            };
            for value in [i16::MIN, -2, -1, 0, 1, 0x1234, i16::MAX] {
                let mut image = vec![0xA5; 8];
                binding
                    .write_raw(&mut image, ChannelRawValue::Int(value))
                    .expect("write");
                let offset = usize::from(channel_index) * 2;
                assert_eq!(&image[offset..offset + 2], &value.to_le_bytes());
                assert_eq!(
                    binding.read_raw(&image).expect("read"),
                    ChannelRawValue::Int(value)
                );
            }
        }
    }

    for (catalog, direction, count) in [
        (ModuleCatalogId::Vdi16, ChannelDirection::Input, 16_u8),
        (ModuleCatalogId::Vdo16, ChannelDirection::Output, 16),
        (ModuleCatalogId::Vmix8, ChannelDirection::Input, 8),
        (ModuleCatalogId::Vmix8, ChannelDirection::Output, 8),
    ] {
        for channel_index in 0..count {
            let binding = HardwareChannelBinding {
                controller_id: ids.next(),
                controller_creation_ordinal: 1,
                module_id: ids.next(),
                location_rank: 0,
                station_creation_ordinal: 0,
                slot_number: 1,
                module_creation_ordinal: u64::from(channel_index),
                channel_id: ids.next(),
                channel_index,
                direction,
                raw_type: PrimitiveType::Bool,
                address: ChannelAddress::Bit {
                    area: direction.area(),
                    byte: if catalog == ModuleCatalogId::Vmix8 {
                        0
                    } else {
                        u32::from(channel_index / 8)
                    },
                    bit: channel_index % 8,
                },
            };
            let mut image = [0_u8; 2];
            binding
                .write_raw(&mut image, ChannelRawValue::Bool(true))
                .expect("write bit");
            let byte = if catalog == ModuleCatalogId::Vmix8 {
                0
            } else {
                usize::from(channel_index / 8)
            };
            assert_eq!(image[byte], 1_u8 << (channel_index % 8));
            assert_eq!(
                binding.read_raw(&image).expect("read bit"),
                ChannelRawValue::Bool(true)
            );
        }
    }
}

#[test]
fn process_image_codec_rejects_wrong_area_invalid_bit_type_and_bounds_without_mutation() {
    let mut ids = Ids::new();
    let base = HardwareChannelBinding {
        controller_id: ids.next(),
        controller_creation_ordinal: 1,
        module_id: ids.next(),
        location_rank: 0,
        station_creation_ordinal: 0,
        slot_number: 1,
        module_creation_ordinal: 1,
        channel_id: ids.next(),
        channel_index: 0,
        direction: ChannelDirection::Input,
        raw_type: PrimitiveType::Bool,
        address: ChannelAddress::Bit {
            area: AddressArea::Input,
            byte: 0,
            bit: 0,
        },
    };

    let mut wrong_area = base.clone();
    wrong_area.address = ChannelAddress::Bit {
        area: AddressArea::Output,
        byte: 0,
        bit: 0,
    };
    let mut image = [0x5A];
    assert_eq!(
        wrong_area.read_raw(&image),
        Err(ProcessImageError::InvalidAddress)
    );
    assert_eq!(
        wrong_area.write_raw(&mut image, ChannelRawValue::Bool(true)),
        Err(ProcessImageError::InvalidAddress)
    );
    assert_eq!(image, [0x5A]);

    let mut invalid_bit = base.clone();
    invalid_bit.address = ChannelAddress::Bit {
        area: AddressArea::Input,
        byte: 0,
        bit: 8,
    };
    assert_eq!(
        invalid_bit.read_raw(&image),
        Err(ProcessImageError::InvalidAddress)
    );
    assert_eq!(
        invalid_bit.write_raw(&mut image, ChannelRawValue::Bool(true)),
        Err(ProcessImageError::InvalidAddress)
    );
    assert_eq!(image, [0x5A]);

    assert_eq!(
        base.write_raw(&mut image, ChannelRawValue::Int(1)),
        Err(ProcessImageError::TypeMismatch)
    );
    assert_eq!(image, [0x5A]);
    let mut mismatched_type = base.clone();
    mismatched_type.raw_type = PrimitiveType::Int;
    assert_eq!(
        mismatched_type.read_raw(&image),
        Err(ProcessImageError::TypeMismatch)
    );

    let mut out_of_bounds = base;
    out_of_bounds.address = ChannelAddress::Word {
        area: AddressArea::Input,
        byte: 0,
    };
    out_of_bounds.raw_type = PrimitiveType::Int;
    assert_eq!(
        out_of_bounds.read_raw(&image),
        Err(ProcessImageError::OutOfBounds)
    );
    assert_eq!(
        out_of_bounds.write_raw(&mut image, ChannelRawValue::Int(0x1234)),
        Err(ProcessImageError::OutOfBounds)
    );
    assert_eq!(image, [0x5A]);
}

fn assert_missing_input(projection: &ChannelConditionProjection, expected: HardwareConditionKey) {
    assert_eq!(projection.cpu_value, ChannelRawValue::Bool(false));
    assert_eq!(projection.quality, ChannelQuality::NotPresent);
    assert!(!projection.delivery_suppressed);
    assert!(projection.active_conditions.contains(&expected));
}

fn assert_missing_output(projection: &ChannelConditionProjection, expected: HardwareConditionKey) {
    assert_eq!(projection.cpu_value, ChannelRawValue::Bool(true));
    assert_eq!(projection.delivered_value, ChannelRawValue::Bool(false));
    assert_eq!(projection.quality, ChannelQuality::NotPresent);
    assert!(projection.delivery_suppressed);
    assert!(projection.active_conditions.contains(&expected));
}

#[test]
fn pull_wrong_module_and_restore_matrix_is_causal_idempotent_and_replayable() {
    for wrong in [false, true] {
        let mut fixture = fixture();
        let set_action = if wrong {
            HardwareFaultAction::InstallWrongModule {
                module_id: fixture.digital_input_module,
                installed_catalog: ModuleCatalogId::Vdo16,
            }
        } else {
            HardwareFaultAction::PullModule(fixture.digital_input_module)
        };
        let expected = if wrong {
            HardwareConditionKey::WrongModule(fixture.digital_input_module)
        } else {
            HardwareConditionKey::ModuleNotPresent(fixture.digital_input_module)
        };
        let command = fixture.command(set_action);
        let first = fixture.engine.apply(command.clone()).expect("set");
        assert!(first.changed);
        assert_eq!(
            first.events.first().expect("event").lifecycle,
            ConditionLifecycle::Activated
        );
        assert_missing_input(
            &fixture.project(fixture.digital_input, ChannelRawValue::Bool(true)),
            expected,
        );
        assert_eq!(fixture.engine.condition_events().len(), 1);
        assert_eq!(
            fixture.engine.apply(command).expect("same command"),
            first,
            "same idempotency key returns the original receipt"
        );
        let repeated = fixture.command(set_action);
        assert!(!fixture.engine.apply(repeated).expect("repeat set").changed);
        assert_eq!(fixture.engine.condition_events().len(), 1);
        let clear_action = if wrong {
            HardwareFaultAction::RestoreConfiguredModule(fixture.digital_input_module)
        } else {
            HardwareFaultAction::RestoreModule(fixture.digital_input_module)
        };
        let clear = fixture.command(clear_action);
        let commands = vec![
            HardwareFaultCommand {
                idempotency_key: Uuid::deterministic_v4(b"replay", 1),
                expected_controller_epoch: 7,
                action: set_action,
            },
            HardwareFaultCommand {
                idempotency_key: Uuid::deterministic_v4(b"replay", 2),
                expected_controller_epoch: 7,
                action: clear_action,
            },
        ];
        assert!(fixture.engine.apply(clear).expect("clear").changed);
        let restored = fixture.project(fixture.digital_input, ChannelRawValue::Bool(true));
        assert_eq!(restored.cpu_value, ChannelRawValue::Bool(true));
        assert_eq!(restored.quality, ChannelQuality::Good);
        let replayed = HardwareConditionEngine::replay(
            fixture.profile.pin(),
            fixture.configuration.clone(),
            fixture.pristine_network.clone(),
            7,
            commands,
        )
        .expect("replay");
        let replay_projection = replayed
            .project_channel(
                fixture.digital_input,
                NaturalChannelSample {
                    raw_value: ChannelRawValue::Bool(true),
                    provider_quality: ChannelQuality::Good,
                    force_overlay_active: false,
                },
            )
            .expect("projection");
        assert_eq!(replay_projection.cpu_value, restored.cpu_value);
        assert_eq!(replay_projection.quality, restored.quality);
        assert_eq!(replayed.snapshot().active_conditions, Vec::new());
    }

    let mut fixture = fixture();
    let pull_output = fixture.command(HardwareFaultAction::PullModule(
        fixture.digital_output_module,
    ));
    fixture.engine.apply(pull_output).expect("pull output");
    assert_missing_output(
        &fixture.project(fixture.digital_output, ChannelRawValue::Bool(true)),
        HardwareConditionKey::ModuleNotPresent(fixture.digital_output_module),
    );

    let mut fixture = self::fixture();
    let wrong_output = fixture.command(HardwareFaultAction::InstallWrongModule {
        module_id: fixture.digital_output_module,
        installed_catalog: ModuleCatalogId::Vdi16,
    });
    fixture
        .engine
        .apply(wrong_output)
        .expect("wrong output module");
    assert_missing_output(
        &fixture.project(fixture.digital_output, ChannelRawValue::Bool(true)),
        HardwareConditionKey::WrongModule(fixture.digital_output_module),
    );
}

#[test]
fn channel_fault_and_wire_break_matrix_preserves_quality_force_and_capability_rules() {
    let mut fixture = fixture();
    let mut ids = Ids::new();
    for catalog in [
        ModuleCatalogId::Vpwr1,
        ModuleCatalogId::VstnH1,
        ModuleCatalogId::Vdi16,
        ModuleCatalogId::Vdo16,
        ModuleCatalogId::Vai4,
        ModuleCatalogId::Vao4,
        ModuleCatalogId::Vmix8,
        ModuleCatalogId::Vrtd4,
        ModuleCatalogId::Vlink2,
    ] {
        let expected_wire_break = matches!(
            catalog,
            ModuleCatalogId::Vai4 | ModuleCatalogId::Vao4 | ModuleCatalogId::Vrtd4
        );
        assert_eq!(
            fixture
                .profile
                .module(catalog)
                .expect("catalog")
                .supports_wire_break,
            expected_wire_break
        );
        let configured = configured_module(&fixture.profile, &mut ids, catalog, 0, 0);
        assert!(
            configured
                .channels
                .iter()
                .all(|channel| channel.diagnostic_capabilities.wire_break == expected_wire_break)
        );
    }

    let digital_fault =
        fixture.command(HardwareFaultAction::SetChannelFault(fixture.digital_input));
    fixture.engine.apply(digital_fault).expect("digital fault");
    let faulted = fixture.project(fixture.digital_input, ChannelRawValue::Bool(true));
    assert_eq!(faulted.cpu_value, ChannelRawValue::Bool(false));
    assert_eq!(faulted.quality, ChannelQuality::Bad);
    assert!(
        faulted
            .active_conditions
            .contains(&HardwareConditionKey::ChannelFault(fixture.digital_input))
    );
    let digital_output_fault =
        fixture.command(HardwareFaultAction::SetChannelFault(fixture.digital_output));
    fixture
        .engine
        .apply(digital_output_fault)
        .expect("digital output fault");
    let faulted_output = fixture.project(fixture.digital_output, ChannelRawValue::Bool(true));
    assert_eq!(faulted_output.cpu_value, ChannelRawValue::Bool(true));
    assert_eq!(faulted_output.delivered_value, ChannelRawValue::Bool(false));
    assert_eq!(faulted_output.quality, ChannelQuality::Bad);
    assert!(faulted_output.delivery_suppressed);
    let clear_digital_output = fixture.command(HardwareFaultAction::ClearChannelFault(
        fixture.digital_output,
    ));
    fixture
        .engine
        .apply(clear_digital_output)
        .expect("clear digital output fault");
    let unsupported = fixture.command(HardwareFaultAction::SetWireBreak(fixture.digital_input));
    let before = fixture.engine.snapshot();
    assert!(matches!(
        fixture.engine.apply(unsupported),
        Err(HardwareConditionError::CapabilityUnavailable(id)) if id == fixture.digital_input
    ));
    assert_eq!(fixture.engine.snapshot(), before, "rejection is atomic");

    let wire = fixture.command(HardwareFaultAction::SetWireBreak(fixture.analog_input));
    fixture.engine.apply(wire).expect("analog wire break");
    let analog_input = fixture.project(fixture.analog_input, ChannelRawValue::Int(12_345));
    assert_eq!(analog_input.cpu_value, ChannelRawValue::Int(0));
    assert_eq!(analog_input.quality, ChannelQuality::Bad);
    let output_wire = fixture.command(HardwareFaultAction::SetWireBreak(fixture.analog_output));
    fixture
        .engine
        .apply(output_wire)
        .expect("output wire break");
    let analog_output = fixture.project(fixture.analog_output, ChannelRawValue::Int(-123));
    assert_eq!(analog_output.cpu_value, ChannelRawValue::Int(-123));
    assert_eq!(analog_output.delivered_value, ChannelRawValue::Int(0));
    assert_eq!(analog_output.quality, ChannelQuality::Bad);
    assert!(analog_output.delivery_suppressed);

    let forced_bad = fixture
        .engine
        .project_channel(
            fixture.analog_output,
            NaturalChannelSample {
                raw_value: ChannelRawValue::Int(-123),
                provider_quality: ChannelQuality::Good,
                force_overlay_active: true,
            },
        )
        .expect("forced bad projection");
    assert_eq!(forced_bad.quality, ChannelQuality::Bad);
    assert!(forced_bad.force_overlay_active);

    let uncertain = fixture
        .engine
        .project_channel(
            fixture.digital_output,
            NaturalChannelSample {
                raw_value: ChannelRawValue::Bool(true),
                provider_quality: ChannelQuality::Uncertain,
                force_overlay_active: true,
            },
        )
        .expect("uncertain force projection");
    assert_eq!(uncertain.quality, ChannelQuality::Uncertain);
    assert!(
        uncertain.force_overlay_active,
        "force remains separate from quality"
    );
}

#[test]
fn station_link_and_vlink_interface_matrix_uses_one_in_memory_condition_path() {
    let mut fixture = fixture();
    let engineering_configuration = fixture.engine.configuration().clone();
    let network_configuration_fingerprint = fixture.engine.network().configuration_fingerprint();
    let station_off = fixture.command(HardwareFaultAction::SetStationAvailable {
        device_id: fixture.station_device,
        available: false,
    });
    fixture.engine.apply(station_off).expect("station off");
    let station_projection = fixture.project(fixture.analog_input, ChannelRawValue::Int(321));
    assert_eq!(station_projection.cpu_value, ChannelRawValue::Int(0));
    assert_eq!(station_projection.quality, ChannelQuality::NotPresent);
    assert!(station_projection.active_conditions.contains(
        &HardwareConditionKey::StationUnavailable(fixture.station_device)
    ));
    assert_eq!(
        fixture.engine.network().devices()[&fixture.station_device].powered_state,
        PoweredState::Unpowered
    );
    let station_output = fixture.project(fixture.analog_output, ChannelRawValue::Int(777));
    assert_eq!(station_output.delivered_value, ChannelRawValue::Int(0));
    assert_eq!(station_output.quality, ChannelQuality::NotPresent);
    assert!(station_output.delivery_suppressed);
    let station_on = fixture.command(HardwareFaultAction::SetStationAvailable {
        device_id: fixture.station_device,
        available: true,
    });
    fixture.engine.apply(station_on).expect("station on");

    let link_off = fixture.command(HardwareFaultAction::SetVirtualLinkAvailable {
        link_id: fixture.link,
        available: false,
    });
    fixture.engine.apply(link_off).expect("link off");
    let link_projection = fixture.project(fixture.analog_output, ChannelRawValue::Int(999));
    assert_eq!(link_projection.quality, ChannelQuality::NotPresent);
    assert!(link_projection.delivery_suppressed);
    assert!(
        link_projection
            .active_conditions
            .contains(&HardwareConditionKey::LinkUnavailable(fixture.link))
    );
    let link_on = fixture.command(HardwareFaultAction::SetVirtualLinkAvailable {
        link_id: fixture.link,
        available: true,
    });
    fixture.engine.apply(link_on).expect("link on");

    let pull_vlink = fixture.command(HardwareFaultAction::PullModule(fixture.vlink_module));
    let pull_receipt = fixture.engine.apply(pull_vlink).expect("pull vlink");
    assert!(pull_receipt.events.iter().any(|event| {
        event.condition == HardwareConditionKey::ModuleNotPresent(fixture.vlink_module)
    }));
    assert!(
        pull_receipt.events.iter().any(|event| {
            event.condition == HardwareConditionKey::LinkUnavailable(fixture.link)
        })
    );
    assert_eq!(
        fixture.engine.network().interfaces()[&fixture.provider_interface].runtime_state,
        RuntimeState::Unavailable
    );
    for port in fixture.provider_ports {
        assert_eq!(
            fixture.engine.network().ports()[&port].runtime_state,
            RuntimeState::Unavailable
        );
    }
    assert_eq!(
        fixture
            .engine
            .network()
            .effective_link_runtime_state(fixture.link)
            .expect("link"),
        RuntimeState::Unavailable
    );
    assert_eq!(
        fixture
            .project(fixture.analog_input, ChannelRawValue::Int(1))
            .quality,
        ChannelQuality::NotPresent
    );

    let power_off = fixture.command(HardwareFaultAction::SetControllerPowered {
        device_id: fixture.controller_device,
        powered: false,
    });
    fixture.engine.apply(power_off).expect("controller off");
    let restore_vlink = fixture.command(HardwareFaultAction::RestoreModule(fixture.vlink_module));
    fixture.engine.apply(restore_vlink).expect("restore vlink");
    assert_eq!(
        fixture.engine.network().interfaces()[&fixture.provider_interface].runtime_state,
        RuntimeState::Unavailable,
        "module restore cannot override an unavailable owner"
    );
    let power_on = fixture.command(HardwareFaultAction::SetControllerPowered {
        device_id: fixture.controller_device,
        powered: true,
    });
    fixture.engine.apply(power_on).expect("controller on");
    assert_eq!(
        fixture.engine.network().interfaces()[&fixture.provider_interface].runtime_state,
        RuntimeState::Available
    );
    assert_eq!(
        fixture
            .engine
            .network()
            .effective_link_runtime_state(fixture.link)
            .expect("link"),
        RuntimeState::Available
    );

    let wrong_vlink = fixture.command(HardwareFaultAction::InstallWrongModule {
        module_id: fixture.vlink_module,
        installed_catalog: ModuleCatalogId::Vdi16,
    });
    fixture.engine.apply(wrong_vlink).expect("wrong vlink");
    assert_eq!(
        fixture.engine.network().interfaces()[&fixture.provider_interface].runtime_state,
        RuntimeState::Unavailable
    );
    let restore_configured = fixture.command(HardwareFaultAction::RestoreConfiguredModule(
        fixture.vlink_module,
    ));
    fixture
        .engine
        .apply(restore_configured)
        .expect("restore configured vlink");
    assert_eq!(
        fixture.engine.network().interfaces()[&fixture.provider_interface].runtime_state,
        RuntimeState::Available
    );
    assert_eq!(fixture.engine.configuration(), &engineering_configuration);
    assert_eq!(
        fixture.engine.network().configuration_fingerprint(),
        network_configuration_fingerprint,
        "runtime commands cannot rewrite engineering/network configuration"
    );
}

#[test]
fn stale_or_conflicting_commands_and_diagnostic_acknowledgement_fail_closed() {
    let mut fixture = fixture();
    let stale = HardwareFaultCommand {
        idempotency_key: Uuid::deterministic_v4(b"stale", 1),
        expected_controller_epoch: 6,
        action: HardwareFaultAction::PullModule(fixture.digital_input_module),
    };
    let before = fixture.engine.snapshot();
    assert!(matches!(
        fixture.engine.apply(stale),
        Err(HardwareConditionError::StaleControllerEpoch {
            expected: 6,
            actual: 7
        })
    ));
    assert_eq!(fixture.engine.snapshot(), before);

    let command = fixture.command(HardwareFaultAction::PullModule(
        fixture.digital_input_module,
    ));
    fixture.engine.apply(command.clone()).expect("pull");
    let conflicting = HardwareFaultCommand {
        idempotency_key: command.idempotency_key,
        expected_controller_epoch: 7,
        action: HardwareFaultAction::PullModule(fixture.digital_output_module),
    };
    let before_conflict = fixture.engine.snapshot();
    assert!(matches!(
        fixture.engine.apply(conflicting),
        Err(HardwareConditionError::IdempotencyConflict(_))
    ));
    assert_eq!(fixture.engine.snapshot(), before_conflict);

    let condition = HardwareConditionKey::ModuleNotPresent(fixture.digital_input_module);
    let causal_before_ack = fixture
        .project(fixture.digital_input, ChannelRawValue::Bool(true))
        .causal_fingerprint;
    assert!(fixture.engine.acknowledge(condition).expect("ack"));
    assert!(
        fixture
            .engine
            .observed_conditions()
            .iter()
            .any(|observed| observed.condition == condition && observed.acknowledged)
    );
    assert_eq!(
        fixture
            .project(fixture.digital_input, ChannelRawValue::Bool(true))
            .causal_fingerprint,
        causal_before_ack,
        "acknowledgement cannot mutate the causal runtime condition"
    );
}

#[test]
fn snapshot_and_replay_preserve_exact_condition_history_and_causal_projection() {
    let mut fixture = fixture();
    let actions = [
        HardwareFaultAction::PullModule(fixture.digital_input_module),
        HardwareFaultAction::RestoreModule(fixture.digital_input_module),
        HardwareFaultAction::SetChannelFault(fixture.digital_input),
        HardwareFaultAction::ClearChannelFault(fixture.digital_input),
        HardwareFaultAction::SetWireBreak(fixture.analog_input),
        HardwareFaultAction::ClearWireBreak(fixture.analog_input),
        HardwareFaultAction::SetStationAvailable {
            device_id: fixture.station_device,
            available: false,
        },
        HardwareFaultAction::SetStationAvailable {
            device_id: fixture.station_device,
            available: true,
        },
        HardwareFaultAction::SetVirtualLinkAvailable {
            link_id: fixture.link,
            available: false,
        },
        HardwareFaultAction::SetVirtualLinkAvailable {
            link_id: fixture.link,
            available: true,
        },
    ];
    let commands: Vec<_> = actions
        .into_iter()
        .map(|action| fixture.command(action))
        .collect();
    for command in commands.clone() {
        fixture.engine.apply(command).expect("live command");
    }
    let replayed = HardwareConditionEngine::replay(
        fixture.profile.pin(),
        fixture.configuration.clone(),
        fixture.pristine_network.clone(),
        7,
        commands,
    )
    .expect("replay");
    assert_eq!(fixture.engine.snapshot(), replayed.snapshot());
    let live_projection = fixture.project(fixture.analog_input, ChannelRawValue::Int(-3_210));
    let replay_projection = replayed
        .project_channel(
            fixture.analog_input,
            NaturalChannelSample {
                raw_value: ChannelRawValue::Int(-3_210),
                provider_quality: ChannelQuality::Good,
                force_overlay_active: false,
            },
        )
        .expect("replay projection");
    assert_eq!(live_projection, replay_projection);
}
