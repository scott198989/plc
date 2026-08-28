#![allow(clippy::too_many_lines)]

use std::collections::BTreeMap;

use plc_hardware::{
    Address, AddressIntent, AddressRequest, BindingKind, CanonicalType, ChannelId,
    ConfiguredModule, ConfiguredState, ControllerCatalogId, ControllerConfig, ControllerId,
    Declaration, DeclarationId, DeclarationKind, DeviceRole, DiagnosticCode, DiscoveryFilter,
    Identifier, InstalledOccupant, ModuleCatalogId, ModuleId, Namespace, PlcValue, PortClass,
    PoweredState, ProfileAllowlist, RackConfig, RackOwner, RackSlot, ReferenceState, RetainPolicy,
    RuntimeState, Scope, ScopeId, ScopeKind, SourceIdentity, SourceObjectId, SymbolAddressArea,
    SymbolError, SymbolUniverse, Tag, TagId, TagKind, TagTable, TagTableId, TrainingProfile, Uuid,
    VirtualDevice, VirtualDeviceId, VirtualDeviceName, VirtualInterface, VirtualInterfaceId,
    VirtualIpAddress, VirtualLink, VirtualLinkId, VirtualNetwork, VirtualPort, VirtualPortId,
    VirtualSubnet, VirtualSubnetId,
};

struct Ids {
    next: u64,
}

impl Ids {
    fn new() -> Self {
        Self { next: 1 }
    }

    fn next<T: From<Uuid>>(&mut self) -> T {
        let value = Uuid::deterministic_v4(b"plc-hardware-contract-fixture", self.next);
        self.next += 1;
        T::from(value)
    }
}

struct CompactFixture {
    profile: TrainingProfile,
    project: plc_hardware::HardwareProject,
    controller_id: ControllerId,
    input_module_id: ModuleId,
    ids: Ids,
}

fn compact_fixture() -> CompactFixture {
    let profile = TrainingProfile::edu21();
    let mut ids = Ids::new();
    let controller_id = ids.next();
    let device_id = ids.next();
    let subnet_id = ids.next();
    let interface_id = ids.next();
    let port_id = ids.next();

    let mut network = VirtualNetwork::new();
    network
        .add_device(VirtualDevice {
            id: device_id,
            creation_ordinal: 1,
            device_name: VirtualDeviceName::parse("compact-a").expect("name"),
            powered_state: PoweredState::Powered,
        })
        .expect("device");
    network
        .add_subnet(VirtualSubnet {
            id: subnet_id,
            creation_ordinal: 1,
            name: "training-cell".to_owned(),
            network_address: VirtualIpAddress::parse("10.21.0.0").expect("network"),
            prefix_length: 24,
        })
        .expect("subnet");
    network
        .add_interface(VirtualInterface {
            id: interface_id,
            creation_ordinal: 1,
            owner_device_id: device_id,
            provider_module_id: None,
            name: "edu-link-1".to_owned(),
            address: Some(VirtualIpAddress::parse("10.21.0.1").expect("address")),
            subnet_id: Some(subnet_id),
            port_class: PortClass::EduLink,
            role: DeviceRole::Controller,
            configured_state: ConfiguredState::Enabled,
            runtime_state: RuntimeState::Available,
        })
        .expect("interface");
    network
        .add_port(VirtualPort {
            id: port_id,
            creation_ordinal: 1,
            owner_interface_id: interface_id,
            name: "port-1".to_owned(),
            configured_state: ConfiguredState::Enabled,
            runtime_state: RuntimeState::Available,
        })
        .expect("port");

    let input_module_id = ids.next();
    let input_channels: Vec<ChannelId> = (0..16).map(|_| ids.next()).collect();
    let input = ConfiguredModule::from_catalog(
        &profile,
        input_module_id,
        10,
        ModuleCatalogId::Vdi16,
        &input_channels,
        &[],
        AddressRequest::Auto,
        AddressRequest::NotUsed,
    )
    .expect("input module");
    let output_module_id = ids.next();
    let output_channels: Vec<ChannelId> = (0..16).map(|_| ids.next()).collect();
    let output = ConfiguredModule::from_catalog(
        &profile,
        output_module_id,
        20,
        ModuleCatalogId::Vdo16,
        &output_channels,
        &[],
        AddressRequest::NotUsed,
        AddressRequest::Auto,
    )
    .expect("output module");

    let rack_id = ids.next();
    let mut slots = BTreeMap::new();
    for number in 1..=8 {
        let installed = match number {
            1 => Some(InstalledOccupant::Module(input.clone())),
            2 => Some(InstalledOccupant::Module(output.clone())),
            _ => None,
        };
        slots.insert(
            number,
            RackSlot {
                id: ids.next(),
                number,
                installed,
            },
        );
    }
    let controller = ControllerConfig {
        id: controller_id,
        creation_ordinal: 1,
        catalog_id: ControllerCatalogId::VctrlC1,
        virtual_device_id: device_id,
        local_rack: RackConfig {
            id: rack_id,
            creation_ordinal: 1,
            owner: RackOwner::Controller(controller_id),
            slots,
        },
        reserved_input_spans: Vec::new(),
        reserved_output_spans: Vec::new(),
        configured_block_count: 0,
    };
    let mut project = plc_hardware::HardwareProject::new(profile.pin(), network);
    project.add_controller(controller).expect("controller");
    CompactFixture {
        profile,
        project,
        controller_id,
        input_module_id,
        ids,
    }
}

#[test]
fn shipped_profile_and_hardware_allocation_build_byte_stable_fingerprints() {
    let mut fixture = compact_fixture();
    assert_eq!(
        ProfileAllowlist::load(&fixture.profile.pin()),
        Ok(fixture.profile.clone())
    );
    assert!(
        fixture
            .project
            .validate(&fixture.profile)
            .iter()
            .any(|diagnostic| {
                diagnostic.code == DiagnosticCode::ChannelConfiguration
                    && diagnostic.message.contains("AUTO")
            })
    );

    let preview = fixture
        .project
        .preview_auto_allocate(&fixture.profile)
        .expect("allocation preview");
    assert_eq!(preview.changes.len(), 2);
    fixture
        .project
        .commit_auto_allocate(&fixture.profile, &preview)
        .expect("atomic allocation commit");
    let artifact = fixture
        .project
        .build(&fixture.profile)
        .expect("hardware build");
    assert_eq!(
        artifact.hardware_fingerprint.to_hex(),
        "a665253219aac1ff9b7c127a4dca68d65269a3cf1fc89210385812a8e06f09e4"
    );
    assert_eq!(
        artifact.network_configuration_fingerprint.to_hex(),
        "937a13c694a26f643ca5a98cb493d143eea625eaace347c2941b4a83c2964869"
    );
    assert_eq!(artifact.channel_bindings.len(), 32);
    assert_eq!(
        artifact.hardware_fingerprint,
        fixture.project.configuration_fingerprint()
    );
    assert_eq!(
        fixture.project.build(&fixture.profile),
        Ok(artifact.clone())
    );

    let new_module_id = fixture.ids.next();
    let channels: Vec<ChannelId> = (0..16).map(|_| fixture.ids.next()).collect();
    let module = ConfiguredModule::from_catalog(
        &fixture.profile,
        new_module_id,
        30,
        ModuleCatalogId::Vdi16,
        &channels,
        &[],
        AddressRequest::Auto,
        AddressRequest::NotUsed,
    )
    .expect("new module");
    fixture
        .project
        .controller_mut(fixture.controller_id)
        .expect("controller")
        .local_rack
        .slots
        .get_mut(&3)
        .expect("slot")
        .installed = Some(InstalledOccupant::Module(module));
    let second_preview = fixture
        .project
        .preview_auto_allocate(&fixture.profile)
        .expect("incremental allocation");
    assert_eq!(second_preview.changes.len(), 1);
    assert_eq!(second_preview.changes[0].module_id, new_module_id);
    assert_eq!(second_preview.changes[0].proposed.start_byte, 2);
    assert_ne!(second_preview.changes[0].module_id, fixture.input_module_id);
}

#[test]
fn illegal_placement_and_overlapping_spans_have_navigable_blocking_diagnostics() {
    let mut fixture = compact_fixture();
    let preview = fixture
        .project
        .preview_auto_allocate(&fixture.profile)
        .expect("preview");
    fixture
        .project
        .commit_auto_allocate(&fixture.profile, &preview)
        .expect("commit");

    let overlapping_id = fixture.ids.next();
    let channels: Vec<ChannelId> = (0..16).map(|_| fixture.ids.next()).collect();
    let overlapping = ConfiguredModule::from_catalog(
        &fixture.profile,
        overlapping_id,
        30,
        ModuleCatalogId::Vdi16,
        &channels,
        &[],
        AddressRequest::Explicit(0),
        AddressRequest::NotUsed,
    )
    .expect("overlapping module");
    fixture
        .project
        .controller_mut(fixture.controller_id)
        .expect("controller")
        .local_rack
        .slots
        .get_mut(&3)
        .expect("slot")
        .installed = Some(InstalledOccupant::Module(overlapping));
    let diagnostics = fixture.project.validate(&fixture.profile);
    let conflict = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagnosticCode::AddressConflict)
        .expect("address conflict");
    assert!(conflict.blocking);
    assert!(conflict.related.len() >= 2);
    assert!(fixture.project.build(&fixture.profile).is_err());

    let power = ConfiguredModule::from_catalog(
        &fixture.profile,
        fixture.ids.next(),
        40,
        ModuleCatalogId::Vpwr1,
        &[],
        &[],
        AddressRequest::NotUsed,
        AddressRequest::NotUsed,
    )
    .expect("power");
    fixture
        .project
        .controller_mut(fixture.controller_id)
        .expect("controller")
        .local_rack
        .slots
        .get_mut(&4)
        .expect("slot")
        .installed = Some(InstalledOccupant::Module(power));
    assert!(
        fixture
            .project
            .validate(&fixture.profile)
            .iter()
            .any(|diagnostic| { diagnostic.code == DiagnosticCode::IllegalPlacementOrCatalog })
    );
}

fn linked_network(link_state: RuntimeState) -> (VirtualNetwork, VirtualInterfaceId, VirtualLinkId) {
    let mut ids = Ids::new();
    let controller_device: VirtualDeviceId = ids.next();
    let station_device: VirtualDeviceId = ids.next();
    let subnet_id: VirtualSubnetId = ids.next();
    let controller_interface: VirtualInterfaceId = ids.next();
    let station_interface: VirtualInterfaceId = ids.next();
    let controller_port: VirtualPortId = ids.next();
    let station_port: VirtualPortId = ids.next();
    let spare_station_port: VirtualPortId = ids.next();
    let link_id: VirtualLinkId = ids.next();
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
    network
        .add_subnet(VirtualSubnet {
            id: subnet_id,
            creation_ordinal: 1,
            name: "cell".to_owned(),
            network_address: VirtualIpAddress::parse("192.0.2.0").expect("network"),
            prefix_length: 24,
        })
        .expect("subnet");
    for (id, owner, ordinal, address, role) in [
        (
            controller_interface,
            controller_device,
            1,
            "192.0.2.1",
            DeviceRole::Controller,
        ),
        (
            station_interface,
            station_device,
            2,
            "192.0.2.2",
            DeviceRole::Station,
        ),
    ] {
        network
            .add_interface(VirtualInterface {
                id,
                creation_ordinal: ordinal,
                owner_device_id: owner,
                provider_module_id: None,
                name: format!("if-{ordinal}"),
                address: Some(VirtualIpAddress::parse(address).expect("address")),
                subnet_id: Some(subnet_id),
                port_class: PortClass::EduLink,
                role,
                configured_state: ConfiguredState::Enabled,
                runtime_state: RuntimeState::Available,
            })
            .expect("interface");
    }
    for (id, owner, ordinal) in [
        (controller_port, controller_interface, 1),
        (station_port, station_interface, 2),
        (spare_station_port, station_interface, 3),
    ] {
        network
            .add_port(VirtualPort {
                id,
                creation_ordinal: ordinal,
                owner_interface_id: owner,
                name: format!("p-{ordinal}"),
                configured_state: ConfiguredState::Enabled,
                runtime_state: RuntimeState::Available,
            })
            .expect("port");
    }
    network
        .add_link(VirtualLink {
            id: link_id,
            creation_ordinal: 1,
            endpoint_port_ids: [controller_port, station_port],
            configured_state: ConfiguredState::Enabled,
            runtime_state: link_state,
        })
        .expect("link");
    (network, controller_interface, link_id)
}

#[test]
fn virtual_discovery_is_deterministic_inert_and_runtime_state_is_separate() {
    let (mut network, query, link) = linked_network(RuntimeState::Available);
    assert!(network.validate_configuration().is_empty());
    let configuration_before = network.configuration_fingerprint();
    let state_before = network.state_fingerprint();
    let visible = network
        .query_visible_devices(query, &DiscoveryFilter::default())
        .expect("discovery");
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].device_name.as_str(), "station-a");

    network
        .set_link_runtime_state(link, RuntimeState::Unavailable)
        .expect("runtime fault");
    assert_eq!(network.configuration_fingerprint(), configuration_before);
    assert_ne!(network.state_fingerprint(), state_before);
    assert!(
        network
            .query_visible_devices(query, &DiscoveryFilter::default())
            .expect("discovery under virtual fault")
            .is_empty()
    );
}

fn declaration(ids: &mut Ids, name: &str, scope_id: ScopeId, kind: DeclarationKind) -> Declaration {
    Declaration {
        id: ids.next(),
        creation_ordinal: ids.next,
        name: Identifier::parse(name).expect("identifier"),
        scope_id,
        namespace: kind.expected_namespace(),
        kind,
        member_scope_id: None,
        deleted: false,
    }
}

#[test]
fn symbol_bindings_survive_rename_and_delete_without_unsafe_name_rebinding() {
    let profile = TrainingProfile::edu21();
    let mut ids = Ids::new();
    let controller_id: ControllerId = ids.next();
    let global_scope: ScopeId = ids.next();
    let block_scope: ScopeId = ids.next();
    let block_id: DeclarationId = ids.next();
    let mut symbols = SymbolUniverse::new(profile.pin());
    symbols
        .add_scope(Scope {
            id: global_scope,
            creation_ordinal: 1,
            kind: ScopeKind::ControllerGlobal(controller_id),
            parent_scope_id: None,
        })
        .expect("global scope");
    symbols
        .add_scope(Scope {
            id: block_scope,
            creation_ordinal: 2,
            kind: ScopeKind::Block {
                controller_id,
                block_id,
            },
            parent_scope_id: Some(global_scope),
        })
        .expect("block scope");

    let speed = declaration(&mut ids, "Speed", global_scope, DeclarationKind::GlobalTag);
    symbols.add_declaration(speed.clone()).expect("global tag");
    let shadow = declaration(
        &mut ids,
        "speed",
        block_scope,
        DeclarationKind::BlockValue(plc_hardware::BlockValueRole::Temp),
    );
    assert_eq!(
        symbols.add_declaration(shadow),
        Err(SymbolError::ShadowingProhibited)
    );

    let motor = declaration(&mut ids, "Motor", global_scope, DeclarationKind::GlobalTag);
    symbols.add_declaration(motor.clone()).expect("motor");
    let reference_id = ids.next();
    symbols
        .create_reference(
            reference_id,
            1,
            &["Motor"],
            Namespace::Value,
            block_scope,
            SourceIdentity {
                object_id: ids.next::<SourceObjectId>(),
                location: "network/1/contact/2".to_owned(),
            },
            BindingKind::Read,
        )
        .expect("reference");
    let rename = symbols.preview_rename(motor.id, "Drive").expect("preview");
    assert_eq!(rename.affected_reference_ids, vec![reference_id]);
    symbols.commit_rename(&rename).expect("rename");
    let ReferenceState::Resolved(binding) = &symbols.references()[&reference_id].state else {
        panic!("reference should remain resolved");
    };
    assert_eq!(binding.target_id, motor.id);
    assert_eq!(binding.display_path[0].as_str(), "Drive");

    symbols.delete_declaration(motor.id).expect("delete");
    assert!(matches!(
        symbols.references()[&reference_id].state,
        ReferenceState::StaleDeleted(_)
    ));
    let replacement = declaration(&mut ids, "Drive", global_scope, DeclarationKind::GlobalTag);
    symbols
        .add_declaration(replacement.clone())
        .expect("replacement");
    assert!(matches!(
        symbols.references()[&reference_id].state,
        ReferenceState::StaleDeleted(_)
    ));
    assert!(
        symbols
            .validate_references()
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::StaleOrDeletedTarget)
    );
    let before_failed_restore = symbols.semantic_fingerprint();
    assert_eq!(
        symbols.restore_declaration(motor.id),
        Err(SymbolError::DuplicateName)
    );
    assert_eq!(symbols.semantic_fingerprint(), before_failed_restore);
    assert!(matches!(
        symbols.references()[&reference_id].state,
        ReferenceState::StaleDeleted(_)
    ));
    symbols
        .rebind_reference(reference_id)
        .expect("explicit rebind");
    let ReferenceState::Resolved(binding) = &symbols.references()[&reference_id].state else {
        panic!("explicitly rebound reference");
    };
    assert_eq!(binding.target_id, replacement.id);

    let unresolved_id = ids.next();
    let unresolved = symbols
        .create_reference(
            unresolved_id,
            2,
            &["MissingValue"],
            Namespace::Value,
            block_scope,
            SourceIdentity {
                object_id: ids.next::<SourceObjectId>(),
                location: "network/2/contact/1".to_owned(),
            },
            BindingKind::Read,
        )
        .expect("unresolved reference draft");
    assert_eq!(unresolved, plc_hardware::Resolution::Unresolved);
    assert!(symbols.validate_references().iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::UnresolvedReference
            && diagnostic.primary.id == unresolved_id.uuid()
    }));
}

#[test]
fn tag_auto_allocation_constants_and_overlap_diagnostics_are_causal() {
    let mut fixture = compact_fixture();
    let hardware_preview = fixture
        .project
        .preview_auto_allocate(&fixture.profile)
        .expect("hardware allocation");
    fixture
        .project
        .commit_auto_allocate(&fixture.profile, &hardware_preview)
        .expect("hardware commit");
    let artifact = fixture.project.build(&fixture.profile).expect("artifact");

    let mut ids = fixture.ids;
    let global_scope: ScopeId = ids.next();
    let table_id: TagTableId = ids.next();
    let mut symbols = SymbolUniverse::new(fixture.profile.pin());
    symbols
        .add_scope(Scope {
            id: global_scope,
            creation_ordinal: 1,
            kind: ScopeKind::ControllerGlobal(fixture.controller_id),
            parent_scope_id: None,
        })
        .expect("scope");
    symbols
        .add_tag_table(TagTable {
            id: table_id,
            controller_id: fixture.controller_id,
            creation_ordinal: 1,
            name: Identifier::parse("DefaultTags").expect("name"),
            is_default: true,
        })
        .expect("table");

    let constant_declaration = declaration(
        &mut ids,
        "AlwaysEnabled",
        global_scope,
        DeclarationKind::GlobalConstant,
    );
    symbols
        .add_declaration(constant_declaration.clone())
        .expect("constant declaration");
    symbols
        .add_tag(Tag {
            id: ids.next(),
            declaration_id: constant_declaration.id,
            controller_id: fixture.controller_id,
            creation_ordinal: 0,
            table_id,
            name: constant_declaration.name.clone(),
            declared_type: CanonicalType::Primitive(plc_hardware::PrimitiveType::Bool),
            address_intent: AddressIntent::None,
            allocated_address: None,
            comment: "Compile-time value".to_owned(),
            start_value: None,
            retain_policy: RetainPolicy::NonRetentive,
            display_format: String::new(),
            kind: TagKind::Constant(PlcValue::Bool(true)),
            hardware_channel_id: None,
        })
        .expect("constant");

    let marker_declaration = declaration(
        &mut ids,
        "MarkerReady",
        global_scope,
        DeclarationKind::GlobalTag,
    );
    symbols
        .add_declaration(marker_declaration.clone())
        .expect("declaration");
    let marker_tag_id: TagId = ids.next();
    symbols
        .add_tag(Tag {
            id: marker_tag_id,
            declaration_id: marker_declaration.id,
            controller_id: fixture.controller_id,
            creation_ordinal: 1,
            table_id,
            name: marker_declaration.name.clone(),
            declared_type: CanonicalType::Primitive(plc_hardware::PrimitiveType::Bool),
            address_intent: AddressIntent::Auto(SymbolAddressArea::Marker),
            allocated_address: None,
            comment: "presentation only".to_owned(),
            start_value: Some(PlcValue::Bool(false)),
            retain_policy: RetainPolicy::Retentive,
            display_format: "default".to_owned(),
            kind: TagKind::Variable,
            hardware_channel_id: None,
        })
        .expect("tag");

    let input_declaration = declaration(
        &mut ids,
        "InputReady",
        global_scope,
        DeclarationKind::GlobalTag,
    );
    symbols
        .add_declaration(input_declaration.clone())
        .expect("input declaration");
    let input_tag_id: TagId = ids.next();
    symbols
        .add_tag(Tag {
            id: input_tag_id,
            declaration_id: input_declaration.id,
            controller_id: fixture.controller_id,
            creation_ordinal: 2,
            table_id,
            name: input_declaration.name.clone(),
            declared_type: CanonicalType::Primitive(plc_hardware::PrimitiveType::Bool),
            address_intent: AddressIntent::Auto(SymbolAddressArea::Input),
            allocated_address: None,
            comment: String::new(),
            start_value: None,
            retain_policy: RetainPolicy::NonRetentive,
            display_format: String::new(),
            kind: TagKind::Variable,
            hardware_channel_id: None,
        })
        .expect("input tag");

    let tag_preview = symbols
        .preview_auto_allocate_tags(&fixture.profile, &fixture.project, &artifact)
        .expect("tag preview");
    assert_eq!(tag_preview.changes.len(), 2);
    symbols
        .commit_auto_allocate_tags(&fixture.profile, &fixture.project, &artifact, &tag_preview)
        .expect("tag commit");
    assert!(
        symbols
            .validate_tags(&fixture.profile, &fixture.project, &artifact)
            .is_empty()
    );
    assert_eq!(
        symbols.tags()[&marker_tag_id].allocated_address,
        Some(Address::MarkerBit { byte: 0, bit: 0 })
    );
    assert_eq!(
        symbols.tags()[&input_tag_id].allocated_address,
        Some(Address::InputBit { byte: 0, bit: 0 })
    );

    let organizational_table_id: TagTableId = ids.next();
    symbols
        .add_tag_table(TagTable {
            id: organizational_table_id,
            controller_id: fixture.controller_id,
            creation_ordinal: 2,
            name: Identifier::parse("OrganizedTags").expect("name"),
            is_default: false,
        })
        .expect("organizational table");
    let before_table_move = symbols.semantic_fingerprint();
    assert!(
        symbols
            .move_tag_to_table(marker_tag_id, organizational_table_id)
            .expect("table move")
    );
    assert_eq!(symbols.semantic_fingerprint(), before_table_move);

    let overlap_declaration = declaration(
        &mut ids,
        "MarkerByte",
        global_scope,
        DeclarationKind::GlobalTag,
    );
    symbols
        .add_declaration(overlap_declaration.clone())
        .expect("overlap declaration");
    symbols
        .add_tag(Tag {
            id: ids.next(),
            declaration_id: overlap_declaration.id,
            controller_id: fixture.controller_id,
            creation_ordinal: 3,
            table_id,
            name: overlap_declaration.name.clone(),
            declared_type: CanonicalType::Primitive(plc_hardware::PrimitiveType::Byte),
            address_intent: AddressIntent::explicit("MB0"),
            allocated_address: Some(Address::MarkerByte { byte: 0 }),
            comment: String::new(),
            start_value: None,
            retain_policy: RetainPolicy::NonRetentive,
            display_format: String::new(),
            kind: TagKind::Variable,
            hardware_channel_id: None,
        })
        .expect("overlap tag");
    assert!(
        symbols
            .validate_tags(&fixture.profile, &fixture.project, &artifact)
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::SymbolOverlap)
    );

    let malformed_declaration = declaration(
        &mut ids,
        "MalformedAddress",
        global_scope,
        DeclarationKind::GlobalTag,
    );
    symbols
        .add_declaration(malformed_declaration.clone())
        .expect("malformed declaration");
    symbols
        .add_tag(Tag {
            id: ids.next(),
            declaration_id: malformed_declaration.id,
            controller_id: fixture.controller_id,
            creation_ordinal: 4,
            table_id,
            name: malformed_declaration.name.clone(),
            declared_type: CanonicalType::Primitive(plc_hardware::PrimitiveType::Bool),
            address_intent: AddressIntent::explicit("http://127.0.0.1:80"),
            allocated_address: None,
            comment: String::new(),
            start_value: None,
            retain_policy: RetainPolicy::NonRetentive,
            display_format: String::new(),
            kind: TagKind::Variable,
            hardware_channel_id: None,
        })
        .expect("malformed tag draft");
    assert!(
        symbols
            .validate_tags(&fixture.profile, &fixture.project, &artifact)
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::MalformedPlcAddress)
    );

    let unaddressed_retain_declaration = declaration(
        &mut ids,
        "UnaddressedRetain",
        global_scope,
        DeclarationKind::GlobalTag,
    );
    symbols
        .add_declaration(unaddressed_retain_declaration.clone())
        .expect("retained declaration");
    let unaddressed_retain_id: TagId = ids.next();
    symbols
        .add_tag(Tag {
            id: unaddressed_retain_id,
            declaration_id: unaddressed_retain_declaration.id,
            controller_id: fixture.controller_id,
            creation_ordinal: 5,
            table_id,
            name: unaddressed_retain_declaration.name.clone(),
            declared_type: CanonicalType::Primitive(plc_hardware::PrimitiveType::Bool),
            address_intent: AddressIntent::None,
            allocated_address: None,
            comment: String::new(),
            start_value: None,
            retain_policy: RetainPolicy::Retentive,
            display_format: String::new(),
            kind: TagKind::Variable,
            hardware_channel_id: None,
        })
        .expect("retained tag draft");
    assert!(
        symbols
            .validate_tags(&fixture.profile, &fixture.project, &artifact)
            .iter()
            .any(|diagnostic| {
                diagnostic.primary.id == unaddressed_retain_id.uuid()
                    && diagnostic.primary.field.as_deref() == Some("retainPolicy")
            })
    );
}

#[test]
fn capability_provider_and_admission_limits_block_invalid_mutations() {
    let mut capability_fixture = compact_fixture();
    let channel_id = {
        let slot = capability_fixture
            .project
            .controller_mut(capability_fixture.controller_id)
            .expect("controller")
            .local_rack
            .slots
            .get_mut(&1)
            .expect("input slot");
        let Some(InstalledOccupant::Module(module)) = &mut slot.installed else {
            panic!("input module");
        };
        module.channels[0].diagnostic_capabilities.wire_break = true;
        module.channels[0].id
    };
    assert!(
        capability_fixture
            .project
            .validate(&capability_fixture.profile)
            .iter()
            .any(|diagnostic| {
                diagnostic.code == DiagnosticCode::ChannelConfiguration
                    && diagnostic.primary.id == channel_id.uuid()
            })
    );

    let mut provider_fixture = compact_fixture();
    let provider_owner_device_id =
        provider_fixture.project.controllers()[&provider_fixture.controller_id].virtual_device_id;
    provider_fixture
        .project
        .network_mut()
        .add_interface(VirtualInterface {
            id: provider_fixture.ids.next(),
            creation_ordinal: 99,
            owner_device_id: provider_owner_device_id,
            provider_module_id: Some(provider_fixture.input_module_id),
            name: "invalid-provider".to_owned(),
            address: None,
            subnet_id: None,
            port_class: PortClass::EduLink,
            role: DeviceRole::Controller,
            configured_state: ConfiguredState::Disabled,
            runtime_state: RuntimeState::Available,
        })
        .expect("provider draft");
    assert!(
        provider_fixture
            .project
            .validate(&provider_fixture.profile)
            .iter()
            .any(|diagnostic| {
                diagnostic.code == DiagnosticCode::RequiredComponentMissing
                    && diagnostic.primary.field.as_deref() == Some("providerModuleId")
            })
    );

    let mut capacity_fixture = compact_fixture();
    for creation_ordinal in 2..=8 {
        let controller_id: ControllerId = capacity_fixture.ids.next();
        let slots = (1..=8)
            .map(|number| {
                (
                    number,
                    RackSlot {
                        id: capacity_fixture.ids.next(),
                        number,
                        installed: None,
                    },
                )
            })
            .collect();
        capacity_fixture
            .project
            .add_controller(ControllerConfig {
                id: controller_id,
                creation_ordinal,
                catalog_id: ControllerCatalogId::VctrlC1,
                virtual_device_id: capacity_fixture.ids.next(),
                local_rack: RackConfig {
                    id: capacity_fixture.ids.next(),
                    creation_ordinal,
                    owner: RackOwner::Controller(controller_id),
                    slots,
                },
                reserved_input_spans: Vec::new(),
                reserved_output_spans: Vec::new(),
                configured_block_count: 0,
            })
            .expect("controller inside profile limit");
    }
    let ninth_id: ControllerId = capacity_fixture.ids.next();
    let ninth = ControllerConfig {
        id: ninth_id,
        creation_ordinal: 9,
        catalog_id: ControllerCatalogId::VctrlC1,
        virtual_device_id: capacity_fixture.ids.next(),
        local_rack: RackConfig {
            id: capacity_fixture.ids.next(),
            creation_ordinal: 9,
            owner: RackOwner::Controller(ninth_id),
            slots: BTreeMap::new(),
        },
        reserved_input_spans: Vec::new(),
        reserved_output_spans: Vec::new(),
        configured_block_count: 0,
    };
    assert!(matches!(
        capacity_fixture.project.add_controller(ninth),
        Err(plc_hardware::HardwareError::ControllerCapacity { maximum: 8 })
    ));
    assert_eq!(capacity_fixture.project.controllers().len(), 8);
}

#[test]
fn generated_illegal_slot_and_address_corpus_never_builds() {
    for illegal_slot in [0_u8, 9, 16, 31, u8::MAX] {
        let mut fixture = compact_fixture();
        let channels: Vec<ChannelId> = (0..16).map(|_| fixture.ids.next()).collect();
        let module = ConfiguredModule::from_catalog(
            &fixture.profile,
            fixture.ids.next(),
            100,
            ModuleCatalogId::Vdi16,
            &channels,
            &[],
            AddressRequest::Explicit(u32::from(illegal_slot)),
            AddressRequest::NotUsed,
        )
        .expect("catalog module");
        fixture
            .project
            .controller_mut(fixture.controller_id)
            .expect("controller")
            .local_rack
            .slots
            .insert(
                illegal_slot,
                RackSlot {
                    id: fixture.ids.next(),
                    number: illegal_slot,
                    installed: Some(InstalledOccupant::Module(module)),
                },
            );
        let diagnostics = fixture.project.validate(&fixture.profile);
        assert!(!diagnostics.is_empty(), "slot {illegal_slot}");
        assert!(fixture.project.build(&fixture.profile).is_err());
    }

    for illegal_start in [1_u32, 1_018, u32::MAX] {
        let mut fixture = compact_fixture();
        let channels: Vec<ChannelId> = (0..4).map(|_| fixture.ids.next()).collect();
        let parameters = (0..4).map(|_| fixture.ids.next()).collect::<Vec<_>>();
        let module = ConfiguredModule::from_catalog(
            &fixture.profile,
            fixture.ids.next(),
            100,
            ModuleCatalogId::Vai4,
            &channels,
            &parameters,
            AddressRequest::Explicit(illegal_start),
            AddressRequest::NotUsed,
        )
        .expect("catalog module");
        fixture
            .project
            .controller_mut(fixture.controller_id)
            .expect("controller")
            .local_rack
            .slots
            .get_mut(&3)
            .expect("legal slot")
            .installed = Some(InstalledOccupant::Module(module));
        let diagnostics = fixture.project.validate(&fixture.profile);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == DiagnosticCode::AddressConflict),
            "start {illegal_start}"
        );
        assert!(fixture.project.build(&fixture.profile).is_err());
    }
}
