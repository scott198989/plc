#![allow(clippy::missing_errors_doc)]

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

use plc_core::{Sha256Digest, Uuid};

use crate::canonical::CanonicalEncoder;
use crate::hardware::{ChannelDirection, ChannelQuality, ConfiguredModule, ModuleRuntimeState};
use crate::ids::{ChannelId, ModuleId, VirtualDeviceId, VirtualLinkId};
use crate::network::{DeviceRole, NetworkError, PoweredState, RuntimeState, VirtualNetwork};
use crate::process_image::ChannelRawValue;
use crate::profile::{
    Capability, ChannelLayout, ModuleCatalogId, ModuleDefinition, ProfileAllowlist, ProfileError,
    ProfilePin, TrainingProfile,
};
use crate::types::PrimitiveType;

/// Declared role of an in-memory virtual device in the hardware runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuntimeDeviceRole {
    Controller,
    Station,
}

impl RuntimeDeviceRole {
    const fn network_role(self) -> DeviceRole {
        match self {
            Self::Controller => DeviceRole::Controller,
            Self::Station => DeviceRole::Station,
        }
    }
}

/// Immutable engineering configuration needed to project one configured
/// module through runtime hardware conditions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeModuleConfiguration {
    pub configured_module: ConfiguredModule,
    pub owner_device_id: VirtualDeviceId,
    pub required_link_id: Option<VirtualLinkId>,
}

/// Immutable, typed configuration registry. Runtime state is deliberately held
/// elsewhere so pull/restore and availability commands cannot delete or rewrite
/// engineering configuration.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuntimeHardwareConfiguration {
    devices: BTreeMap<VirtualDeviceId, RuntimeDeviceRole>,
    modules: BTreeMap<ModuleId, RuntimeModuleConfiguration>,
    channel_owners: BTreeMap<ChannelId, ModuleId>,
}

impl RuntimeHardwareConfiguration {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_device(
        &mut self,
        id: VirtualDeviceId,
        role: RuntimeDeviceRole,
    ) -> Result<(), HardwareConditionError> {
        if self.devices.contains_key(&id) {
            return Err(HardwareConditionError::DuplicateIdentity(id.uuid()));
        }
        self.devices.insert(id, role);
        Ok(())
    }

    pub fn add_module(
        &mut self,
        module: RuntimeModuleConfiguration,
    ) -> Result<(), HardwareConditionError> {
        let module_id = module.configured_module.id;
        if self.modules.contains_key(&module_id) {
            return Err(HardwareConditionError::DuplicateIdentity(module_id.uuid()));
        }
        for channel in &module.configured_module.channels {
            if self.channel_owners.contains_key(&channel.id) {
                return Err(HardwareConditionError::DuplicateIdentity(channel.id.uuid()));
            }
        }
        for channel in &module.configured_module.channels {
            self.channel_owners.insert(channel.id, module_id);
        }
        self.modules.insert(module_id, module);
        Ok(())
    }

    #[must_use]
    pub const fn devices(&self) -> &BTreeMap<VirtualDeviceId, RuntimeDeviceRole> {
        &self.devices
    }

    #[must_use]
    pub const fn modules(&self) -> &BTreeMap<ModuleId, RuntimeModuleConfiguration> {
        &self.modules
    }

    fn validate(
        &self,
        profile: &TrainingProfile,
        network: &VirtualNetwork,
    ) -> Result<(), HardwareConditionError> {
        if self.devices.is_empty() {
            return Err(HardwareConditionError::InvalidConfiguration(
                "runtime device registry is empty",
                Uuid::NIL,
            ));
        }
        for (device_id, role) in &self.devices {
            if !network.devices().contains_key(device_id) {
                return Err(HardwareConditionError::InvalidConfiguration(
                    "configured runtime device is absent from VirtualNetwork",
                    device_id.uuid(),
                ));
            }
            if !network.interfaces().values().any(|interface| {
                interface.owner_device_id == *device_id && interface.role == role.network_role()
            }) {
                return Err(HardwareConditionError::InvalidConfiguration(
                    "runtime device has no interface with its configured role",
                    device_id.uuid(),
                ));
            }
        }
        for (module_id, module) in &self.modules {
            let configured = &module.configured_module;
            let definition = profile.module(configured.catalog_id).ok_or(
                HardwareConditionError::InvalidConfiguration(
                    "module catalog identity is absent from the profile",
                    module_id.uuid(),
                ),
            )?;
            if !self.devices.contains_key(&module.owner_device_id) {
                return Err(HardwareConditionError::InvalidConfiguration(
                    "module owner device is not configured",
                    module_id.uuid(),
                ));
            }
            if !module_channels_match_catalog(configured, definition) {
                return Err(HardwareConditionError::InvalidConfiguration(
                    "module channel configuration or diagnostic capability differs from the profile",
                    module_id.uuid(),
                ));
            }
            if let Some(link_id) = module.required_link_id
                && !network.links().contains_key(&link_id)
            {
                return Err(HardwareConditionError::InvalidConfiguration(
                    "module requires an unknown virtual link",
                    module_id.uuid(),
                ));
            }
            let provided_interfaces: Vec<_> = network
                .interfaces()
                .values()
                .filter(|interface| interface.provider_module_id == Some(*module_id))
                .collect();
            if configured.catalog_id == ModuleCatalogId::Vlink2 {
                if provided_interfaces.len() != 1 {
                    return Err(HardwareConditionError::InvalidConfiguration(
                        "VLINK-2 must provide exactly one virtual interface",
                        module_id.uuid(),
                    ));
                }
                let interface = provided_interfaces[0];
                let port_count = network
                    .ports()
                    .values()
                    .filter(|port| port.owner_interface_id == interface.id)
                    .count();
                if interface.owner_device_id != module.owner_device_id || port_count != 2 {
                    return Err(HardwareConditionError::InvalidConfiguration(
                        "VLINK-2 interface must belong to its device and provide two ports",
                        module_id.uuid(),
                    ));
                }
            } else if !provided_interfaces.is_empty() {
                return Err(HardwareConditionError::InvalidConfiguration(
                    "only VLINK-2 may provide a virtual interface",
                    module_id.uuid(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HardwareConditionKey {
    ModuleNotPresent(ModuleId),
    WrongModule(ModuleId),
    ChannelFault(ChannelId),
    WireBreak(ChannelId),
    ControllerUnpowered(VirtualDeviceId),
    StationUnavailable(VirtualDeviceId),
    LinkUnavailable(VirtualLinkId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HardwareDiagnosticCode {
    ModuleNotPresent,
    WrongModule,
    ChannelFault,
    WireBreak,
    ControllerUnpowered,
    StationUnavailable,
    LinkUnavailable,
}

impl HardwareDiagnosticCode {
    #[must_use]
    pub const fn stable_code(self) -> &'static str {
        match self {
            Self::ModuleNotPresent => "EDU-IO-0001",
            Self::WrongModule => "EDU-IO-0002",
            Self::ChannelFault => "EDU-IO-0003",
            Self::WireBreak => "EDU-IO-0004",
            Self::ControllerUnpowered | Self::StationUnavailable => "EDU-COM-0001",
            Self::LinkUnavailable => "EDU-COM-0002",
        }
    }
}

impl HardwareConditionKey {
    #[must_use]
    pub const fn diagnostic_code(self) -> HardwareDiagnosticCode {
        match self {
            Self::ModuleNotPresent(_) => HardwareDiagnosticCode::ModuleNotPresent,
            Self::WrongModule(_) => HardwareDiagnosticCode::WrongModule,
            Self::ChannelFault(_) => HardwareDiagnosticCode::ChannelFault,
            Self::WireBreak(_) => HardwareDiagnosticCode::WireBreak,
            Self::ControllerUnpowered(_) => HardwareDiagnosticCode::ControllerUnpowered,
            Self::StationUnavailable(_) => HardwareDiagnosticCode::StationUnavailable,
            Self::LinkUnavailable(_) => HardwareDiagnosticCode::LinkUnavailable,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConditionLifecycle {
    Activated,
    Cleared,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HardwareConditionEvent {
    pub sequence: u64,
    pub command_boundary: u64,
    pub condition: HardwareConditionKey,
    pub lifecycle: ConditionLifecycle,
    pub diagnostic_code: HardwareDiagnosticCode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HardwareFaultAction {
    PullModule(ModuleId),
    RestoreModule(ModuleId),
    InstallWrongModule {
        module_id: ModuleId,
        installed_catalog: ModuleCatalogId,
    },
    RestoreConfiguredModule(ModuleId),
    SetChannelFault(ChannelId),
    ClearChannelFault(ChannelId),
    SetWireBreak(ChannelId),
    ClearWireBreak(ChannelId),
    SetControllerPowered {
        device_id: VirtualDeviceId,
        powered: bool,
    },
    SetStationAvailable {
        device_id: VirtualDeviceId,
        available: bool,
    },
    SetVirtualLinkAvailable {
        link_id: VirtualLinkId,
        available: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HardwareFaultCommand {
    pub idempotency_key: Uuid,
    pub expected_controller_epoch: u64,
    pub action: HardwareFaultAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HardwareFaultReceipt {
    pub idempotency_key: Uuid,
    pub command_boundary: u64,
    pub changed: bool,
    pub events: Vec<HardwareConditionEvent>,
    pub state_fingerprint: Sha256Digest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NaturalChannelSample {
    pub raw_value: ChannelRawValue,
    pub provider_quality: ChannelQuality,
    pub force_overlay_active: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelConditionProjection {
    pub channel_id: ChannelId,
    pub direction: ChannelDirection,
    pub cpu_value: ChannelRawValue,
    pub delivered_value: ChannelRawValue,
    pub quality: ChannelQuality,
    pub delivery_suppressed: bool,
    pub force_overlay_active: bool,
    pub active_conditions: Vec<HardwareConditionKey>,
    /// One marker for monitoring, trace, snapshot comparison, and replay
    /// verification. Consumers must use this projection rather than recreate
    /// fault effects independently.
    pub causal_fingerprint: Sha256Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedHardwareCondition {
    pub condition: HardwareConditionKey,
    pub diagnostic_code: HardwareDiagnosticCode,
    pub acknowledged: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HardwareConditionSnapshot {
    pub profile_pin: ProfilePin,
    pub controller_epoch: u64,
    pub command_boundary: u64,
    pub module_states: BTreeMap<ModuleId, ModuleRuntimeState>,
    pub channel_faults: BTreeSet<ChannelId>,
    pub wire_breaks: BTreeSet<ChannelId>,
    pub controller_powered: BTreeMap<VirtualDeviceId, bool>,
    pub station_available: BTreeMap<VirtualDeviceId, bool>,
    pub link_available: BTreeMap<VirtualLinkId, bool>,
    pub active_conditions: Vec<HardwareConditionKey>,
    pub condition_events: Vec<HardwareConditionEvent>,
    pub network_state_fingerprint: Sha256Digest,
    pub state_fingerprint: Sha256Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HardwareConditionEngine {
    profile_pin: ProfilePin,
    configuration: RuntimeHardwareConfiguration,
    network: VirtualNetwork,
    controller_epoch: u64,
    command_boundary: u64,
    next_event_sequence: u64,
    module_states: BTreeMap<ModuleId, ModuleRuntimeState>,
    channel_faults: BTreeSet<ChannelId>,
    wire_breaks: BTreeSet<ChannelId>,
    controller_powered: BTreeMap<VirtualDeviceId, bool>,
    station_available: BTreeMap<VirtualDeviceId, bool>,
    link_available: BTreeMap<VirtualLinkId, bool>,
    events: Vec<HardwareConditionEvent>,
    acknowledged: BTreeSet<HardwareConditionKey>,
    receipts: BTreeMap<Uuid, (HardwareFaultCommand, HardwareFaultReceipt)>,
}

impl HardwareConditionEngine {
    pub fn new(
        profile_pin: ProfilePin,
        configuration: RuntimeHardwareConfiguration,
        network: VirtualNetwork,
        controller_epoch: u64,
    ) -> Result<Self, HardwareConditionError> {
        let profile = ProfileAllowlist::load(&profile_pin)?;
        profile.require_capability(Capability::HardwareFaults)?;
        profile.require_capability(Capability::VirtualNetwork)?;
        let diagnostics = network.validate_configuration();
        if !diagnostics.is_empty() {
            return Err(HardwareConditionError::Network(
                NetworkError::InvalidConfiguration(diagnostics),
            ));
        }
        configuration.validate(&profile, &network)?;
        let module_states = configuration
            .modules
            .keys()
            .copied()
            .map(|id| (id, ModuleRuntimeState::ConfiguredPresent))
            .collect();
        let mut controller_powered = BTreeMap::new();
        let mut station_available = BTreeMap::new();
        for (id, role) in &configuration.devices {
            let powered = network.devices()[id].powered_state == PoweredState::Powered;
            match role {
                RuntimeDeviceRole::Controller => {
                    controller_powered.insert(*id, powered);
                }
                RuntimeDeviceRole::Station => {
                    station_available.insert(*id, powered);
                }
            }
        }
        let link_available = network
            .links()
            .iter()
            .map(|(id, link)| (*id, link.runtime_state == RuntimeState::Available))
            .collect();
        let mut engine = Self {
            profile_pin,
            configuration,
            network,
            controller_epoch,
            command_boundary: 0,
            next_event_sequence: 1,
            module_states,
            channel_faults: BTreeSet::new(),
            wire_breaks: BTreeSet::new(),
            controller_powered,
            station_available,
            link_available,
            events: Vec::new(),
            acknowledged: BTreeSet::new(),
            receipts: BTreeMap::new(),
        };
        engine.reconcile_network()?;
        Ok(engine)
    }

    #[must_use]
    pub const fn configuration(&self) -> &RuntimeHardwareConfiguration {
        &self.configuration
    }

    #[must_use]
    pub const fn network(&self) -> &VirtualNetwork {
        &self.network
    }

    #[must_use]
    pub const fn controller_epoch(&self) -> u64 {
        self.controller_epoch
    }

    #[must_use]
    pub fn condition_events(&self) -> &[HardwareConditionEvent] {
        &self.events
    }

    pub fn apply(
        &mut self,
        command: HardwareFaultCommand,
    ) -> Result<HardwareFaultReceipt, HardwareConditionError> {
        if let Some((prior_command, receipt)) = self.receipts.get(&command.idempotency_key) {
            return if *prior_command == command {
                Ok(receipt.clone())
            } else {
                Err(HardwareConditionError::IdempotencyConflict(
                    command.idempotency_key,
                ))
            };
        }
        if command.expected_controller_epoch != self.controller_epoch {
            return Err(HardwareConditionError::StaleControllerEpoch {
                expected: command.expected_controller_epoch,
                actual: self.controller_epoch,
            });
        }
        let mut candidate = self.clone();
        let before_conditions: BTreeSet<_> = candidate.active_conditions().into_iter().collect();
        candidate.command_boundary = candidate
            .command_boundary
            .checked_add(1)
            .ok_or(HardwareConditionError::SequenceExhausted)?;
        candidate.apply_action(command.action)?;
        candidate.reconcile_network()?;
        let after_conditions: BTreeSet<_> = candidate.active_conditions().into_iter().collect();
        let transitions = before_conditions
            .difference(&after_conditions)
            .copied()
            .map(|condition| (condition, ConditionLifecycle::Cleared))
            .chain(
                after_conditions
                    .difference(&before_conditions)
                    .copied()
                    .map(|condition| (condition, ConditionLifecycle::Activated)),
            );
        let mut events = Vec::new();
        for (condition, lifecycle) in transitions {
            let sequence = candidate.next_event_sequence;
            candidate.next_event_sequence = candidate
                .next_event_sequence
                .checked_add(1)
                .ok_or(HardwareConditionError::SequenceExhausted)?;
            let event = HardwareConditionEvent {
                sequence,
                command_boundary: candidate.command_boundary,
                condition,
                lifecycle,
                diagnostic_code: condition.diagnostic_code(),
            };
            if lifecycle == ConditionLifecycle::Cleared {
                candidate.acknowledged.remove(&condition);
            }
            candidate.events.push(event.clone());
            events.push(event);
        }
        let receipt = HardwareFaultReceipt {
            idempotency_key: command.idempotency_key,
            command_boundary: candidate.command_boundary,
            changed: !events.is_empty(),
            events,
            state_fingerprint: candidate.state_fingerprint(),
        };
        candidate
            .receipts
            .insert(command.idempotency_key, (command, receipt.clone()));
        *self = candidate;
        Ok(receipt)
    }

    pub fn acknowledge(
        &mut self,
        condition: HardwareConditionKey,
    ) -> Result<bool, HardwareConditionError> {
        if !self.active_conditions().contains(&condition) {
            return Err(HardwareConditionError::InactiveCondition(condition));
        }
        Ok(self.acknowledged.insert(condition))
    }

    #[must_use]
    pub fn observed_conditions(&self) -> Vec<ObservedHardwareCondition> {
        self.active_conditions()
            .into_iter()
            .map(|condition| ObservedHardwareCondition {
                condition,
                diagnostic_code: condition.diagnostic_code(),
                acknowledged: self.acknowledged.contains(&condition),
            })
            .collect()
    }

    pub fn project_channel(
        &self,
        channel_id: ChannelId,
        sample: NaturalChannelSample,
    ) -> Result<ChannelConditionProjection, HardwareConditionError> {
        let module_id = *self
            .configuration
            .channel_owners
            .get(&channel_id)
            .ok_or(HardwareConditionError::InvalidTarget(channel_id.uuid()))?;
        let module = &self.configuration.modules[&module_id];
        let channel = module
            .configured_module
            .channels
            .iter()
            .find(|channel| channel.id == channel_id)
            .ok_or(HardwareConditionError::InvalidTarget(channel_id.uuid()))?;
        if !sample.raw_value.matches(channel.raw_type) {
            return Err(HardwareConditionError::RawTypeMismatch(channel_id));
        }
        let default = ChannelRawValue::canonical_default(channel.raw_type)
            .ok_or(HardwareConditionError::RawTypeMismatch(channel_id))?;
        let mut active = self.conditions_for_channel(module_id, channel_id);
        active.sort_unstable();
        active.dedup();
        let not_present = active.iter().any(|condition| {
            matches!(
                condition,
                HardwareConditionKey::ModuleNotPresent(_)
                    | HardwareConditionKey::WrongModule(_)
                    | HardwareConditionKey::ControllerUnpowered(_)
                    | HardwareConditionKey::StationUnavailable(_)
                    | HardwareConditionKey::LinkUnavailable(_)
            )
        });
        let bad = active.iter().any(|condition| {
            matches!(
                condition,
                HardwareConditionKey::ChannelFault(_) | HardwareConditionKey::WireBreak(_)
            )
        });
        let quality = if not_present {
            ChannelQuality::NotPresent
        } else if bad {
            ChannelQuality::Bad
        } else {
            sample.provider_quality
        };
        let (cpu_value, delivered_value, delivery_suppressed) = match channel.direction {
            ChannelDirection::Input if not_present || bad => (default, default, false),
            ChannelDirection::Output if not_present || bad => (sample.raw_value, default, true),
            ChannelDirection::Input | ChannelDirection::Output => {
                (sample.raw_value, sample.raw_value, false)
            }
        };
        let mut projection = ChannelConditionProjection {
            channel_id,
            direction: channel.direction,
            cpu_value,
            delivered_value,
            quality,
            delivery_suppressed,
            force_overlay_active: sample.force_overlay_active,
            active_conditions: active,
            causal_fingerprint: Sha256Digest([0; 32]),
        };
        projection.causal_fingerprint = self.projection_fingerprint(&projection);
        Ok(projection)
    }

    #[must_use]
    pub fn snapshot(&self) -> HardwareConditionSnapshot {
        HardwareConditionSnapshot {
            profile_pin: self.profile_pin.clone(),
            controller_epoch: self.controller_epoch,
            command_boundary: self.command_boundary,
            module_states: self.module_states.clone(),
            channel_faults: self.channel_faults.clone(),
            wire_breaks: self.wire_breaks.clone(),
            controller_powered: self.controller_powered.clone(),
            station_available: self.station_available.clone(),
            link_available: self.link_available.clone(),
            active_conditions: self.active_conditions(),
            condition_events: self.events.clone(),
            network_state_fingerprint: self.network.state_fingerprint(),
            state_fingerprint: self.state_fingerprint(),
        }
    }

    pub fn replay(
        profile_pin: ProfilePin,
        configuration: RuntimeHardwareConfiguration,
        network: VirtualNetwork,
        controller_epoch: u64,
        commands: impl IntoIterator<Item = HardwareFaultCommand>,
    ) -> Result<Self, HardwareConditionError> {
        let mut engine = Self::new(profile_pin, configuration, network, controller_epoch)?;
        for command in commands {
            engine.apply(command)?;
        }
        Ok(engine)
    }

    #[allow(clippy::too_many_lines)]
    fn apply_action(
        &mut self,
        action: HardwareFaultAction,
    ) -> Result<Option<(HardwareConditionKey, ConditionLifecycle)>, HardwareConditionError> {
        match action {
            HardwareFaultAction::PullModule(module_id) => {
                let state = self.module_state_mut(module_id)?;
                match *state {
                    ModuleRuntimeState::ConfiguredPresent => {
                        *state = ModuleRuntimeState::Pulled;
                        Ok(Some((
                            HardwareConditionKey::ModuleNotPresent(module_id),
                            ConditionLifecycle::Activated,
                        )))
                    }
                    ModuleRuntimeState::Pulled => Ok(None),
                    ModuleRuntimeState::WrongCatalogInstalled => {
                        Err(HardwareConditionError::ConditionConflict(module_id.uuid()))
                    }
                }
            }
            HardwareFaultAction::RestoreModule(module_id) => {
                let state = self.module_state_mut(module_id)?;
                match *state {
                    ModuleRuntimeState::Pulled => {
                        *state = ModuleRuntimeState::ConfiguredPresent;
                        Ok(Some((
                            HardwareConditionKey::ModuleNotPresent(module_id),
                            ConditionLifecycle::Cleared,
                        )))
                    }
                    ModuleRuntimeState::ConfiguredPresent => Ok(None),
                    ModuleRuntimeState::WrongCatalogInstalled => {
                        Err(HardwareConditionError::ConditionConflict(module_id.uuid()))
                    }
                }
            }
            HardwareFaultAction::InstallWrongModule {
                module_id,
                installed_catalog,
            } => {
                let configured_catalog = self
                    .configuration
                    .modules
                    .get(&module_id)
                    .ok_or(HardwareConditionError::InvalidTarget(module_id.uuid()))?
                    .configured_module
                    .catalog_id;
                if configured_catalog == installed_catalog {
                    return Err(HardwareConditionError::ConditionConflict(module_id.uuid()));
                }
                let state = self.module_state_mut(module_id)?;
                match *state {
                    ModuleRuntimeState::ConfiguredPresent => {
                        *state = ModuleRuntimeState::WrongCatalogInstalled;
                        Ok(Some((
                            HardwareConditionKey::WrongModule(module_id),
                            ConditionLifecycle::Activated,
                        )))
                    }
                    ModuleRuntimeState::WrongCatalogInstalled => Ok(None),
                    ModuleRuntimeState::Pulled => {
                        Err(HardwareConditionError::ConditionConflict(module_id.uuid()))
                    }
                }
            }
            HardwareFaultAction::RestoreConfiguredModule(module_id) => {
                let state = self.module_state_mut(module_id)?;
                match *state {
                    ModuleRuntimeState::WrongCatalogInstalled => {
                        *state = ModuleRuntimeState::ConfiguredPresent;
                        Ok(Some((
                            HardwareConditionKey::WrongModule(module_id),
                            ConditionLifecycle::Cleared,
                        )))
                    }
                    ModuleRuntimeState::ConfiguredPresent => Ok(None),
                    ModuleRuntimeState::Pulled => {
                        Err(HardwareConditionError::ConditionConflict(module_id.uuid()))
                    }
                }
            }
            HardwareFaultAction::SetChannelFault(channel_id) => {
                self.require_channel_capability(channel_id, false)?;
                Ok(self.channel_faults.insert(channel_id).then_some((
                    HardwareConditionKey::ChannelFault(channel_id),
                    ConditionLifecycle::Activated,
                )))
            }
            HardwareFaultAction::ClearChannelFault(channel_id) => {
                self.require_channel_capability(channel_id, false)?;
                Ok(self.channel_faults.remove(&channel_id).then_some((
                    HardwareConditionKey::ChannelFault(channel_id),
                    ConditionLifecycle::Cleared,
                )))
            }
            HardwareFaultAction::SetWireBreak(channel_id) => {
                self.require_channel_capability(channel_id, true)?;
                Ok(self.wire_breaks.insert(channel_id).then_some((
                    HardwareConditionKey::WireBreak(channel_id),
                    ConditionLifecycle::Activated,
                )))
            }
            HardwareFaultAction::ClearWireBreak(channel_id) => {
                self.require_channel_capability(channel_id, true)?;
                Ok(self.wire_breaks.remove(&channel_id).then_some((
                    HardwareConditionKey::WireBreak(channel_id),
                    ConditionLifecycle::Cleared,
                )))
            }
            HardwareFaultAction::SetControllerPowered { device_id, powered } => {
                let state = self
                    .controller_powered
                    .get_mut(&device_id)
                    .ok_or(HardwareConditionError::InvalidTarget(device_id.uuid()))?;
                let changed = *state != powered;
                *state = powered;
                Ok(changed.then_some((
                    HardwareConditionKey::ControllerUnpowered(device_id),
                    if powered {
                        ConditionLifecycle::Cleared
                    } else {
                        ConditionLifecycle::Activated
                    },
                )))
            }
            HardwareFaultAction::SetStationAvailable {
                device_id,
                available,
            } => {
                let state = self
                    .station_available
                    .get_mut(&device_id)
                    .ok_or(HardwareConditionError::InvalidTarget(device_id.uuid()))?;
                let changed = *state != available;
                *state = available;
                Ok(changed.then_some((
                    HardwareConditionKey::StationUnavailable(device_id),
                    if available {
                        ConditionLifecycle::Cleared
                    } else {
                        ConditionLifecycle::Activated
                    },
                )))
            }
            HardwareFaultAction::SetVirtualLinkAvailable { link_id, available } => {
                let state = self
                    .link_available
                    .get_mut(&link_id)
                    .ok_or(HardwareConditionError::InvalidTarget(link_id.uuid()))?;
                let changed = *state != available;
                *state = available;
                Ok(changed.then_some((
                    HardwareConditionKey::LinkUnavailable(link_id),
                    if available {
                        ConditionLifecycle::Cleared
                    } else {
                        ConditionLifecycle::Activated
                    },
                )))
            }
        }
    }

    fn module_state_mut(
        &mut self,
        module_id: ModuleId,
    ) -> Result<&mut ModuleRuntimeState, HardwareConditionError> {
        self.module_states
            .get_mut(&module_id)
            .ok_or(HardwareConditionError::InvalidTarget(module_id.uuid()))
    }

    fn require_channel_capability(
        &self,
        channel_id: ChannelId,
        wire_break: bool,
    ) -> Result<(), HardwareConditionError> {
        let module_id = self
            .configuration
            .channel_owners
            .get(&channel_id)
            .ok_or(HardwareConditionError::InvalidTarget(channel_id.uuid()))?;
        let channel = self.configuration.modules[module_id]
            .configured_module
            .channels
            .iter()
            .find(|channel| channel.id == channel_id)
            .ok_or(HardwareConditionError::InvalidTarget(channel_id.uuid()))?;
        let supported = if wire_break {
            channel.diagnostic_capabilities.wire_break
        } else {
            channel.diagnostic_capabilities.channel_fault
        };
        if supported {
            Ok(())
        } else {
            Err(HardwareConditionError::CapabilityUnavailable(channel_id))
        }
    }

    fn conditions_for_channel(
        &self,
        module_id: ModuleId,
        channel_id: ChannelId,
    ) -> Vec<HardwareConditionKey> {
        let module = &self.configuration.modules[&module_id];
        let mut conditions = Vec::new();
        match self.module_states[&module_id] {
            ModuleRuntimeState::ConfiguredPresent => {}
            ModuleRuntimeState::Pulled => {
                conditions.push(HardwareConditionKey::ModuleNotPresent(module_id));
            }
            ModuleRuntimeState::WrongCatalogInstalled => {
                conditions.push(HardwareConditionKey::WrongModule(module_id));
            }
        }
        match self.configuration.devices[&module.owner_device_id] {
            RuntimeDeviceRole::Controller if !self.controller_powered[&module.owner_device_id] => {
                conditions.push(HardwareConditionKey::ControllerUnpowered(
                    module.owner_device_id,
                ));
            }
            RuntimeDeviceRole::Station if !self.station_available[&module.owner_device_id] => {
                conditions.push(HardwareConditionKey::StationUnavailable(
                    module.owner_device_id,
                ));
            }
            RuntimeDeviceRole::Controller | RuntimeDeviceRole::Station => {}
        }
        if let Some(link_id) = module.required_link_id
            && self
                .network
                .effective_link_runtime_state(link_id)
                .map_or(true, |state| state == RuntimeState::Unavailable)
        {
            conditions.push(HardwareConditionKey::LinkUnavailable(link_id));
        }
        if self.channel_faults.contains(&channel_id) {
            conditions.push(HardwareConditionKey::ChannelFault(channel_id));
        }
        if self.wire_breaks.contains(&channel_id) {
            conditions.push(HardwareConditionKey::WireBreak(channel_id));
        }
        conditions
    }

    fn active_conditions(&self) -> Vec<HardwareConditionKey> {
        let mut conditions = Vec::new();
        for (id, state) in &self.module_states {
            match state {
                ModuleRuntimeState::ConfiguredPresent => {}
                ModuleRuntimeState::Pulled => {
                    conditions.push(HardwareConditionKey::ModuleNotPresent(*id));
                }
                ModuleRuntimeState::WrongCatalogInstalled => {
                    conditions.push(HardwareConditionKey::WrongModule(*id));
                }
            }
        }
        conditions.extend(
            self.channel_faults
                .iter()
                .copied()
                .map(HardwareConditionKey::ChannelFault),
        );
        conditions.extend(
            self.wire_breaks
                .iter()
                .copied()
                .map(HardwareConditionKey::WireBreak),
        );
        conditions.extend(
            self.controller_powered
                .iter()
                .filter(|(_, powered)| !**powered)
                .map(|(id, _)| HardwareConditionKey::ControllerUnpowered(*id)),
        );
        conditions.extend(
            self.station_available
                .iter()
                .filter(|(_, available)| !**available)
                .map(|(id, _)| HardwareConditionKey::StationUnavailable(*id)),
        );
        conditions.extend(self.link_available.keys().filter_map(|id| {
            (self
                .network
                .effective_link_runtime_state(*id)
                .map_or(true, |state| state == RuntimeState::Unavailable))
            .then_some(HardwareConditionKey::LinkUnavailable(*id))
        }));
        conditions.sort_unstable();
        conditions
    }

    fn reconcile_network(&mut self) -> Result<(), HardwareConditionError> {
        for (device_id, role) in &self.configuration.devices {
            let available = match role {
                RuntimeDeviceRole::Controller => self.controller_powered[device_id],
                RuntimeDeviceRole::Station => self.station_available[device_id],
            };
            self.network.set_device_powered(
                *device_id,
                if available {
                    PoweredState::Powered
                } else {
                    PoweredState::Unpowered
                },
            )?;
            self.network.set_device_attachment_runtime_state(
                *device_id,
                if available {
                    RuntimeState::Available
                } else {
                    RuntimeState::Unavailable
                },
            )?;
        }
        for (module_id, module) in &self.configuration.modules {
            if module.configured_module.catalog_id != ModuleCatalogId::Vlink2 {
                continue;
            }
            let owner_available = match self.configuration.devices[&module.owner_device_id] {
                RuntimeDeviceRole::Controller => self.controller_powered[&module.owner_device_id],
                RuntimeDeviceRole::Station => self.station_available[&module.owner_device_id],
            };
            let available = owner_available
                && self.module_states[module_id] == ModuleRuntimeState::ConfiguredPresent;
            self.network.set_provider_module_runtime_state(
                *module_id,
                if available {
                    RuntimeState::Available
                } else {
                    RuntimeState::Unavailable
                },
            )?;
        }
        for (link_id, available) in &self.link_available {
            self.network.set_link_runtime_state(
                *link_id,
                if *available {
                    RuntimeState::Available
                } else {
                    RuntimeState::Unavailable
                },
            )?;
        }
        Ok(())
    }

    fn projection_fingerprint(&self, projection: &ChannelConditionProjection) -> Sha256Digest {
        let mut encoder = CanonicalEncoder::default();
        encoder.domain("EDU21-HARDWARE-CHANNEL-PROJECTION-V1");
        encoder.digest(self.state_fingerprint());
        encoder.uuid(projection.channel_id.uuid());
        encoder.u8(match projection.direction {
            ChannelDirection::Input => 0,
            ChannelDirection::Output => 1,
        });
        encode_raw(projection.cpu_value, &mut encoder);
        encode_raw(projection.delivered_value, &mut encoder);
        encoder.u8(quality_byte(projection.quality));
        encoder.bool(projection.delivery_suppressed);
        encoder.bool(projection.force_overlay_active);
        encoder.usize(projection.active_conditions.len());
        for condition in &projection.active_conditions {
            encode_condition(*condition, &mut encoder);
        }
        encoder.fingerprint()
    }

    fn state_fingerprint(&self) -> Sha256Digest {
        let mut encoder = CanonicalEncoder::default();
        encoder.domain("EDU21-HARDWARE-CONDITION-STATE-V1");
        encoder.text(&self.profile_pin.id);
        encoder.text(&self.profile_pin.version);
        encoder.digest(self.profile_pin.manifest_hash);
        encoder.u64(self.controller_epoch);
        encoder.u64(self.command_boundary);
        encoder.usize(self.module_states.len());
        for (id, state) in &self.module_states {
            encoder.uuid(id.uuid());
            encoder.u8(match state {
                ModuleRuntimeState::ConfiguredPresent => 0,
                ModuleRuntimeState::Pulled => 1,
                ModuleRuntimeState::WrongCatalogInstalled => 2,
            });
        }
        let active = self.active_conditions();
        encoder.usize(active.len());
        for condition in active {
            encode_condition(condition, &mut encoder);
        }
        encoder.usize(self.events.len());
        for event in &self.events {
            encoder.u64(event.sequence);
            encoder.u64(event.command_boundary);
            encode_condition(event.condition, &mut encoder);
            encoder.u8(match event.lifecycle {
                ConditionLifecycle::Activated => 1,
                ConditionLifecycle::Cleared => 0,
            });
        }
        encoder.digest(self.network.state_fingerprint());
        encoder.fingerprint()
    }
}

fn module_channels_match_catalog(module: &ConfiguredModule, definition: &ModuleDefinition) -> bool {
    if module.channels.len() != definition.channels.channel_count() {
        return false;
    }
    let mut channel_ids = BTreeSet::new();
    let mut parameter_ids = BTreeSet::new();
    module.channels.iter().enumerate().all(|(index, channel)| {
        let Ok(index) = u8::try_from(index) else {
            return false;
        };
        let Some((direction, direction_index, raw_type, has_scaling)) =
            expected_channel_shape(definition.channels, index)
        else {
            return false;
        };
        let scaling_valid = match (&channel.engineering_scaling, has_scaling) {
            (None, false) => true,
            (Some(scaling), true) => {
                scaling.engineering_min.get() < scaling.engineering_max.get()
                    && parameter_ids.insert(scaling.id)
            }
            (None, true) | (Some(_), false) => false,
        };
        channel.declared_ordinal == index
            && channel.direction == direction
            && channel.direction_index == direction_index
            && channel.raw_type == raw_type
            && channel.diagnostic_capabilities.channel_fault
            && channel.diagnostic_capabilities.wire_break == definition.supports_wire_break
            && channel_ids.insert(channel.id)
            && scaling_valid
    })
}

fn expected_channel_shape(
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
        ChannelLayout::AnalogInputs(count) | ChannelLayout::TemperatureInputs(count)
            if ordinal < count =>
        {
            Some((ChannelDirection::Input, ordinal, PrimitiveType::Int, true))
        }
        ChannelLayout::AnalogOutputs(count) if ordinal < count => {
            Some((ChannelDirection::Output, ordinal, PrimitiveType::Int, true))
        }
        ChannelLayout::MixedDigital { inputs, .. } if ordinal < inputs => {
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
        ChannelLayout::None
        | ChannelLayout::DigitalInputs(_)
        | ChannelLayout::DigitalOutputs(_)
        | ChannelLayout::AnalogInputs(_)
        | ChannelLayout::AnalogOutputs(_)
        | ChannelLayout::MixedDigital { .. }
        | ChannelLayout::TemperatureInputs(_) => None,
    }
}

fn encode_raw(value: ChannelRawValue, encoder: &mut CanonicalEncoder) {
    match value {
        ChannelRawValue::Bool(value) => {
            encoder.tag("bool");
            encoder.bool(value);
        }
        ChannelRawValue::Int(value) => {
            encoder.tag("int");
            encoder.i32(i32::from(value));
        }
    }
}

fn quality_byte(quality: ChannelQuality) -> u8 {
    match quality {
        ChannelQuality::Good => 0,
        ChannelQuality::Uncertain => 1,
        ChannelQuality::Bad => 2,
        ChannelQuality::NotPresent => 3,
    }
}

fn encode_condition(condition: HardwareConditionKey, encoder: &mut CanonicalEncoder) {
    match condition {
        HardwareConditionKey::ModuleNotPresent(id) => {
            encoder.tag("module-not-present");
            encoder.uuid(id.uuid());
        }
        HardwareConditionKey::WrongModule(id) => {
            encoder.tag("wrong-module");
            encoder.uuid(id.uuid());
        }
        HardwareConditionKey::ChannelFault(id) => {
            encoder.tag("channel-fault");
            encoder.uuid(id.uuid());
        }
        HardwareConditionKey::WireBreak(id) => {
            encoder.tag("wire-break");
            encoder.uuid(id.uuid());
        }
        HardwareConditionKey::ControllerUnpowered(id) => {
            encoder.tag("controller-unpowered");
            encoder.uuid(id.uuid());
        }
        HardwareConditionKey::StationUnavailable(id) => {
            encoder.tag("station-unavailable");
            encoder.uuid(id.uuid());
        }
        HardwareConditionKey::LinkUnavailable(id) => {
            encoder.tag("link-unavailable");
            encoder.uuid(id.uuid());
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HardwareConditionError {
    Profile(ProfileError),
    Network(NetworkError),
    DuplicateIdentity(Uuid),
    InvalidConfiguration(&'static str, Uuid),
    InvalidTarget(Uuid),
    CapabilityUnavailable(ChannelId),
    ConditionConflict(Uuid),
    InactiveCondition(HardwareConditionKey),
    RawTypeMismatch(ChannelId),
    IdempotencyConflict(Uuid),
    StaleControllerEpoch { expected: u64, actual: u64 },
    SequenceExhausted,
}

impl From<ProfileError> for HardwareConditionError {
    fn from(value: ProfileError) -> Self {
        Self::Profile(value)
    }
}

impl From<NetworkError> for HardwareConditionError {
    fn from(value: NetworkError) -> Self {
        Self::Network(value)
    }
}

impl fmt::Display for HardwareConditionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for HardwareConditionError {}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use plc_core::Uuid;

    use super::{
        HardwareConditionEngine, HardwareConditionError, HardwareFaultAction, HardwareFaultCommand,
        ModuleRuntimeState, RuntimeHardwareConfiguration,
    };
    use crate::{ModuleId, TrainingProfile, VirtualNetwork};

    #[test]
    fn event_sequence_exhaustion_rejects_the_entire_command_atomically() {
        let module_id = ModuleId::from(Uuid::deterministic_v4(b"sequence-module", 1));
        let mut engine = HardwareConditionEngine {
            profile_pin: TrainingProfile::edu21().pin(),
            configuration: RuntimeHardwareConfiguration::new(),
            network: VirtualNetwork::new(),
            controller_epoch: 7,
            command_boundary: 0,
            next_event_sequence: u64::MAX,
            module_states: BTreeMap::from([(module_id, ModuleRuntimeState::ConfiguredPresent)]),
            channel_faults: BTreeSet::default(),
            wire_breaks: BTreeSet::default(),
            controller_powered: BTreeMap::default(),
            station_available: BTreeMap::default(),
            link_available: BTreeMap::default(),
            events: Vec::new(),
            acknowledged: BTreeSet::default(),
            receipts: BTreeMap::default(),
        };
        let before = engine.snapshot();
        assert_eq!(
            engine.apply(HardwareFaultCommand {
                idempotency_key: Uuid::deterministic_v4(b"sequence-command", 1),
                expected_controller_epoch: 7,
                action: HardwareFaultAction::PullModule(module_id),
            }),
            Err(HardwareConditionError::SequenceExhausted)
        );
        assert_eq!(engine.snapshot(), before);
    }
}
