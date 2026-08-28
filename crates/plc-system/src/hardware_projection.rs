use std::collections::BTreeMap;

use plc_core::{
    Lifecycle, ObjectId, PayloadValue, Project, ProjectObject, ProjectObjectKind, Sha256Digest,
    Uuid,
};
use plc_hardware::{
    AddressRequest, AllocationPreview, ChannelId, ChannelLayout, ConfiguredModule, ConfiguredState,
    ControllerCatalogId, ControllerConfig, ControllerId, DeviceRole,
    Diagnostic as HardwareDiagnostic, HardwareArtifact, HardwareError, HardwareProject,
    InstalledOccupant, ModuleCatalogId, ModuleId, ParameterId, PortClass, PoweredState, RackConfig,
    RackId, RackOwner, RackSlot, RuntimeState, SlotId, TrainingProfile, VirtualDevice,
    VirtualDeviceId, VirtualDeviceName, VirtualInterface, VirtualInterfaceId, VirtualIpAddress,
    VirtualNetwork, VirtualPort, VirtualPortId, VirtualSubnet, VirtualSubnetId,
};

const MAX_CANONICAL_CONTROLLERS: usize = 64;
const NETWORK_PREFIX: &str = "192.0.2.0";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProjectDiagnosticPhase {
    CanonicalProjection,
    Hardware,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProjectDiagnostic {
    pub blocking: bool,
    pub code: String,
    pub message: String,
    pub phase: ProjectDiagnosticPhase,
    pub primary_object_id: ObjectId,
    pub related_object_ids: Vec<ObjectId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalHardwareProjection {
    source_document_hash: Sha256Digest,
    source_semantic_fingerprint: Sha256Digest,
    profile: TrainingProfile,
    hardware_project: HardwareProject,
    allocation_preview: Option<AllocationPreview>,
    artifact: Option<HardwareArtifact>,
    diagnostics: Vec<ProjectDiagnostic>,
}

impl CanonicalHardwareProjection {
    #[must_use]
    pub const fn source_document_hash(&self) -> Sha256Digest {
        self.source_document_hash
    }

    #[must_use]
    pub const fn source_semantic_fingerprint(&self) -> Sha256Digest {
        self.source_semantic_fingerprint
    }

    #[must_use]
    pub const fn profile(&self) -> &TrainingProfile {
        &self.profile
    }

    #[must_use]
    pub const fn hardware_project(&self) -> &HardwareProject {
        &self.hardware_project
    }

    #[must_use]
    pub const fn allocation_preview(&self) -> Option<&AllocationPreview> {
        self.allocation_preview.as_ref()
    }

    #[must_use]
    pub const fn artifact(&self) -> Option<&HardwareArtifact> {
        self.artifact.as_ref()
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[ProjectDiagnostic] {
        &self.diagnostics
    }

    #[must_use]
    pub fn can_build(&self) -> bool {
        self.artifact.is_some()
            && !self
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.blocking)
    }
}

#[must_use]
pub fn project_hardware(project: &Project) -> CanonicalHardwareProjection {
    let profile = TrainingProfile::edu21();
    let mut diagnostics = Vec::new();
    validate_profile_pin(project, &profile, &mut diagnostics);

    let active: Vec<_> = project
        .objects()
        .filter(|object| object.lifecycle == Lifecycle::Active)
        .collect();
    let network_objects: Vec<_> = active
        .iter()
        .copied()
        .filter(|object| object.kind == ProjectObjectKind::Network)
        .collect();
    if network_objects.len() != 1 {
        diagnostics.push(project_diagnostic(
            "EDU-SYS-1001",
            format!(
                "Exactly one canonical VirtualNetwork is required; found {}.",
                network_objects.len()
            ),
            project.root_id(),
        ));
    }

    let mut origins = BTreeMap::new();
    let mut network = VirtualNetwork::new();
    let subnet = network_objects.first().and_then(|object| {
        add_canonical_subnet(&mut network, object, &mut origins, &mut diagnostics)
    });

    let controllers: Vec<_> = active
        .iter()
        .copied()
        .filter(|object| object.kind == ProjectObjectKind::Controller)
        .collect();
    if controllers.len() > MAX_CANONICAL_CONTROLLERS {
        diagnostics.push(project_diagnostic(
            "EDU-SYS-1002",
            format!(
                "The canonical project contains {} controllers, exceeding the projection bound of {MAX_CANONICAL_CONTROLLERS}.",
                controllers.len()
            ),
            project.root_id(),
        ));
    }
    for controller in controllers.iter().take(MAX_CANONICAL_CONTROLLERS) {
        add_controller_network(
            &mut network,
            controller,
            subnet,
            &mut origins,
            &mut diagnostics,
        );
    }

    let mut hardware_project = HardwareProject::new(profile.pin(), network);
    for controller in controllers.iter().take(MAX_CANONICAL_CONTROLLERS) {
        if let Some(configuration) = project_controller(
            project,
            controller,
            &profile,
            &mut origins,
            &mut diagnostics,
        ) && let Err(error) = hardware_project.add_controller(configuration)
        {
            diagnostics.push(project_diagnostic(
                "EDU-SYS-1100",
                format!(
                    "The canonical controller could not enter the hardware aggregate: {error}."
                ),
                controller.id,
            ));
        }
    }

    let (allocation_preview, artifact) = finalize_hardware(
        &mut hardware_project,
        &profile,
        project.root_id(),
        &origins,
        &mut diagnostics,
    );

    CanonicalHardwareProjection {
        source_document_hash: project.document_hash(),
        source_semantic_fingerprint: project.semantic_fingerprint(),
        profile,
        hardware_project,
        allocation_preview,
        artifact,
        diagnostics,
    }
}

fn finalize_hardware(
    hardware_project: &mut HardwareProject,
    profile: &TrainingProfile,
    root: ObjectId,
    origins: &BTreeMap<Uuid, ObjectId>,
    diagnostics: &mut Vec<ProjectDiagnostic>,
) -> (Option<AllocationPreview>, Option<HardwareArtifact>) {
    let mut allocation_preview = None;
    match hardware_project.preview_auto_allocate(profile) {
        Ok(preview) => {
            if let Err(error) = hardware_project.commit_auto_allocate(profile, &preview) {
                push_hardware_error(error, root, origins, diagnostics);
            } else {
                allocation_preview = Some(preview);
            }
        }
        Err(error) => push_hardware_error(error, root, origins, diagnostics),
    }

    for diagnostic in hardware_project.validate(profile) {
        diagnostics.push(map_hardware_diagnostic(diagnostic, root, origins));
    }
    normalize_diagnostics(diagnostics);
    let artifact = if diagnostics.iter().any(|diagnostic| diagnostic.blocking) {
        None
    } else {
        match hardware_project.build(profile) {
            Ok(artifact) => Some(artifact),
            Err(error) => {
                push_hardware_error(error, root, origins, diagnostics);
                None
            }
        }
    };
    normalize_diagnostics(diagnostics);
    (allocation_preview, artifact)
}

fn validate_profile_pin(
    project: &Project,
    profile: &TrainingProfile,
    diagnostics: &mut Vec<ProjectDiagnostic>,
) {
    let expected = profile.pin();
    let actual = project.profile();
    if actual.id != expected.id
        || actual.version != expected.version
        || actual.manifest_hash != expected.manifest_hash
    {
        diagnostics.push(project_diagnostic(
            "EDU-SYS-1000",
            "The canonical project is not pinned to the admitted EDU-21 profile manifest.",
            project.root_id(),
        ));
    }
}

fn add_canonical_subnet(
    network: &mut VirtualNetwork,
    object: &ProjectObject,
    origins: &mut BTreeMap<Uuid, ObjectId>,
    diagnostics: &mut Vec<ProjectDiagnostic>,
) -> Option<VirtualSubnetId> {
    let id = VirtualSubnetId::from(derived_uuid(object.id.0, b"virtual-subnet", 1));
    origins.insert(id.uuid(), object.id);
    let address = VirtualIpAddress::parse(NETWORK_PREFIX).expect("fixed documentation subnet");
    let subnet = VirtualSubnet {
        id,
        creation_ordinal: object.creation_ordinal,
        name: object.display_name.clone(),
        network_address: address,
        prefix_length: 24,
    };
    if let Err(error) = network.add_subnet(subnet) {
        diagnostics.push(project_diagnostic(
            "EDU-SYS-1200",
            format!("The canonical VirtualNetwork subnet is invalid: {error:?}."),
            object.id,
        ));
        None
    } else {
        Some(id)
    }
}

fn add_controller_network(
    network: &mut VirtualNetwork,
    controller: &ProjectObject,
    subnet: Option<VirtualSubnetId>,
    origins: &mut BTreeMap<Uuid, ObjectId>,
    diagnostics: &mut Vec<ProjectDiagnostic>,
) {
    let device_id = controller_device_id(controller.id);
    origins.insert(device_id.uuid(), controller.id);
    let name = VirtualDeviceName::parse(&format!("controller-{}", controller.creation_ordinal))
        .expect("bounded canonical device name");
    if let Err(error) = network.add_device(VirtualDevice {
        id: device_id,
        creation_ordinal: controller.creation_ordinal,
        device_name: name,
        powered_state: PoweredState::Powered,
    }) {
        diagnostics.push(project_diagnostic(
            "EDU-SYS-1201",
            format!("The controller's virtual device is invalid: {error:?}."),
            controller.id,
        ));
        return;
    }

    let interface_id = VirtualInterfaceId::from(derived_uuid(controller.id.0, b"interface", 1));
    let port_id = VirtualPortId::from(derived_uuid(controller.id.0, b"port", 1));
    origins.insert(interface_id.uuid(), controller.id);
    origins.insert(port_id.uuid(), controller.id);
    let host = u8::try_from(1 + controller.creation_ordinal % 253).expect("host octet");
    let address = subnet.map(|_| VirtualIpAddress::from_u32(u32::from_be_bytes([192, 0, 2, host])));
    if let Err(error) = network.add_interface(VirtualInterface {
        id: interface_id,
        creation_ordinal: controller.creation_ordinal,
        owner_device_id: device_id,
        provider_module_id: None,
        name: "edu-link-1".to_owned(),
        address,
        subnet_id: subnet,
        port_class: PortClass::EduLink,
        role: DeviceRole::Controller,
        configured_state: ConfiguredState::Enabled,
        runtime_state: RuntimeState::Available,
    }) {
        diagnostics.push(project_diagnostic(
            "EDU-SYS-1202",
            format!("The controller's virtual interface is invalid: {error:?}."),
            controller.id,
        ));
        return;
    }
    if let Err(error) = network.add_port(VirtualPort {
        id: port_id,
        creation_ordinal: controller.creation_ordinal,
        owner_interface_id: interface_id,
        name: "port-1".to_owned(),
        configured_state: ConfiguredState::Enabled,
        runtime_state: RuntimeState::Available,
    }) {
        diagnostics.push(project_diagnostic(
            "EDU-SYS-1203",
            format!("The controller's virtual port is invalid: {error:?}."),
            controller.id,
        ));
    }
}

fn project_controller(
    project: &Project,
    controller: &ProjectObject,
    profile: &TrainingProfile,
    origins: &mut BTreeMap<Uuid, ObjectId>,
    diagnostics: &mut Vec<ProjectDiagnostic>,
) -> Option<ControllerConfig> {
    let Some(catalog) = payload_text(controller, "catalogId").and_then(controller_catalog) else {
        diagnostics.push(project_diagnostic(
            "EDU-SYS-1101",
            "The controller catalogId is missing or unsupported.",
            controller.id,
        ));
        return None;
    };
    let racks = active_children(project, controller.id, ProjectObjectKind::Rack);
    if racks.len() != 1 {
        diagnostics.push(project_diagnostic(
            "EDU-SYS-1102",
            format!(
                "Controller {} requires exactly one canonical local rack; found {}.",
                controller.display_name,
                racks.len()
            ),
            controller.id,
        ));
        return None;
    }
    let rack_object = racks[0];
    let local_rack = project_rack(
        project,
        controller,
        rack_object,
        profile,
        origins,
        diagnostics,
    );
    let controller_id = ControllerId::from(controller.id.0);
    origins.insert(controller_id.uuid(), controller.id);
    Some(ControllerConfig {
        id: controller_id,
        creation_ordinal: controller.creation_ordinal,
        catalog_id: catalog,
        virtual_device_id: controller_device_id(controller.id),
        local_rack,
        reserved_input_spans: Vec::new(),
        reserved_output_spans: Vec::new(),
        configured_block_count: u32::try_from(
            active_children(project, controller.id, ProjectObjectKind::ProgramBlock).len(),
        )
        .unwrap_or(u32::MAX),
    })
}

fn project_rack(
    project: &Project,
    controller: &ProjectObject,
    rack_object: &ProjectObject,
    profile: &TrainingProfile,
    origins: &mut BTreeMap<Uuid, ObjectId>,
    diagnostics: &mut Vec<ProjectDiagnostic>,
) -> RackConfig {
    let catalog = payload_text(controller, "catalogId")
        .and_then(controller_catalog)
        .expect("controller catalog validated before rack projection");
    let definition = profile
        .controller(catalog)
        .expect("admitted controller catalog");
    let mut slots = BTreeMap::new();
    for number in definition.local_first_expansion_slot..=definition.local_last_slot {
        let slot_id = SlotId::from(derived_uuid(
            rack_object.id.0,
            b"rack-slot",
            u64::from(number),
        ));
        origins.insert(slot_id.uuid(), rack_object.id);
        slots.insert(
            number,
            RackSlot {
                id: slot_id,
                number,
                installed: None,
            },
        );
    }
    for module_object in active_children(project, rack_object.id, ProjectObjectKind::Module) {
        let Some(slot) =
            payload_unsigned(module_object, "slot").and_then(|value| u8::try_from(value).ok())
        else {
            diagnostics.push(project_diagnostic(
                "EDU-SYS-1103",
                "The module slot must be a canonical unsigned 8-bit value.",
                module_object.id,
            ));
            continue;
        };
        let Some(module) = project_module(module_object, profile, origins, diagnostics) else {
            continue;
        };
        let entry = slots.entry(slot).or_insert_with(|| {
            let slot_id = SlotId::from(derived_uuid(
                rack_object.id.0,
                b"rack-slot",
                u64::from(slot),
            ));
            origins.insert(slot_id.uuid(), rack_object.id);
            RackSlot {
                id: slot_id,
                number: slot,
                installed: None,
            }
        });
        if entry.installed.is_some() {
            diagnostics.push(ProjectDiagnostic {
                blocking: true,
                code: "EDU-SYS-1104".to_owned(),
                message: format!("More than one canonical module occupies rack slot {slot}."),
                phase: ProjectDiagnosticPhase::CanonicalProjection,
                primary_object_id: module_object.id,
                related_object_ids: vec![rack_object.id],
            });
        } else {
            entry.installed = Some(InstalledOccupant::Module(module));
        }
    }

    let rack_id = RackId::from(rack_object.id.0);
    origins.insert(rack_id.uuid(), rack_object.id);
    RackConfig {
        id: rack_id,
        creation_ordinal: rack_object.creation_ordinal,
        owner: RackOwner::Controller(ControllerId::from(controller.id.0)),
        slots,
    }
}

fn project_module(
    object: &ProjectObject,
    profile: &TrainingProfile,
    origins: &mut BTreeMap<Uuid, ObjectId>,
    diagnostics: &mut Vec<ProjectDiagnostic>,
) -> Option<ConfiguredModule> {
    let Some(catalog) = payload_text(object, "catalogId").and_then(module_catalog) else {
        diagnostics.push(project_diagnostic(
            "EDU-SYS-1110",
            "The module catalogId is missing or unsupported.",
            object.id,
        ));
        return None;
    };
    let definition = profile.module(catalog).expect("admitted module catalog");
    let channel_count = definition.channels.channel_count();
    let parameter_count = scaling_parameter_count(definition.channels);
    let channels: Vec<_> = (0..channel_count)
        .map(|index| {
            let id = ChannelId::from(derived_uuid(
                object.id.0,
                b"channel",
                u64::try_from(index).expect("bounded channel index") + 1,
            ));
            origins.insert(id.uuid(), object.id);
            id
        })
        .collect();
    let parameters: Vec<_> = (0..parameter_count)
        .map(|index| {
            let id = ParameterId::from(derived_uuid(
                object.id.0,
                b"parameter",
                u64::try_from(index).expect("bounded parameter index") + 1,
            ));
            origins.insert(id.uuid(), object.id);
            id
        })
        .collect();
    let (input_request, output_request) = module_address_requests(object, definition.channels);
    let module_id = ModuleId::from(object.id.0);
    origins.insert(module_id.uuid(), object.id);
    match ConfiguredModule::from_catalog(
        profile,
        module_id,
        object.creation_ordinal,
        catalog,
        &channels,
        &parameters,
        input_request,
        output_request,
    ) {
        Ok(module) => Some(module),
        Err(error) => {
            diagnostics.push(project_diagnostic(
                "EDU-SYS-1111",
                format!("The canonical module configuration is invalid: {error}."),
                object.id,
            ));
            None
        }
    }
}

fn module_address_requests(
    object: &ProjectObject,
    layout: ChannelLayout,
) -> (AddressRequest, AddressRequest) {
    let automatic = payload_text(object, "addressIntent") == Some("auto");
    let input = match layout {
        ChannelLayout::DigitalInputs(_)
        | ChannelLayout::AnalogInputs(_)
        | ChannelLayout::TemperatureInputs(_)
        | ChannelLayout::MixedDigital { .. } => payload_unsigned(object, "inputStart")
            .and_then(|value| u32::try_from(value).ok())
            .map_or_else(
                || {
                    if automatic {
                        AddressRequest::Auto
                    } else {
                        AddressRequest::NotUsed
                    }
                },
                AddressRequest::Explicit,
            ),
        ChannelLayout::None
        | ChannelLayout::DigitalOutputs(_)
        | ChannelLayout::AnalogOutputs(_) => AddressRequest::NotUsed,
    };
    let output = match layout {
        ChannelLayout::DigitalOutputs(_)
        | ChannelLayout::AnalogOutputs(_)
        | ChannelLayout::MixedDigital { .. } => payload_unsigned(object, "outputStart")
            .and_then(|value| u32::try_from(value).ok())
            .map_or_else(
                || {
                    if automatic {
                        AddressRequest::Auto
                    } else {
                        AddressRequest::NotUsed
                    }
                },
                AddressRequest::Explicit,
            ),
        ChannelLayout::None
        | ChannelLayout::DigitalInputs(_)
        | ChannelLayout::AnalogInputs(_)
        | ChannelLayout::TemperatureInputs(_) => AddressRequest::NotUsed,
    };
    (input, output)
}

const fn scaling_parameter_count(layout: ChannelLayout) -> usize {
    match layout {
        ChannelLayout::AnalogInputs(count)
        | ChannelLayout::AnalogOutputs(count)
        | ChannelLayout::TemperatureInputs(count) => count as usize,
        ChannelLayout::None
        | ChannelLayout::DigitalInputs(_)
        | ChannelLayout::DigitalOutputs(_)
        | ChannelLayout::MixedDigital { .. } => 0,
    }
}

fn active_children(
    project: &Project,
    parent: ObjectId,
    kind: ProjectObjectKind,
) -> Vec<&ProjectObject> {
    let mut children: Vec<_> = project
        .objects()
        .filter(|object| {
            object.lifecycle == Lifecycle::Active
                && object.parent_id == Some(parent)
                && object.kind == kind
        })
        .collect();
    children.sort_by_key(|object| (object.creation_ordinal, object.id));
    children
}

fn controller_catalog(value: &str) -> Option<ControllerCatalogId> {
    match value {
        "vctrl-c1" | "VCTRL-C1" => Some(ControllerCatalogId::VctrlC1),
        "vctrl-m1" | "VCTRL-M1" => Some(ControllerCatalogId::VctrlM1),
        "vctrl-p1" | "VCTRL-P1" => Some(ControllerCatalogId::VctrlP1),
        _ => None,
    }
}

fn module_catalog(value: &str) -> Option<ModuleCatalogId> {
    match value {
        "vpwr1" | "VPWR-1" => Some(ModuleCatalogId::Vpwr1),
        "vstn-h1" | "VSTN-H1" => Some(ModuleCatalogId::VstnH1),
        "vdi16" | "VDI-16" => Some(ModuleCatalogId::Vdi16),
        "vdo16" | "VDO-16" => Some(ModuleCatalogId::Vdo16),
        "vai4" | "VAI-4" => Some(ModuleCatalogId::Vai4),
        "vao4" | "VAO-4" => Some(ModuleCatalogId::Vao4),
        "vmix8" | "VMIX-8" => Some(ModuleCatalogId::Vmix8),
        "vrtd4" | "VRTD-4" => Some(ModuleCatalogId::Vrtd4),
        "vlink2" | "VLINK-2" => Some(ModuleCatalogId::Vlink2),
        _ => None,
    }
}

fn payload_text<'a>(object: &'a ProjectObject, key: &str) -> Option<&'a str> {
    match object.payload.semantic.get(key) {
        Some(PayloadValue::String(value)) => Some(value),
        _ => None,
    }
}

fn payload_unsigned(object: &ProjectObject, key: &str) -> Option<u64> {
    match object.payload.semantic.get(key) {
        Some(PayloadValue::Unsigned(value)) => Some(*value),
        _ => None,
    }
}

fn push_hardware_error(
    error: HardwareError,
    root: ObjectId,
    origins: &BTreeMap<Uuid, ObjectId>,
    diagnostics: &mut Vec<ProjectDiagnostic>,
) {
    if let HardwareError::Diagnostics(hardware_diagnostics) = error {
        diagnostics.extend(
            hardware_diagnostics
                .into_iter()
                .map(|diagnostic| map_hardware_diagnostic(diagnostic, root, origins)),
        );
    } else {
        diagnostics.push(ProjectDiagnostic {
            blocking: true,
            code: "EDU-SYS-1199".to_owned(),
            message: format!("Hardware projection failed honestly: {error}."),
            phase: ProjectDiagnosticPhase::Hardware,
            primary_object_id: root,
            related_object_ids: Vec::new(),
        });
    }
}

fn map_hardware_diagnostic(
    diagnostic: HardwareDiagnostic,
    root: ObjectId,
    origins: &BTreeMap<Uuid, ObjectId>,
) -> ProjectDiagnostic {
    let primary = origins.get(&diagnostic.primary.id).copied().unwrap_or(root);
    let mut related: Vec<_> = diagnostic
        .related
        .iter()
        .filter_map(|target| origins.get(&target.id).copied())
        .filter(|id| *id != primary)
        .collect();
    related.sort_unstable();
    related.dedup();
    ProjectDiagnostic {
        blocking: diagnostic.blocking,
        code: diagnostic.code.stable_code().to_owned(),
        message: diagnostic.message,
        phase: ProjectDiagnosticPhase::Hardware,
        primary_object_id: primary,
        related_object_ids: related,
    }
}

fn project_diagnostic(
    code: &str,
    message: impl Into<String>,
    primary_object_id: ObjectId,
) -> ProjectDiagnostic {
    ProjectDiagnostic {
        blocking: true,
        code: code.to_owned(),
        message: message.into(),
        phase: ProjectDiagnosticPhase::CanonicalProjection,
        primary_object_id,
        related_object_ids: Vec::new(),
    }
}

fn normalize_diagnostics(diagnostics: &mut Vec<ProjectDiagnostic>) {
    for diagnostic in diagnostics.iter_mut() {
        diagnostic.related_object_ids.sort_unstable();
        diagnostic.related_object_ids.dedup();
    }
    diagnostics.sort();
    diagnostics.dedup();
}

fn derived_uuid(owner: Uuid, domain: &[u8], ordinal: u64) -> Uuid {
    let mut seed = Vec::with_capacity(16 + domain.len());
    seed.extend_from_slice(&owner.into_bytes());
    seed.extend_from_slice(domain);
    Uuid::deterministic_v4(&seed, ordinal)
}

fn controller_device_id(controller_id: ObjectId) -> VirtualDeviceId {
    VirtualDeviceId::from(derived_uuid(controller_id.0, b"virtual-device", 1))
}
