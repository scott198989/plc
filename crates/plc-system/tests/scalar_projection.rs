#![allow(clippy::too_many_lines)]

use std::collections::BTreeMap;

use plc_core::{
    CommandContext, CommandEnvelope, CommandOutcome, DomainCommand, Engine, NewObject, ObjectId,
    Payload, PayloadValue, ProfilePin, Project, ProjectObjectKind, TransactionId, Uuid,
};
use plc_hardware::TrainingProfile;
use plc_program::{BlockId, CanonicalValue, DataType, InterfaceMemberId};
use plc_system::{CanonicalSoftwareProjection, project_software};

const DATA_BLOCK_SCHEMA: &str = "edu.data-block/1";

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
            Uuid::deterministic_v4(b"scalar-projection-document", 1),
            root,
            "Scalar Projection Project",
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
        let result = self.engine.execute(&CommandEnvelope {
            command_id: Uuid::deterministic_v4(b"scalar-projection-command", ordinal),
            transaction_id: TransactionId(Uuid::deterministic_v4(
                b"scalar-projection-transaction",
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
                actor_id: "scalar-projection-test".to_owned(),
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
        });
        assert_eq!(
            result.outcome,
            CommandOutcome::Committed,
            "{:?}",
            result.diagnostics
        );
    }

    fn create_global_db(&mut self, id: ObjectId, members: Vec<PayloadValue>) {
        self.create(
            id,
            ProjectObjectKind::DataBlock,
            self.controller,
            "ScalarDB",
            DATA_BLOCK_SCHEMA,
            BTreeMap::from([
                ("dbKind".to_owned(), PayloadValue::from("GlobalDB")),
                ("engineeringNumber".to_owned(), PayloadValue::Unsigned(1)),
                ("members".to_owned(), PayloadValue::List(members)),
            ]),
        );
    }

    fn projection(&self) -> CanonicalSoftwareProjection {
        project_software(self.engine.project(), self.controller)
    }
}

fn object_id(ordinal: u64) -> ObjectId {
    ObjectId(Uuid::deterministic_v4(b"scalar-projection-object", ordinal))
}

fn member_id(ordinal: u64) -> Uuid {
    Uuid::deterministic_v4(b"scalar-projection-member", ordinal)
}

fn record(fields: impl IntoIterator<Item = (&'static str, PayloadValue)>) -> PayloadValue {
    PayloadValue::Record(
        fields
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

fn interface_member(
    ordinal: u64,
    name: &str,
    type_id: &str,
    default_value: PayloadValue,
) -> PayloadValue {
    record([
        ("id", PayloadValue::from(member_id(ordinal).to_string())),
        ("name", PayloadValue::from(name)),
        ("role", PayloadValue::from("Static")),
        ("typeId", PayloadValue::from(type_id)),
        ("declaredOrder", PayloadValue::Unsigned(ordinal)),
        ("defaultValue", default_value),
    ])
}

fn signed(type_id: &'static str, value: &'static str) -> PayloadValue {
    record([
        ("kind", PayloadValue::from("signed-integer")),
        ("typeId", PayloadValue::from(type_id)),
        ("value", PayloadValue::from(value)),
    ])
}

fn unsigned(type_id: &'static str, value: &'static str) -> PayloadValue {
    record([
        ("kind", PayloadValue::from("unsigned-integer")),
        ("typeId", PayloadValue::from(type_id)),
        ("value", PayloadValue::from(value)),
    ])
}

fn bit_string(type_id: &'static str, bits: &'static str) -> PayloadValue {
    record([
        ("bitsHex", PayloadValue::from(bits)),
        ("kind", PayloadValue::from("bit-string")),
        ("typeId", PayloadValue::from(type_id)),
    ])
}

fn floating(type_id: &'static str, bits: &'static str) -> PayloadValue {
    record([
        ("ieeeHex", PayloadValue::from(bits)),
        ("kind", PayloadValue::from("floating")),
        ("typeId", PayloadValue::from(type_id)),
    ])
}

fn canonical_string(capacity: u64, code_units: &[u64]) -> PayloadValue {
    record([
        ("capacity", PayloadValue::Unsigned(capacity)),
        (
            "codeUnits",
            PayloadValue::List(
                code_units
                    .iter()
                    .copied()
                    .map(PayloadValue::Unsigned)
                    .collect(),
            ),
        ),
        ("kind", PayloadValue::from("string")),
        ("typeId", PayloadValue::from("STRING")),
    ])
}

#[test]
fn every_phase2_scalar_projects_with_exact_type_and_value_identity() {
    let cases = vec![
        (
            "BoolValue",
            "BOOL",
            record([
                ("kind", PayloadValue::from("bool")),
                ("typeId", PayloadValue::from("BOOL")),
                ("value", PayloadValue::Bool(true)),
            ]),
            DataType::Bool,
            CanonicalValue::Bool(true),
        ),
        (
            "SintValue",
            "SINT",
            signed("SINT", "-128"),
            DataType::SInt,
            CanonicalValue::SInt(i8::MIN),
        ),
        (
            "IntValue",
            "INT",
            signed("INT", "-32768"),
            DataType::Int,
            CanonicalValue::Int(i16::MIN),
        ),
        (
            "DintValue",
            "DINT",
            signed("DINT", "-2147483648"),
            DataType::DInt,
            CanonicalValue::DInt(i32::MIN),
        ),
        (
            "LintValue",
            "LINT",
            signed("LINT", "-9223372036854775808"),
            DataType::LInt,
            CanonicalValue::LInt(i64::MIN),
        ),
        (
            "UsintValue",
            "USINT",
            unsigned("USINT", "255"),
            DataType::USInt,
            CanonicalValue::USInt(u8::MAX),
        ),
        (
            "UintValue",
            "UINT",
            unsigned("UINT", "65535"),
            DataType::UInt,
            CanonicalValue::UInt(u16::MAX),
        ),
        (
            "UdintValue",
            "UDINT",
            unsigned("UDINT", "4294967295"),
            DataType::UDInt,
            CanonicalValue::UDInt(u32::MAX),
        ),
        (
            "UlintValue",
            "ULINT",
            unsigned("ULINT", "18446744073709551615"),
            DataType::ULInt,
            CanonicalValue::ULInt(u64::MAX),
        ),
        (
            "ByteValue",
            "BYTE",
            bit_string("BYTE", "FF"),
            DataType::Byte,
            CanonicalValue::Byte(u8::MAX),
        ),
        (
            "WordValue",
            "WORD",
            bit_string("WORD", "FFFF"),
            DataType::Word,
            CanonicalValue::Word(u16::MAX),
        ),
        (
            "DwordValue",
            "DWORD",
            bit_string("DWORD", "FFFFFFFF"),
            DataType::DWord,
            CanonicalValue::DWord(u32::MAX),
        ),
        (
            "LwordValue",
            "LWORD",
            bit_string("LWORD", "FFFFFFFFFFFFFFFF"),
            DataType::LWord,
            CanonicalValue::LWord(u64::MAX),
        ),
        (
            "RealNegativeZero",
            "REAL",
            floating("REAL", "80000000"),
            DataType::Real,
            CanonicalValue::RealBits(0x8000_0000),
        ),
        (
            "LrealCanonicalNan",
            "LREAL",
            floating("LREAL", "7FF8000000000000"),
            DataType::LReal,
            CanonicalValue::LRealBits(0x7FF8_0000_0000_0000),
        ),
        (
            "CharValue",
            "CHAR",
            record([
                ("codeUnit", PayloadValue::Unsigned(255)),
                ("kind", PayloadValue::from("char")),
                ("typeId", PayloadValue::from("CHAR")),
            ]),
            DataType::Char,
            CanonicalValue::Char(u8::MAX),
        ),
        (
            "TimeValue",
            "TIME",
            record([
                ("kind", PayloadValue::from("time")),
                ("milliseconds", PayloadValue::from("-9223372036854775808")),
                ("typeId", PayloadValue::from("TIME")),
            ]),
            DataType::Time,
            CanonicalValue::TimeMilliseconds(i64::MIN),
        ),
        (
            "StringValue",
            "STRING[4]",
            canonical_string(4, &[0, 65, 255, 66]),
            DataType::String { capacity: 4 },
            CanonicalValue::StringBytes(vec![0, 65, 255, 66]),
        ),
    ];
    let mut fixture = Fixture::new();
    let db_id = object_id(100);
    fixture.create_global_db(
        db_id,
        cases
            .iter()
            .enumerate()
            .map(|(index, (name, type_id, value, _, _))| {
                interface_member(index as u64 + 1, name, type_id, value.clone())
            })
            .collect(),
    );

    let projection = fixture.projection();
    assert!(projection.can_compile(), "{:#?}", projection.diagnostics());
    let db =
        &projection.program().blocks()[&BlockId::new(u128::from_be_bytes(db_id.0.into_bytes()))];
    for (index, (_, _, _, expected_type, expected_value)) in cases.iter().enumerate() {
        let member = db
            .interface
            .member(InterfaceMemberId::new(u128::from_be_bytes(
                member_id(index as u64 + 1).into_bytes(),
            )))
            .expect("projected scalar member");
        assert_eq!(&member.data_type, expected_type);
        assert_eq!(member.default_value.as_ref(), Some(expected_value));
        assert!(expected_value.is_compatible_with(expected_type));
    }
}

#[test]
fn malformed_or_mismatched_scalar_payloads_fail_closed_at_projection() {
    let PayloadValue::Record(mut extra_field) = signed("INT", "1") else {
        unreachable!();
    };
    extra_field.insert("opaque".to_owned(), PayloadValue::Bool(true));
    let cases = vec![
        (
            "type mismatch",
            "INT",
            signed("DINT", "1"),
            "does not match declared type",
        ),
        (
            "signed overflow",
            "SINT",
            signed("SINT", "128"),
            "SINT value is out of range",
        ),
        (
            "unsigned overflow",
            "USINT",
            unsigned("USINT", "256"),
            "USINT value is out of range",
        ),
        (
            "noncanonical signed decimal",
            "INT",
            signed("INT", "01"),
            "canonical DecimalInt64",
        ),
        (
            "lowercase bit string",
            "BYTE",
            bit_string("BYTE", "ff"),
            "uppercase hexadecimal",
        ),
        (
            "noncanonical real NaN",
            "REAL",
            floating("REAL", "7F800001"),
            "REAL NaN must use canonical bit pattern",
        ),
        (
            "direct noncanonical real NaN",
            "REAL",
            PayloadValue::Unsigned(0x7F80_0001),
            "REAL NaN must use canonical bit pattern",
        ),
        (
            "char overflow",
            "CHAR",
            record([
                ("codeUnit", PayloadValue::Unsigned(256)),
                ("kind", PayloadValue::from("char")),
                ("typeId", PayloadValue::from("CHAR")),
            ]),
            "CHAR code unit is out of range",
        ),
        (
            "string capacity mismatch",
            "STRING[2]",
            canonical_string(1, &[65]),
            "does not match declared capacity",
        ),
        (
            "string value over capacity",
            "STRING[2]",
            canonical_string(2, &[65, 66, 67]),
            "exceeds its declared capacity",
        ),
        (
            "declared string capacity over limit",
            "STRING[255]",
            canonical_string(255, &[]),
            "exceeds the canonical limit",
        ),
        (
            "unexpected contract field",
            "INT",
            PayloadValue::Record(extra_field),
            "fields must be exactly",
        ),
    ];

    for (index, (name, type_id, value, expected_message)) in cases.into_iter().enumerate() {
        let mut fixture = Fixture::new();
        let db_id = object_id(200 + index as u64);
        fixture.create_global_db(
            db_id,
            vec![interface_member(100 + index as u64, name, type_id, value)],
        );
        let projection = fixture.projection();
        assert!(
            !projection.can_compile(),
            "case '{name}' unexpectedly compiled"
        );
        assert!(
            projection.diagnostics().iter().any(|diagnostic| {
                diagnostic.primary_object_id == db_id
                    && diagnostic.code == "EDU-SYS-2003"
                    && diagnostic.message.contains(expected_message)
            }),
            "case '{name}' did not emit '{expected_message}': {:#?}",
            projection.diagnostics()
        );
    }
}
