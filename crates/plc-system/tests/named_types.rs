#![allow(clippy::too_many_lines)]

use std::collections::BTreeMap;

use plc_core::{
    CommandContext, CommandEnvelope, CommandOutcome, DomainCommand, Engine, NewObject, ObjectId,
    Payload, PayloadValue, ProfilePin, Project, ProjectObjectKind, TransactionId, Uuid,
};
use plc_hardware::{CanonicalType, TrainingProfile, TypeDeclarationId};
use plc_program::{BlockId, DataType, InterfaceMemberId};
use plc_system::project_software;

const NAMED_TYPE_SCHEMA: &str = "edu.named-type/1";
const PROGRAM_BLOCK_SCHEMA: &str = "edu.program-block/1";
const TAG_SCHEMA: &str = "edu.tag/1";

struct Fixture {
    engine: Engine,
    controller: ObjectId,
    next_command: u64,
}

impl Fixture {
    fn new() -> Self {
        let profile = TrainingProfile::edu21().pin();
        let root = object_id(1);
        let project = Project::new(
            Uuid::deterministic_v4(b"named-type-document", 1),
            root,
            "Named Type Project",
            ProfilePin {
                id: profile.id,
                version: profile.version,
                manifest_hash: profile.manifest_hash,
            },
        );
        let controller = object_id(2);
        let mut fixture = Self {
            engine: Engine::new(project).expect("valid project"),
            controller,
            next_command: 1,
        };
        fixture.create(
            controller,
            ProjectObjectKind::Controller,
            root,
            "PLC_1",
            "edu.controller/1",
            BTreeMap::new(),
        );
        fixture
    }

    fn create(
        &mut self,
        id: ObjectId,
        kind: ProjectObjectKind,
        parent_id: ObjectId,
        display_name: &str,
        schema: &str,
        semantic: BTreeMap<String, PayloadValue>,
    ) {
        let ordinal = self.next_command;
        self.next_command += 1;
        let envelope = CommandEnvelope {
            command_id: Uuid::deterministic_v4(b"named-type-command", ordinal),
            transaction_id: TransactionId(Uuid::deterministic_v4(
                b"named-type-transaction",
                ordinal,
            )),
            expected_document_revision: self.engine.project().document_revision(),
            expected_object_revisions: BTreeMap::from([(
                parent_id,
                self.engine
                    .project()
                    .object(parent_id)
                    .expect("parent")
                    .object_revision,
            )]),
            context: CommandContext {
                actor_id: "named-type-test".to_owned(),
                can_mutate: true,
            },
            command: DomainCommand::Create(NewObject {
                id,
                kind,
                parent_id,
                display_name: display_name.to_owned(),
                payload_schema: schema.to_owned(),
                payload: Payload {
                    semantic,
                    presentation: BTreeMap::new(),
                },
            }),
        };
        let result = self.engine.execute(&envelope);
        assert_eq!(
            result.outcome,
            CommandOutcome::Committed,
            "{:?}",
            result.diagnostics
        );
    }

    fn create_type(&mut self, id: ObjectId, display_name: &str, members: Vec<PayloadValue>) {
        self.create(
            id,
            ProjectObjectKind::TypeDefinition,
            self.controller,
            display_name,
            NAMED_TYPE_SCHEMA,
            BTreeMap::from([("members".to_owned(), PayloadValue::List(members))]),
        );
    }
}

fn object_id(ordinal: u64) -> ObjectId {
    ObjectId(Uuid::deterministic_v4(b"named-type-object", ordinal))
}

fn member_id(ordinal: u64) -> Uuid {
    Uuid::deterministic_v4(b"named-type-member", ordinal)
}

fn type_reference(id: ObjectId) -> String {
    format!("TYPE:{id}")
}

fn member(id: Uuid, name: &str, order: u64, ty: PayloadValue) -> PayloadValue {
    PayloadValue::Record(BTreeMap::from([
        ("id".to_owned(), PayloadValue::from(id.to_string())),
        ("name".to_owned(), PayloadValue::from(name)),
        ("declaredOrder".to_owned(), PayloadValue::Unsigned(order)),
        ("typeId".to_owned(), ty),
    ]))
}

fn primitive_member(id: Uuid, name: &str, order: u64, ty: &str) -> PayloadValue {
    member(id, name, order, PayloadValue::from(ty))
}

fn named_member(id: Uuid, name: &str, order: u64, target: ObjectId) -> PayloadValue {
    member(id, name, order, PayloadValue::from(type_reference(target)))
}

fn diagnostic_anchors(
    projection: &plc_system::CanonicalSoftwareProjection,
    code: &str,
) -> Vec<ObjectId> {
    projection
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code == code)
        .map(|diagnostic| diagnostic.primary_object_id)
        .collect()
}

#[test]
fn connected_named_types_project_before_blocks_and_tags() {
    let mut fixture = Fixture::new();
    let state_type = object_id(10);
    let envelope_type = object_id(11);
    fixture.create_type(
        state_type,
        "MotorState",
        vec![primitive_member(member_id(10), "Ready", 0, "BOOL")],
    );
    let anonymous_member = primitive_member(member_id(13), "Code", 0, "INT");
    let array_type = PayloadValue::Record(BTreeMap::from([
        ("kind".to_owned(), PayloadValue::from("array")),
        (
            "dimensions".to_owned(),
            PayloadValue::List(vec![PayloadValue::Record(BTreeMap::from([
                ("lower".to_owned(), PayloadValue::Signed(0)),
                ("upper".to_owned(), PayloadValue::Signed(3)),
            ]))]),
        ),
        (
            "elementType".to_owned(),
            PayloadValue::Record(BTreeMap::from([
                ("kind".to_owned(), PayloadValue::from("anonymous-struct")),
                (
                    "members".to_owned(),
                    PayloadValue::List(vec![anonymous_member]),
                ),
            ])),
        ),
    ]));
    fixture.create_type(
        envelope_type,
        "MotorEnvelope",
        vec![
            named_member(member_id(11), "State", 0, state_type),
            member(member_id(12), "History", 1, array_type),
        ],
    );

    let block = object_id(20);
    let interface_member = member_id(20);
    let interface = PayloadValue::List(vec![PayloadValue::Record(BTreeMap::from([
        (
            "id".to_owned(),
            PayloadValue::from(interface_member.to_string()),
        ),
        ("name".to_owned(), PayloadValue::from("Envelope")),
        ("role".to_owned(), PayloadValue::from("Static")),
        (
            "typeId".to_owned(),
            PayloadValue::from(type_reference(envelope_type)),
        ),
        ("declaredOrder".to_owned(), PayloadValue::Unsigned(0)),
    ]))]);
    fixture.create(
        block,
        ProjectObjectKind::ProgramBlock,
        fixture.controller,
        "MotorBlock",
        PROGRAM_BLOCK_SCHEMA,
        BTreeMap::from([
            ("blockKind".to_owned(), PayloadValue::from("FB")),
            ("engineeringNumber".to_owned(), PayloadValue::Unsigned(1)),
            ("language".to_owned(), PayloadValue::from("SCL")),
            ("sourceText".to_owned(), PayloadValue::from("")),
            ("interface".to_owned(), interface),
        ]),
    );
    let tag = object_id(21);
    fixture.create(
        tag,
        ProjectObjectKind::Tag,
        block,
        "EnvelopeMemory",
        TAG_SCHEMA,
        BTreeMap::from([
            (
                "dataType".to_owned(),
                PayloadValue::from(type_reference(envelope_type)),
            ),
            ("addressArea".to_owned(), PayloadValue::from("M")),
            ("addressIntent".to_owned(), PayloadValue::from("auto")),
            ("tagKind".to_owned(), PayloadValue::from("Memory")),
            ("blockId".to_owned(), PayloadValue::from(block.to_string())),
            (
                "memberId".to_owned(),
                PayloadValue::from(interface_member.to_string()),
            ),
        ]),
    );

    let projection = project_software(fixture.engine.project(), fixture.controller);
    assert!(projection.can_compile(), "{:#?}", projection.diagnostics());
    assert_eq!(projection.named_types().len(), 2);
    let envelope = projection
        .named_type(TypeDeclarationId::new(envelope_type.0))
        .expect("resolved envelope type");
    let CanonicalType::Named { members, .. } = envelope.canonical_type() else {
        panic!("top-level named type");
    };
    assert!(matches!(members[0].ty, CanonicalType::Named { .. }));
    assert!(matches!(
        members[1].ty,
        CanonicalType::Array { ref element_type, .. }
            if matches!(element_type.as_ref(), CanonicalType::AnonymousStruct(_))
    ));
    assert_ne!(envelope.fingerprint().0, [0; 32]);

    let block_id = BlockId::new(u128::from_be_bytes(block.0.into_bytes()));
    let projected_member_id =
        InterfaceMemberId::new(u128::from_be_bytes(interface_member.into_bytes()));
    let projected_member = projection.program().blocks()[&block_id]
        .interface
        .member(projected_member_id)
        .expect("named interface member");
    let expected = DataType::Named(type_reference(envelope_type));
    assert_eq!(projected_member.data_type, expected);
    assert_eq!(projection.tags()[0].data_type, expected);
}

#[test]
fn case_insensitive_duplicate_type_names_anchor_both_objects() {
    let mut fixture = Fixture::new();
    let first = object_id(30);
    let second = object_id(31);
    fixture.create_type(first, "MotorState", Vec::new());
    fixture.create_type(second, "motorstate", Vec::new());

    let projection = project_software(fixture.engine.project(), fixture.controller);
    assert_eq!(
        diagnostic_anchors(&projection, "EDU-SYS-2021"),
        [first, second]
    );
    assert!(projection.named_types().is_empty());
}

#[test]
fn duplicate_member_identity_name_and_order_each_block_the_type() {
    let cases = [
        (
            "identity",
            vec![
                primitive_member(member_id(40), "First", 0, "BOOL"),
                primitive_member(member_id(40), "Second", 1, "INT"),
            ],
        ),
        (
            "name",
            vec![
                primitive_member(member_id(41), "Value", 0, "BOOL"),
                primitive_member(member_id(42), "value", 1, "INT"),
            ],
        ),
        (
            "order",
            vec![
                primitive_member(member_id(43), "First", 0, "BOOL"),
                primitive_member(member_id(44), "Second", 0, "INT"),
            ],
        ),
    ];
    for (expected_message, members) in cases {
        let mut fixture = Fixture::new();
        let ty = object_id(40);
        fixture.create_type(ty, "DuplicateMember", members);

        let projection = project_software(fixture.engine.project(), fixture.controller);
        assert_eq!(diagnostic_anchors(&projection, "EDU-SYS-2020"), [ty]);
        assert!(projection.diagnostics().iter().any(|diagnostic| {
            diagnostic.primary_object_id == ty && diagnostic.message.contains(expected_message)
        }));
        assert!(projection.named_types().is_empty());
    }
}

#[test]
fn missing_named_reference_is_blocking_and_anchored_to_referrer() {
    let mut fixture = Fixture::new();
    let ty = object_id(50);
    fixture.create_type(
        ty,
        "MissingReference",
        vec![named_member(member_id(50), "Missing", 0, object_id(999))],
    );

    let projection = project_software(fixture.engine.project(), fixture.controller);
    assert_eq!(diagnostic_anchors(&projection, "EDU-SYS-2022"), [ty]);
    assert!(projection.diagnostics().iter().any(|diagnostic| {
        diagnostic.primary_object_id == ty && diagnostic.message.contains("does not exist")
    }));
}

#[test]
fn named_type_cycle_blocks_every_participant_with_type_anchors() {
    let mut fixture = Fixture::new();
    let first = object_id(60);
    let second = object_id(61);
    fixture.create_type(
        first,
        "CycleA",
        vec![named_member(member_id(60), "Next", 0, second)],
    );
    fixture.create_type(
        second,
        "CycleB",
        vec![named_member(member_id(61), "Next", 0, first)],
    );

    let projection = project_software(fixture.engine.project(), fixture.controller);
    assert_eq!(
        diagnostic_anchors(&projection, "EDU-SYS-2022"),
        [first, second]
    );
    assert!(
        projection
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code == "EDU-SYS-2022")
            .all(|diagnostic| diagnostic.message.contains("cycle detected"))
    );
    assert!(projection.named_types().is_empty());
}
