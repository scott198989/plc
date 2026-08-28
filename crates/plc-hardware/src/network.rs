#![allow(clippy::missing_errors_doc, clippy::too_many_lines)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

use plc_core::{Sha256Digest, Uuid};

use crate::canonical::CanonicalEncoder;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticTarget, TargetKind};
use crate::ids::{
    ModuleId, VirtualDeviceId, VirtualInterfaceId, VirtualLinkId, VirtualPortId, VirtualSubnetId,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PoweredState {
    Powered,
    Unpowered,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConfiguredState {
    Enabled,
    Disabled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuntimeState {
    Available,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PortClass {
    EduLink,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeviceRole {
    Controller,
    Station,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VirtualDeviceName(String);

impl VirtualDeviceName {
    pub fn parse(input: &str) -> Result<Self, NetworkError> {
        let bytes = input.as_bytes();
        if bytes.is_empty()
            || bytes.len() > 63
            || !bytes[0].is_ascii_lowercase()
            || !bytes
                .iter()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
            || bytes.last() == Some(&b'-')
        {
            return Err(NetworkError::InvalidDeviceName(input.to_owned()));
        }
        Ok(Self(input.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for VirtualDeviceName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VirtualIpAddress([u8; 4]);

impl VirtualIpAddress {
    pub fn parse(input: &str) -> Result<Self, NetworkError> {
        if input.is_empty()
            || input.bytes().any(|byte| byte.is_ascii_whitespace())
            || input.contains(['+', '-', ':', '/', '\\', '[', ']', '%'])
        {
            return Err(NetworkError::InvalidVirtualAddress(input.to_owned()));
        }
        let mut octets = [0_u8; 4];
        let mut count = 0_usize;
        for component in input.split('.') {
            if count == 4
                || component.is_empty()
                || !component.bytes().all(|byte| byte.is_ascii_digit())
                || (component.len() > 1 && component.starts_with('0'))
            {
                return Err(NetworkError::InvalidVirtualAddress(input.to_owned()));
            }
            octets[count] = component
                .parse::<u8>()
                .map_err(|_| NetworkError::InvalidVirtualAddress(input.to_owned()))?;
            count += 1;
        }
        if count != 4 {
            return Err(NetworkError::InvalidVirtualAddress(input.to_owned()));
        }
        Ok(Self(octets))
    }

    #[must_use]
    pub const fn octets(self) -> [u8; 4] {
        self.0
    }

    #[must_use]
    pub const fn as_u32(self) -> u32 {
        u32::from_be_bytes(self.0)
    }

    #[must_use]
    pub const fn from_u32(value: u32) -> Self {
        Self(value.to_be_bytes())
    }
}

impl fmt::Display for VirtualIpAddress {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}.{}.{}.{}",
            self.0[0], self.0[1], self.0[2], self.0[3]
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VirtualDevice {
    pub id: VirtualDeviceId,
    pub creation_ordinal: u64,
    pub device_name: VirtualDeviceName,
    pub powered_state: PoweredState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VirtualInterface {
    pub id: VirtualInterfaceId,
    pub creation_ordinal: u64,
    pub owner_device_id: VirtualDeviceId,
    pub provider_module_id: Option<ModuleId>,
    pub name: String,
    pub address: Option<VirtualIpAddress>,
    pub subnet_id: Option<VirtualSubnetId>,
    pub port_class: PortClass,
    pub role: DeviceRole,
    pub configured_state: ConfiguredState,
    pub runtime_state: RuntimeState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VirtualSubnet {
    pub id: VirtualSubnetId,
    pub creation_ordinal: u64,
    pub name: String,
    pub network_address: VirtualIpAddress,
    pub prefix_length: u8,
}

impl VirtualSubnet {
    #[must_use]
    pub fn mask(&self) -> Option<u32> {
        if self.prefix_length > 30 {
            return None;
        }
        Some(if self.prefix_length == 0 {
            0
        } else {
            u32::MAX << (32 - self.prefix_length)
        })
    }

    #[must_use]
    pub fn broadcast_address(&self) -> Option<VirtualIpAddress> {
        let mask = self.mask()?;
        Some(VirtualIpAddress::from_u32(
            self.network_address.as_u32() | !mask,
        ))
    }

    #[must_use]
    pub fn contains_assignable(&self, address: VirtualIpAddress) -> bool {
        let Some(mask) = self.mask() else {
            return false;
        };
        let address_bits = address.as_u32();
        let network_bits = self.network_address.as_u32();
        (address_bits & mask) == network_bits
            && address_bits != network_bits
            && self
                .broadcast_address()
                .is_some_and(|broadcast| address != broadcast)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VirtualPort {
    pub id: VirtualPortId,
    pub creation_ordinal: u64,
    pub owner_interface_id: VirtualInterfaceId,
    pub name: String,
    pub configured_state: ConfiguredState,
    pub runtime_state: RuntimeState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VirtualLink {
    pub id: VirtualLinkId,
    pub creation_ordinal: u64,
    pub endpoint_port_ids: [VirtualPortId; 2],
    pub configured_state: ConfiguredState,
    pub runtime_state: RuntimeState,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VirtualNetwork {
    devices: BTreeMap<VirtualDeviceId, VirtualDevice>,
    interfaces: BTreeMap<VirtualInterfaceId, VirtualInterface>,
    subnets: BTreeMap<VirtualSubnetId, VirtualSubnet>,
    ports: BTreeMap<VirtualPortId, VirtualPort>,
    links: BTreeMap<VirtualLinkId, VirtualLink>,
}

impl VirtualNetwork {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_device(&mut self, device: VirtualDevice) -> Result<(), NetworkError> {
        insert_unique(&mut self.devices, device.id, device)
    }

    pub fn add_interface(&mut self, interface: VirtualInterface) -> Result<(), NetworkError> {
        insert_unique(&mut self.interfaces, interface.id, interface)
    }

    pub fn add_subnet(&mut self, subnet: VirtualSubnet) -> Result<(), NetworkError> {
        insert_unique(&mut self.subnets, subnet.id, subnet)
    }

    pub fn add_port(&mut self, port: VirtualPort) -> Result<(), NetworkError> {
        insert_unique(&mut self.ports, port.id, port)
    }

    pub fn add_link(&mut self, link: VirtualLink) -> Result<(), NetworkError> {
        insert_unique(&mut self.links, link.id, link)
    }

    #[must_use]
    pub const fn devices(&self) -> &BTreeMap<VirtualDeviceId, VirtualDevice> {
        &self.devices
    }

    #[must_use]
    pub const fn interfaces(&self) -> &BTreeMap<VirtualInterfaceId, VirtualInterface> {
        &self.interfaces
    }

    #[must_use]
    pub const fn subnets(&self) -> &BTreeMap<VirtualSubnetId, VirtualSubnet> {
        &self.subnets
    }

    #[must_use]
    pub const fn ports(&self) -> &BTreeMap<VirtualPortId, VirtualPort> {
        &self.ports
    }

    #[must_use]
    pub const fn links(&self) -> &BTreeMap<VirtualLinkId, VirtualLink> {
        &self.links
    }

    pub fn set_device_powered(
        &mut self,
        id: VirtualDeviceId,
        state: PoweredState,
    ) -> Result<bool, NetworkError> {
        let device = self
            .devices
            .get_mut(&id)
            .ok_or(NetworkError::UnknownIdentity(id.uuid()))?;
        let changed = device.powered_state != state;
        device.powered_state = state;
        Ok(changed)
    }

    pub fn set_link_runtime_state(
        &mut self,
        id: VirtualLinkId,
        state: RuntimeState,
    ) -> Result<bool, NetworkError> {
        let link = self
            .links
            .get_mut(&id)
            .ok_or(NetworkError::UnknownIdentity(id.uuid()))?;
        let changed = link.runtime_state != state;
        link.runtime_state = state;
        Ok(changed)
    }

    #[must_use]
    pub fn validate_configuration(&self) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        self.validate_identities(&mut diagnostics);
        self.validate_subnets(&mut diagnostics);
        self.validate_interfaces(&mut diagnostics);
        self.validate_topology(&mut diagnostics);
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

    pub fn query_visible_devices(
        &self,
        query_interface_id: VirtualInterfaceId,
        filter: &DiscoveryFilter,
    ) -> Result<Vec<DiscoveryResult>, NetworkError> {
        let diagnostics = self.validate_configuration();
        if !diagnostics.is_empty() {
            return Err(NetworkError::InvalidConfiguration(diagnostics));
        }
        let query = self
            .interfaces
            .get(&query_interface_id)
            .ok_or(NetworkError::UnknownIdentity(query_interface_id.uuid()))?;
        let query_subnet = query.subnet_id.ok_or(NetworkError::QueryUnavailable)?;
        if query.configured_state != ConfiguredState::Enabled
            || query.runtime_state != RuntimeState::Available
            || query.address.is_none()
            || self
                .devices
                .get(&query.owner_device_id)
                .is_none_or(|device| device.powered_state != PoweredState::Powered)
        {
            return Err(NetworkError::QueryUnavailable);
        }

        let mut queue = VecDeque::from([query_interface_id]);
        let mut visited_interfaces = BTreeSet::from([query_interface_id]);
        let mut visible_devices = BTreeSet::new();
        let mut ordered_links: Vec<_> = self.links.values().collect();
        ordered_links.sort_by_key(|link| {
            let [first, second] = link.endpoint_port_ids;
            (
                link.creation_ordinal,
                first.min(second),
                first.max(second),
                link.id,
            )
        });

        while let Some(interface_id) = queue.pop_front() {
            let owned_ports: BTreeSet<_> = self
                .ports
                .values()
                .filter(|port| port.owner_interface_id == interface_id)
                .map(|port| port.id)
                .collect();
            for link in &ordered_links {
                let [first, second] = link.endpoint_port_ids;
                let other_port = if owned_ports.contains(&first) {
                    Some(second)
                } else if owned_ports.contains(&second) {
                    Some(first)
                } else {
                    None
                };
                let Some(other_port) = other_port else {
                    continue;
                };
                if !self.link_is_traversable(link) {
                    continue;
                }
                let other_interface = self.ports[&other_port].owner_interface_id;
                let interface = &self.interfaces[&other_interface];
                if interface.subnet_id != Some(query_subnet) {
                    continue;
                }
                if visited_interfaces.insert(other_interface) {
                    queue.push_back(other_interface);
                }
                if interface.owner_device_id != query.owner_device_id {
                    visible_devices.insert(interface.owner_device_id);
                }
            }
        }

        let mut results = Vec::new();
        for device_id in visible_devices {
            let device = &self.devices[&device_id];
            let selected = self
                .interfaces
                .values()
                .filter(|interface| {
                    interface.owner_device_id == device_id
                        && interface.subnet_id == Some(query_subnet)
                        && interface.configured_state == ConfiguredState::Enabled
                        && interface.runtime_state == RuntimeState::Available
                })
                .min_by_key(|interface| (interface.creation_ordinal, interface.id));
            let Some(interface) = selected else {
                continue;
            };
            if !filter.roles.is_empty() && !filter.roles.contains(&interface.role) {
                continue;
            }
            let Some(configured_address) = interface.address else {
                continue;
            };
            results.push(DiscoveryResult {
                subnet_id: query_subnet,
                device_id,
                device_name: device.device_name.clone(),
                device_creation_ordinal: device.creation_ordinal,
                interface_id: interface.id,
                configured_address,
                role: interface.role,
                runtime_state: interface.runtime_state,
                compatible: true,
            });
        }
        results.sort_by_key(|result| {
            (
                self.subnets[&result.subnet_id].creation_ordinal,
                result.device_creation_ordinal,
                result.device_id,
            )
        });
        Ok(results)
    }

    /// Returns configured EDU-LINK reachability without consulting runtime
    /// power or availability state. Hardware compilation uses this to prove
    /// that required engineering topology exists; runtime discovery uses the
    /// stricter state-aware query above.
    #[must_use]
    pub fn is_configured_reachable(
        &self,
        source_device_id: VirtualDeviceId,
        target_device_id: VirtualDeviceId,
    ) -> bool {
        if source_device_id == target_device_id
            || !self.devices.contains_key(&source_device_id)
            || !self.devices.contains_key(&target_device_id)
            || !self.validate_configuration().is_empty()
        {
            return false;
        }
        let mut queue: VecDeque<_> = self
            .interfaces
            .values()
            .filter(|interface| {
                interface.owner_device_id == source_device_id
                    && interface.configured_state == ConfiguredState::Enabled
                    && interface.subnet_id.is_some()
                    && interface.address.is_some()
            })
            .map(|interface| interface.id)
            .collect();
        let mut visited: BTreeSet<_> = queue.iter().copied().collect();
        let mut ordered_links: Vec<_> = self.links.values().collect();
        ordered_links.sort_by_key(|link| {
            let [first, second] = link.endpoint_port_ids;
            (
                link.creation_ordinal,
                first.min(second),
                first.max(second),
                link.id,
            )
        });
        while let Some(interface_id) = queue.pop_front() {
            let interface = &self.interfaces[&interface_id];
            let owned_ports: BTreeSet<_> = self
                .ports
                .values()
                .filter(|port| port.owner_interface_id == interface_id)
                .map(|port| port.id)
                .collect();
            for link in &ordered_links {
                if !self.link_is_configured_traversable(link) {
                    continue;
                }
                let [first, second] = link.endpoint_port_ids;
                let other = if owned_ports.contains(&first) {
                    Some(second)
                } else if owned_ports.contains(&second) {
                    Some(first)
                } else {
                    None
                };
                let Some(other) = other else {
                    continue;
                };
                let other_interface_id = self.ports[&other].owner_interface_id;
                let other_interface = &self.interfaces[&other_interface_id];
                if other_interface.subnet_id != interface.subnet_id {
                    continue;
                }
                if other_interface.owner_device_id == target_device_id {
                    return true;
                }
                if visited.insert(other_interface_id) {
                    queue.push_back(other_interface_id);
                }
            }
        }
        false
    }

    #[must_use]
    pub fn configuration_fingerprint(&self) -> Sha256Digest {
        self.encode(false).fingerprint()
    }

    #[must_use]
    pub fn state_fingerprint(&self) -> Sha256Digest {
        self.encode(true).fingerprint()
    }

    fn validate_identities(&self, diagnostics: &mut Vec<Diagnostic>) {
        let identities: Vec<_> = self
            .devices
            .keys()
            .map(|id| (TargetKind::VirtualDevice, id.uuid()))
            .chain(
                self.interfaces
                    .keys()
                    .map(|id| (TargetKind::VirtualInterface, id.uuid())),
            )
            .chain(
                self.subnets
                    .keys()
                    .map(|id| (TargetKind::VirtualSubnet, id.uuid())),
            )
            .chain(
                self.ports
                    .keys()
                    .map(|id| (TargetKind::VirtualPort, id.uuid())),
            )
            .chain(
                self.links
                    .keys()
                    .map(|id| (TargetKind::VirtualLink, id.uuid())),
            )
            .collect();
        let mut seen = BTreeMap::new();
        for (kind, id) in identities {
            if !id.is_rfc9562_v4() {
                diagnostics.push(Diagnostic::blocking(
                    DiagnosticCode::NetworkTopologyInvalid,
                    DiagnosticTarget::new(kind, id),
                    "VirtualNetwork identities must be RFC 9562 UUIDv4-shaped simulator IDs",
                ));
            }
            if let Some(previous_kind) = seen.insert(id, kind) {
                diagnostics.push(
                    Diagnostic::blocking(
                        DiagnosticCode::NetworkTopologyInvalid,
                        DiagnosticTarget::new(kind, id),
                        "A VirtualNetwork UUID is reused by more than one canonical object",
                    )
                    .related([DiagnosticTarget::new(previous_kind, id)]),
                );
            }
        }
    }

    fn validate_subnets(&self, diagnostics: &mut Vec<Diagnostic>) {
        for subnet in self.subnets.values() {
            let target = DiagnosticTarget::new(TargetKind::VirtualSubnet, subnet.id.uuid());
            let Some(mask) = subnet.mask() else {
                diagnostics.push(Diagnostic::blocking(
                    DiagnosticCode::NetworkAddressNameOrSubnet,
                    target.field("prefixLength"),
                    "EDU-21 prefix length must be from 0 through 30",
                ));
                continue;
            };
            if subnet.network_address.as_u32() & !mask != 0 {
                diagnostics.push(Diagnostic::blocking(
                    DiagnosticCode::NetworkAddressNameOrSubnet,
                    target.clone().field("networkAddress"),
                    "VirtualSubnet networkAddress contains host bits",
                ));
            }
            if subnet.name.is_empty() || subnet.name.len() > 128 {
                diagnostics.push(Diagnostic::blocking(
                    DiagnosticCode::NetworkAddressNameOrSubnet,
                    target.field("name"),
                    "VirtualSubnet name must contain 1 through 128 bytes",
                ));
            }
        }
    }

    fn validate_interfaces(&self, diagnostics: &mut Vec<Diagnostic>) {
        let linked_interfaces: BTreeSet<_> = self
            .links
            .values()
            .flat_map(|link| link.endpoint_port_ids)
            .filter_map(|port_id| self.ports.get(&port_id))
            .map(|port| port.owner_interface_id)
            .collect();
        let mut addresses = BTreeMap::new();
        let mut names: BTreeMap<(VirtualSubnetId, String), VirtualDeviceId> = BTreeMap::new();

        for interface in self.interfaces.values() {
            let target = DiagnosticTarget::new(TargetKind::VirtualInterface, interface.id.uuid());
            let Some(device) = self.devices.get(&interface.owner_device_id) else {
                diagnostics.push(Diagnostic::blocking(
                    DiagnosticCode::NetworkTopologyInvalid,
                    target.clone().field("ownerDeviceId"),
                    "VirtualInterface owner device does not exist",
                ));
                continue;
            };
            if interface.name.is_empty() || interface.name.len() > 128 {
                diagnostics.push(Diagnostic::blocking(
                    DiagnosticCode::NetworkAddressNameOrSubnet,
                    target.clone().field("name"),
                    "VirtualInterface name must contain 1 through 128 bytes",
                ));
            }
            let assignment_required = interface.configured_state == ConfiguredState::Enabled
                || linked_interfaces.contains(&interface.id);
            if assignment_required && (interface.subnet_id.is_none() || interface.address.is_none())
            {
                diagnostics.push(Diagnostic::blocking(
                    DiagnosticCode::NetworkAddressNameOrSubnet,
                    target.clone().field("address/subnetId"),
                    "Enabled or linked EDU-LINK interface requires one subnet and one address",
                ));
                continue;
            }
            if interface.address.is_some() != interface.subnet_id.is_some() {
                diagnostics.push(Diagnostic::blocking(
                    DiagnosticCode::NetworkAddressNameOrSubnet,
                    target.clone().field("address/subnetId"),
                    "Virtual address and subnet membership must be assigned together",
                ));
                continue;
            }
            let (Some(subnet_id), Some(address)) = (interface.subnet_id, interface.address) else {
                continue;
            };
            let Some(subnet) = self.subnets.get(&subnet_id) else {
                diagnostics.push(Diagnostic::blocking(
                    DiagnosticCode::NetworkAddressNameOrSubnet,
                    target.clone().field("subnetId"),
                    "VirtualInterface references a missing VirtualSubnet",
                ));
                continue;
            };
            if !subnet.contains_assignable(address) {
                diagnostics.push(Diagnostic::blocking(
                    DiagnosticCode::NetworkAddressNameOrSubnet,
                    target.clone().field("address"),
                    "VirtualInterface address is outside the subnet or is its network/broadcast value",
                ));
            }
            if let Some(other) = addresses.insert((subnet_id, address), interface.id) {
                diagnostics.push(
                    Diagnostic::blocking(
                        DiagnosticCode::NetworkAddressNameOrSubnet,
                        target.clone().field("address"),
                        "VirtualInterface address duplicates another interface in this subnet",
                    )
                    .related([DiagnosticTarget::new(
                        TargetKind::VirtualInterface,
                        other.uuid(),
                    )]),
                );
            }
            let name_key = (subnet_id, device.device_name.as_str().to_owned());
            if let Some(other_device) = names.insert(name_key, device.id)
                && other_device != device.id
            {
                diagnostics.push(
                    Diagnostic::blocking(
                        DiagnosticCode::NetworkAddressNameOrSubnet,
                        DiagnosticTarget::new(TargetKind::VirtualDevice, device.id.uuid())
                            .field("deviceName"),
                        "VirtualDeviceName duplicates another device in this subnet",
                    )
                    .related([DiagnosticTarget::new(
                        TargetKind::VirtualDevice,
                        other_device.uuid(),
                    )]),
                );
            }
        }
    }

    fn validate_topology(&self, diagnostics: &mut Vec<Diagnostic>) {
        let mut linked_ports = BTreeMap::new();
        for port in self.ports.values() {
            if !self.interfaces.contains_key(&port.owner_interface_id) {
                diagnostics.push(Diagnostic::blocking(
                    DiagnosticCode::NetworkTopologyInvalid,
                    DiagnosticTarget::new(TargetKind::VirtualPort, port.id.uuid())
                        .field("ownerInterfaceId"),
                    "VirtualPort owner interface does not exist",
                ));
            }
            if port.name.is_empty() || port.name.len() > 128 {
                diagnostics.push(Diagnostic::blocking(
                    DiagnosticCode::NetworkTopologyInvalid,
                    DiagnosticTarget::new(TargetKind::VirtualPort, port.id.uuid()).field("name"),
                    "VirtualPort name must contain 1 through 128 bytes",
                ));
            }
        }

        for link in self.links.values() {
            let target = DiagnosticTarget::new(TargetKind::VirtualLink, link.id.uuid());
            let [first_id, second_id] = link.endpoint_port_ids;
            if first_id == second_id {
                diagnostics.push(Diagnostic::blocking(
                    DiagnosticCode::NetworkTopologyInvalid,
                    target.clone().field("endpointPortIds"),
                    "VirtualLink endpoints must be distinct",
                ));
                continue;
            }
            let (Some(first), Some(second)) =
                (self.ports.get(&first_id), self.ports.get(&second_id))
            else {
                diagnostics.push(Diagnostic::blocking(
                    DiagnosticCode::NetworkTopologyInvalid,
                    target.clone().field("endpointPortIds"),
                    "VirtualLink has a dangling endpoint",
                ));
                continue;
            };
            for port_id in [first_id, second_id] {
                if let Some(other) = linked_ports.insert(port_id, link.id) {
                    diagnostics.push(
                        Diagnostic::blocking(
                            DiagnosticCode::NetworkTopologyInvalid,
                            target.clone().field("endpointPortIds"),
                            "EDU-LINK VirtualPort accepts only one VirtualLink",
                        )
                        .related([DiagnosticTarget::new(TargetKind::VirtualLink, other.uuid())]),
                    );
                }
            }
            let (Some(first_interface), Some(second_interface)) = (
                self.interfaces.get(&first.owner_interface_id),
                self.interfaces.get(&second.owner_interface_id),
            ) else {
                continue;
            };
            let compatible_roles = matches!(
                (first_interface.role, second_interface.role),
                (
                    DeviceRole::Controller | DeviceRole::Station,
                    DeviceRole::Station
                ) | (DeviceRole::Station, DeviceRole::Controller)
            );
            if first_interface.owner_device_id == second_interface.owner_device_id
                || first_interface.port_class != second_interface.port_class
                || !compatible_roles
            {
                diagnostics.push(
                    Diagnostic::blocking(
                        DiagnosticCode::NetworkTopologyInvalid,
                        target.clone().field("endpointPortIds"),
                        "VirtualLink endpoints have an incompatible role, port class, or owner-device pairing",
                    )
                    .related([
                        DiagnosticTarget::new(TargetKind::VirtualPort, first.id.uuid()),
                        DiagnosticTarget::new(TargetKind::VirtualPort, second.id.uuid()),
                    ]),
                );
            }
            if first_interface.subnet_id.is_none()
                || first_interface.subnet_id != second_interface.subnet_id
            {
                diagnostics.push(Diagnostic::blocking(
                    DiagnosticCode::NetworkTopologyInvalid,
                    target.field("endpointPortIds"),
                    "VirtualLink endpoints must already belong to the same VirtualSubnet",
                ));
            }
        }
    }

    fn link_is_traversable(&self, link: &VirtualLink) -> bool {
        if link.configured_state != ConfiguredState::Enabled
            || link.runtime_state != RuntimeState::Available
        {
            return false;
        }
        link.endpoint_port_ids.iter().all(|port_id| {
            let Some(port) = self.ports.get(port_id) else {
                return false;
            };
            let Some(interface) = self.interfaces.get(&port.owner_interface_id) else {
                return false;
            };
            let Some(device) = self.devices.get(&interface.owner_device_id) else {
                return false;
            };
            port.configured_state == ConfiguredState::Enabled
                && port.runtime_state == RuntimeState::Available
                && interface.configured_state == ConfiguredState::Enabled
                && interface.runtime_state == RuntimeState::Available
                && device.powered_state == PoweredState::Powered
        })
    }

    fn link_is_configured_traversable(&self, link: &VirtualLink) -> bool {
        link.configured_state == ConfiguredState::Enabled
            && link.endpoint_port_ids.iter().all(|port_id| {
                self.ports.get(port_id).is_some_and(|port| {
                    port.configured_state == ConfiguredState::Enabled
                        && self
                            .interfaces
                            .get(&port.owner_interface_id)
                            .is_some_and(|interface| {
                                interface.configured_state == ConfiguredState::Enabled
                            })
                })
            })
    }

    fn encode(&self, include_runtime: bool) -> CanonicalEncoder {
        let mut encoder = CanonicalEncoder::default();
        encoder.domain(if include_runtime {
            "EDU21-VIRTUAL-NETWORK-STATE-V1"
        } else {
            "EDU21-VIRTUAL-NETWORK-CONFIG-V1"
        });
        encoder.usize(self.devices.len());
        for device in self.devices.values() {
            encoder.uuid(device.id.uuid());
            encoder.u64(device.creation_ordinal);
            encoder.text(device.device_name.as_str());
            if include_runtime {
                encoder.u8(enum_byte(device.powered_state));
            }
        }
        encoder.usize(self.subnets.len());
        for subnet in self.subnets.values() {
            encoder.uuid(subnet.id.uuid());
            encoder.u64(subnet.creation_ordinal);
            encoder.text(&subnet.name);
            for octet in subnet.network_address.octets() {
                encoder.u8(octet);
            }
            encoder.u8(subnet.prefix_length);
        }
        encoder.usize(self.interfaces.len());
        for interface in self.interfaces.values() {
            encoder.uuid(interface.id.uuid());
            encoder.u64(interface.creation_ordinal);
            encoder.uuid(interface.owner_device_id.uuid());
            encoder.option(interface.provider_module_id, |encoder, id| {
                encoder.uuid(id.uuid());
            });
            encoder.text(&interface.name);
            encoder.option(interface.address, |encoder, address| {
                for octet in address.octets() {
                    encoder.u8(octet);
                }
            });
            encoder.option(interface.subnet_id, |encoder, id| encoder.uuid(id.uuid()));
            encoder.u8(enum_byte(interface.port_class));
            encoder.u8(enum_byte(interface.role));
            encoder.u8(enum_byte(interface.configured_state));
            if include_runtime {
                encoder.u8(enum_byte(interface.runtime_state));
            }
        }
        encoder.usize(self.ports.len());
        for port in self.ports.values() {
            encoder.uuid(port.id.uuid());
            encoder.u64(port.creation_ordinal);
            encoder.uuid(port.owner_interface_id.uuid());
            encoder.text(&port.name);
            encoder.u8(enum_byte(port.configured_state));
            if include_runtime {
                encoder.u8(enum_byte(port.runtime_state));
            }
        }
        encoder.usize(self.links.len());
        for link in self.links.values() {
            encoder.uuid(link.id.uuid());
            encoder.u64(link.creation_ordinal);
            encoder.uuid(link.endpoint_port_ids[0].uuid());
            encoder.uuid(link.endpoint_port_ids[1].uuid());
            encoder.u8(enum_byte(link.configured_state));
            if include_runtime {
                encoder.u8(enum_byte(link.runtime_state));
            }
        }
        encoder
    }
}

fn insert_unique<K, V>(map: &mut BTreeMap<K, V>, key: K, value: V) -> Result<(), NetworkError>
where
    K: Copy + Ord + IntoUuid,
{
    if map.contains_key(&key) {
        return Err(NetworkError::DuplicateIdentity(key.into_uuid()));
    }
    map.insert(key, value);
    Ok(())
}

trait IntoUuid {
    fn into_uuid(self) -> Uuid;
}

macro_rules! into_uuid {
    ($($type:ty),+ $(,)?) => {
        $(
            impl IntoUuid for $type {
                fn into_uuid(self) -> Uuid {
                    self.uuid()
                }
            }
        )+
    };
}

into_uuid!(
    VirtualDeviceId,
    VirtualInterfaceId,
    VirtualSubnetId,
    VirtualPortId,
    VirtualLinkId
);

fn enum_byte<T: Copy>(value: T) -> u8
where
    u8: From<T>,
{
    u8::from(value)
}

macro_rules! enum_u8 {
    ($type:ty, {$($variant:path => $value:expr),+ $(,)?}) => {
        impl From<$type> for u8 {
            fn from(value: $type) -> Self {
                match value {
                    $($variant => $value),+
                }
            }
        }
    };
}

enum_u8!(PoweredState, { PoweredState::Powered => 1, PoweredState::Unpowered => 0 });
enum_u8!(ConfiguredState, { ConfiguredState::Enabled => 1, ConfiguredState::Disabled => 0 });
enum_u8!(RuntimeState, { RuntimeState::Available => 1, RuntimeState::Unavailable => 0 });
enum_u8!(PortClass, { PortClass::EduLink => 1 });
enum_u8!(DeviceRole, { DeviceRole::Controller => 1, DeviceRole::Station => 2 });

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiscoveryFilter {
    pub roles: BTreeSet<DeviceRole>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveryResult {
    pub subnet_id: VirtualSubnetId,
    pub device_id: VirtualDeviceId,
    pub device_name: VirtualDeviceName,
    pub device_creation_ordinal: u64,
    pub interface_id: VirtualInterfaceId,
    pub configured_address: VirtualIpAddress,
    pub role: DeviceRole,
    pub runtime_state: RuntimeState,
    pub compatible: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NetworkError {
    InvalidVirtualAddress(String),
    InvalidDeviceName(String),
    DuplicateIdentity(Uuid),
    UnknownIdentity(Uuid),
    InvalidConfiguration(Vec<Diagnostic>),
    QueryUnavailable,
}

impl fmt::Display for NetworkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for NetworkError {}

#[cfg(test)]
mod tests {
    use plc_core::Uuid;

    use crate::diagnostic::DiagnosticCode;
    use crate::ids::{VirtualDeviceId, VirtualSubnetId};

    use super::{
        PoweredState, VirtualDevice, VirtualDeviceName, VirtualIpAddress, VirtualNetwork,
        VirtualSubnet,
    };

    #[test]
    fn virtual_identifiers_reject_endpoint_shaped_and_noncanonical_text() {
        for unsafe_value in [
            "localhost",
            "http://10.1.2.3",
            "10.1.2.3:80",
            "10.1.2.3/path",
            "010.1.2.3",
            "10.1.2",
            "::1",
            " 10.1.2.3",
            "+10.1.2.3",
            "256.1.2.3",
        ] {
            assert!(
                VirtualIpAddress::parse(unsafe_value).is_err(),
                "{unsafe_value}"
            );
        }
        assert_eq!(
            VirtualIpAddress::parse("127.0.0.1")
                .expect("virtual loopback-shaped value remains inert")
                .to_string(),
            "127.0.0.1"
        );
        for inert_value in ["10.0.0.1", "8.8.8.8", "224.0.0.1", "255.255.255.255"] {
            assert_eq!(
                VirtualIpAddress::parse(inert_value)
                    .expect("valid dotted decimal remains simulator data")
                    .to_string(),
                inert_value
            );
        }
        for invalid in ["A", "a_1", "-a", "a-", "a.b", ""] {
            assert!(VirtualDeviceName::parse(invalid).is_err(), "{invalid}");
        }
        assert_eq!(
            VirtualDeviceName::parse("cell-a1")
                .expect("valid virtual name")
                .as_str(),
            "cell-a1"
        );
        assert!(Uuid::deterministic_v4(b"network", 1).is_rfc9562_v4());
    }

    #[test]
    fn cross_kind_uuid_reuse_is_a_blocking_topology_error() {
        let reused = Uuid::deterministic_v4(b"network-collision", 1);
        let mut network = VirtualNetwork::new();
        network
            .add_device(VirtualDevice {
                id: VirtualDeviceId(reused),
                creation_ordinal: 1,
                device_name: VirtualDeviceName::parse("device-a").expect("valid name"),
                powered_state: PoweredState::Powered,
            })
            .expect("device draft");
        network
            .add_subnet(VirtualSubnet {
                id: VirtualSubnetId(reused),
                creation_ordinal: 2,
                name: "Subnet".to_owned(),
                network_address: VirtualIpAddress::parse("10.0.0.0").expect("network"),
                prefix_length: 24,
            })
            .expect("subnet draft");
        assert!(network.validate_configuration().iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::NetworkTopologyInvalid
                && diagnostic.primary.id == reused
        }));
    }
}
