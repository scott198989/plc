use plc_runtime::{
    ArtifactPackage, ArtifactSpec, BlockId, CanonicalValue, CpuState, DiagnosticCode, Instruction,
    MemoryDefinition, MemoryId, Operand, Operation, ProgramBlock, ProgramImage, RestartKind,
    RunOutcome, Sha256, StateDefinition, StateId, StateStart, UniverseId, ValueType,
    VirtualController, VirtualControllerId,
};

const CONTROLLER: VirtualControllerId = VirtualControllerId(0xf001);
const TIMER_STATE: StateId = StateId(1);
const TIMER_Q: MemoryId = MemoryId(1);
const TIMER_ET: MemoryId = MemoryId(2);
const BEFORE: MemoryId = MemoryId(3);
const AFTER: MemoryId = MemoryId(4);

fn memory(id: MemoryId, value_type: ValueType, loaded_start: CanonicalValue) -> MemoryDefinition {
    MemoryDefinition {
        id,
        value_type,
        loaded_start,
        retentive: false,
    }
}

fn timer_overflow_package() -> ArtifactPackage {
    ArtifactPackage::seal_verified(ArtifactSpec::edu21(
        Sha256::digest(b"fault-policy-profile"),
        vec![
            memory(TIMER_Q, ValueType::Bool, CanonicalValue::Bool(false)),
            memory(TIMER_ET, ValueType::TimeMs, CanonicalValue::TimeMs(0)),
            memory(BEFORE, ValueType::I32, CanonicalValue::I32(0)),
            memory(AFTER, ValueType::I32, CanonicalValue::I32(0)),
        ],
        vec![],
        vec![StateDefinition {
            id: TIMER_STATE,
            loaded_start: StateStart::Timer {
                elapsed_ms: u64::MAX,
                output: false,
            },
            retentive: false,
        }],
        ProgramImage {
            startup: None,
            timed: vec![],
            cyclic: ProgramBlock {
                id: BlockId(1),
                instructions: vec![
                    Instruction::new(
                        1,
                        0xf101,
                        Operation::SetMemory {
                            target: BEFORE,
                            value: CanonicalValue::I32(11),
                        },
                    ),
                    Instruction::new(
                        2,
                        0xf102,
                        Operation::TimerOnDelay {
                            input: Operand::Constant(CanonicalValue::Bool(true)),
                            preset_ms: 10,
                            state: TIMER_STATE,
                            output: TIMER_Q,
                            elapsed: TIMER_ET,
                        },
                    ),
                    Instruction::new(
                        3,
                        0xf103,
                        Operation::SetMemory {
                            target: AFTER,
                            value: CanonicalValue::I32(22),
                        },
                    ),
                ],
            },
        },
    ))
    .unwrap()
}

#[test]
fn timer_overflow_is_fatal_at_the_operation_boundary_without_later_writes() {
    let package = timer_overflow_package();
    let mut controller = VirtualController::new(UniverseId(0xf000), CONTROLLER, 0x55aa);
    controller.power_on().unwrap();
    controller.install_verified_artifact(&package).unwrap();
    controller.request_run(RestartKind::Resume).unwrap();

    let RunOutcome::Faulted(fault) = controller.run_scan().unwrap() else {
        panic!("TIME overflow must fault the authoritative runtime");
    };
    assert_eq!(controller.cpu_state(), CpuState::Faulted);
    assert_eq!(fault.code, DiagnosticCode::TimerOverflow);
    assert_eq!(fault.root_occurrence_id, fault.occurrence_id);
    assert_eq!(fault.fault_context.as_ref().unwrap().operation_id, 2);
    assert_eq!(
        fault.fault_context.as_ref().unwrap().source_identity,
        0xf102
    );
    assert_eq!(fault.fault_context.as_ref().unwrap().scan_sequence, 1);
    assert_eq!(fault.virtual_timestamp_ms, 0);
    assert_eq!(
        controller.actual_memory(BEFORE),
        Some(CanonicalValue::I32(11))
    );
    assert_eq!(
        controller.actual_memory(AFTER),
        Some(CanonicalValue::I32(0))
    );
    assert_eq!(
        controller.actual_memory(TIMER_Q),
        Some(CanonicalValue::Bool(false))
    );
    assert_eq!(
        controller.actual_memory(TIMER_ET),
        Some(CanonicalValue::TimeMs(0))
    );
    assert_eq!(
        fault.fault_boundary_state_hash,
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
