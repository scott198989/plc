#![allow(clippy::missing_errors_doc)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use plc_core::Sha256Digest;

use crate::canonical::CanonicalEncoder;

pub const EDU21_PROFILE_ID: &str = "EDU-21 Core";
pub const EDU21_PROFILE_VERSION: &str = "1.0.0";
pub const EDU21_CATALOG_VERSION: &str = "1.0.0";
/// Fine-grained compiler capabilities projected from the shipped training
/// profile. The compiler hashes this ordered material independently while the
/// training-profile manifest remains the authority for whether the compiler
/// and SCL capability families are admitted at all.
pub const EDU21_COMPILER_CAPABILITY_KEYS: [&str; 5] = [
    "scl.assignment",
    "scl.call.fc",
    "scl.expression.baseline",
    "scl.if",
    "scl.return",
];
pub const EDU21_MANIFEST_HASH: Sha256Digest = Sha256Digest([
    0x9f, 0xeb, 0xe0, 0x0e, 0x57, 0x9c, 0x16, 0x19, 0x20, 0x61, 0x0b, 0xe4, 0xd2, 0x07, 0x96, 0x21,
    0xb6, 0x25, 0x52, 0x17, 0xa6, 0x23, 0xf2, 0x9e, 0xe0, 0xf6, 0x56, 0xfc, 0xd9, 0x92, 0xed, 0x9a,
]);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfilePin {
    pub id: String,
    pub version: String,
    pub manifest_hash: Sha256Digest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Capability {
    FictionalHardware,
    AutomaticAddressAllocation,
    AnalogScaling,
    HardwareFaults,
    VirtualNetwork,
    VirtualDiscovery,
    CanonicalTypes,
    Symbols,
    Constants,
    Lad,
    Fbd,
    Scl,
    Compiler,
    DeterministicRuntime,
    Monitoring,
    Modify,
    Force,
    Trace,
    Diagnostics,
    Snapshots,
    Replay,
    VirtualCommissioning,
}

impl Capability {
    pub const ALL: [Self; 22] = [
        Self::FictionalHardware,
        Self::AutomaticAddressAllocation,
        Self::AnalogScaling,
        Self::HardwareFaults,
        Self::VirtualNetwork,
        Self::VirtualDiscovery,
        Self::CanonicalTypes,
        Self::Symbols,
        Self::Constants,
        Self::Lad,
        Self::Fbd,
        Self::Scl,
        Self::Compiler,
        Self::DeterministicRuntime,
        Self::Monitoring,
        Self::Modify,
        Self::Force,
        Self::Trace,
        Self::Diagnostics,
        Self::Snapshots,
        Self::Replay,
        Self::VirtualCommissioning,
    ];

    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::FictionalHardware => "hardware.fictional",
            Self::AutomaticAddressAllocation => "hardware.auto-address",
            Self::AnalogScaling => "hardware.analog-scaling",
            Self::HardwareFaults => "hardware.causal-faults",
            Self::VirtualNetwork => "network.virtual-graph",
            Self::VirtualDiscovery => "network.virtual-discovery",
            Self::CanonicalTypes => "types.canonical",
            Self::Symbols => "symbols.scoped-bindings",
            Self::Constants => "symbols.constants",
            Self::Lad => "language.lad",
            Self::Fbd => "language.fbd",
            Self::Scl => "language.scl",
            Self::Compiler => "compiler.typed-ir",
            Self::DeterministicRuntime => "runtime.deterministic",
            Self::Monitoring => "runtime.monitoring",
            Self::Modify => "runtime.modify",
            Self::Force => "runtime.force",
            Self::Trace => "runtime.trace",
            Self::Diagnostics => "runtime.diagnostics",
            Self::Snapshots => "runtime.snapshots",
            Self::Replay => "runtime.replay",
            Self::VirtualCommissioning => "commissioning.virtual-only",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileLimits {
    pub controllers_per_project: u32,
    pub project_objects: u32,
    pub tags_per_controller: u32,
    pub named_types_per_project: u32,
    pub source_bytes_per_block: u32,
    pub type_nesting: u8,
    pub members_per_type: u32,
    pub array_dimensions: u8,
    pub array_elements: u64,
    pub syntax_nesting: u32,
    pub networks_per_block: u32,
    pub nodes_per_network: u32,
    pub edges_per_network: u32,
    pub dependency_edges: u64,
    pub diagnostics_per_build: u32,
    pub constant_evaluation_operations: u64,
    pub semantic_work_units_per_build: u64,
    pub artifact_package_bytes: u64,
    pub package_bytes: u64,
    pub package_member_bytes: u64,
    pub expanded_package_bytes: u64,
    pub package_members: u32,
    pub package_nesting: u8,
    pub normalized_path_bytes: u32,
    pub string_field_bytes: u32,
    pub expansion_ratio: u32,
    pub watch_tables_per_project: u32,
    pub watch_rows_per_table: u32,
    pub active_subscriptions_per_controller: u32,
    pub retained_samples_per_watch_row: u32,
    pub trace_configurations_per_project: u32,
    pub trace_channels_per_capture: u32,
    pub trace_samples_per_capture: u64,
    pub concurrent_traces_per_controller: u32,
    pub trace_duration_virtual_ms: u64,
    pub trace_trigger_depth: u8,
    pub trace_trigger_nodes: u32,
    pub active_conditions_per_controller: u32,
    pub ordinary_conditions_per_controller: u32,
    pub retained_diagnostic_events: u32,
    pub snapshot_bytes: u64,
}

impl ProfileLimits {
    #[must_use]
    pub const fn edu21() -> Self {
        Self {
            controllers_per_project: 8,
            project_objects: 100_000,
            tags_per_controller: 32_768,
            named_types_per_project: 2_048,
            source_bytes_per_block: 1_048_576,
            type_nesting: 32,
            members_per_type: 4_096,
            array_dimensions: 6,
            array_elements: 1_000_000,
            syntax_nesting: 256,
            networks_per_block: 10_000,
            nodes_per_network: 10_000,
            edges_per_network: 20_000,
            dependency_edges: 1_000_000,
            diagnostics_per_build: 10_000,
            constant_evaluation_operations: 1_000_000,
            semantic_work_units_per_build: 10_000_000,
            artifact_package_bytes: 268_435_456,
            package_bytes: 536_870_912,
            package_member_bytes: 268_435_456,
            expanded_package_bytes: 1_073_741_824,
            package_members: 100_000,
            package_nesting: 32,
            normalized_path_bytes: 512,
            string_field_bytes: 1_048_576,
            expansion_ratio: 100,
            watch_tables_per_project: 64,
            watch_rows_per_table: 512,
            active_subscriptions_per_controller: 2_048,
            retained_samples_per_watch_row: 1_024,
            trace_configurations_per_project: 64,
            trace_channels_per_capture: 64,
            trace_samples_per_capture: 1_000_000,
            concurrent_traces_per_controller: 4,
            trace_duration_virtual_ms: 86_400_000,
            trace_trigger_depth: 32,
            trace_trigger_nodes: 256,
            active_conditions_per_controller: 10_000,
            ordinary_conditions_per_controller: 9_999,
            retained_diagnostic_events: 100_000,
            snapshot_bytes: 268_435_456,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchedulingPolicy {
    pub scan_quantum_ms: u32,
    pub cyclic_main_count: u8,
    pub startup_count: u8,
    pub timed_cyclic_count: u8,
    pub work_units_per_scan: u32,
    pub call_depth: u8,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticPolicy {
    pub unreachable_scl_is_blocking: bool,
    pub multiple_writer_is_blocking: bool,
    pub unsafe_temp_is_blocking: bool,
    pub missing_consumed_return_is_blocking: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ValueLifecycleAction {
    Preserve,
    ReloadLoadedStart,
    RestoreRetainStore,
    ClearThenLoadRetainedStart,
    PreserveCompatibleIdentity,
    PreserveDeclaredCompatibleIdentity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IoLifecycleAction {
    FreshInputOutputDefaultsUntilCommit,
    EffectiveOutputDefaults,
    ResetFreshInput,
    Reset,
    NoMixedScan,
    ResetAsPreviewed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArtifactLifecycleAction {
    Preserve,
    ReplaceAtomically,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ForceLifecycleAction {
    Preserve,
    ClearThroughApprovedPreview,
    RegistryMustBeEmpty,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LifecycleRule {
    pub non_retentive: ValueLifecycleAction,
    pub retentive: ValueLifecycleAction,
    pub io: IoLifecycleAction,
    pub artifact: ArtifactLifecycleAction,
    pub forces: ForceLifecycleAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestartRetentionPolicy {
    pub stop_to_run: LifecycleRule,
    pub run_to_stop: LifecycleRule,
    pub warm_restart: LifecycleRule,
    pub simulated_power_cycle: LifecycleRule,
    pub memory_reset: LifecycleRule,
    pub compatible_code_load: LifecycleRule,
    pub schema_changing_load: LifecycleRule,
}

impl RestartRetentionPolicy {
    #[must_use]
    pub const fn edu21() -> Self {
        use ArtifactLifecycleAction::{Preserve as PreserveArtifact, ReplaceAtomically};
        use ForceLifecycleAction::{
            ClearThroughApprovedPreview, Preserve as PreserveForces, RegistryMustBeEmpty,
        };
        use IoLifecycleAction::{
            EffectiveOutputDefaults, FreshInputOutputDefaultsUntilCommit, NoMixedScan, Reset,
            ResetAsPreviewed, ResetFreshInput,
        };
        use ValueLifecycleAction::{
            ClearThenLoadRetainedStart, Preserve, PreserveCompatibleIdentity,
            PreserveDeclaredCompatibleIdentity, ReloadLoadedStart, RestoreRetainStore,
        };
        Self {
            stop_to_run: LifecycleRule {
                non_retentive: Preserve,
                retentive: Preserve,
                io: FreshInputOutputDefaultsUntilCommit,
                artifact: PreserveArtifact,
                forces: PreserveForces,
            },
            run_to_stop: LifecycleRule {
                non_retentive: Preserve,
                retentive: Preserve,
                io: EffectiveOutputDefaults,
                artifact: PreserveArtifact,
                forces: PreserveForces,
            },
            warm_restart: LifecycleRule {
                non_retentive: ReloadLoadedStart,
                retentive: RestoreRetainStore,
                io: ResetFreshInput,
                artifact: PreserveArtifact,
                forces: PreserveForces,
            },
            simulated_power_cycle: LifecycleRule {
                non_retentive: ReloadLoadedStart,
                retentive: RestoreRetainStore,
                io: Reset,
                artifact: PreserveArtifact,
                forces: ClearThroughApprovedPreview,
            },
            memory_reset: LifecycleRule {
                non_retentive: ReloadLoadedStart,
                retentive: ClearThenLoadRetainedStart,
                io: Reset,
                artifact: PreserveArtifact,
                forces: ClearThroughApprovedPreview,
            },
            compatible_code_load: LifecycleRule {
                non_retentive: PreserveCompatibleIdentity,
                retentive: PreserveCompatibleIdentity,
                io: NoMixedScan,
                artifact: ReplaceAtomically,
                forces: RegistryMustBeEmpty,
            },
            schema_changing_load: LifecycleRule {
                non_retentive: PreserveDeclaredCompatibleIdentity,
                retentive: PreserveDeclaredCompatibleIdentity,
                io: ResetAsPreviewed,
                artifact: ReplaceAtomically,
                forces: RegistryMustBeEmpty,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ControllerCatalogId {
    VctrlC1,
    VctrlM1,
    VctrlP1,
}

impl ControllerCatalogId {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::VctrlC1 => "VCTRL-C1",
            Self::VctrlM1 => "VCTRL-M1",
            Self::VctrlP1 => "VCTRL-P1",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControllerDefinition {
    pub id: ControllerCatalogId,
    pub display_name: &'static str,
    pub local_first_expansion_slot: u8,
    pub local_last_slot: u8,
    pub requires_power_slot_zero: bool,
    pub controller_slot: Option<u8>,
    pub distributed_stations: u8,
    pub input_bytes: u32,
    pub output_bytes: u32,
    pub marker_bytes: u32,
    pub db_data_bytes: u32,
    pub block_capacity: u32,
    pub integrated_interfaces: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ModuleCatalogId {
    Vpwr1,
    VstnH1,
    Vdi16,
    Vdo16,
    Vai4,
    Vao4,
    Vmix8,
    Vrtd4,
    Vlink2,
}

impl ModuleCatalogId {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Vpwr1 => "VPWR-1",
            Self::VstnH1 => "VSTN-H1",
            Self::Vdi16 => "VDI-16",
            Self::Vdo16 => "VDO-16",
            Self::Vai4 => "VAI-4",
            Self::Vao4 => "VAO-4",
            Self::Vmix8 => "VMIX-8",
            Self::Vrtd4 => "VRTD-4",
            Self::Vlink2 => "VLINK-2",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelLayout {
    None,
    DigitalInputs(u8),
    DigitalOutputs(u8),
    AnalogInputs(u8),
    AnalogOutputs(u8),
    MixedDigital { inputs: u8, outputs: u8 },
    TemperatureInputs(u8),
}

impl ChannelLayout {
    #[must_use]
    pub const fn channel_count(self) -> usize {
        match self {
            Self::None => 0,
            Self::DigitalInputs(count)
            | Self::DigitalOutputs(count)
            | Self::AnalogInputs(count)
            | Self::AnalogOutputs(count)
            | Self::TemperatureInputs(count) => count as usize,
            Self::MixedDigital { inputs, outputs } => inputs as usize + outputs as usize,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlacementClass {
    ModularPowerSlotZero,
    StationHeadSlotZero,
    Expansion,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleDefinition {
    pub id: ModuleCatalogId,
    pub display_name: &'static str,
    pub channels: ChannelLayout,
    pub input_bytes: u32,
    pub output_bytes: u32,
    pub placement: PlacementClass,
    pub virtual_ports: u8,
    pub supports_wire_break: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrainingProfile {
    id: &'static str,
    version: &'static str,
    catalog_version: &'static str,
    capabilities: BTreeSet<Capability>,
    limits: ProfileLimits,
    scheduling: SchedulingPolicy,
    diagnostics: DiagnosticPolicy,
    restart_retention: RestartRetentionPolicy,
    controllers: BTreeMap<ControllerCatalogId, ControllerDefinition>,
    modules: BTreeMap<ModuleCatalogId, ModuleDefinition>,
    manifest_hash: Sha256Digest,
}

impl TrainingProfile {
    #[must_use]
    pub fn edu21() -> Self {
        let mut profile = Self {
            id: EDU21_PROFILE_ID,
            version: EDU21_PROFILE_VERSION,
            catalog_version: EDU21_CATALOG_VERSION,
            capabilities: Capability::ALL.into_iter().collect(),
            limits: ProfileLimits::edu21(),
            scheduling: SchedulingPolicy {
                scan_quantum_ms: 10,
                cyclic_main_count: 1,
                startup_count: 1,
                timed_cyclic_count: 8,
                work_units_per_scan: 100_000,
                call_depth: 64,
            },
            diagnostics: DiagnosticPolicy {
                unreachable_scl_is_blocking: false,
                multiple_writer_is_blocking: false,
                unsafe_temp_is_blocking: true,
                missing_consumed_return_is_blocking: true,
            },
            restart_retention: RestartRetentionPolicy::edu21(),
            controllers: controller_catalog(),
            modules: module_catalog(),
            manifest_hash: Sha256Digest([0; 32]),
        };
        profile.manifest_hash = profile.compute_manifest_hash();
        debug_assert!(profile.validate().is_ok());
        profile
    }

    #[must_use]
    pub const fn id(&self) -> &str {
        self.id
    }

    #[must_use]
    pub const fn version(&self) -> &str {
        self.version
    }

    #[must_use]
    pub const fn catalog_version(&self) -> &str {
        self.catalog_version
    }

    #[must_use]
    pub const fn limits(&self) -> &ProfileLimits {
        &self.limits
    }

    #[must_use]
    pub const fn scheduling(&self) -> &SchedulingPolicy {
        &self.scheduling
    }

    #[must_use]
    pub const fn diagnostic_policy(&self) -> &DiagnosticPolicy {
        &self.diagnostics
    }

    #[must_use]
    pub const fn restart_retention_policy(&self) -> &RestartRetentionPolicy {
        &self.restart_retention
    }

    #[must_use]
    pub const fn manifest_hash(&self) -> Sha256Digest {
        self.manifest_hash
    }

    #[must_use]
    pub fn pin(&self) -> ProfilePin {
        ProfilePin {
            id: self.id.to_owned(),
            version: self.version.to_owned(),
            manifest_hash: self.manifest_hash,
        }
    }

    #[must_use]
    pub fn capability_enabled(&self, capability: Capability) -> bool {
        self.capabilities.contains(&capability)
    }

    pub fn require_capability(&self, capability: Capability) -> Result<(), ProfileError> {
        if self.capability_enabled(capability) {
            Ok(())
        } else {
            Err(ProfileError::CapabilityUnavailable(capability))
        }
    }

    /// Returns the strict, lexically ordered compiler capability material
    /// authorized by this profile. A profile without both the compiler and SCL
    /// capability families supplies no compiler surface and therefore fails
    /// closed when projected into the compiler crate.
    #[must_use]
    pub fn compiler_capability_keys(&self) -> &'static [&'static str] {
        if self.capability_enabled(Capability::Compiler) && self.capability_enabled(Capability::Scl)
        {
            &EDU21_COMPILER_CAPABILITY_KEYS
        } else {
            &[]
        }
    }

    #[must_use]
    pub fn controller(&self, id: ControllerCatalogId) -> Option<&ControllerDefinition> {
        self.controllers.get(&id)
    }

    #[must_use]
    pub fn module(&self, id: ModuleCatalogId) -> Option<&ModuleDefinition> {
        self.modules.get(&id)
    }

    #[must_use]
    pub fn controllers(&self) -> &BTreeMap<ControllerCatalogId, ControllerDefinition> {
        &self.controllers
    }

    #[must_use]
    pub fn modules(&self) -> &BTreeMap<ModuleCatalogId, ModuleDefinition> {
        &self.modules
    }

    pub fn validate(&self) -> Result<(), ProfileError> {
        if self.id != EDU21_PROFILE_ID
            || self.version != EDU21_PROFILE_VERSION
            || self.catalog_version != EDU21_CATALOG_VERSION
        {
            return Err(ProfileError::InvalidManifest("immutable identity"));
        }
        if Capability::ALL
            .iter()
            .any(|capability| !self.capabilities.contains(capability))
        {
            return Err(ProfileError::InvalidManifest("required capability"));
        }
        if self.capabilities.len() != Capability::ALL.len() {
            return Err(ProfileError::InvalidManifest("unknown capability"));
        }
        if self.limits.controllers_per_project != 8
            || self.limits.type_nesting == 0
            || self.limits.array_dimensions == 0
            || self
                .limits
                .ordinary_conditions_per_controller
                .saturating_add(1)
                != self.limits.active_conditions_per_controller
        {
            return Err(ProfileError::InvalidManifest("limit inconsistency"));
        }
        if self.controllers.len() != 3 || self.modules.len() != 9 {
            return Err(ProfileError::InvalidManifest("catalog completeness"));
        }
        if self.manifest_hash != EDU21_MANIFEST_HASH
            || self.compute_manifest_hash() != self.manifest_hash
        {
            return Err(ProfileError::HashMismatch);
        }
        Ok(())
    }

    #[must_use]
    pub fn canonical_manifest_bytes(&self) -> Vec<u8> {
        let mut encoder = CanonicalEncoder::default();
        encoder.domain("EDU21-TRAINING-PROFILE-MANIFEST-V1");
        encoder.text(self.id);
        encoder.text(self.version);
        encoder.text(self.catalog_version);
        encoder.usize(self.capabilities.len());
        for capability in &self.capabilities {
            encoder.text(capability.key());
        }
        encode_limits(&self.limits, &mut encoder);
        encode_scheduling(&self.scheduling, &mut encoder);
        encode_diagnostic_policy(&self.diagnostics, &mut encoder);
        encode_restart_policy(&self.restart_retention, &mut encoder);
        encoder.usize(self.controllers.len());
        for definition in self.controllers.values() {
            encode_controller(definition, &mut encoder);
        }
        encoder.usize(self.modules.len());
        for definition in self.modules.values() {
            encode_module(definition, &mut encoder);
        }
        encoder.finish()
    }

    fn compute_manifest_hash(&self) -> Sha256Digest {
        plc_core::sha256(&self.canonical_manifest_bytes())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProfileAllowlist;

impl ProfileAllowlist {
    pub fn load(pin: &ProfilePin) -> Result<TrainingProfile, ProfileError> {
        let profile = TrainingProfile::edu21();
        if pin.id != profile.id || pin.version != profile.version {
            return Err(ProfileError::UnknownProfile);
        }
        if pin.manifest_hash != profile.manifest_hash {
            return Err(ProfileError::HashMismatch);
        }
        profile.validate()?;
        Ok(profile)
    }

    #[must_use]
    pub fn shipped_pins() -> Vec<ProfilePin> {
        vec![TrainingProfile::edu21().pin()]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileError {
    UnknownProfile,
    HashMismatch,
    CapabilityUnavailable(Capability),
    InvalidManifest(&'static str),
}

impl fmt::Display for ProfileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ProfileError {}

fn controller_catalog() -> BTreeMap<ControllerCatalogId, ControllerDefinition> {
    [
        ControllerDefinition {
            id: ControllerCatalogId::VctrlC1,
            display_name: "Compact Controller",
            local_first_expansion_slot: 1,
            local_last_slot: 8,
            requires_power_slot_zero: false,
            controller_slot: None,
            distributed_stations: 2,
            input_bytes: 1_024,
            output_bytes: 1_024,
            marker_bytes: 4_096,
            db_data_bytes: 1_048_576,
            block_capacity: 256,
            integrated_interfaces: 1,
        },
        ControllerDefinition {
            id: ControllerCatalogId::VctrlM1,
            display_name: "Modular Controller",
            local_first_expansion_slot: 2,
            local_last_slot: 15,
            requires_power_slot_zero: true,
            controller_slot: Some(1),
            distributed_stations: 4,
            input_bytes: 8_192,
            output_bytes: 8_192,
            marker_bytes: 65_536,
            db_data_bytes: 8_388_608,
            block_capacity: 4_096,
            integrated_interfaces: 2,
        },
        ControllerDefinition {
            id: ControllerCatalogId::VctrlP1,
            display_name: "Performance Controller",
            local_first_expansion_slot: 2,
            local_last_slot: 31,
            requires_power_slot_zero: true,
            controller_slot: Some(1),
            distributed_stations: 8,
            input_bytes: 32_768,
            output_bytes: 32_768,
            marker_bytes: 262_144,
            db_data_bytes: 33_554_432,
            block_capacity: 16_384,
            integrated_interfaces: 4,
        },
    ]
    .into_iter()
    .map(|definition| (definition.id, definition))
    .collect()
}

fn module_catalog() -> BTreeMap<ModuleCatalogId, ModuleDefinition> {
    [
        module(
            ModuleCatalogId::Vpwr1,
            "Virtual Power Module",
            ChannelLayout::None,
            0,
            0,
            PlacementClass::ModularPowerSlotZero,
            0,
            false,
        ),
        module(
            ModuleCatalogId::VstnH1,
            "Distributed Station Head",
            ChannelLayout::None,
            0,
            0,
            PlacementClass::StationHeadSlotZero,
            2,
            false,
        ),
        module(
            ModuleCatalogId::Vdi16,
            "Digital Input 16",
            ChannelLayout::DigitalInputs(16),
            2,
            0,
            PlacementClass::Expansion,
            0,
            false,
        ),
        module(
            ModuleCatalogId::Vdo16,
            "Digital Output 16",
            ChannelLayout::DigitalOutputs(16),
            0,
            2,
            PlacementClass::Expansion,
            0,
            false,
        ),
        module(
            ModuleCatalogId::Vai4,
            "Analog Input 4",
            ChannelLayout::AnalogInputs(4),
            8,
            0,
            PlacementClass::Expansion,
            0,
            true,
        ),
        module(
            ModuleCatalogId::Vao4,
            "Analog Output 4",
            ChannelLayout::AnalogOutputs(4),
            0,
            8,
            PlacementClass::Expansion,
            0,
            true,
        ),
        module(
            ModuleCatalogId::Vmix8,
            "Mixed Digital 8 and 8",
            ChannelLayout::MixedDigital {
                inputs: 8,
                outputs: 8,
            },
            1,
            1,
            PlacementClass::Expansion,
            0,
            false,
        ),
        module(
            ModuleCatalogId::Vrtd4,
            "Temperature Input 4",
            ChannelLayout::TemperatureInputs(4),
            8,
            0,
            PlacementClass::Expansion,
            0,
            true,
        ),
        module(
            ModuleCatalogId::Vlink2,
            "Virtual Link Module",
            ChannelLayout::None,
            0,
            0,
            PlacementClass::Expansion,
            2,
            false,
        ),
    ]
    .into_iter()
    .map(|definition| (definition.id, definition))
    .collect()
}

#[allow(clippy::too_many_arguments)]
const fn module(
    id: ModuleCatalogId,
    display_name: &'static str,
    channels: ChannelLayout,
    input_bytes: u32,
    output_bytes: u32,
    placement: PlacementClass,
    virtual_ports: u8,
    supports_wire_break: bool,
) -> ModuleDefinition {
    ModuleDefinition {
        id,
        display_name,
        channels,
        input_bytes,
        output_bytes,
        placement,
        virtual_ports,
        supports_wire_break,
    }
}

fn encode_limits(limits: &ProfileLimits, encoder: &mut CanonicalEncoder) {
    let values = [
        u64::from(limits.controllers_per_project),
        u64::from(limits.project_objects),
        u64::from(limits.tags_per_controller),
        u64::from(limits.named_types_per_project),
        u64::from(limits.source_bytes_per_block),
        u64::from(limits.type_nesting),
        u64::from(limits.members_per_type),
        u64::from(limits.array_dimensions),
        limits.array_elements,
        u64::from(limits.syntax_nesting),
        u64::from(limits.networks_per_block),
        u64::from(limits.nodes_per_network),
        u64::from(limits.edges_per_network),
        limits.dependency_edges,
        u64::from(limits.diagnostics_per_build),
        limits.constant_evaluation_operations,
        limits.semantic_work_units_per_build,
        limits.artifact_package_bytes,
        limits.package_bytes,
        limits.package_member_bytes,
        limits.expanded_package_bytes,
        u64::from(limits.package_members),
        u64::from(limits.package_nesting),
        u64::from(limits.normalized_path_bytes),
        u64::from(limits.string_field_bytes),
        u64::from(limits.expansion_ratio),
        u64::from(limits.watch_tables_per_project),
        u64::from(limits.watch_rows_per_table),
        u64::from(limits.active_subscriptions_per_controller),
        u64::from(limits.retained_samples_per_watch_row),
        u64::from(limits.trace_configurations_per_project),
        u64::from(limits.trace_channels_per_capture),
        limits.trace_samples_per_capture,
        u64::from(limits.concurrent_traces_per_controller),
        limits.trace_duration_virtual_ms,
        u64::from(limits.trace_trigger_depth),
        u64::from(limits.trace_trigger_nodes),
        u64::from(limits.active_conditions_per_controller),
        u64::from(limits.ordinary_conditions_per_controller),
        u64::from(limits.retained_diagnostic_events),
        limits.snapshot_bytes,
    ];
    encoder.usize(values.len());
    for value in values {
        encoder.u64(value);
    }
}

fn encode_scheduling(policy: &SchedulingPolicy, encoder: &mut CanonicalEncoder) {
    encoder.u32(policy.scan_quantum_ms);
    encoder.u8(policy.cyclic_main_count);
    encoder.u8(policy.startup_count);
    encoder.u8(policy.timed_cyclic_count);
    encoder.u32(policy.work_units_per_scan);
    encoder.u8(policy.call_depth);
}

fn encode_diagnostic_policy(policy: &DiagnosticPolicy, encoder: &mut CanonicalEncoder) {
    encoder.bool(policy.unreachable_scl_is_blocking);
    encoder.bool(policy.multiple_writer_is_blocking);
    encoder.bool(policy.unsafe_temp_is_blocking);
    encoder.bool(policy.missing_consumed_return_is_blocking);
}

fn encode_restart_policy(policy: &RestartRetentionPolicy, encoder: &mut CanonicalEncoder) {
    for rule in [
        &policy.stop_to_run,
        &policy.run_to_stop,
        &policy.warm_restart,
        &policy.simulated_power_cycle,
        &policy.memory_reset,
        &policy.compatible_code_load,
        &policy.schema_changing_load,
    ] {
        encoder.text(&format!("{:?}", rule.non_retentive));
        encoder.text(&format!("{:?}", rule.retentive));
        encoder.text(&format!("{:?}", rule.io));
        encoder.text(&format!("{:?}", rule.artifact));
        encoder.text(&format!("{:?}", rule.forces));
    }
}

fn encode_controller(definition: &ControllerDefinition, encoder: &mut CanonicalEncoder) {
    encoder.text(definition.id.as_str());
    encoder.text(definition.display_name);
    encoder.u8(definition.local_first_expansion_slot);
    encoder.u8(definition.local_last_slot);
    encoder.bool(definition.requires_power_slot_zero);
    encoder.option(definition.controller_slot, CanonicalEncoder::u8);
    encoder.u8(definition.distributed_stations);
    encoder.u32(definition.input_bytes);
    encoder.u32(definition.output_bytes);
    encoder.u32(definition.marker_bytes);
    encoder.u32(definition.db_data_bytes);
    encoder.u32(definition.block_capacity);
    encoder.u8(definition.integrated_interfaces);
}

fn encode_module(definition: &ModuleDefinition, encoder: &mut CanonicalEncoder) {
    encoder.text(definition.id.as_str());
    encoder.text(definition.display_name);
    encoder.text(&format!("{:?}", definition.channels));
    encoder.u32(definition.input_bytes);
    encoder.u32(definition.output_bytes);
    encoder.text(&format!("{:?}", definition.placement));
    encoder.u8(definition.virtual_ports);
    encoder.bool(definition.supports_wire_break);
}

#[cfg(test)]
mod tests {
    use plc_core::Sha256Digest;

    use super::{ProfileAllowlist, ProfileError, TrainingProfile};

    #[test]
    fn shipped_profile_is_complete_hashed_and_allowlisted_only_by_pin() {
        let first = TrainingProfile::edu21();
        let second = TrainingProfile::edu21();
        assert_eq!(first.manifest_hash(), super::EDU21_MANIFEST_HASH);
        assert_eq!(first.manifest_hash(), second.manifest_hash());
        assert_eq!(
            first.canonical_manifest_bytes(),
            second.canonical_manifest_bytes()
        );
        assert!(first.validate().is_ok());
        assert_eq!(ProfileAllowlist::load(&first.pin()), Ok(first.clone()));

        let mut bad_pin = first.pin();
        bad_pin.manifest_hash = Sha256Digest([0x55; 32]);
        assert_eq!(
            ProfileAllowlist::load(&bad_pin),
            Err(ProfileError::HashMismatch)
        );
    }

    #[test]
    fn shipped_profile_is_the_ordered_compiler_capability_authority() {
        let profile = TrainingProfile::edu21();
        assert_eq!(
            profile.compiler_capability_keys(),
            &super::EDU21_COMPILER_CAPABILITY_KEYS
        );
        assert!(
            profile
                .compiler_capability_keys()
                .windows(2)
                .all(|pair| pair[0] < pair[1])
        );
        assert!(profile.validate().is_ok());
    }
}
