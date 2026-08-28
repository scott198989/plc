use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{self, Write},
};

use plc_commissioning::{
    CompatibilityClass, LoadPreview, MatchComparison, MemoryActionKind, PostLoadMode,
    StateActionKind,
};
use plc_core::{Lifecycle, ObjectId, Project, ProjectObjectKind, Sha256Digest, Uuid};
use plc_observability::{
    DiagnosticSeverity, DisplayBase, ForceId, MonitorState, Quality, RuntimeTarget, StableTargetId,
    TargetReference, TraceConfigId, TraceState,
};
use plc_runtime::{CanonicalValue, CpuState, Hash32, ValueType};
use plc_system::{
    ENGINEERING_REPLAY_ALGORITHM, EngineeringReadModel, EngineeringReplayError,
    EngineeringReplayExecutor, EngineeringSession, EngineeringSessionSnapshot, ProjectDiagnostic,
    ProjectDiagnosticPhase, ReplayDecodeLimits, ReplayPackage, ReplayPackageError,
    ReplayPackageSpec, RestoreApproval, SystemCommandIdentity, SystemError, project_hardware,
    project_software,
};

const SYSTEM_COMMAND_MAGIC: &str = "PES-SYSTEM-COMMAND-1";
const MAX_SYSTEM_COMMAND_BYTES: usize = 4_096;
const MAX_FORCE_REASON_CHARACTERS: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SystemBridgeError {
    CommandLimit,
    InvalidUtf8,
    MalformedCommand(&'static str),
    RuntimeUnavailable,
    NoPendingLoad,
    NoPendingSnapshot,
    System(String),
}

impl fmt::Display for SystemBridgeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CommandLimit => formatter.write_str("runtime command exceeds its wire budget"),
            Self::InvalidUtf8 => formatter.write_str("runtime command is not valid UTF-8"),
            Self::MalformedCommand(detail) => {
                write!(formatter, "runtime command is malformed: {detail}")
            }
            Self::RuntimeUnavailable => {
                formatter.write_str("no valid virtual controller runtime is available")
            }
            Self::NoPendingLoad => {
                formatter.write_str("no verified virtual load preview is pending")
            }
            Self::NoPendingSnapshot => {
                formatter.write_str("no aggregate runtime snapshot is pending")
            }
            Self::System(detail) => write!(
                formatter,
                "virtual engineering system rejected the request: {detail}"
            ),
        }
    }
}

impl From<SystemError> for SystemBridgeError {
    fn from(value: SystemError) -> Self {
        Self::System(format!("{value:?}"))
    }
}

impl From<ReplayPackageError> for SystemBridgeError {
    fn from(value: ReplayPackageError) -> Self {
        Self::System(value.to_string())
    }
}

impl From<EngineeringReplayError> for SystemBridgeError {
    fn from(value: EngineeringReplayError) -> Self {
        Self::System(value.to_string())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SystemBridge {
    session: Option<EngineeringSession>,
    controller_object_id: Option<ObjectId>,
    pending_load: Option<LoadPreview>,
    pending_snapshot: Option<EngineeringSessionSnapshot>,
    source_document_hash: Sha256Digest,
    source_semantic_fingerprint: Sha256Digest,
    unavailable_project: Option<Project>,
    unavailable_reason: Option<String>,
}

impl SystemBridge {
    pub(crate) fn new(project: &Project) -> Self {
        let mut value = Self {
            session: None,
            controller_object_id: None,
            pending_load: None,
            pending_snapshot: None,
            source_document_hash: project.document_hash(),
            source_semantic_fingerprint: project.semantic_fingerprint(),
            unavailable_project: Some(project.clone()),
            unavailable_reason: None,
        };
        value.replace_session(project);
        value
    }

    pub(crate) fn refresh_project(&mut self, project: &Project) -> Result<(), SystemBridgeError> {
        if self.source_document_hash == project.document_hash() {
            return Ok(());
        }
        let semantic_changed = self.source_semantic_fingerprint != project.semantic_fingerprint();
        self.pending_load = None;
        if semantic_changed {
            self.pending_snapshot = None;
        }

        let active_controllers = active_controllers(project);
        let current_controller_present = self
            .controller_object_id
            .is_some_and(|current| active_controllers.contains(&current));
        let preserve_existing = self
            .session
            .as_ref()
            .is_some_and(|session| session.status().loaded || current_controller_present);

        if preserve_existing {
            self.session
                .as_mut()
                .ok_or(SystemBridgeError::RuntimeUnavailable)?
                .refresh_project(project.clone())?;
            self.unavailable_project = None;
            self.unavailable_reason = None;
        } else {
            self.replace_session(project);
        }
        self.source_document_hash = project.document_hash();
        self.source_semantic_fingerprint = project.semantic_fingerprint();
        Ok(())
    }

    pub(crate) fn execute(&mut self, input: &[u8]) -> Result<Vec<u8>, SystemBridgeError> {
        let command = parse_system_command(input)?;
        match command.operation {
            SystemOperation::Build => {
                self.pending_load = None;
                self.session_mut()?.build()?;
                self.pending_snapshot = None;
            }
            SystemOperation::PowerOn => {
                self.pending_load = None;
                self.session_mut()?.power_on()?;
            }
            SystemOperation::PowerOff => {
                self.pending_load = None;
                self.session_mut()?.power_off()?;
            }
            SystemOperation::PreviewLoad(mode) => {
                let preview = self.session_ref()?.preview_load(mode)?;
                self.pending_load = Some(preview);
            }
            SystemOperation::CommitLoad => {
                let preview = self
                    .pending_load
                    .take()
                    .ok_or(SystemBridgeError::NoPendingLoad)?;
                self.session_mut()?.commit_load(&preview)?;
                self.pending_snapshot = None;
            }
            SystemOperation::GoOnline => {
                self.pending_load = None;
                self.session_mut()?.go_online()?;
            }
            SystemOperation::RequestRun => {
                self.pending_load = None;
                self.session_mut()?.request_run()?;
            }
            SystemOperation::RequestStop => {
                self.pending_load = None;
                self.session_mut()?.request_stop()?;
            }
            SystemOperation::RunScan => {
                self.pending_load = None;
                self.session_mut()?.run_scan(command.identity)?;
            }
            SystemOperation::StartMonitoring => {
                self.pending_load = None;
                self.session_mut()?.start_monitoring()?;
            }
            SystemOperation::SetRawInput { target, value } => {
                self.pending_load = None;
                self.session_mut()?
                    .set_raw_virtual_input(command.identity, target, value)?;
            }
            SystemOperation::ModifyOnce { target, value } => {
                self.pending_load = None;
                self.session_mut()?
                    .modify_once(command.identity, target, value)?;
            }
            SystemOperation::CreateForce {
                force,
                target,
                value,
                reason,
            } => {
                self.pending_load = None;
                self.session_mut()?
                    .create_force(command.identity, force, target, value, reason)?;
            }
            SystemOperation::RemoveForce { force, reason } => {
                self.pending_load = None;
                self.session_mut()?
                    .remove_force(command.identity, force, reason)?;
            }
            SystemOperation::ArmTrace(config) => {
                self.pending_load = None;
                self.session_mut()?.arm_trace(config)?;
            }
            SystemOperation::CaptureSnapshot => {
                self.pending_snapshot = Some(self.session_ref()?.capture_snapshot()?);
            }
            SystemOperation::RestoreSnapshot => {
                self.pending_load = None;
                let snapshot = self
                    .pending_snapshot
                    .clone()
                    .ok_or(SystemBridgeError::NoPendingSnapshot)?;
                let preview = self.session_ref()?.preview_restore(&snapshot)?;
                if let Err(error) = self.session_mut()?.commit_restore(
                    &snapshot,
                    &preview,
                    RestoreApproval::approve(&preview),
                ) {
                    self.pending_snapshot = None;
                    return Err(error.into());
                }
            }
        }
        self.runtime_query()
    }

    pub(crate) fn runtime_query(&self) -> Result<Vec<u8>, SystemBridgeError> {
        Ok(self.runtime_query_string()?.into_bytes())
    }

    /// Exports the current captured aggregate snapshot as a canonical closed
    /// replay baseline. It contains no vendor artifact or deployable payload.
    pub(crate) fn export_replay_baseline(&self) -> Result<Vec<u8>, SystemBridgeError> {
        let snapshot = self
            .pending_snapshot
            .as_ref()
            .ok_or(SystemBridgeError::NoPendingSnapshot)?;
        let session = self.session_ref()?;
        let read = session.read_model()?;
        let profile = read
            .profile_fingerprint
            .ok_or(SystemBridgeError::RuntimeUnavailable)?;
        let runtime = session
            .universe()
            .controller(read.runtime_controller_id)
            .map(plc_commissioning::ControllerInstance::runtime)
            .ok_or(SystemBridgeError::RuntimeUnavailable)?;
        let package = ReplayPackage::encode(ReplayPackageSpec::edu21(
            snapshot,
            snapshot.loaded_artifact_fingerprint,
            profile,
            runtime.deterministic_seed(),
            ENGINEERING_REPLAY_ALGORITHM,
            Vec::new(),
            Vec::new(),
        ))?;
        Ok(package.bytes().to_vec())
    }

    /// Reconstructs the captured aggregate snapshot and executes a bounded
    /// replay package through the production simulator ingress interpreter.
    pub(crate) fn verify_replay_package(&self, bytes: &[u8]) -> Result<Vec<u8>, SystemBridgeError> {
        let snapshot = self
            .pending_snapshot
            .as_ref()
            .ok_or(SystemBridgeError::NoPendingSnapshot)?;
        let controller = self
            .controller_object_id
            .ok_or(SystemBridgeError::RuntimeUnavailable)?;
        let package = ReplayPackage::decode(bytes, ReplayDecodeLimits::edu21())?;
        let execution = EngineeringReplayExecutor::execute(
            self.project().clone(),
            controller,
            snapshot,
            &package,
        )?;
        let mut output = String::with_capacity(512);
        output.push_str(r#"{"contentFingerprint":"#);
        push_json_string(&mut output, &package.content_fingerprint().to_hex());
        output.push_str(r#","divergence":"#);
        if let Some(divergence) = &execution.divergence {
            output.push_str(r#"{"boundaryIndex":"#);
            write!(output, "{}", divergence.boundary_index).expect("write to String");
            output.push_str(r#","expectedStateHash":"#);
            match divergence.expected_state_hash {
                Some(hash) => push_json_string(&mut output, &hash.to_hex()),
                None => output.push_str("null"),
            }
            output.push_str(r#","observedStateHash":"#);
            match divergence.observed_state_hash {
                Some(hash) => push_json_string(&mut output, &hash.to_hex()),
                None => output.push_str("null"),
            }
            output.push('}');
        } else {
            output.push_str("null");
        }
        output.push_str(r#","finalSnapshotHash":"#);
        push_json_string(&mut output, &execution.final_snapshot.content_hash.to_hex());
        output.push_str(r#","observedBoundaryCount":"#);
        write!(output, "{}", execution.observed_boundaries.len()).expect("write to String");
        output.push_str(r#","schemaVersion":1,"verified":"#);
        push_bool(&mut output, execution.divergence.is_none());
        output.push('}');
        Ok(output.into_bytes())
    }

    fn runtime_query_string(&self) -> Result<String, SystemBridgeError> {
        let mut output = String::with_capacity(8_192);
        let diagnostics = self.runtime_projection_diagnostics();
        output.push_str(r#"{"availability":"#);
        push_json_string(
            &mut output,
            if self.session.is_some() {
                "READY"
            } else {
                "UNAVAILABLE"
            },
        );
        output.push_str(r#","canBuild":"#);
        push_bool(&mut output, self.can_build());
        output.push_str(r#","diagnostics":["#);
        push_runtime_projection_diagnostics(&mut output, &diagnostics);
        output.push_str(r#"],"reason":"#);
        match &self.unavailable_reason {
            Some(reason) if self.session.is_none() => push_json_string(&mut output, reason),
            _ => output.push_str("null"),
        }
        output.push_str(r#","schemaVersion":1,"session":"#);
        if let Some(session) = &self.session {
            let read = session.read_model()?;
            push_runtime_session(
                &mut output,
                session,
                &read,
                self.pending_load.as_ref(),
                self.pending_snapshot.is_some(),
            );
        } else {
            output.push_str("null");
        }
        output.push_str(r#","sourceDocumentHash":"#);
        push_json_string(&mut output, &self.source_document_hash.to_hex());
        output.push_str(r#","sourceSemanticFingerprint":"#);
        push_json_string(&mut output, &self.source_semantic_fingerprint.to_hex());
        output.push('}');
        Ok(output)
    }

    fn replace_session(&mut self, project: &Project) {
        let controllers = active_controllers(project);
        if controllers.len() != 1 {
            self.session = None;
            self.controller_object_id = None;
            self.unavailable_project = Some(project.clone());
            self.unavailable_reason = Some(format!(
                "Configure exactly one active fictional controller; found {}.",
                controllers.len()
            ));
            return;
        }
        let controller = controllers[0];
        match EngineeringSession::new(project.clone(), controller) {
            Ok(session) => {
                self.session = Some(session);
                self.controller_object_id = Some(controller);
                self.unavailable_project = None;
                self.unavailable_reason = None;
            }
            Err(error) => {
                self.session = None;
                self.controller_object_id = None;
                self.unavailable_project = Some(project.clone());
                self.unavailable_reason = Some(format!(
                    "The canonical virtual controller is unavailable: {error:?}."
                ));
            }
        }
    }

    fn runtime_projection_diagnostics(&self) -> Vec<ProjectDiagnostic> {
        let mut diagnostics = project_hardware(self.project()).diagnostics().to_vec();
        if let Some(controller) = self.controller_object_id {
            diagnostics
                .extend_from_slice(project_software(self.project(), controller).diagnostics());
        }
        if let Some(session) = &self.session {
            diagnostics.extend_from_slice(session.projection_diagnostics());
        }
        diagnostics.sort();
        diagnostics.dedup();
        diagnostics
    }

    fn can_build(&self) -> bool {
        project_hardware(self.project()).can_build()
    }

    fn project(&self) -> &Project {
        self.session
            .as_ref()
            .map(EngineeringSession::project)
            .or(self.unavailable_project.as_ref())
            .expect("a system bridge always retains canonical project truth")
    }

    fn session_ref(&self) -> Result<&EngineeringSession, SystemBridgeError> {
        self.session
            .as_ref()
            .ok_or(SystemBridgeError::RuntimeUnavailable)
    }

    fn session_mut(&mut self) -> Result<&mut EngineeringSession, SystemBridgeError> {
        self.session
            .as_mut()
            .ok_or(SystemBridgeError::RuntimeUnavailable)
    }
}

pub(crate) fn project_system_query(
    project: &Project,
    system: &SystemBridge,
) -> Result<Vec<u8>, SystemBridgeError> {
    let projection = project_hardware(project);
    let artifact = projection.artifact();
    let mut output = String::with_capacity(1_024 + projection.diagnostics().len() * 192);
    output.push_str(r#"{"allocationChangeCount":"#);
    write!(
        output,
        "{}",
        projection
            .allocation_preview()
            .map_or(0, |preview| preview.changes.len())
    )
    .expect("write to String");
    output.push_str(r#","artifactFingerprint":"#);
    if let Some(artifact) = artifact {
        push_json_string(&mut output, &artifact.hardware_fingerprint.to_hex());
    } else {
        output.push_str("null");
    }
    output.push_str(r#","canBuild":"#);
    push_bool(&mut output, projection.can_build());
    output.push_str(r#","channelBindingCount":"#);
    write!(
        output,
        "{}",
        artifact.map_or(0, |artifact| artifact.channel_bindings.len())
    )
    .expect("write to String");
    output.push_str(r#","diagnostics":["#);
    for (index, diagnostic) in projection.diagnostics().iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(r#"{"blocking":"#);
        push_bool(&mut output, diagnostic.blocking);
        output.push_str(r#","code":"#);
        push_json_string(&mut output, &diagnostic.code);
        output.push_str(r#","message":"#);
        push_json_string(&mut output, &diagnostic.message);
        output.push_str(r#","phase":"#);
        push_json_string(&mut output, diagnostic_phase(diagnostic.phase));
        output.push_str(r#","primaryObjectId":"#);
        push_json_string(&mut output, &diagnostic.primary_object_id.to_string());
        output.push_str(r#","relatedObjectIds":["#);
        for (related_index, related) in diagnostic.related_object_ids.iter().enumerate() {
            if related_index > 0 {
                output.push(',');
            }
            push_json_string(&mut output, &related.to_string());
        }
        output.push_str("]}");
    }
    output.push_str(r#"],"profile":{"id":"#);
    push_json_string(&mut output, projection.profile().id());
    output.push_str(r#","manifestHash":"#);
    push_json_string(&mut output, &projection.profile().manifest_hash().to_hex());
    output.push_str(r#","version":"#);
    push_json_string(&mut output, projection.profile().version());
    output.push_str(r#"},"runtime":"#);
    output.push_str(&system.runtime_query_string()?);
    output.push_str(r#","schemaVersion":1,"sourceDocumentHash":"#);
    push_json_string(&mut output, &projection.source_document_hash().to_hex());
    output.push_str(r#","sourceSemanticFingerprint":"#);
    push_json_string(
        &mut output,
        &projection.source_semantic_fingerprint().to_hex(),
    );
    output.push('}');
    Ok(output.into_bytes())
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SystemCommand {
    identity: SystemCommandIdentity,
    operation: SystemOperation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SystemOperation {
    Build,
    PowerOn,
    PowerOff,
    PreviewLoad(PostLoadMode),
    CommitLoad,
    GoOnline,
    RequestRun,
    RequestStop,
    RunScan,
    StartMonitoring,
    SetRawInput {
        target: StableTargetId,
        value: CanonicalValue,
    },
    ModifyOnce {
        target: StableTargetId,
        value: CanonicalValue,
    },
    CreateForce {
        force: ForceId,
        target: StableTargetId,
        value: CanonicalValue,
        reason: String,
    },
    RemoveForce {
        force: ForceId,
        reason: String,
    },
    ArmTrace(TraceConfigId),
    CaptureSnapshot,
    RestoreSnapshot,
}

fn parse_system_command(input: &[u8]) -> Result<SystemCommand, SystemBridgeError> {
    if input.is_empty() || input.len() > MAX_SYSTEM_COMMAND_BYTES {
        return Err(SystemBridgeError::CommandLimit);
    }
    let text = core::str::from_utf8(input).map_err(|_| SystemBridgeError::InvalidUtf8)?;
    if text.contains(['\0', '\r']) {
        return Err(SystemBridgeError::MalformedCommand(
            "forbidden control character",
        ));
    }
    let fields = text.split('\n').collect::<Vec<_>>();
    if fields.len() < 5 || fields[0] != SYSTEM_COMMAND_MAGIC {
        return Err(SystemBridgeError::MalformedCommand(
            "invalid command envelope",
        ));
    }
    let identity = SystemCommandIdentity {
        command_id: parse_wire_uuid(fields[2])?,
        idempotency_key: parse_wire_uuid(fields[3])?,
        author_identity: parse_wire_uuid(fields[4])?,
    };
    let operation = match fields[1] {
        "BUILD" => exact_operation_fields(&fields, 5, SystemOperation::Build)?,
        "POWER_ON" => exact_operation_fields(&fields, 5, SystemOperation::PowerOn)?,
        "POWER_OFF" => exact_operation_fields(&fields, 5, SystemOperation::PowerOff)?,
        "COMMIT_LOAD" => exact_operation_fields(&fields, 5, SystemOperation::CommitLoad)?,
        "GO_ONLINE" => exact_operation_fields(&fields, 5, SystemOperation::GoOnline)?,
        "REQUEST_RUN" => exact_operation_fields(&fields, 5, SystemOperation::RequestRun)?,
        "REQUEST_STOP" => exact_operation_fields(&fields, 5, SystemOperation::RequestStop)?,
        "RUN_SCAN" => exact_operation_fields(&fields, 5, SystemOperation::RunScan)?,
        "START_MONITORING" => exact_operation_fields(&fields, 5, SystemOperation::StartMonitoring)?,
        "CAPTURE_SNAPSHOT" => exact_operation_fields(&fields, 5, SystemOperation::CaptureSnapshot)?,
        "RESTORE_SNAPSHOT" => exact_operation_fields(&fields, 5, SystemOperation::RestoreSnapshot)?,
        "PREVIEW_LOAD" => {
            require_field_count(&fields, 6)?;
            SystemOperation::PreviewLoad(match fields[5] {
                "STOP" => PostLoadMode::Stop,
                "RUN" => PostLoadMode::Run,
                _ => {
                    return Err(SystemBridgeError::MalformedCommand(
                        "invalid post-load mode",
                    ));
                }
            })
        }
        "SET_RAW_INPUT" | "MODIFY_ONCE" => {
            require_field_count(&fields, 8)?;
            let target = StableTargetId(parse_wire_uuid(fields[5])?);
            let value = parse_wire_value(fields[6], fields[7])?;
            if fields[1] == "SET_RAW_INPUT" {
                SystemOperation::SetRawInput { target, value }
            } else {
                SystemOperation::ModifyOnce { target, value }
            }
        }
        "CREATE_FORCE" => {
            require_field_count(&fields, 10)?;
            SystemOperation::CreateForce {
                force: ForceId(parse_wire_uuid(fields[5])?),
                target: StableTargetId(parse_wire_uuid(fields[6])?),
                value: parse_wire_value(fields[7], fields[8])?,
                reason: parse_reason(fields[9])?,
            }
        }
        "REMOVE_FORCE" => {
            require_field_count(&fields, 7)?;
            SystemOperation::RemoveForce {
                force: ForceId(parse_wire_uuid(fields[5])?),
                reason: parse_reason(fields[6])?,
            }
        }
        "ARM_TRACE" => {
            require_field_count(&fields, 6)?;
            SystemOperation::ArmTrace(TraceConfigId(parse_wire_uuid(fields[5])?))
        }
        _ => {
            return Err(SystemBridgeError::MalformedCommand(
                "unsupported operation token",
            ));
        }
    };
    Ok(SystemCommand {
        identity,
        operation,
    })
}

fn exact_operation_fields(
    fields: &[&str],
    expected: usize,
    operation: SystemOperation,
) -> Result<SystemOperation, SystemBridgeError> {
    require_field_count(fields, expected)?;
    Ok(operation)
}

fn require_field_count(fields: &[&str], expected: usize) -> Result<(), SystemBridgeError> {
    if fields.len() == expected {
        Ok(())
    } else {
        Err(SystemBridgeError::MalformedCommand("invalid field count"))
    }
}

fn parse_wire_uuid(value: &str) -> Result<u128, SystemBridgeError> {
    let uuid = Uuid::parse(value)
        .map_err(|_| SystemBridgeError::MalformedCommand("invalid UUID field"))?;
    Ok(u128::from_be_bytes(uuid.into_bytes()))
}

fn parse_wire_value(kind: &str, value: &str) -> Result<CanonicalValue, SystemBridgeError> {
    match kind {
        "BOOL" => match value {
            "true" => Ok(CanonicalValue::Bool(true)),
            "false" => Ok(CanonicalValue::Bool(false)),
            _ => Err(SystemBridgeError::MalformedCommand("invalid BOOL value")),
        },
        "I32" if canonical_integer(value, true) => value
            .parse::<i32>()
            .map(CanonicalValue::I32)
            .map_err(|_| SystemBridgeError::MalformedCommand("I32 value is outside its range")),
        "I64" if canonical_integer(value, true) => value
            .parse::<i64>()
            .map(CanonicalValue::I64)
            .map_err(|_| SystemBridgeError::MalformedCommand("I64 value is outside its range")),
        "U32" if canonical_integer(value, false) => value
            .parse::<u32>()
            .map(CanonicalValue::U32)
            .map_err(|_| SystemBridgeError::MalformedCommand("U32 value is outside its range")),
        "TIME_MS" if canonical_integer(value, true) => value
            .parse::<i64>()
            .map(CanonicalValue::TimeMs)
            .map_err(|_| SystemBridgeError::MalformedCommand("TIME_MS value is outside its range")),
        "I32" | "I64" | "U32" | "TIME_MS" => Err(SystemBridgeError::MalformedCommand(
            "numeric value is not canonical decimal text",
        )),
        _ => Err(SystemBridgeError::MalformedCommand(
            "unsupported runtime value type",
        )),
    }
}

fn canonical_integer(value: &str, signed: bool) -> bool {
    let digits = if signed {
        value.strip_prefix('-').unwrap_or(value)
    } else {
        value
    };
    !digits.is_empty()
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && (digits == "0" || !digits.starts_with('0'))
        && (signed || !value.starts_with('-'))
}

fn parse_reason(value: &str) -> Result<String, SystemBridgeError> {
    if value.is_empty()
        || value.chars().count() > MAX_FORCE_REASON_CHARACTERS
        || value.contains(['\0', '\r', '\n', '\t'])
    {
        return Err(SystemBridgeError::MalformedCommand("invalid force reason"));
    }
    Ok(value.to_owned())
}

fn active_controllers(project: &Project) -> Vec<ObjectId> {
    project
        .objects()
        .filter(|object| {
            object.lifecycle == Lifecycle::Active && object.kind == ProjectObjectKind::Controller
        })
        .map(|object| object.id)
        .collect()
}

fn push_runtime_projection_diagnostics(output: &mut String, diagnostics: &[ProjectDiagnostic]) {
    for (index, diagnostic) in diagnostics.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(r#"{"blocking":"#);
        push_bool(output, diagnostic.blocking);
        output.push_str(r#","code":"#);
        push_json_string(output, &diagnostic.code);
        output.push_str(r#","message":"#);
        push_json_string(output, &diagnostic.message);
        output.push_str(r#","objectId":"#);
        push_json_string(output, &diagnostic.primary_object_id.to_string());
        output.push('}');
    }
}

#[allow(clippy::too_many_lines)]
fn push_runtime_session(
    output: &mut String,
    session: &EngineeringSession,
    read: &EngineeringReadModel,
    pending_load: Option<&LoadPreview>,
    snapshot_available: bool,
) {
    output.push_str(r#"{"buildCurrent":"#);
    push_bool(output, read.status.build_current);
    output.push_str(r#","buildFingerprint":"#);
    push_optional_string(
        output,
        session
            .current_build()
            .map(|build| build.load_package().fingerprint().to_hex()),
    );
    output.push_str(r#","controllerEpoch":"#);
    push_json_string(output, &read.controller_epoch.to_string());
    output.push_str(r#","controllerObjectId":"#);
    push_json_string(output, &read.controller_object_id.to_string());
    output.push_str(r#","cpuState":"#);
    push_json_string(output, cpu_state(read.cpu_state));
    output.push_str(r#","diagnosticReplayHash":"#);
    push_json_string(output, &read.diagnostic_replay_hash.to_hex());
    output.push_str(r#","diagnostics":["#);
    push_runtime_diagnostics(output, session, read);
    output.push_str(r#"],"documentDirty":"#);
    push_bool(output, read.status.document_dirty);
    output.push_str(r#","forceCount":"#);
    write!(output, "{}", read.forces.count).expect("write to String");
    output.push_str(r#","forceRegistryVersion":"#);
    push_json_string(output, &read.forces.registry_version.to_string());
    output.push_str(r#","forces":["#);
    push_forces(output, read);
    output.push_str(r#"],"hardwareToLoaded":"#);
    push_optional_string(
        output,
        read.status
            .hardware_to_loaded
            .map(match_comparison)
            .map(str::to_owned),
    );
    output.push_str(r#","hashes":"#);
    push_runtime_hashes(output, read);
    output.push_str(r#","loadPreview":"#);
    if let Some(preview) = pending_load {
        push_load_preview(output, preview);
    } else {
        output.push_str("null");
    }
    output.push_str(r#","loaded":"#);
    push_bool(output, read.status.loaded);
    output.push_str(r#","loadedArtifactFingerprint":"#);
    push_optional_string(output, read.loaded_artifact_fingerprint.map(Hash32::to_hex));
    output.push_str(r#","monitorState":"#);
    push_json_string(
        output,
        monitor_state(read.status.monitor_state, read.status.build_current),
    );
    output.push_str(r#","online":"#);
    push_bool(output, read.status.online);
    output.push_str(r#","probes":["#);
    push_probes(output, read);
    output.push_str(r#"],"runtimeControllerId":"#);
    push_json_string(output, &wire_uuid(read.runtime_controller_id.0));
    output.push_str(r#","runtimeReplayHash":"#);
    push_json_string(output, &read.runtime_replay_hash.to_hex());
    output.push_str(r#","scanSequence":"#);
    push_json_string(output, &read.scan_sequence.to_string());
    output.push_str(r#","snapshotAvailable":"#);
    push_bool(output, snapshot_available);
    output.push_str(r#","softwareToLoaded":"#);
    push_optional_string(
        output,
        read.status
            .software_to_loaded
            .map(match_comparison)
            .map(str::to_owned),
    );
    output.push_str(r#","traces":["#);
    push_traces(output, read);
    output.push_str(r#"],"universeEpoch":"#);
    push_json_string(output, &read.universe_epoch.to_string());
    output.push_str(r#","universeId":"#);
    push_json_string(output, &wire_uuid(read.universe_id.0));
    output.push_str(r#","virtualTimeMilliseconds":"#);
    push_json_string(output, &read.virtual_time_ms.to_string());
    output.push_str(r#","watches":["#);
    push_watches(output, read);
    output.push_str("]}");
}

fn push_probes(output: &mut String, read: &EngineeringReadModel) {
    for (index, probe) in read.probes.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(r#"{"committedOutputValue":"#);
        push_optional_value(output, probe.committed_output_value);
        output.push_str(r#","deliveredOutputValue":"#);
        push_optional_value(output, probe.delivered_output_value);
        output.push_str(r#","displayName":"#);
        push_json_string(output, &probe.display_name);
        output.push_str(r#","effectiveValue":"#);
        push_optional_value(output, probe.effective_value);
        output.push_str(r#","forcedValue":"#);
        push_optional_value(output, probe.forced_value);
        output.push_str(r#","id":"#);
        push_json_string(output, &wire_uuid(probe.identity.0));
        output.push_str(r#","kind":"#);
        push_json_string(output, runtime_target_kind(probe.runtime_target));
        output.push_str(r#","naturalValue":"#);
        push_optional_value(output, probe.natural_value);
        output.push_str(r#","quality":"#);
        push_json_string(
            output,
            runtime_quality(probe.quality, probe.forced_value.is_some()),
        );
        output.push_str(r#","rawInputValue":"#);
        push_optional_value(output, probe.raw_input_value);
        output.push_str(r#","runtimeAddress":"#);
        push_json_string(output, &runtime_address(probe.runtime_target));
        output.push_str(r#","valueType":"#);
        push_json_string(output, value_type(probe.value_type));
        output.push('}');
    }
}

fn push_forces(output: &mut String, read: &EngineeringReadModel) {
    for (index, projected) in read.forces.entries.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        let force = &projected.force;
        output.push_str(r#"{"forceId":"#);
        push_json_string(output, &wire_uuid(force.id.0));
        output.push_str(r#","reason":"#);
        push_json_string(output, &force.reason);
        output.push_str(r#","targetId":"#);
        push_json_string(output, &wire_uuid(force.target_id.0));
        output.push_str(r#","value":"#);
        push_value(output, force.value);
        output.push('}');
    }
}

fn push_watches(output: &mut String, read: &EngineeringReadModel) {
    for (table_index, table) in read.watches.iter().enumerate() {
        if table_index > 0 {
            output.push(',');
        }
        let latest = table
            .latest_samples
            .iter()
            .copied()
            .collect::<BTreeMap<_, _>>();
        output.push_str(r#"{"id":"#);
        push_json_string(output, &wire_uuid(table.table.id.0));
        output.push_str(r#","name":"#);
        push_json_string(output, &table.table.name);
        output.push_str(r#","rows":["#);
        for (row_index, row) in table.table.rows.iter().enumerate() {
            if row_index > 0 {
                output.push(',');
            }
            let sample = latest.get(&row.id).and_then(|value| *value);
            let target = sample.map_or_else(
                || match &row.target {
                    TargetReference::Stable(value) => *value,
                    TargetReference::SourceOnly(_) => StableTargetId(row.id.0),
                },
                |value| value.target_id,
            );
            output.push_str(r#"{"displayBase":"#);
            push_json_string(output, display_base(row.display_base));
            output.push_str(r#","latestValue":"#);
            if let Some(sample) = sample {
                push_value(output, sample.value);
            } else {
                output.push_str("null");
            }
            output.push_str(r#","quality":"#);
            if let Some(sample) = sample {
                push_json_string(
                    output,
                    runtime_quality(sample.quality, sample.force.is_some()),
                );
            } else {
                output.push_str("null");
            }
            output.push_str(r#","rowId":"#);
            push_json_string(output, &wire_uuid(row.id.0));
            output.push_str(r#","targetId":"#);
            push_json_string(output, &wire_uuid(target.0));
            output.push('}');
        }
        output.push_str("]}");
    }
}

fn push_traces(output: &mut String, read: &EngineeringReadModel) {
    for (index, trace) in read.traces.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(r#"{"captureCount":"#);
        write!(output, "{}", trace.captures.len()).expect("write to String");
        output.push_str(r#","id":"#);
        push_json_string(output, &wire_uuid(trace.config.id.0));
        output.push_str(r#","name":"#);
        push_json_string(output, &trace.config.name);
        output.push_str(r#","state":"#);
        push_json_string(output, trace_state(trace.state));
        output.push('}');
    }
}

fn push_runtime_diagnostics(
    output: &mut String,
    session: &EngineeringSession,
    read: &EngineeringReadModel,
) {
    let active = read
        .diagnostics
        .active
        .iter()
        .map(|condition| condition.incoming_occurrence_id.0)
        .collect::<BTreeSet<_>>();
    let navigation = read
        .diagnostics
        .navigation
        .iter()
        .map(|(identity, result)| (*identity, result.as_ref()))
        .collect::<BTreeMap<_, _>>();
    for (index, event) in read.diagnostics.retained.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        let definition = session
            .diagnostic_ledger()
            .registry()
            .definition(event.definition_id);
        output.push_str(r#"{"active":"#);
        push_bool(output, active.contains(&event.occurrence_id.0));
        output.push_str(r#","code":"#);
        push_json_string(
            output,
            definition.map_or("EDU-UNKNOWN", |value| value.code.0),
        );
        output.push_str(r#","message":"#);
        push_json_string(
            output,
            definition.map_or("Unknown virtual controller diagnostic", |value| {
                value.mnemonic
            }),
        );
        output.push_str(r#","navigationObjectId":"#);
        if let Some(Some(result)) = navigation.get(&event.occurrence_id.0) {
            push_json_string(output, &wire_uuid(result.primary.identity.0));
        } else {
            output.push_str("null");
        }
        output.push_str(r#","occurrenceId":"#);
        push_json_string(output, &wire_uuid(event.occurrence_id.0));
        output.push_str(r#","severity":"#);
        push_json_string(output, diagnostic_severity(event.severity));
        output.push('}');
    }
}

fn push_runtime_hashes(output: &mut String, read: &EngineeringReadModel) {
    if let Some(hashes) = &read.snapshot_hashes {
        output.push_str(r#"{"controllerState":"#);
        push_json_string(output, &hashes.controller_state_hash.to_hex());
        output.push_str(r#","diagnosticReplay":"#);
        push_json_string(output, &hashes.diagnostic_replay_hash.to_hex());
        output.push_str(r#","runtimeReplay":"#);
        push_json_string(output, &hashes.runtime_replay_hash.to_hex());
        output.push_str(r#","universeState":"#);
        push_json_string(output, &hashes.universe_state_hash.to_hex());
        output.push('}');
    } else {
        output.push_str("null");
    }
}

fn push_load_preview(output: &mut String, preview: &LoadPreview) {
    let initialization_count = preview
        .memory_actions()
        .iter()
        .filter(|action| action.kind == MemoryActionKind::Initialize)
        .count()
        + preview
            .state_actions()
            .iter()
            .filter(|action| action.kind == StateActionKind::Initialize)
            .count();
    let removal_count = preview
        .memory_actions()
        .iter()
        .filter(|action| action.kind == MemoryActionKind::Remove)
        .count()
        + preview
            .state_actions()
            .iter()
            .filter(|action| action.kind == StateActionKind::Remove)
            .count();
    output.push_str(r#"{"blockerCount":"#);
    write!(output, "{}", preview.blockers().len()).expect("write to String");
    output.push_str(r#","candidateFingerprint":"#);
    push_json_string(output, &preview.candidate_package_fingerprint().to_hex());
    output.push_str(r#","compatibility":"#);
    push_json_string(output, compatibility(preview.compatibility()));
    output.push_str(r#","initializationCount":"#);
    write!(output, "{initialization_count}").expect("write to String");
    output.push_str(r#","previewFingerprint":"#);
    push_json_string(output, &preview.hash().to_hex());
    output.push_str(r#","previewId":"#);
    push_json_string(output, &wire_uuid(preview.id().0));
    output.push_str(r#","removalCount":"#);
    write!(output, "{removal_count}").expect("write to String");
    output.push_str(r#","requiresStop":"#);
    push_bool(output, preview.requires_stop());
    output.push_str(r#","warningCount":"#);
    write!(output, "{}", preview.warnings().len()).expect("write to String");
    output.push('}');
}

fn push_optional_value(output: &mut String, value: Option<CanonicalValue>) {
    if let Some(value) = value {
        push_value(output, value);
    } else {
        output.push_str("null");
    }
}

fn push_value(output: &mut String, value: CanonicalValue) {
    output.push_str(r#"{"type":"#);
    push_json_string(output, value_type(value.value_type()));
    output.push_str(r#","value":"#);
    match value {
        CanonicalValue::Bool(value) => push_bool(output, value),
        CanonicalValue::I32(value) => push_json_string(output, &value.to_string()),
        CanonicalValue::I64(value) | CanonicalValue::TimeMs(value) => {
            push_json_string(output, &value.to_string());
        }
        CanonicalValue::U32(value) | CanonicalValue::Bits32(value) => {
            push_json_string(output, &value.to_string());
        }
        CanonicalValue::I8(value) => push_json_string(output, &value.to_string()),
        CanonicalValue::I16(value) => push_json_string(output, &value.to_string()),
        CanonicalValue::U8(value) | CanonicalValue::Bits8(value) | CanonicalValue::Char(value) => {
            push_json_string(output, &value.to_string());
        }
        CanonicalValue::U16(value) | CanonicalValue::Bits16(value) => {
            push_json_string(output, &value.to_string());
        }
        CanonicalValue::U64(value) | CanonicalValue::Bits64(value) => {
            push_json_string(output, &value.to_string());
        }
        CanonicalValue::F32(value) => push_json_string(output, &value.bits().to_string()),
        CanonicalValue::F64(value) => push_json_string(output, &value.bits().to_string()),
    }
    output.push('}');
}

fn push_optional_string(output: &mut String, value: Option<String>) {
    if let Some(value) = value {
        push_json_string(output, &value);
    } else {
        output.push_str("null");
    }
}

fn push_bool(output: &mut String, value: bool) {
    output.push_str(if value { "true" } else { "false" });
}

fn diagnostic_phase(value: ProjectDiagnosticPhase) -> &'static str {
    match value {
        ProjectDiagnosticPhase::CanonicalProjection => "canonical-projection",
        ProjectDiagnosticPhase::Hardware => "hardware",
        ProjectDiagnosticPhase::SoftwareProjection => "software-projection",
        ProjectDiagnosticPhase::ObservabilityProjection => "observability-projection",
        ProjectDiagnosticPhase::Compiler => "compiler",
        ProjectDiagnosticPhase::Integration => "integration",
    }
}

const fn value_type(value: ValueType) -> &'static str {
    match value {
        ValueType::Bool => "BOOL",
        ValueType::I32 => "I32",
        ValueType::I64 => "I64",
        ValueType::U32 => "U32",
        ValueType::TimeMs => "TIME_MS",
        ValueType::I8 => "SINT",
        ValueType::I16 => "INT",
        ValueType::U8 => "USINT",
        ValueType::U16 => "UINT",
        ValueType::U64 => "ULINT",
        ValueType::Bits8 => "BYTE",
        ValueType::Bits16 => "WORD",
        ValueType::Bits32 => "DWORD",
        ValueType::Bits64 => "LWORD",
        ValueType::F32 => "REAL_BITS",
        ValueType::F64 => "LREAL_BITS",
        ValueType::Char => "CHAR",
    }
}

const fn cpu_state(value: CpuState) -> &'static str {
    match value {
        CpuState::PoweredOff => "POWERED_OFF",
        CpuState::Startup | CpuState::Resetting => "STARTUP",
        CpuState::Stop => "STOP",
        CpuState::Run => "RUN",
        CpuState::PausedEducational => "PAUSED_EDUCATIONAL",
        CpuState::Faulted => "FAULTED",
    }
}

const fn runtime_target_kind(value: RuntimeTarget) -> &'static str {
    match value {
        RuntimeTarget::Memory(_) => "memory",
        RuntimeTarget::Input(_) => "input",
        RuntimeTarget::Output(_) => "output",
    }
}

fn runtime_address(value: RuntimeTarget) -> String {
    match value {
        RuntimeTarget::Memory(id) => format!("M{}", id.get()),
        RuntimeTarget::Input(id) => format!("I{}", id.get()),
        RuntimeTarget::Output(id) => format!("Q{}", id.get()),
    }
}

const fn runtime_quality(value: Quality, forced: bool) -> &'static str {
    if forced {
        "FORCED"
    } else {
        match value {
            Quality::Good => "GOOD",
            Quality::Uncertain => "STALE",
            Quality::Bad | Quality::NotPresent => "BAD",
        }
    }
}

const fn monitor_state(value: MonitorState, build_current: bool) -> &'static str {
    match value {
        MonitorState::Stopped | MonitorState::Starting | MonitorState::Stopping => "INACTIVE",
        MonitorState::Active => "ACTIVE",
        MonitorState::Degraded if build_current => "DEGRADED",
        MonitorState::Degraded => "STALE",
    }
}

const fn match_comparison(value: MatchComparison) -> &'static str {
    match value {
        MatchComparison::Match => "MATCH",
        MatchComparison::Mismatch => "MISMATCH",
        MatchComparison::NotLoaded => "NOT_LOADED",
        MatchComparison::NotComparable => "NOT_COMPARABLE",
    }
}

const fn display_base(value: DisplayBase) -> &'static str {
    match value {
        DisplayBase::Automatic => "AUTO",
        DisplayBase::Binary => "BIN",
        DisplayBase::Decimal => "DEC",
        DisplayBase::Hexadecimal => "HEX",
    }
}

const fn trace_state(value: TraceState) -> &'static str {
    match value {
        TraceState::Idle => "IDLE",
        TraceState::Validating | TraceState::Armed => "ARMED",
        TraceState::Triggered => "CAPTURING",
        TraceState::Completed => "COMPLETE",
        TraceState::Aborted | TraceState::Error => "ABORTED",
    }
}

const fn diagnostic_severity(value: DiagnosticSeverity) -> &'static str {
    match value {
        DiagnosticSeverity::Info => "INFO",
        DiagnosticSeverity::Warning => "WARNING",
        DiagnosticSeverity::Error => "ERROR",
        DiagnosticSeverity::Fatal => "FATAL",
    }
}

const fn compatibility(value: CompatibilityClass) -> &'static str {
    match value {
        CompatibilityClass::Initial => "INITIAL",
        CompatibilityClass::Identical => "IDENTICAL",
        CompatibilityClass::PackageIdentityOnly => "PACKAGE_IDENTITY_ONLY",
        CompatibilityClass::CodeOnly => "CODE_ONLY",
        CompatibilityClass::StartValueOnly => "START_VALUE_ONLY",
        CompatibilityClass::MemorySchemaChanging => "MEMORY_SCHEMA_CHANGING",
        CompatibilityClass::StatefulSchemaChanging => "STATEFUL_SCHEMA_CHANGING",
        CompatibilityClass::HardwareChanging => "HARDWARE_CHANGING",
        CompatibilityClass::Incompatible => "INCOMPATIBLE",
    }
}

fn wire_uuid(value: u128) -> String {
    Uuid::from_bytes(value.to_be_bytes()).to_string()
}

pub(crate) fn push_json_string(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character <= '\u{1f}' => {
                write!(output, "\\u{:04x}", u32::from(character)).expect("write to String");
            }
            character => output.push(character),
        }
    }
    output.push('"');
}

#[cfg(test)]
mod tests {
    use super::{
        SystemBridgeError, SystemOperation, canonical_integer, parse_system_command,
        push_json_string, wire_uuid,
    };

    const ID: &str = "11111111-1111-4111-8111-111111111111";

    fn command(operation: &str, fields: &[&str]) -> Vec<u8> {
        [&["PES-SYSTEM-COMMAND-1", operation, ID, ID, ID][..], fields]
            .concat()
            .join("\n")
            .into_bytes()
    }

    #[test]
    fn command_parser_admits_exact_scalar_protocol() {
        let parsed = parse_system_command(&command("PREVIEW_LOAD", &["STOP"])).expect("preview");
        assert_eq!(
            parsed.operation,
            SystemOperation::PreviewLoad(plc_commissioning::PostLoadMode::Stop)
        );

        let parsed = parse_system_command(&command(
            "CREATE_FORCE",
            &[ID, ID, "I64", "-9223372036854775808", "operator approval"],
        ))
        .expect("force");
        assert!(matches!(
            parsed.operation,
            SystemOperation::CreateForce { .. }
        ));
    }

    #[test]
    fn malformed_protocol_is_rejected_before_system_dispatch() {
        assert!(matches!(
            parse_system_command(b"PES-SYSTEM-COMMAND-1\nBUILD"),
            Err(SystemBridgeError::MalformedCommand(_))
        ));
        assert!(parse_system_command(&command("BUILD", &["extra"])).is_err());
        assert!(parse_system_command(&command("SET_RAW_INPUT", &[ID, "U32", "01"])).is_err());
        assert!(parse_system_command(&command("SET_RAW_INPUT", &[ID, "BOOL", "TRUE"])).is_err());
        assert!(parse_system_command(&command("REMOVE_FORCE", &[ID, "line\tbreak"])).is_err());
        assert!(parse_system_command(&vec![b'x'; 4_097]).is_err());
    }

    #[test]
    fn integer_and_uuid_wire_encodings_are_canonical() {
        assert!(canonical_integer("0", true));
        assert!(canonical_integer("-0", true));
        assert!(canonical_integer("18446744073709551615", false));
        assert!(!canonical_integer("00", true));
        assert!(!canonical_integer("+1", true));
        assert!(!canonical_integer("-1", false));
        assert_eq!(wire_uuid(0), "00000000-0000-0000-0000-000000000000");
    }

    #[test]
    fn json_string_encoding_is_complete_and_deterministic() {
        let mut output = String::new();
        push_json_string(
            &mut output,
            "quote \" slash \\ line\n tab\t null\0 degree °",
        );
        assert_eq!(
            output,
            "\"quote \\\" slash \\\\ line\\n tab\\t null\\u0000 degree °\""
        );
    }
}
