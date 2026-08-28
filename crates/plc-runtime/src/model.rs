use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::String,
    vec::Vec,
};
use core::{error::Error, fmt};
use plc_types::{
    AggregateLimits, CanonicalF32, CanonicalF64, CanonicalType, PlcValue, PrimitiveType,
    ScalarTypeError, ScalarValue, TypedScalar, explicit_conversion_allowed,
};

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
    I8 = 6,
    I16 = 7,
    U8 = 8,
    U16 = 9,
    U64 = 10,
    Bits8 = 11,
    Bits16 = 12,
    Bits32 = 13,
    Bits64 = 14,
    F32 = 15,
    F64 = 16,
    Char = 17,
}

impl ValueType {
    pub const fn canonical_default(self) -> CanonicalValue {
        match self {
            Self::Bool => CanonicalValue::Bool(false),
            Self::I32 => CanonicalValue::I32(0),
            Self::I64 => CanonicalValue::I64(0),
            Self::U32 => CanonicalValue::U32(0),
            Self::TimeMs => CanonicalValue::TimeMs(0),
            Self::I8 => CanonicalValue::I8(0),
            Self::I16 => CanonicalValue::I16(0),
            Self::U8 => CanonicalValue::U8(0),
            Self::U16 => CanonicalValue::U16(0),
            Self::U64 => CanonicalValue::U64(0),
            Self::Bits8 => CanonicalValue::Bits8(0),
            Self::Bits16 => CanonicalValue::Bits16(0),
            Self::Bits32 => CanonicalValue::Bits32(0),
            Self::Bits64 => CanonicalValue::Bits64(0),
            Self::F32 => CanonicalValue::F32(CanonicalF32::from_bits(0)),
            Self::F64 => CanonicalValue::F64(CanonicalF64::from_bits(0)),
            Self::Char => CanonicalValue::Char(0),
        }
    }

    #[must_use]
    pub const fn primitive_type(self) -> PrimitiveType {
        match self {
            Self::Bool => PrimitiveType::Bool,
            Self::I8 => PrimitiveType::Sint,
            Self::I16 => PrimitiveType::Int,
            Self::I32 => PrimitiveType::Dint,
            Self::I64 => PrimitiveType::Lint,
            Self::U8 => PrimitiveType::Usint,
            Self::U16 => PrimitiveType::Uint,
            Self::U32 => PrimitiveType::Udint,
            Self::U64 => PrimitiveType::Ulint,
            Self::Bits8 => PrimitiveType::Byte,
            Self::Bits16 => PrimitiveType::Word,
            Self::Bits32 => PrimitiveType::Dword,
            Self::Bits64 => PrimitiveType::Lword,
            Self::F32 => PrimitiveType::Real,
            Self::F64 => PrimitiveType::Lreal,
            Self::Char => PrimitiveType::Char,
            Self::TimeMs => PrimitiveType::Time,
        }
    }

    #[must_use]
    pub const fn from_primitive(value: PrimitiveType) -> Option<Self> {
        match value {
            PrimitiveType::Bool => Some(Self::Bool),
            PrimitiveType::Sint => Some(Self::I8),
            PrimitiveType::Int => Some(Self::I16),
            PrimitiveType::Dint => Some(Self::I32),
            PrimitiveType::Lint => Some(Self::I64),
            PrimitiveType::Usint => Some(Self::U8),
            PrimitiveType::Uint => Some(Self::U16),
            PrimitiveType::Udint => Some(Self::U32),
            PrimitiveType::Ulint => Some(Self::U64),
            PrimitiveType::Byte => Some(Self::Bits8),
            PrimitiveType::Word => Some(Self::Bits16),
            PrimitiveType::Dword => Some(Self::Bits32),
            PrimitiveType::Lword => Some(Self::Bits64),
            PrimitiveType::Real => Some(Self::F32),
            PrimitiveType::Lreal => Some(Self::F64),
            PrimitiveType::Char => Some(Self::Char),
            PrimitiveType::Time => Some(Self::TimeMs),
            PrimitiveType::String(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CanonicalValue {
    Bool(bool),
    I32(i32),
    I64(i64),
    U32(u32),
    TimeMs(i64),
    I8(i8),
    I16(i16),
    U8(u8),
    U16(u16),
    U64(u64),
    Bits8(u8),
    Bits16(u16),
    Bits32(u32),
    Bits64(u64),
    F32(CanonicalF32),
    F64(CanonicalF64),
    Char(u8),
}

impl CanonicalValue {
    pub const fn value_type(self) -> ValueType {
        match self {
            Self::Bool(_) => ValueType::Bool,
            Self::I32(_) => ValueType::I32,
            Self::I64(_) => ValueType::I64,
            Self::U32(_) => ValueType::U32,
            Self::TimeMs(_) => ValueType::TimeMs,
            Self::I8(_) => ValueType::I8,
            Self::I16(_) => ValueType::I16,
            Self::U8(_) => ValueType::U8,
            Self::U16(_) => ValueType::U16,
            Self::U64(_) => ValueType::U64,
            Self::Bits8(_) => ValueType::Bits8,
            Self::Bits16(_) => ValueType::Bits16,
            Self::Bits32(_) => ValueType::Bits32,
            Self::Bits64(_) => ValueType::Bits64,
            Self::F32(_) => ValueType::F32,
            Self::F64(_) => ValueType::F64,
            Self::Char(_) => ValueType::Char,
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
            Self::TimeMs(value) => hasher.i64(value),
            Self::I8(value) => hasher.i32(i32::from(value)),
            Self::I16(value) => hasher.i32(i32::from(value)),
            Self::U8(value) => hasher.u32(u32::from(value)),
            Self::U16(value) => hasher.u32(u32::from(value)),
            Self::U64(value) => hasher.u64(value),
            Self::Bits8(value) => hasher.u32(u32::from(value)),
            Self::Bits16(value) => hasher.u32(u32::from(value)),
            Self::Bits32(value) => hasher.u32(value),
            Self::Bits64(value) => hasher.u64(value),
            Self::F32(value) => hasher.u32(value.bits()),
            Self::F64(value) => hasher.u64(value.bits()),
            Self::Char(value) => hasher.u8(value),
        }
    }

    pub fn typed_scalar(self) -> Result<TypedScalar, ScalarTypeError> {
        let data_type = self.value_type().primitive_type();
        let value = match self {
            Self::Bool(value) => ScalarValue::Bool(value),
            Self::I8(value) => ScalarValue::Signed(i64::from(value)),
            Self::I16(value) => ScalarValue::Signed(i64::from(value)),
            Self::I32(value) => ScalarValue::Signed(i64::from(value)),
            Self::I64(value) => ScalarValue::Signed(value),
            Self::U8(value) => ScalarValue::Unsigned(u64::from(value)),
            Self::U16(value) => ScalarValue::Unsigned(u64::from(value)),
            Self::U32(value) => ScalarValue::Unsigned(u64::from(value)),
            Self::U64(value) => ScalarValue::Unsigned(value),
            Self::Bits8(value) => ScalarValue::BitString(u64::from(value)),
            Self::Bits16(value) => ScalarValue::BitString(u64::from(value)),
            Self::Bits32(value) => ScalarValue::BitString(u64::from(value)),
            Self::Bits64(value) => ScalarValue::BitString(value),
            Self::F32(value) => ScalarValue::Real(value),
            Self::F64(value) => ScalarValue::Lreal(value),
            Self::Char(value) => ScalarValue::Char(value),
            Self::TimeMs(value) => ScalarValue::Time(value),
        };
        TypedScalar::new(data_type, value)
    }

    pub fn from_typed_scalar(value: TypedScalar) -> Result<Self, ScalarTypeError> {
        let data_type = value.data_type();
        match (data_type, value.into_value()) {
            (PrimitiveType::Bool, ScalarValue::Bool(value)) => Ok(Self::Bool(value)),
            (PrimitiveType::Sint, ScalarValue::Signed(value)) => i8::try_from(value)
                .map(Self::I8)
                .map_err(|_| ScalarTypeError::ValueDoesNotMatchType),
            (PrimitiveType::Int, ScalarValue::Signed(value)) => i16::try_from(value)
                .map(Self::I16)
                .map_err(|_| ScalarTypeError::ValueDoesNotMatchType),
            (PrimitiveType::Dint, ScalarValue::Signed(value)) => i32::try_from(value)
                .map(Self::I32)
                .map_err(|_| ScalarTypeError::ValueDoesNotMatchType),
            (PrimitiveType::Lint, ScalarValue::Signed(value)) => Ok(Self::I64(value)),
            (PrimitiveType::Usint, ScalarValue::Unsigned(value)) => u8::try_from(value)
                .map(Self::U8)
                .map_err(|_| ScalarTypeError::ValueDoesNotMatchType),
            (PrimitiveType::Uint, ScalarValue::Unsigned(value)) => u16::try_from(value)
                .map(Self::U16)
                .map_err(|_| ScalarTypeError::ValueDoesNotMatchType),
            (PrimitiveType::Udint, ScalarValue::Unsigned(value)) => u32::try_from(value)
                .map(Self::U32)
                .map_err(|_| ScalarTypeError::ValueDoesNotMatchType),
            (PrimitiveType::Ulint, ScalarValue::Unsigned(value)) => Ok(Self::U64(value)),
            (PrimitiveType::Byte, ScalarValue::BitString(value)) => u8::try_from(value)
                .map(Self::Bits8)
                .map_err(|_| ScalarTypeError::ValueDoesNotMatchType),
            (PrimitiveType::Word, ScalarValue::BitString(value)) => u16::try_from(value)
                .map(Self::Bits16)
                .map_err(|_| ScalarTypeError::ValueDoesNotMatchType),
            (PrimitiveType::Dword, ScalarValue::BitString(value)) => u32::try_from(value)
                .map(Self::Bits32)
                .map_err(|_| ScalarTypeError::ValueDoesNotMatchType),
            (PrimitiveType::Lword, ScalarValue::BitString(value)) => Ok(Self::Bits64(value)),
            (PrimitiveType::Real, ScalarValue::Real(value)) => Ok(Self::F32(value)),
            (PrimitiveType::Lreal, ScalarValue::Lreal(value)) => Ok(Self::F64(value)),
            (PrimitiveType::Char, ScalarValue::Char(value)) => Ok(Self::Char(value)),
            (PrimitiveType::Time, ScalarValue::Time(value)) => Ok(Self::TimeMs(value)),
            _ => Err(ScalarTypeError::ValueDoesNotMatchType),
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

/// One aggregate memory cell whose shape and value are owned by the shared
/// `plc-types` canonical authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AggregateMemoryDefinition {
    pub id: MemoryId,
    pub data_type: CanonicalType,
    pub loaded_start: PlcValue,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeAggregateSource {
    Scalar(Operand),
    AggregateMemory(MemoryId),
}

impl RuntimeAggregateSource {
    fn encode(self, hasher: &mut SemanticHasher) {
        match self {
            Self::Scalar(operand) => {
                hasher.u8(1);
                operand.encode(hasher);
            }
            Self::AggregateMemory(memory) => {
                hasher.u8(2);
                hasher.u32(memory.0);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum RuntimeAggregateInstructionCode {
    Fill = 0x0040,
    BlockMove = 0x0041,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuntimeFormalRef {
    Instruction(u16),
    BlockMember(u128),
}

impl RuntimeFormalRef {
    fn encode(self, hasher: &mut SemanticHasher) {
        match self {
            Self::Instruction(id) => {
                hasher.u8(1);
                hasher.u16(id);
            }
            Self::BlockMember(id) => {
                hasher.u8(2);
                hasher.u128(id);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeBoundInput {
    pub formal: RuntimeFormalRef,
    pub source: Operand,
}

impl RuntimeBoundInput {
    fn encode(self, hasher: &mut SemanticHasher) {
        self.formal.encode(hasher);
        self.source.encode(hasher);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeDeclaredOutput {
    pub formal: RuntimeFormalRef,
    pub value_type: ValueType,
}

impl RuntimeDeclaredOutput {
    fn encode(self, hasher: &mut SemanticHasher) {
        self.formal.encode(hasher);
        hasher.u8(self.value_type as u8);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeDisabledBehavior {
    DefaultOutputsNoStateChange,
    PreserveOutputsNoStateChange,
    SuppressEffects,
}

impl RuntimeDisabledBehavior {
    const fn tag(self) -> u8 {
        match self {
            Self::DefaultOutputsNoStateChange => 1,
            Self::PreserveOutputsNoStateChange => 2,
            Self::SuppressEffects => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeActivation {
    pub enable: Operand,
    pub enable_formal: u16,
    pub status_formal: u16,
    pub status_when_disabled: bool,
    pub when_disabled: RuntimeDisabledBehavior,
}

impl RuntimeActivation {
    fn encode(self, hasher: &mut SemanticHasher) {
        self.enable.encode(hasher);
        hasher.u16(self.enable_formal);
        hasher.u16(self.status_formal);
        hasher.bool(self.status_when_disabled);
        hasher.u8(self.when_disabled.tag());
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum RuntimeInstructionCode {
    NoOp = 0x0001,
    Move = 0x0002,
    BoolNot = 0x0010,
    BoolAnd = 0x0011,
    BoolOr = 0x0012,
    BoolXor = 0x0013,
    CompareEqual = 0x0020,
    CompareNotEqual = 0x0021,
    CompareLess = 0x0022,
    CompareLessEqual = 0x0023,
    CompareGreater = 0x0024,
    CompareGreaterEqual = 0x0025,
    Add = 0x0030,
    Subtract = 0x0031,
    Multiply = 0x0032,
    Divide = 0x0033,
    Modulo = 0x0034,
    Limit = 0x0035,
    RisingEdge = 0x0100,
    FallingEdge = 0x0101,
    TimerOnDelay = 0x0110,
    TimerOffDelay = 0x0111,
    TimerPulse = 0x0112,
    CounterUp = 0x0120,
    CounterDown = 0x0121,
    CounterUpDown = 0x0122,
    Probe = 0x0400,
    TraceSample = 0x0401,
    BreakpointMarker = 0x0402,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuntimeInstructionStateKind {
    Edge,
    Timer,
    Counter,
}

impl RuntimeInstructionStateKind {
    const fn tag(self) -> u8 {
        match self {
            Self::Edge => 1,
            Self::Timer => 2,
            Self::Counter => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeInstructionInstance {
    pub stable_id: u128,
    pub kind: RuntimeInstructionStateKind,
    pub retentive: bool,
}

impl RuntimeInstructionInstance {
    fn encode(self, hasher: &mut SemanticHasher) {
        hasher.u128(self.stable_id);
        hasher.u8(self.kind.tag());
        hasher.bool(self.retentive);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeInstructionInvocation {
    pub instruction: RuntimeInstructionCode,
    pub inputs: Vec<RuntimeBoundInput>,
    pub outputs: Vec<RuntimeDeclaredOutput>,
    pub instance: Option<RuntimeInstructionInstance>,
    pub activation: Option<RuntimeActivation>,
}

impl RuntimeInstructionInvocation {
    fn encode(&self, hasher: &mut SemanticHasher) {
        hasher.u16(self.instruction as u16);
        hasher.u64(self.inputs.len() as u64);
        for input in &self.inputs {
            input.encode(hasher);
        }
        hasher.u64(self.outputs.len() as u64);
        for output in &self.outputs {
            output.encode(hasher);
        }
        match self.instance {
            Some(instance) => {
                hasher.bool(true);
                instance.encode(hasher);
            }
            None => hasher.bool(false),
        }
        match self.activation {
            Some(activation) => {
                hasher.bool(true);
                activation.encode(hasher);
            }
            None => hasher.bool(false),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeUnaryOperator {
    Plus,
    Negate,
    Not,
    Absolute,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeBinaryOperator {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeCallKind {
    Function,
    FunctionBlock,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeFrameMemberRole {
    Input,
    Output,
    InOut,
    Static,
    Temp,
    Constant,
    Return,
}

impl RuntimeFrameMemberRole {
    const fn tag(self) -> u8 {
        match self {
            Self::Input => 1,
            Self::Output => 2,
            Self::InOut => 3,
            Self::Static => 4,
            Self::Temp => 5,
            Self::Constant => 6,
            Self::Return => 7,
        }
    }

    pub(crate) const fn persists_in_instance(self) -> bool {
        matches!(
            self,
            Self::Input | Self::Output | Self::InOut | Self::Static
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeFrameMember {
    pub formal: u128,
    pub memory: MemoryId,
    pub value_type: ValueType,
    pub role: RuntimeFrameMemberRole,
    pub declared_order: u32,
    pub initial_value: CanonicalValue,
    pub retentive: bool,
}

impl RuntimeFrameMember {
    fn encode(self, hasher: &mut SemanticHasher) {
        hasher.u128(self.formal);
        hasher.u32(self.memory.0);
        hasher.u8(self.value_type as u8);
        hasher.u8(self.role.tag());
        hasher.u32(self.declared_order);
        self.initial_value.encode(hasher);
        hasher.bool(self.retentive);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeFunctionBlockInstance {
    pub root_instance: u128,
    pub multi_instance_slots: Vec<u128>,
}

impl RuntimeFunctionBlockInstance {
    fn encode(&self, hasher: &mut SemanticHasher) {
        hasher.u128(self.root_instance);
        hasher.u64(self.multi_instance_slots.len() as u64);
        for slot in &self.multi_instance_slots {
            hasher.u128(*slot);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeBlockCall {
    pub kind: RuntimeCallKind,
    pub target_identity: u128,
    pub signature_fingerprint: Hash32,
    pub call_site_identity: u128,
    pub inputs: Vec<RuntimeBoundInput>,
    pub outputs: Vec<RuntimeDeclaredOutput>,
    pub instance: Option<RuntimeFunctionBlockInstance>,
    pub activation: Option<RuntimeActivation>,
    pub frame_members: Vec<RuntimeFrameMember>,
    pub callee: ProgramBlock,
}

impl RuntimeBlockCall {
    fn encode(&self, hasher: &mut SemanticHasher) {
        hasher.u8(match self.kind {
            RuntimeCallKind::Function => 1,
            RuntimeCallKind::FunctionBlock => 2,
        });
        hasher.u128(self.target_identity);
        hasher.hash(self.signature_fingerprint);
        hasher.u128(self.call_site_identity);
        hasher.u64(self.inputs.len() as u64);
        for input in &self.inputs {
            input.encode(hasher);
        }
        hasher.u64(self.outputs.len() as u64);
        for output in &self.outputs {
            output.encode(hasher);
        }
        match &self.instance {
            Some(instance) => {
                hasher.bool(true);
                instance.encode(hasher);
            }
            None => hasher.bool(false),
        }
        match self.activation {
            Some(activation) => {
                hasher.bool(true);
                activation.encode(hasher);
            }
            None => hasher.bool(false),
        }
        hasher.u64(self.frame_members.len() as u64);
        for member in &self.frame_members {
            member.encode(hasher);
        }
        self.callee.encode(hasher);
    }
}

#[must_use]
pub fn runtime_block_signature_fingerprint(
    target_identity: u128,
    members: &[RuntimeFrameMember],
) -> Hash32 {
    let mut hasher = SemanticHasher::new("PES-RUNTIME-BLOCK-SIGNATURE-1");
    hasher.u128(target_identity);
    hasher.u64(members.len() as u64);
    for member in members {
        member.encode(&mut hasher);
    }
    hasher.finish()
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
    Unary {
        operator: RuntimeUnaryOperator,
        operand: Operand,
        target: MemoryId,
    },
    Binary {
        operator: RuntimeBinaryOperator,
        left: Operand,
        right: Operand,
        target: MemoryId,
    },
    Convert {
        source: Operand,
        destination: ValueType,
        target: MemoryId,
    },
    /// Selects a FOR loop's entry condition from the runtime sign of its
    /// entry-evaluated step. A zero step is an invalid argument.
    ForCondition {
        current: Operand,
        terminal: Operand,
        step: Operand,
        target: MemoryId,
    },
    /// Tests a FOR increment in widened mathematical space and publishes only
    /// whether the exact signed next value remains within the terminal bound.
    ForNextWithin {
        current: Operand,
        terminal: Operand,
        step: Operand,
        target: MemoryId,
    },
    /// Executes one verified aggregate instruction atomically. `scalar_leaves`
    /// is independently checked against the target type and participates in
    /// the declared `EDU-WORK-1` cost.
    AggregateInstruction {
        instruction: RuntimeAggregateInstructionCode,
        input: RuntimeAggregateSource,
        target: MemoryId,
        activation: Option<Operand>,
        status: MemoryId,
        scalar_leaves: u32,
    },
    InvokeInstruction(RuntimeInstructionInvocation),
    CallBlock(RuntimeBlockCall),
    InvocationOutput {
        invocation_id: u32,
        formal: RuntimeFormalRef,
        target: MemoryId,
    },
    /// Transfers control to an exact zero-based instruction index in this block.
    Jump {
        target: u32,
    },
    /// Selects one of two exact zero-based instruction indices from a BOOL value.
    Branch {
        condition: Operand,
        when_true: u32,
        when_false: u32,
    },
    /// Returns from this runtime block to its caller or task boundary.
    Return,
}

impl Operation {
    pub const fn work_units(&self) -> u32 {
        match self {
            Self::AggregateInstruction { scalar_leaves, .. } => {
                1_u32.saturating_add(*scalar_leaves)
            }
            _ => 1,
        }
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
            Self::Unary {
                operator,
                operand,
                target,
            } => {
                hasher.u8(11);
                hasher.u8(match operator {
                    RuntimeUnaryOperator::Plus => 1,
                    RuntimeUnaryOperator::Negate => 2,
                    RuntimeUnaryOperator::Not => 3,
                    RuntimeUnaryOperator::Absolute => 4,
                });
                operand.encode(hasher);
                hasher.u32(target.0);
            }
            Self::Binary {
                operator,
                left,
                right,
                target,
            } => {
                hasher.u8(12);
                hasher.u8(match operator {
                    RuntimeBinaryOperator::Multiply => 1,
                    RuntimeBinaryOperator::Divide => 2,
                    RuntimeBinaryOperator::Modulo => 3,
                    RuntimeBinaryOperator::Add => 4,
                    RuntimeBinaryOperator::Subtract => 5,
                    RuntimeBinaryOperator::Equal => 6,
                    RuntimeBinaryOperator::NotEqual => 7,
                    RuntimeBinaryOperator::Less => 8,
                    RuntimeBinaryOperator::LessEqual => 9,
                    RuntimeBinaryOperator::Greater => 10,
                    RuntimeBinaryOperator::GreaterEqual => 11,
                    RuntimeBinaryOperator::And => 12,
                    RuntimeBinaryOperator::Xor => 13,
                    RuntimeBinaryOperator::Or => 14,
                    RuntimeBinaryOperator::Minimum => 15,
                    RuntimeBinaryOperator::Maximum => 16,
                });
                left.encode(hasher);
                right.encode(hasher);
                hasher.u32(target.0);
            }
            Self::Convert {
                source,
                destination,
                target,
            } => {
                hasher.u8(16);
                source.encode(hasher);
                hasher.u8(*destination as u8);
                hasher.u32(target.0);
            }
            Self::ForCondition {
                current,
                terminal,
                step,
                target,
            } => {
                hasher.u8(21);
                current.encode(hasher);
                terminal.encode(hasher);
                step.encode(hasher);
                hasher.u32(target.0);
            }
            Self::ForNextWithin {
                current,
                terminal,
                step,
                target,
            } => {
                hasher.u8(20);
                current.encode(hasher);
                terminal.encode(hasher);
                step.encode(hasher);
                hasher.u32(target.0);
            }
            Self::AggregateInstruction {
                instruction,
                input,
                target,
                activation,
                status,
                scalar_leaves,
            } => {
                hasher.u8(22);
                hasher.u16(*instruction as u16);
                input.encode(hasher);
                hasher.u32(target.0);
                match activation {
                    Some(enable) => {
                        hasher.bool(true);
                        enable.encode(hasher);
                    }
                    None => hasher.bool(false),
                }
                hasher.u32(status.0);
                hasher.u32(*scalar_leaves);
            }
            Self::InvokeInstruction(invocation) => {
                hasher.u8(13);
                invocation.encode(hasher);
            }
            Self::CallBlock(call) => {
                hasher.u8(14);
                call.encode(hasher);
            }
            Self::InvocationOutput {
                invocation_id,
                formal,
                target,
            } => {
                hasher.u8(15);
                hasher.u32(*invocation_id);
                formal.encode(hasher);
                hasher.u32(target.0);
            }
            Self::Jump { target } => {
                hasher.u8(17);
                hasher.u32(*target);
            }
            Self::Branch {
                condition,
                when_true,
                when_false,
            } => {
                hasher.u8(18);
                condition.encode(hasher);
                hasher.u32(*when_true);
                hasher.u32(*when_false);
            }
            Self::Return => hasher.u8(19),
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
    pub aggregate_memory: Vec<AggregateMemoryDefinition>,
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
        Self::edu21_with_aggregates(
            profile_fingerprint,
            memory,
            Vec::new(),
            channels,
            states,
            program,
        )
    }

    pub fn edu21_with_aggregates(
        profile_fingerprint: Hash32,
        memory: Vec<MemoryDefinition>,
        aggregate_memory: Vec<AggregateMemoryDefinition>,
        channels: Vec<ChannelDefinition>,
        states: Vec<StateDefinition>,
        program: ProgramImage,
    ) -> Self {
        Self {
            schema_version: 2,
            runtime_version: RUNTIME_SEMANTICS_VERSION.into(),
            scheduler_version: SCHEDULER_VERSION.into(),
            priority_table_version: PRIORITY_TABLE_VERSION.into(),
            work_cost_version: WORK_COST_VERSION.into(),
            profile_fingerprint,
            memory,
            aggregate_memory,
            channels,
            states,
            program,
        }
    }

    fn normalize(&mut self) {
        self.memory.sort_by_key(|definition| definition.id);
        self.aggregate_memory
            .sort_by_key(|definition| definition.id);
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

        hasher.u64(self.aggregate_memory.len() as u64);
        for definition in &self.aggregate_memory {
            hasher.u32(definition.id.0);
            hasher.bytes(
                &definition
                    .data_type
                    .canonical_bytes(AggregateLimits::edu21())
                    .unwrap_or_default(),
            );
            hasher.bytes(
                &definition
                    .data_type
                    .serialize_value(&definition.loaded_start, AggregateLimits::edu21())
                    .unwrap_or_default(),
            );
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
    DuplicateOrUnorderedAggregateMemory(MemoryId),
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
    InvalidInvocation { block: BlockId, operation_id: u32 },
    InvalidInvocationProjection { block: BlockId, operation_id: u32 },
    InvalidBlockSignature { block: BlockId, operation_id: u32 },
    InvalidControlFlow { block: BlockId, operation_id: u32 },
    RecursiveBlockCall(BlockId),
    DynamicCallDepthExceeded(BlockId),
}

impl fmt::Display for ArtifactError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "artifact rejected: {self:?}")
    }
}

impl Error for ArtifactError {}

fn validate_spec(spec: &ArtifactSpec) -> Result<(), ArtifactError> {
    if spec.schema_version != 2 {
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
        &spec.aggregate_memory,
        |entry| entry.id,
        ArtifactError::DuplicateOrUnorderedAggregateMemory,
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
    for definition in &spec.aggregate_memory {
        if spec
            .memory
            .binary_search_by_key(&definition.id, |entry| entry.id)
            .is_ok()
            || definition
                .data_type
                .validate_value(&definition.loaded_start, AggregateLimits::edu21())
                .is_err()
        {
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
    let mut used_state_ids = BTreeSet::new();
    if let Some(startup) = &spec.program.startup {
        block_ids.push(startup.id);
        validate_block(spec, startup, &mut used_state_ids)?;
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
        validate_block(spec, &task.block, &mut used_state_ids)?;
    }
    block_ids.push(spec.program.cyclic.id);
    validate_block(spec, &spec.program.cyclic, &mut used_state_ids)?;
    block_ids.sort_unstable();
    for pair in block_ids.windows(2) {
        if pair[0] == pair[1] {
            return Err(ArtifactError::DuplicateBlock(pair[0]));
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

fn validate_block(
    spec: &ArtifactSpec,
    block: &ProgramBlock,
    used_state_ids: &mut BTreeSet<StateId>,
) -> Result<(), ArtifactError> {
    validate_block_inner(spec, block, used_state_ids, &mut Vec::new(), 0)
}

fn validate_block_inner(
    spec: &ArtifactSpec,
    block: &ProgramBlock,
    used_state_ids: &mut BTreeSet<StateId>,
    call_path: &mut Vec<BlockId>,
    depth: u8,
) -> Result<(), ArtifactError> {
    if depth > crate::MAX_DYNAMIC_CALL_DEPTH {
        return Err(ArtifactError::DynamicCallDepthExceeded(block.id));
    }
    if call_path.contains(&block.id) {
        return Err(ArtifactError::RecursiveBlockCall(block.id));
    }
    call_path.push(block.id);
    let mut operation_ids = BTreeSet::new();
    let mut invocation_outputs = BTreeMap::<(u32, RuntimeFormalRef), (ValueType, bool)>::new();
    let instruction_count = u32::try_from(block.instructions.len()).unwrap_or(u32::MAX);
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
        match instruction.operation() {
            Operation::Jump { target } if *target >= instruction_count => {
                return Err(ArtifactError::InvalidControlFlow {
                    block: block.id,
                    operation_id: instruction.operation_id,
                });
            }
            Operation::Branch {
                when_true,
                when_false,
                ..
            } if *when_true >= instruction_count || *when_false >= instruction_count => {
                return Err(ArtifactError::InvalidControlFlow {
                    block: block.id,
                    operation_id: instruction.operation_id,
                });
            }
            Operation::InvokeInstruction(invocation) => {
                validate_instruction_invocation(
                    spec,
                    block.id,
                    instruction.operation_id,
                    invocation,
                )?;
                register_invocation_outputs(
                    block.id,
                    instruction.operation_id,
                    &invocation.outputs,
                    &mut invocation_outputs,
                )?;
            }
            Operation::CallBlock(call) => {
                validate_block_call(spec, block.id, instruction.operation_id, call)?;
                register_invocation_outputs(
                    block.id,
                    instruction.operation_id,
                    &call.outputs,
                    &mut invocation_outputs,
                )?;
                validate_block_inner(
                    spec,
                    &call.callee,
                    used_state_ids,
                    call_path,
                    depth.saturating_add(1),
                )?;
            }
            Operation::InvocationOutput {
                invocation_id,
                formal,
                target,
            } => {
                let Some((expected, projected)) =
                    invocation_outputs.get_mut(&(*invocation_id, *formal))
                else {
                    return Err(ArtifactError::InvalidInvocationProjection {
                        block: block.id,
                        operation_id: instruction.operation_id,
                    });
                };
                if *projected || memory_type(spec, *target)? != *expected {
                    return Err(ArtifactError::InvalidInvocationProjection {
                        block: block.id,
                        operation_id: instruction.operation_id,
                    });
                }
                *projected = true;
            }
            Operation::Noop
            | Operation::SetMemory { .. }
            | Operation::Copy { .. }
            | Operation::AddI32 { .. }
            | Operation::DivideI32 { .. }
            | Operation::LoadInput { .. }
            | Operation::StoreOutput { .. }
            | Operation::RisingEdge { .. }
            | Operation::FallingEdge { .. }
            | Operation::TimerOnDelay { .. }
            | Operation::CounterUp { .. }
            | Operation::Unary { .. }
            | Operation::Binary { .. }
            | Operation::Convert { .. }
            | Operation::ForCondition { .. }
            | Operation::ForNextWithin { .. }
            | Operation::AggregateInstruction { .. }
            | Operation::Jump { .. }
            | Operation::Branch { .. }
            | Operation::Return => {}
        }
    }
    if invocation_outputs.values().any(|(_, projected)| !projected) {
        return Err(ArtifactError::InvalidInvocationProjection {
            block: block.id,
            operation_id: 0,
        });
    }
    call_path.pop();
    Ok(())
}

fn register_invocation_outputs(
    block: BlockId,
    operation_id: u32,
    outputs: &[RuntimeDeclaredOutput],
    registry: &mut BTreeMap<(u32, RuntimeFormalRef), (ValueType, bool)>,
) -> Result<(), ArtifactError> {
    for output in outputs {
        if registry
            .insert((operation_id, output.formal), (output.value_type, false))
            .is_some()
        {
            return Err(ArtifactError::InvalidInvocation {
                block,
                operation_id,
            });
        }
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
    let aggregate = |id: MemoryId| {
        spec.aggregate_memory
            .binary_search_by_key(&id, |definition| definition.id)
            .ok()
            .map(|index| &spec.aggregate_memory[index])
            .ok_or(ArtifactError::UnknownMemory(id))
    };
    let canonical_memory_type = |id: MemoryId| -> Result<CanonicalType, ArtifactError> {
        if let Ok(value_type) = memory_type(id) {
            return Ok(CanonicalType::Primitive(value_type.primitive_type()));
        }
        Ok(aggregate(id)?.data_type.clone())
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
        Operation::Unary {
            operator,
            operand,
            target,
        } => {
            let actual = operand_type(*operand)?;
            let primitive = actual.primitive_type();
            let accepted = match operator {
                RuntimeUnaryOperator::Plus => primitive.is_numeric(),
                RuntimeUnaryOperator::Negate | RuntimeUnaryOperator::Absolute => {
                    primitive.is_signed_integer()
                        || matches!(primitive, PrimitiveType::Real | PrimitiveType::Lreal)
                }
                RuntimeUnaryOperator::Not => {
                    primitive == PrimitiveType::Bool || primitive.is_bit_string()
                }
            };
            if !accepted {
                return Err(ArtifactError::TypeMismatch);
            }
            same(memory_type(*target)?, actual)
        }
        Operation::Binary {
            operator,
            left,
            right,
            target,
        } => {
            let left_type = operand_type(*left)?;
            let right_type = operand_type(*right)?;
            same(left_type, right_type)?;
            match operator {
                RuntimeBinaryOperator::And
                | RuntimeBinaryOperator::Xor
                | RuntimeBinaryOperator::Or => {
                    let primitive = left_type.primitive_type();
                    if primitive != PrimitiveType::Bool && !primitive.is_bit_string() {
                        return Err(ArtifactError::TypeMismatch);
                    }
                    same(memory_type(*target)?, left_type)
                }
                RuntimeBinaryOperator::Equal
                | RuntimeBinaryOperator::NotEqual
                | RuntimeBinaryOperator::Less
                | RuntimeBinaryOperator::LessEqual
                | RuntimeBinaryOperator::Greater
                | RuntimeBinaryOperator::GreaterEqual => {
                    same(memory_type(*target)?, ValueType::Bool)
                }
                RuntimeBinaryOperator::Multiply
                | RuntimeBinaryOperator::Divide
                | RuntimeBinaryOperator::Modulo
                | RuntimeBinaryOperator::Add
                | RuntimeBinaryOperator::Subtract
                | RuntimeBinaryOperator::Minimum
                | RuntimeBinaryOperator::Maximum => {
                    let primitive = left_type.primitive_type();
                    if !primitive.is_numeric()
                        || (*operator == RuntimeBinaryOperator::Modulo && !primitive.is_integer())
                    {
                        return Err(ArtifactError::TypeMismatch);
                    }
                    same(memory_type(*target)?, left_type)
                }
            }
        }
        Operation::Convert {
            source,
            destination,
            target,
        } => {
            let source = operand_type(*source)?;
            if !explicit_conversion_allowed(source.primitive_type(), destination.primitive_type()) {
                return Err(ArtifactError::TypeMismatch);
            }
            same(memory_type(*target)?, *destination)
        }
        Operation::ForCondition {
            current,
            terminal,
            step,
            target,
        }
        | Operation::ForNextWithin {
            current,
            terminal,
            step,
            target,
        } => {
            let current_type = operand_type(*current)?;
            same(operand_type(*terminal)?, current_type)?;
            same(operand_type(*step)?, current_type)?;
            if !current_type.primitive_type().is_signed_integer() {
                return Err(ArtifactError::TypeMismatch);
            }
            same(memory_type(*target)?, ValueType::Bool)
        }
        Operation::AggregateInstruction {
            instruction,
            input,
            target,
            activation,
            status,
            scalar_leaves,
        } => {
            same(memory_type(*status)?, ValueType::Bool)?;
            if let Some(enable) = activation {
                same(operand_type(*enable)?, ValueType::Bool)?;
            }
            let target_type = canonical_memory_type(*target)?;
            let expected_leaves = u32::try_from(
                aggregate_scalar_leaf_count(&target_type).ok_or(ArtifactError::TypeMismatch)?,
            )
            .map_err(|_| ArtifactError::TypeMismatch)?;
            if *scalar_leaves != expected_leaves {
                return Err(ArtifactError::TypeMismatch);
            }
            let source_type = match input {
                RuntimeAggregateSource::Scalar(operand) => {
                    CanonicalType::Primitive(operand_type(*operand)?.primitive_type())
                }
                RuntimeAggregateSource::AggregateMemory(memory) => {
                    aggregate(*memory)?.data_type.clone()
                }
            };
            let compatible = match instruction {
                RuntimeAggregateInstructionCode::Fill => {
                    let CanonicalType::Array { element_type, .. } = &target_type else {
                        return Err(ArtifactError::TypeMismatch);
                    };
                    source_type
                        .assignment_compatible_with(element_type, AggregateLimits::edu21())
                        .unwrap_or(false)
                }
                RuntimeAggregateInstructionCode::BlockMove => {
                    if !block_movable_type(&source_type) || !block_movable_type(&target_type) {
                        return Err(ArtifactError::TypeMismatch);
                    }
                    source_type
                        .assignment_compatible_with(&target_type, AggregateLimits::edu21())
                        .unwrap_or(false)
                }
            };
            if compatible {
                Ok(())
            } else {
                Err(ArtifactError::TypeMismatch)
            }
        }
        Operation::Branch { condition, .. } => same(operand_type(*condition)?, ValueType::Bool),
        Operation::Jump { .. } | Operation::Return => Ok(()),
        Operation::InvokeInstruction(_)
        | Operation::CallBlock(_)
        | Operation::InvocationOutput { .. } => Ok(()),
    }
}

fn block_movable_type(data_type: &CanonicalType) -> bool {
    matches!(
        data_type,
        CanonicalType::Primitive(PrimitiveType::String(_))
            | CanonicalType::Array { .. }
            | CanonicalType::AnonymousStruct { .. }
            | CanonicalType::NamedStruct { .. }
    )
}

fn aggregate_scalar_leaf_count(data_type: &CanonicalType) -> Option<u64> {
    match data_type {
        CanonicalType::Primitive(_) => Some(1),
        CanonicalType::Array {
            dimensions,
            element_type,
        } => {
            let count = dimensions.iter().try_fold(1_u64, |count, bound| {
                count.checked_mul(bound.element_count().ok()?)
            })?;
            count.checked_mul(aggregate_scalar_leaf_count(element_type)?)
        }
        CanonicalType::AnonymousStruct { members } | CanonicalType::NamedStruct { members, .. } => {
            members.iter().try_fold(0_u64, |count, member| {
                count.checked_add(aggregate_scalar_leaf_count(&member.data_type)?)
            })
        }
    }
}

const FORMAL_ENABLE: u16 = 0x0001;
const FORMAL_ENABLE_OUTPUT: u16 = 0x0002;
const FORMAL_INPUT: u16 = 0x0010;
const FORMAL_OUTPUT: u16 = 0x0011;
const FORMAL_LEFT: u16 = 0x0020;
const FORMAL_RIGHT: u16 = 0x0021;
const FORMAL_CLOCK: u16 = 0x0030;
const FORMAL_PRESET_TIME: u16 = 0x0031;
const FORMAL_ELAPSED_TIME: u16 = 0x0032;
const FORMAL_COUNT_UP: u16 = 0x0040;
const FORMAL_COUNT_DOWN: u16 = 0x0041;
const FORMAL_RESET: u16 = 0x0042;
const FORMAL_LOAD: u16 = 0x0043;
const FORMAL_PRESET_VALUE: u16 = 0x0044;
const FORMAL_CURRENT_VALUE: u16 = 0x0045;
const FORMAL_QU: u16 = 0x0046;
const FORMAL_QD: u16 = 0x0047;
const FORMAL_MINIMUM: u16 = 0x0050;
const FORMAL_LIMIT_INPUT: u16 = 0x0051;
const FORMAL_MAXIMUM: u16 = 0x0052;
const FORMAL_LIMIT_OUTPUT: u16 = 0x0053;

fn memory_type(spec: &ArtifactSpec, id: MemoryId) -> Result<ValueType, ArtifactError> {
    spec.memory
        .binary_search_by_key(&id, |definition| definition.id)
        .ok()
        .map(|index| spec.memory[index].value_type)
        .ok_or(ArtifactError::UnknownMemory(id))
}

fn operand_type(spec: &ArtifactSpec, operand: Operand) -> Result<ValueType, ArtifactError> {
    match operand {
        Operand::Constant(value) => Ok(value.value_type()),
        Operand::Memory(id) => memory_type(spec, id),
        Operand::Input(id) | Operand::Output(id) => spec
            .channels
            .binary_search_by_key(&id, |definition| definition.id)
            .ok()
            .map(|index| spec.channels[index].value_type)
            .ok_or(ArtifactError::UnknownChannel(id)),
    }
}

fn validate_instruction_invocation(
    spec: &ArtifactSpec,
    block: BlockId,
    operation_id: u32,
    invocation: &RuntimeInstructionInvocation,
) -> Result<(), ArtifactError> {
    if invocation
        .inputs
        .windows(2)
        .any(|pair| pair[0].formal >= pair[1].formal)
        || invocation
            .outputs
            .windows(2)
            .any(|pair| pair[0].formal >= pair[1].formal)
    {
        return invalid_invocation(block, operation_id);
    }
    let mut inputs = BTreeMap::new();
    for input in &invocation.inputs {
        let RuntimeFormalRef::Instruction(formal) = input.formal else {
            return invalid_invocation(block, operation_id);
        };
        inputs.insert(formal, operand_type(spec, input.source)?);
    }
    let mut outputs = BTreeMap::new();
    for output in &invocation.outputs {
        let RuntimeFormalRef::Instruction(formal) = output.formal else {
            return invalid_invocation(block, operation_id);
        };
        outputs.insert(formal, output.value_type);
    }

    let state_kind = match invocation.instruction {
        RuntimeInstructionCode::RisingEdge | RuntimeInstructionCode::FallingEdge => {
            Some(RuntimeInstructionStateKind::Edge)
        }
        RuntimeInstructionCode::TimerOnDelay
        | RuntimeInstructionCode::TimerOffDelay
        | RuntimeInstructionCode::TimerPulse => Some(RuntimeInstructionStateKind::Timer),
        RuntimeInstructionCode::CounterUp
        | RuntimeInstructionCode::CounterDown
        | RuntimeInstructionCode::CounterUpDown => Some(RuntimeInstructionStateKind::Counter),
        RuntimeInstructionCode::NoOp
        | RuntimeInstructionCode::Move
        | RuntimeInstructionCode::BoolNot
        | RuntimeInstructionCode::BoolAnd
        | RuntimeInstructionCode::BoolOr
        | RuntimeInstructionCode::BoolXor
        | RuntimeInstructionCode::CompareEqual
        | RuntimeInstructionCode::CompareNotEqual
        | RuntimeInstructionCode::CompareLess
        | RuntimeInstructionCode::CompareLessEqual
        | RuntimeInstructionCode::CompareGreater
        | RuntimeInstructionCode::CompareGreaterEqual
        | RuntimeInstructionCode::Add
        | RuntimeInstructionCode::Subtract
        | RuntimeInstructionCode::Multiply
        | RuntimeInstructionCode::Divide
        | RuntimeInstructionCode::Modulo
        | RuntimeInstructionCode::Limit
        | RuntimeInstructionCode::Probe
        | RuntimeInstructionCode::TraceSample
        | RuntimeInstructionCode::BreakpointMarker => None,
    };
    match (state_kind, invocation.instance) {
        (None, None) => {}
        (Some(expected), Some(instance))
            if instance.stable_id != 0 && instance.kind == expected => {}
        _ => return invalid_invocation(block, operation_id),
    }

    let disabled = match invocation.instruction {
        RuntimeInstructionCode::Move => Some(RuntimeDisabledBehavior::SuppressEffects),
        RuntimeInstructionCode::BoolNot
        | RuntimeInstructionCode::BoolAnd
        | RuntimeInstructionCode::BoolOr
        | RuntimeInstructionCode::BoolXor
        | RuntimeInstructionCode::CompareEqual
        | RuntimeInstructionCode::CompareNotEqual
        | RuntimeInstructionCode::CompareLess
        | RuntimeInstructionCode::CompareLessEqual
        | RuntimeInstructionCode::CompareGreater
        | RuntimeInstructionCode::CompareGreaterEqual
        | RuntimeInstructionCode::Add
        | RuntimeInstructionCode::Subtract
        | RuntimeInstructionCode::Multiply
        | RuntimeInstructionCode::Divide
        | RuntimeInstructionCode::Modulo
        | RuntimeInstructionCode::Limit
        | RuntimeInstructionCode::RisingEdge
        | RuntimeInstructionCode::FallingEdge => {
            Some(RuntimeDisabledBehavior::DefaultOutputsNoStateChange)
        }
        RuntimeInstructionCode::TimerOnDelay
        | RuntimeInstructionCode::TimerOffDelay
        | RuntimeInstructionCode::TimerPulse
        | RuntimeInstructionCode::CounterUp
        | RuntimeInstructionCode::CounterDown
        | RuntimeInstructionCode::CounterUpDown => {
            Some(RuntimeDisabledBehavior::PreserveOutputsNoStateChange)
        }
        RuntimeInstructionCode::NoOp
        | RuntimeInstructionCode::Probe
        | RuntimeInstructionCode::TraceSample
        | RuntimeInstructionCode::BreakpointMarker => None,
    };
    validate_activation(
        spec,
        block,
        operation_id,
        invocation.activation,
        disabled,
        &outputs,
    )?;

    let valid = match invocation.instruction {
        RuntimeInstructionCode::NoOp | RuntimeInstructionCode::BreakpointMarker => {
            inputs.is_empty() && outputs.is_empty()
        }
        RuntimeInstructionCode::Probe | RuntimeInstructionCode::TraceSample => {
            inputs.len() == 1 && inputs.contains_key(&FORMAL_INPUT) && outputs.is_empty()
        }
        RuntimeInstructionCode::Move => {
            inputs.len() == 1
                && outputs_without_status(&outputs) == 1
                && inputs.get(&FORMAL_INPUT) == outputs.get(&FORMAL_OUTPUT)
        }
        RuntimeInstructionCode::BoolNot => {
            exact_input(&inputs, FORMAL_INPUT, ValueType::Bool)
                && exact_output(&outputs, FORMAL_OUTPUT, ValueType::Bool)
                && outputs_without_status(&outputs) == 1
        }
        RuntimeInstructionCode::BoolAnd
        | RuntimeInstructionCode::BoolOr
        | RuntimeInstructionCode::BoolXor => {
            exact_two_inputs(
                &inputs,
                (FORMAL_LEFT, ValueType::Bool),
                (FORMAL_RIGHT, ValueType::Bool),
            ) && exact_output(&outputs, FORMAL_OUTPUT, ValueType::Bool)
                && outputs_without_status(&outputs) == 1
        }
        RuntimeInstructionCode::CompareEqual
        | RuntimeInstructionCode::CompareNotEqual
        | RuntimeInstructionCode::CompareLess
        | RuntimeInstructionCode::CompareLessEqual
        | RuntimeInstructionCode::CompareGreater
        | RuntimeInstructionCode::CompareGreaterEqual => {
            inputs.len() == 2
                && inputs.get(&FORMAL_LEFT) == inputs.get(&FORMAL_RIGHT)
                && inputs.contains_key(&FORMAL_LEFT)
                && exact_output(&outputs, FORMAL_OUTPUT, ValueType::Bool)
                && outputs_without_status(&outputs) == 1
        }
        RuntimeInstructionCode::Add
        | RuntimeInstructionCode::Subtract
        | RuntimeInstructionCode::Multiply
        | RuntimeInstructionCode::Divide
        | RuntimeInstructionCode::Modulo => {
            inputs.len() == 2
                && inputs.get(&FORMAL_LEFT) == inputs.get(&FORMAL_RIGHT)
                && inputs.get(&FORMAL_LEFT).is_some_and(|value_type| {
                    let primitive = value_type.primitive_type();
                    primitive.is_numeric()
                        && (invocation.instruction != RuntimeInstructionCode::Modulo
                            || primitive.is_integer())
                })
                && outputs.get(&FORMAL_OUTPUT) == inputs.get(&FORMAL_LEFT)
                && outputs_without_status(&outputs) == 1
        }
        RuntimeInstructionCode::Limit => {
            inputs.len() == 3
                && inputs.get(&FORMAL_MINIMUM) == inputs.get(&FORMAL_LIMIT_INPUT)
                && inputs.get(&FORMAL_MINIMUM) == inputs.get(&FORMAL_MAXIMUM)
                && inputs.get(&FORMAL_MINIMUM).is_some_and(|value_type| {
                    let primitive = value_type.primitive_type();
                    primitive.is_numeric() || primitive == PrimitiveType::Time
                })
                && outputs.get(&FORMAL_LIMIT_OUTPUT) == inputs.get(&FORMAL_MINIMUM)
                && outputs_without_status(&outputs) == 1
        }
        RuntimeInstructionCode::RisingEdge | RuntimeInstructionCode::FallingEdge => {
            exact_input(&inputs, FORMAL_CLOCK, ValueType::Bool)
                && exact_output(&outputs, FORMAL_OUTPUT, ValueType::Bool)
                && outputs_without_status(&outputs) == 1
        }
        RuntimeInstructionCode::TimerOnDelay
        | RuntimeInstructionCode::TimerOffDelay
        | RuntimeInstructionCode::TimerPulse => {
            exact_two_inputs(
                &inputs,
                (FORMAL_INPUT, ValueType::Bool),
                (FORMAL_PRESET_TIME, ValueType::TimeMs),
            ) && exact_output(&outputs, FORMAL_OUTPUT, ValueType::Bool)
                && exact_output(&outputs, FORMAL_ELAPSED_TIME, ValueType::TimeMs)
                && outputs_without_status(&outputs) == 2
        }
        RuntimeInstructionCode::CounterUp => {
            exact_three_inputs(
                &inputs,
                (FORMAL_COUNT_UP, ValueType::Bool),
                (FORMAL_RESET, ValueType::Bool),
                (FORMAL_PRESET_VALUE, ValueType::I32),
            ) && exact_output(&outputs, FORMAL_OUTPUT, ValueType::Bool)
                && exact_output(&outputs, FORMAL_CURRENT_VALUE, ValueType::I32)
                && outputs_without_status(&outputs) == 2
        }
        RuntimeInstructionCode::CounterDown => {
            exact_three_inputs(
                &inputs,
                (FORMAL_COUNT_DOWN, ValueType::Bool),
                (FORMAL_LOAD, ValueType::Bool),
                (FORMAL_PRESET_VALUE, ValueType::I32),
            ) && exact_output(&outputs, FORMAL_OUTPUT, ValueType::Bool)
                && exact_output(&outputs, FORMAL_CURRENT_VALUE, ValueType::I32)
                && outputs_without_status(&outputs) == 2
        }
        RuntimeInstructionCode::CounterUpDown => {
            inputs.len() == 5
                && exact_output(&outputs, FORMAL_QU, ValueType::Bool)
                && exact_output(&outputs, FORMAL_QD, ValueType::Bool)
                && exact_output(&outputs, FORMAL_CURRENT_VALUE, ValueType::I32)
                && inputs.get(&FORMAL_COUNT_UP) == Some(&ValueType::Bool)
                && inputs.get(&FORMAL_COUNT_DOWN) == Some(&ValueType::Bool)
                && inputs.get(&FORMAL_RESET) == Some(&ValueType::Bool)
                && inputs.get(&FORMAL_LOAD) == Some(&ValueType::Bool)
                && inputs.get(&FORMAL_PRESET_VALUE) == Some(&ValueType::I32)
                && outputs_without_status(&outputs) == 3
        }
    };
    if valid {
        Ok(())
    } else {
        invalid_invocation(block, operation_id)
    }
}

fn validate_activation(
    spec: &ArtifactSpec,
    block: BlockId,
    operation_id: u32,
    activation: Option<RuntimeActivation>,
    expected: Option<RuntimeDisabledBehavior>,
    outputs: &BTreeMap<u16, ValueType>,
) -> Result<(), ArtifactError> {
    match (expected, activation) {
        (None, None) if !outputs.contains_key(&FORMAL_ENABLE_OUTPUT) => Ok(()),
        (Some(_), None) if !outputs.contains_key(&FORMAL_ENABLE_OUTPUT) => Ok(()),
        (Some(behavior), Some(actual))
            if actual.enable_formal == FORMAL_ENABLE
                && actual.status_formal == FORMAL_ENABLE_OUTPUT
                && !actual.status_when_disabled
                && actual.when_disabled == behavior
                && operand_type(spec, actual.enable)? == ValueType::Bool
                && outputs.get(&FORMAL_ENABLE_OUTPUT) == Some(&ValueType::Bool) =>
        {
            Ok(())
        }
        _ => invalid_invocation(block, operation_id),
    }
}

fn exact_input(inputs: &BTreeMap<u16, ValueType>, formal: u16, value_type: ValueType) -> bool {
    inputs.len() == 1 && inputs.get(&formal) == Some(&value_type)
}

fn exact_two_inputs(
    inputs: &BTreeMap<u16, ValueType>,
    first: (u16, ValueType),
    second: (u16, ValueType),
) -> bool {
    inputs.len() == 2
        && inputs.get(&first.0) == Some(&first.1)
        && inputs.get(&second.0) == Some(&second.1)
}

fn exact_three_inputs(
    inputs: &BTreeMap<u16, ValueType>,
    first: (u16, ValueType),
    second: (u16, ValueType),
    third: (u16, ValueType),
) -> bool {
    inputs.len() == 3
        && inputs.get(&first.0) == Some(&first.1)
        && inputs.get(&second.0) == Some(&second.1)
        && inputs.get(&third.0) == Some(&third.1)
}

fn exact_output(outputs: &BTreeMap<u16, ValueType>, formal: u16, value_type: ValueType) -> bool {
    outputs.get(&formal) == Some(&value_type)
}

fn outputs_without_status(outputs: &BTreeMap<u16, ValueType>) -> usize {
    outputs.len() - usize::from(outputs.contains_key(&FORMAL_ENABLE_OUTPUT))
}

fn invalid_invocation<T>(block: BlockId, operation_id: u32) -> Result<T, ArtifactError> {
    Err(ArtifactError::InvalidInvocation {
        block,
        operation_id,
    })
}

fn validate_block_call(
    spec: &ArtifactSpec,
    block: BlockId,
    operation_id: u32,
    call: &RuntimeBlockCall,
) -> Result<(), ArtifactError> {
    if call.target_identity == 0
        || call.call_site_identity == 0
        || call
            .inputs
            .windows(2)
            .any(|pair| pair[0].formal >= pair[1].formal)
        || call
            .outputs
            .windows(2)
            .any(|pair| pair[0].formal >= pair[1].formal)
        || call
            .frame_members
            .windows(2)
            .any(|pair| pair[0].formal == pair[1].formal)
    {
        return invalid_invocation(block, operation_id);
    }
    if runtime_block_signature_fingerprint(call.target_identity, &call.frame_members)
        != call.signature_fingerprint
    {
        return Err(ArtifactError::InvalidBlockSignature {
            block,
            operation_id,
        });
    }
    let instance_valid = match (&call.kind, &call.instance) {
        (RuntimeCallKind::Function, None) => true,
        (RuntimeCallKind::FunctionBlock, Some(instance)) => {
            instance.root_instance != 0
                && instance.multi_instance_slots.iter().all(|slot| *slot != 0)
        }
        _ => false,
    };
    if !instance_valid {
        return invalid_invocation(block, operation_id);
    }
    let mut members = BTreeMap::new();
    let mut member_memories = BTreeSet::new();
    for member in &call.frame_members {
        if member.formal == 0
            || member.initial_value.value_type() != member.value_type
            || memory_type(spec, member.memory)? != member.value_type
            || !member_memories.insert(member.memory)
            || members.insert(member.formal, *member).is_some()
        {
            return invalid_invocation(block, operation_id);
        }
    }
    for input in &call.inputs {
        let RuntimeFormalRef::BlockMember(formal) = input.formal else {
            return invalid_invocation(block, operation_id);
        };
        let Some(member) = members.get(&formal) else {
            return invalid_invocation(block, operation_id);
        };
        if !matches!(
            member.role,
            RuntimeFrameMemberRole::Input | RuntimeFrameMemberRole::InOut
        ) || operand_type(spec, input.source)? != member.value_type
        {
            return invalid_invocation(block, operation_id);
        }
    }
    let mut status_outputs = BTreeMap::new();
    for output in &call.outputs {
        match output.formal {
            RuntimeFormalRef::BlockMember(formal) => {
                let Some(member) = members.get(&formal) else {
                    return invalid_invocation(block, operation_id);
                };
                if !matches!(
                    member.role,
                    RuntimeFrameMemberRole::Output
                        | RuntimeFrameMemberRole::InOut
                        | RuntimeFrameMemberRole::Return
                ) || member.value_type != output.value_type
                {
                    return invalid_invocation(block, operation_id);
                }
            }
            RuntimeFormalRef::Instruction(formal) => {
                if formal != FORMAL_ENABLE_OUTPUT || output.value_type != ValueType::Bool {
                    return invalid_invocation(block, operation_id);
                }
                status_outputs.insert(formal, output.value_type);
            }
        }
    }
    validate_activation(
        spec,
        block,
        operation_id,
        call.activation,
        Some(RuntimeDisabledBehavior::SuppressEffects),
        &status_outputs,
    )
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
        changed.schema_version = 3;
        let transported =
            ArtifactPackage::from_untrusted_package(changed, package.fingerprint(), true);
        assert!(matches!(
            VerifiedArtifact::accept(&transported),
            Err(ArtifactError::UnsupportedSchema(3))
        ));
    }

    #[test]
    fn package_validation_rejects_out_of_range_control_flow_targets() {
        let mut spec = empty_spec();
        spec.program.cyclic.instructions.push(Instruction::new(
            41,
            0,
            Operation::Jump { target: 1 },
        ));
        assert_eq!(
            ArtifactPackage::seal_verified(spec),
            Err(ArtifactError::InvalidControlFlow {
                block: BlockId(1),
                operation_id: 41,
            })
        );
    }
}
