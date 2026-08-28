use plc_hardware::{
    ConditionLifecycle, HardwareConditionEvent, HardwareConditionKey, HardwareDiagnosticCode,
    ModuleId, Uuid,
};
use plc_observability::*;

fn hash(label: &str) -> Hash32 {
    plc_runtime::Sha256::digest(label.as_bytes())
}

fn context(controller_id: u128) -> ObservationContext {
    ObservationContext {
        universe_id: UniverseId(0xd001),
        universe_epoch: 3,
        controller_id: VirtualControllerId(controller_id),
        controller_epoch: 5,
        session_id: VirtualOnlineSessionId(0xd003),
        session_epoch: 7,
        package_fingerprint: hash("hardware-diagnostic-package"),
        artifact_fingerprint: hash("hardware-diagnostic-artifact"),
        profile_fingerprint: hash("hardware-diagnostic-profile"),
        target_state_hash: hash("hardware-diagnostic-state"),
        cpu_state: CpuState::Stop,
        virtual_timestamp_ms: 40,
        scan_sequence: 2,
        event_sequence: 9,
        publication_boundary: PublicationBoundary::ScanEnd,
    }
}

fn module_condition() -> HardwareConditionKey {
    HardwareConditionKey::ModuleNotPresent(ModuleId::new(Uuid::deterministic_v4(
        b"hardware-diagnostic-module",
        1,
    )))
}

fn event(sequence: u64, lifecycle: ConditionLifecycle) -> HardwareConditionEvent {
    HardwareConditionEvent {
        sequence,
        command_boundary: sequence,
        condition: module_condition(),
        lifecycle,
        diagnostic_code: HardwareDiagnosticCode::ModuleNotPresent,
    }
}

#[test]
fn hardware_activate_duplicate_and_clear_share_one_causal_diagnostic_episode() {
    let mut ledger = DiagnosticLedger::new(
        DiagnosticRegistry::edu21_runtime(),
        DiagnosticLimits::edu21(),
    )
    .unwrap();
    let mut bridge = HardwareDiagnosticBridge::default();

    let activated = bridge
        .ingest_events(
            &mut ledger,
            context(0xd002),
            &[event(1, ConditionLifecycle::Activated)],
        )
        .unwrap();
    assert_eq!(activated.len(), 1);
    assert!(!activated[0].duplicate);
    assert!(activated[0].verify());
    let active = ledger.active_conditions().next().unwrap();
    assert_eq!(
        active.incoming_occurrence_id,
        activated[0].ledger_occurrence_id
    );
    let definition = ledger
        .registry()
        .definition(active.key.definition_id)
        .unwrap();
    assert_eq!(definition.code.0, "EDU-IO-0001");

    let ledger_hash = ledger.ledger_hash();
    let duplicate = bridge
        .ingest_events(
            &mut ledger,
            context(0xd002),
            &[event(1, ConditionLifecycle::Activated)],
        )
        .unwrap();
    assert!(duplicate[0].duplicate);
    assert_eq!(ledger.ledger_hash(), ledger_hash);

    let cleared = bridge
        .ingest_events(
            &mut ledger,
            context(0xd002),
            &[event(2, ConditionLifecycle::Cleared)],
        )
        .unwrap();
    assert_eq!(ledger.active_conditions().len(), 0);
    let retained = ledger.retained_events();
    let clear_event = retained.last().unwrap();
    assert_eq!(clear_event.kind, DiagnosticEventKind::Cleared);
    assert_eq!(
        clear_event.root_occurrence_id,
        activated[0].ledger_occurrence_id
    );
    assert_eq!(cleared[0].ledger_occurrence_id, clear_event.occurrence_id);
    assert_eq!(bridge.replay_hash().unwrap(), bridge.bridge_hash());
}

#[test]
fn invalid_hardware_streams_fail_atomically_without_diagnostic_publication() {
    let mut ledger = DiagnosticLedger::new(
        DiagnosticRegistry::edu21_runtime(),
        DiagnosticLimits::edu21(),
    )
    .unwrap();
    let mut bridge = HardwareDiagnosticBridge::default();
    let ledger_hash = ledger.ledger_hash();
    let bridge_hash = bridge.bridge_hash();

    let mismatched = HardwareConditionEvent {
        diagnostic_code: HardwareDiagnosticCode::WireBreak,
        ..event(1, ConditionLifecycle::Activated)
    };
    assert!(matches!(
        bridge.ingest_events(&mut ledger, context(0xd002), &[mismatched]),
        Err(HardwareDiagnosticBridgeError::ProviderCodeMismatch { .. })
    ));
    assert_eq!(ledger.ledger_hash(), ledger_hash);
    assert_eq!(bridge.bridge_hash(), bridge_hash);

    assert!(matches!(
        bridge.ingest_events(
            &mut ledger,
            context(0xd002),
            &[event(2, ConditionLifecycle::Cleared)]
        ),
        Err(HardwareDiagnosticBridgeError::MissingActivation(_))
    ));
    assert_eq!(ledger.ledger_hash(), ledger_hash);
    assert_eq!(bridge.bridge_hash(), bridge_hash);

    assert!(matches!(
        bridge.ingest_events(
            &mut ledger,
            context(0xd002),
            &[
                event(2, ConditionLifecycle::Activated),
                event(1, ConditionLifecycle::Cleared),
            ]
        ),
        Err(HardwareDiagnosticBridgeError::ProviderOrderInvalid)
    ));
    assert_eq!(ledger.ledger_hash(), ledger_hash);
    assert_eq!(bridge.bridge_hash(), bridge_hash);
}
