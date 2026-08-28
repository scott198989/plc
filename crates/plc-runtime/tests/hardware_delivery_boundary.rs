use plc_runtime::*;

const UNIVERSE: UniverseId = UniverseId(0x7110);
const CONTROLLER: VirtualControllerId = VirtualControllerId(0x7220);
const INPUT: ChannelId = ChannelId(1);
const BOOL_OUTPUT: ChannelId = ChannelId(2);
const INTEGER_OUTPUT: ChannelId = ChannelId(3);
const BOOL_SOURCE: MemoryId = MemoryId(1);
const INTEGER_SOURCE: MemoryId = MemoryId(2);
const DIVISOR: MemoryId = MemoryId(3);
const DIVISION_RESULT: MemoryId = MemoryId(4);

fn package() -> ArtifactPackage {
    ArtifactPackage::seal_verified(ArtifactSpec::edu21(
        Sha256::digest(b"phase-2-hardware-delivery-boundary-profile"),
        vec![
            MemoryDefinition {
                id: BOOL_SOURCE,
                value_type: ValueType::Bool,
                loaded_start: CanonicalValue::Bool(false),
                retentive: false,
            },
            MemoryDefinition {
                id: INTEGER_SOURCE,
                value_type: ValueType::I32,
                loaded_start: CanonicalValue::I32(0),
                retentive: false,
            },
            MemoryDefinition {
                id: DIVISOR,
                value_type: ValueType::I32,
                loaded_start: CanonicalValue::I32(1),
                retentive: false,
            },
            MemoryDefinition {
                id: DIVISION_RESULT,
                value_type: ValueType::I32,
                loaded_start: CanonicalValue::I32(0),
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
                id: BOOL_OUTPUT,
                direction: ChannelDirection::Output,
                value_type: ValueType::Bool,
                canonical_default: CanonicalValue::Bool(false),
            },
            ChannelDefinition {
                id: INTEGER_OUTPUT,
                direction: ChannelDirection::Output,
                value_type: ValueType::I32,
                canonical_default: CanonicalValue::I32(0),
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
                        0x7111,
                        Operation::SetMemory {
                            target: BOOL_SOURCE,
                            value: CanonicalValue::Bool(true),
                        },
                    ),
                    Instruction::new(
                        2,
                        0x7112,
                        Operation::StoreOutput {
                            source: Operand::Memory(BOOL_SOURCE),
                            channel: BOOL_OUTPUT,
                        },
                    ),
                    Instruction::new(
                        3,
                        0x7113,
                        Operation::SetMemory {
                            target: INTEGER_SOURCE,
                            value: CanonicalValue::I32(42),
                        },
                    ),
                    Instruction::new(
                        4,
                        0x7114,
                        Operation::StoreOutput {
                            source: Operand::Memory(INTEGER_SOURCE),
                            channel: INTEGER_OUTPUT,
                        },
                    ),
                    Instruction::new(
                        5,
                        0x7115,
                        Operation::DivideI32 {
                            numerator: Operand::Constant(CanonicalValue::I32(1)),
                            denominator: Operand::Memory(DIVISOR),
                            target: DIVISION_RESULT,
                        },
                    ),
                ],
            },
        },
    ))
    .unwrap()
}

fn stopped() -> VirtualController {
    let mut controller = VirtualController::new(UNIVERSE, CONTROLLER, 0x7330);
    controller.power_on().unwrap();
    controller.install_verified_artifact(&package()).unwrap();
    controller
}

fn running_after_commit() -> VirtualController {
    let mut controller = stopped();
    controller.request_run(RestartKind::Resume).unwrap();
    assert!(matches!(
        controller.run_scan().unwrap(),
        RunOutcome::Completed(_)
    ));
    controller
}

fn faulted_after_natural_output_writes() -> VirtualController {
    let mut controller = stopped();
    let observation = RuntimeBoundaryCommand {
        command_id: 0x7440,
        controller_id: CONTROLLER,
        expected_controller_epoch: controller.controller_epoch(),
        expected_artifact_fingerprint: controller.loaded_fingerprint().unwrap(),
        expected_state_hash: controller.semantic_state_hash(),
        natural_writes: vec![RuntimeNaturalWrite {
            target: RuntimeValueTarget::Memory(DIVISOR),
            value: CanonicalValue::I32(0),
        }],
        force_deltas: vec![],
        audit_context_hash: Sha256::digest(b"arm deterministic divide-by-zero fault"),
    };
    controller.apply_observation_boundary(&observation).unwrap();
    controller.request_run(RestartKind::Resume).unwrap();
    assert!(matches!(
        controller.run_scan().unwrap(),
        RunOutcome::Faulted(_)
    ));
    controller
}

fn command(
    controller: &VirtualController,
    command_id: u128,
    output_overrides: Vec<RuntimeOutputDeliveryOverride>,
) -> RuntimeHardwareBoundaryCommand {
    RuntimeHardwareBoundaryCommand {
        command_id,
        controller_id: CONTROLLER,
        expected_universe_epoch: controller.universe_epoch(),
        expected_controller_epoch: controller.controller_epoch(),
        expected_artifact_fingerprint: controller.loaded_fingerprint().unwrap(),
        expected_state_hash: controller.semantic_state_hash(),
        output_overrides,
        audit_context_hash: Sha256::digest(b"virtual hardware causal projection"),
    }
}

fn bool_override(value: bool, quality: Quality, suppressed: bool) -> RuntimeOutputDeliveryOverride {
    RuntimeOutputDeliveryOverride {
        channel_id: BOOL_OUTPUT,
        delivered_value: CanonicalValue::Bool(value),
        quality,
        suppressed,
    }
}

#[test]
fn run_projection_preserves_cpu_images_and_empty_batch_restores_ordinary_commit() {
    let mut controller = running_after_commit();
    let mut deterministic_peer = controller.clone();
    let natural = controller.natural_output(BOOL_OUTPUT);
    let effective = controller.effective_output(BOOL_OUTPUT);
    let scan_sequence = controller.scan_sequence();
    let initial_replay_hash = controller.replay_hash();
    let suppress = command(
        &controller,
        0x7501,
        vec![bool_override(false, Quality::Bad, true)],
    );

    let receipt = controller.apply_hardware_boundary(&suppress).unwrap();
    let peer_receipt = deterministic_peer
        .apply_hardware_boundary(&suppress)
        .unwrap();
    assert_eq!(receipt, peer_receipt);
    assert_eq!(controller.replay_hash(), deterministic_peer.replay_hash());
    assert_ne!(controller.replay_hash(), initial_replay_hash);
    assert_eq!(receipt.state_hash, controller.semantic_state_hash());
    assert_eq!(receipt.replay_hash, controller.replay_hash());
    assert_eq!(controller.natural_output(BOOL_OUTPUT), natural);
    assert_eq!(controller.effective_output(BOOL_OUTPUT), effective);

    let delivered = controller.boundary().delivered_output(BOOL_OUTPUT).unwrap();
    assert_eq!(delivered.canonical_value, CanonicalValue::Bool(false));
    assert_eq!(delivered.quality, Quality::Bad);
    assert!(delivered.suppressed);
    assert_eq!(
        delivered.delivery_reason,
        DeliveryReason::HardwareSuppressed
    );
    assert_eq!(delivered.output_commit_scan_sequence, Some(scan_sequence));
    assert_eq!(
        controller.replay_events().last().unwrap().kind,
        ReplayEventKind::HardwareBoundary
    );
    assert_eq!(
        controller.replay_events().last().unwrap().result_hash,
        receipt.state_hash
    );

    let restore = command(&controller, 0x7502, vec![]);
    controller.apply_hardware_boundary(&restore).unwrap();
    let restored = controller.boundary().delivered_output(BOOL_OUTPUT).unwrap();
    assert_eq!(restored.canonical_value, CanonicalValue::Bool(true));
    assert_eq!(restored.quality, Quality::Good);
    assert!(!restored.suppressed);
    assert_eq!(restored.delivery_reason, DeliveryReason::RunOutputCommit);
    assert_eq!(restored.output_commit_scan_sequence, Some(scan_sequence));
    assert_eq!(controller.natural_output(BOOL_OUTPUT), natural);
    assert_eq!(controller.effective_output(BOOL_OUTPUT), effective);
}

#[test]
fn stop_pause_and_fault_rebase_before_hardware_projection_and_restore() {
    let mut stopped = stopped();
    let stop_projection = command(
        &stopped,
        0x7601,
        vec![bool_override(true, Quality::Uncertain, false)],
    );
    stopped.apply_hardware_boundary(&stop_projection).unwrap();
    let delivered = stopped.boundary().delivered_output(BOOL_OUTPUT).unwrap();
    assert_eq!(delivered.canonical_value, CanonicalValue::Bool(true));
    assert_eq!(delivered.quality, Quality::Uncertain);
    assert!(!delivered.suppressed);
    assert_eq!(
        delivered.delivery_reason,
        DeliveryReason::HardwareSuppressed
    );
    assert_eq!(delivered.output_commit_scan_sequence, None);
    let stop_restore = command(&stopped, 0x7602, vec![]);
    stopped.apply_hardware_boundary(&stop_restore).unwrap();
    assert_default_delivery(&stopped, DeliveryReason::CpuModeDefault);

    let mut paused = running_after_commit();
    paused.pause_educational().unwrap();
    let paused_natural = paused.natural_output(BOOL_OUTPUT);
    let pause_projection = command(
        &paused,
        0x7603,
        vec![bool_override(false, Quality::Bad, true)],
    );
    paused.apply_hardware_boundary(&pause_projection).unwrap();
    let pause_restore = command(&paused, 0x7604, vec![]);
    paused.apply_hardware_boundary(&pause_restore).unwrap();
    assert_default_delivery(&paused, DeliveryReason::CpuModeDefault);
    assert_eq!(paused.natural_output(BOOL_OUTPUT), paused_natural);

    let mut faulted = faulted_after_natural_output_writes();
    assert_eq!(faulted.cpu_state(), CpuState::Faulted);
    assert_eq!(
        faulted.natural_output(BOOL_OUTPUT),
        Some(CanonicalValue::Bool(true))
    );
    let fault_projection = command(
        &faulted,
        0x7605,
        vec![bool_override(true, Quality::NotPresent, true)],
    );
    faulted.apply_hardware_boundary(&fault_projection).unwrap();
    let fault_restore = command(&faulted, 0x7606, vec![]);
    faulted.apply_hardware_boundary(&fault_restore).unwrap();
    assert_default_delivery(&faulted, DeliveryReason::FatalFaultDefault);
    assert_eq!(
        faulted.natural_output(BOOL_OUTPUT),
        Some(CanonicalValue::Bool(true))
    );
}

fn assert_default_delivery(controller: &VirtualController, reason: DeliveryReason) {
    let delivered = controller.boundary().delivered_output(BOOL_OUTPUT).unwrap();
    assert_eq!(delivered.canonical_value, CanonicalValue::Bool(false));
    assert_eq!(delivered.quality, Quality::Good);
    assert!(!delivered.suppressed);
    assert_eq!(delivered.delivery_reason, reason);
    assert_eq!(delivered.output_commit_scan_sequence, None);
}

#[test]
fn stale_or_invalid_hardware_batches_are_rejected_atomically() {
    let mut controller = running_after_commit();
    let state_hash = controller.semantic_state_hash();
    let replay_hash = controller.replay_hash();
    let delivered = controller
        .boundary()
        .delivered_output(BOOL_OUTPUT)
        .unwrap()
        .clone();
    let valid = command(
        &controller,
        0x7701,
        vec![bool_override(false, Quality::Bad, true)],
    );

    let mut wrong_controller = valid.clone();
    wrong_controller.controller_id = VirtualControllerId(0xdead);
    let mut stale_universe = valid.clone();
    stale_universe.expected_universe_epoch += 1;
    let mut stale_controller = valid.clone();
    stale_controller.expected_controller_epoch += 1;
    let mut stale_artifact = valid.clone();
    stale_artifact.expected_artifact_fingerprint = Sha256::digest(b"stale artifact");
    let mut stale_state = valid.clone();
    stale_state.expected_state_hash = Sha256::digest(b"stale state");
    let mut unknown = valid.clone();
    unknown.output_overrides[0].channel_id = ChannelId(0xffff);
    let mut input_as_output = valid.clone();
    input_as_output.output_overrides[0].channel_id = INPUT;
    let mut type_mismatch = valid.clone();
    type_mismatch.output_overrides[0].delivered_value = CanonicalValue::I32(1);
    let mut duplicate = valid.clone();
    duplicate
        .output_overrides
        .push(bool_override(true, Quality::NotPresent, true));

    let cases = [
        (wrong_controller, RuntimeBoundaryError::WrongController),
        (stale_universe, RuntimeBoundaryError::StaleUniverseEpoch),
        (stale_controller, RuntimeBoundaryError::StaleControllerEpoch),
        (stale_artifact, RuntimeBoundaryError::StaleArtifact),
        (stale_state, RuntimeBoundaryError::StaleState),
        (
            unknown,
            RuntimeBoundaryError::UnknownTarget(RuntimeValueTarget::Output(ChannelId(0xffff))),
        ),
        (
            input_as_output,
            RuntimeBoundaryError::UnknownTarget(RuntimeValueTarget::Output(INPUT)),
        ),
        (
            type_mismatch,
            RuntimeBoundaryError::TypeMismatch {
                target: RuntimeValueTarget::Output(BOOL_OUTPUT),
                expected: ValueType::Bool,
                actual: ValueType::I32,
            },
        ),
        (
            duplicate,
            RuntimeBoundaryError::DuplicateTarget(RuntimeValueTarget::Output(BOOL_OUTPUT)),
        ),
    ];

    for (command, expected) in cases {
        assert_eq!(controller.apply_hardware_boundary(&command), Err(expected));
        assert_eq!(controller.semantic_state_hash(), state_hash);
        assert_eq!(controller.replay_hash(), replay_hash);
        assert_eq!(
            controller.boundary().delivered_output(BOOL_OUTPUT).unwrap(),
            &delivered
        );
    }
}

#[test]
fn transient_or_powered_off_cpu_state_is_rejected_without_mutation() {
    let mut controller = stopped();
    controller.power_off().unwrap();
    let before = controller.semantic_state_hash();
    let command = command(&controller, 0x7801, vec![]);
    assert_eq!(
        controller.apply_hardware_boundary(&command),
        Err(RuntimeBoundaryError::CpuStateDisallowed(
            CpuState::PoweredOff
        ))
    );
    assert_eq!(controller.semantic_state_hash(), before);
}
