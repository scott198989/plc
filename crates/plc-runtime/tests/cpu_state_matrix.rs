use plc_runtime::{
    ArtifactPackage, ArtifactSpec, BlockId, CanonicalValue, CommandError, CpuState, DiagnosticCode,
    Hash32, Instruction, MemoryDefinition, MemoryId, Operand, Operation, ProgramBlock,
    ProgramImage, RestartKind, RunOutcome, Sha256, UniverseId, ValueType, VirtualController,
    VirtualControllerId,
};

const CONTROLLER: VirtualControllerId = VirtualControllerId(0xc001);
const MEMORY: MemoryId = MemoryId(1);

fn hash(label: &str) -> Hash32 {
    Sha256::digest(label.as_bytes())
}

fn package(startup: Option<ProgramBlock>, cyclic: Operation) -> ArtifactPackage {
    ArtifactPackage::seal_verified(ArtifactSpec::edu21(
        hash("cpu-state-matrix-profile"),
        vec![MemoryDefinition {
            id: MEMORY,
            value_type: ValueType::I32,
            loaded_start: CanonicalValue::I32(7),
            retentive: true,
        }],
        vec![],
        vec![],
        ProgramImage {
            startup,
            timed: vec![],
            cyclic: ProgramBlock {
                id: BlockId(2),
                instructions: vec![Instruction::new(2, 0xc002, cyclic)],
            },
        },
    ))
    .unwrap()
}

fn ordinary_package() -> ArtifactPackage {
    package(
        None,
        Operation::AddI32 {
            left: Operand::Memory(MEMORY),
            right: Operand::Constant(CanonicalValue::I32(1)),
            target: MEMORY,
        },
    )
}

fn faulting_cyclic_package() -> ArtifactPackage {
    package(
        None,
        Operation::DivideI32 {
            numerator: Operand::Constant(CanonicalValue::I32(1)),
            denominator: Operand::Constant(CanonicalValue::I32(0)),
            target: MEMORY,
        },
    )
}

fn startup_fault_package() -> ArtifactPackage {
    package(
        Some(ProgramBlock {
            id: BlockId(1),
            instructions: vec![Instruction::new(
                1,
                0xc001,
                Operation::DivideI32 {
                    numerator: Operand::Constant(CanonicalValue::I32(1)),
                    denominator: Operand::Constant(CanonicalValue::I32(0)),
                    target: MEMORY,
                },
            )],
        }),
        Operation::SetMemory {
            target: MEMORY,
            value: CanonicalValue::I32(99),
        },
    )
}

fn controller_in(state: CpuState) -> VirtualController {
    let package = if state == CpuState::Faulted {
        faulting_cyclic_package()
    } else {
        ordinary_package()
    };
    let mut controller = VirtualController::new(UniverseId(0xc000), CONTROLLER, 0x55aa);
    controller.power_on().unwrap();
    controller.install_verified_artifact(&package).unwrap();
    match state {
        CpuState::PoweredOff => controller.power_off().unwrap(),
        CpuState::Stop => {}
        CpuState::Run => controller.request_run(RestartKind::Resume).unwrap(),
        CpuState::PausedEducational => {
            controller.request_run(RestartKind::Resume).unwrap();
            controller.pause_educational().unwrap();
        }
        CpuState::Faulted => {
            controller.request_run(RestartKind::Resume).unwrap();
            assert!(matches!(
                controller.run_scan().unwrap(),
                RunOutcome::Faulted(_)
            ));
        }
        CpuState::Startup | CpuState::Resetting => {
            panic!("transient CPU states cannot be fixture entry states")
        }
    }
    assert_eq!(controller.cpu_state(), state);
    controller
}

#[derive(Clone, Copy, Debug)]
enum Action {
    PowerOn,
    PowerOff,
    RequestRun,
    RequestStop,
    Pause,
    Resume,
    ResetFault,
    PowerCycle,
    MemoryReset,
    RunScan,
}

fn apply(controller: &mut VirtualController, action: Action) -> Result<(), CommandError> {
    match action {
        Action::PowerOn => controller.power_on(),
        Action::PowerOff => controller.power_off(),
        Action::RequestRun => controller.request_run(RestartKind::Resume),
        Action::RequestStop => controller.request_stop(),
        Action::Pause => controller.pause_educational(),
        Action::Resume => controller.resume_educational(),
        Action::ResetFault => controller.reset_fault(),
        Action::PowerCycle => controller.simulated_power_cycle(),
        Action::MemoryReset => controller.memory_reset(),
        Action::RunScan => controller.run_scan().map(|_| ()),
    }
}

fn expected_state(from: CpuState, action: Action) -> Option<CpuState> {
    match (from, action) {
        (CpuState::PoweredOff, Action::PowerOn | Action::PowerCycle) => Some(CpuState::Stop),
        (CpuState::PoweredOff, Action::PowerOff) => Some(CpuState::PoweredOff),
        (CpuState::Stop, Action::PowerCycle | Action::MemoryReset) => Some(CpuState::Stop),
        (CpuState::Stop, Action::PowerOff) => Some(CpuState::PoweredOff),
        (CpuState::Stop, Action::RequestRun) => Some(CpuState::Run),
        (CpuState::Run, Action::PowerOff) => Some(CpuState::PoweredOff),
        (CpuState::Run, Action::RequestStop) => Some(CpuState::Stop),
        (CpuState::Run, Action::Pause) => Some(CpuState::PausedEducational),
        (CpuState::Run, Action::RunScan) => Some(CpuState::Run),
        (CpuState::PausedEducational, Action::PowerOff) => Some(CpuState::PoweredOff),
        (CpuState::PausedEducational, Action::RequestStop | Action::MemoryReset) => {
            Some(CpuState::Stop)
        }
        (CpuState::PausedEducational, Action::Resume) => Some(CpuState::Run),
        (CpuState::Faulted, Action::PowerOff) => Some(CpuState::PoweredOff),
        (CpuState::Faulted, Action::ResetFault | Action::PowerCycle | Action::MemoryReset) => {
            Some(CpuState::Stop)
        }
        _ => None,
    }
}

#[test]
fn every_public_steady_cpu_state_transition_is_classified_and_illegal_commands_are_bounded() {
    let states = [
        CpuState::PoweredOff,
        CpuState::Stop,
        CpuState::Run,
        CpuState::PausedEducational,
        CpuState::Faulted,
    ];
    let actions = [
        Action::PowerOn,
        Action::PowerOff,
        Action::RequestRun,
        Action::RequestStop,
        Action::Pause,
        Action::Resume,
        Action::ResetFault,
        Action::PowerCycle,
        Action::MemoryReset,
        Action::RunScan,
    ];

    for from in states {
        for action in actions {
            let mut controller = controller_in(from);
            let prior_epoch = controller.controller_epoch();
            let prior_time = controller.virtual_time_ms();
            let prior_memory = controller.actual_memory(MEMORY);
            let prior_diagnostics = controller.diagnostics().len();
            let result = apply(&mut controller, action);
            if let Some(to) = expected_state(from, action) {
                result.unwrap_or_else(|error| {
                    panic!("expected {from:?} + {action:?} to be legal: {error:?}")
                });
                assert_eq!(controller.cpu_state(), to, "{from:?} + {action:?}");
            } else {
                let error = result.unwrap_err();
                assert!(
                    matches!(
                        error,
                        CommandError::IllegalCpuTransition { from: actual, .. } if actual == from
                    ),
                    "expected {from:?} + {action:?} to reject as an illegal transition, got {error:?}"
                );
                assert_eq!(controller.cpu_state(), from);
                assert_eq!(controller.controller_epoch(), prior_epoch);
                assert_eq!(controller.virtual_time_ms(), prior_time);
                assert_eq!(controller.actual_memory(MEMORY), prior_memory);
                assert_eq!(controller.diagnostics().len(), prior_diagnostics + 1);
                assert_eq!(
                    controller.diagnostics().last().unwrap().code,
                    DiagnosticCode::IllegalCpuTransition
                );
            }
        }
    }
}

#[test]
fn startup_fault_stops_before_run_and_requires_an_explicit_fault_reset() {
    let package = startup_fault_package();
    let mut controller = VirtualController::new(UniverseId(0xc000), CONTROLLER, 0x55aa);
    assert_eq!(controller.cpu_state(), CpuState::PoweredOff);
    assert_eq!(controller.controller_epoch(), 1);
    assert_eq!(controller.scan_sequence(), 0);
    controller.power_on().unwrap();
    controller.install_verified_artifact(&package).unwrap();

    controller.request_run(RestartKind::Resume).unwrap();
    assert_eq!(controller.cpu_state(), CpuState::Faulted);
    assert_eq!(controller.scan_sequence(), 0);
    assert_eq!(
        controller.actual_memory(MEMORY),
        Some(CanonicalValue::I32(7))
    );
    let fault = controller.diagnostics().last().unwrap();
    assert_eq!(fault.code, DiagnosticCode::ArithmeticDivideByZero);
    assert_eq!(fault.fault_context.as_ref().unwrap().operation_id, 1);

    assert!(matches!(
        controller.request_run(RestartKind::Resume),
        Err(CommandError::IllegalCpuTransition {
            from: CpuState::Faulted,
            ..
        })
    ));
    controller.reset_fault().unwrap();
    assert_eq!(controller.cpu_state(), CpuState::Stop);
}
