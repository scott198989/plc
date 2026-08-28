#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstructionCode(pub u16);

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstructionDefinition {
    pub code: InstructionCode,
    pub mnemonic: &'static str,
    pub category: InstructionCategory,
    pub state_requirement: StateRequirement,
    pub side_effect: SideEffectClass,
    pub work_units: u16,
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
}

pub const PHASE2_INSTRUCTION_REGISTRY_VERSION: &str = "EDU-INSTRUCTION-REGISTRY-2.0.0";

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

macro_rules! definition {
    ($code:ident, $mnemonic:literal, $category:ident, $state:expr, $effect:ident) => {
        InstructionDefinition {
            code: $code,
            mnemonic: $mnemonic,
            category: InstructionCategory::$category,
            state_requirement: $state,
            side_effect: SideEffectClass::$effect,
            work_units: 1,
        }
    };
}

static DEFINITIONS: [InstructionDefinition; 33] = [
    definition!(NO_OP, "NO_OP", Stateless, StateRequirement::None, Pure),
    definition!(MOVE, "MOVE", Stateless, StateRequirement::None, WritesValue),
    definition!(BOOL_NOT, "NOT", Stateless, StateRequirement::None, Pure),
    definition!(BOOL_AND, "AND", Stateless, StateRequirement::None, Pure),
    definition!(BOOL_OR, "OR", Stateless, StateRequirement::None, Pure),
    definition!(BOOL_XOR, "XOR", Stateless, StateRequirement::None, Pure),
    definition!(COMPARE_EQ, "EQ", Stateless, StateRequirement::None, Pure),
    definition!(COMPARE_NE, "NE", Stateless, StateRequirement::None, Pure),
    definition!(COMPARE_LT, "LT", Stateless, StateRequirement::None, Pure),
    definition!(COMPARE_LE, "LE", Stateless, StateRequirement::None, Pure),
    definition!(COMPARE_GT, "GT", Stateless, StateRequirement::None, Pure),
    definition!(COMPARE_GE, "GE", Stateless, StateRequirement::None, Pure),
    definition!(ADD, "ADD", Stateless, StateRequirement::None, Pure),
    definition!(SUBTRACT, "SUB", Stateless, StateRequirement::None, Pure),
    definition!(MULTIPLY, "MUL", Stateless, StateRequirement::None, Pure),
    definition!(DIVIDE, "DIV", Stateless, StateRequirement::None, Pure),
    definition!(MODULO, "MOD", Stateless, StateRequirement::None, Pure),
    definition!(
        RISING_EDGE,
        "R_TRIG",
        Edge,
        StateRequirement::Explicit(StateKind::Edge),
        WritesState
    ),
    definition!(
        FALLING_EDGE,
        "F_TRIG",
        Edge,
        StateRequirement::Explicit(StateKind::Edge),
        WritesState
    ),
    definition!(
        TIMER_ON_DELAY,
        "TON",
        Timer,
        StateRequirement::Explicit(StateKind::Timer),
        WritesState
    ),
    definition!(
        TIMER_OFF_DELAY,
        "TOF",
        Timer,
        StateRequirement::Explicit(StateKind::Timer),
        WritesState
    ),
    definition!(
        TIMER_PULSE,
        "TP",
        Timer,
        StateRequirement::Explicit(StateKind::Timer),
        WritesState
    ),
    definition!(
        COUNTER_UP,
        "CTU",
        Counter,
        StateRequirement::Explicit(StateKind::Counter),
        WritesState
    ),
    definition!(
        COUNTER_DOWN,
        "CTD",
        Counter,
        StateRequirement::Explicit(StateKind::Counter),
        WritesState
    ),
    definition!(
        COUNTER_UP_DOWN,
        "CTUD",
        Counter,
        StateRequirement::Explicit(StateKind::Counter),
        WritesState
    ),
    definition!(CALL_FC, "CALL_FC", Call, StateRequirement::None, CallsBlock),
    definition!(
        CALL_FB,
        "CALL_FB",
        Call,
        StateRequirement::FunctionBlockInstance,
        CallsBlock
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
        ObservesOnly
    ),
    definition!(
        TRACE_SAMPLE,
        "TRACE_SAMPLE",
        Instrumentation,
        StateRequirement::None,
        ObservesOnly
    ),
    definition!(
        BREAKPOINT_MARKER,
        "BREAKPOINT",
        Instrumentation,
        StateRequirement::None,
        ObservesOnly
    ),
];

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
