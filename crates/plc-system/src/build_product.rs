use std::collections::{BTreeMap, BTreeSet};

use plc_commissioning::{
    LoadPackageParts, MemoryMemberSchema, MemoryRole, StateKind as LoadStateKind,
    StateMemberSchema, VirtualLoadPackage,
};
use plc_compiler::{
    CompilerProfile, ComposedFrontendArtifact, FrontendArtifact, ResourceLimits,
    RuntimeArtifactProjection, SourceLanguage, compose_frontend_artifacts,
    lower_scl_frontend_artifact, project_verified_ir_to_runtime,
};
use plc_core::{ObjectId, Project, Sha256Digest, sha256};
use plc_hardware::{
    ChannelAddress, ChannelDirection as HardwareDirection, ChannelId as HardwareChannelId,
    HardwareChannelBinding, PrimitiveType, TrainingProfile,
};
use plc_lad::{LadLimits, lower_lad_to_ir};
use plc_language_tools::lower_fbd_to_verified_ir;
use plc_observability::{
    AccessCapabilities, BitRange, ProbeCatalog, ProbeDefinition, RuntimeTarget, StableTargetId,
};
use plc_program::{BlockId, DataType, InterfaceRole};
use plc_runtime::{
    ArtifactPackage, ArtifactSpec, ChannelDefinition, ChannelDirection, ChannelId, Hash32,
    Instruction, MemoryId, Operand, Operation, ProgramBlock as RuntimeProgramBlock, ProgramImage,
    StateStart, ValueType,
};

use crate::software_projection::object_u128;
use crate::{
    CanonicalAddressArea, CanonicalAddressIntent, CanonicalHardwareProjection,
    CanonicalSoftwareProjection, CanonicalTag, DecodedGraphicalBody, GraphDecodeError,
    ProjectDiagnostic, decode_graphical_body, project_hardware, project_software,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeChannelBinding {
    pub hardware_channel_id: HardwareChannelId,
    pub runtime_channel_id: ChannelId,
    pub hardware: HardwareChannelBinding,
}

#[derive(Clone, Debug)]
struct ResolvedCanonicalTag {
    tag: CanonicalTag,
    program_block: BlockId,
    runtime_block: plc_runtime::BlockId,
    program_memory: MemoryId,
    runtime_target: RuntimeTarget,
    value_type: ValueType,
}

#[derive(Clone, Debug)]
pub struct SystemBuildProduct {
    source_document_hash: Sha256Digest,
    source_semantic_fingerprint: Sha256Digest,
    controller_object_id: ObjectId,
    compiler_artifact: SystemCompilerArtifact,
    runtime_projection: RuntimeArtifactProjection,
    runtime_artifact: ArtifactPackage,
    load_package: VirtualLoadPackage,
    probe_catalog: ProbeCatalog,
    hardware: CanonicalHardwareProjection,
    software: CanonicalSoftwareProjection,
    channel_bindings: Vec<RuntimeChannelBinding>,
}

/// The compiler-owned, independently verified whole-controller result used by
/// the runtime adapter. It is language-neutral and keeps the exact merged
/// source/probe tables needed for navigation.
#[derive(Clone, Debug)]
pub struct SystemCompilerArtifact {
    composed: ComposedFrontendArtifact,
    profile_identity: String,
    profile_version: String,
    profile_manifest_hash: Hash32,
    capability_manifest_hash: Hash32,
}

impl SystemCompilerArtifact {
    #[must_use]
    pub const fn composed(&self) -> &ComposedFrontendArtifact {
        &self.composed
    }

    #[must_use]
    pub fn profile_identity(&self) -> &str {
        &self.profile_identity
    }

    #[must_use]
    pub fn profile_version(&self) -> &str {
        &self.profile_version
    }

    #[must_use]
    pub const fn profile_manifest_hash(&self) -> Hash32 {
        self.profile_manifest_hash
    }

    /// Compatibility alias for commissioning/runtime APIs that still name
    /// this field a fingerprint. It is exactly the canonical profile manifest.
    #[must_use]
    pub const fn profile_fingerprint(&self) -> Hash32 {
        self.profile_manifest_hash
    }

    #[must_use]
    pub const fn capability_manifest_hash(&self) -> Hash32 {
        self.capability_manifest_hash
    }
}

impl SystemBuildProduct {
    #[must_use]
    pub const fn source_document_hash(&self) -> Sha256Digest {
        self.source_document_hash
    }

    #[must_use]
    pub const fn source_semantic_fingerprint(&self) -> Sha256Digest {
        self.source_semantic_fingerprint
    }

    #[must_use]
    pub const fn controller_object_id(&self) -> ObjectId {
        self.controller_object_id
    }

    #[must_use]
    pub const fn compiler_artifact(&self) -> &SystemCompilerArtifact {
        &self.compiler_artifact
    }

    #[must_use]
    pub const fn runtime_projection(&self) -> &RuntimeArtifactProjection {
        &self.runtime_projection
    }

    #[must_use]
    pub const fn runtime_artifact(&self) -> &ArtifactPackage {
        &self.runtime_artifact
    }

    #[must_use]
    pub const fn load_package(&self) -> &VirtualLoadPackage {
        &self.load_package
    }

    #[must_use]
    pub const fn probe_catalog(&self) -> &ProbeCatalog {
        &self.probe_catalog
    }

    #[must_use]
    pub const fn hardware(&self) -> &CanonicalHardwareProjection {
        &self.hardware
    }

    #[must_use]
    pub const fn software(&self) -> &CanonicalSoftwareProjection {
        &self.software
    }

    #[must_use]
    pub fn channel_bindings(&self) -> &[RuntimeChannelBinding] {
        &self.channel_bindings
    }

    #[must_use]
    pub fn runtime_channel_for(&self, hardware: HardwareChannelId) -> Option<ChannelId> {
        self.channel_bindings
            .binary_search_by_key(&hardware, |binding| binding.hardware_channel_id)
            .ok()
            .map(|index| self.channel_bindings[index].runtime_channel_id)
    }
}

#[derive(Clone, Debug)]
pub enum SystemBuildError {
    ProjectionBlocked(Vec<ProjectDiagnostic>),
    Profile(String),
    MissingAuthoredBody(BlockId),
    DuplicateAuthoredBody(BlockId),
    GraphDecode(GraphDecodeError),
    SclFrontend { owner: BlockId, detail: String },
    LadFrontend { owner: BlockId, detail: String },
    FbdFrontend { owner: BlockId, detail: String },
    Composition(String),
    RuntimeProjection(String),
    RuntimeArtifact(String),
    LoadPackage(String),
    ProbeCatalog(String),
}

/// Builds one controller exclusively from an immutable canonical project.
///
/// # Errors
///
/// Returns a typed, deterministic failure at the first trust boundary that
/// cannot produce a complete verified runtime package. Partial artifacts are
/// never published.
pub fn build_project_controller(
    project: &Project,
    controller_object_id: ObjectId,
) -> Result<SystemBuildProduct, SystemBuildError> {
    let hardware = project_hardware(project);
    let software = project_software(project, controller_object_id);
    let mut projection_diagnostics = hardware.diagnostics().to_vec();
    projection_diagnostics.extend_from_slice(software.diagnostics());
    projection_diagnostics.sort();
    projection_diagnostics.dedup();
    if !hardware.can_build() || !software.can_compile() {
        return Err(SystemBuildError::ProjectionBlocked(projection_diagnostics));
    }

    let profile = CompilerProfile::from_training_profile(hardware.profile())
        .map_err(|error| SystemBuildError::Profile(format!("{error:?}")))?;
    let composed = compile_frontends(&software, hardware.profile(), profile.resource_limits())?;
    let runtime_projection = project_verified_ir_to_runtime(
        composed.verified_ir(),
        composed.source_maps(),
        composed.probes(),
        software.program(),
        profile.profile_manifest_hash(),
    )
    .map_err(|error| SystemBuildError::RuntimeProjection(format!("{error:?}")))?;
    let compiler_artifact = SystemCompilerArtifact {
        composed,
        profile_identity: profile.identity().to_owned(),
        profile_version: profile.version().to_owned(),
        profile_manifest_hash: profile.profile_manifest_hash(),
        capability_manifest_hash: profile.capability_manifest_hash(),
    };
    let hardware_artifact = hardware
        .artifact()
        .ok_or_else(|| SystemBuildError::ProjectionBlocked(hardware.diagnostics().to_vec()))?;
    let (channel_definitions, channel_bindings) =
        project_runtime_channels(hardware_artifact.channel_bindings.values(), &hardware)?;
    let compiler_spec = runtime_projection.package().spec();
    let resolved_tags = resolve_tags(&runtime_projection, &software, &channel_bindings)?;
    let mut runtime_spec = ArtifactSpec {
        schema_version: compiler_spec.schema_version,
        runtime_version: compiler_spec.runtime_version.clone(),
        scheduler_version: compiler_spec.scheduler_version.clone(),
        priority_table_version: compiler_spec.priority_table_version.clone(),
        work_cost_version: compiler_spec.work_cost_version.clone(),
        profile_fingerprint: compiler_spec.profile_fingerprint,
        memory: compiler_spec.memory.clone(),
        channels: channel_definitions,
        states: compiler_spec.states.clone(),
        program: compiler_spec.program.clone(),
    };
    inject_io_binding_operations(&mut runtime_spec.program, &resolved_tags)?;
    let runtime_artifact = ArtifactPackage::seal_verified(runtime_spec)
        .map_err(|error| SystemBuildError::RuntimeArtifact(format!("{error:?}")))?;

    let probe_catalog = build_probe_catalog(&runtime_artifact, &resolved_tags)?;
    let load_package = build_load_package(
        project,
        &compiler_artifact,
        &runtime_projection,
        &runtime_artifact,
        hardware_artifact.hardware_fingerprint,
        probe_catalog.catalog_hash(),
    )?;

    Ok(SystemBuildProduct {
        source_document_hash: project.document_hash(),
        source_semantic_fingerprint: project.semantic_fingerprint(),
        controller_object_id,
        compiler_artifact,
        runtime_projection,
        runtime_artifact,
        load_package,
        probe_catalog,
        hardware,
        software,
        channel_bindings,
    })
}

fn compile_frontends(
    software: &CanonicalSoftwareProjection,
    profile: &TrainingProfile,
    compiler_limits: ResourceLimits,
) -> Result<ComposedFrontendArtifact, SystemBuildError> {
    let program = software.program();
    let graphical = software
        .graphical_bodies()
        .iter()
        .map(|body| (body.owner_block_id, body))
        .collect::<BTreeMap<_, _>>();
    if graphical.len() != software.graphical_bodies().len() {
        let mut seen = BTreeSet::new();
        let duplicate = software
            .graphical_bodies()
            .iter()
            .find_map(|body| (!seen.insert(body.owner_block_id)).then_some(body.owner_block_id))
            .expect("length mismatch guarantees a duplicate");
        return Err(SystemBuildError::DuplicateAuthoredBody(duplicate));
    }

    let mut artifacts = Vec::new();
    for (&owner, block) in program.blocks() {
        if !block.kind.is_executable() {
            continue;
        }
        let scl = software.scl_sources().get(&owner);
        let graph = graphical.get(&owner).copied();
        let artifact = match (scl, graph) {
            (Some(_), Some(_)) => return Err(SystemBuildError::DuplicateAuthoredBody(owner)),
            (None, None) => return Err(SystemBuildError::MissingAuthoredBody(owner)),
            (Some(source), None) => lower_scl_frontend_artifact(program, source, compiler_limits)
                .map_err(|error| SystemBuildError::SclFrontend {
                owner,
                detail: format!("{error:?}"),
            })?,
            (None, Some(body)) => {
                match decode_graphical_body(body).map_err(SystemBuildError::GraphDecode)? {
                    DecodedGraphicalBody::Lad(document) => {
                        let limits = profile.limits();
                        let lad_limits = LadLimits {
                            max_networks: profile_limit(
                                limits.networks_per_block,
                                "networks_per_block",
                            )?,
                            max_nodes_per_network: profile_limit(
                                limits.nodes_per_network,
                                "nodes_per_network",
                            )?,
                            max_edges_per_network: profile_limit(
                                limits.edges_per_network,
                                "edges_per_network",
                            )?,
                            max_diagnostics: compiler_limits.max_diagnostics,
                        };
                        let lowered =
                            lower_lad_to_ir(&document, program, lad_limits).map_err(|error| {
                                SystemBuildError::LadFrontend {
                                    owner,
                                    detail: format!("{error:?}"),
                                }
                            })?;
                        FrontendArtifact::new(
                            owner,
                            SourceLanguage::Lad,
                            lowered.verified_ir,
                            lowered.source_maps,
                            lowered.probes,
                        )
                    }
                    DecodedGraphicalBody::Fbd(document) => {
                        validate_fbd_profile_limits(&document, profile)?;
                        let lowered =
                            lower_fbd_to_verified_ir(&document, program).map_err(|error| {
                                SystemBuildError::FbdFrontend {
                                    owner,
                                    detail: format!("{error:?}"),
                                }
                            })?;
                        FrontendArtifact::new(
                            owner,
                            SourceLanguage::Fbd,
                            lowered.verified_ir,
                            lowered.lowered.compiler_source_maps,
                            lowered.lowered.compiler_probes,
                        )
                    }
                }
            }
        };
        artifacts.push(artifact);
    }
    compose_frontend_artifacts(program, &artifacts)
        .map_err(|error| SystemBuildError::Composition(format!("{error:?}")))
}

fn profile_limit(value: u32, field: &'static str) -> Result<usize, SystemBuildError> {
    usize::try_from(value).map_err(|_| {
        SystemBuildError::Profile(format!(
            "training profile limit '{field}' is not representable on this target"
        ))
    })
}

fn validate_fbd_profile_limits(
    document: &plc_language_tools::FbdDocument,
    profile: &TrainingProfile,
) -> Result<(), SystemBuildError> {
    let limits = profile.limits();
    let max_networks = profile_limit(limits.networks_per_block, "networks_per_block")?;
    let max_nodes = profile_limit(limits.nodes_per_network, "nodes_per_network")?;
    let max_edges = profile_limit(limits.edges_per_network, "edges_per_network")?;
    if document.networks.len() > max_networks {
        return Err(SystemBuildError::FbdFrontend {
            owner: document.owner,
            detail: "FBD network count exceeds the admitted training-profile limit".to_owned(),
        });
    }
    if let Some(network) = document
        .networks
        .values()
        .find(|network| network.nodes.len() > max_nodes || network.connections.len() > max_edges)
    {
        return Err(SystemBuildError::FbdFrontend {
            owner: document.owner,
            detail: format!(
                "FBD network {:?} exceeds the admitted training-profile node/edge limits",
                network.id
            ),
        });
    }
    Ok(())
}

fn project_runtime_channels<'a>(
    bindings: impl Iterator<Item = &'a HardwareChannelBinding>,
    hardware: &CanonicalHardwareProjection,
) -> Result<(Vec<ChannelDefinition>, Vec<RuntimeChannelBinding>), SystemBuildError> {
    let mut definitions = Vec::new();
    let mut projected = Vec::new();
    for (index, binding) in bindings.enumerate() {
        let runtime_id = u32::try_from(index + 1).map(ChannelId::new).map_err(|_| {
            SystemBuildError::RuntimeArtifact("runtime channel identity space exhausted".to_owned())
        })?;
        let value_type = hardware_value_type(binding.raw_type).ok_or_else(|| {
            let origin = hardware
                .origin_for(binding.module_id.uuid())
                .map_or_else(|| "unknown module".to_owned(), |id| id.to_string());
            SystemBuildError::RuntimeArtifact(format!(
                "hardware channel {} from {origin} uses unsupported runtime type {}",
                binding.channel_index,
                binding.raw_type.stable_id()
            ))
        })?;
        let direction = match binding.direction {
            HardwareDirection::Input => ChannelDirection::Input,
            HardwareDirection::Output => ChannelDirection::Output,
        };
        definitions.push(ChannelDefinition {
            id: runtime_id,
            direction,
            value_type,
            canonical_default: value_type.canonical_default(),
        });
        projected.push(RuntimeChannelBinding {
            hardware_channel_id: binding.channel_id,
            runtime_channel_id: runtime_id,
            hardware: binding.clone(),
        });
    }
    projected.sort_by_key(|binding| binding.hardware_channel_id);
    Ok((definitions, projected))
}

fn hardware_value_type(value: PrimitiveType) -> Option<ValueType> {
    ValueType::from_primitive(value)
}

fn program_value_type(value: &DataType) -> Option<ValueType> {
    value.primitive_type().and_then(ValueType::from_primitive)
}

fn build_probe_catalog(
    runtime_artifact: &ArtifactPackage,
    resolved_tags: &[ResolvedCanonicalTag],
) -> Result<ProbeCatalog, SystemBuildError> {
    let mut catalog = ProbeCatalog::new(
        runtime_artifact.fingerprint(),
        runtime_artifact.spec().profile_fingerprint,
    );
    for resolved in resolved_tags {
        let tag = &resolved.tag;
        catalog
            .insert(ProbeDefinition {
                id: StableTargetId(tag.stable_identity),
                runtime_target: resolved.runtime_target,
                bit_range: BitRange::whole_value(),
                value_type: resolved.value_type,
                instance_path: Vec::new(),
                capabilities: AccessCapabilities {
                    monitor: true,
                    modify: true,
                    force: true,
                    trace: true,
                    natural_layer: true,
                    effective_layer: true,
                },
                primary_source: None,
                display_name: tag.display_name.clone(),
            })
            .map_err(|error| SystemBuildError::ProbeCatalog(format!("{error:?}")))?;
    }
    Ok(catalog)
}

#[allow(clippy::too_many_lines)]
fn resolve_tags(
    runtime_projection: &RuntimeArtifactProjection,
    software: &CanonicalSoftwareProjection,
    channel_bindings: &[RuntimeChannelBinding],
) -> Result<Vec<ResolvedCanonicalTag>, SystemBuildError> {
    let mut resolved = Vec::new();
    let mut claimed_channels = BTreeSet::new();
    for tag in software.tags() {
        let expected_type = program_value_type(&tag.data_type).ok_or_else(|| {
            SystemBuildError::ProbeCatalog(format!(
                "tag '{}' uses a type unavailable in the runtime value model",
                tag.display_name
            ))
        })?;
        let block = BlockId::new(object_u128(tag.target.block_object_id));
        let Some(origin) = software.program().block(block) else {
            return Err(SystemBuildError::ProbeCatalog(format!(
                "tag '{}' references an unknown canonical block",
                tag.display_name
            )));
        };
        let Some(member) = origin.interface.members.get(&tag.target.member_id) else {
            return Err(SystemBuildError::ProbeCatalog(format!(
                "tag '{}' references an unknown canonical member",
                tag.display_name
            )));
        };
        if program_value_type(&member.data_type) != Some(expected_type) {
            return Err(SystemBuildError::ProbeCatalog(format!(
                "tag '{}' type disagrees with its program member",
                tag.display_name
            )));
        }
        let memory = runtime_projection
            .memory_for(block, tag.target.member_id)
            .ok_or_else(|| {
                SystemBuildError::ProbeCatalog(format!(
                    "tag '{}' member has no authoritative runtime binding",
                    tag.display_name
                ))
            })?;
        let runtime_block = runtime_projection.block_for(block).ok_or_else(|| {
            SystemBuildError::ProbeCatalog(format!(
                "tag '{}' block has no authoritative runtime schedule binding",
                tag.display_name
            ))
        })?;
        let runtime_target = if let Some(hardware) = tag.target.hardware {
            let wanted_direction = match hardware.area {
                CanonicalAddressArea::Input => HardwareDirection::Input,
                CanonicalAddressArea::Output => HardwareDirection::Output,
                CanonicalAddressArea::Memory => {
                    return Err(SystemBuildError::ProbeCatalog(format!(
                        "tag '{}' has an invalid hardware Memory-area target",
                        tag.display_name
                    )));
                }
            };
            let compatible_role = match hardware.area {
                CanonicalAddressArea::Input => matches!(
                    member.role,
                    InterfaceRole::Input | InterfaceRole::InOut | InterfaceRole::Temp
                ),
                CanonicalAddressArea::Output => matches!(
                    member.role,
                    InterfaceRole::Output | InterfaceRole::InOut | InterfaceRole::Temp
                ),
                CanonicalAddressArea::Memory => unreachable!("rejected above"),
            };
            if !compatible_role {
                return Err(SystemBuildError::ProbeCatalog(format!(
                    "tag '{}' binds address area {:?} to incompatible interface role {:?}",
                    tag.display_name, hardware.area, member.role
                )));
            }
            let binding = channel_bindings.iter().find(|binding| {
                binding.hardware.direction == wanted_direction
                    && hardware_value_type(binding.hardware.raw_type) == Some(expected_type)
                    && !claimed_channels.contains(&binding.hardware_channel_id)
                    && address_matches(binding.hardware.address, hardware.intent)
            });
            let Some(binding) = binding else {
                return Err(SystemBuildError::ProbeCatalog(format!(
                    "tag '{}' cannot resolve to a compatible allocated hardware channel",
                    tag.display_name
                )));
            };
            claimed_channels.insert(binding.hardware_channel_id);
            match hardware.area {
                CanonicalAddressArea::Input => RuntimeTarget::Input(binding.runtime_channel_id),
                CanonicalAddressArea::Output => RuntimeTarget::Output(binding.runtime_channel_id),
                CanonicalAddressArea::Memory => unreachable!("handled above"),
            }
        } else {
            RuntimeTarget::Memory(memory)
        };
        resolved.push(ResolvedCanonicalTag {
            tag: tag.clone(),
            program_block: block,
            runtime_block,
            program_memory: memory,
            runtime_target,
            value_type: expected_type,
        });
    }
    Ok(resolved)
}

fn inject_io_binding_operations(
    program: &mut ProgramImage,
    tags: &[ResolvedCanonicalTag],
) -> Result<(), SystemBuildError> {
    let mut by_block: BTreeMap<BlockId, Vec<&ResolvedCanonicalTag>> = BTreeMap::new();
    for tag in tags
        .iter()
        .filter(|tag| !matches!(tag.runtime_target, RuntimeTarget::Memory(_)))
    {
        by_block.entry(tag.program_block).or_default().push(tag);
    }
    for (owner, mut bindings) in by_block {
        bindings.sort_by_key(|binding| binding.tag.stable_identity);
        let runtime_block_id = bindings
            .first()
            .map(|binding| binding.runtime_block)
            .ok_or_else(|| {
                SystemBuildError::RuntimeArtifact(format!(
                    "I/O-bound canonical block {owner:?} has no runtime schedule binding"
                ))
            })?;
        let block = scheduled_block_mut(program, runtime_block_id).ok_or_else(|| {
            SystemBuildError::RuntimeArtifact(format!(
                "runtime block {runtime_block_id:?} is absent from the program image"
            ))
        })?;
        let mut next_operation_id = block
            .instructions
            .iter()
            .map(|instruction| instruction.operation_id)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| {
                SystemBuildError::RuntimeArtifact("runtime operation identity exhausted".to_owned())
            })?;
        let mut prefix = Vec::new();
        let mut suffix = Vec::new();
        for binding in bindings {
            let operation = match binding.runtime_target {
                RuntimeTarget::Input(channel) => Operation::LoadInput {
                    channel,
                    target: binding.program_memory,
                },
                RuntimeTarget::Output(channel) => Operation::StoreOutput {
                    source: Operand::Memory(binding.program_memory),
                    channel,
                },
                RuntimeTarget::Memory(_) => continue,
            };
            let instruction =
                Instruction::new(next_operation_id, binding.tag.stable_identity, operation);
            next_operation_id = next_operation_id.checked_add(1).ok_or_else(|| {
                SystemBuildError::RuntimeArtifact("runtime operation identity exhausted".to_owned())
            })?;
            match binding.runtime_target {
                RuntimeTarget::Input(_) => prefix.push(instruction),
                RuntimeTarget::Output(_) => suffix.push(instruction),
                RuntimeTarget::Memory(_) => {}
            }
        }
        inject_control_flow_safe_io(block, prefix, suffix, next_operation_id)?;
    }
    Ok(())
}

fn inject_control_flow_safe_io(
    block: &mut RuntimeProgramBlock,
    mut prefix: Vec<Instruction>,
    suffix: Vec<Instruction>,
    final_return_id: u32,
) -> Result<(), SystemBuildError> {
    let prefix_len = u32::try_from(prefix.len()).map_err(|_| {
        SystemBuildError::RuntimeArtifact("runtime I/O prefix is too large".to_owned())
    })?;
    let body_len = u32::try_from(block.instructions.len()).map_err(|_| {
        SystemBuildError::RuntimeArtifact("runtime control-flow body is too large".to_owned())
    })?;
    let epilogue_start = prefix_len.checked_add(body_len).ok_or_else(|| {
        SystemBuildError::RuntimeArtifact("runtime control-flow identity exhausted".to_owned())
    })?;
    let has_epilogue = !suffix.is_empty();
    let mut body = Vec::with_capacity(block.instructions.len());
    for instruction in core::mem::take(&mut block.instructions) {
        let operation = match instruction.operation() {
            Operation::Jump { target } => Operation::Jump {
                target: shifted_instruction_target(*target, prefix_len)?,
            },
            Operation::Branch {
                condition,
                when_true,
                when_false,
            } => Operation::Branch {
                condition: *condition,
                when_true: shifted_instruction_target(*when_true, prefix_len)?,
                when_false: shifted_instruction_target(*when_false, prefix_len)?,
            },
            Operation::Return if has_epilogue => Operation::Jump {
                target: epilogue_start,
            },
            operation => operation.clone(),
        };
        body.push(Instruction::new(
            instruction.operation_id,
            instruction.source_identity,
            operation,
        ));
    }
    prefix.append(&mut body);
    prefix.extend(suffix);
    if has_epilogue {
        prefix.push(Instruction::new(final_return_id, 0, Operation::Return));
    }
    block.instructions = prefix;
    Ok(())
}

fn shifted_instruction_target(target: u32, prefix_len: u32) -> Result<u32, SystemBuildError> {
    target.checked_add(prefix_len).ok_or_else(|| {
        SystemBuildError::RuntimeArtifact("runtime control-flow target exhausted".to_owned())
    })
}

fn scheduled_block_mut(
    program: &mut ProgramImage,
    id: plc_runtime::BlockId,
) -> Option<&mut RuntimeProgramBlock> {
    if program.cyclic.id == id {
        return Some(&mut program.cyclic);
    }
    if program.startup.as_ref().is_some_and(|block| block.id == id) {
        return program.startup.as_mut();
    }
    program
        .timed
        .iter_mut()
        .find(|task| task.block.id == id)
        .map(|task| &mut task.block)
}

const fn address_matches(address: ChannelAddress, intent: CanonicalAddressIntent) -> bool {
    match intent {
        CanonicalAddressIntent::Auto => true,
        CanonicalAddressIntent::Explicit {
            byte_offset,
            bit_offset,
        } => match address {
            ChannelAddress::Bit { byte, bit, .. } => byte == byte_offset && bit == bit_offset,
            ChannelAddress::Word { byte, .. } => byte == byte_offset && bit_offset == 0,
        },
    }
}

fn build_load_package(
    project: &Project,
    artifact: &SystemCompilerArtifact,
    runtime_projection: &RuntimeArtifactProjection,
    runtime_artifact: &ArtifactPackage,
    hardware_fingerprint: Sha256Digest,
    probe_catalog_hash: Hash32,
) -> Result<VirtualLoadPackage, SystemBuildError> {
    let memory_by_id = runtime_artifact
        .spec()
        .memory
        .iter()
        .map(|definition| (definition.id, definition))
        .collect::<BTreeMap<_, _>>();
    let mut memory_schema = Vec::new();
    let mut projected_runtime_memory = BTreeSet::new();
    let mut projected_member_ids = BTreeSet::new();
    for binding in runtime_projection.memory_bindings() {
        let definition = memory_by_id
            .get(&binding.memory)
            .expect("runtime projection binding must name sealed memory");
        let member = runtime_projection
            .memory_bindings()
            .iter()
            .find(|candidate| candidate.memory == binding.memory)
            .expect("current binding is present");
        let member_id = member.member.get();
        if !projected_member_ids.insert(member_id) {
            return Err(SystemBuildError::LoadPackage(format!(
                "canonical member identity {member_id:032x} is duplicated across runtime memory"
            )));
        }
        projected_runtime_memory.insert(binding.memory);
        memory_schema.push(MemoryMemberSchema {
            member_id,
            runtime_memory_id: binding.memory,
            value_type: binding.value_type,
            role: MemoryRole::Marker,
            instance_path: vec![binding.owner.get()],
            retentive: definition.retentive,
            loaded_start: definition.loaded_start,
        });
    }
    // The compiler materializes typed SSA results as sealed runtime memory.
    // They are not authoring members, but commissioning still requires every
    // runtime cell to have a stable load-schema identity. Keep those identities
    // in an explicit compiler-temporary domain derived from the artifact and
    // runtime cell, and fail closed on the (vanishingly unlikely) collision
    // with a canonical project member.
    for definition in &runtime_artifact.spec().memory {
        if projected_runtime_memory.contains(&definition.id) {
            continue;
        }
        let member_id = compiler_temporary_member_id(runtime_artifact.fingerprint(), definition.id);
        if !projected_member_ids.insert(member_id) {
            return Err(SystemBuildError::LoadPackage(format!(
                "compiler-temporary identity collision for runtime memory {:?}",
                definition.id
            )));
        }
        memory_schema.push(MemoryMemberSchema {
            member_id,
            runtime_memory_id: definition.id,
            value_type: definition.value_type,
            role: MemoryRole::Marker,
            instance_path: Vec::new(),
            retentive: definition.retentive,
            loaded_start: definition.loaded_start,
        });
    }
    memory_schema.sort_by_key(|member| member.runtime_memory_id);
    let verified_ir_hash = artifact.composed.verified_ir().verification_hash();
    let source_map_hash = artifact.composed.source_maps().fingerprint();
    let build_snapshot_hash = combined_build_hash(
        project.semantic_fingerprint(),
        verified_ir_hash,
        hardware_fingerprint,
    );
    let state_schema = runtime_artifact
        .spec()
        .states
        .iter()
        .map(|definition| StateMemberSchema {
            state_member_id: u128::from(definition.id.get()),
            runtime_state_id: definition.id,
            kind: match definition.loaded_start {
                StateStart::Edge { .. } => LoadStateKind::Edge,
                StateStart::Timer { .. } => LoadStateKind::Timer,
                StateStart::Counter { .. } => LoadStateKind::Counter,
            },
            owner_member_id: 0,
            instance_path: Vec::new(),
            retentive: definition.retentive,
        })
        .collect();
    VirtualLoadPackage::seal_verified(LoadPackageParts {
        runtime_artifact: runtime_artifact.clone(),
        semantic_build_fingerprint: core_hash(project.semantic_fingerprint()),
        verified_ir_fingerprint: verified_ir_hash,
        schedule_fingerprint: runtime_artifact.fingerprint(),
        hardware_fingerprint: core_hash(hardware_fingerprint),
        source_map_fingerprint: source_map_hash,
        probe_identity_fingerprint: probe_catalog_hash,
        capability_fingerprint: artifact.capability_manifest_hash,
        build_snapshot_hash,
        build_is_current: true,
        blocking_diagnostic_count: 0,
        memory_schema,
        state_schema,
    })
    .map_err(|error| SystemBuildError::LoadPackage(format!("{error:?}")))
}

fn compiler_temporary_member_id(artifact: Hash32, memory: MemoryId) -> u128 {
    let mut bytes = Vec::with_capacity(32 + 4 + 24);
    bytes.extend_from_slice(b"PES-COMPILER-TEMP-MEM-1\0");
    bytes.extend_from_slice(artifact.as_bytes());
    bytes.extend_from_slice(&memory.get().to_be_bytes());
    let digest = sha256(&bytes);
    let mut identity = [0_u8; 16];
    identity.copy_from_slice(&digest.0[..16]);
    u128::from_be_bytes(identity)
}

fn combined_build_hash(project: Sha256Digest, compiler: Hash32, hardware: Sha256Digest) -> Hash32 {
    let mut bytes = Vec::with_capacity(32 * 3 + 22);
    bytes.extend_from_slice(b"PES-SYSTEM-BUILD-1\0");
    bytes.extend_from_slice(&project.0);
    bytes.extend_from_slice(compiler.as_bytes());
    bytes.extend_from_slice(&hardware.0);
    core_hash(sha256(&bytes))
}

const fn core_hash(value: Sha256Digest) -> Hash32 {
    Hash32::from_bytes(value.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_injection_shifts_cfg_targets_and_routes_returns_through_output_epilogue() {
        let condition = MemoryId::new(1);
        let output = MemoryId::new(2);
        let mut block = RuntimeProgramBlock {
            id: plc_runtime::BlockId(7),
            instructions: vec![
                Instruction::new(1, 101, Operation::Jump { target: 2 }),
                Instruction::new(
                    2,
                    102,
                    Operation::Branch {
                        condition: Operand::Memory(condition),
                        when_true: 0,
                        when_false: 2,
                    },
                ),
                Instruction::new(3, 103, Operation::Return),
            ],
        };
        let prefix = vec![Instruction::new(
            4,
            104,
            Operation::LoadInput {
                channel: ChannelId::new(1),
                target: condition,
            },
        )];
        let suffix = vec![Instruction::new(
            5,
            105,
            Operation::StoreOutput {
                source: Operand::Memory(output),
                channel: ChannelId::new(2),
            },
        )];

        inject_control_flow_safe_io(&mut block, prefix, suffix, 6).expect("bounded injection");

        assert!(matches!(
            block.instructions[1].operation(),
            Operation::Jump { target: 3 }
        ));
        assert!(matches!(
            block.instructions[2].operation(),
            Operation::Branch {
                when_true: 1,
                when_false: 3,
                ..
            }
        ));
        assert!(matches!(
            block.instructions[3].operation(),
            Operation::Jump { target: 4 }
        ));
        assert!(matches!(
            block.instructions[4].operation(),
            Operation::StoreOutput { .. }
        ));
        assert!(matches!(
            block.instructions[5].operation(),
            Operation::Return
        ));
    }
}
