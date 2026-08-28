use core::fmt;

use plc_core::{DecodeLimits, KernelSession, ProtocolError, Uuid, sha256};

use crate::system_bridge::{SystemBridge, SystemBridgeError, project_system_query};

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
    System(String),
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
            Self::System(message) => formatter.write_str(message),
        }
    }
}

impl From<ProtocolError> for BridgeError {
    fn from(value: ProtocolError) -> Self {
        Self::Protocol(value.to_string())
    }
}

impl From<SystemBridgeError> for BridgeError {
    fn from(value: SystemBridgeError) -> Self {
        Self::System(value.to_string())
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct KernelBridge {
    active: Option<KernelSession>,
    pending_save: Option<PendingSave>,
    system: Option<SystemBridge>,
}

impl KernelBridge {
    pub(crate) fn create(&mut self, request: &[u8]) -> Result<Vec<u8>, BridgeError> {
        check_input(request)?;
        let session = KernelSession::create(request)?;
        self.system = Some(SystemBridge::new(session.project()));
        self.active = Some(session);
        self.pending_save = None;
        self.query()
    }

    pub(crate) fn open(&mut self, package: &[u8]) -> Result<Vec<u8>, BridgeError> {
        check_input(package)?;
        let (session, _) = KernelSession::open(package, DecodeLimits::default())?;
        self.system = Some(SystemBridge::new(session.project()));
        self.active = Some(session);
        self.pending_save = None;
        self.query()
    }

    pub(crate) fn handle(&mut self, request: &[u8]) -> Result<Vec<u8>, BridgeError> {
        check_input(request)?;
        if self.pending_save.is_some() {
            return Err(BridgeError::PendingSaveExists);
        }
        let output = self
            .active
            .as_mut()
            .ok_or(BridgeError::NoActiveSession)?
            .handle(request)
            .map_err(BridgeError::from)?;
        let project = self
            .active
            .as_ref()
            .ok_or(BridgeError::NoActiveSession)?
            .project()
            .clone();
        self.system
            .as_mut()
            .ok_or(BridgeError::NoActiveSession)?
            .refresh_project(&project)?;
        Ok(output)
    }

    pub(crate) fn query(&mut self) -> Result<Vec<u8>, BridgeError> {
        self.active
            .as_mut()
            .ok_or(BridgeError::NoActiveSession)?
            .handle(QUERY_PROJECT)
            .map_err(Into::into)
    }

    pub(crate) fn system_query(&self) -> Result<Vec<u8>, BridgeError> {
        let active = self.active.as_ref().ok_or(BridgeError::NoActiveSession)?;
        let system = self.system.as_ref().ok_or(BridgeError::NoActiveSession)?;
        project_system_query(active.project(), system).map_err(Into::into)
    }

    pub(crate) fn system_command(&mut self, request: &[u8]) -> Result<Vec<u8>, BridgeError> {
        check_input(request)?;
        if self.pending_save.is_some() {
            return Err(BridgeError::PendingSaveExists);
        }
        self.active.as_ref().ok_or(BridgeError::NoActiveSession)?;
        self.system
            .as_mut()
            .ok_or(BridgeError::NoActiveSession)?
            .execute(request)
            .map_err(Into::into)
    }

    pub(crate) fn export_replay_baseline(&self) -> Result<Vec<u8>, BridgeError> {
        self.active.as_ref().ok_or(BridgeError::NoActiveSession)?;
        self.system
            .as_ref()
            .ok_or(BridgeError::NoActiveSession)?
            .export_replay_baseline()
            .map_err(Into::into)
    }

    pub(crate) fn verify_replay_package(&self, package: &[u8]) -> Result<Vec<u8>, BridgeError> {
        check_input(package)?;
        self.active.as_ref().ok_or(BridgeError::NoActiveSession)?;
        self.system
            .as_ref()
            .ok_or(BridgeError::NoActiveSession)?
            .verify_replay_package(package)
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
        let project = self
            .active
            .as_ref()
            .ok_or(BridgeError::NoActiveSession)?
            .project()
            .clone();
        self.system
            .as_mut()
            .ok_or(BridgeError::NoActiveSession)?
            .refresh_project(&project)?;
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
    pub extern "C" fn plc_session_system_query() -> i32 {
        match BRIDGE.with_borrow(KernelBridge::system_query) {
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
    pub extern "C" fn plc_session_system_command(length: u32) -> i32 {
        run_with_input(length, KernelBridge::system_command)
    }

    #[allow(unsafe_code)]
    #[unsafe(no_mangle)]
    pub extern "C" fn plc_session_export_replay_baseline() -> i32 {
        match BRIDGE.with_borrow(KernelBridge::export_replay_baseline) {
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
    pub extern "C" fn plc_session_verify_replay_package(length: u32) -> i32 {
        let input = match read_input(length) {
            Ok(input) => input,
            Err(error) => {
                write_error(error);
                return STATUS_ERROR;
            }
        };
        match BRIDGE.with_borrow(|bridge| bridge.verify_replay_package(&input)) {
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
        let mut output = String::from(r#"{"error":"#);
        crate::system_bridge::push_json_string(&mut output, &error.to_string());
        output.push('}');
        write_output(output.into_bytes());
    }

    fn write_output(output: Vec<u8>) {
        OUTPUT.with_borrow_mut(|destination| *destination = output);
    }
}

#[cfg(test)]
mod tests {
    use super::{BridgeError, KernelBridge, SaveMode};
    use crate::{system_bridge::SystemBridge, test_fixture::RuntimeFixture};
    use plc_core::{DecodeLimits, KernelSession, Project, Uuid, sha256};
    use plc_system::{ReplayDecodeLimits, ReplayPackage};

    const CREATE: &[u8] = br#"{"displayName":"Bridge","documentId":"cda496e1-165b-4ab0-9ddc-9ad749bf75a4","profile":{"id":"EDU-21 Core","manifestHash":"9febe00e579c161920610be4d2079621b6255217a623f29ee0f656fcd992ed9a","version":"1.0.0"},"rootId":"88c521b1-f9f7-4bb0-8dc1-adca746a13a6","schemaVersion":1}"#;

    fn bridge_from_project(project: &Project) -> KernelBridge {
        let session = KernelSession::from_project(project.clone()).expect("fixture kernel");
        KernelBridge {
            active: Some(session),
            pending_save: None,
            system: Some(SystemBridge::new(project)),
        }
    }

    fn runtime_command(operation: &str, fields: &[String], ordinal: u64) -> Vec<u8> {
        let command_id = Uuid::deterministic_v4(b"plc-wasm-command-id", ordinal).to_string();
        let idempotency = Uuid::deterministic_v4(b"plc-wasm-idempotency", ordinal).to_string();
        let author = Uuid::deterministic_v4(b"plc-wasm-author", 1).to_string();
        [
            vec![
                "PES-SYSTEM-COMMAND-1".to_owned(),
                operation.to_owned(),
                command_id,
                idempotency,
                author,
            ],
            fields.to_vec(),
        ]
        .concat()
        .join("\n")
        .into_bytes()
    }

    fn execute_runtime(
        bridge: &mut KernelBridge,
        operation: &str,
        fields: &[String],
        ordinal: u64,
    ) -> String {
        let output = bridge
            .system_command(&runtime_command(operation, fields, ordinal))
            .unwrap_or_else(|error| panic!("{operation} failed: {error}"));
        String::from_utf8(output).expect("runtime JSON")
    }

    fn json_string_field<'a>(input: &'a str, field: &str) -> &'a str {
        let marker = format!(r#""{field}":""#);
        let remainder = input
            .split_once(&marker)
            .unwrap_or_else(|| panic!("missing JSON field {field}"))
            .1;
        remainder
            .split_once('"')
            .unwrap_or_else(|| panic!("unterminated JSON field {field}"))
            .0
    }

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

    #[test]
    fn system_query_uses_the_active_canonical_project() {
        let mut bridge = KernelBridge::default();
        bridge.create(CREATE).expect("create");
        let query = bridge.system_query().expect("system query");
        let text = core::str::from_utf8(&query).expect("UTF-8 JSON");
        assert!(text.contains(r#""sourceDocumentHash":"#));
        assert!(text.contains(r#""sourceSemanticFingerprint":"#));
        assert!(text.contains(r#""code":"EDU-SYS-1001""#));
        assert!(text.contains(r#""canBuild":false"#));
        assert!(text.contains(r#""runtime":{"availability":"UNAVAILABLE""#));
        assert_eq!(query, bridge.system_query().expect("deterministic query"));
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn journey_a_runs_through_the_native_kernel_system_bridge() {
        let fixture = RuntimeFixture::canonical();
        let mut bridge = bridge_from_project(fixture.project());
        let initial = bridge
            .system
            .as_ref()
            .expect("system")
            .runtime_query()
            .expect("runtime query");
        let initial_text = String::from_utf8(initial.clone()).expect("runtime JSON");
        assert!(initial_text.contains(r#""availability":"READY""#));
        assert!(initial_text.contains(r#""canBuild":true"#));
        assert!(initial_text.contains(r#""cpuState":"POWERED_OFF""#));
        assert_eq!(
            initial,
            bridge
                .system
                .as_ref()
                .expect("system")
                .runtime_query()
                .expect("repeat query")
        );

        let built = execute_runtime(&mut bridge, "BUILD", &[], 1);
        assert!(built.contains(r#""buildCurrent":true"#));
        assert!(built.contains(r#""buildFingerprint":"#));
        let build_fingerprint = json_string_field(&built, "buildFingerprint").to_owned();
        execute_runtime(&mut bridge, "POWER_ON", &[], 2);
        let preview = execute_runtime(&mut bridge, "PREVIEW_LOAD", &["STOP".to_owned()], 3);
        assert!(preview.contains(r#""loadPreview":{"blockerCount":0"#));
        assert_eq!(
            json_string_field(&preview, "candidateFingerprint"),
            build_fingerprint
        );
        let loaded = execute_runtime(&mut bridge, "COMMIT_LOAD", &[], 4);
        assert!(loaded.contains(r#""loaded":true"#));
        assert!(loaded.contains(r#""loadPreview":null"#));
        assert_ne!(
            json_string_field(&loaded, "loadedArtifactFingerprint"),
            build_fingerprint
        );
        let online = execute_runtime(&mut bridge, "GO_ONLINE", &[], 5);
        assert!(online.contains(r#""online":true"#));
        execute_runtime(&mut bridge, "START_MONITORING", &[], 6);
        execute_runtime(&mut bridge, "REQUEST_RUN", &[], 7);

        let input = fixture.input_tag.to_string();
        execute_runtime(
            &mut bridge,
            "SET_RAW_INPUT",
            &[input, "BOOL".to_owned(), "true".to_owned()],
            8,
        );
        let scanned = execute_runtime(&mut bridge, "RUN_SCAN", &[], 9);
        assert!(scanned.contains(r#""cpuState":"RUN""#));
        assert!(scanned.contains(r#""rawInputValue":{"type":"BOOL","value":true}"#));
        assert!(scanned.contains(r#""deliveredOutputValue":{"type":"BOOL","value":true}"#));
        assert!(scanned.contains(r#""latestValue":{"type":"BOOL","value":true}"#));

        let output = fixture.output_tag.to_string();
        let modified = execute_runtime(
            &mut bridge,
            "MODIFY_ONCE",
            &[output.clone(), "BOOL".to_owned(), "false".to_owned()],
            10,
        );
        assert!(modified.contains(r#""effectiveValue":{"type":"BOOL","value":false}"#));
        let force_id = Uuid::deterministic_v4(b"plc-wasm-force", 1).to_string();
        let forced = execute_runtime(
            &mut bridge,
            "CREATE_FORCE",
            &[
                force_id.clone(),
                output,
                "BOOL".to_owned(),
                "true".to_owned(),
                "Journey A force".to_owned(),
            ],
            11,
        );
        assert!(forced.contains(r#""forceCount":1"#));
        assert!(forced.contains(r#""quality":"FORCED""#));
        let removed = execute_runtime(
            &mut bridge,
            "REMOVE_FORCE",
            &[force_id, "Journey A release".to_owned()],
            12,
        );
        assert!(removed.contains(r#""forceCount":0"#));

        let trace = execute_runtime(&mut bridge, "ARM_TRACE", &[fixture.trace.to_string()], 13);
        assert!(trace.contains(r#""state":"ARMED""#));
        let capturing = execute_runtime(&mut bridge, "RUN_SCAN", &[], 14);
        assert!(capturing.contains(r#""state":"CAPTURING""#));
        let traced = execute_runtime(&mut bridge, "RUN_SCAN", &[], 15);
        assert!(traced.contains(r#""captureCount":1"#));

        execute_runtime(&mut bridge, "REQUEST_STOP", &[], 16);
        let captured = execute_runtime(&mut bridge, "CAPTURE_SNAPSHOT", &[], 17);
        assert!(captured.contains(r#""snapshotAvailable":true"#));
        execute_runtime(
            &mut bridge,
            "SET_RAW_INPUT",
            &[
                fixture.input_tag.to_string(),
                "BOOL".to_owned(),
                "false".to_owned(),
            ],
            18,
        );
        let restored = execute_runtime(&mut bridge, "RESTORE_SNAPSHOT", &[], 19);
        assert!(restored.contains(r#""rawInputValue":{"type":"BOOL","value":true}"#));
        assert!(restored.contains(r#""snapshotAvailable":true"#));
    }

    #[test]
    fn invalid_runtime_and_pending_state_fail_closed() {
        let mut unavailable = KernelBridge::default();
        unavailable
            .create(CREATE)
            .expect("invalid project remains open");
        assert!(matches!(
            unavailable.system_command(&runtime_command("BUILD", &[], 1)),
            Err(BridgeError::System(_))
        ));

        let fixture = RuntimeFixture::canonical();
        let mut bridge = bridge_from_project(fixture.project());
        execute_runtime(&mut bridge, "BUILD", &[], 2);
        execute_runtime(&mut bridge, "POWER_ON", &[], 3);
        execute_runtime(&mut bridge, "PREVIEW_LOAD", &["STOP".to_owned()], 4);
        execute_runtime(&mut bridge, "POWER_OFF", &[], 5);
        assert!(matches!(
            bridge.system_command(&runtime_command("COMMIT_LOAD", &[], 6)),
            Err(BridgeError::System(message)) if message.contains("no verified virtual load preview")
        ));
        assert!(matches!(
            bridge.system_command(b"PES-SYSTEM-COMMAND-1\nBUILD"),
            Err(BridgeError::System(message)) if message.contains("malformed")
        ));
    }

    #[test]
    fn wasm_system_bridge_exports_and_executes_closed_replay_baseline() {
        let fixture = RuntimeFixture::canonical();
        let mut bridge = bridge_from_project(fixture.project());
        execute_runtime(&mut bridge, "BUILD", &[], 1);
        execute_runtime(&mut bridge, "POWER_ON", &[], 2);
        execute_runtime(&mut bridge, "PREVIEW_LOAD", &["STOP".to_owned()], 3);
        execute_runtime(&mut bridge, "COMMIT_LOAD", &[], 4);
        execute_runtime(&mut bridge, "GO_ONLINE", &[], 5);
        execute_runtime(&mut bridge, "CAPTURE_SNAPSHOT", &[], 6);

        let bytes = bridge
            .export_replay_baseline()
            .expect("production replay baseline export");
        let decoded = ReplayPackage::decode(&bytes, ReplayDecodeLimits::edu21())
            .expect("canonical replay package");
        assert!(decoded.events().is_empty());
        assert!(decoded.boundaries().is_empty());
        let result = bridge
            .verify_replay_package(&bytes)
            .expect("production bridge replay execution");
        let json = String::from_utf8(result).expect("replay result JSON");
        assert!(json.contains(r#""verified":true"#));
        assert!(json.contains(r#""observedBoundaryCount":0"#));
        assert!(json.contains(&decoded.content_fingerprint().to_hex()));
    }

    #[test]
    fn refresh_preserves_loaded_runtime_and_invalidates_stale_opaque_state() {
        let mut fixture = RuntimeFixture::canonical();
        let mut bridge = bridge_from_project(fixture.project());
        execute_runtime(&mut bridge, "BUILD", &[], 1);
        execute_runtime(&mut bridge, "POWER_ON", &[], 2);
        execute_runtime(&mut bridge, "PREVIEW_LOAD", &["STOP".to_owned()], 3);
        execute_runtime(&mut bridge, "COMMIT_LOAD", &[], 4);
        execute_runtime(&mut bridge, "GO_ONLINE", &[], 5);
        execute_runtime(&mut bridge, "REQUEST_RUN", &[], 6);
        execute_runtime(&mut bridge, "REQUEST_STOP", &[], 7);
        execute_runtime(&mut bridge, "CAPTURE_SNAPSHOT", &[], 8);

        fixture.make_hardware_invalid();
        let changed = fixture.project().clone();
        bridge
            .system
            .as_mut()
            .expect("system")
            .refresh_project(&changed)
            .expect("invalid offline project is adopted");
        bridge.active = Some(KernelSession::from_project(changed).expect("changed kernel"));
        let query = bridge.system_query().expect("invalid project query");
        let text = String::from_utf8(query).expect("system JSON");
        assert!(text.contains(r#""runtime":{"availability":"READY""#));
        assert!(text.contains(r#""canBuild":false"#));
        assert!(text.contains(r#""loaded":true"#));
        assert!(text.contains(r#""hardwareToLoaded":"MISMATCH""#));
        assert!(text.contains(r#""snapshotAvailable":false"#));
        assert!(matches!(
            bridge.system_command(&runtime_command("RESTORE_SNAPSHOT", &[], 9)),
            Err(BridgeError::System(message)) if message.contains("no aggregate runtime snapshot")
        ));
    }

    #[test]
    fn save_as_and_presentation_refresh_keep_compatible_runtime_state() {
        let mut fixture = RuntimeFixture::canonical();
        let mut bridge = bridge_from_project(fixture.project());
        execute_runtime(&mut bridge, "BUILD", &[], 1);
        execute_runtime(&mut bridge, "POWER_ON", &[], 2);
        execute_runtime(&mut bridge, "PREVIEW_LOAD", &["STOP".to_owned()], 3);
        execute_runtime(&mut bridge, "COMMIT_LOAD", &[], 4);
        execute_runtime(&mut bridge, "GO_ONLINE", &[], 5);
        execute_runtime(&mut bridge, "CAPTURE_SNAPSHOT", &[], 6);

        let package = bridge
            .prepare_save(SaveMode::SaveAs(Uuid::deterministic_v4(
                b"plc-wasm-save-as",
                1,
            )))
            .expect("Save As package");
        bridge
            .commit_save(package.len(), sha256(&package).0)
            .expect("Save As commit");
        let after_save = bridge
            .system
            .as_ref()
            .expect("system")
            .runtime_query()
            .expect("runtime after Save As");
        let after_save = String::from_utf8(after_save).expect("runtime JSON");
        assert!(after_save.contains(r#""loaded":true"#));
        assert!(after_save.contains(r#""snapshotAvailable":true"#));

        fixture.rename_program_presentation();
        bridge
            .system
            .as_mut()
            .expect("system")
            .refresh_project(fixture.project())
            .expect("presentation refresh");
        let after_presentation = bridge
            .system
            .as_ref()
            .expect("system")
            .runtime_query()
            .expect("presentation query");
        let after_presentation = String::from_utf8(after_presentation).expect("runtime JSON");
        assert!(after_presentation.contains(r#""buildCurrent":true"#));
        assert!(after_presentation.contains(r#""snapshotAvailable":true"#));
    }
}
