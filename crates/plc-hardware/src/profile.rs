#![allow(clippy::missing_errors_doc)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use plc_core::Sha256Digest;

use crate::canonical::CanonicalEncoder;

pub const EDU21_PROFILE_ID: &str = "EDU-21 Core";
pub const EDU21_PROFILE_VERSION: &str = "1.0.0";
pub const EDU21_CATALOG_VERSION: &str = "1.0.0";
pub const EDU21_REQUIRED_MANIFEST_FIELD_COUNT: usize = 222;
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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ManifestScalar {
    Bool(bool),
    Unsigned(u64),
    OptionalUnsigned(Option<u64>),
    Text(String),
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

impl SchedulingPolicy {
    #[must_use]
    pub const fn edu21() -> Self {
        Self {
            scan_quantum_ms: 10,
            cyclic_main_count: 1,
            startup_count: 1,
            timed_cyclic_count: 8,
            work_units_per_scan: 100_000,
            call_depth: 64,
        }
    }
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticPolicy {
    pub unreachable_scl_is_blocking: bool,
    pub multiple_writer_is_blocking: bool,
    pub unsafe_temp_is_blocking: bool,
    pub missing_consumed_return_is_blocking: bool,
}

impl DiagnosticPolicy {
    #[must_use]
    pub const fn edu21() -> Self {
        Self {
            unreachable_scl_is_blocking: false,
            multiple_writer_is_blocking: false,
            unsafe_temp_is_blocking: true,
            missing_consumed_return_is_blocking: true,
        }
    }
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
            scheduling: SchedulingPolicy::edu21(),
            diagnostics: DiagnosticPolicy::edu21(),
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

    /// Returns the normalized, unique field inventory used by the shipped
    /// declarative manifest. This is intentionally data-only and contains no
    /// executable callbacks, endpoint names, transports, or host capability.
    pub fn manifest_fields(&self) -> Result<BTreeMap<String, ManifestScalar>, ProfileError> {
        let mut fields = BTreeMap::new();
        insert_manifest_field(
            &mut fields,
            "profile.id".to_owned(),
            ManifestScalar::Text(self.id.to_owned()),
        )?;
        insert_manifest_field(
            &mut fields,
            "profile.version".to_owned(),
            ManifestScalar::Text(self.version.to_owned()),
        )?;
        insert_manifest_field(
            &mut fields,
            "profile.catalogVersion".to_owned(),
            ManifestScalar::Text(self.catalog_version.to_owned()),
        )?;
        for capability in &self.capabilities {
            insert_manifest_field(
                &mut fields,
                format!("capability.{}", capability.key()),
                ManifestScalar::Bool(true),
            )?;
        }
        insert_limit_fields(&mut fields, &self.limits)?;
        insert_scheduling_fields(&mut fields, &self.scheduling)?;
        insert_diagnostic_fields(&mut fields, &self.diagnostics)?;
        insert_restart_fields(&mut fields, &self.restart_retention)?;
        for definition in self.controllers.values() {
            insert_controller_fields(&mut fields, definition)?;
        }
        for definition in self.modules.values() {
            insert_module_fields(&mut fields, definition)?;
        }
        Ok(fields)
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
        if self.limits != ProfileLimits::edu21() {
            return Err(ProfileError::InvalidManifest("limits"));
        }
        if self.scheduling != SchedulingPolicy::edu21() {
            return Err(ProfileError::InvalidManifest("scheduling"));
        }
        if self.diagnostics != DiagnosticPolicy::edu21() {
            return Err(ProfileError::InvalidManifest("diagnostic policy"));
        }
        if self.restart_retention != RestartRetentionPolicy::edu21() {
            return Err(ProfileError::InvalidManifest("restart/retention"));
        }
        if self.controllers != controller_catalog() {
            return Err(ProfileError::InvalidManifest("controller catalog"));
        }
        if self.modules != module_catalog() {
            return Err(ProfileError::InvalidManifest("module catalog"));
        }
        if self.manifest_fields()?.len() != EDU21_REQUIRED_MANIFEST_FIELD_COUNT {
            return Err(ProfileError::InvalidManifest("manifest field completeness"));
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

const fn value_lifecycle_token(value: ValueLifecycleAction) -> &'static str {
    match value {
        ValueLifecycleAction::Preserve => "Preserve",
        ValueLifecycleAction::ReloadLoadedStart => "ReloadLoadedStart",
        ValueLifecycleAction::RestoreRetainStore => "RestoreRetainStore",
        ValueLifecycleAction::ClearThenLoadRetainedStart => "ClearThenLoadRetainedStart",
        ValueLifecycleAction::PreserveCompatibleIdentity => "PreserveCompatibleIdentity",
        ValueLifecycleAction::PreserveDeclaredCompatibleIdentity => {
            "PreserveDeclaredCompatibleIdentity"
        }
    }
}

const fn io_lifecycle_token(value: IoLifecycleAction) -> &'static str {
    match value {
        IoLifecycleAction::FreshInputOutputDefaultsUntilCommit => {
            "FreshInputOutputDefaultsUntilCommit"
        }
        IoLifecycleAction::EffectiveOutputDefaults => "EffectiveOutputDefaults",
        IoLifecycleAction::ResetFreshInput => "ResetFreshInput",
        IoLifecycleAction::Reset => "Reset",
        IoLifecycleAction::NoMixedScan => "NoMixedScan",
        IoLifecycleAction::ResetAsPreviewed => "ResetAsPreviewed",
    }
}

const fn artifact_lifecycle_token(value: ArtifactLifecycleAction) -> &'static str {
    match value {
        ArtifactLifecycleAction::Preserve => "Preserve",
        ArtifactLifecycleAction::ReplaceAtomically => "ReplaceAtomically",
    }
}

const fn force_lifecycle_token(value: ForceLifecycleAction) -> &'static str {
    match value {
        ForceLifecycleAction::Preserve => "Preserve",
        ForceLifecycleAction::ClearThroughApprovedPreview => "ClearThroughApprovedPreview",
        ForceLifecycleAction::RegistryMustBeEmpty => "RegistryMustBeEmpty",
    }
}

fn channel_layout_token(value: ChannelLayout) -> String {
    match value {
        ChannelLayout::None => "None".to_owned(),
        ChannelLayout::DigitalInputs(count) => format!("DigitalInputs({count})"),
        ChannelLayout::DigitalOutputs(count) => format!("DigitalOutputs({count})"),
        ChannelLayout::AnalogInputs(count) => format!("AnalogInputs({count})"),
        ChannelLayout::AnalogOutputs(count) => format!("AnalogOutputs({count})"),
        ChannelLayout::MixedDigital { inputs, outputs } => {
            format!("MixedDigital {{ inputs: {inputs}, outputs: {outputs} }}")
        }
        ChannelLayout::TemperatureInputs(count) => format!("TemperatureInputs({count})"),
    }
}

const fn placement_token(value: PlacementClass) -> &'static str {
    match value {
        PlacementClass::ModularPowerSlotZero => "ModularPowerSlotZero",
        PlacementClass::StationHeadSlotZero => "StationHeadSlotZero",
        PlacementClass::Expansion => "Expansion",
    }
}

fn insert_manifest_field(
    fields: &mut BTreeMap<String, ManifestScalar>,
    path: String,
    value: ManifestScalar,
) -> Result<(), ProfileError> {
    if fields.insert(path, value).is_some() {
        Err(ProfileError::InvalidManifest("duplicate manifest field"))
    } else {
        Ok(())
    }
}

fn insert_limit_fields(
    fields: &mut BTreeMap<String, ManifestScalar>,
    limits: &ProfileLimits,
) -> Result<(), ProfileError> {
    macro_rules! field {
        ($name:literal, $value:expr) => {
            insert_manifest_field(
                fields,
                concat!("limit.", $name).to_owned(),
                ManifestScalar::Unsigned($value),
            )?;
        };
    }
    field!(
        "controllersPerProject",
        u64::from(limits.controllers_per_project)
    );
    field!("projectObjects", u64::from(limits.project_objects));
    field!("tagsPerController", u64::from(limits.tags_per_controller));
    field!(
        "namedTypesPerProject",
        u64::from(limits.named_types_per_project)
    );
    field!(
        "sourceBytesPerBlock",
        u64::from(limits.source_bytes_per_block)
    );
    field!("typeNesting", u64::from(limits.type_nesting));
    field!("membersPerType", u64::from(limits.members_per_type));
    field!("arrayDimensions", u64::from(limits.array_dimensions));
    field!("arrayElements", limits.array_elements);
    field!("syntaxNesting", u64::from(limits.syntax_nesting));
    field!("networksPerBlock", u64::from(limits.networks_per_block));
    field!("nodesPerNetwork", u64::from(limits.nodes_per_network));
    field!("edgesPerNetwork", u64::from(limits.edges_per_network));
    field!("dependencyEdges", limits.dependency_edges);
    field!(
        "diagnosticsPerBuild",
        u64::from(limits.diagnostics_per_build)
    );
    field!(
        "constantEvaluationOperations",
        limits.constant_evaluation_operations
    );
    field!(
        "semanticWorkUnitsPerBuild",
        limits.semantic_work_units_per_build
    );
    field!("artifactPackageBytes", limits.artifact_package_bytes);
    field!("packageBytes", limits.package_bytes);
    field!("packageMemberBytes", limits.package_member_bytes);
    field!("expandedPackageBytes", limits.expanded_package_bytes);
    field!("packageMembers", u64::from(limits.package_members));
    field!("packageNesting", u64::from(limits.package_nesting));
    field!(
        "normalizedPathBytes",
        u64::from(limits.normalized_path_bytes)
    );
    field!("stringFieldBytes", u64::from(limits.string_field_bytes));
    field!("expansionRatio", u64::from(limits.expansion_ratio));
    field!(
        "watchTablesPerProject",
        u64::from(limits.watch_tables_per_project)
    );
    field!("watchRowsPerTable", u64::from(limits.watch_rows_per_table));
    field!(
        "activeSubscriptionsPerController",
        u64::from(limits.active_subscriptions_per_controller)
    );
    field!(
        "retainedSamplesPerWatchRow",
        u64::from(limits.retained_samples_per_watch_row)
    );
    field!(
        "traceConfigurationsPerProject",
        u64::from(limits.trace_configurations_per_project)
    );
    field!(
        "traceChannelsPerCapture",
        u64::from(limits.trace_channels_per_capture)
    );
    field!("traceSamplesPerCapture", limits.trace_samples_per_capture);
    field!(
        "concurrentTracesPerController",
        u64::from(limits.concurrent_traces_per_controller)
    );
    field!("traceDurationVirtualMs", limits.trace_duration_virtual_ms);
    field!("traceTriggerDepth", u64::from(limits.trace_trigger_depth));
    field!("traceTriggerNodes", u64::from(limits.trace_trigger_nodes));
    field!(
        "activeConditionsPerController",
        u64::from(limits.active_conditions_per_controller)
    );
    field!(
        "ordinaryConditionsPerController",
        u64::from(limits.ordinary_conditions_per_controller)
    );
    field!(
        "retainedDiagnosticEvents",
        u64::from(limits.retained_diagnostic_events)
    );
    field!("snapshotBytes", limits.snapshot_bytes);
    Ok(())
}

fn insert_scheduling_fields(
    fields: &mut BTreeMap<String, ManifestScalar>,
    policy: &SchedulingPolicy,
) -> Result<(), ProfileError> {
    for (name, value) in [
        ("scanQuantumMs", u64::from(policy.scan_quantum_ms)),
        ("cyclicMainCount", u64::from(policy.cyclic_main_count)),
        ("startupCount", u64::from(policy.startup_count)),
        ("timedCyclicCount", u64::from(policy.timed_cyclic_count)),
        ("workUnitsPerScan", u64::from(policy.work_units_per_scan)),
        ("callDepth", u64::from(policy.call_depth)),
    ] {
        insert_manifest_field(
            fields,
            format!("scheduling.{name}"),
            ManifestScalar::Unsigned(value),
        )?;
    }
    Ok(())
}

fn insert_diagnostic_fields(
    fields: &mut BTreeMap<String, ManifestScalar>,
    policy: &DiagnosticPolicy,
) -> Result<(), ProfileError> {
    for (name, value) in [
        (
            "unreachableSclIsBlocking",
            policy.unreachable_scl_is_blocking,
        ),
        (
            "multipleWriterIsBlocking",
            policy.multiple_writer_is_blocking,
        ),
        ("unsafeTempIsBlocking", policy.unsafe_temp_is_blocking),
        (
            "missingConsumedReturnIsBlocking",
            policy.missing_consumed_return_is_blocking,
        ),
    ] {
        insert_manifest_field(
            fields,
            format!("diagnostic.{name}"),
            ManifestScalar::Bool(value),
        )?;
    }
    Ok(())
}

fn insert_restart_fields(
    fields: &mut BTreeMap<String, ManifestScalar>,
    policy: &RestartRetentionPolicy,
) -> Result<(), ProfileError> {
    for (name, rule) in [
        ("stopToRun", &policy.stop_to_run),
        ("runToStop", &policy.run_to_stop),
        ("warmRestart", &policy.warm_restart),
        ("simulatedPowerCycle", &policy.simulated_power_cycle),
        ("memoryReset", &policy.memory_reset),
        ("compatibleCodeLoad", &policy.compatible_code_load),
        ("schemaChangingLoad", &policy.schema_changing_load),
    ] {
        for (field_name, value) in [
            (
                "nonRetentive",
                value_lifecycle_token(rule.non_retentive).to_owned(),
            ),
            (
                "retentive",
                value_lifecycle_token(rule.retentive).to_owned(),
            ),
            ("io", io_lifecycle_token(rule.io).to_owned()),
            (
                "artifact",
                artifact_lifecycle_token(rule.artifact).to_owned(),
            ),
            ("forces", force_lifecycle_token(rule.forces).to_owned()),
        ] {
            insert_manifest_field(
                fields,
                format!("restart.{name}.{field_name}"),
                ManifestScalar::Text(value),
            )?;
        }
    }
    Ok(())
}

fn insert_controller_fields(
    fields: &mut BTreeMap<String, ManifestScalar>,
    definition: &ControllerDefinition,
) -> Result<(), ProfileError> {
    let prefix = format!("controller.{}", definition.id.as_str());
    insert_manifest_field(
        fields,
        format!("{prefix}.id"),
        ManifestScalar::Text(definition.id.as_str().to_owned()),
    )?;
    insert_manifest_field(
        fields,
        format!("{prefix}.displayName"),
        ManifestScalar::Text(definition.display_name.to_owned()),
    )?;
    for (name, value) in [
        (
            "localFirstExpansionSlot",
            u64::from(definition.local_first_expansion_slot),
        ),
        ("localLastSlot", u64::from(definition.local_last_slot)),
        (
            "distributedStations",
            u64::from(definition.distributed_stations),
        ),
        ("inputBytes", u64::from(definition.input_bytes)),
        ("outputBytes", u64::from(definition.output_bytes)),
        ("markerBytes", u64::from(definition.marker_bytes)),
        ("dbDataBytes", u64::from(definition.db_data_bytes)),
        ("blockCapacity", u64::from(definition.block_capacity)),
        (
            "integratedInterfaces",
            u64::from(definition.integrated_interfaces),
        ),
    ] {
        insert_manifest_field(
            fields,
            format!("{prefix}.{name}"),
            ManifestScalar::Unsigned(value),
        )?;
    }
    insert_manifest_field(
        fields,
        format!("{prefix}.controllerSlot"),
        ManifestScalar::OptionalUnsigned(definition.controller_slot.map(u64::from)),
    )?;
    insert_manifest_field(
        fields,
        format!("{prefix}.requiresPowerSlotZero"),
        ManifestScalar::Bool(definition.requires_power_slot_zero),
    )?;
    Ok(())
}

fn insert_module_fields(
    fields: &mut BTreeMap<String, ManifestScalar>,
    definition: &ModuleDefinition,
) -> Result<(), ProfileError> {
    let prefix = format!("module.{}", definition.id.as_str());
    insert_manifest_field(
        fields,
        format!("{prefix}.id"),
        ManifestScalar::Text(definition.id.as_str().to_owned()),
    )?;
    for (name, value) in [
        ("displayName", definition.display_name.to_owned()),
        ("channels", channel_layout_token(definition.channels)),
        (
            "placement",
            placement_token(definition.placement).to_owned(),
        ),
    ] {
        insert_manifest_field(
            fields,
            format!("{prefix}.{name}"),
            ManifestScalar::Text(value),
        )?;
    }
    for (name, value) in [
        ("inputBytes", u64::from(definition.input_bytes)),
        ("outputBytes", u64::from(definition.output_bytes)),
        ("virtualPorts", u64::from(definition.virtual_ports)),
    ] {
        insert_manifest_field(
            fields,
            format!("{prefix}.{name}"),
            ManifestScalar::Unsigned(value),
        )?;
    }
    insert_manifest_field(
        fields,
        format!("{prefix}.supportsWireBreak"),
        ManifestScalar::Bool(definition.supports_wire_break),
    )?;
    Ok(())
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
        encoder.text(value_lifecycle_token(rule.non_retentive));
        encoder.text(value_lifecycle_token(rule.retentive));
        encoder.text(io_lifecycle_token(rule.io));
        encoder.text(artifact_lifecycle_token(rule.artifact));
        encoder.text(force_lifecycle_token(rule.forces));
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
    encoder.text(&channel_layout_token(definition.channels));
    encoder.u32(definition.input_bytes);
    encoder.u32(definition.output_bytes);
    encoder.text(placement_token(definition.placement));
    encoder.u8(definition.virtual_ports);
    encoder.bool(definition.supports_wire_break);
}

#[cfg(test)]
mod tests {
    use plc_core::Sha256Digest;

    use super::{
        Capability, ControllerCatalogId, ForceLifecycleAction, ModuleCatalogId, ProfileAllowlist,
        ProfileError, TrainingProfile,
    };

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

    #[test]
    fn every_manifest_section_is_unique_hash_bound_and_fails_closed() {
        let baseline = TrainingProfile::edu21();
        let fields = baseline.manifest_fields().expect("field inventory");
        assert_eq!(fields.len(), super::EDU21_REQUIRED_MANIFEST_FIELD_COUNT);

        let mut candidates = Vec::new();
        let mut missing_capability = baseline.clone();
        missing_capability.capabilities.remove(&Capability::Force);
        candidates.push(missing_capability);

        let mut invalid_limit = baseline.clone();
        invalid_limit.limits.snapshot_bytes = 0;
        candidates.push(invalid_limit);

        let mut invalid_scheduling = baseline.clone();
        invalid_scheduling.scheduling.scan_quantum_ms = 0;
        candidates.push(invalid_scheduling);

        let mut invalid_diagnostics = baseline.clone();
        invalid_diagnostics.diagnostics.unsafe_temp_is_blocking = false;
        candidates.push(invalid_diagnostics);

        let mut invalid_retention = baseline.clone();
        invalid_retention.restart_retention.warm_restart.forces =
            ForceLifecycleAction::RegistryMustBeEmpty;
        candidates.push(invalid_retention);

        let mut missing_controller = baseline.clone();
        missing_controller
            .controllers
            .remove(&ControllerCatalogId::VctrlC1);
        candidates.push(missing_controller);

        let mut missing_module = baseline.clone();
        missing_module.modules.remove(&ModuleCatalogId::Vdi16);
        candidates.push(missing_module);

        for candidate in candidates {
            assert!(candidate.validate().is_err());
            assert_ne!(
                candidate.compute_manifest_hash(),
                baseline.manifest_hash(),
                "every owned section participates in the normalized manifest hash"
            );
        }
    }
}
