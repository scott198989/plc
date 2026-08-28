#![allow(clippy::missing_errors_doc)]

//! Canonical byte-oriented adapter for thin native or WASM shells.

use std::collections::BTreeMap;
use std::fmt;

use crate::engine::{Engine, EngineError};
use crate::hash::Sha256Digest;
use crate::json::{
    JsonLimits, JsonValue, canonical_json, parse_json, require_only_fields, required,
};
use crate::model::{
    CommandOutcome, Diagnostic, DomainCommandResult, DomainEvent, ObjectId, ProfilePin, Project,
    TransactionId, UndoToken, Uuid, envelope_from_json, envelope_to_json, project_to_json,
};
use crate::package::{
    DecodeLimits, Manifest, PackageError, decode_project_package, encode_project_package,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProtocolError {
    InvalidRequest,
    NonCanonicalRequest,
    UnsupportedVersion(u32),
    UnsupportedOperation(String),
    Engine(EngineError),
    Package(PackageError),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest => formatter.write_str("invalid kernel protocol request"),
            Self::NonCanonicalRequest => formatter.write_str("request is not canonical JSON"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported kernel protocol version {version}")
            }
            Self::UnsupportedOperation(operation) => {
                write!(formatter, "unsupported kernel operation {operation}")
            }
            Self::Engine(error) => write!(formatter, "kernel engine error: {error:?}"),
            Self::Package(error) => write!(formatter, "kernel package error: {error}"),
        }
    }
}

impl std::error::Error for ProtocolError {}

impl From<EngineError> for ProtocolError {
    fn from(value: EngineError) -> Self {
        Self::Engine(value)
    }
}

impl From<PackageError> for ProtocolError {
    fn from(value: PackageError) -> Self {
        Self::Package(value)
    }
}

/// Stateful adapter whose public boundary consists only of owned bytes and
/// deterministic model values. It does not invoke a clock, filesystem, network,
/// device, entropy source, or process API.
#[derive(Clone, Debug)]
pub struct KernelSession {
    engine: Engine,
}

impl KernelSession {
    pub fn from_project(project: Project) -> Result<Self, ProtocolError> {
        Ok(Self {
            engine: Engine::new(project)?,
        })
    }

    /// Creates a session from canonical JSON. UUIDs and the pinned profile hash
    /// are caller-supplied so creation is byte-for-byte replayable.
    pub fn create(request: &[u8]) -> Result<Self, ProtocolError> {
        let value = parse_request(request)?;
        let object = value
            .as_object()
            .map_err(|_| ProtocolError::InvalidRequest)?;
        require_only_fields(
            object,
            &[
                "schemaVersion",
                "documentId",
                "rootId",
                "displayName",
                "profile",
            ],
        )
        .map_err(|_| ProtocolError::InvalidRequest)?;
        check_version(object)?;
        let profile_value =
            required(object, "profile").map_err(|_| ProtocolError::InvalidRequest)?;
        let profile_object = profile_value
            .as_object()
            .map_err(|_| ProtocolError::InvalidRequest)?;
        require_only_fields(profile_object, &["id", "version", "manifestHash"])
            .map_err(|_| ProtocolError::InvalidRequest)?;
        let manifest_hash_source = text(profile_object, "manifestHash")?;
        let manifest_hash = Sha256Digest::from_hex(manifest_hash_source)
            .map_err(|_| ProtocolError::InvalidRequest)?;
        if manifest_hash.to_hex() != manifest_hash_source {
            return Err(ProtocolError::NonCanonicalRequest);
        }
        let project = Project::new(
            parse_uuid(object, "documentId")?,
            ObjectId(parse_uuid(object, "rootId")?),
            text(object, "displayName")?,
            ProfilePin {
                id: text(profile_object, "id")?.to_owned(),
                version: text(profile_object, "version")?.to_owned(),
                manifest_hash,
            },
        );
        Self::from_project(project)
    }

    pub fn open(package: &[u8], limits: DecodeLimits) -> Result<(Self, Manifest), ProtocolError> {
        let (project, manifest) = decode_project_package(package, limits)?;
        Ok((Self::from_project(project)?, manifest))
    }

    #[must_use]
    pub fn project(&self) -> &Project {
        self.engine.project()
    }

    /// Executes one canonical JSON request. Supported operations are
    /// `query-project`, `execute`, `undo`, and `redo`.
    pub fn handle(&mut self, request: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        let value = parse_request(request)?;
        let object = value
            .as_object()
            .map_err(|_| ProtocolError::InvalidRequest)?;
        check_version(object)?;
        let operation = text(object, "operation")?;
        match operation {
            "query-project" => {
                require_only_fields(object, &["schemaVersion", "operation"])
                    .map_err(|_| ProtocolError::InvalidRequest)?;
                Ok(canonical_json(&JsonValue::object([
                    ("schemaVersion".to_owned(), JsonValue::from(1_u32)),
                    ("ok".to_owned(), JsonValue::from(true)),
                    (
                        "project".to_owned(),
                        project_to_json(self.engine.project(), true),
                    ),
                    ("status".to_owned(), status_json(&self.engine)),
                ])))
            }
            "execute" => {
                require_only_fields(object, &["schemaVersion", "operation", "envelope"])
                    .map_err(|_| ProtocolError::InvalidRequest)?;
                let envelope_value =
                    required(object, "envelope").map_err(|_| ProtocolError::InvalidRequest)?;
                let envelope = envelope_from_json(envelope_value)
                    .map_err(|_| ProtocolError::InvalidRequest)?;
                if &envelope_to_json(&envelope) != envelope_value {
                    return Err(ProtocolError::NonCanonicalRequest);
                }
                let result = self.engine.execute(&envelope);
                Ok(canonical_json(&result_response(&result)))
            }
            "undo" => {
                require_only_fields(
                    object,
                    &["schemaVersion", "operation", "transactionId", "undoToken"],
                )
                .map_err(|_| ProtocolError::InvalidRequest)?;
                let result = self.engine.undo(
                    TransactionId(parse_uuid(object, "transactionId")?),
                    UndoToken(parse_uuid(object, "undoToken")?),
                );
                Ok(canonical_json(&result_response(&result)))
            }
            "redo" => {
                require_only_fields(object, &["schemaVersion", "operation", "transactionId"])
                    .map_err(|_| ProtocolError::InvalidRequest)?;
                let result = self
                    .engine
                    .redo(TransactionId(parse_uuid(object, "transactionId")?));
                Ok(canonical_json(&result_response(&result)))
            }
            _ => Err(ProtocolError::UnsupportedOperation(operation.to_owned())),
        }
    }

    /// Returns raw canonical `.vlabproj` bytes. The shell owns durable and
    /// atomic file replacement; this method performs no host I/O.
    pub fn encode_package(&self, application_version: &str) -> Result<Vec<u8>, ProtocolError> {
        encode_project_package(self.engine.project(), application_version).map_err(Into::into)
    }

    /// Encodes and decodes the package before acknowledging the save. The
    /// checkpoint update is metadata-only and does not increment revisions.
    pub fn save_package(&mut self, application_version: &str) -> Result<Vec<u8>, ProtocolError> {
        let before_revision = self.engine.project().document_revision();
        let before_semantic_revision = self.engine.project().semantic_revision();
        let before_hash = self.engine.project().document_hash();
        let package = encode_project_package(self.engine.project(), application_version)?;
        let (verified, manifest) = decode_project_package(&package, DecodeLimits::default())?;
        if verified.document_hash() != before_hash {
            return Err(ProtocolError::Package(PackageError::ProjectMismatch(
                "save verification".to_owned(),
            )));
        }
        let acknowledged = self
            .engine
            .acknowledge_verified_save(before_hash, manifest.package_hash);
        if !acknowledged
            || self.engine.project().document_revision() != before_revision
            || self.engine.project().semantic_revision() != before_semantic_revision
        {
            return Err(ProtocolError::Package(PackageError::ProjectMismatch(
                "save revision stability".to_owned(),
            )));
        }
        Ok(package)
    }

    /// Decodes and verifies raw package bytes before replacing session state.
    pub fn replace_from_package(
        &mut self,
        package: &[u8],
        limits: DecodeLimits,
    ) -> Result<Manifest, ProtocolError> {
        let (project, manifest) = decode_project_package(package, limits)?;
        let engine = Engine::new(project)?;
        self.engine = engine;
        Ok(manifest)
    }
}

fn parse_request(request: &[u8]) -> Result<JsonValue, ProtocolError> {
    let value =
        parse_json(request, JsonLimits::default()).map_err(|_| ProtocolError::InvalidRequest)?;
    if canonical_json(&value) != request {
        return Err(ProtocolError::NonCanonicalRequest);
    }
    Ok(value)
}

fn check_version(object: &BTreeMap<String, JsonValue>) -> Result<(), ProtocolError> {
    let version = u32::try_from(
        required(object, "schemaVersion")
            .map_err(|_| ProtocolError::InvalidRequest)?
            .as_u64()
            .map_err(|_| ProtocolError::InvalidRequest)?,
    )
    .map_err(|_| ProtocolError::InvalidRequest)?;
    if version != 1 {
        return Err(ProtocolError::UnsupportedVersion(version));
    }
    Ok(())
}

fn text<'a>(
    object: &'a BTreeMap<String, JsonValue>,
    name: &'static str,
) -> Result<&'a str, ProtocolError> {
    required(object, name)
        .map_err(|_| ProtocolError::InvalidRequest)?
        .as_str()
        .map_err(|_| ProtocolError::InvalidRequest)
}

fn parse_uuid(
    object: &BTreeMap<String, JsonValue>,
    name: &'static str,
) -> Result<Uuid, ProtocolError> {
    let source = text(object, name)?;
    let parsed = Uuid::parse(source).map_err(|_| ProtocolError::InvalidRequest)?;
    if parsed.to_string() != source {
        return Err(ProtocolError::NonCanonicalRequest);
    }
    Ok(parsed)
}

fn result_response(result: &DomainCommandResult) -> JsonValue {
    JsonValue::object([
        ("schemaVersion".to_owned(), JsonValue::from(1_u32)),
        (
            "ok".to_owned(),
            JsonValue::from(result.outcome == CommandOutcome::Committed),
        ),
        ("result".to_owned(), result_json(result)),
    ])
}

fn status_json(engine: &Engine) -> JsonValue {
    let project = engine.project();
    JsonValue::object([
        (
            "documentDirty".to_owned(),
            JsonValue::from(project.is_document_dirty()),
        ),
        (
            "semanticDirty".to_owned(),
            JsonValue::from(project.is_semantic_dirty()),
        ),
        (
            "documentHash".to_owned(),
            JsonValue::from(project.document_hash().to_hex()),
        ),
        (
            "semanticFingerprint".to_owned(),
            JsonValue::from(project.semantic_fingerprint().to_hex()),
        ),
        (
            "savedDocumentRevision".to_owned(),
            project
                .saved_document_revision()
                .map_or(JsonValue::Null, |revision| {
                    JsonValue::from(revision.to_string())
                }),
        ),
        (
            "savedDocumentHash".to_owned(),
            project
                .saved_document_hash()
                .map_or(JsonValue::Null, |hash| JsonValue::from(hash.to_hex())),
        ),
        (
            "savedSemanticFingerprint".to_owned(),
            project
                .saved_semantic_fingerprint()
                .map_or(JsonValue::Null, |hash| JsonValue::from(hash.to_hex())),
        ),
        ("canUndo".to_owned(), JsonValue::from(engine.can_undo())),
        ("canRedo".to_owned(), JsonValue::from(engine.can_redo())),
        (
            "nextUndoToken".to_owned(),
            engine
                .next_undo_token()
                .map_or(JsonValue::Null, |token| JsonValue::from(token.to_string())),
        ),
    ])
}

fn result_json(result: &DomainCommandResult) -> JsonValue {
    JsonValue::object([
        (
            "outcome".to_owned(),
            JsonValue::from(match result.outcome {
                CommandOutcome::Committed => "committed",
                CommandOutcome::Rejected => "rejected",
                CommandOutcome::Blocked => "blocked",
            }),
        ),
        (
            "transactionId".to_owned(),
            JsonValue::from(result.transaction_id.to_string()),
        ),
        (
            "affectedObjectIds".to_owned(),
            JsonValue::Array(
                result
                    .affected_object_ids
                    .iter()
                    .map(ToString::to_string)
                    .map(JsonValue::from)
                    .collect(),
            ),
        ),
        (
            "domainEvents".to_owned(),
            JsonValue::Array(result.domain_events.iter().map(event_json).collect()),
        ),
        (
            "diagnostics".to_owned(),
            JsonValue::Array(result.diagnostics.iter().map(diagnostic_json).collect()),
        ),
        (
            "undoToken".to_owned(),
            result
                .undo_token
                .map_or(JsonValue::Null, |token| JsonValue::from(token.to_string())),
        ),
        (
            "beforeProjectHash".to_owned(),
            JsonValue::from(result.before_project_hash.to_hex()),
        ),
        (
            "afterProjectHash".to_owned(),
            result
                .after_project_hash
                .map_or(JsonValue::Null, |hash| JsonValue::from(hash.to_hex())),
        ),
    ])
}

fn diagnostic_json(diagnostic: &Diagnostic) -> JsonValue {
    JsonValue::object([
        ("code".to_owned(), JsonValue::from(diagnostic.code.clone())),
        (
            "message".to_owned(),
            JsonValue::from(diagnostic.message.clone()),
        ),
        (
            "objectIds".to_owned(),
            JsonValue::Array(
                diagnostic
                    .object_ids
                    .iter()
                    .map(ToString::to_string)
                    .map(JsonValue::from)
                    .collect(),
            ),
        ),
    ])
}

fn event_json(event: &DomainEvent) -> JsonValue {
    match event {
        DomainEvent::Created(id) => single_event("created", *id),
        DomainEvent::Renamed(id) => single_event("renamed", *id),
        DomainEvent::Moved(id) => single_event("moved", *id),
        DomainEvent::Changed(id) => single_event("changed", *id),
        DomainEvent::Deleted(id) => single_event("deleted", *id),
        DomainEvent::Restored(id) => single_event("restored", *id),
        DomainEvent::Copied { source, copy } => JsonValue::object([
            ("kind".to_owned(), JsonValue::from("copied")),
            ("sourceId".to_owned(), JsonValue::from(source.to_string())),
            ("copyId".to_owned(), JsonValue::from(copy.to_string())),
        ]),
        DomainEvent::ReferenceChanged { source, target } => JsonValue::object([
            ("kind".to_owned(), JsonValue::from("reference-changed")),
            ("sourceId".to_owned(), JsonValue::from(source.to_string())),
            ("targetId".to_owned(), JsonValue::from(target.to_string())),
        ]),
        DomainEvent::DependencyChanged { source, target } => JsonValue::object([
            ("kind".to_owned(), JsonValue::from("dependency-changed")),
            ("sourceId".to_owned(), JsonValue::from(source.to_string())),
            ("targetId".to_owned(), JsonValue::from(target.to_string())),
        ]),
    }
}

fn single_event(kind: &str, id: ObjectId) -> JsonValue {
    JsonValue::object([
        ("kind".to_owned(), JsonValue::from(kind)),
        ("objectId".to_owned(), JsonValue::from(id.to_string())),
    ])
}

#[cfg(test)]
mod tests {
    use super::KernelSession;

    #[test]
    fn create_and_query_are_canonical_bytes() {
        let request = br#"{"displayName":"Protocol","documentId":"cda496e1-165b-4ab0-9ddc-9ad749bf75a4","profile":{"id":"training","manifestHash":"0909090909090909090909090909090909090909090909090909090909090909","version":"1"},"rootId":"88c521b1-f9f7-4bb0-8dc1-adca746a13a6","schemaVersion":1}"#;
        let mut session = KernelSession::create(request).expect("create");
        let response = session
            .handle(br#"{"operation":"query-project","schemaVersion":1}"#)
            .expect("query");
        assert!(response.starts_with(b"{\"ok\":true,\"project\":"));
        let package = session.encode_package("test/1").expect("package");
        let (opened, _) =
            KernelSession::open(&package, crate::DecodeLimits::default()).expect("open");
        assert_eq!(
            opened.project().document_hash(),
            session.project().document_hash()
        );
    }

    #[test]
    fn save_is_revision_stable_and_query_uses_lossless_uint64_strings() {
        let request = br#"{"displayName":"Protocol","documentId":"cda496e1-165b-4ab0-9ddc-9ad749bf75a4","profile":{"id":"training","manifestHash":"0909090909090909090909090909090909090909090909090909090909090909","version":"1"},"rootId":"88c521b1-f9f7-4bb0-8dc1-adca746a13a6","schemaVersion":1}"#;
        let mut session = KernelSession::create(request).expect("create");
        let document_revision = session.project().document_revision();
        let semantic_revision = session.project().semantic_revision();
        let package = session.save_package("test/1").expect("verified save");
        assert!(!package.is_empty());
        assert_eq!(session.project().document_revision(), document_revision);
        assert_eq!(session.project().semantic_revision(), semantic_revision);
        assert!(!session.project().is_document_dirty());

        let mut project = session.engine.into_project();
        project.document_revision = u64::MAX;
        let root_id = project.root_id;
        project
            .objects
            .get_mut(&root_id)
            .expect("root")
            .object_revision = u64::MAX;
        session = KernelSession::from_project(project).expect("max revision session");
        let response = session
            .handle(br#"{"operation":"query-project","schemaVersion":1}"#)
            .expect("query");
        let text = core::str::from_utf8(&response).expect("UTF-8");
        assert!(text.contains("\"documentRevision\":\"18446744073709551615\""));
        assert!(text.contains("\"objectRevision\":\"18446744073709551615\""));
    }
}
