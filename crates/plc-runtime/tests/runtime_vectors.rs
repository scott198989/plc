use plc_runtime::*;

const UNIVERSE: UniverseId = UniverseId(0x1111);
const CONTROLLER: VirtualControllerId = VirtualControllerId(0x2222);
const INPUT: ChannelId = ChannelId(1);
const ANALOG_OUTPUT: ChannelId = ChannelId(2);
const BOOL_OUTPUT: ChannelId = ChannelId(3);

const INPUT_COPY: MemoryId = MemoryId(1);
const NON_RETAIN: MemoryId = MemoryId(2);
const RETAIN: MemoryId = MemoryId(3);
const ORDER_CELL: MemoryId = MemoryId(4);
const RISING: MemoryId = MemoryId(5);
const TIMER_Q: MemoryId = MemoryId(6);
const TIMER_ET: MemoryId = MemoryId(7);
const COUNTER_Q: MemoryId = MemoryId(8);
const COUNTER_CV: MemoryId = MemoryId(9);
const OUTPUT_ECHO: MemoryId = MemoryId(10);

fn instruction(id: u32, operation: Operation) -> Instruction {
    Instruction::new(id, 0x5a00 + u128::from(id), operation)
}

fn deterministic_package() -> ArtifactPackage {
    let memory = vec![
        MemoryDefinition {
            id: INPUT_COPY,
            value_type: ValueType::Bool,
            loaded_start: CanonicalValue::Bool(false),
            retentive: false,
        },
        MemoryDefinition {
            id: NON_RETAIN,
            value_type: ValueType::I32,
            loaded_start: CanonicalValue::I32(0),
            retentive: false,
        },
        MemoryDefinition {
            id: RETAIN,
            value_type: ValueType::I32,
            loaded_start: CanonicalValue::I32(100),
            retentive: true,
        },
        MemoryDefinition {
            id: ORDER_CELL,
            value_type: ValueType::I32,
            loaded_start: CanonicalValue::I32(0),
            retentive: false,
        },
        MemoryDefinition {
            id: RISING,
            value_type: ValueType::Bool,
            loaded_start: CanonicalValue::Bool(false),
            retentive: false,
        },
        MemoryDefinition {
            id: TIMER_Q,
            value_type: ValueType::Bool,
            loaded_start: CanonicalValue::Bool(false),
            retentive: false,
        },
        MemoryDefinition {
            id: TIMER_ET,
            value_type: ValueType::TimeMs,
            loaded_start: CanonicalValue::TimeMs(0),
            retentive: false,
        },
        MemoryDefinition {
            id: COUNTER_Q,
            value_type: ValueType::Bool,
            loaded_start: CanonicalValue::Bool(false),
            retentive: false,
        },
        MemoryDefinition {
            id: COUNTER_CV,
            value_type: ValueType::I32,
            loaded_start: CanonicalValue::I32(0),
            retentive: false,
        },
        MemoryDefinition {
            id: OUTPUT_ECHO,
            value_type: ValueType::I32,
            loaded_start: CanonicalValue::I32(0),
            retentive: false,
        },
    ];
    let channels = vec![
        ChannelDefinition {
            id: INPUT,
            direction: ChannelDirection::Input,
            value_type: ValueType::Bool,
            canonical_default: CanonicalValue::Bool(false),
        },
        ChannelDefinition {
            id: ANALOG_OUTPUT,
            direction: ChannelDirection::Output,
            value_type: ValueType::I32,
            canonical_default: CanonicalValue::I32(0),
        },
        ChannelDefinition {
            id: BOOL_OUTPUT,
            direction: ChannelDirection::Output,
            value_type: ValueType::Bool,
            canonical_default: CanonicalValue::Bool(false),
        },
    ];
    let states = vec![
        StateDefinition {
            id: StateId(1),
            loaded_start: StateStart::Edge { previous: false },
            retentive: false,
        },
        StateDefinition {
            id: StateId(2),
            loaded_start: StateStart::Timer {
                elapsed_ms: 0,
                output: false,
            },
            retentive: false,
        },
        StateDefinition {
            id: StateId(3),
            loaded_start: StateStart::Counter {
                count: 0,
                previous_input: false,
            },
            retentive: true,
        },
    ];
    let program = ProgramImage {
        startup: Some(ProgramBlock {
            id: BlockId(1),
            instructions: vec![instruction(
                1,
                Operation::SetMemory {
                    target: ORDER_CELL,
                    value: CanonicalValue::I32(5),
                },
            )],
        }),
        timed: vec![TimedTask {
            id: TaskId(1),
            first_due_ms: 0,
            period_ms: 20,
            semantic_order: 7,
            block: ProgramBlock {
                id: BlockId(10),
                instructions: vec![instruction(
                    10,
                    Operation::SetMemory {
                        target: ORDER_CELL,
                        value: CanonicalValue::I32(10),
                    },
                )],
            },
        }],
        cyclic: ProgramBlock {
            id: BlockId(20),
            instructions: vec![
                instruction(
                    20,
                    Operation::LoadInput {
                        channel: INPUT,
                        target: INPUT_COPY,
                    },
                ),
                instruction(
                    21,
                    Operation::RisingEdge {
                        source: Operand::Input(INPUT),
                        state: StateId(1),
                        target: RISING,
                    },
                ),
                instruction(
                    22,
                    Operation::TimerOnDelay {
                        input: Operand::Input(INPUT),
                        preset_ms: 30,
                        state: StateId(2),
                        output: TIMER_Q,
                        elapsed: TIMER_ET,
                    },
                ),
                instruction(
                    23,
                    Operation::CounterUp {
                        input: Operand::Input(INPUT),
                        reset: Operand::Constant(CanonicalValue::Bool(false)),
                        preset: 2,
                        state: StateId(3),
                        output: COUNTER_Q,
                        current: COUNTER_CV,
                    },
                ),
                instruction(
                    24,
                    Operation::AddI32 {
                        left: Operand::Memory(ORDER_CELL),
                        right: Operand::Constant(CanonicalValue::I32(1)),
                        target: ORDER_CELL,
                    },
                ),
                instruction(
                    25,
                    Operation::AddI32 {
                        left: Operand::Memory(NON_RETAIN),
                        right: Operand::Constant(CanonicalValue::I32(1)),
                        target: NON_RETAIN,
                    },
                ),
                instruction(
                    26,
                    Operation::AddI32 {
                        left: Operand::Memory(RETAIN),
                        right: Operand::Constant(CanonicalValue::I32(1)),
                        target: RETAIN,
                    },
                ),
                instruction(
                    27,
                    Operation::StoreOutput {
                        source: Operand::Memory(ORDER_CELL),
                        channel: ANALOG_OUTPUT,
                    },
                ),
                instruction(
                    28,
                    Operation::Copy {
                        source: Operand::Output(ANALOG_OUTPUT),
                        target: OUTPUT_ECHO,
                    },
                ),
                instruction(
                    29,
                    Operation::StoreOutput {
                        source: Operand::Memory(TIMER_Q),
                        channel: BOOL_OUTPUT,
                    },
                ),
            ],
        },
    };
    ArtifactPackage::seal_verified(ArtifactSpec::edu21(
        Sha256::digest(b"EDU-21-CORE-1.0"),
        memory,
        channels,
        states,
        program,
    ))
    .unwrap()
}

fn boot(package: &ArtifactPackage, seed: u64) -> VirtualController {
    let mut controller = VirtualController::new(UNIVERSE, CONTROLLER, seed);
    assert_eq!(controller.cpu_state(), CpuState::PoweredOff);
    assert_eq!(controller.universe_epoch(), 1);
    assert_eq!(controller.controller_epoch(), 1);
    controller.power_on().unwrap();
    assert_eq!(controller.cpu_state(), CpuState::Stop);
    assert!(matches!(
        controller.install_verified_artifact(package).unwrap(),
        InstallOutcome::Installed { .. }
    ));
    assert_eq!(controller.controller_epoch(), 2);
    controller.request_run(RestartKind::Resume).unwrap();
    assert_eq!(controller.cpu_state(), CpuState::Run);
    controller
}

fn set_input(controller: &mut VirtualController, id: u128, value: bool) -> InputReceipt {
    controller
        .set_virtual_input_raw(InputCommand {
            command_id: CommandId(id),
            idempotency_key: id + 10_000,
            controller_id: CONTROLLER,
            expected_controller_epoch: controller.controller_epoch(),
            channel_id: INPUT,
            value: CanonicalValue::Bool(value),
            audit_provenance_hash: Sha256::digest(&id.to_be_bytes()),
        })
        .unwrap()
}

fn completed(controller: &mut VirtualController) -> ScanReport {
    match controller.run_scan().unwrap() {
        RunOutcome::Completed(report) => report,
        RunOutcome::Faulted(event) => panic!("unexpected fault: {event:?}"),
    }
}

#[test]
fn cpu_states_and_immutable_load_contract_are_guarded() {
    let package = deterministic_package();
    let mut controller = VirtualController::new(UNIVERSE, CONTROLLER, 9);
    assert!(matches!(
        controller.request_run(RestartKind::Resume),
        Err(CommandError::IllegalCpuTransition {
            from: CpuState::PoweredOff,
            ..
        })
    ));
    controller.power_on().unwrap();
    controller.install_verified_artifact(&package).unwrap();
    let epoch = controller.controller_epoch();
    let hash = controller.semantic_state_hash();
    assert_eq!(
        controller.install_verified_artifact(&package).unwrap(),
        InstallOutcome::IdenticalNoOp {
            fingerprint: package.fingerprint()
        }
    );
    assert_eq!(controller.controller_epoch(), epoch);
    assert_eq!(controller.semantic_state_hash(), hash);

    let mut altered = package.spec().clone();
    altered.profile_fingerprint = Sha256::digest(b"altered");
    let corrupt = ArtifactPackage::from_untrusted_package(altered, package.fingerprint(), true);
    assert!(matches!(
        controller.install_verified_artifact(&corrupt),
        Err(CommandError::Artifact(
            ArtifactError::FingerprintMismatch { .. }
        ))
    ));
    assert_eq!(controller.loaded_fingerprint(), Some(package.fingerprint()));
}

#[test]
fn scan_order_is_timed_before_cyclic_and_outputs_commit_once() {
    let package = deterministic_package();
    let mut controller = boot(&package, 7);
    assert_eq!(
        controller.actual_memory(ORDER_CELL),
        Some(CanonicalValue::I32(5))
    );

    let first = completed(&mut controller);
    assert_eq!(first.executed_blocks, vec![BlockId(10), BlockId(20)]);
    assert_eq!(
        controller.actual_memory(ORDER_CELL),
        Some(CanonicalValue::I32(11))
    );
    assert_eq!(
        controller.actual_memory(OUTPUT_ECHO),
        Some(CanonicalValue::I32(11))
    );
    assert_eq!(
        controller
            .boundary()
            .delivered_output(ANALOG_OUTPUT)
            .unwrap()
            .canonical_value,
        CanonicalValue::I32(11)
    );
    assert_eq!(
        controller
            .boundary()
            .delivered_output(ANALOG_OUTPUT)
            .unwrap()
            .output_commit_scan_sequence,
        Some(1)
    );

    let second = completed(&mut controller);
    assert_eq!(second.executed_blocks, vec![BlockId(20)]);
    assert_eq!(
        controller.actual_memory(ORDER_CELL),
        Some(CanonicalValue::I32(12))
    );
    let third = completed(&mut controller);
    assert_eq!(third.executed_blocks, vec![BlockId(10), BlockId(20)]);
    assert_eq!(
        controller.actual_memory(ORDER_CELL),
        Some(CanonicalValue::I32(11))
    );
    assert_eq!(controller.virtual_time_ms(), 30);
}

#[test]
fn raw_natural_effective_and_delivered_layers_have_scan_boundary_timing() {
    let package = deterministic_package();
    let mut controller = boot(&package, 1);
    completed(&mut controller);
    assert_eq!(
        controller.natural_input(INPUT),
        Some(CanonicalValue::Bool(false))
    );

    let receipt = set_input(&mut controller, 1, true);
    assert_eq!(
        controller
            .boundary()
            .raw_input(INPUT)
            .unwrap()
            .accepted_event_sequence,
        receipt.accepted_event_sequence
    );
    assert_eq!(
        controller.natural_input(INPUT),
        Some(CanonicalValue::Bool(false))
    );
    assert_eq!(
        controller.effective_input(INPUT),
        Some(CanonicalValue::Bool(false))
    );

    completed(&mut controller);
    assert_eq!(
        controller.natural_input(INPUT),
        Some(CanonicalValue::Bool(true))
    );
    assert_eq!(
        controller.effective_input(INPUT),
        Some(CanonicalValue::Bool(true))
    );
    assert_eq!(
        controller.actual_memory(INPUT_COPY),
        Some(CanonicalValue::Bool(true))
    );

    let duplicate = controller
        .set_virtual_input_raw(InputCommand {
            command_id: CommandId(1),
            idempotency_key: 10_001,
            controller_id: CONTROLLER,
            expected_controller_epoch: controller.controller_epoch(),
            channel_id: INPUT,
            value: CanonicalValue::Bool(true),
            audit_provenance_hash: Sha256::digest(&1_u128.to_be_bytes()),
        })
        .unwrap();
    assert!(duplicate.duplicate);
    assert_eq!(
        duplicate.accepted_event_sequence,
        receipt.accepted_event_sequence
    );
}

#[test]
fn timer_edge_and_counter_follow_fixed_scan_vectors() {
    let package = deterministic_package();
    let mut controller = boot(&package, 2);
    set_input(&mut controller, 1, true);

    completed(&mut controller);
    assert_eq!(
        controller.actual_memory(RISING),
        Some(CanonicalValue::Bool(true))
    );
    assert_eq!(
        controller.actual_memory(TIMER_ET),
        Some(CanonicalValue::TimeMs(10))
    );
    assert_eq!(
        controller.actual_memory(TIMER_Q),
        Some(CanonicalValue::Bool(false))
    );
    assert_eq!(
        controller.actual_memory(COUNTER_CV),
        Some(CanonicalValue::I32(1))
    );

    completed(&mut controller);
    assert_eq!(
        controller.actual_memory(RISING),
        Some(CanonicalValue::Bool(false))
    );
    assert_eq!(
        controller.actual_memory(TIMER_ET),
        Some(CanonicalValue::TimeMs(20))
    );
    assert_eq!(
        controller.actual_memory(COUNTER_CV),
        Some(CanonicalValue::I32(1))
    );

    completed(&mut controller);
    assert_eq!(
        controller.actual_memory(TIMER_Q),
        Some(CanonicalValue::Bool(true))
    );
    assert_eq!(
        controller.actual_memory(TIMER_ET),
        Some(CanonicalValue::TimeMs(30))
    );
    assert_eq!(
        controller
            .boundary()
            .delivered_output(BOOL_OUTPUT)
            .unwrap()
            .canonical_value,
        CanonicalValue::Bool(true)
    );

    set_input(&mut controller, 2, false);
    completed(&mut controller);
    set_input(&mut controller, 3, true);
    completed(&mut controller);
    assert_eq!(
        controller.actual_memory(COUNTER_CV),
        Some(CanonicalValue::I32(2))
    );
    assert_eq!(
        controller.actual_memory(COUNTER_Q),
        Some(CanonicalValue::Bool(true))
    );
}

#[test]
fn warm_restart_and_memory_reset_apply_distinct_retention_policies() {
    let package = deterministic_package();
    let mut controller = boot(&package, 3);
    for _ in 0..3 {
        completed(&mut controller);
    }
    assert_eq!(
        controller.actual_memory(NON_RETAIN),
        Some(CanonicalValue::I32(3))
    );
    assert_eq!(
        controller.actual_memory(RETAIN),
        Some(CanonicalValue::I32(103))
    );
    assert_eq!(
        controller.retained_memory(RETAIN),
        Some(CanonicalValue::I32(103))
    );

    controller.request_stop().unwrap();
    controller.request_run(RestartKind::WarmRestart).unwrap();
    assert_eq!(
        controller.actual_memory(NON_RETAIN),
        Some(CanonicalValue::I32(0))
    );
    assert_eq!(
        controller.actual_memory(RETAIN),
        Some(CanonicalValue::I32(103))
    );

    controller.request_stop().unwrap();
    let fingerprint = controller.loaded_fingerprint();
    let epoch = controller.controller_epoch();
    controller.memory_reset().unwrap();
    assert_eq!(controller.controller_epoch(), epoch + 1);
    assert_eq!(controller.scan_sequence(), 0);
    assert_eq!(
        controller.actual_memory(NON_RETAIN),
        Some(CanonicalValue::I32(0))
    );
    assert_eq!(
        controller.actual_memory(RETAIN),
        Some(CanonicalValue::I32(100))
    );
    assert_eq!(
        controller.retained_memory(RETAIN),
        Some(CanonicalValue::I32(100))
    );
    assert_eq!(controller.loaded_fingerprint(), fingerprint);
}

#[test]
fn power_cycle_restores_retain_and_pause_never_advances_virtual_time() {
    let package = deterministic_package();
    let mut controller = boot(&package, 33);
    set_input(&mut controller, 1, true);
    completed(&mut controller);
    assert_eq!(
        controller.actual_memory(TIMER_ET),
        Some(CanonicalValue::TimeMs(10))
    );
    let paused_at = controller.virtual_time_ms();
    controller.pause_educational().unwrap();
    assert_eq!(controller.cpu_state(), CpuState::PausedEducational);
    assert_eq!(controller.virtual_time_ms(), paused_at);
    controller.resume_educational().unwrap();
    completed(&mut controller);
    assert_eq!(
        controller.actual_memory(TIMER_ET),
        Some(CanonicalValue::TimeMs(20))
    );

    let retained = controller.actual_memory(RETAIN);
    controller.request_stop().unwrap();
    let prior_epoch = controller.controller_epoch();
    controller.simulated_power_cycle().unwrap();
    assert_eq!(controller.cpu_state(), CpuState::Stop);
    assert_eq!(controller.controller_epoch(), prior_epoch + 1);
    assert_eq!(controller.scan_sequence(), 0);
    assert_eq!(
        controller.actual_memory(NON_RETAIN),
        Some(CanonicalValue::I32(0))
    );
    assert_eq!(controller.actual_memory(RETAIN), retained);
    assert_eq!(controller.invocation_ordinal(BlockId(20)), Some(0));
    assert_eq!(
        controller
            .boundary()
            .raw_input(INPUT)
            .unwrap()
            .canonical_value,
        CanonicalValue::Bool(false)
    );
}

fn fault_package(noop_count: usize) -> ArtifactPackage {
    let mut instructions = Vec::with_capacity(noop_count + 4);
    for id in 0..noop_count {
        instructions.push(instruction(id as u32 + 1, Operation::Noop));
    }
    if noop_count == 0 {
        instructions.extend([
            instruction(
                1,
                Operation::SetMemory {
                    target: MemoryId(1),
                    value: CanonicalValue::I32(9),
                },
            ),
            instruction(
                2,
                Operation::StoreOutput {
                    source: Operand::Memory(MemoryId(1)),
                    channel: ANALOG_OUTPUT,
                },
            ),
            instruction(
                3,
                Operation::DivideI32 {
                    numerator: Operand::Memory(MemoryId(1)),
                    denominator: Operand::Constant(CanonicalValue::I32(0)),
                    target: MemoryId(2),
                },
            ),
            instruction(
                4,
                Operation::SetMemory {
                    target: MemoryId(3),
                    value: CanonicalValue::I32(77),
                },
            ),
        ]);
    }
    ArtifactPackage::seal_verified(ArtifactSpec::edu21(
        Sha256::digest(b"EDU-21-CORE-1.0"),
        vec![
            MemoryDefinition {
                id: MemoryId(1),
                value_type: ValueType::I32,
                loaded_start: CanonicalValue::I32(0),
                retentive: false,
            },
            MemoryDefinition {
                id: MemoryId(2),
                value_type: ValueType::I32,
                loaded_start: CanonicalValue::I32(0),
                retentive: false,
            },
            MemoryDefinition {
                id: MemoryId(3),
                value_type: ValueType::I32,
                loaded_start: CanonicalValue::I32(0),
                retentive: false,
            },
        ],
        vec![ChannelDefinition {
            id: ANALOG_OUTPUT,
            direction: ChannelDirection::Output,
            value_type: ValueType::I32,
            canonical_default: CanonicalValue::I32(0),
        }],
        vec![],
        ProgramImage {
            startup: None,
            timed: vec![],
            cyclic: ProgramBlock {
                id: BlockId(99),
                instructions,
            },
        },
    ))
    .unwrap()
}

#[test]
fn fatal_fault_keeps_prior_writes_and_blocks_normal_output_commit() {
    let package = fault_package(0);
    let mut controller = boot(&package, 4);
    let event = match controller.run_scan().unwrap() {
        RunOutcome::Faulted(event) => event,
        RunOutcome::Completed(_) => panic!("division by zero must fault"),
    };
    assert_eq!(controller.cpu_state(), CpuState::Faulted);
    assert_eq!(controller.virtual_time_ms(), 0);
    assert_eq!(
        controller.actual_memory(MemoryId(1)),
        Some(CanonicalValue::I32(9))
    );
    assert_eq!(
        controller.actual_memory(MemoryId(3)),
        Some(CanonicalValue::I32(0))
    );
    assert_eq!(
        controller.natural_output(ANALOG_OUTPUT),
        Some(CanonicalValue::I32(9))
    );
    assert_eq!(
        controller.effective_output(ANALOG_OUTPUT),
        Some(CanonicalValue::I32(0))
    );
    let delivered = controller
        .boundary()
        .delivered_output(ANALOG_OUTPUT)
        .unwrap();
    assert_eq!(delivered.canonical_value, CanonicalValue::I32(0));
    assert_eq!(delivered.delivery_reason, DeliveryReason::FatalFaultDefault);
    assert_eq!(delivered.output_commit_scan_sequence, None);
    assert_eq!(event.code, DiagnosticCode::ArithmeticDivideByZero);
    assert_eq!(event.root_occurrence_id, event.occurrence_id);
    assert_eq!(event.fault_context.as_ref().unwrap().operation_id, 3);
    assert_eq!(
        event.fault_boundary_state_hash,
        Some(controller.last_state_hash())
    );
    assert!(
        controller
            .boundary_hashes()
            .last()
            .unwrap()
            .is_fatal_fault()
    );
}

#[test]
fn watchdog_charges_before_the_first_operation_over_budget() {
    let package = fault_package(MAX_WORK_UNITS_PER_SCAN as usize + 1);
    let mut controller = boot(&package, 5);
    let event = match controller.run_scan().unwrap() {
        RunOutcome::Faulted(event) => event,
        RunOutcome::Completed(_) => panic!("work budget must fault"),
    };
    assert_eq!(event.code, DiagnosticCode::WorkUnitBudgetExceeded);
    let context = event.fault_context.unwrap();
    assert_eq!(context.operation_id, MAX_WORK_UNITS_PER_SCAN + 1);
    assert_eq!(context.work_units_before_operation, MAX_WORK_UNITS_PER_SCAN);
}

#[test]
fn snapshot_restore_is_content_addressed_atomic_and_monotonic() {
    let package = deterministic_package();
    let mut controller = boot(&package, 6);
    set_input(&mut controller, 1, true);
    completed(&mut controller);
    controller.request_stop().unwrap();
    let captured_memory = controller.actual_memory(RETAIN);
    let snapshot = controller.capture_snapshot().unwrap();
    assert_ne!(snapshot.content_hash(), Hash32::ZERO);

    controller.request_run(RestartKind::Resume).unwrap();
    completed(&mut controller);
    controller.request_stop().unwrap();
    assert_ne!(controller.actual_memory(RETAIN), captured_memory);
    let prior_universe_epoch = controller.universe_epoch();
    let prior_controller_epoch = controller.controller_epoch();
    let approval = controller.prepare_restore(&snapshot).unwrap();
    let restored_hash = controller.restore_snapshot(&snapshot, approval).unwrap();

    assert_eq!(controller.cpu_state(), CpuState::Stop);
    assert_eq!(controller.universe_epoch(), prior_universe_epoch + 1);
    assert_eq!(controller.controller_epoch(), prior_controller_epoch + 1);
    assert_eq!(controller.scan_sequence(), 0);
    assert_eq!(controller.event_sequence(), 1);
    assert_eq!(controller.actual_memory(RETAIN), captured_memory);
    assert_eq!(controller.last_state_hash(), restored_hash);

    let stale_approval = controller.prepare_restore(&snapshot).unwrap();
    controller
        .set_virtual_input_raw(InputCommand {
            command_id: CommandId(88),
            idempotency_key: 88,
            controller_id: CONTROLLER,
            expected_controller_epoch: controller.controller_epoch(),
            channel_id: INPUT,
            value: CanonicalValue::Bool(false),
            audit_provenance_hash: Hash32::ZERO,
        })
        .unwrap();
    assert_eq!(
        controller.restore_snapshot(&snapshot, stale_approval),
        Err(SnapshotError::ApprovalMismatch)
    );
}

fn run_boolean_vector(
    bits: u8,
    busy_work: usize,
) -> (Hash32, Hash32, Vec<DeliveredOutput>, Vec<DiagnosticEvent>) {
    let package = deterministic_package();
    let mut controller = boot(&package, 0x1234_5678);
    for index in 0..4_u8 {
        let value = bits & (1 << index) != 0;
        set_input(&mut controller, u128::from(index) + 1, value);
        for spin in 0..busy_work {
            core::hint::black_box(spin.wrapping_mul(17));
        }
        completed(&mut controller);
    }
    let outputs = controller.boundary().delivered_outputs().cloned().collect();
    (
        controller.semantic_state_hash(),
        controller.replay_hash(),
        outputs,
        controller.diagnostics().to_vec(),
    )
}

#[test]
fn host_speed_independence_holds_for_all_four_scan_boolean_vectors() {
    for bits in 0_u8..16 {
        let fast = run_boolean_vector(bits, 0);
        let paced = run_boolean_vector(bits, 20_000);
        assert_eq!(
            fast, paced,
            "semantic divergence for input vector {bits:04b}"
        );
    }
}

#[test]
fn journey_e_runtime_determinism_repeats_snapshot_seed_and_events() {
    fn scenario() -> (Hash32, Hash32, Vec<Hash32>, Vec<DiagnosticEvent>) {
        let package = deterministic_package();
        let mut controller = boot(&package, 0x0ddc_0ffe);
        set_input(&mut controller, 1, true);
        completed(&mut controller);
        controller.request_stop().unwrap();
        let snapshot = controller.capture_snapshot().unwrap();
        let approval = controller.prepare_restore(&snapshot).unwrap();
        controller.restore_snapshot(&snapshot, approval).unwrap();
        controller.request_run(RestartKind::Resume).unwrap();
        for (id, value) in [(2, true), (3, false), (4, true), (5, true), (6, false)] {
            set_input(&mut controller, id, value);
            completed(&mut controller);
        }
        (
            snapshot.content_hash(),
            controller.replay_hash(),
            controller
                .boundary_hashes()
                .iter()
                .map(|entry| entry.state_hash)
                .collect(),
            controller.diagnostics().to_vec(),
        )
    }

    let first = scenario();
    let second = scenario();
    assert_eq!(first, second);
    assert_eq!(
        first.0.to_hex(),
        "3a24ae288bfa1dc50b41ceba4d752432af01c1dff08735fb6ef9e07da4282a96"
    );
    assert_eq!(
        first.1.to_hex(),
        "4d119ec16db2890ab22f99f57bf121724388e7235bc825b3a8e014626395678d"
    );
    assert_eq!(
        first.2.last().unwrap().to_hex(),
        "c34bed296c2fa4558e80861a5d4e6c3283e7dd8de3ae929376dd35b064085ab2"
    );
}
