#![allow(clippy::too_many_lines)]

use std::collections::BTreeMap;

use plc_commissioning::PostLoadMode;
use plc_compiler::CompilerProfile;
use plc_core::{
    CommandContext, CommandEnvelope, CommandOutcome, DomainCommand, Engine, NewObject, ObjectId,
    Payload, PayloadValue, ProfilePin, Project, ProjectObjectKind, TransactionId, Uuid,
};
use plc_hardware::TrainingProfile;
use plc_observability::{Quality, StableTargetId};
use plc_runtime::{CanonicalValue, CpuState, Hash32};
use plc_system::{
    EngineeringSession, SystemBuildError, SystemCommandIdentity, SystemError,
    build_project_controller, project_hardware,
};

struct Fixture {
    engine: Engine,
    controller: ObjectId,
    rack: ObjectId,
    input_module: ObjectId,
    program: ObjectId,
    input_tag: ObjectId,
    output_tag: ObjectId,
    input_member: Uuid,
    output_member: Uuid,
    next_command: u64,
}

impl Fixture {
    fn canonical_scl() -> Self {
        let profile = TrainingProfile::edu21().pin();
        let root = object_id(1);
        let fbd_block = object_id(21);
        let scl_block = object_id(22);
        let fbd_input = member_id(5);
        let fbd_output = member_id(6);
        let scl_input = member_id(7);
        let scl_output = member_id(8);
        let fbd_result = member_id(3);
        let scl_result = member_id(4);
        let project = Project::new(
            Uuid::deterministic_v4(b"plc-system-journey-document", 1),
            root,
            "Integrated EDU-21 Project",
            ProfilePin {
                id: profile.id,
                version: profile.version,
                manifest_hash: profile.manifest_hash,
            },
        );
        let mut fixture = Self {
            engine: Engine::new(project).expect("valid project"),
            controller: object_id(11),
            rack: object_id(12),
            input_module: object_id(13),
            program: object_id(20),
            input_tag: object_id(30),
            output_tag: object_id(31),
            input_member: member_id(1),
            output_member: member_id(2),
            next_command: 1,
        };
        fixture.create(
            object_id(10),
            ProjectObjectKind::Network,
            root,
            "Training network",
            "edu.network/1",
            BTreeMap::new(),
        );
        fixture.create(
            fixture.controller,
            ProjectObjectKind::Controller,
            root,
            "PLC_1",
            "edu.controller/1",
            fields(&[("catalogId", PayloadValue::from("vctrl-c1"))]),
        );
        fixture.create(
            fixture.rack,
            ProjectObjectKind::Rack,
            fixture.controller,
            "Local rack",
            "edu.rack/1",
            BTreeMap::new(),
        );
        fixture.create(
            fixture.input_module,
            ProjectObjectKind::Module,
            fixture.rack,
            "Digital input 16",
            "edu.module/1",
            module_payload("vdi16", 1),
        );
        fixture.create(
            object_id(14),
            ProjectObjectKind::Module,
            fixture.rack,
            "Digital output 16",
            "edu.module/1",
            module_payload("vdo16", 2),
        );
        fixture.create(
            fbd_block,
            ProjectObjectKind::ProgramBlock,
            fixture.controller,
            "Invert_FBD",
            "edu.program-block/1",
            fbd_program_payload(fbd_input, fbd_output),
        );
        fixture.create(
            scl_block,
            ProjectObjectKind::ProgramBlock,
            fixture.controller,
            "Invert_SCL",
            "edu.program-block/1",
            scl_program_payload(scl_input, scl_output),
        );
        fixture.create(
            fixture.program,
            ProjectObjectKind::ProgramBlock,
            fixture.controller,
            "Main_cycle",
            "edu.program-block/1",
            lad_program_payload(
                fixture.input_member,
                fixture.output_member,
                fbd_result,
                scl_result,
                (fbd_block, fbd_input, fbd_output),
                (scl_block, scl_input, scl_output),
            ),
        );
        fixture.create(
            object_id(29),
            ProjectObjectKind::SymbolTable,
            fixture.controller,
            "PLC tags",
            "edu.symbol-table/1",
            BTreeMap::new(),
        );
        fixture.create(
            fixture.input_tag,
            ProjectObjectKind::Tag,
            object_id(29),
            "Physical input",
            "edu.tag/1",
            tag_payload("I", "Input", fixture.program, fixture.input_member),
        );
        fixture.create(
            fixture.output_tag,
            ProjectObjectKind::Tag,
            object_id(29),
            "Physical output",
            "edu.tag/1",
            tag_payload("Q", "Output", fixture.program, fixture.output_member),
        );
        fixture.create(
            object_id(40),
            ProjectObjectKind::Generic,
            fixture.controller,
            "Live values",
            "edu.watch-table/1",
            fields(&[(
                "rows",
                PayloadValue::List(vec![record(&[
                    ("id", PayloadValue::from(member_id(10).to_string())),
                    (
                        "targetTag",
                        PayloadValue::from(fixture.output_tag.to_string()),
                    ),
                    ("layer", PayloadValue::from("delivered-output")),
                    ("displayBase", PayloadValue::from("automatic")),
                    ("order", PayloadValue::Unsigned(0)),
                ])]),
            )]),
        );
        fixture
    }

    fn create(
        &mut self,
        id: ObjectId,
        kind: ProjectObjectKind,
        parent_id: ObjectId,
        display_name: &str,
        payload_schema: &str,
        semantic: BTreeMap<String, PayloadValue>,
    ) {
        self.commit(
            DomainCommand::Create(NewObject {
                id,
                kind,
                parent_id,
                display_name: display_name.to_owned(),
                payload_schema: payload_schema.to_owned(),
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
            command_id: Uuid::deterministic_v4(b"plc-system-journey-command", ordinal),
            transaction_id: TransactionId(Uuid::deterministic_v4(
                b"plc-system-journey-transaction",
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
                actor_id: "system-journey-test".to_owned(),
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
    ObjectId(Uuid::deterministic_v4(
        b"plc-system-journey-object",
        ordinal,
    ))
}

fn member_id(ordinal: u64) -> Uuid {
    Uuid::deterministic_v4(b"plc-system-journey-member", ordinal)
}

fn stable_target(id: ObjectId) -> StableTargetId {
    StableTargetId(u128::from_be_bytes(id.0.into_bytes()))
}

fn identity(ordinal: u128) -> SystemCommandIdentity {
    SystemCommandIdentity {
        command_id: ordinal,
        idempotency_key: ordinal + 10_000,
        author_identity: 77,
    }
}

fn fields(values: &[(&str, PayloadValue)]) -> BTreeMap<String, PayloadValue> {
    values
        .iter()
        .map(|(key, value)| ((*key).to_owned(), value.clone()))
        .collect()
}

fn record(values: &[(&str, PayloadValue)]) -> PayloadValue {
    PayloadValue::Record(fields(values))
}

fn module_payload(catalog: &str, slot: u64) -> BTreeMap<String, PayloadValue> {
    fields(&[
        ("catalogId", PayloadValue::from(catalog)),
        ("slot", PayloadValue::Unsigned(slot)),
        ("addressIntent", PayloadValue::from("auto")),
    ])
}

fn interface_member(id: Uuid, name: &str, role: &str, order: u64) -> PayloadValue {
    record(&[
        ("id", PayloadValue::from(id.to_string())),
        ("name", PayloadValue::from(name)),
        ("role", PayloadValue::from(role)),
        ("type", PayloadValue::from("BOOL")),
        ("order", PayloadValue::Unsigned(order)),
        ("requiredOutput", PayloadValue::Bool(role == "output")),
        ("retentive", PayloadValue::Bool(false)),
    ])
}

fn scl_program_payload(input: Uuid, output: Uuid) -> BTreeMap<String, PayloadValue> {
    fields(&[
        ("blockKind", PayloadValue::from("FC")),
        ("engineeringNumber", PayloadValue::Unsigned(2)),
        ("language", PayloadValue::from("SCL")),
        (
            "sourceText",
            PayloadValue::from("Result := NOT InputValue;"),
        ),
        (
            "interface",
            PayloadValue::List(vec![
                interface_member(input, "InputValue", "input", 0),
                interface_member(output, "Result", "output", 1),
            ]),
        ),
    ])
}

fn fbd_program_payload(input: Uuid, output: Uuid) -> BTreeMap<String, PayloadValue> {
    let load_output = graph_id(101);
    let invert_input = graph_id(102);
    let invert_output = graph_id(103);
    let store_input = graph_id(104);
    fields(&[
        ("blockKind", PayloadValue::from("FC")),
        ("engineeringNumber", PayloadValue::Unsigned(1)),
        ("language", PayloadValue::from("FBD")),
        (
            "interface",
            PayloadValue::List(vec![
                interface_member(input, "InputValue", "input", 0),
                interface_member(output, "Result", "output", 1),
            ]),
        ),
        (
            "graph",
            record(&[
                ("schema", PayloadValue::from("edu.fbd-semantic-graph/1")),
                ("documentId", uuid_value(graph_id(100))),
                (
                    "networks",
                    PayloadValue::List(vec![record(&[
                        ("id", uuid_value(graph_id(105))),
                        ("semanticOrder", PayloadValue::Unsigned(0)),
                        (
                            "nodes",
                            PayloadValue::List(vec![
                                record(&[
                                    ("id", uuid_value(graph_id(106))),
                                    ("nodeKind", PayloadValue::from("load-member")),
                                    ("memberId", uuid_value(input)),
                                    ("semanticOrder", PayloadValue::Unsigned(0)),
                                    (
                                        "ports",
                                        PayloadValue::List(vec![fbd_port(
                                            load_output,
                                            "OUT",
                                            "output",
                                            None,
                                        )]),
                                    ),
                                ]),
                                record(&[
                                    ("id", uuid_value(graph_id(107))),
                                    ("nodeKind", PayloadValue::from("instruction")),
                                    ("instructionCode", PayloadValue::Unsigned(0x0010)),
                                    ("semanticOrder", PayloadValue::Unsigned(1)),
                                    ("stateInstanceId", PayloadValue::Null),
                                    (
                                        "ports",
                                        PayloadValue::List(vec![
                                            fbd_port(invert_input, "IN", "input", Some(0x0010)),
                                            fbd_port(invert_output, "OUT", "output", Some(0x0011)),
                                        ]),
                                    ),
                                ]),
                                record(&[
                                    ("id", uuid_value(graph_id(108))),
                                    ("nodeKind", PayloadValue::from("store-member")),
                                    ("memberId", uuid_value(output)),
                                    ("semanticOrder", PayloadValue::Unsigned(2)),
                                    (
                                        "ports",
                                        PayloadValue::List(vec![fbd_port(
                                            store_input,
                                            "IN",
                                            "input",
                                            None,
                                        )]),
                                    ),
                                ]),
                            ]),
                        ),
                        (
                            "connections",
                            PayloadValue::List(vec![
                                fbd_connection(graph_id(109), load_output, invert_input),
                                fbd_connection(graph_id(110), invert_output, store_input),
                            ]),
                        ),
                    ])]),
                ),
            ]),
        ),
    ])
}

fn fbd_port(id: Uuid, name: &str, direction: &str, formal: Option<u64>) -> PayloadValue {
    let mut values = fields(&[
        ("activation", PayloadValue::from("none")),
        ("dataType", PayloadValue::from("BOOL")),
        ("direction", PayloadValue::from(direction)),
        ("effectRole", PayloadValue::from("value")),
        ("id", uuid_value(id)),
        (
            "multiplicity",
            PayloadValue::from(if direction == "output" { "many" } else { "one" }),
        ),
        ("name", PayloadValue::from(name)),
        ("required", PayloadValue::Bool(direction == "input")),
        ("status", PayloadValue::from("active")),
    ]);
    if let Some(formal) = formal {
        values.insert("formalId".to_owned(), PayloadValue::Unsigned(formal));
        values.insert("formalKind".to_owned(), PayloadValue::from("instruction"));
    }
    PayloadValue::Record(values)
}

fn fbd_connection(id: Uuid, source: Uuid, target: Uuid) -> PayloadValue {
    record(&[
        ("id", uuid_value(id)),
        ("kind", PayloadValue::from("data")),
        ("sourcePortId", uuid_value(source)),
        ("targetPortId", uuid_value(target)),
    ])
}

fn lad_program_payload(
    input: Uuid,
    output: Uuid,
    fbd_result: Uuid,
    scl_result: Uuid,
    fbd: (ObjectId, Uuid, Uuid),
    scl: (ObjectId, Uuid, Uuid),
) -> BTreeMap<String, PayloadValue> {
    let source_output = graph_id(201);
    let fbd_call_input = graph_id(202);
    let fbd_call_output = graph_id(203);
    let scl_call_input = graph_id(204);
    let scl_call_output = graph_id(205);
    let fbd_contact_input = graph_id(206);
    let fbd_contact_output = graph_id(207);
    let scl_contact_input = graph_id(208);
    let scl_contact_output = graph_id(209);
    let coil_input = graph_id(210);
    let nodes = vec![
        record(&[
            ("id", uuid_value(graph_id(211))),
            ("nodeKind", PayloadValue::from("power-source")),
            ("semanticOrder", PayloadValue::Unsigned(0)),
            (
                "powerPorts",
                PayloadValue::List(vec![lad_power_port(source_output, "output")]),
            ),
        ]),
        lad_call_node(
            graph_id(212),
            graph_id(213),
            1,
            fbd,
            input,
            fbd_result,
            fbd_call_input,
            fbd_call_output,
        ),
        lad_call_node(
            graph_id(214),
            graph_id(215),
            2,
            scl,
            input,
            scl_result,
            scl_call_input,
            scl_call_output,
        ),
        lad_contact_node(
            graph_id(216),
            graph_id(217),
            3,
            fbd_result,
            fbd_contact_input,
            fbd_contact_output,
        ),
        lad_contact_node(
            graph_id(218),
            graph_id(219),
            4,
            scl_result,
            scl_contact_input,
            scl_contact_output,
        ),
        record(&[
            ("id", uuid_value(graph_id(220))),
            ("nodeKind", PayloadValue::from("coil")),
            ("mode", PayloadValue::from("normal")),
            ("semanticOrder", PayloadValue::Unsigned(5)),
            ("operand", lad_operand(graph_id(221), output)),
            (
                "powerPorts",
                PayloadValue::List(vec![lad_power_port(coil_input, "input")]),
            ),
        ]),
    ];
    let edges = vec![
        lad_edge(graph_id(222), source_output, fbd_call_input),
        lad_edge(graph_id(223), fbd_call_output, scl_call_input),
        lad_edge(graph_id(224), scl_call_output, fbd_contact_input),
        lad_edge(graph_id(225), fbd_contact_output, scl_contact_input),
        lad_edge(graph_id(226), scl_contact_output, coil_input),
    ];
    fields(&[
        ("blockKind", PayloadValue::from("OB")),
        ("engineeringNumber", PayloadValue::Unsigned(1)),
        ("obRole", PayloadValue::from("CyclicMain")),
        ("language", PayloadValue::from("LAD")),
        (
            "interface",
            PayloadValue::List(vec![
                interface_member(input, "InputValue", "temp", 0),
                interface_member(output, "OutputValue", "temp", 1),
                interface_member(fbd_result, "FbdResult", "temp", 2),
                interface_member(scl_result, "SclResult", "temp", 3),
            ]),
        ),
        (
            "graph",
            record(&[
                ("schema", PayloadValue::from("edu.lad-semantic-graph/1")),
                ("documentId", uuid_value(graph_id(200))),
                ("semanticRevision", PayloadValue::Unsigned(0)),
                (
                    "networks",
                    PayloadValue::List(vec![record(&[
                        ("id", uuid_value(graph_id(227))),
                        ("semanticOrder", PayloadValue::Unsigned(0)),
                        ("nodes", PayloadValue::List(nodes)),
                        ("edges", PayloadValue::List(edges)),
                        ("branches", PayloadValue::List(Vec::new())),
                    ])]),
                ),
            ]),
        ),
    ])
}

#[allow(clippy::too_many_arguments)]
fn lad_call_node(
    id: Uuid,
    call_site: Uuid,
    order: u64,
    target: (ObjectId, Uuid, Uuid),
    caller_input: Uuid,
    caller_output: Uuid,
    power_input: Uuid,
    power_output: Uuid,
) -> PayloadValue {
    record(&[
        ("id", uuid_value(id)),
        ("callSiteId", uuid_value(call_site)),
        ("instructionCode", PayloadValue::Unsigned(0x0200)),
        ("nodeKind", PayloadValue::from("call")),
        ("targetBlockId", PayloadValue::from(target.0.to_string())),
        ("instance", PayloadValue::Null),
        ("semanticOrder", PayloadValue::Unsigned(order)),
        (
            "pins",
            PayloadValue::List(vec![
                lad_call_pin(
                    graph_id(300 + order * 4),
                    graph_id(301 + order * 4),
                    "InputValue",
                    "input",
                    target.1,
                    caller_input,
                ),
                lad_call_pin(
                    graph_id(302 + order * 4),
                    graph_id(303 + order * 4),
                    "Result",
                    "output",
                    target.2,
                    caller_output,
                ),
            ]),
        ),
        (
            "powerPorts",
            PayloadValue::List(vec![
                lad_power_port(power_input, "input"),
                lad_power_port(power_output, "output"),
            ]),
        ),
    ])
}

fn lad_call_pin(
    id: Uuid,
    operand_id: Uuid,
    name: &str,
    direction: &str,
    formal_id: Uuid,
    caller_member: Uuid,
) -> PayloadValue {
    record(&[
        ("id", uuid_value(id)),
        ("formalKind", PayloadValue::from("block-member")),
        ("formalId", uuid_value(formal_id)),
        ("name", PayloadValue::from(name)),
        ("direction", PayloadValue::from(direction)),
        ("dataType", PayloadValue::from("BOOL")),
        ("required", PayloadValue::Bool(true)),
        ("status", PayloadValue::from("active")),
        ("binding", lad_operand(operand_id, caller_member)),
    ])
}

fn lad_contact_node(
    id: Uuid,
    operand_id: Uuid,
    order: u64,
    member: Uuid,
    power_input: Uuid,
    power_output: Uuid,
) -> PayloadValue {
    record(&[
        ("id", uuid_value(id)),
        ("nodeKind", PayloadValue::from("contact")),
        ("mode", PayloadValue::from("normally-open")),
        ("semanticOrder", PayloadValue::Unsigned(order)),
        ("operand", lad_operand(operand_id, member)),
        (
            "powerPorts",
            PayloadValue::List(vec![
                lad_power_port(power_input, "input"),
                lad_power_port(power_output, "output"),
            ]),
        ),
    ])
}

fn lad_operand(id: Uuid, member: Uuid) -> PayloadValue {
    record(&[
        ("id", uuid_value(id)),
        ("kind", PayloadValue::from("caller-member")),
        ("memberId", uuid_value(member)),
    ])
}

fn lad_power_port(id: Uuid, direction: &str) -> PayloadValue {
    record(&[
        ("id", uuid_value(id)),
        ("direction", PayloadValue::from(direction)),
    ])
}

fn lad_edge(id: Uuid, source: Uuid, target: Uuid) -> PayloadValue {
    record(&[
        ("id", uuid_value(id)),
        ("sourcePortId", uuid_value(source)),
        ("targetPortId", uuid_value(target)),
    ])
}

fn graph_id(ordinal: u64) -> Uuid {
    Uuid::deterministic_v4(b"plc-system-ui-graph", ordinal)
}

fn uuid_value(id: Uuid) -> PayloadValue {
    PayloadValue::from(id.to_string())
}

fn tag_payload(
    area: &str,
    kind: &str,
    block: ObjectId,
    member: Uuid,
) -> BTreeMap<String, PayloadValue> {
    fields(&[
        ("addressArea", PayloadValue::from(area)),
        ("addressIntent", PayloadValue::from("auto")),
        ("dataType", PayloadValue::from("BOOL")),
        ("tagKind", PayloadValue::from(kind)),
        ("blockId", PayloadValue::from(block.to_string())),
        ("memberId", PayloadValue::from(member.to_string())),
    ])
}

fn loaded_session(fixture: &Fixture) -> EngineeringSession {
    let mut session = EngineeringSession::new(fixture.engine.project().clone(), fixture.controller)
        .expect("session");
    session.build().expect("verified build");
    session.power_on().expect("power on");
    let preview = session.preview_load(PostLoadMode::Stop).expect("preview");
    session.commit_load(&preview).expect("load");
    session.go_online().expect("online");
    session.start_monitoring().expect("monitor");
    session
}

#[test]
fn hardware_compiler_runtime_and_load_share_one_profile_authority() {
    let fixture = Fixture::canonical_scl();
    let training = TrainingProfile::edu21();
    let expected_manifest = Hash32::from_bytes(training.manifest_hash().0);
    let compiler_profile =
        CompilerProfile::from_training_profile(&training).expect("compiler projection");
    let hardware = project_hardware(fixture.engine.project());
    assert!(hardware.can_build());
    assert_eq!(hardware.profile().id(), training.id());
    assert_eq!(hardware.profile().version(), "1.0.0");
    assert_eq!(hardware.profile().manifest_hash(), training.manifest_hash());

    let build = build_project_controller(fixture.engine.project(), fixture.controller)
        .expect("integrated build");
    let compiler = build.compiler_artifact();
    assert_eq!(compiler.profile_identity(), training.id());
    assert_eq!(compiler.profile_version(), training.version());
    assert_eq!(compiler.profile_manifest_hash(), expected_manifest);
    assert_eq!(
        compiler.capability_manifest_hash(),
        compiler_profile.capability_manifest_hash()
    );
    assert_eq!(
        build.runtime_artifact().spec().profile_fingerprint,
        expected_manifest
    );
    assert_eq!(
        build.load_package().profile_fingerprint(),
        expected_manifest
    );
    assert_eq!(
        build.load_package().capability_fingerprint(),
        compiler.capability_manifest_hash()
    );
}

#[test]
fn tampered_and_unapproved_project_profile_pins_block_system_builds() {
    let shipped = TrainingProfile::edu21().pin();
    let pins = [
        ProfilePin {
            id: shipped.id.clone(),
            version: shipped.version.clone(),
            manifest_hash: plc_core::Sha256Digest([0xa5; 32]),
        },
        ProfilePin {
            id: "EDU-21 Experimental".to_owned(),
            version: shipped.version.clone(),
            manifest_hash: shipped.manifest_hash,
        },
        ProfilePin {
            id: shipped.id,
            version: "1.0".to_owned(),
            manifest_hash: shipped.manifest_hash,
        },
    ];
    for (ordinal, pin) in pins.into_iter().enumerate() {
        let ordinal = u64::try_from(ordinal).unwrap();
        let root = object_id(500 + ordinal);
        let project = Project::new(
            Uuid::deterministic_v4(b"plc-system-unapproved-profile", ordinal),
            root,
            "Unapproved profile",
            pin,
        );
        let hardware = project_hardware(&project);
        assert!(!hardware.can_build());
        assert!(hardware.diagnostics().iter().any(|diagnostic| {
            diagnostic.code == "EDU-SYS-1000" && diagnostic.primary_object_id == root
        }));
        assert!(matches!(
            build_project_controller(&project, object_id(999)),
            Err(SystemBuildError::ProjectionBlocked(_))
        ));
    }
}

#[test]
fn one_project_drives_raw_input_through_scl_to_delivered_output() {
    let fixture = Fixture::canonical_scl();
    let mut session = loaded_session(&fixture);
    session.request_run().expect("run");
    session
        .set_raw_virtual_input(
            identity(1),
            stable_target(fixture.input_tag),
            CanonicalValue::Bool(true),
        )
        .expect("input");
    session.run_scan(identity(2)).expect("scan");

    let read = session.read_model().expect("read model");
    let output = read
        .probes
        .iter()
        .find(|probe| probe.identity == stable_target(fixture.output_tag))
        .expect("output probe");
    assert_eq!(output.natural_value, Some(CanonicalValue::Bool(false)));
    assert_eq!(
        output.delivered_output_value,
        Some(CanonicalValue::Bool(false))
    );
    assert_eq!(output.quality, Quality::Good);
    session
        .set_raw_virtual_input(
            identity(3),
            stable_target(fixture.input_tag),
            CanonicalValue::Bool(false),
        )
        .expect("changed input");
    session.run_scan(identity(4)).expect("changed scan");
    let read = session.read_model().expect("changed read model");
    let output = read
        .probes
        .iter()
        .find(|probe| probe.identity == stable_target(fixture.output_tag))
        .expect("changed output probe");
    assert_eq!(output.natural_value, Some(CanonicalValue::Bool(true)));
    assert_eq!(
        output.delivered_output_value,
        Some(CanonicalValue::Bool(true))
    );
    assert_eq!(
        read.watches[0].latest_samples[0]
            .1
            .map(|sample| sample.value),
        Some(CanonicalValue::Bool(true))
    );
}

#[test]
fn aggregate_snapshot_is_deterministic_tamper_safe_and_restores_runtime_values() {
    let fixture = Fixture::canonical_scl();
    let mut session = loaded_session(&fixture);
    session.request_run().expect("run");
    session
        .set_raw_virtual_input(
            identity(10),
            stable_target(fixture.input_tag),
            CanonicalValue::Bool(true),
        )
        .expect("input");
    session.run_scan(identity(11)).expect("scan");
    session.request_stop().expect("stop");
    let snapshot = session.capture_snapshot().expect("snapshot");
    assert_eq!(
        snapshot.content_hash,
        session
            .capture_snapshot()
            .expect("repeat snapshot")
            .content_hash
    );

    let mut corrupted = snapshot.clone();
    corrupted.content_hash = Hash32::ZERO;
    let before_rejection = session.universe().semantic_state_hash();
    assert!(session.restore_snapshot(&corrupted).is_err());
    assert_eq!(before_rejection, session.universe().semantic_state_hash());

    session
        .set_raw_virtual_input(
            identity(12),
            stable_target(fixture.input_tag),
            CanonicalValue::Bool(false),
        )
        .expect("changed input");
    session.restore_snapshot(&snapshot).expect("atomic restore");
    let read = session.read_model().expect("read model");
    let input = read
        .probes
        .iter()
        .find(|probe| probe.identity == stable_target(fixture.input_tag))
        .expect("input probe");
    assert_eq!(input.raw_input_value, Some(CanonicalValue::Bool(true)));
    assert!(read.universe_epoch > 1);
}

#[test]
fn invalid_offline_edit_remains_visible_while_loaded_run_state_is_preserved() {
    let mut fixture = Fixture::canonical_scl();
    let mut session = loaded_session(&fixture);
    session.request_run().expect("run");
    let loaded_fingerprint = session
        .read_model()
        .expect("before")
        .loaded_artifact_fingerprint;

    fixture.commit(
        DomainCommand::SetSemanticField {
            object_id: fixture.input_module,
            key: "slot".to_owned(),
            value: PayloadValue::Unsigned(2),
        },
        &[fixture.input_module],
    );
    let refresh = session
        .refresh_project(fixture.engine.project().clone())
        .expect("invalid offline project is still adopted");
    assert!(refresh.semantic_changed);
    assert!(refresh.loaded_runtime_preserved);
    let status = session.status();
    assert!(status.projection_blocked);
    assert!(status.loaded);
    assert_eq!(status.cpu_state, CpuState::Run);
    assert_eq!(
        status.software_to_loaded,
        Some(plc_commissioning::MatchComparison::Mismatch)
    );
    assert_eq!(
        status.hardware_to_loaded,
        Some(plc_commissioning::MatchComparison::Mismatch)
    );
    assert_eq!(
        session
            .read_model()
            .expect("invalid-state read")
            .loaded_artifact_fingerprint,
        loaded_fingerprint
    );
    assert!(matches!(session.build(), Err(SystemError::Build(_))));
    assert!(matches!(
        session.preview_load(PostLoadMode::Stop),
        Err(SystemError::CurrentBuildStale)
    ));
    session
        .run_scan(identity(30))
        .expect("loaded runtime keeps scanning");
}

#[test]
fn presentation_only_edit_preserves_current_build_and_online_match() {
    let mut fixture = Fixture::canonical_scl();
    let mut session = loaded_session(&fixture);
    let artifact_fingerprint = session
        .current_build()
        .expect("current build")
        .runtime_artifact()
        .fingerprint();

    fixture.commit(
        DomainCommand::SetPresentationField {
            object_id: fixture.program,
            key: "editorLayout".to_owned(),
            value: record(&[
                ("zoomPercent", PayloadValue::Unsigned(125)),
                ("selectedNode", PayloadValue::Null),
            ]),
        },
        &[fixture.program],
    );
    let refresh = session
        .refresh_project(fixture.engine.project().clone())
        .expect("presentation refresh");
    assert!(refresh.document_changed);
    assert!(!refresh.semantic_changed);
    assert!(!refresh.build_invalidated);
    assert!(refresh.loaded_runtime_preserved);

    let status = session.status();
    assert!(status.build_current);
    assert!(status.loaded);
    assert_eq!(
        status.software_to_loaded,
        Some(plc_commissioning::MatchComparison::Match)
    );
    assert_eq!(
        status.hardware_to_loaded,
        Some(plc_commissioning::MatchComparison::Match)
    );
    assert_eq!(
        session
            .current_build()
            .expect("preserved build")
            .runtime_artifact()
            .fingerprint(),
        artifact_fingerprint
    );
}
