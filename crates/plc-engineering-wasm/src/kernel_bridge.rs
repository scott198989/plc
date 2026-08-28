use core::fmt;

use plc_core::{DecodeLimits, KernelSession, ProtocolError, Uuid, sha256};

const APPLICATION_VERSION: &str = "plc-engineering-simulator/0.2.0";
const QUERY_PROJECT: &[u8] = br#"{"operation":"query-project","schemaVersion":1}"#;
const MAX_BRIDGE_INPUT_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SaveMode {
    Save,
    SaveAs(Uuid),
}

#[derive(Clone, Debug)]
struct PendingSave {
    session: KernelSession,
    package_bytes: usize,
    package_digest: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum BridgeError {
    InputLimit,
    InvalidDocumentId,
    NoActiveSession,
    NoPendingSave,
    PendingSaveExists,
    SaveVerificationMismatch,
    Protocol(String),
}

impl fmt::Display for BridgeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputLimit => formatter.write_str("bridge input exceeds its byte limit"),
            Self::InvalidDocumentId => formatter.write_str("Save As document identity is invalid"),
            Self::NoActiveSession => formatter.write_str("no project session is active"),
            Self::NoPendingSave => formatter.write_str("no verified save is pending"),
            Self::PendingSaveExists => formatter.write_str("a verified save is already pending"),
            Self::SaveVerificationMismatch => {
                formatter.write_str("durable save verification does not match the prepared package")
            }
            Self::Protocol(message) => {
                write!(formatter, "project kernel rejected the request: {message}")
            }
        }
    }
}

impl From<ProtocolError> for BridgeError {
    fn from(value: ProtocolError) -> Self {
        Self::Protocol(value.to_string())
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct KernelBridge {
    active: Option<KernelSession>,
    pending_save: Option<PendingSave>,
}

impl KernelBridge {
    pub(crate) fn create(&mut self, request: &[u8]) -> Result<Vec<u8>, BridgeError> {
        check_input(request)?;
        let session = KernelSession::create(request)?;
        self.active = Some(session);
        self.pending_save = None;
        self.query()
    }

    pub(crate) fn open(&mut self, package: &[u8]) -> Result<Vec<u8>, BridgeError> {
        check_input(package)?;
        let (session, _) = KernelSession::open(package, DecodeLimits::default())?;
        self.active = Some(session);
        self.pending_save = None;
        self.query()
    }

    pub(crate) fn handle(&mut self, request: &[u8]) -> Result<Vec<u8>, BridgeError> {
        check_input(request)?;
        if self.pending_save.is_some() {
            return Err(BridgeError::PendingSaveExists);
        }
        self.active
            .as_mut()
            .ok_or(BridgeError::NoActiveSession)?
            .handle(request)
            .map_err(Into::into)
    }

    pub(crate) fn query(&mut self) -> Result<Vec<u8>, BridgeError> {
        self.active
            .as_mut()
            .ok_or(BridgeError::NoActiveSession)?
            .handle(QUERY_PROJECT)
            .map_err(Into::into)
    }

    pub(crate) fn prepare_save(&mut self, mode: SaveMode) -> Result<Vec<u8>, BridgeError> {
        if self.pending_save.is_some() {
            return Err(BridgeError::PendingSaveExists);
        }
        let active = self.active.as_ref().ok_or(BridgeError::NoActiveSession)?;
        let mut pending_session = match mode {
            SaveMode::Save => active.clone(),
            SaveMode::SaveAs(document_id) => {
                let project = active
                    .project()
                    .for_save_as(document_id)
                    .ok_or(BridgeError::InvalidDocumentId)?;
                KernelSession::from_project(project)?
            }
        };
        let package = pending_session.save_package(APPLICATION_VERSION)?;
        let package_digest = sha256(&package).0;
        self.pending_save = Some(PendingSave {
            session: pending_session,
            package_bytes: package.len(),
            package_digest,
        });
        Ok(package)
    }

    pub(crate) fn commit_save(
        &mut self,
        verified_bytes: usize,
        verified_digest: [u8; 32],
    ) -> Result<Vec<u8>, BridgeError> {
        let pending = self.pending_save.take().ok_or(BridgeError::NoPendingSave)?;
        if pending.package_bytes != verified_bytes || pending.package_digest != verified_digest {
            self.pending_save = Some(pending);
            return Err(BridgeError::SaveVerificationMismatch);
        }
        self.active = Some(pending.session);
        self.query()
    }

    pub(crate) fn abort_save(&mut self) -> Result<(), BridgeError> {
        if self.pending_save.take().is_none() {
            return Err(BridgeError::NoPendingSave);
        }
        Ok(())
    }
}

fn check_input(input: &[u8]) -> Result<(), BridgeError> {
    if input.is_empty() || input.len() > MAX_BRIDGE_INPUT_BYTES {
        return Err(BridgeError::InputLimit);
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
mod exports {
    use std::cell::RefCell;

    use super::{BridgeError, KernelBridge, MAX_BRIDGE_INPUT_BYTES, SaveMode};
    use plc_core::Uuid;

    const STATUS_OK: i32 = 0;
    const STATUS_ERROR: i32 = 1;

    std::thread_local! {
        static BRIDGE: RefCell<KernelBridge> = RefCell::new(KernelBridge::default());
        static INPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
        static OUTPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    }

    #[allow(unsafe_code)]
    #[unsafe(no_mangle)]
    pub extern "C" fn plc_input_prepare(length: u32) -> u32 {
        let Ok(length) = usize::try_from(length) else {
            write_error(BridgeError::InputLimit);
            return 0;
        };
        if length > MAX_BRIDGE_INPUT_BYTES {
            write_error(BridgeError::InputLimit);
            return 0;
        }
        INPUT.with_borrow_mut(|input| {
            input.resize(length, 0);
            input.as_mut_ptr() as u32
        })
    }

    #[allow(unsafe_code)]
    #[unsafe(no_mangle)]
    pub extern "C" fn plc_output_pointer() -> u32 {
        OUTPUT.with_borrow(|output| output.as_ptr() as u32)
    }

    #[allow(unsafe_code)]
    #[unsafe(no_mangle)]
    pub extern "C" fn plc_output_length() -> u32 {
        OUTPUT.with_borrow(|output| u32::try_from(output.len()).unwrap_or(0))
    }

    #[allow(unsafe_code)]
    #[unsafe(no_mangle)]
    pub extern "C" fn plc_session_create(length: u32) -> i32 {
        run_with_input(length, KernelBridge::create)
    }

    #[allow(unsafe_code)]
    #[unsafe(no_mangle)]
    pub extern "C" fn plc_session_open(length: u32) -> i32 {
        run_with_input(length, KernelBridge::open)
    }

    #[allow(unsafe_code)]
    #[unsafe(no_mangle)]
    pub extern "C" fn plc_session_handle(length: u32) -> i32 {
        run_with_input(length, KernelBridge::handle)
    }

    #[allow(unsafe_code)]
    #[unsafe(no_mangle)]
    pub extern "C" fn plc_session_prepare_save(mode: u32, length: u32) -> i32 {
        let save_mode = if mode == 0 {
            SaveMode::Save
        } else if mode == 1 {
            let document_id = match read_input(length).and_then(|bytes| {
                let text =
                    core::str::from_utf8(&bytes).map_err(|_| BridgeError::InvalidDocumentId)?;
                Uuid::parse(text).map_err(|_| BridgeError::InvalidDocumentId)
            }) {
                Ok(document_id) => document_id,
                Err(error) => {
                    write_error(error);
                    return STATUS_ERROR;
                }
            };
            SaveMode::SaveAs(document_id)
        } else {
            write_error(BridgeError::InvalidDocumentId);
            return STATUS_ERROR;
        };

        match BRIDGE.with_borrow_mut(|bridge| bridge.prepare_save(save_mode)) {
            Ok(output) => {
                write_output(output);
                STATUS_OK
            }
            Err(error) => {
                write_error(error);
                STATUS_ERROR
            }
        }
    }

    #[allow(unsafe_code)]
    #[unsafe(no_mangle)]
    pub extern "C" fn plc_session_commit_save(verified_bytes: u32, digest_length: u32) -> i32 {
        let digest = match read_input(digest_length).and_then(|bytes| {
            if bytes.len() != 32 {
                return Err(BridgeError::SaveVerificationMismatch);
            }
            let mut digest = [0_u8; 32];
            digest.copy_from_slice(&bytes);
            Ok(digest)
        }) {
            Ok(digest) => digest,
            Err(error) => {
                write_error(error);
                return STATUS_ERROR;
            }
        };
        let verified_bytes = match usize::try_from(verified_bytes) {
            Ok(value) => value,
            Err(_) => {
                write_error(BridgeError::SaveVerificationMismatch);
                return STATUS_ERROR;
            }
        };
        match BRIDGE.with_borrow_mut(|bridge| bridge.commit_save(verified_bytes, digest)) {
            Ok(output) => {
                write_output(output);
                STATUS_OK
            }
            Err(error) => {
                write_error(error);
                STATUS_ERROR
            }
        }
    }

    #[allow(unsafe_code)]
    #[unsafe(no_mangle)]
    pub extern "C" fn plc_session_abort_save() -> i32 {
        match BRIDGE.with_borrow_mut(KernelBridge::abort_save) {
            Ok(()) => {
                write_output(b"null".to_vec());
                STATUS_OK
            }
            Err(error) => {
                write_error(error);
                STATUS_ERROR
            }
        }
    }

    fn run_with_input(
        length: u32,
        operation: impl FnOnce(&mut KernelBridge, &[u8]) -> Result<Vec<u8>, BridgeError>,
    ) -> i32 {
        let input = match read_input(length) {
            Ok(input) => input,
            Err(error) => {
                write_error(error);
                return STATUS_ERROR;
            }
        };
        match BRIDGE.with_borrow_mut(|bridge| operation(bridge, &input)) {
            Ok(output) => {
                write_output(output);
                STATUS_OK
            }
            Err(error) => {
                write_error(error);
                STATUS_ERROR
            }
        }
    }

    fn read_input(length: u32) -> Result<Vec<u8>, BridgeError> {
        let length = usize::try_from(length).map_err(|_| BridgeError::InputLimit)?;
        INPUT.with_borrow(|input| {
            if input.len() != length || input.len() > MAX_BRIDGE_INPUT_BYTES {
                Err(BridgeError::InputLimit)
            } else {
                Ok(input.clone())
            }
        })
    }

    fn write_error(error: BridgeError) {
        let escaped = error.to_string().replace('\\', "\\\\").replace('"', "\\\"");
        write_output(format!(r#"{{"error":"{escaped}"}}"#).into_bytes());
    }

    fn write_output(output: Vec<u8>) {
        OUTPUT.with_borrow_mut(|destination| *destination = output);
    }
}

#[cfg(test)]
mod tests {
    use super::{BridgeError, KernelBridge, SaveMode};
    use plc_core::{DecodeLimits, KernelSession, Uuid, sha256};

    const CREATE: &[u8] = br#"{"displayName":"Bridge","documentId":"cda496e1-165b-4ab0-9ddc-9ad749bf75a4","profile":{"id":"edu-21","manifestHash":"0909090909090909090909090909090909090909090909090909090909090909","version":"1"},"rootId":"88c521b1-f9f7-4bb0-8dc1-adca746a13a6","schemaVersion":1}"#;

    #[test]
    fn create_query_and_open_use_the_real_kernel() {
        let mut bridge = KernelBridge::default();
        let created = bridge.create(CREATE).expect("create");
        assert!(created.starts_with(br#"{"ok":true,"project":"#));

        let package = bridge.prepare_save(SaveMode::Save).expect("prepare");
        let digest = sha256(&package).0;
        let committed = bridge.commit_save(package.len(), digest).expect("commit");
        assert!(
            committed
                .windows(b"\"documentDirty\":false".len())
                .any(|window| window == b"\"documentDirty\":false")
        );

        let mut reopened = KernelBridge::default();
        reopened.open(&package).expect("open");
        let query = reopened.query().expect("query");
        assert_eq!(query, committed);
        KernelSession::open(&package, DecodeLimits::default()).expect("native decode");
    }

    #[test]
    fn save_as_stages_identity_until_durable_commit() {
        let mut bridge = KernelBridge::default();
        let original = bridge.create(CREATE).expect("create");
        let new_document_id = Uuid::parse("11111111-1111-4111-8111-111111111111").expect("uuid");
        let package = bridge
            .prepare_save(SaveMode::SaveAs(new_document_id))
            .expect("prepare Save As");

        assert_eq!(bridge.query().expect("original query"), original);
        assert_eq!(
            bridge.commit_save(package.len(), [0_u8; 32]),
            Err(BridgeError::SaveVerificationMismatch),
        );
        assert_eq!(bridge.query().expect("still original"), original);

        bridge
            .commit_save(package.len(), sha256(&package).0)
            .expect("verified commit");
        let committed = bridge.query().expect("committed query");
        assert!(
            committed
                .windows(b"11111111-1111-4111-8111-111111111111".len())
                .any(|window| window == b"11111111-1111-4111-8111-111111111111")
        );
    }

    #[test]
    fn mutation_is_blocked_while_save_is_pending() {
        let mut bridge = KernelBridge::default();
        bridge.create(CREATE).expect("create");
        bridge.prepare_save(SaveMode::Save).expect("prepare");
        let blocked = bridge.handle(br#"{"operation":"query-project","schemaVersion":1}"#);
        assert!(matches!(blocked, Err(BridgeError::PendingSaveExists)));
        bridge.abort_save().expect("abort");
        bridge.query().expect("query after abort");
    }
}
