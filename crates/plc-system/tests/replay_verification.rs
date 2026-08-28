use std::collections::BTreeMap;

use plc_runtime::{
    Hash32, RUNTIME_SEMANTICS_VERSION, ReplayEventKind, ReplaySegment, SCHEDULER_VERSION,
    UniverseId, VirtualControllerId,
};
use plc_system::{
    ActorKind, ReplayActorProvenance, ReplayBoundaryHash, ReplayBoundaryKind, ReplayCommandResult,
    ReplayDecodeLimits, ReplayPackage, ReplayPackageError, ReplayPackageEvent, ReplayPackageSpec,
    ReplayPayloadValue, ReplayPriorityClass, ReplayResultStatus, ReplayStateRegion,
    ReplayTypedPayload,
};

fn hash(byte: u8) -> Hash32 {
    Hash32::from_bytes([byte; 32])
}

fn payload(kind: ReplayEventKind, field: &str, value: bool) -> ReplayTypedPayload {
    ReplayTypedPayload::new(
        kind,
        BTreeMap::from([(field.to_owned(), ReplayPayloadValue::Bool(value))]),
    )
    .unwrap()
}

fn event(
    segment: ReplaySegment,
    kind: ReplayEventKind,
    event_sequence: u64,
    virtual_timestamp_ms: u64,
    priority: ReplayPriorityClass,
) -> ReplayPackageEvent {
    ReplayPackageEvent {
        segment,
        segment_predecessor: None,
        predecessor_event_sequence: None,
        universe_timeline_branch: false,
        artifact_hash: hash(2),
        profile_hash: hash(3),
        kind,
        event_sequence,
        virtual_timestamp_ms,
        priority,
        actor: ReplayActorProvenance {
            kind: ActorKind::Operator,
            actor_id: 0x100 + u128::from(event_sequence),
            command_id: 0x200 + u128::from(event_sequence),
            idempotency_key: 0x300 + u128::from(event_sequence),
        },
        payload: payload(kind, "requested", true),
        result: ReplayCommandResult::new(
            ReplayResultStatus::Accepted,
            "ACCEPTED",
            payload(kind, "committed", true),
        )
        .unwrap(),
        runtime_payload_hash: hash(u8::try_from(event_sequence + 10).unwrap()),
        runtime_result_hash: hash(u8::try_from(event_sequence + 20).unwrap()),
    }
}

fn detailed_regions(seed: u8) -> BTreeMap<ReplayStateRegion, Hash32> {
    [
        ReplayStateRegion::Cpu,
        ReplayStateRegion::Memory,
        ReplayStateRegion::Io,
        ReplayStateRegion::TimersCountersEdges,
        ReplayStateRegion::Diagnostics,
        ReplayStateRegion::Forces,
        ReplayStateRegion::Trace,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, region)| {
        (
            region,
            hash(seed.checked_add(u8::try_from(index).unwrap()).unwrap()),
        )
    })
    .collect()
}

fn package_with_two_boundaries() -> ReplayPackage {
    let segment = ReplaySegment {
        universe_id: UniverseId(1),
        universe_epoch: 1,
        controller_id: VirtualControllerId(2),
        controller_epoch: 1,
    };
    let events = vec![
        event(
            segment,
            ReplayEventKind::RawInputAccepted,
            6,
            0,
            ReplayPriorityClass::RawInput,
        ),
        event(
            segment,
            ReplayEventKind::ScanCompleted,
            7,
            10,
            ReplayPriorityClass::ScheduledProgram,
        ),
        event(
            segment,
            ReplayEventKind::ScanCompleted,
            8,
            20,
            ReplayPriorityClass::ScheduledProgram,
        ),
        event(
            segment,
            ReplayEventKind::RawInputAccepted,
            9,
            20,
            ReplayPriorityClass::RawInput,
        ),
    ];
    let boundaries = [
        (7, 1, hash(70), detailed_regions(10)),
        (8, 2, hash(71), detailed_regions(30)),
    ]
    .into_iter()
    .map(
        |(event_sequence, scan_sequence, runtime_state_hash, region_hashes)| ReplayBoundaryHash {
            segment,
            kind: ReplayBoundaryKind::ScanEnd,
            event_sequence,
            causal_input_event_sequence: 6,
            scan_sequence,
            virtual_timestamp_ms: scan_sequence * 10,
            runtime_state_hash,
            semantic_state_hash: Hash32::ZERO,
            region_hashes,
        },
    )
    .collect();
    let spec = ReplayPackageSpec {
        initial_snapshot_hash: hash(1),
        artifact_hash: hash(2),
        profile_hash: hash(3),
        deterministic_seed: 99,
        deterministic_algorithm: "XOSHIRO256SS-1".to_owned(),
        runtime_version: RUNTIME_SEMANTICS_VERSION.to_owned(),
        scheduler_version: SCHEDULER_VERSION.to_owned(),
        events,
        boundaries,
    }
    .bind_event_order()
    .unwrap();
    ReplayPackage::encode(spec).unwrap()
}

fn spec_from(package: &ReplayPackage) -> ReplayPackageSpec {
    ReplayPackageSpec {
        initial_snapshot_hash: package.initial_snapshot_hash(),
        artifact_hash: hash(2),
        profile_hash: hash(3),
        deterministic_seed: 99,
        deterministic_algorithm: "XOSHIRO256SS-1".to_owned(),
        runtime_version: RUNTIME_SEMANTICS_VERSION.to_owned(),
        scheduler_version: SCHEDULER_VERSION.to_owned(),
        events: package.events().to_vec(),
        boundaries: package.boundaries().to_vec(),
    }
}

#[test]
fn verification_executor_stops_at_first_divergent_boundary() {
    let package = package_with_two_boundaries();
    let mut calls = Vec::new();
    let divergence = package
        .verify_with(|event| {
            calls.push(event.event_sequence);
            let boundary = package
                .boundaries()
                .iter()
                .find(|boundary| boundary.event_sequence == event.event_sequence)
                .cloned();
            if event.event_sequence == 8 {
                let mut observed = boundary.unwrap();
                observed
                    .region_hashes
                    .insert(ReplayStateRegion::Io, hash(99));
                observed.bind_event_order(package.events())?;
                Ok(Some(observed))
            } else {
                Ok(boundary)
            }
        })
        .unwrap()
        .unwrap();

    assert_eq!(calls, vec![6, 7, 8]);
    assert_eq!(divergence.boundary_index, 1);
    assert_eq!(divergence.differing_regions, vec![ReplayStateRegion::Io]);
    assert_eq!(divergence.causal_event.unwrap().event_sequence, 6);
}

#[test]
fn timeline_branch_links_the_exact_previous_segment_event() {
    let package = package_with_two_boundaries();
    let mut spec = spec_from(&package);
    let previous = spec.events.last().unwrap().segment;
    let mut branch = event(
        ReplaySegment {
            universe_epoch: 2,
            ..previous
        },
        ReplayEventKind::ObservationBoundary,
        1,
        0,
        ReplayPriorityClass::ControllerLifecycle,
    );
    branch.actor.kind = ActorKind::System;
    branch = branch.linked_from(previous, 9, true);
    spec.events.push(branch.clone());
    let encoded = ReplayPackage::encode(spec.clone()).unwrap();
    let decoded = ReplayPackage::decode(encoded.bytes(), ReplayDecodeLimits::edu21()).unwrap();
    let decoded_branch = decoded.events().last().unwrap();
    assert_eq!(decoded_branch.segment_predecessor, Some(previous));
    assert_eq!(decoded_branch.predecessor_event_sequence, Some(9));
    assert!(decoded_branch.universe_timeline_branch);

    branch.predecessor_event_sequence = Some(8);
    *spec.events.last_mut().unwrap() = branch;
    assert_eq!(
        ReplayPackage::encode(spec),
        Err(ReplayPackageError::InvalidSegmentPredecessor(4))
    );
}

#[test]
fn controller_epoch_segment_requires_its_causal_lifecycle_first_record() {
    let package = package_with_two_boundaries();
    let mut spec = spec_from(&package);
    let previous = spec.events.last().unwrap().segment;
    let transition = event(
        ReplaySegment {
            controller_epoch: 2,
            ..previous
        },
        ReplayEventKind::PowerCycle,
        10,
        20,
        ReplayPriorityClass::ControllerLifecycle,
    )
    .linked_from(previous, 9, false);
    spec.events.push(transition);
    assert!(ReplayPackage::encode(spec.clone()).is_ok());

    let invalid = spec.events.last_mut().unwrap();
    invalid.kind = ReplayEventKind::RequestRun;
    invalid.payload = payload(ReplayEventKind::RequestRun, "requested", true);
    invalid.result = ReplayCommandResult::new(
        ReplayResultStatus::Accepted,
        "ACCEPTED",
        payload(ReplayEventKind::RequestRun, "committed", true),
    )
    .unwrap();
    assert_eq!(
        ReplayPackage::encode(spec),
        Err(ReplayPackageError::InvalidSegmentStart(4))
    );
}

#[test]
fn event_order_region_rejects_typed_payload_mutation_without_rebinding() {
    let package = package_with_two_boundaries();
    let mut spec = spec_from(&package);
    spec.events[0].payload = payload(ReplayEventKind::RawInputAccepted, "requested", false);
    assert_eq!(
        ReplayPackage::encode(spec),
        Err(ReplayPackageError::InvalidBoundary(0))
    );
}

#[test]
fn first_divergence_does_not_prevalidate_later_observations() {
    let package = package_with_two_boundaries();
    let mut observed = package.boundaries().to_vec();
    observed[0]
        .region_hashes
        .insert(ReplayStateRegion::Io, hash(99));
    observed[0].bind_event_order(package.events()).unwrap();
    observed[1].event_sequence = 1;

    let divergence = package.first_divergence(&observed).unwrap().unwrap();
    assert_eq!(divergence.boundary_index, 0);
    assert_eq!(divergence.differing_regions, vec![ReplayStateRegion::Io]);
}

#[test]
fn causal_boundary_pointer_must_name_the_latest_accepted_ingress() {
    let package = package_with_two_boundaries();
    let mut spec = spec_from(&package);
    let mut later_input = spec.events[3].clone();
    later_input.event_sequence = 8;
    later_input.virtual_timestamp_ms = 15;
    let mut second_scan = spec.events[2].clone();
    second_scan.event_sequence = 9;
    let mut tail = spec.events[3].clone();
    tail.event_sequence = 10;
    spec.events = vec![
        spec.events[0].clone(),
        spec.events[1].clone(),
        later_input,
        second_scan,
        tail,
    ];
    spec.boundaries[1].event_sequence = 9;
    spec = spec.bind_event_order().unwrap();

    assert_eq!(
        ReplayPackage::encode(spec),
        Err(ReplayPackageError::OrphanBoundary(6))
    );
}

#[test]
fn command_rejection_requires_a_rejected_typed_result() {
    let package = package_with_two_boundaries();
    let mut spec = spec_from(&package);
    spec.events[0].kind = ReplayEventKind::CommandRejected;
    spec.events[0].payload = payload(ReplayEventKind::CommandRejected, "requested", true);
    spec.events[0].result = ReplayCommandResult::new(
        ReplayResultStatus::Accepted,
        "INCORRECTLY_ACCEPTED",
        payload(ReplayEventKind::CommandRejected, "committed", false),
    )
    .unwrap();

    assert_eq!(
        ReplayPackage::encode(spec),
        Err(ReplayPackageError::InvalidToken("event.result.status"))
    );
}

#[test]
fn causal_total_order_rejects_sequence_gaps_and_bad_epoch_reset() {
    let package = package_with_two_boundaries();
    let mut gap = spec_from(&package);
    gap.events[1].event_sequence = 8;
    assert_eq!(
        ReplayPackage::encode(gap),
        Err(ReplayPackageError::NonCanonicalEventOrder)
    );

    let mut bad_reset = spec_from(&package);
    let previous = bad_reset.events.last().unwrap().segment;
    bad_reset.events.push(
        event(
            ReplaySegment {
                universe_epoch: 2,
                ..previous
            },
            ReplayEventKind::ObservationBoundary,
            2,
            0,
            ReplayPriorityClass::ControllerLifecycle,
        )
        .linked_from(previous, 9, true),
    );
    assert_eq!(
        ReplayPackage::encode(bad_reset),
        Err(ReplayPackageError::NonCanonicalEventOrder)
    );
}
