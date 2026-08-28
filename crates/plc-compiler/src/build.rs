use alloc::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    string::{String, ToString},
    vec,
    vec::Vec,
};

use plc_program::{
    BindingActual, BlockId, CanonicalValue, ControllerProgram, DataBlockKind, DataType,
    DependencyEdge, DependencyReason, InstanceOwner, InstructionCategory, InterfaceMember,
    IssueCode, ObDeclaration, PHASE2_INSTRUCTION_REGISTRY_VERSION, ProgramIssue, ProgramUnitKind,
    RetainPolicy, SideEffectClass, StateKind, StateRequirement, VariableRef,
    phase2_instruction_registry, validate_program,
};
use plc_runtime::{
    Hash32, PRIORITY_TABLE_VERSION, RUNTIME_SEMANTICS_VERSION, SCHEDULER_VERSION, WORK_COST_VERSION,
};

use crate::{
    ARITHMETIC_POLICY_VERSION, BUILD_ARTIFACT_SCHEMA, BuildAttemptId, BuildDiagnostic,
    COMPILER_SEMANTICS_VERSION, CONVERSION_POLICY_VERSION, CancellationToken, DiagnosticCode,
    DiagnosticParameter, DiagnosticTarget, PROBE_SCHEMA_VERSION, ProbeTable, ResourceLimit,
    ResourceLimits, SclSource, SemanticNodeId, SourceAnchor, SourceMapTable, TYPE_SYSTEM_VERSION,
    VerifiedIr,
    diagnostic::{RegistryError, phase2_diagnostic_registry},
    hash::CanonicalHasher,
    limits::{WorkMeter, WorkStop},
    lowering::lower_typed_blocks,
    scl::{SyntaxTree, bind_and_typecheck_with_program, parse_scl},
    verify_typed_ir,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompilerProfile {
    identity: String,
    version: String,
    capabilities: Vec<String>,
    fingerprint: Hash32,
    capability_manifest_hash: Hash32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProfileError {
    EmptyIdentity,
    EmptyVersion,
    InvalidCapability,
    CapabilityOrder,
}

impl CompilerProfile {
    #[must_use]
    pub fn edu21_core() -> Self {
        let identity = String::from("EDU-21 Core");
        let version = String::from("1.0");
        let capabilities = vec![
            "scl.assignment".into(),
            "scl.call.fc".into(),
            "scl.expression.baseline".into(),
            "scl.if".into(),
            "scl.return".into(),
        ];
        let capability_manifest_hash = hash_capabilities(&capabilities);
        let mut hasher = CanonicalHasher::new("PES-COMPILER-PROFILE-1");
        hasher.string(&identity);
        hasher.string(&version);
        hasher.hash(capability_manifest_hash);
        Self {
            identity,
            version,
            capabilities,
            fingerprint: hasher.finish(),
            capability_manifest_hash,
        }
    }

    /// Creates a pinned capability profile. Capabilities must already be in
    /// strict lexical order so identity never depends on collection ordering.
    ///
    /// # Errors
    ///
    /// Returns a deterministic schema/order defect for empty or noncanonical
    /// profile input.
    pub fn from_parts(
        identity: impl Into<String>,
        version: impl Into<String>,
        capabilities: Vec<String>,
    ) -> Result<Self, ProfileError> {
        let identity = identity.into();
        let version = version.into();
        if identity.is_empty() {
            return Err(ProfileError::EmptyIdentity);
        }
        if version.is_empty() {
            return Err(ProfileError::EmptyVersion);
        }
        if capabilities.iter().any(|value| {
            value.is_empty()
                || value.len() > 128
                || !value.is_ascii()
                || !value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
        }) {
            return Err(ProfileError::InvalidCapability);
        }
        if !capabilities.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err(ProfileError::CapabilityOrder);
        }
        let capability_manifest_hash = hash_capabilities(&capabilities);
        let mut hasher = CanonicalHasher::new("PES-COMPILER-PROFILE-1");
        hasher.string(&identity);
        hasher.string(&version);
        hasher.hash(capability_manifest_hash);
        let fingerprint = hasher.finish();
        Ok(Self {
            identity,
            version,
            capabilities,
            fingerprint,
            capability_manifest_hash,
        })
    }

    #[must_use]
    pub fn identity(&self) -> &str {
        &self.identity
    }

    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    #[must_use]
    pub fn capabilities(&self) -> &[String] {
        &self.capabilities
    }

    #[must_use]
    pub const fn fingerprint(&self) -> Hash32 {
        self.fingerprint
    }

    #[must_use]
    pub const fn capability_manifest_hash(&self) -> Hash32 {
        self.capability_manifest_hash
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SnapshotError {
    SourceKeyMismatch { key: BlockId, owner: BlockId },
    UnknownSourceOwner(BlockId),
    SourceForDataBlock(BlockId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildSnapshot {
    program: ControllerProgram,
    scl_sources: BTreeMap<BlockId, SclSource>,
    profile: CompilerProfile,
    instruction_registry_hash: Hash32,
    snapshot_hash: Hash32,
}

impl BuildSnapshot {
    /// Captures owned canonical state. Subsequent project/source edits cannot
    /// alter this value or leak into a running build.
    ///
    /// # Errors
    ///
    /// Returns an identity/kind defect when a source map key and source owner
    /// disagree or source is attached to an absent/nonexecutable block.
    pub fn capture(
        program: &ControllerProgram,
        scl_sources: &BTreeMap<BlockId, SclSource>,
        profile: CompilerProfile,
    ) -> Result<Self, SnapshotError> {
        for (&key, source) in scl_sources {
            if key != source.owner() {
                return Err(SnapshotError::SourceKeyMismatch {
                    key,
                    owner: source.owner(),
                });
            }
            let block = program
                .block(key)
                .ok_or(SnapshotError::UnknownSourceOwner(key))?;
            if !block.kind.is_executable() {
                return Err(SnapshotError::SourceForDataBlock(key));
            }
        }
        let instruction_registry_hash = instruction_registry_hash();
        let mut snapshot = Self {
            program: program.clone(),
            scl_sources: scl_sources.clone(),
            profile,
            instruction_registry_hash,
            snapshot_hash: Hash32::ZERO,
        };
        snapshot.snapshot_hash = snapshot.calculate_hash();
        Ok(snapshot)
    }

    #[must_use]
    pub const fn program(&self) -> &ControllerProgram {
        &self.program
    }

    #[must_use]
    pub const fn scl_sources(&self) -> &BTreeMap<BlockId, SclSource> {
        &self.scl_sources
    }

    #[must_use]
    pub const fn profile(&self) -> &CompilerProfile {
        &self.profile
    }

    #[must_use]
    pub const fn instruction_registry_hash(&self) -> Hash32 {
        self.instruction_registry_hash
    }

    #[must_use]
    pub const fn snapshot_hash(&self) -> Hash32 {
        self.snapshot_hash
    }

    fn calculate_hash(&self) -> Hash32 {
        let mut hasher = CanonicalHasher::new("PES-BUILD-SNAPSHOT-1");
        encode_program(&mut hasher, &self.program);
        hasher.hash(self.profile.fingerprint);
        hasher.hash(self.profile.capability_manifest_hash);
        hasher.hash(self.instruction_registry_hash);
        hasher.u64(self.scl_sources.len() as u64);
        for (owner, source) in &self.scl_sources {
            hasher.u128(owner.get());
            hasher.hash(source.revision_hash());
            hasher.bytes(source.text().as_bytes());
        }
        hasher.finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BuildScope {
    CurrentObject(BlockId),
    SoftwareChanges(Vec<BlockId>),
    RebuildAllSoftware,
    VirtualHardware,
    ControllerBuild,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpandedScope {
    requested: BuildScope,
    ordered_units: Vec<BlockId>,
    prerequisites: Vec<BlockId>,
    affected_dependents: Vec<BlockId>,
}

impl ExpandedScope {
    #[must_use]
    pub const fn requested(&self) -> &BuildScope {
        &self.requested
    }

    #[must_use]
    pub fn ordered_units(&self) -> &[BlockId] {
        &self.ordered_units
    }

    #[must_use]
    pub fn prerequisites(&self) -> &[BlockId] {
        &self.prerequisites
    }

    #[must_use]
    pub fn affected_dependents(&self) -> &[BlockId] {
        &self.affected_dependents
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScopeError {
    UnknownObject(BlockId),
    DependencyLimit(ResourceLimit),
    UnsupportedScope,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildAttempt {
    id: BuildAttemptId,
    snapshot: BuildSnapshot,
    requested_scope: BuildScope,
}

impl BuildAttempt {
    #[must_use]
    pub const fn new(
        id: BuildAttemptId,
        snapshot: BuildSnapshot,
        requested_scope: BuildScope,
    ) -> Self {
        Self {
            id,
            snapshot,
            requested_scope,
        }
    }

    #[must_use]
    pub const fn id(&self) -> BuildAttemptId {
        self.id
    }

    #[must_use]
    pub const fn snapshot(&self) -> &BuildSnapshot {
        &self.snapshot
    }

    #[must_use]
    pub const fn requested_scope(&self) -> &BuildScope {
        &self.requested_scope
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CompilerStage {
    ProjectSchemaValidation,
    ProfileAndCatalogResolution,
    HardwareAndAddressValidation,
    VirtualNetworkValidation,
    SymbolAndReferenceResolution,
    DependencyAndSignatureAnalysis,
    LanguageAndControlFlowAnalysis,
    TypeAndConversionChecking,
    CallInstanceAndScheduleValidation,
    CapabilityAndResourceValidation,
    TypedIrLowering,
    IndependentIrVerification,
    SourceMapAndProbeConstruction,
    ReportAndArtifactPublication,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StageMetric {
    pub stage: CompilerStage,
    pub deterministic_work_units: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildOutcome {
    ObjectValid,
    SoftwareValid,
    ArtifactCreated,
    BlockingFailure,
    Cancelled,
    ResourceLimit,
    InternalFailure,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildReport {
    attempt_id: BuildAttemptId,
    snapshot_hash: Hash32,
    requested_scope: BuildScope,
    expanded_scope: Option<ExpandedScope>,
    outcome: BuildOutcome,
    diagnostics: Vec<BuildDiagnostic>,
    stage_metrics: Vec<StageMetric>,
    semantic_fingerprint: Option<Hash32>,
    artifact_fingerprint: Option<Hash32>,
    stale: bool,
}

impl BuildReport {
    #[must_use]
    pub const fn attempt_id(&self) -> BuildAttemptId {
        self.attempt_id
    }

    #[must_use]
    pub const fn snapshot_hash(&self) -> Hash32 {
        self.snapshot_hash
    }

    #[must_use]
    pub const fn requested_scope(&self) -> &BuildScope {
        &self.requested_scope
    }

    #[must_use]
    pub const fn expanded_scope(&self) -> Option<&ExpandedScope> {
        self.expanded_scope.as_ref()
    }

    #[must_use]
    pub const fn outcome(&self) -> BuildOutcome {
        self.outcome
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[BuildDiagnostic] {
        &self.diagnostics
    }

    #[must_use]
    pub fn stage_metrics(&self) -> &[StageMetric] {
        &self.stage_metrics
    }

    #[must_use]
    pub const fn semantic_fingerprint(&self) -> Option<Hash32> {
        self.semantic_fingerprint
    }

    #[must_use]
    pub const fn artifact_fingerprint(&self) -> Option<Hash32> {
        self.artifact_fingerprint
    }

    #[must_use]
    pub const fn is_stale(&self) -> bool {
        self.stale
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DependencyRecord {
    pub dependent: BlockId,
    pub dependency: BlockId,
    pub reason: DependencyReason,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildManifest {
    pub artifact_schema: &'static str,
    pub compiler_semantics_version: &'static str,
    pub type_system_version: &'static str,
    pub arithmetic_policy_version: &'static str,
    pub conversion_policy_version: &'static str,
    pub ir_version: &'static str,
    pub probe_schema_version: &'static str,
    pub instruction_registry_version: String,
    pub instruction_registry_hash: Hash32,
    pub profile_identity: String,
    pub profile_version: String,
    pub profile_hash: Hash32,
    pub capability_manifest_hash: Hash32,
    pub runtime_version: &'static str,
    pub scheduler_version: &'static str,
    pub priority_table_version: &'static str,
    pub work_cost_version: &'static str,
    pub build_scope: BuildScope,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArtifactIntegrityHashes {
    pub memory_schema_hash: Hash32,
    pub start_values_hash: Hash32,
    pub dependency_manifest_hash: Hash32,
    pub source_map_hash: Hash32,
    pub probe_table_hash: Hash32,
    pub verified_ir_hash: Hash32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildArtifact {
    snapshot_hash: Hash32,
    semantic_fingerprint: Hash32,
    package_fingerprint: Hash32,
    canonical_program: ControllerProgram,
    verified_ir: VerifiedIr,
    source_maps: SourceMapTable,
    probe_table: ProbeTable,
    dependencies: Vec<DependencyRecord>,
    manifest: BuildManifest,
    integrity: ArtifactIntegrityHashes,
}

impl BuildArtifact {
    #[must_use]
    pub const fn snapshot_hash(&self) -> Hash32 {
        self.snapshot_hash
    }

    #[must_use]
    pub const fn semantic_fingerprint(&self) -> Hash32 {
        self.semantic_fingerprint
    }

    #[must_use]
    pub const fn package_fingerprint(&self) -> Hash32 {
        self.package_fingerprint
    }

    #[must_use]
    pub const fn verified_ir(&self) -> &VerifiedIr {
        &self.verified_ir
    }

    #[must_use]
    pub const fn source_maps(&self) -> &SourceMapTable {
        &self.source_maps
    }

    #[must_use]
    pub const fn probe_table(&self) -> &ProbeTable {
        &self.probe_table
    }

    #[must_use]
    pub fn dependencies(&self) -> &[DependencyRecord] {
        &self.dependencies
    }

    #[must_use]
    pub const fn manifest(&self) -> &BuildManifest {
        &self.manifest
    }

    #[must_use]
    pub const fn integrity(&self) -> ArtifactIntegrityHashes {
        self.integrity
    }

    /// Deterministically projects the admitted linear SCL subset into the
    /// production [`plc_runtime::ArtifactPackage`] model. Unsupported typed IR
    /// fails explicitly and never receives approximate runtime behavior.
    ///
    /// # Errors
    ///
    /// Returns an identity, type, control-flow, operation, mapping, or runtime
    /// artifact validation defect.
    pub fn runtime_projection(
        &self,
    ) -> Result<crate::RuntimeArtifactProjection, crate::RuntimeAdapterError> {
        crate::runtime_adapter::project_verified_ir_to_runtime(
            &self.verified_ir,
            &self.source_maps,
            &self.probe_table,
            &self.canonical_program,
            self.manifest.profile_hash,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildCompletion {
    report: BuildReport,
    artifact: Option<BuildArtifact>,
}

impl BuildCompletion {
    #[must_use]
    pub const fn report(&self) -> &BuildReport {
        &self.report
    }

    #[must_use]
    pub const fn artifact(&self) -> Option<&BuildArtifact> {
        self.artifact.as_ref()
    }

    #[must_use]
    pub fn into_parts(self) -> (BuildReport, Option<BuildArtifact>) {
        (self.report, self.artifact)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompilerInitError {
    DiagnosticRegistry(RegistryError),
}

pub struct Compiler {
    limits: ResourceLimits,
}

impl Compiler {
    /// Initializes the compiler only after the original diagnostic registry is
    /// proven complete.
    ///
    /// # Errors
    ///
    /// Returns an internal trusted-registry defect. Project input can never
    /// cause compiler initialization to succeed with an incomplete registry.
    pub fn new(limits: ResourceLimits) -> Result<Self, CompilerInitError> {
        phase2_diagnostic_registry()
            .validate()
            .map_err(CompilerInitError::DiagnosticRegistry)?;
        Ok(Self { limits })
    }

    #[must_use]
    pub const fn limits(&self) -> ResourceLimits {
        self.limits
    }

    /// Expands prerequisites and affected dependents using the canonical typed
    /// dependency graph.
    ///
    /// # Errors
    ///
    /// Returns an unknown object, deterministic graph ceiling, or an
    /// unavailable nonsoftware scope.
    pub fn expand_scope(
        &self,
        snapshot: &BuildSnapshot,
        scope: &BuildScope,
    ) -> Result<ExpandedScope, ScopeError> {
        expand_scope(snapshot, scope, self.limits)
    }

    #[must_use]
    pub fn compile(
        &self,
        attempt: &BuildAttempt,
        current_snapshot_hash: Hash32,
        cancellation: Option<&CancellationToken>,
    ) -> BuildCompletion {
        let stale = current_snapshot_hash != attempt.snapshot.snapshot_hash;
        let mut context = BuildContext {
            attempt,
            limits: self.limits,
            meter: WorkMeter::new(self.limits, cancellation),
            diagnostics: Vec::new(),
            metrics: Vec::new(),
            expanded: None,
            stale,
            current_snapshot_hash,
        };
        match context.run() {
            Ok(completion) => completion,
            Err(stop) => context.stop_completion(stop),
        }
    }
}

struct BuildContext<'a> {
    attempt: &'a BuildAttempt,
    limits: ResourceLimits,
    meter: WorkMeter<'a>,
    diagnostics: Vec<BuildDiagnostic>,
    metrics: Vec<StageMetric>,
    expanded: Option<ExpandedScope>,
    stale: bool,
    current_snapshot_hash: Hash32,
}

impl BuildContext<'_> {
    #[allow(clippy::too_many_lines)]
    fn run(&mut self) -> Result<BuildCompletion, WorkStop> {
        self.stage(CompilerStage::ProjectSchemaValidation, 1)?;
        let validation = validate_program(&self.attempt.snapshot.program);

        self.stage(CompilerStage::ProfileAndCatalogResolution, 1)?;
        if self.attempt.snapshot.program.registry_version() != PHASE2_INSTRUCTION_REGISTRY_VERSION
            || self.attempt.snapshot.instruction_registry_hash != instruction_registry_hash()
        {
            self.push_diagnostic(
                DiagnosticCode::REGISTRY_OR_PROFILE_INVALID,
                DiagnosticTarget::Project,
                "instruction registry identity does not match the immutable build snapshot",
            )?;
        }

        // These stages are explicit and zero-work for a software-only scope;
        // they are not simulated as successful hardware/network compilation.
        self.stage(CompilerStage::HardwareAndAddressValidation, 0)?;
        self.stage(CompilerStage::VirtualNetworkValidation, 0)?;
        self.stage(CompilerStage::SymbolAndReferenceResolution, 1)?;
        self.stage(CompilerStage::DependencyAndSignatureAnalysis, 1)?;
        let expanded = match expand_scope(
            &self.attempt.snapshot,
            &self.attempt.requested_scope,
            self.limits,
        ) {
            Ok(value) => value,
            Err(ScopeError::DependencyLimit(limit)) => {
                return Err(WorkStop::Resource(limit));
            }
            Err(ScopeError::UnknownObject(block)) => {
                self.push_diagnostic(
                    DiagnosticCode::UNRESOLVED_REFERENCE,
                    DiagnosticTarget::Object(block),
                    "requested build object does not exist in the snapshot",
                )?;
                return Ok(self.failure(BuildOutcome::BlockingFailure, None, None));
            }
            Err(ScopeError::UnsupportedScope) => {
                self.push_diagnostic(
                    DiagnosticCode::CAPABILITY_UNAVAILABLE,
                    DiagnosticTarget::Project,
                    "this isolated compiler package exposes software scopes only; hardware/controller closure must be supplied by its real owning package",
                )?;
                return Ok(self.failure(BuildOutcome::BlockingFailure, None, None));
            }
        };
        self.expanded = Some(expanded.clone());
        self.meter.charge(expanded.ordered_units.len() as u64)?;

        for issue in validation.issues.iter().filter(|issue| {
            issue.primary_block.is_none()
                || issue
                    .primary_block
                    .is_some_and(|block| expanded.ordered_units.contains(&block))
        }) {
            self.push_program_issue(issue)?;
        }
        if self.has_blocking() {
            return Ok(self.failure(BuildOutcome::BlockingFailure, None, None));
        }

        self.stage(CompilerStage::LanguageAndControlFlowAnalysis, 1)?;
        let mut syntax = BTreeMap::<BlockId, SyntaxTree>::new();
        for &block_id in &expanded.ordered_units {
            self.meter.checkpoint()?;
            let Some(block) = self.attempt.snapshot.program.block(block_id) else {
                continue;
            };
            if !block.kind.is_executable() {
                continue;
            }
            if !block.instructions.is_empty() || !block.calls.is_empty() {
                self.push_diagnostic(
                    DiagnosticCode::CAPABILITY_UNAVAILABLE,
                    DiagnosticTarget::Object(block_id),
                    "a selected SCL block also contains canonical graphical/call body records that this initial SCL slice cannot lower without semantic loss",
                )?;
                continue;
            }
            let Some(source) = self.attempt.snapshot.scl_sources.get(&block_id) else {
                self.push_diagnostic(
                    DiagnosticCode::UNRESOLVED_REFERENCE,
                    DiagnosticTarget::Object(block_id),
                    "selected executable block has no canonical SCL source",
                )?;
                continue;
            };
            let tree = parse_scl(source, self.limits);
            if let Some(limit) = tree.resource_limit() {
                return Err(WorkStop::Resource(limit.clone()));
            }
            self.meter.charge(tree.tokens().len() as u64)?;
            for issue in tree.issues() {
                self.push_scl_issue(block_id, source, issue)?;
            }
            syntax.insert(block_id, tree);
        }
        if self.has_blocking() {
            return Ok(self.failure(BuildOutcome::BlockingFailure, None, None));
        }

        self.stage(CompilerStage::TypeAndConversionChecking, 1)?;
        let mut typed = Vec::new();
        for (&block_id, tree) in &syntax {
            self.meter.checkpoint()?;
            let block = self
                .attempt
                .snapshot
                .program
                .block(block_id)
                .expect("syntax owner came from snapshot block");
            for member in block.interface.members.values() {
                if crate::IrType::from_program_type(&member.data_type).is_none() {
                    self.push_diagnostic(
                        DiagnosticCode::CAPABILITY_UNAVAILABLE,
                        DiagnosticTarget::Member {
                            owner: block_id,
                            member: member.id,
                        },
                        "the initial SCL IR slice does not yet support this canonical member type",
                    )?;
                }
            }
            let (typed_block, issues) =
                bind_and_typecheck_with_program(tree, block, &self.attempt.snapshot.program);
            let source = self
                .attempt
                .snapshot
                .scl_sources
                .get(&block_id)
                .expect("syntax owner has source");
            for issue in &issues {
                self.push_scl_issue(block_id, source, issue)?;
            }
            typed.push((typed_block, source.clone()));
        }
        if self.has_blocking() {
            return Ok(self.failure(BuildOutcome::BlockingFailure, None, None));
        }

        self.stage(CompilerStage::CallInstanceAndScheduleValidation, 1)?;
        self.stage(CompilerStage::CapabilityAndResourceValidation, 1)?;
        for required in [
            "scl.assignment",
            "scl.call.fc",
            "scl.expression.baseline",
            "scl.if",
            "scl.return",
        ] {
            if self
                .attempt
                .snapshot
                .profile
                .capabilities
                .binary_search_by(|candidate| candidate.as_str().cmp(required))
                .is_err()
            {
                self.push_diagnostic(
                    DiagnosticCode::CAPABILITY_UNAVAILABLE,
                    DiagnosticTarget::Project,
                    alloc::format!(
                        "pinned profile does not declare required capability '{required}'"
                    ),
                )?;
            }
        }
        if self.has_blocking() {
            return Ok(self.failure(BuildOutcome::BlockingFailure, None, None));
        }

        self.stage(CompilerStage::TypedIrLowering, 1)?;
        let lowered = match lower_typed_blocks(&typed) {
            Ok(value) => value,
            Err(error) => {
                self.push_diagnostic(
                    DiagnosticCode::COMPILER_INVARIANT_FAILED,
                    DiagnosticTarget::Project,
                    alloc::format!("typed lowering invariant failed: {error:?}"),
                )?;
                return Ok(self.failure(BuildOutcome::InternalFailure, None, None));
            }
        };
        if lowered.operation_count > self.limits.max_ir_operations {
            return Err(WorkStop::Resource(ResourceLimit {
                key: "compiler.ir_operations",
                current: lowered.operation_count as u64,
                maximum: self.limits.max_ir_operations as u64,
            }));
        }
        self.meter.charge(lowered.operation_count as u64)?;

        self.stage(CompilerStage::IndependentIrVerification, 1)?;
        let verified_ir = match verify_typed_ir(
            lowered.ir,
            &lowered.source_maps,
            &lowered.probes,
            &self.attempt.snapshot.program,
        ) {
            Ok(value) => value,
            Err(error) => {
                self.push_diagnostic(
                    DiagnosticCode::IR_VERIFICATION_FAILED,
                    DiagnosticTarget::Project,
                    alloc::format!("independent IR verifier rejected output: {error:?}"),
                )?;
                return Ok(self.failure(BuildOutcome::InternalFailure, None, None));
            }
        };

        self.stage(CompilerStage::SourceMapAndProbeConstruction, 1)?;
        let semantic = verified_ir.program().semantic_fingerprint();
        let artifact = if self.attempt.requested_scope == BuildScope::RebuildAllSoftware {
            Some(
                self.package_artifact(
                    verified_ir,
                    lowered.source_maps,
                    lowered.probes,
                    &validation
                        .dependency_graph
                        .edges()
                        .iter()
                        .copied()
                        .collect::<Vec<_>>(),
                )?,
            )
        } else {
            None
        };

        self.stage(CompilerStage::ReportAndArtifactPublication, 1)?;
        if self.stale {
            self.push_stale_diagnostic()?;
        }
        let outcome = match self.attempt.requested_scope {
            BuildScope::CurrentObject(_) => BuildOutcome::ObjectValid,
            BuildScope::SoftwareChanges(_) => BuildOutcome::SoftwareValid,
            BuildScope::RebuildAllSoftware => BuildOutcome::ArtifactCreated,
            BuildScope::VirtualHardware | BuildScope::ControllerBuild => {
                BuildOutcome::BlockingFailure
            }
        };
        let artifact_fingerprint = artifact.as_ref().map(BuildArtifact::package_fingerprint);
        let report_semantic = artifact
            .as_ref()
            .map_or(semantic, BuildArtifact::semantic_fingerprint);
        Ok(BuildCompletion {
            report: self.report(outcome, Some(report_semantic), artifact_fingerprint),
            artifact,
        })
    }

    fn package_artifact(
        &mut self,
        verified_ir: VerifiedIr,
        source_maps: SourceMapTable,
        probes: ProbeTable,
        dependency_edges: &[DependencyEdge],
    ) -> Result<BuildArtifact, WorkStop> {
        let expanded = self.expanded.as_ref().expect("scope expanded");
        let mut dependencies: Vec<_> = dependency_edges
            .iter()
            .filter(|edge| expanded.ordered_units.contains(&edge.dependent))
            .map(|edge| DependencyRecord {
                dependent: edge.dependent,
                dependency: edge.dependency,
                reason: edge.reason,
            })
            .collect();
        dependencies.sort();
        dependencies.dedup();
        let dependency_manifest_hash = hash_dependencies(&dependencies);
        let memory_hash = memory_schema_hash(
            &self.attempt.snapshot.program,
            &expanded.ordered_units,
            false,
        );
        let start_values_hash = memory_schema_hash(
            &self.attempt.snapshot.program,
            &expanded.ordered_units,
            true,
        );
        let integrity = ArtifactIntegrityHashes {
            memory_schema_hash: memory_hash,
            start_values_hash,
            dependency_manifest_hash,
            source_map_hash: source_maps.fingerprint(),
            probe_table_hash: probes.fingerprint(),
            verified_ir_hash: verified_ir.verification_hash(),
        };
        let manifest = BuildManifest {
            artifact_schema: BUILD_ARTIFACT_SCHEMA,
            compiler_semantics_version: COMPILER_SEMANTICS_VERSION,
            type_system_version: TYPE_SYSTEM_VERSION,
            arithmetic_policy_version: ARITHMETIC_POLICY_VERSION,
            conversion_policy_version: CONVERSION_POLICY_VERSION,
            ir_version: crate::TYPED_IR_VERSION,
            probe_schema_version: PROBE_SCHEMA_VERSION,
            instruction_registry_version: self
                .attempt
                .snapshot
                .program
                .registry_version()
                .to_string(),
            instruction_registry_hash: self.attempt.snapshot.instruction_registry_hash,
            profile_identity: self.attempt.snapshot.profile.identity.clone(),
            profile_version: self.attempt.snapshot.profile.version.clone(),
            profile_hash: self.attempt.snapshot.profile.fingerprint,
            capability_manifest_hash: self.attempt.snapshot.profile.capability_manifest_hash,
            runtime_version: RUNTIME_SEMANTICS_VERSION,
            scheduler_version: SCHEDULER_VERSION,
            priority_table_version: PRIORITY_TABLE_VERSION,
            work_cost_version: WORK_COST_VERSION,
            build_scope: self.attempt.requested_scope.clone(),
        };
        let semantic_fingerprint = artifact_semantic_fingerprint(
            verified_ir.program().semantic_fingerprint(),
            &manifest,
            integrity,
            &dependencies,
        );
        let package_fingerprint = artifact_package_fingerprint(
            self.attempt.snapshot.snapshot_hash,
            semantic_fingerprint,
            &manifest,
            integrity,
            &dependencies,
        );
        let estimated_size = verified_ir
            .program()
            .functions()
            .values()
            .flat_map(|function| function.blocks.values())
            .map(|block| block.operations.len().saturating_mul(96).saturating_add(64))
            .sum::<usize>()
            .saturating_add(source_maps.entries().len().saturating_mul(128))
            .saturating_add(probes.entries().len().saturating_mul(96));
        if estimated_size > self.limits.max_artifact_bytes {
            return Err(WorkStop::Resource(ResourceLimit {
                key: "compiler.artifact_bytes",
                current: estimated_size as u64,
                maximum: self.limits.max_artifact_bytes as u64,
            }));
        }
        Ok(BuildArtifact {
            snapshot_hash: self.attempt.snapshot.snapshot_hash,
            semantic_fingerprint,
            package_fingerprint,
            canonical_program: self.attempt.snapshot.program.clone(),
            verified_ir,
            source_maps,
            probe_table: probes,
            dependencies,
            manifest,
            integrity,
        })
    }

    fn stage(&mut self, stage: CompilerStage, base_work: u64) -> Result<(), WorkStop> {
        let before = self.meter.used();
        self.meter.charge(base_work)?;
        self.metrics.push(StageMetric {
            stage,
            deterministic_work_units: self.meter.used() - before,
        });
        Ok(())
    }

    fn push_program_issue(&mut self, issue: &ProgramIssue) -> Result<(), WorkStop> {
        let code = program_issue_code(issue.code);
        let primary = issue
            .primary_block
            .map_or(DiagnosticTarget::Project, DiagnosticTarget::Object);
        let mut related = Vec::new();
        if let Some(block) = issue.related_block {
            related.push(DiagnosticTarget::Object(block));
        }
        if let (Some(owner), Some(member)) = (issue.primary_block, issue.member) {
            related.push(DiagnosticTarget::Member { owner, member });
        }
        self.push(BuildDiagnostic::new(
            self.attempt.id,
            self.attempt.snapshot.snapshot_hash,
            code,
            primary,
            related,
            Vec::new(),
            alloc::format!("canonical program validation issue: {:?}", issue.code),
        ))
    }

    fn push_scl_issue(
        &mut self,
        block: BlockId,
        source: &SclSource,
        issue: &crate::scl::SclIssue,
    ) -> Result<(), WorkStop> {
        let anchor = SourceAnchor::scl(
            block,
            source.revision_hash(),
            issue.semantic_node.unwrap_or(SemanticNodeId::new(0)),
            issue.range,
        );
        self.push(BuildDiagnostic::new(
            self.attempt.id,
            self.attempt.snapshot.snapshot_hash,
            issue.code,
            DiagnosticTarget::Source(anchor),
            Vec::new(),
            vec![DiagnosticParameter::Range(issue.range)],
            issue.cause.clone(),
        ))
    }

    fn push_diagnostic(
        &mut self,
        code: DiagnosticCode,
        target: DiagnosticTarget,
        cause: impl Into<String>,
    ) -> Result<(), WorkStop> {
        self.push(BuildDiagnostic::new(
            self.attempt.id,
            self.attempt.snapshot.snapshot_hash,
            code,
            target,
            Vec::new(),
            Vec::new(),
            cause,
        ))
    }

    fn push(&mut self, diagnostic: BuildDiagnostic) -> Result<(), WorkStop> {
        if self.diagnostics.len() >= self.limits.max_diagnostics {
            return Err(WorkStop::Resource(ResourceLimit {
                key: "compiler.diagnostics",
                current: (self.diagnostics.len() + 1) as u64,
                maximum: self.limits.max_diagnostics as u64,
            }));
        }
        self.diagnostics.push(diagnostic);
        Ok(())
    }

    fn push_stale_diagnostic(&mut self) -> Result<(), WorkStop> {
        self.push(BuildDiagnostic::new(
            self.attempt.id,
            self.attempt.snapshot.snapshot_hash,
            DiagnosticCode::STALE_BUILD_RESULT,
            DiagnosticTarget::Project,
            Vec::new(),
            vec![
                DiagnosticParameter::Hash(self.attempt.snapshot.snapshot_hash),
                DiagnosticParameter::Hash(self.current_snapshot_hash),
            ],
            "editable state no longer equals the captured immutable build snapshot",
        ))
    }

    fn has_blocking(&self) -> bool {
        self.diagnostics.iter().any(BuildDiagnostic::is_blocking)
    }

    fn failure(
        &mut self,
        outcome: BuildOutcome,
        semantic: Option<Hash32>,
        artifact: Option<Hash32>,
    ) -> BuildCompletion {
        if self.stale {
            let _ = self.push_stale_diagnostic();
        }
        BuildCompletion {
            report: self.report(outcome, semantic, artifact),
            artifact: None,
        }
    }

    fn stop_completion(&mut self, stop: WorkStop) -> BuildCompletion {
        let (outcome, cause, limit) = match stop {
            WorkStop::Cancelled => (
                BuildOutcome::Cancelled,
                "build was cooperatively cancelled".to_string(),
                None,
            ),
            WorkStop::Resource(limit) => (
                BuildOutcome::ResourceLimit,
                alloc::format!(
                    "resource '{}' requested {} with maximum {}",
                    limit.key,
                    limit.current,
                    limit.maximum
                ),
                Some(limit),
            ),
        };
        let parameters = limit.as_ref().map_or_else(Vec::new, |value| {
            vec![
                DiagnosticParameter::Text(value.key.into()),
                DiagnosticParameter::Numeric(value.current),
                DiagnosticParameter::Numeric(value.maximum),
            ]
        });
        if let Some(limit) = &limit {
            let resource_diagnostic = BuildDiagnostic::new(
                self.attempt.id,
                self.attempt.snapshot.snapshot_hash,
                DiagnosticCode::RESOURCE_LIMIT,
                DiagnosticTarget::Project,
                Vec::new(),
                vec![
                    DiagnosticParameter::Text(limit.key.into()),
                    DiagnosticParameter::Numeric(limit.current),
                    DiagnosticParameter::Numeric(limit.maximum),
                ],
                alloc::format!(
                    "deterministic resource '{}' requested {} with maximum {}",
                    limit.key,
                    limit.current,
                    limit.maximum
                ),
            );
            if self.diagnostics.len() < self.limits.max_diagnostics {
                self.diagnostics.push(resource_diagnostic);
            }
        }
        let diagnostic = BuildDiagnostic::new(
            self.attempt.id,
            self.attempt.snapshot.snapshot_hash,
            DiagnosticCode::BUILD_RESOURCE_OR_CANCEL,
            DiagnosticTarget::Project,
            Vec::new(),
            parameters,
            cause,
        );
        if self.diagnostics.len() < self.limits.max_diagnostics {
            self.diagnostics.push(diagnostic);
        }
        if self.stale {
            let _ = self.push_stale_diagnostic();
        }
        BuildCompletion {
            report: self.report(outcome, None, None),
            artifact: None,
        }
    }

    fn report(
        &mut self,
        outcome: BuildOutcome,
        semantic_fingerprint: Option<Hash32>,
        artifact_fingerprint: Option<Hash32>,
    ) -> BuildReport {
        self.diagnostics.sort_by(BuildDiagnostic::ordering);
        self.diagnostics.dedup();
        BuildReport {
            attempt_id: self.attempt.id,
            snapshot_hash: self.attempt.snapshot.snapshot_hash,
            requested_scope: self.attempt.requested_scope.clone(),
            expanded_scope: self.expanded.clone(),
            outcome,
            diagnostics: self.diagnostics.clone(),
            stage_metrics: self.metrics.clone(),
            semantic_fingerprint,
            artifact_fingerprint,
            stale: self.stale,
        }
    }
}

fn expand_scope(
    snapshot: &BuildSnapshot,
    scope: &BuildScope,
    limits: ResourceLimits,
) -> Result<ExpandedScope, ScopeError> {
    if matches!(
        scope,
        BuildScope::VirtualHardware | BuildScope::ControllerBuild
    ) {
        return Err(ScopeError::UnsupportedScope);
    }
    let validation = validate_program(&snapshot.program);
    let graph = &validation.dependency_graph;
    if graph.edges().len() > limits.max_dependency_edges {
        return Err(ScopeError::DependencyLimit(ResourceLimit {
            key: "compiler.dependency_edges",
            current: graph.edges().len() as u64,
            maximum: limits.max_dependency_edges as u64,
        }));
    }
    let initial: BTreeSet<_> = match scope {
        BuildScope::CurrentObject(block) => {
            if snapshot.program.block(*block).is_none() {
                return Err(ScopeError::UnknownObject(*block));
            }
            BTreeSet::from([*block])
        }
        BuildScope::SoftwareChanges(blocks) => {
            let set: BTreeSet<_> = blocks.iter().copied().collect();
            for block in &set {
                if snapshot.program.block(*block).is_none() {
                    return Err(ScopeError::UnknownObject(*block));
                }
            }
            set
        }
        BuildScope::RebuildAllSoftware => snapshot.program.blocks().keys().copied().collect(),
        BuildScope::VirtualHardware | BuildScope::ControllerBuild => unreachable!(),
    };
    let affected_dependents = if matches!(scope, BuildScope::SoftwareChanges(_)) {
        traverse(&initial, |block| {
            graph.dependents_of(block).map(|edge| edge.dependent)
        })
    } else {
        BTreeSet::new()
    };
    let mut roots = initial.clone();
    roots.extend(affected_dependents.iter().copied());
    let prerequisite_closure = traverse(&roots, |block| {
        graph.dependencies_of(block).map(|edge| edge.dependency)
    });
    let prerequisites: BTreeSet<_> = prerequisite_closure.difference(&roots).copied().collect();
    let mut ordered_units = roots;
    ordered_units.extend(prerequisite_closure.iter().copied());
    if ordered_units.len() > limits.max_dependency_edges {
        return Err(ScopeError::DependencyLimit(ResourceLimit {
            key: "compiler.dependency_traversal",
            current: ordered_units.len() as u64,
            maximum: limits.max_dependency_edges as u64,
        }));
    }
    Ok(ExpandedScope {
        requested: scope.clone(),
        ordered_units: ordered_units.into_iter().collect(),
        prerequisites: prerequisites.into_iter().collect(),
        affected_dependents: affected_dependents.difference(&initial).copied().collect(),
    })
}

fn traverse<F, I>(roots: &BTreeSet<BlockId>, neighbors: F) -> BTreeSet<BlockId>
where
    F: Fn(BlockId) -> I,
    I: Iterator<Item = BlockId>,
{
    let mut visited = roots.clone();
    let mut queue: VecDeque<_> = roots.iter().copied().collect();
    while let Some(block) = queue.pop_front() {
        for neighbor in neighbors(block) {
            if visited.insert(neighbor) {
                queue.push_back(neighbor);
            }
        }
    }
    visited
}

fn program_issue_code(code: IssueCode) -> DiagnosticCode {
    match code {
        IssueCode::ModelSchemaMismatch | IssueCode::RegistryVersionMismatch => {
            DiagnosticCode::REGISTRY_OR_PROFILE_INVALID
        }
        IssueCode::ModelLimitExceeded
        | IssueCode::InterfaceLimitExceeded
        | IssueCode::InstructionOrderMismatch
        | IssueCode::CallOrderMismatch
        | IssueCode::BindingOrderMismatch => DiagnosticCode::RESOURCE_LIMIT,
        IssueCode::MissingCallee | IssueCode::UnknownActual | IssueCode::UnknownFormal => {
            DiagnosticCode::UNRESOLVED_REFERENCE
        }
        IssueCode::RecursiveCallCycle => DiagnosticCode::RECURSIVE_CALL_CYCLE,
        IssueCode::MissingBinding => DiagnosticCode::REQUIRED_BINDING_MISSING,
        IssueCode::BindingDirection | IssueCode::DuplicateBinding | IssueCode::AliasConflict => {
            DiagnosticCode::ILLEGAL_OR_OVERLAPPING_BINDING
        }
        IssueCode::MissingInstanceOwner
        | IssueCode::UnexpectedInstanceOwner
        | IssueCode::InvalidInstanceDb
        | IssueCode::InstanceTypeMismatch
        | IssueCode::InvalidMultiInstance
        | IssueCode::StateOwnershipCycle => DiagnosticCode::INSTANCE_INVALID,
        IssueCode::BindingTypeMismatch
        | IssueCode::InstructionStateTypeMismatch
        | IssueCode::MemberValueTypeMismatch => DiagnosticCode::TYPE_MISMATCH,
        IssueCode::UnknownInstruction => DiagnosticCode::CAPABILITY_UNAVAILABLE,
        IssueCode::CallInstructionMismatch
        | IssueCode::MissingInstructionState
        | IssueCode::UnexpectedInstructionState
        | IssueCode::UnknownInstructionState
        | IssueCode::InstructionStateAlias
        | IssueCode::CallInstructionInBody => DiagnosticCode::COMPILER_INVARIANT_FAILED,
        IssueCode::DuplicateMemberName => DiagnosticCode::AMBIGUOUS_REFERENCE,
        IssueCode::BlockKeyMismatch
        | IssueCode::InvalidIdentifier
        | IssueCode::DuplicateEngineeringNumber
        | IssueCode::MissingCyclicMain
        | IssueCode::MultipleCyclicMain
        | IssueCode::MultipleStartup
        | IssueCode::InvalidTimedCyclic
        | IssueCode::InterfaceKeyMismatch
        | IssueCode::InterfaceOrderMismatch
        | IssueCode::DuplicateDeclaredOrder
        | IssueCode::MultipleReturn
        | IssueCode::RoleNotAllowed
        | IssueCode::MemberMetadataIllegal
        | IssueCode::InstanceTypeIllegal
        | IssueCode::BodyNotAllowed
        | IssueCode::DuplicateInstructionUseId
        | IssueCode::DuplicateCallSiteId
        | IssueCode::IllegalCaller
        | IssueCode::IllegalCallee => DiagnosticCode::MALFORMED_STRUCTURE,
    }
}

fn hash_capabilities(capabilities: &[String]) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-CAPABILITY-MANIFEST-1");
    hasher.u64(capabilities.len() as u64);
    for capability in capabilities {
        hasher.string(capability);
    }
    hasher.finish()
}

fn instruction_registry_hash() -> Hash32 {
    let registry = phase2_instruction_registry();
    let mut hasher = CanonicalHasher::new("PES-INSTRUCTION-REGISTRY-1");
    hasher.u16(registry.schema_version);
    hasher.string(registry.semantic_version);
    hasher.u64(registry.definitions().len() as u64);
    for definition in registry.definitions() {
        hasher.u16(definition.code.0);
        hasher.string(definition.mnemonic);
        hasher.u8(match definition.category {
            InstructionCategory::Stateless => 1,
            InstructionCategory::Edge => 2,
            InstructionCategory::Timer => 3,
            InstructionCategory::Counter => 4,
            InstructionCategory::Call => 5,
            InstructionCategory::Control => 6,
            InstructionCategory::Instrumentation => 7,
        });
        match definition.state_requirement {
            StateRequirement::None => hasher.u8(0),
            StateRequirement::Explicit(state) => {
                hasher.u8(1);
                encode_state_kind(&mut hasher, state);
            }
            StateRequirement::FunctionBlockInstance => hasher.u8(2),
        }
        hasher.u8(match definition.side_effect {
            SideEffectClass::Pure => 1,
            SideEffectClass::WritesValue => 2,
            SideEffectClass::WritesState => 3,
            SideEffectClass::CallsBlock => 4,
            SideEffectClass::ControlsFlow => 5,
            SideEffectClass::ObservesOnly => 6,
        });
        hasher.u16(definition.work_units);
    }
    hasher.finish()
}

fn hash_dependencies(dependencies: &[DependencyRecord]) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-DEPENDENCY-MANIFEST-1");
    hasher.u64(dependencies.len() as u64);
    for dependency in dependencies {
        hasher.u128(dependency.dependent.get());
        hasher.u128(dependency.dependency.get());
        hasher.u8(dependency.reason as u8);
    }
    hasher.finish()
}

fn memory_schema_hash(
    program: &ControllerProgram,
    closure: &[BlockId],
    values_only: bool,
) -> Hash32 {
    let domain = if values_only {
        "PES-START-VALUES-1"
    } else {
        "PES-MEMORY-SCHEMA-1"
    };
    let mut hasher = CanonicalHasher::new(domain);
    for block_id in closure {
        let Some(block) = program.block(*block_id) else {
            continue;
        };
        hasher.u128(block.id.get());
        for member in block.interface.members.values() {
            hasher.u128(member.id.get());
            if !values_only {
                encode_data_type(&mut hasher, &member.data_type);
                hasher.u8(member.role as u8);
                hasher.bool(member.retain_policy == Some(RetainPolicy::Retentive));
            }
            encode_optional_value(&mut hasher, member.start_value.as_ref());
            encode_optional_value(&mut hasher, member.default_value.as_ref());
            encode_optional_value(&mut hasher, member.constant_value.as_ref());
        }
    }
    hasher.finish()
}

fn artifact_semantic_fingerprint(
    ir_hash: Hash32,
    manifest: &BuildManifest,
    integrity: ArtifactIntegrityHashes,
    dependencies: &[DependencyRecord],
) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-SEMANTIC-BUILD-1");
    hasher.hash(ir_hash);
    encode_semantic_manifest(&mut hasher, manifest);
    hasher.hash(integrity.memory_schema_hash);
    hasher.hash(integrity.start_values_hash);
    hasher.hash(integrity.dependency_manifest_hash);
    hasher.u64(dependencies.len() as u64);
    hasher.finish()
}

fn artifact_package_fingerprint(
    snapshot_hash: Hash32,
    semantic_fingerprint: Hash32,
    manifest: &BuildManifest,
    integrity: ArtifactIntegrityHashes,
    dependencies: &[DependencyRecord],
) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-ARTIFACT-PACKAGE-1");
    hasher.hash(snapshot_hash);
    hasher.hash(semantic_fingerprint);
    encode_semantic_manifest(&mut hasher, manifest);
    hasher.hash(integrity.memory_schema_hash);
    hasher.hash(integrity.start_values_hash);
    hasher.hash(integrity.dependency_manifest_hash);
    hasher.hash(integrity.source_map_hash);
    hasher.hash(integrity.probe_table_hash);
    hasher.hash(integrity.verified_ir_hash);
    hasher.u64(dependencies.len() as u64);
    for dependency in dependencies {
        hasher.u128(dependency.dependent.get());
        hasher.u128(dependency.dependency.get());
        hasher.u8(dependency.reason as u8);
    }
    // package_fingerprint is intentionally absent from its own hash domain.
    hasher.finish()
}

fn encode_semantic_manifest(hasher: &mut CanonicalHasher, manifest: &BuildManifest) {
    hasher.string(manifest.artifact_schema);
    hasher.string(manifest.compiler_semantics_version);
    hasher.string(manifest.type_system_version);
    hasher.string(manifest.arithmetic_policy_version);
    hasher.string(manifest.conversion_policy_version);
    hasher.string(manifest.ir_version);
    hasher.string(manifest.probe_schema_version);
    hasher.string(&manifest.instruction_registry_version);
    hasher.hash(manifest.instruction_registry_hash);
    hasher.string(&manifest.profile_identity);
    hasher.string(&manifest.profile_version);
    hasher.hash(manifest.profile_hash);
    hasher.hash(manifest.capability_manifest_hash);
    hasher.string(manifest.runtime_version);
    hasher.string(manifest.scheduler_version);
    hasher.string(manifest.priority_table_version);
    hasher.string(manifest.work_cost_version);
    encode_build_scope(hasher, &manifest.build_scope);
}

fn encode_build_scope(hasher: &mut CanonicalHasher, scope: &BuildScope) {
    match scope {
        BuildScope::CurrentObject(block) => {
            hasher.u8(1);
            hasher.u128(block.get());
        }
        BuildScope::SoftwareChanges(blocks) => {
            hasher.u8(2);
            let canonical: BTreeSet<_> = blocks.iter().copied().collect();
            hasher.u64(canonical.len() as u64);
            for block in canonical {
                hasher.u128(block.get());
            }
        }
        BuildScope::RebuildAllSoftware => hasher.u8(3),
        BuildScope::VirtualHardware => hasher.u8(4),
        BuildScope::ControllerBuild => hasher.u8(5),
    }
}

fn encode_program(hasher: &mut CanonicalHasher, program: &ControllerProgram) {
    hasher.u16(program.schema_version());
    hasher.u128(program.controller_id().get());
    hasher.u64(program.semantic_revision());
    hasher.string(program.registry_version());
    hasher.u64(program.blocks().len() as u64);
    for (id, block) in program.blocks() {
        hasher.u128(id.get());
        hasher.string(&block.display_name);
        hasher.u16(block.engineering_number.get());
        encode_program_kind(hasher, block.kind);
        hasher.u64(block.interface.ordered_member_ids.len() as u64);
        for member_id in &block.interface.ordered_member_ids {
            hasher.u128(member_id.get());
        }
        hasher.u64(block.interface.members.len() as u64);
        for (member_id, member) in &block.interface.members {
            hasher.u128(member_id.get());
            encode_member(hasher, member);
        }
        hasher.u64(block.instructions.len() as u64);
        for instruction in &block.instructions {
            hasher.u128(instruction.id.get());
            hasher.u16(instruction.instruction.0);
            encode_optional_variable(hasher, instruction.state_owner.as_ref());
        }
        hasher.u64(block.calls.len() as u64);
        for call in &block.calls {
            hasher.u128(call.id.get());
            hasher.u16(call.instruction.0);
            hasher.u128(call.callee.get());
            match call.instance_owner {
                Some(InstanceOwner::InstanceDb(block)) => {
                    hasher.u8(1);
                    hasher.u128(block.get());
                }
                Some(InstanceOwner::MultiInstance {
                    owner_fb,
                    static_member,
                }) => {
                    hasher.u8(2);
                    hasher.u128(owner_fb.get());
                    hasher.u128(static_member.get());
                }
                None => hasher.u8(0),
            }
            hasher.u64(call.bindings.len() as u64);
            for binding in &call.bindings {
                hasher.u128(binding.formal.get());
                match &binding.actual {
                    BindingActual::Literal(value) => {
                        hasher.u8(1);
                        encode_value(hasher, value);
                    }
                    BindingActual::Variable(variable) => {
                        hasher.u8(2);
                        encode_variable(hasher, variable);
                    }
                }
            }
        }
    }
}

fn encode_member(hasher: &mut CanonicalHasher, member: &InterfaceMember) {
    hasher.u128(member.id.get());
    hasher.string(&member.name);
    hasher.u8(member.role as u8);
    encode_data_type(hasher, &member.data_type);
    hasher.u32(member.declared_order);
    encode_optional_value(hasher, member.default_value.as_ref());
    encode_optional_value(hasher, member.start_value.as_ref());
    encode_optional_value(hasher, member.constant_value.as_ref());
    match member.retain_policy {
        Some(RetainPolicy::NonRetentive) => hasher.u8(1),
        Some(RetainPolicy::Retentive) => hasher.u8(2),
        None => hasher.u8(0),
    }
    hasher.bool(member.required_output_binding);
}

fn encode_program_kind(hasher: &mut CanonicalHasher, kind: ProgramUnitKind) {
    match kind {
        ProgramUnitKind::OrganizationBlock(declaration) => {
            hasher.u8(1);
            match declaration {
                ObDeclaration::CyclicMain => hasher.u8(1),
                ObDeclaration::Startup => hasher.u8(2),
                ObDeclaration::TimedCyclic {
                    period_milliseconds,
                    offset_milliseconds,
                    priority,
                } => {
                    hasher.u8(3);
                    hasher.u32(period_milliseconds);
                    hasher.u32(offset_milliseconds);
                    hasher.u16(priority);
                }
            }
        }
        ProgramUnitKind::Function => hasher.u8(2),
        ProgramUnitKind::FunctionBlock => hasher.u8(3),
        ProgramUnitKind::DataBlock(DataBlockKind::Global) => hasher.u8(4),
        ProgramUnitKind::DataBlock(DataBlockKind::Instance { fb_type }) => {
            hasher.u8(5);
            hasher.u128(fb_type.get());
        }
    }
}

fn encode_data_type(hasher: &mut CanonicalHasher, data_type: &DataType) {
    match data_type {
        DataType::Bool => hasher.u8(1),
        DataType::Int => hasher.u8(2),
        DataType::DInt => hasher.u8(3),
        DataType::Real => hasher.u8(4),
        DataType::Time => hasher.u8(5),
        DataType::String { capacity } => {
            hasher.u8(6);
            hasher.u16(*capacity);
        }
        DataType::Named(name) => {
            hasher.u8(7);
            hasher.string(name);
        }
        DataType::BlockInstance(block) => {
            hasher.u8(8);
            hasher.u128(block.get());
        }
        DataType::InstructionState(state) => {
            hasher.u8(9);
            encode_state_kind(hasher, *state);
        }
    }
}

fn encode_optional_value(hasher: &mut CanonicalHasher, value: Option<&CanonicalValue>) {
    match value {
        Some(value) => {
            hasher.bool(true);
            encode_value(hasher, value);
        }
        None => hasher.bool(false),
    }
}

fn encode_value(hasher: &mut CanonicalHasher, value: &CanonicalValue) {
    match value {
        CanonicalValue::Bool(value) => {
            hasher.u8(1);
            hasher.bool(*value);
        }
        CanonicalValue::Int(value) => {
            hasher.u8(2);
            hasher.i32(i32::from(*value));
        }
        CanonicalValue::DInt(value) => {
            hasher.u8(3);
            hasher.i32(*value);
        }
        CanonicalValue::RealBits(value) => {
            hasher.u8(4);
            hasher.u32(*value);
        }
        CanonicalValue::TimeMilliseconds(value) => {
            hasher.u8(5);
            hasher.i64(*value);
        }
        CanonicalValue::StringBytes(value) => {
            hasher.u8(6);
            hasher.bytes(value);
        }
    }
}

fn encode_optional_variable(hasher: &mut CanonicalHasher, value: Option<&VariableRef>) {
    match value {
        Some(value) => {
            hasher.bool(true);
            encode_variable(hasher, value);
        }
        None => hasher.bool(false),
    }
}

fn encode_variable(hasher: &mut CanonicalHasher, value: &VariableRef) {
    match value {
        VariableRef::CallerMember(member) => {
            hasher.u8(1);
            hasher.u128(member.get());
        }
        VariableRef::DataBlockMember { data_block, member } => {
            hasher.u8(2);
            hasher.u128(data_block.get());
            hasher.u128(member.get());
        }
    }
}

fn encode_state_kind(hasher: &mut CanonicalHasher, value: StateKind) {
    hasher.u8(match value {
        StateKind::Edge => 1,
        StateKind::Timer => 2,
        StateKind::Counter => 3,
    });
}

#[allow(dead_code)]
fn reproducible_failure_evidence(stage: CompilerStage, stable_identity: u128) -> Hash32 {
    let mut hasher = CanonicalHasher::new("PES-COMPILER-FAILURE-EVIDENCE-1");
    hasher.u8(stage as u8);
    hasher.u128(stable_identity);
    hasher.string(COMPILER_SEMANTICS_VERSION);
    hasher.finish()
}
