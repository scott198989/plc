use alloc::{string::String, vec::Vec};
use core::cmp::Ordering;

use plc_program::{BlockId, InterfaceMemberId};
use plc_runtime::Hash32;

use crate::{BuildAttemptId, SourceAnchor, TextRange};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DiagnosticCode(pub &'static str);

impl DiagnosticCode {
    pub const MALFORMED_TOKEN: Self = Self("EDU-SYN-0001");
    pub const MALFORMED_STRUCTURE: Self = Self("EDU-SYN-0002");
    pub const RECOGNIZED_UNSUPPORTED_SYNTAX: Self = Self("EDU-SYN-0003");
    pub const UNRESOLVED_REFERENCE: Self = Self("EDU-REF-0001");
    pub const STALE_OR_DELETED_TARGET: Self = Self("EDU-REF-0002");
    pub const AMBIGUOUS_REFERENCE: Self = Self("EDU-REF-0003");
    pub const TYPE_MISMATCH: Self = Self("EDU-TYPE-0001");
    pub const CONVERSION_UNAVAILABLE: Self = Self("EDU-TYPE-0002");
    pub const CONSTANT_RANGE_OR_ARITHMETIC: Self = Self("EDU-TYPE-0003");
    pub const BOUNDS_OR_SHAPE: Self = Self("EDU-TYPE-0004");
    pub const REQUIRED_BINDING_MISSING: Self = Self("EDU-CALL-0001");
    pub const ILLEGAL_OR_OVERLAPPING_BINDING: Self = Self("EDU-CALL-0002");
    pub const STALE_FORMAL: Self = Self("EDU-CALL-0003");
    pub const RECURSIVE_CALL_CYCLE: Self = Self("EDU-CALL-0004");
    pub const INSTANCE_INVALID: Self = Self("EDU-CALL-0005");
    pub const CAPABILITY_UNAVAILABLE: Self = Self("EDU-CAP-0001");
    pub const RESOURCE_LIMIT: Self = Self("EDU-CAP-0002");
    pub const INVALID_POWER_GRAPH: Self = Self("EDU-LAD-0001");
    pub const INVALID_DATAFLOW_GRAPH: Self = Self("EDU-FBD-0001");
    pub const INVALID_CONTROL_FLOW: Self = Self("EDU-SCL-0001");
    pub const MULTIPLE_WRITER: Self = Self("EDU-SEM-0001");
    pub const ILLEGAL_PLACEMENT_OR_CATALOG: Self = Self("EDU-HW-0001");
    pub const ADDRESS_CONFLICT: Self = Self("EDU-HW-0002");
    pub const CHANNEL_CONFIGURATION: Self = Self("EDU-HW-0003");
    pub const REQUIRED_COMPONENT_MISSING: Self = Self("EDU-HW-0004");
    pub const ADDRESS_NAME_OR_SUBNET: Self = Self("EDU-NET-0001");
    pub const TOPOLOGY_INVALID: Self = Self("EDU-NET-0002");
    pub const STALE_BUILD_RESULT: Self = Self("EDU-BLD-0001");
    pub const BUILD_RESOURCE_OR_CANCEL: Self = Self("EDU-BLD-0002");
    pub const REGISTRY_OR_PROFILE_INVALID: Self = Self("EDU-INT-0001");
    pub const IR_VERIFICATION_FAILED: Self = Self("EDU-INT-0002");
    pub const COMPILER_INVARIANT_FAILED: Self = Self("EDU-INT-0003");
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagnosticSeverity {
    Information,
    Warning,
    Error,
    Internal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagnosticPhase {
    Schema,
    Profile,
    Hardware,
    Network,
    Reference,
    Dependency,
    Syntax,
    Type,
    CallAndSchedule,
    Capability,
    Lowering,
    Verification,
    Publication,
    Internal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagnosticParameterKind {
    Text,
    Range,
    ObjectIdentity,
    MemberIdentity,
    StableIdentity,
    TypeIdentity,
    Numeric,
    Hash,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NavigationRole {
    Primary,
    Related,
    Definition,
    CallSite,
    Source,
    ProjectObject,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RecoveryCategory {
    EditSyntax,
    ResolveReference,
    CorrectType,
    CorrectBinding,
    SelectCapability,
    CorrectStructure,
    CorrectConfiguration,
    RetryBuild,
    RepairInstallation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiagnosticDefinition {
    pub code: DiagnosticCode,
    pub symbolic_name: &'static str,
    pub template: &'static str,
    pub default_severity: DiagnosticSeverity,
    pub blocking: bool,
    pub phase: DiagnosticPhase,
    pub parameters: &'static [DiagnosticParameterKind],
    pub recovery_hint: &'static str,
    pub recovery_category: RecoveryCategory,
    pub navigation_roles: &'static [NavigationRole],
}

const TEXT_RANGE: &[DiagnosticParameterKind] = &[
    DiagnosticParameterKind::Text,
    DiagnosticParameterKind::Range,
];
const TEXT_RANGE_TEXT: &[DiagnosticParameterKind] = &[
    DiagnosticParameterKind::Text,
    DiagnosticParameterKind::Range,
    DiagnosticParameterKind::Text,
];
const OBJECT_TEXT: &[DiagnosticParameterKind] = &[
    DiagnosticParameterKind::ObjectIdentity,
    DiagnosticParameterKind::Text,
];
const OBJECT_MEMBER: &[DiagnosticParameterKind] = &[
    DiagnosticParameterKind::ObjectIdentity,
    DiagnosticParameterKind::MemberIdentity,
];
const OBJECTS: &[DiagnosticParameterKind] = &[
    DiagnosticParameterKind::ObjectIdentity,
    DiagnosticParameterKind::ObjectIdentity,
];
const TYPES: &[DiagnosticParameterKind] = &[
    DiagnosticParameterKind::TypeIdentity,
    DiagnosticParameterKind::TypeIdentity,
];
const NUMBERS: &[DiagnosticParameterKind] = &[
    DiagnosticParameterKind::Numeric,
    DiagnosticParameterKind::Numeric,
    DiagnosticParameterKind::Numeric,
];
const HASHES: &[DiagnosticParameterKind] =
    &[DiagnosticParameterKind::Hash, DiagnosticParameterKind::Hash];
const STABLE_TEXT: &[DiagnosticParameterKind] = &[
    DiagnosticParameterKind::StableIdentity,
    DiagnosticParameterKind::Text,
];
const PRIMARY_RELATED: &[NavigationRole] = &[NavigationRole::Primary, NavigationRole::Related];
const SOURCE_ROLES: &[NavigationRole] = &[NavigationRole::Primary, NavigationRole::Source];
const PROJECT_ROLES: &[NavigationRole] = &[
    NavigationRole::Primary,
    NavigationRole::ProjectObject,
    NavigationRole::Related,
];

macro_rules! definition {
    ($code:expr, $name:literal, $template:literal, $severity:ident, $blocking:literal,
     $phase:ident, $parameters:expr, $hint:literal, $recovery:ident, $roles:expr) => {
        DiagnosticDefinition {
            code: $code,
            symbolic_name: $name,
            template: $template,
            default_severity: DiagnosticSeverity::$severity,
            blocking: $blocking,
            phase: DiagnosticPhase::$phase,
            parameters: $parameters,
            recovery_hint: $hint,
            recovery_category: RecoveryCategory::$recovery,
            navigation_roles: $roles,
        }
    };
}

static DEFINITIONS: [DiagnosticDefinition; 32] = [
    definition!(
        DiagnosticCode::MALFORMED_TOKEN,
        "MALFORMED_TOKEN",
        "The token cannot be interpreted in this language context.",
        Error,
        true,
        Syntax,
        TEXT_RANGE_TEXT,
        "Correct or remove the token.",
        EditSyntax,
        SOURCE_ROLES
    ),
    definition!(
        DiagnosticCode::MALFORMED_STRUCTURE,
        "MALFORMED_STRUCTURE",
        "The construct is incomplete or structurally invalid.",
        Error,
        true,
        Syntax,
        TEXT_RANGE_TEXT,
        "Complete the required structure.",
        EditSyntax,
        SOURCE_ROLES
    ),
    definition!(
        DiagnosticCode::RECOGNIZED_UNSUPPORTED_SYNTAX,
        "RECOGNIZED_UNSUPPORTED_SYNTAX",
        "The syntax is recognized but unavailable in the selected profile.",
        Error,
        true,
        Syntax,
        TEXT_RANGE,
        "Use a supported construct or select an applicable profile.",
        SelectCapability,
        SOURCE_ROLES
    ),
    definition!(
        DiagnosticCode::UNRESOLVED_REFERENCE,
        "UNRESOLVED_REFERENCE",
        "The referenced identity cannot be resolved in the declared scope.",
        Error,
        true,
        Reference,
        OBJECT_TEXT,
        "Create or select an unambiguous existing definition.",
        ResolveReference,
        PRIMARY_RELATED
    ),
    definition!(
        DiagnosticCode::STALE_OR_DELETED_TARGET,
        "STALE_OR_DELETED_TARGET",
        "The retained reference targets an unavailable identity.",
        Error,
        true,
        Reference,
        OBJECT_TEXT,
        "Restore the target or explicitly rebind the reference.",
        ResolveReference,
        PRIMARY_RELATED
    ),
    definition!(
        DiagnosticCode::AMBIGUOUS_REFERENCE,
        "AMBIGUOUS_REFERENCE",
        "The reference has more than one case-insensitive candidate.",
        Error,
        true,
        Reference,
        OBJECTS,
        "Rename or qualify the candidates so exactly one remains.",
        ResolveReference,
        PRIMARY_RELATED
    ),
    definition!(
        DiagnosticCode::TYPE_MISMATCH,
        "TYPE_MISMATCH",
        "The actual type does not satisfy the required type.",
        Error,
        true,
        Type,
        TYPES,
        "Use matching types or an explicitly supported conversion.",
        CorrectType,
        PRIMARY_RELATED
    ),
    definition!(
        DiagnosticCode::CONVERSION_UNAVAILABLE,
        "CONVERSION_UNAVAILABLE",
        "No declared conversion exists for this type pair.",
        Error,
        true,
        Type,
        TYPES,
        "Choose a registered conversion or compatible type.",
        CorrectType,
        PRIMARY_RELATED
    ),
    definition!(
        DiagnosticCode::CONSTANT_RANGE_OR_ARITHMETIC,
        "CONSTANT_RANGE_OR_ARITHMETIC",
        "The constant or compile-time operation exceeds its canonical range.",
        Error,
        true,
        Type,
        TEXT_RANGE,
        "Change the value or operation to remain representable.",
        CorrectType,
        SOURCE_ROLES
    ),
    definition!(
        DiagnosticCode::BOUNDS_OR_SHAPE,
        "BOUNDS_OR_SHAPE",
        "The index or aggregate shape is incompatible with its declaration.",
        Error,
        true,
        Type,
        TEXT_RANGE,
        "Use a valid index and exact declared shape.",
        CorrectType,
        SOURCE_ROLES
    ),
    definition!(
        DiagnosticCode::REQUIRED_BINDING_MISSING,
        "REQUIRED_BINDING_MISSING",
        "A required call formal has no binding.",
        Error,
        true,
        CallAndSchedule,
        OBJECT_MEMBER,
        "Bind the required formal explicitly.",
        CorrectBinding,
        PRIMARY_RELATED
    ),
    definition!(
        DiagnosticCode::ILLEGAL_OR_OVERLAPPING_BINDING,
        "ILLEGAL_OR_OVERLAPPING_BINDING",
        "A call binding has an illegal direction or overlapping writable storage.",
        Error,
        true,
        CallAndSchedule,
        OBJECT_MEMBER,
        "Use legal independent actual storage.",
        CorrectBinding,
        PRIMARY_RELATED
    ),
    definition!(
        DiagnosticCode::STALE_FORMAL,
        "STALE_FORMAL",
        "The call retains a formal identity no longer present on the callee.",
        Error,
        true,
        CallAndSchedule,
        OBJECT_MEMBER,
        "Review and explicitly repair the stale call binding.",
        CorrectBinding,
        PRIMARY_RELATED
    ),
    definition!(
        DiagnosticCode::RECURSIVE_CALL_CYCLE,
        "RECURSIVE_CALL_CYCLE",
        "The call graph contains a recursive cycle.",
        Error,
        true,
        Dependency,
        OBJECTS,
        "Remove the recursive call path.",
        CorrectStructure,
        PRIMARY_RELATED
    ),
    definition!(
        DiagnosticCode::INSTANCE_INVALID,
        "INSTANCE_INVALID",
        "The function-block instance does not match the called block.",
        Error,
        true,
        CallAndSchedule,
        OBJECTS,
        "Select or declare a compatible instance.",
        CorrectBinding,
        PRIMARY_RELATED
    ),
    definition!(
        DiagnosticCode::CAPABILITY_UNAVAILABLE,
        "CAPABILITY_UNAVAILABLE",
        "The requested compiler capability is unavailable in the pinned profile.",
        Error,
        true,
        Capability,
        OBJECT_TEXT,
        "Use a supported capability without approximating its meaning.",
        SelectCapability,
        PRIMARY_RELATED
    ),
    definition!(
        DiagnosticCode::RESOURCE_LIMIT,
        "RESOURCE_LIMIT",
        "A deterministic compiler resource ceiling was exceeded.",
        Error,
        true,
        Capability,
        NUMBERS,
        "Reduce the bounded input or raise the approved profile ceiling.",
        RetryBuild,
        PRIMARY_RELATED
    ),
    definition!(
        DiagnosticCode::INVALID_POWER_GRAPH,
        "INVALID_POWER_GRAPH",
        "The ladder power graph is structurally invalid.",
        Error,
        true,
        Syntax,
        OBJECT_TEXT,
        "Repair the graph structure.",
        CorrectStructure,
        PROJECT_ROLES
    ),
    definition!(
        DiagnosticCode::INVALID_DATAFLOW_GRAPH,
        "INVALID_DATAFLOW_GRAPH",
        "The function-block dataflow graph is structurally invalid.",
        Error,
        true,
        Syntax,
        OBJECT_TEXT,
        "Repair port and edge connectivity.",
        CorrectStructure,
        PROJECT_ROLES
    ),
    definition!(
        DiagnosticCode::INVALID_CONTROL_FLOW,
        "INVALID_CONTROL_FLOW",
        "The SCL control-flow rule is not satisfied.",
        Error,
        true,
        Type,
        TEXT_RANGE,
        "Repair the indicated control-flow path.",
        CorrectStructure,
        SOURCE_ROLES
    ),
    definition!(
        DiagnosticCode::MULTIPLE_WRITER,
        "MULTIPLE_WRITER",
        "More than one graphical writer targets the same storage.",
        Warning,
        false,
        Dependency,
        OBJECTS,
        "Review semantic execution order and writer intent.",
        CorrectStructure,
        PRIMARY_RELATED
    ),
    definition!(
        DiagnosticCode::ILLEGAL_PLACEMENT_OR_CATALOG,
        "ILLEGAL_PLACEMENT_OR_CATALOG",
        "The virtual hardware placement or catalog identity is invalid.",
        Error,
        true,
        Hardware,
        OBJECTS,
        "Choose a compatible fictional catalog placement.",
        CorrectConfiguration,
        PROJECT_ROLES
    ),
    definition!(
        DiagnosticCode::ADDRESS_CONFLICT,
        "ADDRESS_CONFLICT",
        "Virtual address spans overlap.",
        Error,
        true,
        Hardware,
        OBJECTS,
        "Assign non-overlapping virtual spans.",
        CorrectConfiguration,
        PROJECT_ROLES
    ),
    definition!(
        DiagnosticCode::CHANNEL_CONFIGURATION,
        "CHANNEL_CONFIGURATION",
        "A virtual channel parameter is invalid.",
        Error,
        true,
        Hardware,
        OBJECT_TEXT,
        "Correct the channel configuration.",
        CorrectConfiguration,
        PROJECT_ROLES
    ),
    definition!(
        DiagnosticCode::REQUIRED_COMPONENT_MISSING,
        "REQUIRED_COMPONENT_MISSING",
        "A required fictional component is absent.",
        Error,
        true,
        Hardware,
        OBJECTS,
        "Add the required compatible component.",
        CorrectConfiguration,
        PROJECT_ROLES
    ),
    definition!(
        DiagnosticCode::ADDRESS_NAME_OR_SUBNET,
        "ADDRESS_NAME_OR_SUBNET",
        "A virtual address, name, or subnet rule is invalid.",
        Error,
        true,
        Network,
        OBJECT_TEXT,
        "Correct the in-memory virtual network value.",
        CorrectConfiguration,
        PROJECT_ROLES
    ),
    definition!(
        DiagnosticCode::TOPOLOGY_INVALID,
        "TOPOLOGY_INVALID",
        "The virtual topology has an invalid link or role.",
        Error,
        true,
        Network,
        OBJECTS,
        "Repair the virtual topology graph.",
        CorrectConfiguration,
        PROJECT_ROLES
    ),
    definition!(
        DiagnosticCode::STALE_BUILD_RESULT,
        "STALE_BUILD_RESULT",
        "The completed result belongs to an older immutable snapshot.",
        Warning,
        false,
        Publication,
        HASHES,
        "Build the current snapshot when a current result is required.",
        RetryBuild,
        PRIMARY_RELATED
    ),
    definition!(
        DiagnosticCode::BUILD_RESOURCE_OR_CANCEL,
        "BUILD_RESOURCE_OR_CANCEL",
        "The build stopped because of cancellation or a resource ceiling.",
        Error,
        true,
        Publication,
        STABLE_TEXT,
        "Retry after addressing the stated stop reason.",
        RetryBuild,
        PRIMARY_RELATED
    ),
    definition!(
        DiagnosticCode::REGISTRY_OR_PROFILE_INVALID,
        "REGISTRY_OR_PROFILE_INVALID",
        "A trusted registry or profile invariant is invalid.",
        Internal,
        true,
        Internal,
        TEXT_RANGE,
        "Repair the trusted installation before compiling.",
        RepairInstallation,
        PRIMARY_RELATED
    ),
    definition!(
        DiagnosticCode::IR_VERIFICATION_FAILED,
        "IR_VERIFICATION_FAILED",
        "Independent typed-IR verification rejected compiler output.",
        Internal,
        true,
        Verification,
        STABLE_TEXT,
        "Preserve the report and repair the compiler invariant.",
        RepairInstallation,
        PRIMARY_RELATED
    ),
    definition!(
        DiagnosticCode::COMPILER_INVARIANT_FAILED,
        "COMPILER_INVARIANT_FAILED",
        "A compiler invariant failed before artifact publication.",
        Internal,
        true,
        Internal,
        STABLE_TEXT,
        "Preserve reproducible hashes and repair the compiler.",
        RepairInstallation,
        PRIMARY_RELATED
    ),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiagnosticRegistry {
    pub schema_version: u16,
    pub semantic_version: &'static str,
    definitions: &'static [DiagnosticDefinition],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistryError {
    EmptyVersion,
    DuplicateCode(DiagnosticCode),
    IncompleteDefinition(DiagnosticCode),
    MissingRequiredCode(DiagnosticCode),
}

impl DiagnosticRegistry {
    #[must_use]
    pub const fn definitions(self) -> &'static [DiagnosticDefinition] {
        self.definitions
    }

    #[must_use]
    pub fn lookup(self, code: DiagnosticCode) -> Option<&'static DiagnosticDefinition> {
        self.definitions
            .iter()
            .find(|definition| definition.code == code)
    }

    /// Validates trusted registry completeness before compiler initialization.
    ///
    /// # Errors
    ///
    /// Returns the first deterministic schema, duplicate, incomplete, or
    /// missing-required-code defect.
    pub fn validate(self) -> Result<(), RegistryError> {
        if self.semantic_version.is_empty() {
            return Err(RegistryError::EmptyVersion);
        }
        for (index, definition) in self.definitions.iter().enumerate() {
            if definition.symbolic_name.is_empty()
                || definition.template.is_empty()
                || definition.parameters.is_empty()
                || definition.recovery_hint.is_empty()
                || definition.navigation_roles.is_empty()
            {
                return Err(RegistryError::IncompleteDefinition(definition.code));
            }
            if self.definitions[..index]
                .iter()
                .any(|prior| prior.code == definition.code)
            {
                return Err(RegistryError::DuplicateCode(definition.code));
            }
        }
        for required in REQUIRED_CODES {
            if self.lookup(required).is_none() {
                return Err(RegistryError::MissingRequiredCode(required));
            }
        }
        Ok(())
    }
}

const REQUIRED_CODES: [DiagnosticCode; 32] = [
    DiagnosticCode::MALFORMED_TOKEN,
    DiagnosticCode::MALFORMED_STRUCTURE,
    DiagnosticCode::RECOGNIZED_UNSUPPORTED_SYNTAX,
    DiagnosticCode::UNRESOLVED_REFERENCE,
    DiagnosticCode::STALE_OR_DELETED_TARGET,
    DiagnosticCode::AMBIGUOUS_REFERENCE,
    DiagnosticCode::TYPE_MISMATCH,
    DiagnosticCode::CONVERSION_UNAVAILABLE,
    DiagnosticCode::CONSTANT_RANGE_OR_ARITHMETIC,
    DiagnosticCode::BOUNDS_OR_SHAPE,
    DiagnosticCode::REQUIRED_BINDING_MISSING,
    DiagnosticCode::ILLEGAL_OR_OVERLAPPING_BINDING,
    DiagnosticCode::STALE_FORMAL,
    DiagnosticCode::RECURSIVE_CALL_CYCLE,
    DiagnosticCode::INSTANCE_INVALID,
    DiagnosticCode::CAPABILITY_UNAVAILABLE,
    DiagnosticCode::RESOURCE_LIMIT,
    DiagnosticCode::INVALID_POWER_GRAPH,
    DiagnosticCode::INVALID_DATAFLOW_GRAPH,
    DiagnosticCode::INVALID_CONTROL_FLOW,
    DiagnosticCode::MULTIPLE_WRITER,
    DiagnosticCode::ILLEGAL_PLACEMENT_OR_CATALOG,
    DiagnosticCode::ADDRESS_CONFLICT,
    DiagnosticCode::CHANNEL_CONFIGURATION,
    DiagnosticCode::REQUIRED_COMPONENT_MISSING,
    DiagnosticCode::ADDRESS_NAME_OR_SUBNET,
    DiagnosticCode::TOPOLOGY_INVALID,
    DiagnosticCode::STALE_BUILD_RESULT,
    DiagnosticCode::BUILD_RESOURCE_OR_CANCEL,
    DiagnosticCode::REGISTRY_OR_PROFILE_INVALID,
    DiagnosticCode::IR_VERIFICATION_FAILED,
    DiagnosticCode::COMPILER_INVARIANT_FAILED,
];

#[must_use]
pub const fn phase2_diagnostic_registry() -> DiagnosticRegistry {
    DiagnosticRegistry {
        schema_version: 1,
        semantic_version: "EDU-BUILD-DIAGNOSTICS-1",
        definitions: &DEFINITIONS,
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticParameter {
    Text(String),
    Range(TextRange),
    Object(BlockId),
    Member(InterfaceMemberId),
    StableIdentity(u128),
    Type(String),
    Numeric(u64),
    Hash(Hash32),
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticTarget {
    Project,
    Object(BlockId),
    Member {
        owner: BlockId,
        member: InterfaceMemberId,
    },
    Source(SourceAnchor),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildDiagnostic {
    attempt_id: BuildAttemptId,
    snapshot_hash: Hash32,
    code: DiagnosticCode,
    severity: DiagnosticSeverity,
    blocking: bool,
    primary: DiagnosticTarget,
    related: Vec<DiagnosticTarget>,
    parameters: Vec<DiagnosticParameter>,
    cause: String,
    recovery_hint: &'static str,
}

impl BuildDiagnostic {
    pub(crate) fn new(
        attempt_id: BuildAttemptId,
        snapshot_hash: Hash32,
        code: DiagnosticCode,
        primary: DiagnosticTarget,
        related: Vec<DiagnosticTarget>,
        parameters: Vec<DiagnosticParameter>,
        cause: impl Into<String>,
    ) -> Self {
        let definition = phase2_diagnostic_registry()
            .lookup(code)
            .expect("trusted diagnostic code must exist");
        Self {
            attempt_id,
            snapshot_hash,
            code,
            severity: definition.default_severity,
            blocking: definition.blocking,
            primary,
            related,
            parameters,
            cause: cause.into(),
            recovery_hint: definition.recovery_hint,
        }
    }

    #[must_use]
    pub const fn attempt_id(&self) -> BuildAttemptId {
        self.attempt_id
    }

    #[must_use]
    pub const fn snapshot_hash(&self) -> Hash32 {
        self.snapshot_hash
    }

    #[must_use]
    pub const fn code(&self) -> DiagnosticCode {
        self.code
    }

    #[must_use]
    pub const fn severity(&self) -> DiagnosticSeverity {
        self.severity
    }

    #[must_use]
    pub const fn is_blocking(&self) -> bool {
        self.blocking
    }

    #[must_use]
    pub const fn primary(&self) -> &DiagnosticTarget {
        &self.primary
    }

    #[must_use]
    pub fn related(&self) -> &[DiagnosticTarget] {
        &self.related
    }

    #[must_use]
    pub fn parameters(&self) -> &[DiagnosticParameter] {
        &self.parameters
    }

    #[must_use]
    pub fn cause(&self) -> &str {
        &self.cause
    }

    #[must_use]
    pub const fn recovery_hint(&self) -> &'static str {
        self.recovery_hint
    }

    pub(crate) fn ordering(&self, other: &Self) -> Ordering {
        let registry = phase2_diagnostic_registry();
        let left_phase = registry.lookup(self.code).map(|value| value.phase);
        let right_phase = registry.lookup(other.code).map(|value| value.phase);
        (
            left_phase,
            target_order(&self.primary),
            self.code,
            &self.related,
            &self.parameters,
            &self.cause,
        )
            .cmp(&(
                right_phase,
                target_order(&other.primary),
                other.code,
                &other.related,
                &other.parameters,
                &other.cause,
            ))
    }
}

fn target_order(target: &DiagnosticTarget) -> (u128, u32, u128) {
    match target {
        DiagnosticTarget::Project => (0, 0, 0),
        DiagnosticTarget::Object(block) => (block.get(), 0, 0),
        DiagnosticTarget::Member { owner, member } => (owner.get(), 0, member.get()),
        DiagnosticTarget::Source(anchor) => (
            anchor.owner_object_id.get(),
            anchor.text_range.map_or(0, |range| range.start),
            anchor
                .node_id
                .unwrap_or_else(|| u128::from(anchor.semantic_node_id.get())),
        ),
    }
}
