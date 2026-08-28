use std::collections::BTreeMap;

use plc_core::{
    CommandContext, CommandEnvelope, CommandOutcome, DomainCommand, Engine, NewObject, ObjectId,
    Payload, PayloadValue, ProfilePin, Project, ProjectObjectKind, TransactionId, Uuid,
};
use plc_hardware::TrainingProfile;

pub(crate) struct RuntimeFixture {
    pub(crate) engine: Engine,
    pub(crate) input_module: ObjectId,
    pub(crate) input_tag: ObjectId,
    pub(crate) output_tag: ObjectId,
    pub(crate) trace: ObjectId,
    next_command: u64,
}

impl RuntimeFixture {
    #[allow(clippy::too_many_lines)]
    pub(crate) fn canonical() -> Self {
        let profile = TrainingProfile::edu21().pin();
        let root = object_id(1);
        let controller = object_id(3);
        let rack = object_id(4);
        let input_module = object_id(5);
        let program = object_id(7);
        let symbol_table = object_id(8);
        let input_tag = object_id(9);
        let output_tag = object_id(10);
        let trace = object_id(12);
        let input_member = member_id(1);
        let output_member = member_id(2);
        let project = Project::new(
            Uuid::deterministic_v4(b"plc-wasm-fixture-document", 1),
            root,
            "WASM Runtime Journey",
            ProfilePin {
                id: profile.id,
                version: profile.version,
                manifest_hash: profile.manifest_hash,
            },
        );
        let mut fixture = Self {
            engine: Engine::new(project).expect("valid fixture root"),
            input_module,
            input_tag,
            output_tag,
            trace,
            next_command: 1,
        };
        fixture.create(
            object_id(2),
            ProjectObjectKind::Network,
            root,
            "Training network",
            "edu.network/1",
            BTreeMap::new(),
        );
        fixture.create(
            controller,
            ProjectObjectKind::Controller,
            root,
            "PLC_1",
            "edu.controller/1",
            fields(&[("catalogId", PayloadValue::from("vctrl-c1"))]),
        );
        fixture.create(
            rack,
            ProjectObjectKind::Rack,
            controller,
            "Local rack",
            "edu.rack/1",
            BTreeMap::new(),
        );
        fixture.create(
            input_module,
            ProjectObjectKind::Module,
            rack,
            "Digital input 16",
            "edu.module/1",
            module_payload("vdi16", 1),
        );
        fixture.create(
            object_id(6),
            ProjectObjectKind::Module,
            rack,
            "Digital output 16",
            "edu.module/1",
            module_payload("vdo16", 2),
        );
        fixture.create(
            program,
            ProjectObjectKind::ProgramBlock,
            controller,
            "Main_cycle",
            "edu.program-block/1",
            fields(&[
                ("blockKind", PayloadValue::from("OB")),
                ("engineeringNumber", PayloadValue::Unsigned(1)),
                ("obRole", PayloadValue::from("CyclicMain")),
                ("language", PayloadValue::from("LAD")),
                (
                    "interface",
                    PayloadValue::List(vec![
                        interface_member(input_member, "InputValue", "temp", 0),
                        interface_member(output_member, "OutputValue", "temp", 1),
                    ]),
                ),
                ("graph", ladder_graph(input_member, output_member)),
            ]),
        );
        fixture.create(
            symbol_table,
            ProjectObjectKind::SymbolTable,
            controller,
            "PLC tags",
            "edu.symbol-table/1",
            BTreeMap::new(),
        );
        fixture.create(
            input_tag,
            ProjectObjectKind::Tag,
            symbol_table,
            "Physical input",
            "edu.tag/1",
            tag_payload("I", "Input", program, input_member),
        );
        fixture.create(
            output_tag,
            ProjectObjectKind::Tag,
            symbol_table,
            "Physical output",
            "edu.tag/1",
            tag_payload("Q", "Output", program, output_member),
        );
        fixture.create(
            object_id(11),
            ProjectObjectKind::Generic,
            controller,
            "Live outputs",
            "edu.watch-table/1",
            fields(&[(
                "rows",
                PayloadValue::List(vec![record(&[
                    ("id", PayloadValue::from(member_id(11).to_string())),
                    ("targetTag", PayloadValue::from(output_tag.to_string())),
                    ("layer", PayloadValue::from("delivered-output")),
                    ("displayBase", PayloadValue::from("automatic")),
                    ("order", PayloadValue::Unsigned(0)),
                ])]),
            )]),
        );
        fixture.create(
            trace,
            ProjectObjectKind::Generic,
            controller,
            "Output trace",
            "edu.trace-configuration/1",
            fields(&[
                (
                    "channels",
                    PayloadValue::List(vec![record(&[
                        ("id", PayloadValue::from(member_id(12).to_string())),
                        ("alias", PayloadValue::from("Delivered output")),
                        ("targetTag", PayloadValue::from(output_tag.to_string())),
                        ("layer", PayloadValue::from("delivered-output")),
                    ])]),
                ),
                ("trigger", PayloadValue::from("immediate")),
                ("everyScans", PayloadValue::Unsigned(1)),
                ("preSamples", PayloadValue::Unsigned(0)),
                ("postSamples", PayloadValue::Unsigned(1)),
                ("maximumDurationMs", PayloadValue::Unsigned(1_000)),
            ]),
        );
        fixture
    }

    pub(crate) fn project(&self) -> &Project {
        self.engine.project()
    }

    pub(crate) fn make_hardware_invalid(&mut self) {
        self.commit(
            DomainCommand::SetSemanticField {
                object_id: self.input_module,
                key: "slot".to_owned(),
                value: PayloadValue::Unsigned(2),
            },
            &[self.input_module],
        );
    }

    pub(crate) fn rename_program_presentation(&mut self) {
        self.commit(
            DomainCommand::SetPresentationField {
                object_id: object_id(7),
                key: "editorLayout".to_owned(),
                value: record(&[("zoomPercent", PayloadValue::Unsigned(125))]),
            },
            &[object_id(7)],
        );
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
        let result = self.engine.execute(&CommandEnvelope {
            command_id: Uuid::deterministic_v4(b"plc-wasm-fixture-command", ordinal),
            transaction_id: TransactionId(Uuid::deterministic_v4(
                b"plc-wasm-fixture-transaction",
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
                            .expect("fixture precondition")
                            .object_revision,
                    )
                })
                .collect(),
            context: CommandContext {
                actor_id: "plc-wasm-fixture".to_owned(),
                can_mutate: true,
            },
            command,
        });
        assert_eq!(
            result.outcome,
            CommandOutcome::Committed,
            "{:?}",
            result.diagnostics
        );
    }
}

fn object_id(ordinal: u64) -> ObjectId {
    ObjectId(Uuid::deterministic_v4(b"plc-wasm-fixture-object", ordinal))
}

fn member_id(ordinal: u64) -> Uuid {
    Uuid::deterministic_v4(b"plc-wasm-fixture-member", ordinal)
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

fn ladder_graph(input: Uuid, output: Uuid) -> PayloadValue {
    let source_output = member_id(20);
    let contact_input = member_id(21);
    let contact_output = member_id(22);
    let coil_input = member_id(23);
    record(&[
        ("schema", PayloadValue::from("edu.lad-semantic-graph/1")),
        ("documentId", PayloadValue::from(member_id(24).to_string())),
        ("semanticRevision", PayloadValue::Unsigned(0)),
        (
            "networks",
            PayloadValue::List(vec![record(&[
                ("id", PayloadValue::from(member_id(25).to_string())),
                ("semanticOrder", PayloadValue::Unsigned(0)),
                (
                    "nodes",
                    PayloadValue::List(vec![
                        record(&[
                            ("id", PayloadValue::from(member_id(26).to_string())),
                            ("nodeKind", PayloadValue::from("power-source")),
                            ("semanticOrder", PayloadValue::Unsigned(0)),
                            (
                                "powerPorts",
                                PayloadValue::List(vec![ladder_power_port(
                                    source_output,
                                    "output",
                                )]),
                            ),
                        ]),
                        record(&[
                            ("id", PayloadValue::from(member_id(27).to_string())),
                            ("nodeKind", PayloadValue::from("contact")),
                            ("mode", PayloadValue::from("normally-open")),
                            ("semanticOrder", PayloadValue::Unsigned(1)),
                            ("operand", ladder_operand(member_id(28), input)),
                            (
                                "powerPorts",
                                PayloadValue::List(vec![
                                    ladder_power_port(contact_input, "input"),
                                    ladder_power_port(contact_output, "output"),
                                ]),
                            ),
                        ]),
                        record(&[
                            ("id", PayloadValue::from(member_id(29).to_string())),
                            ("nodeKind", PayloadValue::from("coil")),
                            ("mode", PayloadValue::from("normal")),
                            ("semanticOrder", PayloadValue::Unsigned(2)),
                            ("operand", ladder_operand(member_id(30), output)),
                            (
                                "powerPorts",
                                PayloadValue::List(vec![ladder_power_port(coil_input, "input")]),
                            ),
                        ]),
                    ]),
                ),
                (
                    "edges",
                    PayloadValue::List(vec![
                        ladder_edge(member_id(31), source_output, contact_input),
                        ladder_edge(member_id(32), contact_output, coil_input),
                    ]),
                ),
                ("branches", PayloadValue::List(Vec::new())),
            ])]),
        ),
    ])
}

fn ladder_operand(id: Uuid, member: Uuid) -> PayloadValue {
    record(&[
        ("id", PayloadValue::from(id.to_string())),
        ("kind", PayloadValue::from("caller-member")),
        ("memberId", PayloadValue::from(member.to_string())),
    ])
}

fn ladder_power_port(id: Uuid, direction: &str) -> PayloadValue {
    record(&[
        ("id", PayloadValue::from(id.to_string())),
        ("direction", PayloadValue::from(direction)),
    ])
}

fn ladder_edge(id: Uuid, source: Uuid, target: Uuid) -> PayloadValue {
    record(&[
        ("id", PayloadValue::from(id.to_string())),
        ("sourcePortId", PayloadValue::from(source.to_string())),
        ("targetPortId", PayloadValue::from(target.to_string())),
    ])
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
