use plc_runtime::{
    ArtifactPackage, ArtifactSpec, BlockId, CanonicalValue, CpuState, Hash32, Instruction,
    MemoryDefinition, MemoryId, Operand, Operation, ProgramBlock, ProgramImage, ReplayEventKind,
    RestartKind, RuntimeLifecycleError, Sha256, UniverseId, ValueType, VirtualController,
    VirtualControllerId,
};

fn hash(label: &str) -> Hash32 {
    Sha256::digest(label.as_bytes())
}

fn package() -> ArtifactPackage {
    ArtifactPackage::seal_verified(ArtifactSpec::edu21(
        hash("lifecycle-profile"),
        vec![MemoryDefinition {
            id: MemoryId(1),
            value_type: ValueType::I32,
            loaded_start: CanonicalValue::I32(7),
            retentive: true,
        }],
        vec![],
        vec![],
        ProgramImage {
            startup: None,
            timed: vec![],
            cyclic: ProgramBlock {
                id: BlockId(1),
                instructions: vec![Instruction::new(
                    1,
                    0x101,
                    Operation::AddI32 {
                        left: Operand::Memory(MemoryId(1)),
                        right: Operand::Constant(CanonicalValue::I32(1)),
                        target: MemoryId(1),
                    },
                )],
            },
        },
    ))
    .unwrap()
}

fn running_controller(controller_id: VirtualControllerId) -> VirtualController {
    let package = package();
    let mut controller = VirtualController::new(UniverseId(10), controller_id, 0x55aa);
    controller.power_on().unwrap();
    controller.install_verified_artifact(&package).unwrap();
    controller.request_run(RestartKind::Resume).unwrap();
    controller.run_scan().unwrap();
    controller
}

#[test]
fn reidentified_clone_preserves_live_execution_state_and_guards_source_staleness() {
    let source_id = VirtualControllerId(20);
    let clone_id = VirtualControllerId(21);
    let mut source = running_controller(source_id);
    let source_hash = source.semantic_state_hash();
    let staged = source.stage_reidentified_clone(clone_id).unwrap();

    assert_eq!(source.semantic_state_hash(), source_hash);
    assert_eq!(staged.report().source_state_hash, source_hash);
    assert_eq!(staged.report().clone_controller_epoch, 1);

    source.request_stop().unwrap();
    assert_eq!(
        staged.commit(&source).unwrap_err(),
        RuntimeLifecycleError::StaleSource
    );

    source.request_run(RestartKind::Resume).unwrap();
    let stable_source_hash = source.semantic_state_hash();
    let source_epoch = source.controller_epoch();
    let source_value = source.actual_memory(MemoryId(1));
    let source_retain = source.retained_memory(MemoryId(1));
    let source_time = source.virtual_time_ms();
    let source_scan = source.scan_sequence();
    let source_fingerprint = source.loaded_fingerprint();
    let staged = source.stage_reidentified_clone(clone_id).unwrap();
    let (clone, report) = staged.commit(&source).unwrap();

    assert_eq!(source.semantic_state_hash(), stable_source_hash);
    assert_eq!(source.controller_epoch(), source_epoch);
    assert_eq!(clone.controller_id(), clone_id);
    assert_eq!(clone.controller_epoch(), 1);
    assert_eq!(clone.cpu_state(), CpuState::Run);
    assert_eq!(clone.actual_memory(MemoryId(1)), source_value);
    assert_eq!(clone.retained_memory(MemoryId(1)), source_retain);
    assert_eq!(clone.virtual_time_ms(), source_time);
    assert_eq!(clone.scan_sequence(), source_scan);
    assert_eq!(clone.loaded_fingerprint(), source_fingerprint);
    assert_eq!(clone.boundary().controller_id, clone_id);
    assert_eq!(report.clone_state_hash, clone.semantic_state_hash());
    assert_eq!(
        clone.replay_events().last().unwrap().kind,
        ReplayEventKind::InstanceCloned
    );
}

#[test]
fn blank_replacement_is_atomic_epoch_safe_and_exactly_rollbackable() {
    let controller_id = VirtualControllerId(30);
    let mut target = running_controller(controller_id);
    target.request_stop().unwrap();
    let pre_hash = target.semantic_state_hash();
    let pre_epoch = target.controller_epoch();
    let staged = target.stage_blank_replacement().unwrap();

    assert_eq!(target.semantic_state_hash(), pre_hash);
    assert_eq!(staged.report().new_controller_epoch, pre_epoch + 1);

    target.request_run(RestartKind::Resume).unwrap();
    assert_eq!(
        staged.commit(&mut target).unwrap_err(),
        RuntimeLifecycleError::StaleTarget
    );
    target.request_stop().unwrap();

    let backup = target.clone();
    let pre_commit_hash = backup.semantic_state_hash();
    let pre_commit_epoch = backup.controller_epoch();
    let staged = target.stage_blank_replacement().unwrap();
    let report = staged.commit(&mut target).unwrap();
    assert_eq!(target.controller_id(), controller_id);
    assert_eq!(target.controller_epoch(), pre_commit_epoch + 1);
    assert_eq!(target.cpu_state(), CpuState::PoweredOff);
    assert_eq!(target.loaded_fingerprint(), None);
    assert_eq!(target.actual_memory(MemoryId(1)), None);
    assert_eq!(report.new_state_hash, target.semantic_state_hash());
    assert_eq!(
        target.replay_events().last().unwrap().kind,
        ReplayEventKind::InstanceReplaced
    );

    target = backup;
    assert_eq!(target.semantic_state_hash(), pre_commit_hash);
    assert_eq!(target.controller_epoch(), pre_commit_epoch);

    assert_ne!(pre_hash, Hash32::ZERO);
}
