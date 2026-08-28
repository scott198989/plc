use plc_program::{
    AggregateLimits, ArrayBound, CanonicalF32, CanonicalF64, CanonicalType, CanonicalValue,
    DataType, PlcValue, PrimitiveType, ScalarValue, StableUuid, StructFieldValue, StructMember,
    TypedScalar,
};

fn id(discriminator: u8) -> StableUuid {
    let mut bytes = [0_u8; 16];
    bytes[6] = 0x40;
    bytes[8] = 0x80;
    bytes[15] = discriminator;
    StableUuid::from_bytes(bytes).expect("test identity is RFC 9562 UUIDv4")
}

fn scalar(data_type: PrimitiveType, value: ScalarValue) -> PlcValue {
    PlcValue::scalar(TypedScalar::new(data_type, value).expect("oracle scalar is canonical"))
}

fn member(identity: u8, name: &str, order: u32, data_type: CanonicalType) -> StructMember {
    StructMember {
        id: id(identity),
        name: name.to_owned(),
        declared_order: order,
        data_type,
        reusable_default: None,
        comment: String::new(),
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn every_supported_scalar_literal_projects_into_the_single_recursive_value_authority() {
    let cases = [
        (
            DataType::Bool,
            CanonicalValue::Bool(true),
            PrimitiveType::Bool,
            ScalarValue::Bool(true),
        ),
        (
            DataType::SInt,
            CanonicalValue::SInt(i8::MIN),
            PrimitiveType::Sint,
            ScalarValue::Signed(i64::from(i8::MIN)),
        ),
        (
            DataType::Int,
            CanonicalValue::Int(i16::MAX),
            PrimitiveType::Int,
            ScalarValue::Signed(i64::from(i16::MAX)),
        ),
        (
            DataType::DInt,
            CanonicalValue::DInt(i32::MIN),
            PrimitiveType::Dint,
            ScalarValue::Signed(i64::from(i32::MIN)),
        ),
        (
            DataType::LInt,
            CanonicalValue::LInt(i64::MAX),
            PrimitiveType::Lint,
            ScalarValue::Signed(i64::MAX),
        ),
        (
            DataType::USInt,
            CanonicalValue::USInt(u8::MAX),
            PrimitiveType::Usint,
            ScalarValue::Unsigned(u64::from(u8::MAX)),
        ),
        (
            DataType::UInt,
            CanonicalValue::UInt(u16::MAX),
            PrimitiveType::Uint,
            ScalarValue::Unsigned(u64::from(u16::MAX)),
        ),
        (
            DataType::UDInt,
            CanonicalValue::UDInt(u32::MAX),
            PrimitiveType::Udint,
            ScalarValue::Unsigned(u64::from(u32::MAX)),
        ),
        (
            DataType::ULInt,
            CanonicalValue::ULInt(u64::MAX),
            PrimitiveType::Ulint,
            ScalarValue::Unsigned(u64::MAX),
        ),
        (
            DataType::Byte,
            CanonicalValue::Byte(u8::MAX),
            PrimitiveType::Byte,
            ScalarValue::BitString(u64::from(u8::MAX)),
        ),
        (
            DataType::Word,
            CanonicalValue::Word(u16::MAX),
            PrimitiveType::Word,
            ScalarValue::BitString(u64::from(u16::MAX)),
        ),
        (
            DataType::DWord,
            CanonicalValue::DWord(u32::MAX),
            PrimitiveType::Dword,
            ScalarValue::BitString(u64::from(u32::MAX)),
        ),
        (
            DataType::LWord,
            CanonicalValue::LWord(u64::MAX),
            PrimitiveType::Lword,
            ScalarValue::BitString(u64::MAX),
        ),
        (
            DataType::Real,
            CanonicalValue::RealBits(CanonicalF32::QUIET_NAN_BITS),
            PrimitiveType::Real,
            ScalarValue::Real(CanonicalF32::from_bits(CanonicalF32::QUIET_NAN_BITS)),
        ),
        (
            DataType::LReal,
            CanonicalValue::LRealBits(CanonicalF64::new(-0.0).bits()),
            PrimitiveType::Lreal,
            ScalarValue::Lreal(CanonicalF64::new(-0.0)),
        ),
        (
            DataType::Char,
            CanonicalValue::Char(u8::MAX),
            PrimitiveType::Char,
            ScalarValue::Char(u8::MAX),
        ),
        (
            DataType::Time,
            CanonicalValue::TimeMilliseconds(i64::MIN),
            PrimitiveType::Time,
            ScalarValue::Time(i64::MIN),
        ),
        (
            DataType::String { capacity: 254 },
            CanonicalValue::StringBytes(vec![u8::MAX; 254]),
            PrimitiveType::String(254),
            ScalarValue::String(vec![u8::MAX; 254]),
        ),
    ];

    for (program_type, program_value, primitive, scalar_value) in cases {
        let expected = PlcValue::scalar(
            TypedScalar::new(primitive, scalar_value).expect("fixed expected literal is canonical"),
        );
        assert_eq!(
            program_type.canonical_type(),
            Some(CanonicalType::Primitive(primitive))
        );
        assert!(program_value.is_compatible_with(&program_type));
        assert_eq!(program_value.plc_value_for(&program_type), Some(expected));
    }
}

#[test]
fn aggregate_literals_validate_arrays_and_structures_without_private_coercion() {
    let limits = AggregateLimits::edu21();
    let array_type = CanonicalType::Array {
        dimensions: vec![ArrayBound {
            lower: -1,
            upper: 0,
        }],
        element_type: Box::new(CanonicalType::Primitive(PrimitiveType::Dint)),
    };
    let struct_type = CanonicalType::AnonymousStruct {
        members: vec![
            member(
                1,
                "Enabled",
                0,
                CanonicalType::Primitive(PrimitiveType::Bool),
            ),
            member(2, "Samples", 1, array_type.clone()),
        ],
    };
    let literal = PlcValue::Struct(vec![
        StructFieldValue {
            member_id: id(1),
            value: scalar(PrimitiveType::Bool, ScalarValue::Bool(true)),
        },
        StructFieldValue {
            member_id: id(2),
            value: PlcValue::Array(vec![
                scalar(
                    PrimitiveType::Dint,
                    ScalarValue::Signed(i64::from(i32::MIN)),
                ),
                scalar(
                    PrimitiveType::Dint,
                    ScalarValue::Signed(i64::from(i32::MAX)),
                ),
            ]),
        },
    ]);
    struct_type.validate_value(&literal, limits).unwrap();
    let declaration_type = DataType::Aggregate(struct_type.clone());
    let declaration_value = CanonicalValue::Aggregate(literal.clone());
    assert_eq!(declaration_type.canonical_type(), Some(struct_type));
    assert!(declaration_value.is_compatible_with(&declaration_type));
    assert_eq!(
        declaration_value.plc_value_for(&declaration_type),
        Some(literal)
    );

    let wrong_shape = CanonicalValue::Aggregate(PlcValue::Struct(vec![StructFieldValue {
        member_id: id(1),
        value: scalar(PrimitiveType::Bool, ScalarValue::Bool(true)),
    }]));
    assert!(!wrong_shape.is_compatible_with(&declaration_type));
    assert_eq!(wrong_shape.plc_value_for(&declaration_type), None);

    let wrong_element_type = DataType::Aggregate(CanonicalType::Array {
        dimensions: vec![ArrayBound {
            lower: -1,
            upper: 0,
        }],
        element_type: Box::new(CanonicalType::Primitive(PrimitiveType::Udint)),
    });
    let array_literal = CanonicalValue::Aggregate(PlcValue::Array(vec![
        scalar(PrimitiveType::Dint, ScalarValue::Signed(0)),
        scalar(PrimitiveType::Dint, ScalarValue::Signed(1)),
    ]));
    assert!(!array_literal.is_compatible_with(&wrong_element_type));
    assert_eq!(array_literal.plc_value_for(&wrong_element_type), None);
}

#[test]
fn invalid_or_unsupported_literal_forms_fail_closed() {
    assert_eq!(
        CanonicalValue::RealBits(0x7fa1_2345).plc_value_for(&DataType::Real),
        None,
        "noncanonical NaN payloads never enter the program model"
    );
    assert_eq!(
        CanonicalValue::LRealBits(0x7ff0_0000_0000_0001).plc_value_for(&DataType::LReal),
        None
    );
    assert_eq!(
        CanonicalValue::StringBytes(vec![0; 3]).plc_value_for(&DataType::String { capacity: 2 }),
        None
    );
    assert_eq!(
        CanonicalValue::USInt(1).plc_value_for(&DataType::Byte),
        None,
        "same-width integer and bit-string literals do not coerce implicitly"
    );
    assert_eq!(
        CanonicalValue::Bool(true).plc_value_for(&DataType::Named("Unresolved".to_owned())),
        None
    );
    assert_eq!(DataType::String { capacity: 255 }.canonical_type(), None);
}
