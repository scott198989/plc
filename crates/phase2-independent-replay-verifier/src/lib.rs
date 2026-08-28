#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc, clippy::module_name_repetitions)]

//! Independent Phase 2 replay verification over preserved canonical project bytes.
//!
//! This crate is deliberately outside the renderer, `WebView` host, project-file
//! broker, and native evidence finalizer. It decodes the committed `.vlabproj`
//! bytes, reconstructs the exact production engineering workflow, records a
//! canonical replay package, executes that package through the production replay
//! interpreter, and emits a deterministic result which those other components
//! cannot author on its behalf.

use std::fmt::{self, Write as _};

use plc_commissioning::PostLoadMode;
use plc_core::{
    DecodeLimits, Lifecycle, ObjectId, PayloadValue, Project, ProjectObject, ProjectObjectKind,
    Sha256Digest, Uuid, decode_project_package, encode_project_package, sha256,
};
use plc_system::{EngineeringReplayExecutor, EngineeringSession, SystemCommandIdentity};

pub const RESULT_SCHEMA_VERSION: &str = "P2-INDEPENDENT-REPLAY-RESULT-1";
pub const TOOL_IDENTITY: &str = "phase2-independent-replay-verifier/0.1.0";
pub const WORKFLOW_VERSION: &str = "govs-p2-native-replay-workflow-v1";
pub const VERIFICATION_UUID_VERSION: &str = "govs-p2-native-verification-uuid-v1";
pub const PROJECT_TEMPLATE_VERSION: &str = "govs-p2-native-minimal-runnable-project-v1";
pub const EXPECTED_SCAN_COUNT: u64 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeVerificationOrdinalContract {
    pub project_root: u64,
    pub network: u64,
    pub controller: u64,
    pub rack: u64,
    pub input_module: u64,
    pub output_module: u64,
    pub cyclic_main: u64,
    pub interface_members: [u64; 3],
    pub save_as_document: u64,
    pub scan_request: u64,
}

pub const NATIVE_VERIFICATION_ORDINALS: NativeVerificationOrdinalContract =
    NativeVerificationOrdinalContract {
        project_root: 3,
        network: 5,
        controller: 7,
        rack: 9,
        input_module: 11,
        output_module: 13,
        cyclic_main: 15,
        interface_members: [16, 17, 18],
        save_as_document: 20,
        scan_request: 30,
    };

const APPLICATION_VERSION: &str = "plc-engineering-simulator/0.2.0";
const TOOLCHAIN_IDENTITY: &str =
    "rustc 1.94.0 required; exact compiler and binary identity are externally manifest-bound";
const MAX_PROJECT_BYTES: usize = 32 * 1024 * 1024;
const LOCAL_WORKBENCH_AUTHOR_ID: &str = "6c6f6361-6c2d-4777-af72-6b62656e6368";
const PROJECT_TEMPLATE_SPEC: &str = concat!(
    "project-root@3:Phase 2 Native Verification\n",
    "network@5:Virtual network:edu.virtual-network/1:configuredState=enabled\n",
    "controller@7:Controller:edu.controller/1:catalogId=vctrl-c1:profileId=EDU-21 Core:profileVersion=1.0.0\n",
    "rack@9:Local rack:edu.rack/1:slotCount=8\n",
    "module@11:VDI16:edu.module/1:catalogId=vdi16:slot=1:addressIntent=auto\n",
    "module@13:VDO16:edu.module/1:catalogId=vdo16:slot=2:addressIntent=auto\n",
    "program@15:Main_cycle:edu.program-block/1:blockKind=OB:engineeringNumber=1:language=SCL:obRole=CyclicMain:sourceText=empty\n",
    "interface@16:InputValue:temp:BOOL:0:false:false\n",
    "interface@17:OutputValue:temp:BOOL:1:false:false\n",
    "interface@18:WorkingValue:temp:DINT:2:false:false\n",
    "save-as-document@20\n",
    "references=0:dependencies=0:extensions=0\n",
);

const EXACT_WORKFLOW: [WorkflowStep; 9] = [
    WorkflowStep::Build,
    WorkflowStep::PowerOn,
    WorkflowStep::PreviewLoadStop,
    WorkflowStep::CommitLoad,
    WorkflowStep::GoOnline,
    WorkflowStep::RequestRun,
    WorkflowStep::RunOneScan,
    WorkflowStep::RequestStop,
    WorkflowStep::CaptureSnapshot,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorkflowStep {
    Build,
    PreviewLoadStop,
    CommitLoad,
    PowerOn,
    GoOnline,
    RequestRun,
    RunOneScan,
    RequestStop,
    CaptureSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerificationError {
    ProjectSize,
    ProjectDecode(String),
    NonCanonicalProject,
    UnexpectedApplicationVersion(String),
    InvalidVerificationDocumentIdentity,
    UnexpectedProjectTemplate(String),
    AmbiguousControllerState { active: usize, total: usize },
    WorkflowSequenceMismatch,
    WorkflowIdentityMismatch,
    Engineering(String),
    ScanCountMismatch { expected: u64, actual: u64 },
    ReplayDivergence,
    ReplayBoundaryMismatch { expected: usize, actual: usize },
    EmptyReplay,
    MissingReplayLog,
    ClaimedResultMismatch,
}

impl fmt::Display for VerificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProjectSize => {
                formatter.write_str("project bytes are outside the fixed 32 MiB verifier limit")
            }
            Self::ProjectDecode(detail) => {
                write!(formatter, "project package failed closed: {detail}")
            }
            Self::NonCanonicalProject => formatter
                .write_str("project bytes do not round-trip to the exact canonical representation"),
            Self::UnexpectedApplicationVersion(version) => write!(
                formatter,
                "project application version is outside the Phase 2 contract: {version}"
            ),
            Self::InvalidVerificationDocumentIdentity => formatter.write_str(
                "project document identity is not from the fixed native verification UUID sequence",
            ),
            Self::UnexpectedProjectTemplate(detail) => write!(
                formatter,
                "project is not the exact minimal runnable native verification template: {detail}"
            ),
            Self::AmbiguousControllerState { active, total } => write!(
                formatter,
                "project controller state is ambiguous (active={active}, total={total})"
            ),
            Self::WorkflowSequenceMismatch => formatter
                .write_str("requested workflow is not the exact Phase 2 native replay workflow"),
            Self::WorkflowIdentityMismatch => formatter.write_str(
                "scan command identity does not match the request-derived native workflow identity",
            ),
            Self::Engineering(detail) => {
                write!(formatter, "engineering replay failed closed: {detail}")
            }
            Self::ScanCountMismatch { expected, actual } => write!(
                formatter,
                "workflow scan count mismatch (expected={expected}, actual={actual})"
            ),
            Self::ReplayDivergence => formatter.write_str("independent replay execution diverged"),
            Self::ReplayBoundaryMismatch { expected, actual } => write!(
                formatter,
                "independent replay boundary count mismatch (expected={expected}, actual={actual})"
            ),
            Self::EmptyReplay => formatter.write_str("independent replay package is empty"),
            Self::MissingReplayLog => {
                formatter.write_str("canonical replay package contains no events log")
            }
            Self::ClaimedResultMismatch => formatter.write_str(
                "claimed replay result is not the exact independently derived canonical result",
            ),
        }
    }
}

impl std::error::Error for VerificationError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndependentReplayResult {
    pub actual_scan_count: u64,
    pub canonical_replay_sha256: String,
    pub controlled_input_sha256: String,
    pub deterministic_output_sha256: String,
    pub expected_scan_count: u64,
    pub observed_replay_boundary_count: usize,
    pub project_sha256: String,
    pub project_template_identity_sha256: String,
    pub replay_event_count: usize,
    pub replay_log_sha256: String,
    pub replay_package_sha256: String,
    pub runtime_replay_sha256: String,
    pub source_identity_sha256: String,
    pub toolchain_identity: String,
    pub workflow_identity_sha256: String,
}

impl IndependentReplayResult {
    #[must_use]
    pub fn to_canonical_json(&self) -> Vec<u8> {
        let mut output = String::with_capacity(1_600);
        output.push_str("{\"actualScanCount\":");
        write!(output, "{}", self.actual_scan_count).expect("write to String");
        push_string_field(
            &mut output,
            "canonicalReplaySha256",
            &self.canonical_replay_sha256,
        );
        push_string_field(
            &mut output,
            "controlledInputSha256",
            &self.controlled_input_sha256,
        );
        push_string_field(
            &mut output,
            "deterministicOutputSha256",
            &self.deterministic_output_sha256,
        );
        output.push_str(",\"expectedScanCount\":");
        write!(output, "{}", self.expected_scan_count).expect("write to String");
        output.push_str(",\"verifiedReplayBoundaryCount\":");
        write!(output, "{}", self.observed_replay_boundary_count).expect("write to String");
        push_string_field(&mut output, "projectSha256", &self.project_sha256);
        push_string_field(
            &mut output,
            "projectTemplateIdentitySha256",
            &self.project_template_identity_sha256,
        );
        push_string_field(
            &mut output,
            "projectTemplateVersion",
            PROJECT_TEMPLATE_VERSION,
        );
        output.push_str(",\"verifiedReplayEventCount\":");
        write!(output, "{}", self.replay_event_count).expect("write to String");
        push_string_field(&mut output, "replayLogSha256", &self.replay_log_sha256);
        push_string_field(
            &mut output,
            "replayPackageSha256",
            &self.replay_package_sha256,
        );
        output.push_str(",\"result\":\"PASS\"");
        push_string_field(
            &mut output,
            "runtimeReplaySha256",
            &self.runtime_replay_sha256,
        );
        push_string_field(&mut output, "schemaVersion", RESULT_SCHEMA_VERSION);
        push_string_field(
            &mut output,
            "sourceIdentitySha256",
            &self.source_identity_sha256,
        );
        push_string_field(&mut output, "toolIdentity", TOOL_IDENTITY);
        push_string_field(&mut output, "toolchainIdentity", &self.toolchain_identity);
        output.push_str(",\"verified\":true");
        push_string_field(
            &mut output,
            "verificationUuidVersion",
            VERIFICATION_UUID_VERSION,
        );
        push_string_field(
            &mut output,
            "workflowIdentitySha256",
            &self.workflow_identity_sha256,
        );
        push_string_field(&mut output, "workflowVersion", WORKFLOW_VERSION);
        output.push_str("}\n");
        output.into_bytes()
    }

    pub fn verify_exact_claim(&self, claimed: &[u8]) -> Result<(), VerificationError> {
        if claimed == self.to_canonical_json() {
            Ok(())
        } else {
            Err(VerificationError::ClaimedResultMismatch)
        }
    }
}

pub fn verify_project_bytes(bytes: &[u8]) -> Result<IndependentReplayResult, VerificationError> {
    verify_with_contract(bytes, EXACT_WORKFLOW, None)
}

#[allow(clippy::too_many_lines)]
fn verify_with_contract(
    bytes: &[u8],
    workflow: [WorkflowStep; 9],
    supplied_scan_identity: Option<SystemCommandIdentity>,
) -> Result<IndependentReplayResult, VerificationError> {
    if bytes.is_empty() || bytes.len() > MAX_PROJECT_BYTES {
        return Err(VerificationError::ProjectSize);
    }
    if workflow != EXACT_WORKFLOW {
        return Err(VerificationError::WorkflowSequenceMismatch);
    }

    let (project, manifest) = decode_project_package(bytes, DecodeLimits::default())
        .map_err(|error| VerificationError::ProjectDecode(error.to_string()))?;
    if manifest.creation_application_version != APPLICATION_VERSION {
        return Err(VerificationError::UnexpectedApplicationVersion(
            manifest.creation_application_version,
        ));
    }
    let canonical = encode_project_package(&project, APPLICATION_VERSION)
        .map_err(|error| VerificationError::ProjectDecode(error.to_string()))?;
    if canonical != bytes {
        return Err(VerificationError::NonCanonicalProject);
    }

    let document_sequence = verification_uuid_sequence(project.document_id())
        .ok_or(VerificationError::InvalidVerificationDocumentIdentity)?;
    let controllers = project
        .objects()
        .filter(|object| object.kind == ProjectObjectKind::Controller)
        .collect::<Vec<_>>();
    let active = controllers
        .iter()
        .filter(|object| object.lifecycle == Lifecycle::Active)
        .map(|object| object.id)
        .collect::<Vec<_>>();
    if active.len() != 1 {
        return Err(VerificationError::AmbiguousControllerState {
            active: active.len(),
            total: controllers.len(),
        });
    }
    let controller = active[0];
    validate_project_template(&project, document_sequence)?;
    let expected_scan_identity = native_scan_identity(document_sequence)?;
    let scan_identity = supplied_scan_identity.unwrap_or(expected_scan_identity);
    if scan_identity != expected_scan_identity {
        return Err(VerificationError::WorkflowIdentityMismatch);
    }

    let mut session =
        EngineeringSession::new(project.clone(), controller).map_err(engineering_error)?;
    session.build().map_err(engineering_error)?;
    session.power_on().map_err(engineering_error)?;
    let preview = session
        .preview_load(PostLoadMode::Stop)
        .map_err(engineering_error)?;
    session.commit_load(&preview).map_err(engineering_error)?;
    session.go_online().map_err(engineering_error)?;
    session.request_run().map_err(engineering_error)?;
    let before_scan = session
        .read_model()
        .map_err(engineering_error)?
        .scan_sequence;
    session.run_scan(scan_identity).map_err(engineering_error)?;
    let after_scan = session
        .read_model()
        .map_err(engineering_error)?
        .scan_sequence;
    let actual_scan_count = after_scan.checked_sub(before_scan).ok_or_else(|| {
        VerificationError::Engineering("scan sequence moved backwards".to_owned())
    })?;
    if before_scan != 0 || actual_scan_count != EXPECTED_SCAN_COUNT {
        return Err(VerificationError::ScanCountMismatch {
            expected: EXPECTED_SCAN_COUNT,
            actual: actual_scan_count,
        });
    }
    session.request_stop().map_err(engineering_error)?;
    let runtime_replay_sha256 = upper_hash32(
        &session
            .read_model()
            .map_err(engineering_error)?
            .runtime_replay_hash
            .to_hex(),
    );
    let snapshot = session.capture_snapshot().map_err(engineering_error)?;

    let package = EngineeringReplayExecutor::record_validation_package(
        project.clone(),
        controller,
        &snapshot,
    )
    .map_err(engineering_error)?;
    if package.events().is_empty() || package.boundaries().is_empty() {
        return Err(VerificationError::EmptyReplay);
    }
    let execution = EngineeringReplayExecutor::execute(project, controller, &snapshot, &package)
        .map_err(engineering_error)?;
    if execution.divergence.is_some() {
        return Err(VerificationError::ReplayDivergence);
    }
    if execution.observed_boundaries.len() != package.boundaries().len() {
        return Err(VerificationError::ReplayBoundaryMismatch {
            expected: package.boundaries().len(),
            actual: execution.observed_boundaries.len(),
        });
    }
    let replay_log = package
        .members()
        .into_iter()
        .find(|member| member.name == "events.jsonl")
        .map(|member| member.bytes)
        .ok_or(VerificationError::MissingReplayLog)?;

    let project_sha256 = digest_upper(bytes);
    Ok(IndependentReplayResult {
        actual_scan_count,
        canonical_replay_sha256: upper_hash32(&package.content_fingerprint().to_hex()),
        controlled_input_sha256: project_sha256.clone(),
        deterministic_output_sha256: project_sha256.clone(),
        expected_scan_count: EXPECTED_SCAN_COUNT,
        observed_replay_boundary_count: execution.observed_boundaries.len(),
        project_sha256: project_sha256.clone(),
        project_template_identity_sha256: digest_upper(PROJECT_TEMPLATE_SPEC.as_bytes()),
        replay_event_count: package.events().len(),
        replay_log_sha256: digest_upper(replay_log),
        replay_package_sha256: digest_upper(package.bytes()),
        runtime_replay_sha256,
        source_identity_sha256: source_identity_sha256(),
        toolchain_identity: TOOLCHAIN_IDENTITY.to_owned(),
        workflow_identity_sha256: workflow_identity_sha256(document_sequence)?,
    })
}

#[allow(clippy::too_many_lines)]
fn validate_project_template(
    project: &Project,
    document_sequence: u64,
) -> Result<(), VerificationError> {
    if document_sequence != NATIVE_VERIFICATION_ORDINALS.save_as_document {
        return template_error("Save As document UUID ordinal must be exactly 20");
    }
    if project.root_id().0 != fixed_verification_uuid(NATIVE_VERIFICATION_ORDINALS.project_root)? {
        return template_error("project root UUID ordinal must be exactly 3");
    }
    if project.object(project.root_id()).is_none_or(|root| {
        root.kind != ProjectObjectKind::Project
            || root.lifecycle != Lifecycle::Active
            || root.display_name != "Phase 2 Native Verification"
            || root.parent_id.is_some()
    }) {
        return template_error("root identity, lifecycle, parent, or display name drifted");
    }
    let objects = project.objects().collect::<Vec<_>>();
    if objects.len() != 7
        || objects
            .iter()
            .any(|object| object.lifecycle != Lifecycle::Active)
    {
        return template_error(
            "the template must contain exactly seven active objects and no tombstones",
        );
    }
    if project.references().next().is_some()
        || project.dependencies().next().is_some()
        || project.simulator_extensions().next().is_some()
    {
        return template_error("references, dependencies, and simulator extensions must be empty");
    }
    for object in &objects {
        let sequence = verification_uuid_sequence(object.id.0).ok_or_else(|| {
            VerificationError::UnexpectedProjectTemplate(format!(
                "object {} is outside the verification UUID lineage",
                object.id
            ))
        })?;
        if sequence >= document_sequence {
            return template_error(
                "a persisted object UUID is not earlier than the Save As document UUID",
            );
        }
    }

    let root = project.root_id();
    let network = exact_object(project, ProjectObjectKind::Network)?;
    require_object_sequence(network, NATIVE_VERIFICATION_ORDINALS.network)?;
    validate_object(
        network,
        root,
        "Virtual network",
        "edu.virtual-network/1",
        &fields_owned(&[("configuredState", PayloadValue::from("enabled"))]),
    )?;
    let controller = exact_object(project, ProjectObjectKind::Controller)?;
    require_object_sequence(controller, NATIVE_VERIFICATION_ORDINALS.controller)?;
    validate_object(
        controller,
        root,
        "Controller",
        "edu.controller/1",
        &fields_owned(&[
            ("catalogId", PayloadValue::from("vctrl-c1")),
            ("profileId", PayloadValue::from("EDU-21 Core")),
            ("profileVersion", PayloadValue::from("1.0.0")),
        ]),
    )?;
    let rack = exact_object(project, ProjectObjectKind::Rack)?;
    require_object_sequence(rack, NATIVE_VERIFICATION_ORDINALS.rack)?;
    validate_object(
        rack,
        controller.id,
        "Local rack",
        "edu.rack/1",
        &fields_owned(&[("slotCount", PayloadValue::Unsigned(8))]),
    )?;

    let modules = project
        .objects()
        .filter(|object| object.kind == ProjectObjectKind::Module)
        .collect::<Vec<_>>();
    if modules.len() != 2 {
        return template_error("the template must contain exactly two modules");
    }
    let input = modules
        .iter()
        .copied()
        .find(|object| object.display_name == "VDI16")
        .ok_or_else(|| {
            VerificationError::UnexpectedProjectTemplate("VDI16 is absent".to_owned())
        })?;
    require_object_sequence(input, NATIVE_VERIFICATION_ORDINALS.input_module)?;
    validate_object(
        input,
        rack.id,
        "VDI16",
        "edu.module/1",
        &module_fields("vdi16", 1),
    )?;
    let output = modules
        .iter()
        .copied()
        .find(|object| object.display_name == "VDO16")
        .ok_or_else(|| {
            VerificationError::UnexpectedProjectTemplate("VDO16 is absent".to_owned())
        })?;
    require_object_sequence(output, NATIVE_VERIFICATION_ORDINALS.output_module)?;
    validate_object(
        output,
        rack.id,
        "VDO16",
        "edu.module/1",
        &module_fields("vdo16", 2),
    )?;

    let program = exact_object(project, ProjectObjectKind::ProgramBlock)?;
    require_object_sequence(program, NATIVE_VERIFICATION_ORDINALS.cyclic_main)?;
    if program.parent_id != Some(controller.id)
        || program.display_name != "Main_cycle"
        || program.payload_schema != "edu.program-block/1"
        || !program.payload.presentation.is_empty()
    {
        return template_error("the empty SCL CyclicMain object envelope drifted");
    }
    let interface = program
        .payload
        .semantic
        .get("interface")
        .and_then(|value| match value {
            PayloadValue::List(values) => Some(values),
            _ => None,
        })
        .ok_or_else(|| {
            VerificationError::UnexpectedProjectTemplate(
                "CyclicMain interface is absent or not a list".to_owned(),
            )
        })?;
    if interface.len() != 3 {
        return template_error("CyclicMain must contain exactly three interface members");
    }
    let expected_members = [
        ("InputValue", "temp", "BOOL", 0_u64),
        ("OutputValue", "temp", "BOOL", 1_u64),
        ("WorkingValue", "temp", "DINT", 2_u64),
    ];
    let mut normalized_interface = Vec::with_capacity(3);
    for (index, (value, (name, role, data_type, order))) in
        interface.iter().zip(expected_members).enumerate()
    {
        let PayloadValue::Record(member) = value else {
            return template_error("CyclicMain interface member is not a record");
        };
        let identity = member.get("id").and_then(payload_text).ok_or_else(|| {
            VerificationError::UnexpectedProjectTemplate(
                "CyclicMain interface member identity is absent".to_owned(),
            )
        })?;
        let uuid = Uuid::parse(identity).map_err(|_| {
            VerificationError::UnexpectedProjectTemplate(
                "CyclicMain interface member identity is malformed".to_owned(),
            )
        })?;
        let expected_sequence = NATIVE_VERIFICATION_ORDINALS.interface_members[index];
        if verification_uuid_sequence(uuid) != Some(expected_sequence) {
            return template_error("CyclicMain interface member UUID lineage drifted");
        }
        let expected = fields_owned(&[
            ("id", PayloadValue::from(identity)),
            ("name", PayloadValue::from(name)),
            ("order", PayloadValue::Unsigned(order)),
            ("requiredOutput", PayloadValue::Bool(false)),
            ("retentive", PayloadValue::Bool(false)),
            ("role", PayloadValue::from(role)),
            ("type", PayloadValue::from(data_type)),
        ]);
        if *member != expected {
            return template_error("CyclicMain interface member fields drifted");
        }
        normalized_interface.push(value.clone());
    }
    let expected_program = fields_owned(&[
        ("blockKind", PayloadValue::from("OB")),
        ("engineeringNumber", PayloadValue::Unsigned(1)),
        ("interface", PayloadValue::List(normalized_interface)),
        ("language", PayloadValue::from("SCL")),
        ("obRole", PayloadValue::from("CyclicMain")),
        ("sourceText", PayloadValue::from("")),
    ]);
    if program.payload.semantic != expected_program {
        return template_error("CyclicMain semantic payload drifted");
    }
    Ok(())
}

fn require_object_sequence(object: &ProjectObject, expected: u64) -> Result<(), VerificationError> {
    if verification_uuid_sequence(object.id.0) == Some(expected) {
        Ok(())
    } else {
        template_error(&format!(
            "{} UUID ordinal must be exactly {expected}",
            object.display_name
        ))
    }
}

fn exact_object(
    project: &Project,
    kind: ProjectObjectKind,
) -> Result<&ProjectObject, VerificationError> {
    let values = project
        .objects()
        .filter(|object| object.kind == kind)
        .collect::<Vec<_>>();
    if values.len() == 1 {
        Ok(values[0])
    } else {
        template_error(&format!(
            "expected exactly one {kind:?}; found {}",
            values.len()
        ))
    }
}

fn validate_object(
    object: &ProjectObject,
    parent: ObjectId,
    name: &str,
    schema: &str,
    semantic: &std::collections::BTreeMap<String, PayloadValue>,
) -> Result<(), VerificationError> {
    if object.parent_id != Some(parent)
        || object.display_name != name
        || object.payload_schema != schema
        || object.payload.semantic != *semantic
        || !object.payload.presentation.is_empty()
    {
        template_error(&format!("{name} envelope or payload drifted"))
    } else {
        Ok(())
    }
}

fn module_fields(catalog: &str, slot: u64) -> std::collections::BTreeMap<String, PayloadValue> {
    fields_owned(&[
        ("addressIntent", PayloadValue::from("auto")),
        ("catalogId", PayloadValue::from(catalog)),
        ("slot", PayloadValue::Unsigned(slot)),
    ])
}

fn fields_owned(
    values: &[(&str, PayloadValue)],
) -> std::collections::BTreeMap<String, PayloadValue> {
    values
        .iter()
        .map(|(key, value)| ((*key).to_owned(), value.clone()))
        .collect()
}

fn payload_text(value: &PayloadValue) -> Option<&str> {
    match value {
        PayloadValue::String(value) => Some(value),
        _ => None,
    }
}

fn template_error<T>(detail: &str) -> Result<T, VerificationError> {
    Err(VerificationError::UnexpectedProjectTemplate(
        detail.to_owned(),
    ))
}

fn native_scan_identity(
    document_sequence: u64,
) -> Result<SystemCommandIdentity, VerificationError> {
    // Save As persists `document_sequence`; its prepare request, durable commit,
    // open request, and the six preceding runtime requests consume the next nine
    // page-global UUIDs. Scan is therefore exactly document_sequence + 10.
    let scan_sequence = document_sequence
        .checked_add(10)
        .ok_or(VerificationError::InvalidVerificationDocumentIdentity)?;
    if scan_sequence != NATIVE_VERIFICATION_ORDINALS.scan_request {
        return Err(VerificationError::WorkflowIdentityMismatch);
    }
    let command = fixed_verification_uuid(scan_sequence)?;
    let command_text = command.to_string();
    let idempotency = derive_uuid(&format!("{command_text}:runtime:idempotency"));
    let author = Uuid::parse(LOCAL_WORKBENCH_AUTHOR_ID)
        .map_err(|_| VerificationError::WorkflowIdentityMismatch)?;
    Ok(SystemCommandIdentity {
        command_id: uuid_u128(command),
        idempotency_key: uuid_u128(idempotency),
        author_identity: uuid_u128(author),
    })
}

fn workflow_identity_sha256(document_sequence: u64) -> Result<String, VerificationError> {
    const RUNTIME_OPERATIONS: [(&str, u64); 9] = [
        ("BUILD", 4),
        ("POWER_ON", 5),
        ("PREVIEW_LOAD_STOP", 6),
        ("COMMIT_LOAD", 7),
        ("GO_ONLINE", 8),
        ("REQUEST_RUN", 9),
        ("RUN_SCAN", 10),
        ("REQUEST_STOP", 11),
        ("CAPTURE_SNAPSHOT", 12),
    ];
    let mut transcript = Vec::new();
    transcript.extend_from_slice(WORKFLOW_VERSION.as_bytes());
    transcript.push(0);
    transcript.extend_from_slice(VERIFICATION_UUID_VERSION.as_bytes());
    transcript.push(0);
    transcript.extend_from_slice(LOCAL_WORKBENCH_AUTHOR_ID.as_bytes());
    transcript.push(0);
    for (operation, offset) in RUNTIME_OPERATIONS {
        let sequence = document_sequence
            .checked_add(offset)
            .ok_or(VerificationError::InvalidVerificationDocumentIdentity)?;
        let request = fixed_verification_uuid(sequence)?.to_string();
        let idempotency = derive_uuid(&format!("{request}:runtime:idempotency")).to_string();
        transcript.extend_from_slice(operation.as_bytes());
        transcript.push(0);
        transcript.extend_from_slice(request.as_bytes());
        transcript.push(0);
        transcript.extend_from_slice(idempotency.as_bytes());
        transcript.push(0);
    }
    Ok(digest_upper(&transcript))
}

fn verification_uuid_sequence(uuid: Uuid) -> Option<u64> {
    let bytes = uuid.into_bytes();
    if bytes[..10] != [0x2b, 0x42, 0xb8, 0x46, 0x54, 0xd0, 0x4c, 0x61, 0x9b, 0x72] {
        return None;
    }
    let mut value = [0_u8; 8];
    value[2..].copy_from_slice(&bytes[10..]);
    let sequence = u64::from_be_bytes(value);
    (sequence > 0).then_some(sequence)
}

fn fixed_verification_uuid(sequence: u64) -> Result<Uuid, VerificationError> {
    if sequence == 0 || sequence > 0x0000_ffff_ffff_ffff {
        return Err(VerificationError::InvalidVerificationDocumentIdentity);
    }
    Uuid::parse(&format!("2b42b846-54d0-4c61-9b72-{sequence:012x}"))
        .map_err(|_| VerificationError::InvalidVerificationDocumentIdentity)
}

fn derive_uuid(seed: &str) -> Uuid {
    let Sha256Digest(mut bytes) = sha256(seed.as_bytes());
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes[..16].try_into().expect("SHA-256 prefix is 16 bytes"))
}

fn uuid_u128(uuid: Uuid) -> u128 {
    u128::from_be_bytes(uuid.into_bytes())
}

fn engineering_error(error: impl fmt::Debug) -> VerificationError {
    VerificationError::Engineering(format!("{error:?}"))
}

fn digest_upper(bytes: &[u8]) -> String {
    sha256(bytes).to_hex().to_ascii_uppercase()
}

fn upper_hash32(value: &str) -> String {
    value.to_ascii_uppercase()
}

fn source_identity_sha256() -> String {
    const SOURCES: [(&str, &[u8]); 3] = [
        ("Cargo.toml", include_bytes!("../Cargo.toml")),
        ("src/lib.rs", include_bytes!("lib.rs")),
        ("src/main.rs", include_bytes!("main.rs")),
    ];
    let mut identity = Vec::new();
    identity.extend_from_slice(b"P2-INDEPENDENT-REPLAY-SOURCE-1\0");
    for (path, bytes) in SOURCES {
        identity.extend_from_slice(&(path.len() as u64).to_be_bytes());
        identity.extend_from_slice(path.as_bytes());
        identity.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
        identity.extend_from_slice(bytes);
    }
    digest_upper(&identity)
}

fn push_string_field(output: &mut String, key: &str, value: &str) {
    output.push_str(",\"");
    output.push_str(key);
    output.push_str("\":\"");
    for byte in value.bytes() {
        match byte {
            b'"' => output.push_str("\\\""),
            b'\\' => output.push_str("\\\\"),
            b'\n' => output.push_str("\\n"),
            b'\r' => output.push_str("\\r"),
            b'\t' => output.push_str("\\t"),
            0x20..=0x7e => output.push(char::from(byte)),
            _ => write!(output, "\\u{byte:04x}").expect("write to String"),
        }
    }
    output.push('"');
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use plc_core::{
        CommandContext, CommandEnvelope, CommandOutcome, DomainCommand, Engine, NewObject,
        ObjectId, Payload, PayloadValue, ProfilePin, Project, TransactionId,
    };
    use plc_hardware::TrainingProfile;

    use super::*;

    struct Fixture {
        engine: Engine,
        next_command: u64,
    }

    impl Fixture {
        fn runnable() -> Self {
            let profile = TrainingProfile::edu21().pin();
            let root = ObjectId(fixed_verification_uuid(3).expect("root UUID"));
            let project = Project::new(
                fixed_verification_uuid(20).expect("Save As document UUID"),
                root,
                "Phase 2 Native Verification",
                ProfilePin {
                    id: profile.id,
                    version: profile.version,
                    manifest_hash: profile.manifest_hash,
                },
            );
            let mut fixture = Self {
                engine: Engine::new(project).expect("fixture root"),
                next_command: 1,
            };
            fixture.create(
                object_id(5),
                ProjectObjectKind::Network,
                root,
                "Virtual network",
                "edu.virtual-network/1",
                fields(&[("configuredState", PayloadValue::from("enabled"))]),
            );
            let controller = object_id(7);
            fixture.create(
                controller,
                ProjectObjectKind::Controller,
                root,
                "Controller",
                "edu.controller/1",
                fields(&[
                    ("catalogId", PayloadValue::from("vctrl-c1")),
                    ("profileId", PayloadValue::from("EDU-21 Core")),
                    ("profileVersion", PayloadValue::from("1.0.0")),
                ]),
            );
            let rack = object_id(9);
            fixture.create(
                rack,
                ProjectObjectKind::Rack,
                controller,
                "Local rack",
                "edu.rack/1",
                fields(&[("slotCount", PayloadValue::Unsigned(8))]),
            );
            fixture.create(
                object_id(11),
                ProjectObjectKind::Module,
                rack,
                "VDI16",
                "edu.module/1",
                module_payload("vdi16", 1),
            );
            fixture.create(
                object_id(13),
                ProjectObjectKind::Module,
                rack,
                "VDO16",
                "edu.module/1",
                module_payload("vdo16", 2),
            );
            fixture.create(
                object_id(15),
                ProjectObjectKind::ProgramBlock,
                controller,
                "Main_cycle",
                "edu.program-block/1",
                fields(&[
                    ("blockKind", PayloadValue::from("OB")),
                    ("engineeringNumber", PayloadValue::Unsigned(1)),
                    (
                        "interface",
                        PayloadValue::List(vec![
                            interface_member(16, "InputValue", "temp", "BOOL", 0),
                            interface_member(17, "OutputValue", "temp", "BOOL", 1),
                            interface_member(18, "WorkingValue", "temp", "DINT", 2),
                        ]),
                    ),
                    ("language", PayloadValue::from("SCL")),
                    ("obRole", PayloadValue::from("CyclicMain")),
                    ("sourceText", PayloadValue::from("")),
                ]),
            );
            fixture
        }

        fn create(
            &mut self,
            id: ObjectId,
            kind: ProjectObjectKind,
            parent_id: ObjectId,
            display_name: &str,
            payload_schema: &str,
            semantic: BTreeMap<String, PayloadValue>,
        ) {
            let ordinal = self.next_command;
            self.next_command += 1;
            let result = self.engine.execute(&CommandEnvelope {
                command_id: Uuid::deterministic_v4(
                    b"p2-independent-replay-fixture-command",
                    ordinal,
                ),
                transaction_id: TransactionId(Uuid::deterministic_v4(
                    b"p2-independent-replay-fixture-transaction",
                    ordinal,
                )),
                expected_document_revision: self.engine.project().document_revision(),
                expected_object_revisions: BTreeMap::from([(
                    parent_id,
                    self.engine
                        .project()
                        .object(parent_id)
                        .expect("fixture parent")
                        .object_revision,
                )]),
                context: CommandContext {
                    actor_id: "p2-independent-replay-fixture".to_owned(),
                    can_mutate: true,
                },
                command: DomainCommand::Create(NewObject {
                    id,
                    kind,
                    parent_id,
                    display_name: display_name.to_owned(),
                    payload_schema: payload_schema.to_owned(),
                    payload: Payload {
                        semantic,
                        presentation: BTreeMap::new(),
                    },
                }),
            });
            assert_eq!(
                result.outcome,
                CommandOutcome::Committed,
                "{:?}",
                result.diagnostics
            );
        }

        fn bytes(&self) -> Vec<u8> {
            encode_project_package(self.engine.project(), APPLICATION_VERSION)
                .expect("fixture package")
        }
    }

    fn object_id(ordinal: u64) -> ObjectId {
        ObjectId(fixed_verification_uuid(ordinal).expect("object UUID"))
    }

    fn fields(values: &[(&str, PayloadValue)]) -> BTreeMap<String, PayloadValue> {
        values
            .iter()
            .map(|(key, value)| ((*key).to_owned(), value.clone()))
            .collect()
    }

    fn module_payload(catalog: &str, slot: u64) -> BTreeMap<String, PayloadValue> {
        fields(&[
            ("addressIntent", PayloadValue::from("auto")),
            ("catalogId", PayloadValue::from(catalog)),
            ("slot", PayloadValue::Unsigned(slot)),
        ])
    }

    fn interface_member(
        identity: u64,
        name: &str,
        role: &str,
        data_type: &str,
        order: u64,
    ) -> PayloadValue {
        PayloadValue::Record(fields(&[
            (
                "id",
                PayloadValue::from(
                    fixed_verification_uuid(identity)
                        .expect("member UUID")
                        .to_string(),
                ),
            ),
            ("name", PayloadValue::from(name)),
            ("order", PayloadValue::Unsigned(order)),
            ("requiredOutput", PayloadValue::Bool(false)),
            ("retentive", PayloadValue::Bool(false)),
            ("role", PayloadValue::from(role)),
            ("type", PayloadValue::from(data_type)),
        ]))
    }

    #[test]
    fn independently_derives_and_executes_exact_replay() {
        let bytes = Fixture::runnable().bytes();
        let first = verify_project_bytes(&bytes).expect("independent replay");
        let second = verify_project_bytes(&bytes).expect("repeat independent replay");
        assert_eq!(first, second);
        assert_eq!(first.expected_scan_count, 1);
        assert_eq!(first.actual_scan_count, 1);
        assert_eq!(first.project_sha256, first.controlled_input_sha256);
        assert_eq!(first.project_sha256, first.deterministic_output_sha256);
        assert!(!first.replay_package_sha256.is_empty());
        assert_eq!(first.to_canonical_json(), second.to_canonical_json());
    }

    #[test]
    fn mutated_and_malformed_projects_fail_closed() {
        let bytes = Fixture::runnable().bytes();
        let mut mutated = bytes.clone();
        let midpoint = mutated.len() / 2;
        mutated[midpoint] ^= 1;
        assert!(verify_project_bytes(&mutated).is_err());
        assert!(matches!(
            verify_project_bytes(b"not-a-project"),
            Err(VerificationError::ProjectDecode(_))
        ));

        let (project, _) = decode_project_package(&bytes, DecodeLimits::default())
            .expect("canonical fixture package");
        let ordinal_drift = project
            .for_save_as(fixed_verification_uuid(21).expect("drifted document UUID"))
            .expect("different Save As identity");
        let drifted = encode_project_package(&ordinal_drift, APPLICATION_VERSION)
            .expect("canonical drifted package");
        assert!(matches!(
            verify_project_bytes(&drifted),
            Err(VerificationError::UnexpectedProjectTemplate(_))
        ));
    }

    #[test]
    fn fixed_native_uuid_contract_is_explicit_and_scan_is_request_derived() {
        assert_eq!(
            NATIVE_VERIFICATION_ORDINALS,
            NativeVerificationOrdinalContract {
                project_root: 3,
                network: 5,
                controller: 7,
                rack: 9,
                input_module: 11,
                output_module: 13,
                cyclic_main: 15,
                interface_members: [16, 17, 18],
                save_as_document: 20,
                scan_request: 30,
            }
        );
        let identity = native_scan_identity(NATIVE_VERIFICATION_ORDINALS.save_as_document)
            .expect("fixed scan identity");
        assert_eq!(
            identity.command_id,
            uuid_u128(fixed_verification_uuid(30).expect("scan request UUID"))
        );
    }

    #[test]
    fn ambiguous_active_controller_state_fails_closed() {
        let mut fixture = Fixture::runnable();
        let root = fixture.engine.project().root_id();
        fixture.create(
            object_id(19),
            ProjectObjectKind::Controller,
            root,
            "Controller_2",
            "edu.controller/1",
            fields(&[
                ("catalogId", PayloadValue::from("vctrl-c1")),
                ("profileId", PayloadValue::from("EDU-21 Core")),
                ("profileVersion", PayloadValue::from("1.0.0")),
            ]),
        );
        assert_eq!(
            verify_project_bytes(&fixture.bytes()),
            Err(VerificationError::AmbiguousControllerState {
                active: 2,
                total: 2,
            })
        );
    }

    #[test]
    fn wrong_sequence_and_scan_identity_fail_before_execution() {
        let bytes = Fixture::runnable().bytes();
        let mut reordered = EXACT_WORKFLOW;
        reordered.swap(2, 3);
        assert_eq!(
            verify_with_contract(&bytes, reordered, None),
            Err(VerificationError::WorkflowSequenceMismatch)
        );
        let wrong = SystemCommandIdentity {
            command_id: 1,
            idempotency_key: 2,
            author_identity: 3,
        };
        assert_eq!(
            verify_with_contract(&bytes, EXACT_WORKFLOW, Some(wrong)),
            Err(VerificationError::WorkflowIdentityMismatch)
        );
    }

    #[test]
    fn claimed_result_tampering_is_rejected_byte_exactly() {
        let result =
            verify_project_bytes(&Fixture::runnable().bytes()).expect("independent replay");
        let canonical = result.to_canonical_json();
        result.verify_exact_claim(&canonical).expect("exact claim");
        let mut tampered = canonical.clone();
        let offset = tampered
            .windows(64)
            .position(|window| window.iter().all(u8::is_ascii_hexdigit))
            .expect("result hash");
        tampered[offset] = if tampered[offset] == b'A' { b'B' } else { b'A' };
        assert_eq!(
            result.verify_exact_claim(&tampered),
            Err(VerificationError::ClaimedResultMismatch)
        );
        let mut unknown_field = canonical;
        unknown_field.splice(1..1, b"\"forged\":true,".iter().copied());
        assert_eq!(
            result.verify_exact_claim(&unknown_field),
            Err(VerificationError::ClaimedResultMismatch)
        );
    }
}
