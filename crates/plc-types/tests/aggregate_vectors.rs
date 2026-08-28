use plc_types::{
    AggregateLimits, ArrayBound, CanonicalF32, CanonicalF64, CanonicalType, PlcValue,
    PrimitiveType, ScalarValue, StableUuid, StructFieldValue, StructMember, TypeError, TypedScalar,
    assign_value, store_array_element,
};
use std::collections::BTreeSet;

fn id(discriminator: u8) -> StableUuid {
    let mut bytes = [0_u8; 16];
    bytes[6] = 0x40;
    bytes[8] = 0x80;
    bytes[15] = discriminator;
    StableUuid::from_bytes(bytes).unwrap()
}

fn scalar(data_type: PrimitiveType, value: ScalarValue) -> PlcValue {
    PlcValue::Scalar(TypedScalar::new(data_type, value).unwrap())
}

fn sint(value: i8) -> PlcValue {
    scalar(PrimitiveType::Sint, ScalarValue::Signed(i64::from(value)))
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

fn field(identity: u8, value: PlcValue) -> StructFieldValue {
    StructFieldValue {
        member_id: id(identity),
        value,
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(DIGITS[usize::from(*byte >> 4)]));
        encoded.push(char::from(DIGITS[usize::from(*byte & 0x0f)]));
    }
    encoded
}

#[test]
fn nested_named_aggregate_has_one_byte_stable_golden_and_round_trips() {
    let limits = AggregateLimits::edu21();
    let data_type = CanonicalType::NamedStruct {
        id: id(1),
        // Deliberately stored out of order. Canonical form follows declared order.
        members: vec![
            StructMember {
                reusable_default: Some(scalar(
                    PrimitiveType::Real,
                    ScalarValue::Real(CanonicalF32::new(-0.0)),
                )),
                comment: "display-only text is not serialized".to_owned(),
                ..member(
                    12,
                    "Reading",
                    1,
                    CanonicalType::Primitive(PrimitiveType::Real),
                )
            },
            member(
                11,
                "Samples",
                0,
                CanonicalType::Array {
                    dimensions: vec![
                        ArrayBound {
                            lower: -1,
                            upper: 0,
                        },
                        ArrayBound { lower: 2, upper: 3 },
                    ],
                    element_type: Box::new(CanonicalType::Primitive(PrimitiveType::Sint)),
                },
            ),
        ],
    };
    let value = PlcValue::Struct(vec![
        field(
            12,
            scalar(
                PrimitiveType::Real,
                ScalarValue::Real(CanonicalF32::from_bits(CanonicalF32::QUIET_NAN_BITS)),
            ),
        ),
        field(
            11,
            PlcValue::Array(vec![sint(i8::MIN), sint(-1), sint(0), sint(i8::MAX)]),
        ),
    ]);

    let encoded = data_type.serialize_value(&value, limits).unwrap();
    assert_eq!(
        hex(&encoded),
        concat!(
            "5045532d5459502d56414c55452d31000000007b5045532d5459502d545950452d31000004",
            "00000000000040008000000000000001000000020000000000004000800000000000000b00000000",
            "000773616d706c65730202ffffffff0000000000000002000000030102000000000000004000800000",
            "000000000c00000001000772656164696e67010e018000000000000002000000000000400080000000",
            "0000000b000000000000000480ff007f0000000000004000800000000000000c7fc00000"
        )
    );
    assert_eq!(
        data_type.deserialize_value(&encoded, limits).unwrap(),
        PlcValue::Struct(vec![
            field(
                11,
                PlcValue::Array(vec![sint(i8::MIN), sint(-1), sint(0), sint(i8::MAX)]),
            ),
            field(
                12,
                scalar(
                    PrimitiveType::Real,
                    ScalarValue::Real(CanonicalF32::from_bits(CanonicalF32::QUIET_NAN_BITS)),
                ),
            ),
        ])
    );

    let mut no_comment = data_type.clone();
    let CanonicalType::NamedStruct { members, .. } = &mut no_comment else {
        unreachable!();
    };
    members[0].comment.clear();
    assert_eq!(
        data_type.canonical_bytes(limits).unwrap(),
        no_comment.canonical_bytes(limits).unwrap(),
        "comments are deliberately nonsemantic"
    );
}

#[test]
fn arrays_validate_one_through_six_dimensions_and_fail_closed_on_shape_or_capacity() {
    let limits = AggregateLimits::edu21();
    for dimension_count in 1..=6 {
        let dimensions = (0..dimension_count)
            .map(|index| ArrayBound {
                lower: -index,
                upper: -index,
            })
            .collect();
        let data_type = CanonicalType::Array {
            dimensions,
            element_type: Box::new(CanonicalType::Primitive(PrimitiveType::Lreal)),
        };
        let default = data_type.canonical_default(limits).unwrap();
        let bytes = data_type.serialize_value(&default, limits).unwrap();
        assert_eq!(
            data_type.deserialize_value(&bytes, limits).unwrap(),
            default
        );
    }

    let invalid_bound = CanonicalType::Array {
        dimensions: vec![ArrayBound { lower: 1, upper: 0 }],
        element_type: Box::new(CanonicalType::Primitive(PrimitiveType::Bool)),
    };
    assert_eq!(
        invalid_bound.validate(limits),
        Err(TypeError::InvalidArrayBound)
    );

    let seven_dimensions = CanonicalType::Array {
        dimensions: vec![ArrayBound { lower: 0, upper: 0 }; 7],
        element_type: Box::new(CanonicalType::Primitive(PrimitiveType::Bool)),
    };
    assert_eq!(
        seven_dimensions.validate(limits),
        Err(TypeError::InvalidDimensionCount)
    );

    let too_many_elements = CanonicalType::Array {
        dimensions: vec![ArrayBound {
            lower: 0,
            upper: 1_000_000,
        }],
        element_type: Box::new(CanonicalType::Primitive(PrimitiveType::Bool)),
    };
    assert_eq!(
        too_many_elements.validate(limits),
        Err(TypeError::ElementLimit)
    );

    let nested_element_explosion = CanonicalType::Array {
        dimensions: vec![ArrayBound {
            lower: 0,
            upper: 1_000,
        }],
        element_type: Box::new(CanonicalType::Array {
            dimensions: vec![ArrayBound {
                lower: 0,
                upper: 1_000,
            }],
            element_type: Box::new(CanonicalType::Primitive(PrimitiveType::Bool)),
        }),
    };
    assert_eq!(
        nested_element_explosion.validate(limits),
        Err(TypeError::ElementLimit)
    );

    let exact_two = CanonicalType::Array {
        dimensions: vec![ArrayBound {
            lower: -1,
            upper: 0,
        }],
        element_type: Box::new(CanonicalType::Primitive(PrimitiveType::Bool)),
    };
    assert_eq!(
        exact_two.validate(AggregateLimits {
            max_array_elements: 1_000_001,
            ..limits
        }),
        Err(TypeError::InvalidLimits)
    );
    assert_eq!(
        exact_two.validate_value(
            &PlcValue::Array(vec![scalar(PrimitiveType::Bool, ScalarValue::Bool(false),)]),
            limits
        ),
        Err(TypeError::ValueShapeMismatch)
    );
    let value = exact_two.canonical_default(limits).unwrap();
    let constrained = AggregateLimits {
        max_serialized_bytes: 24,
        ..limits
    };
    assert_eq!(
        exact_two.serialize_value(&value, constrained),
        Err(TypeError::CapacityExceeded)
    );
}

#[test]
fn anonymous_assignment_uses_shape_but_remaps_stable_member_identity() {
    let limits = AggregateLimits::edu21();
    let source = CanonicalType::AnonymousStruct {
        members: vec![member(
            20,
            "Count",
            0,
            CanonicalType::Primitive(PrimitiveType::Dint),
        )],
    };
    let mut destination_member = member(
        21,
        "COUNT",
        0,
        CanonicalType::Primitive(PrimitiveType::Dint),
    );
    destination_member.comment = "different nonsemantic comment".to_owned();
    destination_member.reusable_default =
        Some(scalar(PrimitiveType::Dint, ScalarValue::Signed(99)));
    let destination = CanonicalType::AnonymousStruct {
        members: vec![destination_member],
    };
    assert!(
        source
            .assignment_compatible_with(&destination, limits)
            .unwrap()
    );

    let assigned = assign_value(
        &source,
        &PlcValue::Struct(vec![field(
            20,
            scalar(PrimitiveType::Dint, ScalarValue::Signed(-7)),
        )]),
        &destination,
        limits,
    )
    .unwrap();
    assert_eq!(
        assigned,
        PlcValue::Struct(vec![field(
            21,
            scalar(PrimitiveType::Dint, ScalarValue::Signed(-7)),
        )])
    );

    let named_a = CanonicalType::NamedStruct {
        id: id(30),
        members: vec![member(
            31,
            "Count",
            0,
            CanonicalType::Primitive(PrimitiveType::Dint),
        )],
    };
    let named_b = CanonicalType::NamedStruct {
        id: id(32),
        members: vec![member(
            31,
            "Count",
            0,
            CanonicalType::Primitive(PrimitiveType::Dint),
        )],
    };
    assert!(
        !named_a
            .assignment_compatible_with(&named_b, limits)
            .unwrap()
    );
    assert!(!source.assignment_compatible_with(&named_a, limits).unwrap());

    let named_stale_member_identity = CanonicalType::NamedStruct {
        id: id(30),
        members: vec![member(
            33,
            "Count",
            0,
            CanonicalType::Primitive(PrimitiveType::Dint),
        )],
    };
    assert!(
        !named_a
            .assignment_compatible_with(&named_stale_member_identity, limits)
            .unwrap()
    );
}

#[test]
fn bounds_and_replacement_are_validated_before_an_array_store() {
    let limits = AggregateLimits::edu21();
    let data_type = CanonicalType::Array {
        dimensions: vec![
            ArrayBound {
                lower: -1,
                upper: 0,
            },
            ArrayBound { lower: 3, upper: 4 },
        ],
        element_type: Box::new(CanonicalType::Primitive(PrimitiveType::Sint)),
    };
    let original = PlcValue::Array(vec![sint(1), sint(2), sint(3), sint(4)]);
    let changed = store_array_element(&data_type, &original, &[0, 3], &sint(99), limits).unwrap();
    assert_eq!(
        changed,
        PlcValue::Array(vec![sint(1), sint(2), sint(99), sint(4)])
    );
    assert_eq!(
        original,
        PlcValue::Array(vec![sint(1), sint(2), sint(3), sint(4)])
    );

    assert_eq!(
        store_array_element(&data_type, &original, &[0, 5], &sint(99), limits),
        Err(TypeError::Bounds)
    );
    assert_eq!(
        store_array_element(
            &data_type,
            &original,
            &[0, 3],
            &scalar(PrimitiveType::Byte, ScalarValue::BitString(99)),
            limits,
        ),
        Err(TypeError::ValueTypeMismatch)
    );
}

#[test]
fn scalar_identity_float_bits_and_array_bounds_are_part_of_canonical_truth() {
    let limits = AggregateLimits::edu21();
    let sint_type = CanonicalType::Primitive(PrimitiveType::Sint);
    let byte_type = CanonicalType::Primitive(PrimitiveType::Byte);
    assert_ne!(
        sint_type.canonical_bytes(limits).unwrap(),
        byte_type.canonical_bytes(limits).unwrap()
    );
    assert!(
        !sint_type
            .assignment_compatible_with(&byte_type, limits)
            .unwrap()
    );

    let negative_zero = CanonicalType::Primitive(PrimitiveType::Lreal)
        .serialize_value(
            &scalar(
                PrimitiveType::Lreal,
                ScalarValue::Lreal(CanonicalF64::new(-0.0)),
            ),
            limits,
        )
        .unwrap();
    let positive_zero = CanonicalType::Primitive(PrimitiveType::Lreal)
        .serialize_value(
            &scalar(
                PrimitiveType::Lreal,
                ScalarValue::Lreal(CanonicalF64::new(0.0)),
            ),
            limits,
        )
        .unwrap();
    assert_ne!(negative_zero, positive_zero);

    let nan_a = scalar(
        PrimitiveType::Lreal,
        ScalarValue::Lreal(CanonicalF64::from_bits(0x7ff0_0000_0000_0001)),
    );
    let nan_b = scalar(
        PrimitiveType::Lreal,
        ScalarValue::Lreal(CanonicalF64::from_bits(0x7fff_ffff_ffff_ffff)),
    );
    let nan_type = CanonicalType::Primitive(PrimitiveType::Lreal);
    assert_eq!(
        nan_type.serialize_value(&nan_a, limits).unwrap(),
        nan_type.serialize_value(&nan_b, limits).unwrap()
    );

    let array_a = CanonicalType::Array {
        dimensions: vec![ArrayBound { lower: 0, upper: 1 }],
        element_type: Box::new(CanonicalType::Primitive(PrimitiveType::Sint)),
    };
    let array_b = CanonicalType::Array {
        dimensions: vec![ArrayBound { lower: 1, upper: 2 }],
        element_type: Box::new(CanonicalType::Primitive(PrimitiveType::Sint)),
    };
    assert!(
        !array_a
            .assignment_compatible_with(&array_b, limits)
            .unwrap()
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn every_primitive_identity_round_trips_inside_the_recursive_codec() {
    let limits = AggregateLimits::edu21();
    let vectors = vec![
        (
            PrimitiveType::Bool,
            scalar(PrimitiveType::Bool, ScalarValue::Bool(true)),
        ),
        (PrimitiveType::Sint, sint(i8::MIN)),
        (
            PrimitiveType::Int,
            scalar(PrimitiveType::Int, ScalarValue::Signed(i64::from(i16::MIN))),
        ),
        (
            PrimitiveType::Dint,
            scalar(
                PrimitiveType::Dint,
                ScalarValue::Signed(i64::from(0x0102_0304)),
            ),
        ),
        (
            PrimitiveType::Lint,
            scalar(PrimitiveType::Lint, ScalarValue::Signed(i64::MIN)),
        ),
        (
            PrimitiveType::Usint,
            scalar(
                PrimitiveType::Usint,
                ScalarValue::Unsigned(u64::from(u8::MAX)),
            ),
        ),
        (
            PrimitiveType::Uint,
            scalar(
                PrimitiveType::Uint,
                ScalarValue::Unsigned(u64::from(u16::MAX)),
            ),
        ),
        (
            PrimitiveType::Udint,
            scalar(
                PrimitiveType::Udint,
                ScalarValue::Unsigned(u64::from(u32::MAX)),
            ),
        ),
        (
            PrimitiveType::Ulint,
            scalar(PrimitiveType::Ulint, ScalarValue::Unsigned(u64::MAX)),
        ),
        (
            PrimitiveType::Byte,
            scalar(
                PrimitiveType::Byte,
                ScalarValue::BitString(u64::from(u8::MAX)),
            ),
        ),
        (
            PrimitiveType::Word,
            scalar(
                PrimitiveType::Word,
                ScalarValue::BitString(u64::from(u16::MAX)),
            ),
        ),
        (
            PrimitiveType::Dword,
            scalar(
                PrimitiveType::Dword,
                ScalarValue::BitString(u64::from(u32::MAX)),
            ),
        ),
        (
            PrimitiveType::Lword,
            scalar(
                PrimitiveType::Lword,
                ScalarValue::BitString(0x0102_0304_0506_0708),
            ),
        ),
        (
            PrimitiveType::Real,
            scalar(
                PrimitiveType::Real,
                ScalarValue::Real(CanonicalF32::new(f32::INFINITY)),
            ),
        ),
        (
            PrimitiveType::Lreal,
            scalar(
                PrimitiveType::Lreal,
                ScalarValue::Lreal(CanonicalF64::new(f64::NEG_INFINITY)),
            ),
        ),
        (
            PrimitiveType::Char,
            scalar(PrimitiveType::Char, ScalarValue::Char(u8::MAX)),
        ),
        (
            PrimitiveType::String(3),
            scalar(
                PrimitiveType::String(3),
                ScalarValue::String(vec![0, 65, 255]),
            ),
        ),
        (
            PrimitiveType::Time,
            scalar(PrimitiveType::Time, ScalarValue::Time(i64::MIN)),
        ),
    ];
    let mut type_encodings = BTreeSet::new();
    for (primitive, value) in vectors {
        let data_type = CanonicalType::Primitive(primitive);
        assert!(type_encodings.insert(data_type.canonical_bytes(limits).unwrap()));
        let first = data_type.serialize_value(&value, limits).unwrap();
        let second = data_type.serialize_value(&value, limits).unwrap();
        assert_eq!(first, second);
        assert_eq!(data_type.deserialize_value(&first, limits).unwrap(), value);
    }

    let dint_bytes = CanonicalType::Primitive(PrimitiveType::Dint)
        .serialize_value(
            &scalar(
                PrimitiveType::Dint,
                ScalarValue::Signed(i64::from(0x0102_0304)),
            ),
            limits,
        )
        .unwrap();
    assert!(dint_bytes.ends_with(&[0x01, 0x02, 0x03, 0x04]));
    let lword_bytes = CanonicalType::Primitive(PrimitiveType::Lword)
        .serialize_value(
            &scalar(
                PrimitiveType::Lword,
                ScalarValue::BitString(0x0102_0304_0506_0708),
            ),
            limits,
        )
        .unwrap();
    assert!(lword_bytes.ends_with(&[1, 2, 3, 4, 5, 6, 7, 8]));
}

#[test]
fn decoder_rejects_wrong_type_truncation_trailing_bytes_and_noncanonical_nan() {
    let limits = AggregateLimits::edu21();
    let real = CanonicalType::Primitive(PrimitiveType::Real);
    let value = scalar(
        PrimitiveType::Real,
        ScalarValue::Real(CanonicalF32::new(f32::NAN)),
    );
    let bytes = real.serialize_value(&value, limits).unwrap();

    let lreal = CanonicalType::Primitive(PrimitiveType::Lreal);
    assert_eq!(
        lreal.deserialize_value(&bytes, limits),
        Err(TypeError::TypeHeaderMismatch)
    );
    assert_eq!(
        real.deserialize_value(&bytes[..bytes.len() - 1], limits),
        Err(TypeError::Truncated)
    );
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert_eq!(
        real.deserialize_value(&trailing, limits),
        Err(TypeError::TrailingBytes)
    );

    let mut noncanonical_nan = bytes;
    let length = noncanonical_nan.len();
    noncanonical_nan[length - 4..].copy_from_slice(&0x7fa1_2345_u32.to_be_bytes());
    assert_eq!(
        real.deserialize_value(&noncanonical_nan, limits),
        Err(TypeError::NonCanonicalEncoding)
    );
}

#[test]
fn member_identity_name_order_and_default_mismatches_are_rejected() {
    let limits = AggregateLimits::edu21();
    let duplicate_names = CanonicalType::AnonymousStruct {
        members: vec![
            member(
                40,
                "Value",
                0,
                CanonicalType::Primitive(PrimitiveType::Bool),
            ),
            member(
                41,
                "VALUE",
                1,
                CanonicalType::Primitive(PrimitiveType::Bool),
            ),
        ],
    };
    assert_eq!(
        duplicate_names.validate(limits),
        Err(TypeError::DuplicateMemberName)
    );

    let duplicate_identity = CanonicalType::AnonymousStruct {
        members: vec![
            member(
                42,
                "First",
                0,
                CanonicalType::Primitive(PrimitiveType::Bool),
            ),
            member(
                42,
                "Second",
                1,
                CanonicalType::Primitive(PrimitiveType::Bool),
            ),
        ],
    };
    assert_eq!(
        duplicate_identity.validate(limits),
        Err(TypeError::DuplicateMemberIdentity)
    );

    let mut bad_default = member(43, "Flag", 0, CanonicalType::Primitive(PrimitiveType::Bool));
    bad_default.reusable_default = Some(sint(1));
    assert_eq!(
        CanonicalType::AnonymousStruct {
            members: vec![bad_default],
        }
        .validate(limits),
        Err(TypeError::DefaultValueMismatch)
    );
}
