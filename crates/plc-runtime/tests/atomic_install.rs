use plc_runtime::*;

const MEMORY: MemoryId = MemoryId(1);

fn package(step: i32) -> ArtifactPackage {
    ArtifactPackage::seal_verified(ArtifactSpec::edu21(
        Sha256::digest(b"EDU-21-CORE-1.0"),
        vec![MemoryDefinition {
            id: MEMORY,
            value_type: ValueType::I32,
            loaded_start: CanonicalValue::I32(5),
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
                    1,
                    Operation::AddI32 {
                        left: Operand::Memory(MEMORY),
                        right: Operand::Constant(CanonicalValue::I32(step)),
                        target: MEMORY,
                    },
                )],
            },
        },
    ))
    .unwrap()
}

fn running(package: &ArtifactPackage) -> VirtualController {
    let mut controller = VirtualController::new(UniverseId(1), VirtualControllerId(2), 3);
    controller.power_on().unwrap();
    controller.install_verified_artifact(package).unwrap();
    controller.request_run(RestartKind::Resume).unwrap();
    controller
}

#[test]
fn staged_replacement_preserves_live_and_retained_values_without_exposing_stage() {
    let old = package(1);
    let new = package(2);
    let mut target = running(&old);
    assert!(matches!(
        target.run_scan().unwrap(),
        RunOutcome::Completed(_)
    ));
    assert_eq!(target.actual_memory(MEMORY), Some(CanonicalValue::I32(6)));

    let plan = RuntimeStateTransferPlan::new(
        old.fingerprint(),
        new.fingerprint(),
        RuntimeInstallDisposition::ArtifactReplacement,
        vec![MEMORY],
        vec![],
        true,
        true,
    )
    .unwrap();
    let before_hash = target.semantic_state_hash();
    let before_epoch = target.controller_epoch();
    let staged = target.stage_atomic_install(&new, &plan).unwrap();

    assert_eq!(target.semantic_state_hash(), before_hash);
    assert_eq!(target.loaded_fingerprint(), Some(old.fingerprint()));
    assert_eq!(target.controller_epoch(), before_epoch);
    assert_eq!(staged.report().preserved_memory, vec![MEMORY]);
    assert_eq!(staged.report().initialized_memory_count, 0);
    assert_eq!(staged.report().removed_memory_count, 0);

    let report = staged.commit(&mut target).unwrap();
    assert_eq!(report.final_cpu_state, CpuState::Run);
    assert_eq!(target.controller_epoch(), before_epoch + 1);
    assert_eq!(target.scan_sequence(), 0);
    assert_eq!(target.loaded_fingerprint(), Some(new.fingerprint()));
    assert_eq!(target.actual_memory(MEMORY), Some(CanonicalValue::I32(6)));
    assert_eq!(target.retained_memory(MEMORY), Some(CanonicalValue::I32(6)));
    assert!(matches!(
        target.run_scan().unwrap(),
        RunOutcome::Completed(_)
    ));
    assert_eq!(target.actual_memory(MEMORY), Some(CanonicalValue::I32(8)));
}

#[test]
fn dropped_or_stale_stage_cannot_mutate_the_live_target() {
    let old = package(1);
    let new = package(2);
    let mut target = running(&old);
    let plan = RuntimeStateTransferPlan::new(
        old.fingerprint(),
        new.fingerprint(),
        RuntimeInstallDisposition::ArtifactReplacement,
        vec![MEMORY],
        vec![],
        true,
        true,
    )
    .unwrap();

    let before = target.semantic_state_hash();
    let staged = target.stage_atomic_install(&new, &plan).unwrap();
    drop(staged);
    assert_eq!(target.semantic_state_hash(), before);

    let staged = target.stage_atomic_install(&new, &plan).unwrap();
    target.pause_educational().unwrap();
    let changed = target.semantic_state_hash();
    assert!(matches!(
        staged.commit(&mut target),
        Err(AtomicInstallError::StaleTarget)
    ));
    assert_eq!(target.semantic_state_hash(), changed);
    assert_eq!(target.loaded_fingerprint(), Some(old.fingerprint()));
}

#[test]
fn package_identity_change_bumps_epoch_and_preserves_all_runtime_state() {
    let artifact = package(1);
    let mut target = running(&artifact);
    assert!(matches!(
        target.run_scan().unwrap(),
        RunOutcome::Completed(_)
    ));
    let value = target.actual_memory(MEMORY);
    let epoch = target.controller_epoch();
    let plan = RuntimeStateTransferPlan::new(
        artifact.fingerprint(),
        artifact.fingerprint(),
        RuntimeInstallDisposition::PackageIdentityOnly,
        vec![MEMORY],
        vec![],
        true,
        true,
    )
    .unwrap();
    let staged = target.stage_atomic_install(&artifact, &plan).unwrap();
    staged.commit(&mut target).unwrap();
    assert_eq!(target.cpu_state(), CpuState::Run);
    assert_eq!(target.controller_epoch(), epoch + 1);
    assert_eq!(target.actual_memory(MEMORY), value);
}
