#![allow(clippy::too_many_lines)]

use std::collections::BTreeMap;

use plc_core::{
    CommandContext, CommandEnvelope, CommandOutcome, DomainCommand, Engine, NewObject, ObjectId,
    Payload, PayloadValue, ProfilePin, Project, ProjectObjectKind, TransactionId, Uuid,
};
use plc_hardware::TrainingProfile;
use plc_system::project_hardware;

struct Fixture {
    engine: Engine,
    network: ObjectId,
    controller: ObjectId,
    rack: ObjectId,
    input: ObjectId,
    output: ObjectId,
    next_command: u64,
}

impl Fixture {
    fn valid() -> Self {
        let profile = TrainingProfile::edu21().pin();
        let root = object_id(1);
        let project = Project::new(
            Uuid::deterministic_v4(b"plc-system-document", 1),
            root,
            "Canonical EDU-21 Project",
            ProfilePin {
                id: profile.id,
                version: profile.version,
                manifest_hash: profile.manifest_hash,
            },
        );
        let mut fixture = Self {
            engine: Engine::new(project).expect("valid canonical project"),
            network: object_id(10),
            controller: object_id(11),
            rack: object_id(12),
            input: object_id(13),
            output: object_id(14),
            next_command: 1,
        };
        fixture.create(
            fixture.network,
            ProjectObjectKind::Network,
            root,
            "Training network",
            BTreeMap::new(),
        );
        fixture.create(
            fixture.controller,
            ProjectObjectKind::Controller,
            root,
            "PLC_1",
            semantic(&[("catalogId", PayloadValue::from("vctrl-c1"))]),
        );
        fixture.create(
            fixture.rack,
            ProjectObjectKind::Rack,
            fixture.controller,
            "Local rack",
            BTreeMap::new(),
        );
        fixture.create(
            fixture.input,
            ProjectObjectKind::Module,
            fixture.rack,
            "Digital input 16",
            module_payload("vdi16", 1, "auto", None),
        );
        fixture.create(
            fixture.output,
            ProjectObjectKind::Module,
            fixture.rack,
            "Digital output 16",
            module_payload("vdo16", 2, "auto", None),
        );
        fixture
    }

    fn create(
        &mut self,
        id: ObjectId,
        kind: ProjectObjectKind,
        parent_id: ObjectId,
        display_name: &str,
        semantic: BTreeMap<String, PayloadValue>,
    ) {
        self.commit(
            DomainCommand::Create(NewObject {
                id,
                kind,
                parent_id,
                display_name: display_name.to_owned(),
                payload_schema: "edu.system-projection/1".to_owned(),
                payload: Payload {
                    semantic,
                    presentation: BTreeMap::new(),
                },
            }),
            &[parent_id],
        );
    }

    fn commit(&mut self, command: DomainCommand, preconditions: &[ObjectId]) {
        let ordinal = self.next_command;
        self.next_command += 1;
        let envelope = CommandEnvelope {
            command_id: Uuid::deterministic_v4(b"plc-system-command", ordinal),
            transaction_id: TransactionId(Uuid::deterministic_v4(
                b"plc-system-transaction",
                ordinal,
            )),
            expected_document_revision: self.engine.project().document_revision(),
            expected_object_revisions: preconditions
                .iter()
                .map(|id| {
                    (
                        *id,
                        self.engine
                            .project()
                            .object(*id)
                            .expect("precondition object")
                            .object_revision,
                    )
                })
                .collect(),
            context: CommandContext {
                actor_id: "canonical-projection-test".to_owned(),
                can_mutate: true,
            },
            command,
        };
        let result = self.engine.execute(&envelope);
        assert_eq!(
            result.outcome,
            CommandOutcome::Committed,
            "{:?}",
            result.diagnostics
        );
    }
}

fn object_id(ordinal: u64) -> ObjectId {
    ObjectId(Uuid::deterministic_v4(b"plc-system-object", ordinal))
}

fn semantic(values: &[(&str, PayloadValue)]) -> BTreeMap<String, PayloadValue> {
    values
        .iter()
        .map(|(key, value)| ((*key).to_owned(), value.clone()))
        .collect()
}

fn module_payload(
    catalog: &str,
    slot: u64,
    address_intent: &str,
    input_start: Option<u64>,
) -> BTreeMap<String, PayloadValue> {
    let mut values = semantic(&[
        ("catalogId", PayloadValue::from(catalog)),
        ("slot", PayloadValue::Unsigned(slot)),
        ("addressIntent", PayloadValue::from(address_intent)),
    ]);
    if let Some(start) = input_start {
        values.insert("inputStart".to_owned(), PayloadValue::Unsigned(start));
    }
    values
}

#[test]
fn canonical_project_projects_to_real_deterministic_hardware_artifact() {
    let fixture = Fixture::valid();
    let first = project_hardware(fixture.engine.project());
    let second = project_hardware(fixture.engine.project());

    assert!(first.can_build(), "{:?}", first.diagnostics());
    assert_eq!(first, second);
    assert_eq!(
        first.source_document_hash(),
        fixture.engine.project().document_hash()
    );
    assert_eq!(
        first.source_semantic_fingerprint(),
        fixture.engine.project().semantic_fingerprint()
    );
    assert_eq!(
        first
            .artifact()
            .expect("hardware artifact")
            .channel_bindings
            .len(),
        32
    );
    assert_eq!(
        first
            .allocation_preview()
            .expect("automatic address preview")
            .changes
            .len(),
        2
    );
}

#[test]
fn modular_controller_projects_its_core_and_authored_power_supply() {
    let mut fixture = Fixture::valid();
    fixture.commit(
        DomainCommand::SetSemanticField {
            object_id: fixture.controller,
            key: "catalogId".to_owned(),
            value: PayloadValue::from("vctrl-m1"),
        },
        &[fixture.controller],
    );
    fixture.commit(
        DomainCommand::SetSemanticField {
            object_id: fixture.input,
            key: "slot".to_owned(),
            value: PayloadValue::Unsigned(2),
        },
        &[fixture.input],
    );
    fixture.commit(
        DomainCommand::SetSemanticField {
            object_id: fixture.output,
            key: "slot".to_owned(),
            value: PayloadValue::Unsigned(3),
        },
        &[fixture.output],
    );
    let power = object_id(15);
    fixture.create(
        power,
        ProjectObjectKind::Module,
        fixture.rack,
        "Virtual power supply",
        module_payload("vpwr1", 0, "auto", None),
    );

    let projection = project_hardware(fixture.engine.project());
    assert!(projection.can_build(), "{:?}", projection.diagnostics());
    let rack = &projection
        .hardware_project()
        .controllers()
        .get(&plc_hardware::ControllerId::from(fixture.controller.0))
        .expect("projected modular controller")
        .local_rack;
    assert!(matches!(
        rack.slots.get(&1).and_then(|slot| slot.installed.as_ref()),
        Some(plc_hardware::InstalledOccupant::ControllerCore(id)) if *id == plc_hardware::ControllerId::from(fixture.controller.0)
    ));
    assert!(matches!(
        rack.slots.get(&0).and_then(|slot| slot.installed.as_ref()),
        Some(plc_hardware::InstalledOccupant::Module(module))
            if module.catalog_id == plc_hardware::ModuleCatalogId::Vpwr1
    ));
}

#[test]
fn semantic_edits_change_projection_while_presentation_edits_do_not() {
    let mut fixture = Fixture::valid();
    let baseline = project_hardware(fixture.engine.project());
    let baseline_artifact = baseline.artifact().expect("baseline artifact").clone();

    fixture.commit(
        DomainCommand::SetPresentationField {
            object_id: fixture.input,
            key: "canvasX".to_owned(),
            value: PayloadValue::Signed(480),
        },
        &[fixture.input],
    );
    let presentation = project_hardware(fixture.engine.project());
    assert_eq!(
        presentation.source_semantic_fingerprint(),
        baseline.source_semantic_fingerprint()
    );
    assert_eq!(
        presentation
            .artifact()
            .expect("presentation-only artifact")
            .hardware_fingerprint,
        baseline_artifact.hardware_fingerprint
    );

    fixture.commit(
        DomainCommand::SetSemanticField {
            object_id: fixture.input,
            key: "catalogId".to_owned(),
            value: PayloadValue::from("vai4"),
        },
        &[fixture.input],
    );
    let semantic_change = project_hardware(fixture.engine.project());
    assert!(
        semantic_change.can_build(),
        "{:?}",
        semantic_change.diagnostics()
    );
    assert_ne!(
        semantic_change.source_semantic_fingerprint(),
        baseline.source_semantic_fingerprint()
    );
    assert_ne!(
        semantic_change
            .artifact()
            .expect("changed artifact")
            .hardware_fingerprint,
        baseline_artifact.hardware_fingerprint
    );
}

#[test]
fn tombstones_and_invalid_canonical_shapes_are_honored_without_hidden_state() {
    let mut fixture = Fixture::valid();
    fixture.commit(
        DomainCommand::Delete {
            object_id: fixture.output,
        },
        &[fixture.output],
    );
    let after_delete = project_hardware(fixture.engine.project());
    assert!(after_delete.can_build(), "{:?}", after_delete.diagnostics());
    assert_eq!(
        after_delete
            .artifact()
            .expect("artifact without tombstoned output")
            .channel_bindings
            .len(),
        16
    );

    let duplicate = object_id(15);
    fixture.create(
        duplicate,
        ProjectObjectKind::Module,
        fixture.rack,
        "Conflicting input",
        module_payload("vdi16", 1, "auto", None),
    );
    let conflict = project_hardware(fixture.engine.project());
    assert!(!conflict.can_build());
    assert!(conflict.artifact().is_none());
    assert!(conflict.diagnostics().iter().any(|diagnostic| {
        diagnostic.code == "EDU-SYS-1104" && diagnostic.primary_object_id == duplicate
    }));
}

#[test]
fn real_address_conflicts_are_blocking_and_project_anchored() {
    let mut fixture = Fixture::valid();
    fixture.commit(
        DomainCommand::SetSemanticField {
            object_id: fixture.input,
            key: "addressIntent".to_owned(),
            value: PayloadValue::from("explicit"),
        },
        &[fixture.input],
    );
    fixture.commit(
        DomainCommand::SetSemanticField {
            object_id: fixture.input,
            key: "inputStart".to_owned(),
            value: PayloadValue::Unsigned(0),
        },
        &[fixture.input],
    );
    let second_input = object_id(16);
    fixture.create(
        second_input,
        ProjectObjectKind::Module,
        fixture.rack,
        "Overlapping input",
        module_payload("vdi16", 3, "explicit", Some(0)),
    );

    let conflict = project_hardware(fixture.engine.project());
    assert!(!conflict.can_build());
    assert!(conflict.artifact().is_none());
    assert!(conflict.diagnostics().iter().any(|diagnostic| {
        diagnostic.code == "EDU-HW-0002"
            && [fixture.input, second_input].contains(&diagnostic.primary_object_id)
            && diagnostic
                .related_object_ids
                .iter()
                .any(|id| [fixture.input, second_input].contains(id))
    }));
}

#[test]
fn absent_network_and_wrong_profile_are_honest_blockers() {
    let profile = ProfilePin {
        id: "Not EDU-21".to_owned(),
        version: "0".to_owned(),
        manifest_hash: plc_core::Sha256Digest([0; 32]),
    };
    let root = object_id(100);
    let project = Project::new(
        Uuid::deterministic_v4(b"plc-system-invalid-document", 1),
        root,
        "Invalid",
        profile,
    );
    let projection = project_hardware(&project);

    assert!(!projection.can_build());
    assert!(projection.artifact().is_none());
    assert!(projection.diagnostics().iter().any(|diagnostic| {
        diagnostic.code == "EDU-SYS-1000" && diagnostic.primary_object_id == root
    }));
    assert!(projection.diagnostics().iter().any(|diagnostic| {
        diagnostic.code == "EDU-SYS-1001" && diagnostic.primary_object_id == root
    }));
}
