#![allow(clippy::missing_errors_doc, clippy::too_many_lines)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::hash::{Sha256Digest, sha256};
use crate::json::{JsonError, JsonValue, canonical_json, require_only_fields, required};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Uuid([u8; 16]);

impl Uuid {
    pub const NIL: Self = Self([0; 16]);

    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn into_bytes(self) -> [u8; 16] {
        self.0
    }

    /// Creates a deterministic RFC 9562 UUIDv4-shaped identity.
    ///
    /// The caller supplies the seed. A replay must reuse the recorded UUID, not
    /// call this function again with a newly selected seed.
    #[must_use]
    pub fn deterministic_v4(seed: &[u8], ordinal: u64) -> Self {
        let mut input = Vec::with_capacity(seed.len() + 8);
        input.extend_from_slice(seed);
        input.extend_from_slice(&ordinal.to_be_bytes());
        let digest = sha256(&input);
        let mut bytes = [0_u8; 16];
        bytes.copy_from_slice(&digest.0[..16]);
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Self(bytes)
    }

    pub fn parse(input: &str) -> Result<Self, IdParseError> {
        if input.len() != 36 {
            return Err(IdParseError);
        }
        let bytes = input.as_bytes();
        for index in [8, 13, 18, 23] {
            if bytes[index] != b'-' {
                return Err(IdParseError);
            }
        }
        let mut output = [0_u8; 16];
        let mut source = 0;
        for target in &mut output {
            while matches!(source, 8 | 13 | 18 | 23) {
                source += 1;
            }
            let high = hex_value(bytes[source]).ok_or(IdParseError)?;
            let low = hex_value(bytes[source + 1]).ok_or(IdParseError)?;
            *target = (high << 4) | low;
            source += 2;
        }
        Ok(Self(output))
    }

    #[must_use]
    pub fn is_rfc9562_v4(self) -> bool {
        (self.0[6] >> 4) == 4 && (self.0[8] & 0xc0) == 0x80
    }
}

impl fmt::Display for Uuid {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, byte) in self.0.iter().enumerate() {
            if matches!(index, 4 | 6 | 8 | 10) {
                formatter.write_str("-")?;
            }
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for Uuid {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IdParseError;

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub Uuid);

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, formatter)
            }
        }

        impl From<Uuid> for $name {
            fn from(value: Uuid) -> Self {
                Self(value)
            }
        }
    };
}

id_type!(ObjectId);
id_type!(TransactionId);
id_type!(UndoToken);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfilePin {
    pub id: String,
    pub version: String,
    pub manifest_hash: Sha256Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PayloadValue {
    Null,
    Bool(bool),
    Signed(i64),
    Unsigned(u64),
    String(String),
    List(Vec<Self>),
    Record(BTreeMap<String, Self>),
}

impl From<&str> for PayloadValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<String> for PayloadValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<bool> for PayloadValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Payload {
    pub semantic: BTreeMap<String, PayloadValue>,
    pub presentation: BTreeMap<String, PayloadValue>,
}

/// Bounded simulator-owned structured data preserved across package
/// round-trips without becoming an executable class, external reference, or
/// protocol adapter. Extension namespaces are deliberately restricted to the
/// `edu.*` authority owned by this simulator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimulatorExtension {
    namespace: String,
    schema_version: u32,
    data: PayloadValue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SimulatorExtensionError;

impl SimulatorExtension {
    pub fn new(
        namespace: impl Into<String>,
        schema_version: u32,
        data: PayloadValue,
    ) -> Result<Self, SimulatorExtensionError> {
        let extension = Self {
            namespace: namespace.into(),
            schema_version,
            data,
        };
        if extension.is_valid() {
            Ok(extension)
        } else {
            Err(SimulatorExtensionError)
        }
    }

    #[must_use]
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    #[must_use]
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    #[must_use]
    pub const fn data(&self) -> &PayloadValue {
        &self.data
    }

    fn is_valid(&self) -> bool {
        let mut count = 0_usize;
        self.schema_version == 1
            && valid_extension_namespace(&self.namespace)
            && matches!(self.data, PayloadValue::Record(_))
            && valid_payload_value(&self.data, 0, &mut count)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProjectObjectKind {
    Project,
    Folder,
    Controller,
    Rack,
    Module,
    Network,
    SymbolTable,
    Tag,
    TypeDefinition,
    ProgramBlock,
    DataBlock,
    BuildRecord,
    SnapshotReference,
    Generic,
}

impl ProjectObjectKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Folder => "folder",
            Self::Controller => "controller",
            Self::Rack => "rack",
            Self::Module => "module",
            Self::Network => "network",
            Self::SymbolTable => "symbol-table",
            Self::Tag => "tag",
            Self::TypeDefinition => "type-definition",
            Self::ProgramBlock => "program-block",
            Self::DataBlock => "data-block",
            Self::BuildRecord => "build-record",
            Self::SnapshotReference => "snapshot-reference",
            Self::Generic => "generic",
        }
    }

    fn parse(value: &str) -> Result<Self, JsonError> {
        match value {
            "project" => Ok(Self::Project),
            "folder" => Ok(Self::Folder),
            "controller" => Ok(Self::Controller),
            "rack" => Ok(Self::Rack),
            "module" => Ok(Self::Module),
            "network" => Ok(Self::Network),
            "symbol-table" => Ok(Self::SymbolTable),
            "tag" => Ok(Self::Tag),
            "type-definition" => Ok(Self::TypeDefinition),
            "program-block" => Ok(Self::ProgramBlock),
            "data-block" => Ok(Self::DataBlock),
            "build-record" => Ok(Self::BuildRecord),
            "snapshot-reference" => Ok(Self::SnapshotReference),
            "generic" => Ok(Self::Generic),
            _ => Err(JsonError::InvalidSyntax),
        }
    }

    #[must_use]
    pub const fn containment_is_semantic(self) -> bool {
        !matches!(
            self,
            Self::Folder | Self::BuildRecord | Self::SnapshotReference
        )
    }

    #[must_use]
    pub const fn name_is_semantic(self) -> bool {
        !matches!(
            self,
            Self::Project | Self::Folder | Self::BuildRecord | Self::SnapshotReference
        )
    }

    /// Returns whether this object kind can own `child` in the canonical
    /// project graph. Folder nodes are organizational and may own any non-root
    /// project object; engineering nodes use a deliberately narrow matrix.
    #[must_use]
    pub const fn can_contain(self, child: Self) -> bool {
        if matches!(child, Self::Project) {
            return false;
        }
        match self {
            Self::Project | Self::Folder => true,
            Self::Controller => matches!(
                child,
                Self::Rack
                    | Self::SymbolTable
                    | Self::TypeDefinition
                    | Self::ProgramBlock
                    | Self::DataBlock
                    | Self::Generic
            ),
            Self::Rack => matches!(child, Self::Module | Self::Generic),
            Self::Network
            | Self::SymbolTable
            | Self::TypeDefinition
            | Self::ProgramBlock
            | Self::DataBlock
            | Self::Module
            | Self::Generic => matches!(child, Self::Generic | Self::Tag),
            Self::Tag | Self::BuildRecord | Self::SnapshotReference => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lifecycle {
    Active,
    Tombstoned,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectObject {
    pub id: ObjectId,
    pub kind: ProjectObjectKind,
    pub object_revision: u64,
    pub semantic_revision: u64,
    pub creation_ordinal: u64,
    pub parent_id: Option<ObjectId>,
    pub display_name: String,
    pub payload_schema: String,
    pub payload: Payload,
    pub lifecycle: Lifecycle,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewObject {
    pub id: ObjectId,
    pub kind: ProjectObjectKind,
    pub parent_id: ObjectId,
    pub display_name: String,
    pub payload_schema: String,
    pub payload: Payload,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReferenceKind {
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
    Generic,
}

impl ReferenceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Declaration => "declaration",
            Self::Read => "read",
            Self::Write => "write",
            Self::ReadWrite => "read-write",
            Self::Call => "call",
            Self::Instantiate => "instantiate",
            Self::TypeUse => "type-use",
            Self::AddressBind => "address-bind",
            Self::HardwareBind => "hardware-bind",
            Self::HmiBindReserved => "hmi-bind-reserved",
            Self::Generic => "generic",
        }
    }

    fn parse(value: &str) -> Result<Self, JsonError> {
        match value {
            "declaration" => Ok(Self::Declaration),
            "read" => Ok(Self::Read),
            "write" => Ok(Self::Write),
            "read-write" => Ok(Self::ReadWrite),
            "call" => Ok(Self::Call),
            "instantiate" => Ok(Self::Instantiate),
            "type-use" => Ok(Self::TypeUse),
            "address-bind" => Ok(Self::AddressBind),
            "hardware-bind" => Ok(Self::HardwareBind),
            "hmi-bind-reserved" => Ok(Self::HmiBindReserved),
            "generic" => Ok(Self::Generic),
            _ => Err(JsonError::InvalidSyntax),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResolutionState {
    Resolved,
    Unresolved,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReferenceEdge {
    pub source_id: ObjectId,
    pub source_location: String,
    pub target_id: ObjectId,
    pub expected_target_kind: ProjectObjectKind,
    pub kind: ReferenceKind,
    pub resolution: ResolutionState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DependencyReason {
    Containment,
    SymbolUse,
    TypeUse,
    InterfaceBinding,
    BlockCall,
    InstanceOwnership,
    HardwareAddress,
    NetworkAssignment,
    HmiBindingReserved,
    ProfileCapability,
}

impl DependencyReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::Containment => "containment",
            Self::SymbolUse => "symbol-use",
            Self::TypeUse => "type-use",
            Self::InterfaceBinding => "interface-binding",
            Self::BlockCall => "block-call",
            Self::InstanceOwnership => "instance-ownership",
            Self::HardwareAddress => "hardware-address",
            Self::NetworkAssignment => "network-assignment",
            Self::HmiBindingReserved => "hmi-binding-reserved",
            Self::ProfileCapability => "profile-capability",
        }
    }

    fn parse(value: &str) -> Result<Self, JsonError> {
        match value {
            "containment" => Ok(Self::Containment),
            "symbol-use" => Ok(Self::SymbolUse),
            "type-use" => Ok(Self::TypeUse),
            "interface-binding" => Ok(Self::InterfaceBinding),
            "block-call" => Ok(Self::BlockCall),
            "instance-ownership" => Ok(Self::InstanceOwnership),
            "hardware-address" => Ok(Self::HardwareAddress),
            "network-assignment" => Ok(Self::NetworkAssignment),
            "hmi-binding-reserved" => Ok(Self::HmiBindingReserved),
            "profile-capability" => Ok(Self::ProfileCapability),
            _ => Err(JsonError::InvalidSyntax),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DependencyEdge {
    pub source_id: ObjectId,
    pub target_id: ObjectId,
    pub reason: DependencyReason,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SavedCheckpoint {
    pub document_revision: u64,
    pub package_hash: Sha256Digest,
    pub content_hash: Sha256Digest,
    pub semantic_fingerprint: Sha256Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Project {
    pub(crate) document_id: Uuid,
    pub(crate) root_id: ObjectId,
    pub(crate) profile: ProfilePin,
    pub(crate) document_revision: u64,
    pub(crate) semantic_revision: u64,
    pub(crate) next_creation_ordinal: u64,
    pub(crate) objects: BTreeMap<ObjectId, ProjectObject>,
    pub(crate) references: BTreeSet<ReferenceEdge>,
    pub(crate) dependencies: BTreeSet<DependencyEdge>,
    pub(crate) simulator_extensions: BTreeMap<String, SimulatorExtension>,
    pub(crate) saved_checkpoint: Option<SavedCheckpoint>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DerivedIndexes {
    pub source_document_hash: Sha256Digest,
    pub children_by_parent: BTreeMap<ObjectId, Vec<ObjectId>>,
    pub name_scope: BTreeMap<(ObjectId, String), Vec<ObjectId>>,
    pub outgoing_references: BTreeMap<ObjectId, Vec<ReferenceEdge>>,
    pub incoming_references: BTreeMap<ObjectId, Vec<ReferenceEdge>>,
    pub outgoing_dependencies: BTreeMap<ObjectId, Vec<DependencyEdge>>,
    pub incoming_dependencies: BTreeMap<ObjectId, Vec<DependencyEdge>>,
    pub callers_by_callee: BTreeMap<ObjectId, Vec<ObjectId>>,
    pub callees_by_caller: BTreeMap<ObjectId, Vec<ObjectId>>,
    pub unresolved_references: Vec<ReferenceEdge>,
}

impl Project {
    #[must_use]
    pub fn new(
        document_id: Uuid,
        root_id: ObjectId,
        display_name: impl Into<String>,
        profile: ProfilePin,
    ) -> Self {
        let root = ProjectObject {
            id: root_id,
            kind: ProjectObjectKind::Project,
            object_revision: 1,
            semantic_revision: 1,
            creation_ordinal: 1,
            parent_id: None,
            display_name: display_name.into(),
            payload_schema: "edu.project-root/1".to_owned(),
            payload: Payload::default(),
            lifecycle: Lifecycle::Active,
        };
        Self {
            document_id,
            root_id,
            profile,
            document_revision: 1,
            semantic_revision: 1,
            next_creation_ordinal: 2,
            objects: BTreeMap::from([(root_id, root)]),
            references: BTreeSet::new(),
            dependencies: BTreeSet::new(),
            simulator_extensions: BTreeMap::new(),
            saved_checkpoint: None,
        }
    }

    /// Creates a new project with explicitly admitted simulator-owned
    /// structured extensions. Loaded projects preserve these through the
    /// package codec; ordinary engineering commands cannot mutate them.
    pub fn new_with_simulator_extensions(
        document_id: Uuid,
        root_id: ObjectId,
        display_name: impl Into<String>,
        profile: ProfilePin,
        extensions: Vec<SimulatorExtension>,
    ) -> Result<Self, SimulatorExtensionError> {
        let mut project = Self::new(document_id, root_id, display_name, profile);
        for extension in extensions {
            let namespace = extension.namespace.clone();
            if !extension.is_valid()
                || project
                    .simulator_extensions
                    .insert(namespace, extension)
                    .is_some()
            {
                return Err(SimulatorExtensionError);
            }
        }
        Ok(project)
    }

    #[must_use]
    pub const fn document_id(&self) -> Uuid {
        self.document_id
    }

    #[must_use]
    pub const fn root_id(&self) -> ObjectId {
        self.root_id
    }

    #[must_use]
    pub const fn document_revision(&self) -> u64 {
        self.document_revision
    }

    #[must_use]
    pub const fn semantic_revision(&self) -> u64 {
        self.semantic_revision
    }

    #[must_use]
    pub fn profile(&self) -> &ProfilePin {
        &self.profile
    }

    #[must_use]
    pub fn object(&self, id: ObjectId) -> Option<&ProjectObject> {
        self.objects.get(&id)
    }

    pub fn objects(&self) -> impl Iterator<Item = &ProjectObject> {
        self.objects.values()
    }

    pub fn references(&self) -> impl Iterator<Item = &ReferenceEdge> {
        self.references.iter()
    }

    pub fn dependencies(&self) -> impl Iterator<Item = &DependencyEdge> {
        self.dependencies.iter()
    }

    pub fn simulator_extensions(&self) -> impl Iterator<Item = &SimulatorExtension> {
        self.simulator_extensions.values()
    }

    /// Rebuilds every graph index owned by this kernel slice exclusively from
    /// canonical objects and edges. No serialized cache is consulted.
    #[must_use]
    pub fn rebuild_indexes(&self) -> DerivedIndexes {
        let mut children_by_parent: BTreeMap<ObjectId, Vec<ObjectId>> = BTreeMap::new();
        let mut name_scope: BTreeMap<(ObjectId, String), Vec<ObjectId>> = BTreeMap::new();
        for object in self
            .objects
            .values()
            .filter(|object| object.lifecycle == Lifecycle::Active)
        {
            if let Some(parent_id) = object.parent_id {
                children_by_parent
                    .entry(parent_id)
                    .or_default()
                    .push(object.id);
                name_scope
                    .entry((parent_id, object.display_name.clone()))
                    .or_default()
                    .push(object.id);
            }
        }
        for ids in children_by_parent
            .values_mut()
            .chain(name_scope.values_mut())
        {
            ids.sort_by_key(|id| (self.objects[id].creation_ordinal, *id));
        }
        let mut outgoing_references: BTreeMap<ObjectId, Vec<ReferenceEdge>> = BTreeMap::new();
        let mut incoming_references: BTreeMap<ObjectId, Vec<ReferenceEdge>> = BTreeMap::new();
        let mut unresolved_references = Vec::new();
        for edge in &self.references {
            outgoing_references
                .entry(edge.source_id)
                .or_default()
                .push(edge.clone());
            incoming_references
                .entry(edge.target_id)
                .or_default()
                .push(edge.clone());
            if edge.resolution == ResolutionState::Unresolved {
                unresolved_references.push(edge.clone());
            }
        }
        let mut outgoing_dependencies: BTreeMap<ObjectId, Vec<DependencyEdge>> = BTreeMap::new();
        let mut incoming_dependencies: BTreeMap<ObjectId, Vec<DependencyEdge>> = BTreeMap::new();
        let mut callers_by_callee: BTreeMap<ObjectId, Vec<ObjectId>> = BTreeMap::new();
        let mut callees_by_caller: BTreeMap<ObjectId, Vec<ObjectId>> = BTreeMap::new();
        for edge in &self.dependencies {
            outgoing_dependencies
                .entry(edge.source_id)
                .or_default()
                .push(edge.clone());
            incoming_dependencies
                .entry(edge.target_id)
                .or_default()
                .push(edge.clone());
            if edge.reason == DependencyReason::BlockCall {
                callers_by_callee
                    .entry(edge.target_id)
                    .or_default()
                    .push(edge.source_id);
                callees_by_caller
                    .entry(edge.source_id)
                    .or_default()
                    .push(edge.target_id);
            }
        }
        for ids in callers_by_callee
            .values_mut()
            .chain(callees_by_caller.values_mut())
        {
            ids.sort_unstable();
            ids.dedup();
        }
        DerivedIndexes {
            source_document_hash: self.document_hash(),
            children_by_parent,
            name_scope,
            outgoing_references,
            incoming_references,
            outgoing_dependencies,
            incoming_dependencies,
            callers_by_callee,
            callees_by_caller,
            unresolved_references,
        }
    }

    #[must_use]
    pub fn document_hash(&self) -> Sha256Digest {
        sha256(&canonical_json(&document_project_json(self)))
    }

    #[must_use]
    pub fn semantic_fingerprint(&self) -> Sha256Digest {
        sha256(&canonical_json(&semantic_project_json(self)))
    }

    #[must_use]
    pub fn is_document_dirty(&self) -> bool {
        self.saved_checkpoint
            .as_ref()
            .is_none_or(|saved| saved.content_hash != self.document_hash())
    }

    #[must_use]
    pub fn is_semantic_dirty(&self) -> bool {
        self.saved_checkpoint
            .as_ref()
            .is_none_or(|saved| saved.semantic_fingerprint != self.semantic_fingerprint())
    }

    pub(crate) fn mark_saved_verified(&mut self, package_hash: Sha256Digest) {
        self.saved_checkpoint = Some(SavedCheckpoint {
            document_revision: self.document_revision,
            package_hash,
            content_hash: self.document_hash(),
            semantic_fingerprint: self.semantic_fingerprint(),
        });
    }

    #[must_use]
    pub fn saved_document_revision(&self) -> Option<u64> {
        self.saved_checkpoint
            .as_ref()
            .map(|checkpoint| checkpoint.document_revision)
    }

    #[must_use]
    pub fn saved_document_hash(&self) -> Option<Sha256Digest> {
        self.saved_checkpoint
            .as_ref()
            .map(|checkpoint| checkpoint.package_hash)
    }

    #[must_use]
    pub fn saved_semantic_fingerprint(&self) -> Option<Sha256Digest> {
        self.saved_checkpoint
            .as_ref()
            .map(|checkpoint| checkpoint.semantic_fingerprint)
    }

    /// Creates the in-memory content for Save As. Object identities, the root
    /// identity, creation order, and profile pin are preserved; only the
    /// document identity changes. The returned project is intentionally dirty
    /// until its newly encoded package has been verified.
    #[must_use]
    pub fn for_save_as(&self, new_document_id: Uuid) -> Option<Self> {
        if new_document_id == Uuid::NIL
            || !new_document_id.is_rfc9562_v4()
            || new_document_id == self.document_id
        {
            return None;
        }
        let mut copy = self.clone();
        copy.document_id = new_document_id;
        copy.saved_checkpoint = None;
        Some(copy)
    }

    pub fn validate(&self) -> Result<(), ProjectValidationError> {
        if self.document_id == Uuid::NIL || !self.document_id.is_rfc9562_v4() {
            return Err(ProjectValidationError::InvalidIdentity);
        }
        if self.document_revision == 0
            || self.semantic_revision == 0
            || self.semantic_revision > self.document_revision
            || self.next_creation_ordinal < 2
        {
            return Err(ProjectValidationError::InvalidRevision);
        }
        if self.profile.id.is_empty()
            || self.profile.id.len() > 128
            || self.profile.version.is_empty()
            || self.profile.version.len() > 128
        {
            return Err(ProjectValidationError::InvalidProfile);
        }
        let root = self
            .objects
            .get(&self.root_id)
            .ok_or(ProjectValidationError::MissingRoot)?;
        if root.kind != ProjectObjectKind::Project
            || root.parent_id.is_some()
            || root.lifecycle != Lifecycle::Active
            || root.creation_ordinal != 1
        {
            return Err(ProjectValidationError::InvalidRoot);
        }
        let mut ordinals = BTreeSet::new();
        for object in self.objects.values() {
            if object.id.0 == Uuid::NIL
                || !object.id.0.is_rfc9562_v4()
                || object.object_revision == 0
                || object.semantic_revision == 0
                || object.semantic_revision > object.object_revision
                || object.display_name.is_empty()
                || object.display_name.len() > 256
                || object.display_name.chars().any(char::is_control)
                || object.payload_schema.is_empty()
                || object.payload_schema.len() > 128
                || !object.payload_schema.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'/' | b'_' | b'-')
                })
                || !valid_payload_map(&object.payload.semantic)
                || !valid_payload_map(&object.payload.presentation)
                || !ordinals.insert(object.creation_ordinal)
                || object.creation_ordinal >= self.next_creation_ordinal
            {
                return Err(ProjectValidationError::InvalidObject);
            }
            if object.id != self.root_id && object.lifecycle == Lifecycle::Active {
                let parent = object
                    .parent_id
                    .and_then(|id| self.objects.get(&id))
                    .ok_or(ProjectValidationError::Orphan)?;
                if parent.lifecycle != Lifecycle::Active {
                    return Err(ProjectValidationError::Orphan);
                }
                if !parent.kind.can_contain(object.kind) {
                    return Err(ProjectValidationError::IllegalContainment);
                }
            }
            let mut visited = BTreeSet::new();
            let mut cursor = object.parent_id;
            while let Some(parent_id) = cursor {
                if !visited.insert(parent_id) || parent_id == object.id {
                    return Err(ProjectValidationError::ContainmentCycle);
                }
                cursor = self.objects.get(&parent_id).and_then(|item| item.parent_id);
            }
        }
        for edge in &self.references {
            if !self.objects.contains_key(&edge.source_id)
                || edge.target_id.0 == Uuid::NIL
                || !edge.target_id.0.is_rfc9562_v4()
                || edge.source_location.is_empty()
                || edge.source_location.len() > 1024
                || edge.source_location.chars().any(char::is_control)
            {
                return Err(ProjectValidationError::InvalidReference);
            }
            let expected = self.objects.get(&edge.target_id).is_some_and(|target| {
                target.lifecycle == Lifecycle::Active && target.kind == edge.expected_target_kind
            });
            if expected != (edge.resolution == ResolutionState::Resolved) {
                return Err(ProjectValidationError::InvalidReference);
            }
        }
        for edge in &self.dependencies {
            if !self.objects.contains_key(&edge.source_id)
                || !self.objects.contains_key(&edge.target_id)
            {
                return Err(ProjectValidationError::InvalidDependency);
            }
        }
        if self
            .simulator_extensions
            .iter()
            .any(|(namespace, extension)| {
                namespace != extension.namespace() || !extension.is_valid()
            })
        {
            return Err(ProjectValidationError::InvalidExtension);
        }
        Ok(())
    }

    #[must_use]
    pub fn compare(&self, other: &Self) -> Vec<Comparison> {
        let ids: BTreeSet<_> = self
            .objects
            .keys()
            .chain(other.objects.keys())
            .copied()
            .collect();
        ids.into_iter()
            .map(|id| {
                let before = self.objects.get(&id);
                let after = other.objects.get(&id);
                let kind = match (before, after) {
                    (None, Some(_)) => ComparisonKind::Added,
                    (Some(_), None) => ComparisonKind::Removed,
                    (Some(left), Some(right)) if left.lifecycle != right.lifecycle => {
                        if right.lifecycle == Lifecycle::Tombstoned {
                            ComparisonKind::Removed
                        } else {
                            ComparisonKind::Added
                        }
                    }
                    (Some(left), Some(right)) if left.display_name != right.display_name => {
                        ComparisonKind::Renamed
                    }
                    (Some(left), Some(right)) if left.parent_id != right.parent_id => {
                        ComparisonKind::Moved
                    }
                    (Some(left), Some(right)) if left != right => ComparisonKind::Modified,
                    (Some(_), Some(_))
                        if other.references.iter().any(|edge| {
                            edge.resolution == ResolutionState::Unresolved
                                && (edge.source_id == id || edge.target_id == id)
                        }) =>
                    {
                        ComparisonKind::Unresolved
                    }
                    (Some(_), Some(_)) => ComparisonKind::Unchanged,
                    (None, None) => unreachable!("union contains an ID from at least one project"),
                };
                Comparison {
                    object_id: id,
                    kind,
                }
            })
            .collect()
    }

    #[must_use]
    pub fn invalidation_path(
        &self,
        changed: ObjectId,
        affected_reasons: &BTreeSet<DependencyReason>,
    ) -> BTreeMap<ObjectId, Vec<DependencyEdge>> {
        let mut paths = BTreeMap::from([(changed, Vec::new())]);
        let mut queue = vec![changed];
        let mut index = 0;
        while let Some(target) = queue.get(index).copied() {
            index += 1;
            for edge in self
                .dependencies
                .iter()
                .filter(|edge| edge.target_id == target && affected_reasons.contains(&edge.reason))
            {
                if paths.contains_key(&edge.source_id) {
                    continue;
                }
                let mut path = paths.get(&target).cloned().unwrap_or_default();
                path.push(edge.clone());
                paths.insert(edge.source_id, path);
                queue.push(edge.source_id);
            }
        }
        paths
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectValidationError {
    MissingRoot,
    InvalidIdentity,
    InvalidRevision,
    InvalidProfile,
    InvalidRoot,
    InvalidObject,
    Orphan,
    IllegalContainment,
    ContainmentCycle,
    InvalidReference,
    InvalidDependency,
    InvalidExtension,
}

fn valid_payload_map(values: &BTreeMap<String, PayloadValue>) -> bool {
    let mut count = 0_usize;
    values.len() <= 100_000
        && values
            .iter()
            .all(|(key, value)| valid_payload_key(key) && valid_payload_value(value, 0, &mut count))
}

fn valid_payload_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 128
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

fn valid_extension_namespace(namespace: &str) -> bool {
    namespace.len() <= 128
        && namespace.starts_with("edu.")
        && namespace.split('.').all(|segment| {
            !segment.is_empty()
                && segment.len() <= 63
                && segment
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_lowercase)
                && segment
                    .as_bytes()
                    .last()
                    .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
}

fn valid_payload_value(value: &PayloadValue, depth: usize, count: &mut usize) -> bool {
    if depth > 32 || *count >= 1_000_000 {
        return false;
    }
    *count += 1;
    match value {
        PayloadValue::String(value) => value.len() <= 1024 * 1024,
        PayloadValue::List(values) => {
            values.len() <= 100_000
                && values
                    .iter()
                    .all(|value| valid_payload_value(value, depth + 1, count))
        }
        PayloadValue::Record(values) => {
            values.len() <= 100_000
                && values.iter().all(|(key, value)| {
                    valid_payload_key(key) && valid_payload_value(value, depth + 1, count)
                })
        }
        PayloadValue::Null
        | PayloadValue::Bool(_)
        | PayloadValue::Signed(_)
        | PayloadValue::Unsigned(_) => true,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComparisonKind {
    Added,
    Removed,
    Renamed,
    Moved,
    Modified,
    Unresolved,
    Unchanged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Comparison {
    pub object_id: ObjectId,
    pub kind: ComparisonKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandContext {
    pub actor_id: String,
    pub can_mutate: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandEnvelope {
    pub command_id: Uuid,
    pub transaction_id: TransactionId,
    pub expected_document_revision: u64,
    pub expected_object_revisions: BTreeMap<ObjectId, u64>,
    pub context: CommandContext,
    pub command: DomainCommand,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DomainCommand {
    Create(NewObject),
    Rename {
        object_id: ObjectId,
        display_name: String,
    },
    Move {
        object_id: ObjectId,
        parent_id: ObjectId,
    },
    SetSemanticField {
        object_id: ObjectId,
        key: String,
        value: PayloadValue,
    },
    ReplaceSemanticPayload {
        object_id: ObjectId,
        semantic_payload: BTreeMap<String, PayloadValue>,
    },
    SetPresentationField {
        object_id: ObjectId,
        key: String,
        value: PayloadValue,
    },
    Delete {
        object_id: ObjectId,
    },
    CopyClosure {
        roots: Vec<ObjectId>,
        id_map: BTreeMap<ObjectId, ObjectId>,
        destination_parent: ObjectId,
    },
    AddReference(ReferenceEdge),
    RemoveReference(ReferenceEdge),
    AddDependency(DependencyEdge),
    RemoveDependency(DependencyEdge),
}

impl DomainCommand {
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Create(_) => "create",
            Self::Rename { .. } => "rename",
            Self::Move { .. } => "move",
            Self::SetSemanticField { .. } => "set-semantic-field",
            Self::ReplaceSemanticPayload { .. } => "replace-semantic-payload",
            Self::SetPresentationField { .. } => "set-presentation-field",
            Self::Delete { .. } => "delete",
            Self::CopyClosure { .. } => "copy-closure",
            Self::AddReference(_) => "add-reference",
            Self::RemoveReference(_) => "remove-reference",
            Self::AddDependency(_) => "add-dependency",
            Self::RemoveDependency(_) => "remove-dependency",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandOutcome {
    Committed,
    Rejected,
    Blocked,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub object_ids: Vec<ObjectId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DomainEvent {
    Created(ObjectId),
    Renamed(ObjectId),
    Moved(ObjectId),
    Changed(ObjectId),
    Deleted(ObjectId),
    Restored(ObjectId),
    Copied { source: ObjectId, copy: ObjectId },
    ReferenceChanged { source: ObjectId, target: ObjectId },
    DependencyChanged { source: ObjectId, target: ObjectId },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainCommandResult {
    pub outcome: CommandOutcome,
    pub transaction_id: TransactionId,
    pub affected_object_ids: Vec<ObjectId>,
    pub domain_events: Vec<DomainEvent>,
    pub diagnostics: Vec<Diagnostic>,
    pub undo_token: Option<UndoToken>,
    pub before_project_hash: Sha256Digest,
    pub after_project_hash: Option<Sha256Digest>,
}

pub(crate) fn project_to_json(project: &Project, include_revisions: bool) -> JsonValue {
    let objects = project
        .objects
        .values()
        .map(project_object_to_json)
        .collect();
    let references = project.references.iter().map(reference_to_json).collect();
    let dependencies = project
        .dependencies
        .iter()
        .map(dependency_to_json)
        .collect();
    let mut entries = BTreeMap::from([
        (
            "documentId".to_owned(),
            JsonValue::from(project.document_id.to_string()),
        ),
        (
            "rootId".to_owned(),
            JsonValue::from(project.root_id.to_string()),
        ),
        ("profile".to_owned(), profile_to_json(&project.profile)),
        (
            "nextCreationOrdinal".to_owned(),
            JsonValue::from(project.next_creation_ordinal.to_string()),
        ),
        ("objects".to_owned(), JsonValue::Array(objects)),
        ("references".to_owned(), JsonValue::Array(references)),
        ("dependencies".to_owned(), JsonValue::Array(dependencies)),
    ]);
    if include_revisions {
        entries.insert(
            "documentRevision".to_owned(),
            JsonValue::from(project.document_revision.to_string()),
        );
        entries.insert(
            "semanticRevision".to_owned(),
            JsonValue::from(project.semantic_revision.to_string()),
        );
    }
    JsonValue::Object(entries)
}

fn document_project_json(project: &Project) -> JsonValue {
    let JsonValue::Object(mut entries) = project_to_json(project, true) else {
        unreachable!("project serialization is always an object");
    };
    if !project.simulator_extensions.is_empty() {
        entries.insert(
            "simulatorExtensions".to_owned(),
            JsonValue::Array(
                project
                    .simulator_extensions
                    .values()
                    .map(|extension| {
                        JsonValue::object([
                            (
                                "namespace".to_owned(),
                                JsonValue::from(extension.namespace.clone()),
                            ),
                            (
                                "schemaVersion".to_owned(),
                                JsonValue::from(extension.schema_version),
                            ),
                            ("data".to_owned(), payload_value_to_json(&extension.data)),
                        ])
                    })
                    .collect(),
            ),
        );
    }
    JsonValue::Object(entries)
}

fn semantic_project_json(project: &Project) -> JsonValue {
    let objects = project
        .objects
        .values()
        .filter(|object| {
            !matches!(
                object.kind,
                ProjectObjectKind::Project
                    | ProjectObjectKind::Folder
                    | ProjectObjectKind::BuildRecord
                    | ProjectObjectKind::SnapshotReference
            )
        })
        .map(|object| {
            JsonValue::object([
                ("id".to_owned(), JsonValue::from(object.id.to_string())),
                ("kind".to_owned(), JsonValue::from(object.kind.as_str())),
                (
                    "parentId".to_owned(),
                    if object.kind.containment_is_semantic() {
                        semantic_parent_id(project, object)
                            .map_or(JsonValue::Null, |id| JsonValue::from(id.to_string()))
                    } else {
                        JsonValue::Null
                    },
                ),
                (
                    "displayName".to_owned(),
                    JsonValue::from(if object.kind.name_is_semantic() {
                        object.display_name.clone()
                    } else {
                        String::new()
                    }),
                ),
                (
                    "payloadSchema".to_owned(),
                    JsonValue::from(object.payload_schema.clone()),
                ),
                (
                    "semanticPayload".to_owned(),
                    payload_map_to_json(&object.payload.semantic),
                ),
                (
                    "lifecycle".to_owned(),
                    JsonValue::from(match object.lifecycle {
                        Lifecycle::Active => "active",
                        Lifecycle::Tombstoned => "tombstoned",
                    }),
                ),
            ])
        });
    JsonValue::object([
        (
            "rootId".to_owned(),
            JsonValue::from(project.root_id.to_string()),
        ),
        ("profile".to_owned(), profile_to_json(&project.profile)),
        ("objects".to_owned(), JsonValue::Array(objects.collect())),
        (
            "references".to_owned(),
            JsonValue::Array(project.references.iter().map(reference_to_json).collect()),
        ),
        (
            "dependencies".to_owned(),
            JsonValue::Array(
                project
                    .dependencies
                    .iter()
                    .map(dependency_to_json)
                    .collect(),
            ),
        ),
    ])
}

fn semantic_parent_id(project: &Project, object: &ProjectObject) -> Option<ObjectId> {
    let mut cursor = object.parent_id;
    while let Some(parent_id) = cursor {
        let parent = project.objects.get(&parent_id)?;
        if parent.kind != ProjectObjectKind::Folder {
            return Some(parent_id);
        }
        cursor = parent.parent_id;
    }
    None
}

pub(crate) fn project_from_json(value: &JsonValue) -> Result<Project, JsonError> {
    let object = value.as_object()?;
    require_only_fields(
        object,
        &[
            "documentId",
            "rootId",
            "profile",
            "documentRevision",
            "semanticRevision",
            "nextCreationOrdinal",
            "objects",
            "references",
            "dependencies",
        ],
    )?;
    let document_id = Uuid::parse(required(object, "documentId")?.as_str()?)
        .map_err(|_| JsonError::InvalidSyntax)?;
    let root_id = ObjectId(
        Uuid::parse(required(object, "rootId")?.as_str()?).map_err(|_| JsonError::InvalidSyntax)?,
    );
    let profile = profile_from_json(required(object, "profile")?)?;
    let document_revision = decimal_u64(required(object, "documentRevision")?)?;
    let semantic_revision = decimal_u64(required(object, "semanticRevision")?)?;
    let next_creation_ordinal = decimal_u64(required(object, "nextCreationOrdinal")?)?;
    let mut objects = BTreeMap::new();
    for item in required(object, "objects")?.as_array()? {
        let parsed = project_object_from_json(item)?;
        if objects.insert(parsed.id, parsed).is_some() {
            return Err(JsonError::DuplicateKey);
        }
    }
    let mut references = BTreeSet::new();
    for item in required(object, "references")?.as_array()? {
        if !references.insert(reference_from_json(item)?) {
            return Err(JsonError::DuplicateKey);
        }
    }
    let mut dependencies = BTreeSet::new();
    for item in required(object, "dependencies")?.as_array()? {
        if !dependencies.insert(dependency_from_json(item)?) {
            return Err(JsonError::DuplicateKey);
        }
    }
    Ok(Project {
        document_id,
        root_id,
        profile,
        document_revision,
        semantic_revision,
        next_creation_ordinal,
        objects,
        references,
        dependencies,
        simulator_extensions: BTreeMap::new(),
        saved_checkpoint: None,
    })
}

fn profile_to_json(profile: &ProfilePin) -> JsonValue {
    JsonValue::object([
        ("id".to_owned(), JsonValue::from(profile.id.clone())),
        (
            "version".to_owned(),
            JsonValue::from(profile.version.clone()),
        ),
        (
            "manifestHash".to_owned(),
            JsonValue::from(profile.manifest_hash.to_hex()),
        ),
    ])
}

fn profile_from_json(value: &JsonValue) -> Result<ProfilePin, JsonError> {
    let object = value.as_object()?;
    require_only_fields(object, &["id", "version", "manifestHash"])?;
    Ok(ProfilePin {
        id: required(object, "id")?.as_str()?.to_owned(),
        version: required(object, "version")?.as_str()?.to_owned(),
        manifest_hash: Sha256Digest::from_hex(required(object, "manifestHash")?.as_str()?)
            .map_err(|_| JsonError::InvalidSyntax)?,
    })
}

pub(crate) fn project_object_to_json(object: &ProjectObject) -> JsonValue {
    JsonValue::object([
        ("id".to_owned(), JsonValue::from(object.id.to_string())),
        ("kind".to_owned(), JsonValue::from(object.kind.as_str())),
        (
            "objectRevision".to_owned(),
            JsonValue::from(object.object_revision.to_string()),
        ),
        (
            "semanticRevision".to_owned(),
            JsonValue::from(object.semantic_revision.to_string()),
        ),
        (
            "creationOrdinal".to_owned(),
            JsonValue::from(object.creation_ordinal.to_string()),
        ),
        (
            "parentId".to_owned(),
            object
                .parent_id
                .map_or(JsonValue::Null, |id| JsonValue::from(id.to_string())),
        ),
        (
            "displayName".to_owned(),
            JsonValue::from(object.display_name.clone()),
        ),
        (
            "payloadSchema".to_owned(),
            JsonValue::from(object.payload_schema.clone()),
        ),
        (
            "semanticPayload".to_owned(),
            payload_map_to_json(&object.payload.semantic),
        ),
        (
            "presentationPayload".to_owned(),
            payload_map_to_json(&object.payload.presentation),
        ),
        (
            "lifecycle".to_owned(),
            JsonValue::from(match object.lifecycle {
                Lifecycle::Active => "active",
                Lifecycle::Tombstoned => "tombstoned",
            }),
        ),
    ])
}

fn project_object_from_json(value: &JsonValue) -> Result<ProjectObject, JsonError> {
    let object = value.as_object()?;
    require_only_fields(
        object,
        &[
            "id",
            "kind",
            "objectRevision",
            "semanticRevision",
            "creationOrdinal",
            "parentId",
            "displayName",
            "payloadSchema",
            "semanticPayload",
            "presentationPayload",
            "lifecycle",
        ],
    )?;
    let parent_id = match required(object, "parentId")? {
        JsonValue::Null => None,
        value => Some(ObjectId(
            Uuid::parse(value.as_str()?).map_err(|_| JsonError::InvalidSyntax)?,
        )),
    };
    Ok(ProjectObject {
        id: ObjectId(
            Uuid::parse(required(object, "id")?.as_str()?).map_err(|_| JsonError::InvalidSyntax)?,
        ),
        kind: ProjectObjectKind::parse(required(object, "kind")?.as_str()?)?,
        object_revision: decimal_u64(required(object, "objectRevision")?)?,
        semantic_revision: decimal_u64(required(object, "semanticRevision")?)?,
        creation_ordinal: decimal_u64(required(object, "creationOrdinal")?)?,
        parent_id,
        display_name: required(object, "displayName")?.as_str()?.to_owned(),
        payload_schema: required(object, "payloadSchema")?.as_str()?.to_owned(),
        payload: Payload {
            semantic: payload_map_from_json(required(object, "semanticPayload")?)?,
            presentation: payload_map_from_json(required(object, "presentationPayload")?)?,
        },
        lifecycle: match required(object, "lifecycle")?.as_str()? {
            "active" => Lifecycle::Active,
            "tombstoned" => Lifecycle::Tombstoned,
            _ => return Err(JsonError::InvalidSyntax),
        },
    })
}

fn payload_map_to_json(map: &BTreeMap<String, PayloadValue>) -> JsonValue {
    JsonValue::Object(
        map.iter()
            .map(|(key, value)| (key.clone(), payload_value_to_json(value)))
            .collect(),
    )
}

fn payload_map_from_json(value: &JsonValue) -> Result<BTreeMap<String, PayloadValue>, JsonError> {
    value
        .as_object()?
        .iter()
        .map(|(key, value)| Ok((key.clone(), payload_value_from_json(value)?)))
        .collect()
}

pub(crate) fn payload_value_to_json(value: &PayloadValue) -> JsonValue {
    match value {
        PayloadValue::Null => JsonValue::Null,
        PayloadValue::Bool(value) => JsonValue::Bool(*value),
        PayloadValue::Signed(value) => JsonValue::object([
            ("$type".to_owned(), JsonValue::from("i64")),
            ("value".to_owned(), JsonValue::from(value.to_string())),
        ]),
        PayloadValue::Unsigned(value) => JsonValue::object([
            ("$type".to_owned(), JsonValue::from("u64")),
            ("value".to_owned(), JsonValue::from(value.to_string())),
        ]),
        PayloadValue::String(value) => JsonValue::String(value.clone()),
        PayloadValue::List(values) => {
            JsonValue::Array(values.iter().map(payload_value_to_json).collect())
        }
        PayloadValue::Record(values) => JsonValue::object([
            ("$type".to_owned(), JsonValue::from("record")),
            (
                "value".to_owned(),
                JsonValue::Object(
                    values
                        .iter()
                        .map(|(key, value)| (key.clone(), payload_value_to_json(value)))
                        .collect(),
                ),
            ),
        ]),
    }
}

pub(crate) fn payload_value_from_json(value: &JsonValue) -> Result<PayloadValue, JsonError> {
    match value {
        JsonValue::Null => Ok(PayloadValue::Null),
        JsonValue::Bool(value) => Ok(PayloadValue::Bool(*value)),
        JsonValue::String(value) => Ok(PayloadValue::String(value.clone())),
        JsonValue::Array(values) => values
            .iter()
            .map(payload_value_from_json)
            .collect::<Result<_, _>>()
            .map(PayloadValue::List),
        JsonValue::Object(values)
            if values.len() == 2
                && values.contains_key("$type")
                && values.contains_key("value") =>
        {
            let kind = required(values, "$type")?.as_str()?;
            match kind {
                "i64" => {
                    let text = required(values, "value")?.as_str()?;
                    let parsed: i64 = text.parse().map_err(|_| JsonError::InvalidNumber)?;
                    if parsed.to_string() != text {
                        return Err(JsonError::InvalidNumber);
                    }
                    Ok(PayloadValue::Signed(parsed))
                }
                "u64" => {
                    let text = required(values, "value")?.as_str()?;
                    let parsed: u64 = text.parse().map_err(|_| JsonError::InvalidNumber)?;
                    if parsed.to_string() != text {
                        return Err(JsonError::InvalidNumber);
                    }
                    Ok(PayloadValue::Unsigned(parsed))
                }
                "record" => required(values, "value")?
                    .as_object()?
                    .iter()
                    .map(|(key, value)| Ok((key.clone(), payload_value_from_json(value)?)))
                    .collect::<Result<_, _>>()
                    .map(PayloadValue::Record),
                _ => Err(JsonError::InvalidSyntax),
            }
        }
        JsonValue::Object(values) => values
            .iter()
            .map(|(key, value)| Ok((key.clone(), payload_value_from_json(value)?)))
            .collect::<Result<_, _>>()
            .map(PayloadValue::Record),
        JsonValue::Number(_) => Err(JsonError::InvalidSyntax),
    }
}

fn reference_to_json(edge: &ReferenceEdge) -> JsonValue {
    JsonValue::object([
        (
            "sourceId".to_owned(),
            JsonValue::from(edge.source_id.to_string()),
        ),
        (
            "sourceLocation".to_owned(),
            JsonValue::from(edge.source_location.clone()),
        ),
        (
            "targetId".to_owned(),
            JsonValue::from(edge.target_id.to_string()),
        ),
        (
            "expectedTargetKind".to_owned(),
            JsonValue::from(edge.expected_target_kind.as_str()),
        ),
        ("kind".to_owned(), JsonValue::from(edge.kind.as_str())),
        (
            "resolution".to_owned(),
            JsonValue::from(match edge.resolution {
                ResolutionState::Resolved => "resolved",
                ResolutionState::Unresolved => "unresolved",
            }),
        ),
    ])
}

fn reference_from_json(value: &JsonValue) -> Result<ReferenceEdge, JsonError> {
    let object = value.as_object()?;
    require_only_fields(
        object,
        &[
            "sourceId",
            "sourceLocation",
            "targetId",
            "expectedTargetKind",
            "kind",
            "resolution",
        ],
    )?;
    Ok(ReferenceEdge {
        source_id: ObjectId(
            Uuid::parse(required(object, "sourceId")?.as_str()?)
                .map_err(|_| JsonError::InvalidSyntax)?,
        ),
        source_location: required(object, "sourceLocation")?.as_str()?.to_owned(),
        target_id: ObjectId(
            Uuid::parse(required(object, "targetId")?.as_str()?)
                .map_err(|_| JsonError::InvalidSyntax)?,
        ),
        expected_target_kind: ProjectObjectKind::parse(
            required(object, "expectedTargetKind")?.as_str()?,
        )?,
        kind: ReferenceKind::parse(required(object, "kind")?.as_str()?)?,
        resolution: match required(object, "resolution")?.as_str()? {
            "resolved" => ResolutionState::Resolved,
            "unresolved" => ResolutionState::Unresolved,
            _ => return Err(JsonError::InvalidSyntax),
        },
    })
}

fn dependency_to_json(edge: &DependencyEdge) -> JsonValue {
    JsonValue::object([
        (
            "sourceId".to_owned(),
            JsonValue::from(edge.source_id.to_string()),
        ),
        (
            "targetId".to_owned(),
            JsonValue::from(edge.target_id.to_string()),
        ),
        ("reason".to_owned(), JsonValue::from(edge.reason.as_str())),
    ])
}

fn dependency_from_json(value: &JsonValue) -> Result<DependencyEdge, JsonError> {
    let object = value.as_object()?;
    require_only_fields(object, &["sourceId", "targetId", "reason"])?;
    Ok(DependencyEdge {
        source_id: ObjectId(
            Uuid::parse(required(object, "sourceId")?.as_str()?)
                .map_err(|_| JsonError::InvalidSyntax)?,
        ),
        target_id: ObjectId(
            Uuid::parse(required(object, "targetId")?.as_str()?)
                .map_err(|_| JsonError::InvalidSyntax)?,
        ),
        reason: DependencyReason::parse(required(object, "reason")?.as_str()?)?,
    })
}

pub(crate) fn command_to_json(command: &DomainCommand) -> JsonValue {
    match command {
        DomainCommand::Create(spec) => JsonValue::object([
            ("kind".to_owned(), JsonValue::from("create")),
            ("id".to_owned(), JsonValue::from(spec.id.to_string())),
            ("objectKind".to_owned(), JsonValue::from(spec.kind.as_str())),
            (
                "parentId".to_owned(),
                JsonValue::from(spec.parent_id.to_string()),
            ),
            (
                "displayName".to_owned(),
                JsonValue::from(spec.display_name.clone()),
            ),
            (
                "payloadSchema".to_owned(),
                JsonValue::from(spec.payload_schema.clone()),
            ),
            (
                "semanticPayload".to_owned(),
                payload_map_to_json(&spec.payload.semantic),
            ),
            (
                "presentationPayload".to_owned(),
                payload_map_to_json(&spec.payload.presentation),
            ),
        ]),
        DomainCommand::Rename {
            object_id,
            display_name,
        } => JsonValue::object([
            ("kind".to_owned(), JsonValue::from("rename")),
            (
                "objectId".to_owned(),
                JsonValue::from(object_id.to_string()),
            ),
            (
                "displayName".to_owned(),
                JsonValue::from(display_name.clone()),
            ),
        ]),
        DomainCommand::Move {
            object_id,
            parent_id,
        } => JsonValue::object([
            ("kind".to_owned(), JsonValue::from("move")),
            (
                "objectId".to_owned(),
                JsonValue::from(object_id.to_string()),
            ),
            (
                "parentId".to_owned(),
                JsonValue::from(parent_id.to_string()),
            ),
        ]),
        DomainCommand::SetSemanticField {
            object_id,
            key,
            value,
        } => field_command_json("set-semantic-field", *object_id, key, value),
        DomainCommand::ReplaceSemanticPayload {
            object_id,
            semantic_payload,
        } => JsonValue::object([
            (
                "kind".to_owned(),
                JsonValue::from("replace-semantic-payload"),
            ),
            (
                "objectId".to_owned(),
                JsonValue::from(object_id.to_string()),
            ),
            (
                "semanticPayload".to_owned(),
                payload_map_to_json(semantic_payload),
            ),
        ]),
        DomainCommand::SetPresentationField {
            object_id,
            key,
            value,
        } => field_command_json("set-presentation-field", *object_id, key, value),
        DomainCommand::Delete { object_id } => JsonValue::object([
            ("kind".to_owned(), JsonValue::from("delete")),
            (
                "objectId".to_owned(),
                JsonValue::from(object_id.to_string()),
            ),
        ]),
        DomainCommand::CopyClosure {
            roots,
            id_map,
            destination_parent,
        } => JsonValue::object([
            ("kind".to_owned(), JsonValue::from("copy-closure")),
            (
                "roots".to_owned(),
                JsonValue::Array(
                    roots
                        .iter()
                        .map(|id| JsonValue::from(id.to_string()))
                        .collect(),
                ),
            ),
            (
                "idMap".to_owned(),
                JsonValue::Array(
                    id_map
                        .iter()
                        .map(|(source, target)| {
                            JsonValue::Array(vec![
                                JsonValue::from(source.to_string()),
                                JsonValue::from(target.to_string()),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "destinationParent".to_owned(),
                JsonValue::from(destination_parent.to_string()),
            ),
        ]),
        DomainCommand::AddReference(edge) => edge_command_json("add-reference", edge),
        DomainCommand::RemoveReference(edge) => edge_command_json("remove-reference", edge),
        DomainCommand::AddDependency(edge) => dependency_command_json("add-dependency", edge),
        DomainCommand::RemoveDependency(edge) => dependency_command_json("remove-dependency", edge),
    }
}

fn field_command_json(
    kind: &str,
    object_id: ObjectId,
    key: &str,
    value: &PayloadValue,
) -> JsonValue {
    JsonValue::object([
        ("kind".to_owned(), JsonValue::from(kind)),
        (
            "objectId".to_owned(),
            JsonValue::from(object_id.to_string()),
        ),
        ("key".to_owned(), JsonValue::from(key)),
        ("value".to_owned(), payload_value_to_json(value)),
    ])
}

fn edge_command_json(kind: &str, edge: &ReferenceEdge) -> JsonValue {
    JsonValue::object([
        ("kind".to_owned(), JsonValue::from(kind)),
        ("edge".to_owned(), reference_to_json(edge)),
    ])
}

fn dependency_command_json(kind: &str, edge: &DependencyEdge) -> JsonValue {
    JsonValue::object([
        ("kind".to_owned(), JsonValue::from(kind)),
        ("edge".to_owned(), dependency_to_json(edge)),
    ])
}

pub(crate) fn command_from_json(value: &JsonValue) -> Result<DomainCommand, JsonError> {
    let object = value.as_object()?;
    let kind = required(object, "kind")?.as_str()?;
    match kind {
        "create" => {
            require_only_fields(
                object,
                &[
                    "kind",
                    "id",
                    "objectKind",
                    "parentId",
                    "displayName",
                    "payloadSchema",
                    "semanticPayload",
                    "presentationPayload",
                ],
            )?;
            Ok(DomainCommand::Create(NewObject {
                id: parse_object_id(required(object, "id")?)?,
                kind: ProjectObjectKind::parse(required(object, "objectKind")?.as_str()?)?,
                parent_id: parse_object_id(required(object, "parentId")?)?,
                display_name: required(object, "displayName")?.as_str()?.to_owned(),
                payload_schema: required(object, "payloadSchema")?.as_str()?.to_owned(),
                payload: Payload {
                    semantic: payload_map_from_json(required(object, "semanticPayload")?)?,
                    presentation: payload_map_from_json(required(object, "presentationPayload")?)?,
                },
            }))
        }
        "rename" => {
            require_only_fields(object, &["kind", "objectId", "displayName"])?;
            Ok(DomainCommand::Rename {
                object_id: parse_object_id(required(object, "objectId")?)?,
                display_name: required(object, "displayName")?.as_str()?.to_owned(),
            })
        }
        "move" => {
            require_only_fields(object, &["kind", "objectId", "parentId"])?;
            Ok(DomainCommand::Move {
                object_id: parse_object_id(required(object, "objectId")?)?,
                parent_id: parse_object_id(required(object, "parentId")?)?,
            })
        }
        "set-semantic-field" | "set-presentation-field" => {
            require_only_fields(object, &["kind", "objectId", "key", "value"])?;
            let object_id = parse_object_id(required(object, "objectId")?)?;
            let key = required(object, "key")?.as_str()?.to_owned();
            let value = payload_value_from_json(required(object, "value")?)?;
            if kind == "set-semantic-field" {
                Ok(DomainCommand::SetSemanticField {
                    object_id,
                    key,
                    value,
                })
            } else {
                Ok(DomainCommand::SetPresentationField {
                    object_id,
                    key,
                    value,
                })
            }
        }
        "replace-semantic-payload" => {
            require_only_fields(object, &["kind", "objectId", "semanticPayload"])?;
            Ok(DomainCommand::ReplaceSemanticPayload {
                object_id: parse_object_id(required(object, "objectId")?)?,
                semantic_payload: payload_map_from_json(required(object, "semanticPayload")?)?,
            })
        }
        "delete" => {
            require_only_fields(object, &["kind", "objectId"])?;
            Ok(DomainCommand::Delete {
                object_id: parse_object_id(required(object, "objectId")?)?,
            })
        }
        "copy-closure" => {
            require_only_fields(object, &["kind", "roots", "idMap", "destinationParent"])?;
            let roots = required(object, "roots")?
                .as_array()?
                .iter()
                .map(parse_object_id)
                .collect::<Result<_, _>>()?;
            let mut id_map = BTreeMap::new();
            for pair in required(object, "idMap")?.as_array()? {
                let values = pair.as_array()?;
                if values.len() != 2 {
                    return Err(JsonError::InvalidSyntax);
                }
                if id_map
                    .insert(parse_object_id(&values[0])?, parse_object_id(&values[1])?)
                    .is_some()
                {
                    return Err(JsonError::DuplicateKey);
                }
            }
            Ok(DomainCommand::CopyClosure {
                roots,
                id_map,
                destination_parent: parse_object_id(required(object, "destinationParent")?)?,
            })
        }
        "add-reference" | "remove-reference" => {
            require_only_fields(object, &["kind", "edge"])?;
            let edge = reference_from_json(required(object, "edge")?)?;
            if kind == "add-reference" {
                Ok(DomainCommand::AddReference(edge))
            } else {
                Ok(DomainCommand::RemoveReference(edge))
            }
        }
        "add-dependency" | "remove-dependency" => {
            require_only_fields(object, &["kind", "edge"])?;
            let edge = dependency_from_json(required(object, "edge")?)?;
            if kind == "add-dependency" {
                Ok(DomainCommand::AddDependency(edge))
            } else {
                Ok(DomainCommand::RemoveDependency(edge))
            }
        }
        _ => Err(JsonError::InvalidSyntax),
    }
}

pub(crate) fn envelope_to_json(envelope: &CommandEnvelope) -> JsonValue {
    JsonValue::object([
        (
            "commandId".to_owned(),
            JsonValue::from(envelope.command_id.to_string()),
        ),
        (
            "transactionId".to_owned(),
            JsonValue::from(envelope.transaction_id.to_string()),
        ),
        (
            "expectedDocumentRevision".to_owned(),
            JsonValue::from(envelope.expected_document_revision.to_string()),
        ),
        (
            "expectedObjectRevisions".to_owned(),
            JsonValue::Array(
                envelope
                    .expected_object_revisions
                    .iter()
                    .map(|(id, revision)| {
                        JsonValue::Array(vec![
                            JsonValue::from(id.to_string()),
                            JsonValue::from(revision.to_string()),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "context".to_owned(),
            JsonValue::object([
                (
                    "actorId".to_owned(),
                    JsonValue::from(envelope.context.actor_id.clone()),
                ),
                (
                    "canMutate".to_owned(),
                    JsonValue::from(envelope.context.can_mutate),
                ),
            ]),
        ),
        ("command".to_owned(), command_to_json(&envelope.command)),
    ])
}

pub(crate) fn envelope_from_json(value: &JsonValue) -> Result<CommandEnvelope, JsonError> {
    let object = value.as_object()?;
    require_only_fields(
        object,
        &[
            "commandId",
            "transactionId",
            "expectedDocumentRevision",
            "expectedObjectRevisions",
            "context",
            "command",
        ],
    )?;
    let command_id = Uuid::parse(required(object, "commandId")?.as_str()?)
        .map_err(|_| JsonError::InvalidSyntax)?;
    let transaction_id = TransactionId(
        Uuid::parse(required(object, "transactionId")?.as_str()?)
            .map_err(|_| JsonError::InvalidSyntax)?,
    );
    let mut expected_object_revisions = BTreeMap::new();
    for pair in required(object, "expectedObjectRevisions")?.as_array()? {
        let pair = pair.as_array()?;
        if pair.len() != 2 {
            return Err(JsonError::InvalidSyntax);
        }
        let id = parse_object_id(&pair[0])?;
        if expected_object_revisions
            .insert(id, decimal_u64(&pair[1])?)
            .is_some()
        {
            return Err(JsonError::DuplicateKey);
        }
    }
    let context = required(object, "context")?.as_object()?;
    require_only_fields(context, &["actorId", "canMutate"])?;
    Ok(CommandEnvelope {
        command_id,
        transaction_id,
        expected_document_revision: decimal_u64(required(object, "expectedDocumentRevision")?)?,
        expected_object_revisions,
        context: CommandContext {
            actor_id: required(context, "actorId")?.as_str()?.to_owned(),
            can_mutate: required(context, "canMutate")?.as_bool()?,
        },
        command: command_from_json(required(object, "command")?)?,
    })
}

fn parse_object_id(value: &JsonValue) -> Result<ObjectId, JsonError> {
    Uuid::parse(value.as_str()?)
        .map(ObjectId)
        .map_err(|_| JsonError::InvalidSyntax)
}

fn decimal_u64(value: &JsonValue) -> Result<u64, JsonError> {
    let text = value.as_str()?;
    if text.is_empty()
        || (text.len() > 1 && text.starts_with('0'))
        || !text.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(JsonError::InvalidNumber);
    }
    text.parse().map_err(|_| JsonError::InvalidNumber)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{DomainCommand, ObjectId, PayloadValue, Uuid, command_from_json, command_to_json};
    use crate::json::canonical_json;

    #[test]
    fn deterministic_ids_are_v4_shaped_and_parse() {
        let id = Uuid::deterministic_v4(b"project-seed", 7);
        assert!(id.is_rfc9562_v4());
        assert_eq!(Uuid::parse(&id.to_string()), Ok(id));
        assert_ne!(
            ObjectId(id),
            ObjectId(Uuid::deterministic_v4(b"project-seed", 8))
        );
    }

    #[test]
    fn semantic_payload_replacement_has_stable_json_and_round_trips() {
        let object_id = ObjectId(Uuid::deterministic_v4(b"replace-payload", 1));
        let command = DomainCommand::ReplaceSemanticPayload {
            object_id,
            semantic_payload: BTreeMap::from([
                ("enabled".to_owned(), PayloadValue::Bool(true)),
                ("name".to_owned(), PayloadValue::from("Motor")),
            ]),
        };
        let json = command_to_json(&command);
        assert_eq!(
            canonical_json(&json),
            format!(
                "{{\"kind\":\"replace-semantic-payload\",\"objectId\":\"{object_id}\",\"semanticPayload\":{{\"enabled\":true,\"name\":\"Motor\"}}}}"
            )
            .as_bytes()
        );
        assert_eq!(command_from_json(&json), Ok(command));
    }
}
