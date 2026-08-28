#![allow(clippy::missing_errors_doc, clippy::too_many_lines)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use plc_core::{Sha256Digest, Uuid};

use crate::canonical::CanonicalEncoder;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticTarget, TargetKind};
use crate::hardware::{
    AddressArea, ChannelAddress, ChannelDirection, HardwareArtifact, HardwareProject,
};
use crate::ids::{
    ChannelId, ControllerId, DeclarationId, ReferenceId, ScopeId, SourceObjectId, TagId, TagTableId,
};
use crate::profile::{ProfileAllowlist, ProfileError, ProfilePin, TrainingProfile};
use crate::types::{CanonicalType, PlcValue, PrimitiveType, RetainPolicy, TypeError};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Identifier(String);

impl Identifier {
    pub fn parse(input: &str) -> Result<Self, IdentifierError> {
        let bytes = input.as_bytes();
        if bytes.is_empty() {
            return Err(IdentifierError::Empty);
        }
        if bytes.len() > 128 {
            return Err(IdentifierError::TooLong);
        }
        if !input.is_ascii() {
            return Err(IdentifierError::NonAsciiUnsupported);
        }
        if !bytes[0].is_ascii_alphabetic() && bytes[0] != b'_' {
            return Err(IdentifierError::InvalidGrammar);
        }
        if !bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        {
            return Err(IdentifierError::InvalidGrammar);
        }
        if is_reserved_keyword(input) {
            return Err(IdentifierError::ReservedKeyword);
        }
        Ok(Self(input.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn folded(&self) -> String {
        self.0.to_ascii_lowercase()
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentifierError {
    Empty,
    TooLong,
    NonAsciiUnsupported,
    InvalidGrammar,
    ReservedKeyword,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Namespace {
    Value,
    Type,
    Callable,
    Member,
    Label,
    Instruction,
    HardwareChannel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BlockValueRole {
    Input,
    Output,
    InOut,
    Static,
    Temp,
    Constant,
    Return,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScopeKind {
    ControllerGlobal(ControllerId),
    Block {
        controller_id: ControllerId,
        block_id: DeclarationId,
    },
    AggregateMember(DeclarationId),
    InstructionRegistry,
    HardwareCatalog,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scope {
    pub id: ScopeId,
    pub creation_ordinal: u64,
    pub kind: ScopeKind,
    pub parent_scope_id: Option<ScopeId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeclarationKind {
    GlobalTag,
    GlobalConstant,
    GlobalDb,
    InstanceDb,
    BlockValue(BlockValueRole),
    CallableBlock,
    NamedType,
    Member,
    Label,
    Instruction,
    HardwareChannel,
    ErrorSymbol,
}

impl DeclarationKind {
    #[must_use]
    pub const fn expected_namespace(self) -> Namespace {
        match self {
            Self::GlobalTag
            | Self::GlobalConstant
            | Self::GlobalDb
            | Self::InstanceDb
            | Self::BlockValue(_)
            | Self::ErrorSymbol => Namespace::Value,
            Self::CallableBlock => Namespace::Callable,
            Self::NamedType => Namespace::Type,
            Self::Member => Namespace::Member,
            Self::Label => Namespace::Label,
            Self::Instruction => Namespace::Instruction,
            Self::HardwareChannel => Namespace::HardwareChannel,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Declaration {
    pub id: DeclarationId,
    pub creation_ordinal: u64,
    pub name: Identifier,
    pub scope_id: ScopeId,
    pub namespace: Namespace,
    pub kind: DeclarationKind,
    pub member_scope_id: Option<ScopeId>,
    pub deleted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceIdentity {
    pub object_id: SourceObjectId,
    pub location: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BindingKind {
    Declaration,
    Read,
    Write,
    ReadWrite,
    Call,
    Instantiate,
    TypeUse,
    AddressBind,
    HardwareBind,
    HmiBindReserved,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    pub target_id: DeclarationId,
    pub target_kind: DeclarationKind,
    pub owning_scope_id: ScopeId,
    pub source: SourceIdentity,
    pub binding_kind: BindingKind,
    pub target_path: Vec<DeclarationId>,
    pub display_path: Vec<Identifier>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReferenceState {
    Resolved(Binding),
    NeverResolved {
        authored_spelling: String,
        qualification_path: Vec<Identifier>,
        expected_namespace: Namespace,
        scope_id: ScopeId,
        source: SourceIdentity,
        binding_kind: BindingKind,
    },
    StaleDeleted(Binding),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reference {
    pub id: ReferenceId,
    pub creation_ordinal: u64,
    pub state: ReferenceState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolution {
    Resolved(Binding),
    Unresolved,
    Ambiguous(Vec<DeclarationId>),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CrossReferenceIndex {
    by_target: BTreeMap<DeclarationId, BTreeSet<ReferenceId>>,
}

impl CrossReferenceIndex {
    #[must_use]
    pub fn references_to(&self, target_id: DeclarationId) -> Vec<ReferenceId> {
        self.by_target
            .get(&target_id)
            .map_or_else(Vec::new, |values| values.iter().copied().collect())
    }

    fn rebuild(references: &BTreeMap<ReferenceId, Reference>) -> Self {
        let mut by_target: BTreeMap<DeclarationId, BTreeSet<ReferenceId>> = BTreeMap::new();
        for reference in references.values() {
            let binding = match &reference.state {
                ReferenceState::Resolved(binding) | ReferenceState::StaleDeleted(binding) => {
                    binding
                }
                ReferenceState::NeverResolved { .. } => continue,
            };
            for target in &binding.target_path {
                by_target.entry(*target).or_default().insert(reference.id);
            }
        }
        Self { by_target }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagTable {
    pub id: TagTableId,
    pub controller_id: ControllerId,
    pub creation_ordinal: u64,
    pub name: Identifier,
    pub is_default: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TagKind {
    Variable,
    Constant(PlcValue),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SymbolAddressArea {
    Input,
    Output,
    Marker,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AddressIntent {
    None,
    Auto(SymbolAddressArea),
    Explicit {
        authored_text: String,
        parsed: Option<Address>,
    },
}

impl AddressIntent {
    #[must_use]
    pub fn explicit(authored_text: impl Into<String>) -> Self {
        let authored_text = authored_text.into();
        let parsed = Address::parse(&authored_text).ok();
        Self::Explicit {
            authored_text,
            parsed,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Address {
    InputBit { byte: u32, bit: u8 },
    OutputBit { byte: u32, bit: u8 },
    InputWord { byte: u32 },
    OutputWord { byte: u32 },
    MarkerBit { byte: u32, bit: u8 },
    MarkerByte { byte: u32 },
    MarkerWord { byte: u32 },
    MarkerDword { byte: u32 },
    MarkerLword { byte: u32 },
}

impl Address {
    pub fn parse(input: &str) -> Result<Self, AddressError> {
        if input.is_empty() || input.bytes().any(|byte| byte.is_ascii_whitespace()) {
            return Err(AddressError::Malformed);
        }
        let body = input.strip_prefix('%').unwrap_or(input);
        if body.is_empty() || !body.is_ascii() {
            return Err(AddressError::Malformed);
        }
        let upper = body.to_ascii_uppercase();
        if let Some(rest) = upper.strip_prefix("IW") {
            return Ok(Self::InputWord {
                byte: parse_byte(rest)?,
            });
        }
        if let Some(rest) = upper.strip_prefix("QW") {
            return Ok(Self::OutputWord {
                byte: parse_byte(rest)?,
            });
        }
        if let Some(rest) = upper.strip_prefix("MB") {
            return Ok(Self::MarkerByte {
                byte: parse_byte(rest)?,
            });
        }
        if let Some(rest) = upper.strip_prefix("MW") {
            return Ok(Self::MarkerWord {
                byte: parse_byte(rest)?,
            });
        }
        if let Some(rest) = upper.strip_prefix("MD") {
            return Ok(Self::MarkerDword {
                byte: parse_byte(rest)?,
            });
        }
        if let Some(rest) = upper.strip_prefix("ML") {
            return Ok(Self::MarkerLword {
                byte: parse_byte(rest)?,
            });
        }
        if let Some(rest) = upper.strip_prefix('I') {
            let (byte, bit) = parse_bit(rest)?;
            return Ok(Self::InputBit { byte, bit });
        }
        if let Some(rest) = upper.strip_prefix('Q') {
            let (byte, bit) = parse_bit(rest)?;
            return Ok(Self::OutputBit { byte, bit });
        }
        if let Some(rest) = upper.strip_prefix('M') {
            let (byte, bit) = parse_bit(rest)?;
            return Ok(Self::MarkerBit { byte, bit });
        }
        Err(AddressError::Malformed)
    }

    #[must_use]
    pub const fn area(self) -> SymbolAddressArea {
        match self {
            Self::InputBit { .. } | Self::InputWord { .. } => SymbolAddressArea::Input,
            Self::OutputBit { .. } | Self::OutputWord { .. } => SymbolAddressArea::Output,
            Self::MarkerBit { .. }
            | Self::MarkerByte { .. }
            | Self::MarkerWord { .. }
            | Self::MarkerDword { .. }
            | Self::MarkerLword { .. } => SymbolAddressArea::Marker,
        }
    }

    #[must_use]
    pub const fn byte(self) -> u32 {
        match self {
            Self::InputBit { byte, .. }
            | Self::OutputBit { byte, .. }
            | Self::InputWord { byte }
            | Self::OutputWord { byte }
            | Self::MarkerBit { byte, .. }
            | Self::MarkerByte { byte }
            | Self::MarkerWord { byte }
            | Self::MarkerDword { byte }
            | Self::MarkerLword { byte } => byte,
        }
    }

    #[must_use]
    pub const fn width_bits(self) -> u8 {
        match self {
            Self::InputBit { .. } | Self::OutputBit { .. } | Self::MarkerBit { .. } => 1,
            Self::MarkerByte { .. } => 8,
            Self::InputWord { .. } | Self::OutputWord { .. } | Self::MarkerWord { .. } => 16,
            Self::MarkerDword { .. } => 32,
            Self::MarkerLword { .. } => 64,
        }
    }

    #[must_use]
    pub const fn alignment_bytes(self) -> u8 {
        match self {
            Self::InputWord { .. } | Self::OutputWord { .. } | Self::MarkerWord { .. } => 2,
            Self::MarkerDword { .. } => 4,
            Self::MarkerLword { .. } => 8,
            _ => 1,
        }
    }

    fn bit_interval(self) -> Option<(u64, u64)> {
        if self.area() != SymbolAddressArea::Marker {
            return None;
        }
        let base = u64::from(self.byte()).checked_mul(8)?;
        let start = match self {
            Self::MarkerBit { bit, .. } => base.checked_add(u64::from(bit))?,
            _ => base,
        };
        Some((start, start.checked_add(u64::from(self.width_bits()))?))
    }
}

impl fmt::Display for Address {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputBit { byte, bit } => write!(formatter, "I{byte}.{bit}"),
            Self::OutputBit { byte, bit } => write!(formatter, "Q{byte}.{bit}"),
            Self::InputWord { byte } => write!(formatter, "IW{byte}"),
            Self::OutputWord { byte } => write!(formatter, "QW{byte}"),
            Self::MarkerBit { byte, bit } => write!(formatter, "M{byte}.{bit}"),
            Self::MarkerByte { byte } => write!(formatter, "MB{byte}"),
            Self::MarkerWord { byte } => write!(formatter, "MW{byte}"),
            Self::MarkerDword { byte } => write!(formatter, "MD{byte}"),
            Self::MarkerLword { byte } => write!(formatter, "ML{byte}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddressError {
    Malformed,
    BitOutOfRange,
    ByteOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tag {
    pub id: TagId,
    pub declaration_id: DeclarationId,
    pub controller_id: ControllerId,
    pub creation_ordinal: u64,
    pub table_id: TagTableId,
    pub name: Identifier,
    pub declared_type: CanonicalType,
    pub address_intent: AddressIntent,
    pub allocated_address: Option<Address>,
    pub comment: String,
    pub start_value: Option<PlcValue>,
    pub retain_policy: RetainPolicy,
    pub display_format: String,
    pub kind: TagKind,
    pub hardware_channel_id: Option<ChannelId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagAllocationChange {
    pub tag_id: TagId,
    pub previous_address: Option<Address>,
    pub proposed_address: Address,
    pub previous_channel_id: Option<ChannelId>,
    pub proposed_channel_id: Option<ChannelId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagAllocationPreview {
    pub expected_semantic_fingerprint: Sha256Digest,
    pub changes: Vec<TagAllocationChange>,
    pub proposed_semantic_fingerprint: Sha256Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenamePreview {
    pub expected_revision: u64,
    pub declaration_id: DeclarationId,
    pub new_name: Identifier,
    pub affected_reference_ids: Vec<ReferenceId>,
    pub expected_semantic_fingerprint: Sha256Digest,
    pub proposed_semantic_fingerprint: Sha256Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymbolUniverse {
    pub profile_pin: ProfilePin,
    revision: u64,
    scopes: BTreeMap<ScopeId, Scope>,
    declarations: BTreeMap<DeclarationId, Declaration>,
    references: BTreeMap<ReferenceId, Reference>,
    tag_tables: BTreeMap<TagTableId, TagTable>,
    tags: BTreeMap<TagId, Tag>,
    cross_references: CrossReferenceIndex,
}

impl SymbolUniverse {
    #[must_use]
    pub fn new(profile_pin: ProfilePin) -> Self {
        Self {
            profile_pin,
            revision: 0,
            scopes: BTreeMap::new(),
            declarations: BTreeMap::new(),
            references: BTreeMap::new(),
            tag_tables: BTreeMap::new(),
            tags: BTreeMap::new(),
            cross_references: CrossReferenceIndex::default(),
        }
    }

    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub const fn scopes(&self) -> &BTreeMap<ScopeId, Scope> {
        &self.scopes
    }

    #[must_use]
    pub const fn declarations(&self) -> &BTreeMap<DeclarationId, Declaration> {
        &self.declarations
    }

    #[must_use]
    pub const fn references(&self) -> &BTreeMap<ReferenceId, Reference> {
        &self.references
    }

    #[must_use]
    pub const fn tag_tables(&self) -> &BTreeMap<TagTableId, TagTable> {
        &self.tag_tables
    }

    #[must_use]
    pub const fn tags(&self) -> &BTreeMap<TagId, Tag> {
        &self.tags
    }

    #[must_use]
    pub const fn cross_reference_index(&self) -> &CrossReferenceIndex {
        &self.cross_references
    }

    #[must_use]
    pub fn validate_references(&self) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for reference in self.references.values() {
            match &reference.state {
                ReferenceState::Resolved(_) => {}
                ReferenceState::StaleDeleted(binding) => {
                    diagnostics.push(
                        Diagnostic::blocking(
                            DiagnosticCode::StaleOrDeletedTarget,
                            DiagnosticTarget::new(TargetKind::Reference, reference.id.uuid()),
                            "Reference retains its UUID binding, but the bound declaration is deleted",
                        )
                        .related(binding.target_path.iter().map(|id| {
                            DiagnosticTarget::new(TargetKind::Declaration, id.uuid())
                        })),
                    );
                }
                ReferenceState::NeverResolved {
                    authored_spelling,
                    qualification_path,
                    expected_namespace,
                    scope_id,
                    source,
                    binding_kind,
                } => {
                    let resolution = self.resolve_path(
                        qualification_path,
                        *expected_namespace,
                        *scope_id,
                        source.clone(),
                        *binding_kind,
                    );
                    let (code, related) = match resolution {
                        Ok(Resolution::Ambiguous(candidates)) => (
                            DiagnosticCode::AmbiguousReference,
                            candidates
                                .into_iter()
                                .map(|id| DiagnosticTarget::new(TargetKind::Declaration, id.uuid()))
                                .collect(),
                        ),
                        _ => (DiagnosticCode::UnresolvedReference, Vec::new()),
                    };
                    diagnostics.push(
                        Diagnostic::blocking(
                            code,
                            DiagnosticTarget::new(TargetKind::Reference, reference.id.uuid()),
                            "Textual reference has never acquired a stable declaration UUID binding",
                        )
                        .related(related)
                        .parameter("authoredSpelling", authored_spelling.clone())
                        .parameter("sourceLocation", source.location.clone()),
                    );
                }
            }
        }
        diagnostics.sort_by(|left, right| {
            (left.code.stable_code(), left.primary.id, &left.message).cmp(&(
                right.code.stable_code(),
                right.primary.id,
                &right.message,
            ))
        });
        diagnostics
    }

    pub fn add_scope(&mut self, scope: Scope) -> Result<(), SymbolError> {
        if !scope.id.uuid().is_rfc9562_v4() {
            return Err(SymbolError::InvalidIdentity(scope.id.uuid()));
        }
        if self.scopes.contains_key(&scope.id) {
            return Err(SymbolError::DuplicateIdentity(scope.id.uuid()));
        }
        if let Some(parent) = scope.parent_scope_id
            && !self.scopes.contains_key(&parent)
        {
            return Err(SymbolError::UnknownScope(parent));
        }
        self.scopes.insert(scope.id, scope);
        self.bump_revision()?;
        Ok(())
    }

    pub fn add_declaration(&mut self, declaration: Declaration) -> Result<(), SymbolError> {
        let mut candidate = self.clone();
        candidate.validate_new_declaration(&declaration, None)?;
        candidate.declarations.insert(declaration.id, declaration);
        candidate.bump_revision()?;
        *self = candidate;
        Ok(())
    }

    pub fn add_tag_table(&mut self, table: TagTable) -> Result<(), SymbolError> {
        if self.tag_tables.contains_key(&table.id) {
            return Err(SymbolError::DuplicateIdentity(table.id.uuid()));
        }
        if !self
            .scopes
            .values()
            .any(|scope| scope.kind == ScopeKind::ControllerGlobal(table.controller_id))
        {
            return Err(SymbolError::UnknownControllerScope(table.controller_id));
        }
        if self.tag_tables.values().any(|existing| {
            existing.controller_id == table.controller_id
                && existing.name.folded() == table.name.folded()
        }) {
            return Err(SymbolError::DuplicateName);
        }
        if table.is_default
            && self.tag_tables.values().any(|existing| {
                existing.controller_id == table.controller_id && existing.is_default
            })
        {
            return Err(SymbolError::DuplicateDefaultTagTable);
        }
        self.tag_tables.insert(table.id, table);
        self.bump_revision()?;
        Ok(())
    }

    pub fn add_tag(&mut self, tag: Tag) -> Result<(), SymbolError> {
        if self.tags.contains_key(&tag.id) {
            return Err(SymbolError::DuplicateIdentity(tag.id.uuid()));
        }
        let profile = ProfileAllowlist::load(&self.profile_pin)?;
        let current_count = self
            .tags
            .values()
            .filter(|existing| existing.controller_id == tag.controller_id)
            .count();
        if current_count
            >= usize::try_from(profile.limits().tags_per_controller).unwrap_or(usize::MAX)
        {
            return Err(SymbolError::TagCapacity {
                controller_id: tag.controller_id,
                maximum: profile.limits().tags_per_controller,
            });
        }
        let table = self
            .tag_tables
            .get(&tag.table_id)
            .ok_or(SymbolError::UnknownTagTable(tag.table_id))?;
        if table.controller_id != tag.controller_id {
            return Err(SymbolError::TagTableControllerMismatch);
        }
        let declaration = self
            .declarations
            .get(&tag.declaration_id)
            .ok_or(SymbolError::UnknownDeclaration(tag.declaration_id))?;
        if declaration.name != tag.name
            || declaration.namespace != Namespace::Value
            || declaration.deleted
        {
            return Err(SymbolError::TagDeclarationMismatch);
        }
        tag.declared_type.validate(
            profile.limits().type_nesting,
            profile.limits().members_per_type,
            profile.limits().array_dimensions,
            profile.limits().array_elements,
        )?;
        self.tags.insert(tag.id, tag);
        self.bump_revision()?;
        Ok(())
    }

    pub fn move_tag_to_table(
        &mut self,
        tag_id: TagId,
        target_table_id: TagTableId,
    ) -> Result<bool, SymbolError> {
        let tag = self
            .tags
            .get(&tag_id)
            .ok_or(SymbolError::UnknownTag(tag_id))?;
        let target = self
            .tag_tables
            .get(&target_table_id)
            .ok_or(SymbolError::UnknownTagTable(target_table_id))?;
        if target.controller_id != tag.controller_id {
            return Err(SymbolError::TagTableControllerMismatch);
        }
        if tag.table_id == target_table_id {
            return Ok(false);
        }
        let mut candidate = self.clone();
        candidate
            .tags
            .get_mut(&tag_id)
            .ok_or(SymbolError::UnknownTag(tag_id))?
            .table_id = target_table_id;
        candidate.bump_revision()?;
        *self = candidate;
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_reference(
        &mut self,
        id: ReferenceId,
        creation_ordinal: u64,
        authored_path: &[&str],
        expected_namespace: Namespace,
        scope_id: ScopeId,
        source: SourceIdentity,
        binding_kind: BindingKind,
    ) -> Result<Resolution, SymbolError> {
        if binding_kind == BindingKind::HmiBindReserved {
            return Err(SymbolError::ReservedHmiBinding);
        }
        if self.references.contains_key(&id) {
            return Err(SymbolError::DuplicateIdentity(id.uuid()));
        }
        let qualification_path = authored_path
            .iter()
            .map(|segment| Identifier::parse(segment))
            .collect::<Result<Vec<_>, _>>()?;
        if qualification_path.is_empty() {
            return Err(SymbolError::EmptyReferencePath);
        }
        let resolution = self.resolve_path(
            &qualification_path,
            expected_namespace,
            scope_id,
            source.clone(),
            binding_kind,
        )?;
        let state = match &resolution {
            Resolution::Resolved(binding) => ReferenceState::Resolved(binding.clone()),
            Resolution::Unresolved | Resolution::Ambiguous(_) => ReferenceState::NeverResolved {
                authored_spelling: authored_path.join("."),
                qualification_path,
                expected_namespace,
                scope_id,
                source,
                binding_kind,
            },
        };
        self.references.insert(
            id,
            Reference {
                id,
                creation_ordinal,
                state,
            },
        );
        self.rebuild_cross_references();
        self.bump_revision()?;
        Ok(resolution)
    }

    pub fn rebind_reference(&mut self, id: ReferenceId) -> Result<Resolution, SymbolError> {
        let reference = self
            .references
            .get(&id)
            .ok_or(SymbolError::UnknownReference(id))?
            .clone();
        let (path, expected_namespace, scope_id, source, binding_kind) = match reference.state {
            ReferenceState::NeverResolved {
                qualification_path,
                expected_namespace,
                scope_id,
                source,
                binding_kind,
                ..
            } => (
                qualification_path,
                expected_namespace,
                scope_id,
                source,
                binding_kind,
            ),
            ReferenceState::StaleDeleted(binding) => (
                binding.display_path,
                binding.target_kind.expected_namespace(),
                binding.owning_scope_id,
                binding.source,
                binding.binding_kind,
            ),
            ReferenceState::Resolved(binding) => return Ok(Resolution::Resolved(binding)),
        };
        let resolution = self.resolve_path(
            &path,
            expected_namespace,
            scope_id,
            source.clone(),
            binding_kind,
        )?;
        let state = match &resolution {
            Resolution::Resolved(binding) => ReferenceState::Resolved(binding.clone()),
            Resolution::Unresolved | Resolution::Ambiguous(_) => ReferenceState::NeverResolved {
                authored_spelling: path
                    .iter()
                    .map(Identifier::as_str)
                    .collect::<Vec<_>>()
                    .join("."),
                qualification_path: path,
                expected_namespace,
                scope_id,
                source,
                binding_kind,
            },
        };
        self.references
            .get_mut(&id)
            .ok_or(SymbolError::UnknownReference(id))?
            .state = state;
        self.rebuild_cross_references();
        self.bump_revision()?;
        Ok(resolution)
    }

    pub fn delete_declaration(&mut self, id: DeclarationId) -> Result<(), SymbolError> {
        let declaration = self
            .declarations
            .get_mut(&id)
            .ok_or(SymbolError::UnknownDeclaration(id))?;
        if declaration.deleted {
            return Ok(());
        }
        declaration.deleted = true;
        for reference in self.references.values_mut() {
            let should_stale = matches!(
                &reference.state,
                ReferenceState::Resolved(binding) if binding.target_path.contains(&id)
            );
            if should_stale {
                let old = std::mem::replace(
                    &mut reference.state,
                    ReferenceState::NeverResolved {
                        authored_spelling: String::new(),
                        qualification_path: Vec::new(),
                        expected_namespace: Namespace::Value,
                        scope_id: ScopeId(Uuid::NIL),
                        source: SourceIdentity {
                            object_id: SourceObjectId(Uuid::NIL),
                            location: String::new(),
                        },
                        binding_kind: BindingKind::Read,
                    },
                );
                if let ReferenceState::Resolved(binding) = old {
                    reference.state = ReferenceState::StaleDeleted(binding);
                }
            }
        }
        self.rebuild_cross_references();
        self.bump_revision()?;
        Ok(())
    }

    pub fn restore_declaration(&mut self, id: DeclarationId) -> Result<(), SymbolError> {
        let declaration = self
            .declarations
            .get(&id)
            .ok_or(SymbolError::UnknownDeclaration(id))?
            .clone();
        if !declaration.deleted {
            return Ok(());
        }
        self.validate_new_declaration(&declaration, Some(id))?;
        let mut candidate = self.clone();
        candidate
            .declarations
            .get_mut(&id)
            .ok_or(SymbolError::UnknownDeclaration(id))?
            .deleted = false;
        for reference in candidate.references.values_mut() {
            let restorable = match &reference.state {
                ReferenceState::StaleDeleted(binding) => binding.target_path.iter().all(|target| {
                    candidate
                        .declarations
                        .get(target)
                        .is_some_and(|declaration| !declaration.deleted)
                }),
                _ => false,
            };
            if restorable {
                let old = std::mem::replace(
                    &mut reference.state,
                    ReferenceState::NeverResolved {
                        authored_spelling: String::new(),
                        qualification_path: Vec::new(),
                        expected_namespace: Namespace::Value,
                        scope_id: ScopeId(Uuid::NIL),
                        source: SourceIdentity {
                            object_id: SourceObjectId(Uuid::NIL),
                            location: String::new(),
                        },
                        binding_kind: BindingKind::Read,
                    },
                );
                if let ReferenceState::StaleDeleted(binding) = old {
                    reference.state = ReferenceState::Resolved(binding);
                }
            }
        }
        candidate.rebuild_cross_references();
        candidate.bump_revision()?;
        *self = candidate;
        Ok(())
    }

    pub fn preview_rename(
        &self,
        id: DeclarationId,
        new_name: &str,
    ) -> Result<RenamePreview, SymbolError> {
        let new_name = Identifier::parse(new_name)?;
        let declaration = self
            .declarations
            .get(&id)
            .ok_or(SymbolError::UnknownDeclaration(id))?;
        if declaration.deleted {
            return Err(SymbolError::DeletedDeclaration(id));
        }
        self.validate_new_declaration(
            &Declaration {
                name: new_name.clone(),
                ..declaration.clone()
            },
            Some(id),
        )?;
        let mut affected_reference_ids = self.cross_references.references_to(id);
        affected_reference_ids.sort();
        let mut proposed = self.clone();
        proposed.apply_rename(id, &new_name)?;
        Ok(RenamePreview {
            expected_revision: self.revision,
            declaration_id: id,
            new_name,
            affected_reference_ids,
            expected_semantic_fingerprint: self.semantic_fingerprint(),
            proposed_semantic_fingerprint: proposed.semantic_fingerprint(),
        })
    }

    pub fn commit_rename(&mut self, preview: &RenamePreview) -> Result<(), SymbolError> {
        if self.revision != preview.expected_revision
            || self.semantic_fingerprint() != preview.expected_semantic_fingerprint
        {
            return Err(SymbolError::StalePreview);
        }
        let fresh = self.preview_rename(preview.declaration_id, preview.new_name.as_str())?;
        if fresh != *preview {
            return Err(SymbolError::StalePreview);
        }
        let mut candidate = self.clone();
        candidate.apply_rename(preview.declaration_id, &preview.new_name)?;
        candidate.bump_revision()?;
        if candidate.semantic_fingerprint() != preview.proposed_semantic_fingerprint {
            return Err(SymbolError::StalePreview);
        }
        *self = candidate;
        Ok(())
    }

    #[must_use]
    pub fn validate_tags(
        &self,
        profile: &TrainingProfile,
        hardware_project: &HardwareProject,
        artifact: &HardwareArtifact,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        if self.profile_pin != profile.pin() || artifact.profile_pin != profile.pin() {
            diagnostics.push(Diagnostic::blocking(
                DiagnosticCode::ProfileInvalid,
                DiagnosticTarget::new(TargetKind::Profile, Uuid::NIL),
                "Symbol/hardware profile pins do not match EDU-21",
            ));
            return diagnostics;
        }
        let mut controller_counts = BTreeMap::new();
        let mut channels = BTreeMap::new();
        let mut marker_intervals: Vec<(ControllerId, TagId, u64, u64)> = Vec::new();
        for tag in self.tags.values() {
            *controller_counts
                .entry(tag.controller_id)
                .or_insert(0_usize) += 1;
            validate_tag_value(tag, &mut diagnostics);
            let address = match (&tag.address_intent, tag.allocated_address) {
                (AddressIntent::None, None) => None,
                (
                    AddressIntent::Explicit {
                        parsed: Some(parsed),
                        ..
                    },
                    Some(allocated),
                ) if *parsed == allocated => Some(allocated),
                (
                    AddressIntent::Explicit {
                        parsed: None,
                        authored_text,
                    },
                    _,
                ) => {
                    diagnostics.push(
                        Diagnostic::blocking(
                            DiagnosticCode::MalformedPlcAddress,
                            DiagnosticTarget::new(TargetKind::Tag, tag.id.uuid()).field("address"),
                            "PLC address text is malformed and remains unallocated for repair",
                        )
                        .parameter("authoredText", authored_text.clone()),
                    );
                    None
                }
                (AddressIntent::Auto(_), Some(allocated)) => Some(allocated),
                (AddressIntent::Auto(_), None) => {
                    diagnostics.push(Diagnostic::blocking(
                        DiagnosticCode::UnmappedIoAddress,
                        DiagnosticTarget::new(TargetKind::Tag, tag.id.uuid()).field("address"),
                        "Automatic tag address is unresolved; commit the allocation preview",
                    ));
                    None
                }
                _ => {
                    diagnostics.push(Diagnostic::blocking(
                        DiagnosticCode::MalformedPlcAddress,
                        DiagnosticTarget::new(TargetKind::Tag, tag.id.uuid()).field("address"),
                        "Address intent and allocated canonical address disagree",
                    ));
                    None
                }
            };
            if matches!(tag.kind, TagKind::Constant(_)) {
                if address.is_some() || tag.hardware_channel_id.is_some() {
                    diagnostics.push(Diagnostic::blocking(
                        DiagnosticCode::TypeMismatch,
                        DiagnosticTarget::new(TargetKind::Tag, tag.id.uuid()),
                        "A constant has no mutable storage, PLC address, or hardware binding",
                    ));
                }
                continue;
            }
            if tag.retain_policy == RetainPolicy::Retentive && address.is_none() {
                diagnostics.push(Diagnostic::blocking(
                    DiagnosticCode::TypeMismatch,
                    DiagnosticTarget::new(TargetKind::Tag, tag.id.uuid()).field("retainPolicy"),
                    "EDU-21 permits retained global tags only when they bind M storage",
                ));
            }
            let Some(address) = address else {
                continue;
            };
            if let AddressIntent::Auto(requested_area) = tag.address_intent
                && requested_area != address.area()
            {
                diagnostics.push(Diagnostic::blocking(
                    DiagnosticCode::MalformedPlcAddress,
                    DiagnosticTarget::new(TargetKind::Tag, tag.id.uuid()).field("address"),
                    "Automatically allocated address is in a different conceptual area than requested",
                ));
            }
            validate_address_type(tag, address, &mut diagnostics);
            if address.area() == SymbolAddressArea::Marker && tag.hardware_channel_id.is_some() {
                diagnostics.push(Diagnostic::blocking(
                    DiagnosticCode::TypeMismatch,
                    DiagnosticTarget::new(TargetKind::Tag, tag.id.uuid())
                        .field("hardwareChannelId"),
                    "M storage is controller-owned marker memory and cannot bind a hardware channel",
                ));
            }
            if tag.retain_policy == RetainPolicy::Retentive
                && address.area() != SymbolAddressArea::Marker
            {
                diagnostics.push(Diagnostic::blocking(
                    DiagnosticCode::TypeMismatch,
                    DiagnosticTarget::new(TargetKind::Tag, tag.id.uuid()).field("retainPolicy"),
                    "EDU-21 permits retention on addressed global tags only in M storage",
                ));
            }
            if address.byte() % u32::from(address.alignment_bytes()) != 0 {
                diagnostics.push(Diagnostic::blocking(
                    DiagnosticCode::PlcAddressAlignment,
                    DiagnosticTarget::new(TargetKind::Tag, tag.id.uuid()).field("address"),
                    "PLC address violates the baseline natural-alignment rule",
                ));
            }
            match address.area() {
                SymbolAddressArea::Input | SymbolAddressArea::Output => {
                    validate_io_binding(tag, address, artifact, &mut channels, &mut diagnostics);
                }
                SymbolAddressArea::Marker => {
                    validate_marker_capacity(
                        tag,
                        address,
                        profile,
                        hardware_project,
                        &mut diagnostics,
                    );
                    if let Some((start, end)) = address.bit_interval() {
                        marker_intervals.push((tag.controller_id, tag.id, start, end));
                    }
                }
            }
        }
        for (controller, count) in controller_counts {
            if count > usize::try_from(profile.limits().tags_per_controller).unwrap_or(usize::MAX) {
                diagnostics.push(
                    Diagnostic::blocking(
                        DiagnosticCode::ResourceLimit,
                        DiagnosticTarget::new(TargetKind::Controller, controller.uuid()),
                        "Tag count exceeds the EDU-21 per-controller limit",
                    )
                    .parameter("limit", "tags_per_controller")
                    .parameter("current", count.to_string())
                    .parameter("requested", count.to_string())
                    .parameter("maximum", profile.limits().tags_per_controller.to_string()),
                );
            }
        }
        marker_intervals.sort_by_key(|(controller, id, start, _)| (*controller, *start, *id));
        for first in 0..marker_intervals.len() {
            for second in first + 1..marker_intervals.len() {
                if marker_intervals[second].0 != marker_intervals[first].0 {
                    break;
                }
                if marker_intervals[second].2 >= marker_intervals[first].3 {
                    break;
                }
                diagnostics.push(
                    Diagnostic::blocking(
                        DiagnosticCode::SymbolOverlap,
                        DiagnosticTarget::new(TargetKind::Tag, marker_intervals[first].1.uuid())
                            .field("address"),
                        "Marker storage allocations overlap; alias/overlay semantics are unsupported",
                    )
                    .related([DiagnosticTarget::new(
                        TargetKind::Tag,
                        marker_intervals[second].1.uuid(),
                    )]),
                );
            }
        }
        diagnostics.sort_by(|left, right| {
            (
                left.code.stable_code(),
                left.primary.id,
                &left.primary.field,
                &left.message,
            )
                .cmp(&(
                    right.code.stable_code(),
                    right.primary.id,
                    &right.primary.field,
                    &right.message,
                ))
        });
        diagnostics
    }

    pub fn preview_auto_allocate_tags(
        &self,
        profile: &TrainingProfile,
        hardware_project: &HardwareProject,
        artifact: &HardwareArtifact,
    ) -> Result<TagAllocationPreview, SymbolError> {
        let mut candidate = self.clone();
        let mut changes = Vec::new();
        let mut occupied_channels: BTreeSet<_> = candidate
            .tags
            .values()
            .filter(|tag| !matches!(tag.address_intent, AddressIntent::Auto(_)))
            .filter_map(|tag| tag.hardware_channel_id)
            .collect();
        let mut marker_intervals: BTreeMap<ControllerId, Vec<(u64, u64)>> = candidate
            .tags
            .values()
            .filter(|tag| !matches!(tag.address_intent, AddressIntent::Auto(_)))
            .filter_map(|tag| {
                tag.allocated_address
                    .and_then(Address::bit_interval)
                    .map(|interval| (tag.controller_id, interval))
            })
            .fold(BTreeMap::new(), |mut map, (controller, interval)| {
                map.entry(controller).or_default().push(interval);
                map
            });
        let mut tag_ids: Vec<_> = candidate.tags.keys().copied().collect();
        tag_ids.sort_by_key(|id| {
            let tag = &candidate.tags[id];
            (tag.creation_ordinal, *id)
        });
        for tag_id in tag_ids {
            let tag = &candidate.tags[&tag_id];
            let AddressIntent::Auto(area) = tag.address_intent else {
                continue;
            };
            let previous_address = tag.allocated_address;
            let previous_channel_id = tag.hardware_channel_id;
            let (proposed_address, proposed_channel_id) = match area {
                SymbolAddressArea::Input | SymbolAddressArea::Output => {
                    let current = tag.hardware_channel_id.and_then(|channel_id| {
                        artifact
                            .channel_bindings
                            .get(&channel_id)
                            .filter(|binding| {
                                !occupied_channels.contains(&channel_id)
                                    && channel_matches_tag(binding, tag, area)
                                    && tag.allocated_address
                                        == Some(address_from_channel(binding.address))
                            })
                    });
                    let binding = current
                        .or_else(|| {
                            artifact
                                .channel_bindings
                                .values()
                                .filter(|binding| binding.controller_id == tag.controller_id)
                                .filter(|binding| !occupied_channels.contains(&binding.channel_id))
                                .filter(|binding| channel_matches_tag(binding, tag, area))
                                .min_by_key(|binding| {
                                    (
                                        binding.controller_creation_ordinal,
                                        binding.location_rank,
                                        binding.station_creation_ordinal,
                                        binding.slot_number,
                                        binding.channel_index,
                                        binding.channel_id,
                                    )
                                })
                        })
                        .ok_or(SymbolError::NoAutomaticAddressAvailable(tag_id))?;
                    occupied_channels.insert(binding.channel_id);
                    (
                        address_from_channel(binding.address),
                        Some(binding.channel_id),
                    )
                }
                SymbolAddressArea::Marker => {
                    let controller = hardware_project
                        .controllers()
                        .get(&tag.controller_id)
                        .ok_or(SymbolError::UnknownControllerScope(tag.controller_id))?;
                    let definition = profile
                        .controller(controller.catalog_id)
                        .ok_or(SymbolError::UnknownControllerScope(tag.controller_id))?;
                    let occupied = marker_intervals.entry(tag.controller_id).or_default();
                    let preserved = tag.allocated_address.filter(|address| {
                        address.area() == SymbolAddressArea::Marker
                            && address_matches_type(*address, &tag.declared_type)
                            && address.bit_interval().is_some_and(|(start, end)| {
                                end <= u64::from(definition.marker_bytes) * 8
                                    && occupied.iter().all(|(other_start, other_end)| {
                                        start >= *other_end || end <= *other_start
                                    })
                            })
                    });
                    let address = preserved
                        .or_else(|| {
                            first_fit_marker(&tag.declared_type, definition.marker_bytes, occupied)
                        })
                        .ok_or(SymbolError::NoAutomaticAddressAvailable(tag_id))?;
                    let interval = address
                        .bit_interval()
                        .ok_or(SymbolError::NoAutomaticAddressAvailable(tag_id))?;
                    occupied.push(interval);
                    (address, None)
                }
            };
            if previous_address != Some(proposed_address)
                || previous_channel_id != proposed_channel_id
            {
                changes.push(TagAllocationChange {
                    tag_id,
                    previous_address,
                    proposed_address,
                    previous_channel_id,
                    proposed_channel_id,
                });
                let tag = candidate
                    .tags
                    .get_mut(&tag_id)
                    .ok_or(SymbolError::UnknownTag(tag_id))?;
                tag.allocated_address = Some(proposed_address);
                tag.hardware_channel_id = proposed_channel_id;
            }
        }
        let diagnostics = candidate.validate_tags(profile, hardware_project, artifact);
        if !diagnostics.is_empty() {
            return Err(SymbolError::Diagnostics(diagnostics));
        }
        Ok(TagAllocationPreview {
            expected_semantic_fingerprint: self.semantic_fingerprint(),
            changes,
            proposed_semantic_fingerprint: candidate.semantic_fingerprint(),
        })
    }

    pub fn commit_auto_allocate_tags(
        &mut self,
        profile: &TrainingProfile,
        hardware_project: &HardwareProject,
        artifact: &HardwareArtifact,
        preview: &TagAllocationPreview,
    ) -> Result<(), SymbolError> {
        if self.semantic_fingerprint() != preview.expected_semantic_fingerprint {
            return Err(SymbolError::StalePreview);
        }
        let fresh = self.preview_auto_allocate_tags(profile, hardware_project, artifact)?;
        if fresh != *preview {
            return Err(SymbolError::StalePreview);
        }
        let mut candidate = self.clone();
        for change in &preview.changes {
            let tag = candidate
                .tags
                .get_mut(&change.tag_id)
                .ok_or(SymbolError::UnknownTag(change.tag_id))?;
            tag.allocated_address = Some(change.proposed_address);
            tag.hardware_channel_id = change.proposed_channel_id;
        }
        candidate.bump_revision()?;
        if candidate.semantic_fingerprint() != preview.proposed_semantic_fingerprint {
            return Err(SymbolError::StalePreview);
        }
        *self = candidate;
        Ok(())
    }

    #[must_use]
    pub fn semantic_fingerprint(&self) -> Sha256Digest {
        let mut encoder = CanonicalEncoder::default();
        encoder.domain("EDU21-SYMBOL-SEMANTICS-V1");
        encoder.text(&self.profile_pin.id);
        encoder.text(&self.profile_pin.version);
        encoder.digest(self.profile_pin.manifest_hash);
        encoder.usize(self.scopes.len());
        for scope in self.scopes.values() {
            encode_scope(scope, &mut encoder);
        }
        encoder.usize(self.declarations.len());
        for declaration in self.declarations.values() {
            encode_declaration(declaration, &mut encoder);
        }
        encoder.usize(self.references.len());
        for reference in self.references.values() {
            encode_reference(reference, &mut encoder);
        }
        encoder.usize(self.tags.len());
        for tag in self.tags.values() {
            encode_tag_semantics(tag, &mut encoder);
        }
        encoder.fingerprint()
    }

    fn validate_new_declaration(
        &self,
        declaration: &Declaration,
        replacing: Option<DeclarationId>,
    ) -> Result<(), SymbolError> {
        if !declaration.id.uuid().is_rfc9562_v4() {
            return Err(SymbolError::InvalidIdentity(declaration.id.uuid()));
        }
        if replacing.is_none() && self.declarations.contains_key(&declaration.id) {
            return Err(SymbolError::DuplicateIdentity(declaration.id.uuid()));
        }
        if !self.scopes.contains_key(&declaration.scope_id) {
            return Err(SymbolError::UnknownScope(declaration.scope_id));
        }
        if declaration.namespace != declaration.kind.expected_namespace() {
            return Err(SymbolError::NamespaceKindMismatch);
        }
        let duplicate = self.declarations.values().any(|existing| {
            Some(existing.id) != replacing
                && !existing.deleted
                && existing.scope_id == declaration.scope_id
                && existing.namespace == declaration.namespace
                && existing.name.folded() == declaration.name.folded()
        });
        if duplicate {
            return Err(SymbolError::DuplicateName);
        }
        if declaration.namespace == Namespace::Value {
            let controller = self.scope_controller(declaration.scope_id);
            if let Some(controller) = controller {
                let shadow = self.declarations.values().any(|existing| {
                    Some(existing.id) != replacing
                        && !existing.deleted
                        && existing.namespace == Namespace::Value
                        && existing.name.folded() == declaration.name.folded()
                        && self.scope_controller(existing.scope_id) == Some(controller)
                        && existing.scope_id != declaration.scope_id
                        && (self.is_global_scope(existing.scope_id)
                            || self.is_global_scope(declaration.scope_id))
                });
                if shadow {
                    return Err(SymbolError::ShadowingProhibited);
                }
            }
        }
        Ok(())
    }

    fn resolve_path(
        &self,
        path: &[Identifier],
        expected_namespace: Namespace,
        scope_id: ScopeId,
        source: SourceIdentity,
        binding_kind: BindingKind,
    ) -> Result<Resolution, SymbolError> {
        if !self.scopes.contains_key(&scope_id) {
            return Err(SymbolError::UnknownScope(scope_id));
        }
        let first_namespace = if path.len() == 1 {
            expected_namespace
        } else {
            Namespace::Value
        };
        let first_candidates = self.visible_candidates(scope_id, first_namespace, &path[0]);
        if first_candidates.is_empty() {
            return Ok(Resolution::Unresolved);
        }
        if first_candidates.len() > 1 {
            return Ok(Resolution::Ambiguous(first_candidates));
        }
        let mut target_path = vec![first_candidates[0]];
        let mut current = &self.declarations[&first_candidates[0]];
        for segment in &path[1..] {
            let Some(member_scope) = current.member_scope_id else {
                return Ok(Resolution::Unresolved);
            };
            let candidates: Vec<_> = self
                .declarations
                .values()
                .filter(|declaration| {
                    !declaration.deleted
                        && declaration.scope_id == member_scope
                        && declaration.namespace == Namespace::Member
                        && declaration.name.folded() == segment.folded()
                })
                .map(|declaration| declaration.id)
                .collect();
            if candidates.is_empty() {
                return Ok(Resolution::Unresolved);
            }
            if candidates.len() > 1 {
                return Ok(Resolution::Ambiguous(candidates));
            }
            current = &self.declarations[&candidates[0]];
            target_path.push(current.id);
        }
        if current.namespace != expected_namespace
            && !(path.len() > 1 && current.namespace == Namespace::Member)
        {
            return Ok(Resolution::Unresolved);
        }
        Ok(Resolution::Resolved(Binding {
            target_id: current.id,
            target_kind: current.kind,
            owning_scope_id: scope_id,
            source,
            binding_kind,
            target_path,
            display_path: path.to_vec(),
        }))
    }

    fn visible_candidates(
        &self,
        scope_id: ScopeId,
        namespace: Namespace,
        name: &Identifier,
    ) -> Vec<DeclarationId> {
        let folded = name.folded();
        let mut current = Some(scope_id);
        while let Some(scope) = current {
            let matches: Vec<_> = self
                .declarations
                .values()
                .filter(|declaration| {
                    !declaration.deleted
                        && declaration.scope_id == scope
                        && declaration.namespace == namespace
                        && declaration.name.folded() == folded
                })
                .map(|declaration| declaration.id)
                .collect();
            if !matches.is_empty() {
                return matches;
            }
            current = self
                .scopes
                .get(&scope)
                .and_then(|scope| scope.parent_scope_id);
        }
        Vec::new()
    }

    fn apply_rename(
        &mut self,
        id: DeclarationId,
        new_name: &Identifier,
    ) -> Result<(), SymbolError> {
        self.declarations
            .get_mut(&id)
            .ok_or(SymbolError::UnknownDeclaration(id))?
            .name = new_name.clone();
        for reference in self.references.values_mut() {
            let binding = match &mut reference.state {
                ReferenceState::Resolved(binding) | ReferenceState::StaleDeleted(binding) => {
                    binding
                }
                ReferenceState::NeverResolved { .. } => continue,
            };
            for (index, target) in binding.target_path.iter().enumerate() {
                if *target == id {
                    binding.display_path[index] = new_name.clone();
                }
            }
        }
        for tag in self.tags.values_mut() {
            if tag.declaration_id == id {
                tag.name = new_name.clone();
            }
        }
        self.rebuild_cross_references();
        Ok(())
    }

    fn rebuild_cross_references(&mut self) {
        self.cross_references = CrossReferenceIndex::rebuild(&self.references);
    }

    fn scope_controller(&self, scope_id: ScopeId) -> Option<ControllerId> {
        match self.scopes.get(&scope_id)?.kind {
            ScopeKind::ControllerGlobal(controller)
            | ScopeKind::Block {
                controller_id: controller,
                ..
            } => Some(controller),
            ScopeKind::AggregateMember(owner) => self
                .declarations
                .get(&owner)
                .and_then(|declaration| self.scope_controller(declaration.scope_id)),
            ScopeKind::InstructionRegistry | ScopeKind::HardwareCatalog => None,
        }
    }

    fn is_global_scope(&self, scope_id: ScopeId) -> bool {
        self.scopes
            .get(&scope_id)
            .is_some_and(|scope| matches!(scope.kind, ScopeKind::ControllerGlobal(_)))
    }

    fn bump_revision(&mut self) -> Result<(), SymbolError> {
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(SymbolError::RevisionOverflow)?;
        Ok(())
    }
}

fn parse_byte(input: &str) -> Result<u32, AddressError> {
    if input.is_empty()
        || !input.bytes().all(|byte| byte.is_ascii_digit())
        || (input.len() > 1 && input.starts_with('0'))
    {
        return Err(AddressError::Malformed);
    }
    input.parse().map_err(|_| AddressError::ByteOverflow)
}

fn parse_bit(input: &str) -> Result<(u32, u8), AddressError> {
    let (byte, bit) = input.split_once('.').ok_or(AddressError::Malformed)?;
    if bit.len() != 1 || !bit.as_bytes()[0].is_ascii_digit() {
        return Err(AddressError::Malformed);
    }
    let bit = bit.parse::<u8>().map_err(|_| AddressError::BitOutOfRange)?;
    if bit > 7 {
        return Err(AddressError::BitOutOfRange);
    }
    Ok((parse_byte(byte)?, bit))
}

fn is_reserved_keyword(input: &str) -> bool {
    matches!(
        input.to_ascii_uppercase().as_str(),
        "AND"
            | "ARRAY"
            | "BEGIN"
            | "BLOCK_DB"
            | "BOOL"
            | "BYTE"
            | "BY"
            | "CASE"
            | "CHAR"
            | "CONSTANT"
            | "CONTINUE"
            | "CONFIGURATION"
            | "DATE"
            | "DATE_AND_TIME"
            | "DATA_BLOCK"
            | "DINT"
            | "DO"
            | "DWORD"
            | "ELSE"
            | "ELSIF"
            | "END_BLOCK"
            | "END_CASE"
            | "END_CONFIGURATION"
            | "END_DATA_BLOCK"
            | "END_FOR"
            | "END_FUNCTION"
            | "END_FUNCTION_BLOCK"
            | "END_IF"
            | "END_ORGANIZATION_BLOCK"
            | "END_PROGRAM"
            | "END_REPEAT"
            | "END_RESOURCE"
            | "END_STRUCT"
            | "END_TYPE"
            | "END_VAR"
            | "END_WHILE"
            | "EXIT"
            | "FALSE"
            | "FOR"
            | "FUNCTION"
            | "FUNCTION_BLOCK"
            | "IF"
            | "INT"
            | "LINT"
            | "LREAL"
            | "LWORD"
            | "MOD"
            | "NOT"
            | "OF"
            | "OR"
            | "ORGANIZATION_BLOCK"
            | "PROGRAM"
            | "REAL"
            | "REPEAT"
            | "RESOURCE"
            | "RETURN"
            | "SINT"
            | "STRING"
            | "STRUCT"
            | "THEN"
            | "TIME"
            | "TIME_OF_DAY"
            | "TO"
            | "TRUE"
            | "TYPE"
            | "UDINT"
            | "UINT"
            | "ULINT"
            | "UNTIL"
            | "USINT"
            | "VAR"
            | "VAR_CONSTANT"
            | "VAR_CONFIG"
            | "VAR_ACCESS"
            | "VAR_EXTERNAL"
            | "VAR_GLOBAL"
            | "VAR_INPUT"
            | "VAR_IN_OUT"
            | "VAR_OUTPUT"
            | "VAR_STAT"
            | "VAR_TEMP"
            | "WHILE"
            | "WORD"
            | "WSTRING"
            | "XOR"
    )
}

fn validate_tag_value(tag: &Tag, diagnostics: &mut Vec<Diagnostic>) {
    if let Some(start_value) = &tag.start_value
        && tag.declared_type.validate_value(start_value).is_err()
    {
        diagnostics.push(Diagnostic::blocking(
            DiagnosticCode::TypeMismatch,
            DiagnosticTarget::new(TargetKind::Tag, tag.id.uuid()).field("startValue"),
            "Tag start value is not valid for its declared canonical type",
        ));
    }
    if let TagKind::Constant(value) = &tag.kind {
        if tag.declared_type.validate_value(value).is_err() {
            diagnostics.push(Diagnostic::blocking(
                DiagnosticCode::ConstantRangeOrArithmetic,
                DiagnosticTarget::new(TargetKind::Tag, tag.id.uuid()).field("constantValue"),
                "Constant value is outside its declared fixed-width type",
            ));
        }
        if tag.start_value.is_some() || tag.retain_policy == RetainPolicy::Retentive {
            diagnostics.push(Diagnostic::blocking(
                DiagnosticCode::TypeMismatch,
                DiagnosticTarget::new(TargetKind::Tag, tag.id.uuid()),
                "Constant declaration cannot have runtime start/retention state",
            ));
        }
    }
}

fn validate_address_type(tag: &Tag, address: Address, diagnostics: &mut Vec<Diagnostic>) {
    if !address_matches_type(address, &tag.declared_type) {
        diagnostics.push(Diagnostic::blocking(
            DiagnosticCode::TypeMismatch,
            DiagnosticTarget::new(TargetKind::Tag, tag.id.uuid()).field("address"),
            "PLC address area/width is incompatible with the tag's canonical type",
        ));
    }
}

fn address_matches_type(address: Address, ty: &CanonicalType) -> bool {
    let primitive = match ty {
        CanonicalType::Primitive(primitive) => Some(*primitive),
        _ => None,
    };
    match (address, primitive) {
        (
            Address::InputBit { .. } | Address::OutputBit { .. } | Address::MarkerBit { .. },
            Some(PrimitiveType::Bool),
        )
        | (Address::InputWord { .. } | Address::OutputWord { .. }, Some(PrimitiveType::Int)) => {
            true
        }
        (Address::MarkerByte { .. }, Some(primitive)) => {
            matches!(
                primitive,
                PrimitiveType::Sint
                    | PrimitiveType::Usint
                    | PrimitiveType::Byte
                    | PrimitiveType::Char
            )
        }
        (Address::MarkerWord { .. }, Some(primitive)) => {
            matches!(
                primitive,
                PrimitiveType::Int | PrimitiveType::Uint | PrimitiveType::Word
            )
        }
        (Address::MarkerDword { .. }, Some(primitive)) => matches!(
            primitive,
            PrimitiveType::Dint | PrimitiveType::Udint | PrimitiveType::Dword | PrimitiveType::Real
        ),
        (Address::MarkerLword { .. }, Some(primitive)) => matches!(
            primitive,
            PrimitiveType::Lint
                | PrimitiveType::Ulint
                | PrimitiveType::Lword
                | PrimitiveType::Lreal
                | PrimitiveType::Time
        ),
        _ => false,
    }
}

fn validate_io_binding(
    tag: &Tag,
    address: Address,
    artifact: &HardwareArtifact,
    used_channels: &mut BTreeMap<ChannelId, TagId>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(channel_id) = tag.hardware_channel_id else {
        diagnostics.push(Diagnostic::blocking(
            DiagnosticCode::UnmappedIoAddress,
            DiagnosticTarget::new(TargetKind::Tag, tag.id.uuid()).field("hardwareChannelId"),
            "EDU-21 I/Q tag must bind exactly one configured hardware channel",
        ));
        return;
    };
    let Some(binding) = artifact.channel_bindings.get(&channel_id) else {
        diagnostics.push(Diagnostic::blocking(
            DiagnosticCode::UnmappedIoAddress,
            DiagnosticTarget::new(TargetKind::Tag, tag.id.uuid()).field("hardwareChannelId"),
            "I/Q tag references a channel absent from the current hardware artifact",
        ));
        return;
    };
    if binding.controller_id != tag.controller_id
        || address_from_channel(binding.address) != address
    {
        diagnostics.push(Diagnostic::blocking(
            DiagnosticCode::UnmappedIoAddress,
            DiagnosticTarget::new(TargetKind::Tag, tag.id.uuid()).field("address"),
            "I/Q address does not equal the complete bound channel's canonical address",
        ));
    }
    if let Some(other_tag) = used_channels.insert(channel_id, tag.id) {
        diagnostics.push(
            Diagnostic::blocking(
                DiagnosticCode::SymbolOverlap,
                DiagnosticTarget::new(TargetKind::Tag, tag.id.uuid()).field("hardwareChannelId"),
                "Two tags bind the same I/Q hardware channel; aliasing is unsupported",
            )
            .related([DiagnosticTarget::new(TargetKind::Tag, other_tag.uuid())]),
        );
    }
}

fn validate_marker_capacity(
    tag: &Tag,
    address: Address,
    profile: &TrainingProfile,
    hardware_project: &HardwareProject,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(controller) = hardware_project.controllers().get(&tag.controller_id) else {
        diagnostics.push(Diagnostic::blocking(
            DiagnosticCode::PlcAddressCapacity,
            DiagnosticTarget::new(TargetKind::Tag, tag.id.uuid()).field("controllerId"),
            "Tag controller is absent from hardware configuration",
        ));
        return;
    };
    let Some(definition) = profile.controller(controller.catalog_id) else {
        return;
    };
    let Some((_, end)) = address.bit_interval() else {
        return;
    };
    if end > u64::from(definition.marker_bytes) * 8 {
        diagnostics.push(
            Diagnostic::blocking(
                DiagnosticCode::PlcAddressCapacity,
                DiagnosticTarget::new(TargetKind::Tag, tag.id.uuid()).field("address"),
                "Marker address exceeds the selected controller's M-area capacity",
            )
            .parameter("maximumBytes", definition.marker_bytes.to_string()),
        );
    }
}

fn channel_matches_tag(
    binding: &crate::hardware::HardwareChannelBinding,
    tag: &Tag,
    area: SymbolAddressArea,
) -> bool {
    let primitive = match &tag.declared_type {
        CanonicalType::Primitive(primitive) => *primitive,
        _ => return false,
    };
    let expected_direction = match area {
        SymbolAddressArea::Input => ChannelDirection::Input,
        SymbolAddressArea::Output => ChannelDirection::Output,
        SymbolAddressArea::Marker => return false,
    };
    binding.direction == expected_direction
        && matches!(
            (binding.raw_type, primitive),
            (PrimitiveType::Bool, PrimitiveType::Bool) | (PrimitiveType::Int, PrimitiveType::Int)
        )
}

fn address_from_channel(address: ChannelAddress) -> Address {
    match address {
        ChannelAddress::Bit {
            area: AddressArea::Input,
            byte,
            bit,
        } => Address::InputBit { byte, bit },
        ChannelAddress::Bit {
            area: AddressArea::Output,
            byte,
            bit,
        } => Address::OutputBit { byte, bit },
        ChannelAddress::Word {
            area: AddressArea::Input,
            byte,
        } => Address::InputWord { byte },
        ChannelAddress::Word {
            area: AddressArea::Output,
            byte,
        } => Address::OutputWord { byte },
    }
}

fn first_fit_marker(
    ty: &CanonicalType,
    capacity_bytes: u32,
    occupied: &[(u64, u64)],
) -> Option<Address> {
    let primitive = match ty {
        CanonicalType::Primitive(primitive) => *primitive,
        _ => return None,
    };
    if primitive == PrimitiveType::Bool {
        for bit_index in 0..u64::from(capacity_bytes) * 8 {
            if occupied
                .iter()
                .all(|(start, end)| bit_index >= *end || bit_index < *start)
            {
                return Some(Address::MarkerBit {
                    byte: u32::try_from(bit_index / 8).ok()?,
                    bit: u8::try_from(bit_index % 8).ok()?,
                });
            }
        }
        return None;
    }
    let width = primitive.storage_width_bytes()?;
    for byte in 0..capacity_bytes {
        if byte % u32::from(width) != 0 || byte.checked_add(u32::from(width))? > capacity_bytes {
            continue;
        }
        let start = u64::from(byte) * 8;
        let end = start + u64::from(width) * 8;
        if occupied
            .iter()
            .all(|(other_start, other_end)| start >= *other_end || end <= *other_start)
        {
            return match width {
                1 => Some(Address::MarkerByte { byte }),
                2 => Some(Address::MarkerWord { byte }),
                4 => Some(Address::MarkerDword { byte }),
                8 => Some(Address::MarkerLword { byte }),
                _ => None,
            };
        }
    }
    None
}

fn encode_scope(scope: &Scope, encoder: &mut CanonicalEncoder) {
    encoder.uuid(scope.id.uuid());
    encoder.u64(scope.creation_ordinal);
    encoder.text(&format!("{:?}", scope.kind));
    encoder.option(scope.parent_scope_id, |encoder, id| encoder.uuid(id.uuid()));
}

fn encode_declaration(declaration: &Declaration, encoder: &mut CanonicalEncoder) {
    encoder.uuid(declaration.id.uuid());
    encoder.u64(declaration.creation_ordinal);
    encoder.text(declaration.name.as_str());
    encoder.uuid(declaration.scope_id.uuid());
    encoder.text(&format!("{:?}", declaration.namespace));
    encoder.text(&format!("{:?}", declaration.kind));
    encoder.option(declaration.member_scope_id, |encoder, id| {
        encoder.uuid(id.uuid());
    });
    encoder.bool(declaration.deleted);
}

fn encode_reference(reference: &Reference, encoder: &mut CanonicalEncoder) {
    encoder.uuid(reference.id.uuid());
    encoder.u64(reference.creation_ordinal);
    match &reference.state {
        ReferenceState::Resolved(binding) => {
            encoder.tag("resolved");
            encode_binding(binding, encoder);
        }
        ReferenceState::StaleDeleted(binding) => {
            encoder.tag("stale-deleted");
            encode_binding(binding, encoder);
        }
        ReferenceState::NeverResolved {
            authored_spelling,
            qualification_path,
            expected_namespace,
            scope_id,
            source,
            binding_kind,
        } => {
            encoder.tag("never-resolved");
            encoder.text(authored_spelling);
            encoder.usize(qualification_path.len());
            for segment in qualification_path {
                encoder.text(segment.as_str());
            }
            encoder.text(&format!("{expected_namespace:?}"));
            encoder.uuid(scope_id.uuid());
            encode_source(source, encoder);
            encoder.text(&format!("{binding_kind:?}"));
        }
    }
}

fn encode_binding(binding: &Binding, encoder: &mut CanonicalEncoder) {
    encoder.uuid(binding.target_id.uuid());
    encoder.text(&format!("{:?}", binding.target_kind));
    encoder.uuid(binding.owning_scope_id.uuid());
    encode_source(&binding.source, encoder);
    encoder.text(&format!("{:?}", binding.binding_kind));
    encoder.usize(binding.target_path.len());
    for target in &binding.target_path {
        encoder.uuid(target.uuid());
    }
    encoder.usize(binding.display_path.len());
    for segment in &binding.display_path {
        encoder.text(segment.as_str());
    }
}

fn encode_source(source: &SourceIdentity, encoder: &mut CanonicalEncoder) {
    encoder.uuid(source.object_id.uuid());
    encoder.text(&source.location);
}

fn encode_tag_semantics(tag: &Tag, encoder: &mut CanonicalEncoder) {
    encoder.uuid(tag.id.uuid());
    encoder.uuid(tag.declaration_id.uuid());
    encoder.uuid(tag.controller_id.uuid());
    encoder.u64(tag.creation_ordinal);
    encoder.text(tag.name.as_str());
    tag.declared_type.encode(encoder, false);
    encode_address_intent(&tag.address_intent, encoder);
    encoder.option(tag.allocated_address, encode_address);
    encoder.option(tag.start_value.as_ref(), |encoder, value| {
        value.encode(encoder);
    });
    encoder.text(&format!("{:?}", tag.retain_policy));
    match &tag.kind {
        TagKind::Variable => encoder.tag("variable"),
        TagKind::Constant(value) => {
            encoder.tag("constant");
            value.encode(encoder);
        }
    }
    encoder.option(tag.hardware_channel_id, |encoder, id| {
        encoder.uuid(id.uuid());
    });
}

fn encode_address_intent(intent: &AddressIntent, encoder: &mut CanonicalEncoder) {
    match intent {
        AddressIntent::None => encoder.tag("none"),
        AddressIntent::Auto(area) => {
            encoder.tag("auto");
            encoder.text(&format!("{area:?}"));
        }
        AddressIntent::Explicit {
            authored_text,
            parsed,
        } => {
            encoder.tag("explicit");
            encoder.text(authored_text);
            encoder.option(*parsed, encode_address);
        }
    }
}

fn encode_address(encoder: &mut CanonicalEncoder, address: Address) {
    encoder.text(&address.to_string());
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SymbolError {
    Identifier(IdentifierError),
    Type(TypeError),
    Profile(ProfileError),
    InvalidIdentity(Uuid),
    DuplicateIdentity(Uuid),
    DuplicateName,
    ShadowingProhibited,
    NamespaceKindMismatch,
    UnknownScope(ScopeId),
    UnknownControllerScope(ControllerId),
    UnknownDeclaration(DeclarationId),
    DeletedDeclaration(DeclarationId),
    UnknownReference(ReferenceId),
    UnknownTagTable(TagTableId),
    UnknownTag(TagId),
    DuplicateDefaultTagTable,
    TagTableControllerMismatch,
    TagDeclarationMismatch,
    EmptyReferencePath,
    ReservedHmiBinding,
    NoAutomaticAddressAvailable(TagId),
    TagCapacity {
        controller_id: ControllerId,
        maximum: u32,
    },
    RevisionOverflow,
    StalePreview,
    Diagnostics(Vec<Diagnostic>),
}

impl From<IdentifierError> for SymbolError {
    fn from(value: IdentifierError) -> Self {
        Self::Identifier(value)
    }
}

impl From<TypeError> for SymbolError {
    fn from(value: TypeError) -> Self {
        Self::Type(value)
    }
}

impl From<ProfileError> for SymbolError {
    fn from(value: ProfileError) -> Self {
        Self::Profile(value)
    }
}

impl fmt::Display for SymbolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SymbolError {}

#[cfg(test)]
mod tests {
    use super::{Address, Identifier, IdentifierError};

    #[test]
    fn identifier_and_address_negative_corpus_is_precise() {
        for invalid in [
            "",
            "9bad",
            "has-dash",
            "naive\u{00ef}",
            "IF",
            "real",
            "END_BLOCK",
            "VAR_TEMP",
        ] {
            assert!(Identifier::parse(invalid).is_err(), "{invalid}");
        }
        assert_eq!(
            Identifier::parse("Motor_Start")
                .expect("valid identifier")
                .as_str(),
            "Motor_Start"
        );
        assert_eq!(
            Identifier::parse("if"),
            Err(IdentifierError::ReservedKeyword)
        );

        for invalid in [
            "I0",
            "I0.8",
            "IB0",
            "ID0",
            "QW1.0",
            "M-1.0",
            "M1.2.3",
            "DB1.DBX0.0",
            "http://I0.0",
            "I0.0:80",
            "IW02",
            "M00.0",
        ] {
            assert!(Address::parse(invalid).is_err(), "{invalid}");
        }
        assert_eq!(Address::parse("%iw2").expect("valid").to_string(), "IW2");
        assert_eq!(Address::parse("ML8").expect("valid").to_string(), "ML8");
    }
}
