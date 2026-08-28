use alloc::{collections::BTreeSet, string::String, vec::Vec};
use core::{error::Error, fmt};

use crate::{
    PRIORITY_TABLE_VERSION, RUNTIME_SEMANTICS_VERSION, SCAN_QUANTUM_MS, SCHEDULER_VERSION,
    WORK_COST_VERSION,
    hash::{Hash32, SemanticHasher},
};

macro_rules! numeric_id {
    ($name:ident, $inner:ty) => {
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub $inner);

        impl $name {
            pub const fn new(value: $inner) -> Self {
                Self(value)
            }

            pub const fn get(self) -> $inner {
                self.0
            }
        }
    };
}

numeric_id!(ChannelId, u32);
numeric_id!(MemoryId, u32);
numeric_id!(StateId, u32);
numeric_id!(TaskId, u32);
numeric_id!(BlockId, u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ValueType {
    Bool = 1,
    I32 = 2,
    I64 = 3,
    U32 = 4,
    TimeMs = 5,
}

impl ValueType {
    pub const fn canonical_default(self) -> CanonicalValue {
        match self {
            Self::Bool => CanonicalValue::Bool(false),
            Self::I32 => CanonicalValue::I32(0),
            Self::I64 => CanonicalValue::I64(0),
            Self::U32 => CanonicalValue::U32(0),
            Self::TimeMs => CanonicalValue::TimeMs(0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CanonicalValue {
    Bool(bool),
    I32(i32),
    I64(i64),
    U32(u32),
    TimeMs(u64),
}

impl CanonicalValue {
    pub const fn value_type(self) -> ValueType {
        match self {
            Self::Bool(_) => ValueType::Bool,
            Self::I32(_) => ValueType::I32,
            Self::I64(_) => ValueType::I64,
            Self::U32(_) => ValueType::U32,
            Self::TimeMs(_) => ValueType::TimeMs,
        }
    }

    pub const fn as_bool(self) -> Option<bool> {
        if let Self::Bool(value) = self {
            Some(value)
        } else {
            None
        }
    }

    pub const fn as_i32(self) -> Option<i32> {
        if let Self::I32(value) = self {
            Some(value)
        } else {
            None
        }
    }

    pub(crate) fn encode(self, hasher: &mut SemanticHasher) {
        hasher.u8(self.value_type() as u8);
        match self {
            Self::Bool(value) => hasher.bool(value),
            Self::I32(value) => hasher.i32(value),
            Self::I64(value) => hasher.i64(value),
            Self::U32(value) => hasher.u32(value),
            Self::TimeMs(value) => hasher.u64(value),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ChannelDirection {
    Input = 1,
    Output = 2,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelDefinition {
    pub id: ChannelId,
    pub direction: ChannelDirection,
    pub value_type: ValueType,
    pub canonical_default: CanonicalValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryDefinition {
    pub id: MemoryId,
    pub value_type: ValueType,
    pub loaded_start: CanonicalValue,
    pub retentive: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateStart {
    Edge { previous: bool },
    Timer { elapsed_ms: u64, output: bool },
    Counter { count: i32, previous_input: bool },
}

impl StateStart {
    pub(crate) const fn kind_tag(self) -> u8 {
        match self {
            Self::Edge { .. } => 1,
            Self::Timer { .. } => 2,
            Self::Counter { .. } => 3,
        }
    }

    pub(crate) fn encode(self, hasher: &mut SemanticHasher) {
        hasher.u8(self.kind_tag());
        match self {
            Self::Edge { previous } => hasher.bool(previous),
            Self::Timer { elapsed_ms, output } => {
                hasher.u64(elapsed_ms);
                hasher.bool(output);
            }
            Self::Counter {
                count,
                previous_input,
            } => {
                hasher.i32(count);
                hasher.bool(previous_input);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateDefinition {
    pub id: StateId,
    pub loaded_start: StateStart,
    pub retentive: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operand {
    Constant(CanonicalValue),
    Memory(MemoryId),
    Input(ChannelId),
    Output(ChannelId),
}

impl Operand {
    fn encode(self, hasher: &mut SemanticHasher) {
        match self {
            Self::Constant(value) => {
                hasher.u8(1);
                value.encode(hasher);
            }
            Self::Memory(id) => {
                hasher.u8(2);
                hasher.u32(id.0);
            }
            Self::Input(id) => {
                hasher.u8(3);
                hasher.u32(id.0);
            }
            Self::Output(id) => {
                hasher.u8(4);
                hasher.u32(id.0);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operation {
    Noop,
    SetMemory {
        target: MemoryId,
        value: CanonicalValue,
    },
    Copy {
        source: Operand,
        target: MemoryId,
    },
    AddI32 {
        left: Operand,
        right: Operand,
        target: MemoryId,
    },
    DivideI32 {
        numerator: Operand,
        denominator: Operand,
        target: MemoryId,
    },
    LoadInput {
        channel: ChannelId,
        target: MemoryId,
    },
    StoreOutput {
        source: Operand,
        channel: ChannelId,
    },
    RisingEdge {
        source: Operand,
        state: StateId,
        target: MemoryId,
    },
    FallingEdge {
        source: Operand,
        state: StateId,
        target: MemoryId,
    },
    TimerOnDelay {
        input: Operand,
        preset_ms: u64,
        state: StateId,
        output: MemoryId,
        elapsed: MemoryId,
    },
    CounterUp {
        input: Operand,
        reset: Operand,
        preset: i32,
        state: StateId,
        output: MemoryId,
        current: MemoryId,
    },
}

impl Operation {
    pub const fn work_units(&self) -> u32 {
        1
    }

    fn encode(&self, hasher: &mut SemanticHasher) {
        match self {
            Self::Noop => hasher.u8(0),
            Self::SetMemory { target, value } => {
                hasher.u8(1);
                hasher.u32(target.0);
                value.encode(hasher);
            }
            Self::Copy { source, target } => {
                hasher.u8(2);
                source.encode(hasher);
                hasher.u32(target.0);
            }
            Self::AddI32 {
                left,
                right,
                target,
            } => {
                hasher.u8(3);
                left.encode(hasher);
                right.encode(hasher);
                hasher.u32(target.0);
            }
            Self::DivideI32 {
                numerator,
                denominator,
                target,
            } => {
                hasher.u8(4);
                numerator.encode(hasher);
                denominator.encode(hasher);
                hasher.u32(target.0);
            }
            Self::LoadInput { channel, target } => {
                hasher.u8(5);
                hasher.u32(channel.0);
                hasher.u32(target.0);
            }
            Self::StoreOutput { source, channel } => {
                hasher.u8(6);
                source.encode(hasher);
                hasher.u32(channel.0);
            }
            Self::RisingEdge {
                source,
                state,
                target,
            } => {
                hasher.u8(7);
                source.encode(hasher);
                hasher.u32(state.0);
                hasher.u32(target.0);
            }
            Self::FallingEdge {
                source,
                state,
                target,
            } => {
                hasher.u8(8);
                source.encode(hasher);
                hasher.u32(state.0);
                hasher.u32(target.0);
            }
            Self::TimerOnDelay {
                input,
                preset_ms,
                state,
                output,
                elapsed,
            } => {
                hasher.u8(9);
                input.encode(hasher);
                hasher.u64(*preset_ms);
                hasher.u32(state.0);
                hasher.u32(output.0);
                hasher.u32(elapsed.0);
            }
            Self::CounterUp {
                input,
                reset,
                preset,
                state,
                output,
                current,
            } => {
                hasher.u8(10);
                input.encode(hasher);
                reset.encode(hasher);
                hasher.i32(*preset);
                hasher.u32(state.0);
                hasher.u32(output.0);
                hasher.u32(current.0);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instruction {
    pub operation_id: u32,
    pub source_identity: u128,
    operation: Operation,
    work_units: u32,
}

impl Instruction {
    pub fn new(operation_id: u32, source_identity: u128, operation: Operation) -> Self {
        let work_units = operation.work_units();
        Self {
            operation_id,
            source_identity,
            operation,
            work_units,
        }
    }

    pub const fn operation(&self) -> &Operation {
        &self.operation
    }

    pub const fn work_units(&self) -> u32 {
        self.work_units
    }

    fn encode(&self, hasher: &mut SemanticHasher) {
        hasher.u32(self.operation_id);
        hasher.u128(self.source_identity);
        hasher.u32(self.work_units);
        self.operation.encode(hasher);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgramBlock {
    pub id: BlockId,
    pub instructions: Vec<Instruction>,
}

impl ProgramBlock {
    fn encode(&self, hasher: &mut SemanticHasher) {
        hasher.u32(self.id.0);
        hasher.u64(self.instructions.len() as u64);
        for instruction in &self.instructions {
            instruction.encode(hasher);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimedTask {
    pub id: TaskId,
    pub first_due_ms: u64,
    pub period_ms: u64,
    pub semantic_order: u32,
    pub block: ProgramBlock,
}

impl TimedTask {
    fn encode(&self, hasher: &mut SemanticHasher) {
        hasher.u32(self.id.0);
        hasher.u64(self.first_due_ms);
        hasher.u64(self.period_ms);
        hasher.u32(self.semantic_order);
        self.block.encode(hasher);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgramImage {
    pub startup: Option<ProgramBlock>,
    pub timed: Vec<TimedTask>,
    pub cyclic: ProgramBlock,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactSpec {
    pub schema_version: u32,
    pub runtime_version: String,
    pub scheduler_version: String,
    pub priority_table_version: String,
    pub work_cost_version: String,
    pub profile_fingerprint: Hash32,
    pub memory: Vec<MemoryDefinition>,
    pub channels: Vec<ChannelDefinition>,
    pub states: Vec<StateDefinition>,
    pub program: ProgramImage,
}

impl ArtifactSpec {
    pub fn edu21(
        profile_fingerprint: Hash32,
        memory: Vec<MemoryDefinition>,
        channels: Vec<ChannelDefinition>,
        states: Vec<StateDefinition>,
        program: ProgramImage,
    ) -> Self {
        Self {
            schema_version: 1,
            runtime_version: RUNTIME_SEMANTICS_VERSION.into(),
            scheduler_version: SCHEDULER_VERSION.into(),
            priority_table_version: PRIORITY_TABLE_VERSION.into(),
            work_cost_version: WORK_COST_VERSION.into(),
            profile_fingerprint,
            memory,
            channels,
            states,
            program,
        }
    }

    fn normalize(&mut self) {
        self.memory.sort_by_key(|definition| definition.id);
        self.channels.sort_by_key(|definition| definition.id);
        self.states.sort_by_key(|definition| definition.id);
        self.program.timed.sort_by_key(|task| task.id);
    }

    pub fn canonical_fingerprint(&self) -> Hash32 {
        let mut hasher = SemanticHasher::new("PES-ARTIFACT-1");
        hasher.u32(self.schema_version);
        hasher.string(&self.runtime_version);
        hasher.string(&self.scheduler_version);
        hasher.string(&self.priority_table_version);
        hasher.string(&self.work_cost_version);
        hasher.hash(self.profile_fingerprint);

        hasher.u64(self.memory.len() as u64);
        for definition in &self.memory {
            hasher.u32(definition.id.0);
            hasher.u8(definition.value_type as u8);
            definition.loaded_start.encode(&mut hasher);
            hasher.bool(definition.retentive);
        }

        hasher.u64(self.channels.len() as u64);
        for definition in &self.channels {
            hasher.u32(definition.id.0);
            hasher.u8(definition.direction as u8);
            hasher.u8(definition.value_type as u8);
            definition.canonical_default.encode(&mut hasher);
        }

        hasher.u64(self.states.len() as u64);
        for definition in &self.states {
            hasher.u32(definition.id.0);
            definition.loaded_start.encode(&mut hasher);
            hasher.bool(definition.retentive);
        }

        match &self.program.startup {
            Some(block) => {
                hasher.bool(true);
                block.encode(&mut hasher);
            }
            None => hasher.bool(false),
        }
        hasher.u64(self.program.timed.len() as u64);
        for task in &self.program.timed {
            task.encode(&mut hasher);
        }
        self.program.cyclic.encode(&mut hasher);
        hasher.finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactPackage {
    spec: ArtifactSpec,
    declared_fingerprint: Hash32,
    ir_verified: bool,
}

impl ArtifactPackage {
    pub fn seal_verified(mut spec: ArtifactSpec) -> Result<Self, ArtifactError> {
        spec.normalize();
        validate_spec(&spec)?;
        let declared_fingerprint = spec.canonical_fingerprint();
        Ok(Self {
            spec,
            declared_fingerprint,
            ir_verified: true,
        })
    }

    /// Reconstructs an artifact crossing a typed trust boundary. Validation and
    /// fingerprint verification still occur when it is accepted by a runtime.
    pub fn from_untrusted_package(
        spec: ArtifactSpec,
        declared_fingerprint: Hash32,
        ir_verified: bool,
    ) -> Self {
        Self {
            spec,
            declared_fingerprint,
            ir_verified,
        }
    }

    pub const fn spec(&self) -> &ArtifactSpec {
        &self.spec
    }

    pub const fn fingerprint(&self) -> Hash32 {
        self.declared_fingerprint
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedArtifact {
    spec: ArtifactSpec,
    fingerprint: Hash32,
}

impl VerifiedArtifact {
    pub fn accept(package: &ArtifactPackage) -> Result<Self, ArtifactError> {
        if !package.ir_verified {
            return Err(ArtifactError::IrNotVerified);
        }
        validate_spec(&package.spec)?;
        let actual = package.spec.canonical_fingerprint();
        if actual != package.declared_fingerprint {
            return Err(ArtifactError::FingerprintMismatch {
                declared: package.declared_fingerprint,
                actual,
            });
        }
        Ok(Self {
            spec: package.spec.clone(),
            fingerprint: actual,
        })
    }

    pub const fn spec(&self) -> &ArtifactSpec {
        &self.spec
    }

    pub const fn fingerprint(&self) -> Hash32 {
        self.fingerprint
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArtifactError {
    IrNotVerified,
    FingerprintMismatch { declared: Hash32, actual: Hash32 },
    UnsupportedSchema(u32),
    IncompatibleRuntimeVersion,
    IncompatibleSchedulerVersion,
    IncompatiblePriorityVersion,
    IncompatibleWorkCostVersion,
    DuplicateOrUnorderedMemory(MemoryId),
    DuplicateOrUnorderedChannel(ChannelId),
    DuplicateOrUnorderedState(StateId),
    DuplicateTask(TaskId),
    DuplicateBlock(BlockId),
    DuplicateOperation { block: BlockId, operation_id: u32 },
    DuplicateStateUse(StateId),
    TypeMismatch,
    UnknownMemory(MemoryId),
    UnknownChannel(ChannelId),
    WrongChannelDirection(ChannelId),
    UnknownState(StateId),
    WrongStateKind(StateId),
    InvalidTimedSchedule(TaskId),
    InvalidTimerPreset,
    InvalidInstructionCost(u32),
}

impl fmt::Display for ArtifactError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "artifact rejected: {self:?}")
    }
}

impl Error for ArtifactError {}

fn validate_spec(spec: &ArtifactSpec) -> Result<(), ArtifactError> {
    if spec.schema_version != 1 {
        return Err(ArtifactError::UnsupportedSchema(spec.schema_version));
    }
    if spec.runtime_version != RUNTIME_SEMANTICS_VERSION {
        return Err(ArtifactError::IncompatibleRuntimeVersion);
    }
    if spec.scheduler_version != SCHEDULER_VERSION {
        return Err(ArtifactError::IncompatibleSchedulerVersion);
    }
    if spec.priority_table_version != PRIORITY_TABLE_VERSION {
        return Err(ArtifactError::IncompatiblePriorityVersion);
    }
    if spec.work_cost_version != WORK_COST_VERSION {
        return Err(ArtifactError::IncompatibleWorkCostVersion);
    }

    validate_strict_ids(
        &spec.memory,
        |entry| entry.id,
        ArtifactError::DuplicateOrUnorderedMemory,
    )?;
    validate_strict_ids(
        &spec.channels,
        |entry| entry.id,
        ArtifactError::DuplicateOrUnorderedChannel,
    )?;
    validate_strict_ids(
        &spec.states,
        |entry| entry.id,
        ArtifactError::DuplicateOrUnorderedState,
    )?;

    for definition in &spec.memory {
        if definition.loaded_start.value_type() != definition.value_type {
            return Err(ArtifactError::TypeMismatch);
        }
    }
    for definition in &spec.channels {
        if definition.canonical_default.value_type() != definition.value_type {
            return Err(ArtifactError::TypeMismatch);
        }
    }

    let mut task_ids = Vec::new();
    let mut block_ids = Vec::new();
    if let Some(startup) = &spec.program.startup {
        block_ids.push(startup.id);
        validate_block(spec, startup)?;
    }
    for task in &spec.program.timed {
        if task.period_ms == 0
            || task.period_ms % SCAN_QUANTUM_MS != 0
            || task.first_due_ms % SCAN_QUANTUM_MS != 0
        {
            return Err(ArtifactError::InvalidTimedSchedule(task.id));
        }
        if task_ids.contains(&task.id) {
            return Err(ArtifactError::DuplicateTask(task.id));
        }
        task_ids.push(task.id);
        block_ids.push(task.block.id);
        validate_block(spec, &task.block)?;
    }
    block_ids.push(spec.program.cyclic.id);
    validate_block(spec, &spec.program.cyclic)?;
    block_ids.sort_unstable();
    for pair in block_ids.windows(2) {
        if pair[0] == pair[1] {
            return Err(ArtifactError::DuplicateBlock(pair[0]));
        }
    }

    let mut used_state_ids = BTreeSet::new();
    let blocks = spec
        .program
        .startup
        .iter()
        .chain(spec.program.timed.iter().map(|task| &task.block))
        .chain(core::iter::once(&spec.program.cyclic));
    for block in blocks {
        for instruction in &block.instructions {
            let state_id = match instruction.operation() {
                Operation::RisingEdge { state, .. }
                | Operation::FallingEdge { state, .. }
                | Operation::TimerOnDelay { state, .. }
                | Operation::CounterUp { state, .. } => Some(*state),
                _ => None,
            };
            if let Some(state_id) = state_id
                && !used_state_ids.insert(state_id)
            {
                return Err(ArtifactError::DuplicateStateUse(state_id));
            }
        }
    }
    Ok(())
}

fn validate_strict_ids<T, I: Copy + Ord>(
    values: &[T],
    id: impl Fn(&T) -> I,
    error: impl Fn(I) -> ArtifactError,
) -> Result<(), ArtifactError> {
    for pair in values.windows(2) {
        if id(&pair[0]) >= id(&pair[1]) {
            return Err(error(id(&pair[1])));
        }
    }
    Ok(())
}

fn validate_block(spec: &ArtifactSpec, block: &ProgramBlock) -> Result<(), ArtifactError> {
    let mut operation_ids = BTreeSet::new();
    for instruction in &block.instructions {
        if !operation_ids.insert(instruction.operation_id) {
            return Err(ArtifactError::DuplicateOperation {
                block: block.id,
                operation_id: instruction.operation_id,
            });
        }
        if instruction.work_units != instruction.operation.work_units() {
            return Err(ArtifactError::InvalidInstructionCost(
                instruction.operation_id,
            ));
        }
        validate_operation(spec, &instruction.operation)?;
    }
    Ok(())
}

fn validate_operation(spec: &ArtifactSpec, operation: &Operation) -> Result<(), ArtifactError> {
    let memory_type = |id: MemoryId| {
        spec.memory
            .binary_search_by_key(&id, |definition| definition.id)
            .ok()
            .map(|index| spec.memory[index].value_type)
            .ok_or(ArtifactError::UnknownMemory(id))
    };
    let channel = |id: ChannelId| {
        spec.channels
            .binary_search_by_key(&id, |definition| definition.id)
            .ok()
            .map(|index| &spec.channels[index])
            .ok_or(ArtifactError::UnknownChannel(id))
    };
    let state = |id: StateId| {
        spec.states
            .binary_search_by_key(&id, |definition| definition.id)
            .ok()
            .map(|index| &spec.states[index])
            .ok_or(ArtifactError::UnknownState(id))
    };
    let operand_type = |operand: Operand| -> Result<ValueType, ArtifactError> {
        match operand {
            Operand::Constant(value) => Ok(value.value_type()),
            Operand::Memory(id) => memory_type(id),
            Operand::Input(id) => {
                let definition = channel(id)?;
                if definition.direction != ChannelDirection::Input {
                    return Err(ArtifactError::WrongChannelDirection(id));
                }
                Ok(definition.value_type)
            }
            Operand::Output(id) => {
                let definition = channel(id)?;
                if definition.direction != ChannelDirection::Output {
                    return Err(ArtifactError::WrongChannelDirection(id));
                }
                Ok(definition.value_type)
            }
        }
    };
    let same = |actual: ValueType, expected: ValueType| {
        if actual == expected {
            Ok(())
        } else {
            Err(ArtifactError::TypeMismatch)
        }
    };

    match operation {
        Operation::Noop => Ok(()),
        Operation::SetMemory { target, value } => same(value.value_type(), memory_type(*target)?),
        Operation::Copy { source, target } => same(operand_type(*source)?, memory_type(*target)?),
        Operation::AddI32 {
            left,
            right,
            target,
        }
        | Operation::DivideI32 {
            numerator: left,
            denominator: right,
            target,
        } => {
            same(operand_type(*left)?, ValueType::I32)?;
            same(operand_type(*right)?, ValueType::I32)?;
            same(memory_type(*target)?, ValueType::I32)
        }
        Operation::LoadInput {
            channel: id,
            target,
        } => {
            let definition = channel(*id)?;
            if definition.direction != ChannelDirection::Input {
                return Err(ArtifactError::WrongChannelDirection(*id));
            }
            same(definition.value_type, memory_type(*target)?)
        }
        Operation::StoreOutput {
            source,
            channel: id,
        } => {
            let definition = channel(*id)?;
            if definition.direction != ChannelDirection::Output {
                return Err(ArtifactError::WrongChannelDirection(*id));
            }
            same(operand_type(*source)?, definition.value_type)
        }
        Operation::RisingEdge {
            source,
            state: id,
            target,
        }
        | Operation::FallingEdge {
            source,
            state: id,
            target,
        } => {
            same(operand_type(*source)?, ValueType::Bool)?;
            same(memory_type(*target)?, ValueType::Bool)?;
            if !matches!(state(*id)?.loaded_start, StateStart::Edge { .. }) {
                return Err(ArtifactError::WrongStateKind(*id));
            }
            Ok(())
        }
        Operation::TimerOnDelay {
            input,
            preset_ms,
            state: id,
            output,
            elapsed,
        } => {
            same(operand_type(*input)?, ValueType::Bool)?;
            same(memory_type(*output)?, ValueType::Bool)?;
            same(memory_type(*elapsed)?, ValueType::TimeMs)?;
            if *preset_ms == 0 || *preset_ms % SCAN_QUANTUM_MS != 0 {
                return Err(ArtifactError::InvalidTimerPreset);
            }
            if !matches!(state(*id)?.loaded_start, StateStart::Timer { .. }) {
                return Err(ArtifactError::WrongStateKind(*id));
            }
            Ok(())
        }
        Operation::CounterUp {
            input,
            reset,
            state: id,
            output,
            current,
            ..
        } => {
            same(operand_type(*input)?, ValueType::Bool)?;
            same(operand_type(*reset)?, ValueType::Bool)?;
            same(memory_type(*output)?, ValueType::Bool)?;
            same(memory_type(*current)?, ValueType::I32)?;
            if !matches!(state(*id)?.loaded_start, StateStart::Counter { .. }) {
                return Err(ArtifactError::WrongStateKind(*id));
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    fn empty_spec() -> ArtifactSpec {
        ArtifactSpec::edu21(
            Hash32::ZERO,
            vec![],
            vec![],
            vec![],
            ProgramImage {
                startup: None,
                timed: vec![],
                cyclic: ProgramBlock {
                    id: BlockId(1),
                    instructions: vec![],
                },
            },
        )
    }

    #[test]
    fn sealed_package_is_content_addressed_and_untrusted_bytes_are_rechecked() {
        let package = ArtifactPackage::seal_verified(empty_spec()).unwrap();
        assert_eq!(
            package.fingerprint(),
            package.spec().canonical_fingerprint()
        );
        assert!(VerifiedArtifact::accept(&package).is_ok());

        let mut changed = package.spec().clone();
        changed.schema_version = 2;
        let transported =
            ArtifactPackage::from_untrusted_package(changed, package.fingerprint(), true);
        assert!(matches!(
            VerifiedArtifact::accept(&transported),
            Err(ArtifactError::UnsupportedSchema(2))
        ));
    }
}
