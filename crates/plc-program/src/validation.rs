use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::String,
    vec,
    vec::Vec,
};

use crate::{
    BindingActual, BlockId, CallSiteId, ControllerProgram, DataBlockKind, DataType, DependencyEdge,
    DependencyGraph, DependencyReason, InstanceOwner, InstructionUseId, InterfaceMember,
    InterfaceMemberId, InterfaceRole, MAX_BLOCKS_PER_CONTROLLER, MAX_CALLS_PER_BLOCK,
    MAX_INSTRUCTION_USES_PER_BLOCK, MAX_INTERFACE_MEMBERS_PER_BLOCK,
    PHASE2_INSTRUCTION_REGISTRY_VERSION, PROGRAM_MODEL_SCHEMA_VERSION, ProgramBlock,
    ProgramUnitKind, StateRequirement, VariableRef,
    instruction::{CALL_FB, CALL_FC, InstructionCategory, phase2_instruction_registry},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IssueCode {
    ModelSchemaMismatch,
    RegistryVersionMismatch,
    ModelLimitExceeded,
    BlockKeyMismatch,
    InvalidIdentifier,
    DuplicateEngineeringNumber,
    MissingCyclicMain,
    MultipleCyclicMain,
    MultipleStartup,
    InvalidTimedCyclic,
    InterfaceLimitExceeded,
    InterfaceKeyMismatch,
    InterfaceOrderMismatch,
    DuplicateMemberName,
    DuplicateDeclaredOrder,
    MultipleReturn,
    RoleNotAllowed,
    MemberMetadataIllegal,
    MemberValueTypeMismatch,
    InstanceTypeIllegal,
    BodyNotAllowed,
    DuplicateInstructionUseId,
    InstructionOrderMismatch,
    UnknownInstruction,
    CallInstructionInBody,
    MissingInstructionState,
    UnexpectedInstructionState,
    UnknownInstructionState,
    InstructionStateTypeMismatch,
    InstructionStateAlias,
    DuplicateCallSiteId,
    CallOrderMismatch,
    IllegalCaller,
    MissingCallee,
    IllegalCallee,
    CallInstructionMismatch,
    MissingInstanceOwner,
    UnexpectedInstanceOwner,
    InvalidInstanceDb,
    InstanceTypeMismatch,
    InvalidMultiInstance,
    UnknownFormal,
    DuplicateBinding,
    BindingOrderMismatch,
    MissingBinding,
    BindingDirection,
    UnknownActual,
    BindingTypeMismatch,
    AliasConflict,
    RecursiveCallCycle,
    StateOwnershipCycle,
}

/// A stable, localization-free validation record. UI prose is derived from the
/// code and structured identities outside this crate.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProgramIssue {
    pub code: IssueCode,
    pub primary_block: Option<BlockId>,
    pub related_block: Option<BlockId>,
    pub call_site: Option<CallSiteId>,
    pub member: Option<InterfaceMemberId>,
    pub related_member: Option<InterfaceMemberId>,
    pub instruction_use: Option<InstructionUseId>,
    pub cycle: Vec<BlockId>,
}

impl ProgramIssue {
    const fn at(code: IssueCode, block: BlockId) -> Self {
        Self {
            code,
            primary_block: Some(block),
            related_block: None,
            call_site: None,
            member: None,
            related_member: None,
            instruction_use: None,
            cycle: Vec::new(),
        }
    }

    const fn global(code: IssueCode) -> Self {
        Self {
            code,
            primary_block: None,
            related_block: None,
            call_site: None,
            member: None,
            related_member: None,
            instruction_use: None,
            cycle: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CallGraphEdge {
    pub caller: BlockId,
    pub callee: BlockId,
    pub call_site: CallSiteId,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CallGraph {
    edges: BTreeSet<CallGraphEdge>,
}

impl CallGraph {
    #[must_use]
    pub fn edges(&self) -> &BTreeSet<CallGraphEdge> {
        &self.edges
    }

    pub fn callees_of(&self, block: BlockId) -> impl Iterator<Item = CallGraphEdge> + '_ {
        self.edges
            .iter()
            .copied()
            .filter(move |edge| edge.caller == block)
    }

    pub fn callers_of(&self, block: BlockId) -> impl Iterator<Item = CallGraphEdge> + '_ {
        self.edges
            .iter()
            .copied()
            .filter(move |edge| edge.callee == block)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ValidationReport {
    pub issues: Vec<ProgramIssue>,
    pub call_graph: CallGraph,
    pub dependency_graph: DependencyGraph,
}

impl ValidationReport {
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }

    #[must_use]
    pub fn has(&self, code: IssueCode) -> bool {
        self.issues.iter().any(|issue| issue.code == code)
    }
}

#[must_use]
pub fn validate_program(program: &ControllerProgram) -> ValidationReport {
    let mut report = ValidationReport::default();
    if program.schema_version() != PROGRAM_MODEL_SCHEMA_VERSION {
        report
            .issues
            .push(ProgramIssue::global(IssueCode::ModelSchemaMismatch));
    }
    if program.registry_version() != PHASE2_INSTRUCTION_REGISTRY_VERSION {
        report
            .issues
            .push(ProgramIssue::global(IssueCode::RegistryVersionMismatch));
    }
    if program.blocks().len() > MAX_BLOCKS_PER_CONTROLLER {
        report
            .issues
            .push(ProgramIssue::global(IssueCode::ModelLimitExceeded));
    }

    let mut engineering_numbers = BTreeMap::<(&'static str, u16), BlockId>::new();

    for (&key, block) in program.blocks() {
        if key != block.id {
            report
                .issues
                .push(ProgramIssue::at(IssueCode::BlockKeyMismatch, key));
        }
        if !is_identifier(&block.display_name) {
            report
                .issues
                .push(ProgramIssue::at(IssueCode::InvalidIdentifier, block.id));
        }
        let number_key = (
            block.kind.engineering_prefix(),
            block.engineering_number.get(),
        );
        if let Some(previous) = engineering_numbers.insert(number_key, block.id) {
            let mut issue = ProgramIssue::at(IssueCode::DuplicateEngineeringNumber, block.id);
            issue.related_block = Some(previous);
            report.issues.push(issue);
        }

        validate_interface(program, block, &mut report);
        validate_block_body(program, block, &mut report);
        collect_state_dependencies(program, block, &mut report);
    }

    validate_ob_declarations(program, &mut report);

    for block in program.blocks().values() {
        validate_calls(program, block, &mut report);
    }

    let call_adjacency = adjacency_from_calls(&report.call_graph);
    for cycle in directed_cycles(&call_adjacency) {
        let mut issue = ProgramIssue::at(IssueCode::RecursiveCallCycle, cycle[0]);
        issue.cycle = cycle;
        report.issues.push(issue);
    }
    let state_adjacency = adjacency_from_dependencies(
        &report.dependency_graph,
        DependencyReason::MultiInstanceState,
    );
    for cycle in directed_cycles(&state_adjacency) {
        let mut issue = ProgramIssue::at(IssueCode::StateOwnershipCycle, cycle[0]);
        issue.cycle = cycle;
        report.issues.push(issue);
    }

    report.issues.sort();
    report.issues.dedup();
    report
}

fn validate_ob_declarations(program: &ControllerProgram, report: &mut ValidationReport) {
    let mut cyclic_main = Vec::new();
    let mut startup = Vec::new();
    for block in program.blocks().values() {
        match block.kind {
            ProgramUnitKind::OrganizationBlock(crate::ObDeclaration::CyclicMain) => {
                cyclic_main.push(block.id);
            }
            ProgramUnitKind::OrganizationBlock(crate::ObDeclaration::Startup) => {
                startup.push(block.id);
            }
            ProgramUnitKind::OrganizationBlock(crate::ObDeclaration::TimedCyclic {
                period_milliseconds,
                offset_milliseconds,
                priority,
            }) if period_milliseconds < 10
                || offset_milliseconds >= period_milliseconds
                || priority == 0 =>
            {
                report
                    .issues
                    .push(ProgramIssue::at(IssueCode::InvalidTimedCyclic, block.id));
            }
            ProgramUnitKind::OrganizationBlock(_)
            | ProgramUnitKind::Function
            | ProgramUnitKind::FunctionBlock
            | ProgramUnitKind::DataBlock(_) => {}
        }
    }
    match cyclic_main.len() {
        0 => report
            .issues
            .push(ProgramIssue::global(IssueCode::MissingCyclicMain)),
        1 => {}
        _ => {
            for block in cyclic_main {
                report
                    .issues
                    .push(ProgramIssue::at(IssueCode::MultipleCyclicMain, block));
            }
        }
    }
    if startup.len() > 1 {
        for block in startup {
            report
                .issues
                .push(ProgramIssue::at(IssueCode::MultipleStartup, block));
        }
    }
}

fn validate_interface(
    program: &ControllerProgram,
    block: &ProgramBlock,
    report: &mut ValidationReport,
) {
    let interface = &block.interface;
    if interface.members.len() > MAX_INTERFACE_MEMBERS_PER_BLOCK {
        report.issues.push(ProgramIssue::at(
            IssueCode::InterfaceLimitExceeded,
            block.id,
        ));
    }

    let mut expected_order: Vec<_> = interface.members.keys().copied().collect();
    expected_order.sort_by_key(|id| {
        let member = &interface.members[id];
        (
            member.role.canonical_rank(),
            member.declared_order,
            member.id,
        )
    });
    if expected_order != interface.ordered_member_ids {
        report.issues.push(ProgramIssue::at(
            IssueCode::InterfaceOrderMismatch,
            block.id,
        ));
    }

    let mut state = InterfaceValidationState::default();
    for (&key, member) in &interface.members {
        validate_interface_member(program, block, key, member, &mut state, report);
    }
    if state.returns.len() > 1 {
        for member in state.returns {
            push_member_issue(report, IssueCode::MultipleReturn, block.id, member);
        }
    }

    if matches!(
        block.kind,
        ProgramUnitKind::DataBlock(DataBlockKind::Instance { .. })
    ) && !interface.members.is_empty()
    {
        report
            .issues
            .push(ProgramIssue::at(IssueCode::RoleNotAllowed, block.id));
    }
}

#[derive(Default)]
struct InterfaceValidationState {
    names: BTreeMap<String, InterfaceMemberId>,
    orders: BTreeMap<(InterfaceRole, u32), InterfaceMemberId>,
    returns: Vec<InterfaceMemberId>,
}

fn validate_interface_member(
    program: &ControllerProgram,
    block: &ProgramBlock,
    key: InterfaceMemberId,
    member: &InterfaceMember,
    state: &mut InterfaceValidationState,
    report: &mut ValidationReport,
) {
    if key != member.id {
        let mut issue = ProgramIssue::at(IssueCode::InterfaceKeyMismatch, block.id);
        issue.member = Some(key);
        issue.related_member = Some(member.id);
        report.issues.push(issue);
    }
    if !is_identifier(&member.name) {
        push_member_issue(report, IssueCode::InvalidIdentifier, block.id, member.id);
    }
    if let Some(previous) = state
        .names
        .insert(member.name.to_ascii_lowercase(), member.id)
    {
        let mut issue = ProgramIssue::at(IssueCode::DuplicateMemberName, block.id);
        issue.member = Some(member.id);
        issue.related_member = Some(previous);
        report.issues.push(issue);
    }
    if let Some(previous) = state
        .orders
        .insert((member.role, member.declared_order), member.id)
    {
        let mut issue = ProgramIssue::at(IssueCode::DuplicateDeclaredOrder, block.id);
        issue.member = Some(member.id);
        issue.related_member = Some(previous);
        report.issues.push(issue);
    }
    if member.role == InterfaceRole::Return {
        state.returns.push(member.id);
    }
    if !role_allowed(block.kind, member.role) {
        push_member_issue(report, IssueCode::RoleNotAllowed, block.id, member.id);
    }
    validate_member_metadata(block.id, member, report);
    validate_special_member_type(program, block, member, report);
}

fn validate_special_member_type(
    program: &ControllerProgram,
    block: &ProgramBlock,
    member: &InterfaceMember,
    report: &mut ValidationReport,
) {
    match member.data_type {
        DataType::BlockInstance(target) => {
            if block.kind != ProgramUnitKind::FunctionBlock
                || member.role != InterfaceRole::Static
                || !matches!(
                    program.block(target).map(|target_block| target_block.kind),
                    Some(ProgramUnitKind::FunctionBlock)
                )
            {
                let mut issue = ProgramIssue::at(IssueCode::InstanceTypeIllegal, block.id);
                issue.member = Some(member.id);
                issue.related_block = Some(target);
                report.issues.push(issue);
            }
        }
        DataType::InstructionState(_) => {
            if member.role != InterfaceRole::Static
                || !matches!(
                    block.kind,
                    ProgramUnitKind::FunctionBlock
                        | ProgramUnitKind::DataBlock(DataBlockKind::Global)
                )
            {
                push_member_issue(report, IssueCode::InstanceTypeIllegal, block.id, member.id);
            }
        }
        DataType::Bool
        | DataType::Int
        | DataType::DInt
        | DataType::Real
        | DataType::Time
        | DataType::String { .. }
        | DataType::Named(_) => {}
    }
}

const fn role_allowed(kind: ProgramUnitKind, role: InterfaceRole) -> bool {
    match kind {
        ProgramUnitKind::OrganizationBlock(_) => {
            matches!(role, InterfaceRole::Temp | InterfaceRole::Constant)
        }
        ProgramUnitKind::Function => matches!(
            role,
            InterfaceRole::Input
                | InterfaceRole::Output
                | InterfaceRole::InOut
                | InterfaceRole::Temp
                | InterfaceRole::Constant
                | InterfaceRole::Return
        ),
        ProgramUnitKind::FunctionBlock => matches!(
            role,
            InterfaceRole::Input
                | InterfaceRole::Output
                | InterfaceRole::InOut
                | InterfaceRole::Static
                | InterfaceRole::Temp
                | InterfaceRole::Constant
        ),
        ProgramUnitKind::DataBlock(DataBlockKind::Global) => {
            matches!(role, InterfaceRole::Static | InterfaceRole::Constant)
        }
        ProgramUnitKind::DataBlock(DataBlockKind::Instance { .. }) => false,
    }
}

fn validate_member_metadata(
    block: BlockId,
    member: &InterfaceMember,
    report: &mut ValidationReport,
) {
    let legal = match member.role {
        InterfaceRole::Input => {
            member.start_value.is_none()
                && member.constant_value.is_none()
                && member.retain_policy.is_none()
                && !member.required_output_binding
        }
        InterfaceRole::Output => {
            member.default_value.is_none()
                && member.constant_value.is_none()
                && member.retain_policy.is_none()
        }
        InterfaceRole::InOut | InterfaceRole::Temp | InterfaceRole::Return => {
            member.default_value.is_none()
                && member.start_value.is_none()
                && member.constant_value.is_none()
                && member.retain_policy.is_none()
                && !member.required_output_binding
        }
        InterfaceRole::Static => {
            member.default_value.is_none()
                && member.constant_value.is_none()
                && !member.required_output_binding
        }
        InterfaceRole::Constant => {
            member.default_value.is_none()
                && member.start_value.is_none()
                && member.constant_value.is_some()
                && member.retain_policy.is_none()
                && !member.required_output_binding
        }
    };
    if !legal {
        push_member_issue(report, IssueCode::MemberMetadataIllegal, block, member.id);
    }
    for value in [
        member.default_value.as_ref(),
        member.start_value.as_ref(),
        member.constant_value.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        if !value.is_compatible_with(&member.data_type) {
            push_member_issue(report, IssueCode::MemberValueTypeMismatch, block, member.id);
        }
    }
}

fn validate_block_body(
    program: &ControllerProgram,
    block: &ProgramBlock,
    report: &mut ValidationReport,
) {
    validate_body_shape(block, report);
    let registry = *phase2_instruction_registry();
    validate_instruction_order(block, report);
    let mut instruction_ids = BTreeSet::new();
    let mut state_owners = BTreeMap::<VariableRef, InstructionUseId>::new();
    for instruction in &block.instructions {
        if !instruction_ids.insert(instruction.id) {
            let mut issue = ProgramIssue::at(IssueCode::DuplicateInstructionUseId, block.id);
            issue.instruction_use = Some(instruction.id);
            report.issues.push(issue);
        }
        let Some(definition) = registry.lookup(instruction.instruction) else {
            let mut issue = ProgramIssue::at(IssueCode::UnknownInstruction, block.id);
            issue.instruction_use = Some(instruction.id);
            report.issues.push(issue);
            continue;
        };
        if definition.category == InstructionCategory::Call {
            let mut issue = ProgramIssue::at(IssueCode::CallInstructionInBody, block.id);
            issue.instruction_use = Some(instruction.id);
            report.issues.push(issue);
        }
        match definition.state_requirement {
            StateRequirement::None => {
                if instruction.state_owner.is_some() {
                    push_instruction_issue(
                        report,
                        IssueCode::UnexpectedInstructionState,
                        block.id,
                        instruction.id,
                    );
                }
            }
            StateRequirement::Explicit(expected) => {
                let Some(owner) = instruction.state_owner.as_ref() else {
                    push_instruction_issue(
                        report,
                        IssueCode::MissingInstructionState,
                        block.id,
                        instruction.id,
                    );
                    continue;
                };
                match resolve_variable(program, block, owner) {
                    Some(resolved) => {
                        if let VariableRef::DataBlockMember { data_block, .. } = *owner {
                            report.dependency_graph.insert(DependencyEdge {
                                dependent: block.id,
                                dependency: data_block,
                                reason: DependencyReason::DataUse,
                                call_site: None,
                            });
                        }
                        if resolved.data_type != &DataType::InstructionState(expected) {
                            push_instruction_issue(
                                report,
                                IssueCode::InstructionStateTypeMismatch,
                                block.id,
                                instruction.id,
                            );
                        }
                        if let Some(previous) = state_owners.insert(owner.clone(), instruction.id) {
                            let mut issue =
                                ProgramIssue::at(IssueCode::InstructionStateAlias, block.id);
                            issue.instruction_use = Some(instruction.id);
                            issue.related_member = Some(instruction_state_member(owner));
                            issue.member = Some(instruction_state_member(owner));
                            if previous != instruction.id {
                                report.issues.push(issue);
                            }
                        }
                    }
                    None => push_instruction_issue(
                        report,
                        IssueCode::UnknownInstructionState,
                        block.id,
                        instruction.id,
                    ),
                }
            }
            StateRequirement::FunctionBlockInstance => {
                push_instruction_issue(
                    report,
                    IssueCode::CallInstructionInBody,
                    block.id,
                    instruction.id,
                );
            }
        }
    }
}

fn validate_body_shape(block: &ProgramBlock, report: &mut ValidationReport) {
    if matches!(block.kind, ProgramUnitKind::DataBlock(_))
        && (!block.instructions.is_empty() || !block.calls.is_empty())
    {
        report
            .issues
            .push(ProgramIssue::at(IssueCode::BodyNotAllowed, block.id));
    }
    if block.instructions.len() > MAX_INSTRUCTION_USES_PER_BLOCK
        || block.calls.len() > MAX_CALLS_PER_BLOCK
    {
        report
            .issues
            .push(ProgramIssue::at(IssueCode::ModelLimitExceeded, block.id));
    }
}

fn validate_instruction_order(block: &ProgramBlock, report: &mut ValidationReport) {
    if !block
        .instructions
        .windows(2)
        .all(|pair| pair[0].id < pair[1].id)
    {
        report.issues.push(ProgramIssue::at(
            IssueCode::InstructionOrderMismatch,
            block.id,
        ));
    }
}

fn collect_state_dependencies(
    program: &ControllerProgram,
    block: &ProgramBlock,
    report: &mut ValidationReport,
) {
    if let ProgramUnitKind::DataBlock(DataBlockKind::Instance { fb_type }) = block.kind {
        report.dependency_graph.insert(DependencyEdge {
            dependent: block.id,
            dependency: fb_type,
            reason: DependencyReason::InstanceOf,
            call_site: None,
        });
        if !matches!(
            program.block(fb_type).map(|candidate| candidate.kind),
            Some(ProgramUnitKind::FunctionBlock)
        ) {
            let mut issue = ProgramIssue::at(IssueCode::InvalidInstanceDb, block.id);
            issue.related_block = Some(fb_type);
            report.issues.push(issue);
        }
    }
    if block.kind == ProgramUnitKind::FunctionBlock {
        for member in block.interface.members.values() {
            if let DataType::BlockInstance(fb_type) = member.data_type {
                report.dependency_graph.insert(DependencyEdge {
                    dependent: block.id,
                    dependency: fb_type,
                    reason: DependencyReason::MultiInstanceState,
                    call_site: None,
                });
            }
        }
    }
}

fn validate_calls(
    program: &ControllerProgram,
    source_block: &ProgramBlock,
    report: &mut ValidationReport,
) {
    let mut call_ids = BTreeSet::new();
    if !source_block
        .calls
        .windows(2)
        .all(|pair| pair[0].id < pair[1].id)
    {
        report.issues.push(ProgramIssue::at(
            IssueCode::CallOrderMismatch,
            source_block.id,
        ));
    }
    for call in &source_block.calls {
        if !call_ids.insert(call.id) {
            push_call_issue(
                report,
                IssueCode::DuplicateCallSiteId,
                source_block.id,
                call.id,
            );
        }
        if !source_block.kind.is_executable() {
            push_call_issue(report, IssueCode::IllegalCaller, source_block.id, call.id);
        }
        let Some(target_block) = program.block(call.callee) else {
            let mut issue = ProgramIssue::at(IssueCode::MissingCallee, source_block.id);
            issue.call_site = Some(call.id);
            issue.related_block = Some(call.callee);
            report.issues.push(issue);
            continue;
        };

        let legal_callee = matches!(
            target_block.kind,
            ProgramUnitKind::Function | ProgramUnitKind::FunctionBlock
        );
        if !legal_callee {
            let mut issue = ProgramIssue::at(IssueCode::IllegalCallee, source_block.id);
            issue.call_site = Some(call.id);
            issue.related_block = Some(target_block.id);
            report.issues.push(issue);
            continue;
        }

        report.call_graph.edges.insert(CallGraphEdge {
            caller: source_block.id,
            callee: target_block.id,
            call_site: call.id,
        });
        report.dependency_graph.insert(DependencyEdge {
            dependent: source_block.id,
            dependency: target_block.id,
            reason: DependencyReason::Call,
            call_site: Some(call.id),
        });

        let expected_instruction = if target_block.kind == ProgramUnitKind::Function {
            CALL_FC
        } else {
            CALL_FB
        };
        if call.instruction != expected_instruction {
            push_call_issue(
                report,
                IssueCode::CallInstructionMismatch,
                source_block.id,
                call.id,
            );
        }

        validate_instance_owner(program, source_block, target_block, call, report);
        validate_bindings(program, source_block, target_block, call, report);
    }
}

fn validate_instance_owner(
    program: &ControllerProgram,
    source_block: &ProgramBlock,
    target_block: &ProgramBlock,
    call: &crate::CallSite,
    report: &mut ValidationReport,
) {
    match target_block.kind {
        ProgramUnitKind::Function => {
            if call.instance_owner.is_some() {
                push_call_issue(
                    report,
                    IssueCode::UnexpectedInstanceOwner,
                    source_block.id,
                    call.id,
                );
            }
        }
        ProgramUnitKind::FunctionBlock => match call.instance_owner {
            None => push_call_issue(
                report,
                IssueCode::MissingInstanceOwner,
                source_block.id,
                call.id,
            ),
            Some(InstanceOwner::InstanceDb(db_id)) => {
                let valid = matches!(
                    program.block(db_id).map(|db| db.kind),
                    Some(ProgramUnitKind::DataBlock(DataBlockKind::Instance { fb_type }))
                        if fb_type == target_block.id
                );
                if !valid {
                    let mut issue =
                        ProgramIssue::at(IssueCode::InstanceTypeMismatch, source_block.id);
                    issue.call_site = Some(call.id);
                    issue.related_block = Some(db_id);
                    report.issues.push(issue);
                }
            }
            Some(InstanceOwner::MultiInstance {
                owner_fb,
                static_member,
            }) => {
                let valid = source_block.kind == ProgramUnitKind::FunctionBlock
                    && owner_fb == source_block.id
                    && matches!(
                        source_block.interface.member(static_member),
                        Some(member)
                            if member.role == InterfaceRole::Static
                                && member.data_type == DataType::BlockInstance(target_block.id)
                    );
                if !valid {
                    let mut issue =
                        ProgramIssue::at(IssueCode::InvalidMultiInstance, source_block.id);
                    issue.call_site = Some(call.id);
                    issue.member = Some(static_member);
                    issue.related_block = Some(target_block.id);
                    report.issues.push(issue);
                }
            }
        },
        ProgramUnitKind::OrganizationBlock(_) | ProgramUnitKind::DataBlock(_) => {}
    }
}

fn validate_bindings(
    program: &ControllerProgram,
    source_block: &ProgramBlock,
    target_block: &ProgramBlock,
    call: &crate::CallSite,
    report: &mut ValidationReport,
) {
    let mut bound = BTreeMap::<InterfaceMemberId, usize>::new();
    let mut aliases = BTreeMap::<VariableRef, (InterfaceMemberId, bool)>::new();
    if !call
        .bindings
        .windows(2)
        .all(|pair| pair[0].formal < pair[1].formal)
    {
        push_call_issue(
            report,
            IssueCode::BindingOrderMismatch,
            source_block.id,
            call.id,
        );
    }
    for (index, binding) in call.bindings.iter().enumerate() {
        if bound.insert(binding.formal, index).is_some() {
            let mut issue = ProgramIssue::at(IssueCode::DuplicateBinding, source_block.id);
            issue.call_site = Some(call.id);
            issue.member = Some(binding.formal);
            report.issues.push(issue);
        }
        let Some(formal) = target_block.interface.member(binding.formal) else {
            let mut issue = ProgramIssue::at(IssueCode::UnknownFormal, source_block.id);
            issue.call_site = Some(call.id);
            issue.member = Some(binding.formal);
            issue.related_block = Some(target_block.id);
            report.issues.push(issue);
            continue;
        };
        if !formal.role.is_call_formal() {
            let mut issue = ProgramIssue::at(IssueCode::UnknownFormal, source_block.id);
            issue.call_site = Some(call.id);
            issue.member = Some(formal.id);
            report.issues.push(issue);
            continue;
        }
        validate_binding_actual(
            program,
            source_block,
            call.id,
            formal,
            &binding.actual,
            &mut aliases,
            report,
        );
    }

    for formal in target_block.interface.members.values() {
        let required = match formal.role {
            InterfaceRole::Input => formal.default_value.is_none(),
            InterfaceRole::Output => formal.required_output_binding,
            InterfaceRole::InOut | InterfaceRole::Return => true,
            InterfaceRole::Static | InterfaceRole::Temp | InterfaceRole::Constant => false,
        };
        if required && !bound.contains_key(&formal.id) {
            push_binding_issue(
                report,
                IssueCode::MissingBinding,
                source_block.id,
                call.id,
                formal.id,
            );
        }
    }
}

fn validate_binding_actual(
    program: &ControllerProgram,
    source_block: &ProgramBlock,
    call_site: CallSiteId,
    formal: &InterfaceMember,
    actual: &BindingActual,
    aliases: &mut BTreeMap<VariableRef, (InterfaceMemberId, bool)>,
    report: &mut ValidationReport,
) {
    let writable_formal = matches!(
        formal.role,
        InterfaceRole::Output | InterfaceRole::InOut | InterfaceRole::Return
    );
    match actual {
        BindingActual::Literal(value) => {
            if formal.role != InterfaceRole::Input {
                push_binding_issue(
                    report,
                    IssueCode::BindingDirection,
                    source_block.id,
                    call_site,
                    formal.id,
                );
            }
            if !value.is_compatible_with(&formal.data_type) {
                push_binding_issue(
                    report,
                    IssueCode::BindingTypeMismatch,
                    source_block.id,
                    call_site,
                    formal.id,
                );
            }
        }
        BindingActual::Variable(variable) => {
            let Some(resolved) = resolve_variable(program, source_block, variable) else {
                push_binding_issue(
                    report,
                    IssueCode::UnknownActual,
                    source_block.id,
                    call_site,
                    formal.id,
                );
                return;
            };
            if let VariableRef::DataBlockMember { data_block, .. } = *variable {
                report.dependency_graph.insert(DependencyEdge {
                    dependent: source_block.id,
                    dependency: data_block,
                    reason: DependencyReason::DataUse,
                    call_site: Some(call_site),
                });
            }
            if resolved.data_type != &formal.data_type {
                push_binding_issue(
                    report,
                    IssueCode::BindingTypeMismatch,
                    source_block.id,
                    call_site,
                    formal.id,
                );
            }
            if writable_formal && !resolved.writable {
                push_binding_issue(
                    report,
                    IssueCode::BindingDirection,
                    source_block.id,
                    call_site,
                    formal.id,
                );
            }
            if let Some((previous, prior_writable)) =
                aliases.insert(variable.clone(), (formal.id, writable_formal))
                && (writable_formal || prior_writable)
            {
                let mut issue = ProgramIssue::at(IssueCode::AliasConflict, source_block.id);
                issue.call_site = Some(call_site);
                issue.member = Some(formal.id);
                issue.related_member = Some(previous);
                report.issues.push(issue);
            }
        }
    }
}

struct ResolvedVariable<'a> {
    data_type: &'a DataType,
    writable: bool,
}

fn resolve_variable<'a>(
    program: &'a ControllerProgram,
    source_block: &'a ProgramBlock,
    variable: &VariableRef,
) -> Option<ResolvedVariable<'a>> {
    let member = match *variable {
        VariableRef::CallerMember(member) => source_block.interface.member(member)?,
        VariableRef::DataBlockMember { data_block, member } => {
            let block = program.block(data_block)?;
            if !matches!(
                block.kind,
                ProgramUnitKind::DataBlock(DataBlockKind::Global)
            ) {
                return None;
            }
            block.interface.member(member)?
        }
    };
    Some(ResolvedVariable {
        data_type: &member.data_type,
        writable: !matches!(member.role, InterfaceRole::Input | InterfaceRole::Constant),
    })
}

fn adjacency_from_calls(graph: &CallGraph) -> BTreeMap<BlockId, BTreeSet<BlockId>> {
    let mut adjacency = BTreeMap::new();
    for edge in &graph.edges {
        adjacency
            .entry(edge.caller)
            .or_insert_with(BTreeSet::new)
            .insert(edge.callee);
        adjacency.entry(edge.callee).or_insert_with(BTreeSet::new);
    }
    adjacency
}

fn adjacency_from_dependencies(
    graph: &DependencyGraph,
    reason: DependencyReason,
) -> BTreeMap<BlockId, BTreeSet<BlockId>> {
    let mut adjacency = BTreeMap::new();
    for edge in graph.edges().iter().filter(|edge| edge.reason == reason) {
        adjacency
            .entry(edge.dependent)
            .or_insert_with(BTreeSet::new)
            .insert(edge.dependency);
        adjacency
            .entry(edge.dependency)
            .or_insert_with(BTreeSet::new);
    }
    adjacency
}

fn directed_cycles(adjacency: &BTreeMap<BlockId, BTreeSet<BlockId>>) -> Vec<Vec<BlockId>> {
    struct Frame {
        node: BlockId,
        neighbors: Vec<BlockId>,
        next: usize,
    }

    let mut colors = BTreeMap::<BlockId, u8>::new();
    let mut cycles = BTreeSet::<Vec<BlockId>>::new();
    for &root in adjacency.keys() {
        if colors.get(&root).copied().unwrap_or(0) != 0 {
            continue;
        }
        colors.insert(root, 1);
        let mut stack = vec![Frame {
            node: root,
            neighbors: adjacency
                .get(&root)
                .into_iter()
                .flatten()
                .copied()
                .collect(),
            next: 0,
        }];
        while let Some(frame) = stack.last_mut() {
            if let Some(&neighbor) = frame.neighbors.get(frame.next) {
                frame.next += 1;
                match colors.get(&neighbor).copied().unwrap_or(0) {
                    0 => {
                        colors.insert(neighbor, 1);
                        stack.push(Frame {
                            node: neighbor,
                            neighbors: adjacency
                                .get(&neighbor)
                                .into_iter()
                                .flatten()
                                .copied()
                                .collect(),
                            next: 0,
                        });
                    }
                    1 => {
                        if let Some(position) = stack.iter().position(|item| item.node == neighbor)
                        {
                            let cycle = stack[position..].iter().map(|item| item.node).collect();
                            cycles.insert(canonical_cycle(cycle));
                        }
                    }
                    _ => {}
                }
            } else {
                let finished = frame.node;
                stack.pop();
                colors.insert(finished, 2);
            }
        }
    }
    cycles.into_iter().collect()
}

fn canonical_cycle(mut cycle: Vec<BlockId>) -> Vec<BlockId> {
    let minimum_index = cycle
        .iter()
        .enumerate()
        .min_by_key(|(_, block)| **block)
        .map_or(0, |(index, _)| index);
    cycle.rotate_left(minimum_index);
    cycle.push(cycle[0]);
    cycle
}

fn is_identifier(value: &str) -> bool {
    if value.is_empty() || value.len() > 128 || !value.is_ascii() {
        return false;
    }
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

const fn instruction_state_member(variable: &VariableRef) -> InterfaceMemberId {
    match *variable {
        VariableRef::CallerMember(member) | VariableRef::DataBlockMember { member, .. } => member,
    }
}

fn push_member_issue(
    report: &mut ValidationReport,
    code: IssueCode,
    block: BlockId,
    member: InterfaceMemberId,
) {
    let mut issue = ProgramIssue::at(code, block);
    issue.member = Some(member);
    report.issues.push(issue);
}

fn push_instruction_issue(
    report: &mut ValidationReport,
    code: IssueCode,
    block: BlockId,
    instruction_use: InstructionUseId,
) {
    let mut issue = ProgramIssue::at(code, block);
    issue.instruction_use = Some(instruction_use);
    report.issues.push(issue);
}

fn push_call_issue(
    report: &mut ValidationReport,
    code: IssueCode,
    block: BlockId,
    call_site: CallSiteId,
) {
    let mut issue = ProgramIssue::at(code, block);
    issue.call_site = Some(call_site);
    report.issues.push(issue);
}

fn push_binding_issue(
    report: &mut ValidationReport,
    code: IssueCode,
    block: BlockId,
    call_site: CallSiteId,
    member: InterfaceMemberId,
) {
    let mut issue = ProgramIssue::at(code, block);
    issue.call_site = Some(call_site);
    issue.member = Some(member);
    report.issues.push(issue);
}
