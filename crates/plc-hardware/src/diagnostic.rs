use std::collections::BTreeMap;

use plc_core::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    Warning,
    Error,
    Internal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagnosticCode {
    CapabilityUnavailable,
    ResourceLimit,
    ProfileInvalid,
    IllegalPlacementOrCatalog,
    AddressConflict,
    ChannelConfiguration,
    RequiredComponentMissing,
    NetworkAddressNameOrSubnet,
    NetworkTopologyInvalid,
    UnresolvedReference,
    StaleOrDeletedTarget,
    AmbiguousReference,
    TypeMismatch,
    ConversionUnavailable,
    ConstantRangeOrArithmetic,
    BoundsOrShape,
    MalformedPlcAddress,
    PlcAddressAlignment,
    PlcAddressCapacity,
    UnmappedIoAddress,
    SymbolOverlap,
    IdentifierInvalid,
    ShadowingProhibited,
}

impl DiagnosticCode {
    #[must_use]
    pub const fn stable_code(self) -> &'static str {
        match self {
            Self::CapabilityUnavailable => "EDU-CAP-0001",
            Self::ResourceLimit => "EDU-CAP-0002",
            Self::ProfileInvalid => "EDU-INT-0001",
            Self::IllegalPlacementOrCatalog => "EDU-HW-0001",
            Self::AddressConflict => "EDU-HW-0002",
            Self::ChannelConfiguration => "EDU-HW-0003",
            Self::RequiredComponentMissing => "EDU-HW-0004",
            Self::NetworkAddressNameOrSubnet => "EDU-NET-0001",
            Self::NetworkTopologyInvalid => "EDU-NET-0002",
            Self::UnresolvedReference => "EDU-REF-0001",
            Self::StaleOrDeletedTarget => "EDU-REF-0002",
            Self::AmbiguousReference => "EDU-REF-0003",
            Self::TypeMismatch => "EDU-TYPE-0001",
            Self::ConversionUnavailable => "EDU-TYPE-0002",
            Self::ConstantRangeOrArithmetic => "EDU-TYPE-0003",
            Self::BoundsOrShape => "EDU-TYPE-0004",
            Self::MalformedPlcAddress => "EDU-SYM-0001",
            Self::PlcAddressAlignment => "EDU-SYM-0002",
            Self::PlcAddressCapacity => "EDU-SYM-0003",
            Self::UnmappedIoAddress => "EDU-SYM-0004",
            Self::SymbolOverlap => "EDU-SYM-0005",
            Self::IdentifierInvalid => "EDU-SYM-0006",
            Self::ShadowingProhibited => "EDU-SYM-0007",
        }
    }

    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::CapabilityUnavailable => "CAPABILITY_UNAVAILABLE",
            Self::ResourceLimit => "RESOURCE_LIMIT",
            Self::ProfileInvalid => "REGISTRY_OR_PROFILE_INVALID",
            Self::IllegalPlacementOrCatalog => "ILLEGAL_PLACEMENT_OR_CATALOG",
            Self::AddressConflict => "ADDRESS_CONFLICT",
            Self::ChannelConfiguration => "CHANNEL_CONFIGURATION",
            Self::RequiredComponentMissing => "REQUIRED_COMPONENT_MISSING",
            Self::NetworkAddressNameOrSubnet => "ADDRESS_NAME_OR_SUBNET",
            Self::NetworkTopologyInvalid => "TOPOLOGY_INVALID",
            Self::UnresolvedReference => "UNRESOLVED_REFERENCE",
            Self::StaleOrDeletedTarget => "STALE_OR_DELETED_TARGET",
            Self::AmbiguousReference => "AMBIGUOUS_REFERENCE",
            Self::TypeMismatch => "TYPE_MISMATCH",
            Self::ConversionUnavailable => "CONVERSION_UNAVAILABLE",
            Self::ConstantRangeOrArithmetic => "CONSTANT_RANGE_OR_ARITHMETIC",
            Self::BoundsOrShape => "BOUNDS_OR_SHAPE",
            Self::MalformedPlcAddress => "MALFORMED_PLC_ADDRESS",
            Self::PlcAddressAlignment => "PLC_ADDRESS_ALIGNMENT",
            Self::PlcAddressCapacity => "PLC_ADDRESS_CAPACITY",
            Self::UnmappedIoAddress => "UNMAPPED_IO_ADDRESS",
            Self::SymbolOverlap => "SYMBOL_ADDRESS_OVERLAP",
            Self::IdentifierInvalid => "IDENTIFIER_INVALID",
            Self::ShadowingProhibited => "SHADOWING_PROHIBITED",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TargetKind {
    Profile,
    Controller,
    Station,
    Rack,
    Slot,
    Module,
    Channel,
    Parameter,
    VirtualDevice,
    VirtualInterface,
    VirtualSubnet,
    VirtualPort,
    VirtualLink,
    Scope,
    Declaration,
    Reference,
    Tag,
    TagTable,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DiagnosticTarget {
    pub kind: TargetKind,
    pub id: Uuid,
    pub field: Option<String>,
}

impl DiagnosticTarget {
    #[must_use]
    pub fn new(kind: TargetKind, id: Uuid) -> Self {
        Self {
            kind,
            id,
            field: None,
        }
    }

    #[must_use]
    pub fn field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub severity: Severity,
    pub blocking: bool,
    pub message: String,
    pub primary: DiagnosticTarget,
    pub related: Vec<DiagnosticTarget>,
    pub parameters: BTreeMap<String, String>,
}

impl Diagnostic {
    #[must_use]
    pub fn blocking(
        code: DiagnosticCode,
        primary: DiagnosticTarget,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: if matches!(code, DiagnosticCode::ProfileInvalid) {
                Severity::Internal
            } else {
                Severity::Error
            },
            blocking: true,
            message: message.into(),
            primary,
            related: Vec::new(),
            parameters: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn related(mut self, related: impl IntoIterator<Item = DiagnosticTarget>) -> Self {
        self.related.extend(related);
        self.related.sort();
        self.related.dedup();
        self
    }

    #[must_use]
    pub fn parameter(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.parameters.insert(key.into(), value.into());
        self
    }
}
