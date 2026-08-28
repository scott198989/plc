#![allow(clippy::missing_errors_doc)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use plc_core::{Sha256Digest, Uuid};

use crate::canonical::CanonicalEncoder;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticTarget, TargetKind};
use crate::ids::{
    ChannelId, ControllerId, ModuleId, ParameterId, RackId, SlotId, StationId, TagId,
    VirtualDeviceId,
};
use crate::network::{ConfiguredState, DeviceRole, VirtualNetwork};
use crate::profile::{
    Capability, ChannelLayout, ControllerCatalogId, ModuleCatalogId, PlacementClass,
    ProfileAllowlist, ProfileError, ProfilePin, TrainingProfile,
};
use crate::types::{FiniteF64, PrimitiveType, TypeError};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AddressArea {
    Input,
    Output,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AddressRequest {
    NotUsed,
    Auto,
    Explicit(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AddressSpan {
    pub area: AddressArea,
    pub start_byte: u32,
    pub length_bytes: u32,
}

impl AddressSpan {
    #[must_use]
    pub fn end_exclusive(self) -> Option<u32> {
        self.start_byte.checked_add(self.length_bytes)
    }

    #[must_use]
    pub fn overlaps(self, other: Self) -> bool {
        if self.area != other.area {
            return false;
        }
        let Some(self_end) = self.end_exclusive() else {
            return true;
        };
        let Some(other_end) = other.end_exclusive() else {
            return true;
        };
        self.start_byte < other_end && other.start_byte < self_end
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChannelDirection {
    Input,
    Output,
}

impl ChannelDirection {
    #[must_use]
    pub const fn area(self) -> AddressArea {
        match self {
            Self::Input => AddressArea::Input,
            Self::Output => AddressArea::Output,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChannelQuality {
    Good,
    Uncertain,
    Bad,
    NotPresent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChannelDiagnosticCapabilities {
    pub channel_fault: bool,
    pub wire_break: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScalingParameter {
    pub id: ParameterId,
    pub engineering_min: FiniteF64,
    pub engineering_max: FiniteF64,
    pub display_unit: String,
}

impl ScalingParameter {
    pub fn new(
        id: ParameterId,
        engineering_min: f64,
        engineering_max: f64,
        display_unit: impl Into<String>,
    ) -> Result<Self, HardwareError> {
        let engineering_min = FiniteF64::new(engineering_min)?;
        let engineering_max = FiniteF64::new(engineering_max)?;
        if engineering_min.get() >= engineering_max.get() {
            return Err(HardwareError::InvalidScaling);
        }
        let display_unit = display_unit.into();
        if display_unit.len() > 64 {
            return Err(HardwareError::InvalidScaling);
        }
        Ok(Self {
            id,
            engineering_min,
            engineering_max,
            display_unit,
        })
    }

    #[must_use]
    pub fn engineering_from_raw(&self, raw: i16) -> f64 {
        let minimum = self.engineering_min.get();
        let maximum = self.engineering_max.get();
        minimum + ((f64::from(raw) + 32_768.0) / 65_535.0) * (maximum - minimum)
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn raw_from_engineering(&self, value: f64) -> Result<i16, HardwareError> {
        let minimum = self.engineering_min.get();
        let maximum = self.engineering_max.get();
        if !value.is_finite() || value < minimum || value > maximum {
            return Err(HardwareError::EngineeringValueOutOfRange);
        }
        let raw =
            (((value - minimum) / (maximum - minimum)) * 65_535.0 - 32_768.0).round_ties_even();
        if raw < f64::from(i16::MIN) || raw > f64::from(i16::MAX) {
            return Err(HardwareError::EngineeringValueOutOfRange);
        }
        Ok(raw as i16)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelConfig {
    pub id: ChannelId,
    pub declared_ordinal: u8,
    pub direction_index: u8,
    pub direction: ChannelDirection,
    pub raw_type: PrimitiveType,
    pub engineering_scaling: Option<ScalingParameter>,
    pub diagnostic_capabilities: ChannelDiagnosticCapabilities,
    pub symbolic_binding: Option<TagId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModuleRuntimeState {
    ConfiguredPresent,
    Pulled,
    WrongCatalogInstalled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChannelRuntimeState {
    pub quality: ChannelQuality,
    pub channel_fault: bool,
    pub wire_break: bool,
    pub force_overlay_active: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfiguredModule {
    pub id: ModuleId,
    pub creation_ordinal: u64,
    pub catalog_id: ModuleCatalogId,
    pub channels: Vec<ChannelConfig>,
    pub input_request: AddressRequest,
    pub output_request: AddressRequest,
    pub allocated_input: Option<AddressSpan>,
    pub allocated_output: Option<AddressSpan>,
}

impl ConfiguredModule {
    #[allow(clippy::too_many_arguments)]
    pub fn from_catalog(
        profile: &TrainingProfile,
        id: ModuleId,
        creation_ordinal: u64,
        catalog_id: ModuleCatalogId,
        channel_ids: &[ChannelId],
        parameter_ids: &[ParameterId],
        input_request: AddressRequest,
        output_request: AddressRequest,
    ) -> Result<Self, HardwareError> {
        let definition = profile
            .module(catalog_id)
            .ok_or(HardwareError::UnknownCatalog)?;
        if channel_ids.len() != definition.channels.channel_count() {
            return Err(HardwareError::InvalidChannelIdentityCount);
        }
        let scaling_count = match definition.channels {
            ChannelLayout::AnalogInputs(count)
            | ChannelLayout::AnalogOutputs(count)
            | ChannelLayout::TemperatureInputs(count) => usize::from(count),
            _ => 0,
        };
        if parameter_ids.len() != scaling_count {
            return Err(HardwareError::InvalidParameterIdentityCount);
        }
        let channels =
            create_channels(definition.channels, channel_ids, parameter_ids, catalog_id)?;
        let allocated_input =
            explicit_span(AddressArea::Input, definition.input_bytes, input_request)?;
        let allocated_output =
            explicit_span(AddressArea::Output, definition.output_bytes, output_request)?;
        Ok(Self {
            id,
            creation_ordinal,
            catalog_id,
            channels,
            input_request,
            output_request,
            allocated_input,
            allocated_output,
        })
    }

    #[must_use]
    pub fn span(&self, area: AddressArea) -> Option<AddressSpan> {
        match area {
            AddressArea::Input => self.allocated_input,
            AddressArea::Output => self.allocated_output,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstalledOccupant {
    ControllerCore(ControllerId),
    Module(ConfiguredModule),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RackSlot {
    pub id: SlotId,
    pub number: u8,
    pub installed: Option<InstalledOccupant>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RackOwner {
    Controller(ControllerId),
    Station(StationId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RackConfig {
    pub id: RackId,
    pub creation_ordinal: u64,
    pub owner: RackOwner,
    pub slots: BTreeMap<u8, RackSlot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControllerConfig {
    pub id: ControllerId,
    pub creation_ordinal: u64,
    pub catalog_id: ControllerCatalogId,
    pub virtual_device_id: VirtualDeviceId,
    pub local_rack: RackConfig,
    pub reserved_input_spans: Vec<AddressSpan>,
    pub reserved_output_spans: Vec<AddressSpan>,
    pub configured_block_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StationConfig {
    pub id: StationId,
    pub controller_id: ControllerId,
    pub creation_ordinal: u64,
    pub virtual_device_id: VirtualDeviceId,
    pub rack: RackConfig,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AllocationChange {
    pub module_id: ModuleId,
    pub area: AddressArea,
    pub previous: Option<AddressSpan>,
    pub proposed: AddressSpan,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AllocationPreview {
    pub expected_configuration_fingerprint: Sha256Digest,
    pub changes: Vec<AllocationChange>,
    pub proposed_configuration_fingerprint: Sha256Digest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChannelAddress {
    Bit {
        area: AddressArea,
        byte: u32,
        bit: u8,
    },
    Word {
        area: AddressArea,
        byte: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HardwareChannelBinding {
    pub controller_id: ControllerId,
    pub controller_creation_ordinal: u64,
    pub module_id: ModuleId,
    pub location_rank: u8,
    pub station_creation_ordinal: u64,
    pub slot_number: u8,
    pub module_creation_ordinal: u64,
    pub channel_id: ChannelId,
    pub channel_index: u8,
    pub direction: ChannelDirection,
    pub raw_type: PrimitiveType,
    pub address: ChannelAddress,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HardwareArtifact {
    pub profile_pin: ProfilePin,
    pub hardware_fingerprint: Sha256Digest,
    pub network_configuration_fingerprint: Sha256Digest,
    pub channel_bindings: BTreeMap<ChannelId, HardwareChannelBinding>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HardwareProject {
    profile_pin: ProfilePin,
    controllers: BTreeMap<ControllerId, ControllerConfig>,
    stations: BTreeMap<StationId, StationConfig>,
    network: VirtualNetwork,
}

impl HardwareProject {
    #[must_use]
    pub fn new(profile_pin: ProfilePin, network: VirtualNetwork) -> Self {
        Self {
            profile_pin,
            controllers: BTreeMap::new(),
            stations: BTreeMap::new(),
            network,
        }
    }

    pub fn add_controller(&mut self, controller: ControllerConfig) -> Result<(), HardwareError> {
        if self.controllers.contains_key(&controller.id) {
            return Err(HardwareError::DuplicateIdentity(controller.id.uuid()));
        }
        let profile = ProfileAllowlist::load(&self.profile_pin)?;
        if self.controllers.len()
            >= usize::try_from(profile.limits().controllers_per_project).unwrap_or(usize::MAX)
        {
            return Err(HardwareError::ControllerCapacity {
                maximum: profile.limits().controllers_per_project,
            });
        }
        self.controllers.insert(controller.id, controller);
        Ok(())
    }

    pub fn add_station(&mut self, station: StationConfig) -> Result<(), HardwareError> {
        if self.stations.contains_key(&station.id) {
            return Err(HardwareError::DuplicateIdentity(station.id.uuid()));
        }
        let profile = ProfileAllowlist::load(&self.profile_pin)?;
        let controller = self
            .controllers
            .get(&station.controller_id)
            .ok_or(HardwareError::UnknownController(station.controller_id))?;
        let maximum = profile
            .controller(controller.catalog_id)
            .ok_or(HardwareError::UnknownCatalog)?
            .distributed_stations;
        let current = self
            .stations
            .values()
            .filter(|existing| existing.controller_id == station.controller_id)
            .count();
        if current >= usize::from(maximum) {
            return Err(HardwareError::StationCapacity {
                controller_id: station.controller_id,
                maximum,
            });
        }
        self.stations.insert(station.id, station);
        Ok(())
    }

    #[must_use]
    pub const fn profile_pin(&self) -> &ProfilePin {
        &self.profile_pin
    }

    #[must_use]
    pub const fn controllers(&self) -> &BTreeMap<ControllerId, ControllerConfig> {
        &self.controllers
    }

    pub fn controller_mut(&mut self, id: ControllerId) -> Option<&mut ControllerConfig> {
        self.controllers.get_mut(&id)
    }

    #[must_use]
    pub const fn stations(&self) -> &BTreeMap<StationId, StationConfig> {
        &self.stations
    }

    pub fn station_mut(&mut self, id: StationId) -> Option<&mut StationConfig> {
        self.stations.get_mut(&id)
    }

    #[must_use]
    pub const fn network(&self) -> &VirtualNetwork {
        &self.network
    }

    pub fn network_mut(&mut self) -> &mut VirtualNetwork {
        &mut self.network
    }

    #[must_use]
    pub fn validate(&self, profile: &TrainingProfile) -> Vec<Diagnostic> {
        self.validate_internal(profile, true)
    }

    pub fn preview_auto_allocate(
        &self,
        profile: &TrainingProfile,
    ) -> Result<AllocationPreview, HardwareError> {
        profile.require_capability(Capability::AutomaticAddressAllocation)?;
        let diagnostics = self.validate_internal(profile, false);
        if !diagnostics.is_empty() {
            return Err(HardwareError::Diagnostics(diagnostics));
        }
        let mut proposed = self.clone();
        let mut changes = Vec::new();
        let mut controllers: Vec<_> = self.controllers.values().collect();
        controllers.sort_by_key(|controller| (controller.creation_ordinal, controller.id));
        for controller in controllers {
            proposed.allocate_controller(profile, controller.id, &mut changes)?;
        }
        changes.sort_by_key(|change| (change.module_id, change.area));
        Ok(AllocationPreview {
            expected_configuration_fingerprint: self.configuration_fingerprint(),
            proposed_configuration_fingerprint: proposed.configuration_fingerprint(),
            changes,
        })
    }

    pub fn commit_auto_allocate(
        &mut self,
        profile: &TrainingProfile,
        preview: &AllocationPreview,
    ) -> Result<(), HardwareError> {
        if self.configuration_fingerprint() != preview.expected_configuration_fingerprint {
            return Err(HardwareError::StaleAllocationPreview);
        }
        let fresh = self.preview_auto_allocate(profile)?;
        if fresh.changes != preview.changes
            || fresh.proposed_configuration_fingerprint
                != preview.proposed_configuration_fingerprint
        {
            return Err(HardwareError::StaleAllocationPreview);
        }
        let mut candidate = self.clone();
        for change in &preview.changes {
            let module = candidate
                .module_mut(change.module_id)
                .ok_or(HardwareError::UnknownModule(change.module_id))?;
            match change.area {
                AddressArea::Input => module.allocated_input = Some(change.proposed),
                AddressArea::Output => module.allocated_output = Some(change.proposed),
            }
        }
        let diagnostics = candidate.validate(profile);
        if !diagnostics.is_empty() {
            return Err(HardwareError::Diagnostics(diagnostics));
        }
        if candidate.configuration_fingerprint() != preview.proposed_configuration_fingerprint {
            return Err(HardwareError::StaleAllocationPreview);
        }
        *self = candidate;
        Ok(())
    }

    pub fn build(&self, profile: &TrainingProfile) -> Result<HardwareArtifact, HardwareError> {
        let diagnostics = self.validate(profile);
        if !diagnostics.is_empty() {
            return Err(HardwareError::Diagnostics(diagnostics));
        }
        let mut bindings = BTreeMap::new();
        for controller in self.controllers.values() {
            for located in self.modules_for_controller(controller.id) {
                let module = located.module;
                for channel in &module.channels {
                    let binding = channel_binding(controller, &located, channel)
                        .ok_or(HardwareError::UnallocatedChannel(channel.id))?;
                    if bindings.insert(channel.id, binding).is_some() {
                        return Err(HardwareError::DuplicateIdentity(channel.id.uuid()));
                    }
                }
            }
        }
        Ok(HardwareArtifact {
            profile_pin: self.profile_pin.clone(),
            hardware_fingerprint: self.configuration_fingerprint(),
            network_configuration_fingerprint: self.network.configuration_fingerprint(),
            channel_bindings: bindings,
        })
    }

    #[must_use]
    pub fn configuration_fingerprint(&self) -> Sha256Digest {
        let mut encoder = CanonicalEncoder::default();
        encoder.domain("EDU21-HARDWARE-CONFIGURATION-V1");
        encoder.text(&self.profile_pin.id);
        encoder.text(&self.profile_pin.version);
        encoder.digest(self.profile_pin.manifest_hash);
        encoder.digest(self.network.configuration_fingerprint());
        encoder.usize(self.controllers.len());
        for controller in self.controllers.values() {
            encode_controller(controller, &mut encoder);
        }
        encoder.usize(self.stations.len());
        for station in self.stations.values() {
            encode_station(station, &mut encoder);
        }
        encoder.fingerprint()
    }

    fn validate_internal(
        &self,
        profile: &TrainingProfile,
        require_allocated: bool,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        if ProfileAllowlist::load(&self.profile_pin).as_ref() != Ok(profile) {
            diagnostics.push(Diagnostic::blocking(
                DiagnosticCode::ProfileInvalid,
                DiagnosticTarget::new(TargetKind::Profile, Uuid::NIL),
                "Hardware project profile pin does not match the shipped allowlisted profile",
            ));
            return diagnostics;
        }
        if self.controllers.len()
            > usize::try_from(profile.limits().controllers_per_project).unwrap_or(usize::MAX)
        {
            diagnostics.push(
                Diagnostic::blocking(
                    DiagnosticCode::ResourceLimit,
                    DiagnosticTarget::new(TargetKind::Profile, Uuid::NIL),
                    "Controller count exceeds the EDU-21 project limit",
                )
                .parameter("limit", "controllers_per_project")
                .parameter("current", self.controllers.len().to_string())
                .parameter("requested", self.controllers.len().to_string())
                .parameter(
                    "maximum",
                    profile.limits().controllers_per_project.to_string(),
                ),
            );
        }
        diagnostics.extend(self.network.validate_configuration());
        self.validate_identity_uniqueness(&mut diagnostics);
        for controller in self.controllers.values() {
            self.validate_controller(profile, controller, require_allocated, &mut diagnostics);
        }
        for station in self.stations.values() {
            self.validate_station(profile, station, require_allocated, &mut diagnostics);
        }
        self.validate_network_assignments(profile, &mut diagnostics);
        diagnostics.sort_by(|left, right| {
            (
                left.code.stable_code(),
                left.primary.kind,
                left.primary.id,
                &left.primary.field,
                &left.message,
            )
                .cmp(&(
                    right.code.stable_code(),
                    right.primary.kind,
                    right.primary.id,
                    &right.primary.field,
                    &right.message,
                ))
        });
        diagnostics
    }

    fn validate_identity_uniqueness(&self, diagnostics: &mut Vec<Diagnostic>) {
        let mut identities = BTreeMap::new();
        for (kind, id) in self.all_hardware_identities() {
            if !id.is_rfc9562_v4() {
                diagnostics.push(Diagnostic::blocking(
                    DiagnosticCode::IllegalPlacementOrCatalog,
                    DiagnosticTarget::new(kind, id),
                    "Hardware identities must be RFC 9562 UUIDv4-shaped simulator IDs",
                ));
            }
            if let Some(previous_kind) = identities.insert(id, kind) {
                diagnostics.push(
                    Diagnostic::blocking(
                        DiagnosticCode::IllegalPlacementOrCatalog,
                        DiagnosticTarget::new(kind, id),
                        "A hardware UUID is reused by more than one object",
                    )
                    .related([DiagnosticTarget::new(previous_kind, id)]),
                );
            }
        }
    }

    fn all_hardware_identities(&self) -> Vec<(TargetKind, Uuid)> {
        let mut values = Vec::new();
        for controller in self.controllers.values() {
            values.push((TargetKind::Controller, controller.id.uuid()));
            collect_rack_identities(&controller.local_rack, &mut values);
        }
        for station in self.stations.values() {
            values.push((TargetKind::Station, station.id.uuid()));
            collect_rack_identities(&station.rack, &mut values);
        }
        values.extend(
            self.network
                .devices()
                .keys()
                .map(|id| (TargetKind::VirtualDevice, id.uuid())),
        );
        values.extend(
            self.network
                .interfaces()
                .keys()
                .map(|id| (TargetKind::VirtualInterface, id.uuid())),
        );
        values.extend(
            self.network
                .subnets()
                .keys()
                .map(|id| (TargetKind::VirtualSubnet, id.uuid())),
        );
        values.extend(
            self.network
                .ports()
                .keys()
                .map(|id| (TargetKind::VirtualPort, id.uuid())),
        );
        values.extend(
            self.network
                .links()
                .keys()
                .map(|id| (TargetKind::VirtualLink, id.uuid())),
        );
        values
    }

    fn validate_controller(
        &self,
        profile: &TrainingProfile,
        controller: &ControllerConfig,
        require_allocated: bool,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let Some(definition) = profile.controller(controller.catalog_id) else {
            diagnostics.push(Diagnostic::blocking(
                DiagnosticCode::IllegalPlacementOrCatalog,
                DiagnosticTarget::new(TargetKind::Controller, controller.id.uuid())
                    .field("catalogId"),
                "Controller catalog identity is not declared by EDU-21",
            ));
            return;
        };
        if controller.local_rack.owner != RackOwner::Controller(controller.id) {
            diagnostics.push(Diagnostic::blocking(
                DiagnosticCode::IllegalPlacementOrCatalog,
                DiagnosticTarget::new(TargetKind::Rack, controller.local_rack.id.uuid())
                    .field("owner"),
                "Local rack owner does not match its controller",
            ));
        }
        let expected_slots: BTreeSet<u8> = if controller.catalog_id == ControllerCatalogId::VctrlC1
        {
            (1..=definition.local_last_slot).collect()
        } else {
            (0..=definition.local_last_slot).collect()
        };
        self.validate_rack_slots(
            profile,
            &controller.local_rack,
            &expected_slots,
            Some(controller),
            require_allocated,
            diagnostics,
        );
        if definition.requires_power_slot_zero {
            require_module(
                &controller.local_rack,
                0,
                ModuleCatalogId::Vpwr1,
                diagnostics,
            );
            require_controller_core(&controller.local_rack, 1, controller.id, diagnostics);
        }
        if controller.configured_block_count > definition.block_capacity {
            diagnostics.push(
                Diagnostic::blocking(
                    DiagnosticCode::ResourceLimit,
                    DiagnosticTarget::new(TargetKind::Controller, controller.id.uuid())
                        .field("configuredBlockCount"),
                    "Controller block count exceeds its EDU-21 catalog capacity",
                )
                .parameter("limit", "block_capacity")
                .parameter("current", controller.configured_block_count.to_string())
                .parameter("requested", controller.configured_block_count.to_string())
                .parameter("maximum", definition.block_capacity.to_string()),
            );
        }
        let station_count = self
            .stations
            .values()
            .filter(|station| station.controller_id == controller.id)
            .count();
        if station_count > usize::from(definition.distributed_stations) {
            diagnostics.push(
                Diagnostic::blocking(
                    DiagnosticCode::ResourceLimit,
                    DiagnosticTarget::new(TargetKind::Controller, controller.id.uuid())
                        .field("distributedStations"),
                    "Distributed-station count exceeds this controller catalog capacity",
                )
                .parameter("limit", "distributed_stations")
                .parameter("current", station_count.to_string())
                .parameter("requested", station_count.to_string())
                .parameter("maximum", definition.distributed_stations.to_string()),
            );
        }
        self.validate_controller_spans(profile, controller, require_allocated, diagnostics);
    }

    fn validate_station(
        &self,
        profile: &TrainingProfile,
        station: &StationConfig,
        require_allocated: bool,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if !self.controllers.contains_key(&station.controller_id) {
            diagnostics.push(Diagnostic::blocking(
                DiagnosticCode::IllegalPlacementOrCatalog,
                DiagnosticTarget::new(TargetKind::VirtualDevice, station.virtual_device_id.uuid())
                    .field("controllerId"),
                "Distributed station references a missing controller",
            ));
        }
        if station.rack.owner != RackOwner::Station(station.id) {
            diagnostics.push(Diagnostic::blocking(
                DiagnosticCode::IllegalPlacementOrCatalog,
                DiagnosticTarget::new(TargetKind::Rack, station.rack.id.uuid()).field("owner"),
                "Distributed-station rack owner does not match the station",
            ));
        }
        self.validate_rack_slots(
            profile,
            &station.rack,
            &(0..=12).collect(),
            None,
            require_allocated,
            diagnostics,
        );
        require_module(&station.rack, 0, ModuleCatalogId::VstnH1, diagnostics);
    }

    #[allow(clippy::too_many_arguments)]
    fn validate_rack_slots(
        &self,
        profile: &TrainingProfile,
        rack: &RackConfig,
        expected_slots: &BTreeSet<u8>,
        controller: Option<&ControllerConfig>,
        require_allocated: bool,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let actual_slots: BTreeSet<_> = rack.slots.keys().copied().collect();
        if actual_slots != *expected_slots {
            diagnostics.push(Diagnostic::blocking(
                DiagnosticCode::IllegalPlacementOrCatalog,
                DiagnosticTarget::new(TargetKind::Rack, rack.id.uuid()).field("slots"),
                "Rack must contain exactly the immutable EDU-21 slot number set for its owner",
            ));
        }
        for (number, slot) in &rack.slots {
            if *number != slot.number {
                diagnostics.push(Diagnostic::blocking(
                    DiagnosticCode::IllegalPlacementOrCatalog,
                    DiagnosticTarget::new(TargetKind::Slot, slot.id.uuid()).field("number"),
                    "Rack slot map key and slot number disagree",
                ));
            }
            let Some(occupant) = &slot.installed else {
                continue;
            };
            match occupant {
                InstalledOccupant::ControllerCore(id) => {
                    if controller.is_none_or(|owner| owner.id != *id)
                        || controller
                            .and_then(|owner| profile.controller(owner.catalog_id))
                            .and_then(|definition| definition.controller_slot)
                            != Some(*number)
                    {
                        diagnostics.push(Diagnostic::blocking(
                            DiagnosticCode::IllegalPlacementOrCatalog,
                            DiagnosticTarget::new(TargetKind::Slot, slot.id.uuid()),
                            "Controller core occupies an illegal slot or belongs to another controller",
                        ));
                    }
                }
                InstalledOccupant::Module(module) => {
                    self.validate_module(
                        profile,
                        rack,
                        slot,
                        module,
                        require_allocated,
                        diagnostics,
                    );
                }
            }
        }
    }

    fn validate_module(
        &self,
        profile: &TrainingProfile,
        rack: &RackConfig,
        slot: &RackSlot,
        module: &ConfiguredModule,
        require_allocated: bool,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let target = DiagnosticTarget::new(TargetKind::Module, module.id.uuid());
        let Some(definition) = profile.module(module.catalog_id) else {
            diagnostics.push(Diagnostic::blocking(
                DiagnosticCode::IllegalPlacementOrCatalog,
                target.field("catalogId"),
                "Module catalog identity is not declared by EDU-21",
            ));
            return;
        };
        let placement_is_valid = match (rack.owner, definition.placement, slot.number) {
            (RackOwner::Controller(_), PlacementClass::ModularPowerSlotZero, 0)
            | (RackOwner::Station(_), PlacementClass::StationHeadSlotZero, 0)
            | (RackOwner::Station(_), PlacementClass::Expansion, 1..=12) => true,
            (RackOwner::Controller(controller_id), PlacementClass::Expansion, number) => self
                .controllers
                .get(&controller_id)
                .and_then(|controller| profile.controller(controller.catalog_id))
                .is_some_and(|controller| {
                    number >= controller.local_first_expansion_slot
                        && number <= controller.local_last_slot
                }),
            _ => false,
        };
        if !placement_is_valid {
            diagnostics.push(Diagnostic::blocking(
                DiagnosticCode::IllegalPlacementOrCatalog,
                target.clone().field("catalogId/slot"),
                "Module catalog identity is illegal for this rack and slot class",
            ));
        }
        validate_channel_shape(
            module,
            definition.channels,
            definition.supports_wire_break,
            diagnostics,
        );
        validate_module_address_request(
            module,
            AddressArea::Input,
            definition.input_bytes,
            require_allocated,
            diagnostics,
        );
        validate_module_address_request(
            module,
            AddressArea::Output,
            definition.output_bytes,
            require_allocated,
            diagnostics,
        );
    }

    fn validate_controller_spans(
        &self,
        profile: &TrainingProfile,
        controller: &ControllerConfig,
        require_allocated: bool,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let Some(definition) = profile.controller(controller.catalog_id) else {
            return;
        };
        let modules = self.modules_for_controller(controller.id);
        for area in [AddressArea::Input, AddressArea::Output] {
            let capacity = match area {
                AddressArea::Input => definition.input_bytes,
                AddressArea::Output => definition.output_bytes,
            };
            let mut spans: Vec<(Option<ModuleId>, AddressSpan)> = match area {
                AddressArea::Input => controller.reserved_input_spans.clone(),
                AddressArea::Output => controller.reserved_output_spans.clone(),
            }
            .into_iter()
            .map(|span| (None, span))
            .collect();
            for located in &modules {
                if let Some(span) = located.module.span(area) {
                    spans.push((Some(located.module.id), span));
                } else if require_allocated
                    && request_for(located.module, area) == AddressRequest::Auto
                {
                    diagnostics.push(Diagnostic::blocking(
                        DiagnosticCode::ChannelConfiguration,
                        DiagnosticTarget::new(TargetKind::Module, located.module.id.uuid())
                            .field(area_field(area)),
                        "AUTO process-image request is unresolved; commit AutoAllocateAddresses before build",
                    ));
                }
            }
            for (owner, span) in &spans {
                if span.area != area
                    || span.length_bytes == 0
                    || span.end_exclusive().is_none_or(|end| end > capacity)
                {
                    let (kind, id) = owner
                        .map_or((TargetKind::Controller, controller.id.uuid()), |module| {
                            (TargetKind::Module, module.uuid())
                        });
                    diagnostics.push(Diagnostic::blocking(
                        DiagnosticCode::AddressConflict,
                        DiagnosticTarget::new(kind, id).field(area_field(area)),
                        "Process-image span is empty, has the wrong area, overflows, or exceeds controller capacity",
                    ));
                }
            }
            for first in 0..spans.len() {
                for second in first + 1..spans.len() {
                    if !spans[first].1.overlaps(spans[second].1) {
                        continue;
                    }
                    let first_target = span_target(controller.id, spans[first].0, area);
                    let second_target = span_target(controller.id, spans[second].0, area);
                    diagnostics.push(
                        Diagnostic::blocking(
                            DiagnosticCode::AddressConflict,
                            first_target.clone(),
                            "Process-image spans overlap in the same controller address area",
                        )
                        .related([first_target, second_target])
                        .parameter("area", format!("{area:?}"))
                        .parameter("firstStartByte", spans[first].1.start_byte.to_string())
                        .parameter("firstLengthBytes", spans[first].1.length_bytes.to_string())
                        .parameter("secondStartByte", spans[second].1.start_byte.to_string())
                        .parameter(
                            "secondLengthBytes",
                            spans[second].1.length_bytes.to_string(),
                        ),
                    );
                }
            }
        }
    }

    fn validate_network_assignments(
        &self,
        profile: &TrainingProfile,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        for controller in self.controllers.values() {
            let expected = profile
                .controller(controller.catalog_id)
                .map_or(0, |definition| definition.integrated_interfaces);
            validate_device_network_assignment(
                &self.network,
                controller.virtual_device_id,
                DeviceRole::Controller,
                expected,
                diagnostics,
            );
        }
        for station in self.stations.values() {
            validate_device_network_assignment(
                &self.network,
                station.virtual_device_id,
                DeviceRole::Station,
                1,
                diagnostics,
            );
            if let Some(controller) = self.controllers.get(&station.controller_id)
                && !self.network.is_configured_reachable(
                    controller.virtual_device_id,
                    station.virtual_device_id,
                )
            {
                diagnostics.push(Diagnostic::blocking(
                    DiagnosticCode::NetworkTopologyInvalid,
                    DiagnosticTarget::new(TargetKind::Station, station.id.uuid())
                        .field("requiredLink"),
                    "Distributed station is not reachable from its controller through configured EDU-LINK topology",
                ));
            }
        }
        self.validate_provider_interfaces(diagnostics);
    }

    fn validate_provider_interfaces(&self, diagnostics: &mut Vec<Diagnostic>) {
        let provider_modules: BTreeMap<_, _> = self
            .all_modules()
            .into_iter()
            .map(|located| {
                let (owner_device, role) = match located.rack_owner {
                    RackOwner::Controller(id) => (
                        self.controllers
                            .get(&id)
                            .map(|controller| controller.virtual_device_id),
                        DeviceRole::Controller,
                    ),
                    RackOwner::Station(id) => (
                        self.stations
                            .get(&id)
                            .map(|station| station.virtual_device_id),
                        DeviceRole::Station,
                    ),
                };
                (
                    located.module.id,
                    (located.module.catalog_id, owner_device, role),
                )
            })
            .collect();
        for interface in self.network.interfaces().values() {
            let Some(provider_module_id) = interface.provider_module_id else {
                continue;
            };
            let valid = provider_modules.get(&provider_module_id).is_some_and(
                |(catalog, owner_device, role)| {
                    *catalog == ModuleCatalogId::Vlink2
                        && *owner_device == Some(interface.owner_device_id)
                        && *role == interface.role
                },
            );
            if !valid {
                diagnostics.push(Diagnostic::blocking(
                    DiagnosticCode::RequiredComponentMissing,
                    DiagnosticTarget::new(TargetKind::VirtualInterface, interface.id.uuid())
                        .field("providerModuleId"),
                    "A providerModuleId must name an owning-device VLINK-2 and inherit that device's role",
                ));
            }
        }
        for located in self.all_modules() {
            if located.module.catalog_id != ModuleCatalogId::Vlink2 {
                continue;
            }
            let (_, owner_device, role) = provider_modules[&located.module.id];
            let matching: Vec<_> = self
                .network
                .interfaces()
                .values()
                .filter(|interface| interface.provider_module_id == Some(located.module.id))
                .collect();
            if matching.len() != 1
                || matching.first().is_none_or(|interface| {
                    Some(interface.owner_device_id) != owner_device || interface.role != role
                })
            {
                diagnostics.push(Diagnostic::blocking(
                    DiagnosticCode::RequiredComponentMissing,
                    DiagnosticTarget::new(TargetKind::Module, located.module.id.uuid())
                        .field("providedVirtualInterface"),
                    "VLINK-2 must provide exactly one two-port EDU-LINK interface on its owning device",
                ));
                continue;
            }
            let port_count = self
                .network
                .ports()
                .values()
                .filter(|port| port.owner_interface_id == matching[0].id)
                .count();
            if port_count != 2 {
                diagnostics.push(Diagnostic::blocking(
                    DiagnosticCode::RequiredComponentMissing,
                    DiagnosticTarget::new(TargetKind::VirtualInterface, matching[0].id.uuid()),
                    "VLINK-2 provided interface must own exactly two VirtualPorts",
                ));
            }
        }
    }

    fn allocate_controller(
        &mut self,
        profile: &TrainingProfile,
        controller_id: ControllerId,
        changes: &mut Vec<AllocationChange>,
    ) -> Result<(), HardwareError> {
        let controller = self
            .controllers
            .get(&controller_id)
            .ok_or(HardwareError::UnknownController(controller_id))?;
        let definition = profile
            .controller(controller.catalog_id)
            .ok_or(HardwareError::UnknownCatalog)?;
        let descriptors: Vec<_> = self
            .modules_for_controller(controller_id)
            .into_iter()
            .map(|located| ModuleDescriptor {
                id: located.module.id,
                sort_key: located.sort_key,
                input_request: located.module.input_request,
                output_request: located.module.output_request,
                allocated_input: located.module.allocated_input,
                allocated_output: located.module.allocated_output,
                input_length: profile
                    .module(located.module.catalog_id)
                    .map_or(0, |module| module.input_bytes),
                output_length: profile
                    .module(located.module.catalog_id)
                    .map_or(0, |module| module.output_bytes),
                alignment: module_alignment(located.module.catalog_id),
            })
            .collect();
        let reserved = [
            (AddressArea::Input, controller.reserved_input_spans.clone()),
            (
                AddressArea::Output,
                controller.reserved_output_spans.clone(),
            ),
        ];
        for (area, reserved_spans) in reserved {
            let capacity = match area {
                AddressArea::Input => definition.input_bytes,
                AddressArea::Output => definition.output_bytes,
            };
            let mut occupied = reserved_spans;
            for descriptor in &descriptors {
                let request = descriptor.request(area);
                if let AddressRequest::Explicit(start) = request {
                    let length = descriptor.length(area);
                    if length > 0 {
                        occupied.push(AddressSpan {
                            area,
                            start_byte: start,
                            length_bytes: length,
                        });
                    }
                }
            }
            let mut ordered = descriptors.clone();
            ordered.sort_by_key(|descriptor| descriptor.sort_key);
            for descriptor in ordered {
                if descriptor.request(area) != AddressRequest::Auto || descriptor.length(area) == 0
                {
                    continue;
                }
                let old = descriptor.allocated(area);
                let proposed = if let Some(span) = old.filter(|span| {
                    span.area == area
                        && span.length_bytes == descriptor.length(area)
                        && span.start_byte % descriptor.alignment == 0
                        && span.end_exclusive().is_some_and(|end| end <= capacity)
                        && occupied.iter().all(|existing| !span.overlaps(*existing))
                }) {
                    span
                } else {
                    first_fit(
                        area,
                        descriptor.length(area),
                        descriptor.alignment,
                        capacity,
                        &occupied,
                    )
                    .ok_or(HardwareError::AllocationCapacity {
                        module_id: descriptor.id,
                        area,
                    })?
                };
                occupied.push(proposed);
                if old != Some(proposed) {
                    changes.push(AllocationChange {
                        module_id: descriptor.id,
                        area,
                        previous: old,
                        proposed,
                    });
                    let module = self
                        .module_mut(descriptor.id)
                        .ok_or(HardwareError::UnknownModule(descriptor.id))?;
                    match area {
                        AddressArea::Input => module.allocated_input = Some(proposed),
                        AddressArea::Output => module.allocated_output = Some(proposed),
                    }
                }
            }
        }
        Ok(())
    }

    fn modules_for_controller(&self, controller_id: ControllerId) -> Vec<LocatedModule<'_>> {
        let mut values = Vec::new();
        if let Some(controller) = self.controllers.get(&controller_id) {
            collect_modules(
                &controller.local_rack,
                ModuleLocationOrder::Local,
                0,
                &mut values,
            );
        }
        let mut stations: Vec<_> = self
            .stations
            .values()
            .filter(|station| station.controller_id == controller_id)
            .collect();
        stations.sort_by_key(|station| (station.creation_ordinal, station.id));
        for station in stations {
            collect_modules(
                &station.rack,
                ModuleLocationOrder::Distributed,
                station.creation_ordinal,
                &mut values,
            );
        }
        values.sort_by_key(|value| value.sort_key);
        values
    }

    fn all_modules(&self) -> Vec<LocatedModule<'_>> {
        let mut values = Vec::new();
        for controller in self.controllers.values() {
            values.extend(self.modules_for_controller(controller.id));
        }
        values
    }

    fn module_mut(&mut self, id: ModuleId) -> Option<&mut ConfiguredModule> {
        for controller in self.controllers.values_mut() {
            if let Some(module) = rack_module_mut(&mut controller.local_rack, id) {
                return Some(module);
            }
        }
        for station in self.stations.values_mut() {
            if let Some(module) = rack_module_mut(&mut station.rack, id) {
                return Some(module);
            }
        }
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ModuleLocationOrder {
    Local,
    Distributed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ModuleSortKey {
    location: ModuleLocationOrder,
    station_creation_ordinal: u64,
    slot: u8,
    module_creation_ordinal: u64,
    module_id: ModuleId,
}

#[derive(Clone, Copy)]
struct LocatedModule<'a> {
    module: &'a ConfiguredModule,
    rack_owner: RackOwner,
    sort_key: ModuleSortKey,
}

#[derive(Clone, Copy)]
struct ModuleDescriptor {
    id: ModuleId,
    sort_key: ModuleSortKey,
    input_request: AddressRequest,
    output_request: AddressRequest,
    allocated_input: Option<AddressSpan>,
    allocated_output: Option<AddressSpan>,
    input_length: u32,
    output_length: u32,
    alignment: u32,
}

impl ModuleDescriptor {
    const fn request(self, area: AddressArea) -> AddressRequest {
        match area {
            AddressArea::Input => self.input_request,
            AddressArea::Output => self.output_request,
        }
    }

    const fn allocated(self, area: AddressArea) -> Option<AddressSpan> {
        match area {
            AddressArea::Input => self.allocated_input,
            AddressArea::Output => self.allocated_output,
        }
    }

    const fn length(self, area: AddressArea) -> u32 {
        match area {
            AddressArea::Input => self.input_length,
            AddressArea::Output => self.output_length,
        }
    }
}

fn create_channels(
    layout: ChannelLayout,
    channel_ids: &[ChannelId],
    parameter_ids: &[ParameterId],
    catalog_id: ModuleCatalogId,
) -> Result<Vec<ChannelConfig>, HardwareError> {
    let mut channels = Vec::with_capacity(channel_ids.len());
    let mut next_channel = 0_usize;
    let mut next_parameter = 0_usize;
    let mut add = |direction: ChannelDirection,
                   direction_index: u8,
                   raw_type: PrimitiveType,
                   scaling: Option<(f64, f64, &'static str)>|
     -> Result<(), HardwareError> {
        let engineering_scaling = if let Some((minimum, maximum, unit)) = scaling {
            let id = *parameter_ids
                .get(next_parameter)
                .ok_or(HardwareError::InvalidParameterIdentityCount)?;
            next_parameter += 1;
            Some(ScalingParameter::new(id, minimum, maximum, unit)?)
        } else {
            None
        };
        let id = *channel_ids
            .get(next_channel)
            .ok_or(HardwareError::InvalidChannelIdentityCount)?;
        let declared_ordinal =
            u8::try_from(next_channel).map_err(|_| HardwareError::InvalidChannelIdentityCount)?;
        next_channel += 1;
        channels.push(ChannelConfig {
            id,
            declared_ordinal,
            direction_index,
            direction,
            raw_type,
            engineering_scaling,
            diagnostic_capabilities: ChannelDiagnosticCapabilities {
                channel_fault: true,
                wire_break: matches!(
                    catalog_id,
                    ModuleCatalogId::Vai4 | ModuleCatalogId::Vao4 | ModuleCatalogId::Vrtd4
                ),
            },
            symbolic_binding: None,
        });
        Ok(())
    };
    match layout {
        ChannelLayout::None => {}
        ChannelLayout::DigitalInputs(count) => {
            for index in 0..count {
                add(ChannelDirection::Input, index, PrimitiveType::Bool, None)?;
            }
        }
        ChannelLayout::DigitalOutputs(count) => {
            for index in 0..count {
                add(ChannelDirection::Output, index, PrimitiveType::Bool, None)?;
            }
        }
        ChannelLayout::AnalogInputs(count) => {
            for index in 0..count {
                add(
                    ChannelDirection::Input,
                    index,
                    PrimitiveType::Int,
                    Some((0.0, 100.0, "%")),
                )?;
            }
        }
        ChannelLayout::AnalogOutputs(count) => {
            for index in 0..count {
                add(
                    ChannelDirection::Output,
                    index,
                    PrimitiveType::Int,
                    Some((0.0, 100.0, "%")),
                )?;
            }
        }
        ChannelLayout::MixedDigital { inputs, outputs } => {
            for index in 0..inputs {
                add(ChannelDirection::Input, index, PrimitiveType::Bool, None)?;
            }
            for index in 0..outputs {
                add(ChannelDirection::Output, index, PrimitiveType::Bool, None)?;
            }
        }
        ChannelLayout::TemperatureInputs(count) => {
            for index in 0..count {
                add(
                    ChannelDirection::Input,
                    index,
                    PrimitiveType::Int,
                    Some((-200.0, 850.0, "degC")),
                )?;
            }
        }
    }
    Ok(channels)
}

fn explicit_span(
    area: AddressArea,
    length: u32,
    request: AddressRequest,
) -> Result<Option<AddressSpan>, HardwareError> {
    match (length, request) {
        (0, AddressRequest::NotUsed) => Ok(None),
        (0, _) | (_, AddressRequest::NotUsed) => Err(HardwareError::AddressRequestMismatch),
        (_, AddressRequest::Auto) => Ok(None),
        (_, AddressRequest::Explicit(start_byte)) => Ok(Some(AddressSpan {
            area,
            start_byte,
            length_bytes: length,
        })),
    }
}

fn validate_channel_shape(
    module: &ConfiguredModule,
    layout: ChannelLayout,
    supports_wire_break: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let expected_count = layout.channel_count();
    let ids: BTreeSet<_> = module.channels.iter().map(|channel| channel.id).collect();
    let orders: BTreeSet<_> = module
        .channels
        .iter()
        .map(|channel| channel.declared_ordinal)
        .collect();
    if module.channels.len() != expected_count
        || ids.len() != expected_count
        || orders.len() != expected_count
    {
        diagnostics.push(Diagnostic::blocking(
            DiagnosticCode::ChannelConfiguration,
            DiagnosticTarget::new(TargetKind::Module, module.id.uuid()).field("channels"),
            "Configured channel identities/count/order do not match the module catalog",
        ));
        return;
    }
    for channel in &module.channels {
        let expected = channel_expectation(layout, channel.declared_ordinal);
        if expected.is_none_or(|(direction, direction_index, raw_type, scaling)| {
            channel.direction != direction
                || channel.direction_index != direction_index
                || channel.raw_type != raw_type
                || channel.engineering_scaling.is_some() != scaling
                || !channel.diagnostic_capabilities.channel_fault
                || channel.diagnostic_capabilities.wire_break != supports_wire_break
        }) {
            diagnostics.push(Diagnostic::blocking(
                DiagnosticCode::ChannelConfiguration,
                DiagnosticTarget::new(TargetKind::Channel, channel.id.uuid()),
                "Channel direction, index, raw type, engineering projection, or diagnostic capability differs from its catalog definition",
            ));
        }
        if let Some(scaling) = &channel.engineering_scaling
            && (scaling.engineering_min.get() >= scaling.engineering_max.get()
                || !scaling.engineering_min.get().is_finite()
                || !scaling.engineering_max.get().is_finite())
        {
            diagnostics.push(Diagnostic::blocking(
                DiagnosticCode::ChannelConfiguration,
                DiagnosticTarget::new(TargetKind::Parameter, scaling.id.uuid()),
                "Analog scaling requires finite engMin strictly less than engMax",
            ));
        }
    }
}

fn channel_expectation(
    layout: ChannelLayout,
    ordinal: u8,
) -> Option<(ChannelDirection, u8, PrimitiveType, bool)> {
    match layout {
        ChannelLayout::DigitalInputs(count) if ordinal < count => {
            Some((ChannelDirection::Input, ordinal, PrimitiveType::Bool, false))
        }
        ChannelLayout::DigitalOutputs(count) if ordinal < count => Some((
            ChannelDirection::Output,
            ordinal,
            PrimitiveType::Bool,
            false,
        )),
        ChannelLayout::AnalogInputs(count) if ordinal < count => {
            Some((ChannelDirection::Input, ordinal, PrimitiveType::Int, true))
        }
        ChannelLayout::AnalogOutputs(count) if ordinal < count => {
            Some((ChannelDirection::Output, ordinal, PrimitiveType::Int, true))
        }
        ChannelLayout::TemperatureInputs(count) if ordinal < count => {
            Some((ChannelDirection::Input, ordinal, PrimitiveType::Int, true))
        }
        ChannelLayout::MixedDigital { inputs, outputs: _ } if ordinal < inputs => {
            Some((ChannelDirection::Input, ordinal, PrimitiveType::Bool, false))
        }
        ChannelLayout::MixedDigital { inputs, outputs }
            if ordinal < inputs.saturating_add(outputs) =>
        {
            Some((
                ChannelDirection::Output,
                ordinal - inputs,
                PrimitiveType::Bool,
                false,
            ))
        }
        _ => None,
    }
}

fn validate_module_address_request(
    module: &ConfiguredModule,
    area: AddressArea,
    expected_length: u32,
    require_allocated: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let request = request_for(module, area);
    let span = module.span(area);
    let valid = match (expected_length, request, span) {
        (0, AddressRequest::NotUsed, None) => true,
        (0, _, _) | (_, AddressRequest::NotUsed, _) | (_, AddressRequest::Explicit(_), None) => {
            false
        }
        (length, AddressRequest::Explicit(start), Some(span)) => {
            span.area == area && span.start_byte == start && span.length_bytes == length
        }
        (length, AddressRequest::Auto, Some(span)) => {
            span.area == area && span.length_bytes == length
        }
        (_, AddressRequest::Auto, None) => !require_allocated,
    };
    if !valid {
        diagnostics.push(Diagnostic::blocking(
            DiagnosticCode::ChannelConfiguration,
            DiagnosticTarget::new(TargetKind::Module, module.id.uuid()).field(area_field(area)),
            "Requested and allocated process-image spans disagree with the module catalog",
        ));
    }
    if let Some(span) = span {
        let alignment = module_alignment(module.catalog_id);
        if span.start_byte % alignment != 0 {
            diagnostics.push(Diagnostic::blocking(
                DiagnosticCode::AddressConflict,
                DiagnosticTarget::new(TargetKind::Module, module.id.uuid()).field(area_field(area)),
                "Process-image address violates the channel aggregate's natural alignment",
            ));
        }
    }
}

fn module_alignment(catalog: ModuleCatalogId) -> u32 {
    match catalog {
        ModuleCatalogId::Vai4 | ModuleCatalogId::Vao4 | ModuleCatalogId::Vrtd4 => 2,
        _ => 1,
    }
}

fn request_for(module: &ConfiguredModule, area: AddressArea) -> AddressRequest {
    match area {
        AddressArea::Input => module.input_request,
        AddressArea::Output => module.output_request,
    }
}

fn area_field(area: AddressArea) -> &'static str {
    match area {
        AddressArea::Input => "inputSpan",
        AddressArea::Output => "outputSpan",
    }
}

fn span_target(
    controller_id: ControllerId,
    module_id: Option<ModuleId>,
    area: AddressArea,
) -> DiagnosticTarget {
    module_id.map_or_else(
        || {
            DiagnosticTarget::new(TargetKind::Controller, controller_id.uuid())
                .field(area_field(area))
        },
        |module_id| {
            DiagnosticTarget::new(TargetKind::Module, module_id.uuid()).field(area_field(area))
        },
    )
}

fn require_module(
    rack: &RackConfig,
    slot_number: u8,
    catalog_id: ModuleCatalogId,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let valid = rack
        .slots
        .get(&slot_number)
        .and_then(|slot| slot.installed.as_ref())
        .is_some_and(|occupant| {
            matches!(occupant, InstalledOccupant::Module(module) if module.catalog_id == catalog_id)
        });
    if !valid {
        diagnostics.push(Diagnostic::blocking(
            DiagnosticCode::RequiredComponentMissing,
            DiagnosticTarget::new(TargetKind::Rack, rack.id.uuid())
                .field(format!("slot[{slot_number}]")),
            format!(
                "Rack slot {slot_number} requires catalog identity {}",
                catalog_id.as_str()
            ),
        ));
    }
}

fn require_controller_core(
    rack: &RackConfig,
    slot_number: u8,
    controller_id: ControllerId,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let valid = rack
        .slots
        .get(&slot_number)
        .and_then(|slot| slot.installed.as_ref())
        == Some(&InstalledOccupant::ControllerCore(controller_id));
    if !valid {
        diagnostics.push(Diagnostic::blocking(
            DiagnosticCode::RequiredComponentMissing,
            DiagnosticTarget::new(TargetKind::Rack, rack.id.uuid())
                .field(format!("slot[{slot_number}]")),
            "Modular/performance controller requires its controller core in slot 1",
        ));
    }
}

fn validate_device_network_assignment(
    network: &VirtualNetwork,
    device_id: VirtualDeviceId,
    role: DeviceRole,
    expected_integrated_interfaces: u8,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !network.devices().contains_key(&device_id) {
        diagnostics.push(Diagnostic::blocking(
            DiagnosticCode::RequiredComponentMissing,
            DiagnosticTarget::new(TargetKind::VirtualDevice, device_id.uuid()),
            "Configured hardware device is missing from VirtualNetwork",
        ));
        return;
    }
    let interfaces: Vec<_> = network
        .interfaces()
        .values()
        .filter(|interface| interface.owner_device_id == device_id)
        .collect();
    let integrated: Vec<_> = interfaces
        .iter()
        .copied()
        .filter(|interface| interface.provider_module_id.is_none() && interface.role == role)
        .collect();
    if integrated.len() != usize::from(expected_integrated_interfaces) {
        diagnostics.push(
            Diagnostic::blocking(
                DiagnosticCode::RequiredComponentMissing,
                DiagnosticTarget::new(TargetKind::VirtualDevice, device_id.uuid())
                    .field("integratedInterfaces"),
                "VirtualDevice integrated EDU-LINK interface count does not match its catalog",
            )
            .parameter("expected", expected_integrated_interfaces.to_string())
            .parameter("actual", integrated.len().to_string()),
        );
    }
    if !interfaces.iter().any(|interface| {
        interface.role == role
            && interface.configured_state == ConfiguredState::Enabled
            && interface.subnet_id.is_some()
            && interface.address.is_some()
    }) {
        diagnostics.push(Diagnostic::blocking(
            DiagnosticCode::RequiredComponentMissing,
            DiagnosticTarget::new(TargetKind::VirtualDevice, device_id.uuid())
                .field("networkAssignment"),
            "Build-valid controller/station requires an enabled assigned EDU-LINK interface",
        ));
    }
    for interface in integrated {
        let port_count = network
            .ports()
            .values()
            .filter(|port| port.owner_interface_id == interface.id)
            .count();
        let expected_ports = if role == DeviceRole::Station { 2 } else { 1 };
        if port_count != expected_ports {
            diagnostics.push(Diagnostic::blocking(
                DiagnosticCode::RequiredComponentMissing,
                DiagnosticTarget::new(TargetKind::VirtualInterface, interface.id.uuid()),
                format!("Integrated {role:?} interface requires {expected_ports} VirtualPort(s)"),
            ));
        }
    }
}

fn collect_rack_identities(rack: &RackConfig, output: &mut Vec<(TargetKind, Uuid)>) {
    output.push((TargetKind::Rack, rack.id.uuid()));
    for slot in rack.slots.values() {
        output.push((TargetKind::Slot, slot.id.uuid()));
        let Some(InstalledOccupant::Module(module)) = &slot.installed else {
            continue;
        };
        output.push((TargetKind::Module, module.id.uuid()));
        for channel in &module.channels {
            output.push((TargetKind::Channel, channel.id.uuid()));
            if let Some(parameter) = &channel.engineering_scaling {
                output.push((TargetKind::Parameter, parameter.id.uuid()));
            }
        }
    }
}

fn collect_modules<'a>(
    rack: &'a RackConfig,
    location: ModuleLocationOrder,
    station_creation_ordinal: u64,
    output: &mut Vec<LocatedModule<'a>>,
) {
    for slot in rack.slots.values() {
        if let Some(InstalledOccupant::Module(module)) = &slot.installed {
            output.push(LocatedModule {
                module,
                rack_owner: rack.owner,
                sort_key: ModuleSortKey {
                    location,
                    station_creation_ordinal,
                    slot: slot.number,
                    module_creation_ordinal: module.creation_ordinal,
                    module_id: module.id,
                },
            });
        }
    }
}

fn rack_module_mut(rack: &mut RackConfig, id: ModuleId) -> Option<&mut ConfiguredModule> {
    rack.slots.values_mut().find_map(|slot| {
        let Some(InstalledOccupant::Module(module)) = &mut slot.installed else {
            return None;
        };
        (module.id == id).then_some(module)
    })
}

fn first_fit(
    area: AddressArea,
    length: u32,
    alignment: u32,
    capacity: u32,
    occupied: &[AddressSpan],
) -> Option<AddressSpan> {
    let mut start = 0_u32;
    loop {
        start = align_up(start, alignment)?;
        let candidate = AddressSpan {
            area,
            start_byte: start,
            length_bytes: length,
        };
        if candidate.end_exclusive()? > capacity {
            return None;
        }
        if occupied.iter().all(|span| !candidate.overlaps(*span)) {
            return Some(candidate);
        }
        let next = occupied
            .iter()
            .filter(|span| candidate.overlaps(**span))
            .filter_map(|span| span.end_exclusive())
            .max()?;
        if next <= start {
            return None;
        }
        start = next;
    }
}

fn align_up(value: u32, alignment: u32) -> Option<u32> {
    debug_assert!(alignment > 0);
    let remainder = value % alignment;
    if remainder == 0 {
        Some(value)
    } else {
        value.checked_add(alignment - remainder)
    }
}

fn channel_binding(
    controller: &ControllerConfig,
    located: &LocatedModule<'_>,
    channel: &ChannelConfig,
) -> Option<HardwareChannelBinding> {
    let module = located.module;
    let span = module.span(channel.direction.area())?;
    let address = match channel.raw_type {
        PrimitiveType::Bool => ChannelAddress::Bit {
            area: span.area,
            byte: span
                .start_byte
                .checked_add(u32::from(channel.direction_index / 8))?,
            bit: channel.direction_index % 8,
        },
        PrimitiveType::Int => ChannelAddress::Word {
            area: span.area,
            byte: span
                .start_byte
                .checked_add(u32::from(channel.direction_index) * 2)?,
        },
        _ => return None,
    };
    Some(HardwareChannelBinding {
        controller_id: controller.id,
        controller_creation_ordinal: controller.creation_ordinal,
        module_id: module.id,
        location_rank: match located.sort_key.location {
            ModuleLocationOrder::Local => 0,
            ModuleLocationOrder::Distributed => 1,
        },
        station_creation_ordinal: located.sort_key.station_creation_ordinal,
        slot_number: located.sort_key.slot,
        module_creation_ordinal: module.creation_ordinal,
        channel_id: channel.id,
        channel_index: channel.direction_index,
        direction: channel.direction,
        raw_type: channel.raw_type,
        address,
    })
}

fn encode_controller(controller: &ControllerConfig, encoder: &mut CanonicalEncoder) {
    encoder.uuid(controller.id.uuid());
    encoder.u64(controller.creation_ordinal);
    encoder.text(controller.catalog_id.as_str());
    encoder.uuid(controller.virtual_device_id.uuid());
    encode_rack(&controller.local_rack, encoder);
    encode_spans(&controller.reserved_input_spans, encoder);
    encode_spans(&controller.reserved_output_spans, encoder);
    encoder.u32(controller.configured_block_count);
}

fn encode_station(station: &StationConfig, encoder: &mut CanonicalEncoder) {
    encoder.uuid(station.id.uuid());
    encoder.uuid(station.controller_id.uuid());
    encoder.u64(station.creation_ordinal);
    encoder.uuid(station.virtual_device_id.uuid());
    encode_rack(&station.rack, encoder);
}

fn encode_rack(rack: &RackConfig, encoder: &mut CanonicalEncoder) {
    encoder.uuid(rack.id.uuid());
    encoder.u64(rack.creation_ordinal);
    match rack.owner {
        RackOwner::Controller(id) => {
            encoder.tag("controller");
            encoder.uuid(id.uuid());
        }
        RackOwner::Station(id) => {
            encoder.tag("station");
            encoder.uuid(id.uuid());
        }
    }
    encoder.usize(rack.slots.len());
    for slot in rack.slots.values() {
        encoder.uuid(slot.id.uuid());
        encoder.u8(slot.number);
        match &slot.installed {
            None => encoder.tag("empty"),
            Some(InstalledOccupant::ControllerCore(id)) => {
                encoder.tag("controller-core");
                encoder.uuid(id.uuid());
            }
            Some(InstalledOccupant::Module(module)) => {
                encoder.tag("module");
                encode_module(module, encoder);
            }
        }
    }
}

fn encode_module(module: &ConfiguredModule, encoder: &mut CanonicalEncoder) {
    encoder.uuid(module.id.uuid());
    encoder.u64(module.creation_ordinal);
    encoder.text(module.catalog_id.as_str());
    encode_request(module.input_request, encoder);
    encode_request(module.output_request, encoder);
    encoder.option(module.allocated_input, encode_span);
    encoder.option(module.allocated_output, encode_span);
    encoder.usize(module.channels.len());
    for channel in &module.channels {
        encoder.uuid(channel.id.uuid());
        encoder.u8(channel.declared_ordinal);
        encoder.u8(channel.direction_index);
        encoder.u8(match channel.direction {
            ChannelDirection::Input => 1,
            ChannelDirection::Output => 2,
        });
        encoder.text(channel.raw_type.stable_id());
        encoder.option(channel.engineering_scaling.as_ref(), |encoder, scaling| {
            encoder.uuid(scaling.id.uuid());
            encoder.u64(scaling.engineering_min.bits());
            encoder.u64(scaling.engineering_max.bits());
            encoder.text(&scaling.display_unit);
        });
        encoder.bool(channel.diagnostic_capabilities.channel_fault);
        encoder.bool(channel.diagnostic_capabilities.wire_break);
        encoder.option(channel.symbolic_binding, |encoder, id| {
            encoder.uuid(id.uuid());
        });
    }
}

fn encode_request(request: AddressRequest, encoder: &mut CanonicalEncoder) {
    match request {
        AddressRequest::NotUsed => encoder.tag("not-used"),
        AddressRequest::Auto => encoder.tag("auto"),
        AddressRequest::Explicit(start) => {
            encoder.tag("explicit");
            encoder.u32(start);
        }
    }
}

fn encode_span(encoder: &mut CanonicalEncoder, span: AddressSpan) {
    encoder.u8(match span.area {
        AddressArea::Input => 1,
        AddressArea::Output => 2,
    });
    encoder.u32(span.start_byte);
    encoder.u32(span.length_bytes);
}

fn encode_spans(spans: &[AddressSpan], encoder: &mut CanonicalEncoder) {
    let mut ordered = spans.to_vec();
    ordered.sort();
    encoder.usize(ordered.len());
    for span in ordered {
        encode_span(encoder, span);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HardwareError {
    Profile(ProfileError),
    Type(TypeError),
    UnknownCatalog,
    UnknownController(ControllerId),
    UnknownModule(ModuleId),
    ControllerCapacity {
        maximum: u32,
    },
    StationCapacity {
        controller_id: ControllerId,
        maximum: u8,
    },
    DuplicateIdentity(Uuid),
    InvalidChannelIdentityCount,
    InvalidParameterIdentityCount,
    InvalidScaling,
    EngineeringValueOutOfRange,
    AddressRequestMismatch,
    StaleAllocationPreview,
    AllocationCapacity {
        module_id: ModuleId,
        area: AddressArea,
    },
    UnallocatedChannel(ChannelId),
    Diagnostics(Vec<Diagnostic>),
}

impl From<ProfileError> for HardwareError {
    fn from(value: ProfileError) -> Self {
        Self::Profile(value)
    }
}

impl From<TypeError> for HardwareError {
    fn from(value: TypeError) -> Self {
        Self::Type(value)
    }
}

impl fmt::Display for HardwareError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for HardwareError {}
