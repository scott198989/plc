use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::String,
    vec::Vec,
};

use plc_program::{
    BlockId, CanonicalValue, ControllerProgram, DataType, InterfaceMemberId, InterfaceRole,
    ProgramUnitKind,
};
use plc_runtime::Hash32;

use crate::{
    IrBasicBlockId, IrOperationId, IrValueId, ProbeId, SourceAnchor, SourceMapId, TYPED_IR_VERSION,
    hash::CanonicalHasher,
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IrType {
    Bool,
    Int,
    DInt,
    Real,
    Time,
    String { capacity: u16 },
}

impl IrType {
    pub(crate) fn from_program_type(value: &DataType) -> Option<Self> {
        match value {
            DataType::Bool => Some(Self::Bool),
            DataType::Int => Some(Self::Int),
            DataType::DInt => Some(Self::DInt),
            DataType::Real => Some(Self::Real),
            DataType::Time => Some(Self::Time),
            DataType::String { capacity } => Some(Self::String {
                capacity: *capacity,
            }),
            DataType::Named(_) | DataType::BlockInstance(_) | DataType::InstructionState(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeOperationId(pub &'static str);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UnaryOperator {
    Plus,
    Negate,
    Not,
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrValue {
    pub id: IrValueId,
    pub data_type: IrType,
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
    Convert {
        source: IrValueId,
        destination: IrType,
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
            },
            Self::Convert { .. } => RuntimeOperationId("EDU.RT.CONVERT_CHECKED.v1"),
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
    UnknownMember(BlockId, InterfaceMemberId),
    ReadOnlyStore(BlockId, InterfaceMemberId),
    TypeMismatch(BlockId, IrOperationId),
    InvalidConversion(BlockId, IrOperationId),
    MissingResult(BlockId, IrOperationId),
    UnexpectedResult(BlockId, IrOperationId),
    MissingTarget(BlockId, IrBasicBlockId),
    MissingSourceMap(SourceMapId),
    SourceMapKeyMismatch(SourceMapId),
    SourceMapSiteMismatch(SourceMapId),
    EmptySourceMap(SourceMapId),
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
        let mut value_ids = BTreeSet::new();
        for (&block_key, block) in &function.blocks {
            if block_key != block.id {
                return Err(VerificationError::BlockKeyMismatch(
                    function.owner,
                    block_key,
                ));
            }
            let mut value_types = BTreeMap::<IrValueId, IrType>::new();
            for operation in &block.operations {
                if !operation_ids.insert(operation.id) {
                    return Err(VerificationError::DuplicateOperation(
                        function.owner,
                        operation.id,
                    ));
                }
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
                verify_operation(operation, source_block, &value_types, function.owner)?;
                if let Some(result) = &operation.result {
                    if !value_ids.insert(result.id) {
                        return Err(VerificationError::DuplicateValue(function.owner, result.id));
                    }
                    value_types.insert(result.id, result.data_type.clone());
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

fn verify_operation(
    operation: &IrOperation,
    source_block: &plc_program::ProgramBlock,
    values: &BTreeMap<IrValueId, IrType>,
    function: BlockId,
) -> Result<(), VerificationError> {
    let result_type = operation.result.as_ref().map(|value| &value.data_type);
    match &operation.kind {
        IrOperationKind::Constant(value) => {
            let result =
                result_type.ok_or(VerificationError::MissingResult(function, operation.id))?;
            if !canonical_matches_ir(value, result) {
                return Err(VerificationError::TypeMismatch(function, operation.id));
            }
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
        }
        IrOperationKind::Unary { operator, operand } => {
            let operand_type = values
                .get(operand)
                .ok_or(VerificationError::UnknownValue(function, *operand))?;
            let result =
                result_type.ok_or(VerificationError::MissingResult(function, operation.id))?;
            let valid = match operator {
                UnaryOperator::Not => operand_type == &IrType::Bool && result == &IrType::Bool,
                UnaryOperator::Plus | UnaryOperator::Negate => {
                    is_ir_numeric(operand_type) && result == operand_type
                }
            };
            if !valid {
                return Err(VerificationError::TypeMismatch(function, operation.id));
            }
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
        }
    }
    if operation.kind.runtime_operation().0.is_empty() {
        return Err(VerificationError::TypeMismatch(function, operation.id));
    }
    Ok(())
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
            left == &IrType::Bool && result == &IrType::Bool
        }
        BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::Less
        | BinaryOperator::LessEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual => result == &IrType::Bool,
        BinaryOperator::Modulo => matches!(left, IrType::Int | IrType::DInt) && result == left,
        BinaryOperator::Multiply
        | BinaryOperator::Divide
        | BinaryOperator::Add
        | BinaryOperator::Subtract => is_ir_numeric(left) && result == left,
    }
}

fn is_ir_numeric(value: &IrType) -> bool {
    matches!(value, IrType::Int | IrType::DInt | IrType::Real)
}

fn conversion_allowed(source: &IrType, destination: &IrType) -> bool {
    source == destination
        || matches!(
            (source, destination),
            (IrType::Int, IrType::DInt) | (IrType::Int | IrType::DInt, IrType::Real)
        )
}

fn canonical_matches_ir(value: &CanonicalValue, data_type: &IrType) -> bool {
    match (value, data_type) {
        (CanonicalValue::Bool(_), IrType::Bool)
        | (CanonicalValue::Int(_), IrType::Int)
        | (CanonicalValue::DInt(_), IrType::DInt)
        | (CanonicalValue::RealBits(_), IrType::Real)
        | (CanonicalValue::TimeMilliseconds(_), IrType::Time) => true,
        (CanonicalValue::StringBytes(bytes), IrType::String { capacity }) => {
            bytes.len() <= usize::from(*capacity)
        }
        _ => false,
    }
}

fn encode_operation(hasher: &mut CanonicalHasher, operation: &IrOperation) {
    hasher.u32(operation.id.get());
    match &operation.result {
        Some(result) => {
            hasher.bool(true);
            hasher.u32(result.id.get());
            encode_type(hasher, &result.data_type);
        }
        None => hasher.bool(false),
    }
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
        IrOperationKind::Convert {
            source,
            destination,
        } => {
            hasher.u8(6);
            hasher.u32(source.get());
            encode_type(hasher, destination);
        }
    }
    hasher.string(operation.kind.runtime_operation().0);
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
    });
    hasher.u32(anchor.semantic_node_id.get());
    hasher.u32(anchor.text_range.start);
    hasher.u32(anchor.text_range.end);
}

const fn unary_tag(value: UnaryOperator) -> u8 {
    match value {
        UnaryOperator::Plus => 1,
        UnaryOperator::Negate => 2,
        UnaryOperator::Not => 3,
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
    }
}
