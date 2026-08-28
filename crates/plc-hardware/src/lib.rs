#![forbid(unsafe_code)]

//! Deterministic, capability-free EDU-21 profile, fictional hardware,
//! symbol/address, and in-memory virtual-network semantics.
//!
//! All entry points accept ordinary values and return ordinary values. This
//! crate cannot open files, enumerate devices, resolve endpoints, use sockets,
//! read host clocks, or start processes, so the same semantics are usable from
//! native tests and `wasm32`.

mod canonical;
mod condition;
mod diagnostic;
mod hardware;
mod ids;
mod network;
mod process_image;
mod profile;
mod symbols;
mod types;

pub use condition::{
    ChannelConditionProjection, ConditionLifecycle, HardwareConditionEngine,
    HardwareConditionError, HardwareConditionEvent, HardwareConditionKey,
    HardwareConditionSnapshot, HardwareDiagnosticCode, HardwareFaultAction, HardwareFaultCommand,
    HardwareFaultReceipt, NaturalChannelSample, ObservedHardwareCondition, RuntimeDeviceRole,
    RuntimeHardwareConfiguration, RuntimeModuleConfiguration,
};
pub use diagnostic::{Diagnostic, DiagnosticCode, DiagnosticTarget, Severity, TargetKind};
pub use hardware::{
    AddressArea, AddressRequest, AddressSpan, AllocationChange, AllocationPreview, ChannelAddress,
    ChannelConfig, ChannelDiagnosticCapabilities, ChannelDirection, ChannelQuality,
    ChannelRuntimeState, ConfiguredModule, ControllerConfig, HardwareArtifact,
    HardwareChannelBinding, HardwareError, HardwareProject, InstalledOccupant, ModuleRuntimeState,
    RackConfig, RackOwner, RackSlot, ScalingParameter, StationConfig,
};
pub use ids::{
    ChannelId, ControllerId, DeclarationId, ModuleId, ParameterId, RackId, ReferenceId, ScopeId,
    SlotId, SourceObjectId, StationId, TagId, TagTableId, TypeDeclarationId, VirtualDeviceId,
    VirtualInterfaceId, VirtualLinkId, VirtualPortId, VirtualSubnetId,
};
pub use network::{
    ConfiguredState, DeviceRole, DiscoveryFilter, DiscoveryResult, NetworkError, PortClass,
    PoweredState, RuntimeState, VirtualDevice, VirtualDeviceName, VirtualInterface,
    VirtualIpAddress, VirtualLink, VirtualNetwork, VirtualPort, VirtualSubnet,
};
pub use plc_core::{Sha256Digest, Uuid};
pub use process_image::{ChannelRawValue, ProcessImageError};
pub use profile::{
    ArtifactLifecycleAction, Capability, ChannelLayout, ControllerCatalogId, ControllerDefinition,
    DiagnosticPolicy, EDU21_COMPILER_CAPABILITY_KEYS, EDU21_MANIFEST_HASH, EDU21_PROFILE_ID,
    EDU21_PROFILE_VERSION, EDU21_REQUIRED_MANIFEST_FIELD_COUNT, ForceLifecycleAction,
    IoLifecycleAction, LifecycleRule, ManifestScalar, ModuleCatalogId, ModuleDefinition,
    PlacementClass, ProfileAllowlist, ProfileError, ProfileLimits, ProfilePin,
    RestartRetentionPolicy, SchedulingPolicy, TrainingProfile, ValueLifecycleAction,
};
pub use symbols::{
    Address, AddressError, AddressIntent, Binding, BindingKind, BlockValueRole,
    CrossReferenceIndex, Declaration, DeclarationKind, Identifier, IdentifierError, Namespace,
    Reference, ReferenceState, RenamePreview, Resolution, Scope, ScopeKind, SourceIdentity,
    SymbolAddressArea, SymbolError, SymbolUniverse, Tag, TagAllocationChange, TagAllocationPreview,
    TagKind, TagTable,
};
pub use types::{
    ArrayBound, CanonicalF32, CanonicalF64, CanonicalType, FiniteF64, InstructionStateKind,
    PlcValue, PrimitiveType, RetainPolicy, StructMember, TypeError,
};
