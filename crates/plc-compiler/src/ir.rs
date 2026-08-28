use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::String,
    vec::Vec,
};

use plc_program::{
    BlockId, BoundInstructionFormal, CALL_FB, CALL_FC, CanonicalValue, ControllerProgram,
    DataBlockKind, DataType, DisabledExecutionBehavior, InstancePath, InstructionActivationPolicy,
    InstructionCode, InstructionFormalDirection, InstructionFormalId, InterfaceMemberId,
    InterfaceRole, ProgramUnitKind, StateKind, StateRequirement, phase2_instruction_registry,
};
use plc_runtime::Hash32;
use plc_types::{PrimitiveCategory, PrimitiveType, explicit_conversion_allowed};

use crate::{
    IrBasicBlockId, IrOperationId, IrValueId, ProbeId, SourceAnchor, SourceMapId, TYPED_IR_VERSION,
    hash::CanonicalHasher,
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IrType {
    Bool,
    SInt,
    Int,
    DInt,
    LInt,
    USInt,
    UInt,
    UDInt,
    ULInt,
    Byte,
    Word,
    DWord,
    LWord,
    Real,
    LReal,
    Char,
    Time,
    String { capacity: u16 },
}

impl IrType {
    pub(crate) fn from_program_type(value: &DataType) -> Option<Self> {
        match value {
            DataType::Bool => Some(Self::Bool),
            DataType::SInt => Some(Self::SInt),
            DataType::Int => Some(Self::Int),
            DataType::DInt => Some(Self::DInt),
            DataType::LInt => Some(Self::LInt),
            DataType::USInt => Some(Self::USInt),
            DataType::UInt => Some(Self::UInt),
            DataType::UDInt => Some(Self::UDInt),
            DataType::ULInt => Some(Self::ULInt),
            DataType::Byte => Some(Self::Byte),
            DataType::Word => Some(Self::Word),
            DataType::DWord => Some(Self::DWord),
            DataType::LWord => Some(Self::LWord),
            DataType::Real => Some(Self::Real),
            DataType::LReal => Some(Self::LReal),
            DataType::Char => Some(Self::Char),
            DataType::Time => Some(Self::Time),
            DataType::String { capacity } => Some(Self::String {
                capacity: *capacity,
            }),
            DataType::Named(_) | DataType::BlockInstance(_) | DataType::InstructionState(_) => None,
        }
    }

    #[must_use]
    pub fn to_program_type(&self) -> DataType {
        match self {
            Self::Bool => DataType::Bool,
            Self::SInt => DataType::SInt,
            Self::Int => DataType::Int,
            Self::DInt => DataType::DInt,
            Self::LInt => DataType::LInt,
            Self::USInt => DataType::USInt,
            Self::UInt => DataType::UInt,
            Self::UDInt => DataType::UDInt,
            Self::ULInt => DataType::ULInt,
            Self::Byte => DataType::Byte,
            Self::Word => DataType::Word,
            Self::DWord => DataType::DWord,
            Self::LWord => DataType::LWord,
            Self::Real => DataType::Real,
            Self::LReal => DataType::LReal,
            Self::Char => DataType::Char,
            Self::Time => DataType::Time,
            Self::String { capacity } => DataType::String {
                capacity: *capacity,
            },
        }
    }

    #[must_use]
    pub fn primitive_type(&self) -> Option<PrimitiveType> {
        self.to_program_type().primitive_type()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeOperationId(pub &'static str);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UnaryOperator {
    Plus,
    Negate,
    Not,
    Absolute,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BinaryOperator {
    Multiply,
    Divide,
    Modulo,
    Add,
    Subtract,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Xor,
    Or,
    Minimum,
    Maximum,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrValue {
    pub id: IrValueId,
    pub data_type: IrType,
}

/// Stable formal identity shared by registry instructions and block calls.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IrFormalRef {
    Instruction(InstructionFormalId),
    BlockMember(InterfaceMemberId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrBoundInput {
    pub formal: IrFormalRef,
    pub value: IrValueId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrDeclaredOutput {
    pub formal: IrFormalRef,
    pub data_type: IrType,
}

/// Explicit semantic state identity. It is data only and cannot allocate or
/// access host/runtime state.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IrInstanceIdentity {
    Instruction { stable_id: u128, kind: StateKind },
    FunctionBlock(InstancePath),
}

/// Fully materialized EN/ENO behavior copied from the canonical registry.
/// Verification rejects any drift from that registry definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IrActivation {
    pub enable: IrValueId,
    pub enable_formal: InstructionFormalId,
    pub status_formal: InstructionFormalId,
    pub status_when_disabled: bool,
    pub when_disabled: DisabledExecutionBehavior,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IrOperationKind {
    Constant(CanonicalValue),
    LoadMember {
        member: InterfaceMemberId,
    },
    StoreMember {
        target: InterfaceMemberId,
        value: IrValueId,
    },
    Unary {
        operator: UnaryOperator,
        operand: IrValueId,
    },
    Binary {
        operator: BinaryOperator,
        left: IrValueId,
        right: IrValueId,
    },
    /// Decides in widened mathematical space whether one FOR increment remains
    /// on the inclusive terminal side. The widened value is never materialized.
    ForNextWithin {
        current: IrValueId,
        terminal: IrValueId,
        step: IrValueId,
        ascending: bool,
    },
    Convert {
        source: IrValueId,
        destination: IrType,
    },
    /// Registry-defined built-in invocation. Results are explicitly declared
    /// here and materialized by later `InvocationOutput` operations.
    InvokeInstruction {
        instruction: InstructionCode,
        inputs: Vec<IrBoundInput>,
        outputs: Vec<IrDeclaredOutput>,
        instance: Option<IrInstanceIdentity>,
        activation: Option<IrActivation>,
    },
    /// FC/FB call over the canonical block model and `CALL_FC`/`CALL_FB` registry
    /// entries. Copy-in/copy-out formals are stable interface-member IDs.
    CallBlock {
        call_instruction: InstructionCode,
        target: BlockId,
        inputs: Vec<IrBoundInput>,
        outputs: Vec<IrDeclaredOutput>,
        instance: Option<IrInstanceIdentity>,
        activation: Option<IrActivation>,
    },
    /// One typed SSA projection of a preceding invocation's declared output.
    InvocationOutput {
        invocation: IrOperationId,
        formal: IrFormalRef,
    },
}

impl IrOperationKind {
    #[must_use]
    pub const fn runtime_operation(&self) -> RuntimeOperationId {
        match self {
            Self::Constant(_) => RuntimeOperationId("EDU.RT.CONSTANT.v1"),
            Self::LoadMember { .. } => RuntimeOperationId("EDU.RT.LOAD_MEMBER.v1"),
            Self::StoreMember { .. } => RuntimeOperationId("EDU.RT.STORE_MEMBER.v1"),
            Self::Unary { operator, .. } => match operator {
                UnaryOperator::Plus => RuntimeOperationId("EDU.RT.UNARY_PLUS.v1"),
                UnaryOperator::Negate => RuntimeOperationId("EDU.RT.NEGATE.v1"),
                UnaryOperator::Not => RuntimeOperationId("EDU.RT.BOOL_NOT.v1"),
                UnaryOperator::Absolute => RuntimeOperationId("EDU.RT.ABS_CHECKED.v1"),
            },
            Self::Binary { operator, .. } => match operator {
                BinaryOperator::Multiply => RuntimeOperationId("EDU.RT.MULTIPLY.v1"),
                BinaryOperator::Divide => RuntimeOperationId("EDU.RT.DIVIDE_CHECKED.v1"),
                BinaryOperator::Modulo => RuntimeOperationId("EDU.RT.MODULO_CHECKED.v1"),
                BinaryOperator::Add => RuntimeOperationId("EDU.RT.ADD_CHECKED.v1"),
                BinaryOperator::Subtract => RuntimeOperationId("EDU.RT.SUBTRACT_CHECKED.v1"),
                BinaryOperator::Equal => RuntimeOperationId("EDU.RT.COMPARE_EQUAL.v1"),
                BinaryOperator::NotEqual => RuntimeOperationId("EDU.RT.COMPARE_NOT_EQUAL.v1"),
                BinaryOperator::Less => RuntimeOperationId("EDU.RT.COMPARE_LESS.v1"),
                BinaryOperator::LessEqual => RuntimeOperationId("EDU.RT.COMPARE_LESS_EQUAL.v1"),
                BinaryOperator::Greater => RuntimeOperationId("EDU.RT.COMPARE_GREATER.v1"),
                BinaryOperator::GreaterEqual => {
                    RuntimeOperationId("EDU.RT.COMPARE_GREATER_EQUAL.v1")
                }
                BinaryOperator::And => RuntimeOperationId("EDU.RT.BOOL_AND_EAGER.v1"),
                BinaryOperator::Xor => RuntimeOperationId("EDU.RT.BOOL_XOR_EAGER.v1"),
                BinaryOperator::Or => RuntimeOperationId("EDU.RT.BOOL_OR_EAGER.v1"),
                BinaryOperator::Minimum => RuntimeOperationId("EDU.RT.MINIMUM.v1"),
                BinaryOperator::Maximum => RuntimeOperationId("EDU.RT.MAXIMUM.v1"),
            },
            Self::ForNextWithin { .. } => RuntimeOperationId("EDU.RT.FOR_NEXT_WITHIN.v1"),
            Self::Convert { .. } => RuntimeOperationId("EDU.RT.CONVERT_CHECKED.v1"),
            Self::InvokeInstruction { .. } => RuntimeOperationId("EDU.RT.INVOKE_INSTRUCTION.v1"),
            Self::CallBlock { .. } => RuntimeOperationId("EDU.RT.CALL_BLOCK.v1"),
            Self::InvocationOutput { .. } => RuntimeOperationId("EDU.RT.INVOCATION_OUTPUT.v1"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrOperation {
    pub id: IrOperationId,
    pub result: Option<IrValue>,
    pub kind: IrOperationKind,
    pub source_map: SourceMapId,
    pub probe: ProbeId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IrTerminatorKind {
    Jump(IrBasicBlockId),
    Branch {
        condition: IrValueId,
        when_true: IrBasicBlockId,
        when_false: IrBasicBlockId,
    },
    Return,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrTerminator {
    pub kind: IrTerminatorKind,
    pub source_map: SourceMapId,
    pub probe: ProbeId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrBasicBlock {
    pub id: IrBasicBlockId,
    pub operations: Vec<IrOperation>,
    pub terminator: IrTerminator,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrFunction {
    pub owner: BlockId,
    pub source_kind: ProgramUnitKind,
    pub entry: IrBasicBlockId,
    pub blocks: BTreeMap<IrBasicBlockId, IrBasicBlock>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedIrProgram {
    schema_version: String,
    functions: BTreeMap<BlockId, IrFunction>,
}

impl TypedIrProgram {
    #[must_use]
    pub fn from_untrusted_parts(
        schema_version: impl Into<String>,
        functions: BTreeMap<BlockId, IrFunction>,
    ) -> Self {
        Self {
            schema_version: schema_version.into(),
            functions,
        }
    }

    #[must_use]
    pub fn schema_version(&self) -> &str {
        &self.schema_version
    }

    #[must_use]
    pub const fn functions(&self) -> &BTreeMap<BlockId, IrFunction> {
        &self.functions
    }

    #[must_use]
    pub fn semantic_fingerprint(&self) -> Hash32 {
        let mut hasher = CanonicalHasher::new("PES-TYPED-IR-SEMANTIC-1");
        hasher.string(&self.schema_version);
        hasher.u64(self.functions.len() as u64);
        for (owner, function) in &self.functions {
            hasher.u128(owner.get());
            encode_program_kind(&mut hasher, function.source_kind);
            hasher.u32(function.entry.get());
            hasher.u64(function.blocks.len() as u64);
            for (block_id, block) in &function.blocks {
                hasher.u32(block_id.get());
                hasher.u64(block.operations.len() as u64);
                for operation in &block.operations {
                    encode_operation(&mut hasher, operation);
                }
                encode_terminator(&mut hasher, &block.terminator.kind);
            }
        }
        hasher.finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceMapSite {
    pub function: BlockId,
    pub basic_block: IrBasicBlockId,
    pub operation: Option<IrOperationId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceMapEntry {
    pub id: SourceMapId,
    pub site: SourceMapSite,
    pub anchors: Vec<SourceAnchor>,
    pub compiler_generated: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceMapTable {
    entries: BTreeMap<SourceMapId, SourceMapEntry>,
}

impl SourceMapTable {
    #[must_use]
    pub fn from_untrusted_entries(entries: BTreeMap<SourceMapId, SourceMapEntry>) -> Self {
        Self { entries }
    }

    #[must_use]
    pub const fn entries(&self) -> &BTreeMap<SourceMapId, SourceMapEntry> {
        &self.entries
    }

    #[must_use]
    pub fn get(&self, id: SourceMapId) -> Option<&SourceMapEntry> {
        self.entries.get(&id)
    }

    #[must_use]
    pub fn ir_to_source(&self, site: SourceMapSite) -> Vec<&SourceAnchor> {
        self.entries
            .values()
            .filter(|entry| entry.site == site)
            .flat_map(|entry| entry.anchors.iter())
            .collect()
    }

    #[must_use]
    pub fn source_to_ir(&self, anchor: &SourceAnchor) -> Vec<SourceMapSite> {
        self.entries
            .values()
            .filter(|entry| entry.anchors.contains(anchor))
            .map(|entry| entry.site)
            .collect()
    }

    #[must_use]
    pub fn fingerprint(&self) -> Hash32 {
        let mut hasher = CanonicalHasher::new("PES-SOURCE-MAPS-1");
        hasher.u64(self.entries.len() as u64);
        for (id, entry) in &self.entries {
            hasher.u32(id.get());
            encode_site(&mut hasher, entry.site);
            hasher.bool(entry.compiler_generated);
            hasher.u64(entry.anchors.len() as u64);
            for anchor in &entry.anchors {
                encode_anchor(&mut hasher, anchor);
            }
        }
        hasher.finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProbeKind {
    Constant,
    StorageRead,
    StorageWrite,
    Expression,
    Branch,
    Return,
    NetworkPower,
    PortValue,
    EdgeValue,
    Call,
    State,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbeDefinition {
    pub id: ProbeId,
    pub site: SourceMapSite,
    pub kind: ProbeKind,
    pub value_type: Option<IrType>,
    pub source_map: SourceMapId,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProbeTable {
    entries: BTreeMap<ProbeId, ProbeDefinition>,
}

impl ProbeTable {
    #[must_use]
    pub fn from_untrusted_entries(entries: BTreeMap<ProbeId, ProbeDefinition>) -> Self {
        Self { entries }
    }

    #[must_use]
    pub const fn entries(&self) -> &BTreeMap<ProbeId, ProbeDefinition> {
        &self.entries
    }

    #[must_use]
    pub fn get(&self, id: ProbeId) -> Option<&ProbeDefinition> {
        self.entries.get(&id)
    }

    #[must_use]
    pub fn source_to_probes(
        &self,
        source_maps: &SourceMapTable,
        anchor: &SourceAnchor,
    ) -> Vec<ProbeId> {
        let sites: BTreeSet<_> = source_maps.source_to_ir(anchor).into_iter().collect();
        self.entries
            .values()
            .filter(|probe| sites.contains(&probe.site))
            .map(|probe| probe.id)
            .collect()
    }

    #[must_use]
    pub fn fingerprint(&self) -> Hash32 {
        let mut hasher = CanonicalHasher::new("PES-PROBE-TABLE-1");
        hasher.u64(self.entries.len() as u64);
        for (id, probe) in &self.entries {
            hasher.u32(id.get());
            encode_site(&mut hasher, probe.site);
            hasher.u8(probe_kind_tag(probe.kind));
            match &probe.value_type {
                Some(data_type) => {
                    hasher.bool(true);
                    encode_type(&mut hasher, data_type);
                }
                None => hasher.bool(false),
            }
            hasher.u32(probe.source_map.get());
        }
        hasher.finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum VerificationError {
    SchemaVersion,
    FunctionKeyMismatch(BlockId),
    UnknownFunction(BlockId),
    NonExecutableFunction(BlockId),
    MissingEntry(BlockId, IrBasicBlockId),
    BlockKeyMismatch(BlockId, IrBasicBlockId),
    DuplicateOperation(BlockId, IrOperationId),
    DuplicateValue(BlockId, IrValueId),
    UnknownValue(BlockId, IrValueId),
    NonDominatingValue(BlockId, IrValueId, IrBasicBlockId),
    UnknownMember(BlockId, InterfaceMemberId),
    ReadOnlyStore(BlockId, InterfaceMemberId),
    TypeMismatch(BlockId, IrOperationId),
    InvalidConversion(BlockId, IrOperationId),
    UnknownInstruction(BlockId, IrOperationId, InstructionCode),
    UnknownCallee(BlockId, IrOperationId, BlockId),
    InvalidInvocationFormal(BlockId, IrOperationId, IrFormalRef),
    MissingInvocationFormal(BlockId, IrOperationId, IrFormalRef),
    NonCanonicalInvocation(BlockId, IrOperationId),
    InvalidInvocationInstance(BlockId, IrOperationId),
    InvalidActivation(BlockId, IrOperationId),
    UnknownInvocation(BlockId, IrOperationId, IrOperationId),
    DuplicateInvocationOutput(BlockId, IrOperationId, IrFormalRef),
    MissingResult(BlockId, IrOperationId),
    UnexpectedResult(BlockId, IrOperationId),
    MissingTarget(BlockId, IrBasicBlockId),
    MissingSourceMap(SourceMapId),
    SourceMapKeyMismatch(SourceMapId),
    SourceMapSiteMismatch(SourceMapId),
    EmptySourceMap(SourceMapId),
    InvalidSourceAnchor(SourceMapId),
    MissingProbe(ProbeId),
    ProbeKeyMismatch(ProbeId),
    ProbeSiteMismatch(ProbeId),
    ProbeSourceMapMismatch(ProbeId),
    ProbeTypeMismatch(ProbeId),
    OrphanSourceMap(SourceMapId),
    OrphanProbe(ProbeId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedIr {
    program: TypedIrProgram,
    verification_hash: Hash32,
}

impl VerifiedIr {
    #[must_use]
    pub const fn program(&self) -> &TypedIrProgram {
        &self.program
    }

    #[must_use]
    pub const fn verification_hash(&self) -> Hash32 {
        self.verification_hash
    }
}

/// Independently verifies untrusted typed IR and every map/probe reference.
///
/// # Errors
///
/// Returns the first deterministic structural, type, identity, mapping, or
/// probe defect. A failed value is never wrapped as [`VerifiedIr`].
#[allow(clippy::too_many_lines)]
pub fn verify_typed_ir(
    ir: TypedIrProgram,
    source_maps: &SourceMapTable,
    probes: &ProbeTable,
    program: &ControllerProgram,
) -> Result<VerifiedIr, VerificationError> {
    if ir.schema_version != TYPED_IR_VERSION {
        return Err(VerificationError::SchemaVersion);
    }
    let mut used_maps = BTreeSet::new();
    let mut used_probes = BTreeSet::new();
    for (&function_key, function) in &ir.functions {
        if function_key != function.owner {
            return Err(VerificationError::FunctionKeyMismatch(function_key));
        }
        let source_block = program
            .block(function.owner)
            .ok_or(VerificationError::UnknownFunction(function.owner))?;
        if !source_block.kind.is_executable() || source_block.kind != function.source_kind {
            return Err(VerificationError::NonExecutableFunction(function.owner));
        }
        if !function.blocks.contains_key(&function.entry) {
            return Err(VerificationError::MissingEntry(
                function.owner,
                function.entry,
            ));
        }
        let mut operation_ids = BTreeSet::new();
        let mut value_types = BTreeMap::<IrValueId, IrType>::new();
        let mut value_definitions = BTreeMap::<IrValueId, (IrBasicBlockId, usize)>::new();
        for block in function.blocks.values() {
            for (index, operation) in block.operations.iter().enumerate() {
                if !operation_ids.insert(operation.id) {
                    return Err(VerificationError::DuplicateOperation(
                        function.owner,
                        operation.id,
                    ));
                }
                if let Some(result) = &operation.result {
                    if value_types
                        .insert(result.id, result.data_type.clone())
                        .is_some()
                    {
                        return Err(VerificationError::DuplicateValue(function.owner, result.id));
                    }
                    value_definitions.insert(result.id, (block.id, index));
                }
            }
        }
        verify_value_dominance(function, &value_definitions)?;
        for (&block_key, block) in &function.blocks {
            if block_key != block.id {
                return Err(VerificationError::BlockKeyMismatch(
                    function.owner,
                    block_key,
                ));
            }
            let mut invocations = BTreeMap::<IrOperationId, VerifiedInvocation>::new();
            let mut projected_outputs = BTreeSet::<(IrOperationId, IrFormalRef)>::new();
            for operation in &block.operations {
                verify_source_probe(
                    function.owner,
                    block.id,
                    Some(operation.id),
                    operation.source_map,
                    operation.probe,
                    operation.result.as_ref().map(|value| &value.data_type),
                    source_maps,
                    probes,
                    &mut used_maps,
                    &mut used_probes,
                )?;
                let invocation = verify_operation(
                    operation,
                    source_block,
                    program,
                    &value_types,
                    &invocations,
                    &mut projected_outputs,
                    function.owner,
                )?;
                if let Some(invocation) = invocation {
                    invocations.insert(operation.id, invocation);
                }
            }
            verify_terminator(
                function,
                block,
                &value_types,
                source_maps,
                probes,
                &mut used_maps,
                &mut used_probes,
            )?;
        }
    }
    for (&id, entry) in source_maps.entries() {
        if id != entry.id {
            return Err(VerificationError::SourceMapKeyMismatch(id));
        }
        if entry.anchors.is_empty() {
            return Err(VerificationError::EmptySourceMap(id));
        }
        if entry
            .anchors
            .iter()
            .any(|anchor| !anchor.is_well_formed_for(entry.site.function))
        {
            return Err(VerificationError::InvalidSourceAnchor(id));
        }
        if !used_maps.contains(&id) {
            return Err(VerificationError::OrphanSourceMap(id));
        }
    }
    for (&id, entry) in probes.entries() {
        if id != entry.id {
            return Err(VerificationError::ProbeKeyMismatch(id));
        }
        if !used_probes.contains(&id) {
            return Err(VerificationError::OrphanProbe(id));
        }
    }
    let mut hasher = CanonicalHasher::new("PES-VERIFIED-IR-1");
    hasher.hash(ir.semantic_fingerprint());
    hasher.hash(source_maps.fingerprint());
    hasher.hash(probes.fingerprint());
    Ok(VerifiedIr {
        program: ir,
        verification_hash: hasher.finish(),
    })
}

fn verify_value_dominance(
    function: &IrFunction,
    definitions: &BTreeMap<IrValueId, (IrBasicBlockId, usize)>,
) -> Result<(), VerificationError> {
    let reachable = reachable_ir_blocks(function);
    let predecessors = ir_predecessors(function, &reachable);
    let dominators = ir_dominators(function.entry, &reachable, &predecessors);

    for block in function.blocks.values() {
        for (operation_index, operation) in block.operations.iter().enumerate() {
            for value in operation_value_uses(&operation.kind) {
                verify_dominating_use(
                    function.owner,
                    block.id,
                    operation_index,
                    value,
                    definitions,
                    dominators.get(&block.id),
                )?;
            }
        }
        if let IrTerminatorKind::Branch { condition, .. } = block.terminator.kind {
            verify_dominating_use(
                function.owner,
                block.id,
                block.operations.len(),
                condition,
                definitions,
                dominators.get(&block.id),
            )?;
        }
    }
    Ok(())
}

fn reachable_ir_blocks(function: &IrFunction) -> BTreeSet<IrBasicBlockId> {
    let mut reachable = BTreeSet::new();
    let mut pending = alloc::vec![function.entry];
    while let Some(block_id) = pending.pop() {
        if !reachable.insert(block_id) {
            continue;
        }
        let Some(block) = function.blocks.get(&block_id) else {
            continue;
        };
        match block.terminator.kind {
            IrTerminatorKind::Jump(target) => pending.push(target),
            IrTerminatorKind::Branch {
                when_true,
                when_false,
                ..
            } => {
                pending.push(when_false);
                pending.push(when_true);
            }
            IrTerminatorKind::Return => {}
        }
    }
    reachable
}

fn ir_predecessors(
    function: &IrFunction,
    reachable: &BTreeSet<IrBasicBlockId>,
) -> BTreeMap<IrBasicBlockId, BTreeSet<IrBasicBlockId>> {
    let mut predecessors = BTreeMap::<IrBasicBlockId, BTreeSet<IrBasicBlockId>>::new();
    for block_id in reachable {
        predecessors.entry(*block_id).or_default();
    }
    for block_id in reachable {
        let Some(block) = function.blocks.get(block_id) else {
            continue;
        };
        match &block.terminator.kind {
            IrTerminatorKind::Jump(target) => {
                if reachable.contains(target) {
                    predecessors.entry(*target).or_default().insert(*block_id);
                }
            }
            IrTerminatorKind::Branch {
                when_true,
                when_false,
                ..
            } => {
                if reachable.contains(when_true) {
                    predecessors
                        .entry(*when_true)
                        .or_default()
                        .insert(*block_id);
                }
                if reachable.contains(when_false) {
                    predecessors
                        .entry(*when_false)
                        .or_default()
                        .insert(*block_id);
                }
            }
            IrTerminatorKind::Return => {}
        }
    }
    predecessors
}

fn ir_dominators(
    entry: IrBasicBlockId,
    reachable: &BTreeSet<IrBasicBlockId>,
    predecessors: &BTreeMap<IrBasicBlockId, BTreeSet<IrBasicBlockId>>,
) -> BTreeMap<IrBasicBlockId, BTreeSet<IrBasicBlockId>> {
    let mut dominators = BTreeMap::<IrBasicBlockId, BTreeSet<IrBasicBlockId>>::new();
    for block_id in reachable {
        dominators.insert(
            *block_id,
            if *block_id == entry {
                BTreeSet::from([entry])
            } else {
                reachable.clone()
            },
        );
    }
    loop {
        let mut changed = false;
        for block_id in reachable.iter().copied().filter(|id| *id != entry) {
            let incoming = &predecessors[&block_id];
            let mut next = if let Some(first) = incoming.first() {
                dominators[first].clone()
            } else {
                BTreeSet::new()
            };
            for predecessor in incoming.iter().skip(1) {
                next.retain(|candidate| dominators[predecessor].contains(candidate));
            }
            next.insert(block_id);
            if dominators[&block_id] != next {
                dominators.insert(block_id, next);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    dominators
}

fn verify_dominating_use(
    owner: BlockId,
    use_block: IrBasicBlockId,
    use_index: usize,
    value: IrValueId,
    definitions: &BTreeMap<IrValueId, (IrBasicBlockId, usize)>,
    dominators: Option<&BTreeSet<IrBasicBlockId>>,
) -> Result<(), VerificationError> {
    let Some((definition_block, definition_index)) = definitions.get(&value).copied() else {
        return Err(VerificationError::UnknownValue(owner, value));
    };
    let valid = if definition_block == use_block {
        definition_index < use_index
    } else {
        dominators.is_some_and(|blocks| blocks.contains(&definition_block))
    };
    if valid {
        Ok(())
    } else {
        Err(VerificationError::NonDominatingValue(
            owner, value, use_block,
        ))
    }
}

fn operation_value_uses(kind: &IrOperationKind) -> Vec<IrValueId> {
    let mut values = Vec::new();
    match kind {
        IrOperationKind::Constant(_)
        | IrOperationKind::LoadMember { .. }
        | IrOperationKind::InvocationOutput { .. } => {}
        IrOperationKind::StoreMember { value, .. } => values.push(*value),
        IrOperationKind::Unary { operand, .. } => values.push(*operand),
        IrOperationKind::Binary { left, right, .. } => {
            values.push(*left);
            values.push(*right);
        }
        IrOperationKind::ForNextWithin {
            current,
            terminal,
            step,
            ..
        } => {
            values.push(*current);
            values.push(*terminal);
            values.push(*step);
        }
        IrOperationKind::Convert { source, .. } => values.push(*source),
        IrOperationKind::InvokeInstruction {
            inputs, activation, ..
        }
        | IrOperationKind::CallBlock {
            inputs, activation, ..
        } => {
            values.extend(inputs.iter().map(|input| input.value));
            if let Some(activation) = activation {
                values.push(activation.enable);
            }
        }
    }
    values
}

#[allow(clippy::too_many_arguments)]
fn verify_source_probe(
    function: BlockId,
    basic_block: IrBasicBlockId,
    operation: Option<IrOperationId>,
    source_map: SourceMapId,
    probe: ProbeId,
    value_type: Option<&IrType>,
    source_maps: &SourceMapTable,
    probes: &ProbeTable,
    used_maps: &mut BTreeSet<SourceMapId>,
    used_probes: &mut BTreeSet<ProbeId>,
) -> Result<(), VerificationError> {
    let expected_site = SourceMapSite {
        function,
        basic_block,
        operation,
    };
    let map = source_maps
        .get(source_map)
        .ok_or(VerificationError::MissingSourceMap(source_map))?;
    if map.site != expected_site {
        return Err(VerificationError::SourceMapSiteMismatch(source_map));
    }
    let probe_definition = probes
        .get(probe)
        .ok_or(VerificationError::MissingProbe(probe))?;
    if probe_definition.site != expected_site {
        return Err(VerificationError::ProbeSiteMismatch(probe));
    }
    if probe_definition.source_map != source_map {
        return Err(VerificationError::ProbeSourceMapMismatch(probe));
    }
    if probe_definition.value_type.as_ref() != value_type {
        return Err(VerificationError::ProbeTypeMismatch(probe));
    }
    used_maps.insert(source_map);
    used_probes.insert(probe);
    Ok(())
}

#[derive(Clone, Debug)]
struct VerifiedInvocation {
    outputs: BTreeMap<IrFormalRef, IrType>,
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn verify_operation(
    operation: &IrOperation,
    source_block: &plc_program::ProgramBlock,
    program: &ControllerProgram,
    values: &BTreeMap<IrValueId, IrType>,
    invocations: &BTreeMap<IrOperationId, VerifiedInvocation>,
    projected_outputs: &mut BTreeSet<(IrOperationId, IrFormalRef)>,
    function: BlockId,
) -> Result<Option<VerifiedInvocation>, VerificationError> {
    let result_type = operation.result.as_ref().map(|value| &value.data_type);
    let invocation = match &operation.kind {
        IrOperationKind::Constant(value) => {
            let result =
                result_type.ok_or(VerificationError::MissingResult(function, operation.id))?;
            if !canonical_matches_ir(value, result) {
                return Err(VerificationError::TypeMismatch(function, operation.id));
            }
            None
        }
        IrOperationKind::LoadMember { member } => {
            let declared = source_block
                .interface
                .member(*member)
                .ok_or(VerificationError::UnknownMember(function, *member))?;
            let expected = IrType::from_program_type(&declared.data_type)
                .ok_or(VerificationError::TypeMismatch(function, operation.id))?;
            if result_type != Some(&expected) {
                return Err(VerificationError::TypeMismatch(function, operation.id));
            }
            None
        }
        IrOperationKind::StoreMember { target, value } => {
            if result_type.is_some() {
                return Err(VerificationError::UnexpectedResult(function, operation.id));
            }
            let declared = source_block
                .interface
                .member(*target)
                .ok_or(VerificationError::UnknownMember(function, *target))?;
            if matches!(
                declared.role,
                InterfaceRole::Input | InterfaceRole::Constant
            ) {
                return Err(VerificationError::ReadOnlyStore(function, *target));
            }
            let expected = IrType::from_program_type(&declared.data_type)
                .ok_or(VerificationError::TypeMismatch(function, operation.id))?;
            let actual = values
                .get(value)
                .ok_or(VerificationError::UnknownValue(function, *value))?;
            if actual != &expected {
                return Err(VerificationError::TypeMismatch(function, operation.id));
            }
            None
        }
        IrOperationKind::Unary { operator, operand } => {
            let operand_type = values
                .get(operand)
                .ok_or(VerificationError::UnknownValue(function, *operand))?;
            let result =
                result_type.ok_or(VerificationError::MissingResult(function, operation.id))?;
            let valid = match operator {
                UnaryOperator::Not => {
                    operand_type.primitive_type().is_some_and(|primitive| {
                        primitive == PrimitiveType::Bool || primitive.is_bit_string()
                    }) && result == operand_type
                }
                UnaryOperator::Plus => is_ir_numeric(operand_type) && result == operand_type,
                UnaryOperator::Negate | UnaryOperator::Absolute => {
                    operand_type.primitive_type().is_some_and(|primitive| {
                        primitive.is_signed_integer()
                            || matches!(primitive, PrimitiveType::Real | PrimitiveType::Lreal)
                    }) && result == operand_type
                }
            };
            if !valid {
                return Err(VerificationError::TypeMismatch(function, operation.id));
            }
            None
        }
        IrOperationKind::Binary {
            operator,
            left,
            right,
        } => {
            let left_type = values
                .get(left)
                .ok_or(VerificationError::UnknownValue(function, *left))?;
            let right_type = values
                .get(right)
                .ok_or(VerificationError::UnknownValue(function, *right))?;
            let result =
                result_type.ok_or(VerificationError::MissingResult(function, operation.id))?;
            let valid = verify_binary_types(*operator, left_type, right_type, result);
            if !valid {
                return Err(VerificationError::TypeMismatch(function, operation.id));
            }
            None
        }
        IrOperationKind::ForNextWithin {
            current,
            terminal,
            step,
            ..
        } => {
            let current_type = values
                .get(current)
                .ok_or(VerificationError::UnknownValue(function, *current))?;
            let terminal_type = values
                .get(terminal)
                .ok_or(VerificationError::UnknownValue(function, *terminal))?;
            let step_type = values
                .get(step)
                .ok_or(VerificationError::UnknownValue(function, *step))?;
            let signed = current_type
                .primitive_type()
                .is_some_and(PrimitiveType::is_signed_integer);
            if !signed
                || current_type != terminal_type
                || current_type != step_type
                || result_type != Some(&IrType::Bool)
            {
                return Err(VerificationError::TypeMismatch(function, operation.id));
            }
            None
        }
        IrOperationKind::Convert {
            source,
            destination,
        } => {
            let source_type = values
                .get(source)
                .ok_or(VerificationError::UnknownValue(function, *source))?;
            if result_type != Some(destination) || !conversion_allowed(source_type, destination) {
                return Err(VerificationError::InvalidConversion(function, operation.id));
            }
            None
        }
        IrOperationKind::InvokeInstruction {
            instruction,
            inputs,
            outputs,
            instance,
            activation,
        } => Some(verify_instruction_invocation(
            operation,
            *instruction,
            inputs,
            outputs,
            instance.as_ref(),
            activation.as_ref(),
            values,
            function,
        )?),
        IrOperationKind::CallBlock {
            call_instruction,
            target,
            inputs,
            outputs,
            instance,
            activation,
        } => Some(verify_block_call(
            operation,
            *call_instruction,
            *target,
            inputs,
            outputs,
            instance.as_ref(),
            activation.as_ref(),
            values,
            program,
            function,
        )?),
        IrOperationKind::InvocationOutput { invocation, formal } => {
            let declaration =
                invocations
                    .get(invocation)
                    .ok_or(VerificationError::UnknownInvocation(
                        function,
                        operation.id,
                        *invocation,
                    ))?;
            let expected = declaration.outputs.get(formal).ok_or(
                VerificationError::InvalidInvocationFormal(function, operation.id, *formal),
            )?;
            if result_type != Some(expected) {
                return Err(VerificationError::TypeMismatch(function, operation.id));
            }
            if !projected_outputs.insert((*invocation, *formal)) {
                return Err(VerificationError::DuplicateInvocationOutput(
                    function,
                    operation.id,
                    *formal,
                ));
            }
            None
        }
    };
    if operation.kind.runtime_operation().0.is_empty() {
        return Err(VerificationError::TypeMismatch(function, operation.id));
    }
    Ok(invocation)
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn verify_instruction_invocation(
    operation: &IrOperation,
    instruction: InstructionCode,
    inputs: &[IrBoundInput],
    outputs: &[IrDeclaredOutput],
    instance: Option<&IrInstanceIdentity>,
    activation: Option<&IrActivation>,
    values: &BTreeMap<IrValueId, IrType>,
    function: BlockId,
) -> Result<VerifiedInvocation, VerificationError> {
    if operation.result.is_some() {
        return Err(VerificationError::UnexpectedResult(function, operation.id));
    }
    let registry = *phase2_instruction_registry();
    let definition = registry
        .lookup(instruction)
        .ok_or(VerificationError::UnknownInstruction(
            function,
            operation.id,
            instruction,
        ))?;
    if matches!(instruction, CALL_FC | CALL_FB) {
        return Err(VerificationError::InvalidInvocationInstance(
            function,
            operation.id,
        ));
    }
    ensure_canonical_bindings(operation, inputs, outputs, function)?;
    let mut bound_types = BTreeMap::<InstructionFormalId, DataType>::new();
    for input in inputs {
        let IrFormalRef::Instruction(formal_id) = input.formal else {
            return Err(VerificationError::InvalidInvocationFormal(
                function,
                operation.id,
                input.formal,
            ));
        };
        let formal =
            definition
                .formal(formal_id)
                .ok_or(VerificationError::InvalidInvocationFormal(
                    function,
                    operation.id,
                    input.formal,
                ))?;
        if !matches!(
            formal.direction,
            InstructionFormalDirection::Input | InstructionFormalDirection::InOut
        ) {
            return Err(VerificationError::InvalidInvocationFormal(
                function,
                operation.id,
                input.formal,
            ));
        }
        let data_type = values
            .get(&input.value)
            .ok_or(VerificationError::UnknownValue(function, input.value))?
            .to_program_type();
        insert_bound_type(&mut bound_types, formal_id, data_type, operation, function)?;
    }
    let mut declared_outputs = BTreeMap::new();
    for output in outputs {
        let IrFormalRef::Instruction(formal_id) = output.formal else {
            return Err(VerificationError::InvalidInvocationFormal(
                function,
                operation.id,
                output.formal,
            ));
        };
        let formal =
            definition
                .formal(formal_id)
                .ok_or(VerificationError::InvalidInvocationFormal(
                    function,
                    operation.id,
                    output.formal,
                ))?;
        if !matches!(
            formal.direction,
            InstructionFormalDirection::Output
                | InstructionFormalDirection::InOut
                | InstructionFormalDirection::Status
        ) {
            return Err(VerificationError::InvalidInvocationFormal(
                function,
                operation.id,
                output.formal,
            ));
        }
        insert_bound_type(
            &mut bound_types,
            formal_id,
            output.data_type.to_program_type(),
            operation,
            function,
        )?;
        declared_outputs.insert(output.formal, output.data_type.clone());
    }
    add_and_verify_instance_binding(
        definition.state_requirement,
        instance,
        None,
        &mut bound_types,
        operation,
        function,
    )?;
    add_and_verify_activation_bindings(
        definition.activation,
        activation,
        outputs,
        values,
        &mut bound_types,
        operation,
        function,
    )?;
    let bindings = bound_types
        .into_iter()
        .map(|(formal, data_type)| BoundInstructionFormal { formal, data_type });
    registry
        .bind_types(instruction, bindings)
        .map_err(|error| instruction_binding_error(error, function, operation.id))?;
    Ok(VerifiedInvocation {
        outputs: declared_outputs,
    })
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn verify_block_call(
    operation: &IrOperation,
    call_instruction: InstructionCode,
    target: BlockId,
    inputs: &[IrBoundInput],
    outputs: &[IrDeclaredOutput],
    instance: Option<&IrInstanceIdentity>,
    activation: Option<&IrActivation>,
    values: &BTreeMap<IrValueId, IrType>,
    program: &ControllerProgram,
    function: BlockId,
) -> Result<VerifiedInvocation, VerificationError> {
    if operation.result.is_some() {
        return Err(VerificationError::UnexpectedResult(function, operation.id));
    }
    ensure_canonical_bindings(operation, inputs, outputs, function)?;
    let callee = program
        .block(target)
        .ok_or(VerificationError::UnknownCallee(
            function,
            operation.id,
            target,
        ))?;
    let expected_kind = if call_instruction == CALL_FC {
        ProgramUnitKind::Function
    } else if call_instruction == CALL_FB {
        ProgramUnitKind::FunctionBlock
    } else {
        return Err(VerificationError::UnknownInstruction(
            function,
            operation.id,
            call_instruction,
        ));
    };
    if callee.kind != expected_kind {
        return Err(VerificationError::InvalidInvocationInstance(
            function,
            operation.id,
        ));
    }
    let registry = *phase2_instruction_registry();
    let call_definition =
        registry
            .lookup(call_instruction)
            .ok_or(VerificationError::UnknownInstruction(
                function,
                operation.id,
                call_instruction,
            ))?;
    let mut seen_inputs = BTreeSet::new();
    for input in inputs {
        let IrFormalRef::BlockMember(member_id) = input.formal else {
            return Err(VerificationError::InvalidInvocationFormal(
                function,
                operation.id,
                input.formal,
            ));
        };
        let member = callee.interface.member(member_id).ok_or(
            VerificationError::InvalidInvocationFormal(function, operation.id, input.formal),
        )?;
        if !matches!(member.role, InterfaceRole::Input | InterfaceRole::InOut) {
            return Err(VerificationError::InvalidInvocationFormal(
                function,
                operation.id,
                input.formal,
            ));
        }
        let expected = IrType::from_program_type(&member.data_type)
            .ok_or(VerificationError::TypeMismatch(function, operation.id))?;
        if values.get(&input.value) != Some(&expected) {
            return Err(VerificationError::TypeMismatch(function, operation.id));
        }
        seen_inputs.insert(member_id);
    }
    let mut seen_outputs = BTreeSet::new();
    let mut declared_outputs = BTreeMap::new();
    for output in outputs {
        match output.formal {
            IrFormalRef::BlockMember(member_id) => {
                let member = callee.interface.member(member_id).ok_or(
                    VerificationError::InvalidInvocationFormal(
                        function,
                        operation.id,
                        output.formal,
                    ),
                )?;
                if !matches!(
                    member.role,
                    InterfaceRole::Output | InterfaceRole::InOut | InterfaceRole::Return
                ) {
                    return Err(VerificationError::InvalidInvocationFormal(
                        function,
                        operation.id,
                        output.formal,
                    ));
                }
                let expected = IrType::from_program_type(&member.data_type)
                    .ok_or(VerificationError::TypeMismatch(function, operation.id))?;
                if output.data_type != expected {
                    return Err(VerificationError::TypeMismatch(function, operation.id));
                }
                seen_outputs.insert(member_id);
            }
            IrFormalRef::Instruction(formal_id) => {
                let formal = call_definition.formal(formal_id).ok_or(
                    VerificationError::InvalidInvocationFormal(
                        function,
                        operation.id,
                        output.formal,
                    ),
                )?;
                if formal.direction != InstructionFormalDirection::Status
                    || output.data_type != IrType::Bool
                {
                    return Err(VerificationError::InvalidInvocationFormal(
                        function,
                        operation.id,
                        output.formal,
                    ));
                }
            }
        }
        declared_outputs.insert(output.formal, output.data_type.clone());
    }
    for member in callee.interface.members.values() {
        let input_required = match member.role {
            InterfaceRole::Input => member.default_value.is_none(),
            InterfaceRole::InOut => true,
            _ => false,
        };
        if input_required && !seen_inputs.contains(&member.id) {
            return Err(VerificationError::MissingInvocationFormal(
                function,
                operation.id,
                IrFormalRef::BlockMember(member.id),
            ));
        }
        let output_required = member.role == InterfaceRole::InOut
            || (matches!(member.role, InterfaceRole::Output | InterfaceRole::Return)
                && member.required_output_binding);
        if output_required && !seen_outputs.contains(&member.id) {
            return Err(VerificationError::MissingInvocationFormal(
                function,
                operation.id,
                IrFormalRef::BlockMember(member.id),
            ));
        }
    }
    let mut registry_bindings = BTreeMap::new();
    add_and_verify_instance_binding(
        call_definition.state_requirement,
        instance,
        Some((program, target)),
        &mut registry_bindings,
        operation,
        function,
    )?;
    add_and_verify_activation_bindings(
        call_definition.activation,
        activation,
        outputs,
        values,
        &mut registry_bindings,
        operation,
        function,
    )?;
    registry
        .bind_types(
            call_instruction,
            registry_bindings
                .into_iter()
                .map(|(formal, data_type)| BoundInstructionFormal { formal, data_type }),
        )
        .map_err(|error| instruction_binding_error(error, function, operation.id))?;
    Ok(VerifiedInvocation {
        outputs: declared_outputs,
    })
}

fn ensure_canonical_bindings(
    operation: &IrOperation,
    inputs: &[IrBoundInput],
    outputs: &[IrDeclaredOutput],
    function: BlockId,
) -> Result<(), VerificationError> {
    let inputs_canonical = inputs
        .windows(2)
        .all(|pair| pair[0].formal < pair[1].formal);
    let outputs_canonical = outputs
        .windows(2)
        .all(|pair| pair[0].formal < pair[1].formal);
    if !inputs_canonical || !outputs_canonical {
        return Err(VerificationError::NonCanonicalInvocation(
            function,
            operation.id,
        ));
    }
    Ok(())
}

fn insert_bound_type(
    bindings: &mut BTreeMap<InstructionFormalId, DataType>,
    formal: InstructionFormalId,
    data_type: DataType,
    operation: &IrOperation,
    function: BlockId,
) -> Result<(), VerificationError> {
    if let Some(previous) = bindings.get(&formal) {
        if previous != &data_type {
            return Err(VerificationError::TypeMismatch(function, operation.id));
        }
        return Ok(());
    }
    bindings.insert(formal, data_type);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn add_and_verify_activation_bindings(
    policy: InstructionActivationPolicy,
    activation: Option<&IrActivation>,
    outputs: &[IrDeclaredOutput],
    values: &BTreeMap<IrValueId, IrType>,
    bindings: &mut BTreeMap<InstructionFormalId, DataType>,
    operation: &IrOperation,
    function: BlockId,
) -> Result<(), VerificationError> {
    match (policy, activation) {
        (InstructionActivationPolicy::None, None) => {}
        (InstructionActivationPolicy::None, Some(_)) => {
            return Err(VerificationError::InvalidActivation(function, operation.id));
        }
        (InstructionActivationPolicy::EnableStatus { status, .. }, None) => {
            if outputs
                .iter()
                .any(|output| output.formal == IrFormalRef::Instruction(status))
            {
                return Err(VerificationError::InvalidActivation(function, operation.id));
            }
        }
        (
            InstructionActivationPolicy::EnableStatus {
                enable,
                status,
                status_when_disabled,
                when_disabled,
            },
            Some(actual),
        ) => {
            if actual.enable_formal != enable
                || actual.status_formal != status
                || actual.status_when_disabled != status_when_disabled
                || actual.when_disabled != when_disabled
                || values.get(&actual.enable) != Some(&IrType::Bool)
                || !outputs.iter().any(|output| {
                    output.formal == IrFormalRef::Instruction(status)
                        && output.data_type == IrType::Bool
                })
            {
                return Err(VerificationError::InvalidActivation(function, operation.id));
            }
            insert_bound_type(bindings, enable, DataType::Bool, operation, function)?;
            insert_bound_type(bindings, status, DataType::Bool, operation, function)?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn add_and_verify_instance_binding(
    requirement: StateRequirement,
    instance: Option<&IrInstanceIdentity>,
    block_target: Option<(&ControllerProgram, BlockId)>,
    bindings: &mut BTreeMap<InstructionFormalId, DataType>,
    operation: &IrOperation,
    function: BlockId,
) -> Result<(), VerificationError> {
    let valid = match (requirement, instance) {
        (StateRequirement::None, None) => true,
        (
            StateRequirement::Explicit(expected),
            Some(IrInstanceIdentity::Instruction { stable_id, kind }),
        ) if *stable_id != 0 && *kind == expected => {
            insert_bound_type(
                bindings,
                plc_program::FORMAL_STATE,
                DataType::InstructionState(expected),
                operation,
                function,
            )?;
            true
        }
        (
            StateRequirement::FunctionBlockInstance,
            Some(IrInstanceIdentity::FunctionBlock(path)),
        ) => {
            let Some((program, target)) = block_target else {
                return Err(VerificationError::InvalidInvocationInstance(
                    function,
                    operation.id,
                ));
            };
            if !instance_path_targets(program, path, target) {
                return Err(VerificationError::InvalidInvocationInstance(
                    function,
                    operation.id,
                ));
            }
            insert_bound_type(
                bindings,
                plc_program::FORMAL_STATE,
                DataType::BlockInstance(target),
                operation,
                function,
            )?;
            true
        }
        _ => false,
    };
    if !valid {
        return Err(VerificationError::InvalidInvocationInstance(
            function,
            operation.id,
        ));
    }
    Ok(())
}

fn instance_path_targets(
    program: &ControllerProgram,
    path: &InstancePath,
    target: BlockId,
) -> bool {
    let Some(root) = program.block(path.root_instance_db) else {
        return false;
    };
    let ProgramUnitKind::DataBlock(DataBlockKind::Instance { mut fb_type }) = root.kind else {
        return false;
    };
    for member_id in &path.multi_instance_slots {
        let Some(fb) = program.block(fb_type) else {
            return false;
        };
        let Some(member) = fb.interface.member(*member_id) else {
            return false;
        };
        let DataType::BlockInstance(next) = member.data_type else {
            return false;
        };
        if member.role != InterfaceRole::Static {
            return false;
        }
        fb_type = next;
    }
    fb_type == target
}

fn instruction_binding_error(
    error: plc_program::InstructionBindingError,
    function: BlockId,
    operation: IrOperationId,
) -> VerificationError {
    match error {
        plc_program::InstructionBindingError::UnknownInstruction(instruction) => {
            VerificationError::UnknownInstruction(function, operation, instruction)
        }
        plc_program::InstructionBindingError::UnknownFormal(_, formal)
        | plc_program::InstructionBindingError::DuplicateFormal(_, formal)
        | plc_program::InstructionBindingError::TypeConstraint(_, formal) => {
            VerificationError::InvalidInvocationFormal(
                function,
                operation,
                IrFormalRef::Instruction(formal),
            )
        }
        plc_program::InstructionBindingError::MissingRequiredFormal(_, formal) => {
            VerificationError::MissingInvocationFormal(
                function,
                operation,
                IrFormalRef::Instruction(formal),
            )
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn verify_terminator(
    function: &IrFunction,
    block: &IrBasicBlock,
    values: &BTreeMap<IrValueId, IrType>,
    source_maps: &SourceMapTable,
    probes: &ProbeTable,
    used_maps: &mut BTreeSet<SourceMapId>,
    used_probes: &mut BTreeSet<ProbeId>,
) -> Result<(), VerificationError> {
    verify_source_probe(
        function.owner,
        block.id,
        None,
        block.terminator.source_map,
        block.terminator.probe,
        match &block.terminator.kind {
            IrTerminatorKind::Branch { .. } => Some(&IrType::Bool),
            IrTerminatorKind::Jump(_) | IrTerminatorKind::Return => None,
        },
        source_maps,
        probes,
        used_maps,
        used_probes,
    )?;
    match block.terminator.kind {
        IrTerminatorKind::Jump(target) => {
            if !function.blocks.contains_key(&target) {
                return Err(VerificationError::MissingTarget(function.owner, target));
            }
        }
        IrTerminatorKind::Branch {
            condition,
            when_true,
            when_false,
        } => {
            if values.get(&condition) != Some(&IrType::Bool) {
                return Err(VerificationError::UnknownValue(function.owner, condition));
            }
            for target in [when_true, when_false] {
                if !function.blocks.contains_key(&target) {
                    return Err(VerificationError::MissingTarget(function.owner, target));
                }
            }
        }
        IrTerminatorKind::Return => {}
    }
    Ok(())
}

fn verify_binary_types(
    operator: BinaryOperator,
    left: &IrType,
    right: &IrType,
    result: &IrType,
) -> bool {
    if left != right {
        return false;
    }
    match operator {
        BinaryOperator::And | BinaryOperator::Xor | BinaryOperator::Or => {
            left.primitive_type().is_some_and(|primitive| {
                primitive == PrimitiveType::Bool || primitive.is_bit_string()
            }) && result == left
        }
        BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::Less
        | BinaryOperator::LessEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual => {
            left.primitive_type()
                .is_some_and(|primitive| match operator {
                    BinaryOperator::Equal | BinaryOperator::NotEqual => true,
                    _ => matches!(
                        primitive.category(),
                        PrimitiveCategory::SignedInteger
                            | PrimitiveCategory::UnsignedInteger
                            | PrimitiveCategory::FloatingPoint
                            | PrimitiveCategory::Character
                            | PrimitiveCategory::String
                            | PrimitiveCategory::Duration
                    ),
                })
                && result == &IrType::Bool
        }
        BinaryOperator::Modulo => {
            left.primitive_type().is_some_and(PrimitiveType::is_integer) && result == left
        }
        BinaryOperator::Multiply
        | BinaryOperator::Divide
        | BinaryOperator::Add
        | BinaryOperator::Subtract
        | BinaryOperator::Minimum
        | BinaryOperator::Maximum => is_ir_numeric(left) && result == left,
    }
}

fn is_ir_numeric(value: &IrType) -> bool {
    value
        .primitive_type()
        .is_some_and(PrimitiveType::is_numeric)
}

fn conversion_allowed(source: &IrType, destination: &IrType) -> bool {
    source
        .primitive_type()
        .zip(destination.primitive_type())
        .is_some_and(|(source, destination)| explicit_conversion_allowed(source, destination))
}

fn canonical_matches_ir(value: &CanonicalValue, data_type: &IrType) -> bool {
    value.is_compatible_with(&data_type.to_program_type())
}

fn encode_operation(hasher: &mut CanonicalHasher, operation: &IrOperation) {
    hasher.u32(operation.id.get());
    encode_operation_result(hasher, operation.result.as_ref());
    match &operation.kind {
        IrOperationKind::Constant(value) => {
            hasher.u8(1);
            encode_value(hasher, value);
        }
        IrOperationKind::LoadMember { member } => {
            hasher.u8(2);
            hasher.u128(member.get());
        }
        IrOperationKind::StoreMember { target, value } => {
            hasher.u8(3);
            hasher.u128(target.get());
            hasher.u32(value.get());
        }
        IrOperationKind::Unary { operator, operand } => {
            hasher.u8(4);
            hasher.u8(unary_tag(*operator));
            hasher.u32(operand.get());
        }
        IrOperationKind::Binary {
            operator,
            left,
            right,
        } => {
            hasher.u8(5);
            hasher.u8(binary_tag(*operator));
            hasher.u32(left.get());
            hasher.u32(right.get());
        }
        IrOperationKind::ForNextWithin {
            current,
            terminal,
            step,
            ascending,
        } => {
            hasher.u8(10);
            hasher.u32(current.get());
            hasher.u32(terminal.get());
            hasher.u32(step.get());
            hasher.bool(*ascending);
        }
        IrOperationKind::Convert {
            source,
            destination,
        } => {
            hasher.u8(6);
            hasher.u32(source.get());
            encode_type(hasher, destination);
        }
        IrOperationKind::InvokeInstruction {
            instruction,
            inputs,
            outputs,
            instance,
            activation,
        } => {
            hasher.u8(7);
            hasher.u16(instruction.0);
            encode_invocation(
                hasher,
                inputs,
                outputs,
                instance.as_ref(),
                activation.as_ref(),
            );
        }
        IrOperationKind::CallBlock {
            call_instruction,
            target,
            inputs,
            outputs,
            instance,
            activation,
        } => {
            hasher.u8(8);
            hasher.u16(call_instruction.0);
            hasher.u128(target.get());
            encode_invocation(
                hasher,
                inputs,
                outputs,
                instance.as_ref(),
                activation.as_ref(),
            );
        }
        IrOperationKind::InvocationOutput { invocation, formal } => {
            hasher.u8(9);
            hasher.u32(invocation.get());
            encode_formal(hasher, *formal);
        }
    }
    hasher.string(operation.kind.runtime_operation().0);
}

fn encode_operation_result(hasher: &mut CanonicalHasher, result: Option<&IrValue>) {
    match result {
        Some(result) => {
            hasher.bool(true);
            hasher.u32(result.id.get());
            encode_type(hasher, &result.data_type);
        }
        None => hasher.bool(false),
    }
}

fn encode_invocation(
    hasher: &mut CanonicalHasher,
    inputs: &[IrBoundInput],
    outputs: &[IrDeclaredOutput],
    instance: Option<&IrInstanceIdentity>,
    activation: Option<&IrActivation>,
) {
    hasher.u64(inputs.len() as u64);
    for input in inputs {
        encode_formal(hasher, input.formal);
        hasher.u32(input.value.get());
    }
    hasher.u64(outputs.len() as u64);
    for output in outputs {
        encode_formal(hasher, output.formal);
        encode_type(hasher, &output.data_type);
    }
    match instance {
        None => hasher.u8(0),
        Some(IrInstanceIdentity::Instruction { stable_id, kind }) => {
            hasher.u8(1);
            hasher.u128(*stable_id);
            hasher.u8(match kind {
                StateKind::Edge => 1,
                StateKind::Timer => 2,
                StateKind::Counter => 3,
            });
        }
        Some(IrInstanceIdentity::FunctionBlock(path)) => {
            hasher.u8(2);
            hasher.u128(path.root_instance_db.get());
            hasher.u64(path.multi_instance_slots.len() as u64);
            for member in &path.multi_instance_slots {
                hasher.u128(member.get());
            }
        }
    }
    match activation {
        None => hasher.bool(false),
        Some(activation) => {
            hasher.bool(true);
            hasher.u32(activation.enable.get());
            hasher.u16(activation.enable_formal.0);
            hasher.u16(activation.status_formal.0);
            hasher.bool(activation.status_when_disabled);
            hasher.u8(match activation.when_disabled {
                DisabledExecutionBehavior::DefaultOutputsNoStateChange => 1,
                DisabledExecutionBehavior::PreserveOutputsNoStateChange => 2,
                DisabledExecutionBehavior::SuppressEffects => 3,
            });
        }
    }
}

fn encode_formal(hasher: &mut CanonicalHasher, formal: IrFormalRef) {
    match formal {
        IrFormalRef::Instruction(formal) => {
            hasher.u8(1);
            hasher.u16(formal.0);
        }
        IrFormalRef::BlockMember(member) => {
            hasher.u8(2);
            hasher.u128(member.get());
        }
    }
}

fn encode_terminator(hasher: &mut CanonicalHasher, terminator: &IrTerminatorKind) {
    match terminator {
        IrTerminatorKind::Jump(target) => {
            hasher.u8(1);
            hasher.u32(target.get());
        }
        IrTerminatorKind::Branch {
            condition,
            when_true,
            when_false,
        } => {
            hasher.u8(2);
            hasher.u32(condition.get());
            hasher.u32(when_true.get());
            hasher.u32(when_false.get());
        }
        IrTerminatorKind::Return => hasher.u8(3),
    }
}

fn encode_type(hasher: &mut CanonicalHasher, value: &IrType) {
    match value {
        IrType::Bool => hasher.u8(1),
        IrType::Int => hasher.u8(2),
        IrType::DInt => hasher.u8(3),
        IrType::Real => hasher.u8(4),
        IrType::Time => hasher.u8(5),
        IrType::String { capacity } => {
            hasher.u8(6);
            hasher.u16(*capacity);
        }
        IrType::SInt => hasher.u8(7),
        IrType::LInt => hasher.u8(8),
        IrType::USInt => hasher.u8(9),
        IrType::UInt => hasher.u8(10),
        IrType::UDInt => hasher.u8(11),
        IrType::ULInt => hasher.u8(12),
        IrType::Byte => hasher.u8(13),
        IrType::Word => hasher.u8(14),
        IrType::DWord => hasher.u8(15),
        IrType::LWord => hasher.u8(16),
        IrType::LReal => hasher.u8(17),
        IrType::Char => hasher.u8(18),
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
        CanonicalValue::SInt(value) => {
            hasher.u8(7);
            hasher.i32(i32::from(*value));
        }
        CanonicalValue::LInt(value) => {
            hasher.u8(8);
            hasher.i64(*value);
        }
        CanonicalValue::USInt(value) => {
            hasher.u8(9);
            hasher.u8(*value);
        }
        CanonicalValue::UInt(value) => {
            hasher.u8(10);
            hasher.u16(*value);
        }
        CanonicalValue::UDInt(value) => {
            hasher.u8(11);
            hasher.u32(*value);
        }
        CanonicalValue::ULInt(value) => {
            hasher.u8(12);
            hasher.u64(*value);
        }
        CanonicalValue::Byte(value) => {
            hasher.u8(13);
            hasher.u8(*value);
        }
        CanonicalValue::Word(value) => {
            hasher.u8(14);
            hasher.u16(*value);
        }
        CanonicalValue::DWord(value) => {
            hasher.u8(15);
            hasher.u32(*value);
        }
        CanonicalValue::LWord(value) => {
            hasher.u8(16);
            hasher.u64(*value);
        }
        CanonicalValue::LRealBits(value) => {
            hasher.u8(17);
            hasher.u64(*value);
        }
        CanonicalValue::Char(value) => {
            hasher.u8(18);
            hasher.u8(*value);
        }
    }
}

fn encode_program_kind(hasher: &mut CanonicalHasher, value: ProgramUnitKind) {
    match value {
        ProgramUnitKind::OrganizationBlock(declaration) => {
            hasher.u8(1);
            match declaration {
                plc_program::ObDeclaration::CyclicMain => hasher.u8(1),
                plc_program::ObDeclaration::Startup => hasher.u8(2),
                plc_program::ObDeclaration::TimedCyclic {
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
        ProgramUnitKind::DataBlock(_) => hasher.u8(4),
    }
}

fn encode_site(hasher: &mut CanonicalHasher, value: SourceMapSite) {
    hasher.u128(value.function.get());
    hasher.u32(value.basic_block.get());
    match value.operation {
        Some(operation) => {
            hasher.bool(true);
            hasher.u32(operation.get());
        }
        None => hasher.bool(false),
    }
}

fn encode_anchor(hasher: &mut CanonicalHasher, anchor: &SourceAnchor) {
    hasher.u128(anchor.owner_object_id.get());
    hasher.hash(anchor.source_revision_hash);
    hasher.u8(match anchor.language {
        crate::SourceLanguage::Scl => 1,
        crate::SourceLanguage::Lad => 2,
        crate::SourceLanguage::Fbd => 3,
    });
    hasher.u32(anchor.semantic_node_id.get());
    match anchor.text_range {
        Some(range) => {
            hasher.bool(true);
            hasher.u32(range.start);
            hasher.u32(range.end);
        }
        None => hasher.bool(false),
    }
    for value in [
        anchor.network_id,
        anchor.node_id,
        anchor.port_id,
        anchor.edge_id,
        anchor.operand_id,
        anchor.call_site_id,
        anchor.state_instance_id,
    ] {
        match value {
            Some(value) => {
                hasher.bool(true);
                hasher.u128(value);
            }
            None => hasher.bool(false),
        }
    }
}

const fn unary_tag(value: UnaryOperator) -> u8 {
    match value {
        UnaryOperator::Plus => 1,
        UnaryOperator::Negate => 2,
        UnaryOperator::Not => 3,
        UnaryOperator::Absolute => 4,
    }
}

const fn binary_tag(value: BinaryOperator) -> u8 {
    match value {
        BinaryOperator::Multiply => 1,
        BinaryOperator::Divide => 2,
        BinaryOperator::Modulo => 3,
        BinaryOperator::Add => 4,
        BinaryOperator::Subtract => 5,
        BinaryOperator::Equal => 6,
        BinaryOperator::NotEqual => 7,
        BinaryOperator::Less => 8,
        BinaryOperator::LessEqual => 9,
        BinaryOperator::Greater => 10,
        BinaryOperator::GreaterEqual => 11,
        BinaryOperator::And => 12,
        BinaryOperator::Xor => 13,
        BinaryOperator::Or => 14,
        BinaryOperator::Minimum => 15,
        BinaryOperator::Maximum => 16,
    }
}

const fn probe_kind_tag(value: ProbeKind) -> u8 {
    match value {
        ProbeKind::Constant => 1,
        ProbeKind::StorageRead => 2,
        ProbeKind::StorageWrite => 3,
        ProbeKind::Expression => 4,
        ProbeKind::Branch => 5,
        ProbeKind::Return => 6,
        ProbeKind::NetworkPower => 7,
        ProbeKind::PortValue => 8,
        ProbeKind::EdgeValue => 9,
        ProbeKind::Call => 10,
        ProbeKind::State => 11,
    }
}
