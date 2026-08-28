use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};

use plc_program::{
    ADD, BOOL_AND, BOOL_NOT, BOOL_OR, BOOL_XOR, BREAKPOINT_MARKER, BlockId as ProgramBlockId,
    CALL_FB, CALL_FC, COMPARE_EQ, COMPARE_GE, COMPARE_GT, COMPARE_LE, COMPARE_LT, COMPARE_NE,
    COUNTER_DOWN, COUNTER_UP, COUNTER_UP_DOWN, CanonicalValue as ProgramValue, ControllerProgram,
    DIVIDE, DataType, DisabledExecutionBehavior, FALLING_EDGE, InterfaceMember, InterfaceMemberId,
    InterfaceRole, MODULO, MOVE, MULTIPLY, NO_OP, ObDeclaration, PROBE, ProgramUnitKind,
    RISING_EDGE, RetainPolicy, SUBTRACT, StateKind, TIMER_OFF_DELAY, TIMER_ON_DELAY, TIMER_PULSE,
    TRACE_SAMPLE,
};
use plc_runtime::{
    ArtifactError, ArtifactPackage, ArtifactSpec, BlockId as RuntimeBlockId,
    CanonicalValue as RuntimeValue, Instruction as RuntimeInstruction, MemoryDefinition, MemoryId,
    Operand as RuntimeOperand, Operation as RuntimeOperation, ProgramBlock as RuntimeProgramBlock,
    ProgramImage, RuntimeActivation, RuntimeBinaryOperator, RuntimeBlockCall, RuntimeBoundInput,
    RuntimeCallKind, RuntimeDeclaredOutput, RuntimeDisabledBehavior, RuntimeFormalRef,
    RuntimeFrameMember, RuntimeFrameMemberRole, RuntimeFunctionBlockInstance,
    RuntimeInstructionCode, RuntimeInstructionInstance, RuntimeInstructionInvocation,
    RuntimeInstructionStateKind, RuntimeUnaryOperator, TaskId, TimedTask, ValueType,
    runtime_block_signature_fingerprint,
};
use plc_types::{CanonicalF32, CanonicalF64};

use crate::{
    BinaryOperator, IrActivation, IrBasicBlockId, IrBoundInput, IrDeclaredOutput, IrFormalRef,
    IrInstanceIdentity, IrOperation, IrOperationId, IrOperationKind, IrTerminator,
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
    MissingCallableBody(ProgramBlockId),
    RecursiveCall(ProgramBlockId),
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

/// Projects independently verified IR into the Phase 2 runtime operation set.
/// Unsupported semantic operations fail with a stable typed gap; they are
/// never approximated or interpreted in the compiler.
///
/// # Errors
///
/// Returns a deterministic binding, mapping, runtime-artifact, or unsupported
/// operation error. No partial runtime artifact is returned.
pub fn project_verified_ir_to_runtime(
    verified_ir: &VerifiedIr,
    source_maps: &SourceMapTable,
    probes: &ProbeTable,
    program: &ControllerProgram,
    profile_fingerprint: plc_runtime::Hash32,
) -> Result<RuntimeArtifactProjection, RuntimeAdapterError> {
    let ir = verified_ir.program();
    let organization_blocks = collect_organization_blocks(ir, program)?;
    let reachable_blocks = collect_reachable_blocks(ir, program, &organization_blocks)?;
    let block_bindings = allocate_block_bindings(&reachable_blocks)?;
    let block_ids: BTreeMap<_, _> = block_bindings
        .iter()
        .map(|binding| (binding.owner, binding.block))
        .collect();

    let mut next_memory = 1_u32;
    let mut memory = Vec::new();
    let mut memory_bindings = Vec::new();
    let mut member_memory = BTreeMap::new();
    for &owner in &reachable_blocks {
        let block = program
            .block(owner)
            .ok_or(RuntimeAdapterError::MissingCallableBody(owner))?;
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

    let mut value_memory = BTreeMap::new();
    for &owner in &reachable_blocks {
        let function = ir
            .functions()
            .get(&owner)
            .ok_or(RuntimeAdapterError::MissingCallableBody(owner))?;
        for block in function.blocks.values() {
            for operation in &block.operations {
                if let Some(result) = &operation.result {
                    let id = allocate_memory_id(&mut next_memory)?;
                    let value_type = ir_value_type(owner, operation.id, &result.data_type)?;
                    memory.push(MemoryDefinition {
                        id,
                        value_type,
                        loaded_start: value_type.canonical_default(),
                        retentive: false,
                    });
                    value_memory.insert((owner, result.id), id);
                }
            }
        }
    }

    let mut source_bindings = Vec::new();
    let mut lowered_blocks = BTreeMap::new();
    let mut visiting = BTreeSet::new();
    for &owner in &organization_blocks {
        lower_function_recursive(
            owner,
            ir,
            program,
            &block_ids,
            &member_memory,
            &value_memory,
            source_maps,
            probes,
            &mut source_bindings,
            &mut lowered_blocks,
            &mut visiting,
        )?;
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

fn collect_reachable_blocks(
    ir: &TypedIrProgram,
    program: &ControllerProgram,
    roots: &[ProgramBlockId],
) -> Result<Vec<ProgramBlockId>, RuntimeAdapterError> {
    fn visit(
        owner: ProgramBlockId,
        ir: &TypedIrProgram,
        program: &ControllerProgram,
        visited: &mut BTreeSet<ProgramBlockId>,
        visiting: &mut BTreeSet<ProgramBlockId>,
    ) -> Result<(), RuntimeAdapterError> {
        if visited.contains(&owner) {
            return Ok(());
        }
        if !visiting.insert(owner) {
            return Err(RuntimeAdapterError::RecursiveCall(owner));
        }
        let function = ir
            .functions()
            .get(&owner)
            .ok_or(RuntimeAdapterError::MissingCallableBody(owner))?;
        let source = program
            .block(owner)
            .ok_or(RuntimeAdapterError::MissingCallableBody(owner))?;
        if !source.kind.is_executable() {
            return Err(RuntimeAdapterError::MissingCallableBody(owner));
        }
        for block in function.blocks.values() {
            for operation in &block.operations {
                if let IrOperationKind::CallBlock { target, .. } = operation.kind {
                    visit(target, ir, program, visited, visiting)?;
                }
            }
        }
        visiting.remove(&owner);
        visited.insert(owner);
        Ok(())
    }

    let mut visited = BTreeSet::new();
    let mut visiting = BTreeSet::new();
    for root in roots {
        visit(*root, ir, program, &mut visited, &mut visiting)?;
    }
    Ok(visited.into_iter().collect())
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
fn lower_function_recursive(
    owner: ProgramBlockId,
    ir: &TypedIrProgram,
    program: &ControllerProgram,
    runtime_blocks: &BTreeMap<ProgramBlockId, RuntimeBlockId>,
    member_memory: &BTreeMap<(ProgramBlockId, InterfaceMemberId), MemoryId>,
    value_memory: &BTreeMap<(ProgramBlockId, IrValueId), MemoryId>,
    source_maps: &SourceMapTable,
    probes: &ProbeTable,
    source_bindings: &mut Vec<RuntimeSourceBinding>,
    lowered_blocks: &mut BTreeMap<ProgramBlockId, RuntimeProgramBlock>,
    visiting: &mut BTreeSet<ProgramBlockId>,
) -> Result<(), RuntimeAdapterError> {
    if lowered_blocks.contains_key(&owner) {
        return Ok(());
    }
    if !visiting.insert(owner) {
        return Err(RuntimeAdapterError::RecursiveCall(owner));
    }
    let function = ir
        .functions()
        .get(&owner)
        .ok_or(RuntimeAdapterError::MissingCallableBody(owner))?;
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

    for operation in &block.operations {
        if let IrOperationKind::CallBlock { target, .. } = operation.kind {
            lower_function_recursive(
                target,
                ir,
                program,
                runtime_blocks,
                member_memory,
                value_memory,
                source_maps,
                probes,
                source_bindings,
                lowered_blocks,
                visiting,
            )?;
        }
    }

    let mut values = BTreeMap::new();
    let mut instructions = Vec::with_capacity(block.operations.len());
    let runtime_block = runtime_blocks[&owner];
    for operation in &block.operations {
        let result_memory = operation
            .result
            .as_ref()
            .map(|result| value_memory[&(owner, result.id)]);
        let lowered = lower_operation(
            owner,
            operation,
            result_memory,
            &values,
            member_memory,
            program,
            lowered_blocks,
            source_maps,
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
        if let Some(result) = &operation.result {
            values.insert(result.id, value_memory[&(owner, result.id)]);
        }
    }
    source_bindings.push(terminator_source_binding(
        function.owner,
        block.id,
        runtime_block,
        &block.terminator,
        source_maps,
        probes,
    )?);
    lowered_blocks.insert(
        owner,
        RuntimeProgramBlock {
            id: runtime_block,
            instructions,
        },
    );
    visiting.remove(&owner);
    Ok(())
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn lower_operation(
    owner: ProgramBlockId,
    operation: &IrOperation,
    result_memory: Option<MemoryId>,
    values: &BTreeMap<IrValueId, MemoryId>,
    members: &BTreeMap<(ProgramBlockId, InterfaceMemberId), MemoryId>,
    program: &ControllerProgram,
    lowered_blocks: &BTreeMap<ProgramBlockId, RuntimeProgramBlock>,
    source_maps: &SourceMapTable,
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
        IrOperationKind::Unary { operator, operand } => Ok(RuntimeOperation::Unary {
            operator: runtime_unary(*operator),
            operand: value(*operand)?,
            target: result()?,
        }),
        IrOperationKind::Binary {
            operator,
            left,
            right,
        } => Ok(RuntimeOperation::Binary {
            operator: runtime_binary(*operator),
            left: value(*left)?,
            right: value(*right)?,
            target: result()?,
        }),
        IrOperationKind::Convert {
            source,
            destination,
        } => Ok(RuntimeOperation::Convert {
            source: value(*source)?,
            destination: ir_value_type(owner, operation.id, destination)?,
            target: result()?,
        }),
        IrOperationKind::InvokeInstruction {
            instruction,
            inputs,
            outputs,
            instance,
            activation,
        } => Ok(RuntimeOperation::InvokeInstruction(
            RuntimeInstructionInvocation {
                instruction: runtime_instruction(*instruction).ok_or(
                    RuntimeAdapterError::UnsupportedOperation {
                        owner,
                        operation: operation.id,
                        semantic_operation: operation.kind.runtime_operation(),
                    },
                )?,
                inputs: runtime_inputs(owner, operation, inputs, &value)?,
                outputs: runtime_outputs(owner, operation, outputs)?,
                instance: runtime_instruction_instance(instance.as_ref()),
                activation: runtime_activation(owner, operation, activation.as_ref(), &value)?,
            },
        )),
        IrOperationKind::CallBlock {
            call_instruction,
            target,
            inputs,
            outputs,
            instance,
            activation,
        } => {
            let target_block = program
                .block(*target)
                .ok_or(RuntimeAdapterError::MissingCallableBody(*target))?;
            let frame_members = runtime_frame_members(*target, target_block, members)?;
            let kind = if *call_instruction == CALL_FC {
                RuntimeCallKind::Function
            } else if *call_instruction == CALL_FB {
                RuntimeCallKind::FunctionBlock
            } else {
                return Err(RuntimeAdapterError::UnsupportedOperation {
                    owner,
                    operation: operation.id,
                    semantic_operation: operation.kind.runtime_operation(),
                });
            };
            Ok(RuntimeOperation::CallBlock(RuntimeBlockCall {
                kind,
                target_identity: target.get(),
                signature_fingerprint: runtime_block_signature_fingerprint(
                    target.get(),
                    &frame_members,
                ),
                call_site_identity: runtime_call_site_identity(
                    owner,
                    operation,
                    *target,
                    source_maps,
                )?,
                inputs: runtime_inputs(owner, operation, inputs, &value)?,
                outputs: runtime_outputs(owner, operation, outputs)?,
                instance: runtime_function_block_instance(instance.as_ref()),
                activation: runtime_activation(owner, operation, activation.as_ref(), &value)?,
                frame_members,
                callee: lowered_blocks
                    .get(target)
                    .cloned()
                    .ok_or(RuntimeAdapterError::MissingCallableBody(*target))?,
            }))
        }
        IrOperationKind::InvocationOutput { invocation, formal } => {
            Ok(RuntimeOperation::InvocationOutput {
                invocation_id: invocation.get(),
                formal: runtime_formal(*formal),
                target: result()?,
            })
        }
    }
}

const fn runtime_unary(operator: UnaryOperator) -> RuntimeUnaryOperator {
    match operator {
        UnaryOperator::Plus => RuntimeUnaryOperator::Plus,
        UnaryOperator::Negate => RuntimeUnaryOperator::Negate,
        UnaryOperator::Not => RuntimeUnaryOperator::Not,
        UnaryOperator::Absolute => RuntimeUnaryOperator::Absolute,
    }
}

const fn runtime_binary(operator: BinaryOperator) -> RuntimeBinaryOperator {
    match operator {
        BinaryOperator::Multiply => RuntimeBinaryOperator::Multiply,
        BinaryOperator::Divide => RuntimeBinaryOperator::Divide,
        BinaryOperator::Modulo => RuntimeBinaryOperator::Modulo,
        BinaryOperator::Add => RuntimeBinaryOperator::Add,
        BinaryOperator::Subtract => RuntimeBinaryOperator::Subtract,
        BinaryOperator::Equal => RuntimeBinaryOperator::Equal,
        BinaryOperator::NotEqual => RuntimeBinaryOperator::NotEqual,
        BinaryOperator::Less => RuntimeBinaryOperator::Less,
        BinaryOperator::LessEqual => RuntimeBinaryOperator::LessEqual,
        BinaryOperator::Greater => RuntimeBinaryOperator::Greater,
        BinaryOperator::GreaterEqual => RuntimeBinaryOperator::GreaterEqual,
        BinaryOperator::And => RuntimeBinaryOperator::And,
        BinaryOperator::Xor => RuntimeBinaryOperator::Xor,
        BinaryOperator::Or => RuntimeBinaryOperator::Or,
        BinaryOperator::Minimum => RuntimeBinaryOperator::Minimum,
        BinaryOperator::Maximum => RuntimeBinaryOperator::Maximum,
    }
}

fn runtime_instruction(code: plc_program::InstructionCode) -> Option<RuntimeInstructionCode> {
    Some(if code == NO_OP {
        RuntimeInstructionCode::NoOp
    } else if code == MOVE {
        RuntimeInstructionCode::Move
    } else if code == BOOL_NOT {
        RuntimeInstructionCode::BoolNot
    } else if code == BOOL_AND {
        RuntimeInstructionCode::BoolAnd
    } else if code == BOOL_OR {
        RuntimeInstructionCode::BoolOr
    } else if code == BOOL_XOR {
        RuntimeInstructionCode::BoolXor
    } else if code == COMPARE_EQ {
        RuntimeInstructionCode::CompareEqual
    } else if code == COMPARE_NE {
        RuntimeInstructionCode::CompareNotEqual
    } else if code == COMPARE_LT {
        RuntimeInstructionCode::CompareLess
    } else if code == COMPARE_LE {
        RuntimeInstructionCode::CompareLessEqual
    } else if code == COMPARE_GT {
        RuntimeInstructionCode::CompareGreater
    } else if code == COMPARE_GE {
        RuntimeInstructionCode::CompareGreaterEqual
    } else if code == ADD {
        RuntimeInstructionCode::Add
    } else if code == SUBTRACT {
        RuntimeInstructionCode::Subtract
    } else if code == MULTIPLY {
        RuntimeInstructionCode::Multiply
    } else if code == DIVIDE {
        RuntimeInstructionCode::Divide
    } else if code == MODULO {
        RuntimeInstructionCode::Modulo
    } else if code == RISING_EDGE {
        RuntimeInstructionCode::RisingEdge
    } else if code == FALLING_EDGE {
        RuntimeInstructionCode::FallingEdge
    } else if code == TIMER_ON_DELAY {
        RuntimeInstructionCode::TimerOnDelay
    } else if code == TIMER_OFF_DELAY {
        RuntimeInstructionCode::TimerOffDelay
    } else if code == TIMER_PULSE {
        RuntimeInstructionCode::TimerPulse
    } else if code == COUNTER_UP {
        RuntimeInstructionCode::CounterUp
    } else if code == COUNTER_DOWN {
        RuntimeInstructionCode::CounterDown
    } else if code == COUNTER_UP_DOWN {
        RuntimeInstructionCode::CounterUpDown
    } else if code == PROBE {
        RuntimeInstructionCode::Probe
    } else if code == TRACE_SAMPLE {
        RuntimeInstructionCode::TraceSample
    } else if code == BREAKPOINT_MARKER {
        RuntimeInstructionCode::BreakpointMarker
    } else {
        return None;
    })
}

fn runtime_inputs(
    owner: ProgramBlockId,
    operation: &IrOperation,
    inputs: &[IrBoundInput],
    value: &impl Fn(IrValueId) -> Result<RuntimeOperand, RuntimeAdapterError>,
) -> Result<Vec<RuntimeBoundInput>, RuntimeAdapterError> {
    inputs
        .iter()
        .map(|input| {
            Ok(RuntimeBoundInput {
                formal: runtime_formal(input.formal),
                source: value(input.value).map_err(|_| RuntimeAdapterError::UnknownValue {
                    owner,
                    operation: operation.id,
                    value: input.value,
                })?,
            })
        })
        .collect()
}

fn runtime_outputs(
    owner: ProgramBlockId,
    operation: &IrOperation,
    outputs: &[IrDeclaredOutput],
) -> Result<Vec<RuntimeDeclaredOutput>, RuntimeAdapterError> {
    outputs
        .iter()
        .map(|output| {
            Ok(RuntimeDeclaredOutput {
                formal: runtime_formal(output.formal),
                value_type: ir_value_type(owner, operation.id, &output.data_type)?,
            })
        })
        .collect()
}

const fn runtime_formal(formal: IrFormalRef) -> RuntimeFormalRef {
    match formal {
        IrFormalRef::Instruction(id) => RuntimeFormalRef::Instruction(id.0),
        IrFormalRef::BlockMember(id) => RuntimeFormalRef::BlockMember(id.get()),
    }
}

fn runtime_activation(
    owner: ProgramBlockId,
    operation: &IrOperation,
    activation: Option<&IrActivation>,
    value: &impl Fn(IrValueId) -> Result<RuntimeOperand, RuntimeAdapterError>,
) -> Result<Option<RuntimeActivation>, RuntimeAdapterError> {
    activation
        .map(|activation| {
            Ok(RuntimeActivation {
                enable: value(activation.enable).map_err(|_| {
                    RuntimeAdapterError::UnknownValue {
                        owner,
                        operation: operation.id,
                        value: activation.enable,
                    }
                })?,
                enable_formal: activation.enable_formal.0,
                status_formal: activation.status_formal.0,
                status_when_disabled: activation.status_when_disabled,
                when_disabled: match activation.when_disabled {
                    DisabledExecutionBehavior::DefaultOutputsNoStateChange => {
                        RuntimeDisabledBehavior::DefaultOutputsNoStateChange
                    }
                    DisabledExecutionBehavior::PreserveOutputsNoStateChange => {
                        RuntimeDisabledBehavior::PreserveOutputsNoStateChange
                    }
                    DisabledExecutionBehavior::SuppressEffects => {
                        RuntimeDisabledBehavior::SuppressEffects
                    }
                },
            })
        })
        .transpose()
}

fn runtime_instruction_instance(
    instance: Option<&IrInstanceIdentity>,
) -> Option<RuntimeInstructionInstance> {
    let Some(IrInstanceIdentity::Instruction { stable_id, kind }) = instance else {
        return None;
    };
    Some(RuntimeInstructionInstance {
        stable_id: *stable_id,
        kind: match kind {
            StateKind::Edge => RuntimeInstructionStateKind::Edge,
            StateKind::Timer => RuntimeInstructionStateKind::Timer,
            StateKind::Counter => RuntimeInstructionStateKind::Counter,
        },
        retentive: false,
    })
}

fn runtime_function_block_instance(
    instance: Option<&IrInstanceIdentity>,
) -> Option<RuntimeFunctionBlockInstance> {
    let Some(IrInstanceIdentity::FunctionBlock(path)) = instance else {
        return None;
    };
    Some(RuntimeFunctionBlockInstance {
        root_instance: path.root_instance_db.get(),
        multi_instance_slots: path
            .multi_instance_slots
            .iter()
            .map(|slot| slot.get())
            .collect(),
    })
}

fn runtime_frame_members(
    owner: ProgramBlockId,
    block: &plc_program::ProgramBlock,
    members: &BTreeMap<(ProgramBlockId, InterfaceMemberId), MemoryId>,
) -> Result<Vec<RuntimeFrameMember>, RuntimeAdapterError> {
    block
        .interface
        .ordered_member_ids
        .iter()
        .map(|id| {
            let member = block
                .interface
                .member(*id)
                .ok_or(RuntimeAdapterError::UnknownMember { owner, member: *id })?;
            let value_type = member_value_type(owner, member)?;
            Ok(RuntimeFrameMember {
                formal: id.get(),
                memory: *members
                    .get(&(owner, *id))
                    .ok_or(RuntimeAdapterError::UnknownMember { owner, member: *id })?,
                value_type,
                role: match member.role {
                    InterfaceRole::Input => RuntimeFrameMemberRole::Input,
                    InterfaceRole::Output => RuntimeFrameMemberRole::Output,
                    InterfaceRole::InOut => RuntimeFrameMemberRole::InOut,
                    InterfaceRole::Static => RuntimeFrameMemberRole::Static,
                    InterfaceRole::Temp => RuntimeFrameMemberRole::Temp,
                    InterfaceRole::Constant => RuntimeFrameMemberRole::Constant,
                    InterfaceRole::Return => RuntimeFrameMemberRole::Return,
                },
                declared_order: member.declared_order,
                initial_value: member_loaded_start(owner, member, value_type)?,
                retentive: member.retain_policy == Some(RetainPolicy::Retentive),
            })
        })
        .collect()
}

fn runtime_call_site_identity(
    owner: ProgramBlockId,
    operation: &IrOperation,
    target: ProgramBlockId,
    source_maps: &SourceMapTable,
) -> Result<u128, RuntimeAdapterError> {
    let source_map = source_maps
        .get(operation.source_map)
        .ok_or(RuntimeAdapterError::MissingSourceMap(operation.source_map))?;
    let mut authored = source_map
        .anchors
        .iter()
        .filter_map(|anchor| anchor.call_site_id)
        .collect::<BTreeSet<_>>();
    if authored.len() > 1 {
        return Err(RuntimeAdapterError::MappingMismatch(operation.source_map));
    }
    if let Some(identity) = authored.pop_first() {
        return Ok(identity);
    }
    let mut hasher = crate::hash::CanonicalHasher::new("PES-RUNTIME-CALL-SITE-1");
    hasher.u128(owner.get());
    hasher.u32(operation.id.get());
    hasher.u128(target.get());
    let bytes = hasher.finish().0;
    Ok(u128::from_be_bytes(
        bytes[..16].try_into().expect("16-byte call-site identity"),
    ))
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
    if let Some(value_type) = member
        .data_type
        .primitive_type()
        .and_then(ValueType::from_primitive)
    {
        Ok(value_type)
    } else {
        Err(RuntimeAdapterError::UnsupportedMemberType {
            owner,
            member: member.id,
            data_type: member.data_type.clone(),
        })
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
    if let Some(value_type) = data_type
        .primitive_type()
        .and_then(ValueType::from_primitive)
    {
        Ok(value_type)
    } else {
        Err(RuntimeAdapterError::UnsupportedIrType {
            owner,
            operation,
            data_type: data_type.clone(),
        })
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
        ProgramValue::SInt(value) => Some(RuntimeValue::I8(*value)),
        ProgramValue::Int(value) => Some(RuntimeValue::I16(*value)),
        ProgramValue::DInt(value) => Some(RuntimeValue::I32(*value)),
        ProgramValue::LInt(value) => Some(RuntimeValue::I64(*value)),
        ProgramValue::USInt(value) => Some(RuntimeValue::U8(*value)),
        ProgramValue::UInt(value) => Some(RuntimeValue::U16(*value)),
        ProgramValue::UDInt(value) => Some(RuntimeValue::U32(*value)),
        ProgramValue::ULInt(value) => Some(RuntimeValue::U64(*value)),
        ProgramValue::Byte(value) => Some(RuntimeValue::Bits8(*value)),
        ProgramValue::Word(value) => Some(RuntimeValue::Bits16(*value)),
        ProgramValue::DWord(value) => Some(RuntimeValue::Bits32(*value)),
        ProgramValue::LWord(value) => Some(RuntimeValue::Bits64(*value)),
        ProgramValue::RealBits(value) => Some(RuntimeValue::F32(CanonicalF32::from_bits(*value))),
        ProgramValue::LRealBits(value) => Some(RuntimeValue::F64(CanonicalF64::from_bits(*value))),
        ProgramValue::Char(value) => Some(RuntimeValue::Char(*value)),
        ProgramValue::TimeMilliseconds(value) => Some(RuntimeValue::TimeMs(*value)),
        ProgramValue::StringBytes(_) => None,
    }
}
