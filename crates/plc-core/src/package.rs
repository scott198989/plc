#![allow(clippy::missing_errors_doc)]

//! Canonical, bounded, in-memory `.vlabproj` logical package support.
//!
//! The physical representation is deliberately small and deterministic. It is
//! not a ZIP file: entries are length-framed, uncompressed, sorted logical
//! paths followed by a SHA-256 trailer. Host shells may atomically replace a
//! destination file only after this module has encoded and decoded the bytes.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::hash::{Sha256Digest, sha256};
use crate::json::{
    JsonLimits, JsonValue, canonical_json, parse_json, require_only_fields, required,
};
use crate::model::{
    ObjectId, ProfilePin, Project, SimulatorExtension, Uuid, payload_value_from_json,
    payload_value_to_json, project_from_json, project_object_to_json, project_to_json,
};

const MAGIC: &[u8; 8] = b"VLABPKG1";
const CONTAINER_VERSION: u32 = 1;
const MANIFEST_PATH: &str = "manifest.json";
const PROJECT_PATH: &str = "project/project.json";
const PROFILE_PATH: &str = "profile/pin.json";
const MIGRATION_HISTORY_PATH: &str = "migration/history.json";
const DISPOSABLE_INDEX_PATH: &str = "index/manifest.json";
const ASSET_INDEX_PATH: &str = "assets/manifest.json";
const BUILD_RECORD_INDEX_PATH: &str = "build-records/manifest.json";
const SNAPSHOT_INDEX_PATH: &str = "snapshots/manifest.json";
const EXTENSION_PREFIX: &str = "extensions/simulator/";
const PACKAGE_KIND: &str = "plc-engineering-project";
const KERNEL_CAPABILITY: &str = "project-kernel-v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodeLimits {
    pub max_package_bytes: usize,
    pub max_entries: usize,
    pub max_entry_bytes: usize,
    pub max_total_entry_bytes: usize,
    /// The physical codec is intentionally uncompressed, so every admitted
    /// non-empty member has an expansion ratio of exactly one.
    pub max_expansion_ratio: usize,
    pub max_path_bytes: usize,
    pub max_image_bytes: usize,
    pub max_json_depth: usize,
    pub max_json_string_bytes: usize,
    pub max_json_collection_items: usize,
    pub max_json_values: usize,
    pub max_total_objects: usize,
}

impl Default for DecodeLimits {
    fn default() -> Self {
        Self {
            max_package_bytes: 64 * 1024 * 1024,
            max_entries: 100_000,
            max_entry_bytes: 32 * 1024 * 1024,
            max_total_entry_bytes: 48 * 1024 * 1024,
            max_expansion_ratio: 1,
            max_path_bytes: 512,
            max_image_bytes: 16 * 1024 * 1024,
            max_json_depth: 64,
            max_json_string_bytes: 1024 * 1024,
            max_json_collection_items: 100_000,
            max_json_values: 1_000_000,
            max_total_objects: 100_000,
        }
    }
}

impl DecodeLimits {
    fn json(self) -> JsonLimits {
        JsonLimits {
            max_depth: self.max_json_depth,
            max_string_bytes: self.max_json_string_bytes,
            max_collection_items: self.max_json_collection_items,
            max_total_values: self.max_json_values,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackageError {
    BadMagic,
    UnsupportedContainerVersion(u32),
    UnsupportedDocumentVersion(u32),
    UnsupportedCapability(String),
    Truncated,
    TrailingData,
    LimitExceeded(&'static str),
    InvalidPath(String),
    DuplicatePath(String),
    NonCanonicalOrder,
    ContainerIntegrityMismatch,
    InvalidJson(String),
    NonCanonicalJson(String),
    InvalidManifest,
    InventoryMismatch(String),
    ProjectMismatch(String),
    InvalidProject,
    InvalidExtension(String),
    IntegerOverflow,
}

impl fmt::Display for PackageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadMagic => formatter.write_str("invalid VLAB package magic"),
            Self::UnsupportedContainerVersion(version) => {
                write!(formatter, "unsupported container version {version}")
            }
            Self::UnsupportedDocumentVersion(version) => {
                write!(formatter, "unsupported document version {version}")
            }
            Self::UnsupportedCapability(capability) => {
                write!(formatter, "unsupported required capability {capability}")
            }
            Self::Truncated => formatter.write_str("truncated VLAB package"),
            Self::TrailingData => formatter.write_str("trailing package data"),
            Self::LimitExceeded(limit) => write!(formatter, "package limit exceeded: {limit}"),
            Self::InvalidPath(path) => write!(formatter, "unsafe logical path: {path}"),
            Self::DuplicatePath(path) => write!(formatter, "duplicate logical path: {path}"),
            Self::NonCanonicalOrder => formatter.write_str("entries are not canonically ordered"),
            Self::ContainerIntegrityMismatch => {
                formatter.write_str("container integrity digest does not match")
            }
            Self::InvalidJson(path) => write!(formatter, "invalid JSON entry: {path}"),
            Self::NonCanonicalJson(path) => write!(formatter, "non-canonical JSON entry: {path}"),
            Self::InvalidManifest => formatter.write_str("invalid package manifest"),
            Self::InventoryMismatch(path) => write!(formatter, "inventory mismatch: {path}"),
            Self::ProjectMismatch(path) => write!(formatter, "project entry mismatch: {path}"),
            Self::InvalidProject => {
                formatter.write_str("decoded project violates kernel invariants")
            }
            Self::InvalidExtension(namespace) => {
                write!(formatter, "invalid simulator extension: {namespace}")
            }
            Self::IntegerOverflow => formatter.write_str("package integer overflow"),
        }
    }
}

impl std::error::Error for PackageError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageInventoryEntry {
    pub path: String,
    pub media_type: String,
    pub schema_id: String,
    pub size: u64,
    pub sha256: Sha256Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub schema_version: u32,
    pub document_format_version: u32,
    pub minimum_reader_version: u32,
    pub canonicalization_version: u32,
    pub object_index_version: u32,
    pub package_kind: String,
    pub document_id: Uuid,
    pub root_object_id: ObjectId,
    pub profile: ProfilePin,
    pub creation_application_version: String,
    pub required_capabilities: Vec<String>,
    pub inventory: Vec<PackageInventoryEntry>,
    pub package_hash: Sha256Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManifestInspection {
    pub manifest: Manifest,
    pub editable: bool,
    pub compatibility_explanation: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LogicalPackage {
    entries: BTreeMap<String, Vec<u8>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArchiveReference {
    pub document_id: Uuid,
    pub package_hash: Sha256Digest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArchiveReceipt {
    pub reference: ArchiveReference,
    /// True means the archived bytes are the verified last-saved baseline and
    /// the current in-memory edits were deliberately not included.
    pub unsaved_changes_excluded: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectArchive {
    packages: BTreeMap<ArchiveReference, Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadOnlyProjectReference {
    project: Project,
    manifest: Manifest,
}

impl ReadOnlyProjectReference {
    #[must_use]
    pub const fn project(&self) -> &Project {
        &self.project
    }

    #[must_use]
    pub const fn manifest(&self) -> &Manifest {
        &self.manifest
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeImportPreview {
    Unsupported {
        code: &'static str,
        message: &'static str,
        source_digest: Sha256Digest,
    },
}

/// Phase 2 keeps native vendor import explicitly staged. Callers receive a
/// typed, deterministic preview outcome rather than a silent no-op.
#[must_use]
pub fn preview_native_import(source: &[u8]) -> NativeImportPreview {
    NativeImportPreview::Unsupported {
        code: "P2_NATIVE_IMPORT_UNSUPPORTED",
        message: "Native vendor-project import is reserved for a later implementation slice.",
        source_digest: sha256(source),
    }
}

impl ProjectArchive {
    pub fn archive_verified_package(
        &mut self,
        package: &[u8],
        limits: DecodeLimits,
    ) -> Result<ArchiveReference, PackageError> {
        let (_, manifest) = decode_project_package(package, limits)?;
        let reference = ArchiveReference {
            document_id: manifest.document_id,
            package_hash: manifest.package_hash,
        };
        if let Some(existing) = self.packages.get(&reference) {
            if existing != package {
                return Err(PackageError::InventoryMismatch(
                    "archive identity collision".to_owned(),
                ));
            }
        } else {
            self.packages.insert(reference, package.to_vec());
        }
        Ok(reference)
    }

    pub fn archive_last_saved(
        &mut self,
        live_project: &Project,
        last_saved_package: &[u8],
        limits: DecodeLimits,
    ) -> Result<ArchiveReceipt, PackageError> {
        let (saved_project, manifest) = decode_project_package(last_saved_package, limits)?;
        if live_project.document_id() != saved_project.document_id()
            || live_project.root_id() != saved_project.root_id()
            || live_project.saved_document_hash() != Some(manifest.package_hash)
        {
            return Err(PackageError::ProjectMismatch(
                "last-saved archive baseline".to_owned(),
            ));
        }
        let reference = self.archive_verified_package(last_saved_package, limits)?;
        Ok(ArchiveReceipt {
            reference,
            unsaved_changes_excluded: live_project.is_document_dirty(),
        })
    }

    #[must_use]
    pub fn retrieve(&self, reference: ArchiveReference) -> Option<&[u8]> {
        self.packages.get(&reference).map(Vec::as_slice)
    }

    pub fn open_read_only(
        &self,
        reference: ArchiveReference,
        limits: DecodeLimits,
    ) -> Result<Option<ReadOnlyProjectReference>, PackageError> {
        let Some(package) = self.packages.get(&reference) else {
            return Ok(None);
        };
        let (project, manifest) = decode_project_package(package, limits)?;
        Ok(Some(ReadOnlyProjectReference { project, manifest }))
    }

    pub fn retrieve_as(
        &self,
        reference: ArchiveReference,
        new_document_id: Uuid,
        limits: DecodeLimits,
    ) -> Result<Option<Project>, PackageError> {
        let Some(package) = self.packages.get(&reference) else {
            return Ok(None);
        };
        let (project, _) = decode_project_package(package, limits)?;
        let retrieved = project
            .for_save_as(new_document_id)
            .ok_or_else(|| PackageError::ProjectMismatch("retrieve DocumentId".to_owned()))?;
        Ok(Some(retrieved))
    }
}

impl LogicalPackage {
    pub fn new(entries: BTreeMap<String, Vec<u8>>) -> Result<Self, PackageError> {
        validate_entry_paths(&entries, DecodeLimits::default())?;
        Ok(Self { entries })
    }

    #[must_use]
    pub fn entries(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.entries
    }

    #[must_use]
    pub fn entry(&self, path: &str) -> Option<&[u8]> {
        self.entries.get(path).map(Vec::as_slice)
    }

    pub fn insert(&mut self, path: String, bytes: Vec<u8>) -> Result<(), PackageError> {
        validate_path(&path, DecodeLimits::default().max_path_bytes)?;
        if self
            .entries
            .keys()
            .any(|existing| existing.eq_ignore_ascii_case(&path))
        {
            return Err(PackageError::DuplicatePath(path));
        }
        self.entries.insert(path, bytes);
        Ok(())
    }

    pub fn encode(&self) -> Result<Vec<u8>, PackageError> {
        validate_entry_paths(&self.entries, DecodeLimits::default())?;
        let count = u32::try_from(self.entries.len()).map_err(|_| PackageError::IntegerOverflow)?;
        let mut output = Vec::new();
        output.extend_from_slice(MAGIC);
        output.extend_from_slice(&CONTAINER_VERSION.to_le_bytes());
        output.extend_from_slice(&count.to_le_bytes());
        for (path, bytes) in &self.entries {
            let path_len = u32::try_from(path.len()).map_err(|_| PackageError::IntegerOverflow)?;
            let data_len = u64::try_from(bytes.len()).map_err(|_| PackageError::IntegerOverflow)?;
            output.extend_from_slice(&path_len.to_le_bytes());
            output.extend_from_slice(&data_len.to_le_bytes());
            output.extend_from_slice(path.as_bytes());
            output.extend_from_slice(bytes);
        }
        let final_size = output
            .len()
            .checked_add(32)
            .ok_or(PackageError::IntegerOverflow)?;
        if final_size > DecodeLimits::default().max_package_bytes {
            return Err(PackageError::LimitExceeded("package bytes"));
        }
        output.extend_from_slice(&sha256(&output).0);
        Ok(output)
    }

    pub fn decode(input: &[u8], limits: DecodeLimits) -> Result<Self, PackageError> {
        if input.len() > limits.max_package_bytes {
            return Err(PackageError::LimitExceeded("package bytes"));
        }
        if input.len() < MAGIC.len() + 8 + 32 {
            return Err(PackageError::Truncated);
        }
        let payload_len = input.len().checked_sub(32).ok_or(PackageError::Truncated)?;
        let mut expected = [0_u8; 32];
        expected.copy_from_slice(&input[payload_len..]);
        if sha256(&input[..payload_len]) != Sha256Digest(expected) {
            return Err(PackageError::ContainerIntegrityMismatch);
        }
        let mut reader = Reader::new(&input[..payload_len]);
        if reader.take(MAGIC.len())? != MAGIC {
            return Err(PackageError::BadMagic);
        }
        let version = reader.u32()?;
        if version != CONTAINER_VERSION {
            return Err(PackageError::UnsupportedContainerVersion(version));
        }
        let count = usize::try_from(reader.u32()?).map_err(|_| PackageError::IntegerOverflow)?;
        if count > limits.max_entries {
            return Err(PackageError::LimitExceeded("entry count"));
        }
        let mut entries = BTreeMap::new();
        let mut folded = BTreeSet::new();
        let mut prior: Option<String> = None;
        let mut total = 0_usize;
        for _ in 0..count {
            let path_len =
                usize::try_from(reader.u32()?).map_err(|_| PackageError::IntegerOverflow)?;
            if path_len > limits.max_path_bytes {
                return Err(PackageError::LimitExceeded("path bytes"));
            }
            let data_len =
                usize::try_from(reader.u64()?).map_err(|_| PackageError::IntegerOverflow)?;
            if data_len > limits.max_entry_bytes {
                return Err(PackageError::LimitExceeded("entry bytes"));
            }
            if data_len > 0 && limits.max_expansion_ratio < 1 {
                return Err(PackageError::LimitExceeded("expansion ratio"));
            }
            total = total
                .checked_add(data_len)
                .ok_or(PackageError::IntegerOverflow)?;
            if total > limits.max_total_entry_bytes {
                return Err(PackageError::LimitExceeded("total entry bytes"));
            }
            let path = core::str::from_utf8(reader.take(path_len)?)
                .map_err(|_| PackageError::InvalidPath("non-UTF-8".to_owned()))?
                .to_owned();
            validate_path(&path, limits.max_path_bytes)?;
            if is_image_path(&path) && data_len > limits.max_image_bytes {
                return Err(PackageError::LimitExceeded("image bytes"));
            }
            if prior.as_ref().is_some_and(|value| value >= &path) {
                return Err(PackageError::NonCanonicalOrder);
            }
            if !folded.insert(path.to_ascii_lowercase()) {
                return Err(PackageError::DuplicatePath(path));
            }
            let bytes = reader.take(data_len)?.to_vec();
            entries.insert(path.clone(), bytes);
            prior = Some(path);
        }
        if !reader.is_empty() {
            return Err(PackageError::TrailingData);
        }
        Ok(Self { entries })
    }
}

pub fn encode_project_package(
    project: &Project,
    application_version: &str,
) -> Result<Vec<u8>, PackageError> {
    if application_version.is_empty() || application_version.len() > 256 {
        return Err(PackageError::InvalidManifest);
    }
    project
        .validate()
        .map_err(|_| PackageError::InvalidProject)?;
    let mut payload_entries = BTreeMap::new();
    payload_entries.insert(
        PROJECT_PATH.to_owned(),
        canonical_json(&project_to_json(project, true)),
    );
    payload_entries.insert(
        PROFILE_PATH.to_owned(),
        canonical_json(&profile_json(project.profile())),
    );
    payload_entries.insert(MIGRATION_HISTORY_PATH.to_owned(), b"[]".to_vec());
    payload_entries.insert(
        DISPOSABLE_INDEX_PATH.to_owned(),
        canonical_json(&disposable_index_json(project)),
    );
    payload_entries.insert(ASSET_INDEX_PATH.to_owned(), b"[]".to_vec());
    payload_entries.insert(BUILD_RECORD_INDEX_PATH.to_owned(), b"[]".to_vec());
    payload_entries.insert(SNAPSHOT_INDEX_PATH.to_owned(), b"[]".to_vec());
    for object in project.objects() {
        payload_entries.insert(
            format!("project/objects/{}.json", object.id),
            canonical_json(&project_object_to_json(object)),
        );
    }
    for extension in project.simulator_extensions() {
        payload_entries.insert(
            extension_path(extension.namespace()),
            canonical_json(&extension_json(extension)),
        );
    }
    let inventory = inventory_for(&payload_entries)?;
    let mut manifest = Manifest {
        schema_version: 1,
        document_format_version: 1,
        minimum_reader_version: 1,
        canonicalization_version: 1,
        object_index_version: 1,
        package_kind: PACKAGE_KIND.to_owned(),
        document_id: project.document_id(),
        root_object_id: project.root_id(),
        profile: project.profile().clone(),
        creation_application_version: application_version.to_owned(),
        required_capabilities: vec![KERNEL_CAPABILITY.to_owned()],
        inventory,
        package_hash: Sha256Digest([0; 32]),
    };
    manifest.package_hash = manifest_identity_hash(&manifest);
    let mut entries = payload_entries;
    entries.insert(
        MANIFEST_PATH.to_owned(),
        canonical_json(&manifest_json(&manifest)),
    );
    LogicalPackage::new(entries)?.encode()
}

pub fn decode_project_package(
    input: &[u8],
    limits: DecodeLimits,
) -> Result<(Project, Manifest), PackageError> {
    let logical = LogicalPackage::decode(input, limits)?;
    let manifest_bytes = logical
        .entry(MANIFEST_PATH)
        .ok_or(PackageError::InvalidManifest)?;
    let manifest_value = parse_canonical_json(MANIFEST_PATH, manifest_bytes, limits)?;
    let manifest = manifest_from_json(&manifest_value)?;
    validate_manifest(&manifest)?;

    let payload_entries: BTreeMap<_, _> = logical
        .entries
        .iter()
        .filter(|(path, _)| path.as_str() != MANIFEST_PATH)
        .map(|(path, bytes)| (path.clone(), bytes.clone()))
        .collect();
    let actual_inventory = inventory_for(&payload_entries)?;
    if manifest.inventory != actual_inventory {
        return Err(PackageError::InventoryMismatch(
            "manifest inventory".to_owned(),
        ));
    }
    if manifest.package_hash != manifest_identity_hash(&manifest) {
        return Err(PackageError::InventoryMismatch("package hash".to_owned()));
    }
    for (path, bytes) in &payload_entries {
        if is_json_path(path) {
            let _ = parse_canonical_json(path, bytes, limits)?;
        }
    }

    let project_bytes = logical
        .entry(PROJECT_PATH)
        .ok_or_else(|| PackageError::ProjectMismatch(PROJECT_PATH.to_owned()))?;
    let project_value = parse_canonical_json(PROJECT_PATH, project_bytes, limits)?;
    let project_object = project_value
        .as_object()
        .map_err(|_| PackageError::ProjectMismatch(PROJECT_PATH.to_owned()))?;
    let object_count = project_object
        .get("objects")
        .ok_or_else(|| PackageError::ProjectMismatch(PROJECT_PATH.to_owned()))?
        .as_array()
        .map_err(|_| PackageError::ProjectMismatch(PROJECT_PATH.to_owned()))?
        .len();
    if object_count > limits.max_total_objects {
        return Err(PackageError::LimitExceeded("total objects"));
    }
    let mut project = project_from_json(&project_value)
        .map_err(|_| PackageError::ProjectMismatch(PROJECT_PATH.to_owned()))?;
    if canonical_json(&project_to_json(&project, true)) != project_bytes {
        return Err(PackageError::NonCanonicalJson(PROJECT_PATH.to_owned()));
    }
    project.simulator_extensions = decode_simulator_extensions(&logical, limits)?;
    project
        .validate()
        .map_err(|_| PackageError::InvalidProject)?;
    if project.document_id() != manifest.document_id
        || project.root_id() != manifest.root_object_id
        || project.profile() != &manifest.profile
    {
        return Err(PackageError::ProjectMismatch(
            "manifest identity".to_owned(),
        ));
    }
    verify_profile_entry(&logical, &manifest, limits)?;
    verify_object_entries(&logical, &project)?;
    if logical.entry(MIGRATION_HISTORY_PATH) != Some(b"[]".as_slice()) {
        return Err(PackageError::ProjectMismatch(
            MIGRATION_HISTORY_PATH.to_owned(),
        ));
    }
    for path in [
        ASSET_INDEX_PATH,
        BUILD_RECORD_INDEX_PATH,
        SNAPSHOT_INDEX_PATH,
    ] {
        if logical.entry(path) != Some(b"[]".as_slice()) {
            return Err(PackageError::ProjectMismatch(path.to_owned()));
        }
    }
    // The serialized index is explicitly disposable: parsing it above proves
    // it is safe canonical JSON, but all authoritative indices are rebuilt by
    // `Project` from the verified object graph rather than trusting its claims.
    if logical.entry(DISPOSABLE_INDEX_PATH).is_none() {
        return Err(PackageError::ProjectMismatch(
            DISPOSABLE_INDEX_PATH.to_owned(),
        ));
    }
    project.mark_saved_verified(manifest.package_hash);
    Ok((project, manifest))
}

/// Verifies container integrity, closed-schema manifest syntax, inventory, and
/// logical package identity without interpreting project objects. This is the
/// only supported path for showing metadata from a newer incompatible schema.
pub fn inspect_package_manifest(
    input: &[u8],
    limits: DecodeLimits,
) -> Result<ManifestInspection, PackageError> {
    let logical = LogicalPackage::decode(input, limits)?;
    let manifest_bytes = logical
        .entry(MANIFEST_PATH)
        .ok_or(PackageError::InvalidManifest)?;
    let value = parse_canonical_json(MANIFEST_PATH, manifest_bytes, limits)?;
    let manifest = manifest_from_json(&value)?;
    validate_manifest_common(&manifest)?;
    let payload_entries: BTreeMap<_, _> = logical
        .entries
        .iter()
        .filter(|(path, _)| path.as_str() != MANIFEST_PATH)
        .map(|(path, bytes)| (path.clone(), bytes.clone()))
        .collect();
    if inventory_for(&payload_entries)? != manifest.inventory
        || manifest_identity_hash(&manifest) != manifest.package_hash
    {
        return Err(PackageError::InventoryMismatch(
            "manifest-only inspection".to_owned(),
        ));
    }
    let unsupported_capability = manifest
        .required_capabilities
        .iter()
        .find(|capability| capability.as_str() != KERNEL_CAPABILITY);
    let editable = manifest.document_format_version == 1
        && manifest.minimum_reader_version <= 1
        && unsupported_capability.is_none();
    let compatibility_explanation = if editable {
        None
    } else if manifest.document_format_version > 1 || manifest.minimum_reader_version > 1 {
        Some(format!(
            "Document schema {} requires a newer compatible reader.",
            manifest.document_format_version
        ))
    } else if let Some(capability) = unsupported_capability {
        Some(format!(
            "Required capability {capability} is not supported."
        ))
    } else {
        Some("Document schema is not editable by this reader.".to_owned())
    };
    Ok(ManifestInspection {
        manifest,
        editable,
        compatibility_explanation,
    })
}

fn verify_profile_entry(
    package: &LogicalPackage,
    manifest: &Manifest,
    limits: DecodeLimits,
) -> Result<(), PackageError> {
    let bytes = package
        .entry(PROFILE_PATH)
        .ok_or_else(|| PackageError::ProjectMismatch(PROFILE_PATH.to_owned()))?;
    let value = parse_canonical_json(PROFILE_PATH, bytes, limits)?;
    if value != profile_json(&manifest.profile) {
        return Err(PackageError::ProjectMismatch(PROFILE_PATH.to_owned()));
    }
    Ok(())
}

fn verify_object_entries(package: &LogicalPackage, project: &Project) -> Result<(), PackageError> {
    let expected: BTreeMap<_, _> = project
        .objects()
        .map(|object| {
            (
                format!("project/objects/{}.json", object.id),
                canonical_json(&project_object_to_json(object)),
            )
        })
        .collect();
    let actual: BTreeMap<_, _> = package
        .entries
        .iter()
        .filter(|(path, _)| path.starts_with("project/objects/"))
        .map(|(path, bytes)| (path.clone(), bytes.clone()))
        .collect();
    if actual != expected {
        return Err(PackageError::ProjectMismatch("project/objects".to_owned()));
    }
    Ok(())
}

fn validate_manifest(manifest: &Manifest) -> Result<(), PackageError> {
    validate_manifest_common(manifest)?;
    if manifest.document_format_version != 1 {
        return Err(PackageError::UnsupportedDocumentVersion(
            manifest.document_format_version,
        ));
    }
    for capability in &manifest.required_capabilities {
        if capability != KERNEL_CAPABILITY {
            return Err(PackageError::UnsupportedCapability(capability.clone()));
        }
    }
    if manifest.required_capabilities != vec![KERNEL_CAPABILITY.to_owned()] {
        return Err(PackageError::InvalidManifest);
    }
    Ok(())
}

fn validate_manifest_common(manifest: &Manifest) -> Result<(), PackageError> {
    if manifest.schema_version != 1
        || manifest.canonicalization_version != 1
        || manifest.object_index_version != 1
        || manifest.package_kind != PACKAGE_KIND
        || manifest.document_id == Uuid::NIL
        || manifest.root_object_id.0 == Uuid::NIL
        || manifest.creation_application_version.is_empty()
    {
        return Err(PackageError::InvalidManifest);
    }
    Ok(())
}

fn inventory_for(
    entries: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<PackageInventoryEntry>, PackageError> {
    entries
        .iter()
        .map(|(path, bytes)| {
            Ok(PackageInventoryEntry {
                path: path.clone(),
                media_type: "application/json".to_owned(),
                schema_id: schema_for_path(path)
                    .ok_or_else(|| PackageError::InventoryMismatch(path.clone()))?
                    .to_owned(),
                size: u64::try_from(bytes.len()).map_err(|_| PackageError::IntegerOverflow)?,
                sha256: sha256(bytes),
            })
        })
        .collect()
}

fn manifest_identity_hash(manifest: &Manifest) -> Sha256Digest {
    let JsonValue::Object(mut identity) = manifest_json(manifest) else {
        unreachable!("manifest serialization is always an object");
    };
    identity.remove("packageHash");
    sha256(&canonical_json(&JsonValue::Object(identity)))
}

fn schema_for_path(path: &str) -> Option<&'static str> {
    match path {
        PROJECT_PATH => Some("edu.project-aggregate/1"),
        PROFILE_PATH => Some("edu.training-profile-pin/1"),
        MIGRATION_HISTORY_PATH => Some("edu.migration-history/1"),
        DISPOSABLE_INDEX_PATH => Some("edu.derived-index-cache/1"),
        ASSET_INDEX_PATH => Some("edu.asset-index/1"),
        BUILD_RECORD_INDEX_PATH => Some("edu.build-record-index/1"),
        SNAPSHOT_INDEX_PATH => Some("edu.snapshot-index/1"),
        _ if path.starts_with("project/objects/") && is_json_path(path) => {
            Some("edu.project-object/1")
        }
        _ if extension_namespace_from_path(path).is_some() => Some("edu.simulator-extension/1"),
        _ => None,
    }
}

fn is_json_path(path: &str) -> bool {
    path.rsplit_once('.')
        .is_some_and(|(_, extension)| extension == "json")
}

fn is_image_path(path: &str) -> bool {
    path.starts_with("assets/images/")
        && path.rsplit_once('.').is_some_and(|(_, extension)| {
            matches!(extension, "gif" | "jpeg" | "jpg" | "png" | "webp")
        })
}

fn manifest_json(manifest: &Manifest) -> JsonValue {
    JsonValue::object([
        (
            "schemaVersion".to_owned(),
            JsonValue::from(manifest.schema_version),
        ),
        (
            "documentFormatVersion".to_owned(),
            JsonValue::from(manifest.document_format_version),
        ),
        (
            "minimumReaderVersion".to_owned(),
            JsonValue::from(manifest.minimum_reader_version),
        ),
        (
            "canonicalizationVersion".to_owned(),
            JsonValue::from(manifest.canonicalization_version),
        ),
        (
            "objectIndexVersion".to_owned(),
            JsonValue::from(manifest.object_index_version),
        ),
        (
            "packageKind".to_owned(),
            JsonValue::from(manifest.package_kind.clone()),
        ),
        (
            "documentId".to_owned(),
            JsonValue::from(manifest.document_id.to_string()),
        ),
        (
            "rootObjectId".to_owned(),
            JsonValue::from(manifest.root_object_id.to_string()),
        ),
        ("profile".to_owned(), profile_json(&manifest.profile)),
        (
            "creationApplicationVersion".to_owned(),
            JsonValue::from(manifest.creation_application_version.clone()),
        ),
        (
            "requiredCapabilities".to_owned(),
            JsonValue::Array(
                manifest
                    .required_capabilities
                    .iter()
                    .cloned()
                    .map(JsonValue::from)
                    .collect(),
            ),
        ),
        (
            "inventory".to_owned(),
            JsonValue::Array(manifest.inventory.iter().map(inventory_json).collect()),
        ),
        (
            "packageHash".to_owned(),
            JsonValue::from(manifest.package_hash.to_hex()),
        ),
    ])
}

fn manifest_from_json(value: &JsonValue) -> Result<Manifest, PackageError> {
    let object = value
        .as_object()
        .map_err(|_| PackageError::InvalidManifest)?;
    require_only_fields(
        object,
        &[
            "schemaVersion",
            "documentFormatVersion",
            "minimumReaderVersion",
            "canonicalizationVersion",
            "objectIndexVersion",
            "packageKind",
            "documentId",
            "rootObjectId",
            "profile",
            "creationApplicationVersion",
            "requiredCapabilities",
            "inventory",
            "packageHash",
        ],
    )
    .map_err(|_| PackageError::InvalidManifest)?;
    let parse_u32 = |name: &'static str| -> Result<u32, PackageError> {
        u32::try_from(
            required(object, name)
                .map_err(|_| PackageError::InvalidManifest)?
                .as_u64()
                .map_err(|_| PackageError::InvalidManifest)?,
        )
        .map_err(|_| PackageError::InvalidManifest)
    };
    let document_id = Uuid::parse(text_field(object, "documentId")?)
        .map_err(|_| PackageError::InvalidManifest)?;
    let root_object_id = ObjectId(
        Uuid::parse(text_field(object, "rootObjectId")?)
            .map_err(|_| PackageError::InvalidManifest)?,
    );
    let profile =
        profile_from_json(required(object, "profile").map_err(|_| PackageError::InvalidManifest)?)?;
    let required_capabilities = required(object, "requiredCapabilities")
        .map_err(|_| PackageError::InvalidManifest)?
        .as_array()
        .map_err(|_| PackageError::InvalidManifest)?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .map_err(|_| PackageError::InvalidManifest)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if required_capabilities
        .windows(2)
        .any(|pair| pair[0] >= pair[1])
    {
        return Err(PackageError::InvalidManifest);
    }
    let inventory = required(object, "inventory")
        .map_err(|_| PackageError::InvalidManifest)?
        .as_array()
        .map_err(|_| PackageError::InvalidManifest)?
        .iter()
        .map(inventory_from_json)
        .collect::<Result<Vec<_>, _>>()?;
    if inventory
        .windows(2)
        .any(|pair| pair[0].path >= pair[1].path)
    {
        return Err(PackageError::InvalidManifest);
    }
    Ok(Manifest {
        schema_version: parse_u32("schemaVersion")?,
        document_format_version: parse_u32("documentFormatVersion")?,
        minimum_reader_version: parse_u32("minimumReaderVersion")?,
        canonicalization_version: parse_u32("canonicalizationVersion")?,
        object_index_version: parse_u32("objectIndexVersion")?,
        package_kind: text_field(object, "packageKind")?.to_owned(),
        document_id,
        root_object_id,
        profile,
        creation_application_version: text_field(object, "creationApplicationVersion")?.to_owned(),
        required_capabilities,
        inventory,
        package_hash: digest_field(object, "packageHash")?,
    })
}

fn profile_json(profile: &ProfilePin) -> JsonValue {
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

fn disposable_index_json(project: &Project) -> JsonValue {
    JsonValue::object([
        ("disposable".to_owned(), JsonValue::from(true)),
        (
            "sourceDocumentHash".to_owned(),
            JsonValue::from(project.document_hash().to_hex()),
        ),
        (
            "objectCount".to_owned(),
            JsonValue::from(
                u64::try_from(project.objects().count())
                    .unwrap_or(u64::MAX)
                    .to_string(),
            ),
        ),
    ])
}

fn extension_path(namespace: &str) -> String {
    format!("{EXTENSION_PREFIX}{namespace}.json")
}

fn extension_namespace_from_path(path: &str) -> Option<&str> {
    path.strip_prefix(EXTENSION_PREFIX)
        .and_then(|tail| tail.strip_suffix(".json"))
        .filter(|namespace| !namespace.is_empty() && !namespace.contains('/'))
}

fn extension_json(extension: &SimulatorExtension) -> JsonValue {
    JsonValue::object([
        (
            "namespace".to_owned(),
            JsonValue::from(extension.namespace().to_owned()),
        ),
        (
            "schemaVersion".to_owned(),
            JsonValue::from(extension.schema_version()),
        ),
        ("data".to_owned(), payload_value_to_json(extension.data())),
    ])
}

fn decode_simulator_extensions(
    package: &LogicalPackage,
    limits: DecodeLimits,
) -> Result<BTreeMap<String, SimulatorExtension>, PackageError> {
    let mut extensions = BTreeMap::new();
    for (path, bytes) in package
        .entries()
        .iter()
        .filter(|(path, _)| path.starts_with(EXTENSION_PREFIX))
    {
        let path_namespace = extension_namespace_from_path(path)
            .ok_or_else(|| PackageError::InvalidExtension(path.clone()))?;
        let value = parse_canonical_json(path, bytes, limits)?;
        let object = value
            .as_object()
            .map_err(|_| PackageError::InvalidExtension(path_namespace.to_owned()))?;
        require_only_fields(object, &["namespace", "schemaVersion", "data"])
            .map_err(|_| PackageError::InvalidExtension(path_namespace.to_owned()))?;
        let namespace = required(object, "namespace")
            .and_then(JsonValue::as_str)
            .map_err(|_| PackageError::InvalidExtension(path_namespace.to_owned()))?;
        let schema_version = required(object, "schemaVersion")
            .and_then(JsonValue::as_u64)
            .ok()
            .and_then(|version| u32::try_from(version).ok())
            .ok_or_else(|| PackageError::InvalidExtension(path_namespace.to_owned()))?;
        if namespace != path_namespace {
            return Err(PackageError::InvalidExtension(path_namespace.to_owned()));
        }
        let data = payload_value_from_json(
            required(object, "data")
                .map_err(|_| PackageError::InvalidExtension(path_namespace.to_owned()))?,
        )
        .map_err(|_| PackageError::InvalidExtension(path_namespace.to_owned()))?;
        let extension = SimulatorExtension::new(namespace, schema_version, data)
            .map_err(|_| PackageError::InvalidExtension(path_namespace.to_owned()))?;
        if extensions.insert(namespace.to_owned(), extension).is_some() {
            return Err(PackageError::InvalidExtension(namespace.to_owned()));
        }
    }
    Ok(extensions)
}

fn profile_from_json(value: &JsonValue) -> Result<ProfilePin, PackageError> {
    let object = value
        .as_object()
        .map_err(|_| PackageError::InvalidManifest)?;
    require_only_fields(object, &["id", "version", "manifestHash"])
        .map_err(|_| PackageError::InvalidManifest)?;
    Ok(ProfilePin {
        id: text_field(object, "id")?.to_owned(),
        version: text_field(object, "version")?.to_owned(),
        manifest_hash: digest_field(object, "manifestHash")?,
    })
}

fn inventory_json(entry: &PackageInventoryEntry) -> JsonValue {
    JsonValue::object([
        ("path".to_owned(), JsonValue::from(entry.path.clone())),
        (
            "mediaType".to_owned(),
            JsonValue::from(entry.media_type.clone()),
        ),
        (
            "schemaId".to_owned(),
            JsonValue::from(entry.schema_id.clone()),
        ),
        ("size".to_owned(), JsonValue::from(entry.size.to_string())),
        ("sha256".to_owned(), JsonValue::from(entry.sha256.to_hex())),
    ])
}

fn inventory_from_json(value: &JsonValue) -> Result<PackageInventoryEntry, PackageError> {
    let object = value
        .as_object()
        .map_err(|_| PackageError::InvalidManifest)?;
    require_only_fields(object, &["path", "mediaType", "schemaId", "size", "sha256"])
        .map_err(|_| PackageError::InvalidManifest)?;
    let path = text_field(object, "path")?.to_owned();
    validate_path(&path, DecodeLimits::default().max_path_bytes)?;
    let size_text = text_field(object, "size")?;
    if size_text.is_empty()
        || (size_text.len() > 1 && size_text.starts_with('0'))
        || !size_text.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(PackageError::InvalidManifest);
    }
    Ok(PackageInventoryEntry {
        path,
        media_type: text_field(object, "mediaType")?.to_owned(),
        schema_id: text_field(object, "schemaId")?.to_owned(),
        size: size_text
            .parse()
            .map_err(|_| PackageError::InvalidManifest)?,
        sha256: digest_field(object, "sha256")?,
    })
}

fn text_field<'a>(
    object: &'a BTreeMap<String, JsonValue>,
    name: &'static str,
) -> Result<&'a str, PackageError> {
    required(object, name)
        .map_err(|_| PackageError::InvalidManifest)?
        .as_str()
        .map_err(|_| PackageError::InvalidManifest)
}

fn digest_field(
    object: &BTreeMap<String, JsonValue>,
    name: &'static str,
) -> Result<Sha256Digest, PackageError> {
    let source = text_field(object, name)?;
    let digest = Sha256Digest::from_hex(source).map_err(|_| PackageError::InvalidManifest)?;
    if digest.to_hex() != source {
        return Err(PackageError::InvalidManifest);
    }
    Ok(digest)
}

fn parse_canonical_json(
    path: &str,
    bytes: &[u8],
    limits: DecodeLimits,
) -> Result<JsonValue, PackageError> {
    let value =
        parse_json(bytes, limits.json()).map_err(|_| PackageError::InvalidJson(path.to_owned()))?;
    if canonical_json(&value) != bytes {
        return Err(PackageError::NonCanonicalJson(path.to_owned()));
    }
    Ok(value)
}

fn validate_entry_paths(
    entries: &BTreeMap<String, Vec<u8>>,
    limits: DecodeLimits,
) -> Result<(), PackageError> {
    if entries.len() > limits.max_entries {
        return Err(PackageError::LimitExceeded("entry count"));
    }
    let mut folded = BTreeSet::new();
    let mut total = 0_usize;
    for (path, bytes) in entries {
        validate_path(path, limits.max_path_bytes)?;
        if bytes.len() > limits.max_entry_bytes {
            return Err(PackageError::LimitExceeded("entry bytes"));
        }
        if !bytes.is_empty() && limits.max_expansion_ratio < 1 {
            return Err(PackageError::LimitExceeded("expansion ratio"));
        }
        if is_image_path(path) && bytes.len() > limits.max_image_bytes {
            return Err(PackageError::LimitExceeded("image bytes"));
        }
        total = total
            .checked_add(bytes.len())
            .ok_or(PackageError::IntegerOverflow)?;
        if total > limits.max_total_entry_bytes {
            return Err(PackageError::LimitExceeded("total entry bytes"));
        }
        if !folded.insert(path.to_ascii_lowercase()) {
            return Err(PackageError::DuplicatePath(path.clone()));
        }
    }
    Ok(())
}

fn validate_path(path: &str, max_path_bytes: usize) -> Result<(), PackageError> {
    if path.is_empty()
        || path.len() > max_path_bytes
        || !path.is_ascii()
        || path.starts_with('/')
        || path.ends_with('/')
        || path.contains('\\')
        || path.contains(':')
        || path.contains('\0')
    {
        return Err(PackageError::InvalidPath(path.to_owned()));
    }
    for segment in path.split('/') {
        let folded = segment.trim_end_matches(['.', ' ']).to_ascii_lowercase();
        let stem = folded.split('.').next().unwrap_or_default();
        let reserved = matches!(stem, "con" | "prn" | "aux" | "nul")
            || (stem.len() == 4
                && (stem.starts_with("com") || stem.starts_with("lpt"))
                && stem.as_bytes()[3].is_ascii_digit()
                && stem.as_bytes()[3] != b'0');
        if segment.is_empty()
            || matches!(segment, "." | "..")
            || segment.len() > 255
            || folded.len() != segment.len()
            || reserved
            || segment.chars().any(char::is_control)
        {
            return Err(PackageError::InvalidPath(path.to_owned()));
        }
    }
    Ok(())
}

struct Reader<'a> {
    bytes: &'a [u8],
    index: usize,
}

impl<'a> Reader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, index: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], PackageError> {
        let end = self
            .index
            .checked_add(count)
            .ok_or(PackageError::IntegerOverflow)?;
        let value = self
            .bytes
            .get(self.index..end)
            .ok_or(PackageError::Truncated)?;
        self.index = end;
        Ok(value)
    }

    fn u32(&mut self) -> Result<u32, PackageError> {
        let mut bytes = [0_u8; 4];
        bytes.copy_from_slice(self.take(4)?);
        Ok(u32::from_le_bytes(bytes))
    }

    fn u64(&mut self) -> Result<u64, PackageError> {
        let mut bytes = [0_u8; 8];
        bytes.copy_from_slice(self.take(8)?);
        Ok(u64::from_le_bytes(bytes))
    }

    const fn is_empty(&self) -> bool {
        self.index == self.bytes.len()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::{
        CommandContext, CommandEnvelope, CommandOutcome, DomainCommand, Engine, ObjectId,
        ProfilePin, Project, Sha256Digest, TransactionId, Uuid,
    };

    use super::{
        DISPOSABLE_INDEX_PATH, DecodeLimits, LogicalPackage, MANIFEST_PATH, PackageError,
        ProjectArchive, canonical_json, decode_project_package, encode_project_package,
        inspect_package_manifest, inventory_for, manifest_from_json, manifest_identity_hash,
        manifest_json, parse_json,
    };

    fn fixture() -> Project {
        Project::new(
            Uuid::deterministic_v4(b"package-document", 1),
            ObjectId(Uuid::deterministic_v4(b"package-root", 1)),
            "Package",
            ProfilePin {
                id: "EDU-21".to_owned(),
                version: "1.0".to_owned(),
                manifest_hash: Sha256Digest([3; 32]),
            },
        )
    }

    #[test]
    fn container_is_deterministic_and_rejects_corruption() {
        let package = LogicalPackage::new(BTreeMap::from([
            ("b/data.json".to_owned(), b"{}".to_vec()),
            ("a/data.json".to_owned(), b"[]".to_vec()),
        ]))
        .expect("valid package");
        let first = package.encode().expect("encode");
        let second = package.encode().expect("encode");
        assert_eq!(first, second);
        assert_eq!(
            LogicalPackage::decode(&first, DecodeLimits::default()),
            Ok(package)
        );
        let mut corrupt = first;
        corrupt[20] ^= 0x40;
        assert_eq!(
            LogicalPackage::decode(&corrupt, DecodeLimits::default()),
            Err(PackageError::ContainerIntegrityMismatch)
        );
    }

    #[test]
    fn rejects_traversal_and_case_collisions() {
        assert!(matches!(
            LogicalPackage::new(BTreeMap::from([("../escape".to_owned(), Vec::new())])),
            Err(PackageError::InvalidPath(_))
        ));
        assert!(matches!(
            LogicalPackage::new(BTreeMap::from([
                ("A/file".to_owned(), Vec::new()),
                ("a/file".to_owned(), Vec::new()),
            ])),
            Err(PackageError::DuplicatePath(_))
        ));
    }

    #[test]
    fn disposable_cache_is_never_an_authoritative_claim() {
        let project = fixture();
        let bytes = encode_project_package(&project, "test/1").expect("package");
        let logical = LogicalPackage::decode(&bytes, DecodeLimits::default()).expect("logical");
        let mut entries = logical.entries().clone();
        entries.insert(
            DISPOSABLE_INDEX_PATH.to_owned(),
            br#"{"disposable":true,"objectCount":"999999","sourceDocumentHash":"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"}"#
                .to_vec(),
        );
        let manifest_value = parse_json(
            entries.get(MANIFEST_PATH).expect("manifest"),
            DecodeLimits::default().json(),
        )
        .expect("manifest JSON");
        let mut manifest = manifest_from_json(&manifest_value).expect("manifest model");
        let payload: BTreeMap<_, _> = entries
            .iter()
            .filter(|(path, _)| path.as_str() != MANIFEST_PATH)
            .map(|(path, value)| (path.clone(), value.clone()))
            .collect();
        manifest.inventory = inventory_for(&payload).expect("inventory");
        manifest.package_hash = manifest_identity_hash(&manifest);
        entries.insert(
            MANIFEST_PATH.to_owned(),
            canonical_json(&manifest_json(&manifest)),
        );
        let with_false_cache = LogicalPackage::new(entries)
            .expect("logical")
            .encode()
            .expect("encode");
        let (decoded, _) = decode_project_package(&with_false_cache, DecodeLimits::default())
            .expect("cache is discarded and graph indexes are rebuilt");
        assert_eq!(decoded.document_hash(), project.document_hash());
        assert_eq!(
            decoded.rebuild_indexes().source_document_hash,
            project.document_hash()
        );
    }

    #[test]
    fn undeclared_member_fails_before_project_interpretation() {
        let bytes = encode_project_package(&fixture(), "test/1").expect("package");
        let logical = LogicalPackage::decode(&bytes, DecodeLimits::default()).expect("logical");
        let mut entries = logical.entries().clone();
        entries.insert("unexpected/payload.json".to_owned(), b"{}".to_vec());
        let attack = LogicalPackage::new(entries)
            .expect("safe physical paths")
            .encode()
            .expect("physical encoding");
        assert!(matches!(
            decode_project_package(&attack, DecodeLimits::default()),
            Err(PackageError::InventoryMismatch(_))
        ));
    }

    #[test]
    fn newer_schema_allows_verified_metadata_inspection_but_not_editable_open() {
        let bytes = encode_project_package(&fixture(), "future/1").expect("package");
        let logical = LogicalPackage::decode(&bytes, DecodeLimits::default()).expect("logical");
        let mut entries = logical.entries().clone();
        let manifest_value = parse_json(
            entries.get(MANIFEST_PATH).expect("manifest"),
            DecodeLimits::default().json(),
        )
        .expect("manifest JSON");
        let mut manifest = manifest_from_json(&manifest_value).expect("manifest model");
        manifest.document_format_version = 2;
        manifest.minimum_reader_version = 2;
        manifest.package_hash = manifest_identity_hash(&manifest);
        entries.insert(
            MANIFEST_PATH.to_owned(),
            canonical_json(&manifest_json(&manifest)),
        );
        let future = LogicalPackage::new(entries)
            .expect("logical")
            .encode()
            .expect("future package");
        assert_eq!(
            decode_project_package(&future, DecodeLimits::default()),
            Err(PackageError::UnsupportedDocumentVersion(2))
        );
        let inspection = inspect_package_manifest(&future, DecodeLimits::default())
            .expect("verified metadata inspection");
        assert!(!inspection.editable);
        assert_eq!(inspection.manifest.document_id, fixture().document_id());
        assert!(inspection.compatibility_explanation.is_some());
    }

    #[test]
    fn archive_excludes_unsaved_edits_and_retrieve_assigns_only_document_id() {
        let package = encode_project_package(&fixture(), "archive/1").expect("package");
        let (opened, manifest) =
            decode_project_package(&package, DecodeLimits::default()).expect("open");
        let root = opened.root_id();
        let original_ids: Vec<_> = opened.objects().map(|object| object.id).collect();
        let mut engine = Engine::new(opened).expect("engine");
        let rename = CommandEnvelope {
            command_id: Uuid::deterministic_v4(b"archive-command", 1),
            transaction_id: TransactionId(Uuid::deterministic_v4(b"archive-transaction", 1)),
            expected_document_revision: engine.project().document_revision(),
            expected_object_revisions: BTreeMap::from([(
                root,
                engine.project().object(root).expect("root").object_revision,
            )]),
            context: CommandContext {
                actor_id: "archive-test".to_owned(),
                can_mutate: true,
            },
            command: DomainCommand::Rename {
                object_id: root,
                display_name: "Unsaved rename".to_owned(),
            },
        };
        assert_eq!(engine.execute(&rename).outcome, CommandOutcome::Committed);
        assert!(engine.project().is_document_dirty());

        let mut archive = ProjectArchive::default();
        let receipt = archive
            .archive_last_saved(engine.project(), &package, DecodeLimits::default())
            .expect("archive last saved");
        assert!(receipt.unsaved_changes_excluded);
        assert_eq!(receipt.reference.package_hash, manifest.package_hash);
        let reference = archive
            .open_read_only(receipt.reference, DecodeLimits::default())
            .expect("read-only")
            .expect("archived package");
        assert_eq!(
            reference.project().object(root).expect("root").display_name,
            "Package"
        );

        let new_document_id = Uuid::deterministic_v4(b"archive-retrieve", 1);
        let retrieved = archive
            .retrieve_as(receipt.reference, new_document_id, DecodeLimits::default())
            .expect("retrieve")
            .expect("archived package");
        assert_eq!(retrieved.document_id(), new_document_id);
        assert_eq!(retrieved.root_id(), root);
        assert_eq!(
            retrieved
                .objects()
                .map(|object| object.id)
                .collect::<Vec<_>>(),
            original_ids
        );
    }
}
