#![no_std]
#![forbid(unsafe_code)]

//! Canonical, capability-free Phase 2 PLC program organization model.
//!
//! This crate describes controller programs, interfaces, calls, state ownership,
//! dependency graphs, instruction metadata, validation, and deterministic
//! invalidation. It deliberately contains no parser, compiler, executor, file
//! access, clock, network, process, device, or entropy capability.

extern crate alloc;

mod ids;
mod instruction;
mod invalidation;
mod model;
mod types;
mod validation;

pub use ids::{BlockId, CallSiteId, ControllerId, InstructionUseId, InterfaceMemberId};
pub use instruction::{
    ADD, BOOL_AND, BOOL_NOT, BOOL_OR, BOOL_XOR, BRANCH, BREAKPOINT_MARKER, BoundInstructionFormal,
    BoundInstructionSignature, CALL_FB, CALL_FC, COMPARE_EQ, COMPARE_GE, COMPARE_GT, COMPARE_LE,
    COMPARE_LT, COMPARE_NE, COUNTER_DOWN, COUNTER_UP, COUNTER_UP_DOWN, DIVIDE,
    DisabledExecutionBehavior, FALLING_EDGE, FORMAL_CLOCK, FORMAL_COUNT_DOWN, FORMAL_COUNT_UP,
    FORMAL_CURRENT_VALUE, FORMAL_ELAPSED_TIME, FORMAL_ENABLE, FORMAL_ENABLE_OUTPUT, FORMAL_INPUT,
    FORMAL_LEFT, FORMAL_LOAD, FORMAL_OUTPUT, FORMAL_PRESET_TIME, FORMAL_PRESET_VALUE, FORMAL_QD,
    FORMAL_QU, FORMAL_RESET, FORMAL_RIGHT, FORMAL_STATE, InstructionActivationPolicy,
    InstructionBindingError, InstructionCategory, InstructionCode, InstructionDefinition,
    InstructionFormalDefinition, InstructionFormalDirection, InstructionFormalId,
    InstructionLvalueConstraint, InstructionRegistry, InstructionRegistryError,
    InstructionTypeConstraint, JUMP, MODULO, MOVE, MULTIPLY, NO_OP,
    PHASE2_INSTRUCTION_REGISTRY_VERSION, PROBE, RETURN, RISING_EDGE, SUBTRACT, SideEffectClass,
    StateKind, StateRequirement, TIMER_OFF_DELAY, TIMER_ON_DELAY, TIMER_PULSE, TRACE_SAMPLE,
    phase2_instruction_registry,
};
pub use invalidation::{
    InterfaceDelta, InvalidationCode, InvalidationError, InvalidationExplanation, InvalidationPlan,
};
pub use model::{
    BindingActual, CallSite, ControllerProgram, DataBlockKind, DependencyEdge, DependencyGraph,
    DependencyReason, InstanceOwner, InstancePath, InstructionUse, ObDeclaration, ParameterBinding,
    ProgramBlock, ProgramEditError, ProgramUnitKind, VariableRef,
};
pub use plc_types::{CanonicalF32, CanonicalF64, PrimitiveCategory, PrimitiveType};
pub use types::{
    BlockInterface, CanonicalValue, DataType, EngineeringNumber, InterfaceMember, InterfaceRole,
    RetainPolicy,
};
pub use validation::{
    CallGraph, CallGraphEdge, IssueCode, ProgramIssue, ValidationReport, validate_program,
};

/// Canonical schema version for the P2-04 program aggregate.
pub const PROGRAM_MODEL_SCHEMA_VERSION: u16 = 1;

/// Defensive model bounds. They are validation limits, not execution budgets.
pub const MAX_BLOCKS_PER_CONTROLLER: usize = 16_384;
pub const MAX_INTERFACE_MEMBERS_PER_BLOCK: usize = 4_096;
pub const MAX_CALLS_PER_BLOCK: usize = 8_192;
pub const MAX_INSTRUCTION_USES_PER_BLOCK: usize = 65_536;
