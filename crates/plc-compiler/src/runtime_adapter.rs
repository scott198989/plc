use alloc::{collections::BTreeMap, vec::Vec};

use plc_program::{
    BlockId as ProgramBlockId, CanonicalValue as ProgramValue, ControllerProgram, DataType,
    InterfaceMember, InterfaceMemberId, ObDeclaration, ProgramUnitKind, RetainPolicy,
};
use plc_runtime::{
    ArtifactError, ArtifactPackage, ArtifactSpec, BlockId as RuntimeBlockId,
    CanonicalValue as RuntimeValue, Instruction as RuntimeInstruction, MemoryDefinition, MemoryId,
    Operand as RuntimeOperand, Operation as RuntimeOperation, ProgramBlock as RuntimeProgramBlock,
    ProgramImage, TaskId, TimedTask, ValueType,
};

use crate::{
    BinaryOperator, IrBasicBlockId, IrOperation, IrOperationId, IrOperationKind, IrTerminator,
    IrTerminatorKind, IrType, IrValueId, ProbeId, ProbeKind, ProbeTable, RuntimeOperationId,
    SourceAnchor, SourceMapId, SourceMapSite, SourceMapTable, TypedIrProgram, UnaryOperator,
    VerifiedIr,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeMemoryBinding {
    pub owner: ProgramBlockId,
    pub member: InterfaceMemberId,
    pub memory: MemoryId,
    pub value_type: ValueType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeBlockBinding {
    pub owner: ProgramBlockId,
    pub block: RuntimeBlockId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuntimeMappedSite {
    Instruction {
        block: RuntimeBlockId,
        operation_id: u32,
        source_identity: u128,
    },
    BlockReturn {
        block: RuntimeBlockId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeSourceBinding {
    pub runtime_site: RuntimeMappedSite,
    pub compiler_site: SourceMapSite,
    pub source_map: SourceMapId,
    pub probe: ProbeId,
    pub probe_kind: ProbeKind,
    pub anchors: Vec<SourceAnchor>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeArtifactProjection {
    package: ArtifactPackage,
    memory_bindings: Vec<RuntimeMemoryBinding>,
    block_bindings: Vec<RuntimeBlockBinding>,
    source_bindings: Vec<RuntimeSourceBinding>,
}

impl RuntimeArtifactProjection {
    #[must_use]
    pub const fn package(&self) -> &ArtifactPackage {
        &self.package
    }

    #[must_use]
    pub fn memory_bindings(&self) -> &[RuntimeMemoryBinding] {
        &self.memory_bindings
    }

    #[must_use]
    pub fn block_bindings(&self) -> &[RuntimeBlockBinding] {
        &self.block_bindings
    }

    #[must_use]
    pub fn source_bindings(&self) -> &[RuntimeSourceBinding] {
        &self.source_bindings
    }

    #[must_use]
    pub fn memory_for(&self, owner: ProgramBlockId, member: InterfaceMemberId) -> Option<MemoryId> {
        self.memory_bindings
            .binary_search_by_key(&(owner, member), |binding| (binding.owner, binding.member))
            .ok()
            .map(|index| self.memory_bindings[index].memory)
    }

    #[must_use]
    pub fn block_for(&self, owner: ProgramBlockId) -> Option<RuntimeBlockId> {
        self.block_bindings
            .binary_search_by_key(&owner, |binding| binding.owner)
            .ok()
            .map(|index| self.block_bindings[index].block)
    }

    #[must_use]
    pub fn source_for(&self, source_identity: u128) -> Option<&RuntimeSourceBinding> {
        self.source_bindings.iter().find(|binding| {
            matches!(
                binding.runtime_site,
                RuntimeMappedSite::Instruction {
                    source_identity: candidate,
                    ..
                } if candidate == source_identity
            )
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeAdapterError {
    MissingOrganizationBlock(ProgramBlockId),
    MissingCyclicMain,
    MultipleCyclicMain,
    MultipleStartup,
    IdentityExhausted(&'static str),
    UnsupportedMemberType {
        owner: ProgramBlockId,
        member: InterfaceMemberId,
        data_type: DataType,
    },
    UnsupportedMemberValue {
        owner: ProgramBlockId,
        member: InterfaceMemberId,
    },
    UnsupportedIrType {
        owner: ProgramBlockId,
        operation: IrOperationId,
        data_type: IrType,
    },
    UnsupportedControlFlow {
        owner: ProgramBlockId,
        basic_block: IrBasicBlockId,
    },
    UnsupportedOperation {
        owner: ProgramBlockId,
        operation: IrOperationId,
        semantic_operation: RuntimeOperationId,
    },
    UnknownMember {
        owner: ProgramBlockId,
        member: InterfaceMemberId,
    },
    UnknownValue {
        owner: ProgramBlockId,
        operation: IrOperationId,
        value: IrValueId,
    },
    MissingSourceMap(SourceMapId),
    MissingProbe(ProbeId),
    MappingMismatch(SourceMapId),
    RuntimeArtifact(ArtifactError),
}

impl From<ArtifactError> for RuntimeAdapterError {
    fn from(value: ArtifactError) -> Self {
        Self::RuntimeArtifact(value)
    }
}

pub(crate) fn lower_verified_ir_to_runtime(
    verified_ir: &VerifiedIr,
    source_maps: &SourceMapTable,
    probes: &ProbeTable,
    program: &ControllerProgram,
    profile_fingerprint: plc_runtime::Hash32,
) -> Result<RuntimeArtifactProjection, RuntimeAdapterError> {
    let ir = verified_ir.program();
    let organization_blocks = collect_organization_blocks(ir, program)?;
    let block_bindings = allocate_block_bindings(&organization_blocks)?;
    let block_ids: BTreeMap<_, _> = block_bindings
        .iter()
        .map(|binding| (binding.owner, binding.block))
        .collect();

    let mut next_memory = 1_u32;
    let mut memory = Vec::new();
    let mut memory_bindings = Vec::new();
    let mut member_memory = BTreeMap::new();
    for &owner in &organization_blocks {
        let block = program
            .block(owner)
            .ok_or(RuntimeAdapterError::MissingOrganizationBlock(owner))?;
        for member in block.interface.members.values() {
            let memory_id = allocate_memory_id(&mut next_memory)?;
            let value_type = member_value_type(owner, member)?;
            let loaded_start = member_loaded_start(owner, member, value_type)?;
            memory.push(MemoryDefinition {
                id: memory_id,
                value_type,
                loaded_start,
                retentive: member.retain_policy == Some(RetainPolicy::Retentive),
            });
            memory_bindings.push(RuntimeMemoryBinding {
                owner,
                member: member.id,
                memory: memory_id,
                value_type,
            });
            member_memory.insert((owner, member.id), memory_id);
        }
    }

    let mut source_bindings = Vec::new();
    let mut lowered_blocks = BTreeMap::new();
    for &owner in &organization_blocks {
        let function = ir
            .functions()
            .get(&owner)
            .ok_or(RuntimeAdapterError::MissingOrganizationBlock(owner))?;
        let runtime_block = block_ids[&owner];
        let lowered = lower_function(
            function,
            runtime_block,
            &member_memory,
            &mut next_memory,
            &mut memory,
            source_maps,
            probes,
            &mut source_bindings,
        )?;
        lowered_blocks.insert(owner, lowered);
    }

    let program_image = build_program_image(&organization_blocks, program, &mut lowered_blocks)?;
    let package = ArtifactPackage::seal_verified(ArtifactSpec::edu21(
        profile_fingerprint,
        memory,
        Vec::new(),
        Vec::new(),
        program_image,
    ))?;
    Ok(RuntimeArtifactProjection {
        package,
        memory_bindings,
        block_bindings,
        source_bindings,
    })
}

fn build_program_image(
    organization_blocks: &[ProgramBlockId],
    program: &ControllerProgram,
    lowered_blocks: &mut BTreeMap<ProgramBlockId, RuntimeProgramBlock>,
) -> Result<ProgramImage, RuntimeAdapterError> {
    let mut cyclic = None;
    let mut startup = None;
    let mut timed = Vec::new();
    let mut next_task = 1_u32;
    for &owner in organization_blocks {
        let block = program
            .block(owner)
            .ok_or(RuntimeAdapterError::MissingOrganizationBlock(owner))?;
        let lowered = lowered_blocks
            .remove(&owner)
            .ok_or(RuntimeAdapterError::MissingOrganizationBlock(owner))?;
        match block.kind {
            ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain) => {
                if cyclic.replace(lowered).is_some() {
                    return Err(RuntimeAdapterError::MultipleCyclicMain);
                }
            }
            ProgramUnitKind::OrganizationBlock(ObDeclaration::Startup) => {
                if startup.replace(lowered).is_some() {
                    return Err(RuntimeAdapterError::MultipleStartup);
                }
            }
            ProgramUnitKind::OrganizationBlock(ObDeclaration::TimedCyclic {
                period_milliseconds,
                offset_milliseconds,
                priority,
            }) => {
                let task = TaskId::new(next_task);
                next_task = next_task
                    .checked_add(1)
                    .ok_or(RuntimeAdapterError::IdentityExhausted("runtime task"))?;
                timed.push(TimedTask {
                    id: task,
                    first_due_ms: u64::from(offset_milliseconds),
                    period_ms: u64::from(period_milliseconds),
                    semantic_order: u32::from(priority),
                    block: lowered,
                });
            }
            ProgramUnitKind::Function
            | ProgramUnitKind::FunctionBlock
            | ProgramUnitKind::DataBlock(_) => {
                return Err(RuntimeAdapterError::MissingOrganizationBlock(owner));
            }
        }
    }
    Ok(ProgramImage {
        startup,
        timed,
        cyclic: cyclic.ok_or(RuntimeAdapterError::MissingCyclicMain)?,
    })
}

fn collect_organization_blocks(
    ir: &TypedIrProgram,
    program: &ControllerProgram,
) -> Result<Vec<ProgramBlockId>, RuntimeAdapterError> {
    let mut result = Vec::new();
    for (&owner, block) in program.blocks() {
        if matches!(block.kind, ProgramUnitKind::OrganizationBlock(_)) {
            if !ir.functions().contains_key(&owner) {
                return Err(RuntimeAdapterError::MissingOrganizationBlock(owner));
            }
            result.push(owner);
        }
    }
    Ok(result)
}

fn allocate_block_bindings(
    owners: &[ProgramBlockId],
) -> Result<Vec<RuntimeBlockBinding>, RuntimeAdapterError> {
    let mut next = 1_u32;
    owners
        .iter()
        .map(|&owner| {
            let block = RuntimeBlockId::new(next);
            next = next
                .checked_add(1)
                .ok_or(RuntimeAdapterError::IdentityExhausted("runtime block"))?;
            Ok(RuntimeBlockBinding { owner, block })
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn lower_function(
    function: &crate::IrFunction,
    runtime_block: RuntimeBlockId,
    member_memory: &BTreeMap<(ProgramBlockId, InterfaceMemberId), MemoryId>,
    next_memory: &mut u32,
    memory: &mut Vec<MemoryDefinition>,
    source_maps: &SourceMapTable,
    probes: &ProbeTable,
    source_bindings: &mut Vec<RuntimeSourceBinding>,
) -> Result<RuntimeProgramBlock, RuntimeAdapterError> {
    if function.blocks.len() != 1 || function.entry != *function.blocks.keys().next().unwrap() {
        return Err(RuntimeAdapterError::UnsupportedControlFlow {
            owner: function.owner,
            basic_block: function.entry,
        });
    }
    let block = function.blocks.get(&function.entry).ok_or(
        RuntimeAdapterError::UnsupportedControlFlow {
            owner: function.owner,
            basic_block: function.entry,
        },
    )?;
    if block.terminator.kind != IrTerminatorKind::Return {
        return Err(RuntimeAdapterError::UnsupportedControlFlow {
            owner: function.owner,
            basic_block: block.id,
        });
    }

    let mut values = BTreeMap::new();
    let mut instructions = Vec::with_capacity(block.operations.len());
    for operation in &block.operations {
        let result_memory = match &operation.result {
            Some(result) => {
                let id = allocate_memory_id(next_memory)?;
                let value_type = ir_value_type(function.owner, operation.id, &result.data_type)?;
                memory.push(MemoryDefinition {
                    id,
                    value_type,
                    loaded_start: value_type.canonical_default(),
                    retentive: false,
                });
                values.insert(result.id, id);
                Some(id)
            }
            None => None,
        };
        let lowered = lower_operation(
            function.owner,
            operation,
            result_memory,
            &values,
            member_memory,
        )?;
        let source_identity = pack_source_identity(runtime_block, operation);
        let binding = operation_source_binding(
            function.owner,
            block.id,
            runtime_block,
            operation,
            source_identity,
            source_maps,
            probes,
        )?;
        source_bindings.push(binding);
        instructions.push(RuntimeInstruction::new(
            operation.id.get(),
            source_identity,
            lowered,
        ));
    }
    source_bindings.push(terminator_source_binding(
        function.owner,
        block.id,
        runtime_block,
        &block.terminator,
        source_maps,
        probes,
    )?);
    Ok(RuntimeProgramBlock {
        id: runtime_block,
        instructions,
    })
}

fn lower_operation(
    owner: ProgramBlockId,
    operation: &IrOperation,
    result_memory: Option<MemoryId>,
    values: &BTreeMap<IrValueId, MemoryId>,
    members: &BTreeMap<(ProgramBlockId, InterfaceMemberId), MemoryId>,
) -> Result<RuntimeOperation, RuntimeAdapterError> {
    let result = || {
        result_memory.ok_or(RuntimeAdapterError::UnsupportedOperation {
            owner,
            operation: operation.id,
            semantic_operation: operation.kind.runtime_operation(),
        })
    };
    let value = |id| {
        values.get(&id).copied().map(RuntimeOperand::Memory).ok_or(
            RuntimeAdapterError::UnknownValue {
                owner,
                operation: operation.id,
                value: id,
            },
        )
    };
    match &operation.kind {
        IrOperationKind::Constant(value_to_set) => Ok(RuntimeOperation::SetMemory {
            target: result()?,
            value: runtime_constant(owner, operation.id, value_to_set)?,
        }),
        IrOperationKind::LoadMember { member } => Ok(RuntimeOperation::Copy {
            source: RuntimeOperand::Memory(*members.get(&(owner, *member)).ok_or(
                RuntimeAdapterError::UnknownMember {
                    owner,
                    member: *member,
                },
            )?),
            target: result()?,
        }),
        IrOperationKind::StoreMember {
            target,
            value: source,
        } => Ok(RuntimeOperation::Copy {
            source: value(*source)?,
            target: *members
                .get(&(owner, *target))
                .ok_or(RuntimeAdapterError::UnknownMember {
                    owner,
                    member: *target,
                })?,
        }),
        IrOperationKind::Unary {
            operator: UnaryOperator::Plus,
            operand,
        } => Ok(RuntimeOperation::Copy {
            source: value(*operand)?,
            target: result()?,
        }),
        IrOperationKind::Binary {
            operator: BinaryOperator::Add,
            left,
            right,
        } => Ok(RuntimeOperation::AddI32 {
            left: value(*left)?,
            right: value(*right)?,
            target: result()?,
        }),
        IrOperationKind::Binary {
            operator: BinaryOperator::Divide,
            left,
            right,
        } => Ok(RuntimeOperation::DivideI32 {
            numerator: value(*left)?,
            denominator: value(*right)?,
            target: result()?,
        }),
        IrOperationKind::Convert {
            source,
            destination: IrType::Int | IrType::DInt,
        } => Ok(RuntimeOperation::Copy {
            source: value(*source)?,
            target: result()?,
        }),
        IrOperationKind::Unary { .. }
        | IrOperationKind::Binary { .. }
        | IrOperationKind::Convert { .. } => Err(RuntimeAdapterError::UnsupportedOperation {
            owner,
            operation: operation.id,
            semantic_operation: operation.kind.runtime_operation(),
        }),
    }
}

fn operation_source_binding(
    owner: ProgramBlockId,
    basic_block: IrBasicBlockId,
    runtime_block: RuntimeBlockId,
    operation: &IrOperation,
    source_identity: u128,
    source_maps: &SourceMapTable,
    probes: &ProbeTable,
) -> Result<RuntimeSourceBinding, RuntimeAdapterError> {
    let expected_site = SourceMapSite {
        function: owner,
        basic_block,
        operation: Some(operation.id),
    };
    source_binding(
        RuntimeMappedSite::Instruction {
            block: runtime_block,
            operation_id: operation.id.get(),
            source_identity,
        },
        expected_site,
        operation.source_map,
        operation.probe,
        source_maps,
        probes,
    )
}

fn terminator_source_binding(
    owner: ProgramBlockId,
    basic_block: IrBasicBlockId,
    runtime_block: RuntimeBlockId,
    terminator: &IrTerminator,
    source_maps: &SourceMapTable,
    probes: &ProbeTable,
) -> Result<RuntimeSourceBinding, RuntimeAdapterError> {
    source_binding(
        RuntimeMappedSite::BlockReturn {
            block: runtime_block,
        },
        SourceMapSite {
            function: owner,
            basic_block,
            operation: None,
        },
        terminator.source_map,
        terminator.probe,
        source_maps,
        probes,
    )
}

fn source_binding(
    runtime_site: RuntimeMappedSite,
    expected_site: SourceMapSite,
    source_map: SourceMapId,
    probe: ProbeId,
    source_maps: &SourceMapTable,
    probes: &ProbeTable,
) -> Result<RuntimeSourceBinding, RuntimeAdapterError> {
    let map = source_maps
        .get(source_map)
        .ok_or(RuntimeAdapterError::MissingSourceMap(source_map))?;
    let probe_definition = probes
        .get(probe)
        .ok_or(RuntimeAdapterError::MissingProbe(probe))?;
    if map.site != expected_site
        || probe_definition.site != expected_site
        || probe_definition.source_map != source_map
    {
        return Err(RuntimeAdapterError::MappingMismatch(source_map));
    }
    Ok(RuntimeSourceBinding {
        runtime_site,
        compiler_site: expected_site,
        source_map,
        probe,
        probe_kind: probe_definition.kind,
        anchors: map.anchors.clone(),
    })
}

const fn pack_source_identity(runtime_block: RuntimeBlockId, operation: &IrOperation) -> u128 {
    (runtime_block.get() as u128) << 96
        | (operation.source_map.get() as u128) << 64
        | (operation.probe.get() as u128) << 32
        | operation.id.get() as u128
}

fn allocate_memory_id(next: &mut u32) -> Result<MemoryId, RuntimeAdapterError> {
    let id = MemoryId::new(*next);
    *next = next
        .checked_add(1)
        .ok_or(RuntimeAdapterError::IdentityExhausted("runtime memory"))?;
    Ok(id)
}

fn member_value_type(
    owner: ProgramBlockId,
    member: &InterfaceMember,
) -> Result<ValueType, RuntimeAdapterError> {
    match member.data_type {
        DataType::Bool => Ok(ValueType::Bool),
        DataType::Int | DataType::DInt => Ok(ValueType::I32),
        DataType::Time => Ok(ValueType::TimeMs),
        DataType::Real
        | DataType::String { .. }
        | DataType::Named(_)
        | DataType::BlockInstance(_)
        | DataType::InstructionState(_) => Err(RuntimeAdapterError::UnsupportedMemberType {
            owner,
            member: member.id,
            data_type: member.data_type.clone(),
        }),
    }
}

fn member_loaded_start(
    owner: ProgramBlockId,
    member: &InterfaceMember,
    value_type: ValueType,
) -> Result<RuntimeValue, RuntimeAdapterError> {
    let initial = member
        .constant_value
        .as_ref()
        .or(member.start_value.as_ref())
        .or(member.default_value.as_ref());
    initial.map_or(Ok(value_type.canonical_default()), |value| {
        runtime_member_value(owner, member.id, value)
    })
}

fn ir_value_type(
    owner: ProgramBlockId,
    operation: IrOperationId,
    data_type: &IrType,
) -> Result<ValueType, RuntimeAdapterError> {
    match data_type {
        IrType::Bool => Ok(ValueType::Bool),
        IrType::Int | IrType::DInt => Ok(ValueType::I32),
        IrType::Time => Ok(ValueType::TimeMs),
        IrType::Real | IrType::String { .. } => Err(RuntimeAdapterError::UnsupportedIrType {
            owner,
            operation,
            data_type: data_type.clone(),
        }),
    }
}

fn runtime_member_value(
    owner: ProgramBlockId,
    member: InterfaceMemberId,
    value: &ProgramValue,
) -> Result<RuntimeValue, RuntimeAdapterError> {
    convert_value(value).ok_or(RuntimeAdapterError::UnsupportedMemberValue { owner, member })
}

fn runtime_constant(
    owner: ProgramBlockId,
    operation: IrOperationId,
    value: &ProgramValue,
) -> Result<RuntimeValue, RuntimeAdapterError> {
    convert_value(value).ok_or(RuntimeAdapterError::UnsupportedOperation {
        owner,
        operation,
        semantic_operation: RuntimeOperationId("EDU.RT.CONSTANT.v1"),
    })
}

fn convert_value(value: &ProgramValue) -> Option<RuntimeValue> {
    match value {
        ProgramValue::Bool(value) => Some(RuntimeValue::Bool(*value)),
        ProgramValue::Int(value) => Some(RuntimeValue::I32(i32::from(*value))),
        ProgramValue::DInt(value) => Some(RuntimeValue::I32(*value)),
        ProgramValue::TimeMilliseconds(value) => {
            u64::try_from(*value).ok().map(RuntimeValue::TimeMs)
        }
        ProgramValue::RealBits(_) | ProgramValue::StringBytes(_) => None,
    }
}
