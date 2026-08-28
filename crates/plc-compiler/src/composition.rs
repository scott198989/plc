use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};

use plc_program::{
    BlockId, ControllerProgram, DataType, InterfaceMemberId, ProgramIssue, validate_program,
};

use crate::{
    Hash32, IrActivation, IrBasicBlock, IrBasicBlockId, IrBoundInput, IrFunction, IrOperation,
    IrOperationId, IrOperationKind, IrTerminator, IrTerminatorKind, IrType, IrValue, IrValueId,
    ProbeDefinition, ProbeId, ProbeTable, ResourceLimit, ResourceLimits, SclSource, SourceLanguage,
    SourceMapEntry, SourceMapId, SourceMapSite, SourceMapTable, TYPED_IR_VERSION, TypedIrProgram,
    VerificationError, VerifiedIr,
    lowering::{LoweringError, lower_typed_blocks},
    scl::{SclIssue, bind_and_typecheck_with_program, parse_scl},
    verify_typed_ir,
};

/// One independently verified, single-block frontend result.
///
/// This compiler-owned DTO avoids dependencies from `plc-compiler` back to
/// language frontends. SCL, LAD, and FBD adapters can all project their shared
/// [`VerifiedIr`], [`SourceMapTable`], and [`ProbeTable`] values into it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrontendArtifact {
    owner: BlockId,
    language: SourceLanguage,
    verified_ir: VerifiedIr,
    source_maps: SourceMapTable,
    probes: ProbeTable,
}

impl FrontendArtifact {
    #[must_use]
    pub const fn new(
        owner: BlockId,
        language: SourceLanguage,
        verified_ir: VerifiedIr,
        source_maps: SourceMapTable,
        probes: ProbeTable,
    ) -> Self {
        Self {
            owner,
            language,
            verified_ir,
            source_maps,
            probes,
        }
    }

    #[must_use]
    pub const fn owner(&self) -> BlockId {
        self.owner
    }

    #[must_use]
    pub const fn language(&self) -> SourceLanguage {
        self.language
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
    pub const fn probes(&self) -> &ProbeTable {
        &self.probes
    }
}

/// A single, re-verified whole-controller IR assembled from language-native
/// frontend artifacts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposedFrontendArtifact {
    verified_ir: VerifiedIr,
    source_maps: SourceMapTable,
    probes: ProbeTable,
    owner_languages: BTreeMap<BlockId, SourceLanguage>,
}

impl ComposedFrontendArtifact {
    #[must_use]
    pub const fn verified_ir(&self) -> &VerifiedIr {
        &self.verified_ir
    }

    #[must_use]
    pub const fn source_maps(&self) -> &SourceMapTable {
        &self.source_maps
    }

    #[must_use]
    pub const fn probes(&self) -> &ProbeTable {
        &self.probes
    }

    #[must_use]
    pub const fn owner_languages(&self) -> &BTreeMap<BlockId, SourceLanguage> {
        &self.owner_languages
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompositionIdentityKind {
    BasicBlock,
    Operation,
    Value,
    SourceMap,
    Probe,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompositionError {
    UnknownOwner(BlockId),
    NonExecutableOwner(BlockId),
    DuplicateOwner(BlockId),
    MissingOwner(BlockId),
    ArtifactFunctionCount {
        owner: BlockId,
        count: usize,
    },
    ArtifactOwnerMismatch {
        declared: BlockId,
        actual: BlockId,
    },
    SourceLanguageMismatch {
        owner: BlockId,
        source_map: SourceMapId,
        expected: SourceLanguage,
        actual: SourceLanguage,
    },
    InputVerification {
        owner: BlockId,
        error: VerificationError,
    },
    InputVerificationHashMismatch {
        owner: BlockId,
        supplied: Hash32,
        reverified: Hash32,
    },
    IdentitySpaceExhausted(CompositionIdentityKind),
    MissingIdentityRemap {
        owner: BlockId,
        kind: CompositionIdentityKind,
        original: u32,
    },
    ComposedVerification(VerificationError),
}

/// Public, stable categories for invariant failures from the existing SCL to
/// shared-IR lowering stage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SclLoweringFailure {
    UnsupportedType(DataType),
    ErrorNode,
    IdentitySpaceExhausted,
    MissingBasicBlock(IrBasicBlockId),
    DuplicateTerminator(IrBasicBlockId),
}

impl SclLoweringFailure {
    fn from_internal(value: LoweringError) -> Self {
        match value {
            LoweringError::UnsupportedType(data_type) => Self::UnsupportedType(data_type),
            LoweringError::ErrorNode => Self::ErrorNode,
            LoweringError::IdentityOverflow => Self::IdentitySpaceExhausted,
            LoweringError::MissingBlock(block) => Self::MissingBasicBlock(block),
            LoweringError::DuplicateTerminator(block) => Self::DuplicateTerminator(block),
        }
    }
}

/// Typed failure returned by [`lower_scl_frontend_artifact`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SclFrontendError {
    UnknownOwner(BlockId),
    OwnerMismatch {
        source: BlockId,
        block: BlockId,
    },
    NonExecutableOwner(BlockId),
    InvalidProgram(Vec<ProgramIssue>),
    CanonicalBodyConflict(BlockId),
    UnsupportedInterfaceType {
        owner: BlockId,
        member: InterfaceMemberId,
        data_type: DataType,
    },
    ResourceLimit(ResourceLimit),
    Diagnostics(Vec<SclIssue>),
    Lowering(SclLoweringFailure),
    Verification(VerificationError),
}

/// Runs the production SCL lexer, recovery parser, contextual binder/type
/// checker, shared typed-IR lowering, and independent verifier for exactly one
/// canonical block.
///
/// The full [`ControllerProgram`] is supplied to semantic analysis and IR
/// verification, so FC calls resolve against canonical block/interface
/// identities. No AST or language-specific executable form escapes this API.
///
/// # Errors
///
/// Returns typed canonical-program, authored-diagnostic, resource, lowering,
/// or verification failures. Invalid source never yields a partial artifact.
pub fn lower_scl_frontend_artifact(
    program: &ControllerProgram,
    source: &SclSource,
    limits: ResourceLimits,
) -> Result<FrontendArtifact, SclFrontendError> {
    let owner = source.owner();
    let block = program
        .block(owner)
        .ok_or(SclFrontendError::UnknownOwner(owner))?;
    if block.id != owner {
        return Err(SclFrontendError::OwnerMismatch {
            source: owner,
            block: block.id,
        });
    }
    if !block.kind.is_executable() {
        return Err(SclFrontendError::NonExecutableOwner(owner));
    }

    let validation = validate_program(program);
    if !validation.is_valid() {
        return Err(SclFrontendError::InvalidProgram(validation.issues));
    }
    if !block.instructions.is_empty() || !block.calls.is_empty() {
        return Err(SclFrontendError::CanonicalBodyConflict(owner));
    }

    let syntax = parse_scl(source, limits);
    if let Some(limit) = syntax.resource_limit() {
        return Err(SclFrontendError::ResourceLimit(limit.clone()));
    }
    if !syntax.issues().is_empty() {
        return Err(SclFrontendError::Diagnostics(syntax.issues().to_vec()));
    }
    for member in block.interface.members.values() {
        if IrType::from_program_type(&member.data_type).is_none() {
            return Err(SclFrontendError::UnsupportedInterfaceType {
                owner,
                member: member.id,
                data_type: member.data_type.clone(),
            });
        }
    }

    let (typed, issues) = bind_and_typecheck_with_program(&syntax, block, program);
    if !issues.is_empty() {
        if issues.len() > limits.max_diagnostics {
            return Err(SclFrontendError::ResourceLimit(ResourceLimit {
                key: "compiler.diagnostics",
                current: u64::try_from(issues.len()).unwrap_or(u64::MAX),
                maximum: u64::try_from(limits.max_diagnostics).unwrap_or(u64::MAX),
            }));
        }
        return Err(SclFrontendError::Diagnostics(issues));
    }
    let lowered = lower_typed_blocks(&[(typed, source.clone())])
        .map_err(|error| SclFrontendError::Lowering(SclLoweringFailure::from_internal(error)))?;
    if lowered.operation_count > limits.max_ir_operations {
        return Err(SclFrontendError::ResourceLimit(ResourceLimit {
            key: "compiler.ir_operations",
            current: u64::try_from(lowered.operation_count).unwrap_or(u64::MAX),
            maximum: u64::try_from(limits.max_ir_operations).unwrap_or(u64::MAX),
        }));
    }
    let verified_ir = verify_typed_ir(lowered.ir, &lowered.source_maps, &lowered.probes, program)
        .map_err(SclFrontendError::Verification)?;
    Ok(FrontendArtifact::new(
        owner,
        SourceLanguage::Scl,
        verified_ir,
        lowered.source_maps,
        lowered.probes,
    ))
}

/// Deterministically composes one verified frontend artifact for every
/// executable controller block into a single shared typed IR.
///
/// Artifacts are ordered by canonical [`BlockId`], not caller order. Numeric
/// IR, source-map, and probe identities are rekeyed into collision-free global
/// namespaces. Canonical block/member identities and every native
/// [`crate::SourceAnchor`] are preserved verbatim.
///
/// # Errors
///
/// Returns a deterministic owner, language, verification, identity-space, or
/// merged-verification defect. Missing executable owners fail closed.
pub fn compose_frontend_artifacts(
    program: &ControllerProgram,
    artifacts: &[FrontendArtifact],
) -> Result<ComposedFrontendArtifact, CompositionError> {
    let ordered = validate_and_order_inputs(program, artifacts)?;
    let mut allocators = IdentityAllocators::default();
    let mut functions = BTreeMap::new();
    let mut source_maps = BTreeMap::new();
    let mut probes = BTreeMap::new();
    let mut owner_languages = BTreeMap::new();

    for artifact in ordered {
        let function = artifact
            .verified_ir
            .program()
            .functions()
            .get(&artifact.owner)
            .ok_or(CompositionError::ArtifactFunctionCount {
                owner: artifact.owner,
                count: artifact.verified_ir.program().functions().len(),
            })?;
        let remap = ArtifactRemap::allocate(function, artifact, &mut allocators)?;
        functions.insert(artifact.owner, remap_function(function, &remap)?);
        remap_source_maps(artifact, &remap, &mut source_maps)?;
        remap_probes(artifact, &remap, &mut probes)?;
        owner_languages.insert(artifact.owner, artifact.language);
    }

    let source_maps = SourceMapTable::from_untrusted_entries(source_maps);
    let probes = ProbeTable::from_untrusted_entries(probes);
    let ir = TypedIrProgram::from_untrusted_parts(TYPED_IR_VERSION, functions);
    let verified_ir = verify_typed_ir(ir, &source_maps, &probes, program)
        .map_err(CompositionError::ComposedVerification)?;
    Ok(ComposedFrontendArtifact {
        verified_ir,
        source_maps,
        probes,
        owner_languages,
    })
}

fn validate_and_order_inputs<'a>(
    program: &ControllerProgram,
    artifacts: &'a [FrontendArtifact],
) -> Result<Vec<&'a FrontendArtifact>, CompositionError> {
    let mut ordered: Vec<_> = artifacts.iter().collect();
    ordered.sort_by_key(|artifact| artifact.owner);

    let mut previous_owner = None;
    for artifact in &ordered {
        if previous_owner == Some(artifact.owner) {
            return Err(CompositionError::DuplicateOwner(artifact.owner));
        }
        previous_owner = Some(artifact.owner);
        let owner = program
            .block(artifact.owner)
            .ok_or(CompositionError::UnknownOwner(artifact.owner))?;
        if !owner.kind.is_executable() {
            return Err(CompositionError::NonExecutableOwner(artifact.owner));
        }
        validate_artifact(program, artifact)?;
    }

    let supplied: BTreeSet<_> = ordered.iter().map(|artifact| artifact.owner).collect();
    for (&owner, block) in program.blocks() {
        if block.kind.is_executable() && !supplied.contains(&owner) {
            return Err(CompositionError::MissingOwner(owner));
        }
    }
    Ok(ordered)
}

fn validate_artifact(
    program: &ControllerProgram,
    artifact: &FrontendArtifact,
) -> Result<(), CompositionError> {
    let functions = artifact.verified_ir.program().functions();
    if functions.len() != 1 {
        return Err(CompositionError::ArtifactFunctionCount {
            owner: artifact.owner,
            count: functions.len(),
        });
    }
    let actual = *functions
        .keys()
        .next()
        .ok_or(CompositionError::ArtifactFunctionCount {
            owner: artifact.owner,
            count: 0,
        })?;
    if actual != artifact.owner {
        return Err(CompositionError::ArtifactOwnerMismatch {
            declared: artifact.owner,
            actual,
        });
    }
    validate_anchor_languages(artifact)?;

    let reverified = verify_typed_ir(
        artifact.verified_ir.program().clone(),
        &artifact.source_maps,
        &artifact.probes,
        program,
    )
    .map_err(|error| CompositionError::InputVerification {
        owner: artifact.owner,
        error,
    })?;
    if reverified.verification_hash() != artifact.verified_ir.verification_hash() {
        return Err(CompositionError::InputVerificationHashMismatch {
            owner: artifact.owner,
            supplied: artifact.verified_ir.verification_hash(),
            reverified: reverified.verification_hash(),
        });
    }
    Ok(())
}

fn validate_anchor_languages(artifact: &FrontendArtifact) -> Result<(), CompositionError> {
    for (&source_map, entry) in artifact.source_maps.entries() {
        for anchor in &entry.anchors {
            if anchor.language != artifact.language {
                return Err(CompositionError::SourceLanguageMismatch {
                    owner: artifact.owner,
                    source_map,
                    expected: artifact.language,
                    actual: anchor.language,
                });
            }
        }
    }
    Ok(())
}

#[derive(Default)]
struct IdentityAllocators {
    basic_blocks: IdentityAllocator,
    operations: IdentityAllocator,
    values: IdentityAllocator,
    source_maps: IdentityAllocator,
    probes: IdentityAllocator,
}

#[derive(Default)]
struct IdentityAllocator {
    allocated: u64,
}

impl IdentityAllocator {
    fn next(&mut self, kind: CompositionIdentityKind) -> Result<u32, CompositionError> {
        self.allocated = self
            .allocated
            .checked_add(1)
            .ok_or(CompositionError::IdentitySpaceExhausted(kind))?;
        u32::try_from(self.allocated).map_err(|_| CompositionError::IdentitySpaceExhausted(kind))
    }
}

struct ArtifactRemap {
    owner: BlockId,
    basic_blocks: BTreeMap<IrBasicBlockId, IrBasicBlockId>,
    operations: BTreeMap<IrOperationId, IrOperationId>,
    values: BTreeMap<IrValueId, IrValueId>,
    source_maps: BTreeMap<SourceMapId, SourceMapId>,
    probes: BTreeMap<ProbeId, ProbeId>,
}

impl ArtifactRemap {
    fn allocate(
        function: &IrFunction,
        artifact: &FrontendArtifact,
        allocators: &mut IdentityAllocators,
    ) -> Result<Self, CompositionError> {
        let mut basic_blocks = BTreeMap::new();
        for &id in function.blocks.keys() {
            basic_blocks.insert(
                id,
                IrBasicBlockId::new(
                    allocators
                        .basic_blocks
                        .next(CompositionIdentityKind::BasicBlock)?,
                ),
            );
        }

        let mut operation_ids = BTreeSet::new();
        let mut value_ids = BTreeSet::new();
        for block in function.blocks.values() {
            for operation in &block.operations {
                operation_ids.insert(operation.id);
                if let Some(result) = &operation.result {
                    value_ids.insert(result.id);
                }
            }
        }
        let mut operations = BTreeMap::new();
        for id in operation_ids {
            operations.insert(
                id,
                IrOperationId::new(
                    allocators
                        .operations
                        .next(CompositionIdentityKind::Operation)?,
                ),
            );
        }
        let mut values = BTreeMap::new();
        for id in value_ids {
            values.insert(
                id,
                IrValueId::new(allocators.values.next(CompositionIdentityKind::Value)?),
            );
        }
        let mut source_maps = BTreeMap::new();
        for &id in artifact.source_maps.entries().keys() {
            source_maps.insert(
                id,
                SourceMapId::new(
                    allocators
                        .source_maps
                        .next(CompositionIdentityKind::SourceMap)?,
                ),
            );
        }
        let mut probes = BTreeMap::new();
        for &id in artifact.probes.entries().keys() {
            probes.insert(
                id,
                ProbeId::new(allocators.probes.next(CompositionIdentityKind::Probe)?),
            );
        }
        Ok(Self {
            owner: artifact.owner,
            basic_blocks,
            operations,
            values,
            source_maps,
            probes,
        })
    }

    fn basic_block(&self, id: IrBasicBlockId) -> Result<IrBasicBlockId, CompositionError> {
        self.basic_blocks
            .get(&id)
            .copied()
            .ok_or(CompositionError::MissingIdentityRemap {
                owner: self.owner,
                kind: CompositionIdentityKind::BasicBlock,
                original: id.get(),
            })
    }

    fn operation(&self, id: IrOperationId) -> Result<IrOperationId, CompositionError> {
        self.operations
            .get(&id)
            .copied()
            .ok_or(CompositionError::MissingIdentityRemap {
                owner: self.owner,
                kind: CompositionIdentityKind::Operation,
                original: id.get(),
            })
    }

    fn value(&self, id: IrValueId) -> Result<IrValueId, CompositionError> {
        self.values
            .get(&id)
            .copied()
            .ok_or(CompositionError::MissingIdentityRemap {
                owner: self.owner,
                kind: CompositionIdentityKind::Value,
                original: id.get(),
            })
    }

    fn source_map(&self, id: SourceMapId) -> Result<SourceMapId, CompositionError> {
        self.source_maps
            .get(&id)
            .copied()
            .ok_or(CompositionError::MissingIdentityRemap {
                owner: self.owner,
                kind: CompositionIdentityKind::SourceMap,
                original: id.get(),
            })
    }

    fn probe(&self, id: ProbeId) -> Result<ProbeId, CompositionError> {
        self.probes
            .get(&id)
            .copied()
            .ok_or(CompositionError::MissingIdentityRemap {
                owner: self.owner,
                kind: CompositionIdentityKind::Probe,
                original: id.get(),
            })
    }
}

fn remap_function(
    function: &IrFunction,
    remap: &ArtifactRemap,
) -> Result<IrFunction, CompositionError> {
    let mut blocks = BTreeMap::new();
    for block in function.blocks.values() {
        let remapped = remap_basic_block(block, remap)?;
        blocks.insert(remapped.id, remapped);
    }
    Ok(IrFunction {
        owner: function.owner,
        source_kind: function.source_kind,
        entry: remap.basic_block(function.entry)?,
        blocks,
    })
}

fn remap_basic_block(
    block: &IrBasicBlock,
    remap: &ArtifactRemap,
) -> Result<IrBasicBlock, CompositionError> {
    let operations = block
        .operations
        .iter()
        .map(|operation| remap_operation(operation, remap))
        .collect::<Result<_, _>>()?;
    Ok(IrBasicBlock {
        id: remap.basic_block(block.id)?,
        operations,
        terminator: remap_terminator(&block.terminator, remap)?,
    })
}

fn remap_operation(
    operation: &IrOperation,
    remap: &ArtifactRemap,
) -> Result<IrOperation, CompositionError> {
    Ok(IrOperation {
        id: remap.operation(operation.id)?,
        result: operation
            .result
            .as_ref()
            .map(|result| {
                Ok(IrValue {
                    id: remap.value(result.id)?,
                    data_type: result.data_type.clone(),
                })
            })
            .transpose()?,
        kind: remap_operation_kind(&operation.kind, remap)?,
        source_map: remap.source_map(operation.source_map)?,
        probe: remap.probe(operation.probe)?,
    })
}

fn remap_operation_kind(
    kind: &IrOperationKind,
    remap: &ArtifactRemap,
) -> Result<IrOperationKind, CompositionError> {
    Ok(match kind {
        IrOperationKind::Constant(value) => IrOperationKind::Constant(value.clone()),
        IrOperationKind::LoadMember { member } => IrOperationKind::LoadMember { member: *member },
        IrOperationKind::StoreMember { target, value } => IrOperationKind::StoreMember {
            target: *target,
            value: remap.value(*value)?,
        },
        IrOperationKind::Unary { operator, operand } => IrOperationKind::Unary {
            operator: *operator,
            operand: remap.value(*operand)?,
        },
        IrOperationKind::Binary {
            operator,
            left,
            right,
        } => IrOperationKind::Binary {
            operator: *operator,
            left: remap.value(*left)?,
            right: remap.value(*right)?,
        },
        IrOperationKind::ForNextWithin {
            current,
            terminal,
            step,
            ascending,
        } => IrOperationKind::ForNextWithin {
            current: remap.value(*current)?,
            terminal: remap.value(*terminal)?,
            step: remap.value(*step)?,
            ascending: *ascending,
        },
        IrOperationKind::Convert {
            source,
            destination,
        } => IrOperationKind::Convert {
            source: remap.value(*source)?,
            destination: destination.clone(),
        },
        IrOperationKind::InvokeInstruction {
            instruction,
            inputs,
            outputs,
            instance,
            activation,
        } => IrOperationKind::InvokeInstruction {
            instruction: *instruction,
            inputs: remap_inputs(inputs, remap)?,
            outputs: outputs.clone(),
            instance: instance.clone(),
            activation: remap_activation(*activation, remap)?,
        },
        IrOperationKind::CallBlock {
            call_instruction,
            target,
            inputs,
            outputs,
            instance,
            activation,
        } => IrOperationKind::CallBlock {
            call_instruction: *call_instruction,
            target: *target,
            inputs: remap_inputs(inputs, remap)?,
            outputs: outputs.clone(),
            instance: instance.clone(),
            activation: remap_activation(*activation, remap)?,
        },
        IrOperationKind::InvocationOutput { invocation, formal } => {
            IrOperationKind::InvocationOutput {
                invocation: remap.operation(*invocation)?,
                formal: *formal,
            }
        }
    })
}

fn remap_inputs(
    inputs: &[IrBoundInput],
    remap: &ArtifactRemap,
) -> Result<Vec<IrBoundInput>, CompositionError> {
    inputs
        .iter()
        .map(|input| {
            Ok(IrBoundInput {
                formal: input.formal,
                value: remap.value(input.value)?,
            })
        })
        .collect()
}

fn remap_activation(
    activation: Option<IrActivation>,
    remap: &ArtifactRemap,
) -> Result<Option<IrActivation>, CompositionError> {
    activation
        .map(|activation| {
            Ok(IrActivation {
                enable: remap.value(activation.enable)?,
                ..activation
            })
        })
        .transpose()
}

fn remap_terminator(
    terminator: &IrTerminator,
    remap: &ArtifactRemap,
) -> Result<IrTerminator, CompositionError> {
    let kind = match terminator.kind {
        IrTerminatorKind::Jump(target) => IrTerminatorKind::Jump(remap.basic_block(target)?),
        IrTerminatorKind::Branch {
            condition,
            when_true,
            when_false,
        } => IrTerminatorKind::Branch {
            condition: remap.value(condition)?,
            when_true: remap.basic_block(when_true)?,
            when_false: remap.basic_block(when_false)?,
        },
        IrTerminatorKind::Return => IrTerminatorKind::Return,
    };
    Ok(IrTerminator {
        kind,
        source_map: remap.source_map(terminator.source_map)?,
        probe: remap.probe(terminator.probe)?,
    })
}

fn remap_source_maps(
    artifact: &FrontendArtifact,
    remap: &ArtifactRemap,
    destination: &mut BTreeMap<SourceMapId, SourceMapEntry>,
) -> Result<(), CompositionError> {
    for entry in artifact.source_maps.entries().values() {
        let id = remap.source_map(entry.id)?;
        destination.insert(
            id,
            SourceMapEntry {
                id,
                site: remap_site(entry.site, remap)?,
                anchors: entry.anchors.clone(),
                compiler_generated: entry.compiler_generated,
            },
        );
    }
    Ok(())
}

fn remap_probes(
    artifact: &FrontendArtifact,
    remap: &ArtifactRemap,
    destination: &mut BTreeMap<ProbeId, ProbeDefinition>,
) -> Result<(), CompositionError> {
    for probe in artifact.probes.entries().values() {
        let id = remap.probe(probe.id)?;
        destination.insert(
            id,
            ProbeDefinition {
                id,
                site: remap_site(probe.site, remap)?,
                kind: probe.kind,
                value_type: probe.value_type.clone(),
                source_map: remap.source_map(probe.source_map)?,
            },
        );
    }
    Ok(())
}

fn remap_site(
    site: SourceMapSite,
    remap: &ArtifactRemap,
) -> Result<SourceMapSite, CompositionError> {
    Ok(SourceMapSite {
        function: site.function,
        basic_block: remap.basic_block(site.basic_block)?,
        operation: site
            .operation
            .map(|operation| remap.operation(operation))
            .transpose()?,
    })
}
