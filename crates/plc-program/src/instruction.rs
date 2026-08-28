use alloc::{collections::BTreeSet, vec::Vec};

use crate::DataType;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstructionCode(pub u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstructionFormalId(pub u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StateKind {
    Edge,
    Timer,
    Counter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateRequirement {
    None,
    Explicit(StateKind),
    FunctionBlockInstance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum InstructionCategory {
    Stateless,
    Edge,
    Timer,
    Counter,
    Call,
    Control,
    Instrumentation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SideEffectClass {
    Pure,
    WritesValue,
    WritesState,
    CallsBlock,
    ControlsFlow,
    ObservesOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InstructionFormalDirection {
    Input,
    Output,
    InOut,
    Activation,
    Status,
    State,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InstructionTypeConstraint {
    Bool,
    Int,
    DInt,
    Real,
    Time,
    String,
    Numeric,
    Integer,
    AnyValue,
    SameAs(InstructionFormalId),
    InstructionState(StateKind),
    FunctionBlockInstance,
}

impl InstructionTypeConstraint {
    #[must_use]
    pub fn accepts(
        self,
        candidate: &DataType,
        bound_formals: &[(InstructionFormalId, DataType)],
    ) -> bool {
        match self {
            Self::Bool => candidate == &DataType::Bool,
            Self::Int => candidate == &DataType::Int,
            Self::DInt => candidate == &DataType::DInt,
            Self::Real => candidate == &DataType::Real,
            Self::Time => candidate == &DataType::Time,
            Self::String => matches!(candidate, DataType::String { .. }),
            Self::Numeric => candidate
                .primitive_type()
                .is_some_and(plc_types::PrimitiveType::is_numeric),
            Self::Integer => candidate
                .primitive_type()
                .is_some_and(plc_types::PrimitiveType::is_integer),
            Self::AnyValue => candidate.primitive_type().is_some(),
            Self::SameAs(formal) => bound_formals
                .iter()
                .find(|(candidate_id, _)| *candidate_id == formal)
                .is_some_and(|(_, expected)| expected == candidate),
            Self::InstructionState(kind) => candidate == &DataType::InstructionState(kind),
            Self::FunctionBlockInstance => matches!(candidate, DataType::BlockInstance(_)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InstructionLvalueConstraint {
    Value,
    Writable,
    ReadWrite,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstructionFormalDefinition {
    pub id: InstructionFormalId,
    pub name: &'static str,
    pub direction: InstructionFormalDirection,
    pub type_constraint: InstructionTypeConstraint,
    pub required: bool,
    pub lvalue: InstructionLvalueConstraint,
}

/// One canonical type binding in a registry-owned instruction instantiation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoundInstructionFormal {
    pub formal: InstructionFormalId,
    pub data_type: DataType,
}

/// Deterministic structural identity for a fully bound instruction signature.
/// The bindings are sorted by stable formal identity and contain every
/// required formal, including explicit state.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoundInstructionSignature {
    instruction: InstructionCode,
    formals: Vec<BoundInstructionFormal>,
}

impl BoundInstructionSignature {
    #[must_use]
    pub const fn instruction(&self) -> InstructionCode {
        self.instruction
    }

    #[must_use]
    pub fn formals(&self) -> &[BoundInstructionFormal] {
        &self.formals
    }

    #[must_use]
    pub fn data_type(&self, formal: InstructionFormalId) -> Option<&DataType> {
        self.formals
            .binary_search_by_key(&formal, |binding| binding.formal)
            .ok()
            .map(|index| &self.formals[index].data_type)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstructionBindingError {
    UnknownInstruction(InstructionCode),
    UnknownFormal(InstructionCode, InstructionFormalId),
    DuplicateFormal(InstructionCode, InstructionFormalId),
    MissingRequiredFormal(InstructionCode, InstructionFormalId),
    TypeConstraint(InstructionCode, InstructionFormalId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DisabledExecutionBehavior {
    DefaultOutputsNoStateChange,
    PreserveOutputsNoStateChange,
    SuppressEffects,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstructionActivationPolicy {
    None,
    EnableStatus {
        enable: InstructionFormalId,
        status: InstructionFormalId,
        status_when_disabled: bool,
        when_disabled: DisabledExecutionBehavior,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstructionDefinition {
    pub code: InstructionCode,
    pub mnemonic: &'static str,
    pub category: InstructionCategory,
    pub state_requirement: StateRequirement,
    pub side_effect: SideEffectClass,
    pub work_units: u16,
    pub formals: &'static [InstructionFormalDefinition],
    pub activation: InstructionActivationPolicy,
}

impl InstructionDefinition {
    #[must_use]
    pub fn formal(self, id: InstructionFormalId) -> Option<&'static InstructionFormalDefinition> {
        self.formals.iter().find(|formal| formal.id == id)
    }

    #[must_use]
    pub fn formal_by_name(self, name: &str) -> Option<&'static InstructionFormalDefinition> {
        self.formals
            .iter()
            .find(|formal| formal.name.eq_ignore_ascii_case(name))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstructionRegistryError {
    DefinitionOrder,
    DuplicateFormalId(InstructionCode, InstructionFormalId),
    DuplicateFormalName(InstructionCode),
    InvalidSameAs(InstructionCode, InstructionFormalId),
    InvalidFormal(InstructionCode, InstructionFormalId),
    InvalidActivation(InstructionCode),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstructionRegistry {
    pub schema_version: u16,
    pub semantic_version: &'static str,
    definitions: &'static [InstructionDefinition],
}

impl InstructionRegistry {
    #[must_use]
    pub const fn definitions(self) -> &'static [InstructionDefinition] {
        self.definitions
    }

    #[must_use]
    pub fn lookup(self, code: InstructionCode) -> Option<&'static InstructionDefinition> {
        self.definitions
            .binary_search_by_key(&code, |definition| definition.code)
            .ok()
            .map(|index| &self.definitions[index])
    }

    /// Validates canonical definition order plus every formal and activation
    /// invariant in this immutable registry.
    ///
    /// # Errors
    ///
    /// Returns the first deterministic schema defect.
    pub fn validate(self) -> Result<(), InstructionRegistryError> {
        if self
            .definitions
            .windows(2)
            .any(|pair| pair[0].code >= pair[1].code)
        {
            return Err(InstructionRegistryError::DefinitionOrder);
        }
        for definition in self.definitions {
            validate_definition(definition)?;
        }
        Ok(())
    }

    /// Canonicalizes and validates a typed instruction instantiation against
    /// the single shipped registry. Frontends may retain incomplete authored
    /// graphs, but verified IR must carry a successfully bound signature.
    ///
    /// # Errors
    ///
    /// Returns a stable unknown, duplicate, missing, or type-constraint error
    /// without producing a partial bound signature.
    pub fn bind_types(
        self,
        instruction: InstructionCode,
        bindings: impl IntoIterator<Item = BoundInstructionFormal>,
    ) -> Result<BoundInstructionSignature, InstructionBindingError> {
        let definition = self
            .lookup(instruction)
            .ok_or(InstructionBindingError::UnknownInstruction(instruction))?;
        let mut formals: Vec<_> = bindings.into_iter().collect();
        formals.sort_by_key(|binding| binding.formal);
        for pair in formals.windows(2) {
            if pair[0].formal == pair[1].formal {
                return Err(InstructionBindingError::DuplicateFormal(
                    instruction,
                    pair[0].formal,
                ));
            }
        }
        for binding in &formals {
            if definition.formal(binding.formal).is_none() {
                return Err(InstructionBindingError::UnknownFormal(
                    instruction,
                    binding.formal,
                ));
            }
        }
        for formal in definition.formals.iter().filter(|formal| formal.required) {
            if formals
                .binary_search_by_key(&formal.id, |binding| binding.formal)
                .is_err()
            {
                return Err(InstructionBindingError::MissingRequiredFormal(
                    instruction,
                    formal.id,
                ));
            }
        }
        let type_pairs: Vec<_> = formals
            .iter()
            .map(|binding| (binding.formal, binding.data_type.clone()))
            .collect();
        for binding in &formals {
            let Some(formal) = definition.formal(binding.formal) else {
                return Err(InstructionBindingError::UnknownFormal(
                    instruction,
                    binding.formal,
                ));
            };
            if !formal
                .type_constraint
                .accepts(&binding.data_type, &type_pairs)
            {
                return Err(InstructionBindingError::TypeConstraint(
                    instruction,
                    binding.formal,
                ));
            }
        }
        Ok(BoundInstructionSignature {
            instruction,
            formals,
        })
    }
}

pub const PHASE2_INSTRUCTION_REGISTRY_VERSION: &str = "EDU-INSTRUCTION-REGISTRY-2.1.0";

pub const NO_OP: InstructionCode = InstructionCode(0x0001);
pub const MOVE: InstructionCode = InstructionCode(0x0002);
pub const BOOL_NOT: InstructionCode = InstructionCode(0x0010);
pub const BOOL_AND: InstructionCode = InstructionCode(0x0011);
pub const BOOL_OR: InstructionCode = InstructionCode(0x0012);
pub const BOOL_XOR: InstructionCode = InstructionCode(0x0013);
pub const COMPARE_EQ: InstructionCode = InstructionCode(0x0020);
pub const COMPARE_NE: InstructionCode = InstructionCode(0x0021);
pub const COMPARE_LT: InstructionCode = InstructionCode(0x0022);
pub const COMPARE_LE: InstructionCode = InstructionCode(0x0023);
pub const COMPARE_GT: InstructionCode = InstructionCode(0x0024);
pub const COMPARE_GE: InstructionCode = InstructionCode(0x0025);
pub const ADD: InstructionCode = InstructionCode(0x0030);
pub const SUBTRACT: InstructionCode = InstructionCode(0x0031);
pub const MULTIPLY: InstructionCode = InstructionCode(0x0032);
pub const DIVIDE: InstructionCode = InstructionCode(0x0033);
pub const MODULO: InstructionCode = InstructionCode(0x0034);
pub const RISING_EDGE: InstructionCode = InstructionCode(0x0100);
pub const FALLING_EDGE: InstructionCode = InstructionCode(0x0101);
pub const TIMER_ON_DELAY: InstructionCode = InstructionCode(0x0110);
pub const TIMER_OFF_DELAY: InstructionCode = InstructionCode(0x0111);
pub const TIMER_PULSE: InstructionCode = InstructionCode(0x0112);
pub const COUNTER_UP: InstructionCode = InstructionCode(0x0120);
pub const COUNTER_DOWN: InstructionCode = InstructionCode(0x0121);
pub const COUNTER_UP_DOWN: InstructionCode = InstructionCode(0x0122);
pub const CALL_FC: InstructionCode = InstructionCode(0x0200);
pub const CALL_FB: InstructionCode = InstructionCode(0x0201);
pub const BRANCH: InstructionCode = InstructionCode(0x0300);
pub const JUMP: InstructionCode = InstructionCode(0x0301);
pub const RETURN: InstructionCode = InstructionCode(0x0302);
pub const PROBE: InstructionCode = InstructionCode(0x0400);
pub const TRACE_SAMPLE: InstructionCode = InstructionCode(0x0401);
pub const BREAKPOINT_MARKER: InstructionCode = InstructionCode(0x0402);

pub const FORMAL_ENABLE: InstructionFormalId = InstructionFormalId(0x0001);
pub const FORMAL_ENABLE_OUTPUT: InstructionFormalId = InstructionFormalId(0x0002);
pub const FORMAL_INPUT: InstructionFormalId = InstructionFormalId(0x0010);
pub const FORMAL_OUTPUT: InstructionFormalId = InstructionFormalId(0x0011);
pub const FORMAL_LEFT: InstructionFormalId = InstructionFormalId(0x0020);
pub const FORMAL_RIGHT: InstructionFormalId = InstructionFormalId(0x0021);
pub const FORMAL_CLOCK: InstructionFormalId = InstructionFormalId(0x0030);
pub const FORMAL_PRESET_TIME: InstructionFormalId = InstructionFormalId(0x0031);
pub const FORMAL_ELAPSED_TIME: InstructionFormalId = InstructionFormalId(0x0032);
pub const FORMAL_COUNT_UP: InstructionFormalId = InstructionFormalId(0x0040);
pub const FORMAL_COUNT_DOWN: InstructionFormalId = InstructionFormalId(0x0041);
pub const FORMAL_RESET: InstructionFormalId = InstructionFormalId(0x0042);
pub const FORMAL_LOAD: InstructionFormalId = InstructionFormalId(0x0043);
pub const FORMAL_PRESET_VALUE: InstructionFormalId = InstructionFormalId(0x0044);
pub const FORMAL_CURRENT_VALUE: InstructionFormalId = InstructionFormalId(0x0045);
pub const FORMAL_QU: InstructionFormalId = InstructionFormalId(0x0046);
pub const FORMAL_QD: InstructionFormalId = InstructionFormalId(0x0047);
pub const FORMAL_STATE: InstructionFormalId = InstructionFormalId(0x00ff);

macro_rules! formal {
    ($id:ident, $name:literal, $direction:ident, $constraint:expr, $required:expr, $lvalue:ident) => {
        InstructionFormalDefinition {
            id: $id,
            name: $name,
            direction: InstructionFormalDirection::$direction,
            type_constraint: $constraint,
            required: $required,
            lvalue: InstructionLvalueConstraint::$lvalue,
        }
    };
}

const ENABLE: InstructionFormalDefinition = formal!(
    FORMAL_ENABLE,
    "EN",
    Activation,
    InstructionTypeConstraint::Bool,
    false,
    Value
);
const ENABLE_OUTPUT: InstructionFormalDefinition = formal!(
    FORMAL_ENABLE_OUTPUT,
    "ENO",
    Status,
    InstructionTypeConstraint::Bool,
    false,
    Writable
);

static EMPTY_FORMALS: [InstructionFormalDefinition; 0] = [];
static MOVE_FORMALS: [InstructionFormalDefinition; 4] = [
    ENABLE,
    formal!(
        FORMAL_INPUT,
        "IN",
        Input,
        InstructionTypeConstraint::AnyValue,
        true,
        Value
    ),
    formal!(
        FORMAL_OUTPUT,
        "OUT",
        Output,
        InstructionTypeConstraint::SameAs(FORMAL_INPUT),
        true,
        Writable
    ),
    ENABLE_OUTPUT,
];
static BOOL_UNARY_FORMALS: [InstructionFormalDefinition; 4] = [
    ENABLE,
    formal!(
        FORMAL_INPUT,
        "IN",
        Input,
        InstructionTypeConstraint::Bool,
        true,
        Value
    ),
    formal!(
        FORMAL_OUTPUT,
        "OUT",
        Output,
        InstructionTypeConstraint::Bool,
        true,
        Writable
    ),
    ENABLE_OUTPUT,
];
static BOOL_BINARY_FORMALS: [InstructionFormalDefinition; 5] = [
    ENABLE,
    formal!(
        FORMAL_LEFT,
        "A",
        Input,
        InstructionTypeConstraint::Bool,
        true,
        Value
    ),
    formal!(
        FORMAL_RIGHT,
        "B",
        Input,
        InstructionTypeConstraint::Bool,
        true,
        Value
    ),
    formal!(
        FORMAL_OUTPUT,
        "OUT",
        Output,
        InstructionTypeConstraint::Bool,
        true,
        Writable
    ),
    ENABLE_OUTPUT,
];
static COMPARE_FORMALS: [InstructionFormalDefinition; 5] = [
    ENABLE,
    formal!(
        FORMAL_LEFT,
        "A",
        Input,
        InstructionTypeConstraint::AnyValue,
        true,
        Value
    ),
    formal!(
        FORMAL_RIGHT,
        "B",
        Input,
        InstructionTypeConstraint::SameAs(FORMAL_LEFT),
        true,
        Value
    ),
    formal!(
        FORMAL_OUTPUT,
        "OUT",
        Output,
        InstructionTypeConstraint::Bool,
        true,
        Writable
    ),
    ENABLE_OUTPUT,
];
static NUMERIC_FORMALS: [InstructionFormalDefinition; 5] = [
    ENABLE,
    formal!(
        FORMAL_LEFT,
        "A",
        Input,
        InstructionTypeConstraint::Numeric,
        true,
        Value
    ),
    formal!(
        FORMAL_RIGHT,
        "B",
        Input,
        InstructionTypeConstraint::SameAs(FORMAL_LEFT),
        true,
        Value
    ),
    formal!(
        FORMAL_OUTPUT,
        "OUT",
        Output,
        InstructionTypeConstraint::SameAs(FORMAL_LEFT),
        true,
        Writable
    ),
    ENABLE_OUTPUT,
];
static INTEGER_FORMALS: [InstructionFormalDefinition; 5] = [
    ENABLE,
    formal!(
        FORMAL_LEFT,
        "A",
        Input,
        InstructionTypeConstraint::Integer,
        true,
        Value
    ),
    formal!(
        FORMAL_RIGHT,
        "B",
        Input,
        InstructionTypeConstraint::SameAs(FORMAL_LEFT),
        true,
        Value
    ),
    formal!(
        FORMAL_OUTPUT,
        "OUT",
        Output,
        InstructionTypeConstraint::SameAs(FORMAL_LEFT),
        true,
        Writable
    ),
    ENABLE_OUTPUT,
];
static EDGE_FORMALS: [InstructionFormalDefinition; 5] = [
    ENABLE,
    formal!(
        FORMAL_CLOCK,
        "CLK",
        Input,
        InstructionTypeConstraint::Bool,
        true,
        Value
    ),
    formal!(
        FORMAL_OUTPUT,
        "Q",
        Output,
        InstructionTypeConstraint::Bool,
        true,
        Writable
    ),
    ENABLE_OUTPUT,
    formal!(
        FORMAL_STATE,
        "STATE",
        State,
        InstructionTypeConstraint::InstructionState(StateKind::Edge),
        true,
        ReadWrite
    ),
];
static TIMER_FORMALS: [InstructionFormalDefinition; 7] = [
    ENABLE,
    formal!(
        FORMAL_INPUT,
        "IN",
        Input,
        InstructionTypeConstraint::Bool,
        true,
        Value
    ),
    formal!(
        FORMAL_PRESET_TIME,
        "PT",
        Input,
        InstructionTypeConstraint::Time,
        true,
        Value
    ),
    formal!(
        FORMAL_OUTPUT,
        "Q",
        Output,
        InstructionTypeConstraint::Bool,
        true,
        Writable
    ),
    formal!(
        FORMAL_ELAPSED_TIME,
        "ET",
        Output,
        InstructionTypeConstraint::Time,
        true,
        Writable
    ),
    ENABLE_OUTPUT,
    formal!(
        FORMAL_STATE,
        "STATE",
        State,
        InstructionTypeConstraint::InstructionState(StateKind::Timer),
        true,
        ReadWrite
    ),
];
static COUNTER_UP_FORMALS: [InstructionFormalDefinition; 8] = [
    ENABLE,
    formal!(
        FORMAL_COUNT_UP,
        "CU",
        Input,
        InstructionTypeConstraint::Bool,
        true,
        Value
    ),
    formal!(
        FORMAL_RESET,
        "R",
        Input,
        InstructionTypeConstraint::Bool,
        true,
        Value
    ),
    formal!(
        FORMAL_PRESET_VALUE,
        "PV",
        Input,
        InstructionTypeConstraint::DInt,
        true,
        Value
    ),
    formal!(
        FORMAL_OUTPUT,
        "Q",
        Output,
        InstructionTypeConstraint::Bool,
        true,
        Writable
    ),
    formal!(
        FORMAL_CURRENT_VALUE,
        "CV",
        Output,
        InstructionTypeConstraint::DInt,
        true,
        Writable
    ),
    ENABLE_OUTPUT,
    formal!(
        FORMAL_STATE,
        "STATE",
        State,
        InstructionTypeConstraint::InstructionState(StateKind::Counter),
        true,
        ReadWrite
    ),
];
static COUNTER_DOWN_FORMALS: [InstructionFormalDefinition; 8] = [
    ENABLE,
    formal!(
        FORMAL_COUNT_DOWN,
        "CD",
        Input,
        InstructionTypeConstraint::Bool,
        true,
        Value
    ),
    formal!(
        FORMAL_LOAD,
        "LD",
        Input,
        InstructionTypeConstraint::Bool,
        true,
        Value
    ),
    formal!(
        FORMAL_PRESET_VALUE,
        "PV",
        Input,
        InstructionTypeConstraint::DInt,
        true,
        Value
    ),
    formal!(
        FORMAL_OUTPUT,
        "Q",
        Output,
        InstructionTypeConstraint::Bool,
        true,
        Writable
    ),
    formal!(
        FORMAL_CURRENT_VALUE,
        "CV",
        Output,
        InstructionTypeConstraint::DInt,
        true,
        Writable
    ),
    ENABLE_OUTPUT,
    formal!(
        FORMAL_STATE,
        "STATE",
        State,
        InstructionTypeConstraint::InstructionState(StateKind::Counter),
        true,
        ReadWrite
    ),
];
static COUNTER_UP_DOWN_FORMALS: [InstructionFormalDefinition; 11] = [
    ENABLE,
    formal!(
        FORMAL_COUNT_UP,
        "CU",
        Input,
        InstructionTypeConstraint::Bool,
        true,
        Value
    ),
    formal!(
        FORMAL_COUNT_DOWN,
        "CD",
        Input,
        InstructionTypeConstraint::Bool,
        true,
        Value
    ),
    formal!(
        FORMAL_RESET,
        "R",
        Input,
        InstructionTypeConstraint::Bool,
        true,
        Value
    ),
    formal!(
        FORMAL_LOAD,
        "LD",
        Input,
        InstructionTypeConstraint::Bool,
        true,
        Value
    ),
    formal!(
        FORMAL_PRESET_VALUE,
        "PV",
        Input,
        InstructionTypeConstraint::DInt,
        true,
        Value
    ),
    formal!(
        FORMAL_QU,
        "QU",
        Output,
        InstructionTypeConstraint::Bool,
        true,
        Writable
    ),
    formal!(
        FORMAL_QD,
        "QD",
        Output,
        InstructionTypeConstraint::Bool,
        true,
        Writable
    ),
    formal!(
        FORMAL_CURRENT_VALUE,
        "CV",
        Output,
        InstructionTypeConstraint::DInt,
        true,
        Writable
    ),
    ENABLE_OUTPUT,
    formal!(
        FORMAL_STATE,
        "STATE",
        State,
        InstructionTypeConstraint::InstructionState(StateKind::Counter),
        true,
        ReadWrite
    ),
];
static CALL_FC_FORMALS: [InstructionFormalDefinition; 2] = [ENABLE, ENABLE_OUTPUT];
static CALL_FB_FORMALS: [InstructionFormalDefinition; 3] = [
    ENABLE,
    ENABLE_OUTPUT,
    formal!(
        FORMAL_STATE,
        "INSTANCE",
        State,
        InstructionTypeConstraint::FunctionBlockInstance,
        true,
        ReadWrite
    ),
];
static OBSERVE_FORMALS: [InstructionFormalDefinition; 1] = [formal!(
    FORMAL_INPUT,
    "IN",
    Input,
    InstructionTypeConstraint::AnyValue,
    true,
    Value
)];

const fn activation(when_disabled: DisabledExecutionBehavior) -> InstructionActivationPolicy {
    InstructionActivationPolicy::EnableStatus {
        enable: FORMAL_ENABLE,
        status: FORMAL_ENABLE_OUTPUT,
        status_when_disabled: false,
        when_disabled,
    }
}

macro_rules! definition {
    ($code:ident, $mnemonic:literal, $category:ident, $state:expr, $effect:ident, $formals:ident, $activation:expr) => {
        InstructionDefinition {
            code: $code,
            mnemonic: $mnemonic,
            category: InstructionCategory::$category,
            state_requirement: $state,
            side_effect: SideEffectClass::$effect,
            work_units: 1,
            formals: &$formals,
            activation: $activation,
        }
    };
    ($code:ident, $mnemonic:literal, $category:ident, $state:expr, $effect:ident) => {
        definition!(
            $code,
            $mnemonic,
            $category,
            $state,
            $effect,
            EMPTY_FORMALS,
            InstructionActivationPolicy::None
        )
    };
}

static DEFINITIONS: [InstructionDefinition; 33] = [
    definition!(NO_OP, "NO_OP", Stateless, StateRequirement::None, Pure),
    definition!(
        MOVE,
        "MOVE",
        Stateless,
        StateRequirement::None,
        WritesValue,
        MOVE_FORMALS,
        activation(DisabledExecutionBehavior::SuppressEffects)
    ),
    definition!(
        BOOL_NOT,
        "NOT",
        Stateless,
        StateRequirement::None,
        Pure,
        BOOL_UNARY_FORMALS,
        activation(DisabledExecutionBehavior::DefaultOutputsNoStateChange)
    ),
    definition!(
        BOOL_AND,
        "AND",
        Stateless,
        StateRequirement::None,
        Pure,
        BOOL_BINARY_FORMALS,
        activation(DisabledExecutionBehavior::DefaultOutputsNoStateChange)
    ),
    definition!(
        BOOL_OR,
        "OR",
        Stateless,
        StateRequirement::None,
        Pure,
        BOOL_BINARY_FORMALS,
        activation(DisabledExecutionBehavior::DefaultOutputsNoStateChange)
    ),
    definition!(
        BOOL_XOR,
        "XOR",
        Stateless,
        StateRequirement::None,
        Pure,
        BOOL_BINARY_FORMALS,
        activation(DisabledExecutionBehavior::DefaultOutputsNoStateChange)
    ),
    definition!(
        COMPARE_EQ,
        "EQ",
        Stateless,
        StateRequirement::None,
        Pure,
        COMPARE_FORMALS,
        activation(DisabledExecutionBehavior::DefaultOutputsNoStateChange)
    ),
    definition!(
        COMPARE_NE,
        "NE",
        Stateless,
        StateRequirement::None,
        Pure,
        COMPARE_FORMALS,
        activation(DisabledExecutionBehavior::DefaultOutputsNoStateChange)
    ),
    definition!(
        COMPARE_LT,
        "LT",
        Stateless,
        StateRequirement::None,
        Pure,
        COMPARE_FORMALS,
        activation(DisabledExecutionBehavior::DefaultOutputsNoStateChange)
    ),
    definition!(
        COMPARE_LE,
        "LE",
        Stateless,
        StateRequirement::None,
        Pure,
        COMPARE_FORMALS,
        activation(DisabledExecutionBehavior::DefaultOutputsNoStateChange)
    ),
    definition!(
        COMPARE_GT,
        "GT",
        Stateless,
        StateRequirement::None,
        Pure,
        COMPARE_FORMALS,
        activation(DisabledExecutionBehavior::DefaultOutputsNoStateChange)
    ),
    definition!(
        COMPARE_GE,
        "GE",
        Stateless,
        StateRequirement::None,
        Pure,
        COMPARE_FORMALS,
        activation(DisabledExecutionBehavior::DefaultOutputsNoStateChange)
    ),
    definition!(
        ADD,
        "ADD",
        Stateless,
        StateRequirement::None,
        Pure,
        NUMERIC_FORMALS,
        activation(DisabledExecutionBehavior::DefaultOutputsNoStateChange)
    ),
    definition!(
        SUBTRACT,
        "SUB",
        Stateless,
        StateRequirement::None,
        Pure,
        NUMERIC_FORMALS,
        activation(DisabledExecutionBehavior::DefaultOutputsNoStateChange)
    ),
    definition!(
        MULTIPLY,
        "MUL",
        Stateless,
        StateRequirement::None,
        Pure,
        NUMERIC_FORMALS,
        activation(DisabledExecutionBehavior::DefaultOutputsNoStateChange)
    ),
    definition!(
        DIVIDE,
        "DIV",
        Stateless,
        StateRequirement::None,
        Pure,
        NUMERIC_FORMALS,
        activation(DisabledExecutionBehavior::DefaultOutputsNoStateChange)
    ),
    definition!(
        MODULO,
        "MOD",
        Stateless,
        StateRequirement::None,
        Pure,
        INTEGER_FORMALS,
        activation(DisabledExecutionBehavior::DefaultOutputsNoStateChange)
    ),
    definition!(
        RISING_EDGE,
        "R_TRIG",
        Edge,
        StateRequirement::Explicit(StateKind::Edge),
        WritesState,
        EDGE_FORMALS,
        activation(DisabledExecutionBehavior::DefaultOutputsNoStateChange)
    ),
    definition!(
        FALLING_EDGE,
        "F_TRIG",
        Edge,
        StateRequirement::Explicit(StateKind::Edge),
        WritesState,
        EDGE_FORMALS,
        activation(DisabledExecutionBehavior::DefaultOutputsNoStateChange)
    ),
    definition!(
        TIMER_ON_DELAY,
        "TON",
        Timer,
        StateRequirement::Explicit(StateKind::Timer),
        WritesState,
        TIMER_FORMALS,
        activation(DisabledExecutionBehavior::PreserveOutputsNoStateChange)
    ),
    definition!(
        TIMER_OFF_DELAY,
        "TOF",
        Timer,
        StateRequirement::Explicit(StateKind::Timer),
        WritesState,
        TIMER_FORMALS,
        activation(DisabledExecutionBehavior::PreserveOutputsNoStateChange)
    ),
    definition!(
        TIMER_PULSE,
        "TP",
        Timer,
        StateRequirement::Explicit(StateKind::Timer),
        WritesState,
        TIMER_FORMALS,
        activation(DisabledExecutionBehavior::PreserveOutputsNoStateChange)
    ),
    definition!(
        COUNTER_UP,
        "CTU",
        Counter,
        StateRequirement::Explicit(StateKind::Counter),
        WritesState,
        COUNTER_UP_FORMALS,
        activation(DisabledExecutionBehavior::PreserveOutputsNoStateChange)
    ),
    definition!(
        COUNTER_DOWN,
        "CTD",
        Counter,
        StateRequirement::Explicit(StateKind::Counter),
        WritesState,
        COUNTER_DOWN_FORMALS,
        activation(DisabledExecutionBehavior::PreserveOutputsNoStateChange)
    ),
    definition!(
        COUNTER_UP_DOWN,
        "CTUD",
        Counter,
        StateRequirement::Explicit(StateKind::Counter),
        WritesState,
        COUNTER_UP_DOWN_FORMALS,
        activation(DisabledExecutionBehavior::PreserveOutputsNoStateChange)
    ),
    definition!(
        CALL_FC,
        "CALL_FC",
        Call,
        StateRequirement::None,
        CallsBlock,
        CALL_FC_FORMALS,
        activation(DisabledExecutionBehavior::SuppressEffects)
    ),
    definition!(
        CALL_FB,
        "CALL_FB",
        Call,
        StateRequirement::FunctionBlockInstance,
        CallsBlock,
        CALL_FB_FORMALS,
        activation(DisabledExecutionBehavior::SuppressEffects)
    ),
    definition!(
        BRANCH,
        "BRANCH",
        Control,
        StateRequirement::None,
        ControlsFlow
    ),
    definition!(JUMP, "JUMP", Control, StateRequirement::None, ControlsFlow),
    definition!(
        RETURN,
        "RETURN",
        Control,
        StateRequirement::None,
        ControlsFlow
    ),
    definition!(
        PROBE,
        "PROBE",
        Instrumentation,
        StateRequirement::None,
        ObservesOnly,
        OBSERVE_FORMALS,
        InstructionActivationPolicy::None
    ),
    definition!(
        TRACE_SAMPLE,
        "TRACE_SAMPLE",
        Instrumentation,
        StateRequirement::None,
        ObservesOnly,
        OBSERVE_FORMALS,
        InstructionActivationPolicy::None
    ),
    definition!(
        BREAKPOINT_MARKER,
        "BREAKPOINT",
        Instrumentation,
        StateRequirement::None,
        ObservesOnly
    ),
];

fn validate_definition(definition: &InstructionDefinition) -> Result<(), InstructionRegistryError> {
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    for formal in definition.formals {
        if !ids.insert(formal.id) {
            return Err(InstructionRegistryError::DuplicateFormalId(
                definition.code,
                formal.id,
            ));
        }
        if !names.insert(formal.name.to_ascii_lowercase()) {
            return Err(InstructionRegistryError::DuplicateFormalName(
                definition.code,
            ));
        }
        let lvalue_valid = matches!(
            (formal.direction, formal.lvalue),
            (
                InstructionFormalDirection::Input | InstructionFormalDirection::Activation,
                InstructionLvalueConstraint::Value
            ) | (
                InstructionFormalDirection::Output | InstructionFormalDirection::Status,
                InstructionLvalueConstraint::Writable
            ) | (
                InstructionFormalDirection::InOut | InstructionFormalDirection::State,
                InstructionLvalueConstraint::ReadWrite
            )
        );
        if !lvalue_valid {
            return Err(InstructionRegistryError::InvalidFormal(
                definition.code,
                formal.id,
            ));
        }
        if let InstructionTypeConstraint::SameAs(referenced) = formal.type_constraint
            && (referenced == formal.id
                || !definition.formals.iter().any(|item| item.id == referenced))
        {
            return Err(InstructionRegistryError::InvalidSameAs(
                definition.code,
                formal.id,
            ));
        }
    }
    match definition.activation {
        InstructionActivationPolicy::None => {}
        InstructionActivationPolicy::EnableStatus {
            enable,
            status,
            status_when_disabled,
            ..
        } => {
            let enable = definition.formal(enable);
            let status = definition.formal(status);
            if !matches!(
                enable,
                Some(InstructionFormalDefinition {
                    direction: InstructionFormalDirection::Activation,
                    type_constraint: InstructionTypeConstraint::Bool,
                    ..
                })
            ) || !matches!(
                status,
                Some(InstructionFormalDefinition {
                    direction: InstructionFormalDirection::Status,
                    type_constraint: InstructionTypeConstraint::Bool,
                    ..
                })
            ) || status_when_disabled
            {
                return Err(InstructionRegistryError::InvalidActivation(definition.code));
            }
        }
    }
    let state_formal = definition
        .formals
        .iter()
        .find(|formal| formal.direction == InstructionFormalDirection::State);
    let state_valid = match definition.state_requirement {
        StateRequirement::None => state_formal.is_none(),
        StateRequirement::Explicit(kind) => state_formal.is_some_and(|formal| {
            formal.type_constraint == InstructionTypeConstraint::InstructionState(kind)
        }),
        StateRequirement::FunctionBlockInstance => state_formal.is_some_and(|formal| {
            formal.type_constraint == InstructionTypeConstraint::FunctionBlockInstance
        }),
    };
    if !state_valid {
        return Err(InstructionRegistryError::InvalidFormal(
            definition.code,
            FORMAL_STATE,
        ));
    }
    Ok(())
}

static REGISTRY: InstructionRegistry = InstructionRegistry {
    schema_version: 1,
    semantic_version: PHASE2_INSTRUCTION_REGISTRY_VERSION,
    definitions: &DEFINITIONS,
};

/// Returns the only Phase 2 instruction registry owned by this crate.
#[must_use]
pub const fn phase2_instruction_registry() -> &'static InstructionRegistry {
    &REGISTRY
}
