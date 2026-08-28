use std::collections::{BTreeMap, BTreeSet};

use plc_program::{
    BindingActual, BlockId, BlockInterface, BoundInstructionFormal, CALL_FB, CALL_FC, CallSite,
    CallSiteId, CanonicalValue, ControllerId, ControllerProgram, DataBlockKind, DataType,
    DependencyReason, EngineeringNumber, FORMAL_INPUT, FORMAL_OUTPUT, InstanceOwner,
    InstructionActivationPolicy, InstructionBindingError, InstructionCategory, InstructionCode,
    InstructionUse, InstructionUseId, InterfaceMember, InterfaceMemberId, InterfaceRole,
    InvalidationCode, IssueCode, MOVE, NO_OP, ObDeclaration, PHASE2_INSTRUCTION_REGISTRY_VERSION,
    ParameterBinding, ProgramBlock, ProgramUnitKind, StateKind, StateRequirement, TIMER_ON_DELAY,
    VariableRef, phase2_instruction_registry, validate_program,
};

const MAIN: BlockId = BlockId::new(1);
const LEAF_FC: BlockId = BlockId::new(10);
const HELPER_FC: BlockId = BlockId::new(11);
const MOTOR_FB: BlockId = BlockId::new(20);
const MOTOR_DB: BlockId = BlockId::new(30);

const MAIN_IN: InterfaceMemberId = InterfaceMemberId::new(1_001);
const MAIN_OUT: InterfaceMemberId = InterfaceMemberId::new(1_002);
const MAIN_FB_OUT: InterfaceMemberId = InterfaceMemberId::new(1_003);
const LEAF_IN: InterfaceMemberId = InterfaceMemberId::new(2_001);
const LEAF_OUT: InterfaceMemberId = InterfaceMemberId::new(2_002);
const HELPER_IN: InterfaceMemberId = InterfaceMemberId::new(3_001);
const HELPER_OUT: InterfaceMemberId = InterfaceMemberId::new(3_002);
const MOTOR_IN: InterfaceMemberId = InterfaceMemberId::new(4_001);
const MOTOR_OUT: InterfaceMemberId = InterfaceMemberId::new(4_002);
const MOTOR_TIMER_STATE: InterfaceMemberId = InterfaceMemberId::new(4_003);

fn number(value: u16) -> EngineeringNumber {
    EngineeringNumber::new(value).expect("test engineering number is nonzero")
}

fn plain_member(
    id: InterfaceMemberId,
    name: &str,
    role: InterfaceRole,
    data_type: DataType,
    order: u32,
) -> InterfaceMember {
    InterfaceMember::plain(id, name, role, data_type, order)
}

fn required_output(id: InterfaceMemberId, name: &str, order: u32) -> InterfaceMember {
    let mut member = plain_member(id, name, InterfaceRole::Output, DataType::Bool, order);
    member.required_output_binding = true;
    member
}

fn input_binding(formal: InterfaceMemberId, actual: InterfaceMemberId) -> ParameterBinding {
    ParameterBinding {
        formal,
        actual: BindingActual::Variable(VariableRef::CallerMember(actual)),
    }
}

fn edit_block(program: &mut ControllerProgram, id: BlockId, edit: impl FnOnce(&mut ProgramBlock)) {
    let mut block = program.block(id).expect("fixture block exists").clone();
    edit(&mut block);
    program
        .replace_block(block)
        .expect("fixture replacement succeeds");
}

// The complete fixture is intentionally kept together so every acceptance test
// starts from the same auditable controller topology.
#[allow(clippy::too_many_lines)]
fn valid_program() -> ControllerProgram {
    let leaf_interface = BlockInterface::from_members([
        plain_member(LEAF_IN, "Enable", InterfaceRole::Input, DataType::Bool, 0),
        required_output(LEAF_OUT, "Result", 0),
    ]);
    let leaf = ProgramBlock::new(
        LEAF_FC,
        "NormalizeSignal",
        number(1),
        ProgramUnitKind::Function,
        leaf_interface,
    );

    let helper_interface = BlockInterface::from_members([
        plain_member(HELPER_IN, "Source", InterfaceRole::Input, DataType::Bool, 0),
        required_output(HELPER_OUT, "Target", 0),
    ]);
    let mut helper = ProgramBlock::new(
        HELPER_FC,
        "PrepareSignal",
        number(2),
        ProgramUnitKind::Function,
        helper_interface,
    );
    helper.calls.push(CallSite {
        id: CallSiteId::new(101),
        instruction: CALL_FC,
        callee: LEAF_FC,
        bindings: vec![
            input_binding(LEAF_IN, HELPER_IN),
            input_binding(LEAF_OUT, HELPER_OUT),
        ],
        instance_owner: None,
    });

    let motor_interface = BlockInterface::from_members([
        plain_member(MOTOR_IN, "Run", InterfaceRole::Input, DataType::Bool, 0),
        required_output(MOTOR_OUT, "Running", 0),
        plain_member(
            MOTOR_TIMER_STATE,
            "DelayState",
            InterfaceRole::Static,
            DataType::InstructionState(StateKind::Timer),
            0,
        ),
    ]);
    let mut motor = ProgramBlock::new(
        MOTOR_FB,
        "Motor",
        number(1),
        ProgramUnitKind::FunctionBlock,
        motor_interface,
    );
    motor.instructions.push(InstructionUse {
        id: InstructionUseId::new(201),
        instruction: TIMER_ON_DELAY,
        state_owner: Some(VariableRef::CallerMember(MOTOR_TIMER_STATE)),
    });

    let motor_db = ProgramBlock::new(
        MOTOR_DB,
        "MotorInstance",
        number(100),
        ProgramUnitKind::DataBlock(DataBlockKind::Instance { fb_type: MOTOR_FB }),
        BlockInterface::default(),
    );

    let main_interface = BlockInterface::from_members([
        plain_member(MAIN_IN, "Raw", InterfaceRole::Temp, DataType::Bool, 0),
        plain_member(MAIN_OUT, "Prepared", InterfaceRole::Temp, DataType::Bool, 1),
        plain_member(
            MAIN_FB_OUT,
            "Running",
            InterfaceRole::Temp,
            DataType::Bool,
            2,
        ),
    ]);
    let mut main = ProgramBlock::new(
        MAIN,
        "Main",
        number(1),
        ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain),
        main_interface,
    );
    main.calls.push(CallSite {
        id: CallSiteId::new(1),
        instruction: CALL_FC,
        callee: HELPER_FC,
        bindings: vec![
            input_binding(HELPER_IN, MAIN_IN),
            input_binding(HELPER_OUT, MAIN_OUT),
        ],
        instance_owner: None,
    });
    main.calls.push(CallSite {
        id: CallSiteId::new(2),
        instruction: CALL_FB,
        callee: MOTOR_FB,
        bindings: vec![
            input_binding(MOTOR_IN, MAIN_OUT),
            input_binding(MOTOR_OUT, MAIN_FB_OUT),
        ],
        instance_owner: Some(InstanceOwner::InstanceDb(MOTOR_DB)),
    });

    let mut program = ControllerProgram::new(ControllerId::new(7));
    for block in [main, leaf, helper, motor, motor_db] {
        program.insert_block(block).expect("unique fixture block");
    }
    program
}

#[test]
fn valid_aggregate_exposes_explicit_call_and_dependency_graphs() {
    let program = valid_program();
    let report = validate_program(&program);
    assert_eq!(
        program.registry_version(),
        PHASE2_INSTRUCTION_REGISTRY_VERSION
    );
    assert!(report.is_valid(), "unexpected issues: {:#?}", report.issues);

    let calls: BTreeSet<_> = report
        .call_graph
        .edges()
        .iter()
        .map(|edge| (edge.caller, edge.callee))
        .collect();
    assert_eq!(
        calls,
        BTreeSet::from([(MAIN, HELPER_FC), (MAIN, MOTOR_FB), (HELPER_FC, LEAF_FC),])
    );

    assert!(report.dependency_graph.edges().iter().any(|edge| {
        edge.dependent == MOTOR_DB
            && edge.dependency == MOTOR_FB
            && edge.reason == DependencyReason::InstanceOf
    }));
}

#[test]
fn shared_registry_is_closed_ordered_versioned_and_descriptive() {
    let registry = *phase2_instruction_registry();
    assert_eq!(
        registry.semantic_version,
        PHASE2_INSTRUCTION_REGISTRY_VERSION
    );
    assert_eq!(registry.schema_version, 1);
    assert!(
        registry
            .definitions()
            .windows(2)
            .all(|pair| pair[0].code < pair[1].code)
    );

    let categories: BTreeSet<_> = registry
        .definitions()
        .iter()
        .map(|definition| definition.category)
        .collect();
    assert_eq!(
        categories,
        BTreeSet::from([
            InstructionCategory::Stateless,
            InstructionCategory::Edge,
            InstructionCategory::Timer,
            InstructionCategory::Counter,
            InstructionCategory::Call,
            InstructionCategory::Control,
            InstructionCategory::Instrumentation,
        ])
    );
    assert_eq!(
        registry
            .lookup(TIMER_ON_DELAY)
            .map(|entry| entry.state_requirement),
        Some(StateRequirement::Explicit(StateKind::Timer))
    );
    assert_eq!(
        registry
            .lookup(CALL_FB)
            .map(|entry| entry.state_requirement),
        Some(StateRequirement::FunctionBlockInstance)
    );
    assert!(registry.lookup(InstructionCode(u16::MAX)).is_none());
    assert_eq!(registry.validate(), Ok(()));
    assert!(matches!(
        registry.lookup(MOVE).map(|entry| entry.activation),
        Some(InstructionActivationPolicy::EnableStatus { .. })
    ));

    let bound = registry
        .bind_types(
            MOVE,
            [
                BoundInstructionFormal {
                    formal: FORMAL_OUTPUT,
                    data_type: DataType::DInt,
                },
                BoundInstructionFormal {
                    formal: FORMAL_INPUT,
                    data_type: DataType::DInt,
                },
            ],
        )
        .expect("registry canonicalizes stable formal identities");
    assert_eq!(bound.instruction(), MOVE);
    assert_eq!(bound.formals()[0].formal, FORMAL_INPUT);
    assert_eq!(bound.data_type(FORMAL_OUTPUT), Some(&DataType::DInt));
    assert_eq!(
        registry.bind_types(
            MOVE,
            [
                BoundInstructionFormal {
                    formal: FORMAL_INPUT,
                    data_type: DataType::DInt,
                },
                BoundInstructionFormal {
                    formal: FORMAL_OUTPUT,
                    data_type: DataType::Bool,
                },
            ],
        ),
        Err(InstructionBindingError::TypeConstraint(MOVE, FORMAL_OUTPUT))
    );
}

#[test]
fn public_interface_change_invalidates_direct_and_transitive_callers() {
    let program = valid_program();
    let mut replacement = program.block(LEAF_FC).unwrap().interface.clone();
    let mut optional = plain_member(
        InterfaceMemberId::new(2_003),
        "OptionalGate",
        InterfaceRole::Input,
        DataType::Bool,
        1,
    );
    optional.default_value = Some(CanonicalValue::Bool(false));
    replacement = BlockInterface::from_members(
        replacement
            .members
            .into_values()
            .chain(std::iter::once(optional)),
    );

    let plan = program
        .explain_interface_change(LEAF_FC, &replacement)
        .expect("legal interface replacement");
    assert!(plan.delta.public_signature_changed);
    assert_eq!(plan.delta.added, vec![InterfaceMemberId::new(2_003)]);

    let by_block: BTreeMap<_, _> = plan
        .explanations
        .iter()
        .map(|explanation| (explanation.invalidated_block, explanation))
        .collect();
    assert_eq!(by_block.len(), 3);
    assert_eq!(
        by_block[&LEAF_FC].code,
        InvalidationCode::OwnInterfaceChanged
    );
    assert_eq!(
        by_block[&HELPER_FC].code,
        InvalidationCode::CalledInterfaceChanged
    );
    assert_eq!(
        by_block[&HELPER_FC].dependency_path,
        vec![HELPER_FC, LEAF_FC]
    );
    assert_eq!(
        by_block[&MAIN].code,
        InvalidationCode::TransitiveDependencyChanged
    );
    assert_eq!(
        by_block[&MAIN].dependency_path,
        vec![MAIN, HELPER_FC, LEAF_FC]
    );
}

#[test]
fn private_temp_change_does_not_invalidate_callers() {
    let program = valid_program();
    let leaf = program.block(LEAF_FC).unwrap();
    let replacement = BlockInterface::from_members(leaf.interface.members.values().cloned().chain(
        std::iter::once(plain_member(
            InterfaceMemberId::new(2_004),
            "Scratch",
            InterfaceRole::Temp,
            DataType::Bool,
            0,
        )),
    ));
    let plan = program
        .explain_interface_change(LEAF_FC, &replacement)
        .unwrap();
    assert!(!plan.delta.public_signature_changed);
    assert_eq!(plan.explanations.len(), 1);
    assert_eq!(plan.explanations[0].invalidated_block, LEAF_FC);
}

#[test]
fn fb_layout_change_invalidates_callers_and_instance_databases() {
    let program = valid_program();
    let motor = program.block(MOTOR_FB).unwrap();
    let replacement =
        BlockInterface::from_members(motor.interface.members.values().cloned().chain(
            std::iter::once(plain_member(
                InterfaceMemberId::new(4_004),
                "Cycles",
                InterfaceRole::Static,
                DataType::DInt,
                1,
            )),
        ));
    let plan = program
        .explain_interface_change(MOTOR_FB, &replacement)
        .unwrap();
    assert!(plan.delta.instance_layout_changed);
    let blocks: BTreeSet<_> = plan
        .explanations
        .iter()
        .map(|explanation| explanation.invalidated_block)
        .collect();
    assert_eq!(blocks, BTreeSet::from([MAIN, MOTOR_FB, MOTOR_DB]));
    assert_eq!(
        plan.explanations
            .iter()
            .find(|item| item.invalidated_block == MOTOR_DB)
            .unwrap()
            .code,
        InvalidationCode::InstanceLayoutChanged
    );
}

#[test]
fn recursive_calls_are_blocking_and_report_a_canonical_cycle() {
    let mut program = valid_program();
    edit_block(&mut program, LEAF_FC, |leaf| {
        leaf.calls.push(CallSite {
            id: CallSiteId::new(102),
            instruction: CALL_FC,
            callee: HELPER_FC,
            bindings: vec![
                input_binding(HELPER_IN, LEAF_IN),
                input_binding(HELPER_OUT, LEAF_OUT),
            ],
            instance_owner: None,
        });
    });
    let report = validate_program(&program);
    assert!(report.has(IssueCode::RecursiveCallCycle));
    let cycle = &report
        .issues
        .iter()
        .find(|issue| issue.code == IssueCode::RecursiveCallCycle)
        .unwrap()
        .cycle;
    assert_eq!(cycle, &vec![LEAF_FC, HELPER_FC, LEAF_FC]);
}

#[test]
fn missing_mismatched_and_aliased_instruction_state_are_blocking() {
    let mut missing = valid_program();
    edit_block(&mut missing, MOTOR_FB, |motor| {
        motor.instructions[0].state_owner = None;
    });
    assert!(validate_program(&missing).has(IssueCode::MissingInstructionState));

    let mut mismatch = valid_program();
    edit_block(&mut mismatch, MOTOR_FB, |motor| {
        motor
            .interface
            .members
            .get_mut(&MOTOR_TIMER_STATE)
            .unwrap()
            .data_type = DataType::InstructionState(StateKind::Edge);
    });
    assert!(validate_program(&mismatch).has(IssueCode::InstructionStateTypeMismatch));

    let mut alias = valid_program();
    edit_block(&mut alias, MOTOR_FB, |motor| {
        motor.instructions.push(InstructionUse {
            id: InstructionUseId::new(202),
            instruction: TIMER_ON_DELAY,
            state_owner: Some(VariableRef::CallerMember(MOTOR_TIMER_STATE)),
        });
    });
    assert!(validate_program(&alias).has(IssueCode::InstructionStateAlias));
}

#[test]
fn writable_call_aliases_are_rejected() {
    let mut program = valid_program();
    edit_block(&mut program, HELPER_FC, |helper| {
        helper.calls[0].bindings[0].actual =
            BindingActual::Variable(VariableRef::CallerMember(HELPER_OUT));
    });
    let report = validate_program(&program);
    assert!(report.has(IssueCode::AliasConflict));
}

#[test]
fn canonical_order_is_enforced_for_interfaces_instructions_calls_and_bindings() {
    let mut program = valid_program();
    edit_block(&mut program, LEAF_FC, |leaf| {
        leaf.interface.ordered_member_ids.reverse();
    });
    edit_block(&mut program, MAIN, |main| {
        main.calls.reverse();
    });
    edit_block(&mut program, HELPER_FC, |helper| {
        helper.calls[0].bindings.reverse();
    });
    edit_block(&mut program, MOTOR_FB, |motor| {
        motor.instructions.push(InstructionUse {
            id: InstructionUseId::new(100),
            instruction: NO_OP,
            state_owner: None,
        });
    });
    let report = validate_program(&program);
    assert!(report.has(IssueCode::InterfaceOrderMismatch));
    assert!(report.has(IssueCode::InstructionOrderMismatch));
    assert!(report.has(IssueCode::CallOrderMismatch));
    assert!(report.has(IssueCode::BindingOrderMismatch));
}

#[test]
fn global_db_bindings_create_explicit_data_dependencies() {
    const GLOBAL_DB: BlockId = BlockId::new(80);
    const DB_SOURCE: InterfaceMemberId = InterfaceMemberId::new(8_001);
    const DB_TARGET: InterfaceMemberId = InterfaceMemberId::new(8_002);

    let global = ProgramBlock::new(
        GLOBAL_DB,
        "SharedData",
        number(101),
        ProgramUnitKind::DataBlock(DataBlockKind::Global),
        BlockInterface::from_members([
            plain_member(
                DB_SOURCE,
                "Source",
                InterfaceRole::Static,
                DataType::Bool,
                0,
            ),
            plain_member(
                DB_TARGET,
                "Target",
                InterfaceRole::Static,
                DataType::Bool,
                1,
            ),
        ]),
    );
    let mut program = valid_program();
    program.insert_block(global).unwrap();
    edit_block(&mut program, MAIN, |main| {
        main.calls[0].bindings = vec![
            ParameterBinding {
                formal: HELPER_IN,
                actual: BindingActual::Variable(VariableRef::DataBlockMember {
                    data_block: GLOBAL_DB,
                    member: DB_SOURCE,
                }),
            },
            ParameterBinding {
                formal: HELPER_OUT,
                actual: BindingActual::Variable(VariableRef::DataBlockMember {
                    data_block: GLOBAL_DB,
                    member: DB_TARGET,
                }),
            },
        ];
    });
    let report = validate_program(&program);
    assert!(report.is_valid(), "unexpected issues: {:#?}", report.issues);
    assert!(report.dependency_graph.edges().iter().any(|edge| {
        edge.dependent == MAIN
            && edge.dependency == GLOBAL_DB
            && edge.reason == DependencyReason::DataUse
    }));

    let replacement = BlockInterface::from_members(
        program
            .block(GLOBAL_DB)
            .unwrap()
            .interface
            .members
            .values()
            .cloned()
            .chain(std::iter::once(plain_member(
                InterfaceMemberId::new(8_003),
                "Spare",
                InterfaceRole::Static,
                DataType::Bool,
                2,
            ))),
    );
    let plan = program
        .explain_interface_change(GLOBAL_DB, &replacement)
        .unwrap();
    assert!(plan.delta.data_layout_changed);
    assert_eq!(
        plan.explanations
            .iter()
            .find(|item| item.invalidated_block == MAIN)
            .unwrap()
            .code,
        InvalidationCode::DataLayoutChanged
    );
}

#[test]
fn real_nan_defaults_require_the_registry_wide_canonical_encoding() {
    let add_threshold = |program: &mut ControllerProgram, bits| {
        edit_block(program, LEAF_FC, |leaf| {
            let mut value = plain_member(
                InterfaceMemberId::new(2_050),
                "Threshold",
                InterfaceRole::Input,
                DataType::Real,
                1,
            );
            value.default_value = Some(CanonicalValue::RealBits(bits));
            leaf.interface = BlockInterface::from_members(
                leaf.interface
                    .members
                    .values()
                    .cloned()
                    .chain(std::iter::once(value)),
            );
        });
    };

    let mut noncanonical = valid_program();
    add_threshold(&mut noncanonical, 0x7f80_0001);
    assert!(validate_program(&noncanonical).has(IssueCode::MemberValueTypeMismatch));

    let mut canonical = valid_program();
    add_threshold(&mut canonical, 0x7fc0_0000);
    assert!(validate_program(&canonical).is_valid());
}

#[test]
fn call_binding_direction_type_and_completeness_are_checked_by_stable_formal_id() {
    let mut program = valid_program();
    edit_block(&mut program, HELPER_FC, |helper| {
        let call = &mut helper.calls[0];
        call.bindings.remove(1);
        call.bindings.push(ParameterBinding {
            formal: InterfaceMemberId::new(99_999),
            actual: BindingActual::Literal(CanonicalValue::DInt(1)),
        });
    });
    let report = validate_program(&program);
    assert!(report.has(IssueCode::MissingBinding));
    assert!(report.has(IssueCode::UnknownFormal));
}

#[test]
fn role_and_metadata_legality_is_blocking() {
    let mut program = valid_program();
    let mut illegal_static = plain_member(
        InterfaceMemberId::new(8_001),
        "IllegalState",
        InterfaceRole::Static,
        DataType::Bool,
        0,
    );
    illegal_static.default_value = Some(CanonicalValue::Bool(true));
    edit_block(&mut program, LEAF_FC, |leaf| {
        leaf.interface = BlockInterface::from_members(
            leaf.interface
                .members
                .values()
                .cloned()
                .chain(std::iter::once(illegal_static)),
        );
    });
    let report = validate_program(&program);
    assert!(report.has(IssueCode::RoleNotAllowed));
    assert!(report.has(IssueCode::MemberMetadataIllegal));
}

#[test]
fn ob_declarations_and_engineering_number_spaces_are_checked() {
    let mut program = valid_program();
    program
        .insert_block(ProgramBlock::new(
            BlockId::new(50),
            "SecondMain",
            number(2),
            ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain),
            BlockInterface::default(),
        ))
        .unwrap();
    program
        .insert_block(ProgramBlock::new(
            BlockId::new(51),
            "BadTimed",
            number(3),
            ProgramUnitKind::OrganizationBlock(ObDeclaration::TimedCyclic {
                period_milliseconds: 5,
                offset_milliseconds: 5,
                priority: 0,
            }),
            BlockInterface::default(),
        ))
        .unwrap();
    program
        .insert_block(ProgramBlock::new(
            BlockId::new(52),
            "DuplicateFcNumber",
            number(1),
            ProgramUnitKind::Function,
            BlockInterface::default(),
        ))
        .unwrap();
    let report = validate_program(&program);
    assert!(report.has(IssueCode::MultipleCyclicMain));
    assert!(report.has(IssueCode::InvalidTimedCyclic));
    assert!(report.has(IssueCode::DuplicateEngineeringNumber));
}

#[test]
fn unknown_or_misplaced_instruction_metadata_is_rejected() {
    let mut unknown = valid_program();
    edit_block(&mut unknown, MOTOR_FB, |motor| {
        motor.instructions[0].instruction = InstructionCode(0xffff);
    });
    assert!(validate_program(&unknown).has(IssueCode::UnknownInstruction));

    let mut call_in_body = valid_program();
    edit_block(&mut call_in_body, MOTOR_FB, |motor| {
        motor.instructions[0].instruction = CALL_FB;
    });
    assert!(validate_program(&call_in_body).has(IssueCode::CallInstructionInBody));
}

#[test]
fn model_and_registry_versions_are_not_silently_upgraded() {
    let valid = valid_program();
    let blocks = valid.blocks().clone();
    let stale = ControllerProgram::from_parts(99, ControllerId::new(7), 2, "old-registry", blocks);
    let report = validate_program(&stale);
    assert!(report.has(IssueCode::ModelSchemaMismatch));
    assert!(report.has(IssueCode::RegistryVersionMismatch));
}

#[test]
fn fb_instances_and_multi_instances_have_independent_structural_state_paths() {
    const CHILD_FB: BlockId = BlockId::new(60);
    const PARENT_FB: BlockId = BlockId::new(61);
    const PARENT_DB_A: BlockId = BlockId::new(62);
    const PARENT_DB_B: BlockId = BlockId::new(63);
    const CHILD_SLOT: InterfaceMemberId = InterfaceMemberId::new(6_001);

    let child = ProgramBlock::new(
        CHILD_FB,
        "Child",
        number(10),
        ProgramUnitKind::FunctionBlock,
        BlockInterface::default(),
    );
    let mut parent = ProgramBlock::new(
        PARENT_FB,
        "Parent",
        number(11),
        ProgramUnitKind::FunctionBlock,
        BlockInterface::from_members([plain_member(
            CHILD_SLOT,
            "ChildState",
            InterfaceRole::Static,
            DataType::BlockInstance(CHILD_FB),
            0,
        )]),
    );
    let child_owner = InstanceOwner::MultiInstance {
        owner_fb: PARENT_FB,
        static_member: CHILD_SLOT,
    };
    parent.calls.push(CallSite {
        id: CallSiteId::new(601),
        instruction: CALL_FB,
        callee: CHILD_FB,
        bindings: vec![],
        instance_owner: Some(child_owner),
    });
    let db_a = ProgramBlock::new(
        PARENT_DB_A,
        "ParentA",
        number(110),
        ProgramUnitKind::DataBlock(DataBlockKind::Instance { fb_type: PARENT_FB }),
        BlockInterface::default(),
    );
    let db_b = ProgramBlock::new(
        PARENT_DB_B,
        "ParentB",
        number(111),
        ProgramUnitKind::DataBlock(DataBlockKind::Instance { fb_type: PARENT_FB }),
        BlockInterface::default(),
    );
    let mut main = ProgramBlock::new(
        MAIN,
        "Main",
        number(1),
        ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain),
        BlockInterface::default(),
    );
    for (id, db) in [
        (CallSiteId::new(602), PARENT_DB_A),
        (CallSiteId::new(603), PARENT_DB_B),
    ] {
        main.calls.push(CallSite {
            id,
            instruction: CALL_FB,
            callee: PARENT_FB,
            bindings: vec![],
            instance_owner: Some(InstanceOwner::InstanceDb(db)),
        });
    }
    let mut program = ControllerProgram::new(ControllerId::new(8));
    for block in [main, child, parent, db_a, db_b] {
        program.insert_block(block).unwrap();
    }
    let report = validate_program(&program);
    assert!(report.is_valid(), "unexpected issues: {:#?}", report.issues);
    assert!(report.dependency_graph.edges().iter().any(|edge| {
        edge.dependent == PARENT_FB
            && edge.dependency == CHILD_FB
            && edge.reason == DependencyReason::MultiInstanceState
    }));

    let parent_a = InstanceOwner::InstanceDb(PARENT_DB_A)
        .materialize_path(None)
        .unwrap();
    let parent_b = InstanceOwner::InstanceDb(PARENT_DB_B)
        .materialize_path(None)
        .unwrap();
    let child_a = child_owner.materialize_path(Some(&parent_a)).unwrap();
    let child_b = child_owner.materialize_path(Some(&parent_b)).unwrap();
    assert_ne!(child_a, child_b);
    assert_eq!(child_a.multi_instance_slots, vec![CHILD_SLOT]);
    assert_eq!(child_b.multi_instance_slots, vec![CHILD_SLOT]);
    assert_eq!(child_a.root_instance_db, PARENT_DB_A);
    assert_eq!(child_b.root_instance_db, PARENT_DB_B);
}

#[test]
fn state_ownership_cycles_and_wrong_instance_bindings_block_validation() {
    const FB_A: BlockId = BlockId::new(70);
    const FB_B: BlockId = BlockId::new(71);
    const SLOT_A: InterfaceMemberId = InterfaceMemberId::new(7_001);
    const SLOT_B: InterfaceMemberId = InterfaceMemberId::new(7_002);

    let fb_a = ProgramBlock::new(
        FB_A,
        "A",
        number(20),
        ProgramUnitKind::FunctionBlock,
        BlockInterface::from_members([plain_member(
            SLOT_A,
            "BState",
            InterfaceRole::Static,
            DataType::BlockInstance(FB_B),
            0,
        )]),
    );
    let fb_b = ProgramBlock::new(
        FB_B,
        "B",
        number(21),
        ProgramUnitKind::FunctionBlock,
        BlockInterface::from_members([plain_member(
            SLOT_B,
            "AState",
            InterfaceRole::Static,
            DataType::BlockInstance(FB_A),
            0,
        )]),
    );
    let main = ProgramBlock::new(
        MAIN,
        "Main",
        number(1),
        ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain),
        BlockInterface::default(),
    );
    let mut program = ControllerProgram::new(ControllerId::new(9));
    for block in [main, fb_a, fb_b] {
        program.insert_block(block).unwrap();
    }
    assert!(validate_program(&program).has(IssueCode::StateOwnershipCycle));

    let mut wrong_db = valid_program();
    edit_block(&mut wrong_db, MAIN, |main| {
        main.calls[1].instance_owner = Some(InstanceOwner::InstanceDb(BlockId::new(999)));
    });
    assert!(validate_program(&wrong_db).has(IssueCode::InstanceTypeMismatch));
}

#[test]
fn diagnostics_and_graphs_are_deterministic_across_insertion_order() {
    let first = valid_program();
    let mut second = ControllerProgram::new(first.controller_id());
    for block in first.blocks().values().rev().cloned() {
        second.insert_block(block).unwrap();
    }
    assert_eq!(validate_program(&first), validate_program(&second));
}
