use plc_runtime::*;

const UNIVERSE: UniverseId = UniverseId(0x5010);
const CONTROLLER: VirtualControllerId = VirtualControllerId(0x5020);
const INTEGER: MemoryId = MemoryId(1);
const INPUT_IMAGE: MemoryId = MemoryId(2);
const INPUT: ChannelId = ChannelId(1);
const OUTPUT: ChannelId = ChannelId(2);

fn package() -> ArtifactPackage {
    ArtifactPackage::seal_verified(ArtifactSpec::edu21(
        Sha256::digest(b"P2-10-observation-runtime-profile"),
        vec![
            MemoryDefinition {
                id: INTEGER,
                value_type: ValueType::I32,
                loaded_start: CanonicalValue::I32(0),
                retentive: true,
            },
            MemoryDefinition {
                id: INPUT_IMAGE,
                value_type: ValueType::Bool,
                loaded_start: CanonicalValue::Bool(false),
                retentive: false,
            },
        ],
        vec![
            ChannelDefinition {
                id: INPUT,
                direction: ChannelDirection::Input,
                value_type: ValueType::Bool,
                canonical_default: CanonicalValue::Bool(false),
            },
            ChannelDefinition {
                id: OUTPUT,
                direction: ChannelDirection::Output,
                value_type: ValueType::Bool,
                canonical_default: CanonicalValue::Bool(false),
            },
        ],
        vec![],
        ProgramImage {
            startup: None,
            timed: vec![],
            cyclic: ProgramBlock {
                id: BlockId(1),
                instructions: vec![
                    Instruction::new(
                        1,
                        0x5101,
                        Operation::AddI32 {
                            left: Operand::Memory(INTEGER),
                            right: Operand::Constant(CanonicalValue::I32(1)),
                            target: INTEGER,
                        },
                    ),
                    Instruction::new(
                        2,
                        0x5102,
                        Operation::LoadInput {
                            channel: INPUT,
                            target: INPUT_IMAGE,
                        },
                    ),
                    Instruction::new(
                        3,
                        0x5103,
                        Operation::StoreOutput {
                            source: Operand::Memory(INPUT_IMAGE),
                            channel: OUTPUT,
                        },
                    ),
                ],
            },
        },
    ))
    .unwrap()
}

fn stopped() -> VirtualController {
    let package = package();
    let mut controller = VirtualController::new(UNIVERSE, CONTROLLER, 0x55aa);
    controller.power_on().unwrap();
    controller.install_verified_artifact(&package).unwrap();
    controller
}

fn running() -> VirtualController {
    let mut controller = stopped();
    controller.request_run(RestartKind::Resume).unwrap();
    controller
}

fn boundary_command(controller: &VirtualController) -> RuntimeBoundaryCommand {
    RuntimeBoundaryCommand {
        command_id: 0x6001,
        controller_id: CONTROLLER,
        expected_controller_epoch: controller.controller_epoch(),
        expected_artifact_fingerprint: controller.loaded_fingerprint().unwrap(),
        expected_state_hash: controller.semantic_state_hash(),
        natural_writes: vec![],
        force_deltas: vec![],
        audit_context_hash: Sha256::digest(b"runtime-observation-test"),
    }
}

fn scan_command(controller: &VirtualController) -> RuntimeScanCommand {
    RuntimeScanCommand {
        command_id: 0x6002,
        controller_id: CONTROLLER,
        expected_controller_epoch: controller.controller_epoch(),
        expected_artifact_fingerprint: controller.loaded_fingerprint().unwrap(),
        expected_state_hash: controller.semantic_state_hash(),
        pre_program_writes: vec![],
        post_program_writes: vec![],
        force_deltas: vec![],
        audit_context_hash: Sha256::digest(b"runtime-observation-scan-test"),
    }
}

#[test]
fn invalid_aggregate_is_rejected_without_partial_mutation() {
    let mut controller = stopped();
    let before = controller.semantic_state_hash();
    let mut command = boundary_command(&controller);
    command.natural_writes = vec![
        RuntimeNaturalWrite {
            target: RuntimeValueTarget::Memory(INTEGER),
            value: CanonicalValue::I32(7),
        },
        RuntimeNaturalWrite {
            target: RuntimeValueTarget::Input(INPUT),
            value: CanonicalValue::I32(8),
        },
    ];

    assert!(matches!(
        controller.apply_observation_boundary(&command),
        Err(RuntimeBoundaryError::TypeMismatch {
            target: RuntimeValueTarget::Input(INPUT),
            expected: ValueType::Bool,
            actual: ValueType::I32,
        })
    ));
    assert_eq!(controller.semantic_state_hash(), before);
    assert_eq!(
        controller.actual_memory(INTEGER),
        Some(CanonicalValue::I32(0))
    );
}

#[test]
fn memory_force_masks_reads_while_natural_program_writes_continue() {
    let mut controller = running();
    let mut command = scan_command(&controller);
    command.force_deltas.push(RuntimeForceDelta {
        target: RuntimeValueTarget::Memory(INTEGER),
        value: Some(CanonicalValue::I32(9)),
    });
    controller.run_scan_with_observation(&command).unwrap();
    assert_eq!(
        controller.actual_memory(INTEGER),
        Some(CanonicalValue::I32(10))
    );
    assert_eq!(
        controller.effective_value(RuntimeValueTarget::Memory(INTEGER)),
        Some(CanonicalValue::I32(9))
    );

    controller.run_scan().unwrap();
    assert_eq!(
        controller.actual_memory(INTEGER),
        Some(CanonicalValue::I32(10))
    );

    let mut remove = scan_command(&controller);
    remove.force_deltas.push(RuntimeForceDelta {
        target: RuntimeValueTarget::Memory(INTEGER),
        value: None,
    });
    controller.run_scan_with_observation(&remove).unwrap();
    assert_eq!(
        controller.actual_memory(INTEGER),
        Some(CanonicalValue::I32(11))
    );
    assert_eq!(
        controller.force_overlay(RuntimeValueTarget::Memory(INTEGER)),
        None
    );
}

#[test]
fn input_and_output_publication_boundaries_keep_natural_and_effective_layers_distinct() {
    let mut controller = running();
    let mut command = scan_command(&controller);
    command.pre_program_writes.push(RuntimeNaturalWrite {
        target: RuntimeValueTarget::Input(INPUT),
        value: CanonicalValue::Bool(true),
    });
    command.post_program_writes.push(RuntimeNaturalWrite {
        target: RuntimeValueTarget::Output(OUTPUT),
        value: CanonicalValue::Bool(true),
    });
    command.force_deltas = vec![
        RuntimeForceDelta {
            target: RuntimeValueTarget::Input(INPUT),
            value: Some(CanonicalValue::Bool(false)),
        },
        RuntimeForceDelta {
            target: RuntimeValueTarget::Output(OUTPUT),
            value: Some(CanonicalValue::Bool(false)),
        },
    ];

    let receipt = controller.run_scan_with_observation(&command).unwrap();
    assert_eq!(receipt.applied_pre_program_writes.len(), 1);
    assert_eq!(receipt.applied_post_program_writes.len(), 1);
    assert_eq!(
        controller.natural_input(INPUT),
        Some(CanonicalValue::Bool(true))
    );
    assert_eq!(
        controller.effective_input(INPUT),
        Some(CanonicalValue::Bool(false))
    );
    assert_eq!(
        controller.actual_memory(INPUT_IMAGE),
        Some(CanonicalValue::Bool(false))
    );
    assert_eq!(
        controller.natural_output(OUTPUT),
        Some(CanonicalValue::Bool(true))
    );
    assert_eq!(
        controller.effective_output(OUTPUT),
        Some(CanonicalValue::Bool(false))
    );
    assert_eq!(
        controller
            .boundary()
            .delivered_output(OUTPUT)
            .unwrap()
            .canonical_value,
        CanonicalValue::Bool(false)
    );
}

#[test]
fn clone_and_snapshot_state_include_forces_and_reset_clears_them() {
    let mut source = stopped();
    let mut command = boundary_command(&source);
    command.force_deltas.push(RuntimeForceDelta {
        target: RuntimeValueTarget::Memory(INTEGER),
        value: Some(CanonicalValue::I32(41)),
    });
    source.apply_observation_boundary(&command).unwrap();
    let snapshot = source.capture_snapshot().unwrap();
    let staged = source
        .stage_reidentified_clone(VirtualControllerId(0x5030))
        .unwrap();
    let (mut clone, _) = staged.commit(&source).unwrap();
    assert_eq!(
        clone.force_overlay(RuntimeValueTarget::Memory(INTEGER)),
        Some(CanonicalValue::I32(41))
    );
    assert_ne!(snapshot.content_hash(), Hash32::ZERO);

    clone.memory_reset().unwrap();
    assert!(clone.force_overlays().is_empty());
    assert_eq!(clone.actual_memory(INTEGER), Some(CanonicalValue::I32(0)));
}
