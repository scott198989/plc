#![deny(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;

pub mod protocol;
mod sha256;

#[allow(unsafe_code)]
#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::WindowsProjectStorage;

pub const BROKER_PROTOCOL_VERSION: u16 = 1;
pub const PROJECT_EXTENSION: &str = ".vlabproj";
pub const MAX_PROJECT_BYTES: usize = 32 * 1024 * 1024;
const MAX_PROJECT_NAME_CODE_UNITS: usize = 255;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrokerErrorCode {
    AccessUnavailable,
    AttestationFailed,
    InvalidExtension,
    InvalidFileName,
    InvalidFrame,
    ProjectTooLarge,
    ProtocolMismatch,
    ReadFailed,
    StaleGrant,
    UnknownGrant,
    WriteFailed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BrokerError {
    pub code: BrokerErrorCode,
    pub message: &'static str,
}

impl BrokerError {
    pub const fn new(code: BrokerErrorCode, message: &'static str) -> Self {
        Self { code, message }
    }
}

impl fmt::Display for BrokerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message)
    }
}

impl std::error::Error for BrokerError {}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ProjectFileName(String);

impl ProjectFileName {
    pub fn parse(value: &str) -> Result<Self, BrokerError> {
        if !value.to_ascii_lowercase().ends_with(PROJECT_EXTENSION) {
            return Err(BrokerError::new(
                BrokerErrorCode::InvalidExtension,
                "Project files must use the .vlabproj extension.",
            ));
        }
        if value.encode_utf16().count() > MAX_PROJECT_NAME_CODE_UNITS
            || value.trim() != value
            || value.ends_with('.')
            || !value.is_ascii()
        {
            return Err(invalid_file_name());
        }
        let stem = &value[..value.len() - PROJECT_EXTENSION.len()];
        if stem.is_empty()
            || stem.trim() != stem
            || stem.ends_with('.')
            || stem == "."
            || stem == ".."
            || stem.bytes().any(|byte| {
                !byte.is_ascii_alphanumeric()
                    && !matches!(byte, b' ' | b'-' | b'_' | b'.' | b'(' | b')')
            })
            || is_reserved_windows_stem(stem)
        {
            return Err(invalid_file_name());
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn invalid_file_name() -> BrokerError {
    BrokerError::new(
        BrokerErrorCode::InvalidFileName,
        "A project name must be one bounded ASCII file name, never a path or host target.",
    )
}

fn is_reserved_windows_stem(stem: &str) -> bool {
    let candidate = stem
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches(['.', ' '])
        .to_ascii_uppercase();
    matches!(candidate.as_str(), "AUX" | "CON" | "NUL" | "PRN")
        || candidate
            .strip_prefix("COM")
            .or_else(|| candidate.strip_prefix("LPT"))
            .is_some_and(|suffix| {
                matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
            })
}

fn validate_bytes(bytes: &[u8]) -> Result<(), BrokerError> {
    if bytes.is_empty() || bytes.len() > MAX_PROJECT_BYTES {
        return Err(BrokerError::new(
            BrokerErrorCode::ProjectTooLarge,
            "Project payloads must be between 1 byte and 32 MiB.",
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixedFileSystem {
    Ntfs,
    Refs,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackingAttestation {
    pub protocol_version: u16,
    pub file_system: FixedFileSystem,
    pub volume_serial: u64,
    pub fixed_drive: bool,
    pub native_local: bool,
    pub provider_backed: bool,
    pub redirected: bool,
    pub removable: bool,
    pub special: bool,
}

impl BackingAttestation {
    pub fn validate(self) -> Result<Self, BrokerError> {
        if self.protocol_version != BROKER_PROTOCOL_VERSION
            || !self.fixed_drive
            || !self.native_local
            || self.provider_backed
            || self.redirected
            || self.removable
            || self.special
        {
            return Err(BrokerError::new(
                BrokerErrorCode::AttestationFailed,
                "The project root is not proven fixed, native, local, and non-provider-backed.",
            ));
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GrantId(u64);

impl GrantId {
    pub const fn from_wire(value: u64) -> Self {
        Self(value)
    }

    pub const fn to_wire(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestedFile<Token> {
    pub name: ProjectFileName,
    pub token: Token,
    pub size: usize,
}

pub trait ProjectStorage {
    type Token: Clone + Eq;

    fn attest_root(&mut self) -> Result<BackingAttestation, BrokerError>;
    fn list_projects(&mut self) -> Result<Vec<AttestedFile<Self::Token>>, BrokerError>;
    fn inspect_existing(
        &mut self,
        name: &ProjectFileName,
    ) -> Result<AttestedFile<Self::Token>, BrokerError>;
    fn read_attested(&mut self, file: &AttestedFile<Self::Token>) -> Result<Vec<u8>, BrokerError>;
    fn replace_verified(
        &mut self,
        name: &ProjectFileName,
        expected: Option<&Self::Token>,
        bytes: &[u8],
    ) -> Result<AttestedFile<Self::Token>, BrokerError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestEnvelope {
    pub protocol_version: u16,
    pub request_id: u64,
    pub request: BrokerRequest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BrokerRequest {
    Handshake,
    ListProjects,
    Open { name: String },
    SaveAs { name: String, bytes: Vec<u8> },
    Save { grant_id: GrantId, bytes: Vec<u8> },
    Revoke { grant_id: GrantId },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResponseEnvelope {
    pub protocol_version: u16,
    pub request_id: u64,
    pub response: Result<BrokerResponse, BrokerError>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BrokerResponse {
    Handshake {
        attestation: BackingAttestation,
    },
    Projects {
        names: Vec<String>,
    },
    Opened {
        display_name: String,
        grant_id: GrantId,
        bytes: Vec<u8>,
    },
    Saved {
        display_name: String,
        grant_id: GrantId,
        verified_bytes: usize,
    },
    Revoked,
}

#[derive(Clone)]
struct Grant<Token> {
    name: ProjectFileName,
    token: Token,
}

pub struct ProjectFileBroker<Storage: ProjectStorage> {
    storage: Storage,
    attestation: BackingAttestation,
    grants: BTreeMap<u64, Grant<Storage::Token>>,
    next_grant_id: u64,
}

impl<Storage: ProjectStorage> ProjectFileBroker<Storage> {
    pub fn initialize(mut storage: Storage) -> Result<Self, BrokerError> {
        let attestation = storage.attest_root()?.validate()?;
        Ok(Self {
            storage,
            attestation,
            grants: BTreeMap::new(),
            next_grant_id: 1,
        })
    }

    pub fn execute(&mut self, envelope: RequestEnvelope) -> ResponseEnvelope {
        let response = if envelope.protocol_version == BROKER_PROTOCOL_VERSION {
            self.execute_request(envelope.request)
        } else {
            Err(BrokerError::new(
                BrokerErrorCode::ProtocolMismatch,
                "The native project broker protocol version is unsupported.",
            ))
        };
        ResponseEnvelope {
            protocol_version: BROKER_PROTOCOL_VERSION,
            request_id: envelope.request_id,
            response,
        }
    }

    fn execute_request(&mut self, request: BrokerRequest) -> Result<BrokerResponse, BrokerError> {
        match request {
            BrokerRequest::Handshake => Ok(BrokerResponse::Handshake {
                attestation: self.attestation,
            }),
            BrokerRequest::ListProjects => {
                let mut projects = self.storage.list_projects()?;
                projects.sort_by(|left, right| left.name.cmp(&right.name));
                Ok(BrokerResponse::Projects {
                    names: projects
                        .into_iter()
                        .map(|entry| entry.name.as_str().to_owned())
                        .collect(),
                })
            }
            BrokerRequest::Open { name } => {
                let name = ProjectFileName::parse(&name)?;
                let file = self.storage.inspect_existing(&name)?;
                if file.name != name || file.size == 0 || file.size > MAX_PROJECT_BYTES {
                    return Err(BrokerError::new(
                        BrokerErrorCode::ReadFailed,
                        "The attested project metadata is inconsistent or out of bounds.",
                    ));
                }
                let bytes = self.storage.read_attested(&file)?;
                validate_bytes(&bytes)?;
                if bytes.len() != file.size {
                    return Err(BrokerError::new(
                        BrokerErrorCode::ReadFailed,
                        "The project changed after attestation and was not opened.",
                    ));
                }
                let grant_id = self.allocate_grant(file.name.clone(), file.token)?;
                Ok(BrokerResponse::Opened {
                    display_name: name.as_str().to_owned(),
                    grant_id,
                    bytes,
                })
            }
            BrokerRequest::SaveAs { name, bytes } => {
                let name = ProjectFileName::parse(&name)?;
                validate_bytes(&bytes)?;
                let file = self.storage.replace_verified(&name, None, &bytes)?;
                if file.name != name || file.size != bytes.len() {
                    return Err(BrokerError::new(
                        BrokerErrorCode::WriteFailed,
                        "The native broker did not verify the complete replacement.",
                    ));
                }
                let grant_id = self.allocate_grant(file.name.clone(), file.token)?;
                Ok(BrokerResponse::Saved {
                    display_name: name.as_str().to_owned(),
                    grant_id,
                    verified_bytes: bytes.len(),
                })
            }
            BrokerRequest::Save { grant_id, bytes } => {
                validate_bytes(&bytes)?;
                let Some(grant) = self.grants.get(&grant_id.0).cloned() else {
                    return Err(BrokerError::new(
                        BrokerErrorCode::UnknownGrant,
                        "The project file grant is no longer active.",
                    ));
                };
                let file =
                    self.storage
                        .replace_verified(&grant.name, Some(&grant.token), &bytes)?;
                if file.name != grant.name || file.size != bytes.len() {
                    self.grants.remove(&grant_id.0);
                    return Err(BrokerError::new(
                        BrokerErrorCode::WriteFailed,
                        "The native broker did not verify the complete replacement.",
                    ));
                }
                self.grants.insert(
                    grant_id.0,
                    Grant {
                        name: file.name.clone(),
                        token: file.token,
                    },
                );
                Ok(BrokerResponse::Saved {
                    display_name: file.name.as_str().to_owned(),
                    grant_id,
                    verified_bytes: bytes.len(),
                })
            }
            BrokerRequest::Revoke { grant_id } => {
                self.grants.remove(&grant_id.0);
                Ok(BrokerResponse::Revoked)
            }
        }
    }

    fn allocate_grant(
        &mut self,
        name: ProjectFileName,
        token: Storage::Token,
    ) -> Result<GrantId, BrokerError> {
        let id = self.next_grant_id;
        if id == 0 || self.grants.contains_key(&id) {
            return Err(BrokerError::new(
                BrokerErrorCode::AccessUnavailable,
                "The native broker cannot allocate another opaque grant without collision.",
            ));
        }
        self.next_grant_id = id.checked_add(1).ok_or_else(|| {
            BrokerError::new(
                BrokerErrorCode::AccessUnavailable,
                "The native broker exhausted its opaque grant identity space.",
            )
        })?;
        self.grants.insert(id, Grant { name, token });
        Ok(GrantId(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct FakeToken(u64);

    struct FakeStorage {
        attestation: BackingAttestation,
        effects: Vec<String>,
        file: Option<(ProjectFileName, FakeToken, Vec<u8>)>,
        reject_inspection: bool,
    }

    impl FakeStorage {
        fn safe() -> Self {
            Self {
                attestation: BackingAttestation {
                    protocol_version: BROKER_PROTOCOL_VERSION,
                    file_system: FixedFileSystem::Ntfs,
                    volume_serial: 7,
                    fixed_drive: true,
                    native_local: true,
                    provider_backed: false,
                    redirected: false,
                    removable: false,
                    special: false,
                },
                effects: Vec::new(),
                file: Some((
                    ProjectFileName::parse("cell-a.vlabproj").unwrap(),
                    FakeToken(9),
                    vec![1, 3, 3, 7],
                )),
                reject_inspection: false,
            }
        }
    }

    impl ProjectStorage for FakeStorage {
        type Token = FakeToken;

        fn attest_root(&mut self) -> Result<BackingAttestation, BrokerError> {
            self.effects.push("attest-root".into());
            Ok(self.attestation)
        }

        fn list_projects(&mut self) -> Result<Vec<AttestedFile<Self::Token>>, BrokerError> {
            self.effects.push("list".into());
            Ok(self
                .file
                .as_ref()
                .map(|(name, token, bytes)| AttestedFile {
                    name: name.clone(),
                    token: token.clone(),
                    size: bytes.len(),
                })
                .into_iter()
                .collect())
        }

        fn inspect_existing(
            &mut self,
            name: &ProjectFileName,
        ) -> Result<AttestedFile<Self::Token>, BrokerError> {
            self.effects.push(format!("inspect:{}", name.as_str()));
            if self.reject_inspection {
                return Err(BrokerError::new(
                    BrokerErrorCode::AttestationFailed,
                    "unsafe backing",
                ));
            }
            let (actual, token, bytes) = self
                .file
                .as_ref()
                .ok_or_else(|| BrokerError::new(BrokerErrorCode::ReadFailed, "missing project"))?;
            Ok(AttestedFile {
                name: actual.clone(),
                token: token.clone(),
                size: bytes.len(),
            })
        }

        fn read_attested(
            &mut self,
            file: &AttestedFile<Self::Token>,
        ) -> Result<Vec<u8>, BrokerError> {
            self.effects.push("read".into());
            let (_, token, bytes) = self.file.as_ref().unwrap();
            if token != &file.token {
                return Err(BrokerError::new(BrokerErrorCode::StaleGrant, "stale"));
            }
            Ok(bytes.clone())
        }

        fn replace_verified(
            &mut self,
            name: &ProjectFileName,
            expected: Option<&Self::Token>,
            bytes: &[u8],
        ) -> Result<AttestedFile<Self::Token>, BrokerError> {
            self.effects.push("replace".into());
            if let Some(expected) = expected
                && self.file.as_ref().map(|file| &file.1) != Some(expected)
            {
                return Err(BrokerError::new(BrokerErrorCode::StaleGrant, "stale grant"));
            }
            let next = FakeToken(self.file.as_ref().map_or(1, |file| file.1.0 + 1));
            self.file = Some((name.clone(), next.clone(), bytes.to_vec()));
            Ok(AttestedFile {
                name: name.clone(),
                token: next,
                size: bytes.len(),
            })
        }
    }

    fn request(request: BrokerRequest) -> RequestEnvelope {
        RequestEnvelope {
            protocol_version: BROKER_PROTOCOL_VERSION,
            request_id: 41,
            request,
        }
    }

    #[test]
    fn unsafe_root_attestations_fail_initialization() {
        for mutation in [
            |value: &mut BackingAttestation| value.fixed_drive = false,
            |value: &mut BackingAttestation| value.native_local = false,
            |value: &mut BackingAttestation| value.provider_backed = true,
            |value: &mut BackingAttestation| value.redirected = true,
            |value: &mut BackingAttestation| value.removable = true,
            |value: &mut BackingAttestation| value.special = true,
        ] {
            let mut storage = FakeStorage::safe();
            mutation(&mut storage.attestation);
            assert_eq!(
                ProjectFileBroker::initialize(storage).err().unwrap().code,
                BrokerErrorCode::AttestationFailed
            );
        }
    }

    #[test]
    fn invalid_or_target_shaped_names_are_rejected_before_host_effects() {
        let storage = FakeStorage::safe();
        let mut broker = ProjectFileBroker::initialize(storage).unwrap();
        for name in [
            r"C:\unsafe\cell.vlabproj",
            r"\\server\share\cell.vlabproj",
            r"\\.\pipe\cell.vlabproj",
            "file://server/cell.vlabproj",
            "https://server/cell.vlabproj",
            "../cell.vlabproj",
            "PRN.vlabproj",
            "COM1.vlabproj",
            "cell:stream.vlabproj",
            "cell.vlabproj ",
            "café.vlabproj",
        ] {
            let response = broker.execute(request(BrokerRequest::Open { name: name.into() }));
            assert!(response.response.is_err(), "{name}");
        }
        assert_eq!(broker.storage.effects, ["attest-root"]);
    }

    #[test]
    fn unsafe_file_attestation_prevents_selected_byte_io() {
        let mut storage = FakeStorage::safe();
        storage.reject_inspection = true;
        let mut broker = ProjectFileBroker::initialize(storage).unwrap();
        let response = broker.execute(request(BrokerRequest::Open {
            name: "cell-a.vlabproj".into(),
        }));
        assert_eq!(
            response.response.unwrap_err().code,
            BrokerErrorCode::AttestationFailed
        );
        assert_eq!(
            broker.storage.effects,
            ["attest-root", "inspect:cell-a.vlabproj"]
        );
    }

    #[test]
    fn open_save_and_revoke_keep_storage_tokens_behind_opaque_grants() {
        let storage = FakeStorage::safe();
        let mut broker = ProjectFileBroker::initialize(storage).unwrap();
        let opened = broker.execute(request(BrokerRequest::Open {
            name: "cell-a.vlabproj".into(),
        }));
        let BrokerResponse::Opened {
            display_name,
            grant_id,
            bytes,
        } = opened.response.unwrap()
        else {
            panic!("expected open response")
        };
        assert_eq!(display_name, "cell-a.vlabproj");
        assert_eq!(bytes, [1, 3, 3, 7]);

        let saved = broker.execute(request(BrokerRequest::Save {
            grant_id,
            bytes: vec![4, 5, 6],
        }));
        assert!(matches!(
            saved.response,
            Ok(BrokerResponse::Saved {
                verified_bytes: 3,
                ..
            })
        ));
        broker.execute(request(BrokerRequest::Revoke { grant_id }));
        let rejected = broker.execute(request(BrokerRequest::Save {
            grant_id,
            bytes: vec![8],
        }));
        assert_eq!(
            rejected.response.unwrap_err().code,
            BrokerErrorCode::UnknownGrant
        );
    }

    #[test]
    fn protocol_mismatch_and_oversize_payloads_fail_before_storage_effects() {
        let storage = FakeStorage::safe();
        let mut broker = ProjectFileBroker::initialize(storage).unwrap();
        let mismatch = broker.execute(RequestEnvelope {
            protocol_version: BROKER_PROTOCOL_VERSION + 1,
            request_id: 1,
            request: BrokerRequest::ListProjects,
        });
        assert_eq!(
            mismatch.response.unwrap_err().code,
            BrokerErrorCode::ProtocolMismatch
        );
        let oversize = broker.execute(request(BrokerRequest::SaveAs {
            name: "cell.vlabproj".into(),
            bytes: vec![0; MAX_PROJECT_BYTES + 1],
        }));
        assert_eq!(
            oversize.response.unwrap_err().code,
            BrokerErrorCode::ProjectTooLarge
        );
        assert_eq!(broker.storage.effects, ["attest-root"]);
    }

    #[test]
    fn grant_collision_and_exhaustion_fail_closed_without_overwriting_authority() {
        let storage = FakeStorage::safe();
        let mut broker = ProjectFileBroker::initialize(storage).unwrap();
        let first = broker.execute(request(BrokerRequest::Open {
            name: "cell-a.vlabproj".into(),
        }));
        assert!(matches!(first.response, Ok(BrokerResponse::Opened { .. })));

        broker.next_grant_id = 1;
        let collision = broker.execute(request(BrokerRequest::Open {
            name: "cell-a.vlabproj".into(),
        }));
        assert_eq!(
            collision.response.unwrap_err().code,
            BrokerErrorCode::AccessUnavailable
        );
        assert_eq!(broker.grants.len(), 1);

        broker.next_grant_id = u64::MAX;
        let exhausted = broker.execute(request(BrokerRequest::Open {
            name: "cell-a.vlabproj".into(),
        }));
        assert_eq!(
            exhausted.response.unwrap_err().code,
            BrokerErrorCode::AccessUnavailable
        );
        assert_eq!(broker.grants.len(), 1);
    }
}
