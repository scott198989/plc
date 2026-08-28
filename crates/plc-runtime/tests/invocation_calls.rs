use plc_runtime::*;
use plc_types::CanonicalF32;

const UNIVERSE: UniverseId = UniverseId(0xCA11);
const CONTROLLER: VirtualControllerId = VirtualControllerId(0xC011);

const FORMAL_ENABLE: u16 = 0x0001;
const FORMAL_ENABLE_OUTPUT: u16 = 0x0002;
const FORMAL_INPUT: u16 = 0x0010;
const FORMAL_OUTPUT: u16 = 0x0011;
const FORMAL_CLOCK: u16 = 0x0030;
const FORMAL_PRESET_TIME: u16 = 0x0031;
const FORMAL_ELAPSED_TIME: u16 = 0x0032;
const FORMAL_MINIMUM: u16 = 0x0050;
const FORMAL_LIMIT_INPUT: u16 = 0x0051;
const FORMAL_MAXIMUM: u16 = 0x0052;
const FORMAL_LIMIT_OUTPUT: u16 = 0x0053;

fn memory(id: u32, value: CanonicalValue, retentive: bool) -> MemoryDefinition {
    MemoryDefinition {
        id: MemoryId(id),
        value_type: value.value_type(),
        loaded_start: value,
        retentive,
    }
}

fn instruction(id: u32, source_identity: u128, operation: Operation) -> Instruction {
    Instruction::new(id, source_identity, operation)
}

fn package(memory: Vec<MemoryDefinition>, cyclic: ProgramBlock) -> ArtifactPackage {
    ArtifactPackage::seal_verified(ArtifactSpec::edu21(
        Sha256::digest(b"invocation-call-profile"),
        memory,
        vec![],
        vec![],
        ProgramImage {
            startup: None,
            timed: vec![],
            cyclic,
        },
    ))
    .expect("runtime call artifact must verify")
}

fn running(package: &ArtifactPackage) -> VirtualController {
    let mut controller = VirtualController::new(UNIVERSE, CONTROLLER, 0x5eed);
    controller.power_on().expect("power on");
    controller
        .install_verified_artifact(package)
        .expect("verified artifact load");
    controller
        .request_run(RestartKind::Resume)
        .expect("RUN transition");
    controller
}

fn limit_invocation(
    minimum: Operand,
    input: Operand,
    maximum: Operand,
    value_type: ValueType,
    activation: Option<RuntimeActivation>,
) -> RuntimeInstructionInvocation {
    let mut outputs = vec![RuntimeDeclaredOutput {
        formal: RuntimeFormalRef::Instruction(FORMAL_LIMIT_OUTPUT),
        value_type,
    }];
    if activation.is_some() {
        outputs.insert(
            0,
            RuntimeDeclaredOutput {
                formal: RuntimeFormalRef::Instruction(FORMAL_ENABLE_OUTPUT),
                value_type: ValueType::Bool,
            },
        );
    }
    RuntimeInstructionInvocation {
        instruction: RuntimeInstructionCode::Limit,
        inputs: vec![
            RuntimeBoundInput {
                formal: RuntimeFormalRef::Instruction(FORMAL_MINIMUM),
                source: minimum,
            },
            RuntimeBoundInput {
                formal: RuntimeFormalRef::Instruction(FORMAL_LIMIT_INPUT),
                source: input,
            },
            RuntimeBoundInput {
                formal: RuntimeFormalRef::Instruction(FORMAL_MAXIMUM),
                source: maximum,
            },
        ],
        outputs,
        instance: None,
        activation,
    }
}

#[test]
fn limit_clamps_and_runtime_invalid_range_faults_before_output_store() {
    let output = MemoryId(1);
    let invalid_output = MemoryId(2);
    let artifact = package(
        vec![
            memory(1, CanonicalValue::I32(-1), false),
            memory(2, CanonicalValue::I32(77), false),
        ],
        ProgramBlock {
            id: BlockId(1),
            instructions: vec![
                instruction(
                    1,
                    0x501,
                    Operation::InvokeInstruction(limit_invocation(
                        Operand::Constant(CanonicalValue::I32(10)),
                        Operand::Constant(CanonicalValue::I32(99)),
                        Operand::Constant(CanonicalValue::I32(20)),
                        ValueType::I32,
                        None,
                    )),
                ),
                invocation_projection(2, 1, FORMAL_LIMIT_OUTPUT, output),
                instruction(
                    3,
                    0x503,
                    Operation::InvokeInstruction(limit_invocation(
                        Operand::Constant(CanonicalValue::I32(20)),
                        Operand::Constant(CanonicalValue::I32(15)),
                        Operand::Constant(CanonicalValue::I32(10)),
                        ValueType::I32,
                        None,
                    )),
                ),
                invocation_projection(4, 3, FORMAL_LIMIT_OUTPUT, invalid_output),
            ],
        },
    );
    let mut controller = running(&artifact);
    let event = match controller.run_scan().expect("scan") {
        RunOutcome::Faulted(event) => event,
        RunOutcome::Completed(_) => panic!("dynamic MN greater than MX must fault"),
    };
    assert_eq!(event.code, DiagnosticCode::InvalidArgument);
    assert_eq!(
        controller.actual_memory(output),
        Some(CanonicalValue::I32(20))
    );
    assert_eq!(
        controller.actual_memory(invalid_output),
        Some(CanonicalValue::I32(77)),
        "the failing LIMIT must not publish or project an OUT value"
    );
}

#[test]
fn floating_limit_canonicalizes_any_nan_without_range_fault() {
    let output = MemoryId(1);
    let artifact = package(
        vec![memory(
            1,
            CanonicalValue::F32(CanonicalF32::new(1.0)),
            false,
        )],
        ProgramBlock {
            id: BlockId(1),
            instructions: vec![
                instruction(
                    1,
                    0x511,
                    Operation::InvokeInstruction(limit_invocation(
                        Operand::Constant(CanonicalValue::F32(CanonicalF32::new(20.0))),
                        Operand::Constant(CanonicalValue::F32(CanonicalF32::new(f32::NAN))),
                        Operand::Constant(CanonicalValue::F32(CanonicalF32::new(10.0))),
                        ValueType::F32,
                        None,
                    )),
                ),
                invocation_projection(2, 1, FORMAL_LIMIT_OUTPUT, output),
            ],
        },
    );
    let mut controller = running(&artifact);
    assert!(matches!(
        controller.run_scan(),
        Ok(RunOutcome::Completed(_))
    ));
    assert_eq!(
        controller.actual_memory(output),
        Some(CanonicalValue::F32(CanonicalF32::new(f32::NAN)))
    );
    assert!(controller.diagnostics().is_empty());
}

#[test]
fn disabled_limit_publishes_default_out_and_false_eno_without_reading_values() {
    let output = MemoryId(2);
    let eno = MemoryId(3);
    let activation = RuntimeActivation {
        enable: Operand::Memory(MemoryId(1)),
        enable_formal: FORMAL_ENABLE,
        status_formal: FORMAL_ENABLE_OUTPUT,
        status_when_disabled: false,
        when_disabled: RuntimeDisabledBehavior::DefaultOutputsNoStateChange,
    };
    let artifact = package(
        vec![
            memory(1, CanonicalValue::Bool(false), false),
            memory(2, CanonicalValue::I64(91), false),
            memory(3, CanonicalValue::Bool(true), false),
        ],
        ProgramBlock {
            id: BlockId(1),
            instructions: vec![
                instruction(
                    1,
                    0x521,
                    Operation::InvokeInstruction(limit_invocation(
                        Operand::Constant(CanonicalValue::I64(100)),
                        Operand::Constant(CanonicalValue::I64(0)),
                        Operand::Constant(CanonicalValue::I64(-100)),
                        ValueType::I64,
                        Some(activation),
                    )),
                ),
                invocation_projection(2, 1, FORMAL_ENABLE_OUTPUT, eno),
                invocation_projection(3, 1, FORMAL_LIMIT_OUTPUT, output),
            ],
        },
    );
    let mut controller = running(&artifact);
    assert!(matches!(
        controller.run_scan(),
        Ok(RunOutcome::Completed(_))
    ));
    assert_eq!(
        controller.actual_memory(output),
        Some(CanonicalValue::I64(0))
    );
    assert_eq!(
        controller.actual_memory(eno),
        Some(CanonicalValue::Bool(false))
    );
}

#[test]
fn disabled_move_publishes_false_eno_and_suppresses_downstream_effects() {
    let enable = MemoryId(1);
    let source = MemoryId(2);
    let projected_output = MemoryId(3);
    let projected_eno = MemoryId(4);
    let downstream = MemoryId(5);
    let invoke = RuntimeInstructionInvocation {
        instruction: RuntimeInstructionCode::Move,
        inputs: vec![RuntimeBoundInput {
            formal: RuntimeFormalRef::Instruction(FORMAL_INPUT),
            source: Operand::Memory(source),
        }],
        outputs: vec![
            RuntimeDeclaredOutput {
                formal: RuntimeFormalRef::Instruction(FORMAL_ENABLE_OUTPUT),
                value_type: ValueType::Bool,
            },
            RuntimeDeclaredOutput {
                formal: RuntimeFormalRef::Instruction(FORMAL_OUTPUT),
                value_type: ValueType::I32,
            },
        ],
        instance: None,
        activation: Some(RuntimeActivation {
            enable: Operand::Memory(enable),
            enable_formal: FORMAL_ENABLE,
            status_formal: FORMAL_ENABLE_OUTPUT,
            status_when_disabled: false,
            when_disabled: RuntimeDisabledBehavior::SuppressEffects,
        }),
    };
    let artifact = package(
        vec![
            memory(1, CanonicalValue::Bool(false), false),
            memory(2, CanonicalValue::I32(42), false),
            memory(3, CanonicalValue::I32(7), false),
            memory(4, CanonicalValue::Bool(true), false),
            memory(5, CanonicalValue::I32(99), false),
        ],
        ProgramBlock {
            id: BlockId(1),
            instructions: vec![
                instruction(1, 0x101, Operation::InvokeInstruction(invoke)),
                instruction(
                    2,
                    0x102,
                    Operation::InvocationOutput {
                        invocation_id: 1,
                        formal: RuntimeFormalRef::Instruction(FORMAL_OUTPUT),
                        target: projected_output,
                    },
                ),
                instruction(
                    3,
                    0x103,
                    Operation::InvocationOutput {
                        invocation_id: 1,
                        formal: RuntimeFormalRef::Instruction(FORMAL_ENABLE_OUTPUT),
                        target: projected_eno,
                    },
                ),
                instruction(
                    4,
                    0x104,
                    Operation::Copy {
                        source: Operand::Memory(projected_output),
                        target: downstream,
                    },
                ),
                instruction(
                    5,
                    0x105,
                    Operation::SetMemory {
                        target: enable,
                        value: CanonicalValue::Bool(true),
                    },
                ),
            ],
        },
    );
    let mut controller = running(&artifact);
    let report = match controller.run_scan().expect("scan") {
        RunOutcome::Completed(report) => report,
        RunOutcome::Faulted(event) => panic!("disabled MOVE faulted: {event:?}"),
    };

    assert_eq!(report.work_units, 5);
    assert_eq!(
        controller.actual_memory(projected_output),
        Some(CanonicalValue::I32(7))
    );
    assert_eq!(
        controller.actual_memory(projected_eno),
        Some(CanonicalValue::Bool(false))
    );
    assert_eq!(
        controller.actual_memory(downstream),
        Some(CanonicalValue::I32(99))
    );
    let enabled = match controller.run_scan().expect("enabled scan") {
        RunOutcome::Completed(report) => report,
        RunOutcome::Faulted(event) => panic!("enabled MOVE faulted: {event:?}"),
    };
    assert_eq!(enabled.work_units, 5);
    assert_eq!(
        controller.actual_memory(projected_output),
        Some(CanonicalValue::I32(42))
    );
    assert_eq!(
        controller.actual_memory(projected_eno),
        Some(CanonicalValue::Bool(true))
    );
    assert_eq!(
        controller.actual_memory(downstream),
        Some(CanonicalValue::I32(42))
    );
    assert!(controller.diagnostics().is_empty());
}

#[test]
#[allow(clippy::too_many_lines)]
fn disabled_stateful_modes_preserve_or_default_without_state_changes() {
    let timer = RuntimeInstructionInvocation {
        instruction: RuntimeInstructionCode::TimerOnDelay,
        inputs: vec![
            RuntimeBoundInput {
                formal: RuntimeFormalRef::Instruction(FORMAL_INPUT),
                source: Operand::Constant(CanonicalValue::Bool(true)),
            },
            RuntimeBoundInput {
                formal: RuntimeFormalRef::Instruction(FORMAL_PRESET_TIME),
                source: Operand::Constant(CanonicalValue::TimeMs(30)),
            },
        ],
        outputs: vec![
            RuntimeDeclaredOutput {
                formal: RuntimeFormalRef::Instruction(FORMAL_ENABLE_OUTPUT),
                value_type: ValueType::Bool,
            },
            RuntimeDeclaredOutput {
                formal: RuntimeFormalRef::Instruction(FORMAL_OUTPUT),
                value_type: ValueType::Bool,
            },
            RuntimeDeclaredOutput {
                formal: RuntimeFormalRef::Instruction(FORMAL_ELAPSED_TIME),
                value_type: ValueType::TimeMs,
            },
        ],
        instance: Some(RuntimeInstructionInstance {
            stable_id: 0x710,
            kind: RuntimeInstructionStateKind::Timer,
            retentive: false,
        }),
        activation: Some(RuntimeActivation {
            enable: Operand::Memory(MemoryId(1)),
            enable_formal: FORMAL_ENABLE,
            status_formal: FORMAL_ENABLE_OUTPUT,
            status_when_disabled: false,
            when_disabled: RuntimeDisabledBehavior::PreserveOutputsNoStateChange,
        }),
    };
    let edge = RuntimeInstructionInvocation {
        instruction: RuntimeInstructionCode::RisingEdge,
        inputs: vec![RuntimeBoundInput {
            formal: RuntimeFormalRef::Instruction(FORMAL_CLOCK),
            source: Operand::Constant(CanonicalValue::Bool(true)),
        }],
        outputs: vec![
            RuntimeDeclaredOutput {
                formal: RuntimeFormalRef::Instruction(FORMAL_ENABLE_OUTPUT),
                value_type: ValueType::Bool,
            },
            RuntimeDeclaredOutput {
                formal: RuntimeFormalRef::Instruction(FORMAL_OUTPUT),
                value_type: ValueType::Bool,
            },
        ],
        instance: Some(RuntimeInstructionInstance {
            stable_id: 0x711,
            kind: RuntimeInstructionStateKind::Edge,
            retentive: false,
        }),
        activation: Some(RuntimeActivation {
            enable: Operand::Memory(MemoryId(5)),
            enable_formal: FORMAL_ENABLE,
            status_formal: FORMAL_ENABLE_OUTPUT,
            status_when_disabled: false,
            when_disabled: RuntimeDisabledBehavior::DefaultOutputsNoStateChange,
        }),
    };
    let artifact = package(
        vec![
            memory(1, CanonicalValue::Bool(true), false),
            memory(2, CanonicalValue::Bool(false), false),
            memory(3, CanonicalValue::TimeMs(0), false),
            memory(4, CanonicalValue::Bool(false), false),
            memory(5, CanonicalValue::Bool(false), false),
            memory(6, CanonicalValue::Bool(true), false),
            memory(7, CanonicalValue::Bool(true), false),
        ],
        ProgramBlock {
            id: BlockId(1),
            instructions: vec![
                instruction(1, 0x201, Operation::InvokeInstruction(timer)),
                invocation_projection(2, 1, FORMAL_ENABLE_OUTPUT, MemoryId(4)),
                invocation_projection(3, 1, FORMAL_OUTPUT, MemoryId(2)),
                invocation_projection(4, 1, FORMAL_ELAPSED_TIME, MemoryId(3)),
                instruction(
                    5,
                    0x205,
                    Operation::SetMemory {
                        target: MemoryId(1),
                        value: CanonicalValue::Bool(false),
                    },
                ),
                instruction(6, 0x206, Operation::InvokeInstruction(edge)),
                invocation_projection(7, 6, FORMAL_ENABLE_OUTPUT, MemoryId(7)),
                invocation_projection(8, 6, FORMAL_OUTPUT, MemoryId(6)),
                instruction(
                    9,
                    0x209,
                    Operation::SetMemory {
                        target: MemoryId(5),
                        value: CanonicalValue::Bool(true),
                    },
                ),
            ],
        },
    );
    let mut controller = running(&artifact);
    assert!(matches!(
        controller.run_scan(),
        Ok(RunOutcome::Completed(_))
    ));
    assert_eq!(
        controller.actual_memory(MemoryId(2)),
        Some(CanonicalValue::Bool(false))
    );
    assert_eq!(
        controller.actual_memory(MemoryId(3)),
        Some(CanonicalValue::TimeMs(10))
    );
    assert_eq!(
        controller.actual_memory(MemoryId(4)),
        Some(CanonicalValue::Bool(true))
    );
    assert_eq!(
        controller.actual_memory(MemoryId(6)),
        Some(CanonicalValue::Bool(false))
    );
    assert_eq!(
        controller.actual_memory(MemoryId(7)),
        Some(CanonicalValue::Bool(false))
    );

    assert!(matches!(
        controller.run_scan(),
        Ok(RunOutcome::Completed(_))
    ));
    assert_eq!(
        controller.actual_memory(MemoryId(2)),
        Some(CanonicalValue::Bool(false))
    );
    assert_eq!(
        controller.actual_memory(MemoryId(3)),
        Some(CanonicalValue::TimeMs(10))
    );
    assert_eq!(
        controller.actual_memory(MemoryId(4)),
        Some(CanonicalValue::Bool(false))
    );
    assert_eq!(
        controller.actual_memory(MemoryId(6)),
        Some(CanonicalValue::Bool(true))
    );
    assert_eq!(
        controller.actual_memory(MemoryId(7)),
        Some(CanonicalValue::Bool(true))
    );

    assert!(matches!(
        controller.run_scan(),
        Ok(RunOutcome::Completed(_))
    ));
    assert_eq!(
        controller.actual_memory(MemoryId(3)),
        Some(CanonicalValue::TimeMs(10))
    );
    assert_eq!(
        controller.actual_memory(MemoryId(6)),
        Some(CanonicalValue::Bool(false))
    );
}

fn invocation_projection(
    operation_id: u32,
    invocation_id: u32,
    formal: u16,
    target: MemoryId,
) -> Instruction {
    instruction(
        operation_id,
        0x9000 + u128::from(operation_id),
        Operation::InvocationOutput {
            invocation_id,
            formal: RuntimeFormalRef::Instruction(formal),
            target,
        },
    )
}

fn accumulator_call(call_site_identity: u128, root_instance: u128, input: i32) -> RuntimeBlockCall {
    let members = vec![
        RuntimeFrameMember {
            formal: 0xA001,
            memory: MemoryId(10),
            value_type: ValueType::I32,
            role: RuntimeFrameMemberRole::Input,
            declared_order: 0,
            initial_value: CanonicalValue::I32(0),
            retentive: false,
        },
        RuntimeFrameMember {
            formal: 0xA002,
            memory: MemoryId(11),
            value_type: ValueType::I32,
            role: RuntimeFrameMemberRole::Output,
            declared_order: 0,
            initial_value: CanonicalValue::I32(0),
            retentive: false,
        },
        RuntimeFrameMember {
            formal: 0xA003,
            memory: MemoryId(12),
            value_type: ValueType::I32,
            role: RuntimeFrameMemberRole::Static,
            declared_order: 0,
            initial_value: CanonicalValue::I32(0),
            retentive: true,
        },
    ];
    RuntimeBlockCall {
        kind: RuntimeCallKind::FunctionBlock,
        target_identity: 0xFB01,
        signature_fingerprint: runtime_block_signature_fingerprint(0xFB01, &members),
        call_site_identity,
        inputs: vec![RuntimeBoundInput {
            formal: RuntimeFormalRef::BlockMember(0xA001),
            source: Operand::Constant(CanonicalValue::I32(input)),
        }],
        outputs: vec![RuntimeDeclaredOutput {
            formal: RuntimeFormalRef::BlockMember(0xA002),
            value_type: ValueType::I32,
        }],
        instance: Some(RuntimeFunctionBlockInstance {
            root_instance,
            multi_instance_slots: vec![],
        }),
        activation: None,
        frame_members: members,
        callee: ProgramBlock {
            id: BlockId(50),
            instructions: vec![
                instruction(
                    1,
                    0x501,
                    Operation::AddI32 {
                        left: Operand::Memory(MemoryId(12)),
                        right: Operand::Memory(MemoryId(10)),
                        target: MemoryId(12),
                    },
                ),
                instruction(
                    2,
                    0x502,
                    Operation::Copy {
                        source: Operand::Memory(MemoryId(12)),
                        target: MemoryId(11),
                    },
                ),
            ],
        },
    }
}

fn accumulator_package() -> ArtifactPackage {
    let first_call = accumulator_call(0xC001, 0xD001, 1);
    let second_call = accumulator_call(0xC002, 0xD002, 10);
    package(
        vec![
            memory(10, CanonicalValue::I32(0), false),
            memory(11, CanonicalValue::I32(0), false),
            memory(12, CanonicalValue::I32(0), true),
            memory(13, CanonicalValue::I32(0), false),
            memory(14, CanonicalValue::I32(0), false),
        ],
        ProgramBlock {
            id: BlockId(1),
            instructions: vec![
                instruction(1, 0x101, Operation::CallBlock(first_call)),
                instruction(
                    2,
                    0x102,
                    Operation::InvocationOutput {
                        invocation_id: 1,
                        formal: RuntimeFormalRef::BlockMember(0xA002),
                        target: MemoryId(13),
                    },
                ),
                instruction(3, 0x103, Operation::CallBlock(second_call)),
                instruction(
                    4,
                    0x104,
                    Operation::InvocationOutput {
                        invocation_id: 3,
                        formal: RuntimeFormalRef::BlockMember(0xA002),
                        target: MemoryId(14),
                    },
                ),
            ],
        },
    )
}

fn run_accumulator_scan(controller: &mut VirtualController, a: i32, b: i32) -> ScanReport {
    let report = match controller.run_scan().expect("accumulator scan") {
        RunOutcome::Completed(report) => report,
        RunOutcome::Faulted(event) => panic!("FB accumulator faulted: {event:?}"),
    };
    assert_eq!(
        controller.actual_memory(MemoryId(13)),
        Some(CanonicalValue::I32(a))
    );
    assert_eq!(
        controller.actual_memory(MemoryId(14)),
        Some(CanonicalValue::I32(b))
    );
    report
}

#[test]
fn explicit_fb_instances_snapshot_and_follow_retain_reset_policy() {
    let artifact = accumulator_package();
    let mut controller = running(&artifact);
    let first = run_accumulator_scan(&mut controller, 1, 10);
    assert_eq!(first.work_units, 10);
    assert_eq!(
        first.executed_blocks,
        vec![BlockId(1), BlockId(50), BlockId(50)]
    );
    assert_eq!(first.call_boundaries.len(), 4);
    assert_eq!(first.call_boundaries[0].kind, CallBoundaryKind::Enter);
    assert_eq!(first.call_boundaries[1].kind, CallBoundaryKind::Return);
    assert_ne!(
        first.call_boundaries[0].instance,
        first.call_boundaries[2].instance
    );
    let snapshot = controller.capture_snapshot().expect("RUN snapshot");

    run_accumulator_scan(&mut controller, 2, 20);
    controller.request_stop().expect("STOP for restore");
    let approval = controller
        .prepare_restore(&snapshot)
        .expect("restore approval");
    controller
        .restore_snapshot(&snapshot, approval)
        .expect("atomic snapshot restore");
    controller
        .request_run(RestartKind::Resume)
        .expect("resume restored snapshot");
    run_accumulator_scan(&mut controller, 2, 20);

    controller.request_stop().expect("STOP for power cycle");
    controller
        .simulated_power_cycle()
        .expect("warm power cycle");
    controller
        .request_run(RestartKind::Resume)
        .expect("RUN after power cycle");
    run_accumulator_scan(&mut controller, 3, 30);

    controller.request_stop().expect("STOP for memory reset");
    controller.memory_reset().expect("full memory reset");
    controller
        .request_run(RestartKind::Resume)
        .expect("RUN after memory reset");
    run_accumulator_scan(&mut controller, 1, 10);
}

#[test]
fn nested_runtime_fault_retains_the_exact_callee_source_identity() {
    let members = vec![];
    let call = RuntimeBlockCall {
        kind: RuntimeCallKind::Function,
        target_identity: 0xFC01,
        signature_fingerprint: runtime_block_signature_fingerprint(0xFC01, &members),
        call_site_identity: 0xCA11,
        inputs: vec![],
        outputs: vec![],
        instance: None,
        activation: None,
        frame_members: members,
        callee: ProgramBlock {
            id: BlockId(77),
            instructions: vec![instruction(
                7,
                0xF007,
                Operation::DivideI32 {
                    numerator: Operand::Constant(CanonicalValue::I32(1)),
                    denominator: Operand::Constant(CanonicalValue::I32(0)),
                    target: MemoryId(1),
                },
            )],
        },
    };
    let artifact = package(
        vec![memory(1, CanonicalValue::I32(0), false)],
        ProgramBlock {
            id: BlockId(1),
            instructions: vec![instruction(1, 0xF001, Operation::CallBlock(call))],
        },
    );
    let mut controller = running(&artifact);
    let event = match controller.run_scan().expect("faulting scan") {
        RunOutcome::Faulted(event) => event,
        RunOutcome::Completed(report) => panic!("faulting call completed: {report:?}"),
    };
    assert_eq!(event.code, DiagnosticCode::ArithmeticDivideByZero);
    let context = event.fault_context.expect("causal runtime context");
    assert_eq!(context.block_id, BlockId(77));
    assert_eq!(context.operation_id, 7);
    assert_eq!(context.source_identity, 0xF007);
    assert_eq!(context.scan_sequence, 1);
    assert_eq!(context.work_units_before_operation, 1);
    assert_eq!(controller.cpu_state(), CpuState::Faulted);
}

#[test]
fn unwritten_optional_fc_output_uses_the_declared_frame_initial_value() {
    let members = vec![RuntimeFrameMember {
        formal: 0xB001,
        memory: MemoryId(1),
        value_type: ValueType::I32,
        role: RuntimeFrameMemberRole::Output,
        declared_order: 0,
        initial_value: CanonicalValue::I32(9),
        retentive: false,
    }];
    let call = RuntimeBlockCall {
        kind: RuntimeCallKind::Function,
        target_identity: 0xFC09,
        signature_fingerprint: runtime_block_signature_fingerprint(0xFC09, &members),
        call_site_identity: 0xCA19,
        inputs: vec![],
        outputs: vec![RuntimeDeclaredOutput {
            formal: RuntimeFormalRef::BlockMember(0xB001),
            value_type: ValueType::I32,
        }],
        instance: None,
        activation: None,
        frame_members: members,
        callee: ProgramBlock {
            id: BlockId(9),
            instructions: vec![],
        },
    };
    let artifact = package(
        vec![memory(1, CanonicalValue::I32(0), false)],
        ProgramBlock {
            id: BlockId(1),
            instructions: vec![
                instruction(1, 0x901, Operation::CallBlock(call)),
                instruction(
                    2,
                    0x902,
                    Operation::InvocationOutput {
                        invocation_id: 1,
                        formal: RuntimeFormalRef::BlockMember(0xB001),
                        target: MemoryId(1),
                    },
                ),
            ],
        },
    );
    let mut controller = running(&artifact);
    assert!(matches!(
        controller.run_scan(),
        Ok(RunOutcome::Completed(_))
    ));
    assert_eq!(
        controller.actual_memory(MemoryId(1)),
        Some(CanonicalValue::I32(9))
    );
}

fn nested_call_block(remaining_calls: u8, id: u32) -> ProgramBlock {
    if remaining_calls == 0 {
        return ProgramBlock {
            id: BlockId(id),
            instructions: vec![instruction(1, u128::from(id), Operation::Noop)],
        };
    }
    let callee = nested_call_block(remaining_calls - 1, id + 1);
    let target_identity = 0x1000 + u128::from(id);
    ProgramBlock {
        id: BlockId(id),
        instructions: vec![instruction(
            1,
            u128::from(id),
            Operation::CallBlock(RuntimeBlockCall {
                kind: RuntimeCallKind::Function,
                target_identity,
                signature_fingerprint: runtime_block_signature_fingerprint(target_identity, &[]),
                call_site_identity: 0x2000 + u128::from(id),
                inputs: vec![],
                outputs: vec![],
                instance: None,
                activation: None,
                frame_members: vec![],
                callee,
            }),
        )],
    }
}

#[test]
fn invalid_signatures_and_excessive_call_depth_fail_closed_at_load_contract() {
    let deep = ArtifactSpec::edu21(
        Hash32::ZERO,
        vec![],
        vec![],
        vec![],
        ProgramImage {
            startup: None,
            timed: vec![],
            cyclic: nested_call_block(MAX_DYNAMIC_CALL_DEPTH + 1, 1),
        },
    );
    assert!(matches!(
        ArtifactPackage::seal_verified(deep),
        Err(ArtifactError::DynamicCallDepthExceeded(_))
    ));

    let bad_call = RuntimeBlockCall {
        kind: RuntimeCallKind::Function,
        target_identity: 0xFC01,
        signature_fingerprint: Hash32::ZERO,
        call_site_identity: 0xCA11,
        inputs: vec![],
        outputs: vec![],
        instance: None,
        activation: None,
        frame_members: vec![],
        callee: ProgramBlock {
            id: BlockId(2),
            instructions: vec![],
        },
    };
    let invalid_signature = ArtifactSpec::edu21(
        Hash32::ZERO,
        vec![],
        vec![],
        vec![],
        ProgramImage {
            startup: None,
            timed: vec![],
            cyclic: ProgramBlock {
                id: BlockId(1),
                instructions: vec![instruction(1, 1, Operation::CallBlock(bad_call))],
            },
        },
    );
    assert!(matches!(
        ArtifactPackage::seal_verified(invalid_signature),
        Err(ArtifactError::InvalidBlockSignature {
            block: BlockId(1),
            operation_id: 1
        })
    ));
}
